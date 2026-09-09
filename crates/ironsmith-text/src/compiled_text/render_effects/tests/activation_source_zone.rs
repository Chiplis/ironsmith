use super::*;
const TEXT: &str = "Whenever an opponent activates an ability of an artifact, creature, or land on the battlefield, if it isn't a mana ability, this creature deals 2 damage to that player.";
#[test]
fn activation_source_zone_matches_types_zone_activator_and_nonmana() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Harsh Mentor")
        .card_types(vec![CardType::Creature])
        .parse_text(TEXT)
        .unwrap();
    for kind in [
        CardType::Artifact,
        CardType::Creature,
        CardType::Land,
        CardType::Enchantment,
        CardType::Planeswalker,
    ] {
        for zone in [Zone::Battlefield, Zone::Hand, Zone::Graveyard, Zone::Exile] {
            for own in [false, true] {
                for mana in [false, true] {
                    let mut game =
                        crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let alice = game.players[0].id;
                    let bob = game.players[1].id;
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                    let card =
                        crate::card::CardBuilder::new(crate::ids::CardId::new(), "Ability Source")
                            .card_types(vec![kind])
                            .build();
                    let activator = if own { alice } else { bob };
                    let source = game.create_object_from_card(&card, activator, zone);
                    let event = crate::triggers::TriggerEvent::new_with_provenance(
                        crate::events::AbilityActivatedEvent::new(source, activator, mana),
                        crate::provenance::ProvNodeId::default(),
                    );
                    let triggers = crate::triggers::check_triggers(&game, &event);
                    let matches = matches!(
                        kind,
                        CardType::Artifact | CardType::Creature | CardType::Land
                    ) && zone == Zone::Battlefield
                        && !own
                        && !mana;
                    assert_eq!(
                        triggers.len(),
                        usize::from(matches),
                        "{kind:?} {zone:?} own={own} mana={mana}"
                    );
                    if matches {
                        let mut queue = crate::triggers::TriggerQueue::new();
                        for trigger in triggers {
                            queue.add(trigger);
                        }
                        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                        assert_eq!(game.players[0].life, 20);
                        assert_eq!(game.players[1].life, 18);
                    }
                }
            }
        }
    }
}
#[test]
fn activation_source_zone_renders_explicit_battlefield_and_nonmana_restrictions() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Harsh Mentor")
        .card_types(vec![CardType::Creature])
        .parse_text(TEXT)
        .unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("on the battlefield"), "{rendered}");
    assert!(
        rendered.contains("if it isn't a mana ability"),
        "{rendered}"
    );
}

#[test]
fn activation_source_zone_does_not_add_an_unauthored_location() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Activation Test")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever an opponent activates an ability of a creature that isn't a mana ability, this creature deals 1 damage to that player.").unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(!rendered.contains("on the battlefield"), "{rendered}");
}
