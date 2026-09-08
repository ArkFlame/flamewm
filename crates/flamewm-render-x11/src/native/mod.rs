//! Native target/presenter convergence (J07).
//! Transactional geometry commit + explicit [`PresentationState`] machine.

pub mod cursor;
pub mod image;
pub mod target;

pub use target::{GeometryCommit, PresentationState, presenter_error};
