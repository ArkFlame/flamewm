use std::env;
use std::path::PathBuf;

use flamewm_render_compiler::{CompileOptions, compile_file, encode};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let ui = manifest.join("../../ui/settings.html");
    let css = manifest.join("../../ui/settings.css");
    println!("cargo:rerun-if-changed={}", ui.display());
    println!("cargo:rerun-if-changed={}", css.display());
    let compiled = compile_file(&ui, &[], CompileOptions::default()).expect("compile settings UI");
    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("out directory")).join("flamewm-settings.rwr");
    std::fs::write(
        output,
        encode(&compiled.document).expect("encode settings UI"),
    )
    .expect("write settings UI");
}
