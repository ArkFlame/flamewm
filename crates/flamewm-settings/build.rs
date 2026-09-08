use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use flamewm_render_compiler::{CompileOptions, compile_file, encode};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let ui = root.join("ui/settings.html");
    let css = root.join("ui/settings.css");
    let raster = root.join("assets/raster");

    println!("cargo:rerun-if-changed={}", ui.display());
    println!("cargo:rerun-if-changed={}", css.display());
    println!("cargo:rerun-if-changed={}", raster.display());

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let staged_ui = out_dir.join("settings");
    let staged_assets = staged_ui.join("assets");
    fs::create_dir_all(&staged_assets).expect("create staged settings assets");
    fs::copy(&ui, staged_ui.join("settings.html")).expect("stage settings UI");
    fs::copy(&css, staged_ui.join("settings.css")).expect("stage settings CSS");
    for entry in fs::read_dir(&raster).expect("read settings raster assets") {
        let entry = entry.expect("read settings raster asset entry");
        let asset = entry.path();
        if asset.is_file() {
            fs::copy(&asset, staged_assets.join(entry.file_name()))
                .expect("stage settings raster asset");
        }
    }

    let staged_html = staged_ui.join("settings.html");
    let compiled = compile_file(&staged_html, &[], CompileOptions::default())
        .unwrap_or_else(|error| panic!("failed to compile {}: {error}", ui.display()));
    let output = out_dir.join("flamewm-settings.rwr");
    fs::write(
        output,
        encode(&compiled.document).expect("encode settings UI"),
    )
    .expect("write settings UI");
}
