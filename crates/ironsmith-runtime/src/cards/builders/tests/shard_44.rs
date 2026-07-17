use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn as_enters_program_count(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .filter(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return false;
            };
            matches!(
                static_ability.compiled_model().map(|model| &model.payload),
                Some(ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram { .. })
            )
        })
        .count()
}

#[test]
pub(super) fn named_generic_as_enters_setup_cards_keep_typed_timing_and_line_position() {
    let cases = [
        (
            "Crowd-Control Warden",
            "as this creature enters or is turned face up,",
            "where x is the number of other creatures you control",
        ),
        (
            "Dermotaxi",
            "imprint — as this vehicle enters,",
            "exile a creature card from a graveyard",
        ),
        (
            "Monstrous War-Leech",
            "as this creature enters,",
            "if it was kicked, mill four cards",
        ),
        (
            "Overlaid Terrain",
            "as this enchantment enters,",
            "sacrifice all lands you control",
        ),
        (
            "Rescuer Sphinx",
            "as this creature enters,",
            "you may return a nonland permanent you control",
        ),
        (
            "Thief of Blood",
            "as this creature enters,",
            "remove all counters from",
        ),
        (
            "Wood Elemental",
            "as this creature enters,",
            "sacrifice any number of untapped forests",
        ),
    ];

    for (name, expected_prefix, expected_body) in cases {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            as_enters_program_count(&definition),
            1,
            "{name} should compile its setup as one typed as-enters program: {definition:#?}"
        );
        let lines = unprocessed_compiled_lines(&definition);
        let setup_idx = lines
            .iter()
            .position(|line| line.to_ascii_lowercase().starts_with(expected_prefix))
            .unwrap_or_else(|| panic!("{name} lost its as-enters timing surface: {lines:#?}"));
        assert!(
            lines[setup_idx]
                .to_ascii_lowercase()
                .contains(expected_body),
            "{name} entry program body collapsed or changed: {lines:#?}"
        );
        assert!(definition.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::AsEntersEffectProgram
            )
        }));
        let oracle_first_line = match name {
            "Crowd-Control Warden" | "Dermotaxi" | "Overlaid Terrain" | "Wood Elemental" => 0,
            "Monstrous War-Leech" | "Rescuer Sphinx" | "Thief of Blood" => 1,
            _ => unreachable!(),
        };
        assert_eq!(
            setup_idx, oracle_first_line,
            "{name} setup moved away from its authored line position: {lines:#?}"
        );
    }

    for name in ["Rescuer Sphinx", "Thief of Blood"] {
        let lines = unprocessed_compiled_lines(&parse_oracle_card_definition(name));
        assert!(
            lines.iter().any(|line| line
                .to_ascii_lowercase()
                .contains("this creature enters with a +1/+1 counter on it")),
            "{name} lost its authored enters-with counter surface: {lines:#?}"
        );
    }

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition("Thief of Blood"))
            .join("\n"),
        "Flying\nAs this creature enters, remove all counters from all permanents. This creature enters with a +1/+1 counter on it for each counter removed this way."
    );
    assert_eq!(
        unprocessed_compiled_lines(&parse_oracle_card_definition("Wood Elemental")).join("\n"),
        "As this creature enters, sacrifice any number of untapped Forests.\nThis creature's power and toughness are each equal to the number of Forests sacrificed as it entered."
    );
}

#[test]
pub(super) fn crowd_control_warden_as_enters_counters_reach_the_prospective_permanent() {
    let definition = parse_oracle_card_definition("Crowd-Control Warden");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let other = CardDefinitionBuilder::new(CardId::new(), "Other Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_definition(&other, alice, Zone::Battlefield);
    let hand_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut decisions)
        .expect("Crowd-Control Warden should enter");

    assert_eq!(
        game.object(entered.new_id).and_then(|object| {
            object
                .counters
                .get(&crate::object::CounterType::PlusOnePlusOne)
                .copied()
        }),
        Some(1),
        "the counter added during setup must be committed on the new battlefield object"
    );
}

#[test]
pub(super) fn dermotaxi_as_enters_exile_link_moves_to_the_entered_permanent() {
    let definition = parse_oracle_card_definition("Dermotaxi");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Imprint Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_definition(&creature, alice, Zone::Graveyard);
    let hand_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut decisions)
        .expect("Dermotaxi should enter");

    assert!(game.get_exiled_with_source_links(hand_id).is_empty());
    let linked = game.get_exiled_with_source_links(entered.new_id);
    assert_eq!(linked.len(), 1);
    assert_eq!(
        game.object(linked[0]).map(|object| object.zone),
        Some(Zone::Exile)
    );
}

#[test]
pub(super) fn overlaid_terrain_as_enters_setup_finishes_before_entry_commit() {
    let definition = parse_oracle_card_definition("Overlaid Terrain");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let land = CardDefinitionBuilder::new(CardId::new(), "Setup Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    game.create_object_from_definition(&land, alice, Zone::Battlefield);
    game.create_object_from_definition(&land, alice, Zone::Battlefield);
    let hand_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut decisions)
        .expect("Overlaid Terrain should enter");

    assert!(game.object(entered.new_id).is_some());
    assert!(game.battlefield.iter().copied().all(|id| {
        game.object(id).is_none_or(|object| {
            game.controller_of(object) != alice || !object.card_types.contains(&CardType::Land)
        })
    }));
}

#[test]
pub(super) fn wood_elemental_remembers_only_forests_sacrificed_as_it_entered() {
    let definition = parse_oracle_card_definition("Wood Elemental");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let forest = CardDefinitionBuilder::new(CardId::new(), "Setup Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    game.create_object_from_definition(&forest, alice, Zone::Battlefield);
    game.create_object_from_definition(&forest, alice, Zone::Battlefield);
    let hand_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut decisions)
        .expect("Wood Elemental should enter");

    assert_eq!(game.calculated_power(entered.new_id), Some(2));
    assert_eq!(game.calculated_toughness(entered.new_id), Some(2));

    game.create_object_from_definition(&forest, alice, Zone::Battlefield);
    assert_eq!(game.calculated_power(entered.new_id), Some(2));
    assert_eq!(game.calculated_toughness(entered.new_id), Some(2));
}

#[test]
pub(super) fn thief_of_blood_removes_every_permanents_counters_and_keeps_the_total() {
    let definition = parse_oracle_card_definition("Thief of Blood");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let permanent = CardDefinitionBuilder::new(CardId::new(), "Countered Permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    let first = game.create_object_from_definition(&permanent, alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&permanent, alice, Zone::Battlefield);
    game.object_mut(first)
        .expect("first permanent")
        .counters
        .insert(crate::object::CounterType::Charge, 2);
    game.object_mut(second)
        .expect("second permanent")
        .counters
        .insert(crate::object::CounterType::Charge, 3);
    let hand_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut decisions)
        .expect("Thief of Blood should enter");

    assert_eq!(
        game.object(first).unwrap().counters.values().sum::<u32>(),
        0
    );
    assert_eq!(
        game.object(second).unwrap().counters.values().sum::<u32>(),
        0
    );
    assert_eq!(
        game.object(entered.new_id)
            .and_then(|object| object
                .counters
                .get(&crate::object::CounterType::PlusOnePlusOne))
            .copied(),
        Some(5)
    );
}
