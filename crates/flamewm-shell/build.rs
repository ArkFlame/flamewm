use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use flamewm_render_compiler::{compile_file, encode, CompileOptions};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let ui = root.join("ui/shell/index.html");
    let css = root.join("ui/shell/flamewm.css");
    let raster = root.join("assets/raster");

    println!("cargo:rerun-if-changed={}", ui.display());
    println!("cargo:rerun-if-changed={}", css.display());
    println!("cargo:rerun-if-changed={}", raster.display());

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let staged_ui = out_dir.join("shell");
    let staged_assets = staged_ui.join("assets");
    fs::create_dir_all(&staged_assets).expect("create staged shell assets");
    fs::copy(&ui, staged_ui.join("index.html")).expect("stage shell UI");
    fs::copy(&css, staged_ui.join("flamewm.css")).expect("stage shell CSS");
    for entry in fs::read_dir(&raster).expect("read shell raster assets") {
        let entry = entry.expect("read shell raster asset entry");
        let asset = entry.path();
        if asset.is_file() {
            fs::copy(&asset, staged_assets.join(entry.file_name()))
                .expect("stage shell raster asset");
        }
    }

    let staged_html = staged_ui.join("index.html");
    let compiled = compile_file(&staged_html, &[], CompileOptions::default())
        .unwrap_or_else(|error| panic!("failed to compile {}: {error}", ui.display()));
    let bytes = encode(&compiled.document).expect("compiled FlameWM UI must encode");
    fs::write(out_dir.join("flamewm-shell.rwr"), bytes).expect("write compiled FlameWM UI");
}
