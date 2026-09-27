//! Image sniffing: format detection from magic bytes and intrinsic size
//! from the header only. PNG and JPEG bytes are stored in the page file
//! for decode-at-paint; GIF/WebP/AVIF/others become placeholders of their
//! intrinsic size (plan §5 fallbacks).

use page_format::ImageFormat;

/// What the loader learned about an image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    /// Encoding.
    pub format: ImageFormat,
    /// Intrinsic width in px (0 if unknown).
    pub width: u16,
    /// Intrinsic height in px (0 if unknown).
    pub height: u16,
    /// Whether the bytes should be stored in the page file.
    pub store: bool,
}

/// Sniffs `bytes` (optionally guided by a MIME type for SVG text).
pub fn sniff(bytes: &[u8], mime: Option<&str>) -> ImageInfo {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let (w, h) = png_size(bytes).unwrap_or((0, 0));
        return ImageInfo {
            format: ImageFormat::Png,
            width: w,
            height: h,
            store: true,
        };
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        let (w, h) = jpeg_size(bytes).unwrap_or((0, 0));
        return ImageInfo {
            format: ImageFormat::Jpeg,
            width: w,
            height: h,
            store: true,
        };
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        let (w, h) = gif_size(bytes).unwrap_or((0, 0));
        return placeholder(w, h);
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        let (w, h) = webp_size(bytes).unwrap_or((0, 0));
        return placeholder(w, h);
    }
    if is_svg(bytes, mime) {
        let (w, h) = svg_size(bytes).unwrap_or((0, 0));
        return ImageInfo {
            format: ImageFormat::Svg,
            width: w,
            height: h,
            store: true,
        };
    }
    placeholder(0, 0)
}

fn placeholder(w: u16, h: u16) -> ImageInfo {
    ImageInfo {
        format: ImageFormat::Unsupported,
        width: w,
        height: h,
        store: false,
    }
}

fn clamp(v: u32) -> u16 {
    v.min(u16::MAX as u32) as u16
}

