use super::*;
const TEXT: &str = "{1}, {T}, Sacrifice a creature: Put a soul counter on this land. Then if there are three or more soul counters on it, remove those counters, transform it, then untap it. Activate only as a sorcery.";

#[test]
fn counter_threshold_transform_untaps_only_after_threshold() {
    let front_id = crate::ids::CardId::new();
    let back_id = crate::ids::CardId::new();
    let mut front = crate::CardDefinitionBuilder::new(front_id, "Threshold Shelter")
        .card_types(vec![CardType::Land]).parse_text(TEXT).unwrap();
    let mut back = crate::CardDefinitionBuilder::new(back_id, "Threshold Creature")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(3,7)).build();
    front.card.other_face = Some(back_id);
    front.card.other_face_name = Some(back.card.name.clone());
    front.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
    back.card.other_face = Some(front_id);
    back.card.other_face_name = Some(front.card.name.clone());
    back.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
    for initial in [0, 1, 2, 4] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()],20);
        let alice = game.players[0].id;
        // Prime both linked definitions in the game-local cache.
        game.create_object_from_definition(&back, alice, Zone::Exile);
        let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        game.tap(source);
        game.add_counters(source, crate::object::CounterType::Named("soul".into()), initial);
        game.add_counters(source, crate::object::CounterType::Charge, 2);
        let crate::ability::AbilityKind::Activated(ability) = &front.abilities[0].kind else { panic!("activated"); };
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        for segment in &ability.effects.segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game,effect,&mut ctx).unwrap(); }
        }
        assert_eq!(game.transform_count(source), if initial >= 2 {1} else {0});
        assert_eq!(game.is_tapped(source),initial < 2);
        assert_eq!(game.counter_count(source,crate::object::CounterType::Named("soul".into())),if initial >= 2 {0} else {initial+1});
        assert_eq!(game.counter_count(source,crate::object::CounterType::Charge),2,"only remove the referenced counter kind");
        assert_eq!(game.object(source).unwrap().name,if initial >= 2 {"Threshold Creature"} else {"Threshold Shelter"});
    }
}

