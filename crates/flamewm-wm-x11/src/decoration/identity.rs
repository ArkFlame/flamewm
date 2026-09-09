//! Decoration identity: catalog-backed app-icon resolution order.
//!
//! Priority: live `_NET_WM_ICON` pixels, catalog `DesktopEntry` `Icon=`
//! raster via the icon resolver, generic fallback, coarse `WM_CLASS` skin
//! role as last resort. WM_CLASS parsing itself is pure; catalog/resolver
//! lookups stay in the manager.

use flamewm_applications::WindowApplicationIdentity;

use crate::chrome;

/// Canonical catalog identity from `WM_CLASS` (instance\0class).
#[must_use]
pub fn window_identity(wm_class: &str) -> WindowApplicationIdentity {
    let mut parts = wm_class.split('\0');
    let instance = parts.next().unwrap_or("").trim().to_owned();
    let class = parts.next().unwrap_or("").trim().to_owned();
    let executable = if !instance.is_empty() {
        instance.clone()
    } else {
        class.clone()
    };
    WindowApplicationIdentity::new(instance, class, executable)
}

/// Coarse `WM_CLASS` role fallback. Pure, mints no state.
#[must_use]
pub fn class_role_fallback(wm_class: &str) -> Option<flamewm_skin::icons::IconRole> {
    crate::chrome::icon_role_for_class(wm_class)
}

/// Scene icon role for a resolved source: native pixels map to `Note`,
/// class roles map through, catalog rasters and generic stay `None` (the
/// raster itself is the paint input).
#[must_use]
pub fn scene_role_for(source: &chrome::AppIconSource) -> Option<flamewm_skin::icons::IconRole> {
    match source {
        chrome::AppIconSource::ClassRole(role) => Some(*role),
        chrome::AppIconSource::Native(_) => Some(flamewm_skin::icons::IconRole::Note),
        chrome::AppIconSource::CatalogRaster(_) | chrome::AppIconSource::Generic => None,
    }
}
