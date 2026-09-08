//! SVG decode via resvg/usvg + tiny-skia. Alpha is preserved.

use crate::{RgbaImage, check_size_limits, unpremultiply_swapped_to_rgba};

/// Returns natural (intrinsic) SVG dimensions rounded up to whole pixels.
pub fn natural_size(bytes: &[u8]) -> Result<(u32, u32), String> {
    let tree = resvg::usvg::Tree::from_data_nested(bytes, &resvg::usvg::Options::default())
        .map_err(|error| error.to_string())?;
    let size = tree.size();
    if !size.width().is_finite()
        || !size.height().is_finite()
        || size.width() <= 0.0
        || size.height() <= 0.0
    {
        return Err("SVG dimensions must be finite and non-zero".to_string());
    }
    Ok((size.width().ceil() as u32, size.height().ceil() as u32))
}

/// Renders SVG to an exact target size, preserving alpha.
pub fn render(bytes: &[u8], width: u32, height: u32) -> Result<RgbaImage, String> {
    check_size_limits(width, height)?;
    let tree = resvg::usvg::Tree::from_data_nested(bytes, &resvg::usvg::Options::default())
        .map_err(|error| error.to_string())?;
    let svg_size = tree.size();
    if !svg_size.width().is_finite()
        || !svg_size.height().is_finite()
        || svg_size.width() <= 0.0
        || svg_size.height() <= 0.0
    {
        return Err("SVG dimensions must be finite and non-zero".to_string());
    }
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("requested dimensions overflow")?;
    let transform = resvg::tiny_skia::Transform::from_scale(
        width as f32 / svg_size.width(),
        height as f32 / svg_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    if pixmap.width() != width || pixmap.height() != height {
        return Err("raster dimensions do not match request".to_string());
    }
    let pixels = unpremultiply_swapped_to_rgba(pixmap.data());
    RgbaImage::from_rgba8(width, height, pixels)
        .ok_or_else(|| "requested dimensions overflow".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "../../assets/web/flamewm-icon.svg";

    #[test]
    fn renders_fixture_with_alpha() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
        let bytes = std::fs::read(path).expect("SVG fixture");
        let image = render(&bytes, 8, 8).unwrap();
        assert_eq!((image.width, image.height), (8, 8));
        assert_eq!(image.pixels.len(), 8 * 8 * 4);
        assert!(
            image
                .pixels
                .chunks_exact(4)
                .any(|pixel| pixel[3] > 0 && pixel[..3] != [0, 0, 0])
        );
    }

    #[test]
    fn rejects_invalid_svg() {
        assert!(render(b"not svg", 8, 8).is_err());
    }
}
