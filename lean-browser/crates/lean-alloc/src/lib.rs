//! Tagging, counting global allocator (plan §3, §7).
//!
//! [`LeanAlloc`] wraps [`std::alloc::System`] and attributes every
//! allocation to the *component tag* that is current on the allocating
//! thread ([`Tag`], set with [`scope`]). For each tag it keeps live bytes,
//! peak live bytes and allocation counts, plus an optional budget: when a
//! tag's live bytes exceed its budget the allocator records the overrun
//! and, in strict mode (debug builds and harness runs), aborts the process
//! with the tag's name on stderr. Release builds only record it, so the
//! harness can read it back through [`snapshot`].
//!
//! Frees are attributed to the tag stored in a header written in front of
//! every block (16 bytes, or the requested alignment if larger), so per-tag
//! live counts are exact regardless of which tag is current when memory is
//! released. That header is the whole cost of the allocator.
//!
//! Nothing here allocates, locks, or panics: the allocator is called from
//! inside `malloc`, so it may only touch atomics, thread-local `Cell`s and
//! raw stderr writes.
//!
//! ```
//! use lean_alloc::{scope, Tag};
//!
//! #[global_allocator]
//! static ALLOC: lean_alloc::LeanAlloc = lean_alloc::LeanAlloc;
//!
//! let _layout = scope(Tag::Layout);
//! let v: Vec<u8> = vec![0; 4096];
//! assert!(lean_alloc::snapshot().live(Tag::Layout) >= 4096);
//! drop(v);
//! ```

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::*};

/// Component tags. Each is one row in the stats table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Tag {
    /// Allocations made while no scope is active (startup, std internals).
    Untagged = 0,
    /// Page file mapping bookkeeping and the `heights[]` array.
    PageFile = 1,
    /// Viewport layout tree (transient, dropped after paint).
    Layout = 2,
    /// Glyph cache (A8 masks).
    GlyphCache = 3,
    /// Strip buffer.
    Strip = 4,
    /// Image decode scratch.
    Image = 5,
    /// Text shaping contexts and scratch.
    Text = 6,
    /// IPC framing buffers.
    Ipc = 7,
    /// Renderer-owned form state.
    FormState = 8,
    /// Window system client state (winit, softbuffer, wayland).
    Window = 9,
    /// Accessibility tree.
    A11y = 10,
    /// Loader: HTML parse and DOM arena.
    Dom = 11,
    /// Loader: CSS parse and cascade.
    Cascade = 12,
    /// Loader: compression passes and page-file serialization.
    Compress = 13,
    /// Loader: network fetch buffers.
    Fetch = 14,
    /// Harness and tests.
    Harness = 15,
}

impl Tag {
    /// Number of tags.
    pub const COUNT: usize = 16;

    /// All tags in id order.
    pub const ALL: [Tag; Tag::COUNT] = [
        Tag::Untagged,
        Tag::PageFile,
        Tag::Layout,
        Tag::GlyphCache,
        Tag::Strip,
        Tag::Image,
        Tag::Text,
        Tag::Ipc,
        Tag::FormState,
        Tag::Window,
        Tag::A11y,
        Tag::Dom,
        Tag::Cascade,
        Tag::Compress,
        Tag::Fetch,
        Tag::Harness,
    ];

    /// Short name for reports.
    pub const fn name(self) -> &'static str {
        match self {
            Tag::Untagged => "untagged",
            Tag::PageFile => "page_file",
            Tag::Layout => "layout",
            Tag::GlyphCache => "glyph_cache",
            Tag::Strip => "strip",
            Tag::Image => "image",
            Tag::Text => "text",
            Tag::Ipc => "ipc",
            Tag::FormState => "form_state",
            Tag::Window => "window",
            Tag::A11y => "a11y",
            Tag::Dom => "dom",
            Tag::Cascade => "cascade",
            Tag::Compress => "compress",
            Tag::Fetch => "fetch",
            Tag::Harness => "harness",
        }
    }

    fn from_index(i: u8) -> Tag {
        Tag::ALL.get(i as usize).copied().unwrap_or(Tag::Untagged)
    }
}

/// Per-tag counters.
struct Counters {
    live: AtomicUsize,
    peak: AtomicUsize,
    allocs: AtomicU64,
    frees: AtomicU64,
    budget: AtomicUsize,
    exceeded: AtomicBool,
}

impl Counters {
    const fn new() -> Self {
        Counters {
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            allocs: AtomicU64::new(0),
            frees: AtomicU64::new(0),
            budget: AtomicUsize::new(usize::MAX),
            exceeded: AtomicBool::new(false),
        }
    }
}

