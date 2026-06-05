//! Grant temporary "you may cast/play this tagged card" permissions.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::grant::Grantable;
use crate::grant_registry::GrantSource;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
pub use ironsmith_core::GrantPlayTaggedDuration;

/// Grant temporary permission to cast or play cards tagged in the current context.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantPlayTaggedEffect {
    pub tag: TagKey,
    pub player: PlayerFilter,
    pub duration: GrantPlayTaggedDuration,
    pub allow_land: bool,
    pub allow_any_color_for_cast: bool,
    pub while_on_top_of_library: bool,
    pub filter: Option<ObjectFilter>,
}

impl GrantPlayTaggedEffect {
    pub fn new(
        tag: impl Into<TagKey>,
        player: PlayerFilter,
        duration: GrantPlayTaggedDuration,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    ) -> Self {
        Self {
            tag: tag.into(),
            player,
            duration,
            allow_land,
            allow_any_color_for_cast,
            while_on_top_of_library: false,
            filter: None,
        }
    }

    pub fn while_on_top_of_library(mut self) -> Self {
        self.while_on_top_of_library = true;
        self
    }

    pub fn while_on_top_of_library_if(mut self, enabled: bool) -> Self {
        self.while_on_top_of_library = enabled;
        self
    }

    pub fn with_filter(mut self, filter: ObjectFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn until_your_next_turn(tag: impl Into<TagKey>, player: PlayerFilter) -> Self {
        Self::new(
            tag,
            player,
            GrantPlayTaggedDuration::UntilYourNextTurnEnd,
            true,
            false,
        )
    }

    /// Compute the turn number corresponding to the end of `player`'s next turn.
    ///
    /// This simulates `GameState::next_turn` turn selection logic (including
    /// multiplayer turn order, queued extra turns, and skipped turns) without
    /// mutating game state.
    fn next_turn_number_for_player(game: &GameState, player: crate::ids::PlayerId) -> u32 {
        if game.turn_store.turn_order.is_empty() {
            return game.turn.turn_number;
        }

        let mut simulated_active_player = game.turn.active_player;
        let mut simulated_turn_number = game.turn.turn_number;
        let mut simulated_extra_turns = game.turn_store.extra_turns.clone();
        let mut simulated_skip_next_turn = game.turn_store.skip_next_turn.clone();

        // Defensive bound to avoid pathological infinite loops if state is invalid.
        let max_iterations = game
            .turn_store
            .turn_order
            .len()
            .saturating_mul(16)
            .saturating_add(simulated_extra_turns.len().saturating_mul(2))
            .saturating_add(16)
            .max(1);

        for _ in 0..max_iterations {
            let next_player = if !simulated_extra_turns.is_empty() {
                simulated_extra_turns.remove(0)
            } else {
                let current_index = game
                    .turn_store
                    .turn_order
                    .iter()
                    .position(|&p| p == simulated_active_player)
                    .unwrap_or(0);

                let mut next_index = (current_index + 1) % game.turn_store.turn_order.len();
                let start_index = next_index;

                loop {
                    let candidate = game.turn_store.turn_order[next_index];
                    let is_in_game = game.player(candidate).is_some_and(|p| p.is_in_game());

                    if is_in_game {
                        if simulated_skip_next_turn.remove(&candidate) {
                            next_index = (next_index + 1) % game.turn_store.turn_order.len();
                            if next_index == start_index {
                                break;
                            }
                            continue;
                        }
                        break;
                    }

                    next_index = (next_index + 1) % game.turn_store.turn_order.len();
                    if next_index == start_index {
                        break;
                    }
                }

                game.turn_store.turn_order[next_index]
            };

            simulated_turn_number = simulated_turn_number.saturating_add(1);
            simulated_active_player = next_player;
            if simulated_active_player == player {
                return simulated_turn_number;
            }
        }

        // Fallback should be unreachable in valid games.
        game.turn.turn_number.saturating_add(1)
    }

    fn expires_end_of_turn(&self, game: &GameState, player: crate::ids::PlayerId) -> u32 {
        match self.duration {
            GrantPlayTaggedDuration::UntilEndOfTurn => game.turn.turn_number,
            GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                Self::next_turn_number_for_player(game, player)
            }
            GrantPlayTaggedDuration::ForAsLongAsExiled => u32::MAX,
            GrantPlayTaggedDuration::ForAsLongAsYouControlSource => u32::MAX,
        }
    }
}

