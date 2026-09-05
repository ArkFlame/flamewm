use std::env;
use std::fs;
use std::path::PathBuf;

use rustwebrender_compiler::{compile_file, encode, CompileOptions};

fn main() {
    if let Err(error) = run() {
        eprintln!("rwr-compile: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut css = Vec::new();
    let mut strict = true;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => output = Some(PathBuf::from(args.next().ok_or("--output requires a path")?)),
            "--css" => css.push(PathBuf::from(args.next().ok_or("--css requires a path")?)),
            "--allow-unsupported" => strict = false,
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            value if value.starts_with('-') => return Err(format!("unknown option '{value}'")),
            value => {
                if input.replace(PathBuf::from(value)).is_some() {
                    return Err("only one HTML input is accepted".to_string());
                }
            }
        }
    }

    let input = input.ok_or("missing HTML input; run rwr-compile --help")?;
    let output = output.unwrap_or_else(|| input.with_extension("rwr"));
    let compiled = compile_file(&input, &css, CompileOptions { strict })?;
    for warning in &compiled.warnings {
        eprintln!("warning: {warning}");
    }
    let bytes = encode(&compiled.document)?;
    fs::write(&output, &bytes).map_err(|error| format!("failed to write {}: {error}", output.display()))?;
    eprintln!(
        "compiled {} -> {} ({} nodes, {} variables, {} assets, {} bytes, fingerprint {:016x})",
        input.display(),
        output.display(),
        compiled.document.nodes.len(),
        compiled.document.variables.len(),
        compiled.document.assets.len(),
        bytes.len(),
        compiled.document.source_fingerprint,
    );
    Ok(())
}

fn print_help() {
    println!("RustWebRender 0.0.9 build-time compiler");
    println!("usage: rwr-compile [--css file.css] [--allow-unsupported] [-o output.rwr] input.html");
}
