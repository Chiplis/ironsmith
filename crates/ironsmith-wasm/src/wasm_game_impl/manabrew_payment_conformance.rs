//! Mana-payment contract and live continuation regressions.
//! IRONSMITH_CONFORMANCE_REPORT saves every verdict in the matrix.
use super::*;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::compute_legal_actions;
use ironsmith::decisions::context::ManaPaymentContext;
use ironsmith::mana_payment::*;

fn fixture() -> WasmGame {
    let mut g = WasmGame::new();
    g.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 42);
    g
}
fn card(g: &mut WasmGame, name: &str, types: Vec<CardType>, zone: Zone) -> ObjectId {
    let c = ironsmith::card::CardBuilder::new(CardId::new(), name)
        .card_types(types)
        .build();
    g.game
        .create_object_from_card(&c, PlayerId::from_index(0), zone)
}
fn land(g: &mut WasmGame, symbols: Vec<ManaSymbol>) -> ObjectId {
    let id = card(g, "Audit land", vec![CardType::Land], Zone::Battlefield);
    g.game
        .object_mut(id)
        .unwrap()
        .abilities_mut()
        .push(ironsmith::ability::Ability::mana(
            ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::tap()),
            symbols,
        ));
    id
}
fn request(g: &mut WasmGame, cost: ManaCost) -> ManaPaymentRequest {
    let source = card(g, "Audit spell", vec![CardType::Sorcery], Zone::Stack);
    ManaPaymentRequest::new(
        PlayerId::from_index(0),
        source,
        ironsmith::costs::PaymentReason::CastSpell,
        cost,
    )
}
fn context(g: &WasmGame, r: &ManaPaymentRequest) -> ManaPaymentContext {
    let plan = plan_mana_payment(&g.game, r)
        .ok()
        .and_then(|p| p.into_iter().next())
        .unwrap_or_else(|| unfunded_mana_payment_plan(&g.game, r));
    ManaPaymentContext::new(r.payer, r.source, "Audit spell", r.clone(), plan)
}
fn open(g: &WasmGame, c: &ManaPaymentContext) -> ManabrewOpenPrompt {
    let (input, binding) = g
        .build_manabrew_prompt(&DecisionContext::ManaPayment(c.clone()))
        .unwrap();
    ManabrewOpenPrompt {
        prompt_id: 1,
        deciding_player: c.player,
        decision_hash: 1,
        source_card_id: None,
        source_card: None,
        input,
        binding,
    }
}
fn payment(o: &ManabrewOpenPrompt) -> &PayManaCostInput {
    match &o.input {
        PromptInput::PayManaCost(p) => p,
        _ => panic!("wrong family"),
    }
}
fn replan(g: &WasmGame, o: &ManabrewOpenPrompt, id: &str) -> ManaPaymentPreferences {
    let a = g
        .manabrew_response_action(
            o,
            PromptOutput::PayManaCost(PayManaCostOutput::Act {
                action_id: id.into(),
            }),
        )
        .unwrap();
    match a {
        ManabrewResponseAction::Dispatch(UiCommand::ManaPayment { response }) => {
            match response.into_runtime().unwrap() {
                ManaPaymentResponse::Replan { preferences } => preferences,
                _ => panic!("expected replan"),
            }
        }
        _ => panic!("expected payment dispatch"),
    }
}
fn pay_allowed(g: &WasmGame, o: &ManabrewOpenPrompt, auto: bool) -> bool {
    g.manabrew_response_action(
        o,
        PromptOutput::PayManaCost(PayManaCostOutput::Pay { auto }),
    )
    .is_ok()
}

fn live_fixture() -> (WasmGame, ObjectId, ObjectId) {
    live_fixture_with(ManaCost::from_symbols(vec![ManaSymbol::Blue]), |_| {})
}

