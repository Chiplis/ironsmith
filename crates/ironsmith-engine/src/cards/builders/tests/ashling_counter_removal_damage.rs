#![cfg(ironsmith_runtime_parser_tests)]

use super::*;

const ORACLE: &str = "{1}{R}: Put a +1/+1 counter on Ashling. If this is the third time this ability has resolved this turn, remove all +1/+1 counters from Ashling, and it deals that much damage to each creature and each player.";

fn definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Ashling the Pilgrim")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(10, 10))
        .parse_text(ORACLE)
        .expect("typed counter-removal fanout should parse")
}

#[test]
fn parsed_ashling_binds_both_damage_arms_to_the_removal_and_renders_exactly() {
    let definition = definition();
    let debug = format!("{:#?}", definition.abilities);
    assert_eq!(
        debug.matches("amount: PriorEffectMetric").count(),
        2,
        "{debug}"
    );
    assert_eq!(debug.matches("Removed").count(), 2, "{debug}");
    assert!(
        !debug.contains("amount: EffectValue"),
        "damage must not bind to an intervening damage producer: {debug}"
    );
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![ORACLE.to_string()]
    );
}

#[test]
fn parsed_ashling_third_resolution_uses_one_removed_count_for_all_recipients() {
    let definition = definition();
    let AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
        panic!("expected activated ability: {:#?}", definition.abilities);
    };
    let program = activated.effects.clone();
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other_definition = CardDefinitionBuilder::new(CardId::new(), "Other Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(10, 10))
        .build();
    let other = game.create_object_from_definition(&other_definition, bob, Zone::Battlefield);

    for _ in 0..3 {
        game.push_to_stack(
            crate::game_state::StackEntry::ability(source, alice, program.clone())
                .with_ability_index(0),
        );
        crate::game_loop::resolve_stack_entry(&mut game).expect("ability should resolve");
    }

    assert_eq!(
        game.counter_count(source, crate::object::CounterType::PlusOnePlusOne),
        0
    );
    assert_eq!(game.player(alice).expect("alice").life, 17);
    assert_eq!(game.player(bob).expect("bob").life, 17);
    assert_eq!(game.damage_on(source), 3);
    assert_eq!(game.damage_on(other), 3);
}
