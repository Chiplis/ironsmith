//! Prevention effect system.
//!
//! Prevention effects are a subset of replacement effects that specifically
//! prevent damage. Per MTG Rule 615, they use "prevent" language.
//!
//! Key rules:
//! - Rule 615.1: Prevention effects are replacement effects
//! - Rule 615.7: "Prevent the next N damage" creates a shield that tracks remaining prevention
//!               and is allocated by the affected player across simultaneous sources
//! - Rule 615.12: Prevention effects still apply once to unpreventable damage, prevent zero,
//!                retain their shields, and perform any additional effects

use crate::color::Color;
use crate::effect::{Effect, Until};
use crate::effects::ResolvedTarget;
use crate::game_state::TargetAssignment;
use crate::ids::{ObjectId, PlayerId};
use crate::types::CardType;
pub use ironsmith_core::{DamageFilter, PreventionTarget};
use std::collections::HashMap;

/// Unique identifier for a prevention shield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreventionShieldId(pub u64);

impl PreventionShieldId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// A prevention shield that can prevent a limited amount of damage.
///
/// These are created by effects like "Prevent the next 3 damage that would be
/// dealt to you this turn" or Circle of Protection effects.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventionShield {
    /// Unique identifier for this shield
    pub id: PreventionShieldId,

    /// The source that created this shield (e.g., Circle of Protection: Red)
    pub source: ObjectId,

    /// The controller of this shield effect
    pub controller: PlayerId,

    /// What this shield protects
    pub protected: PreventionTarget,

    /// Amount of damage remaining to prevent.
    /// None means infinite (e.g., Fog prevents all combat damage)
    pub amount_remaining: Option<u32>,

    /// How long this shield lasts
    pub duration: Until,

    /// Filter for what damage this shield applies to
    pub damage_filter: DamageFilter,

    /// Effects to execute using the prevented amount when this shield prevents damage.
    pub follow_up_effects: Vec<Effect>,

    /// Targets chosen when the prevention shield was created, for delayed follow-ups.
    pub follow_up_targets: Vec<ResolvedTarget>,

    /// Target assignment ranges for delayed follow-ups.
    pub follow_up_target_assignments: Vec<TargetAssignment>,

    /// Turn this shield was created (for end-of-turn cleanup)
    pub created_turn: u32,
}

impl PreventionShield {
    /// Create a new prevention shield.
    pub fn new(
        source: ObjectId,
        controller: PlayerId,
        protected: PreventionTarget,
        amount: Option<u32>,
        duration: Until,
    ) -> Self {
        Self {
            id: PreventionShieldId(0), // Set when added to manager
            source,
            controller,
            protected,
            amount_remaining: amount,
            duration,
            damage_filter: DamageFilter::default(),
            follow_up_effects: Vec::new(),
            follow_up_targets: Vec::new(),
            follow_up_target_assignments: Vec::new(),
            created_turn: 0, // Set when added to manager
        }
    }

    /// Set the damage filter.
    pub fn with_filter(mut self, filter: DamageFilter) -> Self {
        self.damage_filter = filter;
        self
    }

    /// Execute these effects with the amount of damage this shield prevented.
    pub fn with_follow_up_effects(mut self, effects: Vec<Effect>) -> Self {
        self.follow_up_effects = effects;
        self
    }

    /// Store targets chosen for delayed follow-up effects.
    pub fn with_follow_up_targets(mut self, targets: Vec<ResolvedTarget>) -> Self {
        self.follow_up_targets = targets;
        self
    }

    /// Store target assignment ranges for delayed follow-up effects.
    pub fn with_follow_up_target_assignments(
        mut self,
        target_assignments: Vec<TargetAssignment>,
    ) -> Self {
        self.follow_up_target_assignments = target_assignments;
        self
    }

    /// Check if this shield has any prevention remaining.
    pub fn has_prevention_remaining(&self) -> bool {
        self.amount_remaining.is_none_or(|a| a > 0)
    }

    /// Check if this shield is exhausted (amount remaining is 0).
    pub fn is_exhausted(&self) -> bool {
        self.amount_remaining == Some(0)
    }

