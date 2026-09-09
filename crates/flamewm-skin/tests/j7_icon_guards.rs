//! J7 icon/alpha ratchet: skin roles, resolver fallback, symbolic tint.

use flamewm_skin::icons::{IconRole, IconSource, IconTreatment};

#[test]
fn real_app_icon_roles_default_to_original() {
    for role in [
        IconRole::Start,
        IconRole::Browser,
        IconRole::Terminal,
        IconRole::Files,
        IconRole::Code,
    ] {
        assert_eq!(role.treatment(), IconTreatment::Original, "{role:?}");
        let source: IconSource = role.source();
        assert_eq!(source.treatment, IconTreatment::Original, "{role:?}");
    }
}

#[test]
fn shell_symbolic_roles_do_not_default_to_original() {
    for role in [
        IconRole::Volume,
        IconRole::VolumeMuted,
        IconRole::Wifi,
        IconRole::Play,
        IconRole::Pause,
    ] {
        assert_eq!(
            role.treatment(),
            IconTreatment::SymbolicForeground,
            "{role:?}"
        );
    }
}

#[test]
fn shell_symbolic_sources_carry_no_embedded_breeze_rgb() {
    for role in [
        IconRole::Volume,
        IconRole::Wifi,
        IconRole::Minimize,
        IconRole::Close,
    ] {
        let source = role.source();
        assert_eq!(
            source.treatment,
            IconTreatment::SymbolicForeground,
            "{role:?}"
        );
        assert!(
            !source.path.contains(".ppm"),
            "symbolic source must not be PPM: {}",
            source.path
        );
    }
}
