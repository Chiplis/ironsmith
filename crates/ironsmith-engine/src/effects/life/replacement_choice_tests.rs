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
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        WouldLoseLifeMatcher::you().matches_event(event, ctx)
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
        game.create_object_from_card(
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
            assert!(outcome.events.is_empty());
        }
    }
}
