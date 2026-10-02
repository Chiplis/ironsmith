use super::{ExchangeLifeTotalsEffect, LoseLifeEffect, SetLifeTotalEffect};
use crate::card::{CardBuilder, PowerToughness};
use crate::decision::DecisionMaker;
use crate::decisions::context::SelectOptionsContext;
use crate::effect::Until;
use crate::effects::{
    EffectExecutor, ExchangeValueOperand, ExchangeValuesEffect, ExecutionContext, RadiationEffect,
};
use crate::events::life::matchers::{WouldGainLifeMatcher, WouldLoseLifeMatcher};
use crate::events::{EventContext, GameEventType, ReplacementMatcher};
use crate::game_state::GameState;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::object::CounterType;
use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

#[derive(Clone, Debug)]
struct SmallLifeLoss(u32);

impl ReplacementMatcher for SmallLifeLoss {
    fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &crate::events::context::PreparedEventContext) -> bool {
        WouldLoseLifeMatcher::you().matches_prepared_event(event, ctx)
            && crate::events::downcast_event::<crate::events::LifeLossEvent>(event)
                .is_some_and(|event| event.amount <= self.0)
    }
    fn display(&self) -> String {
        format!("When you would lose {} or less life", self.0)
    }
}

struct ChooseReplacement {
    source: ObjectId,
    pause: bool,
    pending: bool,
    calls: usize,
}

impl DecisionMaker for ChooseReplacement {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        assert_eq!(ctx.player, PlayerId::from_index(0));
        let legal = ctx
            .options
            .iter()
            .filter(|option| option.legal)
            .collect::<Vec<_>>();
        assert_eq!(legal.len(), 2, "the player must receive both replacements");
        let chosen = legal
            .iter()
            .find(|option| option.object_id == Some(self.source))
            .unwrap();
        assert_ne!(
            chosen.index, legal[0].index,
            "exercise the non-default replacement"
        );
        self.calls += 1;
        self.pending = self.pause;
        if self.pause {
            Vec::new()
        } else {
            vec![chosen.index]
        }
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}

fn permanent(game: &mut GameState, name: &str, toughness: i32) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, toughness))
            .build(),
        PlayerId::from_index(0),
        Zone::Battlefield,
    )
}

fn replacements(game: &mut GameState, gain: bool, loss_limit: u32) -> ObjectId {
    let alice = PlayerId::from_index(0);
    let first = permanent(game, "First replacement", 1);
    let second = permanent(game, "Second replacement", 1);
    let (first_effect, second_effect) = if gain {
        (
            ReplacementEffect::with_matcher(
                first,
                alice,
                WouldGainLifeMatcher::you(),
                ReplacementAction::Modify(EventModification::Add(1)),
            ),
            ReplacementEffect::with_matcher(
                second,
                alice,
                WouldGainLifeMatcher::you(),
                ReplacementAction::Double,
            ),
        )
    } else {
        (
            ReplacementEffect::with_matcher(
                first,
                alice,
                SmallLifeLoss(loss_limit),
                ReplacementAction::Double,
            ),
            ReplacementEffect::with_matcher(
                second,
                alice,
                WouldLoseLifeMatcher::you(),
                ReplacementAction::Double,
            ),
        )
    };
    game.effect_store
        .replacement_effects
        .add_resolution_effect(first_effect);
    game.effect_store
        .replacement_effects
        .add_resolution_effect(second_effect);
    second
}

