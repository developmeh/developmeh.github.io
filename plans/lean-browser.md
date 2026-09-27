# Lean Browser: implementation plan

## 1. Summary and non-goals

Lean Browser is a limited web browser whose single optimizing target is **minimal resident private memory** in the long-lived process, measured as `Private_Dirty` from `/proc/<pid>/smaps_rollup`. Everything else (page load latency, CPU, disk) is spent freely to buy RAM.

The design splits the work in two: a **short-lived loader** that does everything untrusted and everything allocation-heavy (fetch, HTML parse, CSS cascade, optional JS run-to-idle, DOM compression) and then exits, and a **long-lived renderer** that memory-maps a compact, pre-validated, read-only **page file** and paints the viewport in immediate mode through a small strip buffer. Read-only file-backed mmap pages are `Private_Clean`/`Shared_Clean`, never `Private_Dirty`, so the size of the page file barely affects the target metric; only what the renderer *allocates* does.

**Non-goals** (explicitly, for the whole plan horizon):

- Web compatibility with app-like sites (SPAs, WebGL, WebRTC, service workers, WebAssembly).
- Full CSS. We implement a subset chosen for *consistent* rather than *complete* layout.
- Fast loads or smooth 60 fps scrolling. Scroll may repaint from scratch and re-layout the viewport.
- Persistent profiles, history, caches, or any per-origin storage (rejected on fingerprinting grounds).
- Speculative DOM pruning with undo-on-access (rejected as unsound).
- Non-Linux platforms before M4. The sandbox is Linux-only; the renderer should compile elsewhere but is not measured there.

## 2. Architecture and process model

```
+---------------------------------------------------------------------------+
| renderer (long-lived, trusted, small)                                     |
|  winit/softbuffer window   <- strip buffer (<=64 rows, RGBA8)              |
|  mmap(page.lpg)  --rkyv check once-->  &ArchivedPage (read-only, clean)   |
|  viewport layout (own block/inline + taffy flex/grid), heights[] per      |
|  top-level block, glyph cache (KB-capped), swash on mmap'd font           |
|  images: decode-at-paint from blob in page file, bounded scratch          |
|  accesskit tree built on demand from node role/name                        |
+------------------------+--------------------------------------------------+
                         | pipes: length-prefixed postcard messages (§2.2)
                         v
+---------------------------------------------------------------------------+
| loader (per navigation; exits)                                            |
|   broker (parent): HTTP(S) via ureq+rustls, session cookie jar,           |
|                    seccomp: sockets allowed, no exec/ptrace/open(rw)      |
|      | pipe: fetch requests / raw bytes                                   |
|   worker (child, seccomp strict: read/write/mmap/brk/exit only):          |
|     html5ever -> DOM -> lightningcss cascade -> [QuickJS run-to-idle]     |
|     -> freeze -> compression passes -> rkyv serialize -> page.lpg         |
|     -> also writes sources.bundle (raw HTML/CSS/images) for re-cascade    |
+---------------------------------------------------------------------------+
        session dir (tmpfs or disk, wiped on exit): page.lpg, sources.bundle,
        cookies.jar (session-only), form-state (renderer-owned)
```

### 2.1 Process model

- The renderer is started once. For every navigation it spawns `lean-loader`, waits, receives a `Done` message with the page file path, `munmap`s the old page, `mmap`s the new one, validates, and repaints. The loader's exit returns all parse/cascade/JS memory to the OS.
- The loader internally forks into a **broker** (network) and a **worker** (parsing). Only these two ever see untrusted bytes. The worker requests subresources (CSS, images) from the broker over a pipe.
- The renderer never allocates in proportion to page size except for the `heights[]` array (4 bytes per top-level block) and a small form-state vector.

### 2.2 Loader<->renderer protocol

Transport: the loader's stdin/stdout, framed as `u32 length + postcard bytes` (postcard is tiny and `no_std`; rkyv is reserved for the page file). All messages carry `protocol_version: u16`. Enums:

Renderer -> loader:
- `Load { url, method: Get|Post, body: Option<Vec<u8>>, content_type, referrer: Option<url>, viewport: { width_css_px, height_css_px, dpr_x100: u16 }, session_dir, js: Off|RunOnce, fonts: FontManifest }`
- `Recascade { sources_bundle_path, viewport }` — re-run cascade+compression from the stored raw sources without touching the network (used on breakpoint crossings; see §2.3).
- `Cancel`

