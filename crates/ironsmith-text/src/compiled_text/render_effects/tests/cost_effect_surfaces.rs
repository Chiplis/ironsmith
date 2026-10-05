use super::*;

fn render(text: &str, card_types: Vec<CardType>) -> String {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Typed Cost Surface Probe")
            .card_types(card_types)
            .parse_text(text)
            .expect("typed cost surface should compile");
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn activated_ability_cost_increase_uses_its_typed_sacrifice_cost() {
    let text = "Activated abilities of nontoken Rebels cost an additional \"Sacrifice a land\" to activate.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Typed Cost Surface Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(text)
            .expect("typed cost increase should compile");
    let AbilityKind::Static(ability) = &definition.abilities[0].kind else {
        panic!("cost increase should be static")
    };
    let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease { increase, .. } =
        &ability.compiled_model().expect("compiled model").payload
    else {
        panic!("typed activated-ability cost increase payload was lost")
    };
    assert_eq!(describe_total_cost(increase), "Sacrifice a land");
    assert_eq!(
        restore_modeled_value_surface(ability, ability.display()),
        text.trim_end_matches('.')
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn graveyard_cast_grant_uses_its_typed_sacrifice_cost() {
    let text = "Once during each of your turns, you may cast an instant or sorcery spell from your graveyard by sacrificing a creature in addition to paying its other costs. If a spell cast this way would be put into your graveyard, exile it instead.";
    assert_eq!(render(text, vec![CardType::Enchantment]), text);
}

#[test]
fn graveyard_cast_grant_uses_its_typed_choose_then_exile_cost() {
    let text = "You may cast this card from your graveyard by exiling four instant and/or sorcery cards from your graveyard in addition to paying its other costs.";
    let rendered = render(text, vec![CardType::Creature]);
    assert_eq!(rendered, text);
    assert!(!rendered.contains("Effect"), "{rendered}");
    assert!(rendered.contains("by exiling four "), "{rendered}");
    assert!(
        rendered.contains("cards from your graveyard in addition to paying its other costs"),
        "{rendered}"
    );
}

#[test]
fn granted_nonmana_ward_uses_the_typed_cost_inside_quotes() {
    let text = "Permanents you control have \"Ward—Sacrifice a permanent.\"";
    assert_eq!(render(text, vec![CardType::Creature]), text);
}

#[test]
fn blink_with_entry_counter_preserves_then_and_the_authored_reference() {
    for text in [
        "Exile target artifact or creature, then return it to the battlefield under its owner's control with a +1/+1 counter on it.",
        "Exile target artifact or creature, then return that card to the battlefield under its owner's control with a +1/+1 counter on it.",
    ] {
        assert_eq!(render(text, vec![CardType::Instant]), text);
    }
}

#[test]
fn prevention_followup_and_delayed_pact_payment_share_the_public_statement_route() {
    let text = "The next time a source of your choice would deal damage to you this turn, prevent that damage. You gain life equal to the damage prevented this way.\nAt the beginning of your next upkeep, pay {1}{W}{W}. If you don't, you lose the game.";
    // Both paragraphs resolve as one spell program; preserve every instruction.
    assert_eq!(
        render(text, vec![CardType::Instant]).replace("\n", " "),
        text.replace("\n", " ")
    );
}

#[test]
fn fixed_mana_output_keeps_its_typed_on_spend_copy_program() {
    let text = "{T}: Add {R}. When that mana is spent to cast a red instant or sorcery spell, copy that spell and you may choose new targets for the copy.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mana Spend Copy Probe")
            .card_types(vec![CardType::Artifact])
            .parse_text(text)
            .expect("typed mana-spend copy surface should compile");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn graveyard_cast_grant_rider_tracks_selected_method_across_stack_destinations() {
    use crate::grant::DerivedAlternativeCastRuntimeExt;
    let oracle = "Once during each of your turns, you may cast an instant or sorcery spell from your graveyard by sacrificing a creature in addition to paying its other costs. If a spell cast this way would be put into your graveyard, exile it instead.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Linked permission probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(oracle)
            .unwrap();
    assert_eq!(
        definition.abilities.len(),
        1,
        "the rider belongs to the permission, not a separate static replacement"
    );
    for selected in [false, true] {
        for destination in [Zone::Graveyard, Zone::Hand, Zone::Exile] {
            for source_present in [false, true] {
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let source = game.create_object_from_definition(
                    &definition,
                    alice,
                    if source_present {
                        Zone::Battlefield
                    } else {
                        Zone::Graveyard
                    },
                );
                let card = crate::card::CardBuilder::new(
                    crate::ids::CardId::new(),
                    "Linked permission spell",
                )
                .card_types(vec![CardType::Instant])
                .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                    crate::mana::ManaSymbol::Generic(1),
                ]]))
                .build();
                let spell = game.create_object_from_card(&card, alice, Zone::Stack);
                game.stack
                    .push(crate::game_state::StackEntry::new(spell, alice));
                let stable = game.object(spell).unwrap().stable_id;
                if selected {
                    let grant = definition
                        .abilities
                        .iter()
                        .find_map(|ability| match &ability.kind {
                            AbilityKind::Static(ability) => ability.grant_spec(),
                            _ => None,
                        })
                        .unwrap();
                    let crate::grant::Grantable::DerivedAlternativeCast(derived) = &grant.grantable
                    else {
                        panic!("derived permission");
                    };
                    let mut origin = game.object(spell).unwrap().clone();
                    origin.zone = Zone::Graveyard;
                    let method = derived.materialize_for(&origin).unwrap();
                    assert!(matches!(
                        method,
                        crate::alternative_cast::AlternativeCastingMethod::FromZone {
                            exiles_after_resolution: true,
                            ..
                        }
                    ));
                    game.object_mut(spell).unwrap().cast_alternative_method =
                        Some(Box::new(method));
                }
                let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                crate::effects::execute_effect(
                    &mut game,
                    &Effect::new(crate::effects::MoveToZoneEffect::new(
                        crate::target::ChooseSpec::SpecificObject(spell),
                        destination,
                        false,
                    )),
                    &mut ctx,
                )
                .unwrap();
                let actual = game
                    .object(game.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone;
                assert_eq!(
                    actual,
                    if selected && destination == Zone::Graveyard {
                        Zone::Exile
                    } else {
                        destination
                    },
                    "selected={selected}, destination={destination:?}, source_present={source_present}"
                );
            }
        }
    }
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

#[test]
fn linked_graveyard_permission_pays_cost_and_exiles_on_resolution_or_counter() {
    let oracle = "Once during each of your turns, you may cast an instant or sorcery spell from your graveyard by sacrificing a creature in addition to paying its other costs. If a spell cast this way would be put into your graveyard, exile it instead.";
    assert_linked_permission_cast_outcome(oracle);
}

#[test]
fn qualified_graveyard_permission_rider_exiles_after_actual_cast() {
    for subject in ["an instant or sorcery spell", "a spell"] {
        let oracle = format!(
            "Once during each of your turns, you may cast {subject} from your graveyard by sacrificing a creature in addition to paying its other costs. If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead."
        );
        for change_controller in [false, true] {
            assert_permission_cast_outcome(&oracle, CardType::Instant, true, change_controller);
        }
        assert_eq!(render(&oracle, vec![CardType::Enchantment]), oracle);
    }
}

#[test]
fn qualified_graveyard_permission_rider_excludes_creature_spells() {
    let oracle = "Once during each of your turns, you may cast a spell from your graveyard by sacrificing a creature in addition to paying its other costs. If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead.";
    for change_controller in [false, true] {
        assert_permission_cast_outcome(oracle, CardType::Creature, false, change_controller);
    }
}

fn assert_linked_permission_cast_outcome(oracle: &str) {
    assert_permission_cast_outcome(oracle, CardType::Instant, true, false);
}

#[test]
fn qualified_graveyard_permission_rider_does_not_apply_to_ordinary_casting() {
    let oracle = "Once during each of your turns, you may cast a spell from your graveyard by sacrificing a creature in addition to paying its other costs. If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead.";
    for change_controller in [false, true] {
        assert_permission_cast_from(
            oracle,
            CardType::Instant,
            false,
            change_controller,
            Zone::Hand,
        );
    }
}

fn assert_permission_cast_outcome(
    oracle: &str,
    spell_type: CardType,
    expect_exile: bool,
    change_controller: bool,
) {
    assert_permission_cast_from(
        oracle,
        spell_type,
        expect_exile,
        change_controller,
        Zone::Graveyard,
    );
}

#[test]
fn equal_cost_graveyard_permissions_keep_the_selected_rider_identity() {
    let plain = "Once during each of your turns, you may cast a spell from your graveyard by sacrificing a creature in addition to paying its other costs.";
    let with_rider = format!(
        "{plain} If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead."
    );
    for rider_first in [false, true] {
        let oracle = if rider_first {
            format!("{with_rider}\n{plain}")
        } else {
            format!("{plain}\n{with_rider}")
        };
        for selected_index in [0, 1] {
            let expect_exile = (selected_index == 0) == rider_first;
            assert_permission_cast_from_selected(
                &oracle,
                CardType::Instant,
                expect_exile,
                false,
                Zone::Graveyard,
                Some(selected_index),
            );
        }
    }
}

fn assert_permission_cast_from(
    oracle: &str,
    spell_type: CardType,
    expect_exile: bool,
    change_controller: bool,
    origin: Zone,
) {
    assert_permission_cast_from_selected(
        oracle,
        spell_type,
        expect_exile,
        change_controller,
        origin,
        None,
    );
}

#[test]
fn equal_cost_graveyard_permissions_keep_riders_across_sources_and_intrinsic_offsets() {
    let plain = "Once during each of your turns, you may cast a spell from your graveyard by sacrificing a creature in addition to paying its other costs.";
    let with_rider = format!(
        "{plain} If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead."
    );
    for setup in [
        (true, false, false),
        (false, true, false),
        (true, true, false),
    ] {
        for rider_first in [false, true] {
            let oracle = if rider_first {
                format!("{with_rider}\n{plain}")
            } else {
                format!("{plain}\n{with_rider}")
            };
            for selected in [0, 1] {
                for change_controller in [false, true] {
                    let expect_exile = (selected == 0) == rider_first;
                    let selected_index = selected + usize::from(setup.1);
                    assert_permission_cast_with_sources(
                        &oracle,
                        CardType::Instant,
                        expect_exile,
                        change_controller,
                        Zone::Graveyard,
                        Some(selected_index),
                        setup,
                    );
                }
            }
        }
    }
}

#[test]
fn equal_cost_graveyard_permissions_keep_separate_once_turn_uses() {
    let plain = "Once during each of your turns, you may cast a spell from your graveyard by sacrificing a creature in addition to paying its other costs.";
    let oracle = format!(
        "{plain}\n{plain} If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead."
    );
    // First verify independent-source usage, then the two abilities of one source.
    for separate_sources in [true, false] {
        assert_permission_cast_with_sources(
            &oracle,
            CardType::Instant,
            false,
            false,
            Zone::Graveyard,
            Some(0),
            (separate_sources, false, true),
        );
    }
}

fn assert_permission_cast_from_selected(
    oracle: &str,
    spell_type: CardType,
    expect_exile: bool,
    change_controller: bool,
    origin: Zone,
    selected_index: Option<usize>,
) {
    assert_permission_cast_with_sources(
        oracle,
        spell_type,
        expect_exile,
        change_controller,
        origin,
        selected_index,
        (false, false, false),
    );
}

fn assert_permission_cast_with_sources(
    oracle: &str,
    spell_type: CardType,
    expect_exile: bool,
    change_controller: bool,
    origin: Zone,
    selected_index: Option<usize>,
    setup: (bool, bool, bool),
) {
    let permission =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Actual cast permission")
            .card_types(vec![CardType::Enchantment])
            .parse_text(oracle)
            .unwrap();
    let spell_definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Actual permission spell")
            .card_types(vec![spell_type])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Generic(1),
            ]]))
            .parse_text("You gain 1 life.")
            .unwrap();
    for countered in [false, true] {
        if setup.2 && countered {
            continue;
        }
        // A creature must resolve onto the battlefield; use countering to test its stack departure.
        if spell_type == CardType::Creature && !countered {
            continue;
        }
        for remove_source in [false, true] {
            if setup.2 && remove_source {
                continue;
            }
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = crate::game_state::Phase::FirstMain;
            game.turn.step = None;
            let mut source = if setup.0 {
                let mut first = None;
                for line in oracle.lines() {
                    let definition = crate::CardDefinitionBuilder::new(
                        crate::ids::CardId::new(),
                        "Separate permission source",
                    )
                    .card_types(vec![CardType::Enchantment])
                    .parse_text(line)
                    .unwrap();
                    let id =
                        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                    first.get_or_insert(id);
                }
                first.unwrap()
            } else {
                game.create_object_from_definition(&permission, alice, Zone::Battlefield)
            };
            let body =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Permission cost body")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(1, 1))
                    .build();
            let sacrificed = game.create_object_from_card(&body, alice, Zone::Battlefield);
            let spell = game.create_object_from_definition(&spell_definition, alice, origin);
            if setup.1 {
                game.object_mut(spell).unwrap().alternative_casts.push(
                    crate::alternative_cast::AlternativeCastingMethod::Flashback {
                        total_cost: crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(
                            vec![vec![crate::mana::ManaSymbol::Generic(1)]],
                        )),
                    },
                );
            }
            let stable = game.object(spell).unwrap().stable_id;
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(crate::mana::ManaSymbol::Colorless, 1);
            let action = crate::decision::compute_legal_actions(&game,alice).expect("fixture has complete replacement state").into_iter().find(|action| match action {
                crate::decision::LegalAction::CastSpell {spell_id, casting_method, ..} => *spell_id == spell
                    && selected_index.is_none_or(|expected| matches!(casting_method,
                        crate::alternative_cast::CastingMethod::PlayFrom {use_alternative: Some(actual), ..} if *actual == expected)),
                _ => false,
            })
                .expect("linked permission must expose a payable graveyard cast");
            if let crate::decision::LegalAction::CastSpell {
                casting_method:
                    crate::alternative_cast::CastingMethod::PlayFrom {
                        source: selected_source,
                        ..
                    },
                ..
            } = &action
            {
                source = *selected_source;
            }
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
                if game.stack.iter().any(|entry| !entry.is_ability) {
                    break;
                }
                let crate::decision::GameProgress::NeedsDecisionCtx(choice) = progress else {
                    panic!("cast stalled: {progress:?}");
                };
                progress = crate::game_loop::apply_decision_context_with_dm(
                    &mut game, &mut queue, &mut state, &choice, &mut dm,
                )
                .unwrap();
            }
            let stack_id = game
                .stack
                .iter()
                .find(|entry| !entry.is_ability)
                .expect("spell must reach stack")
                .object_id;
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
            if origin == Zone::Graveyard {
                assert!(
                    game.object(sacrificed).is_none(),
                    "real casting must pay sacrifice cost"
                );
                assert!(matches!(
                    game.object(stack_id)
                        .unwrap()
                        .cast_alternative_method
                        .as_deref(),
                    Some(crate::alternative_cast::AlternativeCastingMethod::FromZone { .. })
                ));
            } else {
                assert!(
                    game.object(sacrificed).is_some(),
                    "ordinary casting must not pay permission cost"
                );
                assert!(
                    game.object(stack_id)
                        .unwrap()
                        .cast_alternative_method
                        .is_none()
                );
            }
            if change_controller {
                let bob = game.players[1].id;
                game.set_current_controller(stack_id, bob)
                    .expect("finite controller fixture must refresh successfully");
                assert_eq!(game.current_controller(stack_id), Some(bob));
            }
            if remove_source {
                game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            }
            if countered {
                let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                crate::effects::execute_effect(
                    &mut game,
                    &Effect::new(crate::effects::CounterEffect::new(
                        crate::target::ChooseSpec::SpecificObject(stack_id),
                    )),
                    &mut ctx,
                )
                .unwrap();
            } else {
                crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            assert_eq!(
                game.object(game.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone,
                if expect_exile {
                    Zone::Exile
                } else {
                    Zone::Graveyard
                }
            );
            let beneficiary = if change_controller {
                game.players[1].id
            } else {
                alice
            };
            assert_eq!(
                game.player(beneficiary).unwrap().life,
                if countered { 20 } else { 21 },
                "spell_type={spell_type:?} changed_controller={change_controller} removed_source={remove_source} origin={origin:?}"
            );
            assert!(!game.stack.iter().any(|entry| entry.object_id == stack_id));
            if setup.2 {
                game.create_object_from_card(&body, alice, Zone::Battlefield);
                let next =
                    game.create_object_from_definition(&spell_definition, alice, Zone::Graveyard);
                game.player_mut(alice)
                    .unwrap()
                    .mana_pool
                    .add(crate::mana::ManaSymbol::Colorless, 1);
                let available = crate::decision::compute_legal_actions(&game, alice)
                    .expect("fixture has complete replacement state");
                let has_index = |expected| {
                    available.iter().any(|action| matches!(action,
                    crate::decision::LegalAction::CastSpell {spell_id, casting_method:
                        crate::alternative_cast::CastingMethod::PlayFrom {use_alternative: Some(actual), ..}, ..}
                        if *spell_id == next && *actual == expected))
                };
                assert!(!has_index(0), "the used permission must be exhausted");
                assert!(
                    has_index(1),
                    "the unselected permission must retain its once-turn use; separate_sources={}",
                    setup.0
                );
                let next_stable = game.object(next).unwrap().stable_id;
                let next_action = available.into_iter().find(|action| matches!(action,
                    crate::decision::LegalAction::CastSpell {spell_id, casting_method:
                        crate::alternative_cast::CastingMethod::PlayFrom {use_alternative: Some(1), ..}, ..} if *spell_id == next)).unwrap();
                let mut second_state =
                    crate::game_loop::PriorityLoopState::new(game.players_in_game());
                let mut second_queue = crate::triggers::TriggerQueue::new();
                let mut progress = crate::game_loop::apply_priority_response_with_dm(
                    &mut game,
                    &mut second_queue,
                    &mut second_state,
                    &crate::game_loop::PriorityResponse::PriorityAction(next_action),
                    &mut dm,
                )
                .unwrap();
                for _ in 0..15 {
                    if game.stack.iter().any(|entry| !entry.is_ability) {
                        break;
                    }
                    let crate::decision::GameProgress::NeedsDecisionCtx(choice) = progress else {
                        panic!("second cast stalled: {progress:?}");
                    };
                    progress = crate::game_loop::apply_decision_context_with_dm(
                        &mut game,
                        &mut second_queue,
                        &mut second_state,
                        &choice,
                        &mut dm,
                    )
                    .unwrap();
                }
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
                crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(
                    game.object(game.find_object_by_stable_id(next_stable).unwrap())
                        .unwrap()
                        .zone,
                    Zone::Exile
                );
                assert_eq!(game.player(alice).unwrap().life, 22);
                let third =
                    game.create_object_from_definition(&spell_definition, alice, Zone::Graveyard);
                game.create_object_from_card(&body, alice, Zone::Battlefield);
                game.player_mut(alice)
                    .unwrap()
                    .mana_pool
                    .add(crate::mana::ManaSymbol::Colorless, 1);
                let available = crate::decision::compute_legal_actions(&game, alice)
                    .expect("fixture has complete replacement state");
                assert!(
                    !available.iter().any(|action| matches!(action,
                    crate::decision::LegalAction::CastSpell {spell_id, ..} if *spell_id == third)),
                    "both exact permissions must now be exhausted"
                );
                let checkpoint = game.clone();
                assert_eq!(
                    checkpoint.turn_store.grant_cast_uses_this_turn,
                    game.turn_store.grant_cast_uses_this_turn
                );
                game.next_turn();
                game.next_turn();
                game.turn.active_player = alice;
                game.turn.priority_player = Some(alice);
                game.turn.phase = crate::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.player_mut(alice)
                    .unwrap()
                    .mana_pool
                    .add(crate::mana::ManaSymbol::Colorless, 1);
                assert!(game.turn_store.grant_cast_uses_this_turn.is_empty());
                assert!(
                    crate::decision::compute_legal_actions(&game, alice)
                        .expect("fixture has complete replacement state")
                        .iter()
                        .any(|action| matches!(action,
                    crate::decision::LegalAction::CastSpell {spell_id, ..} if *spell_id == third)),
                    "turn transition must reset permission use"
                );
            }
        }
    }
}

