//! Complete source token-rule siblings remain independent executable abilities.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith::decision::SelectFirstDecisionMaker;

#[test]
fn actual_honey_pot_retains_breach_and_its_independent_blocking_restriction() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json"
    )).unwrap();
    let source = fixture["mechanics"].as_array().unwrap().iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|source| source["source_card_key"] == "Honey Pot_BTB").unwrap();
    for definition in source_support::definitions(source).unwrap() {
        let actor = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id = game.create_object_from_definition(&definition, actor, Zone::Hand);
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(id, actor, vec![
            ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(id), Zone::Battlefield, false),
        ]));
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut decisions = SelectFirstDecisionMaker;
        for _ in 0..16 {
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut decisions).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions).unwrap();
            if game.stack.is_empty() { break; }
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut decisions).unwrap();
        }
        assert!(game.stack.is_empty());
        let tokens: Vec<_> = game.battlefield.iter().copied().filter(|id|
            game.object(*id).is_some_and(|object| object.kind == ironsmith::object::ObjectKind::Token)
        ).collect();
        assert_eq!(tokens.len(), 2);
        for token in tokens {
            assert_eq!(game.calculated_power(token), Some(1));
            assert_eq!(game.calculated_toughness(token), Some(1));
            assert!(game.current_has_static_ability_id(token, ironsmith::static_abilities::StaticAbilityId::CantBlock));
            let abilities = game.current_characteristics(token).unwrap().abilities;
            assert_eq!(abilities.iter().filter(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_))).count(), 1,
                "one complete combat trigger, never a lost or duplicated sibling");
        }
    }
}
