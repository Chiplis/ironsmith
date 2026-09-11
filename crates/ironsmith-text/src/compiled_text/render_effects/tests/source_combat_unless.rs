use super::*;

#[test]
fn source_attack_block_restrictions_track_hand_and_blessing_conditions() {
    for requirement in [
        "you have the city's blessing",
        "you have one or fewer cards in hand",
        "you have seven or more cards in hand",
    ] {
        let oracle = format!("This creature can't attack or block unless {requirement}.");
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Conditional Combat Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(&oracle)
        .unwrap();
        assert!(
            definition.spell_effect.is_none(),
            "intrinsic restriction became spell effect: {requirement}"
        );
        for count in [0, 1, 2, 6, 7, 8] {
            for blessed in [false, true] {
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other")
                    .card_types(vec![CardType::Creature])
                    .build();
                let friend = game.create_object_from_card(&card, alice, Zone::Battlefield);
                let enemy = game.create_object_from_card(&card, bob, Zone::Battlefield);
                for _ in 0..count {
                    game.create_object_from_card(&card, alice, Zone::Hand);
                }
                game.grant_citys_blessing(bob);
                if blessed {
                    game.grant_citys_blessing(alice);
                }
                game.refresh_continuous_state();
                let allowed = match requirement {
                    "you have the city's blessing" => blessed,
                    "you have one or fewer cards in hand" => count <= 1,
                    _ => count >= 7,
                };
                assert_eq!(
                    game.can_attack(source),
                    allowed,
                    "{requirement} count={count} blessed={blessed}"
                );
                assert_eq!(game.can_block_attacker(source, enemy), allowed);
                assert!(game.can_attack(friend));
                assert!(game.can_block_attacker(friend, enemy));
            }
        }
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [oracle]
        );
    }
}
