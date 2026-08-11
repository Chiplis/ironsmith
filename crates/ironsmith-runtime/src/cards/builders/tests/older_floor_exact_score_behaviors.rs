#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn vanilla_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn vanilla_land(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

fn resolve_untargeted_spell(
    game: &mut crate::GameState,
    definition: &CardDefinition,
    controller: PlayerId,
) {
    let source = game.create_object_from_definition(definition, controller, Zone::Stack);
    let program = definition
        .spell_effect
        .as_ref()
        .expect("the parsed sorcery should have a resolution program");
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, controller, &mut decisions);
    crate::game_loop::execute_resolution_program(
        game,
        &mut context,
        controller,
        source,
        program,
        None,
        &[],
    )
    .expect("the parsed sorcery should resolve");
}

fn owned_zone_count(game: &crate::GameState, player: PlayerId, zone: Zone) -> usize {
    game.objects_in_zone(zone)
        .into_iter()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.owner == player)
        })
        .count()
}

#[test]
fn minions_murmurs_counts_only_its_controllers_creatures_for_both_results() {
    let definition = parse_oracle_card_definition("Minions' Murmurs");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for name in ["Alice Creature A", "Alice Creature B"] {
        game.create_object_from_definition(&vanilla_creature(name), alice, Zone::Battlefield);
    }
    game.create_object_from_definition(&vanilla_creature("Bob Creature"), bob, Zone::Battlefield);
    for name in ["Draw A", "Draw B", "Undrawn Decoy"] {
        game.create_object_from_definition(&vanilla_land(name), alice, Zone::Library);
    }

    resolve_untargeted_spell(&mut game, &definition, alice);

    assert_eq!(
        owned_zone_count(&game, alice, Zone::Hand),
        2,
        "only Alice's two creatures should set the draw count"
    );
    assert_eq!(
        game.life_total(alice),
        18,
        "the same X value should make Alice lose exactly two life"
    );
    assert_eq!(game.life_total(bob), 20, "the opponent is unaffected");
}

#[test]
fn moon_vigil_adherents_counts_controlled_creatures_and_own_graveyard_creature_cards() {
    let mut definition = parse_oracle_card_definition("Moon-Vigil Adherents");
    // The compact oracle-text fixture intentionally carries no printed P/T
    // metadata. Supply Moon-Vigil Adherents' printed 0/0 so this scenario
    // exercises the continuous count rather than an incomplete fixture.
    definition.card.power_toughness = Some(PowerToughness::fixed(0, 0));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.create_object_from_definition(
        &vanilla_creature("Alice Battlefield Creature"),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &vanilla_creature("Alice Graveyard Creature"),
        alice,
        Zone::Graveyard,
    );
    game.create_object_from_definition(
        &vanilla_creature("Bob Battlefield Creature"),
        bob,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &vanilla_creature("Bob Graveyard Creature"),
        bob,
        Zone::Graveyard,
    );
    game.create_object_from_definition(
        &vanilla_land("Alice Graveyard Land"),
        alice,
        Zone::Graveyard,
    );

    assert_eq!(
        game.object(source).and_then(crate::object::Object::power),
        Some(0),
        "the parsed printed 0/0 must survive object materialization"
    );
    game.refresh_continuous_state();
    assert_eq!(game.calculated_power(source), Some(3));
    assert_eq!(game.calculated_toughness(source), Some(3));
}

#[test]
fn multani_counts_controlled_lands_and_own_graveyard_land_cards() {
    let mut definition = parse_oracle_card_definition("Multani, Yavimaya's Avatar");
    // The compact oracle-text fixture intentionally carries no printed P/T
    // metadata. A zero numeric baseline is the executable value of Multani's
    // printed */* before its characteristic count is applied.
    definition.card.power_toughness = Some(PowerToughness::fixed(0, 0));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for name in ["Alice Land A", "Alice Land B"] {
        game.create_object_from_definition(&vanilla_land(name), alice, Zone::Battlefield);
    }
    game.create_object_from_definition(
        &vanilla_land("Alice Graveyard Land"),
        alice,
        Zone::Graveyard,
    );
    game.create_object_from_definition(
        &vanilla_land("Bob Battlefield Land"),
        bob,
        Zone::Battlefield,
    );
    game.create_object_from_definition(&vanilla_land("Bob Graveyard Land"), bob, Zone::Graveyard);
    game.create_object_from_definition(
        &vanilla_creature("Alice Graveyard Creature"),
        alice,
        Zone::Graveyard,
    );

    assert_eq!(
        game.object(source).and_then(crate::object::Object::power),
        Some(0),
        "the explicit fixture baseline must survive object materialization"
    );
    game.refresh_continuous_state();
    assert_eq!(game.calculated_power(source), Some(3));
    assert_eq!(game.calculated_toughness(source), Some(3));
}

#[test]
fn venerated_teacher_puts_two_level_counters_on_each_controlled_level_up_creature_only() {
    let definition = parse_oracle_card_definition("Venerated Teacher");
    let leveler = parse_oracle_card_definition("Student of Warfare");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let first = game.create_object_from_definition(&leveler, alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&leveler, alice, Zone::Battlefield);
    let enemy = game.create_object_from_definition(&leveler, bob, Zone::Battlefield);
    let ordinary = game.create_object_from_definition(
        &vanilla_creature("Ordinary Creature"),
        alice,
        Zone::Battlefield,
    );
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Venerated Teacher should have an enters trigger");
    let mut context = crate::effects::ExecutionContext::new_default(source, alice);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut context)
            .expect("Venerated Teacher's enters trigger should resolve");
    }

    for controlled_leveler in [first, second] {
        assert_eq!(
            game.counter_count(controlled_leveler, crate::object::CounterType::Level),
            2
        );
    }
    assert_eq!(
        game.counter_count(enemy, crate::object::CounterType::Level),
        0,
        "an opponent's level-up creature is excluded"
    );
    assert_eq!(
        game.counter_count(ordinary, crate::object::CounterType::Level),
        0,
        "a controlled creature without level up is excluded"
    );
}
