use super::*;
const ORACLE: &str = "Lifelink\nWhenever another nontoken creature you control dies, exile Liliana, Heretical Healer, then return her to the battlefield transformed under her owner's control. If you do, create a 2/2 black Zombie creature token.";

#[test]
fn named_self_exile_transform_returns_to_owner_and_gates_token() {
    let front_id = crate::ids::CardId::new();
    let back_id = crate::ids::CardId::new();
    let mut front = crate::CardDefinitionBuilder::new(front_id, "Liliana, Heretical Healer")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 3))
        .parse_text(ORACLE)
        .unwrap();
    let mut back = crate::CardDefinitionBuilder::new(back_id, "Liliana, Defiant Necromancer")
        .card_types(vec![CardType::Planeswalker])
        .build();
    front.card.other_face = Some(back_id);
    front.card.other_face_name = Some(back.card.name.clone());
    front.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
    back.card.other_face = Some(front_id);
    back.card.other_face_name = Some(front.card.name.clone());
    back.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
    for source_leaves in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        game.create_object_from_definition(&back, alice, Zone::Exile);
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        game.set_current_controller(source, bob);
        let dying = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Dying creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build();
        let dying = game.create_object_from_card(&dying, bob, Zone::Battlefield);
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(dying).unwrap(), &game);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(
                dying,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let ability = &triggers[0].ability;
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
            &game,
            &ability.effects,
            bob,
            Some(source),
            None,
        );
        assert!(
            requirements.is_empty(),
            "self-exile does not select a named card"
        );
        if source_leaves {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        }
        let mut ctx =
            crate::effects::EffectContext::new_default(source, bob).with_triggering_event(event);
        for segment in &ability.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        let returned = game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().name == "Liliana, Defiant Necromancer");
        let zombie = game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().kind == crate::object::ObjectKind::Token);
        assert_eq!(returned.is_some(), !source_leaves);
        assert_eq!(zombie.is_some(), !source_leaves);
        if let Some(id) = returned {
            assert_eq!(game.controller_of_id(*id), Some(alice));
        }
        if let Some(id) = zombie {
            assert_eq!(game.controller_of_id(*id), Some(bob));
            assert_eq!(game.current_power(*id), Some(2));
            assert_eq!(game.current_toughness(*id), Some(2));
        }
    }
}

#[test]
fn named_self_exile_transform_keeps_full_name_and_return_pronouns() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Liliana, Heretical Healer")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .parse_text(ORACLE)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        ORACLE
    );
}
