#[path = "control_copy_attach_shapes/common.rs"]
mod common;
#[path = "control_copy_attach_shapes/control.rs"]
mod control;
#[path = "control_copy_attach_shapes/destinations.rs"]
mod destinations;
#[path = "control_copy_attach_shapes/life.rs"]
mod life;
#[path = "control_copy_attach_shapes/looked_put.rs"]
mod looked_put;

pub(crate) use common::*;
pub(crate) use control::*;
pub(crate) use destinations::*;
pub(crate) use life::*;
pub(crate) use looked_put::*;
