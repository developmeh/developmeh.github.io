//! Decode-at-paint images (plan §7): PNG rows are streamed and box-filtered
//! straight into a display-size buffer; JPEGs are decoded by
//! `jpeg-decoder` at the nearest DCT scale (1/8 steps) at or above the
//! display size and then box-filtered. Anything whose buffers would exceed
//! the caps, or that cannot be decoded, is drawn as a placeholder.

use std::io::Cursor;

use lean_alloc::{scope, Tag};
use page_format::ImageFormat;

/// Cap on the display-size RGBA buffer (plan §7).
pub const DISPLAY_CAP: usize = 512 * 1024;
/// Cap on the whole-image intermediate that JPEG (`jpeg-decoder` has no
/// row API) and interlaced PNG (Adam7 needs every pass) decode into: 4x
/// the display cap, a deliberate deviation from plan §7's 512 KB scratch
/// recorded in STATUS.md. Also the `png` crate's byte limit.
pub const WHOLE_IMAGE_SCRATCH_CAP: usize = 4 * DISPLAY_CAP;

/// A decoded image at display size, premultiplied RGBA8.
#[derive(Debug)]
pub struct Decoded {
    /// Width in device px.
    pub width: u32,
    /// Height in device px.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub data: Vec<u8>,
}

/// Why an image is drawn as a placeholder.
#[derive(Debug, PartialEq, Eq)]
pub enum ImageError {
    /// The display-size buffer or the decode scratch would exceed its cap.
    TooLarge,
    /// SVG or an unknown format.
    Unsupported,
    /// The codec rejected the bytes.
    Decode(String),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::TooLarge => write!(f, "image exceeds the decode cap"),
            ImageError::Unsupported => write!(f, "unsupported image format"),
            ImageError::Decode(m) => write!(f, "decode error: {m}"),
        }
    }
}

/// Accumulates source rows into a display-size buffer with a box filter
/// (or nearest neighbour when upscaling).
struct Resampler {
    sw: u32,
    sh: u32,
    dw: u32,
    dh: u32,
    /// Straight-alpha sums for the current target row, `dw * 4`.
    acc: Vec<u32>,
    count: u32,
    cur_ty: Option<u32>,
    out: Vec<u8>,
    row_avg: Vec<u32>,
}

/// Bytes of an RGBA8 buffer of `w x h`, or `None` when the product
/// overflows or exceeds `cap`. All size arithmetic goes through here so a
/// page cannot make the renderer multiply two `u32`s unchecked.
fn rgba_bytes(w: u32, h: u32, cap: usize) -> Option<usize> {
    let n = u64::from(w).checked_mul(u64::from(h))?.checked_mul(4)?;
    (n <= cap as u64).then_some(n as usize)
}

impl Resampler {
    fn new(sw: u32, sh: u32, dw: u32, dh: u32) -> Result<Resampler, ImageError> {
        let out_len = rgba_bytes(dw, dh, DISPLAY_CAP).ok_or(ImageError::TooLarge)?;
        // Per-row scratch is 16 B per display column; the row cap follows
        // from the buffer cap (a 1-row image cannot exceed DISPLAY_CAP / 4
        // columns).
        Ok(Resampler {
            sw,
            sh,
            dw,
            dh,
            acc: vec![0; dw as usize * 4],
            count: 0,
            cur_ty: None,
            out: vec![0; out_len],
            row_avg: vec![0; dw as usize * 4],
        })
    }

    /// Feeds source row `sy` as straight RGBA8.
    fn push_row(&mut self, sy: u32, rgba: &[u8]) {
        // Horizontal reduce into row_avg (straight alpha, 0..255).
        let (sw, dw) = (self.sw as u64, self.dw as u64);
        for tx in 0..self.dw as u64 {
            let x0 = (tx * sw / dw) as usize;
            let x1 = (((tx + 1) * sw) / dw).max(tx * sw / dw + 1) as usize;
            let x1 = x1.min(self.sw as usize).max(x0 + 1);
            let mut s = [0u32; 4];
            for px in rgba[x0 * 4..x1 * 4].chunks_exact(4) {
                for (acc, &v) in s.iter_mut().zip(px) {
                    *acc += u32::from(v);
                }
            }
            let n = (x1 - x0) as u32;
            let t = tx as usize * 4;
            for (dst, v) in self.row_avg[t..t + 4].iter_mut().zip(s) {
                *dst = v / n;
            }
        }
        // Vertical: which target rows does sy feed?
        let (sh, dh) = (self.sh as u64, self.dh as u64);
        let ty0 = (u64::from(sy) * dh / sh) as u32;
        let ty1 = (((u64::from(sy) + 1) * dh).div_ceil(sh) as u32)
            .max(ty0 + 1)
            .min(self.dh);
        for ty in ty0..ty1 {
            if self.cur_ty != Some(ty) {
                self.flush();
                self.cur_ty = Some(ty);
            }
            for (a, v) in self.acc.iter_mut().zip(&self.row_avg) {
                *a += *v;
            }
            self.count += 1;
        }
    }

