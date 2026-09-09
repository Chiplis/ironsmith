use super::*;
const TEXT: &str = "Reach, trample\nThis creature can't attack or block unless its power is 6 or greater.\n{T}, Sacrifice another artifact: Draw a card. Put a +1/+1 counter on this creature.";
#[test]
fn source_power_combat_gate_tracks_current_power_on_battlefield() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Technodrome")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Construct]).power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(TEXT).unwrap();
    assert!(definition.spell_effect.is_none(), "combat gate belongs to static abilities");
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other_def = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other Creature")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
    let other = game.create_object_from_card(&other_def, alice, Zone::Battlefield);
    for counters in [0, 2, 3, 4, 2, 0] {
        game.remove_counters(source, CounterType::PlusOnePlusOne, 10, None, None);
        game.add_counters(source, CounterType::PlusOnePlusOne, counters);
        game.refresh_continuous_state();
        assert_eq!(game.can_attack(source), counters >= 3, "power={}", counters+3);
        assert_eq!(game.can_block(source), counters >= 3, "power={}", counters+3);
        assert!(game.can_attack(other));
        assert!(game.can_block(other));
    }
}
#[test]
fn source_power_combat_gate_keeps_static_text_order() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Technodrome")
        .card_types(vec![CardType::Artifact, CardType::Creature]).parse_text(TEXT).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), TEXT);
}
