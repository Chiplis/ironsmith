use super::*;
use ironsmith::decisions::context::BooleanContext;
use ironsmith::static_abilities::StaticAbilityId;
#[derive(Default)]
struct Payment { pay: bool }
impl DecisionMaker for Payment {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.pay }
    fn decide_objects(&mut self, _: &GameState, _: &SelectObjectsContext) -> Vec<ObjectId> {
        panic!("Kheru's returned card is selected at random");
    }
}
fn stack_phase(game: &mut GameState, event: TriggerEvent, dm: &mut Payment) {
    game.queue_trigger_event(Default::default(), event);
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
fn upkeep(game: &mut GameState, dm: &mut Payment) {
    game.turn.active_player = A;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
    game.mark_upkeep_began(A);
    stack_phase(game, TriggerEvent::new(ironsmith::events::phase::BeginningOfUpkeepEvent::new(A), Default::default()), dm);
}
fn end_step(game: &mut GameState, player: PlayerId, dm: &mut Payment) {
    game.turn.active_player = player;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(ironsmith::game_state::Step::End);
    stack_phase(game, TriggerEvent::new(ironsmith::events::phase::BeginningOfEndStepEvent::new(player), Default::default()), dm);
}
fn movement(game: &mut GameState, source: ObjectId, object: ObjectId, zone: Zone, dm: &mut Payment) {
    let mut context = EffectContext::new(source, A, dm);
    let result = execute_effect(game, &Effect::move_to_zone(ChooseSpec::SpecificObject(object), zone, false), &mut context).unwrap();
    drop(context);
    for event in result.events { game.queue_trigger_event(Default::default(), event); }
}
fn has_riders(game: &GameState, object: ObjectId, expected: bool) {
    for ability in [StaticAbilityId::Flying, StaticAbilityId::Trample, StaticAbilityId::Haste] {
        assert_eq!(game.current_has_static_ability_id(object, ability), expected, "{ability:?}");
    }
}
#[test]
fn kheru_keeps_payment_decline_empty_pool_and_returned_identity_separate() {
    for definition in definitions("Kheru Lich Lord") {
        for pay in [false, true] {
            for has_card in [false, true] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let own = has_card.then(|| creature(&mut game, A, Zone::Graveyard, "Own returned creature", "Bear", 2));
                let foreign = creature(&mut game, B, Zone::Graveyard, "Foreign graveyard creature", "Bear", 2);
                let mut dm = Payment { pay };
                let before_mana = game.player(A).unwrap().mana_pool.total();
                let before_random = game.irreversible_random_count();
                upkeep(&mut game, &mut dm);
                assert_eq!(game.stack.len(), 1);
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(game.player(A).unwrap().mana_pool.total(), before_mana - if pay { 3 } else { 0 });
                assert_eq!(game.irreversible_random_count(), before_random + u64::from(pay && has_card));
                assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
                has_riders(&game, source, false);
                if pay && has_card {
                    assert!(game.object(own.unwrap()).is_none());
                    let returned = game.battlefield.iter().find(|id| **id != source).copied().unwrap();
                    has_riders(&game, returned, true);
                    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
                } else {
                    assert_eq!(game.battlefield, vec![source]);
                    assert!(game.effect_store.delayed_triggers.is_empty());
                    if let Some(own) = own { assert_eq!(game.object(own).unwrap().zone, Zone::Graveyard); }
                }
            }
        }
    }
}
#[test]
fn kheru_rider_survives_source_departure_and_uses_ability_controllers_end_step() {
    for definition in definitions("Kheru Lich Lord") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        creature(&mut game, A, Zone::Graveyard, "Returned creature", "Bear", 2);
        let mut dm = Payment { pay: true };
        upkeep(&mut game, &mut dm);
        movement(&mut game, source, source, Zone::Hand, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let returned = game.battlefield[0];
        let stable = game.object(returned).unwrap().stable_id;
        has_riders(&game, returned, true);
        game.set_current_controller(returned, B).unwrap();
        end_step(&mut game, B, &mut dm);
        assert!(game.stack.is_empty(), "the next end step belongs to the resolving ability's controller");
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        end_step(&mut game, A, &mut dm);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].controller, A);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Exile);
        assert!(game.effect_store.delayed_triggers.is_empty());
    }
}
#[test]
fn kheru_leave_replacement_and_delayed_exile_do_not_follow_a_new_incarnation() {
    for definition in definitions("Kheru Lich Lord") {
        for destination in [Zone::Hand, Zone::Graveyard, Zone::Library, Zone::Exile] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            creature(&mut game, A, Zone::Graveyard, "Returned creature", "Bear", 2);
            let unrelated = creature(&mut game, A, Zone::Battlefield, "Unrelated creature", "Bear", 2);
            let mut dm = Payment { pay: true };
            upkeep(&mut game, &mut dm);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let returned = game.battlefield.iter().find(|id| **id != source && **id != unrelated).copied().unwrap();
            let stable = game.object(returned).unwrap().stable_id;
            has_riders(&game, returned, true);
            movement(&mut game, source, source, Zone::Hand, &mut dm);
            movement(&mut game, source, returned, destination, &mut dm);
            let exiled = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "{destination:?}");
            movement(&mut game, source, exiled, Zone::Battlefield, &mut dm);
            let fresh = game.find_object_by_stable_id(stable).unwrap();
            assert_ne!(fresh, returned);
            has_riders(&game, fresh, false);
            end_step(&mut game, A, &mut dm);
            while !game.stack.is_empty() { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
            assert_eq!(game.object(fresh).unwrap().zone, Zone::Battlefield, "stale delayed exile cannot follow a fresh incarnation");
            movement(&mut game, source, fresh, Zone::Hand, &mut dm);
            let fresh = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(fresh).unwrap().zone, Zone::Hand, "stale replacement cannot follow a fresh incarnation");
            assert_eq!(game.object(unrelated).unwrap().zone, Zone::Battlefield);
        }
    }
}
#[test]
fn countering_kherus_delayed_exile_does_not_expire_the_granted_abilities() {
    for definition in definitions("Kheru Lich Lord") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        creature(&mut game, A, Zone::Graveyard, "Returned creature", "Bear", 2);
        let mut dm = Payment { pay: true };
        upkeep(&mut game, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let returned = game.battlefield.iter().find(|id| **id != source).copied().unwrap();
        end_step(&mut game, A, &mut dm);
        assert_eq!(game.stack.len(), 1);
        game.stack.pop(); // Counter the one-shot delayed ability.
        game.turn.turn_number += 1;
        end_step(&mut game, B, &mut dm);
        assert!(game.stack.is_empty());
        has_riders(&game, returned, true);
    }
}