#[test]
fn life_total_and_exchange_effects_forward_pending_and_selected_replacements() {
    // Set-life proposals and direct effects must use the same decision maker.
    for gain in [true, false] {
        for case in 0..5 {
            if gain && case == 4 {
                continue;
            }
            for pause in [true, false] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let next = if gain { 23 } else { 17 };
                game.player_mut(bob).unwrap().life = next;
                let source = permanent(&mut game, "Exchange source", next);
                let second = replacements(&mut game, gain, 3);
                let mut dm = ChooseReplacement {
                    source: second,
                    pause,
                    pending: false,
                    calls: 0,
                };
                let mut ctx =
                    ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
                let effect: Box<dyn EffectExecutor> = match case {
                    0 | 1 => Box::new(SetLifeTotalEffect::you(next)),
                    2 => Box::new(ExchangeLifeTotalsEffect::new(
                        PlayerFilter::You,
                        PlayerFilter::Specific(bob),
                    )),
                    3 => Box::new(ExchangeValuesEffect::new(
                        ExchangeValueOperand::LifeTotal(PlayerFilter::You),
                        ExchangeValueOperand::Toughness(ChooseSpec::Source),
                        Until::Forever,
                    )),
                    4 => Box::new(LoseLifeEffect::you(3)),
                    _ => unreachable!(),
                };
                let outcome = if case == 1 || case == 4 {
                    effect
                        .prepare_simultaneous_player_action(&game, &mut ctx)
                        .unwrap()
                        .commit(&mut game, &mut ctx)
                        .unwrap()
                } else {
                    effect.execute(&mut game, &mut ctx).unwrap()
                };
                assert_eq!(dm.calls, 1, "gain={gain}, case={case}, pause={pause}");
                assert_eq!(dm.pending, pause);
                assert_eq!(
                    game.player(alice).unwrap().life,
                    if pause {
                        20
                    } else if gain {
                        27
                    } else {
                        14
                    }
                );
                if pause {
                    assert!(outcome.events.is_empty());
                    assert_eq!(
                        game.player(bob).unwrap().life,
                        next,
                        "pending exchange cannot update its other player"
                    );
                    assert_eq!(
                        game.calculated_toughness(source),
                        Some(next),
                        "pending exchange cannot update its other operand"
                    );
                } else if case == 2 {
                    assert_eq!(game.player(bob).unwrap().life, 20);
                } else if case == 3 {
                    assert_eq!(game.calculated_toughness(source), Some(20));
                }
            }
        }
    }
}

#[test]
fn radiation_life_loss_waits_before_removing_a_rad_counter() {
    for pause in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = permanent(&mut game, "Radiation source", 1);
        let second = replacements(&mut game, false, 1);
        game.player_mut(alice)
            .unwrap()
            .add_counters(CounterType::Rad, 1);
        let milled = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Milled nonland")
                .card_types(vec![CardType::Sorcery])
                .build(),
            alice,
            Zone::Library,
        );
        let mut dm = ChooseReplacement {
            source: second,
            pause,
            pending: false,
            calls: 0,
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let outcome = RadiationEffect::new().execute(&mut game, &mut ctx).unwrap();
        assert_eq!(dm.calls, 1);
        assert_eq!(dm.pending, pause);
        assert_eq!(
            game.player(alice).unwrap().life,
            if pause { 20 } else { 18 }
        );
        assert_eq!(
            game.player(alice).unwrap().counter_count(CounterType::Rad),
            if pause { 1 } else { 0 }
        );
        if pause {
            assert_eq!(game.object(milled).unwrap().zone, Zone::Library);
        } else {
            assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        }
        if pause {
            assert!(outcome.events.is_empty());
        }
    }
}

#[test]
fn identical_resolution_replacements_from_one_source_each_apply_once() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, "Repeated ability source", 1);
    for _ in 0..2 {
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldGainLifeMatcher::you(),
                ReplacementAction::Double,
            ),
        );
    }
    let mut ctx = ExecutionContext::new_default(source, alice);
    let outcome = super::GainLifeEffect::new(1, ChooseSpec::Player(PlayerFilter::You))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(game.player(alice).unwrap().life, 24);
    assert_eq!(outcome.count_or_zero(), 4);
    assert_eq!(outcome.events.len(), 1);
}

#[test]
fn life_effects_commit_redirected_players_and_publish_resolved_events() {
    use crate::replacement::{RedirectTarget, RedirectWhich};
    for gain in [false, true] {
        for simultaneous in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let source = permanent(&mut game, "Redirect source", 1);
            let action = ReplacementAction::Redirect {
                target: RedirectTarget::ToPlayer(bob),
                which: RedirectWhich::First,
            };
            let replacement = if gain {
                ReplacementEffect::with_matcher(source, alice, WouldGainLifeMatcher::you(), action)
            } else {
                ReplacementEffect::with_matcher(source, alice, WouldLoseLifeMatcher::you(), action)
            };
            game.effect_store
                .replacement_effects
                .add_resolution_effect(replacement);
            let effect: Box<dyn EffectExecutor> = if gain {
                Box::new(super::GainLifeEffect::new(
                    3,
                    ChooseSpec::Player(PlayerFilter::You),
                ))
            } else {
                Box::new(LoseLifeEffect::you(3))
            };
            let mut ctx = ExecutionContext::new_default(source, alice);
            let outcome = if simultaneous {
                effect
                    .prepare_simultaneous_player_action(&game, &mut ctx)
                    .unwrap()
                    .commit(&mut game, &mut ctx)
                    .unwrap()
            } else {
                effect.execute(&mut game, &mut ctx).unwrap()
            };
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, if gain { 23 } else { 17 });
            assert_eq!(outcome.count_or_zero(), 3);
            assert_eq!(outcome.events.len(), 1);
            let player = if gain {
                outcome.events[0]
                    .downcast::<crate::events::LifeGainEvent>()
                    .unwrap()
                    .player
            } else {
                outcome.events[0]
                    .downcast::<crate::events::LifeLossEvent>()
                    .unwrap()
                    .player
            };
            assert_eq!(player, bob);
        }
    }
}

