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
- ~~`corpus/` and `refs/` are empty placeholders~~ — done in the
  integration stage below (8 pages, 24 Chromium references).
- ~~lean-measure / lean-diff skeletons~~ — done in the integration stage
  below.
- **lean-alloc `--stats` IPC over a debug socket** (§7): stats are currently
  printed at exit with `--stats`; the socket comes with the IPC work.
- **Renderer**: no layout, text, images, scrolling, IPC client, or window.
  The `window` feature compiles its dependencies but `window_mode()` returns
  an error; the winit/softbuffer window and the R1 baseline measurement
  are M0 work.
- **Loader**: fetch, parse, cascade, compression and serialization are
  done (see "Stage: loader (M1–M2)" below); sandbox and IPC are M3.
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
  plus an end sentinel; 4 B/unit). A unit is a run of `group` consecutive
  top-level blocks, `group = ceil(top_level / 16k)` (1 on every corpus
  page), so a page with more than 16 k top-level blocks keeps every block
  and `tops[]` stays ≤ 64 KB; no unit list is copied out of the mapped
  page (`Page.top_level` is read in place; only a page without
  `top_level` derives the body's block children, else the body itself). The `html`/`body` chain contributes width, auto
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
  from plan §5). Recursion is bounded by `MAX_DEPTH = 96` (a deviation,
  see below).
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
  glyph cache (plan §7's 256 KB component: 184 KB of masks in a map
  pre-sized for 448 entries that never reallocates, ~67 KB, under
  `Tag::GlyphCache` with a 256 KB `lean-alloc` budget; a mask larger than
  the cap is parked in one oversize slot until the next miss, so
  `bytes() ≤ cap` always holds). `font-size` is clamped to 512 CSS px in
  `Layouter::style` and glyphs above 1024 device px draw nothing, so a
  hostile page cannot make the rasterizer allocate unboundedly. Fonts are discovered without
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
- **Images** (`paint/image.rs`): non-interlaced PNG decoded row by row
  (`EXPAND`, `STRIP_16`, `ALPHA`) and box-filtered into a display-size
  premultiplied buffer (≤ 512 KB, else placeholder; every size product is
  checked in `u64` before any allocation, so a `2^31 px` `<img>` is a
  placeholder rather than an overflow); JPEG via `jpeg-decoder::scale`
  (nearest 1/8 ≥ display size) and interlaced PNG through a whole-image
  scratch (≤ 2 MB, see deviations), then box-filtered (L8, RGB, CMYK).
  One decoded image is kept across bands within a paint so a tall image
  is not re-decoded per band.
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
- **JPEG and interlaced PNG are not row-streamed**: `jpeg-decoder` has
  no row API and Adam7 needs every pass, so both decode whole into a
  scratch capped at `WHOLE_IMAGE_SCRATCH_CAP` = 2 MB (4x plan §7's 512 KB
  transient; a larger image is a placeholder). Non-interlaced PNG *is*
  streamed. Images whose display buffer exceeds 512 KB draw as
  placeholders as planned.
- **`lean-alloc` budgets** (`renderer::install_budgets`, called at
  `lean-browser` start): strip 320 KB and glyph cache 256 KB as planned;
  **layout 512 KB** instead of 256 KB because the blog page's viewport
  layout tree peaks at 290 KB (below), and **image 2.6 MB** (display
  buffer + whole-image scratch + codec state) instead of 512 KB for the
  reason above. Debug builds abort on an overrun (lean-alloc strict
  mode), release builds log once per tag. No corpus page trips a budget.
- **Recursive layout**: block/inline layout recurses (plan §7 asks for
  iterative traversals); depth is capped at `MAX_DEPTH = 96`, below which
  a deeper subtree is dropped from layout (not painted). The default 8 MB
  main-thread stack holds 96 frames easily; the plan's 128 KB stack
  target has not been tried.
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

## Stage: loader (M1–M2)

Scope: document mode, no JS, no seccomp/Landlock, no broker/worker split,
no IPC. Everything lives in `crates/loader` (lib + `lean-loader` bin);
no other crate and no workspace manifest was touched.

### Done

- **Fetch** (`fetch.rs`): `file://`, `data:` (base64/percent) and
  `http(s)://` via ureq 3 (rustls, gzip/brotli, redirects ≤ 10, 30 s
  timeout, non-2xx is an error). Plan §11 caps: 20 MB per resource,
  100 MB per page, 200 subresources. `url_from_arg` turns a path into a
  `file://` URL. Tests cover data URLs, file limits and a loopback HTTP
  server.
