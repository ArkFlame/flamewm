use resvg::{tiny_skia, usvg};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CANVAS: u32 = 48;
const SAMPLE_SCALE: f32 = 10.0;
const GLYPH_SIZE: f32 = 44.0;

fn main() {
    let source_dir = Path::new("assets/icons");
    let output = PathBuf::from(env::var("OUT_DIR").expect("build output directory must exist"));
    println!("cargo:rerun-if-changed={}", source_dir.display());
    let mut icons = fs::read_dir(source_dir)
        .expect("icon source directory must be readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("svg" | "png" | "webp")
            )
        })
        .collect::<Vec<_>>();
    icons.sort();
    let mut names = HashSet::new();
    let mut registry =
        String::from("fn registered_icon(name: &str) -> Option<ImageData> {\n    match name {\n");
    for source in icons {
        println!("cargo:rerun-if-changed={}", source.display());
        let name = source
            .file_stem()
            .and_then(|name| name.to_str())
            .expect("icon name must be valid UTF-8");
        assert!(names.insert(name.to_owned()), "duplicate icon name: {name}");
        let destination = output.join(format!("{name}.png"));
        match source.extension().and_then(|ext| ext.to_str()) {
            Some("svg") => rasterize_svg(&source, &destination),
            Some("png" | "webp") => {
                fs::copy(&source, &destination)
                    .expect("raster icon must be copied into build output");
            }
            _ => unreachable!(),
        }
        registry.push_str(&format!(
            "        \"{name}\" => Some(ImageData::from_bytes(include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{name}.png\"))).expect(\"bundled icon must be valid\")),\n"
        ));
    }
    registry.push_str("        _ => None,\n    }\n}\n");
    fs::write(output.join("icons_registry.rs"), registry)
        .expect("icon registry must be written into build output");
}

fn rasterize_svg(source: &Path, destination: &Path) {
    let svg =
        normalize_colors(fs::read_to_string(source).expect("bundled icon source must be readable"));
    let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default())
        .expect("bundled icon source must be valid SVG");
    let source_size = tree.size();
    let sample_size = (24.0 * SAMPLE_SCALE) as u32;
    let sample_scale = SAMPLE_SCALE / source_size.width().max(source_size.height()) * 24.0;
    let mut sample =
        tiny_skia::Pixmap::new(sample_size, sample_size).expect("icon sample pixmap must fit");
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(sample_scale, sample_scale),
        &mut sample.as_mut(),
    );
    let (left, top, width, height) = alpha_bounds(&sample);
    let source_width = width as f32 / sample_scale;
    let source_height = height as f32 / sample_scale;
    let scale = GLYPH_SIZE / source_width.max(source_height);
    let tx = (CANVAS as f32 - source_width * scale) / 2.0 - left as f32 / sample_scale * scale;
    let ty = (CANVAS as f32 - source_height * scale) / 2.0 - top as f32 / sample_scale * scale;
    let mut pixmap = tiny_skia::Pixmap::new(CANVAS, CANVAS).expect("icon pixmap must fit");
    resvg::render(
        &tree,
        tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, tx, ty),
        &mut pixmap.as_mut(),
    );
    pixmap
        .save_png(destination)
        .expect("rasterized icon must be writable");
}

fn normalize_colors(mut svg: String) -> String {
    svg = svg.replace("currentColor", "#f4f4f5");
    for attribute in ["fill=\"", "stroke=\""] {
        let mut cursor = 0;
        while let Some(offset) = svg[cursor..].find(attribute) {
            let value_start = cursor + offset + attribute.len();
            let Some(end_offset) = svg[value_start..].find('"') else {
                break;
            };
            let value_end = value_start + end_offset;
            if &svg[value_start..value_end] != "none" {
                svg.replace_range(value_start..value_end, "#f4f4f5");
            }
            cursor = value_end + 1;
        }
    }
    let mut cursor = 0;
    while let Some(offset) = svg[cursor..].find('#') {
        let start = cursor + offset;
        let end = start + 7;
        if end <= svg.len()
            && svg[start + 1..end]
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            svg.replace_range(start..end, "#f4f4f5");
            cursor = start + 7;
        } else {
            cursor = start + 1;
        }
    }
    svg
}

fn alpha_bounds(pixmap: &tiny_skia::Pixmap) -> (u32, u32, u32, u32) {
    let mut left = pixmap.width();
    let mut top = pixmap.height();
    let mut right = 0;
    let mut bottom = 0;
    for (index, pixel) in pixmap.data().chunks_exact(4).enumerate() {
        if pixel[3] == 0 {
            continue;
        }
        let x = index as u32 % pixmap.width();
        let y = index as u32 / pixmap.width();
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    (left, top, right - left + 1, bottom - top + 1)
}
