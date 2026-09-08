use flamewm_ui_x11::{UiDocument, decode_document};

const COMPILED_UI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-settings.rwr"));

pub fn document() -> Result<UiDocument, String> {
    decode_document(COMPILED_UI)
}
