use creamui_core::{BoxedWidget, Styled};
use creamui_image::{Image, ImageData, ImageFit};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub fn pixel_icon(name: &str, size: f32) -> BoxedWidget {
    static CACHE: OnceLock<Mutex<HashMap<String, ImageData>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let data = cache
        .lock()
        .ok()
        .and_then(|cache| cache.get(name).cloned())
        .or_else(|| {
            let data = registered_icon(name)?;
            if let Ok(mut cache) = cache.lock() {
                cache.insert(name.to_owned(), data.clone());
            }
            Some(data)
        })
        .unwrap_or_else(|| panic!("missing bundled icon: {name}"));
    Box::new(
        Image::new(data)
            .layout(creamui_core::layout::Style {
                size: creamui_widgets::layout::fixed(size, size),
                ..Default::default()
            })
            .fit(ImageFit::Contain),
    )
}

include!(concat!(env!("OUT_DIR"), "/icons_registry.rs"));
