//! Native owner scenarios authored without execution during the campaign gate.
use super::*;
use crate::card::CardBuilder;
use crate::decision::DecisionMaker;
use crate::decisions::context::{BooleanContext, NumberContext};
use crate::effect::{Effect, OutcomeValue};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::object::CounterType;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::ObjectFilter;
use crate::types::CardType;
use crate::zone::Zone;
const A: PlayerId = PlayerId::from_index(0);
const KIND: CounterType = CounterType::PlusOnePlusOne;
fn object(game: &mut GameState, count: u32) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Transfer witness").card_types(vec![CardType::Artifact]).build();
    let id = game.create_object_from_card(&card, A, Zone::Battlefield);
    if count > 0 { game.object_mut(id).unwrap().counters.insert(KIND, count); }
    id
}
fn fixture() -> (GameState, ObjectId, ObjectId, ObjectId, MoveCountersEffect) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let destination = object(&mut game, 0); let first = object(&mut game, 2); let second = object(&mut game, 3);
    let mut filter = ObjectFilter::permanent(); filter.other = true;
    let effect = MoveCountersEffect::all(KIND, ChooseSpec::All(filter), ChooseSpec::Source);
    (game, destination, first, second, effect)
}
fn removal_replacement(game: &mut GameState, source: ObjectId, donor: ObjectId, action: ReplacementAction)
    -> crate::replacement::ReplacementEffectId {
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
        crate::events::counters::matchers::WouldRemoveCountersMatcher::new(ObjectFilter::specific(donor), Some(KIND)), action))
}
fn run(game: &mut GameState, source: ObjectId, effect: &MoveCountersEffect, dm: &mut dyn DecisionMaker)
    -> Result<EffectOutcome, ExecutionError> {
    crate::effects::execute_effect(game, &Effect::new(effect.clone()), &mut ExecutionContext::new(source, A, dm))
}

#[test]
fn every_donor_contributes_and_the_destination_receives_one_combined_original() {
    let (mut game, destination, first, second, effect) = fixture();
    game.object_mut(first).unwrap().counters.insert(CounterType::Charge, 7);
    let result = run(&mut game, destination, &effect, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.counter_count(first, KIND), 0); assert_eq!(game.counter_count(second, KIND), 0);
    assert_eq!(game.counter_count(destination, KIND), 5); assert_eq!(game.counter_count(first, CounterType::Charge), 7);
    assert_eq!(result.as_count(), Some(5));
    assert_eq!(result.events_of_type::<crate::events::MarkersChangedEvent>().filter(|event| event.is_added()).count(), 1);
    assert_eq!(result.events_of_type::<crate::events::MarkersChangedEvent>().filter(|event| event.is_removed()).count(), 2);
}

