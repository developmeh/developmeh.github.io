//! Visual comparison metrics (plan §9): grayscale SSIM over 8×8 windows and
//! the percentage of pixels differing by more than 12/255 in any channel.
//!
//! Own implementation on purpose: ~80 lines instead of an OpenCV
//! dependency, and the harness's memory does not matter.

use std::path::Path;

use image::RgbaImage;

/// Per-channel tolerance for the mismatch metric.
pub const TOLERANCE: u8 = 12;
/// SSIM window edge in pixels.
pub const WINDOW: u32 = 8;

/// What two images differ by.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DiffResult {
    /// Mean SSIM over the 8×8 grayscale windows of the common area (1 = identical).
    pub ssim: f64,
    /// Percentage of pixels in the common area with any channel off by more than [`TOLERANCE`].
    pub mismatch_pct: f64,
    /// Candidate size.
    pub candidate: (u32, u32),
    /// Reference size.
    pub reference: (u32, u32),
}

impl DiffResult {
    /// Whether the two images had the same dimensions.
    pub fn same_size(&self) -> bool {
        self.candidate == self.reference
    }
}

/// Loads two PNGs and compares them. Sizes may differ: the metrics are then
/// computed over the top-left common area and the result records both sizes.
pub fn diff_files(candidate: &Path, reference: &Path) -> Result<DiffResult, String> {
    let load = |p: &Path| {
        image::open(p)
            .map(|i| i.into_rgba8())
            .map_err(|e| format!("{}: {e}", p.display()))
    };
    Ok(diff_images(&load(candidate)?, &load(reference)?))
}

/// Compares two decoded images.
pub fn diff_images(a: &RgbaImage, b: &RgbaImage) -> DiffResult {
    let w = a.width().min(b.width());
    let h = a.height().min(b.height());
    let ga = gray(a, w, h);
    let gb = gray(b, w, h);
    DiffResult {
        ssim: ssim(&ga, &gb, w, h),
        mismatch_pct: mismatch_pct(a, b, w, h),
        candidate: a.dimensions(),
        reference: b.dimensions(),
    }
}

/// Rec. 601 luma, composited over white so transparent pixels compare as
/// the renderer paints them.
fn gray(img: &RgbaImage, w: u32, h: u32) -> Vec<f64> {
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            let a = f64::from(p[3]) / 255.0;
            let c = |v: u8| f64::from(v) * a + 255.0 * (1.0 - a);
            out.push(0.299 * c(p[0]) + 0.587 * c(p[1]) + 0.114 * c(p[2]));
        }
    }
    out
}

fn mismatch_pct(a: &RgbaImage, b: &RgbaImage, w: u32, h: u32) -> f64 {
    if w == 0 || h == 0 {
        return 100.0;
    }
    let mut bad = 0u64;
    for y in 0..h {
        for x in 0..w {
            let p = a.get_pixel(x, y).0;
            let q = b.get_pixel(x, y).0;
            if p.iter()
                .zip(q.iter())
                .any(|(x, y)| x.abs_diff(*y) > TOLERANCE)
            {
                bad += 1;
            }
        }
    }
    100.0 * bad as f64 / f64::from(w * h)
}

/// Mean SSIM over non-overlapping 8×8 windows (partial edge windows are
/// included when at least 2×2 pixels remain).
fn ssim(a: &[f64], b: &[f64], w: u32, h: u32) -> f64 {
    const C1: f64 = (0.01 * 255.0) * (0.01 * 255.0);
    const C2: f64 = (0.03 * 255.0) * (0.03 * 255.0);
    if w == 0 || h == 0 {
        return 0.0;
    }
    let (w, h) = (w as usize, h as usize);
    let win = WINDOW as usize;
    let mut sum = 0.0;
    let mut count = 0usize;
    let mut y0 = 0;
    while y0 < h {
        let y1 = (y0 + win).min(h);
        let mut x0 = 0;
        while x0 < w {
            let x1 = (x0 + win).min(w);
            let n = ((y1 - y0) * (x1 - x0)) as f64;
            if y1 - y0 >= 2 && x1 - x0 >= 2 {
                let (mut sa, mut sb, mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0, 0.0, 0.0);
                for y in y0..y1 {
                    for x in x0..x1 {
                        let (p, q) = (a[y * w + x], b[y * w + x]);
                        sa += p;
                        sb += q;
                        saa += p * p;
                        sbb += q * q;
                        sab += p * q;
                    }
                }
                let (ma, mb) = (sa / n, sb / n);
                let va = (saa / n - ma * ma).max(0.0);
                let vb = (sbb / n - mb * mb).max(0.0);
                let cov = sab / n - ma * mb;
                sum += ((2.0 * ma * mb + C1) * (2.0 * cov + C2))
                    / ((ma * ma + mb * mb + C1) * (va + vb + C2));
                count += 1;
            }
            x0 = x1;
        }
        y0 = y1;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(c))
    }

    #[test]
    fn identical_images_score_one() {
        let mut a = solid(32, 24, [255, 255, 255, 255]);
        for x in 0..32 {
            a.put_pixel(x, 10, Rgba([0, 0, 0, 255]));
        }
        let r = diff_images(&a, &a);
        assert!((r.ssim - 1.0).abs() < 1e-9);
        assert_eq!(r.mismatch_pct, 0.0);
        assert!(r.same_size());
    }

    #[test]
    fn inverted_images_score_low() {
        let mut a = solid(16, 16, [255, 255, 255, 255]);
        let mut b = solid(16, 16, [255, 255, 255, 255]);
        for x in 0..16 {
            a.put_pixel(x, 4, Rgba([0, 0, 0, 255]));
            b.put_pixel(x, 12, Rgba([0, 0, 0, 255]));
        }
        let r = diff_images(&a, &b);
        assert!(r.ssim < 0.9, "{r:?}");
        assert!((r.mismatch_pct - 12.5).abs() < 1e-9, "{r:?}");
    }

    #[test]
    fn small_differences_are_tolerated_in_mismatch() {
        let a = solid(8, 8, [100, 100, 100, 255]);
        let b = solid(8, 8, [100, 100, 112, 255]);
        let r = diff_images(&a, &b);
        assert_eq!(r.mismatch_pct, 0.0);
        let c = solid(8, 8, [100, 100, 113, 255]);
        assert_eq!(diff_images(&a, &c).mismatch_pct, 100.0);
    }

    #[test]
    fn different_sizes_use_common_area() {
        let a = solid(20, 20, [0, 0, 0, 255]);
        let b = solid(10, 30, [0, 0, 0, 255]);
        let r = diff_images(&a, &b);
        assert!(!r.same_size());
        assert!((r.ssim - 1.0).abs() < 1e-9);
    }
}
