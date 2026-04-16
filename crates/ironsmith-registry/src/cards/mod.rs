pub mod builders;

#[cfg(not(test))]
#[path = "../../../ironsmith-runtime/src/cards/definitions/mod.rs"]
pub mod definitions;

#[cfg(not(test))]
#[path = "../../../ironsmith-runtime/src/cards/tokens/mod.rs"]
pub mod tokens;

pub type CardDefinition = ironsmith::cards::CardDefinition;
pub type CardRegistry = crate::CardRegistry;
pub use builders::CardDefinitionBuilder;
