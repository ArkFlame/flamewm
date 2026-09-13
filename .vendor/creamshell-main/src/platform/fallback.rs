use super::{DesktopBackend, OpenWindow, Playback};
use creamui_render::WindowHandle;

pub struct FallbackBackend;

impl DesktopBackend for FallbackBackend {
    fn prepare_window(&self, window: &WindowHandle) {
        window.set_always_on_top(true);
    }

    fn windows(&self) -> Vec<OpenWindow> {
        Vec::new()
    }

    fn activate_window(&self, _id: &str) {}
    fn playback(&self) -> Option<Playback> {
        None
    }
    fn toggle_playback(&self) {}
}
