use std::env;

use flamewm_render_core::{decode, RuntimeDocument};
use flamewm_render_x11::{run_with_controller, ActionEvent, X11Config, X11WindowRole};
use flamewm_shell::{ShellRuntime, ShellSnapshot};

const COMPILED_UI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-shell.rwr"));

fn main() {
    if let Err(error) = execute() {
        eprintln!("flamewm-shell: {error}");
        std::process::exit(1);
    }
}

fn execute() -> Result<(), String> {
    let document = RuntimeDocument::new(decode(COMPILED_UI)?)?;
    let mut shell = ShellRuntime::default();
    shell.replace_snapshot(ShellSnapshot::default());
    let mut start_open = false;
    let config = X11Config {
        width: 1350,
        height: 641,
        title: "FlameWM Shell".to_owned(),
        role: X11WindowRole::Desktop,
    };
    run_with_controller(document, config, move |event: &ActionEvent, document| {
        shell.dispatch(&event.action);
        for control in shell.drain_controls() {
            if matches!(control, flamewm_shell::ShellControl::ToggleStart) {
                start_open = !start_open;
            }
            if matches!(control, flamewm_shell::ShellControl::CloseStart) {
                start_open = false;
            }
            apply_control(document, control, start_open)?;
        }
        Ok(())
    })
}

fn apply_control(
    document: &mut RuntimeDocument,
    control: flamewm_shell::ShellControl,
    start_open: bool,
) -> Result<(), String> {
    match control {
        flamewm_shell::ShellControl::ToggleStart => document.set_visible("start-menu", start_open),
        flamewm_shell::ShellControl::CloseStart => document.set_visible("start-menu", false),
        _ => Ok(()),
    }
}