Loader -> renderer:
- `Progress { phase: Fetching|Parsing|Cascading|Scripting|Compressing|Writing, bytes }`
- `Done { page_path, sources_bundle_path, final_url, title, breakpoints: Vec<u16>, forms: u16 }`
- `Error { kind: Dns|Tls|Http(status)|Timeout|TooLarge|Sandbox, message }`

Navigation events originate in the renderer: a click on a node with `flags.LINK` resolves `href` from the attribute table and sends `Load`. A submit collects fields from the page file's form table, overlays user-typed values from the renderer's small `form_state: Vec<(node_idx, String)>` (the only mutable per-page state), url-encodes or multipart-encodes, and sends `Load{Post}`. Fragment navigation (`#id`) is handled in the renderer from the attribute table without a loader round trip.

### 2.3 Viewport-dependent styles: decision

The page file is **cascaded for one viewport-width bucket**. The loader records every `width`/`min-width`/`max-width` media-query breakpoint it encountered (in CSS px) in `Done.breakpoints` and in the page header. The renderer:

- On resize within the same bucket: re-layout only (renderer-side; cheap, no loader).
- On crossing a breakpoint: debounce 300 ms after the resize ends, then send `Recascade` against the sources bundle. No network; the loader worker re-parses and rewrites the page file. The renderer keeps painting the old page until `Done`.

Lengths in the style table are stored as `(unit, value)` where unit is one of `Px, Percent, Vw, Vh, Auto` — `em`/`rem`/`pt` are resolved to px in the loader (font-size is per-node computed), while `%`, `vw`, `vh` remain symbolic so that intra-bucket resizes need no re-cascade. `prefers-color-scheme` is fixed to `light`; `hover`/`pointer` media queries evaluate as `hover: none`, `pointer: coarse` (deterministic, low-fingerprint).

## 3. Cargo workspace layout

```
lean-browser/
  Cargo.toml            (workspace, resolver = "2", profile.release: opt-level="s", lto="fat", panic="abort", codegen-units=1, strip=true)
  crates/
    page-format/        rkyv structs, header/versioning, validation, reader API
    lean-alloc/         tagging global allocator with per-component budgets
    loader/             bin: lean-loader (broker + worker, sandbox)
    css-subset/         shared cascade data model (property enums, style table) used by loader (write) and renderer (read)
    renderer/           bin: lean-browser (window, layout, paint, a11y, IPC client)
    harness/            bin: lean-measure, lean-diff; corpus manifests; CI entry point
  corpus/               20 frozen pages served locally (see §9)
  refs/                 Chromium reference PNGs, pinned version in refs/VERSION
  plans/lean-browser.md
```

Per-crate dependencies (all pure Rust unless noted; each is justified by "what would we otherwise have to write?"):

**page-format**: `rkyv` 0.8 (+`bytecheck`) — zero-copy archive with validation, the whole point of the format. `memmap2` — map the file read-only. No other deps.

**lean-alloc**: none. Wraps `std::alloc::System` (musl mallocng in the static build) and counts bytes per thread-local component tag; exposes stats for the harness. Optional `dlmalloc` feature for comparison.

**css-subset**: `serde` is *not* used; types are shared as rkyv archives. `bitflags` for node flags. Nothing else.

**loader**: `html5ever` + `markup5ever_rcdom` (HTML5 parsing; we then convert RcDom to our own arena and drop it). `lightningcss` (parsing stylesheets, media queries, `@import`, `@layer`, nested rules, and property parsing; we use its parsed values, not its printer). `cssparser` (transitively; direct use for `style=""` attributes and `calc`). `selectors` (the Servo selector matching engine — writing a correct `:nth-child`/attribute matcher is error-prone). `ureq` with `rustls`/`webpki-roots` (HTTP/1.1 client, no async runtime, small). `url`. `flate2`/`brotli` (content-encoding). `rquickjs` (QuickJS bindings; only in the `js` feature, M5). `seccompiler` (build seccomp-BPF filters in Rust, rust-vmm maintained). `rustix` (fork/pipe/landlock without libc footguns). `postcard` (IPC). `rkyv`.

