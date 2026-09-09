use super::*;
const TEXT: &str = "Whenever you play a land from exile or cast a spell from exile, you gain 2 life.\nDraw Arcanum — {T}: Look at the top card of your library. You may exile it face down.\nPlay Arcanum — {T}: Until end of turn, you may play cards exiled with Urianger Augurelt. Spells you cast this way cost {2} less to cast.";
fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Urianger Augurelt")
        .card_types(vec![CardType::Creature])
        .parse_text(TEXT)
        .unwrap()
}
#[test]
fn linked_permission_discount_applies_only_to_its_own_casts() {
    use crate::alternative_cast::CastingMethod;
    let definition = definition();
    let activated = definition
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .last()
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other_source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let base = crate::mana::ManaCost::from_symbols(vec![
        crate::mana::ManaSymbol::Generic(4),
        crate::mana::ManaSymbol::Blue,
    ]);
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Same Name")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(base.clone())
        .build();
    let linked = game.create_object_from_card(&card, alice, Zone::Exile);
    let unlinked = game.create_object_from_card(&card, alice, Zone::Exile);
    game.add_exiled_with_source_link(source, linked);
    game.add_exiled_with_source_link(other_source, linked);
    let other_permission = crate::effects::GrantPlayTaggedEffect::new(
        TagKey::from("__source_exiled__"),
        PlayerFilter::You,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        true,
        false,
    );
    let mut other_ctx = crate::effects::EffectContext::new_default(other_source, alice);
    crate::effects::execute_effect(&mut game, &Effect::new(other_permission), &mut other_ctx)
        .unwrap();
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for effect in &activated.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    for (id, method, expected) in [
        (linked, CastingMethod::Normal, "{4}{U}"),
        (
            unlinked,
            CastingMethod::PlayFrom {
                source,
                zone: Zone::Exile,
                use_alternative: None,
            },
            "{4}{U}",
        ),
        (
            linked,
            CastingMethod::PlayFrom {
                source: other_source,
                zone: Zone::Exile,
                use_alternative: None,
            },
            "{4}{U}",
        ),
        (
            linked,
            CastingMethod::PlayFrom {
                source,
                zone: Zone::Exile,
                use_alternative: None,
            },
            "{2}{U}",
        ),
    ] {
        let actual = crate::decision::calculate_effective_mana_cost_for_casting_method(
            &game,
            alice,
            game.object(id).unwrap(),
            &base,
            &method,
        );
        assert_eq!(actual.to_oracle(), expected, "{method:?}");
    }
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(crate::mana::ManaSymbol::Colorless, 2);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(crate::mana::ManaSymbol::Blue, 1);
    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(!actions.iter().any(|a| matches!(a, crate::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == unlinked)));
    let action = actions.into_iter().find(|a| matches!(a, crate::decision::LegalAction::CastSpell { spell_id, casting_method: CastingMethod::PlayFrom { source: grant_source, .. }, .. } if *spell_id == linked && *grant_source == source)).expect("discount must make the linked spell affordable");
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
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
        let crate::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("cast stalled: {progress:?}");
        };
        progress = crate::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    assert!(game.stack.iter().any(|entry| !entry.is_ability));
    let stack_id = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    let exiled_again = game.move_object_by_effect(stack_id, Zone::Exile).unwrap();
    assert!(
        game.object(exiled_again)
            .unwrap()
            .cast_play_from_constraints
            .is_none()
    );
    assert_eq!(
        crate::decision::calculate_effective_mana_cost_for_casting_method(
            &game,
            alice,
            game.object(exiled_again).unwrap(),
            &base,
            &CastingMethod::PlayFrom {
                source,
                zone: Zone::Exile,
                use_alternative: None
            }
        ),
        base
    );
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
}

