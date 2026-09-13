#[cfg(not(target_os = "windows"))]
mod kwin_dbus;
#[cfg(target_os = "windows")]
mod windows;

use crate::platform::OpenWindow;
use creamui_render::WindowHandle;

pub trait DesktopIntegration {
    fn prepare_window(&self, window: &WindowHandle);
    fn windows(&self) -> Vec<OpenWindow>;
    fn activate_window(&self, id: &str);
}

pub struct Fallback;
#[cfg(not(target_os = "windows"))]
pub use kwin_dbus::KWinDbus;
#[cfg(target_os = "windows")]
pub use windows::WindowsDesktop;

impl DesktopIntegration for Fallback {
    fn prepare_window(&self, window: &WindowHandle) {
        window.set_always_on_top(true);
    }
    fn windows(&self) -> Vec<OpenWindow> {
        Vec::new()
    }
    fn activate_window(&self, _id: &str) {}
}
