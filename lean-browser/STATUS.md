# Status

Tracks what each stage of the M0–M2 build has done and what it deliberately
left for later. Nothing is skipped silently: if it is not here and not in
the code, it has not been considered.

## Stage: project skeleton (this stage)

### Done

- **Workspace** (`Cargo.toml`): six crates per plan §3, resolver 2, release
  profile `opt-level="s"`, `lto="fat"`, `panic="abort"`,
  `codegen-units=1`, `strip=true`. Every third-party dependency for M0–M5
  is in `[workspace.dependencies]`; `Cargo.lock` is committed and settled.
  `cargo build --workspace`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo fmt --check` all pass. `cargo check -p renderer --features window`
  and `cargo check -p loader --features js` also compile.
- **page-format** (plan §4), fully implemented: 32-byte header with magic,
  versions, flags, `archive_len`, CRC-32; rkyv 0.8 `Page`/`Node`/`Attr`/
  `ImageRef`/`Form` with bytecheck; `encode`/`write_to_path` (tmp + rename);
  `PageFile::open` (read-only private mmap) and `PageFile::from_bytes`; the
  four validation steps (header, CRC, `rkyv::access`, linear semantic pass
  over nodes, styles, attrs, images, forms, fields, top-level blocks,
  breakpoints, UTF-8 and char-boundary checks on the text blob).
  `const` assertion that `size_of::<ArchivedNode>() == 32`. Tests: round
  trip, mmap of a written file, header rejection, *every* single-bit flip
  in the file is rejected (except the unchecked `flags`/`reserved` bytes and
  a downward flip of `min_reader_version`), a CRC-bypassing corruption sweep
  that must never panic, and one test per semantic invariant.
- **lean-alloc**, fully implemented: `GlobalAlloc` wrapper around `System`
  with a thread-local component tag (`scope(Tag::…)` guard), exact per-tag
  live/peak/alloc/free counters (the tag is stored in a per-block header so
  frees are attributed correctly whatever tag is current), a global total,
  per-tag budgets with a recorded overrun flag, strict mode (abort with the
  tag name on stderr; default on in debug builds) and a text report. Tests
  cover attribution, realloc, nested scopes, budgets, large alignments.
- **css-subset** data model: `ComputedStyle` (fixed-size; every §5 property
  the renderer needs for M2, plus flex and grid enums/placements), `Length`
  with `Px/Percent/Vw/Vh/Auto`, `Rgba`, `TextDecoration` bitflags,
  `GridTrack`/`TrackListRef` (track lists live in `Page.tracks`),
  `NodeFlags`, and `StyleTable` (first-use-order `u16` interning with the
  65 000 cap and a `quantize_lengths` helper for the overflow fallback).
  Fieldless enums are archived as themselves (`#[rkyv(as = Self)]`), so the
  renderer reads the same enum types the loader writes.
- **Binaries** (skeleton level): `lean-loader --write-empty` writes a valid
  empty page; `lean-browser --headless --page … --paint-png … --viewport WxH
  [--stats]` maps + validates a page and paints a solid white viewport
  through a reused strip buffer (`rows = min(64, 320 KB / (width*4))`)
  with `lean-alloc` installed and reporting; `lean-measure --self|--pid`
  prints `smaps_rollup` figures; `lean-diff a.png b.png` prints the
  mismatched-pixel percentage at the 12/255 tolerance.
- **harness** lib: `smaps_rollup` parser with tests.
- **Nix**: flake-parts module project (`flake.nix` → `nix/flake-module.nix`
  → `toolchain.nix`, `packages.nix`, `devshell.nix`, `checks.nix`,
  `formatter.nix`), crane build from `Cargo.lock`, rust-overlay toolchain,
  `flakeModules.default` exported with its own locked inputs, systems
  `x86_64-linux` and `aarch64-linux`.
- `README.md`, `.gitignore`, `rust-toolchain.toml`, `rustfmt.toml`,
  `clippy.toml`.

### Deviations from the plan (deliberate)

- **`ArchivedNode` is 32 bytes** by shrinking the plan's `_pad: u16` to a
  single `reserved: u8` and moving `role`/`name_len` next to `kind`/`flags`
  so no alignment padding is inserted (plan erratum: the listed fields sum
  to 33).
- **`ArchivedComputedStyle` is 344 bytes**, not the plan's ~112: an
  archived `Length` is 8 bytes (`f32` alignment) rather than 5. The table is
  file-backed and clean, so this does not touch `Private_Dirty`; packing
  lengths into 4 bytes (fixed point) is a possible later optimisation.
- **`Page` has two extra tables**: `tracks` (grid track lists referenced by
  `ComputedStyle::grid_template_*`, keeping the style struct fixed-size) and
  `fields` (node indices of form fields; `Form.first_field/field_count`
  index into it).
- **`parcel_selectors` replaces `selectors`** and `cssparser` is pinned to
  lightningcss's 0.37 (see README "Dependencies").
- `ureq` is declared with `rustls`, `gzip` and `brotli` features (ureq 3
  bundles content-decoding); `flate2`/`brotli` are still declared for the
  loader's own use of raw streams.