- **HTML** (`dom.rs`): html5ever 0.40 parses straight into an arena via
  an own `TreeSink` (scripting disabled, so `<noscript>` content is parsed
  and rendered per §6.1). `markup5ever_rcdom` is **not** used: the locked
  0.39 rcdom depends on html5ever 0.39 and cannot share types with 0.40,
  and the sink is ~150 lines. Template contents are detached fragments.
- **Stylesheets** (`css/rules.rs`): UA sheet (`css/ua.rs`, Chromium-like
  defaults: serif body font, 8px body margin, heading sizes, 40px list
  indent), `<style>`, `<link rel=stylesheet>` with `media=""`, `@import`
  (depth ≤ 3, media lists stacked), `@media` evaluated for the viewport
  bucket, `@layer` flattened in order, `@supports` (declarations checked
  against what we parse, `selector()` against the supported subset),
  `@font-face`/`@keyframes`/`@page`/`@container` ignored. Rules are
  bucketed by the rightmost id/class/tag for matching.
- **Media queries** (`css/media.rs`): width/height ranges (legacy
  `min-`/`max-`, range syntax, intervals), `screen`/`all` match, `print`
  does not; fixed answers `prefers-color-scheme: light`, `hover: none`,
  `pointer: coarse`, `prefers-reduced-motion: no-preference`, orientation
  from the viewport. Every width breakpoint is collected into
  `Page.breakpoints` (sorted, px; `em` at 16px).
- **Selectors** (`css/select.rs`): `parcel_selectors::Element` for arena
  nodes; the §5 subset is tested (type, universal, class, id, all attribute
  operators incl. the `i` flag, descendant/child/`+`/`~`, `:root`,
  `:first/last/only-child`, `:nth-child(an+b)`, `:nth-of-type`, `:not`,
  `:is`/`:where`, `:link`/`:any-link`, `:empty`, `::before`/`::after`).
  `:hover`/`:focus`/`:active`/`:visited`/`:target` never match;
  `:has()`, `&` nesting, `::slotted`, `::part`, `:host` are dropped before
  matching (parcel_selectors panics on them). `:lang()`, `:enabled`,
  `:disabled`, `:checked`, `:required`, `:optional` are evaluated from
  attributes (cheap and deterministic).
- **Cascade** (`css/cascade.rs`, `css/values.rs`): origin/importance
  levels UA < presentational hints < author < `style=""` < author
  `!important` < UA `!important`, then specificity, then order. Custom
  properties inherit and `var()` is substituted (with fallbacks, nested
  functions, cycle depth 16) by serializing the token list and re-parsing
  the declaration. CSS-wide keywords (`inherit`, `initial`, `unset`,
  `revert*`) work per property. `font-size` is computed first so `em`
  resolves; `rem`, `pt`, `ch` (0.5em), `ex`, `lh`, absolute units resolve
  to px; `%`, `vw`, `vh` stay symbolic; `calc()` of px/em/% is evaluated,
  and `min()`/`max()`/`clamp()` when all operands share a unit.
  `line-height: <number>` is carried as a factor so children recompute it
  against their own font size. `currentcolor` borders resolve after all
  declarations; `border-style: none` zeroes the width. Blockification for
  floats, absolute/fixed and flex/grid items. Presentational hints:
  `width`/`height` on replaced elements and tables, `align`, `bgcolor`,
  `<body text>`, `<font color>`, `nowrap`, `table[border]`.
  `::before`/`::after` with string (and `attr()`) content become `Pseudo`
  nodes styled against the originating element.
- **Freeze** (`freeze.rs`, `tree.rs`): text runs get the parent's
  inherited style plus propagated `text-decoration`; whitespace is
  collapsed per `white-space` (single spaces kept at run edges so the
  renderer still handles cross-run collapsing); list markers are
  generated as `Marker` nodes (`disc`/`circle`/`square`/`decimal` with
  `<ol start>`, `reversed`, `<li value>`); `<img>` is fetched and
  sniffed; inline `<svg>` becomes one `Svg` node whose blob is the
  serialized markup; forms and fields are recorded. Inline runs directly
  under `<body>` are wrapped in anonymous block boxes so every
  `top_level` entry is block-level (scrollbar units) — since the review
  fixes this runs from `compress` after passes 1-4, so whitespace between
  body blocks is dropped instead of wrapped (no empty units) and a
  collapsed body-level wrapper's inline children share one anonymous
  block (one line, as before the collapse).
