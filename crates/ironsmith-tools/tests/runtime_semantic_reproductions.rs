//! Opt-in semantic audit. The test passing means the report was generated, NOT
//! that every card passed. Inspect each report row's status and expected/actual.
//! Run with cargo test -p ironsmith-tools --test runtime_semantic_reproductions
//! -- --ignored --nocapture. Engine bugs are deliberately left unfixed here.

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_priority_response_with_dm,
    drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::StackEntry;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, Effect, GameState, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;

fn fingerprint(path: &std::path::Path) -> Value {
    let mut reader = std::fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let size = reader.read(&mut buffer).unwrap();
        if size == 0 {
            break;
        }
        digest.update(&buffer[..size]);
    }
    let sha256: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    json!({"path": path.display().to_string(), "sha256": sha256})
}

fn player(index: u8) -> PlayerId {
    PlayerId::from_index(index)
}

fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = player(0);
    game.turn.priority_player = Some(player(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}

fn filler() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Semantic audit filler")
        .card_types(vec![CardType::Land])
        .build()
}

fn populate(game: &mut GameState, definition: &CardDefinition, seat: u8, zone: Zone, count: usize) {
    for _ in 0..count {
        game.create_object_from_definition(definition, player(seat), zone);
    }
}

fn resolve_pending(game: &mut GameState) -> Result<usize, String> {
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut resolved = 0;
    for _ in 0..32 {
        drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(resolved);
        }
        resolve_stack_entry_with(game, &mut dm).map_err(|e| e.to_string())?;
        resolved += 1;
    }
    Err("bounded fixture exceeded 32 stack resolutions".into())
}

fn master(
    definition: &CardDefinition,
    graveyards_after_death: [usize; 3],
) -> Result<Value, String> {
    let mut game = setup();
    let card = filler();
    populate(&mut game, &card, 0, Zone::Library, 64);
    for (seat, count) in graveyards_after_death.iter().copied().enumerate() {
        populate(
            &mut game,
            &card,
            seat as u8,
            Zone::Graveyard,
            count - usize::from(seat == 0),
        );
    }
    let source = game.create_object_from_definition(definition, player(0), Zone::Battlefield);
    game.move_object_by_effect(source, Zone::Graveyard)
        .ok_or("death move failed")?;
    let sizes: Vec<_> = game.players.iter().map(|p| p.graveyard.len()).collect();
    if sizes != graveyards_after_death {
        return Err(format!("invalid graveyard fixture: {sizes:?}"));
    }
    let resolved = resolve_pending(&mut game)?;
    Ok(json!({"drawn": game.player(player(0)).unwrap().hand.len(), "resolved_triggers": resolved}))
}

fn spinneret(definition: &CardDefinition, amount: u32) -> Result<Value, String> {
    let mut game = setup();
    populate(&mut game, &filler(), 0, Zone::Library, 8);
    let source = game.create_object_from_definition(definition, player(0), Zone::Battlefield);
    // The damage replacement processor alone does not apply damage or enqueue
    // its trigger event. Resolve an ordinary damage effect to exercise both.
    game.push_to_stack(StackEntry::ability(
        source,
        player(0),
        vec![Effect::deal_damage(
            amount as i32,
            ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(
                player(1),
            )),
        )],
    ));
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .map_err(|e| e.to_string())?;
    if game.player(player(1)).unwrap().life != 20 - amount as i32 {
        return Err("fixture damage was not applied".into());
    }
    let resolved = resolve_pending(&mut game)?;
    Ok(json!({"exiled": game.exile.len(), "resolved_triggers": resolved}))
}

fn edge(definition: &CardDefinition, land: &CardDefinition, lands: usize) -> Result<Value, String> {
    let mut game = setup();
    populate(&mut game, land, 0, Zone::Battlefield, lands);
    populate(&mut game, land, 0, Zone::Library, 4);
    let source = game.create_object_from_definition(definition, player(0), Zone::Stack);
    game.push_to_stack(StackEntry::new(source, player(0)));
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .map_err(|e| e.to_string())?;
    Ok(json!({"battlefield_lands_after": game.battlefield.len(),
              "library_after": game.player(player(0)).unwrap().library.len()}))
}

