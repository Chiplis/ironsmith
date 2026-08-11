#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const CONDITIONAL_BASE_PT_LINE: &str =
    "As long as enchanted permanent is a creature, it has base power and toughness 1/1.";

#[test]
fn awakened_awareness_sets_the_enchanted_creatures_base_pt_not_the_auras() {
    let definition = parse_oracle_card_definition("Awakened Awareness");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == CONDITIONAL_BASE_PT_LINE),
        "Awakened Awareness must retain the attached-object subject: {rendered:#?}"
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("EnchantedPermanentIsCreature"), "{debug}");
    assert!(debug.contains("\"enchanted\""), "{debug}");

    let creature = CardDefinitionBuilder::new(CardId::new(), "Awakened Host")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let host = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(host),));

    assert_eq!(
        game.current_power(host),
        Some(1),
        "the enchanted creature's base power must become 1"
    );
    assert_eq!(
        game.current_toughness(host),
        Some(1),
        "the enchanted creature's base toughness must become 1"
    );
    assert_eq!(
        game.current_power(aura),
        None,
        "the Aura itself must not become the 1/1 object"
    );
}
