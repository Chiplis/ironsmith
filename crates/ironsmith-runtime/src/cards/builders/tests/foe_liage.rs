#![cfg(ironsmith_runtime_parser_tests)]

use super::*;

#[test]
fn foe_liage_preserves_the_etb_active_turn_qualifier_end_to_end() {
    let oracle = "Whenever a land enters during your turn, put a +1/+1 counter on this creature.";
    let definition = CardDefinitionBuilder::new(CardId::new(), "Foe-liage Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Plant, Subtype::Mutant])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(oracle)
        .expect("ETB trigger with an active-turn qualifier should parse");

    assert_eq!(canonical_compiled_lines(&definition), vec![oracle]);
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!(
            "expected a triggered ability, got {:#?}",
            definition.abilities
        );
    };
    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .unwrap_or_else(|| panic!("expected a zone-change trigger, got {triggered:#?}"));
    assert_eq!(zone_change.during_turn, Some(PlayerFilter::You));
}
