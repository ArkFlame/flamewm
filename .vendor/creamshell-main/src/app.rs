use crate::bar::{
    build_dock, warn_unknown_widgets, BarActions, CreamTheme, SystemStatus, DOCK_HEIGHT,
};
use crate::config::{BarPosition, ShellConfig};
use crate::desktop;
use crate::integrations::{audio::AudioIntegration, Registry};
use crate::panels::{app_drawer, clock, control_center, current_playing, weather};
use crate::platform::Playback;
use chrono::Local;
use creamui_core::{BoxedWidget, Point, Rect, Size};
use creamui_reactive::Signal;
use creamui_render::{AppBuilder, AppHandle, PopupOptions, WindowHandle, WindowOptions};
use creamui_theme::Color;
use std::rc::Rc;
use std::{cell::RefCell, process::Child, process::Command, time::Duration};

pub fn run() {
    let config = Rc::new(ShellConfig::load());
    warn_unknown_widgets(&config.bar.layout);
    let bar_role = match config.bar.position {
        BarPosition::Top => creamui_render::platform::WindowRole::TopPanel,
        BarPosition::Bottom => creamui_render::platform::WindowRole::BottomPanel,
    };
    let popup_opens_below = matches!(config.bar.position, BarPosition::Top);

    let integrations = Registry::detect();
    let applications = app_drawer::AppCatalog::new();
    let drawer_state = app_drawer::DrawerState::default();
    let desktop_state = desktop::DesktopState::default();
    let initial_windows = integrations.desktop.windows();
    let windows = Signal::new(initial_windows);
    let launcher_open = Signal::new(false);
    let active_window = Signal::new(None);
    let ready_backend = integrations.desktop.clone();
    let playback_backend = integrations.audio.clone();
    let playback_state = Signal::new(playback_backend.playback());
    let bar_window: Rc<RefCell<Option<WindowHandle>>> = Rc::new(RefCell::new(None));
    let bar_playback_backend = integrations.audio.clone();
    let bar_windows_backend = integrations.desktop.clone();
    let control_network = integrations.network.clone();
    let control_brightness = integrations.brightness.clone();
    let control_battery = integrations.battery.clone();
    let control_volume = integrations.volume.clone();
    let control_bluetooth = integrations.bluetooth.clone();
    let brightness_level = Signal::new(integrations.brightness.level());
    let volume_level = Signal::new(integrations.volume.level());
    let wifi_enabled = Signal::new(integrations.network.enabled());
    let bluetooth_powered = Signal::new(integrations.bluetooth.powered());
    let clock_format = Rc::new(config.widgets.clock.format.clone());
    let clock_text = Signal::new(Local::now().format(&clock_format).to_string());
    let status_network = integrations.network.clone();
    let status_battery = integrations.battery.clone();
    let status_bluetooth = integrations.bluetooth.clone();
    let status_volume = integrations.volume.clone();
    let refresh_backend = integrations.desktop.clone();
    let refresh_windows = windows.clone();
    let refresh = Rc::new(move || {
        let next = refresh_backend.windows();
        refresh_windows.set(next);
    });
    let activate_backend = integrations.desktop.clone();
    let activate = Rc::new(move |id: String| activate_backend.activate_window(&id));

    AppBuilder::new()
        .keep_running()
        .on_started(move |app| {
            let desktop_state = desktop_state.clone();
            desktop_state.start_loading(&app);
            let drag_overlay_state = desktop_state.clone();
            app.append_window(
                WindowOptions {
                    title: "CreamShell Desktop".into(),
                    width: 1280,
                    height: 720,
                    decorations: false,
                    resizable: false,
                    transparent: false,
                    role: creamui_render::platform::WindowRole::Desktop,
                    theme: CreamTheme::theme(),
                    ..Default::default()
                },
                desktop::BACKGROUND,
                |_| {},
                move |viewport| desktop::build(viewport, desktop_state.clone()),
            );
            app.append_window(
                WindowOptions {
                    title: "CreamShell Drag Overlay".into(),
                    width: 1280,
                    height: 720,
                    decorations: false,
                    resizable: false,
                    transparent: true,
                    role: creamui_render::platform::WindowRole::Overlay,
                    theme: CreamTheme::theme(),
                    ..Default::default()
                },
                Color::rgba(0, 0, 0, 0),
                |_| {},
                move |viewport| desktop::build_drag_overlay(viewport, drag_overlay_state.clone()),
            );
            schedule_playback_refresh(
                app.clone(),
                playback_backend.clone(),
                playback_state.clone(),
            );
            schedule_clock_refresh(app.clone(), clock_text.clone(), clock_format.clone());
            applications.start_loading(&app);
            for delay in [Duration::from_secs(1), Duration::from_secs(3)] {
                let backend = bar_windows_backend.clone();
                let target = windows.clone();
                app.spawn_background(
                    move || std::thread::sleep(delay),
                    move |_| target.set(backend.windows()),
                );
            }
            let weather_app = app.clone();
            let weather_handle: Rc<RefCell<Option<WindowHandle>>> = Rc::new(RefCell::new(None));
            let weather_bar = bar_window.clone();
            let open_weather = Rc::new(move |anchor: Point| {
                if let Some(handle) = weather_handle.borrow_mut().take() {
                    if handle.is_open() {
                        handle.close();
                        return;
                    }
                }
                let weather_handle = weather_handle.clone();
                let bar_for_popup = weather_bar.clone();
                let Some(popup) = popup_for(&bar_for_popup, anchor, popup_opens_below) else {
                    return;
                };
                weather_app.append_popup(
                    popup_options("CreamShell Weather", weather::WIDTH, weather::HEIGHT),
                    popup,
                    Color::rgba(0, 0, 0, 0),
                    move |window| {
                        *weather_handle.borrow_mut() = Some(window.clone());
                        window.set_always_on_top(true);
                        close_on_focus_lost(&window);
                    },
                    weather::build,
                );
            });
            let clock_app = app.clone();
            let clock_handle: Rc<RefCell<Option<WindowHandle>>> = Rc::new(RefCell::new(None));
            let clock_bar = bar_window.clone();
            let open_clock = Rc::new(move |anchor: Point| {
                if let Some(handle) = clock_handle.borrow_mut().take() {
                    if handle.is_open() {
                        handle.close();
                        return;
                    }
                }
                let clock_handle = clock_handle.clone();
                let bar_for_popup = clock_bar.clone();
                let Some(popup) = popup_for(&bar_for_popup, anchor, popup_opens_below) else {
                    return;
                };
                clock_app.append_popup(
                    popup_options("CreamShell Clock", clock::WIDTH, clock::HEIGHT),
                    popup,
                    Color::rgba(0, 0, 0, 0),
                    move |window| {
                        *clock_handle.borrow_mut() = Some(window.clone());
                        window.set_always_on_top(true);
                        close_on_focus_lost(&window);
                    },
                    clock::build,
                );
            });
            let music_app = app.clone();
            let music_backend = playback_backend.clone();
            let music_handle: Rc<RefCell<Option<WindowHandle>>> = Rc::new(RefCell::new(None));
            let music_bar = bar_window.clone();
            let open_current_playing = Rc::new(move |anchor: Point| {
                if let Some(handle) = music_handle.borrow_mut().take() {
                    if handle.is_open() {
                        handle.close();
                        return;
                    }
                }
                let music_handle = music_handle.clone();
                let backend = music_backend.clone();
                let playback_state = playback_state.clone();
                let bar_for_popup = music_bar.clone();
                let Some(popup) = popup_for(&bar_for_popup, anchor, popup_opens_below) else {
                    return;
                };
                music_app.append_popup(
                    popup_options(
                        "CreamShell Current Playing",
                        current_playing::WIDTH,
                        current_playing::HEIGHT,
                    ),
                    popup,
                    Color::rgba(0, 0, 0, 0),
                    move |window| {
                        *music_handle.borrow_mut() = Some(window.clone());
                        window.set_always_on_top(true);
                        close_on_focus_lost(&window);
                    },
                    move |size| {
                        current_playing::build(
                            size,
                            playback_state.get(),
                            {
                                let backend = backend.clone();
                                Rc::new(move || backend.toggle_playback())
                            },
                            {
                                let backend = backend.clone();
                                Rc::new(move || backend.previous())
                            },
                            {
                                let backend = backend.clone();
                                Rc::new(move || backend.next())
                            },
                            {
                                let backend = backend.clone();
                                let playback_state = playback_state.clone();
                                Rc::new(move |progress| {
                                    if let Some(playback) = playback_state.peek() {
                                        if let Some(length) = parse_playback_time(&playback.length)
                                        {
                                            backend.seek(progress * length);
                                        }
                                    }
                                })
                            },
                        )
                    },
                );
            });
            let drawer_app = app.clone();
            let drawer_catalog = applications.clone();
            let drawer_state = drawer_state.clone();
            let drawer_handle: Rc<RefCell<Option<WindowHandle>>> = Rc::new(RefCell::new(None));
            let drawer_bar = bar_window.clone();
            let open_app_drawer = Rc::new(move |anchor: Point| {
                if let Some(handle) = drawer_handle.borrow_mut().take() {
                    if handle.is_open() {
                        handle.close();
                        return;
                    }
                }
                let drawer_handle = drawer_handle.clone();
                let close_handle = drawer_handle.clone();
                let close_drawer: Rc<dyn Fn()> = Rc::new(move || {
                    if let Some(window) = close_handle.borrow_mut().take() {
                        window.close();
                    }
                });
                let catalog = drawer_catalog.clone();
                let state = drawer_state.clone();
                let bar_for_popup = drawer_bar.clone();
                let Some(popup) = popup_for(&bar_for_popup, anchor, popup_opens_below) else {
                    return;
                };
                drawer_app.append_popup(
                    popup_options(
                        "CreamShell App Drawer",
                        app_drawer::WIDTH,
                        app_drawer::HEIGHT,
                    ),
                    popup,
                    Color::rgba(0, 0, 0, 0),
                    move |window| {
                        *drawer_handle.borrow_mut() = Some(window.clone());
                        window.set_always_on_top(true);
                        close_on_focus_lost(&window);
                    },
                    move |size| {
                        app_drawer::build(
                            size,
                            catalog.clone(),
                            state.clone(),
                            close_drawer.clone(),
                        )
                    },
                );
            });
            let awake_process: Rc<RefCell<Option<Child>>> = Rc::new(RefCell::new(None));
            let awake = Signal::new(false);
            let keep_awake = {
                let process = awake_process.clone();
                let awake = awake.clone();
                Rc::new(move || {
                    let mut process = process.borrow_mut();
                    if let Some(mut child) = process.take() {
                        let _ = child.kill();
                        awake.set(false);
                    } else if let Ok(child) = Command::new("systemd-inhibit")
                        .args([
                            "--what=idle:sleep",
                            "--why=CreamShell",
                            "--mode=block",
                            "sleep",
                            "infinity",
                        ])
                        .spawn()
                    {
                        *process = Some(child);
                        awake.set(true);
                    }
                })
            };
            let control_app = app.clone();
            let control_handle: Rc<RefCell<Option<WindowHandle>>> = Rc::new(RefCell::new(None));
            let dock_volume = volume_level.clone();
            let control_bar = bar_window.clone();
            let control_tray_config = config.clone();
            let open_control_center = Rc::new(move |anchor: Point| {
                if let Some(handle) = control_handle.borrow_mut().take() {
                    if handle.is_open() {
                        handle.close();
                        return;
                    }
                }
                let control_handle = control_handle.clone();
                let toggle = keep_awake.clone();
                let awake = awake.clone();
                let network = control_network.clone();
                let brightness = control_brightness.clone();
                let battery = control_battery.clone();
                let volume = control_volume.clone();
                let bluetooth = control_bluetooth.clone();
                let brightness_level = brightness_level.clone();
                let volume_level = volume_level.clone();
                let wifi_enabled = wifi_enabled.clone();
                let bluetooth_powered = bluetooth_powered.clone();
                let control_tray_config = control_tray_config.clone();
                brightness_level.set(brightness.level());
                volume_level.set(volume.level());
                wifi_enabled.set(network.enabled());
                bluetooth_powered.set(bluetooth.powered());
                let bar_for_popup = control_bar.clone();
                let Some(popup) = popup_for(&bar_for_popup, anchor, popup_opens_below) else {
                    return;
                };
                control_app.append_popup(
                    popup_options(
                        "CreamShell Control Center",
                        control_center::WIDTH,
                        control_center::HEIGHT,
                    ),
                    popup,
                    Color::rgba(0, 0, 0, 0),
                    move |window| {
                        *control_handle.borrow_mut() = Some(window.clone());
                        window.set_always_on_top(true);
                        close_on_focus_lost(&window);
                    },
                    move |size| {
                        control_center::build_with_integrations(
                            size,
                            network.clone(),
                            brightness.clone(),
                            battery.clone(),
                            volume.clone(),
                            bluetooth.clone(),
                            toggle.clone(),
                            awake.get(),
                            brightness_level.clone(),
                            volume_level.clone(),
                            wifi_enabled.clone(),
                            bluetooth_powered.clone(),
                            &control_tray_config.tray,
                        )
                    },
                );
            });

            app.append_window(
                WindowOptions {
                    title: "CreamShell".into(),
                    width: 800,
                    height: DOCK_HEIGHT,
                    decorations: false,
                    resizable: false,
                    transparent: true,
                    role: bar_role,
                    theme: CreamTheme::theme(),
                    ..Default::default()
                },
                Color::rgba(0, 0, 0, 0),
                {
                    let bar_window = bar_window.clone();
                    move |window: WindowHandle| {
                        ready_backend.prepare_window(&window);
                        *bar_window.borrow_mut() = Some(window);
                    }
                },
                {
                    let config = config.clone();
                    move |viewport: Size| -> BoxedWidget {
                        let status = SystemStatus {
                            network_connected: status_network.connected(),
                            network_strength: status_network.strength(),
                            battery_percentage: status_battery.percentage(),
                            battery_charging: status_battery.charging(),
                            bluetooth_powered: status_bluetooth.powered(),
                            bluetooth_connected: status_bluetooth.connected(),
                            volume: dock_volume.get(),
                            volume_muted: status_volume.muted(),
                        };
                        build_dock(
                            viewport,
                            windows.get(),
                            launcher_open.clone(),
                            active_window.clone(),
                            status,
                            clock_text.clone(),
                            BarActions {
                                refresh_windows: refresh.clone(),
                                activate_window: activate.clone(),
                                current_playback: {
                                    let backend = bar_playback_backend.clone();
                                    Rc::new(move || backend.playback())
                                },
                                toggle_playback: {
                                    let backend = bar_playback_backend.clone();
                                    Rc::new(move || backend.toggle_playback())
                                },
                                previous_playback: {
                                    let backend = bar_playback_backend.clone();
                                    Rc::new(move || backend.previous())
                                },
                                next_playback: {
                                    let backend = bar_playback_backend.clone();
                                    Rc::new(move || backend.next())
                                },
                                open_weather: open_weather.clone(),
                                open_clock: open_clock.clone(),
                                open_current_playing: open_current_playing.clone(),
                                open_app_drawer: open_app_drawer.clone(),
                                open_control_center: open_control_center.clone(),
                            },
                            &config,
                        )
                    }
                },
            );
        })
        .run();
}

