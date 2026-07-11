use super::*;

#[path = "token_copy_control_shapes/sequences.rs"]
mod sequences;
pub(crate) use sequences::*;

#[path = "token_copy_control_shapes/choices.rs"]
mod choices;
pub(crate) use choices::*;

#[path = "token_copy_control_shapes/surfaces.rs"]
mod surfaces;
pub(crate) use surfaces::*;
