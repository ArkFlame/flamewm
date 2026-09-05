use std::env;
use std::fs;
use std::path::PathBuf;

use flamewm_render_compiler::{CompileOptions, compile_str, encode};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let html_path = manifest.join("../../ui/desktop/index.html");
    let css_path = manifest.join("../../ui/desktop/desktop.css");
    println!("cargo:rerun-if-changed={}", html_path.display());
    println!("cargo:rerun-if-changed={}", css_path.display());
    let html = fs::read_to_string(&html_path).expect("read desktop UI");
    let items = (0..128)
        .map(|index| format!("<button id=\"desktop-item-{index}\" class=\"desktop-item\" data-action=\"desktop.item.{index}\"><div class=\"desktop-glyph\">+</div><div id=\"desktop-label-{index}\" class=\"desktop-label\"></div></button>"))
        .collect::<Vec<_>>()
        .join("\n  ");
    let html = html.replace("<!--ITEMS-->", &items);
    let css = fs::read_to_string(&css_path).expect("read desktop CSS");
    let output = compile_str(&html, &css, CompileOptions::default()).expect("compile desktop UI");
    let bytes = encode(&output.document).expect("encode desktop UI");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out.join("flamewm-desktop.rwr"), bytes).expect("write desktop UI");
}
