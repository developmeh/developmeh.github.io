//! Font discovery and memory mapping (plan §7: fonts are mmap'd, read-only,
//! zero dirty pages; no font database is kept in memory).
//!
//! No fonts are bundled in git. A face is found, in order of preference:
//!
//! 1. `--font <file>`: one TrueType/OpenType file used for every family and
//!    variant (the fastest way to get deterministic output in CI);
//! 2. `--font-dir <dir>` or the `LEAN_FONT_DIR` environment variable: a
//!    directory searched recursively for the well-known file names below;
//! 3. the common system directories (`/usr/share/fonts`,
//!    `/usr/local/share/fonts`, `~/.local/share/fonts`, `~/.fonts`,
//!    `/run/current-system/sw/share/X11/fonts`, `$XDG_DATA_DIRS/*/fonts`).
//!
//! The search is fontconfig-free: it matches file names, not font metadata,
//! against a fixed list per generic family (Noto, DejaVu, Liberation,
//! FreeFont, ...). A missing variant falls back to the family's regular
//! face, a missing family to sans, and with nothing found at all layout
//! still runs (text measures as zero and paints nothing) so block-only
//! pages and tests work anywhere.

use std::path::{Path, PathBuf};

use css_subset::{FontFamily, FontStyle, FontWeight};
use memmap2::Mmap;
use swash::FontRef;

/// Index of a face in a [`FontSet`].
pub type FaceId = u8;

/// Which variant of a family (index into the per-family table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Variant {
    /// Normal weight, upright.
    Regular = 0,
    /// Bold, upright.
    Bold = 1,
    /// Normal weight, italic/oblique.
    Italic = 2,
    /// Bold italic/oblique.
    BoldItalic = 3,
}

impl Variant {
    /// All variants in table order.
    pub const ALL: [Variant; 4] = [
        Variant::Regular,
        Variant::Bold,
        Variant::Italic,
        Variant::BoldItalic,
    ];

    fn of(weight: FontWeight, style: FontStyle) -> Variant {
        match (weight, style) {
            (FontWeight::Normal, FontStyle::Normal) => Variant::Regular,
            (FontWeight::Bold, FontStyle::Normal) => Variant::Bold,
            (FontWeight::Normal, FontStyle::Italic) => Variant::Italic,
            (FontWeight::Bold, FontStyle::Italic) => Variant::BoldItalic,
        }
    }
}

/// Well-known file stems (lower-case, no extension) per family and variant,
/// in order of preference. Noto matches the Chromium references (plan §9).
const CANDIDATES: [[&[&str]; 4]; 3] = [
    // Sans
    [
        &[
            "notosans-regular",
            "dejavusans",
            "liberationsans-regular",
            "freesans",
            "arimo-regular",
            "roboto-regular",
            "opensans-regular",
            "cantarell-regular",
            "arial",
        ],
        &[
            "notosans-bold",
            "dejavusans-bold",
            "liberationsans-bold",
            "freesansbold",
            "arimo-bold",
            "roboto-bold",
            "opensans-bold",
            "cantarell-bold",
            "arialbd",
        ],
        &[
            "notosans-italic",
            "dejavusans-oblique",
            "liberationsans-italic",
            "freesansoblique",
            "arimo-italic",
            "roboto-italic",
            "opensans-italic",
            "ariali",
        ],
        &[
            "notosans-bolditalic",
            "dejavusans-boldoblique",
            "liberationsans-bolditalic",
            "freesansboldoblique",
            "arimo-bolditalic",
            "roboto-bolditalic",
            "opensans-bolditalic",
            "arialbi",
        ],
    ],
    // Serif
    [
        &[
            "notoserif-regular",
            "dejavuserif",
            "liberationserif-regular",
            "freeserif",
            "tinos-regular",
            "times",
        ],
        &[
            "notoserif-bold",
            "dejavuserif-bold",
            "liberationserif-bold",
            "freeserifbold",
            "tinos-bold",
            "timesbd",
        ],
        &[
            "notoserif-italic",
            "dejavuserif-italic",
            "liberationserif-italic",
            "freeserifitalic",
            "tinos-italic",
            "timesi",
        ],
        &[
            "notoserif-bolditalic",
            "dejavuserif-bolditalic",
            "liberationserif-bolditalic",
            "freeserifbolditalic",
            "tinos-bolditalic",
            "timesbi",
        ],
    ],
    // Mono
    [
        &[
            "notosansmono-regular",
            "dejavusansmono",
            "liberationmono-regular",
            "freemono",
            "cousine-regular",
            "cour",
        ],
        &[
            "notosansmono-bold",
            "dejavusansmono-bold",
            "liberationmono-bold",
            "freemonobold",
            "cousine-bold",
            "courbd",
        ],
        &[
            "dejavusansmono-oblique",
            "liberationmono-italic",
            "freemonooblique",
            "cousine-italic",
            "couri",
        ],
        &[
            "dejavusansmono-boldoblique",
            "liberationmono-bolditalic",
            "freemonoboldoblique",
            "cousine-bolditalic",
            "courbi",
        ],
    ],
];

