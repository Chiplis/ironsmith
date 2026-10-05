//! Untap effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect_with_choice_description;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::UntapEffect;

/// Effect that untaps permanents.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Untap target creature (targeted - can fizzle)
/// let effect = UntapEffect::target(ChooseSpec::creature());
///
/// // Untap all creatures you control (non-targeted - cannot fizzle)
/// let effect = UntapEffect::all(ObjectFilter::creature().you_control());
/// ```
impl EffectExecutor for UntapEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let choice_description = match self.target.base() {
                ChooseSpec::Object(filter) => {
                    Some(format!("Choose {} to untap", filter.description()))
                }
                _ => Some("Choose permanent to untap".to_string()),
            };
            let objects = resolve_objects_for_effect_with_choice_description(
                game,
                ctx,
                &self.target,
                choice_description,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let actor = self
                .actor
                .as_ref()
                .map(|actor| crate::effects::helpers::resolve_player_filter(game, actor, ctx))
                .transpose()?
                .unwrap_or(ctx.controller);
            let before = crate::events::other::before_tap_state_snapshots(game);
            let selected_count = objects.len();
            let mut outcomes = Vec::new();
            for object in objects {
                outcomes.push(
                    crate::events::processing::process_untap_with_execution_context(
                        game, object, ctx,
                    )?,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
            }
            let count = outcomes.iter().map(EffectOutcome::count_or_zero).sum();
            let mut outcome = EffectOutcome::aggregate_summing_counts(outcomes);
            outcome.set_value(crate::effect::OutcomeValue::Count(count));
            for event in &mut outcome.events {
                if event.simultaneous_batch().is_some() {
                    continue;
                }
                if let Some(untapped) = event.downcast::<crate::events::PermanentUntappedEvent>() {
                    let mut untapped = untapped.clone();
                    untapped.actor = Some(actor);
                    *event = event.with_inner_event(untapped);
                }
            }
            crate::events::other::bind_before_tap_state_snapshots(&mut outcome.events, &before);
            crate::events::other::group_tap_state_events(game, &mut outcome.events, ctx.provenance);
            if self.target.is_target() && self.target.is_single() {
                // A legal target resolves even when no untap happens. Preserve
                // the complete payload while retaining that target policy.
                let summary = if selected_count > 0 {
                    EffectOutcome::resolved()
                } else {
                    EffectOutcome::target_invalid()
                };
                outcome.set_status(summary.status);
                outcome.set_value(summary.value);
            }
            Ok(outcome)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(
                checkpoint,
                result.is_ok() && ctx.decision_maker.awaiting_choice(),
            );
            context_checkpoint.restore(ctx);
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to untap"
    }

    fn is_untap_source_cost(&self) -> bool {
        matches!(self.target, ChooseSpec::Source)
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.target, ChooseSpec::Source) {
            Some("{Q}".to_string())
        } else {
            None
        }
    }
}

impl CostExecutableEffect for UntapEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if matches!(self.target, ChooseSpec::Source) {
            if !game.is_tapped(source) {
                return Err(crate::effects::CostValidationError::AlreadyUntapped);
            }

            if game.object(source).is_some()
                && game.current_is_creature(source)
                && game.is_summoning_sick(source)
                && !game.current_has_static_ability_id(
                    source,
                    crate::static_abilities::StaticAbilityId::Haste,
                )
                && !game.activates_abilities_as_though_haste(source)
            {
                return Err(crate::effects::CostValidationError::SummoningSickness);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        tapped: bool,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        if tapped {
            game.tap(id);
        }
        id
    }

    // === Targeted untap tests ===

