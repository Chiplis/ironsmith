//! Source-authored regressions for retained movement boundaries. Not executed.
use super::*;
use crate::decision::DecisionMaker;
use crate::decisions::context::{BooleanContext, SelectObjectsContext, ViewCardsContext};
use crate::effect::{Effect, EffectId, EffectPredicate, Value};
use crate::events::EnterBattlefieldEvent;
use crate::filter::ObjectFilterExt as _;
use crate::ids::{CardId, ObjectId, StableId};
use crate::object::CounterType;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::PlayerFilter;

fn card(game: &mut GameState, name: &str, zone: Zone) -> ObjectId {
    let definition = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&definition, PlayerId(0), zone)
}

fn counter_replacement(game: &mut GameState, source: ObjectId, name: &str, effects: Vec<Effect>) {
    game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
        source, PlayerId(0),
        crate::events::counters::matchers::WouldPutCountersMatcher::new(
            ObjectFilter::default().named(name).in_zone(Zone::Exile),
            Some(CounterType::PlusOnePlusOne),
        ),
        ReplacementAction::Instead(effects),
    ));
}

#[derive(Debug, Clone)]
struct AddLaterOriginalCounter(StableId);
impl EffectExecutor for AddLaterOriginalCounter {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        let entrant = game.find_object_by_stable_id(self.0).expect("original entrant");
        assert!(!game.retained_action_observations().any(|event| {
            event.downcast::<EnterBattlefieldEvent>().is_some_and(|entry|
                entry.object == entrant && entry.completed_snapshot.is_some())
        }), "the earlier counter Instead prefix must not publish an incomplete entry");
        crate::effects::execute_effect(game, &Effect::put_counters(
            CounterType::PlusOnePlusOne, 1, ChooseSpec::SpecificObject(entrant),
        ), ctx)
    }
}

#[test]
fn arrival_counter_instead_prefix_waits_for_later_counter_original_before_entry_observation() {
    for dispatched in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = card(&mut game, "Counter boundary source", Zone::Battlefield);
        let first = card(&mut game, "First counter original", Zone::Hand);
        let second = card(&mut game, "Later counter original", Zone::Hand);
        let entrant = card(&mut game, "Observed entrant", Zone::Hand);
        let entrant_stable = game.object(entrant).unwrap().stable_id;
        counter_replacement(&mut game, source, "First counter original", vec![Effect::gain_life(1)]);
        counter_replacement(&mut game, source, "Later counter original", vec![Effect::new(AddLaterOriginalCounter(entrant_stable))]);
        let snapshots = [first, second, entrant].map(|id| ObjectSnapshot::from_object(game.object(id).unwrap(), &game));
        let mut effect = MoveToZoneEffect::new(ChooseSpec::Tagged("moving".into()), Zone::Exile, false)
            .with_entry_counter(ironsmith_core::BattlefieldEntryCounterSpec::new(
                CounterType::PlusOnePlusOne, Value::Fixed(1), ironsmith_core::BattlefieldEntryCounterSurface::Inline,
            ));
        effect.tagged_destinations = vec![("exiled".into(), Zone::Exile), ("entered".into(), Zone::Battlefield)];
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
        ctx.set_tagged_objects("moving", snapshots.to_vec());
        ctx.set_tagged_objects("exiled", snapshots[..2].to_vec());
        ctx.set_tagged_objects("entered", vec![snapshots[2].clone()]);
        if dispatched { crate::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap(); }
        else { effect.execute(&mut game, &mut ctx).unwrap(); }
        let entrant = game.find_object_by_stable_id(entrant_stable).unwrap();
        assert_eq!(game.counter_count(entrant, CounterType::PlusOnePlusOne), 2);
        let completed = game.retained_action_observations().find_map(|event| {
            event.downcast::<EnterBattlefieldEvent>().filter(|entry| entry.object == entrant)
                .and_then(|entry| entry.completed_snapshot.clone())
        }).expect("completed original entry receipt");
        assert_eq!(completed.counters.get(&CounterType::PlusOnePlusOne), Some(&2));
        assert!(!game.has_open_simultaneous_action());
    }
}

#[derive(Debug, Clone)]
struct FailWithPendingChoice;
impl EffectExecutor for FailWithPendingChoice {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        game.player_mut(ctx.controller).unwrap().life += 7;
        ctx.decision_maker.decide_boolean(game, &BooleanContext::new(
            ctx.controller, Some(ctx.source), "Pending before typed failure",
        ));
        Err(ExecutionError::IncompleteEvidence("typed failure after pending decision".into()))
    }
}
#[derive(Default)]
struct PauseBoolean { pending: bool }
impl DecisionMaker for PauseBoolean {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.pending = true;
        false
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}

