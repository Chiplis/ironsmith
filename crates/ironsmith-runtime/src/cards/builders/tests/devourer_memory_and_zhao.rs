#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn devourer_of_memory_keeps_library_origin_and_both_source_modifications() {
    let definition = parse_oracle_card_definition("Devourer of Memory");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Whenever one or more cards are put into your graveyard from your library, this creature gets +1/+1 until end of turn and can't be blocked this turn.",
            "{1}{U}{B}: Mill a card.",
        ]
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Devourer must keep its graveyard trigger");
    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .expect("graveyard trigger must lower to a zone-change matcher");
    assert_eq!(
        zone_change.from,
        crate::triggers::ZonePattern::Specific(Zone::Library)
    );
    assert_eq!(
        zone_change.to,
        crate::triggers::ZonePattern::Specific(Zone::Graveyard)
    );
    assert_eq!(
        zone_change.count_mode,
        crate::triggers::CountMode::OneOrMore
    );
    let debug = format!("{:#?}", triggered.effects);
    assert!(debug.contains("ApplyContinuousEffect"), "{debug}");
    assert!(debug.contains("BeBlocked"), "{debug}");
    assert!(debug.contains("EndOfTurn"), "{debug}");
}

fn land(name: &str, basic: bool) -> CardDefinition {
    let mut builder =
        CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Land]);
    if basic {
        builder = builder.supertypes(vec![Supertype::Basic]);
    }
    builder.build()
}

#[test]
fn zhao_keeps_exact_counter_threshold_and_nonbasic_land_rules() {
    let definition = parse_oracle_card_definition("Zhao, the Moon Slayer");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Menace",
            "Nonbasic lands enter tapped.",
            "{7}: Put a conqueror counter on Zhao.",
            "As long as Zhao has a conqueror counter on him, nonbasic lands are Mountains.",
        ]
    );

    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let nonbasic =
        game.create_object_from_definition(&land("Threshold Nonbasic", false), alice, Zone::Hand);
    let nonbasic = game
        .move_object_with_etb_processing(nonbasic, Zone::Battlefield)
        .expect("nonbasic land should enter");
    assert!(game.is_tapped(nonbasic.new_id));

    let basic =
        game.create_object_from_definition(&land("Threshold Basic", true), alice, Zone::Hand);
    let basic = game
        .move_object_with_etb_processing(basic, Zone::Battlefield)
        .expect("basic land should enter");
    assert!(!game.is_tapped(basic.new_id));

    assert!(
        !game
            .current_subtypes(nonbasic.new_id)
            .unwrap_or_default()
            .contains(&Subtype::Mountain)
    );
    game.add_counters(source, CounterType::Named("conqueror"), 1);
    assert!(
        game.current_subtypes(nonbasic.new_id)
            .unwrap_or_default()
            .contains(&Subtype::Mountain)
    );
    game.remove_counters(source, CounterType::Named("conqueror"), 1, None, None)
        .expect("conqueror counter should be removed");
    assert!(
        !game
            .current_subtypes(nonbasic.new_id)
            .unwrap_or_default()
            .contains(&Subtype::Mountain)
    );
}