    /// Reduce the amount of prevention remaining.
    /// Returns the amount that was actually prevented (may be less than requested if insufficient).
    pub fn reduce(&mut self, amount: u32) -> u32 {
        match self.amount_remaining {
            Some(remaining) => {
                let prevented = remaining.min(amount);
                self.amount_remaining = Some(remaining - prevented);
                prevented
            }
            None => {
                // Infinite prevention - prevent all
                amount
            }
        }
    }

    /// Create a "prevent the next N damage" shield.
    pub fn prevent_next_n(
        source: ObjectId,
        controller: PlayerId,
        protected: PreventionTarget,
        n: u32,
    ) -> Self {
        Self::new(source, controller, protected, Some(n), Until::EndOfTurn)
    }

    /// Create a "prevent all damage" shield (like Fog).
    pub fn prevent_all(
        source: ObjectId,
        controller: PlayerId,
        protected: PreventionTarget,
    ) -> Self {
        Self::new(source, controller, protected, None, Until::EndOfTurn)
    }

    /// Create a Circle of Protection style shield.
    pub fn circle_of_protection(source: ObjectId, controller: PlayerId, color: Color) -> Self {
        Self::prevent_next_n(source, controller, PreventionTarget::You, u32::MAX)
            .with_filter(DamageFilter::from_color(color))
    }
}

/// Manages all prevention shields in the game.
#[derive(Debug, Clone, Default)]
pub struct PreventionEffectManager {
    /// All active prevention shields
    shields: Vec<PreventionShield>,

    /// Next shield ID to assign
    next_id: u64,

    /// Current turn number (for duration tracking)
    current_turn: u32,

    /// Follow-ups produced by shields selected in the unified CR 616 loop.
    pending_follow_ups: Vec<PendingPreventionFollowUp>,
}

/// A follow-up to run after a prevention shield is applied to damage.
///
/// `prevented` is zero when CR 615.12 applies the prevention effect to
/// unpreventable damage without preventing any of it.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventionFollowUp {
    pub source: ObjectId,
    pub controller: PlayerId,
    pub prevented: u32,
    pub effects: Vec<Effect>,
    pub targets: Vec<ResolvedTarget>,
    pub target_assignments: Vec<TargetAssignment>,
}

/// A prevention follow-up paired with the exact damage event it modified.
#[derive(Debug, Clone)]
pub struct PendingPreventionFollowUp {
    pub follow_up: PreventionFollowUp,
    pub damage: crate::events::DamageEvent,
    pub provenance: crate::provenance::ProvNodeId,
}

/// Result of applying prevention to a single damage assignment.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreventionApplicationResult {
    pub remaining: u32,
    pub follow_ups: Vec<PreventionFollowUp>,
}

impl PreventionEffectManager {
    /// Create a new empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot active prevention shields.
    pub fn shields(&self) -> &[PreventionShield] {
        &self.shields
    }

    /// Get the next shield id (for deterministic state hashing).
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Get the current turn number (for deterministic state hashing).
    pub fn current_turn(&self) -> u32 {
        self.current_turn
    }

    /// Apply one specifically chosen shield to a damage amount.
    pub fn apply_chosen_shield(
        &mut self,
        id: PreventionShieldId,
        damage: u32,
        can_prevent: bool,
        max_amount: Option<u32>,
    ) -> PreventionApplicationResult {
        if damage == 0 {
            return PreventionApplicationResult::default();
        }
        let Some(shield) = self.get_shield_mut(id) else {
            return PreventionApplicationResult {
                remaining: damage,
                follow_ups: Vec::new(),
            };
        };
        let prevented = if can_prevent {
            shield.reduce(damage.min(max_amount.unwrap_or(u32::MAX)))
        } else {
            0
        };
        let follow_ups = if !shield.follow_up_effects.is_empty() {
            vec![PreventionFollowUp {
                source: shield.source,
                controller: shield.controller,
                prevented,
                effects: shield.follow_up_effects.clone(),
                targets: shield.follow_up_targets.clone(),
                target_assignments: shield.follow_up_target_assignments.clone(),
            }]
        } else {
            Vec::new()
        };
        if can_prevent {
            self.cleanup_exhausted();
        }
        PreventionApplicationResult {
            remaining: damage - prevented,
            follow_ups,
        }
    }

    /// Queue a chosen shield's follow-up with its exact pre-damage event.
    pub fn queue_follow_up(
        &mut self,
        follow_up: PreventionFollowUp,
        damage: crate::events::DamageEvent,
        provenance: crate::provenance::ProvNodeId,
    ) {
        self.pending_follow_ups.push(PendingPreventionFollowUp {
            follow_up,
            damage,
            provenance,
        });
    }

