use flamewm_render_core::RuntimeDocument;

use crate::{SurfaceHandle, SurfaceRuntime, UiBackendError};

pub fn reconcile<F>(
    runtime: &mut SurfaceRuntime,
    surface: SurfaceHandle,
    mutation: F,
) -> Result<(), UiBackendError>
where
    F: FnOnce(&mut RuntimeDocument) -> Result<(), String>,
{
    mutation(runtime.document_mut(surface)?).map_err(UiBackendError::Document)?;
    runtime.redraw(surface)
}

pub fn update<F>(
    runtime: &mut SurfaceRuntime,
    surface: SurfaceHandle,
    mutation: F,
) -> Result<(), UiBackendError>
where
    F: FnOnce(&mut RuntimeDocument),
{
    mutation(runtime.document_mut(surface)?);
    runtime.redraw(surface)
}
