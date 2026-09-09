//! Decoration interaction: control hit-testing and drag state.
//!
//! Pure helpers over skin `control_button_geometries`; glyph pixels never
//! affect hit rects.

use flamewm_skin::recipes::window_chrome::{
    SceneRect, WindowControlRole, control_button_geometries,
};

/// Title-button hit target derived from skin geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameControl {
    Minimize,
    Maximize,
    Close,
}

impl FrameControl {
    #[must_use]
    pub const fn role(self) -> crate::chrome::ControlRole {
        match self {
            Self::Minimize => crate::chrome::ControlRole::Minimize,
            Self::Maximize => crate::chrome::ControlRole::Maximize,
            Self::Close => crate::chrome::ControlRole::Close,
        }
    }
}

/// Shared title-button hit targets from skin `control_button_geometries`.
#[must_use]
pub fn frame_control_at(frame_width: u32, titlebar_height: u16, x: i16) -> Option<FrameControl> {
    let width = i32::try_from(frame_width).unwrap_or(i32::MAX);
    let titlebar = SceneRect::new(0, 0, width, i32::from(titlebar_height));
    let geometries = control_button_geometries(titlebar);
    let x = i32::from(x);
    for geometry in geometries.iter().rev() {
        if x >= geometry.bounds.x {
            return Some(match geometry.role {
                WindowControlRole::Minimize => FrameControl::Minimize,
                WindowControlRole::Close => FrameControl::Close,
                _ => FrameControl::Maximize,
            });
        }
    }
    None
}
