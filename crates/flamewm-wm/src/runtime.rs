use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use flamewm_control_dbus::ControlServer;
use flamewm_platform::host::PlatformHost;
use flamewm_reactor::Reactor;
use flamewm_x11_desktop::X11Desktop;

use crate::atoms::AnyError;
use crate::wm::{WmConfig, run_with_hook};

pub fn run(config: WmConfig) -> Result<(), AnyError> {
    let engine = X11Desktop::connect()?;
    let settings_path = settings_path();
    let mut host = PlatformHost::new(engine, settings_path);
    let control = ControlServer::connect_session()?;
    let mut reactor = Reactor::new()?;
    let started = Instant::now();
    let mut host_started = false;

    run_with_hook(config, move || {
        if !host_started {
            host.start()?;
            host_started = true;
        }
        // BusPump owns libdbus' connection. Its API exposes a watch FD but not an owned
        // AsFd source, so it cannot safely be registered in Reactor yet.
        reactor.dispatch(Some(Duration::from_millis(16)))?;
        let now_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        control.on_ready(&mut host, now_ms)?;
        Ok(())
    })
}

fn settings_path() -> PathBuf {
    env::var_os("FLAMEWM_SETTINGS_PATH")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("FLAMEWM_CONFIG_DIR").map(|path| PathBuf::from(path).join("settings.toml"))
        })
        .unwrap_or_else(|| PathBuf::from("settings.toml"))
}