    /// Drain follow-ups produced by the unified replacement loop.
    pub fn take_pending_follow_ups(&mut self) -> Vec<PendingPreventionFollowUp> {
        std::mem::take(&mut self.pending_follow_ups)
    }

    /// Add a new prevention shield.
    pub fn add_shield(&mut self, mut shield: PreventionShield) -> PreventionShieldId {
        let id = PreventionShieldId::new(self.next_id);
        self.next_id += 1;
        shield.id = id;
        shield.created_turn = self.current_turn;
        self.shields.push(shield);
        id
    }

    /// Remove a shield by ID.
    pub fn remove_shield(&mut self, id: PreventionShieldId) {
        self.shields.retain(|s| s.id != id);
    }

    /// Remove all shields from a specific source.
    pub fn remove_shields_from_source(&mut self, source: ObjectId) {
        self.shields.retain(|s| s.source != source);
    }

    /// Remove all exhausted shields.
    pub fn cleanup_exhausted(&mut self) {
        self.shields.retain(|s| !s.is_exhausted());
    }

    /// Clean up shields at end of turn.
    pub fn cleanup_end_of_turn(&mut self) {
        self.shields
            .retain(|s| !matches!(s.duration, Until::EndOfTurn));
    }

    /// Set the current turn number.
    pub fn set_turn(&mut self, turn: u32) {
        self.current_turn = turn;
    }

    /// Get all shields that could apply to damage to a player.
    pub fn get_shields_for_player(&self, player: PlayerId) -> Vec<&PreventionShield> {
        self.shields
            .iter()
            .filter(|s| s.has_prevention_remaining())
            .filter(|s| match &s.protected {
                PreventionTarget::Player(p) => *p == player,
                PreventionTarget::Players => true,
                PreventionTarget::You => s.controller == player,
                PreventionTarget::YouAndPermanentsYouControl
                | PreventionTarget::YouAndPermanentsMatching(_) => s.controller == player,
                PreventionTarget::All => true,
                _ => false,
            })
            .collect()
    }

    /// Get all shields that could apply to damage to a permanent.
    pub fn get_shields_for_permanent(
        &self,
        permanent: ObjectId,
        controller: PlayerId,
        protected_filter_matches: &HashMap<PreventionShieldId, bool>,
    ) -> Vec<&PreventionShield> {
        self.shields
            .iter()
            .filter(|s| s.has_prevention_remaining())
            .filter(|s| match &s.protected {
                PreventionTarget::Permanent(p) => *p == permanent,
                PreventionTarget::YouAndPermanentsYouControl => s.controller == controller,
                PreventionTarget::PermanentsMatching(_)
                | PreventionTarget::YouAndPermanentsMatching(_) => {
                    protected_filter_matches
                        .get(&s.id)
                        .copied()
                        .unwrap_or(false)
                }
                PreventionTarget::All => true,
                _ => false,
            })
            .collect()
    }

    /// Get a mutable shield by ID.
    pub fn get_shield_mut(&mut self, id: PreventionShieldId) -> Option<&mut PreventionShield> {
        self.shields.iter_mut().find(|s| s.id == id)
    }

    /// Apply prevention to damage.
    ///
    /// This finds applicable shields and reduces the damage amount,
    /// consuming shield prevention as needed.
    ///
    /// Returns the amount of damage remaining after prevention.
    ///
    /// Per Rule 615.12: If damage "can't be prevented", this still checks shields
    /// but doesn't actually reduce damage or consume shield amounts.
    pub fn apply_prevention_to_player(
        &mut self,
        player: PlayerId,
        damage: u32,
        is_combat: bool,
        source: ObjectId,
        source_colors: &crate::color::ColorSet,
        source_card_types: &[CardType],
        can_be_prevented: bool,
    ) -> u32 {
        let source_filter_matches = HashMap::new();
        self.apply_prevention_to_player_with_follow_ups(
            player,
            damage,
            is_combat,
            source,
            source_colors,
            source_card_types,
            can_be_prevented,
            &source_filter_matches,
        )
        .remaining
    }

