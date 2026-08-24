pub mod builders;

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
#[allow(dead_code)]
#[path = "../../../ironsmith-engine/src/cards/definitions/mod.rs"]
pub mod definitions;

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
mod handwritten_registry;

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
pub use handwritten_registry::register_builtin_handwritten_cards_if;

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
#[path = "../../../ironsmith-engine/src/cards/tokens/mod.rs"]
pub mod tokens;

pub type CardDefinition = ironsmith::cards::CardDefinition;
pub type CardRegistry = crate::CardRegistry;
pub use builders::CardDefinitionBuilder;