**renderer**: `winit` + `softbuffer` (window and CPU presentation; softbuffer uses wl_shm/X11 SHM buffers which are `Shared_Dirty`, not `Private_Dirty`). `tiny-skia` (rasterizer; supports clipping, rounded rects, A8 masks). `swash` (shaping and glyph rasterization straight from font bytes; no font database kept in memory). `memmap2` (fonts, page file). `taffy` (flex and grid only; block and inline formatting are ours because taffy has neither floats nor line boxes). `unicode-linebreak` (UAX #14 tables, ~30 KB static, needed for correct wrapping). `png` and `jpeg-decoder` (streaming row decoders; `jpeg-decoder` exposes DCT downscale via `Decoder::scale`, which is why it is chosen over `zune-jpeg`). `accesskit` + `accesskit_winit` (platform a11y; zero cost until an AT connects). `postcard`, `rustix`. Explicitly **not** in the renderer: parley, cosmic-text, fontdb, lightningcss, resvg, image.

**harness**: `procfs`-free hand parser of `smaps_rollup` (five lines of code, avoids a dep). `image` (PNG load for diffing; harness memory is irrelevant). Own SSIM implementation (~80 lines) to avoid `opencv`. `serde_json` for the CI report. `tiny_http` to serve the corpus locally.

## 4. Page file format

Magic `b"LEANPG\0\0"`, then a fixed 32-byte header outside the rkyv archive (so version checks happen before touching rkyv):

```rust
#[repr(C)]
pub struct Header {
    pub magic: [u8; 8],
    pub format_version: u16,   // bump on any incompatible change
    pub min_reader_version: u16,
    pub flags: u32,            // bit0 = JS ran, bit1 = truncated (limits hit)
    pub archive_len: u64,
    pub archive_crc32: u32,    // cheap integrity check before rkyv check
    pub reserved: [u8; 4],
}
```

Archive root (rkyv 0.8, `#[derive(Archive, Serialize)]`, `#[rkyv(check_bytes)]`, all offsets little-endian):

```rust
#[derive(Archive, Serialize)]
pub struct Page {
    pub final_url: String,
    pub title: String,
    pub viewport_width: u16,          // cascade bucket
    pub breakpoints: Vec<u16>,        // sorted, css px
    pub nodes: Vec<Node>,             // pre-order, index 0 = root
    pub text: Vec<u8>,                // UTF-8 blob; runs are NOT NUL-terminated
    pub styles: Vec<ComputedStyle>,   // interned; node.style indexes here
    pub attrs: Vec<Attr>,             // sorted by node; binary search
    pub images: Vec<ImageRef>,
    pub blobs: Vec<u8>,               // compressed image bytes and inline SVG, concatenated
    pub forms: Vec<Form>,
    pub top_level: Vec<u32>,          // node indices of body's block children (scrollbar units)
}

#[derive(Archive, Serialize)]
pub struct Node {
    pub style: u16,        // index into styles; 0xFFFF = "not rendered" (never present in frozen pages)
    pub kind: u8,          // Element(tag id) | Text | Image | Svg | FormControl | LineBreak | Marker
    pub flags: u8,         // LINK, ANCHOR_TARGET, FOCUSABLE, BLOCK, INLINE_REPLACED, PRE, HAS_ATTRS, IS_FORM_FIELD
    pub text_off: u32,     // Text: run start; Element: unused
    pub text_len: u32,
    pub parent: u32,       // u32::MAX for root
    pub first_child: u32,  // u32::MAX if none
    pub next_sibling: u32,
    pub role: u8,          // ARIA-ish role enum (Heading1..6, Link, Button, TextField, Img, List, ListItem, Paragraph, Generic, ...)
    pub name_off: u32,     // accessible name in text blob (may alias text_off)
    pub name_len: u16,
    pub _pad: u16,
}
// sizeof(ArchivedNode) = 32 bytes.

#[derive(Archive, Serialize)]
pub struct Attr { pub node: u32, pub key: AttrKey /*u8*/, pub val_off: u32, pub val_len: u32 }
// Only kept keys: Href, Src, Alt, Id, Name, Value, Type, Action, Method, Placeholder, Checked, For, Lang, Title, AriaLabel, Colspan, Rowspan.

#[derive(Archive, Serialize)]
pub struct ImageRef { pub node: u32, pub blob_off: u32, pub blob_len: u32, pub width: u16, pub height: u16, pub format: u8 /* Jpeg|Png|Svg|Unsupported */ }

#[derive(Archive, Serialize)]
pub struct Form { pub node: u32, pub action_off: u32, pub action_len: u32, pub method: u8, pub first_field: u32, pub field_count: u32 }
```

`ComputedStyle` (in `css-subset`) is a fixed-size struct of enums and `Length {unit: u8, value: f32}` fields, ~112 bytes archived; interning keeps the table to a few hundred entries.

**Validation** (once, at map time): (1) header magic/version/`min_reader_version`; (2) CRC32 of the archive region; (3) `rkyv::access::<ArchivedPage, rkyv::rancor::Error>` structural check; (4) a single linear semantic pass: every `parent/first_child/next_sibling` is `< nodes.len()` or `MAX`, pre-order invariant (`first_child == i+1` when present), every offset+len within its blob, every `style < styles.len()`, `attrs` sorted. Any failure rejects the page and shows an error page (never a partial render). After validation the renderer indexes the archive without further checks.

**Size estimates** (labelled estimates): per node 32 B + amortized attrs (~2 B) + text (~20 B for text nodes); a compressed blog post ~1,200 nodes -> ~40 KB nodes + 30 KB text + 25 KB styles ~ 100 KB plus image blobs; a news front page ~15,000 nodes -> ~700 KB plus blobs. All of this is file-backed and clean in the renderer.

## 5. Supported CSS subset

**Selectors**: type, universal, class, id, attribute (`[a]`, `=`, `~=`, `|=`, `^=`, `$=`, `*=`, case flag), descendant, child, `+`, `~`, `:root`, `:first-child`, `:last-child`, `:only-child`, `:nth-child(an+b)`, `:nth-of-type`, `:not(<compound>)`, `:is()/:where()`, `:link`, `:empty`, `::before`/`::after` with string `content` only. Unsupported and treated as never-matching: `:hover`, `:focus`, `:active`, `:visited` (privacy), `:has()`, `::marker`, `::selection`, `:target`.

**At-rules**: `@media` (width/min/max-width, `screen`, `all`, `prefers-color-scheme` fixed light, `hover`/`pointer` fixed), `@import` (depth <= 3, same fetch limits), `@layer` (flattened in declaration order via lightningcss), `@supports` (evaluated against our property list), `@font-face` ignored, `@keyframes` ignored.

**Properties**: `display` (block, inline, inline-block, flex, inline-flex, grid, none, list-item, table*), `position` (static, relative, absolute, fixed, sticky), `top/right/bottom/left`, `float` (left/right, simple), `clear`, `width/height/min-*/max-*`, `margin-*` (incl. `auto` for centering), `padding-*`, `border-*-width/style/color` (solid/none/dashed-as-solid), `border-radius`, `box-sizing`, `overflow` (visible, hidden), `color`, `background-color`, `opacity`, `visibility`, `font-family` (generic family mapping to bundled Noto Sans/Serif/Mono), `font-size`, `font-weight` (bucketed to 400/700), `font-style`, `line-height`, `text-align`, `text-decoration`, `text-transform`, `text-indent`, `white-space` (normal, nowrap, pre, pre-wrap), `word-break: break-all`, `vertical-align` (baseline, middle, top, bottom), `list-style-type` (disc, circle, square, decimal, none), `z-index` (within a stacking context only), `gap`, all flex properties, `grid-template-columns/rows`, `grid-column/row`, `grid-area` (M4), `var()` custom properties (own substitution pass before lightningcss property parsing), `calc()` of px/%/em/vw. Units: px, em, rem, %, vw, vh, pt, ch (approximated as 0.5em).

**Unsupported, with fallbacks**:

| Feature | Fallback |
|---|---|
| web fonts | generic family; letter widths differ, accepted |
| `background-image`, gradients | `background-color` only; if none, transparent |
| `transform`, `filter`, `mix-blend-mode`, `box-shadow`, `text-shadow` | ignored |
| transitions/animations | final computed values, no motion |
| `position: fixed` | treated as `absolute` against the initial containing block (scrolls away) |
| `position: sticky` | treated as `relative` |
| `overflow: scroll/auto` | `hidden` with clipping; no inner scrollbars |
| table layout | M4: auto layout emulated via grid with `colspan` -> `grid-column: span n`; before M4 tables become block/inline-block |
| `columns` | single column |
| `writing-mode`, bidi, RTL | LTR horizontal; RTL text rendered LTR-ordered (documented defect) |
| shadow DOM, `<slot>` | light DOM flattened, slot content rendered in place |
| `<iframe>` | placeholder box with a link to the frame URL |
| `<video>/<audio>/<canvas>/<object>` | placeholder box with `role=Generic`, poster not loaded |
| GIF, WebP, AVIF | placeholder box of intrinsic size (dimensions read from header) |
| inline SVG | M3: placeholder; M4: subset (rect, circle, path, viewBox, fill/stroke) via tiny-skia paths |
| `:hover` UI | none; links show underline from stylesheet only |

## 6. Compression passes (loader, after cascade, frozen pages only)

Run in this order; each pass records counts in the harness report.

1. **Drop never-rendered nodes**: comments, doctype, `head`, `meta`, `link`, `title` (after extracting title), `script`, `style`, `template`, `noscript` when JS ran (kept and rendered when JS is off), and any subtree with computed `display: none`. Safety: `display: none` subtrees are dropped only in document mode; in interactive mode (M6) they are retained with `style = hidden` because JS may toggle them.
2. **Whitespace-only text**: drop a text node consisting solely of ASCII/Unicode whitespace when its parent's `white-space` is `normal`/`nowrap` *and* both adjacent siblings (or the parent edge) are block-level boxes. Never drop inside `pre`/`pre-wrap` or between two inline boxes (it is a rendered space).
3. **Collapse transparent wrappers**: remove an element E and reparent its children when all hold: E is `display: block` or `inline` (not flex/grid/inline-block/list-item/table); E's computed style has zero margin, padding, border, no background, `opacity: 1`, `visibility: visible`, `overflow: visible`, `position: static`, no `float`/`clear`, `width/height/min/max: auto`; E's parent is not a flex or grid container (E might be an item whose removal changes item count); E has no `id`, no kept attribute, no `role`/`aria-*`, is not a link/form/form field/anchor target, and is not a heading or landmark for a11y; and E's children's *inherited* computed values are unchanged by removal (guaranteed by comparing interned style ids of children before and after, since inheritance is already resolved into computed styles). Wrappers with a single text child are also merged with pass 4.
4. **Merge adjacent inline text runs** with identical style id and identical parent, no intervening element; runs are concatenated in the text blob. Never across a `<br>`, an anchor boundary, or `white-space` change.
5. **Intern computed styles**: hash the full `ComputedStyle` struct; assign `u16` ids in first-use order. If more than 65,000 distinct styles occur (pathological), the loader quantizes lengths to 0.5 px and retries; if still over, sets `flags.truncated` and maps overflow to the nearest existing style by property distance.
6. **Deduplicate inline SVG** (and identical `data:` images): content-hash the serialized markup; second and later occurrences reference the same blob range.
7. **Attribute trimming**: keep only `AttrKey` keys; `class` and `style` are gone after cascade. `href` values are resolved to absolute URLs.

Every pass is a pure function on the arena with a debug-mode assertion that re-running the cascade on the compressed tree reproduces identical style ids (a "compression is style-preserving" property test).

## 7. Renderer memory design

Build: `x86_64-unknown-linux-musl`, static-pie, `panic = "abort"`, single thread (no rayon, no tokio); winit's event loop on the main thread only. Static musl removes the dynamic loader's dirty pages and glibc's per-thread arenas; mallocng returns freed pages with `madvise`/`munmap` aggressively, and we call `malloc_trim` equivalents (musl: freed large chunks are unmapped) after each paint.

| Component | Target (estimate) | Enforcement |
|---|---|---|
| binary `.data`/`.bss`/relocations | <= 200 KB | static-pie, `opt-level="s"`, LTO, no `regex`/`serde_json` in renderer; measured in M0 |
| main-thread stack touched | <= 128 KB | iterative traversals; stack size set to 512 KB, deep recursion forbidden by lint (`clippy::recursion` custom check in harness) |
| allocator overhead | <= 64 KB | mallocng single arena; `lean-alloc` reports live bytes per tag |
| page file mapping | 0 dirty | `MAP_PRIVATE`, `PROT_READ`; never written |
| font files | 0 dirty | mmap'd, read-only; 3 bundled Noto faces |
| swash shape/scale contexts | <= 64 KB | one `ShapeContext` and one `ScaleContext` created at start, reused |
| glyph cache (A8 masks) | 256 KB hard cap | LRU by bytes; eviction on insert; keyed by (face, size*64, glyph id) |
| strip buffer | <= 320 KB | `rows = min(64, 320 KB / (width * 4))`; single allocation reused for the process lifetime |
| viewport layout tree (block/inline + taffy) | <= 256 KB transient | built per paint for top-level blocks intersecting the viewport, dropped after paint; `shrink_to_fit` unnecessary because it is freed |
| `heights[]` per top-level block | 4 B x N, cap 64 KB | if N > 16k, adjacent blocks are grouped into scrollbar units |
| image decode scratch | 512 KB transient cap | `jpeg-decoder` with `scale` to nearest 1/8 >= display size, row-by-row box filter into a display-size buffer; PNG via streaming rows; images whose display-size buffer exceeds the cap draw as placeholders |
| form state, IPC framing | <= 16 KB | bounded `Vec`; IPC buffer 8 KB reused |
| winit + softbuffer + wayland client state | <= 400 KB (biggest unknown) | measured in M0; wl_shm buffer itself is `Shared_Dirty` and reported separately |
| accesskit tree | 0 until AT connects; then <= 2x nodes x 48 B | built lazily in `ActivationHandler`; viewport-only nodes plus landmarks |
| **Total steady state, blog page** | **<= 2.0 MB Private_Dirty** | CI gate (§8) |
| **Total steady state, news front page** | **<= 4.0 MB** | CI gate |

Painting: for each strip, clear, walk the viewport layout tree, draw boxes/text/images whose bounds intersect the strip, blit the strip into the softbuffer surface. Single-buffered surface (softbuffer with one wl_shm buffer; tearing accepted). RGB565 is **rejected**: tiny-skia rasterizes RGBA8 premultiplied only and softbuffer wants 0RGB u32; a 565 strip would need a conversion pass for zero `Private_Dirty` gain since the window buffer is shared memory. Revisit only if a DRM/fbdev backend is added.

The `lean-alloc` allocator tags allocations with a thread-local component id; in debug and harness builds exceeding a component budget panics with the tag, and in release it logs once. The harness reads the per-tag high-water marks via a `--stats` IPC command over a debug socket.

## 8. Milestones

Effort is for one experienced Rust engineer; all numbers are targets, not measurements.

**M0 – Harness and skeleton (2 weeks).** Workspace, `lean-alloc`, a renderer that opens a window and paints a solid color through the strip path, a loader stub that writes an empty page file, `lean-measure` (spawns the renderer headless with `--paint-png`, samples `smaps_rollup` 10x over 5 s after settle, reports max/median `Private_Dirty`, `Shared_Dirty`, RSS), `lean-diff` (SSIM + pixel mismatch %), GitHub Actions job with xvfb/weston headless and a JSON report artifact plus a Markdown summary comment. Corpus captured (§9) and Chromium refs generated with a pinned Playwright Chromium. Exit: empty renderer `Private_Dirty` measured and recorded as baseline; **target <= 1.0 MB**; if winit alone exceeds 800 KB, open the R1 experiment before M1.

**M1 – Page format and plain HTML (3 weeks).** html5ever -> arena -> page file with UA stylesheet only; validation; renderer does block/inline layout for text, headings, lists, links, `<br>`, `<pre>`; scrolling with `heights[]`; swash text with glyph cache. Exit: 5 hand-written HTML-only corpus pages, SSIM >= 0.70 against Chromium with matching fonts; renderer <= 1.5 MB.

**M2 – CSS subset and compression (4 weeks).** lightningcss cascade, selectors, media queries, `var()`, interning, all seven compression passes with the style-preservation property test; renderer supports the §5 property list minus flex/grid/tables; images decoded at paint. Exit: 10 static pages incl. the developmeh.com blog (Zola-generated, syntax-highlighted code blocks) at SSIM >= 0.85 median; renderer <= 2.0 MB median; compression reduces node count by >= 40% on the corpus median.

**M3 – Document mode end-to-end (4 weeks).** Full IPC, broker/worker split with seccomp and Landlock, session cookie jar, link navigation, forms (GET/POST, text/checkbox/radio/select/submit), fragment navigation, resize with breakpoint re-cascade, error pages, `noscript` rendering, keyboard link navigation and find-in-page. Exit: all 20 corpus pages load without loader crash under the sandbox; median SSIM >= 0.80, minimum >= 0.60; renderer `Private_Dirty` **median <= 2.5 MB, max <= 4.0 MB**; loader peak RSS reported (informational, <= 150 MB).

**M4 – Layout completeness and accessibility (3 weeks).** taffy flex and grid, table emulation, floats, inline SVG subset, accesskit exposure, heading/landmark navigation. Exit: median SSIM >= 0.85 across the full corpus; a11y: AT-SPI tree visible in `accerciser` for every corpus page; memory gates unchanged.

**M5 – JS run-to-idle in the loader (4 weeks).** rquickjs with `JS_SetMemoryLimit(64 MB)` and a 5 s CPU budget; curated DOM surface (`document.querySelector*`, `getElementById`, `createElement`, `appendChild/remove`, `textContent`, `innerHTML` via html5ever fragment parsing, `classList`, `get/setAttribute`, `style.<prop>`, `addEventListener` (recorded, never fired), `setTimeout` drained until idle, `fetch` denied, `localStorage` in-memory). Scripts throwing on unsupported APIs are aborted individually; the page snapshot continues. Exit: 5 JS-enhanced corpus pages (e.g. a page with client-side rendered table of contents) reach SSIM >= 0.80; renderer memory unchanged since JS never enters it.

**M6 – Interactive mode (6+ weeks, later).** Renderer keeps the loader worker alive with a capped QuickJS heap (`JS_SetMemoryLimit(16 MB)`), the DOM stays mutable, and compression pass 1 keeps `display: none`; events are forwarded over IPC and the page file is rewritten incrementally. Not in scope for numeric gates yet.

**M7 – Hardening (ongoing).** `cargo fuzz` targets for the page-file validator and the loader's HTML/CSS input, memory regression gate at +5% over the recorded baseline per page.

## 9. Test and measurement strategy

**Corpus selection criteria** (20 pages, frozen as a local static snapshot so results are reproducible; served by the harness on `127.0.0.1`):
- 5 hand-built pages exercising specific features (block/inline, flex, grid, forms, images), including a full build of the developmeh.com Zola blog (index, a post with code blocks and images, tag page).
- 8 real-world static documents: documentation sites, a Wikipedia article, a long-form news article, a README-style page, an RFC as HTML.
- 4 "heavy" static pages: a news front page (>10k nodes), a product listing, a page with 50+ images, a page with 500+ distinct computed styles.
- 3 JS-dependent pages (for M5).
- Exclusion: pages requiring login, pages with anti-bot gating, pages whose snapshots exceed 5 MB.

**Measurement**: `lean-measure` launches the renderer with `--headless --paint-png out.png --viewport 1280x800 --dpr 1`, waits for `Done`, scrolls to three positions (top, 50%, bottom) painting each, samples `smaps_rollup` 10 times over 5 s after the last paint, and records max and median `Private_Dirty`, `Shared_Dirty`, `Rss`, plus `lean-alloc` per-tag peaks. Loader peak RSS is captured via `wait4` rusage. CI publishes a table per page and fails if any gate in §8 regresses by more than 5%.

**Visual diff**: Chromium refs at the same viewport, DPR 1, same bundled Noto fonts installed via `fontconfig` in the reference container, `--disable-gpu --hide-scrollbars`, JS disabled for document-mode refs. Metrics: grayscale SSIM (8x8 windows) and mismatched-pixel % at a 12/255 tolerance. Thresholds per milestone as above. A `refs/VERSION` pin plus a `make refs` job; refs are committed.

**Unit/property tests**: compression style-preservation (§6), page-file round trip, validator rejects mutated files (bit-flip fuzz), IPC message round trips, layout unit tests against hand-computed boxes, seccomp tests that the worker is killed on `socket(2)`.

## 10. Risks and open questions

- **R1: winit/wayland baseline dirty memory dominates.** Experiment in M0: measure winit+softbuffer empty window versus a minimal `wayland-client` (Rust backend) hand-rolled `xdg_shell` surface versus `x11rb`. If winit exceeds 800 KB, adopt the hand-rolled Wayland path behind a feature flag.
- **R2: Viewport-only layout with absolute/fixed positioning against far ancestors.** Resolution: containing blocks are always within a top-level block or the ICB; ICB-relative boxes are placed from `heights[]`. `fixed` degrades to `absolute` (documented). Experiment: count how many corpus pages rely on fixed headers; if >30%, add a "pinned band" that paints the first fixed element at the top permanently.
- **R3: Re-layout of top-level blocks on every scroll costs CPU on huge blocks** (e.g. one `<div>` wrapping the whole page after wrapper collapse fails). Mitigation: pass 3 has a "split" rule — a block child of body larger than 64 rows in the estimated height can be treated as a set of scrollbar units using its own block children. Open: measure worst-case layout time; accept up to 200 ms per scroll step.
- **R4: Glyph cache thrash on text-heavy pages** is CPU-only by design; measure paint time, accept up to 100 ms per frame.
- **R5: Progressive JPEG cannot be row-streamed** by `jpeg-decoder` without a full-image buffer. Resolution: images with a display-size buffer under the 512 KB scratch cap decode fully; otherwise the loader transcodes progressive JPEGs to baseline (loader memory is free) at fetch time.
- **R6: Style table overflow (>65k)** handled by quantization (§6.5); verify on the heavy corpus page.
- **R7: JS DOM surface is a bottomless pit.** Scope is fixed by the M5 list; a page-level allowlist is not attempted. Measure the fraction of corpus scripts that complete without throwing and report it, but do not gate on it.
- **R8: rkyv validation walking the whole file** touches every page once; they are clean and evictable, so this costs I/O, not `Private_Dirty`. Verify in M1 by measuring before/after validation.
- **Open: session cookies.** Decision proposed: in-memory session jar in the broker, wiped on exit, no persistence; document the trade-off (logins do not survive restart).
- **Open: DPR > 1.** Strip buffer cost scales with device pixels; plan is to keep the 320 KB cap and shrink rows (a 2560-wide DPR 2 window gets 32 rows).

## 11. Security model

- **Trust boundary**: the broker (network bytes, TLS via rustls) and the worker (HTML/CSS/JS/image bytes) are untrusted-input handlers; the renderer trusts only a page file that passed validation, and never touches the network or untrusted files.
- **Worker sandbox** (applied before reading any input): `seccompiler` allowlist of `read, write, pread64, mmap, munmap, mprotect(no PROT_EXEC), brk, madvise, futex, clock_gettime, exit_group, close, fstat, lseek, sigaltstack, rt_sigreturn`; everything else `SECCOMP_RET_KILL_PROCESS`. Landlock restricts the filesystem to the session directory (read/write) and the font directory (read). `PR_SET_NO_NEW_PRIVS`, `PR_SET_DUMPABLE=0`. A resource limit (`RLIMIT_AS` 1 GB, CPU 30 s) bounds pathological pages. Subresource fetches go through the broker pipe with a size cap (20 MB per resource, 100 MB per page) and a count cap (200).
- **Broker sandbox**: seccomp allowlist adds `socket, connect, getsockopt, setsockopt, poll, sendto, recvfrom, shutdown, getrandom`; no `execve`, `ptrace`, `openat` beyond the session dir (Landlock). Only `http`/`https` schemes; no redirects to `file:` or loopback addresses; no HTTP/2 (smaller code). Cookies are `SameSite=Lax` semantics, session-only.
- **Renderer**: no network capability at all (seccomp denies `socket`); it can `execve` only `lean-loader` (checked path) and open only the session dir and fonts.
- **Page file validation** is a hard gate; a file that fails validation is deleted and an error page shown. The validator and the parser are fuzz targets (M7).
- **Fingerprinting**: no persistent state, fixed media query answers, bundled fonts only, no `:visited`, no per-origin cache, no JS access to timing or screen size beyond the viewport bucket.
- **Memory safety**: `#![forbid(unsafe_code)]` everywhere except `page-format` (mmap) and `lean-alloc`, reviewed as a unit. QuickJS is C; it runs only inside the worker sandbox.

## 12. Accessibility plan

- Every node carries `role` and an accessible name from day one (M1), computed in the loader from tag, `role=`, `aria-label`, `alt`, `<label for>`, heading level, landmark tags (`nav`, `main`, `header`, `footer`, `aside`), and list/table structure. Names live in the text blob and usually alias the node's own text (zero extra bytes).
- Compression pass 3 never collapses a node with a non-generic role, a name, or a landmark, so the a11y structure survives compression.
- M3 ships keyboard access without an AT: Tab/Shift-Tab across `FOCUSABLE` nodes with a visible focus ring, Enter to activate links and submit forms, `/` find-in-page, `h`/`H` heading navigation, `Ctrl+L` URL entry, `Ctrl+=/-` page zoom (re-layout only, re-cascade if it crosses a breakpoint).
- M4 exposes the tree through `accesskit_winit`. The tree is built lazily in the activation handler; to bound memory it includes landmarks, headings, and all nodes intersecting the viewport plus a "scroll to reveal" affordance, and it is rebuilt on scroll. Live regions and focus events are forwarded.
- Low-vision support: page zoom, a high-contrast toggle that overrides `color`/`background-color` at paint time (no re-cascade), and minimum font size.
- Reader assistance: because the page file has clean roles, a "reader mode" that renders only `main`/article content is a trivial filter and is included in M4.

## Critical files for implementation

- `crates/page-format/src/lib.rs` — the rkyv structs, header, and validator (the contract between loader and renderer).
- `crates/loader/src/compress.rs` — the seven compression passes and their safety rules.
- `crates/loader/src/sandbox.rs` — broker/worker split, seccomp and Landlock policies.
- `crates/renderer/src/layout/mod.rs` — block/inline formatting, `heights[]`, viewport-only layout with taffy for flex/grid.
- `crates/renderer/src/paint/strip.rs` — strip buffer painting, glyph cache, decode-at-paint images.
- `crates/harness/src/measure.rs` — `smaps_rollup` sampling and the CI gates.