#[test]
fn linked_permission_discount_executor_keeps_pool_player_duration_and_colored_mana() {
    use crate::alternative_cast::CastingMethod;
    for discount in [
        vec![crate::mana::ManaSymbol::Generic(9)],
        vec![
            crate::mana::ManaSymbol::Generic(1),
            crate::mana::ManaSymbol::Blue,
        ],
    ] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
        let base = crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Generic(4),
            crate::mana::ManaSymbol::Blue,
        ]);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Pool Card")
            .card_types(vec![CardType::Instant])
            .mana_cost(base.clone())
            .build();
        let ids = [
            game.create_object_from_card(&card, alice, Zone::Exile),
            game.create_object_from_card(&card, alice, Zone::Exile),
            game.create_object_from_card(&card, alice, Zone::Exile),
        ];
        let snapshots = ids[..2]
            .iter()
            .map(|id| {
                crate::snapshot::ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)
            })
            .collect();
        let tag = TagKey::from("pool");
        let mut tags = std::collections::HashMap::new();
        tags.insert(tag.clone(), snapshots);
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_tagged_objects(tags);
        let effect = crate::effects::GrantPlayTaggedEffect::new(
            tag,
            PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            false,
        )
        .with_spell_cost_reduction(crate::mana::ManaCost::from_symbols(discount.clone()));
        crate::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let method = CastingMethod::PlayFrom {
            source,
            zone: Zone::Exile,
            use_alternative: None,
        };
        for (index, id) in ids.iter().enumerate() {
            for player in [alice, bob] {
                let expected = if index < 2 && player == alice {
                    if discount.len() == 1 { "{U}" } else { "{3}" }
                } else {
                    "{4}{U}"
                };
                assert_eq!(
                    crate::decision::calculate_effective_mana_cost_for_casting_method(
                        &game,
                        player,
                        game.object(*id).unwrap(),
                        &base,
                        &method
                    )
                    .to_oracle(),
                    expected
                );
            }
        }
        game.next_turn();
        for id in ids {
            assert_eq!(
                crate::decision::calculate_effective_mana_cost_for_casting_method(
                    &game,
                    alice,
                    game.object(id).unwrap(),
                    &base,
                    &method
                ),
                base
            );
        }
    }
}

#[test]
fn linked_permission_discount_renders_the_linked_pool_and_discount() {
    let rendered = crate::compiled_text::compiled_text_lines(&definition()).join("\n");
    assert!(
        rendered.contains("you may play cards exiled with"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Spells you cast this way cost {2} less to cast"),
        "{rendered}"
    );
    assert!(!rendered.contains("with that name"), "{rendered}");
}

#[test]
fn linked_permission_discount_parser_preserves_costs_and_does_not_capture_global_reductions() {
    for (noun, card_type, cost) in [
        ("artifact", CardType::Artifact, "{3}"),
        ("creature", CardType::Creature, "{1}{U}"),
    ] {
        for linked in [true, false] {
            let followup = if linked {
                format!("Spells you cast this way cost {cost} less to cast.")
            } else {
                format!("Spells you cast cost {cost} less to cast this turn.")
            };
            let text = format!(
                "{{T}}: Until end of turn, you may play cards exiled with this {noun}. {followup}"
            );
            let definition =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Permission Source")
                    .card_types(vec![card_type])
                    .parse_text(&text)
                    .unwrap();
            let activated = definition
                .abilities
                .iter()
                .find_map(|a| match &a.kind {
                    AbilityKind::Activated(a) => Some(a),
                    _ => None,
                })
                .unwrap();
            let grant = (&activated.effects)
                .into_iter()
                .find_map(|e| e.downcast_ref::<crate::effects::GrantPlayTaggedEffect>())
                .unwrap();
            assert_eq!(
                grant.spell_cost_reduction.as_ref().map(|c| c.to_oracle()),
                linked.then(|| cost.to_string())
            );
            let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
            assert!(
                rendered.contains(&format!("cards exiled with this {noun}")),
                "{rendered}"
            );
            if linked {
                assert!(rendered.contains(&followup), "{rendered}");
            }
        }
    }
}