- `taffy` is 0.14 (plan does not pin), `png` 0.18, `tiny-skia` 0.12,
  `winit` 0.30 (0.31 is still beta) with `accesskit_winit` 0.34.

### Deferred / not done in this stage

- **`flake.lock` is not generated** (no Nix in the build environment) and
  the Nix files could not be evaluated. First `nix build` creates the lock;
  review the modules then. Known uncertainties: `pkgs.nixfmt-rfc-style`
  (an alias in recent nixpkgs), and crane's `cleanSourceWith` filter keeping
  `corpus/`, `refs/`, `fonts/`, `syntaxes/`.
- **No static musl / static-pie package** (plan §7) in Nix yet.
- **No GitHub Actions job** for lean-browser: `.github/` is outside the
  `lean-browser/` directory this work is confined to. The `checks` in
  `nix/checks.nix` are the intended CI entry point.
- **`corpus/` and `refs/`** are empty placeholders; corpus capture and
  Chromium reference generation (§9) are M0 harness work.
- **lean-measure**: renderer launch, three scroll positions, 10 samples over
  5 s, JSON report and gates (§9) not yet implemented; only the sampler is.
- **lean-diff**: SSIM (8×8 grayscale windows) not yet implemented; only the
  pixel-mismatch metric is.
- **lean-alloc `--stats` IPC over a debug socket** (§7): stats are currently
  printed at exit with `--stats`; the socket comes with the IPC work.
- **Renderer**: no layout, text, images, scrolling, IPC client, or window.
  The `window` feature compiles its dependencies but `window_mode()` returns
  an error; the winit/softbuffer window and the R1 baseline measurement
  are M0 work.
- **Loader**: no fetch, HTML parse, cascade, compression, sandbox or IPC;
  only `--write-empty`.
- **css-subset**: no cascade logic (that is the loader's, on top of
  lightningcss); `inherit_from` covers the default-inherited properties
  only. Flex and grid enums exist but their layout is out of M2 scope.
- **IPC message types** (§2.2, postcard) not defined yet; `serde` and
  `postcard` are declared for loader, renderer and harness.
- **Seccomp/Landlock sandbox** (§11): `seccompiler` and `rustix` are
  declared; nothing implemented (M3).
- **JS** (`rquickjs`, M5): declared behind the loader's `js` feature only.

## Stage: renderer, M0–M2 (`crates/renderer`)

Built concurrently with the loader stage; only `crates/renderer/` and this
file were touched. Page files for the tests come from `page-format`'s
writer through `renderer::testing::PageBuilder` (also behind
`lean-browser --write-demo`), so nothing here depends on the loader.

### Done

- **Document / `heights[]`** (`document.rs`): `PageFile` map + validate,
  then one `f32` per scrollbar unit (`tops[]`, the unit's border-box top,
  plus an end sentinel; 4 B/unit, grouped when a page has more than 16 k
  units). Units are `Page.top_level`, else the body's block children, else
  the body itself. The `html`/`body` chain contributes width, auto
  margins, padding/border and collapsed margins; the canvas background is
  `html`'s, else `body`'s, else white. `layout_viewport(scroll)` lays out
  only the units intersecting the viewport into a transient tree.
- **Block layout** (`layout/block.rs`): `width`/`height`/`min-*`/`max-*`
  with `box-sizing`, `%`/`vw`/`vh`, auto-margin centring, margin
  collapsing (siblings, first/last child through parents without
  padding/border, BFC roots excluded), `overflow: hidden` clipping,
  `position: relative`, `absolute` (and `fixed` as absolute, plan §5)
  against the nearest positioned ancestor or the unit (plan R2), replaced
  elements with intrinsic ratio, `display: list-item` with outside markers
  (disc/circle/square/decimal), `inline-block` shrink-to-fit from
  min/max-content measurement, `table*` as block/inline-block and
  flex/grid containers as block with blockified children (pre-M4 fallbacks
  from plan §5). Recursion is bounded by `MAX_DEPTH = 96`.