    #[test]
    fn test_untap_tapped_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice, true);

        assert!(game.is_tapped(creature_id));

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = UntapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(!game.is_tapped(creature_id));
    }

    #[test]
    fn test_untap_already_untapped_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice, false);

        assert!(!game.is_tapped(creature_id));

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = UntapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Still resolves even if already untapped
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(!game.is_tapped(creature_id));
    }

    #[test]
    fn test_untap_nonexistent_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let fake_id = game.new_object_id();

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(fake_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: ChooseSpec::target(ChooseSpec::creature()),
                range: 0..1,
            }]);

        let effect = UntapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // For single target, returns Resolved (target existed in ctx.targets)
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn test_untap_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::target(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_untap_source_cost_validation_and_description() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice, false);
        let effect = UntapEffect::with_spec(ChooseSpec::Source);

        assert_eq!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, creature_id, alice),
            Err(crate::effects::CostValidationError::AlreadyUntapped)
        );

        game.tap(creature_id);
        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(
                &effect,
                &game,
                creature_id,
                alice,
            )
                .is_ok()
        );
        assert!(effect.is_untap_source_cost());
        assert_eq!(effect.cost_description().as_deref(), Some("{Q}"));
    }

    #[test]
    fn test_untap_source_cost_validation_respects_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice, true);
        let effect = UntapEffect::with_spec(ChooseSpec::Source);
        game.set_summoning_sick(creature_id);

        assert_eq!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, creature_id, alice),
            Err(crate::effects::CostValidationError::SummoningSickness)
        );
    }

    #[test]
    fn test_untap_source_cost_validation_allows_haste_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice, true);
        let effect = UntapEffect::with_spec(ChooseSpec::Source);
        game.set_summoning_sick(creature_id);
        game.object_mut(creature_id)
            .expect("creature should exist")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::haste()));

        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(
                &effect,
                &game,
                creature_id,
                alice,
            )
            .is_ok()
        );
    }

    #[test]
    fn test_untap_get_target_spec() {
        let effect = UntapEffect::target(ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }

    #[test]
    fn test_untap_clone_box() {
        let effect = UntapEffect::target(ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("UntapEffect"));
    }

    // === UntapAll tests (using UntapEffect::all) ===

    #[test]
    fn test_untap_all_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Bear", alice, true);
        let creature2 = create_creature(&mut game, "Wolf", alice, true);
        let creature3 = create_creature(&mut game, "Lion", bob, true);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        assert!(!game.is_tapped(creature1));
        assert!(!game.is_tapped(creature2));
        assert!(!game.is_tapped(creature3));
    }

    #[test]
    fn test_untap_all_your_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let alice_creature = create_creature(&mut game, "Bear", alice, true);
        let bob_creature = create_creature(&mut game, "Wolf", bob, true);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::all(ObjectFilter::creature().you_control());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(!game.is_tapped(alice_creature));
        assert!(game.is_tapped(bob_creature));
    }

    #[test]
    fn test_untap_all_skips_already_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature1 = create_creature(&mut game, "Bear", alice, true);
        let creature2 = create_creature(&mut game, "Wolf", alice, false);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only 1 was actually untapped (the tapped one)
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(!game.is_tapped(creature1));
        assert!(!game.is_tapped(creature2));
    }

    #[test]
    fn test_untap_all_no_matching_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // No creatures exist
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = UntapEffect::all(ObjectFilter::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_untap_all_no_target_spec() {
        let effect = UntapEffect::all(ObjectFilter::creature());
        // All effects don't have a target spec
        assert!(effect.get_target_spec().is_none());
    }

    #[test]
    fn test_untap_all_clone_box() {
        let effect = UntapEffect::all(ObjectFilter::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("UntapEffect"));
    }
}

