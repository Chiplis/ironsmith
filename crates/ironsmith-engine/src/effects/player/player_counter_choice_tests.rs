use super::*;
use crate::ability::Ability;
use crate::card::CardBuilder;
use crate::decision::DecisionMaker;
use crate::decisions::context::{ProliferateContext, SelectOptionsContext};
use crate::decisions::specs::ProliferateResponse;
use crate::effects::{
    DoubleCountersEffect, EnergyCountersEffect, ExperienceCountersEffect, ForPlayersEffect,
    PoisonCountersEffect, ProliferateEffect,
};
use crate::events::EventKind;
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;
use std::collections::VecDeque;

// Replay consumes the supplied prefix and pauses at the next unanswered
// replacement. A second pass starts from the original game checkpoint.
struct CounterChoiceQueue {
    answers: VecDeque<usize>,
    expected_player: Option<PlayerId>,
    seen_players: Vec<PlayerId>,
    pending: bool,
    replacement_calls: usize,
    proliferate_calls: usize,
}

impl CounterChoiceQueue {
    fn new(player: PlayerId, answers: &[usize]) -> Self {
        Self {
            answers: answers.iter().copied().collect(),
            expected_player: Some(player),
            seen_players: Vec::new(),
            pending: false,
            replacement_calls: 0,
            proliferate_calls: 0,
        }
    }
}

impl DecisionMaker for CounterChoiceQueue {
    fn awaiting_choice(&self) -> bool {
        self.pending
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        assert!(
            !self.pending,
            "counter execution continued beyond a pending choice"
        );
        if let Some(player) = self.expected_player {
            assert_eq!(ctx.player, player);
        }
        self.seen_players.push(ctx.player);
        assert_eq!(ctx.options.len(), 2);
        assert!(ctx.description.contains("replacement"));
        self.replacement_calls += 1;
        match self.answers.pop_front() {
            Some(index) => vec![index],
            None => {
                self.pending = true;
                Vec::new()
            }
        }
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &ProliferateContext,
    ) -> ProliferateResponse {
        assert!(
            !self.pending,
            "another proliferate ran before answering its replacement"
        );
        self.proliferate_calls += 1;
        ProliferateResponse {
            permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
            players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
        }
    }
}

fn counter_replacement_game() -> (GameState, ObjectId, PlayerId, PlayerId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (name, ability) in [
        (
            "One extra counter",
            StaticAbility::add_player_counters_placement_replacement(
                PlayerFilter::Any,
                None,
                1,
                "Add one counter".into(),
            ),
        ),
        (
            "Twice as many counters",
            StaticAbility::double_player_counters_replacement(
                PlayerFilter::Any,
                None,
                "Double the counters".into(),
            ),
        ),
    ] {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(object)
            .unwrap()
            .abilities_mut()
            .push(Ability::static_ability(ability));
    }
    game.update_replacement_effects();
    let source = game.new_object_id();
    (game, source, alice, bob)
}

#[test]
fn every_player_counter_effect_waits_and_honors_the_second_replacement() {
    let bob = PlayerId::from_index(1);
    let effects: Vec<(CounterType, Box<dyn EffectExecutor>)> = vec![
        (
            CounterType::Poison,
            Box::new(PoisonCountersEffect::new(1, PlayerFilter::Specific(bob))),
        ),
        (
            CounterType::Energy,
            Box::new(EnergyCountersEffect::new(1, PlayerFilter::Specific(bob))),
        ),
        (
            CounterType::Experience,
            Box::new(ExperienceCountersEffect::new(
                1,
                PlayerFilter::Specific(bob),
            )),
        ),
        (
            CounterType::Rad,
            Box::new(PlayerCountersEffect::new(
                CounterType::Rad,
                1,
                PlayerFilter::Specific(bob),
            )),
        ),
        (
            CounterType::Named("ticket".into()),
            Box::new(crate::effects::TicketCountersEffect::new(1, PlayerFilter::Specific(bob))),
        ),
    ];
    for (counter_type, effect) in effects {
        let (checkpoint, source, alice, bob) = counter_replacement_game();
        for (answers, expected, pending) in [
            (&[][..], 0, true),
            (&[1][..], 3, false),
            (&[0][..], 4, false),
        ] {
            let mut game = checkpoint.clone();
            let mut dm = CounterChoiceQueue::new(bob, answers);
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(
                game.player(bob).unwrap().counter_count(counter_type),
                expected,
                "{counter_type:?}, {answers:?}"
            );
            assert_eq!(dm.pending, pending);
            assert_eq!(dm.replacement_calls, 1);
            assert_eq!(outcome.events.len(), usize::from(!pending));
            if pending {
                assert_eq!(outcome.count_or_zero(), 0);
            }
        }
    }
}

