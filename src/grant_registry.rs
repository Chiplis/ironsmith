//! Grant Registry for tracking granted effects.
//!
//! This module provides a unified system for tracking effects granted to cards:
//! - Alternative casting costs (flashback, escape, etc.)
//! - Abilities granted to cards in non-battlefield zones (flash, cycling, etc.)
//!
//! Effects can be granted by:
//! - One-shot effects with duration (e.g., Snapcaster Mage grants flashback until end of turn)
//! - Static abilities (e.g., Underworld Breach grants escape while on battlefield)

use crate::alternative_cast::AlternativeCastingMethod;
use crate::grant::{DerivedAlternativeCast, Grantable};
use crate::ids::{ObjectId, PlayerId};
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;
use crate::zone::Zone;

/// How a grant was created, determining when it expires.
#[derive(Debug, Clone, PartialEq)]
pub enum GrantSource {
    /// From a one-shot effect with a duration.
    /// The grant expires at end of turn (or other specified time).
    Effect {
        /// The object that created this grant (for tracking/display).
        source_id: ObjectId,
        /// Turn number when this grant expires (at end of that turn).
        expires_end_of_turn: u32,
    },
    /// From a static ability on a permanent.
    /// The grant exists only while the source is on the battlefield.
    StaticAbility {
        /// The permanent providing this grant.
        source_id: ObjectId,
    },
}

impl GrantSource {
    /// Create a grant sourced from a resolving effect that lasts through end of turn.
    pub fn until_end_of_turn(source_id: ObjectId, turn: u32) -> Self {
        GrantSource::Effect {
            source_id,
            expires_end_of_turn: turn,
        }
    }

    /// Source object that provided this grant.
    pub fn source_id(&self) -> ObjectId {
        match self {
            GrantSource::Effect { source_id, .. } => *source_id,
            GrantSource::StaticAbility { source_id } => *source_id,
        }
    }

    /// Check if this grant is still valid.
    pub fn is_valid(&self, game: &crate::game_state::GameState) -> bool {
        match self {
            GrantSource::Effect {
                expires_end_of_turn,
                ..
            } => {
                // Valid until the end of the specified turn
                game.turn.turn_number <= *expires_end_of_turn
            }
            GrantSource::StaticAbility { source_id } => {
                // Valid only while source is on battlefield
                game.battlefield.contains(source_id)
            }
        }
    }

    /// Check if this grant is still valid using raw data (for cleanup).
    pub fn is_valid_raw(&self, turn_number: u32, battlefield: &[ObjectId]) -> bool {
        match self {
            GrantSource::Effect {
                expires_end_of_turn,
                ..
            } => {
                // Valid until the end of the specified turn
                turn_number <= *expires_end_of_turn
            }
            GrantSource::StaticAbility { source_id } => {
                // Valid only while source is on battlefield
                battlefield.contains(source_id)
            }
        }
    }
}

/// Normalized lifetime for a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantLifetime {
    /// Valid until the end of the specified turn.
    UntilEndOfTurn { source_id: ObjectId, turn: u32 },
    /// Valid while the source remains on the battlefield.
    WhileSourceOnBattlefield(ObjectId),
}

impl GrantLifetime {
    pub fn source_id(&self) -> ObjectId {
        match self {
            GrantLifetime::UntilEndOfTurn { source_id, .. } => *source_id,
            GrantLifetime::WhileSourceOnBattlefield(source_id) => *source_id,
        }
    }
}

impl GrantSource {
    pub fn lifetime(&self) -> GrantLifetime {
        match self {
            GrantSource::Effect {
                expires_end_of_turn,
                source_id,
            } => GrantLifetime::UntilEndOfTurn {
                source_id: *source_id,
                turn: *expires_end_of_turn,
            },
            GrantSource::StaticAbility { source_id } => {
                GrantLifetime::WhileSourceOnBattlefield(*source_id)
            }
        }
    }
}

/// A granted alternative casting method for a specific card.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantedAlternativeCast {
    pub method: AlternativeCastingMethod,
    pub source_id: ObjectId,
    pub zone: Zone,
}

/// A grant that allows playing cards from a zone as though from hand.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantedPlayFrom {
    pub source_id: ObjectId,
    pub zone: Zone,
}

/// A unified grant that can represent either an ability or alternative casting method.
#[derive(Debug, Clone, PartialEq)]
pub struct Grant {
    /// The specific card that receives this grant (for targeted grants like Snapcaster).
    /// If None, uses the filter instead.
    pub target_id: Option<ObjectId>,
    /// Filter for cards that receive this grant (for blanket grants like Underworld Breach).
    /// Only used if target_id is None.
    pub filter: Option<ObjectFilter>,
    /// The zone where this grant applies.
    pub zone: Zone,
    /// The player who can use this grant.
    pub player: PlayerId,
    /// What is being granted (ability or alternative casting method).
    pub grantable: Grantable,
    /// How this grant was created.
    pub source: GrantSource,
}

