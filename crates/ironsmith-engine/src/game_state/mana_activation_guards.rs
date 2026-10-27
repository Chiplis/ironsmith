//! CR 605.3c locks one acquired ability until its resolution finishes.
use super::*;

impl GameState {
    /// A resumed native announcement retains its existing acquisition. A new
    /// announcement cannot enter another active occurrence of that ability.
    pub(crate) fn begin_mana_activation_guard(
        &mut self,
        announcement: crate::provenance::ProvNodeId,
        source: ObjectId,
        origin: crate::continuous::AbilityOrigin,
    ) -> bool {
        let guards = &self.auxiliary_tracking.active_mana_abilities;
        if let Some(existing) = guards.get(&announcement) {
            return existing == &(source, origin);
        }
        if guards
            .values()
            .any(|existing| existing == &(source, origin.clone()))
        {
            return false;
        }
        self.auxiliary_tracking_mut()
            .active_mana_abilities
            .insert(announcement, (source, origin));
        self.bump_mutation_revision();
        true
    }

    pub(crate) fn end_mana_activation_guard(
        &mut self,
        announcement: crate::provenance::ProvNodeId,
    ) {
        if self
            .auxiliary_tracking
            .active_mana_abilities
            .contains_key(&announcement)
        {
            self.auxiliary_tracking_mut()
                .active_mana_abilities
                .remove(&announcement);
            self.bump_mutation_revision();
        }
    }

    pub(crate) fn active_mana_ability_locks(
        &self,
    ) -> Vec<(ObjectId, crate::continuous::AbilityOrigin)> {
        self.auxiliary_tracking
            .active_mana_abilities
            .values()
            .cloned()
            .collect()
    }

    pub(crate) fn has_active_mana_ability_on(&self, source: ObjectId) -> bool {
        self.auxiliary_tracking
            .active_mana_abilities
            .values()
            .any(|(host, _)| *host == source)
    }

    pub(crate) fn mana_ability_occurrence_is_active(
        &self,
        source: ObjectId,
        origin: &crate::continuous::AbilityOrigin,
    ) -> bool {
        self.auxiliary_tracking
            .active_mana_abilities
            .values()
            .any(|(host, active)| *host == source && active == origin)
    }
}

#[cfg(test)]
#[path = "mana_activation_guard_tests.rs"]
mod tests;
