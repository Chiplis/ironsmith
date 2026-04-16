pub mod builders;
pub mod tokens;

pub type CardDefinition = ironsmith_core::CardDefinition<
    crate::ability::Ability,
    crate::effect::Effect,
    crate::costs::Cost,
    crate::alternative_cast::AlternativeCastingMethod,
    crate::cost::OptionalCost,
>;

pub use crate::diagnostics::{ParseAnnotations, TextSpan};
pub use builders::CardDefinitionBuilder;
