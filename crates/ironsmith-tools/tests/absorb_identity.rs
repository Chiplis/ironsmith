//! Frozen atlas observation 25095041: collective optional copying after bounce.
#[test]
fn absorb_identity_returns_to_owner_and_copies_all_or_none_from_battlefield() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::game_state::Target;
    use ironsmith::ids::CardId;
    use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};
    struct Pick {
        target: ObjectId,
        accept: bool,
        prompts: usize,
    }
    impl DecisionMaker for Pick {
        fn decide_targets(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            assert_eq!(ctx.requirements.len(), 1);
            assert!(
                ctx.requirements[0]
                    .legal_targets
                    .contains(&Target::Object(self.target))
            );
            vec![Target::Object(self.target)]
        }
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            self.prompts += 1;
            self.accept
        }
    }
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Absorb Identity",
    )
    .unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let donor = CardDefinitionBuilder::new(CardId::new(), "Copy donor fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(4, 6))
        .with_ability(ironsmith::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::flying(),
        ))
        .build();
    let shape = CardDefinitionBuilder::new(CardId::new(), "Shapeshifter fixture")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![ironsmith::Subtype::Shapeshifter])
        .power_toughness(ironsmith::card::PowerToughness::fixed(1, 2))
        .build();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for accept in [false, true] {
        for change in 0..4 {
            for copied in [false, true] {
                let invalid = change == 1 || change == 2;
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = alice;
                game.turn.priority_player = Some(alice);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                let target = game.create_object_from_definition(&donor, bob, Zone::Battlefield);
                game.set_current_controller(target, alice);
                let stable = game.object(target).unwrap().stable_id;
                use ironsmith::effects::EffectExecutor;
                let mut setup_dm = ironsmith::decision::SelectFirstDecisionMaker;
                if copied {
                    let copied_definition =
                        CardDefinitionBuilder::new(CardId::new(), "Prior copy fixture")
                            .card_types(vec![CardType::Creature])
                            .power_toughness(ironsmith::card::PowerToughness::fixed(7, 8))
                            .build();
                    let template = game.create_object_from_definition(
                        &copied_definition,
                        bob,
                        Zone::Battlefield,
                    );
                    ironsmith::effects::ApplyContinuousEffect::new_runtime(
                        ironsmith::continuous::EffectTarget::Specific(target),
                        ironsmith::effects::RuntimeModification::CopyOf {
                            source: ironsmith::target::ChooseSpec::SpecificObject(template),
                            preserve_source_abilities: false,
                            name_override: None,
                            name_override_surface: None,
                            add_supertypes: vec![],
                            copy_exception_surface: None,
                        },
                        ironsmith::effect::Until::EndOfTurn,
                    )
                    .execute(
                        &mut game,
                        &mut ironsmith::effects::EffectContext::new(target, alice, &mut setup_dm),
                    )
                    .unwrap();
                }
                let expected_power = if copied { 7 } else { 4 };
                let expected_toughness = if copied { 8 } else { 6 };
                ironsmith::effects::ApplyContinuousEffect::new(
                    ironsmith::continuous::EffectTarget::Specific(target),
                    ironsmith::continuous::Modification::ModifyPowerToughness {
                        power: 9,
                        toughness: 9,
                    },
                    ironsmith::effect::Until::EndOfTurn,
                )
                .execute(
                    &mut game,
                    &mut ironsmith::effects::EffectContext::new(target, alice, &mut setup_dm),
                )
                .unwrap();
                game.object_mut(target)
                    .unwrap()
                    .add_counters(ironsmith::object::CounterType::PlusOnePlusOne, 2);
                game.tap(target);
                assert_eq!(game.calculated_power(target), Some(expected_power + 11));
                let first = game.create_object_from_definition(&shape, alice, Zone::Battlefield);
                let second = game.create_object_from_definition(&shape, alice, Zone::Battlefield);
                let opponent = game.create_object_from_definition(&shape, bob, Zone::Battlefield);
                let unrelated =
                    game.create_object_from_definition(&donor, alice, Zone::Battlefield);
                let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
                game.player_mut(alice)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Blue, 2);
                let mut dm = Pick {
                    target,
                    accept,
                    prompts: 0,
                };
                let action = compute_legal_actions(&game, alice)
                    .into_iter()
                    .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==spell))
                    .unwrap();
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut state = PriorityLoopState::new(game.players_in_game());
                let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
                    &mut game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::PriorityAction(action),
                    &mut dm,
                )
                .unwrap();
                for _ in 0..24 {
                    if !game.stack.is_empty() {
                        break;
                    }
                    let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                        panic!("{progress:?}")
                    };
                    progress = ironsmith::game_loop::apply_decision_context_with_dm(
                        &mut game, &mut queue, &mut state, &ctx, &mut dm,
                    )
                    .unwrap();
                }
                assert_eq!(game.stack.len(), 1);
                if invalid {
                    let exiled = game.move_object_by_effect(target, Zone::Exile).unwrap();
                    if change == 2 {
                        game.move_object_with_etb_processing(exiled, Zone::Battlefield)
                            .unwrap();
                    }
                }
                if change == 3 {
                    game.set_current_controller(target, bob);
                }
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let current = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(
                    game.object(current).unwrap().zone,
                    if change == 1 {
                        Zone::Exile
                    } else if change == 2 {
                        Zone::Battlefield
                    } else {
                        Zone::Hand
                    }
                );
                if !invalid {
                    assert!(game.player(bob).unwrap().hand.contains(&current));
                }
                assert_eq!(
                    dm.prompts,
                    if invalid { 0 } else { 1 },
                    "one collective optional choice"
                );
                for id in [first, second] {
                    assert_eq!(
                        game.calculated_power(id),
                        Some(if accept && !invalid {
                            expected_power
                        } else {
                            1
                        })
                    );
                    assert_eq!(
                        game.calculated_toughness(id),
                        Some(if accept && !invalid {
                            expected_toughness
                        } else {
                            2
                        })
                    );
                    assert!(!game.is_tapped(id), "copy does not copy tapped status");
                    assert_eq!(
                        game.current_has_static_ability_id(
                            id,
                            ironsmith::static_abilities::StaticAbilityId::Flying
                        ),
                        accept && !invalid && !copied
                    );
                    assert_eq!(
                        game.current_characteristics(id).unwrap().name.as_ref(),
                        if accept && !invalid {
                            if copied {
                                "Prior copy fixture"
                            } else {
                                "Copy donor fixture"
                            }
                        } else {
                            "Shapeshifter fixture"
                        }
                    );
                }
                assert_eq!(game.calculated_power(opponent), Some(1));
                assert_eq!(game.calculated_power(unrelated), Some(4));
                let newcomer = game.create_object_from_definition(&shape, alice, Zone::Battlefield);
                assert_eq!(
                    game.calculated_power(newcomer),
                    Some(1),
                    "copy set is fixed at resolution"
                );
                if accept && !invalid {
                    game.set_current_controller(first, bob);
                    assert_eq!(
                        game.calculated_power(first),
                        Some(expected_power),
                        "a controller change does not change the locked copy set"
                    );
                }
                game.effect_store.continuous_effects.cleanup_end_of_turn();
                game.next_turn();
                assert_eq!(game.calculated_power(first), Some(1));
                assert_eq!(game.calculated_power(second), Some(1));
            }
        }
    }
}

