#![allow(dead_code)]

pub mod audio;
pub mod battery;
pub mod bluetooth;
pub mod brightness;
pub mod desktop;
pub mod network;
pub mod volume;

#[cfg(target_os = "windows")]
mod windows;

use std::rc::Rc;

pub struct IntegrationRegistry {
    pub desktop: Rc<dyn desktop::DesktopIntegration>,
    pub audio: Rc<dyn audio::AudioIntegration>,
    pub brightness: Rc<dyn brightness::BrightnessIntegration>,
    pub battery: Rc<dyn battery::BatteryIntegration>,
    pub volume: Rc<dyn volume::VolumeIntegration>,
    pub network: Rc<dyn network::NetworkIntegration>,
    pub bluetooth: Rc<dyn bluetooth::BluetoothIntegration>,
}

pub type Registry = IntegrationRegistry;

impl IntegrationRegistry {
    pub fn detect() -> Self {
        Self {
            #[cfg(not(target_os = "windows"))]
            desktop: desktop::KWinDbus::detect()
                .map(|item| Rc::new(item) as Rc<dyn desktop::DesktopIntegration>)
                .unwrap_or_else(|| Rc::new(desktop::Fallback)),
            #[cfg(target_os = "windows")]
            desktop: desktop::WindowsDesktop::detect()
                .map(|item| Rc::new(item) as Rc<dyn desktop::DesktopIntegration>)
                .unwrap_or_else(|| Rc::new(desktop::Fallback)),
            #[cfg(not(target_os = "windows"))]
            audio: audio::Mpris::detect()
                .map(|item| Rc::new(item) as Rc<dyn audio::AudioIntegration>)
                .unwrap_or_else(|| Rc::new(audio::Fallback)),
            #[cfg(target_os = "windows")]
            audio: audio::WindowsMedia::detect()
                .map(|item| Rc::new(item) as Rc<dyn audio::AudioIntegration>)
                .unwrap_or_else(|| Rc::new(audio::Fallback)),
            #[cfg(not(target_os = "windows"))]
            brightness: brightness::BrightnessCtl::detect()
                .map(|item| Rc::new(item) as Rc<dyn brightness::BrightnessIntegration>)
                .unwrap_or_else(|| Rc::new(brightness::Fallback)),
            #[cfg(target_os = "windows")]
            brightness: brightness::WindowsBrightness::detect()
                .map(|item| Rc::new(item) as Rc<dyn brightness::BrightnessIntegration>)
                .unwrap_or_else(|| Rc::new(brightness::Fallback)),
            #[cfg(not(target_os = "windows"))]
            battery: battery::UPower::detect()
                .map(|item| Rc::new(item) as Rc<dyn battery::BatteryIntegration>)
                .unwrap_or_else(|| Rc::new(battery::Fallback)),
            #[cfg(target_os = "windows")]
            battery: battery::WindowsBattery::detect()
                .map(|item| Rc::new(item) as Rc<dyn battery::BatteryIntegration>)
                .unwrap_or_else(|| Rc::new(battery::Fallback)),
            #[cfg(not(target_os = "windows"))]
            volume: volume::PipeWire::detect()
                .map(|item| Rc::new(item) as Rc<dyn volume::VolumeIntegration>)
                .unwrap_or_else(|| Rc::new(volume::Fallback)),
            #[cfg(target_os = "windows")]
            volume: volume::WindowsVolume::detect()
                .map(|item| Rc::new(item) as Rc<dyn volume::VolumeIntegration>)
                .unwrap_or_else(|| Rc::new(volume::Fallback)),
            #[cfg(not(target_os = "windows"))]
            network: network::NetworkManager::detect()
                .map(|item| Rc::new(item) as Rc<dyn network::NetworkIntegration>)
                .unwrap_or_else(|| Rc::new(network::Fallback)),
            #[cfg(target_os = "windows")]
            network: network::WindowsNetwork::detect()
                .map(|item| Rc::new(item) as Rc<dyn network::NetworkIntegration>)
                .unwrap_or_else(|| Rc::new(network::Fallback)),
            #[cfg(not(target_os = "windows"))]
            bluetooth: bluetooth::BluetoothCtl::detect()
                .map(|item| Rc::new(item) as Rc<dyn bluetooth::BluetoothIntegration>)
                .unwrap_or_else(|| Rc::new(bluetooth::Fallback)),
            #[cfg(target_os = "windows")]
            bluetooth: bluetooth::WindowsBluetooth::detect()
                .map(|item| Rc::new(item) as Rc<dyn bluetooth::BluetoothIntegration>)
                .unwrap_or_else(|| Rc::new(bluetooth::Fallback)),
        }
    }
}