#[test]
fn life_loss_instead_executes_payload_without_losing_original_amount() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, "Loss replacement", 1);
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            WouldLoseLifeMatcher::you(),
            ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(5)]),
        ));
    let mut ctx = ExecutionContext::new_default(source, alice);
    let outcome = LoseLifeEffect::you(3).execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 25);
    assert_eq!(
        outcome.count_or_zero(),
        0,
        "none of the original life loss happened"
    );
    assert_eq!(outcome.events.len(), 1);
    assert!(
        outcome.events[0]
            .downcast::<crate::events::LifeGainEvent>()
            .is_some()
    );
}

#[test]
fn nested_life_replacement_preserves_history_but_not_across_independent_events() {
    use crate::effect::{Effect, EventValueSpec, Value};
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, "Nested loss source", 1);
    game.effect_store.replacement_effects.add_resolution_effect(
        ReplacementEffect::with_matcher(
            source,
            alice,
            WouldLoseLifeMatcher::you(),
            ReplacementAction::Double,
        )
        .with_priority_override(crate::events::ReplacementPriority::SelfReplacement),
    );
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            WouldLoseLifeMatcher::you(),
            ReplacementAction::Instead(vec![Effect::lose_life(Value::EventValue(
                EventValueSpec::Amount,
            ))]),
        ));
    let mut ctx = ExecutionContext::new_default(source, alice);
    for expected_life in [14, 8] {
        let outcome = LoseLifeEffect::you(3).execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.player(alice).unwrap().life, expected_life);
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(
            outcome.events[0]
                .downcast::<crate::events::LifeLossEvent>()
                .unwrap()
                .amount,
            6
        );
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
        assert!(
            ctx.replacement
                .suppressed_replacement_effect_keys
                .is_empty()
        );
        assert!(ctx.triggering_event.is_none());
    }
}

#[test]
fn life_replacement_payload_errors_propagate_without_original_loss() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, "Invalid payload source", 1);
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            WouldLoseLifeMatcher::you(),
            ReplacementAction::Instead(vec![
                crate::effect::Effect::gain_life(1),
                crate::effect::Effect::gain_life(crate::effect::Value::X),
            ]),
        ));
    let mut ctx = ExecutionContext::new_default(source, alice);
    assert!(LoseLifeEffect::you(3).execute(&mut game, &mut ctx).is_err());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(ctx.source, source);
    assert!(ctx.triggering_event.is_none());
}

#[test]
fn pending_life_replacement_payload_does_not_execute_later_instructions() {
    use crate::effect::Effect;
    struct Choice {
        pause: bool,
        pending: bool,
    }
    impl DecisionMaker for Choice {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.pending = self.pause;
            !self.pause
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, "Interactive payload source", 1);
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                WouldLoseLifeMatcher::you(),
                ReplacementAction::Instead(vec![
                    Effect::may(vec![Effect::gain_life(1)]),
                    Effect::gain_life(2),
                ]),
            ));
    let mut dm = Choice {
        pause: true,
        pending: false,
    };
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    let outcome = LoseLifeEffect::you(3).execute(&mut game, &mut ctx).unwrap();
    assert!(outcome.events.is_empty());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some(),
        "pending payload must retain its one-shot replacement"
    );
    drop(ctx);
    dm.pause = false;
    dm.pending = false;
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    LoseLifeEffect::you(3).execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 23);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
}

#[test]
fn set_life_total_executes_instead_payload_for_both_entry_points() {
    for simultaneous in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = permanent(&mut game, "Set life source", 1);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(source, alice, WouldLoseLifeMatcher::you(),
                ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(5)])),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SetLifeTotalEffect::you(17);
        let outcome = if simultaneous {
            effect.prepare_simultaneous_player_action(&game, &mut ctx).unwrap().commit(&mut game, &mut ctx).unwrap()
        } else { effect.execute(&mut game, &mut ctx).unwrap() };
        assert_eq!(game.player(alice).unwrap().life, 25);
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.events.len(), 1);
        assert!(outcome.events[0].downcast::<crate::events::LifeGainEvent>().is_some());
    }
}