fn schedule_playback_refresh(
    app: AppHandle,
    backend: Rc<dyn AudioIntegration>,
    state: Signal<Option<Playback>>,
) {
    let next_app = app.clone();
    let next_backend = backend.clone();
    let next_state = state.clone();
    app.spawn_background(
        || std::thread::sleep(Duration::from_millis(250)),
        move |_| {
            state.set(backend.playback());
            schedule_playback_refresh(next_app, next_backend, next_state);
        },
    );
}

fn schedule_clock_refresh(app: AppHandle, state: Signal<String>, format: Rc<String>) {
    let next_app = app.clone();
    let next_state = state.clone();
    let next_format = format.clone();
    app.spawn_background(
        || std::thread::sleep(Duration::from_secs(1)),
        move |_| {
            let formatted = Local::now().format(&format).to_string();
            if state.peek() != formatted {
                state.set(formatted);
            }
            schedule_clock_refresh(next_app, next_state, next_format);
        },
    );
}

fn parse_playback_time(value: &str) -> Option<f64> {
    let (minutes, seconds) = value.split_once(':')?;
    Some(minutes.parse::<f64>().ok()? * 60.0 + seconds.parse::<f64>().ok()?)
}

fn close_on_focus_lost(window: &WindowHandle) {
    let handle = window.clone();
    window.on_focus_lost(move || handle.close());
}

fn popup_for(
    bar: &Rc<RefCell<Option<WindowHandle>>>,
    anchor: Point,
    opens_below: bool,
) -> Option<PopupOptions> {
    let bar_ref = bar.borrow();
    bar_ref.as_ref().map(|bar| {
        let anchor_rect = if opens_below {
            Rect {
                x: anchor.x,
                y: anchor.y,
                width: 1.0,
                height: (DOCK_HEIGHT as f32 - anchor.y).max(1.0),
            }
        } else {
            Rect {
                x: anchor.x,
                y: 0.0,
                width: 1.0,
                height: (anchor.y + 1.0).max(1.0),
            }
        };
        let popup = PopupOptions::new(bar.clone(), anchor_rect);
        if opens_below {
            popup.below()
        } else {
            popup.above()
        }
    })
}

fn popup_options(title: &str, width: u32, height: u32) -> WindowOptions {
    WindowOptions {
        title: title.into(),
        width,
        height,
        decorations: false,
        resizable: false,
        transparent: true,
        theme: CreamTheme::theme(),
        ..Default::default()
    }
}