static COUNTERS: [Counters; Tag::COUNT] = [const { Counters::new() }; Tag::COUNT];
static TOTAL: Counters = Counters::new();
static STRICT: AtomicBool = AtomicBool::new(cfg!(debug_assertions));

thread_local! {
    static CURRENT: Cell<u8> = const { Cell::new(0) };
}

/// Minimum header written in front of every block. The header grows to the
/// requested alignment when that is larger, so the returned pointer is
/// always correctly aligned.
const HEADER: usize = 16;

/// The allocator. Install with `#[global_allocator]`.
pub struct LeanAlloc;

impl LeanAlloc {
    /// Header size for a request: the tag byte plus padding up to the
    /// larger of 16 and the requested alignment.
    #[inline]
    fn header(layout: Layout) -> usize {
        layout.align().max(HEADER)
    }

    #[inline]
    fn outer_layout(layout: Layout) -> Option<Layout> {
        let header = Self::header(layout);
        let size = layout.size().checked_add(header)?;
        Layout::from_size_align(size, header).ok()
    }
}

// SAFETY: every block handed out is `header(layout)` bytes into a block
// obtained from `System` with a layout of size + header and alignment
// header; `dealloc` and `realloc` reconstruct the same outer layout from the
// same `layout`, so the pairing with `System` is exact.
unsafe impl GlobalAlloc for LeanAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some(outer) = Self::outer_layout(layout) else {
            return std::ptr::null_mut();
        };
        let header = Self::header(layout);
        // SAFETY: `outer` is a valid non-zero-size layout.
        let base = unsafe { System.alloc(outer) };
        if base.is_null() {
            return base;
        }
        let tag = CURRENT.with(Cell::get);
        // SAFETY: `base` points to at least HEADER bytes.
        unsafe { base.write(tag) };
        record_alloc(tag, layout.size());
        // SAFETY: header < outer.size().
        unsafe { base.add(header) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` was returned by `alloc`, so `ptr - header` is the
        // base of the outer block and holds the tag byte.
        let base = unsafe { ptr.sub(Self::header(layout)) };
        let tag = unsafe { base.read() };
        record_free(tag, layout.size());
        let outer = Self::outer_layout(layout).expect("layout was valid at alloc time");
        // SAFETY: same outer layout as the matching `alloc`.
        unsafe { System.dealloc(base, outer) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let header = Self::header(layout);
        // SAFETY: as in `dealloc`.
        let base = unsafe { ptr.sub(header) };
        let tag = unsafe { base.read() };
        let outer = Self::outer_layout(layout).expect("layout was valid at alloc time");
        let Some(new_outer_size) = new_size.checked_add(header) else {
            return std::ptr::null_mut();
        };
        // SAFETY: `base`/`outer` match the original allocation and the new
        // size is non-zero.
        let new_base = unsafe { System.realloc(base, outer, new_outer_size) };
        if new_base.is_null() {
            return new_base;
        }
        record_free(tag, layout.size());
        record_alloc(tag, new_size);
        // The tag byte moved with the block contents; nothing to rewrite.
        // SAFETY: header < new_outer_size.
        unsafe { new_base.add(header) }
    }
}

#[inline]
fn bump_peak(c: &Counters, live: usize) {
    let mut peak = c.peak.load(Relaxed);
    while live > peak {
        match c.peak.compare_exchange_weak(peak, live, Relaxed, Relaxed) {
            Ok(_) => break,
            Err(p) => peak = p,
        }
    }
}

#[inline]
fn record_alloc(tag: u8, size: usize) {
    let c = &COUNTERS[tag as usize];
    let live = c.live.fetch_add(size, Relaxed) + size;
    c.allocs.fetch_add(1, Relaxed);
    bump_peak(c, live);
    let total = TOTAL.live.fetch_add(size, Relaxed) + size;
    TOTAL.allocs.fetch_add(1, Relaxed);
    bump_peak(&TOTAL, total);
    if live > c.budget.load(Relaxed) && !c.exceeded.swap(true, Relaxed) {
        budget_exceeded(Tag::from_index(tag), live);
    }
}

#[inline]
fn record_free(tag: u8, size: usize) {
    let c = &COUNTERS[tag as usize];
    c.live.fetch_sub(size, Relaxed);
    c.frees.fetch_add(1, Relaxed);
    TOTAL.live.fetch_sub(size, Relaxed);
    TOTAL.frees.fetch_add(1, Relaxed);
}

/// Called once per tag on the first overrun. Writes to fd 2 directly
/// (no formatting machinery, no allocation) and aborts in strict mode.
#[cold]
fn budget_exceeded(tag: Tag, live: usize) {
    use std::io::Write;
    let mut buf = [0u8; 96];
    let mut n = 0;
    for part in [
        b"lean-alloc: budget exceeded for tag `" as &[u8],
        tag.name().as_bytes(),
        b"` at ",
    ] {
        for &b in part {
            if n < buf.len() {
                buf[n] = b;
                n += 1;
            }
        }
    }
    let mut digits = [0u8; 20];
    let mut d = digits.len();
    let mut v = live;
    loop {
        d -= 1;
        digits[d] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    for &b in &digits[d..] {
        if n < buf.len() {
            buf[n] = b;
            n += 1;
        }
    }
    for &b in b" bytes\n" {
        if n < buf.len() {
            buf[n] = b;
            n += 1;
        }
    }
    let _ = std::io::stderr().write_all(&buf[..n]);
    if STRICT.load(Relaxed) {
        std::process::abort();
    }
}