fn lost_isle(definition: &CardDefinition, counters: u32) -> Result<Value, String> {
    let mut game = setup();
    populate(&mut game, &filler(), 0, Zone::Library, 32);
    let source = game.create_object_from_definition(definition, player(0), Zone::Battlefield);
    game.object_mut(source)
        .unwrap()
        .add_counters(CounterType::Named("verse".into()), counters);
    game.player_mut(player(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 6);
    let action = compute_legal_actions(&game, player(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
        .ok_or("fixture has no legal activation")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..12 {
        if !game.stack.is_empty() {
            break;
        }
        let response = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref ctx)) => {
                let index = ctx
                    .options
                    .iter()
                    .find(|o| o.legal)
                    .ok_or("no legal cost option")?
                    .index;
                PriorityResponse::NextCostChoice(index)
            }
            GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ref ctx)) => {
                PriorityResponse::ManaPaymentPlan(
                    ironsmith::mana_payment::ManaPaymentResponse::Confirm {
                        plan_id: ctx.plan.id,
                        request_hash: ctx.plan.request_hash,
                    },
                )
            }
            ref other => {
                return Err(format!(
                    "unsupported activation fixture decision: {other:?}"
                ));
            }
        };
        progress =
            apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &response, &mut dm)
                .map_err(|e| e.to_string())?;
    }
    if game.stack.is_empty() {
        return Err("activation fixture did not reach stack".into());
    }
    if !game.exile.iter().any(|id| {
        game.object(*id)
            .is_some_and(|o| o.name == definition.name())
    }) {
        return Err("source was not exiled to pay activation cost".into());
    }
    resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| e.to_string())?;
    Ok(
        json!({"drawn": game.player(player(0)).unwrap().hand.len(), "extra_turns": game.turn_store.extra_turns.len()}),
    )
}

fn tunnel_ignus(
    definition: &CardDefinition,
    land: &CardDefinition,
    lands: usize,
) -> Result<Value, String> {
    let mut game = setup();
    game.create_object_from_definition(definition, player(1), Zone::Battlefield);
    let mut resolved = 0;
    for expected_count in 1..=lands {
        let source = game.create_object_from_definition(land, player(0), Zone::Hand);
        game.move_object_by_effect(source, Zone::Battlefield)
            .ok_or("land move failed")?;
        resolved += resolve_pending(&mut game)?;
        let recorded = game
            .turn_store
            .turn_history
            .lands_entered_under_controller(player(0));
        if recorded != expected_count as u32 {
            return Err(format!(
                "fixture land entry history: expected {expected_count}, got {recorded}"
            ));
        }
    }
    Ok(json!({"alice_life": game.player(player(0)).unwrap().life,
              "resolved_triggers": resolved}))
}

fn ramos(definition: &CardDefinition, colors: usize) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, player(0), Zone::Battlefield);
    let symbols = match colors {
        0 => vec![ManaSymbol::Generic(1)],
        1 => vec![ManaSymbol::Blue],
        2 => vec![ManaSymbol::Blue, ManaSymbol::Red],
        _ => return Err("unsupported fixture color count".into()),
    };
    let spell = CardDefinitionBuilder::new(CardId::new(), "Semantic audit color fixture")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(symbols))
        .build();
    let spell_id = game.create_object_from_definition(&spell, player(0), Zone::Hand);
    let observed_colors = game
        .current_colors(spell_id)
        .ok_or("fixture spell has no colors")?
        .count();
    if observed_colors != colors as u32 {
        return Err(format!(
            "fixture spell colors: expected {colors}, got {observed_colors}"
        ));
    }
    game.player_mut(player(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    game.player_mut(player(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 2);
    let action = compute_legal_actions(&game, player(0)).expect("fixture has complete replacement state").into_iter()
        .find(|action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id))
        .ok_or("fixture has no legal spell cast")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..12 {
        if !game.stack.is_empty() {
            break;
        }
        let response = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref ctx)) => {
                PriorityResponse::NextCostChoice(
                    ctx.options
                        .iter()
                        .find(|o| o.legal)
                        .ok_or("no legal cost option")?
                        .index,
                )
            }
            GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ref ctx)) => {
                PriorityResponse::ManaPaymentPlan(
                    ironsmith::mana_payment::ManaPaymentResponse::Confirm {
                        plan_id: ctx.plan.id,
                        request_hash: ctx.plan.request_hash,
                    },
                )
            }
            ref other => return Err(format!("unsupported cast fixture decision: {other:?}")),
        };
        progress =
            apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &response, &mut dm)
                .map_err(|e| e.to_string())?;
    }
    if game.stack.is_empty() {
        return Err("casting fixture did not reach stack".into());
    }
    drain_pending_trigger_events(&mut game, &mut queue);
    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).map_err(|e| e.to_string())?;
    resolve_pending(&mut game)?;
    Ok(
        json!({"plus_one_counters": game.object(source).ok_or("Ramos disappeared")?
        .counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0)}),
    )
}

