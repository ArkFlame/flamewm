use std::os::fd::RawFd;

use std::collections::HashMap;

use flamewm_render_core::RuntimeDocument;
use flamewm_render_x11::{SurfaceConfig as RenderSurfaceConfig, SurfaceController, SurfaceId};

use crate::{UiDocumentView, UiTemplate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceRole {
    Normal,
    Desktop,
    Dock,
    PopupMenu,
    DropdownMenu,
}

impl From<SurfaceRole> for flamewm_render_x11::SurfaceRole {
    fn from(role: SurfaceRole) -> Self {
        match role {
            SurfaceRole::Normal => Self::Normal,
            SurfaceRole::Desktop => Self::Desktop,
            SurfaceRole::Dock => Self::Dock,
            SurfaceRole::PopupMenu => Self::PopupMenu,
            SurfaceRole::DropdownMenu => Self::DropdownMenu,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub role: SurfaceRole,
    pub initially_visible: bool,
    pub x: i32,
    pub y: i32,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            width: 1350,
            height: 641,
            title: "FlameWM UI".to_string(),
            role: SurfaceRole::Normal,
            initially_visible: true,
            x: 0,
            y: 0,
        }
    }
}

impl From<SurfaceConfig> for RenderSurfaceConfig {
    fn from(config: SurfaceConfig) -> Self {
        Self {
            width: config.width,
            height: config.height,
            title: config.title,
            role: config.role.into(),
            initially_visible: config.initially_visible,
            x: config.x,
            y: config.y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SurfaceHandle(SurfaceId);

impl SurfaceHandle {
    pub(crate) fn from_id(id: SurfaceId) -> Self {
        Self(id)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum UiBackendError {
    Renderer(String),
    Document(String),
}

impl From<String> for UiBackendError {
    fn from(error: String) -> Self {
        Self::Renderer(error)
    }
}

pub struct SurfaceRuntime {
    pub(crate) controller: SurfaceController,
    /// Retained scroll offsets keyed by (surface, node identity). Survives
    /// relayout; pruned when nodes disappear.
    scroll_offsets: HashMap<(u64, u32), flamewm_render_core::ScrollState>,
    thumb_drag: Option<ThumbDrag>,
}

#[derive(Clone, Copy, Debug)]
struct ThumbDrag {
    surface: u64,
    node: u32,
    horizontal: bool,
}

impl SurfaceRuntime {
    pub fn new() -> Result<Self, UiBackendError> {
        Ok(Self {
            controller: SurfaceController::new().map_err(UiBackendError::Renderer)?,
            scroll_offsets: HashMap::new(),
            thumb_drag: None,
        })
    }

    pub fn create_surface(
        &mut self,
        template: UiTemplate,
        config: SurfaceConfig,
    ) -> Result<SurfaceHandle, UiBackendError> {
        self.controller
            .create_surface(template.document, config.into())
            .map(SurfaceHandle)
            .map_err(UiBackendError::Renderer)
    }

    pub fn show(&mut self, surface: SurfaceHandle) -> Result<(), UiBackendError> {
        self.controller.show(surface.0).map_err(Into::into)
    }

    pub fn hide(&mut self, surface: SurfaceHandle) -> Result<(), UiBackendError> {
        self.controller.hide(surface.0).map_err(Into::into)
    }

    pub fn raise(&self, surface: SurfaceHandle) -> Result<(), UiBackendError> {
        self.controller.raise(surface.0).map_err(Into::into)
    }

    pub fn move_resize(
        &self,
        surface: SurfaceHandle,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<(), UiBackendError> {
        self.controller
            .move_resize(surface.0, x, y, width, height)
            .map_err(Into::into)
    }

    pub fn destroy(&mut self, surface: SurfaceHandle) -> Result<(), UiBackendError> {
        self.controller
            .destroy_surface(surface.0)
            .map_err(Into::into)
    }

    pub fn redraw(&mut self, surface: SurfaceHandle) -> Result<(), UiBackendError> {
        self.controller.redraw(surface.0).map_err(Into::into)
    }

    pub fn redraw_dirty(&mut self) -> Result<(), UiBackendError> {
        self.controller.redraw_dirty().map_err(Into::into)
    }

    pub fn connection_fd(&self) -> RawFd {
        self.controller.connection_fd()
    }

    pub fn grab_pointer(&mut self, surface: SurfaceHandle) -> Result<(), UiBackendError> {
        self.controller.grab_pointer(surface.0).map_err(Into::into)
    }

    pub fn ungrab_pointer(&mut self, surface: SurfaceHandle) -> Result<(), UiBackendError> {
        self.controller
            .ungrab_pointer(surface.0)
            .map_err(Into::into)
    }

    pub fn is_pointer_grabbed(&self, surface: SurfaceHandle) -> Result<bool, UiBackendError> {
        self.controller
            .is_pointer_grabbed(surface.0)
            .map_err(UiBackendError::Renderer)
    }

    pub fn with_document<R>(
        &mut self,
        surface: SurfaceHandle,
        apply: impl FnOnce(&mut UiDocumentView<'_>) -> Result<R, String>,
    ) -> Result<R, UiBackendError> {
        let document = self
            .controller
            .document_mut(surface.0)
            .map_err(UiBackendError::Document)?;
        apply(&mut UiDocumentView { document }).map_err(UiBackendError::Document)
    }

    pub(crate) fn document_mut(
        &mut self,
        surface: SurfaceHandle,
    ) -> Result<&mut RuntimeDocument, UiBackendError> {
        self.controller
            .document_mut(surface.0)
            .map_err(UiBackendError::Document)
    }

    /// Retained scroll offset for one node, keyed by (surface, node).
    pub fn scroll_offset(
        &self,
        surface: SurfaceHandle,
        node: u32,
    ) -> flamewm_render_core::ScrollState {
        self.scroll_offsets
            .get(&(surface.0.get(), node))
            .copied()
            .unwrap_or_default()
    }

    /// Apply a semantic scroll delta (wheel ticks, keyboard, programmatic)
    /// to the retained offset, clamped to [0, max].
    pub fn apply_scroll_delta(
        &mut self,
        surface: SurfaceHandle,
        node: u32,
        delta: flamewm_render_core::ScrollDelta,
        max_x: f32,
        max_y: f32,
    ) -> flamewm_render_core::ScrollState {
        let current = self.scroll_offset(surface, node);
        let next = flamewm_render_core::ScrollState {
            offset_x: current.offset_x + delta.dx,
            offset_y: current.offset_y + delta.dy,
        }
        .clamped(max_x, max_y);
        self.scroll_offsets.insert((surface.0.get(), node), next);
        next
    }

    /// Set a retained offset directly (thumb drag target).
    pub fn set_scroll_offset(
        &mut self,
        surface: SurfaceHandle,
        node: u32,
        offset: flamewm_render_core::ScrollState,
        max_x: f32,
        max_y: f32,
    ) -> flamewm_render_core::ScrollState {
        let next = offset.clamped(max_x, max_y);
        self.scroll_offsets.insert((surface.0.get(), node), next);
        next
    }

    /// Begin a thumb drag on a scrollbar thumb.
    pub fn begin_thumb_drag(&mut self, surface: SurfaceHandle, node: u32, horizontal: bool) {
        self.thumb_drag = Some(ThumbDrag {
            surface: surface.0.get(),
            node,
            horizontal,
        });
    }

    /// Move an active thumb drag: pointer position on the track maps to a
    /// content offset in [0, max_offset].
    pub fn move_thumb_drag(
        &mut self,
        surface: SurfaceHandle,
        pointer_in_track: f32,
        track_len: f32,
        thumb_len: f32,
        max_offset: f32,
    ) -> Option<flamewm_render_core::ScrollState> {
        let drag = self.thumb_drag?;
        if drag.surface != surface.0.get() {
            return None;
        }
        let offset = flamewm_ui_core::scroll::thumb_drag_offset(
            pointer_in_track,
            track_len,
            thumb_len,
            max_offset,
        );
        let current = self.scroll_offset(surface, drag.node);
        let next = if drag.horizontal {
            flamewm_render_core::ScrollState {
                offset_x: offset,
                offset_y: current.offset_y,
            }
        } else {
            flamewm_render_core::ScrollState {
                offset_x: current.offset_x,
                offset_y: offset,
            }
        };
        self.scroll_offsets.insert((drag.surface, drag.node), next);
        Some(next)
    }

    pub fn end_thumb_drag(&mut self) {
        self.thumb_drag = None;
    }

    /// Global rect of one node (layout box in surface coordinates).
    /// Needs a display; queries the render-owned layout via redraw path.
    /// Returns the document-space box by recomputing layout at the last
    /// known surface size when available.
    pub fn node_global_rect(
        &mut self,
        surface: SurfaceHandle,
        node: u32,
    ) -> Result<flamewm_render_core::Rect, UiBackendError> {
        let document = self
            .controller
            .document_mut(surface.0)
            .map_err(UiBackendError::Document)?;
        let (width, height) = document_space_extent(document);
        let layout = flamewm_render_core::LayoutEngine::compute(
            document,
            width,
            height,
            flamewm_render_core::InteractionState::default(),
        );
        layout
            .boxes
            .get(node as usize)
            .map(|entry| entry.rect)
            .ok_or_else(|| UiBackendError::Document(format!("node {node} has no layout box")))
    }

    /// Intrinsic document size: content extent of the root node.
    pub fn document_intrinsic_size(
        &mut self,
        surface: SurfaceHandle,
    ) -> Result<(f32, f32), UiBackendError> {
        let document = self
            .controller
            .document_mut(surface.0)
            .map_err(UiBackendError::Document)?;
        let (width, height) = document_space_extent(document);
        let layout = flamewm_render_core::LayoutEngine::compute(
            document,
            width,
            height,
            flamewm_render_core::InteractionState::default(),
        );
        let extent = layout.content_extent(document.document.root);
        Ok((extent.width.max(0.0), extent.height.max(0.0)))
    }
}

fn document_space_extent(document: &RuntimeDocument) -> (f32, f32) {
    // Last-known surface size is render-owned; fall back to root style size,
    // then to a conservative default so queries stay total without display.
    let style = document.runtime_style(
        document.document.root,
        flamewm_render_core::InteractionState::default(),
    );
    let width = match style.width {
        flamewm_render_core::Length::Px(value) => value,
        _ => 1350.0,
    };
    let height = match style.height {
        flamewm_render_core::Length::Px(value) => value,
        _ => 641.0,
    };
    (width.max(1.0), height.max(1.0))
}
