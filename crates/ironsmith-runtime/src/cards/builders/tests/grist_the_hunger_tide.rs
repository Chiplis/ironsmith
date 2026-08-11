#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const GRIST_TEXT: &str = "As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.\n+1: Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way, put a loyalty counter on Grist and repeat this process.\n−2: You may sacrifice a creature. When you do, destroy target creature or planeswalker.\n−5: Each opponent loses life equal to the number of creature cards in your graveyard.";

#[test]
fn grist_rejoins_typed_characteristics_and_shared_repeat_condition() {
    let definition = parse_oracle_card_definition("Grist, the Hunger Tide");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), GRIST_TEXT);
    assert!(definition.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::SourceLineStaticGroup
        )
    }));

    let repeat = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_loyalty_ability => activated
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<crate::effects::RepeatProcessEffect>()),
            _ => None,
        })
        .expect("Grist's +1 should remain one executable repeat process");
    let [producer, conditional] = repeat.effects.as_slice() else {
        panic!("Grist's repeat should retain producer and conditional: {repeat:#?}");
    };
    let producer = producer
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("the mill producer should retain its result ID");
    let conditional = conditional
        .downcast_ref::<crate::effects::IfEffect>()
        .expect("the loyalty action should retain its typed result gate");
    assert_eq!(producer.id, repeat.condition);
    assert_eq!(conditional.condition, repeat.condition);
    assert_eq!(conditional.predicate, repeat.predicate);
}

#[test]
fn grist_is_a_one_one_insect_creature_in_every_zone_except_the_battlefield() {
    let definition = parse_oracle_card_definition("Grist, the Hunger Tide");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);

    for zone in [
        Zone::Hand,
        Zone::Stack,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Library,
        Zone::Command,
    ] {
        let grist = game.create_object_from_definition(&definition, alice, zone);
        let card_types = game
            .current_card_types(grist)
            .expect("Grist has card types");
        assert!(
            card_types.contains(&CardType::Planeswalker),
            "Grist keeps its planeswalker type in {zone:?}: {card_types:?}"
        );
        assert!(
            card_types.contains(&CardType::Creature),
            "Grist is a creature in {zone:?}: {card_types:?}"
        );
        assert!(
            game.current_has_subtype(grist, Subtype::Insect),
            "Grist is an Insect in {zone:?}"
        );
        assert_eq!(
            game.current_power(grist),
            Some(1),
            "Grist power in {zone:?}"
        );
        assert_eq!(
            game.current_toughness(grist),
            Some(1),
            "Grist toughness in {zone:?}"
        );
    }

    let battlefield_grist =
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let battlefield_types = game
        .current_card_types(battlefield_grist)
        .expect("battlefield Grist has card types");
    assert_eq!(battlefield_types, vec![CardType::Planeswalker]);
    assert!(!game.current_has_subtype(battlefield_grist, Subtype::Insect));
    assert_eq!(game.current_power(battlefield_grist), None);
    assert_eq!(game.current_toughness(battlefield_grist), None);
}
