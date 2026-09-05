use flamewm_render_x11::{ControllerEvent, X11Config, run_with_controller_events};

use crate::SettingsApplication;
use crate::control::SettingsControl;
use crate::view;

pub fn run() -> Result<(), String> {
    let mut application = SettingsApplication::new(
        SettingsControl::connect_session().map_err(|error| error.to_string())?,
    );
    let mut document = view::document()?;
    application.refresh(&mut document)?;
    run_with_controller_events(
        document,
        super::normal_window_config(1350, 641),
        move |event, document| {
            if let ControllerEvent::Action(action) = event {
                application.handle_action(action, document)?;
            }
            Ok(())
        },
    )
}
