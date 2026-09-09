use super::*;

const TEXT: &str = "This artifact enters tapped.\n{2}, {T}, Sacrifice this artifact: You take the initiative, gain 3 life, draw a card, and create a Treasure token. Activate only as a sorcery.";
fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Dungeoneer's Pack")
        .card_types(vec![CardType::Artifact])
        .parse_text(TEXT)
        .unwrap()
}

#[test]
fn initiative_action_list_executes_each_action_once_after_the_source_leaves() {
    let definition = definition();
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .unwrap();
    for previous_holder in [None, Some(0), Some(1)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        game.set_initiative(previous_holder.map(|index| game.players[index].id));
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Drawn Card")
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
        game.create_object_from_card(&card, alice, Zone::Library);
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_source_snapshot(snapshot);
        for effect in &activated.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.initiative, Some(alice));
        assert_eq!(game.player(alice).unwrap().life, 23);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
        assert_eq!(game.player(alice).unwrap().library.len(), 1);
        assert!(game.player(bob).unwrap().hand.is_empty());
        assert_eq!(game.battlefield.len(), 1);
        let treasure = game.battlefield[0];
        assert_eq!(game.controller_of_id(treasure), Some(alice));
        assert!(game.current_has_subtype(treasure, Subtype::Treasure));
    }
}

#[test]
fn initiative_action_list_renders_one_shared_subject() {
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition()).join("\n"),
        TEXT
    );
}
