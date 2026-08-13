#[path = "combat_shapes/attachments.rs"]
mod attachments;
#[path = "combat_shapes/conditions.rs"]
mod conditions;
#[path = "combat_shapes/damage.rs"]
mod damage;

pub(crate) use attachments::*;
pub(crate) use conditions::*;
pub(crate) use damage::*;
