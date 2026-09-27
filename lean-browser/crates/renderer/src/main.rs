//! `lean-browser`: the long-lived renderer (plan §2, §7).
//!
//! M0 skeleton: the headless path maps and validates a page file (if
//! given) and paints a solid background through the strip buffer into a
//! PNG. Layout, text and the window (feature `window`) come with M1+.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lean_alloc::{scope, Tag};
use page_format::PageFile;

#[global_allocator]
static ALLOC: lean_alloc::LeanAlloc = lean_alloc::LeanAlloc;

/// Strip buffer cap from plan §7.
const STRIP_BYTES_CAP: usize = 320 * 1024;
/// Maximum rows per strip.
const STRIP_MAX_ROWS: usize = 64;

const USAGE: &str = "\
usage: lean-browser [--headless] --paint-png <out.png> [--viewport <WxH>] [--dpr <n>]
                    [--page <page.lpg>] [--stats]

  --headless            no window (the only mode without the `window` feature)
  --paint-png <path>    paint the viewport into a PNG and exit
  --viewport <WxH>      viewport in CSS px (default 1280x800)
  --dpr <n>             device pixel ratio (default 1)
  --page <path>         page file to map and validate before painting
  --stats               print lean-alloc per-tag peaks to stderr at exit
";

struct Options {
    headless: bool,
    paint_png: Option<PathBuf>,
    width: u32,
    height: u32,
    dpr: u32,
    page: Option<PathBuf>,
    stats: bool,
}

fn parse_args() -> Result<Options, String> {
    let mut o = Options {
        headless: false,
        paint_png: None,
        width: 1280,
        height: 800,
        dpr: 1,
        page: None,
        stats: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--headless" => o.headless = true,
            "--stats" => o.stats = true,
            "--paint-png" => o.paint_png = Some(PathBuf::from(value()?)),
            "--page" => o.page = Some(PathBuf::from(value()?)),
            "--dpr" => o.dpr = value()?.parse().map_err(|_| "--dpr needs an integer")?,
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

    let page = match &opts.page {
        Some(path) => {
            let _tag = scope(Tag::PageFile);
            match PageFile::open(path) {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("lean-browser: rejected page file {}: {e}", path.display());
                    return ExitCode::FAILURE;
                }
            }
        }
        None => None,
    };
    if let Some(p) = &page {
        eprintln!(
            "lean-browser: mapped {} ({} nodes, {} styles, {} bytes)",
            p.page().final_url.as_str(),
            p.page().nodes.len(),
            p.page().styles.len(),
            p.len()
        );
    }

    let result = match (&opts.paint_png, opts.headless) {
        (Some(out), _) => paint_png(&opts, out),
        (None, true) => Ok(()),
        (None, false) => window_mode(),
    };

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

/// Paints the viewport strip by strip into a PNG. For now every strip is
/// the canvas colour; the paint walk over the layout tree plugs in here.
fn paint_png(opts: &Options, out: &Path) -> Result<(), String> {
    let width = (opts.width * opts.dpr) as usize;
    let height = (opts.height * opts.dpr) as usize;
    let rows = (STRIP_BYTES_CAP / (width * 4)).clamp(1, STRIP_MAX_ROWS);

    let file = File::create(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| e.to_string())?
        .into_stream_writer_with_size(width * 4)
        .map_err(|e| e.to_string())?;

    // One strip allocation reused for every band (plan §7).
    let mut strip = {
        let _tag = scope(Tag::Strip);
        vec![0u8; width * rows * 4]
    };
    let mut y = 0;
    while y < height {
        let band = rows.min(height - y);
        let px = &mut strip[..width * band * 4];
        for p in px.chunks_exact_mut(4) {
            p.copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        }
        std::io::Write::write_all(&mut writer, px).map_err(|e| e.to_string())?;
        y += band;
    }
    writer.finish().map_err(|e| e.to_string())
}

#[cfg(feature = "window")]
fn window_mode() -> Result<(), String> {
    Err("window mode is not implemented yet (M0 skeleton); use --headless --paint-png".into())
}

#[cfg(not(feature = "window"))]
fn window_mode() -> Result<(), String> {
    Err("built without the `window` feature; use --headless --paint-png".into())
}
