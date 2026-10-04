use super::*;
use crate::effect::Effect;
use crate::events::ReplacementPriority;
use crate::events::damage::matchers::{DamageFromSourceMatcher, DamageToPlayerMatcher};
use crate::ids::{CardId, PlayerId};
use crate::replacement::{
    EventModification, RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect,
};
use crate::target::ObjectFilter;
use crate::zone::Zone;

fn setup() -> (GameState, crate::ids::ObjectId) {
    let mut game = GameState::new(
        vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dave".into()],
        20,
    );
    let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Split source")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let source =
        game.create_object_from_definition(&definition, PlayerId::from_index(0), Zone::Battlefield);
    game.take_pending_trigger_events();
    (game, source)
}

fn terminal(game: &mut GameState, source: crate::ids::ObjectId, player: PlayerId, mode: u8) {
    let action = match mode {
        0 => return,
        1 => ReplacementAction::Prevent,
        _ => ReplacementAction::Instead(vec![Effect::gain_life(1)]),
    };
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            PlayerId::from_index(0),
            DamageToPlayerMatcher::new(PlayerFilter::Specific(player)),
            action,
        ));
}

#[test]
fn split_damage_keeps_split_time_history_through_modified_prevented_and_instead_branches() {
    for before_split in [false, true] {
        for mode in 0..3 {
            let (mut game, source) = setup();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let carol = PlayerId::from_index(2);
            let mut multiplier = ReplacementEffect::with_matcher(
                source,
                alice,
                DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            );
            let mut redirect = ReplacementEffect::with_matcher(
                source,
                alice,
                DamageToPlayerMatcher::new(PlayerFilter::Specific(bob)),
                ReplacementAction::RedirectDamageAmount {
                    target: RedirectTarget::ToPlayer(carol),
                    which: RedirectWhich::First,
                    amount: 3,
                },
            );
            if before_split {
                multiplier.priority_override = Some(ReplacementPriority::SelfReplacement);
            } else {
                redirect.priority_override = Some(ReplacementPriority::SelfReplacement);
            }
            game.effect_store
                .replacement_effects
                .add_resolution_effect(multiplier);
            game.effect_store
                .replacement_effects
                .add_resolution_effect(redirect);
            terminal(&mut game, source, carol, mode);
            let remainder = if before_split { 7 } else { 4 };
            let redirected = if before_split { 3 } else { 6 };
            for iteration in 1..=2 {
                let mut ctx = ExecutionContext::new_default(source, alice);
                let outcome = DealDamageEffect::new(5, ChooseSpec::SpecificPlayer(bob))
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(
                    outcome.count_or_zero(),
                    remainder + if mode == 0 { redirected } else { 0 }
                );
                assert_eq!(i64::from(game.player(bob).unwrap().life), 20 - iteration * remainder);
                assert_eq!(
                    i64::from(game.player(carol).unwrap().life),
                    20 - if mode == 0 { iteration * redirected } else { 0 }
                );
                assert_eq!(
                    i64::from(game.player(alice).unwrap().life),
                    20 + if mode == 2 { iteration } else { 0 }
                );
                assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
                assert!(
                    ctx.replacement
                        .suppressed_replacement_effect_keys
                        .is_empty()
                );
                let mut events = outcome.events;
                events.extend(game.take_pending_trigger_events());
                let damage = events
                    .iter()
                    .filter_map(|event| event.downcast::<DamageEvent>())
                    .collect::<Vec<_>>();
                assert_eq!(damage.len(), if mode == 0 { 2 } else { 1 });
                assert_eq!(
                    damage
                        .iter()
                        .filter(|event| event.target == DamageTarget::Player(bob))
                        .map(|event| event.amount)
                        .collect::<Vec<_>>(),
                    vec![remainder as u32]
                );
                assert_eq!(
                    events
                        .iter()
                        .filter_map(|event| event.downcast::<LifeGainEvent>())
                        .count(),
                    usize::from(mode == 2)
                );
            }
        }
    }
}

#[test]
fn repeated_partial_redirects_preserve_all_remainders_after_terminal_primary_outcomes() {
    for mode in 0..3 {
        let (mut game, source) = setup();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let dave = PlayerId::from_index(3);
        for (from, to, amount) in [(bob, carol, 3), (carol, dave, 1)] {
            game.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    DamageToPlayerMatcher::new(PlayerFilter::Specific(from)),
                    ReplacementAction::RedirectDamageAmount {
                        target: RedirectTarget::ToPlayer(to),
                        which: RedirectWhich::First,
                        amount,
                    },
                ),
            );
        }
        terminal(&mut game, source, dave, mode);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = DealDamageEffect::new(5, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(outcome.count_or_zero(), if mode == 0 { 5 } else { 4 });
        assert_eq!(i64::from(game.player(bob).unwrap().life), 18);
        assert_eq!(i64::from(game.player(carol).unwrap().life), 18);
        assert_eq!(
            game.player(dave).unwrap().life,
            if mode == 0 { 19 } else { 20 }
        );
        assert_eq!(
            i64::from(game.player(alice).unwrap().life),
            if mode == 2 { 21 } else { 20 }
        );
        let mut events = outcome.events;
        events.extend(game.take_pending_trigger_events());
        let amounts = events
            .iter()
            .filter_map(|event| event.downcast::<DamageEvent>())
            .map(|event| (event.target, event.amount))
            .collect::<Vec<_>>();
        assert_eq!(amounts.len(), if mode == 0 { 3 } else { 2 });
        assert!(amounts.contains(&(DamageTarget::Player(bob), 2)));
        assert!(amounts.contains(&(DamageTarget::Player(carol), 2)));
        if mode == 0 {
            assert!(amounts.contains(&(DamageTarget::Player(dave), 1)));
        }
    }
}