#[test]
fn pending_arrival_counter_failure_remains_an_error_and_rolls_back_original_movement() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let source = card(&mut game, "Pending failure source", Zone::Battlefield);
    let selected = card(&mut game, "Failing counter original", Zone::Hand);
    counter_replacement(&mut game, source, "Failing counter original", vec![Effect::new(FailWithPendingChoice)]);
    let next_id = game.next_object_id_counter();
    game.take_pending_trigger_events();
    let effect = MoveToZoneEffect::new(ChooseSpec::SpecificObject(selected), Zone::Exile, false)
        .with_entry_counter(ironsmith_core::BattlefieldEntryCounterSpec::new(
            CounterType::PlusOnePlusOne, Value::Fixed(1), ironsmith_core::BattlefieldEntryCounterSurface::Inline,
        ));
    let mut dm = PauseBoolean::default();
    let mut ctx = ExecutionContext::new(source, PlayerId(0), &mut dm);
    let result = effect.execute(&mut game, &mut ctx);
    assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(ref message))
        if message == "typed failure after pending decision"));
    assert_eq!(game.object(selected).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(PlayerId(0)).unwrap().life, 20);
    assert_eq!(game.next_object_id_counter(), next_id);
    assert!(game.exile.is_empty());
    assert!(game.take_pending_trigger_events().is_empty());
    assert!(!game.has_open_simultaneous_action());
}

#[derive(Default)]
struct PrivateInputs { openings: usize, views: usize, pause: bool, pending: bool }
impl DecisionMaker for PrivateInputs {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { false }
    fn decide_objects(&mut self, _: &GameState, _: &SelectObjectsContext) -> Vec<ObjectId> {
        self.openings += 1;
        self.pending = self.pause;
        Vec::new()
    }
    fn view_cards(&mut self, _: &GameState, _: PlayerId, _: &[ObjectId], _: &ViewCardsContext) { self.views += 1; }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn private_hand_spec() -> ChooseSpec {
    ChooseSpec::All(ObjectFilter::creature().in_zone(Zone::Hand).owned_by(PlayerFilter::You))
}

#[test]
fn untaken_optional_and_conditional_movement_never_requests_private_hand_inputs() {
    for branch in 0..4 {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = card(&mut game, "Private input source", Zone::Battlefield);
        let private = game.create_hidden_card_placeholder(PlayerId(0), Zone::Hand, 0, "private-unreached".into());
        let movement = Effect::new(MoveToZoneEffect::new(private_hand_spec(), Zone::Exile, false));
        let program = match branch {
            0 => Effect::may(vec![movement]),
            1 => Effect::new(crate::effects::IfEffect::if_then(EffectId(911), EffectPredicate::Happened, vec![movement])),
            2 => Effect::new(crate::effects::SequenceEffect::new(vec![Effect::gain_life(1), Effect::may(vec![movement])])),
            _ => Effect::with_id(912, Effect::may(vec![movement])),
        };
        let mut dm = PrivateInputs::default();
        let mut ctx = ExecutionContext::new(source, PlayerId(0), &mut dm);
        ctx.store_outcome(EffectId(911), EffectOutcome::count(0));
        crate::effects::execute_effect(&mut game, &program, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!((dm.openings, dm.views), (0, 0));
        assert!(game.is_hidden_card_placeholder(private));
        assert!(!game.is_publicly_revealed_hidden_card(private));
        assert_eq!(game.object(private).unwrap().zone, Zone::Hand);
        assert!(game.exile.is_empty());
    }
}

#[test]
fn reached_native_movement_settles_its_own_all_matching_hand_input_before_any_move() {
    for owner in 0..5 {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = card(&mut game, "Reached movement source", Zone::Battlefield);
        let private = game.create_hidden_card_placeholder(PlayerId(0), Zone::Hand, 0, "private-reached".into());
        let spec = private_hand_spec();
        let effect = match owner {
            0 => Effect::new(MoveToZoneEffect::new(spec, Zone::Exile, false)),
            1 => Effect::new(crate::effects::PutOntoBattlefieldEffect::you_control(spec, false)),
            2 => Effect::new(crate::effects::ShuffleObjectsIntoLibraryEffect::new(spec, PlayerFilter::You)),
            3 => Effect::new(crate::effects::ExileEffect::with_spec(spec)),
            _ => Effect::new(crate::effects::ExileUntilEffect::source_leaves(spec)),
        };
        let mut dm = PrivateInputs { pause: true, ..Default::default() };
        let mut ctx = ExecutionContext::new(source, PlayerId(0), &mut dm);
        crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        drop(ctx);
        assert_eq!((dm.openings, dm.views), (1, 0));
        assert_eq!(game.object(private).unwrap().zone, Zone::Hand);
        assert!(!game.is_publicly_revealed_hidden_card(private));
        assert!(game.is_hidden_card_placeholder(private));
    }
}

#[test]
fn every_sacrifice_adapter_keeps_typed_failure_when_its_addition_also_has_pending_input() {
    for adapter in 0..3 {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = card(&mut game, "Sacrifice failure source", Zone::Battlefield);
        let selected = card(&mut game, "Sacrifice failure original", Zone::Battlefield);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, PlayerId(0),
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(selected), Some(Zone::Battlefield), Some(Zone::Graveyard),
            ),
            ReplacementAction::Additionally(vec![Effect::new(FailWithPendingChoice)]),
        ));
        let filter = ObjectFilter::creature().named("Sacrifice failure original");
        let effect = match adapter {
            0 => Effect::new(crate::effects::SacrificeEffect::you(filter, 1)),
            1 => Effect::new(crate::effects::EachPlayerSacrificesEffect::new(filter, 1, PlayerFilter::You)),
            _ => Effect::new(crate::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(selected))),
        };
        game.take_pending_trigger_events();
        let next_id = game.next_object_id_counter();
        let snapshot = ObjectSnapshot::from_object(game.object(selected).unwrap(), &game);
        let mut dm = PauseBoolean::default();
        let mut ctx = ExecutionContext::new(source, PlayerId(0), &mut dm);
        ctx.set_tagged_objects("unchanged", vec![snapshot]);
        let result = effect.0.execute(&mut game, &mut ctx);
        assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(ref message))
            if message == "typed failure after pending decision"), "adapter {adapter}: {result:?}");
        assert_eq!(game.object(selected).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(PlayerId(0)).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), next_id);
        assert_eq!(ctx.get_tagged_all("unchanged").unwrap()[0].object_id, selected);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert!(game.player(PlayerId(0)).unwrap().graveyard.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(!game.has_open_simultaneous_action());
    }
}

