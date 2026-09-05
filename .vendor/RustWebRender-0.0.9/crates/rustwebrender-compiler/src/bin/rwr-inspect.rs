use std::env;
use std::fs;

use rustwebrender_core::decode;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: rwr-inspect file.rwr");
        std::process::exit(2);
    });
    let bytes = fs::read(&path).unwrap_or_else(|error| {
        eprintln!("rwr-inspect: failed to read {path}: {error}");
        std::process::exit(1);
    });
    let document = decode(&bytes).unwrap_or_else(|error| {
        eprintln!("rwr-inspect: {error}");
        std::process::exit(1);
    });
    println!("format=RWRB/2");
    println!("assets={}", document.assets.len());
    println!("fingerprint={:016x}", document.source_fingerprint);
    println!("nodes={}", document.nodes.len());
    println!("variables={}", document.variables.len());
    for (index, variable) in document.variables.iter().enumerate() {
        println!("var[{index}] {} = #{:02x}{:02x}{:02x}{:02x}", variable.name, variable.default.r, variable.default.g, variable.default.b, variable.default.a);
    }
    for (index, node) in document.nodes.iter().enumerate() {
        println!("node[{index}] kind={:?} parent={:?} id={:?} action={:?} text={:?}", node.kind, node.parent, node.id, node.action, node.text);
    }
}
