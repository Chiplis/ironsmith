#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "I, II — Discard a card, then draw a card.\nIII — Reveal up to five nonland cards from your hand. For each of those cards that has the same mana value as another card revealed this way, create a Treasure token.";

fn hand_card(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(mana_value),
        ]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

fn chapter_three(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered.trigger.saga_chapters() == Some(&[3]) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("expected chapter III")
}

#[test]
fn vault_21_keeps_the_distinct_same_mana_value_relation() {
    let definition = parse_oracle_card_definition("Vault 21: House Gambit");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        ORACLE,
        "{:#?}",
        chapter_three(&definition).effects
    );
    let debug = format!("{:#?}", chapter_three(&definition).effects);
    assert!(debug.contains("SameManaValueAsAnotherTagged"), "{debug}");
    assert!(debug.contains("ForEachTaggedEffect"), "{debug}");
}

#[test]
fn vault_21_creates_treasures_only_for_cards_with_a_different_equal_value_card() {
    let definition = parse_oracle_card_definition("Vault 21: House Gambit");
    let chapter = chapter_three(&definition).clone();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for (name, mana_value) in [
        ("Pair Two A", 2),
        ("Pair Two B", 2),
        ("Unique Three", 3),
        ("Pair Four A", 4),
        ("Pair Four B", 4),
    ] {
        game.create_object_from_definition(&hand_card(name, mana_value), alice, Zone::Hand);
    }

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &chapter.effects,
        None,
        &[],
    )
    .expect("chapter III should resolve");

    let treasures = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            game.controller_of_id(*id) == Some(alice)
                && game
                    .object(*id)
                    .is_some_and(|object| object.subtypes.contains(&Subtype::Treasure))
        })
        .count();
    assert_eq!(
        treasures, 4,
        "the unique mana-value card must not count itself"
    );
}