#[cfg(test)]
mod replacement_pending_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectOptionsContext;
    use crate::effect::Effect;
    use crate::events::permanents::matchers::WouldBecomeUntappedMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    struct Choice {
        pause: bool,
        pending: bool,
    }
    impl DecisionMaker for Choice {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.options.len(), 2);
            self.pending = self.pause;
            if self.pause { vec![] } else { vec![1] }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    #[test]
    fn pending_untap_replacement_does_not_execute_default_payload() {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let permanent = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Untap subject").build(),
            player,
            Zone::Battlefield,
        );
        game.tap(permanent);
        for amount in [1, 2] {
            game.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    permanent,
                    player,
                    WouldBecomeUntappedMatcher::new(ObjectFilter::specific(permanent)),
                    ReplacementAction::Instead(vec![Effect::gain_life(amount)]),
                ),
            );
        }
        let mut dm = Choice {
            pause: true,
            pending: false,
        };
        assert_eq!(
            crate::events::processing::process_untap(&mut game, permanent, &mut dm)
                .unwrap()
                .count_or_zero(),
            0
        );
        assert!(dm.pending);
        assert!(game.is_tapped(permanent));
        assert_eq!(
            game.player(player).unwrap().life,
            20,
            "pending choice cannot execute a replacement"
        );

        // Resume the same proposal with an explicit non-default choice.
        dm.pause = false;
        dm.pending = false;
        assert_eq!(
            crate::events::processing::process_untap(&mut game, permanent, &mut dm)
                .unwrap()
                .count_or_zero(),
            0
        );
        assert_eq!(game.player(player).unwrap().life, 22);
        assert!(game.is_tapped(permanent));
    }
}

