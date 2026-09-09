use super::*;
const TEXT: &str = "Whenever Gran-Gran becomes tapped, draw a card, then discard a card.\nNoncreature spells you cast cost {1} less to cast as long as there are three or more Lesson cards in your graveyard.";
#[test]
fn graveyard_threshold_discount_counts_only_matching_cards_for_the_sources_controller() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Gran-Gran")
        .card_types(vec![CardType::Creature])
        .parse_text(TEXT)
        .unwrap();
    let base = crate::mana::ManaCost::from_symbols(vec![
        crate::mana::ManaSymbol::Generic(3),
        crate::mana::ManaSymbol::Blue,
    ]);
    for count in [0, 2, 3, 4] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let lesson = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Lesson Fixture")
            .card_types(vec![CardType::Sorcery])
            .subtypes(vec![Subtype::Lesson])
            .build();
        for _ in 0..count {
            game.create_object_from_card(&lesson, alice, Zone::Graveyard);
        }
        for _ in 0..4 {
            game.create_object_from_card(&lesson, bob, Zone::Graveyard);
            game.create_object_from_card(&lesson, alice, Zone::Exile);
        }
        let other = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Ordinary Card")
            .card_types(vec![CardType::Sorcery])
            .build();
        for _ in 0..4 {
            game.create_object_from_card(&other, alice, Zone::Graveyard);
        }
        for caster in [alice, bob] {
            for types in [
                vec![CardType::Instant],
                vec![CardType::Artifact],
                vec![CardType::Creature],
                vec![CardType::Artifact, CardType::Creature],
            ] {
                let noncreature = !types.contains(&CardType::Creature);
                let card =
                    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Candidate Spell")
                        .card_types(types)
                        .mana_cost(base.clone())
                        .build();
                let spell = game.create_object_from_card(&card, caster, Zone::Hand);
                let cost = crate::decision::calculate_effective_mana_cost(
                    &game,
                    caster,
                    game.object(spell).unwrap(),
                    &base,
                );
                assert_eq!(
                    cost.to_oracle(),
                    if count >= 3 && caster == alice && noncreature {
                        "{2}{U}"
                    } else {
                        "{3}{U}"
                    },
                    "count={count} caster={caster:?} noncreature={noncreature}"
                );
            }
        }
    }
}

#[test]
fn graveyard_threshold_discount_keeps_leading_and_trailing_conditions() {
    let text = "During your turn, noncreature spells you cast cost {1} less to cast as long as there are two or more artifact cards in your graveyard.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Conditional Reducer")
            .card_types(vec![CardType::Enchantment])
            .parse_text(text)
            .unwrap();
    let base = crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(3)]);
    for own_turn in [true, false] {
        for count in [1, 2] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            game.turn.active_player = if own_turn { alice } else { bob };
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let artifact =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Artifact Fixture")
                    .card_types(vec![CardType::Artifact])
                    .build();
            for _ in 0..count {
                game.create_object_from_card(&artifact, alice, Zone::Graveyard);
            }
            let instant = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Spell")
                .card_types(vec![CardType::Instant])
                .mana_cost(base.clone())
                .build();
            let id = game.create_object_from_card(&instant, alice, Zone::Hand);
            assert_eq!(
                crate::decision::calculate_effective_mana_cost(
                    &game,
                    alice,
                    game.object(id).unwrap(),
                    &base
                )
                .to_oracle(),
                if own_turn && count == 2 { "{2}" } else { "{3}" }
            );
        }
    }
}

#[test]
fn graveyard_threshold_discount_renders_its_condition() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Gran-Gran")
        .card_types(vec![CardType::Creature])
        .parse_text(TEXT)
        .unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("As long as there are three or more lesson cards in your graveyard"),
        "{rendered}"
    );
}