fn png_size(b: &[u8]) -> Option<(u16, u16)> {
    if b.len() < 24 || &b[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(b[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(b[20..24].try_into().ok()?);
    Some((clamp(w), clamp(h)))
}

/// Walks JPEG markers to the first SOF segment.
fn jpeg_size(b: &[u8]) -> Option<(u16, u16)> {
    let mut i = 2;
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = b[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        // Standalone markers without a length.
        if matches!(marker, 0x01 | 0xD0..=0xD7) {
            i += 2;
            continue;
        }
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        let is_sof = matches!(
            marker,
            0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD | 0xCE | 0xCF
        );
        if is_sof {
            if i + 9 > b.len() {
                return None;
            }
            let h = u16::from_be_bytes([b[i + 5], b[i + 6]]);
            let w = u16::from_be_bytes([b[i + 7], b[i + 8]]);
            return Some((w, h));
        }
        if marker == 0xD9 || marker == 0xDA {
            return None;
        }
        i += 2 + len;
    }
    None
}

fn gif_size(b: &[u8]) -> Option<(u16, u16)> {
    if b.len() < 10 {
        return None;
    }
    Some((
        u16::from_le_bytes([b[6], b[7]]),
        u16::from_le_bytes([b[8], b[9]]),
    ))
}

fn webp_size(b: &[u8]) -> Option<(u16, u16)> {
    if b.len() < 30 {
        return None;
    }
    match &b[12..16] {
        b"VP8 " => {
            // Key frame: start code at 23..26, then 14-bit width/height.
            if b[23..26] != [0x9D, 0x01, 0x2A] {
                return None;
            }
            let w = u16::from_le_bytes([b[26], b[27]]) & 0x3FFF;
            let h = u16::from_le_bytes([b[28], b[29]]) & 0x3FFF;
            Some((w, h))
        }
        b"VP8L" => {
            if b[20] != 0x2F {
                return None;
            }
            let bits = u32::from_le_bytes([b[21], b[22], b[23], b[24]]);
            let w = (bits & 0x3FFF) + 1;
            let h = ((bits >> 14) & 0x3FFF) + 1;
            Some((clamp(w), clamp(h)))
        }
        b"VP8X" => {
            let w = 1 + u32::from_le_bytes([b[24], b[25], b[26], 0]);
            let h = 1 + u32::from_le_bytes([b[27], b[28], b[29], 0]);
            Some((clamp(w), clamp(h)))
        }
        _ => None,
    }
}

fn is_svg(bytes: &[u8], mime: Option<&str>) -> bool {
    if mime.is_some_and(|m| m.contains("svg")) {
        return true;
    }
    let head = &bytes[..bytes.len().min(512)];
    let text = String::from_utf8_lossy(head);
    text.contains("<svg")
}

/// Reads `width`/`height` (px) or, failing that, `viewBox` from the root
/// `<svg>` tag.
fn svg_size(bytes: &[u8]) -> Option<(u16, u16)> {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(2048)]);
    let start = text.find("<svg")?;
    let tag_end = text[start..].find('>').map(|e| start + e).unwrap_or(text.len());
    let tag = &text[start..tag_end];
    let attr = |name: &str| -> Option<String> {
        let mut rest = tag;
        while let Some(pos) = rest.find(name) {
            let after = &rest[pos + name.len()..];
            let before_ok = pos == 0 || !rest.as_bytes()[pos - 1].is_ascii_alphanumeric();
            let after_trim = after.trim_start();
            if before_ok && after_trim.starts_with('=') {
                let v = after_trim[1..].trim_start();
                let quote = v.chars().next()?;
                if quote == '"' || quote == '\'' {
                    let end = v[1..].find(quote)?;
                    return Some(v[1..1 + end].to_string());
                }
                let end = v.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(v.len());
                return Some(v[..end].to_string());
            }
            rest = &rest[pos + name.len()..];
        }
        None
    };
    let px = |v: String| -> Option<u16> {
        let digits: String = v
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let n: f32 = digits.parse().ok()?;
        if v.trim().ends_with('%') {
            return None;
        }
        Some(n.round().clamp(0.0, 65535.0) as u16)
    };
    if let (Some(w), Some(h)) = (attr("width").and_then(px), attr("height").and_then(px)) {
        return Some((w, h));
    }
    let vb = attr("viewBox")?;
    let parts: Vec<f32> = vb
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() == 4 {
        return Some((
            parts[2].round().clamp(0.0, 65535.0) as u16,
            parts[3].round().clamp(0.0, 65535.0) as u16,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png() {
        let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        b.extend_from_slice(&300u32.to_be_bytes());
        b.extend_from_slice(&200u32.to_be_bytes());
        b.extend_from_slice(&[8, 6, 0, 0, 0]);
        let i = sniff(&b, None);
        assert_eq!(i.format, ImageFormat::Png);
        assert_eq!((i.width, i.height), (300, 200));
        assert!(i.store);
    }

    #[test]
    fn jpeg() {
        // SOI, APP0 (length 16), SOF0 with height 40 width 60.
        let mut b = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        b.extend_from_slice(&[0u8; 14]);
        b.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 8, 0, 40, 0, 60, 3]);
        let i = sniff(&b, None);
        assert_eq!(i.format, ImageFormat::Jpeg);
        assert_eq!((i.width, i.height), (60, 40));
    }

    #[test]
    fn gif_webp_and_unknown_are_placeholders() {
        let mut g = b"GIF89a".to_vec();
        g.extend_from_slice(&[10, 0, 20, 0]);
        let i = sniff(&g, None);
        assert_eq!(i.format, ImageFormat::Unsupported);
        assert_eq!((i.width, i.height), (10, 20));
        assert!(!i.store);

        let mut w = b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x2F".to_vec();
        // width-1 = 15, height-1 = 7 packed little-endian: 15 | 7<<14
        let bits: u32 = 15 | (7 << 14);
        w.extend_from_slice(&bits.to_le_bytes());
        w.extend_from_slice(&[0; 8]);
        let i = sniff(&w, None);
        assert_eq!((i.width, i.height), (16, 8));
        assert_eq!(sniff(b"hello", None).format, ImageFormat::Unsupported);
    }

    #[test]
    fn svg_sizes() {
        let i = sniff(br#"<?xml version="1.0"?><svg xmlns="x" width="120px" height="30"><rect/></svg>"#, None);
        assert_eq!(i.format, ImageFormat::Svg);
        assert_eq!((i.width, i.height), (120, 30));
        let i = sniff(b"<svg viewBox='0 0 640 480'></svg>", Some("image/svg+xml"));
        assert_eq!((i.width, i.height), (640, 480));
        let i = sniff(b"<svg width='100%'></svg>", None);
        assert_eq!((i.width, i.height), (0, 0));
    }
}
