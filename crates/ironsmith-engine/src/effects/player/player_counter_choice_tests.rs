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
        (&[1][..], 4, true, 2),
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
        (&[1][..], 4, 2, true, 2),
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
        assert_eq!(outcome.events.len(), answers.len());
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
        (&[1][..], 3, 0, true, vec![alice, bob]),
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
    }
}