- **Compression** (`compress.rs`), all seven passes of §6 with per-pass
  counts: (1) head/meta/link/title/script/style/template/base and
  `display: none` subtrees; (2) text runs made only of CSS collapsible
  whitespace (space, tab, LF, CR, FF; `&nbsp;` is kept) between block
  boxes under collapsing `white-space`; (3) transparent wrappers with the
  full safety list (no box properties, inherited set and custom-property
  scope identical to the parent's, no id/kept attribute/role/aria-*, not
  a link/form/field/anchor target/landmark/heading, no generated content,
  block wrappers only when their children are all blocks or their
  siblings are all blocks, inline wrappers only with
  inline content); (4) adjacent text runs with identical style; (5) style
  interning with the 65 000 cap, 0.5 px quantization retry and
  nearest-style fallback + truncated flag; (6) content-hash blob
  deduplication (inline SVG and identical images); (7) attribute trimming
  to `AttrKey` with `href`/`src`/`action` resolved against `<base>`.
- **Style-preservation property** (`check.rs`): after compression every
  live node is recomputed against its parent *in the compressed tree*
  (elements through `Cascade::compute`, pseudo-elements through their
  originating element, text/markers/anonymous boxes through their derived
  style) and must be identical. Runs in debug builds, in every test and
  with `--check`.
- **Accessibility** (`a11y.rs`): role from tag, `role=""` (with
  `aria-level`), `href`, input type, `alt=""` (presentation), landmark
  scoping (`header`/`footer` inside sectioning content are generic,
  `section` is a region only when named); name from `aria-label`,
  `aria-labelledby`, `alt`, `<label for>`/wrapping label, button
  text/value, placeholder, `<svg><title>`, content for links/headings/
  buttons/labels/captions/headers, `title`. Names are interned in the
  same blob as text, so a name equal to a node's text aliases it.
- **Serialization** (`serialize.rs`): pre-order numbering, one interned
  text blob, sorted attribute table, image/form/field tables,
  `top_level`. Every produced page passes `page_format::validate` in the
  tests, and the renderer's headless `--paint-png` maps and paints it.
- **CLI**: `lean-loader <url-or-path> --out page.lpg [--viewport WxH]
  [--stats] [--check]`; `--stats` prints `frozen nodes` and each pass's
  before/after counts plus page table sizes. `--write-empty` is kept.
- **Tests**: 52 unit tests across the modules and two fixture-driven
  integration tests (`tests/fixtures/blog/` – a Zola-like post with an
  imported/layered stylesheet, syntax-highlighted code, lists, images,
  table, grid; `tests/fixtures/forms.html`).

### Deviations from the plan (deliberate)

- `markup5ever_rcdom` unused (version mismatch with html5ever 0.40, see
  above); the dependency stays declared in the workspace.
- `calc()` mixing px and `%` cannot be stored in `Length {unit, value}`;
  the percentage part is kept (`calc(100% - 20px)` → `100%`). Mixed
  `min()/max()/clamp()` fall back to the preferred/zero value.
- `line-height: <number>` inherits as a factor (spec-correct) while
  `css-subset` stores the resolved px; the factor lives in the cascade
  context and in `FNode::ctx_key` for pass 3.
- Whitespace inside text runs is collapsed in the loader (smaller blob);
  a single space is kept at run edges so the renderer's line breaking
  still decides about cross-run collapsing.
- Text nodes carry `inherit_from(parent)` plus the accumulated
  `text-decoration` of their ancestors, not the parent's full style, so
  runs merge across collapsed wrappers.
- Pass 5 statistics report "node styles → interned styles"
  (before = live nodes) rather than distinct-before/after.
- `display: contents` and ruby fall back to `inline`; `sub`/`super`
  vertical alignment map to bottom/top; `font-weight` buckets at 600.

### Deferred / not done in this stage

- **Sandbox, broker/worker split, IPC** (`sandbox.rs`, postcard messages,
  `Progress`/`Done`/`Error`, `Recascade`, sources bundle): M3. The loader
  is a single process and writes only the page file.
- **Charset detection**: bytes are decoded as UTF-8 (BOM stripped);
  `<meta charset>`/HTTP charset other than UTF-8 are not honoured.
- **HTTPS trust store**: ureq uses webpki roots; there is no way to add
  a custom CA (`SSL_CERT_FILE`) yet. In this build environment the
  sandbox proxy refuses `CONNECT` to outside hosts (403), so live HTTPS
  loads could only be tested against a loopback HTTP server.
- **Hidden form fields**: `input[type=hidden]` is `display: none` in the
  UA sheet and is dropped by pass 1, so it is missing from
  `Page.fields`; M3's form submission must retain it (e.g. keep hidden
  inputs as unrendered field nodes).
- **List markers are emitted twice** when painted: the loader emits
  `Marker` nodes (with correct `<ol start>`/`value` ordinals) as the page
  format describes, and the renderer currently also draws its own
  bullets/numbers for `display: list-item`. One side must own markers;
  the loader's node is the one that knows the ordinal.
- **Nested style rules** (`.a { .b { } }`) inside a style rule's `rules`
  list are not matched (only top-level `&`-less nesting via
  `CssRule::Nesting`).
