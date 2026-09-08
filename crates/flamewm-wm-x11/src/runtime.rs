use std::cell::RefCell;
use std::env;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

use calloop::{Interest, RegistrationToken};

use crate::X11Desktop;
use flamewm_api::ports::FdEvents;
use flamewm_api::system::{AudioSnapshot, SystemAction};
use flamewm_applications::ApplicationCatalog;
use flamewm_control_dbus::ControlServer;
use flamewm_control_wire::ControlSignal;
use flamewm_integrations_linux::{
    LinuxIntegrationRuntime, LinuxIntegrationSource, pulse::PulseAdapter,
};
use flamewm_platform::host::PlatformHost;
use flamewm_reactor::{FdAction, Reactor};

use crate::atoms::AnyError;
use crate::wm::{WmConfig, run_with_hook};

pub fn run(config: WmConfig) -> Result<(), AnyError> {
    let settings_path = settings_path();
    let mut host = None;
    let control = ControlServer::connect_session()?;
    let mut reactor = Reactor::new()?;
    let integrations = Rc::new(RefCell::new(LinuxIntegrationRuntime::connect()?));
    if let Err(error) = integrations.borrow_mut().start() {
        integrations.borrow_mut().stop();
        return Err(error.into());
    }
    let provider_error = Rc::new(RefCell::new(None));
    let mut network_registration = match register_provider(
        &mut reactor,
        &integrations,
        &provider_error,
        LinuxIntegrationSource::NetworkManager,
    ) {
        Ok(registration) => registration,
        Err(error) => {
            integrations.borrow_mut().stop();
            return Err(error);
        }
    };
    let mut mpris_registration = match register_provider(
        &mut reactor,
        &integrations,
        &provider_error,
        LinuxIntegrationSource::Mpris,
    ) {
        Ok(registration) => registration,
        Err(error) => {
            integrations.borrow_mut().stop();
            return Err(error);
        }
    };
    let (audio_sender, audio_receiver) = mpsc::sync_channel::<AudioSnapshot>(32);
    let (audio_reader, mut audio_writer) = match UnixStream::pair() {
        Ok(pair) => pair,
        Err(error) => {
            integrations.borrow_mut().stop();
            return Err(error.into());
        }
    };
    if let Err(error) = audio_reader.set_nonblocking(true) {
        integrations.borrow_mut().stop();
        return Err(error.into());
    }
    if let Err(error) = audio_writer.set_nonblocking(true) {
        integrations.borrow_mut().stop();
        return Err(error.into());
    }
    let pulse = match PulseAdapter::connect(move |snapshot| {
        if audio_sender.try_send(snapshot).is_ok() {
            let _ = audio_writer.write(&[1]);
        }
    }) {
        Ok(pulse) => Rc::new(pulse),
        Err(error) => {
            integrations.borrow_mut().stop();
            return Err(error.into());
        }
    };
    let audio_updates = Rc::new(RefCell::new(Vec::<AudioSnapshot>::new()));
    let audio_updates_for_callback = Rc::clone(&audio_updates);
    let _audio_registration = match reactor.register_fd_with_source_action(
        audio_reader,
        Interest::READ,
        move |_, source, _| {
            let mut wake_buffer = [0_u8; 128];
            loop {
                match source.read(&mut wake_buffer) {
                    Ok(0) => return FdAction::Remove,
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(_) => return FdAction::Remove,
                }
            }
            let mut updates = audio_updates_for_callback.borrow_mut();
            loop {
                match audio_receiver.try_recv() {
                    Ok(snapshot) => updates.push(snapshot),
                    Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
                }
            }
            FdAction::Continue
        },
    ) {
        Ok(token) => token,
        Err(error) => {
            drop(pulse);
            integrations.borrow_mut().stop();
            return Err(error.into());
        }
    };
    let started = Instant::now();
    let mut host_started = false;
    let integrations_for_hook = Rc::clone(&integrations);
    let provider_error_for_hook = Rc::clone(&provider_error);
    let pulse_for_hook = Rc::clone(&pulse);

    let result = run_with_hook(config, move |conn, screen| {
        if host.is_none() {
            let mut new_host = PlatformHost::new(
                X11Desktop::from_connection(conn.clone(), screen)?,
                settings_path.clone(),
            );
            let catalog = ApplicationCatalog::discover()?;
            let _ = new_host.replace_applications(catalog.all());
            let integrations = Rc::clone(&integrations_for_hook);
            let pulse = Rc::clone(&pulse_for_hook);
            new_host.set_system_action_handler(Box::new(move |action, snapshot| match action {
                SystemAction::SetVolume(action) => pulse.set_volume(action),
                SystemAction::SetMute(action) => pulse.set_mute(action),
                SystemAction::SetWifiEnabled(_)
                | SystemAction::ConnectKnown { .. }
                | SystemAction::ConnectWifi { .. }
                | SystemAction::SubmitNetworkSecret { .. }
                | SystemAction::CancelNetworkSecret { .. }
                | SystemAction::Disconnect
                | SystemAction::Scan
                | SystemAction::Play { .. }
                | SystemAction::Pause { .. }
                | SystemAction::PlayPause { .. }
                | SystemAction::Next { .. }
                | SystemAction::Previous { .. } => integrations.borrow_mut().perform_action(
                    action,
                    &snapshot.network,
                    &snapshot.media,
                ),
            }));
            host = Some(new_host);
        }
        let host = host.as_mut().expect("host initialized");
        if !host_started {
            host.start()?;
            host_started = true;
        }
        reactor.dispatch(Some(Duration::from_millis(16)))?;
        {
            let mut audio_updates = audio_updates.borrow_mut();
            for snapshot in audio_updates.drain(..) {
                if host.system_mut().update_audio(snapshot) {
                    control.emit_signal(&ControlSignal::SystemChanged {
                        revision: host.system_snapshot().revision,
                    })?;
                }
            }
        }
        if let Some(error) = provider_error_for_hook.borrow_mut().take() {
            return Err(error.into());
        }
        {
            let mut integrations = integrations_for_hook.borrow_mut();
            let _ = integrations.reconcile()?;
            let network = integrations.network_snapshot();
            let media = integrations.media_snapshot();
            if host.system_mut().update_network(network) {
                control.emit_signal(&ControlSignal::SystemChanged {
                    revision: host.system_snapshot().revision,
                })?;
            }
            if host.system_mut().update_media(media) {
                control.emit_signal(&ControlSignal::SystemChanged {
                    revision: host.system_snapshot().revision,
                })?;
            }
        }
        refresh_provider(
            &mut reactor,
            &mut network_registration,
            &integrations_for_hook,
            &provider_error_for_hook,
        )?;
        refresh_provider(
            &mut reactor,
            &mut mpris_registration,
            &integrations_for_hook,
            &provider_error_for_hook,
        )?;
        let now_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        control.on_ready(host, now_ms)?;
        Ok(())
    });
    drop(pulse);
    integrations.borrow_mut().stop();
    result
}

