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
