//! Create token effect implementation.

use crate::cards::CardDefinition;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::lifecycle::{
    TokenCleanupOptions, TokenEntryOptions, apply_token_battlefield_entry,
    create_replacement_additional_tokens, remaining_token_slots, schedule_token_cleanup,
};

/// Effect that creates token creatures or other token permanents.
///
/// # Fields
///
/// * `token` - The token definition (use CardDefinitionBuilder with .token())
/// * `count` - How many tokens to create
/// * `controller` - Who controls the tokens
/// * `suppress_aura_attachment_choice` - Whether Aura token attachment is handled by a later effect
/// * `enters_tapped` - Whether the tokens enter tapped
/// * `enters_attacking` - Whether the tokens enter attacking
/// * `exile_at_end_of_combat` - Whether to exile the tokens at end of combat
/// * `sacrifice_at_end_of_combat` - Whether to sacrifice the tokens at end of combat
/// * `sacrifice_at_next_end_step` - Whether to sacrifice the tokens at the
///   beginning of the next end step.
/// * `exile_at_next_end_step` - Whether to exile the tokens at the beginning
///   of the next end step.
///
/// # Example
///
/// ```ignore
/// // Create two 1/1 white Soldier tokens
/// let soldier = CardDefinitionBuilder::new(CardId::new(), "Soldier")
///     .token()
///     .card_types(vec![CardType::Creature])
///     .subtypes(vec![Subtype::Soldier])
///     .color_indicator(ColorSet::WHITE)
///     .power_toughness(PowerToughness::fixed(1, 1))
///     .build();
/// let effect = CreateTokenEffect::new(soldier, 2, PlayerFilter::You);
///
/// // Create a 4/4 Angel token that enters tapped and attacking, exiled at EOC
/// let angel = CardDefinitionBuilder::new(CardId::new(), "Angel")
///     .token()
///     .card_types(vec![CardType::Creature])
///     .subtypes(vec![Subtype::Angel])
///     .color_indicator(ColorSet::WHITE)
///     .power_toughness(PowerToughness::fixed(4, 4))
///     .flying()
///     .build();
/// let effect = CreateTokenEffect::one(angel)
///     .tapped()
///     .attacking()
///     .exile_at_end_of_combat();
/// ```
pub type CreateTokenEffect = ironsmith_core::CreateTokenEffect<CardDefinition>;

