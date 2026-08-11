#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Devoid\nWhen you cast this spell, for each opponent, choose a creature that player controls. Create a token that's a copy of one of those creatures, except its power is equal to the total power of those creatures, its toughness is equal to the total toughness of those creatures, and it's a colorless Eldrazi creature.";

fn cast_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("cast trigger")
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

#[test]
fn aggregate_participant_copy_keeps_exact_typed_structure_and_surface() {
    let definition = parse_oracle_card_definition("Benthic Anomaly");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        ORACLE,
        "authoritative cast-trigger program: {:#?}",
        cast_trigger(&definition).effects
    );

    let [choice_segment, copy_segment] = cast_trigger(&definition).effects.segments.as_slice()
    else {
        panic!("expected choice and copy segments: {definition:#?}");
    };
    let [for_players_effect] = choice_segment.default_effects.as_slice() else {
        panic!("expected participant choice");
    };
    let for_players = for_players_effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("typed opponent iteration");
    assert_eq!(for_players.filter, PlayerFilter::Opponent);
    let [choose_effect] = for_players.effects.as_slice() else {
        panic!("expected one choice per opponent");
    };
    let choose = choose_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("typed creature choice");
    assert_eq!(choose.chooser, PlayerFilter::You);
    assert!(!choose.replace_tagged_objects);

    let [copy_effect] = copy_segment.default_effects.as_slice() else {
        panic!("expected one token-copy effect");
    };
    let copy = copy_effect
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()
        .expect("typed token copy");
    let (power, toughness) = copy
        .set_base_power_toughness_value
        .as_ref()
        .expect("dynamic copiable base P/T");
    assert!(matches!(power.unhinted(), Value::TotalPower(_)));
    assert!(matches!(toughness.unhinted(), Value::TotalToughness(_)));
    assert_eq!(copy.set_colors, Some(crate::color::ColorSet::new()));
    assert_eq!(
        copy.set_card_types.as_deref(),
        Some(&[CardType::Creature][..])
    );
    assert_eq!(copy.set_subtypes.as_deref(), Some(&[Subtype::Eldrazi][..]));
}

#[test]
fn aggregate_participant_copy_uses_every_choice_for_pt_and_is_colorless_eldrazi() {
    let definition = parse_oracle_card_definition("Benthic Anomaly");
    let triggered = cast_trigger(&definition).clone();
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.create_object_from_definition(&creature("Bob Choice", 2, 3), bob, Zone::Battlefield);
    game.create_object_from_definition(&creature("Cara Choice", 4, 5), cara, Zone::Battlefield);
    let before = game.objects_in_zone(Zone::Battlefield);

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("aggregate copy trigger should resolve");

    let created = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .find(|id| !before.contains(id) && game.controller_of_id(*id) == Some(alice))
        .expect("one token copy should be created");
    assert_eq!(game.current_power(created), Some(6));
    assert_eq!(game.current_toughness(created), Some(8));
    assert_eq!(
        game.current_colors(created),
        Some(crate::color::ColorSet::new())
    );
    assert_eq!(
        game.current_card_types(created),
        Some(vec![CardType::Creature])
    );
    assert_eq!(game.current_subtypes(created), Some(vec![Subtype::Eldrazi]));
}
