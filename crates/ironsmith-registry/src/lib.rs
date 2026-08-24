//! Registry ownership crate for the split workspace.

pub mod cards;

pub use ironsmith::cards::{
    CardRegistry, builtin_registry, clear_runtime_custom_cards,
    generated_definition_has_unimplemented_content,
    generated_definition_unsupported_mechanics_message, linked_face_definition_by_name_or_id,
    meld_counterpart_name, register_runtime_custom_card, reject_unsupported_generated_definition,
    unsupported_generated_definition_error,
};
pub use ironsmith_compiler_runtime::*;
pub use ironsmith_runtime_catalog::{ArtifactRegistrationError, CardRegistryArtifactExt};

pub use ironsmith::ability;
pub use ironsmith::alternative_cast;
pub use ironsmith::card;
pub use ironsmith::color;
pub use ironsmith::cost;
pub use ironsmith::costs;
pub use ironsmith::effect;
pub use ironsmith::effects;
pub use ironsmith::events;
pub use ironsmith::filter;
pub use ironsmith::ids;
pub use ironsmith::mana;
pub use ironsmith::object;
pub use ironsmith::resolution;
pub use ironsmith::static_abilities;
pub use ironsmith::tag;
pub use ironsmith::target;
pub use ironsmith::triggers;
pub use ironsmith::types;
pub use ironsmith::zone;

pub use ironsmith_compiler::{
    CardTextError, CompilerFacade, CompilerSourceDocument, ParseAnnotations, TextSpan,
    WorkspaceSplitMarker,
};

/// Concrete registry/catalog ownership boundary.
#[derive(Debug, Clone, Default)]
pub struct RegistryCatalog {
    registry: CardRegistry,
}

impl RegistryCatalog {
    pub fn new() -> Self {
        Self {
            registry: CardRegistry::new(),
        }
    }

    pub fn with_builtin_cards() -> Self {
        let registry = CardRegistry::with_builtin_cards();
        #[cfg(all(feature = "handwritten-parse-support", not(test)))]
        let mut registry = registry;
        #[cfg(all(feature = "handwritten-parse-support", not(test)))]
        cards::register_builtin_handwritten_cards_if(&mut registry, |_| true);
        Self { registry }
    }

    pub fn into_inner(self) -> CardRegistry {
        self.registry
    }

    pub fn inner(&self) -> &CardRegistry {
        &self.registry
    }
}

/// Concrete registry loading/build ownership boundary.
#[derive(Debug, Clone, Default)]
pub struct RegistryLoader;

impl RegistryLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load_builtin(&self) -> RegistryCatalog {
        RegistryCatalog::with_builtin_cards()
    }
}
