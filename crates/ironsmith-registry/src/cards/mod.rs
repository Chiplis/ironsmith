pub mod builders;

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
#[allow(dead_code)]
#[path = "../../../ironsmith-engine/src/cards/definitions/mod.rs"]
pub mod definitions;

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
#[path = "../../../ironsmith-engine/src/cards/tokens/mod.rs"]
pub mod tokens;

pub type CardDefinition = ironsmith::cards::CardDefinition;
pub type CardRegistry = crate::CardRegistry;
pub use builders::CardDefinitionBuilder;