struct ProviderRegistration {
    source: LinuxIntegrationSource,
    fd: i32,
    events: FdEvents,
    token: RegistrationToken,
}

fn register_provider(
    reactor: &mut Reactor,
    integrations: &Rc<RefCell<LinuxIntegrationRuntime>>,
    provider_error: &Rc<RefCell<Option<flamewm_api::FlameError>>>,
    source: LinuxIntegrationSource,
) -> Result<ProviderRegistration, AnyError> {
    let watch = integrations.borrow().watch(source);
    let integrations = Rc::clone(integrations);
    let provider_error = Rc::clone(provider_error);
    let token = reactor.register_raw_fd_with_action(
        watch.fd,
        interest_for(watch.events),
        move |_, _| {
            if let Err(error) = integrations.borrow_mut().on_ready(source) {
                let mut provider_error = provider_error.borrow_mut();
                if provider_error.is_none() {
                    *provider_error = Some(error);
                }
            }
            FdAction::Continue
        },
    )?;
    Ok(ProviderRegistration {
        source,
        fd: watch.fd,
        events: watch.events,
        token,
    })
}

fn refresh_provider(
    reactor: &mut Reactor,
    registration: &mut ProviderRegistration,
    integrations: &Rc<RefCell<LinuxIntegrationRuntime>>,
    provider_error: &Rc<RefCell<Option<flamewm_api::FlameError>>>,
) -> Result<(), AnyError> {
    let watch = integrations.borrow().watch(registration.source);
    if watch.fd == registration.fd && watch.events == registration.events {
        return Ok(());
    }
    reactor.remove(registration.token)?;
    *registration = register_provider(reactor, integrations, provider_error, registration.source)?;
    Ok(())
}

fn interest_for(events: FdEvents) -> Interest {
    match (
        events.contains(FdEvents::READABLE),
        events.contains(FdEvents::WRITABLE),
    ) {
        (true, true) => Interest::BOTH,
        (false, true) => Interest::WRITE,
        _ => Interest::READ,
    }
}

fn settings_path() -> PathBuf {
    env::var_os("FLAMEWM_SETTINGS_PATH")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("FLAMEWM_CONFIG_DIR").map(|path| PathBuf::from(path).join("settings.toml"))
        })
        .unwrap_or_else(|| PathBuf::from("settings.toml"))
}