#[test]
fn pending_second_exchange_replacement_does_not_commit_the_first_player() {
    struct Choice { pause: bool, pending: bool }
    impl DecisionMaker for Choice {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.player, PlayerId::from_index(1));
            self.pending = self.pause;
            if self.pause { Vec::new() } else { vec![1] }
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.player_mut(bob).unwrap().life = 23;
    let source = permanent(&mut game, "Exchange source", 1);
    for _ in 0..2 {
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(source, alice,
                WouldLoseLifeMatcher::new(PlayerFilter::Specific(bob)), ReplacementAction::Double),
        );
    }
    let effect = ExchangeLifeTotalsEffect::new(PlayerFilter::You, PlayerFilter::Specific(bob));
    let mut dm = Choice { pause: true, pending: false };
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    let outcome = effect.execute(&mut game, &mut ctx).unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    assert!(outcome.events.is_empty());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(bob).unwrap().life, 23);
    drop(ctx);
    dm.pause = false;
    dm.pending = false;
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    let outcome = effect.execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 23);
    assert_eq!(game.player(bob).unwrap().life, 11);
    assert_eq!(outcome.events.len(), 2);
}


#[test]
fn exchange_replacement_choices_use_apnap_and_precommit_life_totals() {
    #[derive(Default)]
    struct Choices { players: Vec<PlayerId> }
    impl DecisionMaker for Choices {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 20);
            assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 23);
            self.players.push(ctx.player);
            vec![0]
        }
    }
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.player_mut(bob).unwrap().life = 23;
    let source = permanent(&mut game, "Exchange source", 1);
    for _ in 0..2 {
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(source, alice,
                WouldGainLifeMatcher::new(PlayerFilter::Specific(alice)), ReplacementAction::Double),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(source, alice,
                WouldLoseLifeMatcher::new(PlayerFilter::Specific(bob)), ReplacementAction::Double),
        );
    }
    // Authored order is deliberately the reverse of APNAP order.
    let effect = ExchangeLifeTotalsEffect::new(PlayerFilter::Specific(bob), PlayerFilter::You);
    let mut dm = Choices::default();
    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
    let outcome = effect.execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 32);
    assert_eq!(game.player(bob).unwrap().life, 11);
    assert_eq!(outcome.events.len(), 2);
    drop(ctx);
    assert_eq!(dm.players, vec![alice, bob]);
}


#[test]
fn radiation_replacements_preserve_radiation_flag_and_execute_instead() {
    #[derive(Clone, Debug)]
    struct RadiationLoss;
    impl ReplacementMatcher for RadiationLoss {
        fn matches_prepared_event(&self, event: &dyn GameEventType, _: &crate::events::context::PreparedEventContext) -> bool {
            crate::events::downcast_event::<crate::events::LifeLossEvent>(event)
                .is_some_and(|loss| loss.from_radiation)
        }
        fn display(&self) -> String { "Radiation life loss".into() }
    }
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, "Replacement source", 1);
    game.player_mut(alice).unwrap().add_counters(CounterType::Rad, 1);
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Nonland").card_types(vec![CardType::Sorcery]).build(),
        alice, Zone::Library,
    );
    game.effect_store.replacement_effects.add_resolution_effect(
        ReplacementEffect::with_matcher(source, alice, RadiationLoss,
            ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(3)])),
    );
    let mut ctx = ExecutionContext::new_default(source, alice);
    let outcome = RadiationEffect::new().execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 23);
    assert_eq!(game.player(alice).unwrap().counter_count(CounterType::Rad), 0);
    assert!(outcome.events.iter().any(|event| event.downcast::<crate::events::LifeGainEvent>().is_some()));
    assert!(!outcome.events.iter().any(|event| event.downcast::<crate::events::LifeLossEvent>().is_some()));
}


#[test]
fn radiation_payload_error_restores_milling_life_and_counters() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = permanent(&mut game, "Replacement source", 1);
    game.player_mut(alice).unwrap().add_counters(CounterType::Rad, 1);
    let card = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Nonland").card_types(vec![CardType::Sorcery]).build(),
        alice, Zone::Library,
    );
    game.effect_store.replacement_effects.add_resolution_effect(
        ReplacementEffect::with_matcher(source, alice, WouldLoseLifeMatcher::you(),
            ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(3),
                crate::effect::Effect::gain_life(crate::effect::Value::X)])),
    );
    let mut ctx = ExecutionContext::new_default(source, alice);
    assert!(RadiationEffect::new().execute(&mut game, &mut ctx).is_err());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(alice).unwrap().counter_count(CounterType::Rad), 1);
    assert_eq!(game.object(card).unwrap().zone, Zone::Library);
}
