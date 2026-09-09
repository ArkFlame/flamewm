//! J7 icon/alpha ratchet: resolver fallback is alpha-bearing; treatments
//! come from role policy, never filename inference.

use std::path::Path;

use flamewm_integrations_linux::icons::{IconRequest, IconResolver, IconSize};

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn generic_fallback_is_alpha_bearing_not_ppm() {
    let mut resolver = IconResolver::new(workspace_root(), Vec::new(), Vec::new(), 0);
    let raster = resolver
        .prepare(
            IconRequest::path(workspace_root().join("missing-j7-icon.png")),
            IconSize::new(16, 8),
        )
        .expect("packaged fallback");
    assert!(raster.source.ends_with(".svg") || raster.source.ends_with(".png"));
    assert!(!raster.source.ends_with(".ppm"));
    assert_eq!(raster.pixels.len(), 8 * 8 * 4);
}

#[test]
fn theme_generation_bump_invalidates_cache_without_filename_treatment() {
    let mut resolver = IconResolver::new(workspace_root(), Vec::new(), Vec::new(), 1);
    let size = IconSize::new(16, 8);
    let before = resolver.cache_len();
    let _ = resolver.prepare(
        IconRequest::path(workspace_root().join("missing-j7-bump.png")),
        size,
    );
    let after_first = resolver.cache_len();
    assert!(after_first > before);
    resolver.notify_theme_changed(2);
    let _ = resolver.prepare(
        IconRequest::path(workspace_root().join("missing-j7-bump.png")),
        size,
    );
    assert!(resolver.cache_len() > after_first);
    assert_eq!(resolver.theme_generation(), 2);
}