impl EffectExecutor for GrantPlayTaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let snapshots = ctx.get_tagged_all(self.tag.as_str()).cloned().or_else(|| {
            (self.tag.as_str() == "__source_exiled__").then(|| {
                ctx.tagged_objects
                    .iter()
                    .filter(|(tag, _)| tag.as_str().starts_with("__sentence_helper_exiled"))
                    .flat_map(|(_, snapshots)| snapshots.iter().cloned())
                    .collect::<Vec<_>>()
            })
        });
        let Some(snapshots) = snapshots.filter(|snapshots| !snapshots.is_empty()) else {
            return Ok(EffectOutcome::count(0));
        };

        let expires_end_of_turn = self.expires_end_of_turn(game, player_id);
        let mut granted = 0usize;
        let mut seen = std::collections::HashSet::new();
        let mut mana_permission_stable_ids = Vec::new();
        for snapshot in snapshots {
            let mut object_id = snapshot.object_id;
            if game.object(object_id).is_none() {
                if let Some(found) = game.find_object_by_stable_id(snapshot.stable_id) {
                    object_id = found;
                } else {
                    continue;
                }
            }

            let Some(object) = game.object(object_id) else {
                continue;
            };
            let filter_ctx = ctx.filter_context(game);
            if self
                .filter
                .as_ref()
                .is_some_and(|filter| !filter.matches(object, &filter_ctx, game))
            {
                continue;
            }
            if (!self.allow_land && object.is_land()) || !seen.insert(object_id) {
                continue;
            }

            if self.allow_any_color_for_cast && !object.is_land() {
                mana_permission_stable_ids.push(object.stable_id);
            }

            let source = if self.while_on_top_of_library {
                GrantSource::EffectWhileStableCardOnTopOfLibrary {
                    source_id: ctx.source,
                    expires_end_of_turn,
                    stable_id: object.stable_id,
                    player: object.owner,
                    library_top_revision: game.library_top_revision(object.owner),
                }
            } else if self.duration == GrantPlayTaggedDuration::ForAsLongAsYouControlSource {
                GrantSource::EffectWhileControlled {
                    source_id: ctx.source,
                    controller: player_id,
                }
            } else {
                GrantSource::Effect {
                    source_id: ctx.source,
                    expires_end_of_turn,
                }
            };
            if self.duration == GrantPlayTaggedDuration::ForAsLongAsExiled {
                game.effect_store.grant_registry.grant_to_stable_card(
                    object_id,
                    object.stable_id,
                    object.zone,
                    player_id,
                    Grantable::PlayFrom,
                    source,
                );
            } else {
                game.effect_store.grant_registry.grant_to_card(
                    object_id,
                    object.zone,
                    player_id,
                    Grantable::PlayFrom,
                    source,
                );
            }
            granted += 1;
        }

        if self.allow_any_color_for_cast && !mana_permission_stable_ids.is_empty() {
            game.effect_store.mana_spend_effects.permissions.push(
                crate::game_state::ActiveManaSpendPermission {
                    permission:
                        crate::effect::ManaSpendPermission::any_color_for_casting_stable_ids(
                            crate::target::PlayerFilter::You,
                            mana_permission_stable_ids,
                        ),
                    controller: player_id,
                    source: crate::game_state::ManaSpendPermissionSource::Effect {
                        source_id: ctx.source,
                        expires_end_of_turn,
                    },
                },
            );
        }

        Ok(EffectOutcome::count(granted as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Zone;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use std::collections::HashSet;

    #[test]
    fn grant_play_tagged_until_your_next_turn_applies_to_tagged_exile_cards() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(1), "Exiled Card").build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled card"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(100);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "tagged card should be playable from exile"
        );

        let grant = game
            .effect_store
            .grant_registry
            .grants
            .first()
            .expect("grant should exist");
        match grant.source {
            GrantSource::Effect {
                expires_end_of_turn,
                ..
            } => {
                assert_eq!(
                    expires_end_of_turn,
                    game.turn.turn_number + 2,
                    "when cast on your own turn, permission should last through your next turn"
                );
            }
            _ => panic!("expected effect grant source"),
        }
    }

    #[test]
    fn grant_play_tagged_until_your_next_turn_uses_multiplayer_turn_order() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);

        // Alice is active now. In a 3-player game, Alice's next turn ends at +3.
        game.turn.active_player = alice;
        game.turn.turn_number = 10;

        let expires = GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You)
            .expires_end_of_turn(&game, alice);
        assert_eq!(
            expires, 13,
            "duration should last through Alice's next turn in multiplayer"
        );
    }

    #[test]
    fn grant_play_tagged_until_your_next_turn_respects_extra_and_skipped_turns() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Grant on Bob's turn with queued extra turn for Alice.
        game.turn.active_player = bob;
        game.turn.turn_number = 20;
        game.turn_store.extra_turns = vec![alice];
        let expires_with_extra =
            GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You)
                .expires_end_of_turn(&game, alice);
        assert_eq!(
            expires_with_extra, 21,
            "queued extra turn for Alice should make her next turn immediate"
        );

        // If Alice's next turn is skipped, duration should extend to the following turn she takes.
        game.turn_store.extra_turns.clear();
        game.turn.active_player = bob;
        game.turn.turn_number = 30;
        game.turn_store.skip_next_turn = HashSet::from([alice]);
        let expires_with_skip =
            GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You)
                .expires_end_of_turn(&game, alice);
        assert_eq!(
            expires_with_skip, 34,
            "skipped next turn should defer expiration to Alice's subsequent turn"
        );
    }

    #[test]
    fn grant_play_tagged_any_color_permission_survives_refresh_until_turn_ends() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(2), "Exiled Spell")
            .card_types(vec![crate::types::CardType::Instant])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled spell"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(101);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::UntilEndOfTurn,
            false,
            true,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert!(game.can_spend_mana_as_any_color(alice, Some(exiled_id)));

        game.update_cant_effects();
        assert!(
            game.can_spend_mana_as_any_color(alice, Some(exiled_id)),
            "temporary effect-sourced mana permission should survive tracker refreshes"
        );
    }

    #[test]
    fn grant_play_tagged_for_as_long_as_exiled_uses_open_ended_exile_permission() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(3), "Exiled Spell")
            .card_types(vec![crate::types::CardType::Instant])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled spell"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(102);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::ForAsLongAsExiled,
            true,
            true,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "tagged card should be playable from exile"
        );
        assert!(game.can_spend_mana_as_any_color(alice, Some(exiled_id)));

        game.turn.turn_number = game.turn.turn_number.saturating_add(20);
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "while-exiled grant should not expire at end of turn"
        );
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Graveyard,
                alice
            ),
            "grant remains tied to the exile zone"
        );

        let graveyard_id = game
            .move_object(
                exiled_id,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
            )
            .expect("test card should move to graveyard");
        assert!(
            !game.can_spend_mana_as_any_color(alice, Some(graveyard_id)),
            "any-mana permission for spells cast this way should not apply after the card leaves exile"
        );
    }

    #[test]
    fn gwen_stacy_grant_play_permission_ends_when_you_lose_control_of_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(10), "Gwen Stacy source").build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let card = CardBuilder::new(CardId::from_raw(11), "Exiled Card").build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled card"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
            true,
            false,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "Gwen Stacy permission should apply while you control the source"
        );

        game.set_current_controller(source_id, bob);
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "Gwen Stacy permission should end once you lose control of the source"
        );
    }

    #[test]
    fn gwen_stacy_grant_play_permission_survives_turn_changes_while_controlled() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(12), "Gwen Stacy source").build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let card = CardBuilder::new(CardId::from_raw(13), "Exiled Card").build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled card"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::new(
            "it",
            PlayerFilter::You,
            GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
            true,
            false,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        game.turn.turn_number = game.turn.turn_number.saturating_add(5);
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "Gwen Stacy permission should not expire by turn count while source stays controlled"
        );
    }
}