- **Inline layout** (`layout/inline.rs`): all five `white-space` values
  (tabs to 8-column stops in `pre`), `text-transform`, `text-indent`,
  `text-align` (`justify` = left), `text-decoration` propagated to
  descendants, `word-break: break-all`, `<br>`, `<wbr>`, inline box
  fragments with background/border/padding per line, `vertical-align`
  (baseline, middle, top, bottom), the container strut, hanging trailing
  spaces, `unicode-linebreak` (UAX #14) opportunities, greedy first-fit
  with overflow when a word never fits.
- **Text** (`text.rs`, `fonts.rs`): one `swash` `ShapeContext` and one
  `ScaleContext` for the process, per-character face fallback across all
  mapped faces, hinted A8 masks rasterized at device size, LRU-by-bytes
  glyph cache capped at 256 KB (plan §7). Fonts are discovered without
  fontconfig (`--font`, `--font-dir`/`LEAN_FONT_DIR`, then system dirs;
  see `crates/renderer/README.md`) and memory-mapped read-only.
- **Painting** (`paint/mod.rs`): the single strip allocation
  (`rows = min(64, 320 KB / (width*4))`), band loop, iterative tree walk
  with rect clips and multiplicative opacity, backgrounds, per-side solid
  borders (dashed/dotted as solid), `border-radius` (rounded background;
  rounded ring border when uniform), underline/overline/line-through,
  `visibility: hidden`, placeholders for unsupported replaced content and
  form controls, and headless `--paint-png` streaming each band into the
  PNG encoder. `--dpr` scales layout-to-device.
- **Images** (`paint/image.rs`): PNG decoded row by row (`EXPAND`,
  `STRIP_16`, `ALPHA`) and box-filtered into a display-size premultiplied
  buffer (≤ 512 KB, else placeholder); JPEG via `jpeg-decoder::scale`
  (nearest 1/8 ≥ display size) then box-filtered (L8, RGB, CMYK). One
  decoded image is kept across bands within a paint so a tall image is
  not re-decoded per band.
- **Window mode** (`window.rs`, feature `window`): winit 0.30 +
  softbuffer 0.4, wheel/keyboard scrolling, resize → re-layout, left
  click on a link prints its `href` to stdout, `q`/Escape quit. It
  compiles under `clippy -D warnings` but could not be run (no display).
- **CLI**: `lean-browser page.lpg --headless --paint-png out.png
  --viewport 1280x800 [--scroll-to 0.5 | --scroll <px>] [--dpr n]
  [--font f | --font-dir d] [--list-fonts] [--dump-boxes] [--stats]
  [--write-demo p]`.
- **Tests**: 21 unit tests (fonts, glyph cache eviction, shaping and
  rasterization with system fonts, white-space processing, image
  resampling, mask blending), 11 layout tests against hand-computed boxes
  (margins/padding/borders/auto centring, collapse-through vs BFC,
  inline-block alignment, absolute/relative/fixed, unit selection when
  scrolled, replaced ratio under `max-width`, wrapping + `text-align`,
  `nowrap`/`pre`/`<br>`, list markers, link hit-testing, top-aligned
  atomics) and 5 golden-ish paint tests (pixel-exact borders/fills,
  scrolling, DPR 2 across two bands, seamless band stitching, decoded PNG
  and placeholder pixels, glyph ink). Text tests skip themselves when the
  machine has no font.

### Deviations from the plan (deliberate)

- **One `unsafe` in the renderer** (`fonts::map_readonly`, `memmap2`), the
  same read-only private mapping `page-format` uses. Plan §11 lists only
  `page-format` and `lean-alloc`; the clean fix is a font-mapping helper
  in `page-format`, which this stage could not edit.
- **JPEG is not row-streamed**: `jpeg-decoder` has no row API, so the
  DCT-scaled image is decoded whole (≤ 2 MB transient) and box-filtered.
  PNG *is* streamed. Images whose display buffer exceeds 512 KB draw as
  placeholders as planned.
- **`tops[]` instead of `heights[]`**: same 4 B/unit; storing top edges
  makes unit selection a binary search.
- **No bundled Noto**: fonts are found on the machine (see README);
  Chromium comparisons need `LEAN_FONT_DIR` pointing at the same Noto
  files. Glyphs are placed at integer device pixels (no subpixel
  positioning), which will cost some SSIM against Chromium.
- **`renderer::testing`** (page builder + demo page) is compiled into the
  library (`#[doc(hidden)]`) so integration tests and `--write-demo` share
  it.

### Deferred / approximated (renderer)

- Floats and `clear` are laid out in flow (plan puts floats in M4 even
  though §5 lists `float` "simple").
- Flex, grid (`taffy` is declared but unused), table layout with
  `colspan`: M4.
- `z-index`/stacking contexts: paint order is tree order. `opacity`
  multiplies primitive alpha rather than compositing a group.
- Rect clips only: a rounded background under an `overflow: hidden`
  ancestor is not clipped to it.
- Margin collapse-through of empty (zero-height) blocks is not modelled;
  negative margins use the CSS 2.1 sum rule only at sibling boundaries.
- Absolute boxes: static position approximated by the flow cursor; the
  unit is the initial containing block (plan R2), so `fixed` scrolls
  away. `inline-block` baselines are their bottom margin edge (CSS uses
  the last line box).
- Percent heights against an auto-height containing block resolve to
  auto; `text-align: justify` renders as left; RTL/bidi is LTR-ordered
  (documented defect in §5).
- Form controls draw as outlined boxes; `<img>` without an image record,
  inline SVG, iframes and media draw as grey placeholders (M3/M4).
- No IPC client, loader spawning, fragment navigation, keyboard focus
  ring, find-in-page (M3); no accesskit tree (M4, deps compile only).
- `tops[]` is recomputed by laying every unit out once at open and on
  every resize (CPU only, plan-accepted); no incremental relayout.
- Headless `--paint-png` allocates the PNG encoder's zlib buffers
  (untagged, ~380 KB peak); they do not exist in window mode.
- `malloc_trim`/musl static-pie packaging and a `LEAN_FONT_DIR` for the
  Nix package are build/Nix work not touched here.