fn confounding_conundrum(
    definition: &CardDefinition,
    land: &CardDefinition,
    lands: usize,
) -> Result<Value, String> {
    let mut game = setup();
    game.create_object_from_definition(definition, player(1), Zone::Battlefield);
    let mut resolved = 0;
    for expected_count in 1..=lands {
        let source = game.create_object_from_definition(land, player(0), Zone::Hand);
        game.move_object_by_effect(source, Zone::Battlefield)
            .ok_or("land move failed")?;
        resolved += resolve_pending(&mut game)?;
        let recorded = game
            .turn_store
            .turn_history
            .lands_entered_under_controller(player(0));
        if recorded != expected_count as u32 {
            return Err(format!(
                "fixture land entry history: expected {expected_count}, got {recorded}"
            ));
        }
    }
    let battlefield_lands = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|object| object.card_types.contains(&CardType::Land))
        })
        .count();
    Ok(json!({"lands_on_battlefield": battlefield_lands,
              "lands_returned_to_hand": game.player(player(0)).unwrap().hand.len(),
              "resolved_triggers": resolved}))
}

fn carpet(
    definition: &CardDefinition,
    island: &CardDefinition,
    islands: usize,
    active: u8,
) -> Result<Value, String> {
    let mut game = setup();
    game.turn.active_player = player(active);
    game.turn.priority_player = Some(player(active));
    game.create_object_from_definition(definition, player(0), Zone::Battlefield);
    populate(&mut game, island, 1, Zone::Battlefield, islands);
    let event = ironsmith::triggers::generate_step_trigger_events(&game)
        .ok_or("fixture phase generated no event")?;
    let mut queue = TriggerQueue::new();
    for entry in ironsmith::triggers::check_triggers(&game, &event) {
        queue.add(entry);
    }
    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
        .map_err(|e| e.to_string())?;
    let resolved = resolve_pending(&mut game)?;
    Ok(
        json!({"mana_added": game.player(player(0)).unwrap().mana_pool.total(),
              "resolved_triggers": resolved}),
    )
}

fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: Value,
    expected: Value,
    result: Result<Value, String>,
    scope: &str,
) {
    let (status, actual) = match result {
        Ok(actual) if actual == expected => ("passed", actual),
        Ok(actual) => ("semantic_mismatch", actual),
        Err(error) => ("execution_or_fixture_error", json!({"error": error})),
    };
    rows.push(json!({"card": card, "scenario": scenario, "status": status,
                     "expected": expected, "actual": actual, "scope": scope}));
}

