#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Haste\nAt the beginning of your upkeep, if this card is in your graveyard with three or more creature cards above it, you may put this card onto the battlefield.";

fn upkeep(player: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .build()
}

fn source_trigger_count(game: &crate::GameState, source: ObjectId, player: PlayerId) -> usize {
    crate::triggers::check_triggers(game, &upkeep(player))
        .into_iter()
        .filter(|entry| entry.source == source)
        .count()
}

#[test]
fn nether_shadow_keeps_the_ordered_graveyard_condition() {
    let definition = parse_oracle_card_definition("Nether Shadow");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some((ability, triggered)),
            _ => None,
        })
        .expect("Nether Shadow should have an upkeep trigger");
    assert_eq!(triggered.0.functional_zones, vec![Zone::Graveyard]);
    let crate::effect::Condition::SourceInGraveyardWithCardsAbove { filter, count } =
        triggered.1.intervening_if.as_ref().expect("condition")
    else {
        panic!("unexpected condition: {:#?}", triggered.1.intervening_if);
    };
    assert_eq!(*count, 3);
    assert_eq!(filter.card_types, vec![CardType::Creature]);
}

#[test]
fn nether_shadow_counts_only_matching_cards_above_it() {
    let definition = parse_oracle_card_definition("Nether Shadow");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);

    let below = creature("Creature Below");
    for _ in 0..3 {
        game.create_object_from_definition(&below, alice, Zone::Graveyard);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
    assert_eq!(source_trigger_count(&game, source, alice), 0);

    let above = creature("Creature Above");
    for _ in 0..2 {
        game.create_object_from_definition(&above, alice, Zone::Graveyard);
    }
    assert_eq!(source_trigger_count(&game, source, alice), 0);
    game.create_object_from_definition(&above, alice, Zone::Graveyard);
    assert_eq!(source_trigger_count(&game, source, alice), 1);
}