#[test]
fn repeated_proliferate_stops_at_each_player_counter_replacement() {
    let (mut checkpoint, source, alice, bob) = counter_replacement_game();
    checkpoint.player_mut(bob).unwrap().energy_counters = 1;
    for (answers, expected, pending, calls) in [
        (&[][..], 1, true, 1),
        (&[1][..], 1, true, 2),
        (&[1, 1][..], 7, false, 2),
    ] {
        let mut game = checkpoint.clone();
        let mut dm = CounterChoiceQueue::new(bob, answers);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = ProliferateEffect::new(2)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(bob).unwrap().energy_counters, expected);
        assert_eq!(dm.pending, pending);
        assert_eq!(dm.replacement_calls, calls);
        assert_eq!(dm.proliferate_calls, calls);
        if pending {
            assert_eq!(outcome.count_or_zero(), 0);
            assert!(outcome.events.is_empty());
            let mut dm = CounterChoiceQueue::new(bob, &[1, 1]);
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let replay = ProliferateEffect::new(2)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.player(bob).unwrap().energy_counters, 7);
            assert_eq!(replay.count_or_zero(), 2);
            assert_eq!(replay.events.len(), 4);
            assert!(!dm.pending);
            assert!(game.take_pending_trigger_events().is_empty());
        }
        if !pending {
            assert_eq!(outcome.count_or_zero(), 2);
            assert_eq!(
                outcome
                    .events
                    .iter()
                    .filter(|event| event.kind() == EventKind::KeywordAction)
                    .count(),
                2
            );
        }
    }
}

