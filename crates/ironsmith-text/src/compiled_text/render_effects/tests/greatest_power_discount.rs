use super::*;
const TEXT: &str = "This spell costs {X} less to cast, where X is the greatest power among creatures you control.\nTrample";
#[test]
fn greatest_power_discount_uses_current_controlled_battlefield_maximum() {
    let cost = crate::mana::ManaCost::from_symbols(vec![
        crate::mana::ManaSymbol::Generic(7),
        crate::mana::ManaSymbol::Red,
    ]);
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Molten Monstrosity")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Hellion])
            .power_toughness(crate::card::PowerToughness::fixed(5, 5))
            .mana_cost(cost.clone())
            .parse_text(TEXT)
            .unwrap();
    for (powers, expected) in [
        (vec![], "{7}{R}"),
        (vec![2, 4], "{3}{R}"),
        (vec![-3, -1], "{7}{R}"),
        (vec![9], "{R}"),
    ] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        for (player, zone, power) in powers
            .iter()
            .map(|p| (alice, Zone::Battlefield, *p))
            .chain([
                (bob, Zone::Battlefield, 20),
                (alice, Zone::Graveyard, 15),
                (alice, Zone::Hand, 12),
            ])
        {
            let creature =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Scope Creature")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(power, 5))
                    .build();
            game.create_object_from_card(&creature, player, zone);
        }
        let actual = crate::decision::calculate_effective_mana_cost(
            &game,
            alice,
            game.object(source).unwrap(),
            &cost,
        );
        assert_eq!(actual.to_oracle(), expected, "powers={powers:?}");
        if powers == vec![2, 4] {
            let strongest = game
                .battlefield
                .iter()
                .copied()
                .find(|id| {
                    game.controller_of_id(*id) == Some(alice)
                        && game.calculated_power(*id) == Some(4)
                })
                .unwrap();
            game.add_counters(strongest, CounterType::PlusOnePlusOne, 2);
            game.refresh_continuous_state();
            assert_eq!(
                crate::decision::calculate_effective_mana_cost(
                    &game,
                    alice,
                    game.object(source).unwrap(),
                    &cost
                )
                .to_oracle(),
                "{1}{R}"
            );
            game.set_current_controller(strongest, bob);
            assert_eq!(
                crate::decision::calculate_effective_mana_cost(
                    &game,
                    alice,
                    game.object(source).unwrap(),
                    &cost
                )
                .to_oracle(),
                "{5}{R}"
            );
        }
    }
}
#[test]
fn greatest_power_discount_renders_the_typed_scope() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Molten Monstrosity")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn greatest_power_discount_also_applies_to_an_artifact_spell() {
    let cost = crate::mana::ManaCost::from_symbols(vec![
        crate::mana::ManaSymbol::Generic(6),
        crate::mana::ManaSymbol::Green,
        crate::mana::ManaSymbol::Green,
    ]);
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "The Skullspore Nexus")
            .card_types(vec![CardType::Artifact])
            .mana_cost(cost.clone())
            .parse_text(TEXT.lines().next().unwrap())
            .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    for power in [2, 4, 9] {
        let creature =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Discount Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(power, 5))
                .build();
        game.create_object_from_card(&creature, alice, Zone::Battlefield);
        assert_eq!(
            crate::decision::calculate_effective_mana_cost(
                &game,
                alice,
                game.object(source).unwrap(),
                &cost
            )
            .to_oracle(),
            match power {
                2 => "{4}{G}{G}",
                4 => "{2}{G}{G}",
                _ => "{G}{G}",
            }
        );
    }
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT.lines().next().unwrap()
    );
}