- **Presentational hints** beyond the list above (e.g. `<center>` is in
  the UA sheet, but `valign`, `cellpadding`, `<font size>` are not).
- **`content`** other than strings and `attr()` (counters, quotes,
  `url()`) produces no pseudo-element.
- **`::marker` styling** and `list-style-position: inside` are ignored;
  markers are always generated as the first child.
- **Corpus measurement**: the ≥ 40 % node-reduction gate (M2 exit) is a
  corpus median and needs the harness corpus; the blog fixture reaches
  ~30 % (179 → 126 nodes; pass 7 removes 60 % of attributes).
- **Table emulation** (M4): table display values are recorded, `colspan`/
  `rowspan` kept; no layout mapping.
- **`@import` cycles** are bounded only by depth (≤ 3), not detected.

## Stage: integration and measurement (harness, corpus, M0–M2 numbers)

Loader and renderer were built by separate agents; this stage reconciled
them, implemented the harness (plan §9), built the corpus, generated
Chromium references and measured. Everything below was produced on the
build machine (Linux 6.18, 4 CPUs, no display, glibc-dynamic release
build, `opt-level="s"`, `lto="fat"`), not on the plan's musl static-pie
target (not packaged yet).

### Reconciliation (edits outside the harness)

- **List markers were painted twice**: the loader emits a `Marker` node
  as the first child of every `display: list-item` (it knows `<ol start>`,
  `reversed`, `<li value>`), and the renderer synthesised its own. Now the
  renderer's `level()` skips `Marker` nodes in the inline flow and
  `add_marker` uses the marker node's text when one exists (outside
  position, aligned with the item's first line); it still synthesises a
  marker for builder-made pages without one (renderer tests).
- **Renderer `--serve`** (headless command loop on stdin: `paint <fraction>
  <out.png>`, `stats`, `quit`) so the harness can sample the process while
  it is alive after the last paint, with the page map, fonts, shaping
  contexts and strip buffer allocated once as a window would have them.
  Only the PNG encoder is per paint.
- **Renderer opts out of transparent huge pages**
  (`rustix::thread::disable_transparent_huge_pages`, `thread` feature added
  to the workspace `rustix` dependency). Before this, one measurement run
  reported 1.7–3.0 MB `Private_Dirty` on pages that measure 1.1–1.6 MB in
  every other run, with the figure changing while the process was idle; a
  2 MB anonymous folio backing the ~1 MB heap is the only mechanism that
  fits (the run did not yet record `AnonHugePages`, so this is inferred).
  The sampler now records `AnonHugePages` per sample and the report shows
  it (0 kB in every run since).
- **Loader** `--stats` prints `peak_rss_kb` (`VmHWM` of its own process):
  plan §9 wants `wait4` rusage, but rustix 1.x has no `getrusage` and
  `libc` is not in the dependency list; the harness also polls
  `/proc/<pid>/status` while the loader runs as an independent lower bound
  (they agree within 100 kB on every page).
- **Loader font families**: `family_by_name` tested vendor names in the
  sans list before the serif list, so `"DejaVu Serif"` resolved to sans.
  Serif is now checked first.
- **Renderer fixes found by the corpus** (each with a regression test in
  `crates/renderer/tests/layout.rs`): a `width: auto` block clamped by
  `max-width` now centres with `margin: 0 auto` (CSS 2.1 §10.4 re-runs the
  width rules with the clamped width; the page wrapper of every corpus
  page depends on it); a mandatory break at `</span>\n<span>` inside
  `pre` was claimed by both the text piece ending there and the next
  inline box, producing an empty line after every highlighted span line;
  `opacity` on an inline element now reaches its text runs (fragments and
  runs are flat siblings in the layout tree, so the paint walk could not
  propagate it); `overflow: auto`/`scroll` clip like `hidden` (no
  scrollbars are drawn).

### Harness (`crates/harness`)

- `lib.rs`: `smaps_rollup` parser (`Rss`, `Private_Dirty`, `Shared_Dirty`,
  `Private_Clean`, `AnonHugePages`).
- `diff.rs`: own grayscale SSIM (8×8 non-overlapping windows, Rec. 601
  luma over white, standard C1/C2) and mismatched-pixel % at 12/255 per
  channel; mismatched sizes compare the common area and flag it. Tests.
- `measure.rs`: `corpus/manifest.json`, loader run + stats parsing +
  RSS polling, renderer `--serve` driver (top / 50 % / bottom paints,
  N samples over the settle window, `lean-alloc` tags), per-paint diff
  against `refs/<name>-<pos>.png`. Tests for the parsers and summaries.
