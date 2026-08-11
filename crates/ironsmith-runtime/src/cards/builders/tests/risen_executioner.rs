#![cfg(ironsmith_runtime_parser_tests)]

use super::*;

const ORACLE: &str = "This creature can't block.\nOther Zombie creatures you control get +1/+1.\nYou may cast this creature from your graveyard if you pay {1} more to cast it for each other creature card in your graveyard.";

fn definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Risen Executioner")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Warrior])
        .power_toughness(crate::card::PowerToughness::fixed(4, 3))
        .parse_text(ORACLE)
        .expect("source graveyard permission with a dynamic surcharge should parse")
}

fn graveyard_probe(name: &str, card_type: CardType) -> crate::card::Card {
    let mut builder = CardBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(crate::card::PowerToughness::fixed(1, 1));
    }
    builder.build()
}

#[test]
fn risen_executioner_casts_from_graveyard_with_one_generic_tax_per_other_creature_card() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let definition = definition();
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("GrantSpec"), "{debug}");
    assert!(debug.contains("CostIncrease"), "{debug}");
    assert!(debug.contains("Count("), "{debug}");
    assert_eq!(
        canonical_compiled_lines(&definition),
        ORACLE.lines().collect::<Vec<_>>()
    );

    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
    game.create_object_from_card(
        &graveyard_probe("Creature One", CardType::Creature),
        alice,
        Zone::Graveyard,
    );
    game.create_object_from_card(
        &graveyard_probe("Creature Two", CardType::Creature),
        alice,
        Zone::Graveyard,
    );
    game.create_object_from_card(
        &graveyard_probe("Noncreature Probe", CardType::Artifact),
        alice,
        Zone::Graveyard,
    );

    assert!(game.effect_store.grant_registry.card_can_play_from_zone(
        &game,
        source,
        Zone::Graveyard,
        alice,
    ));
    let source_object = game
        .object(source)
        .expect("Risen should remain in graveyard");
    let base_cost = source_object
        .mana_cost
        .as_ref()
        .expect("Risen should have a mana cost");
    let graveyard_method = CastingMethod::PlayFrom {
        source,
        zone: Zone::Graveyard,
        use_alternative: None,
    };
    assert_eq!(
        crate::decision::calculate_effective_mana_cost_for_casting_method(
            &game,
            alice,
            source_object,
            base_cost,
            &graveyard_method,
        )
        .to_oracle(),
        "{4}{B}{B}"
    );

    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Black, 6);
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::PlayFrom { use_alternative: None, .. },
            } if *spell_id == source
        )),
        "expected a taxed graveyard cast action, got {actions:?}"
    );
}
