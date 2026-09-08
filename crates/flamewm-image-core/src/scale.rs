//! Deterministic RGBA8 scaling: nearest-neighbor and bilinear.

use crate::RgbaImage;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScaleFilter {
    Nearest,
    Bilinear,
}

/// Scales to an exact target size. Returns `None` on size overflow.
/// Bilinear operates per-channel including alpha (straight alpha); border
/// samples clamp to the edge.
#[must_use]
pub fn scale_rgba(
    source: &RgbaImage,
    width: u32,
    height: u32,
    filter: ScaleFilter,
) -> Option<RgbaImage> {
    if width == 0 || height == 0 || source.width == 0 || source.height == 0 {
        return None;
    }
    let pixel_count = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    let mut pixels = vec![0u8; pixel_count];
    match filter {
        ScaleFilter::Nearest => scale_nearest(source, width, height, &mut pixels),
        ScaleFilter::Bilinear => scale_bilinear(source, width, height, &mut pixels),
    }
    RgbaImage::from_rgba8(width, height, pixels)
}

fn scale_nearest(source: &RgbaImage, width: u32, height: u32, output: &mut [u8]) {
    let target_w = width as usize;
    let target_h = height as usize;
    let src_w = source.width as usize;
    let src_h = source.height as usize;
    for y in 0..target_h {
        let source_y = y * src_h / target_h;
        for x in 0..target_w {
            let source_x = x * src_w / target_w;
            let source_index = (source_y * src_w + source_x) * 4;
            let output_index = (y * target_w + x) * 4;
            output[output_index..output_index + 4]
                .copy_from_slice(&source.pixels[source_index..source_index + 4]);
        }
    }
}

fn scale_bilinear(source: &RgbaImage, width: u32, height: u32, output: &mut [u8]) {
    let target_w = width as usize;
    let target_h = height as usize;
    let src_w = source.width as usize;
    let src_h = source.height as usize;
    let x_ratio = src_w as f32 / target_w as f32;
    let y_ratio = src_h as f32 / target_h as f32;
    for y in 0..target_h {
        let source_y = (y as f32 + 0.5) * y_ratio - 0.5;
        let y0 = source_y.floor().max(0.0) as usize;
        let y1 = (y0 + 1).min(src_h - 1);
        let fy = (source_y - y0 as f32).clamp(0.0, 1.0);
        for x in 0..target_w {
            let source_x = (x as f32 + 0.5) * x_ratio - 0.5;
            let x0 = source_x.floor().max(0.0) as usize;
            let x1 = (x0 + 1).min(src_w - 1);
            let fx = (source_x - x0 as f32).clamp(0.0, 1.0);
            let output_index = (y * target_w + x) * 4;
            for channel in 0..4 {
                let a = f32::from(sample(source, x0, y0, channel, src_w));
                let b = f32::from(sample(source, x1, y0, channel, src_w));
                let c = f32::from(sample(source, x0, y1, channel, src_w));
                let d = f32::from(sample(source, x1, y1, channel, src_w));
                let value = a * (1.0 - fx) * (1.0 - fy)
                    + b * fx * (1.0 - fy)
                    + c * (1.0 - fx) * fy
                    + d * fx * fy;
                output[output_index + channel] = value.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

fn sample(source: &RgbaImage, x: usize, y: usize, channel: usize, stride: usize) -> u8 {
    source.pixels[(y * stride + x) * 4 + channel]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_rgba() -> RgbaImage {
        RgbaImage::from_rgba8(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 128]).unwrap()
    }

    #[test]
    fn nearest_matches_expected_mapping() {
        let scaled = scale_rgba(&fixture_rgba(), 2, 2, ScaleFilter::Nearest).unwrap();
        assert_eq!(
            scaled.pixels,
            vec![
                255, 0, 0, 255, 0, 255, 0, 128, 255, 0, 0, 255, 0, 255, 0, 128
            ]
        );
    }

    #[test]
    fn bilinear_uniform_stays_uniform() {
        let source = RgbaImage::from_rgba8(
            2,
            2,
            vec![
                10, 20, 30, 200, 10, 20, 30, 200, 10, 20, 30, 200, 10, 20, 30, 200,
            ],
        )
        .unwrap();
        let scaled = scale_rgba(&source, 4, 4, ScaleFilter::Bilinear).unwrap();
        assert!(
            scaled
                .pixels
                .chunks_exact(4)
                .all(|pixel| pixel == [10, 20, 30, 200])
        );
    }
}