/// Where to look for fonts.
#[derive(Clone, Debug, Default)]
pub enum FontSource {
    /// One file for everything.
    File(PathBuf),
    /// One directory, searched recursively.
    Dir(PathBuf),
    /// `LEAN_FONT_DIR`, then the system directories.
    #[default]
    Auto,
}

/// One memory-mapped font file.
struct Face {
    path: PathBuf,
    map: Mmap,
}

/// The faces the renderer can draw with, mapped read-only.
#[derive(Default)]
pub struct FontSet {
    faces: Vec<Face>,
    /// `table[family][variant]` = face index, if any.
    table: [[Option<FaceId>; 4]; 3],
    searched: Vec<PathBuf>,
}

impl FontSet {
    /// A set with no faces: text measures as zero and is not painted.
    pub fn empty() -> FontSet {
        FontSet::default()
    }

    /// Whether any face is available.
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// Number of mapped faces.
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// Finds and maps fonts per the module documentation.
    pub fn load(source: &FontSource) -> Result<FontSet, String> {
        let mut set = FontSet::empty();
        match source {
            FontSource::File(path) => {
                let id = set.map_face(path)?;
                for family in &mut set.table {
                    for slot in family.iter_mut() {
                        *slot = Some(id);
                    }
                }
            }
            FontSource::Dir(dir) => {
                set.searched.push(dir.clone());
                let found = scan_dirs(&set.searched);
                set.pick(&found);
            }
            FontSource::Auto => {
                set.searched = default_dirs();
                let found = scan_dirs(&set.searched);
                set.pick(&found);
            }
        }
        Ok(set)
    }

    /// Picks the best candidate for every family/variant from the scan
    /// results, mapping each distinct file once.
    fn pick(&mut self, found: &[(String, PathBuf)]) {
        for (fam, table) in CANDIDATES.iter().enumerate() {
            for (var, names) in table.iter().enumerate() {
                let hit = names
                    .iter()
                    .find_map(|name| found.iter().find(|(stem, _)| stem == name));
                if let Some((_, path)) = hit {
                    if let Ok(id) = self.map_face_dedup(path) {
                        self.table[fam][var] = Some(id);
                    }
                }
            }
        }
    }

    fn map_face_dedup(&mut self, path: &Path) -> Result<FaceId, String> {
        if let Some(i) = self.faces.iter().position(|f| f.path == path) {
            return Ok(i as FaceId);
        }
        self.map_face(path)
    }

    fn map_face(&mut self, path: &Path) -> Result<FaceId, String> {
        if self.faces.len() >= 64 {
            return Err("too many font faces".into());
        }
        let map = map_readonly(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if FontRef::from_index(&map, 0).is_none() {
            return Err(format!("{}: not a usable font file", path.display()));
        }
        self.faces.push(Face {
            path: path.to_path_buf(),
            map,
        });
        Ok((self.faces.len() - 1) as FaceId)
    }

    /// The face to use for a computed font, with fallbacks (variant ->
    /// regular of the family -> sans -> any face). `None` only when the
    /// set is empty.
    pub fn face(&self, family: FontFamily, weight: FontWeight, style: FontStyle) -> Option<FaceId> {
        let fam = family as usize;
        let var = Variant::of(weight, style) as usize;
        self.table[fam][var]
            .or(self.table[fam][Variant::Regular as usize])
            .or(self.table[FontFamily::Sans as usize][var])
            .or(self.table[FontFamily::Sans as usize][0])
            .or_else(|| self.table.iter().flatten().flatten().next().copied())
            .or(if self.faces.is_empty() { None } else { Some(0) })
    }

    /// A `swash` view of a mapped face. Cheap: it parses only the table
    /// directory of the mapped bytes.
    pub fn font_ref(&self, id: FaceId) -> Option<FontRef<'_>> {
        self.faces
            .get(id as usize)
            .and_then(|f| FontRef::from_index(&f.map, 0))
    }