- `report.rs`: JSON (`serde_json`) and Markdown report, medians, the §8 M2
  gates (`Private_Dirty` median ≤ 2048 kB, every page's max ≤ its target,
  node-reduction median ≥ 40 %, SSIM median ≥ 0.85).
- `lean-measure --corpus <dir> [--out] [--refs] [--viewport] [--samples]
  [--settle-ms] [--font-dir] [--only] [--no-baseline] [--gate]`, plus the
  old `--self` / `--pid`. `lean-diff a.png b.png [--json] [--min-ssim]`.
- Not done: serving the corpus over `127.0.0.1` (`tiny_http` stays
  declared; pages are loaded as `file://`, which exercises the same loader
  path except the HTTP client), the `--stats` debug socket (the `stats`
  serve command replaces it for now), the +5 % regression gate against a
  recorded baseline (the gates are absolute; `report.json` is the
  baseline to diff against).

### Corpus (`corpus/`, 8 pages, `manifest.json`)

Hand-written: `block-inline` (borders, radius, centring, inline-block,
positioning, visibility/opacity, text-transform/indent), `lists` (nesting,
`start`/`value`/`reversed`, `list-style: none` + `::before`, `dl`),
`pre-code` (Zola-like highlight spans, tabs, long lines, `pre-wrap`,
`pre-line`, `kbd`), `images` (48×32 RGB and 32×32 RGBA PNG, 64×48
baseline JPEG, natural/scaled/percent sizes, missing image; assets
generated by `tools/gen-assets.py` and `tools/gen-jpeg.cjs`),
`media-queries` (width ranges incl. `em`, fixed answers, custom properties
overridden in a query, `@supports`, a `media="print"` sheet),
`wrapper-soup` (12-deep unstyled divs, 3-deep spans per word, a section
with a border every third level; `tools/gen-wrapper-soup.py`).
From this blog: `blog-testing-shell-scripts` (tech-dives, 5 code blocks)
and `blog-the-good-sergeant` (soft-wares, prose), rendered from
`content/*.md` by `tools/md2html.py` (stdlib markdown → the theme's
`main > article > #wrap` + byline + nav-after-article markup, Zola-style
`<pre data-lang style="…">` blocks with inline-style spans) against
`blog/main.css`, a hand-inlined approximation of the theme's SCSS. Two
deliberate departures from the live site, both because of M2 scope: the
theme's `main { display: flex }` two-column layout is laid out as blocks
(nav below the article; flex is M4) and `article { width: calc(100% -
23rem) }` is written as `margin-right: 20rem` because the loader keeps only
the `%` part of a mixed `%`/`px` `calc()` (loader deferred item; the real
site would render full-width until that is fixed). Fonts are pinned to
DejaVu in the corpus CSS so the renderer and Chromium use the same faces.
`content/` was not modified.

### Chromium references (`refs/`)

Node 22 and Playwright 1.56.1 with Chromium 141.0.7390.37 from
`/opt/pw-browsers` worked in the build environment, so `refs/` holds 24
PNGs (`<name>-{top,mid,bottom}.png`, 1280×800, DPR 1, JS disabled,
`--disable-gpu --hide-scrollbars --font-render-hinting=none`) generated by
`corpus/tools/make-refs.cjs`; `refs/VERSION` pins the versions.

### Results (release build, viewport 1280×800, 10 samples over 5 s after the last paint)

Two consecutive full runs agree to within 4 kB on every renderer figure.

| page | loader ms | loader peak RSS | nodes before → after | reduction | styles | page file | Private_Dirty median / max | RSS max | SSIM top / mid / bottom | mismatch % |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| empty (baseline) | 3 | – | – | – | – | 528 B | **704 / 704 kB** | 4716 kB | – | – |
| block-inline | 7 | 6180 kB | 107 → 80 | 25.2 % | 45 | 20.1 kB | 1272 / 1272 kB | 6564 kB | 0.859 / 0.889 / 0.891 | 9.2 / 6.8 / 6.5 |
| lists | 6 | 6120 kB | 142 → 90 | 36.6 % | 34 | 15.6 kB | 1192 / 1192 kB | 6268 kB | 0.969 / 0.969 / 0.966 | 2.6 / 2.9 / 2.3 |
| pre-code | 7 | 6096 kB | 71 → 53 | 25.4 % | 34 | 14.4 kB | 1228 / 1228 kB | 6284 kB | 0.975 / 0.930 / 0.971 | 2.0 / 3.7 / 2.1 |
| images | 6 | 6072 kB | 53 → 34 | 35.8 % | 21 | 11.1 kB | 1384 / 1384 kB | 6152 kB | 0.945 / 0.921 / 0.938 | 3.4 / 4.6 / 4.0 |
| media-queries | 6 | 6384 kB | 53 → 26 | 50.9 % | 19 | 8.1 kB | 1180 / 1180 kB | 6076 kB | 0.897 / 0.897 / 0.897 | 7.7 / 7.7 / 7.7 |
| wrapper-soup | 6 | 6256 kB | 262 → 42 | 84.0 % | 15 | 7.9 kB | 1200 / 1200 kB | 6072 kB | 0.876 / 0.897 / 0.879 | 7.6 / 7.1 / 7.1 |
| blog-testing-shell-scripts | 11 | 7452 kB | 582 → 484 | 16.8 % | 62 | 50.2 kB | 1620 / 1620 kB | 7100 kB | 0.702 / 0.932 / 0.962 | 13.3 / 7.5 / 2.0 |
| blog-the-good-sergeant | 7 | 6852 kB | 191 → 129 | 32.5 % | 40 | 25.4 kB | 1396 / 1396 kB | 6608 kB | 0.660 / 0.913 / 0.971 | 16.0 / 13.3 / 1.9 |