    /// Apply prevention to damage to a player, collecting any shield follow-ups.
    pub fn apply_prevention_to_player_with_follow_ups(
        &mut self,
        player: PlayerId,
        damage: u32,
        is_combat: bool,
        source: ObjectId,
        source_colors: &crate::color::ColorSet,
        source_card_types: &[CardType],
        can_be_prevented: bool,
        source_filter_matches: &HashMap<PreventionShieldId, bool>,
    ) -> PreventionApplicationResult {
        if damage == 0 {
            return PreventionApplicationResult::default();
        }

        let mut remaining = damage;
        let mut follow_ups = Vec::new();

        // Find applicable shields
        let shield_ids: Vec<PreventionShieldId> = self
            .get_shields_for_player(player)
            .iter()
            .filter(|s| {
                let source_filter_ok = s.damage_filter.from_source.is_none()
                    || source_filter_matches.get(&s.id).copied().unwrap_or(false);
                source_filter_ok
                    && s.damage_filter
                        .matches(is_combat, source, source_colors, source_card_types)
            })
            .map(|s| s.id)
            .collect();

        // Apply prevention from each shield
        for id in shield_ids {
            if remaining == 0 {
                break;
            }

            if let Some(shield) = self.get_shield_mut(id)
                && can_be_prevented
            {
                // Normal prevention - reduce damage and consume shield
                let prevented = shield.reduce(remaining);
                remaining -= prevented;
                if prevented > 0 && !shield.follow_up_effects.is_empty() {
                    follow_ups.push(PreventionFollowUp {
                        source: shield.source,
                        controller: shield.controller,
                        prevented,
                        effects: shield.follow_up_effects.clone(),
                        targets: shield.follow_up_targets.clone(),
                        target_assignments: shield.follow_up_target_assignments.clone(),
                    });
                }
            }
            // If can't be prevented: shield is "applied" but doesn't prevent
            // and doesn't get consumed (per Rule 615.12)
        }

        // Clean up exhausted shields
        self.cleanup_exhausted();

        PreventionApplicationResult {
            remaining,
            follow_ups,
        }
    }

    /// Apply prevention to damage to a permanent.
    ///
    /// Similar to apply_prevention_to_player but for creatures/planeswalkers.
    pub fn apply_prevention_to_permanent(
        &mut self,
        permanent: ObjectId,
        controller: PlayerId,
        damage: u32,
        is_combat: bool,
        source: ObjectId,
        source_colors: &crate::color::ColorSet,
        source_card_types: &[CardType],
        can_be_prevented: bool,
    ) -> u32 {
        let source_filter_matches = HashMap::new();
        let protected_filter_matches = HashMap::new();
        self.apply_prevention_to_permanent_with_follow_ups(
            permanent,
            controller,
            damage,
            is_combat,
            source,
            source_colors,
            source_card_types,
            can_be_prevented,
            &source_filter_matches,
            &protected_filter_matches,
        )
        .remaining
    }