    fn flush(&mut self) {
        let Some(ty) = self.cur_ty else { return };
        if self.count == 0 || ty >= self.dh {
            return;
        }
        let row = &mut self.out[ty as usize * self.dw as usize * 4..][..self.dw as usize * 4];
        for (px, s) in row.chunks_exact_mut(4).zip(self.acc.chunks_exact(4)) {
            let a = s[3] / self.count;
            // Premultiply.
            px[0] = ((s[0] / self.count) * a / 255) as u8;
            px[1] = ((s[1] / self.count) * a / 255) as u8;
            px[2] = ((s[2] / self.count) * a / 255) as u8;
            px[3] = a as u8;
        }
        self.acc.iter_mut().for_each(|a| *a = 0);
        self.count = 0;
    }

    fn finish(mut self) -> Decoded {
        self.flush();
        Decoded {
            width: self.dw,
            height: self.dh,
            data: self.out,
        }
    }
}

/// Decodes `bytes` to `dw x dh` device pixels.
pub fn decode(bytes: &[u8], format: ImageFormat, dw: u32, dh: u32) -> Result<Decoded, ImageError> {
    if dw == 0 || dh == 0 || rgba_bytes(dw, dh, DISPLAY_CAP).is_none() {
        return Err(ImageError::TooLarge);
    }
    let _tag = scope(Tag::Image);
    match format {
        ImageFormat::Png => decode_png(bytes, dw, dh),
        ImageFormat::Jpeg => decode_jpeg(bytes, dw, dh),
        ImageFormat::Svg | ImageFormat::Unsupported => Err(ImageError::Unsupported),
    }
}

/// Expands one PNG output row (after `EXPAND | STRIP_16 | ALPHA`) to RGBA8.
fn expand_png_row(row: &[u8], channels: usize, out: &mut Vec<u8>) {
    out.clear();
    match channels {
        1 => out.extend(row.iter().flat_map(|&g| [g, g, g, 255])),
        2 => out.extend(row.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]])),
        3 => out.extend(row.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255])),
        _ => out.extend_from_slice(&row[..row.len() / 4 * 4]),
    }
}

fn decode_png(bytes: &[u8], dw: u32, dh: u32) -> Result<Decoded, ImageError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(
        png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
    );
    decoder.set_limits(png::Limits {
        bytes: WHOLE_IMAGE_SCRATCH_CAP,
    });
    let mut reader = decoder
        .read_info()
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    let (sw, sh, interlaced) = {
        let info = reader.info();
        (info.width, info.height, info.interlaced)
    };
    if sw == 0 || sh == 0 {
        return Err(ImageError::Decode("empty image".into()));
    }
    let channels = reader.output_color_type().0.samples();
    let mut rs = Resampler::new(sw, sh, dw, dh)?;
    let mut rgba = Vec::with_capacity(sw as usize * 4);
    if interlaced {
        // Adam7 needs the whole image; only allowed within the scratch cap
        // (the same 2 MB the JPEG path uses; see STATUS.md).
        let size = reader.output_buffer_size().ok_or(ImageError::TooLarge)?;
        if size > WHOLE_IMAGE_SCRATCH_CAP {
            return Err(ImageError::TooLarge);
        }
        let mut buf = vec![0u8; size];
        let info = reader
            .next_frame(&mut buf)
            .map_err(|e| ImageError::Decode(e.to_string()))?;
        let stride = info.line_size;
        for sy in 0..sh {
            let row = &buf[sy as usize * stride..][..stride];
            expand_png_row(row, channels, &mut rgba);
            rs.push_row(sy, &rgba);
        }
    } else {
        let mut sy = 0;
        while let Some(row) = reader
            .next_row()
            .map_err(|e| ImageError::Decode(e.to_string()))?
        {
            if sy >= sh {
                break;
            }
            expand_png_row(row.data(), channels, &mut rgba);
            rs.push_row(sy, &rgba);
            sy += 1;
        }
    }
    Ok(rs.finish())
}