#[test]
fn doubling_player_counters_waits_before_processing_the_next_counter_type() {
    let (mut checkpoint, source, alice, bob) = counter_replacement_game();
    checkpoint.player_mut(bob).unwrap().energy_counters = 1;
    checkpoint.player_mut(bob).unwrap().experience_counters = 2;
    for (answers, energy, experience, pending, calls) in [
        (&[][..], 1, 2, true, 1),
        (&[1][..], 1, 2, true, 2),
        (&[1, 1][..], 4, 7, false, 2),
    ] {
        let mut game = checkpoint.clone();
        let mut dm = CounterChoiceQueue::new(bob, answers);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = DoubleCountersEffect::new(None, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(bob).unwrap().energy_counters, energy);
        assert_eq!(game.player(bob).unwrap().experience_counters, experience);
        assert_eq!(dm.pending, pending);
        assert_eq!(dm.replacement_calls, calls);
        assert_eq!(
            outcome.events.len(),
            if pending { 0 } else { answers.len() }
        );
        if !pending {
            assert_eq!(outcome.count_or_zero(), 8);
        }
        if pending {
            assert_eq!(outcome.count_or_zero(), 0);
            let mut dm = CounterChoiceQueue::new(bob, &[1, 1]);
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = DoubleCountersEffect::new(None, ChooseSpec::SpecificPlayer(bob))
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.player(bob).unwrap().energy_counters, 4);
            assert_eq!(game.player(bob).unwrap().experience_counters, 7);
            assert_eq!(outcome.count_or_zero(), 8);
            assert_eq!(outcome.events.len(), 2);
            assert!(!dm.pending);
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[test]
fn simultaneous_player_counters_stop_before_the_next_player_when_replacement_is_pending() {
    let (checkpoint, source, alice, bob) = counter_replacement_game();
    let effect = ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![crate::effect::Effect::new(PlayerCountersEffect::new(
            CounterType::Rad,
            1,
            PlayerFilter::IteratedPlayer,
        ))],
    );
    for (answers, alice_counters, bob_counters, pending, players) in [
        (&[][..], 0, 0, true, vec![alice]),
        (&[1][..], 0, 0, true, vec![alice, bob]),
        (&[1, 1][..], 3, 3, false, vec![alice, bob]),
    ] {
        let mut game = checkpoint.clone();
        let mut dm = CounterChoiceQueue::new(alice, answers);
        dm.expected_player = None;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            game.player(alice).unwrap().counter_count(CounterType::Rad),
            alice_counters
        );
        assert_eq!(
            game.player(bob).unwrap().counter_count(CounterType::Rad),
            bob_counters
        );
        assert_eq!(dm.pending, pending);
        assert_eq!(dm.seen_players, players);
        assert_eq!(outcome.events.len(), if pending { 0 } else { 2 });
        assert!(game.take_pending_trigger_events().is_empty());
        if pending {
            // Replay against the actual restored state, not a fresh fixture.
            let mut resumed = CounterChoiceQueue::new(alice, &[1, 1]);
            resumed.expected_player = None;
            let mut resumed_ctx = ExecutionContext::new(source, alice, &mut resumed);
            let completed = effect.execute(&mut game, &mut resumed_ctx).unwrap();
            assert!(!resumed_ctx.decision_maker.awaiting_choice());
            assert_eq!(
                game.player(alice).unwrap().counter_count(CounterType::Rad),
                3
            );
            assert_eq!(game.player(bob).unwrap().counter_count(CounterType::Rad), 3);
            assert_eq!(resumed.seen_players, vec![alice, bob]);
            assert_eq!(completed.events.len(), 2);
            let markers = completed
                .events
                .iter()
                .map(|event| {
                    event
                        .downcast::<crate::events::MarkersChangedEvent>()
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                markers[0].location,
                crate::marker::MarkerLocation::Player(alice)
            );
            assert_eq!(
                markers[1].location,
                crate::marker::MarkerLocation::Player(bob)
            );
            assert!(markers.iter().all(|event| event.amount == 3));
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[test]
fn counter_outcome_reports_the_resolved_placement_amount_for_every_player_effect() {
    let bob = PlayerId::from_index(1);
    let effects: Vec<Box<dyn EffectExecutor>> = vec![
        Box::new(PoisonCountersEffect::new(1, PlayerFilter::Specific(bob))),
        Box::new(EnergyCountersEffect::new(1, PlayerFilter::Specific(bob))),
        Box::new(ExperienceCountersEffect::new(
            1,
            PlayerFilter::Specific(bob),
        )),
        Box::new(PlayerCountersEffect::new(
            CounterType::Rad,
            1,
            PlayerFilter::Specific(bob),
        )),
    ];
    for effect in effects {
        let (mut game, source, alice, bob) = counter_replacement_game();
        let mut dm = CounterChoiceQueue::new(bob, &[1]);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            outcome.count_or_zero(),
            3,
            "summary must use the resolved count"
        );
        let marker = outcome.events[0]
            .downcast::<crate::events::MarkersChangedEvent>()
            .unwrap();
        assert_eq!(marker.amount, 3);
    }
}

fn check_player_counter_replacement(instead: bool) {
    use crate::replacement::{RedirectTarget, RedirectWhich, ReplacementAction};
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Counter source").build(),
        alice,
        Zone::Battlefield,
    );
    let mut replacement = StaticAbility::double_player_counters_replacement(
        PlayerFilter::Specific(alice),
        Some(CounterType::Rad),
        "Counter proposal".into(),
    )
    .generate_replacement_effect(source, alice)
    .unwrap();
    replacement.replacement = if instead {
        ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2)])
    } else {
        ReplacementAction::Redirect {
            target: RedirectTarget::ToPlayer(bob),
            which: RedirectWhich::First,
        }
    };
    game.effect_store
        .replacement_effects
        .add_resolution_effect(replacement);
    let mut ctx = ExecutionContext::new_default(source, alice);
    let outcome = PlayerCountersEffect::new(CounterType::Rad, 2, PlayerFilter::Specific(alice))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        game.player(alice).unwrap().counter_count(CounterType::Rad),
        0
    );
    if instead {
        assert_eq!(game.player(alice).unwrap().life, 22);
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert_eq!(outcome.events.len(), 1);
        assert!(
            outcome.events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .is_some()
        );
    } else {
        assert_eq!(game.player(bob).unwrap().counter_count(CounterType::Rad), 2);
        assert_eq!(outcome.count_or_zero(), 2);
        assert_eq!(outcome.events.len(), 1);
        let marker = outcome.events[0]
            .downcast::<crate::events::MarkersChangedEvent>()
            .unwrap();
        assert_eq!(marker.location, crate::marker::MarkerLocation::Player(bob));
        assert_eq!(marker.amount, 2);
    }
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn player_counter_replacement_preserves_redirect() {
    check_player_counter_replacement(false);
}
#[test]
fn player_counter_replacement_preserves_instead_outcomes() {
    check_player_counter_replacement(true);
}

