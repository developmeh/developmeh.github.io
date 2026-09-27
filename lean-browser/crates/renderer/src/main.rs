//! `lean-browser`: the long-lived renderer binary (plan §2, §7).
//!
//! Headless (works without a display):
//!
//! ```text
//! lean-browser page.lpg --headless --paint-png out.png --viewport 1280x800 [--scroll-to 0.5]
//! ```
//!
//! Window mode needs the `window` cargo feature (winit + softbuffer).

#![deny(unsafe_code)]

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lean_alloc::{scope, Tag};
use renderer::fonts::FontSource;
use renderer::{paint_viewport, Document, FontSet, PaintParams, StripBuffer, TextEngine};

#[global_allocator]
static ALLOC: lean_alloc::LeanAlloc = lean_alloc::LeanAlloc;

const USAGE: &str = "\
usage: lean-browser [<page.lpg>] [--headless] [--paint-png <out.png>] [--viewport <WxH>]
                    [--dpr <n>] [--scroll-to <0..1> | --scroll <px>] [--page <page.lpg>]
                    [--font <file> | --font-dir <dir>] [--list-fonts] [--dump-boxes] [--stats]
                    [--write-demo <page.lpg>]

  <page.lpg> / --page   page file to map and validate (required to paint content)
  --headless            no window (the only mode without the `window` feature)
  --paint-png <path>    paint the viewport into a PNG and exit
  --viewport <WxH>      viewport in CSS px (default 1280x800)
  --dpr <n>             device pixel ratio (default 1)
  --scroll-to <f>       scroll to fraction f of the scrollable range (0 = top, 1 = bottom)
  --scroll <px>         scroll to an absolute offset in CSS px
  --font <file>         use one TrueType/OpenType file for every family
  --font-dir <dir>      search this directory for fonts (else $LEAN_FONT_DIR, then system dirs)
  --list-fonts          print which font file each family/variant resolved to
  --dump-boxes          print the viewport layout tree to stderr
  --stats               print lean-alloc per-tag peaks to stderr at exit
  --write-demo <path>   write a demonstration page file (no loader needed) and exit
";

struct Options {
    headless: bool,
    paint_png: Option<PathBuf>,
    width: u32,
    height: u32,
    dpr: u32,
    page: Option<PathBuf>,
    scroll_to: Option<f32>,
    scroll_px: Option<f32>,
    font: FontSource,
    list_fonts: bool,
    dump_boxes: bool,
    stats: bool,
    write_demo: Option<PathBuf>,
}

