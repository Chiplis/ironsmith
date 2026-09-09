use super::*;

#[test]
fn max_speed_bonus_tracks_its_controller_and_only_its_source() {
    for subject in ["This creature", "Speed Probe"] {
        let oracle = format!("Start your engines!\nMax speed — {subject} gets +1/+2.");
        let mut definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Speed Probe")
            .card_types(vec![CardType::Creature]).parse_text(&oracle).unwrap();
        definition.card.power_toughness = Some(crate::card::PowerToughness::fixed(2, 1));
        assert!(definition.spell_effect.is_none());
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let other = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 3)).build();
        let friend = game.create_object_from_card(&other, alice, Zone::Battlefield);
        let enemy = game.create_object_from_card(&other, bob, Zone::Battlefield);
        for (own_speed, opponent_speed) in [(0, 4), (1, 4), (3, 4), (4, 0), (4, 4), (2, 4)] {
            game.player_mut(alice).unwrap().speed = Some(own_speed);
            game.player_mut(bob).unwrap().speed = Some(opponent_speed);
            game.refresh_continuous_state();
            let bonus = if own_speed >= 4 { 1 } else { 0 };
            assert_eq!(game.current_power(source), Some(2 + bonus), "{subject}: speed {own_speed}");
            assert_eq!(game.current_toughness(source), Some(1 + 2 * bonus));
            for recipient in [friend, enemy] {
                assert_eq!(game.current_power(recipient), Some(2));
                assert_eq!(game.current_toughness(recipient), Some(3));
            }
        }
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            "Start your engines!\nMax speed — This creature gets +1/+2.");
    }
}