    /// Apply prevention to damage to a permanent, collecting any shield follow-ups.
    pub fn apply_prevention_to_permanent_with_follow_ups(
        &mut self,
        permanent: ObjectId,
        controller: PlayerId,
        damage: u32,
        is_combat: bool,
        source: ObjectId,
        source_colors: &crate::color::ColorSet,
        source_card_types: &[CardType],
        can_be_prevented: bool,
        source_filter_matches: &HashMap<PreventionShieldId, bool>,
        protected_filter_matches: &HashMap<PreventionShieldId, bool>,
    ) -> PreventionApplicationResult {
        if damage == 0 {
            return PreventionApplicationResult::default();
        }

        let mut remaining = damage;
        let mut follow_ups = Vec::new();

        // Find applicable shields
        let shield_ids: Vec<PreventionShieldId> = self
            .get_shields_for_permanent(permanent, controller, protected_filter_matches)
            .iter()
            .filter(|s| {
                let source_filter_ok = s.damage_filter.from_source.is_none()
                    || source_filter_matches.get(&s.id).copied().unwrap_or(false);
                source_filter_ok
                    && s.damage_filter
                        .matches(is_combat, source, source_colors, source_card_types)
            })
            .map(|s| s.id)
            .collect();

        // Apply prevention from each shield
        for id in shield_ids {
            if remaining == 0 {
                break;
            }

            if let Some(shield) = self.get_shield_mut(id)
                && can_be_prevented
            {
                let prevented = shield.reduce(remaining);
                remaining -= prevented;
                if prevented > 0 && !shield.follow_up_effects.is_empty() {
                    follow_ups.push(PreventionFollowUp {
                        source: shield.source,
                        controller: shield.controller,
                        prevented,
                        effects: shield.follow_up_effects.clone(),
                        targets: shield.follow_up_targets.clone(),
                        target_assignments: shield.follow_up_target_assignments.clone(),
                    });
                }
            }
        }

        // Clean up exhausted shields
        self.cleanup_exhausted();

        PreventionApplicationResult {
            remaining,
            follow_ups,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prevention_shield_creation() {
        let shield = PreventionShield::prevent_next_n(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            PreventionTarget::You,
            3,
        );

        assert!(shield.has_prevention_remaining());
        assert!(!shield.is_exhausted());
        assert_eq!(shield.amount_remaining, Some(3));
    }

    #[test]
    fn test_shield_reduction() {
        let mut shield = PreventionShield::prevent_next_n(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            PreventionTarget::You,
            5,
        );

        // Prevent 3 damage
        let prevented = shield.reduce(3);
        assert_eq!(prevented, 3);
        assert_eq!(shield.amount_remaining, Some(2));

        // Prevent 3 more (only 2 remaining)
        let prevented = shield.reduce(3);
        assert_eq!(prevented, 2);
        assert_eq!(shield.amount_remaining, Some(0));
        assert!(shield.is_exhausted());
    }

    #[test]
    fn test_infinite_prevention() {
        let mut shield = PreventionShield::prevent_all(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            PreventionTarget::You,
        );

        // Infinite prevention prevents all
        let prevented = shield.reduce(1000);
        assert_eq!(prevented, 1000);
        assert!(!shield.is_exhausted());
    }

    #[test]
    fn test_damage_filter_combat() {
        let filter = DamageFilter::combat();

        let colors = crate::color::ColorSet::RED;
        let types = vec![CardType::Creature];

        assert!(filter.matches(true, ObjectId::from_raw(1), &colors, &types));
        assert!(!filter.matches(false, ObjectId::from_raw(1), &colors, &types));
    }

    #[test]
    fn test_damage_filter_color() {
        let filter = DamageFilter::from_color(Color::Red);

        let red = crate::color::ColorSet::RED;
        let blue = crate::color::ColorSet::BLUE;
        let types = vec![CardType::Creature];

        assert!(filter.matches(false, ObjectId::from_raw(1), &red, &types));
        assert!(!filter.matches(false, ObjectId::from_raw(1), &blue, &types));
    }

    #[test]
    fn test_prevention_manager() {
        let mut manager = PreventionEffectManager::new();

        let shield = PreventionShield::prevent_next_n(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            PreventionTarget::You,
            5,
        );

        manager.add_shield(shield);
        assert_eq!(manager.shields().len(), 1);

        // Apply prevention
        let colors = crate::color::ColorSet::COLORLESS;
        let types = vec![];
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            3,
            false,
            ObjectId::from_raw(2),
            &colors,
            &types,
            true, // can be prevented
        );

        assert_eq!(remaining, 0); // All 3 damage prevented
        assert_eq!(manager.shields()[0].amount_remaining, Some(2)); // 2 remaining

        // Apply more prevention
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            10,
            false,
            ObjectId::from_raw(2),
            &colors,
            &types,
            true,
        );

        assert_eq!(remaining, 8); // Only 2 could be prevented
        assert!(manager.shields().is_empty()); // Shield exhausted and removed
    }

    #[test]
    fn filter_based_shield_requires_the_damaged_permanent_to_match() {
        let mut manager = PreventionEffectManager::new();
        let permanent = ObjectId::from_raw(2);
        let shield_id = manager.add_shield(PreventionShield::prevent_all(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            PreventionTarget::PermanentsMatching(crate::target::ObjectFilter::creature()),
        ));

        assert!(
            manager
                .get_shields_for_permanent(
                    permanent,
                    PlayerId::from_index(0),
                    &HashMap::from([(shield_id, false)]),
                )
                .is_empty(),
            "a filter shield must not be offered for a nonmatching permanent"
        );
        assert_eq!(
            manager
                .get_shields_for_permanent(
                    permanent,
                    PlayerId::from_index(0),
                    &HashMap::from([(shield_id, true)]),
                )
                .len(),
            1,
            "the same shield should be offered when the protected-object filter matches"
        );
    }