#[derive(Debug, Clone)]
struct ObserveMillSibling { sibling: StableId, zone: Zone }
impl EffectExecutor for ObserveMillSibling {
    fn execute(&self, game: &mut GameState, _: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        let object = game.find_object_by_stable_id(self.sibling).expect("retained sibling card");
        assert_eq!(game.object(object).unwrap().zone, self.zone,
            "a replacement prefix precedes sibling originals; its draw suffix follows them");
        Ok(EffectOutcome::resolved())
    }
}

#[test]
fn mill_keeps_replacement_draw_and_suffix_in_both_ordinary_and_simultaneous_completion() {
    for simultaneous in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = card(&mut game, "Mill continuation source", Zone::Battlefield);
        let replaced = card(&mut game, "Replaced mill original", Zone::Library);
        let replaced_stable = game.object(replaced).unwrap().stable_id;
        let definition = crate::card::CardBuilder::new(CardId::new(), "Other mill original")
            .card_types(vec![CardType::Creature]).build();
        let sibling = game.create_object_from_card(&definition, PlayerId(1), Zone::Library);
        let sibling_stable = game.object(sibling).unwrap().stable_id;
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, PlayerId(0),
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(replaced), Some(Zone::Library), Some(Zone::Graveyard),
            ),
            ReplacementAction::Instead(vec![
                Effect::new(ObserveMillSibling { sibling: sibling_stable, zone: Zone::Library }),
                Effect::gain_life(2),
                Effect::new(crate::effects::DrawCardsEffect::you(1)),
                Effect::new(ObserveMillSibling { sibling: sibling_stable,
                    zone: if simultaneous { Zone::Graveyard } else { Zone::Library } }),
                Effect::gain_life(3),
            ]),
        ));
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
        let outputs = if simultaneous {
            crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![
                Effect::new(crate::effects::MillEffect::new(1, PlayerFilter::IteratedPlayer)),
            ]).execute_with_outputs(&mut game, &mut ctx).unwrap()
        } else {
            crate::effects::MillEffect::new(1, PlayerFilter::You)
                .execute_with_outputs(&mut game, &mut ctx).unwrap()
        };
        let drawn = game.find_object_by_stable_id(replaced_stable).unwrap();
        let sibling = game.find_object_by_stable_id(sibling_stable).unwrap();
        assert_eq!(game.object(drawn).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(sibling).unwrap().zone,
            if simultaneous { Zone::Graveyard } else { Zone::Library });
        assert_eq!(game.player(PlayerId(0)).unwrap().life, 25,
            "both retained non-draw instructions execute exactly once");
        let draws = outputs.outcome.events.iter().filter_map(|event| event.downcast::<crate::events::CardsDrawnEvent>())
            .filter(|draw| draw.player == PlayerId(0)).collect::<Vec<_>>();
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].cards, vec![drawn]);
        assert!(!game.has_open_simultaneous_action());
    }
}
