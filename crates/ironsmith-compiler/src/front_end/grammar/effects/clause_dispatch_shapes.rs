#[path = "clause_dispatch_shapes/core.rs"]
mod core;
pub(crate) use core::*;

#[path = "clause_dispatch_shapes/direct.rs"]
mod direct;
pub(crate) use direct::*;

#[path = "clause_dispatch_shapes/permissions.rs"]
mod permissions;
pub(crate) use permissions::*;

#[path = "clause_dispatch_shapes/relational.rs"]
mod relational;
pub(crate) use relational::*;
