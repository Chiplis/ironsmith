use super::*;

const TEXT: &str = "Enchant creature\nThis Aura enters with four task counters on it.\nEnchanted creature can't attack or block. It loses all abilities and has \"{T}: Remove a task counter from Heliod's Punishment. Then if it has no task counters on it, destroy Heliod's Punishment.\"";

#[test]
fn attached_source_counter_release_removes_counters_from_the_granting_aura() {
    run_attached_source_counter_release(false);
}

#[test]
fn attached_source_counter_release_with_activation_granting_source_context() {
    run_attached_source_counter_release(true);
}

fn run_attached_source_counter_release(capture_granting_source: bool) {
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
                .map(require_plain_entry_for_test)
                .expect("entry execution must succeed in this scenario")
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
                let mut entry =
                    crate::game_state::StackEntry::ability(captive, bob, activated.effects)
                        .with_ability_index(index);
                if capture_granting_source {
                    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        game.object(aura).expect("granting Aura exists"), &game,
                    );
                    entry = entry.with_tagged_objects(std::collections::HashMap::from([(
                        crate::tag::TagKey::from(crate::tag::GRANTING_SOURCE_TAG),
                        vec![snapshot],
                    )]));
                }
                if capture_granting_source {
                    game.push_to_stack(entry);
                } else {
                    game.turn.active_player = bob;
                    game.turn.priority_player = Some(bob);
                    game.turn.phase = crate::game_state::Phase::FirstMain;
                    game.turn.step = None;
                    game.remove_summoning_sickness(captive);
                    game.untap(captive);
                    let action = crate::decision::compute_legal_actions(&game, bob).unwrap().into_iter()
                        .find(|action| matches!(action, crate::decision::LegalAction::ActivateAbility { source, ability_index, .. } if *source == captive && *ability_index == index))
                        .expect("granted release ability must be legally activatable");
                    let mut state =
                        crate::game_loop::PriorityLoopState::new(game.players_in_game());
                    let mut queue = crate::triggers::TriggerQueue::new();
                    let mut dm = crate::decision::SelectFirstDecisionMaker;
                    let mut progress = crate::game_loop::apply_priority_response_with_dm(
                        &mut game,
                        &mut queue,
                        &mut state,
                        &crate::game_loop::PriorityResponse::PriorityAction(action),
                        &mut dm,
                    )
                    .unwrap();
                    for _ in 0..15 {
                        if !game.stack.is_empty() {
                            break;
                        }
                        let crate::decision::GameProgress::NeedsDecisionCtx(choice) = progress
                        else {
                            panic!("activation stalled: {progress:?}");
                        };
                        progress = crate::game_loop::apply_decision_context_with_dm(
                            &mut game, &mut queue, &mut state, &choice, &mut dm,
                        )
                        .unwrap();
                    }
                    assert!(!game.stack.is_empty());
                }
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

// These fixtures expect a plain completed entry. Reject a continuation or
// retained added instructions rather than silently projecting them away.
fn require_plain_entry_for_test(
    receipt: crate::game_state::EntryCommitResult,
) -> Option<crate::game_state::EntersResult> {
    assert!(!receipt.pending, "fixture requires completed entry");
    assert!(
        receipt.programs.is_empty(),
        "fixture must finish retained entry replacement programs"
    );
    receipt.original.into_result()
}
