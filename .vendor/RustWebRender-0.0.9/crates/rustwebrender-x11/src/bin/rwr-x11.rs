use std::env;
use std::fs;

use rustwebrender_core::{decode, RuntimeDocument};
use rustwebrender_x11::{run, X11Config};

fn main() {
    if let Err(error) = execute() {
        eprintln!("rwr-x11: {error}");
        std::process::exit(1);
    }
}

fn execute() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut path = None;
    let mut config = X11Config::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--width" => config.width = args.next().ok_or("--width requires a value")?.parse().map_err(|_| "invalid --width")?,
            "--height" => config.height = args.next().ok_or("--height requires a value")?.parse().map_err(|_| "invalid --height")?,
            "--title" => config.title = args.next().ok_or("--title requires a value")?,
            "-h" | "--help" => {
                println!("usage: rwr-x11 [--width N] [--height N] [--title TEXT] file.rwr");
                return Ok(());
            }
            value if value.starts_with('-') => return Err(format!("unknown option '{value}'")),
            value => {
                if path.replace(value.to_string()).is_some() {
                    return Err("only one .rwr input is accepted".to_string());
                }
            }
        }
    }
    let path = path.ok_or("missing .rwr input")?;
    let bytes = fs::read(&path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let compiled = decode(&bytes)?;
    let runtime = RuntimeDocument::new(compiled)?;
    run(runtime, config)
}