/// Registry for tracking all granted effects.
#[derive(Debug, Clone, Default)]
pub struct GrantRegistry {
    /// All grants (unified storage).
    pub grants: Vec<Grant>,
}

impl GrantRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a grant to the registry.
    pub fn add_grant(&mut self, grant: Grant) {
        self.grants.push(grant);
    }

    /// Add a grant for a specific card.
    pub fn grant_to_card(
        &mut self,
        target_id: ObjectId,
        zone: Zone,
        player: PlayerId,
        grantable: Grantable,
        source: GrantSource,
    ) {
        self.grants.push(Grant {
            target_id: Some(target_id),
            filter: None,
            zone,
            player,
            grantable,
            source,
        });
    }

    /// Add a grant for cards matching a filter.
    pub fn grant_to_filter(
        &mut self,
        filter: ObjectFilter,
        zone: Zone,
        player: PlayerId,
        grantable: Grantable,
        source: GrantSource,
    ) {
        let filter = normalize_grant_filter(filter);
        self.grants.push(Grant {
            target_id: None,
            filter: Some(filter),
            zone,
            player,
            grantable,
            source,
        });
    }

    /// Add a filter grant from a resolving effect until end of turn.
    pub fn grant_to_filter_until_end_of_turn(
        &mut self,
        filter: ObjectFilter,
        zone: Zone,
        player: PlayerId,
        grantable: Grantable,
        source_id: ObjectId,
        turn: u32,
    ) {
        self.grant_to_filter(
            filter,
            zone,
            player,
            grantable,
            GrantSource::until_end_of_turn(source_id, turn),
        );
    }

    /// Add an alternative cast grant for a specific card.
    pub fn grant_alternative_cast_to_card(
        &mut self,
        target_id: ObjectId,
        zone: Zone,
        player: PlayerId,
        method: AlternativeCastingMethod,
        source: GrantSource,
    ) {
        self.grant_to_card(
            target_id,
            zone,
            player,
            Grantable::AlternativeCast(method),
            source,
        );
    }

    /// Add an alternative cast grant for cards matching a filter.
    pub fn grant_alternative_cast_to_filter(
        &mut self,
        filter: ObjectFilter,
        zone: Zone,
        player: PlayerId,
        method: AlternativeCastingMethod,
        source: GrantSource,
    ) {
        self.grant_to_filter(
            filter,
            zone,
            player,
            Grantable::AlternativeCast(method),
            source,
        );
    }

    /// Add an ability grant for cards matching a filter.
    pub fn grant_ability(
        &mut self,
        filter: ObjectFilter,
        zone: Zone,
        player: PlayerId,
        ability: StaticAbility,
        source: GrantSource,
    ) {
        self.grant_to_filter(filter, zone, player, Grantable::Ability(ability), source);
    }

    /// Add an ability grant for a specific card.
    ///
    /// This is used for one-shot effects that grant abilities to a specific target
    /// (e.g., "target creature gains flying until end of turn").
    pub fn grant_ability_to_card(
        &mut self,
        target_id: ObjectId,
        zone: Zone,
        player: PlayerId,
        ability: StaticAbility,
        source: GrantSource,
    ) {
        self.grant_to_card(target_id, zone, player, Grantable::Ability(ability), source);
    }

    /// Get all grants for a specific card.
    ///
    /// This returns grants from both:
    /// - Stored grants (effect-based like Snapcaster Mage)
    /// - Static ability grants computed on-the-fly (like Underworld Breach, Valley Floodcaller)
    pub fn get_grants_for_card(
        &self,
        game: &crate::game_state::GameState,
        card_id: ObjectId,
        card_zone: Zone,
        player: PlayerId,
    ) -> Vec<Grant> {
        let mut result = Vec::new();

        // Build filter context once
        let ctx = game.filter_context_for(player, None);

        // 1. Collect stored grants
        for grant in &self.grants {
            if matches!(grant.source, GrantSource::StaticAbility { .. }) {
                continue;
            }

            // Check if grant is still valid
            if !grant.source.is_valid(game) {
                continue;
            }

            // Check player matches
            if grant.player != player {
                continue;
            }

            // Check zone matches
            if grant.zone != card_zone {
                continue;
            }

            // Check if this grant applies to this card
            let matches = if let Some(target_id) = grant.target_id {
                // Targeted grant - must match exactly
                target_id == card_id
            } else if let Some(ref filter) = grant.filter {
                // Filter-based grant - check if card matches filter
                if let Some(card) = game.object(card_id) {
                    filter.matches(card, &ctx, game)
                } else {
                    false
                }
            } else {
                false
            };

            if matches {
                result.push(grant.clone());
            }
        }

        // 2. Compute grants from static abilities on demand so static and
        // effect-based grants don't drift apart.
        let card = match game.object(card_id) {
            Some(c) => c,
            None => return result,
        };
        for grant in self.static_grants(game) {
            if grant.player != player || grant.zone != card_zone {
                continue;
            }

            let matches = if let Some(target_id) = grant.target_id {
                target_id == card_id
            } else if let Some(ref filter) = grant.filter {
                filter.matches(card, &ctx, game)
            } else {
                false
            };

            if matches {
                result.push(grant);
            }
        }

        result
    }

    /// Check if a card has a specific granted ability.
    pub fn card_has_granted_ability(
        &self,
        game: &crate::game_state::GameState,
        card_id: ObjectId,
        card_zone: Zone,
        player: PlayerId,
        ability: &StaticAbility,
    ) -> bool {
        self.get_grants_for_card(game, card_id, card_zone, player)
            .iter()
            .any(|grant| match &grant.grantable {
                Grantable::Ability(a) => a == ability,
                _ => false,
            })
    }

    /// Check if a card has been granted "play from zone" (Yawgmoth's Will, etc.).
    pub fn card_can_play_from_zone(
        &self,
        game: &crate::game_state::GameState,
        card_id: ObjectId,
        zone: Zone,
        player: PlayerId,
    ) -> bool {
        self.get_grants_for_card(game, card_id, zone, player)
            .iter()
            .any(|grant| matches!(grant.grantable, Grantable::PlayFrom))
    }

    /// Get all granted alternative casting methods for a card.
    pub fn granted_alternative_casts_for_card(
        &self,
        game: &crate::game_state::GameState,
        card_id: ObjectId,
        zone: Zone,
        player: PlayerId,
    ) -> Vec<GrantedAlternativeCast> {
        self.get_grants_for_card(game, card_id, zone, player)
            .into_iter()
            .filter_map(|grant| materialize_granted_alternative_cast(game, card_id, grant))
            .collect()
    }

    /// Get all "play from zone" grants for a card.
    pub fn granted_play_from_for_card(
        &self,
        game: &crate::game_state::GameState,
        card_id: ObjectId,
        zone: Zone,
        player: PlayerId,
    ) -> Vec<GrantedPlayFrom> {
        self.get_grants_for_card(game, card_id, zone, player)
            .into_iter()
            .filter_map(|grant| match grant.grantable {
                Grantable::PlayFrom => Some(GrantedPlayFrom {
                    source_id: grant.source.source_id(),
                    zone: grant.zone,
                }),
                _ => None,
            })
            .collect()
    }

    /// Remove all grants from a specific source.
    pub fn remove_grants_from_source(&mut self, source_id: ObjectId) {
        self.grants.retain(|grant| {
            !matches!(&grant.source,
                GrantSource::Effect { source_id: sid, .. } |
                GrantSource::StaticAbility { source_id: sid }
                if *sid == source_id
            )
        });
    }

    /// Clean up expired grants (call at end of turn).
    pub fn cleanup_expired(&mut self, turn_number: u32, battlefield: &[ObjectId]) {
        self.grants
            .retain(|grant| grant.source.is_valid_raw(turn_number, battlefield));
    }

    /// Snapshot currently active grants, including static grants computed on demand.
    pub fn active_grants(&self, game: &crate::game_state::GameState) -> Vec<Grant> {
        let mut active: Vec<Grant> = self
            .grants
            .iter()
            .filter(|grant| {
                !matches!(grant.source, GrantSource::StaticAbility { .. })
                    && grant.source.is_valid(game)
            })
            .cloned()
            .collect();
        active.extend(self.static_grants(game));
        active
    }

    fn static_grants(&self, game: &crate::game_state::GameState) -> Vec<Grant> {
        use crate::ability::AbilityKind;
        use crate::game_loop::player_matches_filter_with_combat;

        let mut grants = Vec::new();

        for &perm_id in &game.battlefield {
            let Some(perm) = game.object(perm_id) else {
                continue;
            };

            let controller = perm.controller;

            for ability in &perm.abilities {
                if let AbilityKind::Static(s) = &ability.kind {
                    if let Some(spec) = s.grant_spec() {
                        let combat = game.combat.as_ref();
                        for player in game.players.iter().filter(|player| {
                            player.is_in_game()
                                && player_matches_filter_with_combat(
                                    player.id,
                                    &spec.beneficiary,
                                    game,
                                    controller,
                                    combat,
                                )
                        }) {
                            grants.push(Grant {
                                target_id: None,
                                filter: Some(normalize_grant_filter(spec.filter.clone())),
                                zone: spec.zone,
                                player: player.id,
                                grantable: spec.grantable.clone(),
                                source: GrantSource::StaticAbility { source_id: perm_id },
                            });
                        }
                    }
                }
            }
        }

        grants
    }
}

