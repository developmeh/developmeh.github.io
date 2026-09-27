# Lean Browser

A limited web browser whose single optimizing target is minimal resident
private memory (`Private_Dirty`) in the long-lived process. A short-lived
**loader** does everything untrusted and allocation-heavy (fetch, parse,
cascade, compress) and writes a compact, validated, read-only **page file**;
the long-lived **renderer** memory-maps that file and paints the viewport
through a small strip buffer. The full design is in
[`../plans/lean-browser.md`](../plans/lean-browser.md); what is and is not
done yet is in [`STATUS.md`](STATUS.md).

## Layout

```
Cargo.toml              workspace; every third-party dependency is declared here
crates/
  page-format/          page file: header, rkyv structs, writer, mmap reader, validator
  lean-alloc/           tagging, counting global allocator with per-tag budgets
  css-subset/           shared cascade data model (ComputedStyle, enums, interning)
  loader/               bin lean-loader
  renderer/             bin lean-browser (feature `window` adds winit/softbuffer/accesskit)
  harness/              bins lean-measure, lean-diff; smaps_rollup sampler, SSIM, report
flake.nix, nix/         flake-parts modules (packages, devShell, checks, formatter)
corpus/                 frozen test pages (manifest.json), assets, generators in tools/
refs/                   Chromium reference PNGs (<name>-{top,mid,bottom}.png) + VERSION pin
.github-workflow-example.yml   how CI would run `nix flake check` and the measurement
```

## Building with cargo

Requires a stable Rust toolchain (`rust-toolchain.toml` selects `stable`
with clippy, rustfmt, rust-src and rust-analyzer).

```sh
cd lean-browser
cargo build --workspace                 # headless renderer, no display needed
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p renderer --features window   # windowed renderer (winit/softbuffer/accesskit)
cargo build -p loader --features js         # QuickJS support (M5), needs a C compiler
```

Nothing outside the `window` feature links a system library; winit and
softbuffer `dlopen` libwayland / libxkbcommon / libX11 at run time.

### Try it

```sh
# Load a page (file path, file://, http(s):// or data: URL) into a page file,
# printing node counts before/after each compression pass.
cargo run -p loader --bin lean-loader -- crates/loader/tests/fixtures/blog/post.html \
    --out /tmp/post.lpg --viewport 1280x800 --stats
cargo run -p renderer --bin lean-browser -- --headless --page /tmp/post.lpg \
    --paint-png /tmp/out.png --viewport 1280x800 --stats
cargo run -p harness --bin lean-diff -- /tmp/out.png /tmp/out.png
cargo run -p harness --bin lean-measure -- --self
cargo run -p loader --bin lean-loader -- --write-empty /tmp/empty.lpg   # root node only
```

`lean-loader` runs the document-mode pipeline of plan §2 in one process:
fetch (plan §11 size caps) → html5ever → UA + author stylesheets
(`@import`, `@media` for the viewport bucket, `@layer`, `@supports`) →
lightningcss/parcel_selectors cascade with `var()`, `em`/`rem`/`pt`
resolution and inheritance → accessible roles and names → the seven
compression passes of plan §6 (`--check` re-runs the cascade on the
compressed tree and fails if any style changed) → `page.lpg`.

The renderer refuses any page file that fails the four validation steps
(header, CRC-32, rkyv structural check, semantic pass) and exits non-zero.

### Measuring (plan §9)

```sh
cargo build --release --workspace
./target/release/lean-measure --corpus corpus            # -> target/measure/report.{json,md}
./target/release/lean-measure --corpus corpus --only lists --settle-ms 1000 --samples 4
./target/release/lean-diff target/measure/lists-top.png refs/lists-top.png [--json]
```

