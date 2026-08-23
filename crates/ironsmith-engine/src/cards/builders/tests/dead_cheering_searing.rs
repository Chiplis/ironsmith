#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;
use crate::decision::{AutoPassDecisionMaker, SelectFirstDecisionMaker};

fn triggered_ability(definition: &CardDefinition) -> &TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability")
}

fn zone_count_owned_by(game: &crate::GameState, zone: Zone, owner: PlayerId) -> usize {
    game.objects_in_zone(zone)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| object.owner == owner)
        .count()
}

#[test]
fn dead_mans_chest_keeps_dying_creature_power_owner_and_exiled_permission() {
    let definition = parse_oracle_card_definition("Dead Man's Chest");
    let oracle = &oracle_text_by_name()["Dead Man's Chest"];
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle.as_str(),
        "{:#?}",
        triggered_ability(&definition).effects,
    );

    let triggered = triggered_ability(&definition);
    let debug = format!("{:#?}", triggered.effects);
    assert!(
        debug.contains("ExileTopOfLibraryEffect")
            && debug.contains("PowerOf")
            && debug.contains("triggering")
            && debug.contains("OwnerOf")
            && debug.contains("ForAsLongAsExiled")
            && debug.contains("AnyType"),
        "the death snapshot must supply both power and library owner while the exact exiled set supplies the cast permission: {debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Chest Victim")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 4))
        .build();
    let victim = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    for name in ["Bob Top A", "Bob Top B", "Bob Remainder"] {
        let card = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_definition(&card, bob, Zone::Library);
    }
    let alice_card = CardDefinitionBuilder::new(CardId::new(), "Alice Library Sentinel")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_definition(&alice_card, alice, Zone::Library);

    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(victim).expect("victim exists"),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            victim,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut decisions = AutoPassDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_triggering_event(event);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Dead Man's Chest trigger resolves");

    assert_eq!(zone_count_owned_by(&game, Zone::Exile, bob), 2);
    assert_eq!(zone_count_owned_by(&game, Zone::Library, bob), 1);
    assert_eq!(zone_count_owned_by(&game, Zone::Library, alice), 1);
}

#[test]
fn cheering_crowd_event_player_gets_the_counter_scaled_mana() {
    let definition = parse_oracle_card_definition("Cheering Crowd");
    let oracle = &oracle_text_by_name()["Cheering Crowd"];
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle.as_str(),
        "{:#?}",
        definition.spell_effect,
    );

    let triggered = triggered_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfPrecombatMainPhaseEvent::new(bob),
        crate::provenance::ProvNodeId::default(),
    );
    game.push_to_stack(
        crate::game_state::StackEntry::ability(source, alice, triggered.effects.clone())
            .with_triggering_event(event),
    );
    let mut decisions = SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Cheering Crowd trigger resolves");

    assert_eq!(
        game.counter_count(source, crate::object::CounterType::PlusOnePlusOne),
        1
    );
    assert_eq!(game.player(bob).expect("Bob exists").mana_pool.colorless, 1);
    assert_eq!(
        game.player(alice)
            .expect("Alice exists")
            .mana_pool
            .colorless,
        0
    );
}

fn damage_pair(
    effects: &[Effect],
) -> (
    &crate::effects::DealDamageEffect,
    &crate::effects::DealDamageEffect,
) {
    let [sequence] = effects else {
        panic!("expected one sequence: {effects:#?}");
    };
    let sequence = if let Some(with_id) = sequence.downcast_ref::<crate::effects::WithIdEffect>() {
        with_id.effect.as_ref()
    } else {
        sequence
    };
    let sequence = sequence
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("coordinated damage sequence");
    let [first, second] = sequence.effects.as_slice() else {
        panic!("expected damage pair: {sequence:#?}");
    };
    let first = first
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .expect("first damage");
    let tagged = second
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("tagged second damage");
    let second = tagged
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .expect("second damage");
    (first, second)
}

#[test]
fn searing_blaze_replacement_reuses_both_announced_targets_and_source_line() {
    let definition = parse_oracle_card_definition("Searing Blaze");
    let oracle = &oracle_text_by_name()["Searing Blaze"];
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle.as_str()
    );

    let program = definition.spell_effect.as_ref().expect("spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one replacement segment: {program:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one Landfall replacement: {segment:#?}");
    };
    let (default_first, default_second) = damage_pair(&segment.default_effects);
    let (replacement_first, replacement_second) = damage_pair(&branch.replacement_effects);
    assert_eq!(replacement_first.target, default_first.target);
    assert_eq!(replacement_second.target, default_second.target);
    assert!(branch.starts_new_source_line);

    for (had_land_enter, expected_damage) in [(false, 1_u32), (true, 3_u32)] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = CardDefinitionBuilder::new(CardId::new(), "Blaze Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 8))
            .build();
        let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        if had_land_enter {
            let land = CardDefinitionBuilder::new(CardId::new(), "Landfall Witness")
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_definition(&land, alice, Zone::Hand);
            game.move_object_by_effect(land, Zone::Battlefield)
                .expect("land enters");
        }

        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        game.push_to_stack(
            crate::game_state::StackEntry::new(spell, alice)
                .with_targets(vec![
                    crate::game_state::Target::Player(bob),
                    crate::game_state::Target::Object(target),
                ])
                .with_target_assignments(vec![
                    crate::game_state::TargetAssignment {
                        spec: default_first.target.clone(),
                        range: 0..1,
                    },
                    crate::game_state::TargetAssignment {
                        spec: default_second.target.clone(),
                        range: 1..2,
                    },
                ]),
        );
        crate::game_loop::resolve_stack_entry(&mut game).expect("Searing Blaze resolves");

        assert_eq!(
            20 - game.player(bob).expect("Bob exists").life,
            expected_damage as i32
        );
        assert_eq!(game.damage_on(target), expected_damage);
    }
}
