#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "At the beginning of your upkeep, if this card is the only creature card in your graveyard, you may return this card to the battlefield.";

fn upkeep(player: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

fn card(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

#[test]
fn nether_spirit_keeps_the_only_creature_gate_and_graveyard_functional_zone() {
    let definition = parse_oracle_card_definition("Nether Spirit");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let ability = definition
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
        .expect("Nether Spirit should have an upkeep trigger");
    assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        unreachable!();
    };
    let crate::effect::Condition::ValueComparison {
        left: crate::effect::Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: crate::effect::Value::Fixed(1),
    } = triggered
        .intervening_if
        .as_ref()
        .expect("the authored `only` clause must remain an intervening condition")
    else {
        panic!("expected an exact graveyard count: {triggered:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.card_types, vec![CardType::Creature]);
}

#[test]
fn nether_spirit_triggers_only_as_the_sole_creature_card_in_its_owners_graveyard() {
    let definition = parse_oracle_card_definition("Nether Spirit");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);

    let source_triggers = |game: &crate::GameState| {
        crate::triggers::check_triggers(game, &upkeep(alice))
            .into_iter()
            .filter(|entry| entry.source == source)
            .count()
    };
    assert_eq!(source_triggers(&game), 1);

    game.create_object_from_definition(
        &card("Irrelevant Sorcery", CardType::Sorcery),
        alice,
        Zone::Graveyard,
    );
    assert_eq!(
        source_triggers(&game),
        1,
        "noncreature cards must not suppress the trigger"
    );

    game.create_object_from_definition(
        &card("Other Creature", CardType::Creature),
        alice,
        Zone::Graveyard,
    );
    assert_eq!(
        source_triggers(&game),
        0,
        "another creature card in the owner's graveyard suppresses the trigger"
    );
}