(Re-measured after the review fixes below. "Nodes before" is now the
frozen tree before any anonymous body wrapper is added, hence 2 fewer
than the previous run on every page; the empty trailing anonymous block
each page carried is gone. `Private_Dirty` is 30–50 kB higher per page
than before the fixes: the glyph map is now allocated once at its final
size, so its touched buckets are dirty from the first glyph instead of
growing on demand.)

`Shared_Dirty` and `AnonHugePages` were 0 kB on every sample (no window,
no THP).

| plan gate | measured | target | result |
|---|---:|---:|---|
| M0: empty renderer `Private_Dirty` | 704 kB | ≤ 1.0 MB | pass (no winit; the R1 window experiment is still open) |
| M1/M2: renderer `Private_Dirty` median over the corpus | 1272 kB | ≤ 2.0 MB (M2), ≤ 1.5 MB (M1) | pass |
| M2: blog page ≤ 2.0 MB | 1620 kB (worst page) | ≤ 2.0 MB | pass |
| M2: compression node reduction, corpus median | 35.9 % | ≥ 40 % | **fail** |
| M2: SSIM median vs Chromium | 0.92 (24 paints) | ≥ 0.85 | pass |
| M1: SSIM ≥ 0.70 on the hand-written pages | min 0.859 | ≥ 0.70 | pass |

Where the renderer's dirty memory goes (blog-testing-shell-scripts,
`/proc/<pid>/smaps`, ~1620 kB): ~1050 kB glibc `[heap]` (the layout
tree's peak of 290 kB — over the plan's 256 KB target, see the renderer
deviations —, the PNG encoder's ~380 kB zlib buffers and the shaping
scratch are freed but glibc keeps the arena; `lean-alloc` reports 1.19 MB
peak / far less live), 324 kB the strip buffer (its own mmap), 76 kB the
binary's `.data`/relocations (dynamic PIE), 68 kB stack, ~100 kB libc/ld
and TLS. The empty page's 704 kB is the same minus layout/text: strip
(320 kB) + untagged startup allocations (392 kB peak, mostly std and font
discovery) + binary/stack. In the lean-alloc table (`report.md`)
`glyph_cache` now covers the cached masks plus the pre-sized map (58 kB
empty, 88 kB peak on the blog page); `text` is shaping and rasterizer
scratch only (masks are rendered under `text` and copied into the cache
under `glyph_cache`). The plan's musl static-pie build with
mallocng returning freed chunks would attack the first and third items;
neither is done here.

Loader peak RSS is 6–7.5 MB on every page (plan informational cap
150 MB); loads take 6–30 ms.

### What works

- The full document-mode pipeline on all eight pages: `lean-loader` →
  page file → `lean-browser --serve` painting three scroll positions →
  smaps sampling → SSIM against Chromium, without a display.
- Every hand-written page scores SSIM ≥ 0.86 at every position; lists,
  pre/code and images ≥ 0.92. The two blog pages score 0.91–0.97 at mid
  and bottom.
- Compression: wrapper-soup 264 → 44 nodes (83 %), media-queries 49 %.

### What does not (and why the reduction gate fails)

- **Node reduction median 35.9 % < 40 %.** The hand-written pages are
  already lean markup: pass 3 (transparent wrappers) removes nothing on
  five of them and pass 2 (whitespace between blocks) does most of the
  work. The blog pages, the plan's stated target, reach only 17 % and
  32 %: Zola's syntax highlighting is one styled `<span>` per token, and
  every token span has a distinct colour, so it is neither a transparent
  wrapper nor mergeable with its neighbours. A pass that folded a styled
  span with a single text child into a text run carrying the span's style
  id (text runs already carry a style) would remove ~130 of the 486 nodes
  on the shell-scripts page; it is a loader change and is deferred.
