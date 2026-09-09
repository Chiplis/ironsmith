use super::*;
const TEXT: &str = "When Nyla enters, exile up to one target creature card from your graveyard. If you do, you lose X life and create X Clue tokens, where X is that card's mana value.\nAt the beginning of your end step, if you control no Clues, return target card exiled with Nyla to its owner's hand.";

#[test]
fn linked_exile_clues_count_mana_value_and_gate_return() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Nyla, Shirshu Sleuth")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    let abilities: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|ability| {
            if let AbilityKind::Triggered(triggered) = &ability.kind {
                Some(triggered)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(abilities.len(), 2);
    for mana_value in [0, 3] {
        for chosen in [false, true] {
            for return_case in 0..4 {
                let block_return = return_case == 1;
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let card =
                    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Exile candidate")
                        .card_types(vec![CardType::Creature])
                        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                            crate::mana::ManaSymbol::Generic(mana_value),
                        ]]))
                        .build();
                let victim = game.create_object_from_card(&card, alice, Zone::Graveyard);
                let opposing = game.create_object_from_card(&card, bob, Zone::Graveyard);
                let unrelated = game.create_object_from_card(&card, alice, Zone::Exile);
                let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                    .with_targets(if chosen {
                        vec![crate::effects::ResolvedTarget::Object(victim)]
                    } else {
                        vec![]
                    });
                ctx.snapshot_targets(&game);
                for effect in &abilities[0].effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
                let count = if chosen { mana_value as usize } else { 0 };
                assert_eq!(game.player(alice).unwrap().life, 20 - count as i32);
                let clues: Vec<_> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| {
                        game.object(*id).is_some_and(|object| {
                            object.subtypes.contains(&crate::types::Subtype::Clue)
                        })
                    })
                    .collect();
                assert_eq!(clues.len(), count);
                assert_eq!(
                    game.get_exiled_with_source_links(source).len(),
                    usize::from(chosen)
                );
                assert_eq!(game.object(opposing).unwrap().zone, Zone::Graveyard);
                for clue in clues {
                    game.move_object_by_effect(clue, Zone::Graveyard).unwrap();
                }
                let clue = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Clue")
                    .card_types(vec![CardType::Artifact])
                    .subtypes(vec![crate::types::Subtype::Clue])
                    .build();
                game.create_object_from_card(&clue, bob, Zone::Battlefield);
                if block_return {
                    game.create_object_from_card(&clue, alice, Zone::Battlefield);
                }
                if chosen {
                    let event = crate::triggers::TriggerEvent::new_with_provenance(
                        crate::events::phase::BeginningOfEndStepEvent::new(alice),
                        crate::provenance::ProvNodeId::default(),
                    );
                    let mut queue = crate::triggers::TriggerQueue::new();
                    for trigger in crate::triggers::check_triggers(&game, &event) {
                        queue.add(trigger);
                    }
                    assert_eq!(
                        queue.entries.len(),
                        usize::from(!block_return),
                        "intervening trigger gate"
                    );
                    let mut decisions = crate::decision::SelectFirstDecisionMaker;
                    crate::game_loop::put_triggers_on_stack_with_dm(
                        &mut game,
                        &mut queue,
                        &mut decisions,
                    )
                    .unwrap();
                    assert_eq!(
                        game.stack.len(),
                        usize::from(!block_return),
                        "linked target must be announceable: choices={:?}",
                        abilities[1].choices
                    );
                    if !block_return {
                        let linked = game.get_exiled_with_source_links(source)[0];
                        assert_eq!(
                            game.stack[0].targets,
                            vec![crate::game_state::Target::Object(linked)]
                        );
                        if return_case == 2 {
                            game.create_object_from_card(&clue, alice, Zone::Battlefield);
                        }
                        if return_case == 3 {
                            let moved =
                                game.move_object_by_effect(linked, Zone::Graveyard).unwrap();
                            game.move_object_by_effect(moved, Zone::Exile).unwrap();
                        }
                        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                    }
                    assert_eq!(
                        game.player(alice).unwrap().hand.len(),
                        usize::from(return_case == 0),
                        "mana={mana_value} chosen={chosen} return_case={return_case}"
                    );
                }
                assert_eq!(game.object(unrelated).unwrap().zone, Zone::Exile);
            }
        }
    }
}

#[test]
fn linked_exile_clue_text_preserves_shared_actor_and_zero_count() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Nyla, Shirshu Sleuth")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert_eq!(
        rendered,
        TEXT.replace("exiled with Nyla", "exiled with this creature")
    );
}