#[test]
fn player_counter_instead_payload_error_restores_partial_payload_and_one_shot() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Counter source").build(),
        alice,
        Zone::Battlefield,
    );
    let mut replacement = StaticAbility::double_player_counters_replacement(
        PlayerFilter::Specific(alice),
        Some(CounterType::Rad),
        "Counter proposal".into(),
    )
    .generate_replacement_effect(source, alice)
    .unwrap();
    replacement.replacement = crate::replacement::ReplacementAction::Instead(vec![
        crate::effect::Effect::gain_life(2),
        crate::effect::Effect::lose_life(Value::X),
    ]);
    let one_shot = game
        .effect_store
        .replacement_effects
        .add_one_shot_effect(replacement);
    let mut ctx = ExecutionContext::new_default(source, alice);
    let result = PlayerCountersEffect::new(CounterType::Rad, 2, PlayerFilter::Specific(alice))
        .execute(&mut game, &mut ctx);
    assert!(result.is_err(), "counter replacement errors must escape");
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(alice).unwrap().counter_count(CounterType::Rad),
        0
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn nested_player_counter_payload_reads_modified_amount_and_retains_history() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Counter source").build(),
        alice,
        Zone::Battlefield,
    );
    let prototype = StaticAbility::double_player_counters_replacement(
        PlayerFilter::Specific(alice),
        Some(CounterType::Rad),
        "Counter proposal".into(),
    )
    .generate_replacement_effect(source, alice)
    .unwrap();
    game.effect_store.replacement_effects.add_resolution_effect(
        prototype
            .clone()
            .with_priority_override(crate::events::ReplacementPriority::SelfReplacement),
    );
    let mut instead = prototype;
    instead.replacement =
        crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::new(
            PlayerCountersEffect::new(
                CounterType::Rad,
                Value::EventValue(crate::effect::EventValueSpec::Amount),
                PlayerFilter::Specific(alice),
            ),
        )]);
    game.effect_store
        .replacement_effects
        .add_resolution_effect(instead);
    let mut ctx = ExecutionContext::new_default(source, alice);
    for expected in [4, 8] {
        let outcome = PlayerCountersEffect::new(CounterType::Rad, 2, PlayerFilter::Specific(alice))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            game.player(alice).unwrap().counter_count(CounterType::Rad),
            expected
        );
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(
            outcome.events[0]
                .downcast::<crate::events::MarkersChangedEvent>()
                .unwrap()
                .amount,
            4
        );
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
        assert!(
            ctx.replacement
                .suppressed_replacement_effect_keys
                .is_empty()
        );
    }
}

