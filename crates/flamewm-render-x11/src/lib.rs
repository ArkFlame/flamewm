mod xft;
mod xlib;
mod xrender;
mod xshape;

use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::ptr;

use flamewm_reactor::{FdAction, Reactor};
use flamewm_render_core::RuntimeDocument;
pub use flamewm_render_core::{ActionEvent, ActionPhase, ControllerEvent};

mod app;
mod config;
mod external_drawable;
mod ffi;
mod native;
mod runtime;
mod surface_controller;

use app::X11App;
pub use config::*;
pub use external_drawable::*;
pub use native::target::GeometryCommit;
pub use native::{PresentationState, presenter_error};
pub use runtime::*;
pub use surface_controller::*;
