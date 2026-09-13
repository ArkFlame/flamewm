use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use flamewm_api::settings::AppearanceMode;
use flamewm_session_core::{build_launch_plan_with_appearance, default_paths_from_home};
use flamewm_wm::{WmConfig, run};
use flamewm_xsettings::claim_manager;

fn main() {
    apply_session_environment();
    claim_xsettings_manager();
    if let Err(error) = run(WmConfig::from_env()) {
        eprintln!("flamewm: {error}");
        std::process::exit(1);
    }
}

/// Consume session-core environment so the WM process and every child it
/// spawns inherit the FlameWM identity (FlameWM, never KDE) plus cursor and
/// appearance state. Dark is the default; explicit user values win except
/// for FlameWM-owned identity keys.
fn apply_session_environment() {
    let existing: BTreeMap<String, String> = env::vars().collect();
    let appearance = existing
        .get("FLAMEWM_APPEARANCE")
        .and_then(|value| AppearanceMode::parse(value))
        .unwrap_or_default();
    let home = env::var_os("HOME").map_or_else(|| PathBuf::from("/root"), PathBuf::from);
    let bindir = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("/usr/bin"));
    let paths = default_paths_from_home(bindir, home);
    let mut pending: BTreeMap<String, String> = BTreeMap::new();
    // session-core contributes cursor, identity, and appearance-marker
    // state only. No generic toolkit default injection: explicit inherited
    // GTK_THEME/QT_STYLE_OVERRIDE pass through untouched, never synthesized.
    let plan = build_launch_plan_with_appearance(&paths, &[], &existing, None, appearance);
    if let Some(process) = plan.processes.first() {
        for (key, value) in &process.environment {
            if key == "XDG_CURRENT_DESKTOP" && is_kde(value) {
                continue;
            }
            if key.starts_with("FLAMEWM_") || env::var_os(key).is_none() {
                pending.insert(key.clone(), value.clone());
            }
        }
    }
    if let Some(current) = env::var_os("XDG_CURRENT_DESKTOP") {
        if current.to_string_lossy().eq_ignore_ascii_case("kde") {
            pending.insert("XDG_CURRENT_DESKTOP".to_owned(), "FlameWM".to_owned());
        }
    }
    flamewm_session_core::apply_current_process_env(&pending);
}

fn is_kde(value: &str) -> bool {
    value.eq_ignore_ascii_case("kde")
}

/// Start the FlameWM XSettings owner. Refuses to replace an existing
/// `_XSETTINGS_S0` owner; the WM keeps running without XSettings ownership.
fn claim_xsettings_manager() {
    match claim_manager(&XpropSelectionProbe) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("flamewm: XSettings manager not claimed: {error}");
        }
    }
}

/// Selection state probe without a native X11 dependency: `xprop` is the
/// same readiness tool the Xephyr harness already requires.
struct XpropSelectionProbe;

impl flamewm_xsettings::SelectionOwner for XpropSelectionProbe {
    fn selection_owner_exists(&self, selection: &str) -> bool {
        let output = Command::new("xprop").arg("-root").arg(selection).output();
        match output {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                !text.contains("not found.")
            }
            _ => false,
        }
    }
}