fn materialize_granted_alternative_cast(
    game: &crate::game_state::GameState,
    card_id: ObjectId,
    grant: Grant,
) -> Option<GrantedAlternativeCast> {
    let method = match grant.grantable {
        Grantable::AlternativeCast(method) => method,
        Grantable::DerivedAlternativeCast(spec) => {
            let card = game.object(card_id)?;
            materialize_derived_alternative_cast(card, spec)?
        }
        Grantable::Ability(_) | Grantable::PlayFrom => return None,
    };

    Some(GrantedAlternativeCast {
        method,
        source_id: grant.source.source_id(),
        zone: grant.zone,
    })
}

fn materialize_derived_alternative_cast(
    card: &crate::object::Object,
    spec: DerivedAlternativeCast,
) -> Option<AlternativeCastingMethod> {
    spec.materialize_for(card)
}

fn normalize_grant_filter(mut filter: ObjectFilter) -> ObjectFilter {
    dedupe_vec(&mut filter.card_types);
    dedupe_vec(&mut filter.all_card_types);
    dedupe_vec(&mut filter.excluded_card_types);
    dedupe_vec(&mut filter.subtypes);
    dedupe_vec(&mut filter.excluded_subtypes);
    dedupe_vec(&mut filter.supertypes);
    dedupe_vec(&mut filter.excluded_supertypes);
    filter
}

