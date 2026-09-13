#[cfg(not(target_os = "windows"))]
mod upower;
#[cfg(target_os = "windows")]
mod windows;

pub trait BatteryIntegration {
    fn percentage(&self) -> Option<u8>;
    fn charging(&self) -> bool;
}

#[cfg(not(target_os = "windows"))]
pub use upower::UPower;
#[cfg(target_os = "windows")]
pub use windows::WindowsBattery;
pub struct Fallback;
impl BatteryIntegration for Fallback {
    fn percentage(&self) -> Option<u8> {
        None
    }
    fn charging(&self) -> bool {
        false
    }
}