impl EffectExecutor for CreateTokenEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Token creation involves no player choices; defer to commit so the
        // whole each-player action lands as one batch.
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }


    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id =
            crate::effects::helpers::resolve_player_filter(game, &self.controller, ctx)?;
        // CR 800.4b/800.4d: no token is created under the control of, or owned
        // by, a player who has left the game.
        if !game
            .player(controller_id)
            .is_some_and(|player| player.is_in_game())
        {
            return Ok(EffectOutcome::with_objects(Vec::new()));
        }
        let base_count = resolve_value(game, &self.count, ctx)?.max(0) as u32;
        let token_preview =
            game.object_from_token_definition(ObjectId::from_raw(0), &self.token, controller_id);
        let replacement = crate::events::processing::process_token_creation_for_token_with_event(
            game,
            controller_id,
            base_count,
            Some(token_preview.clone()),
            ctx.cause.clone(),
            &mut ctx.decision_maker,
        );
        let count = (replacement.count as usize).min(remaining_token_slots(game, controller_id));
        let cleanup_options = TokenCleanupOptions::new(
            self.exile_at_end_of_combat,
            self.sacrifice_at_end_of_combat,
            self.sacrifice_at_next_end_step,
            self.exile_at_next_end_step,
            self.next_end_step_player.clone(),
        );
        let entry_options = TokenEntryOptions::new(self.enters_tapped, self.enters_attacking);

        let mut created_ids = Vec::with_capacity(count);
        let mut events = Vec::with_capacity(count);
        let pending_start = game.effect_store.pending_trigger_events.len();

        for _ in 0..count {
            let id = game.new_object_id();
            let mut token_obj = game.object_from_token_definition(id, &self.token, controller_id);
            token_obj.zone = Zone::Command;
            let token_is_creature = token_obj.is_creature();

            game.add_object(token_obj);
            let entry_result = if self.suppress_aura_attachment_choice {
                game.move_object_with_etb_processing_without_aura_attachment_choice(
                    id,
                    Zone::Battlefield,
                    &mut ctx.decision_maker,
                )
            } else {
                game.move_object_with_etb_processing_with_dm(
                    id,
                    Zone::Battlefield,
                    &mut ctx.decision_maker,
                )
            };
            let Some(entry_result) = entry_result else {
                game.remove_object(id);
                continue;
            };
            let entered_id = entry_result.new_id;
            created_ids.push(entered_id);
            let entered_battlefield = game
                .object(entered_id)
                .is_some_and(|obj| obj.zone == Zone::Battlefield);

            if entered_battlefield {
                let effective_tapped = entry_result.enters_tapped || self.enters_tapped;
                let entered_is_creature = game.current_is_creature(entered_id);
                let tracks_creature_etb = entered_is_creature || token_is_creature;
                apply_token_battlefield_entry(
                    game,
                    ctx,
                    entered_id,
                    controller_id,
                    tracks_creature_etb,
                    entry_options,
                    Zone::Command,
                    effective_tapped,
                    &mut events,
                )?;

                schedule_token_cleanup(
                    game,
                    ctx,
                    entered_id,
                    controller_id,
                    cleanup_options.clone(),
                )?;
            }
        }

        let primary_created_count = created_ids.len() as u32;
        if primary_created_count > 0 {
            game.queue_trigger_event(
                ctx.provenance,
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CreateTokensEvent::with_token_cause(
                        controller_id,
                        primary_created_count,
                        token_preview,
                        ctx.cause.clone(),
                    ),
                    ctx.provenance,
                ),
            );
        }

        let additional_ids = create_replacement_additional_tokens(
            game,
            ctx,
            controller_id,
            &replacement.additional_tokens,
            &mut events,
        )?;
        created_ids.extend(additional_ids);

        if created_ids.len() > 1 {
            let batch_objects = created_ids.clone();
            let removed_events =
                game.remove_pending_trigger_events_matching_from(pending_start, |event| {
                    event
                        .downcast::<crate::events::zones::ZoneChangeEvent>()
                        .is_some_and(|zone_change| {
                            zone_change.from == Zone::Command
                                && zone_change.to == Zone::Battlefield
                                && zone_change
                                    .objects
                                    .iter()
                                    .all(|object_id| batch_objects.contains(object_id))
                        })
                });
            if !removed_events.is_empty() {
                let cause = removed_events
                    .iter()
                    .find_map(|event| {
                        event
                            .downcast::<crate::events::zones::ZoneChangeEvent>()
                            .map(|zone_change| zone_change.cause.clone())
                    })
                    .unwrap_or_else(crate::events::cause::EventCause::effect);
                let snapshots = removed_events
                    .iter()
                    .filter_map(|event| event.downcast::<crate::events::zones::ZoneChangeEvent>())
                    .flat_map(|zone_change| zone_change.snapshots().iter().cloned())
                    .collect();
                let event = crate::events::zones::ZoneChangeEvent::batch_with_snapshots(
                    created_ids.clone(),
                    Zone::Command,
                    Zone::Battlefield,
                    cause,
                    snapshots,
                );
                game.queue_trigger_event(
                    ctx.provenance,
                    crate::triggers::TriggerEvent::new_with_provenance(event, ctx.provenance),
                );
            }
        }

        let created_stable_ids: Vec<_> = created_ids
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.stable_id))
            .collect();
        if !created_stable_ids.is_empty() {
            game.record_ui_effect_event(
                "tokens_created",
                Some(controller_id),
                None,
                created_stable_ids,
                Some(created_ids.len() as i64),
                Some(self.token.card.name.to_string()),
            );
        }

        Ok(EffectOutcome::with_objects(created_ids.clone())
            .with_events(events)
            .with_affected_objects_from_game(game, created_ids))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.controller_target.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to create tokens"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::cards::definitions::tayam_luminous_enigma;
    use crate::cards::tokens::treasure_token_definition;
    use crate::color::{Color, ColorSet};
    use crate::compiled_text::canonical_compiled_lines;
    use crate::ids::{CardId, PlayerId};
    use crate::object::{CounterType, ObjectKind};
    use crate::static_abilities::StaticAbility;
    use crate::test_prelude::*;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn soldier_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Soldier")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Soldier])
            .color_indicator(ColorSet::from(Color::White))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn goblin_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Goblin")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Goblin])
            .color_indicator(ColorSet::from(Color::Red))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn zombie_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Zombie")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn beast_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Beast")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Beast])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    }

    fn spirit_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Spirit")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn xorn_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Xorn")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elemental])
            .parse_text(
                "If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token.",
            )
            .expect("Xorn should parse strictly")
    }

    fn fancy_treasure_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Fancy Treasure")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Treasure])
            .build()
    }

    #[test]
    fn xorn_strict_parser_and_compiled_text_regression() {
        let def = xorn_definition();
        let rendered = canonical_compiled_lines(&def).join(" ");

        assert_eq!(
            rendered,
            "If you would create one or more treasure tokens, instead create those tokens plus an additional treasure token."
        );
    }

    #[test]
    fn create_token_records_created_objects_as_affected_memory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = CreateTokenEffect::you(soldier_token(), 2)
            .execute(&mut game, &mut ctx)
            .expect("tokens should be created");

        let affected = outcome
            .affected_objects()
            .expect("created tokens should be affected objects");
        assert_eq!(affected.len(), 2);
        let memory = outcome
            .affected_object_memory()
            .expect("created token LKI should be recorded");
        assert_eq!(memory.len(), 2);
        assert!(memory.iter().all(|m| m.controller == alice));
        assert!(memory.iter().all(|m| m.is_token));
    }

    #[test]
    fn test_create_single_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::one(soldier_token());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.name, "Soldier");
            assert_eq!(token.kind, ObjectKind::Token);
            assert!(token.is_creature());
            assert_eq!(token.power(), Some(1));
            assert_eq!(token.toughness(), Some(1));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_multiple_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(goblin_token(), 3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 3);
            for id in ids {
                let token = game.object(id).unwrap();
                assert_eq!(token.name, "Goblin");
                assert_eq!(token.kind, ObjectKind::Token);
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn create_token_replacement_doubles_tokens_created_under_your_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let doubler = CardDefinitionBuilder::new(CardId::new(), "Token Doubler")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::double_token_creation_replacement(
                    PlayerFilter::You,
                    "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.".to_string(),
                ),
            ))
            .build();
        game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::one(soldier_token())
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().all(|id| {
            game.object(*id)
                .is_some_and(|token| token.name == "Soldier" && token.kind == ObjectKind::Token)
        }));
    }

    #[test]
    fn create_token_replacement_does_not_double_other_players_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let doubler = CardDefinitionBuilder::new(CardId::new(), "Token Doubler")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::double_token_creation_replacement(
                    PlayerFilter::You,
                    "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.".to_string(),
                ),
            ))
            .build();
        game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::new(soldier_token(), 1, PlayerFilter::Specific(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(game.controller_of(token), bob);
    }

    #[test]
    fn xorn_adds_one_treasure_token_to_your_treasure_creation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let xorn = xorn_definition();
        game.create_object_from_definition(&xorn, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::you(fancy_treasure_token(), 2)
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 3, "Xorn should add exactly one Treasure token");
        let fancy_count = ids
            .iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|token| token.name == "Fancy Treasure")
            })
            .count();
        let normal_count = ids
            .iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|token| token.name == "Treasure")
            })
            .count();
        assert_eq!(
            fancy_count, 2,
            "the original token batch should be preserved"
        );
        assert_eq!(normal_count, 1, "Xorn should add one normal Treasure token");
        assert!(ids.iter().all(|id| {
            game.object(*id)
                .is_some_and(|token| token.subtypes.contains(&Subtype::Treasure))
        }));
    }

    #[test]
    fn xorn_does_not_add_tokens_to_non_treasure_token_creation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let xorn = xorn_definition();
        game.create_object_from_definition(&xorn, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::you(soldier_token(), 2)
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2, "Xorn should ignore non-Treasure tokens");
    }

    #[test]
    fn xorn_does_not_add_tokens_to_other_players_treasure_creation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let xorn = xorn_definition();
        game.create_object_from_definition(&xorn, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result =
            CreateTokenEffect::new(treasure_token_definition(), 2, PlayerFilter::Specific(bob))
                .execute(&mut game, &mut ctx)
                .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(
            ids.len(),
            2,
            "Xorn should only affect its controller's Treasure creation"
        );
        assert!(ids.iter().all(|id| {
            game.object(*id)
                .is_some_and(|token| game.controller_of(token) == bob)
        }));
    }

    #[test]
    fn create_token_caps_tokens_per_controller_at_500() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = CreateTokenEffect::you(soldier_token(), 501)
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 500);

        let result = CreateTokenEffect::you(soldier_token(), 2)
            .execute(&mut game, &mut ctx)
            .unwrap();
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert!(ids.is_empty());
    }

    #[test]
    fn test_create_zero_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(zombie_token(), 0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert!(ids.is_empty());
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_tracks_creature_etb() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(beast_token(), 2);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should have tracked 2 creatures entering
        assert_eq!(
            game.turn_store
                .turn_history
                .creatures_entered_under_controller(alice),
            2
        );
    }

    #[test]
    fn test_create_token_for_other_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        // Use Specific instead of Opponent since Opponent requires targeting context
        let effect = CreateTokenEffect::new(spirit_token(), 1, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token = game.object(ids[0]).unwrap();
            assert_eq!(game.controller_of(token), bob);
            assert_eq!(
                token.owner, bob,
                "the player who creates the token should own it"
            );
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn multiplayer_800_4b_d_does_not_create_tokens_for_player_who_left() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        assert!(game.leave_game(alice));
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = CreateTokenEffect::you(soldier_token(), 2)
            .execute(&mut game, &mut ctx)
            .expect("the creation instruction should be skipped cleanly");

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected an empty object outcome");
        };
        assert!(ids.is_empty());
        assert!(game.battlefield.is_empty());
    }

    #[test]
    fn test_create_token_clone_box() {
        let effect = CreateTokenEffect::one(soldier_token());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CreateTokenEffect"));
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn test_created_creature_token_gets_etb_replacement_counter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let _tayam =
            game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::one(soldier_token());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let created_id = match result.value {
            crate::effect::OutcomeValue::Objects(ids) => {
                *ids.first().expect("expected created token")
            }
            other => panic!("expected created token object ids, got {other:?}"),
        };

        let token = game.object(created_id).expect("created token should exist");
        assert_eq!(
            token.counters.get(&CounterType::Vigilance).copied(),
            Some(1),
            "token creature should get Tayam's additional vigilance counter on entry"
        );
    }
}