    /// Every face id, for coverage fallback.
    pub fn face_ids(&self) -> impl Iterator<Item = FaceId> {
        (0..self.faces.len() as u8).map(|i| i as FaceId)
    }

    /// Human-readable summary for `--list-fonts` and error messages.
    pub fn describe(&self) -> String {
        let mut s = String::new();
        if self.faces.is_empty() {
            s.push_str("no fonts found; searched:\n");
            for d in &self.searched {
                s.push_str(&format!("  {}\n", d.display()));
            }
            s.push_str("pass --font <file> or --font-dir <dir>, or set LEAN_FONT_DIR\n");
            return s;
        }
        let families = ["sans", "serif", "mono"];
        let variants = ["regular", "bold", "italic", "bold-italic"];
        for (fam, table) in self.table.iter().enumerate() {
            for (var, slot) in table.iter().enumerate() {
                let path = slot
                    .and_then(|id| self.faces.get(id as usize))
                    .map(|f| f.path.display().to_string())
                    .unwrap_or_else(|| "(fallback)".into());
                s.push_str(&format!("{:<5} {:<11} {path}\n", families[fam], variants[var]));
            }
        }
        s
    }
}

/// Maps a file read-only and private. This is the renderer's one use of
/// `unsafe` (plan §11 lists only `page-format` and `lean-alloc`; see
/// STATUS.md "Deviations"). Font files are never written while mapped, so
/// the only hazard `memmap2` documents (truncation by another process
/// raising `SIGBUS`) cannot arise in practice.
#[allow(unsafe_code)]
fn map_readonly(path: &Path) -> std::io::Result<Mmap> {
    let file = std::fs::File::open(path)?;
    // SAFETY: read-only private mapping of a font file that nothing in
    // this process writes; see the function documentation.
    unsafe { Mmap::map(&file) }
}

/// The directories searched in `Auto` mode, existing ones only.
fn default_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os("LEAN_FONT_DIR") {
        dirs.push(PathBuf::from(d));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
    }
    dirs.push(PathBuf::from("/usr/share/fonts"));
    dirs.push(PathBuf::from("/usr/local/share/fonts"));
    dirs.push(PathBuf::from("/run/current-system/sw/share/X11/fonts"));
    if let Some(xdg) = std::env::var_os("XDG_DATA_DIRS") {
        for d in std::env::split_paths(&xdg) {
            dirs.push(d.join("fonts"));
        }
    }
    dirs.retain(|d| d.is_dir());
    dirs
}

/// Recursively lists `.ttf`/`.otf` files under `dirs` as
/// `(lower-case stem, path)`, bounded in depth and count.
fn scan_dirs(dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    const MAX_DEPTH: usize = 6;
    const MAX_FILES: usize = 20_000;
    let mut out = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = dirs.iter().map(|d| (d.clone(), 0)).collect();
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth < MAX_DEPTH {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            let ext_ok = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("ttf") || e.eq_ignore_ascii_case("otf"));
            if !ext_ok {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                out.push((stem.to_ascii_lowercase(), path));
                if out.len() >= MAX_FILES {
                    return out;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set_has_no_face() {
        let set = FontSet::empty();
        assert!(set.is_empty());
        assert_eq!(
            set.face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal),
            None
        );
        assert!(set.describe().contains("no fonts found"));
    }

    #[test]
    fn variant_mapping() {
        assert_eq!(
            Variant::of(FontWeight::Bold, FontStyle::Italic),
            Variant::BoldItalic
        );
        assert_eq!(
            Variant::of(FontWeight::Normal, FontStyle::Normal),
            Variant::Regular
        );
    }

    #[test]
    fn auto_discovery_falls_back_gracefully() {
        // Whatever the machine has, loading must not fail and every
        // family/variant must resolve to *some* face when any face exists.
        let set = FontSet::load(&FontSource::Auto).unwrap();
        if set.is_empty() {
            eprintln!("no system fonts; skipping");
            return;
        }
        for fam in [FontFamily::Sans, FontFamily::Serif, FontFamily::Mono] {
            for w in [FontWeight::Normal, FontWeight::Bold] {
                for s in [FontStyle::Normal, FontStyle::Italic] {
                    let id = set.face(fam, w, s).unwrap();
                    assert!(set.font_ref(id).is_some());
                }
            }
        }
    }

    #[test]
    fn missing_dir_yields_empty_set() {
        let set = FontSet::load(&FontSource::Dir(PathBuf::from("/nonexistent/fonts"))).unwrap();
        assert!(set.is_empty());
    }
}
