pub mod builders;

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
#[allow(dead_code)]
#[path = "../../../ironsmith-engine/src/cards/definitions/mod.rs"]
pub mod definitions;

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
pub fn register_builtin_handwritten_cards_if<F>(registry: &mut CardRegistry, include: F)
where
    F: FnMut(&str) -> bool,
{
    definitions::catalog::HandwrittenCatalog::Standard.register(registry, include);
}

#[cfg(all(not(test), feature = "handwritten-parse-support"))]
#[path = "../../../ironsmith-engine/src/cards/tokens/mod.rs"]
pub mod tokens;

pub type CardDefinition = ironsmith::cards::CardDefinition;
pub type CardRegistry = crate::CardRegistry;
pub use builders::CardDefinitionBuilder;