struct Numbers { answers: std::collections::VecDeque<u32>, bounds: Vec<(u32, u32)>, stop_after: Option<usize>, pending: bool }
impl DecisionMaker for Numbers {
    fn decide_number(&mut self, _: &GameState, context: &NumberContext) -> u32 {
        self.bounds.push((context.min, context.max));
        if self.stop_after == Some(self.bounds.len()) { self.pending = true; return context.min; }
        let answer = self.answers.pop_front().unwrap(); assert!(answer >= context.min && answer <= context.max); answer
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
#[test]
fn independent_zero_and_partial_allocations_are_chosen_before_any_mutation() {
    let (mut game, destination, first, second, mut effect) = fixture();
    effect.count = ironsmith_core::effect::CounterMoveAmount::AnyNumber;
    let saved = game.clone();
    let mut pending = Numbers { answers: [1].into(), bounds: vec![], stop_after: Some(2), pending: false };
    let result = run(&mut game, destination, &effect, &mut pending).unwrap();
    assert!(pending.pending); assert!(result.events.is_empty());
    assert_eq!(game.counter_count(first, KIND), 2); assert_eq!(game.counter_count(second, KIND), 3);
    assert_eq!(game.counter_count(destination, KIND), 0);
    for mut branch in [game, saved] {
        let mut choices = Numbers { answers: [0, 2].into(), bounds: vec![], stop_after: None, pending: false };
        let result = run(&mut branch, destination, &effect, &mut choices).unwrap();
        assert_eq!(choices.bounds, vec![(0, 2), (0, 3)]);
        assert_eq!(branch.counter_count(first, KIND), 2); assert_eq!(branch.counter_count(second, KIND), 1);
        assert_eq!(branch.counter_count(destination, KIND), 2); assert_eq!(result.count_or_zero(), 2);
    }
}

#[test]
fn removal_added_program_runs_after_all_donors_and_the_placement_original() {
    let (mut game, destination, first, second, effect) = fixture();
    removal_replacement(&mut game, destination, first, ReplacementAction::Additionally(vec![
        Effect::remove_counters(KIND, 99, ChooseSpec::SpecificObject(second)),
        Effect::put_counters(KIND, 1, ChooseSpec::SpecificObject(destination)),
    ]));
    let out = run(&mut game, destination, &effect, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.counter_count(first, KIND), 0); assert_eq!(game.counter_count(second, KIND), 0);
    assert_eq!(game.counter_count(destination, KIND), 6, "the second donor contributes before the appended removal");
    assert_eq!(out.count_or_zero(), 5, "appended placement does not inflate the movement receipt");
}

struct CompletionChoice { destination: ObjectId, first: ObjectId, second: ObjectId, pause: bool, pending: bool }
impl DecisionMaker for CompletionChoice {
    fn decide_boolean(&mut self, game: &GameState, _: &BooleanContext) -> bool {
        assert_eq!(game.counter_count(self.first, KIND), 0);
        assert_eq!(game.counter_count(self.second, KIND), 0);
        assert_eq!(game.counter_count(self.destination, KIND), 5, "completion sees every original");
        self.pending = self.pause; !self.pause
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
#[test]
fn pending_completion_restores_all_originals_and_one_shots_before_native_retry() {
    let (mut game, destination, first, second, effect) = fixture();
    let shield = removal_replacement(&mut game, destination, first,
        ReplacementAction::Additionally(vec![Effect::may(vec![Effect::gain_life(2)])]));
    let mut dm = CompletionChoice { destination, first, second, pause: true, pending: false };
    let out = run(&mut game, destination, &effect, &mut dm).unwrap();
    assert!(dm.pending); assert_eq!(out.value, OutcomeValue::Count(0)); assert!(out.events.is_empty());
    assert_eq!(game.counter_count(first, KIND), 2); assert_eq!(game.counter_count(second, KIND), 3);
    assert_eq!(game.counter_count(destination, KIND), 0); assert_eq!(game.player(A).unwrap().life, 20);
    assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
    let mut recovered = game.clone(); dm.pause = false; dm.pending = false;
    let out = run(&mut recovered, destination, &effect, &mut dm).unwrap();
    assert_eq!(out.count_or_zero(), 5); assert_eq!(recovered.player(A).unwrap().life, 22);
    assert_eq!(recovered.counter_count(destination, KIND), 5);
    assert!(recovered.effect_store.replacement_effects.get_effect(shield).is_none());
    assert_eq!(out.events_of_type::<crate::events::LifeGainEvent>().count(), 1);
}

#[derive(Debug, Clone)] struct Fail;
impl EffectExecutor for Fail {
    fn execute(&self, _: &mut GameState, _: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        Err(ExecutionError::InternalError("counter completion failed".into()))
    }
}
#[test]
fn failed_completion_restores_donors_destination_and_replacement_consumption() {
    let (mut game, destination, first, second, effect) = fixture();
    let shield = removal_replacement(&mut game, destination, first,
        ReplacementAction::Additionally(vec![Effect::gain_life(2), Effect::new(Fail)]));
    assert!(run(&mut game, destination, &effect, &mut crate::decision::SelectFirstDecisionMaker).is_err());
    assert_eq!(game.counter_count(first, KIND), 2); assert_eq!(game.counter_count(second, KIND), 3);
    assert_eq!(game.counter_count(destination, KIND), 0); assert_eq!(game.player(A).unwrap().life, 20);
    assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
}

#[test]
fn original_removal_receipt_excludes_prevention_and_placement_multiplication() {
    let (mut game, destination, first, second, effect) = fixture();
    removal_replacement(&mut game, destination, first, ReplacementAction::Prevent);
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(destination, A,
        crate::events::counters::matchers::WouldPutCountersMatcher::any(),
        ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
    let out = run(&mut game, destination, &effect, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.counter_count(first, KIND), 2); assert_eq!(game.counter_count(second, KIND), 0);
    assert_eq!(game.counter_count(destination, KIND), 10, "replacement of removal does not rewrite the separate put budget");
    assert_eq!(out.count_or_zero(), 3, "only three original counters were actually removed");
    assert_eq!(out.instruction_result().count_or_zero(), 3,
        "the prepared aggregate retains the transfer's exact primary receipt");
    assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>()
        .filter(|event| event.is_added()).count(), 1,
        "retained child routing does not duplicate physical placement history");
}

#[test]
fn combined_count_overflow_rolls_back_without_capping_a_donor() {
    let (mut game, destination, first, second, effect) = fixture();
    game.object_mut(first).unwrap().counters.insert(KIND, u32::MAX);
    assert!(run(&mut game, destination, &effect, &mut crate::decision::SelectFirstDecisionMaker).is_err());
    assert_eq!(game.counter_count(first, KIND), u32::MAX); assert_eq!(game.counter_count(second, KIND), 3);
    assert_eq!(game.counter_count(destination, KIND), 0);
}