/// Guard returned by [`scope`]; restores the previous tag when dropped.
#[must_use = "the tag is only active while the guard is alive"]
pub struct ScopeGuard {
    previous: u8,
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        CURRENT.with(|c| c.set(self.previous));
    }
}

/// Makes `tag` the current tag on this thread until the guard drops.
pub fn scope(tag: Tag) -> ScopeGuard {
    let previous = CURRENT.with(|c| c.replace(tag as u8));
    ScopeGuard { previous }
}

/// The tag currently active on this thread.
pub fn current() -> Tag {
    Tag::from_index(CURRENT.with(Cell::get))
}

/// Sets the live-bytes budget for `tag` (`usize::MAX` = unlimited).
pub fn set_budget(tag: Tag, bytes: usize) {
    let c = &COUNTERS[tag as usize];
    c.budget.store(bytes, Relaxed);
    c.exceeded.store(false, Relaxed);
}

/// Strict mode aborts the process on the first budget overrun. Defaults to
/// on in debug builds and off in release; the harness turns it on.
pub fn set_strict(strict: bool) {
    STRICT.store(strict, Relaxed);
}

/// Resets every peak to the current live value and clears overrun flags,
/// so the next measurement window starts fresh.
pub fn reset_peaks() {
    for c in COUNTERS.iter().chain(std::iter::once(&TOTAL)) {
        c.peak.store(c.live.load(Relaxed), Relaxed);
        c.exceeded.store(false, Relaxed);
    }
}

/// Stats of one tag at snapshot time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TagStats {
    /// Bytes currently allocated under this tag.
    pub live: usize,
    /// High-water mark of `live` since start or the last [`reset_peaks`].
    pub peak: usize,
    /// Allocations (including reallocations) so far.
    pub allocs: u64,
    /// Frees (including reallocations) so far.
    pub frees: u64,
    /// Configured budget (`usize::MAX` = unlimited).
    pub budget: usize,
    /// Whether `live` ever exceeded `budget`.
    pub exceeded: bool,
}

impl TagStats {
    fn read(c: &Counters) -> TagStats {
        TagStats {
            live: c.live.load(Relaxed),
            peak: c.peak.load(Relaxed),
            allocs: c.allocs.load(Relaxed),
            frees: c.frees.load(Relaxed),
            budget: c.budget.load(Relaxed),
            exceeded: c.exceeded.load(Relaxed),
        }
    }
}

/// A consistent-enough point-in-time copy of every counter.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    tags: [TagStats; Tag::COUNT],
    total: TagStats,
}

impl Snapshot {
    /// Stats for one tag.
    pub fn get(&self, tag: Tag) -> TagStats {
        self.tags[tag as usize]
    }

    /// Live bytes under `tag`.
    pub fn live(&self, tag: Tag) -> usize {
        self.tags[tag as usize].live
    }

    /// Peak live bytes under `tag`.
    pub fn peak(&self, tag: Tag) -> usize {
        self.tags[tag as usize].peak
    }

    /// Stats summed over every tag.
    pub fn total(&self) -> TagStats {
        self.total
    }

