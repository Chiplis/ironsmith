#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn find_nested<T: Clone + 'static>(effect: &crate::effect::Effect) -> Option<T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested::<T>(child);
        }
    });
    found
}

fn permanent(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn storm_of_forms_uses_distinct_counter_types_and_keeps_oracle_line_order() {
    let definition = parse_oracle_card_definition("Storm of Forms");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Storm of Forms"]
    );
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Storm of Forms should trigger when it is cast");
    let copy = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| find_nested::<crate::effects::CopySpellEffect>(effect))
        .expect("cast trigger should copy the source spell");
    assert!(copy.target_reference_pronoun);
    let Value::DistinctCounterTypesAmong(filter) = &copy.count else {
        panic!("copy count must be distinct counter types: {copy:#?}");
    };
    assert_eq!(
        filter,
        &ObjectFilter::permanent_card()
            .in_zone(Zone::Battlefield)
            .you_control()
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let first = game.create_object_from_definition(&permanent("First"), alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&permanent("Second"), alice, Zone::Battlefield);
    let opposing =
        game.create_object_from_definition(&permanent("Opposing"), bob, Zone::Battlefield);
    game.object_mut(first)
        .expect("first permanent")
        .add_counters(CounterType::PlusOnePlusOne, 2);
    game.object_mut(second)
        .expect("second permanent")
        .add_counters(CounterType::PlusOnePlusOne, 1);
    game.object_mut(second)
        .expect("second permanent")
        .add_counters(CounterType::Shield, 1);
    game.object_mut(opposing)
        .expect("opposing permanent")
        .add_counters(CounterType::Loyalty, 4);

    let ctx = crate::effects::ExecutionContext::new_default(source, alice);
    assert_eq!(
        crate::effects::resolve_value(&game, &copy.count, &ctx),
        Ok(2),
        "duplicate +1/+1 counters count once, Shield counts once, and the opponent's Loyalty counter is excluded"
    );
}