#[cfg(test)]
mod resolved_event_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::{Effect, Value};
    use crate::events::PermanentUntappedEvent;
    use crate::events::permanents::matchers::WouldBecomeUntappedMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    #[test]
    fn redirected_untap_commits_and_reports_resolved_permanent() {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let original = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Original").build(),
            player,
            Zone::Battlefield,
        );
        let redirected = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Redirected").build(),
            player,
            Zone::Battlefield,
        );
        game.tap(original);
        game.tap(redirected);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                original,
                player,
                WouldBecomeUntappedMatcher::new(ObjectFilter::specific(original)),
                ReplacementAction::Redirect {
                    target: RedirectTarget::ToObject(redirected),
                    which: RedirectWhich::First,
                },
            ),
        );
        let mut ctx = ExecutionContext::new_default(original, player);
        let outcome = UntapEffect::with_spec(ChooseSpec::Source)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(game.is_tapped(original));
        assert!(!game.is_tapped(redirected));
        assert_eq!(outcome.events.len(), 1);
        let event =
            crate::events::downcast_event::<PermanentUntappedEvent>(outcome.events[0].inner())
                .unwrap();
        assert_eq!(event.permanent, redirected);
    }

    #[test]
    fn untap_instead_error_restores_payload_and_one_shot() {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let original = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Original").build(),
            player,
            Zone::Battlefield,
        );
        game.tap(original);
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                original,
                player,
                WouldBecomeUntappedMatcher::new(ObjectFilter::specific(original)),
                ReplacementAction::Instead(vec![Effect::gain_life(2), Effect::lose_life(Value::X)]),
            ),
        );
        let mut ctx = ExecutionContext::new_default(original, player);
        let result = UntapEffect::with_spec(ChooseSpec::Source).execute(&mut game, &mut ctx);
        assert!(
            result.is_err(),
            "replacement payload errors must escape untap"
        );
        assert_eq!(game.player(player).unwrap().life, 20);
        assert!(game.is_tapped(original));
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(replacement)
                .is_some()
        );
    }
    #[test]
    fn later_untap_payload_pause_or_error_restores_entire_operation_and_replays_once() {
        struct Answers {
            pause_at: Option<usize>,
            calls: usize,
            pending: bool,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(!self.pending, "no later question while untap is pending");
                self.pending = self.pause_at == Some(self.calls);
                self.calls += 1;
                !self.pending
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for step in [false, true] {
            for pause_at in [None, Some(0), Some(1)] {
                let player = PlayerId::from_index(0);
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let mut objects = Vec::new();
                for name in ["First", "Second", "Third"] {
                    let id = game.create_object_from_card(
                        &CardBuilder::new(CardId::new(), name).build(),
                        player,
                        Zone::Battlefield,
                    );
                    game.tap(id);
                    game.set_summoning_sick(id);
                    objects.push(id);
                }
                game.turn.active_player = player;
                let payload = if pause_at.is_some() {
                    vec![
                        Effect::gain_life(2),
                        Effect::may(vec![Effect::gain_life(1)]),
                        Effect::may(vec![Effect::gain_life(3)]),
                    ]
                } else {
                    vec![Effect::gain_life(2), Effect::lose_life(Value::X)]
                };
                let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
                    ReplacementEffect::with_matcher(
                        objects[1],
                        player,
                        WouldBecomeUntappedMatcher::new(ObjectFilter::specific(objects[1])),
                        ReplacementAction::Instead(payload),
                    ),
                );
                game.take_pending_trigger_events();
                let mut dm = Answers {
                    pause_at,
                    calls: 0,
                    pending: false,
                };
                let result = if step {
                    crate::turn::execute_untap_step_with(&mut game, &mut dm)
                        .map(|_| EffectOutcome::count(0))
                } else {
                    let mut ctx = ExecutionContext::new(objects[0], player, &mut dm);
                    UntapEffect::all(ObjectFilter::permanent()).execute(&mut game, &mut ctx)
                };
                assert_eq!(result.is_err(), pause_at.is_none());
                if let Ok(outcome) = result {
                    assert!(outcome.events.is_empty());
                }
                assert_eq!(game.player(player).unwrap().life, 20);
                for id in &objects {
                    assert!(game.is_tapped(*id));
                    assert!(game.is_summoning_sick(*id));
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(one_shot)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
                if let Some(pause_at) = pause_at {
                    assert_eq!(dm.calls, pause_at + 1);
                    let mut dm = Answers {
                        pause_at: None,
                        calls: 0,
                        pending: false,
                    };
                    let events = if step {
                        crate::turn::execute_untap_step_with(&mut game, &mut dm).unwrap();
                        game.take_pending_trigger_events()
                    } else {
                        let mut ctx = ExecutionContext::new(objects[0], player, &mut dm);
                        let outcome = UntapEffect::all(ObjectFilter::permanent())
                            .execute(&mut game, &mut ctx)
                            .unwrap();
                        assert_eq!(outcome.count_or_zero(), 2);
                        assert!(game.take_pending_trigger_events().is_empty());
                        outcome.events
                    };
                    assert!(!dm.pending);
                    assert_eq!(dm.calls, 2);
                    assert_eq!(game.player(player).unwrap().life, 26);
                    assert!(!game.is_tapped(objects[0]));
                    assert!(game.is_tapped(objects[1]));
                    assert!(!game.is_tapped(objects[2]));
                    assert!(
                        game.effect_store
                            .replacement_effects
                            .get_effect(one_shot)
                            .is_none()
                    );
                    assert_eq!(events.len(), 5);
                    assert_eq!(
                        events
                            .iter()
                            .filter(|event| event.downcast::<PermanentUntappedEvent>().is_some())
                            .count(),
                        2
                    );
                    assert_eq!(
                        events
                            .iter()
                            .filter(|event| event
                                .downcast::<crate::events::LifeGainEvent>()
                                .is_some())
                            .count(),
                        3
                    );
                }
            }
        }
    }

    #[test]
    fn untap_instead_inherits_parent_effects_and_suppresses_history_for_nested_untap() {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let permanent = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Subject").build(),
            player,
            Zone::Battlefield,
        );
        game.tap(permanent);
        let mut ctx = ExecutionContext::new_default(permanent, player);
        let mut replacement = ReplacementEffect::with_matcher(
            permanent,
            player,
            WouldBecomeUntappedMatcher::new(ObjectFilter::specific(permanent)),
            ReplacementAction::Instead(vec![
                Effect::untap(ChooseSpec::SpecificObject(permanent)),
                Effect::gain_life(2),
            ]),
        );
        replacement.id = crate::replacement::ReplacementEffectId(987654);
        ctx.replacement
            .additional_replacement_effects
            .push(replacement);
        let outcome = UntapEffect::with_spec(ChooseSpec::Source)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            outcome.count_or_zero(),
            0,
            "the original untap was replaced"
        );
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert!(
            !game.is_tapped(permanent),
            "nested untap must not repeat the parent replacement"
        );
        assert_eq!(game.player(player).unwrap().life, 22);
        assert_eq!(outcome.events.len(), 2);
        assert!(
            outcome.events[0]
                .downcast::<PermanentUntappedEvent>()
                .is_some()
        );
        assert!(
            outcome.events[1]
                .downcast::<crate::events::LifeGainEvent>()
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
        assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
    }
    #[test]
    fn delayed_untap_replacement_error_restores_the_step_and_every_scheduled_action() {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = player;
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Scheduler").build(),
            player,
            Zone::Battlefield,
        );
        let permanent = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Subject").build(),
            PlayerId::from_index(1),
            Zone::Battlefield,
        );
        game.tap(permanent);
        let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                permanent,
                player,
                WouldBecomeUntappedMatcher::new(ObjectFilter::specific(permanent)),
                ReplacementAction::Instead(vec![Effect::gain_life(2), Effect::lose_life(Value::X)]),
            ),
        );
        for effects in [
            vec![Effect::gain_life(1)],
            vec![Effect::untap(ChooseSpec::SpecificObject(permanent))],
            vec![Effect::gain_life(5)],
        ] {
            let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
                crate::triggers::Trigger::as_permanents_untap(
                    crate::target::PlayerFilter::You,
                    true,
                ),
                effects,
                true,
                Vec::new(),
                crate::target::PlayerFilter::You,
            )
            .watch_ability_source();
            let mut ctx = ExecutionContext::new_default(source, player);
            schedule.execute(&mut game, &mut ctx).unwrap();
        }
        assert_eq!(game.effect_store.delayed_triggers.len(), 3);
        game.take_pending_trigger_events();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = crate::turn::execute_untap_step_with(&mut game, &mut dm);
        assert!(
            result.is_err(),
            "a scheduled untap must not swallow its replacement error"
        );
        assert_eq!(game.player(player).unwrap().life, 20);
        assert!(game.is_tapped(permanent));
        assert_eq!(game.effect_store.delayed_triggers.len(), 3);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }
    #[test]
    fn scheduled_untap_pause_stops_later_actions_and_replays_the_whole_step_once() {
        struct Answers {
            pause: bool,
            pending: bool,
            calls: usize,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(
                    !self.pending,
                    "later scheduled actions cannot ask while pending"
                );
                self.calls += 1;
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = player;
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Scheduler").build(),
            player,
            Zone::Battlefield,
        );
        let permanent = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Subject").build(),
            PlayerId::from_index(1),
            Zone::Battlefield,
        );
        game.tap(permanent);
        let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                permanent,
                player,
                WouldBecomeUntappedMatcher::new(ObjectFilter::specific(permanent)),
                ReplacementAction::Instead(vec![
                    Effect::gain_life(2),
                    Effect::may(vec![Effect::gain_life(1)]),
                ]),
            ),
        );
        for effects in [
            vec![Effect::gain_life(1)],
            vec![Effect::untap(ChooseSpec::SpecificObject(permanent))],
            vec![Effect::may(vec![Effect::gain_life(5)])],
        ] {
            let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
                crate::triggers::Trigger::as_permanents_untap(
                    crate::target::PlayerFilter::You,
                    true,
                ),
                effects,
                true,
                Vec::new(),
                crate::target::PlayerFilter::You,
            )
            .watch_ability_source();
            let mut ctx = ExecutionContext::new_default(source, player);
            schedule.execute(&mut game, &mut ctx).unwrap();
        }
        game.take_pending_trigger_events();
        let mut dm = Answers {
            pause: true,
            pending: false,
            calls: 0,
        };
        crate::turn::execute_untap_step_with(&mut game, &mut dm).unwrap();
        assert!(dm.pending);
        assert_eq!(dm.calls, 1);
        assert_eq!(game.player(player).unwrap().life, 20);
        assert!(game.is_tapped(permanent));
        assert_eq!(game.effect_store.delayed_triggers.len(), 3);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        let mut dm = Answers {
            pause: false,
            pending: false,
            calls: 0,
        };
        crate::turn::execute_untap_step_with(&mut game, &mut dm).unwrap();
        assert!(!dm.pending);
        assert_eq!(dm.calls, 2);
        assert_eq!(game.player(player).unwrap().life, 29);
        assert!(game.is_tapped(permanent));
        assert!(game.effect_store.delayed_triggers.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
        let events = game
            .turn_store
            .turn_history
            .projected_records()
            .map(|record| &record.event)
            .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 4);
        assert!(
            events
                .iter()
                .all(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
        );
    }
    #[test]
    fn scheduled_untap_preserves_player_references_through_replacement_and_replay() {
        struct Answers {
            pause: bool,
            pending: bool,
            calls: usize,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(!self.pending);
                self.calls += 1;
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for pause in [false, true] {
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            let source = game.create_object_from_card(
                &CardBuilder::new(CardId::new(), "Scheduler").build(),
                alice,
                Zone::Battlefield,
            );
            let permanent = game.create_object_from_card(
                &CardBuilder::new(CardId::new(), "Subject").build(),
                bob,
                Zone::Battlefield,
            );
            game.tap(permanent);
            let player = ChooseSpec::Player(crate::target::PlayerFilter::TaggedPlayer(
                "beneficiary".into(),
            ));
            let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    permanent,
                    alice,
                    WouldBecomeUntappedMatcher::new(ObjectFilter::specific(permanent)),
                    ReplacementAction::Instead(vec![
                        Effect::gain_life(2),
                        Effect::may(vec![Effect::gain_life(3)]),
                    ]),
                ),
            );
            let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
                crate::triggers::Trigger::as_permanents_untap(
                    crate::target::PlayerFilter::You,
                    true,
                ),
                vec![
                    Effect::gain_life_player(2, player.clone()),
                    Effect::untap(ChooseSpec::SpecificObject(permanent)),
                    Effect::may(vec![Effect::gain_life_player(3, player)]),
                ],
                true,
                Vec::new(),
                crate::target::PlayerFilter::You,
            )
            .watch_ability_source();
            let mut ctx = ExecutionContext::new_default(source, alice);
            ctx.set_tagged_players("beneficiary", vec![bob]);
            schedule.execute(&mut game, &mut ctx).unwrap();
            // Scheduling must own the reference, independent of later context changes.
            ctx.set_tagged_players("beneficiary", vec![alice]);
            drop(ctx);
            assert_eq!(game.effect_store.delayed_triggers.len(), 1);
            game.take_pending_trigger_events();
            let mut dm = Answers {
                pause,
                pending: false,
                calls: 0,
            };
            crate::turn::execute_untap_step_with(&mut game, &mut dm).unwrap();
            assert_eq!(dm.calls, if pause { 1 } else { 2 });
            if pause {
                assert!(dm.pending);
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert_eq!(game.effect_store.delayed_triggers.len(), 1);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(one_shot)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
                let mut replay = Answers {
                    pause: false,
                    pending: false,
                    calls: 0,
                };
                crate::turn::execute_untap_step_with(&mut game, &mut replay).unwrap();
                assert_eq!(replay.calls, 2);
            }
            assert_eq!(game.player(alice).unwrap().life, 25);
            assert_eq!(game.player(bob).unwrap().life, 25);
            assert!(game.is_tapped(permanent));
            assert!(game.effect_store.delayed_triggers.is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(one_shot)
                    .is_none()
            );
            let events = game
                .turn_store
                .turn_history
                .projected_records()
                .map(|record| &record.event)
                .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                .collect::<Vec<_>>();
            assert_eq!(events.len(), 4);
            for player in [alice, bob] {
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| event
                            .downcast::<crate::events::LifeGainEvent>()
                            .is_some_and(|event| event.player == player))
                        .count(),
                    2
                );
            }
        }
    }
}