fn decode_jpeg(bytes: &[u8], dw: u32, dh: u32) -> Result<Decoded, ImageError> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(WHOLE_IMAGE_SCRATCH_CAP);
    decoder
        .read_info()
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    let (sw, sh) = decoder
        .scale(
            dw.min(u16::MAX as u32) as u16,
            dh.min(u16::MAX as u32) as u16,
        )
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    let info = decoder
        .info()
        .ok_or(ImageError::Decode("no header".into()))?;
    let bpp = info.pixel_format.pixel_bytes();
    if u64::from(sw) * u64::from(sh) * bpp as u64 > WHOLE_IMAGE_SCRATCH_CAP as u64 {
        return Err(ImageError::TooLarge);
    }
    let pixels = decoder
        .decode()
        .map_err(|e| ImageError::Decode(e.to_string()))?;
    let (sw, sh) = (u32::from(sw), u32::from(sh));
    if sw == 0 || sh == 0 {
        return Err(ImageError::Decode("empty image".into()));
    }
    let mut rs = Resampler::new(sw, sh, dw, dh)?;
    let mut rgba = Vec::with_capacity(sw as usize * 4);
    let stride = sw as usize * bpp;
    for sy in 0..sh {
        let row = &pixels[sy as usize * stride..][..stride];
        rgba.clear();
        match info.pixel_format {
            jpeg_decoder::PixelFormat::L8 => rgba.extend(row.iter().flat_map(|&g| [g, g, g, 255])),
            jpeg_decoder::PixelFormat::L16 => {
                rgba.extend(row.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], 255]))
            }
            jpeg_decoder::PixelFormat::RGB24 => {
                rgba.extend(row.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]))
            }
            jpeg_decoder::PixelFormat::CMYK32 => rgba.extend(row.chunks_exact(4).flat_map(|p| {
                let k = u32::from(p[3]);
                let f = |c: u8| ((255 - u32::from(c)) * (255 - k) / 255) as u8;
                [f(p[0]), f(p[1]), f(p[2]), 255]
            })),
        }
        rs.push_row(sy, &rgba);
    }
    Ok(rs.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes a tiny RGBA PNG with the `png` crate.
    pub(crate) fn make_png(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w, h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().unwrap();
            let mut data = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    data.extend_from_slice(&f(x, y));
                }
            }
            wr.write_image_data(&data).unwrap();
        }
        out
    }

    #[test]
    fn png_downscales_with_box_filter() {
        // 4x4 image, left half red, right half blue -> 2x1.
        let png = make_png(4, 4, |x, _| {
            if x < 2 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        });
        let d = decode(&png, ImageFormat::Png, 2, 1).unwrap();
        assert_eq!((d.width, d.height), (2, 1));
        assert_eq!(&d.data[0..4], &[255, 0, 0, 255]);
        assert_eq!(&d.data[4..8], &[0, 0, 255, 255]);
    }

    #[test]
    fn png_upscales_nearest() {
        let png = make_png(1, 1, |_, _| [10, 20, 30, 255]);
        let d = decode(&png, ImageFormat::Png, 3, 2).unwrap();
        assert_eq!(d.data.len(), 3 * 2 * 4);
        assert!(d.data.chunks_exact(4).all(|p| p == [10, 20, 30, 255]));
    }

    #[test]
    fn png_alpha_is_premultiplied() {
        let png = make_png(2, 2, |_, _| [200, 100, 0, 128]);
        let d = decode(&png, ImageFormat::Png, 1, 1).unwrap();
        assert_eq!(d.data, vec![100, 50, 0, 128]);
    }

    #[test]
    fn caps_and_unsupported() {
        assert_eq!(
            decode(&[], ImageFormat::Svg, 1, 1).err(),
            Some(ImageError::Unsupported)
        );
        assert_eq!(
            decode(&[], ImageFormat::Png, 1000, 1000).err(),
            Some(ImageError::TooLarge)
        );
        // Sizes whose byte count overflows usize arithmetic are rejected
        // before any multiplication (a page may style an <img> this big).
        assert_eq!(
            decode(&[], ImageFormat::Png, 1 << 31, 1 << 31).err(),
            Some(ImageError::TooLarge)
        );
        assert_eq!(
            decode(&[], ImageFormat::Png, u32::MAX, 1).err(),
            Some(ImageError::TooLarge)
        );
        assert_eq!(rgba_bytes(256, 512, DISPLAY_CAP), Some(DISPLAY_CAP));
        assert_eq!(rgba_bytes(256, 513, DISPLAY_CAP), None);
        assert!(matches!(
            decode(b"not a png", ImageFormat::Png, 1, 1),
            Err(ImageError::Decode(_))
        ));
        assert!(matches!(
            decode(b"not a jpeg", ImageFormat::Jpeg, 1, 1),
            Err(ImageError::Decode(_))
        ));
    }
}
