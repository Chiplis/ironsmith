use super::*;
const TEXT: &str = "Enchant player\nWhenever enchanted player casts a spell other than the first spell they cast each turn or copies a spell, this Aura deals 2 damage to them.";
#[test]
fn enchanted_repeat_cast_copy_triggers_after_first_cast_and_on_every_copy() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Curse of Shaken Faith")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura, Subtype::Curse])
            .parse_text(TEXT)
            .unwrap();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let carol = game.players[2].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.object_mut(source).unwrap().attached_to =
        Some(crate::object::AttachmentTarget::Player(bob));
    game.player_mut(bob).unwrap().attachments.push(source);
    game.refresh_continuous_state();
    let spell_card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Event Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Generic(1),
        ]))
        .build();
    let mut expected_life = 20;
    for turn in 0..2 {
        if turn != 0 {
            game.next_turn();
        }
        for (player, copied, expected) in [
            (bob, true, true),
            (alice, true, false),
            (bob, false, false),
            (carol, false, false),
            (bob, false, true),
            (bob, false, true),
            (carol, false, false),
            (bob, true, true),
        ] {
            let spell_owner = if copied { carol } else { player };
            let spell = game.create_object_from_card(&spell_card, spell_owner, Zone::Stack);
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
            let event = if copied {
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCopiedEvent::new(spell, player),
                    crate::provenance::ProvNodeId::default(),
                )
            } else {
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCastEvent::new_with_snapshot(
                        spell,
                        player,
                        Zone::Hand,
                        snapshot.clone(),
                    ),
                    crate::provenance::ProvNodeId::default(),
                )
            };
            game.turn_store
                .turn_history
                .record_event(&event, Some(snapshot), None);
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(
                triggers.len(),
                usize::from(expected),
                "turn={turn} player={player:?} copied={copied}"
            );
            if expected {
                let mut queue = crate::triggers::TriggerQueue::new();
                for trigger in triggers {
                    queue.add(trigger);
                }
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                expected_life -= 2;
            }
            assert_eq!(game.player(bob).unwrap().life, expected_life);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(carol).unwrap().life, 20);
        }
    }
}
#[test]
fn enchanted_repeat_cast_copy_renders_the_shared_actor_and_nonfirst_scope() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Curse of Shaken Faith")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura, Subtype::Curse])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn enchanted_repeat_cast_copy_keeps_an_authored_that_player_recipient() {
    let text = TEXT.replace("to them", "to that player");
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Curse of Shaken Faith")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura, Subtype::Curse])
            .parse_text(&text)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}
