use crate::decision::FallbackStrategy;
use crate::decisions::{ask_choose_one, ask_may_choice};
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, helpers::resolve_player_filter};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::other::DieRolledEvent;
use crate::filter::PlayerFilterExt as _;
use crate::game_state::GameState;
use crate::static_abilities::DieRollResultAdjustmentSpec;
use crate::target::PlayerFilter;

#[derive(Debug, Clone)]
struct AvailableDieRollAdjustment {
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    spec: DieRollResultAdjustmentSpec,
}

/// Roll a die for a player using the game's deterministic RNG.
#[derive(Debug, Clone, PartialEq)]
pub struct RollDieEffect {
    pub player: PlayerFilter,
    pub sides: u32,
    pub die_text: Option<String>,
}

impl RollDieEffect {
    pub fn new(player: PlayerFilter, sides: u32) -> Self {
        Self {
            player,
            sides,
            die_text: None,
        }
    }

    pub fn new_with_die_text(player: PlayerFilter, sides: u32, die_text: Option<String>) -> Self {
        Self {
            player,
            sides,
            die_text,
        }
    }
}

impl EffectExecutor for RollDieEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if self.sides == 0 {
            return Ok(EffectOutcome::count(0));
        }
        if let Some(forced) = game.take_forced_die_roll() {
            let result = forced.clamp(1, self.sides);
            let Some(adjusted) = apply_die_roll_result_adjustments(game, ctx, player, result)
            else {
                return Ok(EffectOutcome::count(0));
            };
            game.turn_store.turn_history.record_die_roll(player, adjusted);
            return Ok(EffectOutcome::count(adjusted as i32)
                .with_event(crate::triggers::TriggerEvent::new_with_provenance(
                    DieRolledEvent::new(player, ctx.source, adjusted, self.sides),
                    ctx.provenance,
                ))
                .with_execution_fact(ExecutionFact::ChosenNumber(adjusted)));
        }

        let mut faces: Vec<u32> = (1..=self.sides).collect();
        game.shuffle_slice(&mut faces);
        let result = faces[0];
        let Some(adjusted) = apply_die_roll_result_adjustments(game, ctx, player, result) else {
            return Ok(EffectOutcome::count(0));
        };
        game.turn_store.turn_history.record_die_roll(player, adjusted);
        Ok(EffectOutcome::count(adjusted as i32)
            .with_event(crate::triggers::TriggerEvent::new_with_provenance(
                DieRolledEvent::new(player, ctx.source, adjusted, self.sides),
                ctx.provenance,
            ))
            .with_execution_fact(ExecutionFact::ChosenNumber(adjusted)))
    }
}

fn available_die_roll_adjustments(
    game: &GameState,
    player: crate::ids::PlayerId,
) -> Vec<AvailableDieRollAdjustment> {
    game.battlefield
        .iter()
        .filter_map(|source| {
            let object = game.object(*source)?;
            let controller = game.controller_of(object);
            let filter_context = game.filter_context_for(controller, Some(*source));
            object.abilities.iter().find_map(|ability| {
                let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    return None;
                };
                let spec = static_ability.die_roll_result_adjustment_spec()?;
                if spec.once_each_turn
                    && game
                        .turn_store
                        .turn_history
                        .die_roll_result_adjusted_this_turn(*source)
                {
                    return None;
                }
                if !spec.player.matches_player(player, &filter_context) {
                    return None;
                }
                if !game.can_pay_life(controller, spec.life_cost) {
                    return None;
                }
                Some(AvailableDieRollAdjustment {
                    source: *source,
                    controller,
                    spec,
                })
            })
        })
        .collect()
}

