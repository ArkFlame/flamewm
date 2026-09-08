use flamewm_ui_x11::{UiControllerEvent, UiWindowRole, run_with_controller_events_role};

use crate::SettingsApplication;
use crate::control::SettingsControl;
use crate::view;

pub fn run() -> Result<(), String> {
    let mut application = SettingsApplication::new(
        SettingsControl::connect_session().map_err(|error| error.to_string())?,
    );
    let mut document = view::document()?;
    application.refresh(&mut document)?;
    run_with_controller_events_role(
        document,
        super::normal_window_config(720, 480),
        UiWindowRole::Normal,
        move |event, document| {
            let UiControllerEvent::Action(action) = event;
            application.handle_action(action, document)?;
            Ok(())
        },
    )
}