#[test]
fn player_counter_limit_commits_its_first_event_then_locks_subsequent_events() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Counter limit").build(),
        alice,
        Zone::Battlefield,
    );
    let replacement = StaticAbility::player_counter_per_turn_limit_replacement(
        PlayerFilter::Specific(alice),
        CounterType::Poison,
        1,
        "Limit poison".into(),
    )
    .generate_replacement_effect(source, alice)
    .unwrap();
    game.effect_store
        .replacement_effects
        .add_resolution_effect(replacement);
    let first = game
        .add_player_counters_with_source(alice, CounterType::Poison, 4, Some(source), Some(alice))
        .unwrap();
    assert_eq!(first.count_or_zero(), 1);
    assert_eq!(game.player(alice).unwrap().poison_counters, 1);
    assert_eq!(first.events.len(), 1);
    assert_eq!(
        first.events[0]
            .downcast::<crate::events::MarkersChangedEvent>()
            .unwrap()
            .count_after,
        Some(1)
    );
    let next = game
        .add_player_counters_with_source(alice, CounterType::Poison, 4, Some(source), None)
        .unwrap();
    assert_eq!(next.count_or_zero(), 0);
    assert!(next.events.is_empty());
    assert_eq!(game.player(alice).unwrap().poison_counters, 1);
}

#[test]
fn player_counter_payload_pause_or_error_restores_earlier_kinds_and_replays_once() {
    struct Answers {
        pause: bool,
        pending: bool,
        calls: usize,
    }
    impl DecisionMaker for Answers {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            assert!(
                !self.pending,
                "counter payload cannot ask later questions while pending"
            );
            self.calls += 1;
            self.pending = self.pause;
            !self.pause
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
        fn decide_proliferate(
            &mut self,
            _: &GameState,
            ctx: &ProliferateContext,
        ) -> ProliferateResponse {
            assert!(!self.pending);
            ProliferateResponse {
                permanents: Vec::new(),
                players: ctx
                    .eligible_players
                    .iter()
                    .map(|(player, _)| *player)
                    .collect(),
            }
        }
    }
    for operation in 0..3 {
        for pending_case in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let source = game.create_object_from_card(
                &CardBuilder::new(CardId::new(), "Counter source").build(),
                alice,
                Zone::Battlefield,
            );
            game.player_mut(bob).unwrap().energy_counters = 1;
            game.player_mut(bob).unwrap().experience_counters = 1;
            let mut replacement = StaticAbility::double_player_counters_replacement(
                PlayerFilter::Specific(bob),
                Some(CounterType::Experience),
                "Counter proposal".into(),
            )
            .generate_replacement_effect(source, alice)
            .unwrap();
            let mut payload = vec![crate::effect::Effect::gain_life(2)];
            if pending_case {
                payload.push(crate::effect::Effect::may(vec![
                    crate::effect::Effect::gain_life(1),
                ]));
                payload.push(crate::effect::Effect::may(vec![
                    crate::effect::Effect::gain_life(3),
                ]));
            } else {
                payload.push(crate::effect::Effect::lose_life(Value::X));
            }
            replacement.replacement = crate::replacement::ReplacementAction::Instead(payload);
            let one_shot = game
                .effect_store
                .replacement_effects
                .add_one_shot_effect(replacement);
            let effect: Box<dyn EffectExecutor> = match operation {
                0 => Box::new(PlayerCountersEffect::new(
                    CounterType::Experience,
                    2,
                    PlayerFilter::Specific(bob),
                )),
                1 => Box::new(DoubleCountersEffect::new(
                    None,
                    ChooseSpec::SpecificPlayer(bob),
                )),
                _ => Box::new(ProliferateEffect::new(2)),
            };
            game.take_pending_trigger_events();
            let mut dm = Answers {
                pause: pending_case,
                pending: false,
                calls: 0,
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ctx.iteration.iterated_player = Some(bob);
            ctx.set_tagged_players("retained", vec![alice]);
            let result = effect.execute(&mut game, &mut ctx);
            assert_eq!(result.is_err(), !pending_case);
            assert_eq!(ctx.iteration.iterated_player, Some(bob));
            assert_eq!(ctx.get_tagged_players("retained"), Some(&vec![alice]));
            if pending_case {
                let outcome = result.unwrap();
                assert_eq!(outcome.count_or_zero(), 0);
                assert!(outcome.events.is_empty());
            }
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().energy_counters, 1);
            assert_eq!(game.player(bob).unwrap().experience_counters, 1);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(one_shot)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            if pending_case {
                assert_eq!(dm.calls, 1);
                let mut dm = Answers {
                    pause: false,
                    pending: false,
                    calls: 0,
                };
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                let outcome = effect.execute(&mut game, &mut ctx).unwrap();
                assert_eq!(dm.calls, 2);
                assert!(!dm.pending);
                assert_eq!(game.player(alice).unwrap().life, 26);
                assert_eq!(
                    game.player(bob).unwrap().energy_counters,
                    match operation {
                        0 => 1,
                        1 => 2,
                        _ => 3,
                    }
                );
                assert_eq!(
                    game.player(bob).unwrap().experience_counters,
                    if operation == 2 { 2 } else { 1 }
                );
                assert_eq!(outcome.count_or_zero(), operation);
                assert_eq!(
                    outcome.events.len(),
                    match operation {
                        0 => 3,
                        1 => 4,
                        _ => 8,
                    }
                );
                assert_eq!(
                    outcome
                        .events
                        .iter()
                        .filter(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
                        .count(),
                    3
                );
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(one_shot)
                        .is_none()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            }
        }
    }
}