- **SSIM 0.66–0.70 at the top of both blog pages.** Our layout matches
  Chromium's line breaks, but the byline / heading block is 6–9 px shorter
  (line-height rounding and the `h1` margin against the byline's border),
  which shifts every subsequent text row by up to a third of a line and
  the 8×8-window SSIM punishes that on text-heavy screens. Mid and bottom
  paints, where the scroll offsets happen to realign, score 0.91–0.97.
- Remaining visible differences on the corpus (all recorded, none fixed):
  `border-radius` with non-uniform border widths draws square corners
  (uniform ring only); `position: relative` on an inline element does not
  offset it; a missing image draws a grey placeholder where Chromium
  draws the broken-image icon plus alt text; image scaling is a box filter
  (Chromium: bilinear); `vertical-align: middle` text next to images sits
  ~5 px higher; glyphs are placed at integer device pixels.
- The `pre-line` and `pre-wrap` blocks, nested list markers and
  `::before` generated content match Chromium.

### Deferred (consolidated from all stages)

Skeleton stage: `flake.lock` not generated / Nix modules not evaluated
(no Nix here); no static musl / static-pie package; no `lean-alloc`
debug-socket IPC; IPC message types (§2.2) undefined; seccomp/Landlock
(M3); JS (M5); `ArchivedComputedStyle` 344 B vs the ~112 B estimate.

Loader stage: sandbox/broker/worker/IPC (M3); charset detection beyond
UTF-8; custom CA / `SSL_CERT_FILE`; hidden form fields dropped by pass 1;
mixed `%`/`px` `calc()`; nested style rules; `content` beyond strings and
`attr()`; `::marker` styling; presentational hints beyond the listed set;
table emulation (M4); `@import` cycle detection; `markup5ever_rcdom`
unused. New: fold single-text styled inline spans into styled text runs
(the node-reduction gate).

Renderer stage: floats/clear in flow; flex, grid, tables (M4); z-index /
stacking contexts; rounded clips; empty-block collapse-through; absolute
static position approximated; `fixed` scrolls; percent heights against
auto containers; `justify`; RTL; form controls / SVG / iframes as
placeholders; IPC client, navigation, focus ring, find-in-page (M3);
accesskit (M4); subpixel glyph positioning; JPEG not row-streamed; one
`unsafe` in `fonts.rs`; PNG encoder buffers in headless mode; window mode
never run (no display). New: inline `position: relative`, non-uniform
rounded borders, broken-image alt rendering, bilinear image scaling,
`malloc_trim`-equivalent after paint (glibc keeps ~1 MB of freed arena).

Harness stage: corpus HTTP serving (`tiny_http`), `wait4` rusage (loader
self-reports `VmHWM`), the +5 % regression gate against a stored
baseline, GitHub Actions job itself (`.github-workflow-example.yml` shows
it; `.github/` deploys the blog and was not touched), the R1 winit
baseline experiment (needs a display), and 12 more corpus pages of the
plan's 20 (real-world documents, heavy pages, JS pages).

## Stage: review fixes (after the M0–M2 review)

A code review of the M0–M2 build verified the findings below against the
built binaries; each is fixed here, with a regression test where the
finding had a repro. Numbers above were re-measured afterwards
(`lean-measure --corpus corpus`, release build). `cargo build`, `cargo
test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt
--check` pass on the workspace; `git status` shows changes only under
`lean-browser/`.

### Blockers

- **Body-level wrapper collapse split one line into one unit per child**
  (`compress.rs` pass 3 vs `freeze::wrap_body_inlines`). The anonymous
  wrapping of inline body children now runs *after* passes 1-4 (from
  `compress`), so a collapsed `<div class="container">hello <em>x</em>
  world</div>` leaves one anonymous block. Pass 3 additionally accepts a
  block wrapper whose own children are all blocks (layout-neutral
  regardless of its siblings) and refuses one with inline children among
  inline siblings (collapsing it would merge lines). Tests:
  `body_level_wrapper_with_inline_children_stays_one_block`,
  `block_wrapper_among_inline_siblings_collapses_only_with_block_children`;
  `lean-browser --dump-boxes` on the repro shows one block, one line.
- **More than 16 k top-level blocks dropped two thirds of the page**
  (`document.rs` used `step_by`). Units are now contiguous ranges of
  `group` top-level blocks; `relayout` and `layout_viewport` flow each
  group with collapsed margins between its blocks, so `tops[]` stays
  ≤ 16 k entries and every block is laid out and painted. Test
  `more_than_max_units_blocks_are_grouped_not_dropped` (16 389 blocks:
  total height equals the sum of all blocks and margins, both blocks of a
  grouped unit have the right rects). A 20 000-`<p>` page now reports
  10 000 units and 692 516 px (all paragraphs) instead of 13 334 units
  and a third of the content.

### Majors

- **Empty anonymous block per newline between body blocks** (`freeze.rs`).
  Fixed by the reordering above: pass 2 drops the whitespace before the
  wrapper is built. Test `newlines_between_body_blocks_leave_no_empty_units`
  (`<body>\n<div>a</div>\n<div>b</div>\n</body>` → exactly two
  top-level blocks; 300 `<p>` + newlines → 300 blocks, 602 nodes). The
  20 000-`<p>` page serialises 40 002 nodes instead of 60 003.
- **Image size arithmetic overflowed** (`paint/image.rs`): `dw * dh * 4`
  is computed with `u64::checked_mul` (`rgba_bytes`) before any
  allocation, in `decode` and in `Resampler::new`; a `2^31 × 2^31` `<img>`
  is a placeholder. Test in `caps_and_unsupported`.
- **Unbounded glyph rasterization** (`text.rs`): `TextEngine::glyph`
  refuses sizes above `MAX_GLYPH_PX` = 1024 device px (and non-finite
  ones); `Layouter::style` clamps `font-size` to `MAX_FONT_PX` = 512 CSS
  px once, so metrics, line heights and run rects are bounded too. A
  100 000 px / 60 000 px / 20 000 px `M` page paints in a debug build
  without panicking. Test `huge_sizes_are_refused`.
- **Glyph cache cap was not hard** (`GlyphCache::insert`): a mask larger
  than the cap goes into a single `oversize` slot (dropped on the next
  insert) and is never counted; the map is pre-sized for 448 entries and
  never reallocates (evictions keep the live count under half its
  capacity, hashbrown's in-place rehash threshold). `bytes() ≤ cap`
  always. Tests `oversize_glyph_is_not_cached`,
  `entry_count_is_capped_and_the_map_never_grows`.
- **`Tag::GlyphCache` did not measure the cache** (`text.rs`): the mask
  is rendered under `Tag::Text` and the bytes the cache keeps are copied
  under `Tag::GlyphCache` after room is made, so the tag covers exactly
  the cache (masks + map) and a `set_budget(GlyphCache, 256 KB)` can
  fire. The `text` column in the lean-alloc table is shaping/raster
  scratch only.

### Minors

- **Paint blends could overflow `i32`** (`paint/mod.rs`): `blend_mask`
  and `blend_rgba` take `i64` origins and iterate only the source span
  inside the clip (`clipped_span`); `paint_text` adds mask offsets in
  `i64`. Test `blends_placed_near_i32_max_do_not_overflow`; the
  `padding-left: 2147481600px` `<img>` repro paints in a debug build.
- **No `set_budget` calls**: `renderer::install_budgets` sets strip
  320 KB, glyph cache 256 KB, layout 512 KB, image 2.6 MB at start; the
  two figures above the plan's are recorded as deviations (renderer
  section). The harness measures the renderer as a subprocess, so the
  budgets it observes are these.
- **Interlaced PNG whole-image path undocumented**: the constant is now
  `WHOLE_IMAGE_SCRATCH_CAP` (2 MB) and both JPEG and Adam7 PNG are listed
  under the renderer deviations.
- **Pass 2 dropped `&nbsp;`**: pass 2 tests the CSS collapsible set
  (space, tab, LF, CR, FF) like `collapse_text`. Test
  `nbsp_between_blocks_is_kept`.
- **Recursive layout not listed as a deviation**: added to the renderer
  deviations with the `MAX_DEPTH` truncation behaviour.
- **Nix** (not evaluable here, written against the crane/nixpkgs APIs):
  `libx11`/`libxcursor`/`libxi`/`libxrandr`/`libxcb` top-level names
  instead of the deprecated `xorg.*` aliases; `--locked` restored in
  every `cargoExtraArgs`; `cargoFmt` gets `pname`/`version`; a
  `lean.cargoSrc` option (`craneLib.cleanCargoSource`) feeds
  `cargoArtifacts`, the packages, clippy and fmt so corpus/refs changes
  no longer rebuild third-party crates, and a second
  `cargoArtifactsWindow` (with `renderer/window`) backs the windowed
  package; the test check gets `pkgs.dejavu_fonts` (`lean.testFonts`)
  and `LEAN_FONT_DIR` so the shaping tests run in the sandbox, and the
  test source filter now includes `tests/fixtures/`.

### Still open after this stage

- The ≥ 40 % node-reduction gate still fails (35.9 % median; the blog
  pages' per-token `<span>`s remain the reason; see "What does not").
- The layout-tree peak (290 KB on the blog page) exceeds the plan's
  256 KB; the budget is set at 512 KB rather than the plan figure.
- Layout is recursive (depth-capped), not iterative.
