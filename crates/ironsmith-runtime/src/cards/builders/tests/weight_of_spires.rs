#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Weight of Spires deals damage to target creature equal to the number of nonbasic lands that creature's controller controls.";

fn durable_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 20))
        .build()
}

fn land(name: &str, basic: bool) -> CardDefinition {
    let mut builder =
        CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Land]);
    if basic {
        builder = builder.supertypes(vec![crate::types::Supertype::Basic]);
    }
    builder.build()
}

fn create_lands(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    prefix: &str,
    nonbasic: usize,
    basic: usize,
) {
    for index in 0..nonbasic {
        game.create_object_from_definition(
            &land(&format!("{prefix} Nonbasic {index}"), false),
            controller,
            Zone::Battlefield,
        );
    }
    for index in 0..basic {
        game.create_object_from_definition(
            &land(&format!("{prefix} Basic {index}"), true),
            controller,
            Zone::Battlefield,
        );
    }
}

fn nested_damage(effect: &Effect) -> Option<crate::effects::DealDamageEffect> {
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some(damage.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_damage(child);
        }
    });
    found
}

#[test]
fn exact_text_and_target_controller_count_are_preserved_end_to_end() {
    let definition = parse_oracle_card_definition("Weight of Spires");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let damage = definition
        .spell_effect
        .as_ref()
        .expect("Weight of Spires should be an instant spell")
        .flattened_default_effects()
        .into_iter()
        .find_map(nested_damage)
        .expect("Weight of Spires should keep its typed damage instruction");
    let Value::Count(counted) = damage.amount.unhinted() else {
        panic!("damage must retain the controller-relative land count: {damage:#?}");
    };
    assert_eq!(
        counted.controller,
        Some(PlayerFilter::ControllerOf(ObjectRef::Target))
    );
    assert_eq!(counted.card_types, [CardType::Land]);
    assert!(counted.excluded_supertypes.contains(&Supertype::Basic));
    assert!(matches!(
        damage.target.base(),
        ChooseSpec::Object(filter) if filter.card_types == [CardType::Creature]
    ));

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    create_lands(&mut game, alice, "Alice", 1, 0);
    create_lands(&mut game, bob, "Bob", 3, 2);

    let target = game.create_object_from_definition(
        &durable_creature("Bob's Target"),
        bob,
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(target)]),
    );

    crate::game_loop::resolve_stack_entry(&mut game).expect("Weight of Spires should resolve");
    assert_eq!(
        game.damage_on(target),
        3,
        "count only the nonbasic lands controlled by the targeted creature's controller"
    );
}

#[test]
fn basic_lands_and_other_players_nonbasic_lands_do_not_enter_the_count() {
    let definition = parse_oracle_card_definition("Weight of Spires");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    create_lands(&mut game, alice, "Alice", 4, 0);
    create_lands(&mut game, bob, "Bob", 0, 3);

    let target = game.create_object_from_definition(
        &durable_creature("Basic-Land Target"),
        bob,
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(target)]),
    );

    crate::game_loop::resolve_stack_entry(&mut game).expect("Weight of Spires should resolve");
    assert_eq!(game.damage_on(target), 0);
}
