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
use crate::filter::ObjectFilterExt as _;
use crate::grant::{
    DerivedAlternativeCast, DerivedAlternativeCastRuntimeExt, GrantUsageLimit, Grantable,
};
use crate::ids::{ObjectId, PlayerId, StableId};
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
    /// From a resolving effect that lasts while the source remains controlled by a player.
    EffectWhileControlled {
        source_id: ObjectId,
        controller: PlayerId,
    },
    /// From a resolving effect that lasts until end of turn only while a
    /// particular physical card remains on top of a player's library.
    EffectWhileStableCardOnTopOfLibrary {
        source_id: ObjectId,
        expires_end_of_turn: u32,
        stable_id: StableId,
        player: PlayerId,
        library_top_revision: u64,
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
            GrantSource::EffectWhileControlled { source_id, .. } => *source_id,
            GrantSource::EffectWhileStableCardOnTopOfLibrary { source_id, .. } => *source_id,
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
            GrantSource::EffectWhileControlled {
                source_id,
                controller,
            } => game.object(*source_id).is_some_and(|source| {
                source.zone == Zone::Battlefield && game.controller_of(source) == *controller
            }),
            GrantSource::EffectWhileStableCardOnTopOfLibrary {
                expires_end_of_turn,
                stable_id,
                player,
                library_top_revision,
                ..
            } => {
                game.turn.turn_number <= *expires_end_of_turn
                    && stable_card_is_top_of_library_at_revision(
                        game,
                        *stable_id,
                        *player,
                        *library_top_revision,
                    )
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
            GrantSource::EffectWhileControlled { source_id, .. } => battlefield.contains(source_id),
            GrantSource::EffectWhileStableCardOnTopOfLibrary {
                expires_end_of_turn,
                ..
            } => turn_number <= *expires_end_of_turn,
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
    WhileSourceControlledBy {
        source_id: ObjectId,
        controller: PlayerId,
    },
    WhileStableCardOnTopOfLibrary {
        source_id: ObjectId,
        stable_id: StableId,
        player: PlayerId,
        turn: u32,
        library_top_revision: u64,
    },
}

impl GrantLifetime {
    pub fn source_id(&self) -> ObjectId {
        match self {
            GrantLifetime::UntilEndOfTurn { source_id, .. } => *source_id,
            GrantLifetime::WhileSourceOnBattlefield(source_id) => *source_id,
            GrantLifetime::WhileSourceControlledBy { source_id, .. } => *source_id,
            GrantLifetime::WhileStableCardOnTopOfLibrary { source_id, .. } => *source_id,
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
            GrantSource::EffectWhileControlled {
                source_id,
                controller,
            } => GrantLifetime::WhileSourceControlledBy {
                source_id: *source_id,
                controller: *controller,
            },
            GrantSource::EffectWhileStableCardOnTopOfLibrary {
                source_id,
                stable_id,
                player,
                expires_end_of_turn,
                library_top_revision,
            } => GrantLifetime::WhileStableCardOnTopOfLibrary {
                source_id: *source_id,
                stable_id: *stable_id,
                player: *player,
                turn: *expires_end_of_turn,
                library_top_revision: *library_top_revision,
            },
        }
    }
}

pub(crate) fn stable_card_is_top_of_library_at_revision(
    game: &crate::game_state::GameState,
    stable_id: StableId,
    player: PlayerId,
    library_top_revision: u64,
) -> bool {
    game.library_top_revision(player) == library_top_revision
        && stable_card_is_top_of_library(game, stable_id, player)
}

pub(crate) fn stable_card_is_top_of_library(
    game: &crate::game_state::GameState,
    stable_id: StableId,
    player: PlayerId,
) -> bool {
    let Some(player_state) = game.player(player) else {
        return false;
    };
    let Some(top_id) = player_state.library.last().copied() else {
        return false;
    };
    game.object(top_id)
        .is_some_and(|object| object.stable_id == stable_id)
}

/// A granted alternative casting method for a specific card.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantedAlternativeCast {
    pub method: AlternativeCastingMethod,
    pub source_id: ObjectId,
    pub zone: Zone,
    pub usage_limit: Option<GrantUsageLimit>,
}