fn live_fixture_with(
    cost: ManaCost,
    setup: impl FnOnce(&mut WasmGame),
) -> (WasmGame, ObjectId, ObjectId) {
    let mut g = fixture();
    let alice = PlayerId::from_index(0);
    g.game.turn.active_player = alice;
    g.game.turn.priority_player = Some(alice);
    g.game.turn.turn_number = 1;
    g.game.turn.phase = Phase::FirstMain;
    g.game.turn.step = None;
    g.runner = Some(ironsmith::turn_runner::TurnRunner::from_state_for_sync(
        ironsmith::turn_runner::TurnState::FirstMainPriority,
    ));
    g.runner_awaiting_priority = true;
    g.priority_state.restore_priority_tracker_for_sync(0, 2);
    let land = land(&mut g, vec![ManaSymbol::Blue]);
    setup(&mut g);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Live audit spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(cost)
        .build();
    let spell = g
        .game
        .create_object_from_definition(&spell, alice, Zone::Hand);
    let actions = compute_legal_actions(&g.game, alice);
    let index = actions
        .iter()
        .position(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .unwrap();
    g.dispatch_live_priority_response(
        DecisionContext::Priority(ironsmith::decisions::context::PriorityContext::new(
            alice, actions,
        )),
        UiCommand::PriorityAction {
            action_index: Some(index),
            action_ref: None,
        },
    )
    .unwrap();
    assert!(matches!(
        g.pending_decision,
        Some(DecisionContext::ManaPayment(_))
    ));
    (g, land, spell)
}

// Exercise the exact Rust translation/continuation used by manabrewRespond,
// without calling wasm-bindgen's JS serialization on a native target.
fn live_response(g: &mut WasmGame, output: PayManaCostOutput) {
    live_output(g, PromptOutput::PayManaCost(output));
}

fn live_output(g: &mut WasmGame, output: PromptOutput) {
    let prompt = g.ensure_manabrew_prompt().unwrap().unwrap();
    let open = g
        .validate_manabrew_response(PlayerId::from_index(0), prompt.prompt_id, &output)
        .unwrap()
        .clone();
    let ManabrewResponseAction::Dispatch(command) =
        g.manabrew_response_action(&open, output).unwrap()
    else {
        panic!("payment must dispatch")
    };
    let decision = g.pending_decision.take().unwrap();
    if g.pending_live_continuation.is_some() {
        g.dispatch_live_priority_continuation(decision, command)
            .unwrap();
    } else {
        g.dispatch_live_priority_response(decision, command)
            .unwrap();
    }
    g.manabrew_open_prompt = None;
}

fn choose_mana_source(g: &mut WasmGame, source: ObjectId, index: usize) {
    let p = g.ensure_manabrew_prompt().unwrap().unwrap();
    let PromptInput::PayManaCost(p) = p.input else {
        panic!("expected payment")
    };
    let action = p
        .actions
        .iter()
        .find(|a| {
            matches!(&a.kind, PaymentActionKind::ActivateManaAbility(v)
        if v.card_id == object_id(&g.game, source) && v.ability_index == index)
        })
        .unwrap();
    live_response(
        g,
        PayManaCostOutput::Act {
            action_id: action.id.clone(),
        },
    );
}

#[test]
fn nested_mana_activation_completes_through_protocol() {
    let _guard = crate::test_id_counter_guard();
    for preselect_land in [false, true] {
        let mut filter = None;
        let (mut g, land, _) = live_fixture_with(ManaCost::new().add_generic(2), |g| {
            let def = CardDefinitionBuilder::new(CardId::new(), "Protocol filter")
                .card_types(vec![CardType::Artifact])
                .with_ability(ironsmith::ability::Ability::mana(
                    ironsmith::cost::TotalCost::from_costs(vec![
                        ironsmith::costs::Cost::mana(ManaCost::new().add_generic(1)),
                        ironsmith::costs::Cost::tap(),
                    ]),
                    vec![ManaSymbol::Colorless, ManaSymbol::Colorless],
                ))
                .build();
            filter = Some(g.game.create_object_from_definition(
                &def,
                PlayerId::from_index(0),
                Zone::Battlefield,
            ));
        });
        if preselect_land {
            choose_mana_source(&mut g, land, 0);
        }
        choose_mana_source(&mut g, filter.unwrap(), 0);
        assert_eq!(
            g.current_mana_payment_view().unwrap().source_name,
            "Protocol filter"
        );
        choose_mana_source(&mut g, land, 0);
        live_response(&mut g, PayManaCostOutput::Pay { auto: false });
        assert_eq!(
            g.current_mana_payment_view().unwrap().source_name,
            "Live audit spell"
        );
        live_response(&mut g, PayManaCostOutput::Pay { auto: false });
        assert!(g.priority_state.pending_cast.is_none());
        assert_eq!(
            g.game.stack.len(),
            1,
            "nested payment must cast the spell, preselected={preselect_land}"
        );
        assert!(g.game.is_tapped(land));
        assert!(g.game.is_tapped(filter.unwrap()));
        assert_eq!(
            g.game
                .player(PlayerId::from_index(0))
                .unwrap()
                .mana_pool
                .total(),
            0
        );
    }
}

#[test]
fn sacrifice_mana_activation_exposes_and_honors_card_choice() {
    let _guard = crate::test_id_counter_guard();
    let mut tower = None;
    let (mut g, _, _) = live_fixture_with(ManaCost::from_symbols(vec![ManaSymbol::Black]), |g| {
        tower = Some(g.game.create_object_from_definition(
            &ironsmith_registry_test::cards::definitions::phyrexian_tower(),
            PlayerId::from_index(0),
            Zone::Battlefield,
        ));
        card(
            g,
            "Keep creature",
            vec![CardType::Creature],
            Zone::Battlefield,
        );
        card(
            g,
            "Chosen sacrifice",
            vec![CardType::Creature],
            Zone::Battlefield,
        );
    });
    choose_mana_source(&mut g, tower.unwrap(), 1);
    let p = g.ensure_manabrew_prompt().unwrap().unwrap();
    let PromptInput::ChooseCards(p) = p.input else {
        panic!("sacrifice must expose card selection")
    };
    let id = p
        .cards
        .iter()
        .find(|c| c.identity.name == "Chosen sacrifice")
        .unwrap()
        .id
        .clone();
    live_output(
        &mut g,
        PromptOutput::ChooseCards(ChooseCardsOutput::ChooseCardsDecision {
            chosen_card_ids: vec![id],
        }),
    );
    assert!(g.game.battlefield.iter().any(|id| {
        g.game
            .object(*id)
            .is_some_and(|c| c.name == "Keep creature")
    }));
    assert!(
        g.game
            .player(PlayerId::from_index(0))
            .unwrap()
            .graveyard
            .iter()
            .any(|id| g
                .game
                .object(*id)
                .is_some_and(|c| c.name == "Chosen sacrifice"))
    );
    live_response(&mut g, PayManaCostOutput::Pay { auto: false });
    assert_eq!(g.game.stack.len(), 1);
    assert_eq!(
        g.game
            .player(PlayerId::from_index(0))
            .unwrap()
            .mana_pool
            .black,
        1
    );
}

#[test]
fn payment_contract_matrix() {
    let _guard = crate::test_id_counter_guard();
    let mut rows = Vec::new();
    let mut check = |id: &str, category: &str, ok: bool, detail: String| {
        println!("{} {id}: {detail}", if ok { "PASS" } else { "FAIL" });
        rows.push(serde_json::json!({"id":id,"category":category,"pass":ok,"detail":detail}));
    };
    let alice = PlayerId::from_index(0);
    let mut g = fixture();
    let l = land(&mut g, vec![ManaSymbol::Blue]);
    let mut r = request(&mut g, ManaCost::from_symbols(vec![ManaSymbol::Blue]));
    let c = context(&g, &r);
    let o = open(&g, &c);
    let wire = serde_json::to_value(&o.input).unwrap();
    check(
        "wire.input",
        "contract",
        wire["type"] == "payManaCost"
            && wire["cardId"].is_string()
            && wire["manaCost"] == "{U}"
            && wire["actions"].is_array()
            && wire.get("gameView").is_none(),
        wire.to_string(),
    );
    for (id, value, valid) in [
        (
            "act",
            serde_json::json!({"type":"act","actionId":"x"}),
            true,
        ),
        (
            "pay.manual",
            serde_json::json!({"type":"pay","auto":false}),
            true,
        ),
        (
            "pay.auto",
            serde_json::json!({"type":"pay","auto":true}),
            true,
        ),
        ("pay.default", serde_json::json!({"type":"pay"}), true),
        ("cancel", serde_json::json!({"type":"cancel"}), true),
        ("act.missing_id", serde_json::json!({"type":"act"}), false),
        (
            "act.numeric_id",
            serde_json::json!({"type":"act","actionId":1}),
            false,
        ),
        (
            "pay.invalid_auto",
            serde_json::json!({"type":"pay","auto":"yes"}),
            false,
        ),
        ("unknown", serde_json::json!({"type":"bogus"}), false),
    ] {
        check(
            &format!("wire.{id}"),
            "contract",
            serde_json::from_value::<PayManaCostOutput>(value.clone()).is_ok() == valid,
            value.to_string(),
        );
    }
    check(
        "manual.incomplete",
        "contract",
        !payment(&o).can_confirm_from_pool && !pay_allowed(&g, &o, false),
        "manual pay cannot silently activate a land".into(),
    );
    check(
        "auto.available",
        "contract",
        pay_allowed(&g, &o, true),
        "auto pay maps to confirmation".into(),
    );
    check(
        "cancel.mapping",
        "contract",
        matches!(
            g.manabrew_response_action(&o, PromptOutput::PayManaCost(PayManaCostOutput::Cancel)),
            Ok(ManabrewResponseAction::Dispatch(UiCommand::ManaPayment {
                response: ManaPaymentCommand::Cancel
            }))
        ),
        "cancel maps to cancellation".into(),
    );
    let ids: HashSet<_> = payment(&o).actions.iter().map(|a| &a.id).collect();
    check(
        "actions.unique",
        "contract",
        ids.len() == payment(&o).actions.len(),
        "action IDs unique within prompt".into(),
    );
    let activation = payment(&o)
        .actions
        .iter()
        .find(|a| matches!(a.kind, PaymentActionKind::ActivateManaAbility(_)))
        .unwrap();
    check(
        "actions.reference",
        "contract",
        match &activation.kind {
            PaymentActionKind::ActivateManaAbility(a) => {
                a.card_id == object_id(&g.game, l) && a.is_mana_ability && a.ability_index == 0
            }
            _ => false,
        },
        "activation references the actual source and ability".into(),
    );
    g.pending_decision = Some(DecisionContext::ManaPayment(c.clone()));
    let p = g.ensure_manabrew_prompt().unwrap().unwrap();
    check(
        "prompt.nonzero",
        "contract",
        p.prompt_id > 0,
        format!("prompt {}", p.prompt_id),
    );
    check(
        "prompt.stable",
        "contract",
        g.ensure_manabrew_prompt().unwrap().unwrap().prompt_id == p.prompt_id,
        "unchanged decision reuses prompt".into(),
    );
    for (id, player, pid, output, code) in [
        (
            "zero",
            alice,
            0,
            PromptOutput::PayManaCost(PayManaCostOutput::Cancel),
            ProtocolErrorCode::StalePrompt,
        ),
        (
            "stale",
            alice,
            p.prompt_id + 1,
            PromptOutput::PayManaCost(PayManaCostOutput::Cancel),
            ProtocolErrorCode::StalePrompt,
        ),
        (
            "wrong_player",
            PlayerId::from_index(1),
            p.prompt_id,
            PromptOutput::PayManaCost(PayManaCostOutput::Cancel),
            ProtocolErrorCode::WrongPlayer,
        ),
        (
            "wrong_family",
            alice,
            p.prompt_id,
            PromptOutput::ChooseBoolean(ChooseBooleanOutput::Decision { value: true }),
            ProtocolErrorCode::WrongPromptType,
        ),
        (
            "unknown_action",
            alice,
            p.prompt_id,
            PromptOutput::PayManaCost(PayManaCostOutput::Act {
                action_id: "unknown".into(),
            }),
            ProtocolErrorCode::UnknownActionId,
        ),
    ] {
        let err = g.validate_manabrew_response(player, pid, &output).err();
        check(
            &format!("validation.{id}"),
            "contract",
            err.as_ref().is_some_and(|e| e.code == code),
            format!("{err:?}"),
        );
    }
    check(
        "error.resend",
        "contract",
        g.manabrew_result(
            Some(alice),
            Some(protocol_error(
                ProtocolErrorCode::UnknownActionId,
                "audit",
                Some(p.prompt_id),
            )),
        )
        .prompt
        .is_some_and(|q| q.prompt_id == p.prompt_id),
        "recoverable error retains prompt".into(),
    );
    check(
        "privacy.spectator",
        "contract",
        g.manabrew_result(None, None).prompt.is_none(),
        "spectator gets no payment prompt".into(),
    );
    check(
        "privacy.other_player",
        "contract",
        g.manabrew_result(Some(PlayerId::from_index(1)), None)
            .prompt
            .is_none(),
        "other player gets no payment prompt".into(),
    );
    r.preferences = replan(&g, &o, &activation.id);
    let selected = context(&g, &r);
    let so = open(&g, &selected);
    g.pending_decision = Some(DecisionContext::ManaPayment(selected.clone()));
    check(
        "prompt.advances",
        "contract",
        g.ensure_manabrew_prompt().unwrap().unwrap().prompt_id > p.prompt_id,
        "selection produces new prompt ID".into(),
    );
    check(
        "manual.selected",
        "contract",
        payment(&so).can_confirm_from_pool && pay_allowed(&g, &so, false),
        "selected land permits manual confirmation".into(),
    );
    check(
        "state.pool",
        "contract",
        g.manabrew_projected_payment_state(alice).unwrap().0.blue == 1,
        "selected land projects blue mana".into(),
    );
    check(
        "state.tapped",
        "state_consistency",
        g.manabrew_zone_cards([l], Some(alice), false)
            .iter()
            .any(|c| matches!(c, CardView::Visible(c) if c.tapped)),
        "selected tap action must be reflected in card state as well as pool".into(),
    );
    let undo = payment(&so)
        .actions
        .iter()
        .find(|a| matches!(a.kind, PaymentActionKind::UndoMana { .. }));
    check(
        "undo.offered",
        "contract",
        undo.is_some(),
        "selected land offers undo".into(),
    );
    if let Some(undo) = undo {
        r.preferences = replan(&g, &so, &undo.id);
        check(
            "undo.restores",
            "contract",
            r.preferences.required_activations.is_empty()
                && !payment(&open(&g, &context(&g, &r))).can_confirm_from_pool,
            "undo removes selection and requires payment again".into(),
        );
    }

    // Unfunded contexts are real runtime placeholders, not fabricated invalid plans.
    let mut empty = fixture();
    let er = request(&mut empty, ManaCost::from_symbols(vec![ManaSymbol::Blue]));
    let ec = context(&empty, &er);
    let eo = open(&empty, &ec);
    check(
        "unfunded.confirm",
        "contract",
        !payment(&eo).can_confirm_from_pool && !pay_allowed(&empty, &eo, false),
        format!(
            "payable={}, canConfirmFromPool={}",
            ec.plan.payable,
            payment(&eo).can_confirm_from_pool
        ),
    );
    check(
        "unfunded.cost",
        "contract",
        payment(&eo).mana_cost == "{U}",
        format!("remaining cost {:?}", payment(&eo).mana_cost),
    );

    // Exercise every mana color, generic, hybrid, explicit colorless and X from pool.
    for (name, cost, pool_color, x) in [
        (
            "white",
            ManaCost::from_symbols(vec![ManaSymbol::White]),
            ManaSymbol::White,
            0,
        ),
        (
            "blue",
            ManaCost::from_symbols(vec![ManaSymbol::Blue]),
            ManaSymbol::Blue,
            0,
        ),
        (
            "black",
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
            ManaSymbol::Black,
            0,
        ),
        (
            "red",
            ManaCost::from_symbols(vec![ManaSymbol::Red]),
            ManaSymbol::Red,
            0,
        ),
        (
            "green",
            ManaCost::from_symbols(vec![ManaSymbol::Green]),
            ManaSymbol::Green,
            0,
        ),
        (
            "colorless",
            ManaCost::from_symbols(vec![ManaSymbol::Colorless]),
            ManaSymbol::Colorless,
            0,
        ),
        (
            "generic",
            ManaCost::new().add_generic(1),
            ManaSymbol::Blue,
            0,
        ),
        (
            "hybrid",
            ManaCost::from_pips(vec![vec![ManaSymbol::White, ManaSymbol::Blue]]),
            ManaSymbol::Blue,
            0,
        ),
        (
            "x",
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ManaSymbol::Blue,
            1,
        ),
        ("zero", ManaCost::new(), ManaSymbol::Blue, 0),
    ] {
        let mut g = fixture();
        g.game
            .player_mut(alice)
            .unwrap()
            .mana_pool
            .add(pool_color, 1);
        let mut r = request(&mut g, cost);
        r.x_value = x;
        let c = context(&g, &r);
        let o = open(&g, &c);
        let result = execute_mana_payment_plan(
            &mut g.game,
            &r,
            &c.plan,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        );
        check(
            &format!("pool.{name}"),
            "engine",
            c.plan.payable
                && payment(&o).can_confirm_from_pool
                && matches!(result, Ok(ManaPaymentExecution::Paid)),
            format!("cost={}, result={result:?}", payment(&o).mana_cost),
        );
    }

    for (name, keyword, resource) in [
        (
            "convoke",
            ironsmith::static_abilities::StaticAbility::convoke(),
            PaymentResourceKind::Convoke,
        ),
        (
            "improvise",
            ironsmith::static_abilities::StaticAbility::improvise(),
            PaymentResourceKind::Improvise,
        ),
        (
            "delve",
            ironsmith::static_abilities::StaticAbility::delve(),
            PaymentResourceKind::Delve,
        ),
    ] {
        let mut g = fixture();
        let source = card(
            &mut g,
            "Resource",
            vec![CardType::Creature, CardType::Artifact],
            if name == "delve" {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            },
        );
        let mut r = request(&mut g, ManaCost::new().add_generic(1));
        g.game
            .object_mut(r.source)
            .unwrap()
            .abilities_mut()
            .push(ironsmith::ability::Ability::static_ability(keyword));
        let o = open(&g, &context(&g, &r));
        let a = payment(&o).actions.iter().find(|a| matches!(&a.kind, PaymentActionKind::UseResource { resource: k, .. } if *k == resource));
        check(
            &format!("resource.{name}.offer"),
            "payment_ui",
            a.is_some(),
            format!("resource {source:?}; actions={:?}", payment(&o).actions),
        );
        if let Some(a) = a {
            r.preferences = replan(&g, &o, &a.id);
            let c = context(&g, &r);
            let selected = open(&g, &c);
            check(
                &format!("resource.{name}.select"),
                "contract",
                c.plan.payable
                    && payment(&selected).can_confirm_from_pool
                    && payment(&selected).mana_cost.is_empty(),
                "selection reduces generic cost to zero".into(),
            );
            let release = payment(&selected).actions.iter().find(|a| matches!(&a.kind, PaymentActionKind::ReleaseResource { resource:k, .. } if *k == resource));
            check(
                &format!("resource.{name}.release"),
                "contract",
                release.is_some_and(|a| {
                    replan(&g, &selected, &a.id)
                        .required_alternatives
                        .is_empty()
                }),
                "release restores resource selection".into(),
            );
            let result = execute_mana_payment_plan(
                &mut g.game,
                &r,
                &c.plan,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            );
            check(
                &format!("resource.{name}.commit"),
                "engine",
                matches!(result, Ok(ManaPaymentExecution::Paid))
                    && if name == "delve" {
                        g.game
                            .exile
                            .iter()
                            .any(|id| g.game.object(*id).is_some_and(|c| c.name == "Resource"))
                    } else {
                        g.game.is_tapped(source)
                    },
                format!("{result:?}"),
            );
        }
    }

    let mut g = fixture();
    g.game.player_mut(alice).unwrap().mana_pool.black = 2;
    let mut r = request(
        &mut g,
        ManaCost::from_pips(vec![vec![ManaSymbol::Black, ManaSymbol::Life(2)]; 2]),
    );
    let o = open(&g, &context(&g, &r));
    let life = payment(&o)
        .actions
        .iter()
        .find(|a| matches!(a.kind, PaymentActionKind::PayLife { .. }))
        .unwrap();
    check(
        "life.partial",
        "choice_coverage",
        payment(&o)
            .actions
            .iter()
            .any(|a| matches!(a.kind, PaymentActionKind::PayLife { amount: 2 })),
        format!("two Phyrexian pips; offered {:?}", life.kind),
    );
    r.preferences = replan(&g, &o, &life.id);
    let c = context(&g, &r);
    let o = open(&g, &c);
    let mut partial_game = g.game.clone();
    let partial_result = execute_mana_payment_plan(
        &mut partial_game,
        &r,
        &c.plan,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    );
    check(
        "life.partial_commit",
        "engine",
        matches!(partial_result, Ok(ManaPaymentExecution::Paid))
            && partial_game.player(alice).unwrap().life == 18
            && partial_game.player(alice).unwrap().mana_pool.black == 1,
        format!("one pip paid with life, one with mana: {partial_result:?}"),
    );
    let second = payment(&o)
        .actions
        .iter()
        .find(|a| matches!(a.kind, PaymentActionKind::PayLife { amount: 2 }))
        .unwrap();
    r.preferences = replan(&g, &o, &second.id);
    let c = context(&g, &r);
    let o = open(&g, &c);
    g.pending_decision = Some(DecisionContext::ManaPayment(c.clone()));
    check(
        "life.projected",
        "contract",
        g.manabrew_projected_payment_state(alice).unwrap().1 == 16
            && payment(&o).can_confirm_from_pool,
        "four life selected; manual confirmation enabled".into(),
    );
    let result = execute_mana_payment_plan(
        &mut g.game,
        &r,
        &c.plan,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    );
    check(
        "life.commit",
        "engine",
        matches!(result, Ok(ManaPaymentExecution::Paid))
            && g.game.player(alice).unwrap().life == 16
            && g.game.player(alice).unwrap().mana_pool.black == 2,
        format!("{result:?}"),
    );

    // A filter with an unpaid activation cost is available through the manual engine path.
    let mut g = fixture();
    land(&mut g, vec![ManaSymbol::Colorless]);
    let filter = card(
        &mut g,
        "Audit filter",
        vec![CardType::Artifact],
        Zone::Battlefield,
    );
    g.game
        .object_mut(filter)
        .unwrap()
        .abilities_mut()
        .push(ironsmith::ability::Ability::mana(
            ironsmith::cost::TotalCost::from_costs(vec![
                ironsmith::costs::Cost::mana(ManaCost::new().add_generic(1)),
                ironsmith::costs::Cost::tap(),
            ]),
            vec![ManaSymbol::Blue],
        ));
    let r = request(&mut g, ManaCost::from_symbols(vec![ManaSymbol::Blue]));
    let o = open(&g, &context(&g, &r));
    let engine_offers = manual_mana_abilities(&g.game, &r).contains(&(filter, 0));
    let adapter_offers = payment(&o).actions.iter().any(|a| matches!(&a.kind, PaymentActionKind::ActivateManaAbility(v) if v.card_id == object_id(&g.game, filter)));
    check(
        "nested.activation",
        "choice_coverage",
        engine_offers && adapter_offers,
        format!("engine offers={engine_offers}, adapter offers={adapter_offers}"),
    );

    // Request restrictions and multiple abilities on the same permanent.
    let mut g = fixture();
    let source = land(&mut g, vec![ManaSymbol::Blue]);
    g.game
        .object_mut(source)
        .unwrap()
        .abilities_mut()
        .push(ironsmith::ability::Ability::mana(
            ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::tap()),
            vec![ManaSymbol::Red],
        ));
    let mut r = request(&mut g, ManaCost::new().add_generic(1));
    let o = open(&g, &context(&g, &r));
    check(
        "multi_ability.offered",
        "contract",
        payment(&o)
            .actions
            .iter()
            .filter(|a| matches!(a.kind, PaymentActionKind::ActivateManaAbility(_)))
            .count()
            == 2,
        "two abilities get distinct advertised actions".into(),
    );
    let red = payment(&o).actions.iter().find(|a| matches!(&a.kind, PaymentActionKind::ActivateManaAbility(v) if v.ability_index == 1)).unwrap();
    r.preferences = replan(&g, &o, &red.id);
    let c = context(&g, &r);
    check(
        "multi_ability.exact",
        "contract",
        c.plan.mana_ability_steps.len() == 1 && c.plan.mana_ability_steps[0].ability_index == 1,
        "red selection is preserved".into(),
    );
    let o = open(&g, &c);
    check(
        "multi_ability.exclusive",
        "contract",
        !payment(&o)
            .actions
            .iter()
            .any(|a| matches!(a.kind, PaymentActionKind::ActivateManaAbility(_))),
        "cannot tap selected permanent twice".into(),
    );
    r.preferences = ManaPaymentPreferences::default();
    r.allow_mana_abilities = false;
    let o = open(&g, &context(&g, &r));
    check(
        "restriction.no_mana_abilities",
        "rules",
        !payment(&o)
            .actions
            .iter()
            .any(|a| matches!(a.kind, PaymentActionKind::ActivateManaAbility(_))),
        "request disallows activating mana abilities".into(),
    );

    let mut g = fixture();
    let source = card(
        &mut g,
        "Repeatable",
        vec![CardType::Artifact],
        Zone::Battlefield,
    );
    g.game
        .object_mut(source)
        .unwrap()
        .abilities_mut()
        .push(ironsmith::ability::Ability {
            kind: ironsmith::ability::AbilityKind::Activated(
                ironsmith::ability::ActivatedAbility::mana_with_costs(
                    ironsmith::cost::TotalCost::free(),
                    vec![],
                    vec![ManaSymbol::Blue],
                ),
            ),
            functional_zones: vec![Zone::Battlefield],
        });
    let mut r = request(&mut g, ManaCost::new().add_generic(2));
    for _ in 0..2 {
        let o = open(&g, &context(&g, &r));
        let a = payment(&o)
            .actions
            .iter()
            .find(|a| matches!(a.kind, PaymentActionKind::ActivateManaAbility(_)))
            .unwrap();
        r.preferences = replan(&g, &o, &a.id);
    }
    let c = context(&g, &r);
    let o = open(&g, &c);
    check(
        "repeat.count",
        "contract",
        c.plan.mana_ability_steps.len() == 2 && payment(&o).can_confirm_from_pool,
        "same ability may be selected twice when repeatable".into(),
    );
    let undo = payment(&o)
        .actions
        .iter()
        .find(|a| matches!(a.kind, PaymentActionKind::UndoMana { .. }))
        .unwrap();
    check(
        "repeat.undo_one",
        "contract",
        replan(&g, &o, &undo.id).required_activations.len() == 1,
        "undo removes one occurrence".into(),
    );

    // Choice-bearing activation must let the player select the sacrificed creature.
    let mut g = fixture();
    let tower = g.game.create_object_from_definition(
        &ironsmith_registry_test::cards::definitions::phyrexian_tower(),
        alice,
        Zone::Battlefield,
    );
    card(
        &mut g,
        "Victim one",
        vec![CardType::Creature],
        Zone::Battlefield,
    );
    card(
        &mut g,
        "Victim two",
        vec![CardType::Creature],
        Zone::Battlefield,
    );
    let r = request(&mut g, ManaCost::new().add_generic(1));
    let o = open(&g, &context(&g, &r));
    let tower_action = payment(&o).actions.iter().find(|a| matches!(&a.kind, PaymentActionKind::ActivateManaAbility(v) if v.card_id == object_id(&g.game, tower) && v.ability_index == 1));
    let routes_interactively = tower_action.is_some_and(|a| {
        matches!(
            g.manabrew_response_action(
                &o,
                PromptOutput::PayManaCost(PayManaCostOutput::Act {
                    action_id: a.id.clone()
                })
            ),
            Ok(ManabrewResponseAction::Dispatch(UiCommand::ManaPayment {
                response: ManaPaymentCommand::Activate { .. }
            }))
        )
    });
    check(
        "sacrifice.advertised",
        "choice_coverage",
        tower_action.is_some(),
        format!(
            "tower sacrifice offered={}; routes to interactive Activate={routes_interactively}",
            tower_action.is_some()
        ),
    );

    for (name, cost, mana, any_color, expected) in [
        (
            "wrong_color",
            ManaSymbol::Blue,
            ManaSymbol::Red,
            false,
            false,
        ),
        (
            "explicit_colorless",
            ManaSymbol::Colorless,
            ManaSymbol::Blue,
            false,
            false,
        ),
        (
            "as_any_color",
            ManaSymbol::Blue,
            ManaSymbol::Red,
            true,
            true,
        ),
        (
            "any_color_not_colorless",
            ManaSymbol::Colorless,
            ManaSymbol::Blue,
            true,
            false,
        ),
    ] {
        let mut g = fixture();
        g.game.player_mut(alice).unwrap().mana_pool.add(mana, 1);
        let mut r = request(&mut g, ManaCost::from_symbols(vec![cost]));
        r.spend_policy = ironsmith::player::ManaSpendPolicy::from_any_color(any_color);
        let c = context(&g, &r);
        check(
            &format!("spend_policy.{name}"),
            "engine",
            c.plan.payable == expected,
            format!("payable={}, expected={expected}", c.plan.payable),
        );
    }
    for snow in [false, true] {
        let mut g = fixture();
        let def = CardDefinitionBuilder::new(CardId::new(), "Snow audit land")
            .card_types(vec![CardType::Land])
            .supertypes(if snow {
                vec![ironsmith::types::Supertype::Snow]
            } else {
                vec![]
            })
            .with_ability(ironsmith::ability::Ability::mana(
                ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::tap()),
                vec![ManaSymbol::Blue],
            ))
            .build();
        g.game
            .create_object_from_definition(&def, alice, Zone::Battlefield);
        let r = request(&mut g, ManaCost::from_symbols(vec![ManaSymbol::Snow]));
        let c = context(&g, &r);
        check(
            &format!("snow.source_{snow}"),
            "engine",
            c.plan.payable == snow,
            format!("payable={}, snow source={snow}", c.plan.payable),
        );
        if c.plan.payable {
            let result = execute_mana_payment_plan(
                &mut g.game,
                &r,
                &c.plan,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            );
            check(
                "snow.commit",
                "engine",
                matches!(result, Ok(ManaPaymentExecution::Paid)),
                format!("{result:?}"),
            );
        }
    }
    for (name, life, allowed) in [("disabled", 20, false), ("insufficient", 1, true)] {
        let mut g = fixture();
        g.game.player_mut(alice).unwrap().life = life;
        let mut r = request(
            &mut g,
            ManaCost::from_pips(vec![vec![ManaSymbol::Black, ManaSymbol::Life(2)]]),
        );
        r.allow_life_payment = allowed;
        let o = open(&g, &context(&g, &r));
        check(
            &format!("life.{name}"),
            "rules",
            !payment(&o)
                .actions
                .iter()
                .any(|a| matches!(a.kind, PaymentActionKind::PayLife { .. })),
            "no illegal life-payment action advertised".into(),
        );
    }

    for mode in ["auto", "manual", "cancel", "undo_then_auto"] {
        let (mut g, land, spell) = live_fixture();
        if mode != "auto" {
            let p = g.ensure_manabrew_prompt().unwrap().unwrap();
            let PromptInput::PayManaCost(p) = p.input else {
                panic!("payment required")
            };
            let action = p
                .actions
                .iter()
                .find(|a| matches!(a.kind, PaymentActionKind::ActivateManaAbility(_)))
                .unwrap();
            live_response(
                &mut g,
                PayManaCostOutput::Act {
                    action_id: action.id.clone(),
                },
            );
            check(
                &format!("live.{mode}.reissued"),
                "lifecycle",
                matches!(g.pending_decision, Some(DecisionContext::ManaPayment(_)))
                    && g.priority_state.pending_cast.is_some(),
                "act retains payment and pending cast".into(),
            );
        }
        if mode == "undo_then_auto" {
            let p = g.ensure_manabrew_prompt().unwrap().unwrap();
            let PromptInput::PayManaCost(p) = p.input else {
                panic!("payment required")
            };
            let undo = p
                .actions
                .iter()
                .find(|a| matches!(a.kind, PaymentActionKind::UndoMana { .. }))
                .unwrap();
            live_response(
                &mut g,
                PayManaCostOutput::Act {
                    action_id: undo.id.clone(),
                },
            );
        }
        live_response(
            &mut g,
            if mode == "cancel" {
                PayManaCostOutput::Cancel
            } else {
                PayManaCostOutput::Pay {
                    auto: mode != "manual",
                }
            },
        );
        let in_hand = g.game.player(alice).unwrap().hand.iter().any(|id| {
            g.game
                .object(*id)
                .is_some_and(|c| c.name == "Live audit spell")
        });
        let on_stack = g.game.stack.iter().any(|s| {
            g.game
                .object(s.object_id)
                .is_some_and(|c| c.name == "Live audit spell")
        });
        check(
            &format!("live.{mode}.final"),
            "lifecycle",
            g.priority_state.pending_cast.is_none()
                && if mode == "cancel" {
                    in_hand && !on_stack && !g.game.is_tapped(land)
                } else {
                    !in_hand && on_stack && g.game.is_tapped(land)
                },
            format!(
                "original spell={spell:?}, in_hand={in_hand}, on_stack={on_stack}, land tapped={}",
                g.game.is_tapped(land)
            ),
        );
        check(
            &format!("live.{mode}.pool"),
            "lifecycle",
            g.game.player(alice).unwrap().mana_pool.total() == 0,
            "no extra mana produced or spent twice".into(),
        );
    }

    drop(check);
    if let Ok(path) = std::env::var("IRONSMITH_CONFORMANCE_REPORT") {
        std::fs::write(path, serde_json::to_string_pretty(&rows).unwrap()).unwrap();
    }
    let failures = rows.iter().filter(|r| r["pass"] == false).count();
    println!(
        "CONFORMANCE: {} checks, {} passed, {failures} failed",
        rows.len(),
        rows.len() - failures
    );
    assert_eq!(failures, 0, "see complete conformance matrix above");
}