fn dedupe_vec<T: Eq + std::hash::Hash + Copy>(values: &mut Vec<T>) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(*value));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::card::CardBuilder;
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::ManaCost;
    use crate::target::PlayerFilter;
    use crate::types::CardType;

    #[test]
    fn test_grant_registry_creation() {
        let registry = GrantRegistry::new();
        assert!(registry.grants.is_empty());
    }

    #[test]
    fn test_unified_grant_storage() {
        let mut registry = GrantRegistry::new();
        let player = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let target_id = ObjectId::from_raw(2);

        // Add an alternative cast grant
        registry.grant_alternative_cast_to_card(
            target_id,
            Zone::Graveyard,
            player,
            AlternativeCastingMethod::Flashback {
                total_cost: crate::cost::TotalCost::mana(ManaCost::new()),
            },
            GrantSource::Effect {
                source_id,
                expires_end_of_turn: 1,
            },
        );

        // Add an ability grant
        registry.grant_ability(
            ObjectFilter::default(),
            Zone::Hand,
            player,
            StaticAbility::flash(),
            GrantSource::StaticAbility { source_id },
        );

        // Both should be in the unified grants list
        assert_eq!(registry.grants.len(), 2);

        // First grant should be alternative cast
        assert!(matches!(
            &registry.grants[0].grantable,
            Grantable::AlternativeCast(AlternativeCastingMethod::Flashback { .. })
        ));

        // Second grant should be ability
        assert!(matches!(
            &registry.grants[1].grantable,
            Grantable::Ability(_)
        ));
    }

    #[test]
    fn test_grant_to_filter_until_end_of_turn_uses_effect_source() {
        let mut registry = GrantRegistry::new();
        let player = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(7);

        registry.grant_to_filter_until_end_of_turn(
            ObjectFilter::nonland(),
            Zone::Graveyard,
            player,
            Grantable::play_from(),
            source_id,
            3,
        );

        assert_eq!(registry.grants.len(), 1);
        assert_eq!(
            registry.grants[0].source,
            GrantSource::until_end_of_turn(source_id, 3)
        );
    }

    #[test]
    fn test_static_any_player_flash_grant_applies_to_each_player() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = CardBuilder::new(crate::ids::CardId::from_raw(50), "Shared Flash")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("source permanent should exist")
            .abilities
            .push(Ability::static_ability(StaticAbility::grants(
                crate::grant::GrantSpec::flash_to_spells().with_beneficiary(PlayerFilter::Any),
            )));

        let alice_spell = CardBuilder::new(crate::ids::CardId::from_raw(51), "Alice Spell")
            .card_types(vec![CardType::Sorcery])
            .build();
        let bob_spell = CardBuilder::new(crate::ids::CardId::from_raw(52), "Bob Spell")
            .card_types(vec![CardType::Sorcery])
            .build();
        let alice_spell_id = game.create_object_from_card(&alice_spell, alice, Zone::Hand);
        let bob_spell_id = game.create_object_from_card(&bob_spell, bob, Zone::Hand);

        let flash = StaticAbility::flash();
        assert!(game.grant_registry.card_has_granted_ability(
            &game,
            alice_spell_id,
            Zone::Hand,
            alice,
            &flash,
        ));
        assert!(game.grant_registry.card_has_granted_ability(
            &game,
            bob_spell_id,
            Zone::Hand,
            bob,
            &flash,
        ));
    }
}