struct Answers {
    pause: bool,
    calls: usize,
    pending: bool,
}
impl crate::decision::DecisionMaker for Answers {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert!(
            !self.pending,
            "stop at the unanswered split-branch decision"
        );
        self.calls += 1;
        self.pending = self.pause && self.calls == 2;
        !self.pending
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}

#[test]
fn split_damage_payload_failure_and_pause_restore_all_branches_and_replay_once() {
    for on_remainder in [false, true] {
        for pause in [false, true] {
            let (mut game, source) = setup();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let carol = PlayerId::from_index(2);
            let mut redirect = ReplacementEffect::with_matcher(
                source,
                alice,
                DamageToPlayerMatcher::new(PlayerFilter::Specific(bob)),
                ReplacementAction::RedirectDamageAmount {
                    target: RedirectTarget::ToPlayer(carol),
                    which: RedirectWhich::First,
                    amount: 2,
                },
            );
            redirect.priority_override = Some(ReplacementPriority::SelfReplacement);
            game.effect_store
                .replacement_effects
                .add_resolution_effect(redirect);
            let mut payload = vec![Effect::gain_life(2)];
            if pause {
                payload.extend([
                    Effect::may(vec![Effect::gain_life(1)]),
                    Effect::may(vec![Effect::gain_life(3)]),
                ]);
            } else {
                payload.push(Effect::gain_life(crate::effect::Value::X));
            }
            let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    DamageToPlayerMatcher::new(PlayerFilter::Specific(if on_remainder {
                        bob
                    } else {
                        carol
                    })),
                    ReplacementAction::Instead(payload),
                ),
            );
            let mut dm = Answers {
                pause,
                calls: 0,
                pending: false,
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let result = DealDamageEffect::new(5, ChooseSpec::SpecificPlayer(bob))
                .execute(&mut game, &mut ctx);
            if pause {
                let outcome = result.unwrap();
                assert_eq!(outcome.count_or_zero(), 0);
                assert!(outcome.events.is_empty());
                assert!(ctx.decision_maker.awaiting_choice());
            } else {
                assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
            }
            assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
            assert!(
                ctx.replacement
                    .suppressed_replacement_effect_keys
                    .is_empty()
            );
            drop(ctx);
            assert_eq!(i64::from(game.player(alice).unwrap().life), 20);
            assert_eq!(i64::from(game.player(bob).unwrap().life), 20);
            assert_eq!(i64::from(game.player(carol).unwrap().life), 20);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(one_shot)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            if pause {
                assert_eq!(dm.calls, 2);
                let mut replay = Answers {
                    pause: false,
                    calls: 0,
                    pending: false,
                };
                let mut ctx = ExecutionContext::new(source, alice, &mut replay);
                let outcome = DealDamageEffect::new(5, ChooseSpec::SpecificPlayer(bob))
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(outcome.count_or_zero(), if on_remainder { 2 } else { 3 });
                assert_eq!(i64::from(game.player(alice).unwrap().life), 26);
                assert_eq!(
                    i64::from(game.player(bob).unwrap().life),
                    if on_remainder { 20 } else { 17 }
                );
                assert_eq!(
                    i64::from(game.player(carol).unwrap().life),
                    if on_remainder { 18 } else { 20 }
                );
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(one_shot)
                        .is_none()
                );
                let mut events = outcome.events;
                events.extend(game.take_pending_trigger_events());
                let damage = events
                    .iter()
                    .filter_map(|event| event.downcast::<DamageEvent>())
                    .collect::<Vec<_>>();
                assert_eq!(damage.len(), 1);
                assert_eq!(
                    (damage[0].target, damage[0].amount),
                    if on_remainder {
                        (DamageTarget::Player(carol), 2)
                    } else {
                        (DamageTarget::Player(bob), 3)
                    }
                );
                assert_eq!(
                    events
                        .iter()
                        .filter_map(|event| event.downcast::<LifeGainEvent>())
                        .count(),
                    3
                );
            }
        }
    }
}
