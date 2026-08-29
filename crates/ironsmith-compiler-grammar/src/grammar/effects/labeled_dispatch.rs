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

pub use ability_candidates::*;
pub use cost_reduction::*;
pub use labels::*;
pub use passive_addition::*;
pub use surface::*;
pub use token_copy::*;
