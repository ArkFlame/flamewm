use flamewm_render_core::{RuntimeDocument, decode};

const COMPILED_UI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-settings.rwr"));

pub fn document() -> Result<RuntimeDocument, String> {
    RuntimeDocument::new(decode(COMPILED_UI)?)
}