#[test]
fn graveyard_permission_survives_sacrificing_its_provider_during_payment() {
    let oracle = "Once during each of your turns, you may cast a spell from your graveyard by sacrificing a creature in addition to paying its other costs. If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead.";
    for countered in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;
        let provider = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Sacrificed permission provider",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .parse_text(oracle)
        .unwrap();
        let source = game.create_object_from_definition(&provider, alice, Zone::Battlefield);
        let source_stable = game.object(source).unwrap().stable_id;
        let other =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other permission provider")
                .card_types(vec![CardType::Artifact])
                .build();
        let other = game.create_object_from_card(&other, alice, Zone::Battlefield);
        let expensive = crate::alternative_cast::AlternativeCastingMethod::FromZone {
            name: "Other graveyard permission".into(),
            zone: Zone::Graveyard,
            total_cost: crate::cost::TotalCost::mana(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Generic(3),
            ])),
            condition: None,
            exiles_after_resolution: false,
            entry_counters: Vec::new(),
        };
        game.object_mut(other).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::grants(crate::grant::GrantSpec::new(
                    crate::grant::Grantable::AlternativeCast(expensive.clone()),
                    crate::filter::ObjectFilter::default(),
                    Zone::Graveyard,
                )),
            ),
        );
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Provider payment spell")
                .card_types(vec![CardType::Instant])
                .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                    crate::mana::ManaSymbol::Generic(1),
                ]))
                .parse_text("You gain 1 life.")
                .unwrap();
        let spell = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let stable = game.object(spell).unwrap().stable_id;
        let grants = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, spell, Zone::Graveyard, alice);
        assert_eq!(grants[0].source_id, source);
        let selected_method = grants[0].method.clone();
        let selected_identity = grants[0].permission_identity.clone().unwrap();
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 1);
        let action = crate::decision::compute_legal_actions(&game, alice).expect("fixture has complete replacement state").into_iter().find(|action| matches!(action,
            crate::decision::LegalAction::CastSpell {spell_id, casting_method:
                crate::alternative_cast::CastingMethod::PlayFrom {source: selected_source, use_alternative: Some(0), ..}, ..}
                if *spell_id == spell && *selected_source == source)).expect("provider sacrifice must be a payable cost");
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
            if game.stack.iter().any(|entry| !entry.is_ability) {
                break;
            }
            let crate::decision::GameProgress::NeedsDecisionCtx(choice) = progress else {
                panic!("provider cast stalled: {progress:?}");
            };
            progress = crate::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &choice, &mut dm,
            )
            .unwrap();
        }
        let stack = game
            .stack
            .iter()
            .find(|entry| !entry.is_ability)
            .expect("paid spell reaches stack")
            .object_id;
        assert!(
            game.object(source).is_none(),
            "the only creature must actually be sacrificed as payment"
        );
        assert_eq!(
            game.object(game.find_object_by_stable_id(source_stable).unwrap())
                .unwrap()
                .zone,
            Zone::Graveyard
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(
            game.object(stack).unwrap().cast_alternative_method_owned(),
            Some(selected_method.clone())
        );
        assert_eq!(
            crate::decision::resolve_play_from_alternative_method(
                &game,
                alice,
                game.object(stack).unwrap(),
                Zone::Graveyard,
                0
            ),
            Some(selected_method)
        );
        let remaining = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, stack, Zone::Graveyard, alice);
        assert_eq!(
            remaining[0].method, expensive,
            "the current index occupant really differs from the selected method"
        );
        assert!(
            game.turn_store
                .grant_cast_uses_this_turn
                .contains(&(alice, selected_identity))
        );
        if countered {
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            crate::effects::execute_effect(
                &mut game,
                &Effect::new(crate::effects::CounterEffect::new(
                    crate::target::ChooseSpec::SpecificObject(stack),
                )),
                &mut ctx,
            )
            .unwrap();
        } else {
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            if countered { 20 } else { 21 }
        );
    }
}