/// A grant that allows playing cards from a zone as though from hand.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantedPlayFrom {
    pub source_id: ObjectId,
    pub zone: Zone,
    pub usage_limit: Option<GrantUsageLimit>,
}

/// A unified grant that can represent either an ability or alternative casting method.
#[derive(Debug, Clone, PartialEq)]
pub struct Grant {
    /// The specific card that receives this grant (for targeted grants like Snapcaster).
    /// If None, uses the filter instead.
    pub target_id: Option<ObjectId>,
    /// Stable card identity for targeted grants that track "that card" across zone changes.
    pub target_stable_id: Option<StableId>,
    /// Filter for cards that receive this grant (for blanket grants like Underworld Breach).
    /// Only used if target_id is None.
    pub filter: Option<ObjectFilter>,
    /// The zone where this grant applies.
    pub zone: Zone,
    /// The player who can use this grant.
    pub player: PlayerId,
    /// What is being granted (ability or alternative casting method).
    pub grantable: Grantable,
    /// How often this grant may be used from the same source.
    pub usage_limit: Option<GrantUsageLimit>,
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
            target_stable_id: None,
            filter: None,
            zone,
            player,
            grantable,
            usage_limit: None,
            source,
        });
    }

    /// Add a grant for a specific physical card that should survive object-id changes.
    pub fn grant_to_stable_card(
        &mut self,
        target_id: ObjectId,
        target_stable_id: StableId,
        zone: Zone,
        player: PlayerId,
        grantable: Grantable,
        source: GrantSource,
    ) {
        self.grants.push(Grant {
            target_id: Some(target_id),
            target_stable_id: Some(target_stable_id),
            filter: None,
            zone,
            player,
            grantable,
            usage_limit: None,
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
            target_stable_id: None,
            filter: Some(filter),
            zone,
            player,
            grantable,
            usage_limit: None,
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

    /// Add an alternative cast grant for a specific physical card that should survive object-id changes.
    pub fn grant_alternative_cast_to_stable_card(
        &mut self,
        target_id: ObjectId,
        target_stable_id: StableId,
        zone: Zone,
        player: PlayerId,
        method: AlternativeCastingMethod,
        source: GrantSource,
    ) {
        self.grant_to_stable_card(
            target_id,
            target_stable_id,
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
        let card = game.object(card_id);
        let card_stable_id = card.map(|card| card.stable_id);

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
                // Targeted grant - match the current object id, or the stable card
                // identity when the permission explicitly tracks "that card".
                target_id == card_id
                    || grant.target_stable_id.zip(card_stable_id).is_some_and(
                        |(target_stable_id, card_stable_id)| target_stable_id == card_stable_id,
                    )
            } else if let Some(ref filter) = grant.filter {
                // Filter-based grant - check if card matches filter
                if let Some(card) = card {
                    filter.matches(card, &grant_filter_context(&ctx, grant, game), game)
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
        let card = match card {
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
                filter.matches(card, &grant_filter_context(&ctx, &grant, game), game)
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
                    usage_limit: grant.usage_limit,
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
                GrantSource::EffectWhileControlled { source_id: sid, .. } |
                GrantSource::EffectWhileStableCardOnTopOfLibrary { source_id: sid, .. } |
                GrantSource::StaticAbility { source_id: sid }
                if *sid == source_id
            )
        });
    }

    /// Remove grants tied to a physical card's continuous presence in a zone.
    pub fn remove_stable_card_grants_for_zone(&mut self, stable_id: StableId, zone: Zone) {
        self.grants
            .retain(|grant| grant.target_stable_id != Some(stable_id) || grant.zone != zone);
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

        let mut collect_from_source = |source_id: ObjectId, source_is_battlefield: bool| {
            let Some(source) = game.object(source_id) else {
                return;
            };

            let controller = game.controller_of(source);

            for ability in &source.abilities {
                let AbilityKind::Static(s) = &ability.kind else {
                    continue;
                };
                if !s.is_active(game, source_id) {
                    continue;
                }
                let Some(spec) = s.grant_spec() else {
                    continue;
                };

                let is_source_self_grant = spec.filter == ObjectFilter::source();
                if !source_is_battlefield
                    && source.zone != Zone::Command
                    && (!is_source_self_grant || spec.zone != source.zone)
                {
                    continue;
                }

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
                        target_id: is_source_self_grant.then_some(source_id),
                        target_stable_id: None,
                        filter: (!is_source_self_grant)
                            .then(|| normalize_grant_filter(spec.filter.clone())),
                        zone: spec.zone,
                        player: player.id,
                        grantable: spec.grantable.clone(),
                        usage_limit: spec.usage_limit,
                        source: GrantSource::StaticAbility { source_id },
                    });
                }
            }
        };

        for &perm_id in &game.battlefield {
            collect_from_source(perm_id, true);
        }

        for zone in [Zone::Graveyard, Zone::Exile, Zone::Command] {
            for source_id in crate::object_query::candidate_ids_for_zone(game, Some(zone)) {
                if game.battlefield.contains(&source_id) {
                    continue;
                }
                collect_from_source(source_id, false);
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
    let (method, usage_limit) = match grant.grantable {
        Grantable::AlternativeCast(method) => (method, None),
        Grantable::DerivedAlternativeCast(spec) => {
            let card = game.object(card_id)?;
            let usage_limit = spec.usage_limit();
            (
                materialize_derived_alternative_cast(card, spec)?,
                usage_limit,
            )
        }
        Grantable::Ability(_) | Grantable::PlayFrom => return None,
    };

    Some(GrantedAlternativeCast {
        method,
        source_id: grant.source.source_id(),
        zone: grant.zone,
        usage_limit,
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

fn grant_filter_context(
    ctx: &crate::filter::FilterContext,
    grant: &Grant,
    game: &crate::game_state::GameState,
) -> crate::filter::FilterContext {
    let mut ctx = ctx.clone();
    let source_id = grant.source.source_id();
    let source_exiled = game
        .get_exiled_with_source_links(source_id)
        .iter()
        .filter_map(|id| {
            game.object(*id)
                .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        ctx.tagged_objects
            .insert(crate::tag::SOURCE_EXILED_TAG.into(), source_exiled);
    }
    ctx
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
    use crate::events::{KeywordActionEvent, KeywordActionKind, RawEvent};
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::ManaCost;
    use crate::provenance::ProvNodeId;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::{SURVEILLED_THIS_TURN_TAG, TagKey};
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use std::collections::HashMap;

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
    fn test_static_self_play_from_graveyard_grant_is_active_in_graveyard() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(crate::ids::CardId::from_raw(60), "Max Speed Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::new())
            .build();
        let card_id = game.create_object_from_card(&card, alice, Zone::Graveyard);
        let max_speed = crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::Speed(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(4),
        };
        let play_self = StaticAbility::grants(crate::grant::GrantSpec::new(
            Grantable::play_from(),
            ObjectFilter::source(),
            Zone::Graveyard,
        ))
        .with_condition(max_speed)
        .expect("play-from grant should accept a static condition");
        game.object_mut(card_id)
            .expect("graveyard card should exist")
            .abilities
            .push(Ability::static_ability(play_self));

        assert!(
            game.effect_store
                .grant_registry
                .granted_play_from_for_card(&game, card_id, Zone::Graveyard, alice)
                .is_empty(),
            "self grant should be gated before max speed"
        );

        game.increase_speed(alice, 4);

        assert_eq!(
            game.effect_store
                .grant_registry
                .granted_play_from_for_card(&game, card_id, Zone::Graveyard, alice)
                .len(),
            1,
            "self grant should apply to the card while it is in the graveyard"
        );
    }

    #[test]
    fn test_static_graveyard_cast_grant_materializes_with_usage_limit() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = CardBuilder::new(crate::ids::CardId::from_raw(61), "Broodship Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        let grant_spec = crate::grant::GrantSpec::new(
            Grantable::once_each_turn_graveyard_cast_from_cards_mana_cost(vec![
                crate::costs::Cost::sacrifice(ObjectFilter::land().you_control()),
            ]),
            ObjectFilter {
                card_types: vec![CardType::Artifact],
                ..ObjectFilter::default()
            },
            Zone::Graveyard,
        );
        game.object_mut(source_id)
            .expect("source permanent should exist")
            .abilities
            .push(Ability::static_ability(StaticAbility::grants(grant_spec)));

        let card = CardBuilder::new(crate::ids::CardId::from_raw(62), "Buried Artifact")
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::new())
            .build();
        let card_id = game.create_object_from_card(&card, alice, Zone::Graveyard);

        let grants = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, card_id, Zone::Graveyard, alice);
        assert_eq!(grants.len(), 1);
        assert_eq!(
            grants[0].usage_limit,
            Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns)
        );
        assert_eq!(grants[0].method.cast_from_zone(), Zone::Graveyard);
        assert!(!grants[0].method.exiles_after_resolution());
        assert_eq!(grants[0].method.non_mana_costs().len(), 1);
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
        assert!(game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            alice_spell_id,
            Zone::Hand,
            alice,
            &flash,
        ));
        assert!(game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            bob_spell_id,
            Zone::Hand,
            bob,
            &flash,
        ));
    }

    #[test]
    fn surveilled_graveyard_static_grants_apply_only_to_surveilled_cards() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let mut filter = ObjectFilter {
            zone: Some(Zone::Graveyard),
            owner: Some(PlayerFilter::You),
            surveilled_this_turn: true,
            ..Default::default()
        };
        let play_grant = StaticAbility::grants(
            crate::grant::GrantSpec::new(Grantable::play_from(), filter.clone(), Zone::Graveyard)
                .with_beneficiary(PlayerFilter::You),
        );
        filter.excluded_card_types.push(CardType::Land);
        let life_grant = StaticAbility::grants(
            crate::grant::GrantSpec::new(
                Grantable::life_equal_mana_value_from_zone(Zone::Graveyard, None),
                filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        );
        let source_card = CardBuilder::new(crate::ids::CardId::from_raw(91_510), "Eye Stand-In")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("source should exist")
            .abilities
            .extend([
                Ability::static_ability(play_grant),
                Ability::static_ability(life_grant),
            ]);

        let spell_card = CardBuilder::new(crate::ids::CardId::from_raw(91_511), "Seen Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::new())
            .build();
        let seen_spell = game.create_object_from_card(&spell_card, alice, Zone::Graveyard);
        let unseen_spell = game.create_object_from_card(&spell_card, alice, Zone::Graveyard);
        let land_card = CardBuilder::new(crate::ids::CardId::from_raw(91_512), "Seen Land")
            .card_types(vec![CardType::Land])
            .build();
        let seen_land = game.create_object_from_card(&land_card, alice, Zone::Graveyard);

        let mut object_tags = HashMap::new();
        object_tags.insert(
            TagKey::from(SURVEILLED_THIS_TURN_TAG),
            vec![
                ObjectSnapshot::from_object(game.object(seen_spell).unwrap(), &game),
                ObjectSnapshot::from_object(game.object(seen_land).unwrap(), &game),
            ],
        );
        let event = RawEvent::new(
            KeywordActionEvent::new(KeywordActionKind::Surveil, alice, source_id, 2)
                .with_object_tags(object_tags),
            ProvNodeId::default(),
        );
        game.turn_store
            .turn_history
            .record_event(&event, None, None);

        assert_eq!(
            game.effect_store
                .grant_registry
                .granted_play_from_for_card(&game, seen_spell, Zone::Graveyard, alice)
                .len(),
            1,
            "surveilled graveyard spell should be playable"
        );
        assert_eq!(
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, seen_spell, Zone::Graveyard, alice)
                .len(),
            1,
            "surveilled graveyard spell should get the life-cost cast"
        );
        assert!(
            game.effect_store
                .grant_registry
                .granted_play_from_for_card(&game, unseen_spell, Zone::Graveyard, alice)
                .is_empty(),
            "non-surveilled graveyard spell should not be playable"
        );
        assert_eq!(
            game.effect_store
                .grant_registry
                .granted_play_from_for_card(&game, seen_land, Zone::Graveyard, alice)
                .len(),
            1,
            "surveilled graveyard land should be playable"
        );
        assert!(
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, seen_land, Zone::Graveyard, alice)
                .is_empty(),
            "surveilled graveyard land should not get a spell life-cost alternative"
        );
    }
}