    /// Tags whose budget was exceeded.
    pub fn exceeded(&self) -> impl Iterator<Item = Tag> + '_ {
        Tag::ALL
            .into_iter()
            .filter(|t| self.tags[*t as usize].exceeded)
    }

    /// `(tag, stats)` pairs in tag order.
    pub fn iter(&self) -> impl Iterator<Item = (Tag, TagStats)> + '_ {
        Tag::ALL.into_iter().map(|t| (t, self.tags[t as usize]))
    }

    /// One line per tag with a non-zero peak, plus the total: the format
    /// `lean-measure` prints.
    pub fn report(&self) -> String {
        let mut out = String::new();
        for (tag, s) in self.iter() {
            if s.peak == 0 && s.allocs == 0 {
                continue;
            }
            out.push_str(&format!(
                "{:<12} live {:>10} peak {:>10} allocs {:>8} frees {:>8}{}\n",
                tag.name(),
                s.live,
                s.peak,
                s.allocs,
                s.frees,
                if s.exceeded { "  OVER BUDGET" } else { "" }
            ));
        }
        out.push_str(&format!(
            "{:<12} live {:>10} peak {:>10} allocs {:>8} frees {:>8}\n",
            "total", self.total.live, self.total.peak, self.total.allocs, self.total.frees
        ));
        out
    }
}

/// Reads every counter.
pub fn snapshot() -> Snapshot {
    let mut tags = [TagStats::default(); Tag::COUNT];
    for (i, c) in COUNTERS.iter().enumerate() {
        tags[i] = TagStats::read(c);
    }
    Snapshot {
        tags,
        total: TagStats::read(&TOTAL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[global_allocator]
    static ALLOC: LeanAlloc = LeanAlloc;

    #[test]
    fn attributes_allocations_to_the_current_tag() {
        let before = snapshot().live(Tag::Image);
        let v: Vec<u8>;
        {
            let _g = scope(Tag::Image);
            assert_eq!(current(), Tag::Image);
            v = vec![7u8; 100_000];
        }
        assert_eq!(current(), Tag::Untagged);
        let during = snapshot();
        assert!(during.live(Tag::Image) >= before + 100_000);
        assert!(during.peak(Tag::Image) >= before + 100_000);
        // Freed outside the scope: still attributed to Image via the header.
        drop(v);
        let after = snapshot();
        assert_eq!(after.live(Tag::Image), before);
        assert!(after.peak(Tag::Image) >= before + 100_000);
        assert!(after.get(Tag::Image).frees >= 1);
    }

    #[test]
    fn realloc_keeps_the_tag_and_counts() {
        let base = snapshot().live(Tag::Ipc);
        let mut v: Vec<u64> = Vec::new();
        {
            let _g = scope(Tag::Ipc);
            v.reserve_exact(16);
        }
        // Grow outside the scope: realloc must still charge Ipc.
        let allocs_before = snapshot().get(Tag::Ipc).allocs;
        v.reserve_exact(4096);
        let s = snapshot();
        assert!(s.live(Tag::Ipc) >= base + 4096 * 8);
        assert!(s.get(Tag::Ipc).allocs > allocs_before);
        drop(v);
        assert_eq!(snapshot().live(Tag::Ipc), base);
    }

    #[test]
    fn nested_scopes_restore() {
        let _a = scope(Tag::Layout);
        {
            let _b = scope(Tag::Text);
            assert_eq!(current(), Tag::Text);
        }
        assert_eq!(current(), Tag::Layout);
    }

    #[test]
    fn budgets_record_overruns_without_aborting_when_lenient() {
        set_strict(false);
        set_budget(Tag::Harness, 1024);
        let _g = scope(Tag::Harness);
        let big = vec![0u8; 4096];
        let s = snapshot();
        assert!(s.get(Tag::Harness).exceeded);
        assert!(s.exceeded().any(|t| t == Tag::Harness));
        assert!(s.report().contains("OVER BUDGET"));
        drop(big);
        set_budget(Tag::Harness, usize::MAX);
        assert!(!snapshot().get(Tag::Harness).exceeded);
    }

    #[test]
    fn big_alignment_is_honoured() {
        let layout = Layout::from_size_align(100, 4096).unwrap();
        // SAFETY: valid non-zero layout; freed below with the same layout.
        let p = unsafe { std::alloc::alloc(layout) };
        assert!(!p.is_null());
        assert_eq!(p as usize % 4096, 0);
        unsafe { std::alloc::dealloc(p, layout) };
    }

    #[test]
    fn totals_track_everything() {
        // Other test threads allocate and free concurrently, so only
        // monotonic counters are compared exactly.
        let t0 = snapshot().total();
        let v = vec![1u32; 10_000];
        let t1 = snapshot().total();
        assert!(t1.peak >= 40_000);
        assert!(t1.allocs > t0.allocs);
        drop(v);
        reset_peaks();
        let t2 = snapshot().total();
        assert_eq!(t2.peak, t2.live);
    }
}