    #[test]
    fn test_unpreventable_damage() {
        let mut manager = PreventionEffectManager::new();

        let shield = PreventionShield::prevent_next_n(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            PreventionTarget::You,
            5,
        );

        manager.add_shield(shield);

        // Apply unpreventable damage
        let colors = crate::color::ColorSet::COLORLESS;
        let types = vec![];
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            3,
            false,
            ObjectId::from_raw(2),
            &colors,
            &types,
            false, // can't be prevented
        );

        // Damage not prevented
        assert_eq!(remaining, 3);

        // Shield NOT consumed (per Rule 615.12)
        assert_eq!(manager.shields()[0].amount_remaining, Some(5));
    }

    /// Test Rule 615.12: Unpreventable damage doesn't consume shields,
    /// so subsequent preventable damage can still be prevented.
    #[test]
    fn test_rule_615_12_shield_preserved_after_unpreventable() {
        let mut manager = PreventionEffectManager::new();

        let shield = PreventionShield::prevent_next_n(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            PreventionTarget::You,
            5,
        );

        manager.add_shield(shield);
        let colors = crate::color::ColorSet::COLORLESS;
        let types = vec![];

        // Step 1: Apply unpreventable damage (3 damage)
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            3,
            false,
            ObjectId::from_raw(2),
            &colors,
            &types,
            false, // can't be prevented
        );
        assert_eq!(remaining, 3); // Full damage goes through
        assert_eq!(manager.shields()[0].amount_remaining, Some(5)); // Shield intact

        // Step 2: Apply preventable damage (4 damage)
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            4,
            false,
            ObjectId::from_raw(2),
            &colors,
            &types,
            true, // can be prevented
        );
        assert_eq!(remaining, 0); // All 4 damage prevented
        assert_eq!(manager.shields()[0].amount_remaining, Some(1)); // 1 remaining

        // Step 3: Apply more unpreventable damage (2 damage)
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            2,
            false,
            ObjectId::from_raw(2),
            &colors,
            &types,
            false, // can't be prevented
        );
        assert_eq!(remaining, 2); // Full damage goes through
        assert_eq!(manager.shields()[0].amount_remaining, Some(1)); // Still 1 remaining

        // Step 4: Apply final preventable damage (3 damage)
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            3,
            false,
            ObjectId::from_raw(2),
            &colors,
            &types,
            true, // can be prevented
        );
        assert_eq!(remaining, 2); // Only 1 prevented, 2 go through
        assert!(manager.shields().is_empty()); // Shield now exhausted
    }

    /// Test that multiple shields work correctly with mixed preventable/unpreventable damage.
    #[test]
    fn test_multiple_shields_with_unpreventable() {
        let mut manager = PreventionEffectManager::new();

        // Add two shields
        let shield1 = PreventionShield::prevent_next_n(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            PreventionTarget::You,
            3,
        );
        let shield2 = PreventionShield::prevent_next_n(
            ObjectId::from_raw(2),
            PlayerId::from_index(0),
            PreventionTarget::You,
            3,
        );

        manager.add_shield(shield1);
        manager.add_shield(shield2);
        assert_eq!(manager.shields().len(), 2);

        let colors = crate::color::ColorSet::COLORLESS;
        let types = vec![];

        // Unpreventable damage - neither shield consumed
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            10,
            false,
            ObjectId::from_raw(3),
            &colors,
            &types,
            false,
        );
        assert_eq!(remaining, 10);
        assert_eq!(manager.shields().len(), 2);
        assert_eq!(manager.shields()[0].amount_remaining, Some(3));
        assert_eq!(manager.shields()[1].amount_remaining, Some(3));

        // Preventable damage - uses both shields (5 damage)
        let remaining = manager.apply_prevention_to_player(
            PlayerId::from_index(0),
            5,
            false,
            ObjectId::from_raw(3),
            &colors,
            &types,
            true,
        );
        assert_eq!(remaining, 0); // All prevented
        // First shield exhausted (3), second shield used 2
        assert_eq!(manager.shields().len(), 1); // One exhausted and removed
        assert_eq!(manager.shields()[0].amount_remaining, Some(1));
    }
}
