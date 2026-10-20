use super::*;

#[test]
fn multi_zone_search_shuffle_depends_on_instruction_and_search_event_not_found_zone() {
    for conditional in [false, true] {
        let text = if conditional {
            "Search your library and graveyard for a card named Nissa, Nature's Artisan, reveal it, and put it into your hand. If you search your library this way, shuffle."
        } else {
            "Search your library and graveyard for a card named Nissa, Nature's Artisan, reveal it, put it into your hand, then shuffle."
        };
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Search Probe")
                .card_types(vec![CardType::Sorcery])
                .parse_text(text)
                .unwrap();
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("Nissa, Nature's Artisan"), "{rendered}");
        for blocked in [false, true] {
            for found_zone in [None, Some(Zone::Library), Some(Zone::Graveyard)] {
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
                if let Some(zone) = found_zone {
                    let card = crate::card::CardBuilder::new(
                        crate::ids::CardId::new(),
                        "Nissa, Nature's Artisan",
                    )
                    .card_types(vec![CardType::Creature])
                    .build();
                    game.create_object_from_card(&card, alice, zone);
                }
                let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                if blocked {
                    crate::effects::execute_effect(
                        &mut game,
                        &Effect::cant_until(
                            crate::effect::Restriction::SearchLibraries(PlayerFilter::You),
                            Until::EndOfTurn,
                        ),
                        &mut ctx,
                    )
                    .unwrap();
                    assert!(!game.can_search_library(alice));
                }
                let before = game.irreversible_random_count();
                for segment in &definition.spell_effect.as_ref().unwrap().segments {
                    for effect in &segment.default_effects {
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                }
                let found = found_zone.is_some() && !(blocked && found_zone == Some(Zone::Library));
                assert_eq!(
                    game.player(alice).unwrap().hand.len(),
                    usize::from(found),
                    "{text}; blocked={blocked}; found={found_zone:?}"
                );
                assert_eq!(
                    game.irreversible_random_count() - before,
                    u64::from(!conditional || !blocked),
                    "{text}; blocked={blocked}; found={found_zone:?}"
                );
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
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let owner = if other_owner { bob } else { alice };
            let source = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
            let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Graveyard Card")
                .card_types(vec![CardType::Creature])
                .build();
            if !empty_graveyard {
                game.create_object_from_card(&card, alice, Zone::Graveyard);
            }
            game.create_object_from_card(&card, bob, Zone::Graveyard);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            let AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
                panic!("activated ability");
            };
            let mut shuffled = Vec::new();
            for segment in &activated.effects.segments {
                for effect in &segment.default_effects {
                    let outcome =
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    for event in &outcome.events {
                        if let Some(shuffle) =
                            event.downcast::<crate::events::ShuffleLibraryEvent>()
                        {
                            shuffled.push(shuffle.player);
                            assert!(
                                game.object(source).is_none(),
                                "source must move before shuffling"
                            );
                            assert!(
                                game.player(alice).unwrap().graveyard.is_empty(),
                                "graveyard must move before shuffling"
                            );
                        }
                    }
                }
            }
            let mut expected = vec![alice];
            if other_owner {
                expected.push(bob);
            }
            shuffled.sort();
            expected.sort();
            assert_eq!(
                shuffled, expected,
                "other_owner={other_owner}; empty={empty_graveyard}"
            );
            assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
            assert_eq!(game.player(alice).unwrap().life, 25);
        }
    }
}

#[test]
fn plural_library_search_reveals_the_same_found_collection() {
    for (text, kind) in [
        ("Search your library for up to two basic land cards and reveal them. Put one into your hand and the other into your graveyard. Then shuffle.", CardType::Sorcery),
        ("Search your library for up to two creature cards and reveal them. Put one into your hand and the other into your graveyard. Then shuffle.", CardType::Sorcery),
        ("Search your library for three creature cards and reveal them. If you reveal three cards with different names, choose one of them at random and put that card into your hand. Shuffle the rest into your library.", CardType::Instant),
        ("Search your library for up to four basic land cards and reveal them. Put one of them onto the battlefield tapped under an opponent's control if the {3}{G} cost was paid. Put two of them onto the battlefield tapped under your control and the rest into your hand. Then shuffle.", CardType::Sorcery),
        ("When this creature enters, search your library for up to two basic land cards and reveal them, then roll a d20.\n9 or less | Put those cards into your hand, then shuffle.\n10—19 | Put one of those cards onto the battlefield tapped and the other into your hand, then shuffle.\n20+ | Put those cards onto the battlefield tapped, then shuffle.", CardType::Creature),
    ] {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Plural search probe")
            .card_types(vec![kind])
            .parse_text(text)
            .expect("plural search should compile");
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join(" ");
        assert!(rendered.contains("reveal them"), "{rendered}");
        assert!(!rendered.contains("Reveal it"), "{rendered}");
    }
}

#[test]
fn singular_library_search_keeps_singular_reveal() {
    let text = "Search your library for a basic land card and reveal it. Put it into your hand, then shuffle.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Single search probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("single search should compile");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join(" ");
    assert!(rendered.contains("reveal it"), "{rendered}");
    assert!(!rendered.contains("reveal them"), "{rendered}");
}

#[test]
fn revealed_three_card_name_check_keeps_random_pick_inside_searched_set() {
    let text = "Search your library for three creature cards and reveal them. If you reveal three cards with different names, choose one of them at random and put that card into your hand. Shuffle the rest into your library.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Searched-card selection probe")
        .card_types(vec![CardType::Instant])
        .parse_text(text)
        .expect("revealed-card random pick should compile");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join(" ");
    assert_eq!(rendered.trim_end_matches('.'), text.trim_end_matches('.'));
    assert!(!rendered.contains("choose a permanent at random"), "{rendered}");
}

#[test]
fn searched_four_card_conditional_partition_keeps_every_destination() {
    let text = "You may pay {3}{G} rather than pay this spell's mana cost.\nSearch your library for up to four basic land cards and reveal them. Put one of them onto the battlefield tapped under an opponent's control if the {3}{G} cost was paid. Put two of them onto the battlefield tapped under your control and the rest into your hand. Then shuffle.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Conditional searched partition")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("searched four-card partition should compile");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert_eq!(rendered.trim_end_matches('.'), text.trim_end_matches('.'));
}

#[test]
fn searched_pair_d20_branches_keep_chosen_card_and_its_complement() {
    let text = "When this creature enters, search your library for up to two basic land cards and reveal them, then roll a d20.\n9 or less | Put those cards into your hand, then shuffle.\n10—19 | Put one of those cards onto the battlefield tapped and the other into your hand, then shuffle.\n20+ | Put those cards onto the battlefield tapped, then shuffle.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "D20 searched pair")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("d20 searched pair should compile");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("search your library for up to two basic land cards and reveal them, then roll a d20"), "{rendered}");
    assert!(rendered.contains("10—19 | Put one of those cards onto the battlefield tapped and the other into your hand, then shuffle"), "{rendered}");
    assert!(!rendered.contains("all card in your library"), "{rendered}");
}
