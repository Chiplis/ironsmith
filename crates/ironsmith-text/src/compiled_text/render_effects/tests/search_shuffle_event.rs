use super::*;

#[test]
fn multi_zone_search_shuffle_depends_on_instruction_and_search_event_not_found_zone() {
    for conditional in [false, true] {
        let text = if conditional {
            "Search your library and graveyard for a card named Nissa, Nature's Artisan, reveal it, and put it into your hand. If you search your library this way, shuffle."
        } else {
            "Search your library and graveyard for a card named Nissa, Nature's Artisan, reveal it, put it into your hand, then shuffle."
        };
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Search Probe")
            .card_types(vec![CardType::Sorcery]).parse_text(text).unwrap();
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("Nissa, Nature's Artisan"), "{rendered}");
        for blocked in [false, true] {
            for found_zone in [None, Some(Zone::Library), Some(Zone::Graveyard)] {
                let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
                if let Some(zone) = found_zone {
                    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Nissa, Nature's Artisan")
                        .card_types(vec![CardType::Creature]).build();
                    game.create_object_from_card(&card, alice, zone);
                }
                let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                if blocked {
                    crate::effects::execute_effect(&mut game,
                        &Effect::cant_until(crate::effect::Restriction::SearchLibraries(PlayerFilter::You), Until::EndOfTurn),
                        &mut ctx).unwrap();
                    assert!(!game.can_search_library(alice));
                }
                let before = game.irreversible_random_count();
                for segment in &definition.spell_effect.as_ref().unwrap().segments {
                    for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
                }
                let found = found_zone.is_some() && !(blocked && found_zone == Some(Zone::Library));
                assert_eq!(game.player(alice).unwrap().hand.len(), usize::from(found), "{text}; blocked={blocked}; found={found_zone:?}");
                assert_eq!(game.irreversible_random_count() - before, u64::from(!conditional || !blocked),
                    "{text}; blocked={blocked}; found={found_zone:?}");
            }
        }
    }
}

#[test]
fn combined_source_and_graveyard_shuffle_moves_all_cards_before_one_shuffle_per_owner() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Combined Shuffle Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("{2}, {T}: You gain 5 life. Shuffle this artifact and your graveyard into their owner's library.")
        .unwrap();
    for other_owner in [false, true] {
        for empty_graveyard in [false, true] {
            let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let owner = if other_owner { bob } else { alice };
            let source = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
            let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Graveyard Card").card_types(vec![CardType::Creature]).build();
            if !empty_graveyard { game.create_object_from_card(&card, alice, Zone::Graveyard); }
            game.create_object_from_card(&card, bob, Zone::Graveyard);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            let AbilityKind::Activated(activated) = &definition.abilities[0].kind else { panic!("activated ability"); };
            let mut shuffled = Vec::new();
            for segment in &activated.effects.segments {
                for effect in &segment.default_effects {
                    let outcome = crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    for event in &outcome.events {
                        if let Some(shuffle) = event.downcast::<crate::events::ShuffleLibraryEvent>() {
                            shuffled.push(shuffle.player);
                            assert!(game.object(source).is_none(), "source must move before shuffling");
                            assert!(game.player(alice).unwrap().graveyard.is_empty(), "graveyard must move before shuffling");
                        }
                    }
                }
            }
            let mut expected = vec![alice];
            if other_owner { expected.push(bob); }
            shuffled.sort(); expected.sort();
            assert_eq!(shuffled, expected, "other_owner={other_owner}; empty={empty_graveyard}");
            assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
            assert_eq!(game.player(alice).unwrap().life, 25);
        }
    }
}
