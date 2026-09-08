use super::*;
use crate::xlib::*;

pub fn run(document: RuntimeDocument, config: X11Config) -> Result<(), String> {
    run_with_handler(document, config, |action| {
        println!("FLAMEWM_RENDER_ACTION {action}")
    })
}

/// Return default X screen root dimensions.
pub fn root_geometry() -> Result<(u32, u32), String> {
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed; DISPLAY is unset or unreachable".to_string());
        }
        let screen = XDefaultScreen(display);
        let width = XDisplayWidth(display, screen).max(0) as u32;
        let height = XDisplayHeight(display, screen).max(0) as u32;
        XCloseDisplay(display);
        if width == 0 || height == 0 {
            return Err("X11 root geometry is empty".to_string());
        }
        Ok((width, height))
    }
}

pub fn run_with_controller_events<F>(
    document: RuntimeDocument,
    config: X11Config,
    mut on_event: F,
) -> Result<(), String>
where
    F: FnMut(&ControllerEvent, &mut RuntimeDocument) -> Result<(), String>,
{
    run_with_controller(document, config, move |event, document| {
        on_event(&ControllerEvent::Action(event.clone()), document)
    })
}

pub fn run_with_controller_events_with_reactor<F, R>(
    mut document: RuntimeDocument,
    config: X11Config,
    reactor: &mut Reactor,
    mut on_event: F,
    mut on_reactor: R,
) -> Result<(), String>
where
    F: FnMut(&ControllerEvent, &mut RuntimeDocument) -> Result<(), String>,
    R: FnMut(&mut RuntimeDocument) -> Result<(), String>,
{
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed; DISPLAY is unset or unreachable".to_string());
        }
        let mut app = match X11App::new(display, &config) {
            Ok(app) => app,
            Err(error) => {
                XCloseDisplay(display);
                return Err(error);
            }
        };
        let x_fd = XConnectionNumber(display);
        let token = reactor
            .register_raw_fd_with_action(x_fd, calloop::Interest::READ, |_, _| FdAction::Continue)
            .map_err(|error| error.to_string())?;
        let result = app.event_loop_with_reactor(
            &mut document,
            &mut |event, document| on_event(&ControllerEvent::Action(event.clone()), document),
            reactor,
            &mut on_reactor,
        );
        reactor.remove(token).map_err(|error| error.to_string())?;
        drop(app);
        XCloseDisplay(display);
        result
    }
}

pub fn run_with_handler<F>(
    document: RuntimeDocument,
    config: X11Config,
    mut on_action: F,
) -> Result<(), String>
where
    F: FnMut(&str),
{
    run_with_controller(document, config, move |event, _document| {
        if event.phase == ActionPhase::Release && event.inside {
            on_action(&event.action);
        }
        Ok(())
    })
}

pub fn run_with_controller<F>(
    mut document: RuntimeDocument,
    config: X11Config,
    mut on_action: F,
) -> Result<(), String>
where
    F: FnMut(&ActionEvent, &mut RuntimeDocument) -> Result<(), String>,
{
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed; DISPLAY is unset or unreachable".to_string());
        }
        let mut app = match X11App::new(display, &config) {
            Ok(app) => app,
            Err(error) => {
                XCloseDisplay(display);
                return Err(error);
            }
        };
        let result = app.event_loop(&mut document, &mut on_action);
        drop(app);
        XCloseDisplay(display);
        result
    }
}
