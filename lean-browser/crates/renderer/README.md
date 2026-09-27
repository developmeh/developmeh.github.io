# renderer (`lean-browser`)

The long-lived renderer from plan §2/§7: maps a validated page file, lays
out only the scrollbar units that intersect the viewport, and paints
through a strip buffer of at most 64 rows / 320 KB.

```
src/
  main.rs          CLI; headless --paint-png; window dispatch
  lib.rs           deny(unsafe_code) crate root
  document.rs      Document: PageFile + tops[] (one f32 per top-level block), units in view
  fonts.rs         font discovery and read-only mmap (the crate's single `unsafe`)
  text.rs          swash ShapeContext/ScaleContext, per-char face fallback, LRU glyph cache (256 KB)
  layout/mod.rs    LayoutTree/LayoutBox, Layouter, unit + absolute-position frames
  layout/block.rs  widths, heights, margin collapsing, positioning, list markers, intrinsic widths
  layout/inline.rs items, white-space, UAX #14 pieces, greedy lines, placement
  paint/mod.rs     StripBuffer, band loop, iterative paint walk, borders/radii/decorations
  paint/image.rs   decode-at-paint PNG (row streamed) / JPEG (DCT-scaled), box filter, caps
  window.rs        feature `window`: winit + softbuffer, scrolling, link click prints the URL
  testing.rs       PageBuilder (writes page files via page-format) and the demo page
tests/layout.rs    hand-computed boxes (blocks need no font; text tests skip without one)
tests/paint.rs     pixel-exact strip painting, scrolling, DPR, images, placeholders
```

## Running headless (no display needed)

```sh
cargo run -p renderer --bin lean-browser -- --write-demo /tmp/demo.lpg
cargo run -p renderer --bin lean-browser -- /tmp/demo.lpg --headless \
    --paint-png /tmp/out.png --viewport 1280x800 [--scroll-to 0.5] [--dpr 2] [--stats]
```

`--dump-boxes` prints the viewport layout tree, `--list-fonts` the resolved
faces, `--stats` the `lean-alloc` per-tag peaks. A page file that fails
validation is rejected with a non-zero exit.

Window mode: `cargo run -p renderer --features window -- page.lpg`. Wheel,
arrows, Page Up/Down, Space, Home and End scroll; a left click on a link
prints its `href` to stdout (navigation itself is M3); `q`/Escape quit.

## Fonts

No fonts are bundled in git (plan §7 bundles three Noto faces in the
*package*, not the repository). At start the renderer resolves one face per
generic family (sans, serif, mono) and variant (regular, bold, italic,
bold-italic), memory-maps each file read-only (zero `Private_Dirty`) and
never reads font metadata beyond what `swash` needs:

1. `--font <file>` uses one TrueType/OpenType file for everything — the way
   to get deterministic output in CI;
2. `--font-dir <dir>` (or `LEAN_FONT_DIR`) is searched recursively;
3. otherwise `~/.local/share/fonts`, `~/.fonts`, `/usr/share/fonts`,
   `/usr/local/share/fonts`, `/run/current-system/sw/share/X11/fonts` and
   `$XDG_DATA_DIRS/*/fonts`.

Matching is fontconfig-free and by file name: for each family/variant the
first hit from a fixed preference list (`NotoSans-Regular`, `DejaVuSans`,
`LiberationSans-Regular`, `FreeSans`, `Arimo`, `Roboto`, `OpenSans`,
`Cantarell`, `arial`, ... and the serif/mono equivalents) wins. A missing
variant falls back to the family's regular face, a missing family to sans,
and a character missing from the chosen face is shaped with any other
mapped face that covers it. With no font at all the renderer still runs
(text measures as zero and is not painted), which keeps block-only tests
independent of the machine.

For the Chromium comparisons in plan §9 point `LEAN_FONT_DIR` at the same
Noto files the reference container installs.

## Memory shape (what allocates)

| what | where | lifetime |
|---|---|---|
| page file, fonts | mmap, read-only | process |
| `tops[]` | `Document` (4 B per unit, grouped above 16k units) | per page |
| strip buffer | `StripBuffer` (≤ 320 KB, resized only on width change) | process |
| glyph cache | `TextEngine` (≤ 256 KB of A8 masks, LRU by bytes) | process |
| layout tree | `Document::layout_viewport` | one paint |
| image decode | `paint::image` (display buffer ≤ 512 KB; JPEG scratch ≤ 2 MB) | one band |
