#[path = "for_each_shapes/facts.rs"]
mod facts;
#[path = "for_each_shapes/modifier.rs"]
mod modifier;
#[path = "for_each_shapes/participants.rs"]
mod participants;
#[path = "for_each_shapes/power.rs"]
mod power;
#[path = "for_each_shapes/subjects.rs"]
mod subjects;

pub(crate) use facts::*;
pub(crate) use modifier::*;
pub(crate) use participants::*;
pub(crate) use power::*;
pub(crate) use subjects::*;
