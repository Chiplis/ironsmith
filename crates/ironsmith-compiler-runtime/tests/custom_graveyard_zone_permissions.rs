//! Actual source providers/consumers, plus real payment completion and entry.
//! Requires source95's complete fixture materialized by the parent frontend.
#[path = "custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::costs::{Cost, CostContext, CostPayer};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use std::collections::VecDeque;
const A: PlayerId = PlayerId(0);
fn actual(name: &str) -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-actions-status-whole-source-complete.json")).unwrap();
    let card = fixture["mechanics"].as_array().unwrap().iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["name"] == name).expect("complete actual source");
    source_support::definitions(card).unwrap_or_else(|error| panic!("{name}: {error}"))
}
fn ordinary(name: &str, text: &str) -> CardDefinition {
    source_support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()[0].clone()
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
    game
}
fn exile_cost(resource: ObjectId) -> Cost {
    Cost::try_effect(Effect::new(ironsmith::effects::ExileEffect::with_spec(
        ChooseSpec::Object(ObjectFilter::specific(resource).in_zone(Zone::Graveyard)).with_count(ironsmith::ChoiceCount::exactly(1)))))
        .expect("public ordinary exile cost")
}
#[test]
fn actual_warden_blocks_graveyard_cast_cost_and_real_movement_then_departure_removes_the_lock() {
    for guard_definition in actual("Warden of Avaz") {
        let mut game = game();
        let spell = ordinary("Ordinary graveyard spell", "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.\nFlashback {0}");
        let resource = game.create_object_from_definition(&spell, A, Zone::Graveyard);
        let stable = game.object(resource).unwrap().stable_id;
        let method = ironsmith::alternative_cast::CastingMethod::Alternative(0);
        assert!(ironsmith::decision::can_cast_spell(&game, A, game.object(resource).unwrap(), &method));
        let guard = game.create_object_from_definition(&guard_definition, A, Zone::Battlefield);
        assert!(!ironsmith::decision::can_cast_spell(&game, A, game.object(resource).unwrap(), &method));
        let cost = exile_cost(resource); let mut dm = SelectFirstDecisionMaker;
        let mut payment = CostContext::new(guard, A, &mut dm);
        assert!(cost.can_pay(&game, &payment).is_err());
        assert!(cost.pay(&mut game, &mut payment).is_err());
        assert_eq!(game.object(resource).unwrap().zone, Zone::Graveyard);
        for destination in [Zone::Hand, Zone::Library, Zone::Battlefield, Zone::Exile, Zone::Stack] {
            assert!(game.move_object_by_effect(resource, destination).is_none(), "cannot bypass prohibition via {destination:?}");
        }
        assert!(game.stack.is_empty());
        game.move_object_by_effect(guard, Zone::Exile).unwrap();
        assert!(ironsmith::decision::can_cast_spell(&game, A, game.object(resource).unwrap(), &method), "current departed provider must not leave stale cast prohibition");
        let mut payment = CostContext::new(resource, A, &mut dm);
        assert!(cost.can_pay(&game, &payment).is_ok()); cost.pay(&mut game, &mut payment).unwrap();
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
    }
}
struct Answers(VecDeque<bool>);
impl DecisionMaker for Answers {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.0.pop_front().unwrap_or(true) }
}
/// A real one-blue-color payment, not a fabricated completion event. Its
/// immutable resource snapshot supplies exactly one action amount.
fn collect_one_color(game: &mut GameState, actor: ObjectId, resource: ObjectId, dm: &mut Answers) -> usize {
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(resource).unwrap(), game);
    let program = Effect::new(ironsmith::effects::NamedActionProgramEffect {
        label: "collect backing".into(), player: PlayerFilter::You,
        completion: ironsmith_core::effect::NamedActionProgramCompletion::AcceptedPayment,
        objects: Some(ChooseSpec::Tagged("paid_backing_resource".into())),
        effects: vec![Effect::new(ironsmith::effects::MayEffect::new(vec![Effect::new(ironsmith::effects::ExileEffect::with_spec(
            ChooseSpec::Object(ObjectFilter::specific(resource).in_zone(Zone::Graveyard)).with_count(ironsmith::ChoiceCount::exactly(1))))]).with_pay_as_cost(true))],
    });
    let mut ctx = EffectContext::new(actor, A, dm);
    ctx.set_tagged_objects("paid_backing_resource", vec![snapshot]);
    let outcome = execute_effect(game, &program, &mut ctx).unwrap();
    let events = outcome.events;
    let completions = events.iter().filter(|event| event.downcast::<ironsmith::events::NamedActionEvent>().is_some()).count();
    for event in events { game.queue_trigger_event(event.provenance(), event); }
    completions
}
fn actor_and_resource(game: &mut GameState) -> (ObjectId, ObjectId) {
    let actor = game.create_object_from_definition(&ordinary("Payment source", "Type: Artifact"), A, Zone::Battlefield);
    let resource = game.create_object_from_definition(&ordinary("Blue contribution", "Mana cost: {U}\nType: Sorcery\nYou gain 1 life."), A, Zone::Graveyard);
    (actor, resource)
}
#[test]
fn actual_withered_graveyard_trigger_resolves_both_optional_branches_and_enters_tapped() {
    for definition in actual("Withered Sentinel") { for pay in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        let (actor, resource) = actor_and_resource(&mut game);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1);
        let mut dm = Answers(VecDeque::from(vec![true, pay]));
        assert_eq!(collect_one_color(&mut game, actor, resource, &mut dm), 1);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(queue.entries.len(), 1, "actual source functions in its graveyard and sees one completed color contribution");
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrival).unwrap().zone, if pay { Zone::Battlefield } else { Zone::Hand });
        if pay { assert!(game.is_tapped(arrival), "the actual source's enters-tapped sibling is retained"); }
    }}
}
#[test]
fn actual_withered_does_not_function_on_battlefield_and_warden_cannot_publish_an_unpaid_completion() {
    for (definition, guard_definition) in actual("Withered Sentinel").into_iter().zip(actual("Warden of Avaz")) {
        for (zone, blocked) in [(Zone::Battlefield, false), (Zone::Graveyard, true)] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, zone);
            let (actor, resource) = actor_and_resource(&mut game);
            if blocked { game.create_object_from_definition(&guard_definition, A, Zone::Battlefield); }
            let mut dm = Answers(VecDeque::from(vec![true]));
            assert_eq!(collect_one_color(&mut game, actor, resource, &mut dm), usize::from(!blocked));
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert!(queue.entries.is_empty());
            assert_eq!(game.object(source).unwrap().zone, zone);
            if blocked { assert_eq!(game.object(resource).unwrap().zone, Zone::Graveyard); }
        }
    }
}
#[test]
fn graveyard_qualification_is_event_time_not_an_intervening_if_on_the_independent_stack_ability() {
    for definition in actual("Withered Sentinel") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        let (actor, resource) = actor_and_resource(&mut game);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1);
        let mut dm = Answers(VecDeque::from(vec![true, true]));
        assert_eq!(collect_one_color(&mut game, actor, resource, &mut dm), 1);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "the captured targetless ability still offers its optional payment");
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile, "a different-zone incarnation is not returned from the graveyard");
    }
}
