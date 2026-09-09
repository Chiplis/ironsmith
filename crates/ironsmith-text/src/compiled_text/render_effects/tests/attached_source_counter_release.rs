use super::*;

const TEXT: &str = "Enchant creature\nThis Aura enters with four task counters on it.\nEnchanted creature can't attack or block. It loses all abilities and has \"{T}: Remove a task counter from Heliod's Punishment. Then if it has no task counters on it, destroy Heliod's Punishment.\"";

#[test]
fn attached_source_counter_release_removes_counters_from_the_granting_aura() {
    for name in ["Heliod's Punishment", "Binding Hourglass"] {
        for holder_counters in [0, 2] {
            let oracle = TEXT.replace("Heliod's Punishment", name);
            let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Aura])
                .parse_text(&oracle)
                .unwrap();
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let aura = game.create_object_from_definition(&definition, alice, Zone::Hand);
            let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Captive")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(3, 5))
                .with_ability(Ability::static_ability(
                    crate::static_abilities::StaticAbility::flying(),
                ))
                .build();
            let captive = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
            let attacker = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
            let task = crate::CounterType::Named("task".into());
            let aura = game
                .move_object_with_etb_processing(aura, Zone::Battlefield)
                .unwrap()
                .new_id;
            assert_eq!(
                game.object(aura).unwrap().counters.get(&task).copied(),
                Some(4)
            );
            if holder_counters > 0 {
                game.add_counters(captive, task, holder_counters);
            }
            assert!(
                game.attach_object_to_target(
                    aura,
                    crate::object::AttachmentTarget::Object(captive)
                )
            );
            game.refresh_continuous_state();
            game.update_cant_effects();
            assert!(!game.current_has_static_ability_id(
                captive,
                crate::static_abilities::StaticAbilityId::Flying
            ));
            assert!(!game.can_block_attacker(captive, attacker));
            for remaining in (0..4).rev() {
                let abilities = game.current_abilities(captive).unwrap();
                let (index, activated) = abilities
                    .iter()
                    .enumerate()
                    .find_map(|(i, a)| match &a.kind {
                        AbilityKind::Activated(activated) => Some((i, activated.clone())),
                        _ => None,
                    })
                    .expect("enchanted creature must retain its granted release ability");
                game.push_to_stack(
                    crate::game_state::StackEntry::ability(captive, bob, activated.effects)
                        .with_ability_index(index),
                );
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(
                    game.object(captive)
                        .unwrap()
                        .counters
                        .get(&task)
                        .copied()
                        .unwrap_or(0),
                    holder_counters,
                    "creature counters must remain untouched"
                );
                if remaining > 0 {
                    assert_eq!(
                        game.object(aura).unwrap().counters.get(&task).copied(),
                        Some(remaining)
                    );
                } else {
                    assert!(!game.battlefield.contains(&aura));
                    assert!(
                        game.player(alice)
                            .unwrap()
                            .graveyard
                            .iter()
                            .any(|id| game.object(*id).is_some_and(|o| o.name == name))
                    );
                }
            }
            game.refresh_continuous_state();
            game.update_cant_effects();
            assert!(game.current_has_static_ability_id(
                captive,
                crate::static_abilities::StaticAbilityId::Flying
            ));
            assert!(game.can_block_attacker(captive, attacker));
        }
    }
}

#[test]
fn attached_source_counter_release_retains_named_grant_and_shared_predicates() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Heliod's Punishment")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
