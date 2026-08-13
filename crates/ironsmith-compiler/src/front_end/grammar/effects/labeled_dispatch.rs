#[path = "labeled_dispatch/ability_candidates.rs"]
mod ability_candidates;
#[path = "labeled_dispatch/common.rs"]
mod common;
#[path = "labeled_dispatch/cost_reduction.rs"]
mod cost_reduction;
#[path = "labeled_dispatch/labels.rs"]
mod labels;
#[path = "labeled_dispatch/passive_addition.rs"]
mod passive_addition;
#[path = "labeled_dispatch/surface.rs"]
mod surface;
#[path = "labeled_dispatch/token_copy.rs"]
mod token_copy;

pub(crate) use ability_candidates::*;
pub(crate) use cost_reduction::*;
pub(crate) use labels::*;
pub(crate) use passive_addition::*;
pub(crate) use surface::*;
pub(crate) use token_copy::*;