#[test]
fn absorb_identity_preserves_targeting_and_collective_copy_structure() {
    use ironsmith::effects::{
        ApplyContinuousEffect, MayEffect, ReturnToHandEffect, RuntimeModification, TaggedEffect,
    };
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Absorb Identity",
    )
    .unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    assert!(definition.canonical_text.contains("You may have all Shapeshifters you control become copies of that creature until end of turn"),"{}",definition.canonical_text);
    let program = definition.spell_effect.as_ref().unwrap();
    assert_eq!(program.segments.len(), 2);
    let bounce = program.segments[0].default_effects[0]
        .downcast_ref::<TaggedEffect>()
        .unwrap();
    let returned = bounce.effect.downcast_ref::<ReturnToHandEffect>().unwrap();
    assert!(returned.spec.is_target());
    assert_eq!(returned.spec.count().min, 1);
    assert_eq!(returned.spec.count().max, Some(1));
    let may = program.segments[1].default_effects[0]
        .downcast_ref::<MayEffect>()
        .unwrap();
    assert_eq!(may.effects.len(), 1);
    let tagged = may.effects[0].downcast_ref::<TaggedEffect>().unwrap();
    let copy = tagged
        .effect
        .downcast_ref::<ApplyContinuousEffect>()
        .unwrap();
    let Some(ironsmith::target::ChooseSpec::All(filter)) = &copy.target_spec else {
        panic!("copy must enumerate its non-targeted set")
    };
    assert_eq!(filter.zone, Some(ironsmith::Zone::Battlefield));
    assert_eq!(
        filter.controller,
        Some(ironsmith::target::PlayerFilter::You)
    );
    assert_eq!(filter.subtypes, vec![ironsmith::Subtype::Shapeshifter]);
    assert!(
        filter.card_types.is_empty(),
        "Shapeshifters need not be creatures"
    );
    assert!(copy.lock_filter_at_resolution);
    assert_eq!(copy.until, ironsmith::effect::Until::EndOfTurn);
    let [RuntimeModification::CopyOf { source, .. }] = copy.runtime_modifications.as_slice() else {
        panic!("expected typed copy")
    };
    assert!(!source.is_target());
    let ironsmith::target::ChooseSpec::Object(reference) = source.base() else {
        panic!("expected departed object reference")
    };
    assert_eq!(reference.zone, Some(ironsmith::Zone::Battlefield));
    assert_eq!(reference.tagged_constraints.len(), 1);
    assert_eq!(reference.tagged_constraints[0].tag, bounce.tag);
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled
    );
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
}