#[test]
fn simultaneous_ticket_counters_stop_and_replay_after_each_recipient() {
    let (checkpoint, source, alice, bob) = counter_replacement_game();
    let effect = ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![crate::effect::Effect::new(crate::effects::TicketCountersEffect::new(
            1,
            PlayerFilter::IteratedPlayer,
        ))],
    );
    for (answers, alice_counters, bob_counters, pending, players) in [
        (&[][..], 0, 0, true, vec![alice]),
        (&[1][..], 0, 0, true, vec![alice, bob]),
        (&[1, 1][..], 3, 3, false, vec![alice, bob]),
    ] {
        let mut game = checkpoint.clone();
        let mut dm = CounterChoiceQueue::new(alice, answers);
        dm.expected_player = None;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            game.player(alice).unwrap().counter_count(CounterType::Named("ticket".into())),
            alice_counters
        );
        assert_eq!(
            game.player(bob).unwrap().counter_count(CounterType::Named("ticket".into())),
            bob_counters
        );
        assert_eq!(dm.pending, pending);
        assert_eq!(dm.seen_players, players);
        assert_eq!(outcome.events.len(), if pending { 0 } else { 2 });
        assert!(game.take_pending_trigger_events().is_empty());
        if pending {
            // Replay against the actual restored state, not a fresh fixture.
            let mut resumed = CounterChoiceQueue::new(alice, &[1, 1]);
            resumed.expected_player = None;
            let mut resumed_ctx = ExecutionContext::new(source, alice, &mut resumed);
            let completed = effect.execute(&mut game, &mut resumed_ctx).unwrap();
            assert!(!resumed_ctx.decision_maker.awaiting_choice());
            assert_eq!(
                game.player(alice).unwrap().counter_count(CounterType::Named("ticket".into())),
                3
            );
            assert_eq!(game.player(bob).unwrap().counter_count(CounterType::Named("ticket".into())), 3);
            assert_eq!(resumed.seen_players, vec![alice, bob]);
            assert_eq!(completed.events.len(), 2);
            let markers = completed
                .events
                .iter()
                .map(|event| {
                    event
                        .downcast::<crate::events::MarkersChangedEvent>()
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                markers[0].location,
                crate::marker::MarkerLocation::Player(alice)
            );
            assert_eq!(
                markers[1].location,
                crate::marker::MarkerLocation::Player(bob)
            );
            assert!(markers.iter().all(|event| event.amount == 3));
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

