#[cfg(not(target_os = "windows"))]
mod pipewire;
#[cfg(target_os = "windows")]
mod windows;

pub trait VolumeIntegration {
    fn level(&self) -> f32;
    fn muted(&self) -> bool;
    fn set_level(&self, level: f32);
}

#[cfg(not(target_os = "windows"))]
pub use pipewire::PipeWire;
#[cfg(target_os = "windows")]
pub use windows::WindowsVolume;
pub struct Fallback;
impl VolumeIntegration for Fallback {
    fn level(&self) -> f32 {
        0.0
    }
    fn muted(&self) -> bool {
        false
    }
    fn set_level(&self, _: f32) {}
}
