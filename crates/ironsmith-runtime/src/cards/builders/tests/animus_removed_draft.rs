#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const CONDITIONAL_LADDER: &str = "If you removed a creature card with flying from the draft with cards named Draft Mimic, this creature has flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, and vigilance.";
const ANIMUS_COMPILED_LINES: &[&str] = &[
    "Draft this card face up.",
    "As you draft a card, you may remove it from the draft face up.",
    "If you removed a creature card with flying from the draft with cards named Animus of Predation, this creature has flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, and vigilance.",
];

fn draft_mimic_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Draft Mimic")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .parse_text(CONDITIONAL_LADDER)
        .expect("removed-from-draft characteristic ladder should parse")
}

fn creature_with_keyword(
    name: &str,
    keyword: crate::static_abilities::StaticAbility,
) -> crate::cards::CardDefinition {
    let mut definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .build();
    definition
        .abilities
        .push(crate::ability::Ability::static_ability(keyword));
    definition
}

#[test]
fn removed_from_draft_ladder_keeps_typed_conditions_and_exact_surface() {
    let definition = draft_mimic_definition();
    let debug = format!("{:#?}", definition.abilities);
    assert_eq!(
        debug.matches("PlayerRemovedDraftCardMatching").count(),
        11,
        "each keyword grant must retain its independent draft condition: {debug}"
    );
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![CONDITIONAL_LADDER.to_string()]
    );
}

#[test]
fn removed_from_draft_cards_grant_only_matching_printed_keywords_for_player_and_group() {
    let definition = draft_mimic_definition();
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let flying_card = creature_with_keyword(
        "Flying Draft Card",
        crate::static_abilities::StaticAbility::flying(),
    );
    let flying = game.create_object_from_definition(&flying_card, alice, Zone::OutsideGame);
    assert!(game.record_card_removed_from_draft(bob, flying, "Draft Mimic"));
    assert!(
        !game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Flying,
        ),
        "another player's draft record must not satisfy the source controller's condition"
    );
    assert!(game.record_card_removed_from_draft(alice, flying, "Draft Mimic"));
    assert!(
        game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Flying,
        )
    );
    assert!(
        !game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::FirstStrike,
        ),
        "a flying card must not satisfy unrelated characteristic filters"
    );

    let first_strike_card = creature_with_keyword(
        "First Strike Draft Card",
        crate::static_abilities::StaticAbility::first_strike(),
    );
    let first_strike =
        game.create_object_from_definition(&first_strike_card, alice, Zone::OutsideGame);
    assert!(game.record_card_removed_from_draft(alice, first_strike, "Other Group"));
    assert!(!game.current_has_static_ability_id(
        source,
        crate::static_abilities::StaticAbilityId::FirstStrike,
    ));
    assert!(game.record_card_removed_from_draft(alice, first_strike, "draft mimic"));
    assert!(
        game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::FirstStrike,
        ),
        "recording a matching printed keyword must invalidate continuous state and grant it"
    );
}

#[test]
fn cards_json_animus_keeps_the_exact_ladder_and_executes_its_named_group_condition() {
    let definition = parse_oracle_card_definition("Animus of Predation");
    assert_eq!(
        canonical_compiled_lines(&definition),
        ANIMUS_COMPILED_LINES
            .iter()
            .map(|line| (*line).to_string())
            .collect::<Vec<_>>()
    );
    let debug = format!("{:#?}", definition.abilities);
    assert_eq!(
        debug.matches("PlayerRemovedDraftCardMatching").count(),
        11,
        "every expanded keyword must retain its independent draft condition: {debug}"
    );

    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let flying_card = creature_with_keyword(
        "Animus Flying Draft Card",
        crate::static_abilities::StaticAbility::flying(),
    );
    let flying = game.create_object_from_definition(&flying_card, alice, Zone::OutsideGame);

    assert!(game.record_card_removed_from_draft(bob, flying, "Animus of Predation"));
    assert!(
        !game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Flying,
        )
    );
    assert!(game.record_card_removed_from_draft(alice, flying, "Other Draft Group"));
    assert!(
        !game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Flying,
        )
    );
    assert!(game.record_card_removed_from_draft(alice, flying, "animus of predation"));
    assert!(
        game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Flying,
        )
    );
}
