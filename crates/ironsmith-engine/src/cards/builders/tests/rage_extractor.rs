#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

const ORACLE: &str = "Whenever you cast a spell with {H} in its mana cost, this artifact deals damage equal to that spell's mana value to any target.";

fn rage_trigger(
    definition: &CardDefinition,
) -> (
    &crate::ability::TriggeredAbility,
    &crate::triggers::SpellCastTrigger,
) {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>()
                .map(|cast| (triggered, cast)),
            _ => None,
        })
        .expect("Rage Extractor should retain its spell-cast trigger")
}

#[test]
fn rage_extractor_keeps_the_typed_phyrexian_mana_cost_filter() {
    let definition = parse_oracle_card_definition("Rage Extractor");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let (_, trigger) = rage_trigger(&definition);
    let filter = trigger
        .filter
        .as_ref()
        .expect("the trigger should constrain the triggering spell");
    assert!(filter.has_mana_cost, "{filter:#?}");
    assert!(filter.has_phyrexian_mana_symbol, "{filter:#?}");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let phyrexian = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
            crate::mana::ManaSymbol::Life(2),
        ]]))
        .build();
    let ordinary = CardDefinitionBuilder::new(CardId::new(), "Ordinary Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Blue,
        ]))
        .build();
    let phyrexian_id = game.create_object_from_definition(&phyrexian, alice, Zone::Stack);
    let ordinary_id = game.create_object_from_definition(&ordinary, alice, Zone::Stack);
    let ctx = TriggerContext::for_source(source, alice, &game);

    for (spell, expected) in [(phyrexian_id, true), (ordinary_id, false)] {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        assert_eq!(trigger.matches(&event, &ctx), expected);
    }
}
