//! Registry ownership crate for the split workspace.

pub mod cards;
mod compiler_runtime;

mod generated_registry {
    include!(concat!(env!("OUT_DIR"), "/generated_registry.rs"));
}

mod generated_meld_counterparts {
    include!(concat!(env!("OUT_DIR"), "/generated_meld_counterparts.rs"));
}

#[path = "runtime_registry_impl.rs"]
mod registry_impl;

pub use registry_impl::*;
pub use compiler_runtime::*;

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
        Self {
            registry: CardRegistry::with_builtin_cards(),
        }
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

#[cfg(test)]
pub(crate) fn register_builtin_handwritten_cards_if_for_runtime_tests<F>(
    _registry: &mut CardRegistry,
    _include_constructor_key: F,
) where
    F: FnMut(&str) -> bool,
{
}