fn apply_die_roll_result_adjustments(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: crate::ids::PlayerId,
    result: u32,
) -> Option<u32> {
    let mut adjusted = result;
    for adjustment in available_die_roll_adjustments(game, player) {
        let description = format!(
            "pay {} life to increase or decrease the die result by {}",
            adjustment.spec.life_cost, adjustment.spec.amount
        );
        let should_adjust = ask_may_choice(
            game,
            &mut ctx.decision_maker,
            adjustment.controller,
            adjustment.source,
            description,
            FallbackStrategy::Decline,
        );
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        if !should_adjust {
            continue;
        }
        let options = [
            ("Increase".to_string(), adjustment.spec.amount as i32),
            ("Decrease".to_string(), -(adjustment.spec.amount as i32)),
        ];
        let chosen_delta = ask_choose_one(
            game,
            &mut ctx.decision_maker,
            adjustment.controller,
            adjustment.source,
            &options,
        );
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        let delta = chosen_delta.unwrap_or(adjustment.spec.amount as i32);
        if !game.pay_life(adjustment.controller, adjustment.spec.life_cost) {
            continue;
        }
        adjusted = if delta.is_negative() {
            adjusted.saturating_sub(delta.unsigned_abs())
        } else {
            adjusted.saturating_add(delta as u32)
        };
        if adjustment.spec.once_each_turn {
            game.turn_store
                .turn_history
                .record_die_roll_result_adjustment(adjustment.source);
        }
    }
    Some(adjusted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::{BooleanContext, SelectOptionsContext};
    use crate::effect::{Comparison, Effect, EffectId, EffectPredicate};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    struct DieAdjustmentDecisionMaker {
        accept: bool,
        option: usize,
    }

    impl DecisionMaker for DieAdjustmentDecisionMaker {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.accept
        }

        fn decide_options(&mut self, _game: &GameState, _ctx: &SelectOptionsContext) -> Vec<usize> {
            vec![self.option]
        }
    }

    fn add_die_adjustment_source(game: &mut GameState, controller: PlayerId) -> ObjectId {
        let card =
            crate::CardDefinitionBuilder::new(CardId::from_raw(91_001), "Die Adjustment Source")
                .card_types(vec![CardType::Creature])
                .with_ability(Ability::static_ability(
                    StaticAbility::die_roll_result_adjustment(
                        PlayerFilter::You,
                        1,
                        1,
                        true,
                        "After you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.",
                    ),
                ))
                .build();
        game.create_object_from_definition(&card, controller, Zone::Battlefield)
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    fn add_compiled_night_shift_source(game: &mut GameState, controller: PlayerId) -> ObjectId {
        let text = "After you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.\nWhenever you roll a 6, create a 2/2 black Zombie Employee creature token.";
        let card = crate::CardDefinitionBuilder::new(
            CardId::from_raw(91_002),
            "Night Shift of the Living Dead",
        )
        .card_types(vec![CardType::Enchantment])
        .parse_text(text)
        .expect("Night Shift should compile");
        game.create_object_from_definition(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn roll_die_is_deterministic_for_a_seed_and_consumes_randomness() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(7);

        let before = game.irreversible_random_count();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = execute_effect(
            &mut game,
            &Effect::roll_die(20, PlayerFilter::You),
            &mut ctx,
        )
        .expect("die roll should resolve");
        let rolled = outcome.as_count().expect("die roll should produce a count");

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "die rolls should consume irreversible randomness"
        );
        assert!(
            (1..=20).contains(&rolled),
            "expected a valid d20 result, got {rolled}"
        );
    }

    #[test]
    fn roll_die_uses_queued_forced_result_before_rng() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(7);
        game.force_next_die_roll(6);

        let before = game.irreversible_random_count();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = execute_effect(
            &mut game,
            &Effect::roll_die(20, PlayerFilter::You),
            &mut ctx,
        )
        .expect("die roll should resolve");

        assert_eq!(outcome.as_count(), Some(6));
        assert!(
            game.turn_store
                .turn_history
                .player_rolled_result_this_turn(alice, 6),
            "die rolls should be available to this-turn roll conditions"
        );
        assert_eq!(
            game.irreversible_random_count(),
            before,
            "forced die rolls should not consume RNG state"
        );
    }

    #[test]
    fn roll_die_adjustment_declined_leaves_result_and_life_unchanged() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = add_die_adjustment_source(&mut game, alice);
        game.force_next_die_roll(5);

        let mut dm = DieAdjustmentDecisionMaker {
            accept: false,
            option: 0,
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::roll_die(6, PlayerFilter::You),
            &mut ctx,
        )
        .expect("die roll should resolve");

        assert_eq!(outcome.as_count(), Some(5));
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(
            game.turn_store
                .turn_history
                .player_rolled_result_this_turn(alice, 5)
        );
        assert!(
            !game
                .turn_store
                .turn_history
                .die_roll_result_adjusted_this_turn(source)
        );
    }

    #[test]
    fn roll_die_adjustment_can_increase_or_decrease_recorded_result() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = add_die_adjustment_source(&mut game, alice);
        game.force_next_die_roll(5);

        let mut increase_dm = DieAdjustmentDecisionMaker {
            accept: true,
            option: 0,
        };
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_decision_maker(&mut increase_dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::roll_die(6, PlayerFilter::You),
            &mut ctx,
        )
        .expect("die roll should resolve");

        assert_eq!(outcome.as_count(), Some(6));
        assert_eq!(game.player(alice).unwrap().life, 19);
        assert!(
            game.turn_store
                .turn_history
                .player_rolled_result_this_turn(alice, 6)
        );

        game.turn_store.turn_history.clear_for_new_turn();
        game.force_next_die_roll(6);
        let mut decrease_dm = DieAdjustmentDecisionMaker {
            accept: true,
            option: 1,
        };
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_decision_maker(&mut decrease_dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::roll_die(6, PlayerFilter::You),
            &mut ctx,
        )
        .expect("die roll should resolve");

        assert_eq!(outcome.as_count(), Some(5));
        assert_eq!(game.player(alice).unwrap().life, 18);
        assert!(
            game.turn_store
                .turn_history
                .player_rolled_result_this_turn(alice, 5)
        );
    }

    #[test]
    fn roll_die_adjustment_can_increase_above_the_die_face_count() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = add_die_adjustment_source(&mut game, alice);
        game.force_next_die_roll(6);

        let mut dm = DieAdjustmentDecisionMaker {
            accept: true,
            option: 0,
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::roll_die(6, PlayerFilter::You),
            &mut ctx,
        )
        .expect("die roll should resolve");

        let die_event = outcome
            .events
            .first()
            .and_then(|event| event.downcast::<DieRolledEvent>())
            .expect("roll should emit a die-rolled event");
        assert_eq!(outcome.as_count(), Some(7));
        assert_eq!(die_event.result, 7);
        assert!(
            game.turn_store
                .turn_history
                .player_rolled_result_this_turn(alice, 7)
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn night_shift_adjusted_roll_emits_six_for_result_based_triggers() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = add_compiled_night_shift_source(&mut game, alice);
        game.force_next_die_roll(5);

        let mut dm = DieAdjustmentDecisionMaker {
            accept: true,
            option: 0,
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::roll_die(6, PlayerFilter::You),
            &mut ctx,
        )
        .expect("die roll should resolve");

        let die_event = outcome
            .events
            .first()
            .and_then(|event| event.downcast::<DieRolledEvent>())
            .expect("roll should emit a die-rolled event");
        assert_eq!(outcome.as_count(), Some(6));
        assert_eq!(die_event.result, 6);
        assert!(
            game.turn_store
                .turn_history
                .player_rolled_result_this_turn(alice, 6),
            "adjusted result should be recorded for result-based die-roll triggers"
        );
    }

    #[test]
    fn roll_die_adjustment_applies_only_once_each_turn() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = add_die_adjustment_source(&mut game, alice);

        let mut dm = DieAdjustmentDecisionMaker {
            accept: true,
            option: 0,
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        game.force_next_die_roll(4);
        let first = execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
            .expect("first die roll should resolve");
        game.force_next_die_roll(4);
        let second = execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
            .expect("second die roll should resolve");

        assert_eq!(first.as_count(), Some(5));
        assert_eq!(second.as_count(), Some(4));
        assert_eq!(game.player(alice).unwrap().life, 19);

        game.turn_store.turn_history.clear_for_new_turn();
        game.force_next_die_roll(4);
        let third = execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
            .expect("next-turn die roll should resolve");

        assert_eq!(third.as_count(), Some(5));
        assert_eq!(game.player(alice).unwrap().life, 18);
    }

    #[test]
    fn roll_die_outcome_drives_value_based_if_result_branches() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(2);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = execute_effect(
            &mut game,
            &Effect::with_id(0, Effect::roll_die(20, PlayerFilter::You)),
            &mut ctx,
        )
        .expect("die roll should resolve");
        let rolled = outcome.as_count().expect("die roll should produce a count");

        execute_effect(
            &mut game,
            &Effect::if_then(
                EffectId(0),
                EffectPredicate::Value(Comparison::BetweenInclusive(
                    rolled.saturating_sub(1).max(1),
                    rolled,
                )),
                vec![Effect::gain_life(3)],
            ),
            &mut ctx,
        )
        .expect("if-result branch should resolve");

        execute_effect(
            &mut game,
            &Effect::if_then(
                EffectId(0),
                EffectPredicate::Value(Comparison::BetweenInclusive(
                    rolled.saturating_add(1),
                    rolled.saturating_add(2),
                )),
                vec![Effect::gain_life(5)],
            ),
            &mut ctx,
        )
        .expect("non-matching if-result branch should resolve");

        assert_eq!(game.player(alice).unwrap().life, 23);
    }
}
