use std::cell::RefCell;
use std::env;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Instant;

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

struct WmBuilderFields {
    discover_counter: &'static flamewm_profiler::CounterPoint,
}

fn wm_builder_fields() -> &'static WmBuilderFields {
    use std::sync::OnceLock;
    static FIELDS: OnceLock<WmBuilderFields> = OnceLock::new();
    FIELDS.get_or_init(|| WmBuilderFields {
        discover_counter: Box::leak(Box::new(flamewm_profiler::CounterPoint::new(
            "wm.catalog.discover.count",
        ))),
    })
}

/// Discover-once composition root: single `ApplicationCatalog::discover`
/// per WM process, counted under `wm.catalog.discover.count`. The returned
/// `Arc` is injected into the launch adapter, `DecorationManager`, and the
/// host snapshot; launch/repaint paths never rediscover.
fn discover_catalog_once() -> Result<std::sync::Arc<ApplicationCatalog>, AnyError> {
    wm_builder_fields().discover_counter.increment();
    Ok(std::sync::Arc::new(ApplicationCatalog::discover()?))
}

pub fn run(config: WmConfig) -> Result<(), AnyError> {
    flamewm_profiler::init_process("flamewm-wm");
    let settings_path = settings_path();
    let mut host = None;
    let control = ControlServer::connect_session()?;
    let reactor = Rc::new(RefCell::new(Reactor::new()?));
    let integrations = Rc::new(RefCell::new(LinuxIntegrationRuntime::connect()?));
    if let Err(error) = integrations.borrow_mut().start() {
        integrations.borrow_mut().stop();
        return Err(error.into());
    }
    let provider_error = Rc::new(RefCell::new(None));
    let mut network_registration = match register_provider(
        &reactor,
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
        &reactor,
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
    let _audio_registration = match reactor.borrow_mut().register_fd_with_source_action(
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
    // Profiler report timer on the reactor; interval from
    // `FLAMEWM_PROFILE_INTERVAL` (floored at 10s, default 60s), never
    // hardcoded here. No timer thread.
    let _profiler_registration = reactor
        .borrow_mut()
        .register_timer(profile_report_interval(), true, || {
            if flamewm_profiler::enabled() {
                let report = flamewm_profiler::report_window();
                if !report.is_empty() {
                    eprintln!("{report}");
                }
            }
        })
        .map_err(|error| {
            drop(pulse.clone());
            integrations.borrow_mut().stop();
            AnyError::from(io::Error::new(io::ErrorKind::Other, error.to_string()))
        })?;
    let _startup_guard = flamewm_profiler::start("wm.startup.catalog");
    let catalog = discover_catalog_once()?;
    drop(_startup_guard);
    let catalog_for_hook = std::sync::Arc::clone(&catalog);
    let started = Instant::now();
    let mut host_started = false;
    let integrations_for_hook = Rc::clone(&integrations);
    let provider_error_for_hook = Rc::clone(&provider_error);
    let pulse_for_hook = Rc::clone(&pulse);
    let audio_updates_for_hook = Rc::clone(&audio_updates);
    let reactor_for_hook = Rc::clone(&reactor);

    let result = run_with_hook(config, &catalog, &reactor, move |conn, screen| {
        // Scoped per-turn span: the whole-process `wm.loop` guard above has
        // been removed so loop-turn CPU is attributed per turn, not once
        // across the full process lifetime.
        let _turn_guard = flamewm_profiler::start("wm.loop");
        if host.is_none() {
            let mut new_host = PlatformHost::new(
                X11Desktop::from_connection(
                    conn.clone(),
                    screen,
                    std::sync::Arc::clone(&catalog_for_hook),
                )?,
                settings_path.clone(),
            );
            let _ = new_host.replace_applications(catalog_for_hook.all());
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
            let _host_guard = flamewm_profiler::start("wm.startup.host");
            host.start()?;
            host_started = true;
        }
        // ProviderDirty audio domain: drain only snapshots the audio fd delivered.
        {
            let mut audio_updates = audio_updates_for_hook.borrow_mut();
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
        // ProviderDirty network/media domains: reconcile event-coalesced state
        // without polling; snapshots only refresh on reported change.
        {
            let mut integrations = integrations_for_hook.borrow_mut();
            if integrations.reconcile()? {
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
        }
        refresh_provider(
            &reactor_for_hook,
            &mut network_registration,
            &integrations_for_hook,
            &provider_error_for_hook,
        )?;
        refresh_provider(
            &reactor_for_hook,
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
    reactor: &Rc<RefCell<Reactor>>,
    integrations: &Rc<RefCell<LinuxIntegrationRuntime>>,
    provider_error: &Rc<RefCell<Option<flamewm_api::FlameError>>>,
    source: LinuxIntegrationSource,
) -> Result<ProviderRegistration, AnyError> {
    let watch = integrations.borrow().watch(source);
    let integrations = Rc::clone(integrations);
    let provider_error = Rc::clone(provider_error);
    let token = reactor.borrow_mut().register_raw_fd_with_action(
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
    reactor: &Rc<RefCell<Reactor>>,
    registration: &mut ProviderRegistration,
    integrations: &Rc<RefCell<LinuxIntegrationRuntime>>,
    provider_error: &Rc<RefCell<Option<flamewm_api::FlameError>>>,
) -> Result<(), AnyError> {
    let watch = integrations.borrow().watch(registration.source);
    if watch.fd == registration.fd && watch.events == registration.events {
        return Ok(());
    }
    reactor.borrow_mut().remove(registration.token)?;
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

/// Profiler report interval shared with `flamewm-profiler`; never hardcode
/// the delay here.
fn profile_report_interval() -> std::time::Duration {
    std::time::Duration::from_secs(flamewm_profiler::profile_interval_secs())
}

fn settings_path() -> PathBuf {
    env::var_os("FLAMEWM_SETTINGS_PATH")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("FLAMEWM_CONFIG_DIR").map(|path| PathBuf::from(path).join("settings.toml"))
        })
        .unwrap_or_else(|| PathBuf::from("settings.toml"))
}
