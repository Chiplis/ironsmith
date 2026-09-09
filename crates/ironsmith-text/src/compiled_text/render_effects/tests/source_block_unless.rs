use super::*;

#[test]
fn source_block_permission_tracks_other_creatures_and_their_controller() {
    for (clause, source_type, allowed_types) in [
        ("a Vampire", Subtype::Zombie, vec![Subtype::Vampire]),
        ("another Zombie", Subtype::Zombie, vec![Subtype::Zombie]),
        ("another Minotaur", Subtype::Minotaur, vec![Subtype::Minotaur]),
        ("another Wolf or Werewolf", Subtype::Wolf, vec![Subtype::Wolf, Subtype::Werewolf]),
    ] {
        let oracle = format!("This creature can't block unless you control {clause}.");
        let mut definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Block Probe")
            .card_types(vec![CardType::Creature]).parse_text(&oracle).unwrap();
        definition.card.subtypes = vec![source_type];
        assert!(definition.spell_effect.is_none(), "{clause}");
        for subtype in allowed_types.iter().copied().chain([Subtype::Human]) {
            let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other")
                .card_types(vec![CardType::Creature]).subtypes(vec![subtype]).build();
            let enemy = game.create_object_from_card(&card, bob, Zone::Battlefield);
            game.refresh_continuous_state();
            assert!(!game.can_block_attacker(source, enemy), "source alone: {clause}");
            assert!(game.can_attack(source));
            let friend = game.create_object_from_card(&card, alice, Zone::Battlefield);
            game.refresh_continuous_state();
            assert_eq!(game.can_block_attacker(source, enemy), allowed_types.contains(&subtype), "{clause}: {subtype:?}");
            assert!(game.can_block_attacker(friend, enemy));
            game.move_object_by_effect(friend, Zone::Graveyard);
            game.refresh_continuous_state();
            assert!(!game.can_block_attacker(source, enemy), "support removed: {clause}");
        }
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n").to_ascii_lowercase(), oracle.to_ascii_lowercase());
    }
}