#[test]
#[ignore = "manual audit emits observed semantic mismatches; inspect report statuses"]
fn report_semantic_execution_results() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fingerprint_paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_semantic_reproductions.rs"),
    ];
    let fingerprints_before: Vec<_> = fingerprint_paths
        .iter()
        .map(|path| fingerprint(path))
        .collect();
    let names = [
        "The Master of Lake-town",
        "Spinneret and Spiderling",
        "Edge of Autumn",
        "Lost Isle Calling",
        "Tunnel Ignus",
        "Ramos, Dragon Engine",
        "Confounding Conundrum",
        "Carpet of Flowers",
        "Island",
        "Forest",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let definitions: HashMap<_, _> = payloads
        .into_values()
        .flatten()
        .map(|payload| {
            let definition =
                ironsmith_tools::compile_runtime_definition_from_payload(&payload).unwrap();
            (payload.name, definition)
        })
        .collect();
    let mut rows = Vec::new();
    for sizes in [[6, 0, 0], [7, 0, 0], [8, 0, 0], [6, 7, 8], [7, 7, 7]] {
        let name = "The Master of Lake-town";
        record(
            &mut rows,
            name,
            json!({"graveyards_after_death": sizes}),
            json!({"drawn": sizes.iter().filter(|n| **n >= 7).count(), "resolved_triggers": 1}),
            master(&definitions[name], sizes),
            "actual source zone change, event drain, queue, stack, draws",
        );
    }
    for amount in [1, 3, 4, 5] {
        let expected = usize::from(amount >= 4);
        let name = "Spinneret and Spiderling";
        record(
            &mut rows,
            name,
            json!({"damage": amount}),
            json!({"exiled": expected, "resolved_triggers": expected}),
            spinneret(&definitions[name], amount),
            "damage event producer, queue, stack, exile outcome",
        );
    }
    for lands in [0, 4, 5, 7] {
        let found = usize::from(lands <= 4);
        let name = "Edge of Autumn";
        record(
            &mut rows,
            name,
            json!({"lands_before": lands}),
            json!({"battlefield_lands_after": lands + found, "library_after": 4 - found}),
            edge(&definitions[name], &definitions["Forest"], lands),
            "spell stack resolution with legal searchable Forest; casting costs bypassed",
        );
    }
    for counters in [5, 6, 7, 8] {
        let name = "Lost Isle Calling";
        record(
            &mut rows,
            name,
            json!({"verse_counters": counters}),
            json!({"drawn": counters, "extra_turns": usize::from(counters >= 7)}),
            lost_isle(&definitions[name], counters),
            "legal activation and paid exile cost, then stack resolution",
        );
    }
    for lands in [1, 2, 3] {
        let name = "Tunnel Ignus";
        record(
            &mut rows,
            name,
            json!({"lands_entered_this_turn": lands}),
            json!({"alice_life": 20 - 3 * (lands - 1), "resolved_triggers": lands - 1}),
            tunnel_ignus(&definitions[name], &definitions["Forest"], lands),
            "actual land zone changes and asserted history count, trigger matching, stack resolution",
        );
    }
    for colors in [0, 1, 2] {
        let name = "Ramos, Dragon Engine";
        record(
            &mut rows,
            name,
            json!({"spell_colors": colors}),
            json!({"plus_one_counters": colors}),
            ramos(&definitions[name], colors),
            "legal instant cast with paid mana, normal cast event, queue and stack resolution",
        );
    }
    for lands in [1, 2, 3] {
        let name = "Confounding Conundrum";
        record(
            &mut rows,
            name,
            json!({"lands_entered_this_turn": lands}),
            json!({"lands_on_battlefield": 1, "lands_returned_to_hand": lands - 1,
                   "resolved_triggers": lands - 1}),
            confounding_conundrum(&definitions[name], &definitions["Forest"], lands),
            "actual land zone changes and asserted history count, trigger matching, stack resolution",
        );
    }
    for (islands, active) in [(1, 0), (2, 0), (2, 1)] {
        let name = "Carpet of Flowers";
        let is_own_main = active == 0;
        record(
            &mut rows,
            name,
            json!({"opponent_islands": islands, "active_player": active}),
            json!({"mana_added": if is_own_main { islands } else { 0 },
                   "resolved_triggers": usize::from(is_own_main)}),
            carpet(&definitions[name], &definitions["Island"], islands, active),
            "engine phase-event producer, matcher, target choice and resolution; main phase preconfigured",
        );
    }
    let out = std::env::var_os("IR_RUNTIME_SEMANTIC_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("reports/runtime-audit/semantic-execution.json"));
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let fingerprints_after: Vec<_> = fingerprint_paths
        .iter()
        .map(|path| fingerprint(path))
        .collect();
    let report = json!({"scope": "29 expected-result scenarios on eight suspect cards; not corpus completeness",
                        "provenance": {"compiler": "fresh in-process compilation from cards.json using this test executable",
                          "artifacts_before": fingerprints_before, "artifacts_after": fingerprints_after,
                          "artifacts_unchanged": fingerprints_before == fingerprints_after},
                        "rows": rows});
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Semantic execution report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}
