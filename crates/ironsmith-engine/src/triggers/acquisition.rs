//! Typed trigger acquisition, separate from its changing executable words.
use crate::continuous::AbilityOrigin;
use crate::ids::ObjectId;
use ironsmith_core::LinkedExileDefinition;
use std::hash::{Hash, Hasher};

/// Runtime-only evidence: a retained authored occurrence acquired by one exact
/// object incarnation through one existing origin. Copies rebind the host and
/// origin; immutable text transformations retain the whole acquisition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TriggerAcquisition {
    pub definition: LinkedExileDefinition,
    pub host: ObjectId,
    pub origin: AbilityOrigin,
}

impl TriggerAcquisition {
    pub fn identity(&self) -> super::TriggerIdentity {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        b"ironsmith-trigger-acquisition-v1".hash(&mut hasher);
        self.hash(&mut hasher);
        super::TriggerIdentity(hasher.finish())
    }
}

/// Only an actual acquisition owner calls this. In particular, a missing
/// snapshot origin is never reconstructed from a donor's current abilities.
pub(crate) fn bind_ability(
    ability: &mut crate::ability::Ability,
    host: Option<ObjectId>,
    origin: &AbilityOrigin,
) {
    if let crate::ability::AbilityKind::Triggered(triggered) = &mut ability.kind {
        triggered.trigger.acquisition = host.zip(triggered.effects.retained_trigger_definition())
            .map(|(host, definition)| TriggerAcquisition {
                definition, host, origin: origin.clone(),
            });
    }
}

#[cfg(test)]
#[path = "acquisition_tests.rs"]
mod tests;