fn parse_args() -> Result<Options, String> {
    let mut o = Options {
        headless: false,
        paint_png: None,
        width: 1280,
        height: 800,
        dpr: 1,
        page: None,
        scroll_to: None,
        scroll_px: None,
        font: FontSource::Auto,
        list_fonts: false,
        dump_boxes: false,
        stats: false,
        write_demo: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--headless" => o.headless = true,
            "--stats" => o.stats = true,
            "--list-fonts" => o.list_fonts = true,
            "--dump-boxes" => o.dump_boxes = true,
            "--paint-png" => o.paint_png = Some(PathBuf::from(value()?)),
            "--page" => o.page = Some(PathBuf::from(value()?)),
            "--write-demo" => o.write_demo = Some(PathBuf::from(value()?)),
            "--font" => o.font = FontSource::File(PathBuf::from(value()?)),
            "--font-dir" => o.font = FontSource::Dir(PathBuf::from(value()?)),
            "--dpr" => o.dpr = value()?.parse().map_err(|_| "--dpr needs an integer")?,
            "--scroll-to" => {
                o.scroll_to = Some(value()?.parse().map_err(|_| "--scroll-to needs a number")?)
            }
            "--scroll" => {
                o.scroll_px = Some(value()?.parse().map_err(|_| "--scroll needs a number")?)
            }
            "--viewport" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--viewport needs WxH")?;
                o.width = w.parse().map_err(|_| "bad viewport width")?;
                o.height = h.parse().map_err(|_| "bad viewport height")?;
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            other if !other.starts_with('-') && o.page.is_none() => {
                o.page = Some(PathBuf::from(other))
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if o.width == 0 || o.height == 0 || o.dpr == 0 {
        return Err("viewport and dpr must be non-zero".into());
    }
    Ok(o)
}

fn main() -> ExitCode {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("lean-browser: {msg}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = run(&opts);
    if opts.stats {
        eprint!("{}", lean_alloc::snapshot().report());
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lean-browser: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(opts: &Options) -> Result<(), String> {
    if let Some(path) = &opts.write_demo {
        renderer::testing::demo_page(opts.width as u16)
            .write(path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        eprintln!("lean-browser: wrote demo page to {}", path.display());
        if opts.page.is_none() && opts.paint_png.is_none() {
            return Ok(());
        }
    }
    let fonts = FontSet::load(&opts.font)?;
    if opts.list_fonts {
        eprint!("{}", fonts.describe());
    }
    if fonts.is_empty() {
        eprintln!("lean-browser: warning: no fonts found; text will not be painted");
    }
    let mut text = TextEngine::default();
    let viewport = (opts.width as f32, opts.height as f32);

    let doc = match &opts.page {
        Some(path) => Some(Document::open(path, viewport, &fonts, &mut text)?),
        None => None,
    };
    if let Some(d) = &doc {
        let p = d.page();
        eprintln!(
            "lean-browser: mapped {} ({} nodes, {} styles, {} units, {:.0} px tall)",
            p.final_url.as_str(),
            p.nodes.len(),
            p.styles.len(),
            d.units().len(),
            d.content_height()
        );
    }

    match (&opts.paint_png, opts.headless) {
        (Some(out), _) => paint_png(opts, doc.as_ref(), &fonts, &mut text, out),
        (None, true) => Ok(()),
        (None, false) => window_mode(opts, doc, fonts, text),
    }
}

fn scroll_offset(opts: &Options, doc: Option<&Document>) -> f32 {
    let max = doc.map_or(0.0, Document::max_scroll);
    match (opts.scroll_px, opts.scroll_to) {
        (Some(px), _) => px.clamp(0.0, max),
        (None, Some(f)) => (f.clamp(0.0, 1.0) * max).round(),
        (None, None) => 0.0,
    }
}

/// Paints the viewport strip by strip into a PNG.
fn paint_png(
    opts: &Options,
    doc: Option<&Document>,
    fonts: &FontSet,
    text: &mut TextEngine,
    out: &Path,
) -> Result<(), String> {
    let params = PaintParams {
        scroll_y: scroll_offset(opts, doc),
        viewport_w: opts.width as f32,
        viewport_h: opts.height as f32,
        dpr: opts.dpr as f32,
        background: doc.map_or(css_subset::Rgba::WHITE, Document::background),
    };
    let (w, h) = params.device_size();

    let tree = match doc {
        Some(d) => d.layout_viewport(params.scroll_y, fonts, text),
        None => renderer::LayoutTree::default(),
    };
    if opts.dump_boxes {
        eprint!("{}", tree.dump());
    }

    let file = File::create(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| e.to_string())?
        .into_stream_writer_with_size(w as usize * 4)
        .map_err(|e| e.to_string())?;

    let mut strip = StripBuffer::new(w);
    let empty = page_format::Page::empty("about:blank", opts.width as u16);
    let empty_bytes;
    let empty_file;
    let page = match doc {
        Some(d) => d.page(),
        None => {
            empty_bytes = page_format::encode(&empty, 0).map_err(|e| e.to_string())?;
            empty_file =
                page_format::PageFile::from_bytes(empty_bytes).map_err(|e| e.to_string())?;
            empty_file.page()
        }
    };
    paint_viewport(
        page,
        fonts,
        text,
        &tree,
        &params,
        &mut strip,
        |bytes, _, _| std::io::Write::write_all(&mut writer, bytes).map_err(|e| e.to_string()),
    )?;
    drop(tree);
    writer.finish().map_err(|e| e.to_string())
}

#[cfg(feature = "window")]
fn window_mode(
    opts: &Options,
    doc: Option<Document>,
    fonts: FontSet,
    text: TextEngine,
) -> Result<(), String> {
    let _tag = scope(Tag::Window);
    let doc = doc.ok_or("window mode needs a page file")?;
    renderer::window::run(
        doc,
        fonts,
        text,
        (opts.width, opts.height),
        scroll_offset(opts, None),
    )
}

#[cfg(not(feature = "window"))]
fn window_mode(
    _opts: &Options,
    _doc: Option<Document>,
    _fonts: FontSet,
    _text: TextEngine,
) -> Result<(), String> {
    let _tag = scope(Tag::Window);
    Err("built without the `window` feature; use --headless --paint-png".into())
}
