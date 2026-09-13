#[cfg(not(target_os = "windows"))]
mod bluetoothctl;
#[cfg(target_os = "windows")]
mod windows;

pub trait BluetoothIntegration {
    fn powered(&self) -> bool;
    fn connected(&self) -> bool;
    fn device_name(&self) -> Option<String>;
    fn set_powered(&self, powered: bool);
}

#[cfg(not(target_os = "windows"))]
pub use bluetoothctl::BluetoothCtl;
#[cfg(target_os = "windows")]
pub use windows::WindowsBluetooth;

pub struct Fallback;

impl BluetoothIntegration for Fallback {
    fn powered(&self) -> bool {
        false
    }
    fn connected(&self) -> bool {
        false
    }
    fn device_name(&self) -> Option<String> {
        None
    }
    fn set_powered(&self, _: bool) {}
}