For every page in `corpus/manifest.json`, `lean-measure` runs
`lean-loader … --stats` (recording wall time, the loader's own peak RSS,
per-pass node counts and the page file size), then starts
`lean-browser page.lpg --serve --viewport 1280x800` and drives it over
stdin: `paint 0 top.png`, `paint 0.5 mid.png`, `paint 1 bottom.png`, then
samples `/proc/<pid>/smaps_rollup` ten times over five seconds while the
renderer is still alive, then `stats` (lean-alloc per-tag live/peak) and
`quit`. Each PNG is compared with `refs/<name>-<position>.png` when it
exists (grayscale SSIM over 8×8 windows and the percentage of pixels off by
more than 12/255). The report has one row per page plus the empty-page
baseline, the plan §8 gates (`--gate` makes a failed gate exit 1) and the
per-pass compression counts. The renderer is measured without a window: in
`--serve` mode everything a window would hold (page map, fonts, shaping
contexts, strip buffer) is allocated once and kept, only the PNG encoder is
per paint.

Fonts: the corpus pins DejaVu (`corpus.css`, `blog/main.css`) so the
renderer's file-stem discovery and Chromium's fontconfig pick the same
faces; pass `--font-dir` (or set `LEAN_FONT_DIR`) when DejaVu is not in a
system font directory.

Chromium references are generated with the Playwright Chromium pinned in
`refs/VERSION` (JS disabled, `--disable-gpu --hide-scrollbars`, DPR 1):

```sh
NODE_PATH=/opt/node22/lib/node_modules PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers \
    node corpus/tools/make-refs.cjs [--only name]
```

The two blog pages are rendered from this repository's `content/` by
`corpus/tools/md2html.py` (Zola is not available in the build environment;
the script approximates the theme's markup and inline-style syntax
highlighting). `corpus/tools/gen-assets.py`, `gen-jpeg.cjs` and
`gen-wrapper-soup.py` regenerate the tiny images and the wrapper-soup page.

## Building with Nix

The flake is a [flake-parts](https://flake.parts) *module project*:
`flake.nix` only wires inputs and imports `nix/flake-module.nix`, which in
turn imports one module per concern:

| module              | provides                                                     |
|---------------------|--------------------------------------------------------------|
| `nix/toolchain.nix` | `perSystem.lean.*` options: pkgs, Rust toolchain (rust-overlay), crane, source filter, winit libraries, shared crane args |
| `nix/packages.nix`  | `packages.lean-browser` (all bins, headless), `packages.lean-browser-window`, `packages.default` |
| `nix/devshell.nix`  | `devShells.lean-browser` / `default`: toolchain, clippy, rust-analyzer, pkg-config, wayland/x11/libxkbcommon, `LD_LIBRARY_PATH` |
| `nix/checks.nix`    | `checks.lean-browser-{build,clippy,test,fmt}`                 |
| `nix/formatter.nix` | `formatter` = nixfmt                                          |

Supported systems: `x86_64-linux`, `aarch64-linux`.

```sh
cd lean-browser
nix build                 # packages.default -> result/bin/{lean-browser,lean-loader,lean-measure,lean-diff}
nix build .#lean-browser-window
nix flake check           # clippy -D warnings, cargo test, rustfmt, build
nix develop               # dev shell
nix fmt                   # format the Nix files
```

There is no `flake.lock` in the repository yet (it could not be generated in
the environment this skeleton was written in); the first `nix build` creates
it, and it should then be committed.

### Reusing the module from another flake

The module is exported as `flakeModules.default` and carries its own
locked inputs, so the importing flake needs neither `crane` nor
`rust-overlay`:

```nix
{
  inputs.lean-browser.url = "github:developmeh/developmeh.github.io?dir=lean-browser";
  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      imports = [ inputs.lean-browser.flakeModules.default ];
      # Optional overrides, e.g. a pinned toolchain or a different nixpkgs:
      # perSystem = { config, ... }: { lean.toolchain = ...; lean.pkgs = ...; };
    };
}
```

## Dependencies

All third-party crates any milestone needs are declared once in
`[workspace.dependencies]` and referenced from the crates with
`workspace = true`, so later work on `loader` and `renderer` does not need to
touch `Cargo.lock`. Two intentional deviations from the plan's list:

- `parcel_selectors` instead of `selectors`: it is lightningcss's fork of
  the Servo `selectors` crate with the same matching API, and it is the type
  lightningcss's parsed selectors actually use.
- `cssparser` is pinned to the version lightningcss depends on (0.37) to
  avoid two copies.
