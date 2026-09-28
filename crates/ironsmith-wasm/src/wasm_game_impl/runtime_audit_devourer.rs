//! Opt-in audit: real native pregame-shell action followed by normal first upkeep.
use super::*;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, ViewCardsContext};
use ironsmith::turn_runner::{TurnAction, TurnRunner};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

struct AuditChoices {
    keep: Option<String>,
    trace: Vec<Value>,
}
impl DecisionMaker for AuditChoices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        let selected = self.keep.is_some();
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = c
            .candidates
            .iter()
            .filter(|o| {
                o.legal
                    && self
                        .keep
                        .as_ref()
                        .is_some_and(|name| g.object(o.id).is_some_and(|o| o.name == *name))
            })
            .take(c.max.unwrap_or(1))
            .map(|o| o.id)
            .collect::<Vec<_>>();
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":selected.iter().filter_map(|id|g.object(*id).map(|o|o.name.to_string())).collect::<Vec<_>>()}));
        selected
    }
    fn view_cards(
        &mut self,
        g: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &ViewCardsContext,
    ) {
        self.trace.push(json!({"choice":"view","viewer":viewer.index(),"context":format!("{ctx:?}"),"cards":cards.iter().filter_map(|id|g.object(*id).map(|o|o.name.to_string())).collect::<Vec<_>>()}));
    }
}
fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut AuditChoices) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        if g.stack.is_empty() {
            return Ok(());
        }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            apply_priority_response_with_dm(
                g,
                q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Err("priority resolution budget".into())
}
fn run(
    defs: &HashMap<String, ironsmith::CardDefinition>,
    reveal: bool,
    keep: Option<&str>,
) -> Result<(Value, Value, Value), String> {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 806);
    let alice = PlayerId(0);
    let bob = PlayerId(1);
    // Library bottom->top: untouched Plains below four distinct cards, then the real opening hand.
    for name in [
        "Plains", "Plains", "Plains", "Swamp", "Mountain", "Island", "Forest",
    ] {
        wasm.game
            .create_object_from_definition(&defs[name], alice, Zone::Library);
    }
    for _ in 0..6 {
        wasm.game
            .create_object_from_definition(&defs["Plains"], alice, Zone::Library);
    }
    wasm.game
        .create_object_from_definition(&defs["Devourer of Destiny"], alice, Zone::Library);
    for _ in 0..14 {
        wasm.game
            .create_object_from_definition(&defs["Plains"], bob, Zone::Library);
    }
    wasm.finish_match_setup(7)?;
    for _ in 0..2 {
        let context = wasm
            .build_pregame_decision()
            .map_err(|_| "native pregame context error")?
            .ok_or("missing keep prompt")?;
        let DecisionContext::Priority(c) = context else {
            return Err("expected pregame priority".into());
        };
        if !c.actions.contains(&LegalAction::KeepOpeningHand) {
            return Err("KeepOpeningHand unavailable".into());
        }
        wasm.apply_pregame_priority_action(LegalAction::KeepOpeningHand)
            .map_err(|_| "keep action failed")?;
        wasm.normalize_pregame_state()
            .map_err(|_| "pregame normalization failed")?;
    }
    let source = wasm
        .game
        .player(alice)
        .unwrap()
        .hand
        .iter()
        .copied()
        .find(|id| {
            wasm.game
                .object(*id)
                .is_some_and(|o| o.name == "Devourer of Destiny")
        })
        .ok_or("source not actually drawn into opening hand")?;
    let actions = wasm.available_pregame_actions(alice);
    let offered = actions
        .iter()
        .find(|a| matches!(a,LegalAction::UsePregameAction{card_id,..}if *card_id==source))
        .cloned();
    if reveal {
        let a = offered.clone().ok_or("opening reveal action unavailable")?;
        wasm.apply_pregame_priority_action(a)
            .map_err(|_| "opening reveal action failed")?;
    }
    let delayed_before = wasm.game.effect_store.delayed_triggers.len();
    let public_reveal = wasm
        .active_audit_viewed_cards
        .iter()
        .any(|v| v.public && v.cards.contains(&source));
    let reoffered = wasm
        .available_pregame_actions(alice)
        .iter()
        .any(|a| matches!(a,LegalAction::UsePregameAction{card_id,..}if *card_id==source));
    for _ in 0..2 {
        wasm.apply_pregame_priority_action(LegalAction::ContinuePregame)
            .map_err(|_| "continue pregame failed")?;
        wasm.normalize_pregame_state()
            .map_err(|_| "pregame normalization failed")?;
    }
    if wasm.pregame.is_some() {
        return Err("pregame did not finish".into());
    }
    let mut runner = TurnRunner::new();
    let mut dm = AuditChoices {
        keep: keep.map(str::to_string),
        trace: vec![],
    };
    let mut upkeep = false;
    let mut error = None;
    for _ in 0..24 {
        match runner
            .advance(&mut wasm.game, &mut wasm.trigger_queue)
            .map_err(|e| e.to_string())?
        {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                let is_upkeep = wasm.game.turn.step == Some(Step::Upkeep);
                error = finish(&mut wasm.game, &mut wasm.trigger_queue, &mut dm).err();
                runner.priority_done();
                if is_upkeep {
                    upkeep = true;
                    break;
                }
                if error.is_some() {
                    break;
                }
            }
            other => return Err(format!("unexpected first-turn runner decision:{other:?}")),
        }
    }
    if !upkeep {
        return Err("first upkeep was not reached".into());
    }
    let library = wasm
        .game
        .player(alice)
        .unwrap()
        .library
        .iter()
        .filter_map(|id| wasm.game.object(*id).map(|o| o.name.to_string()))
        .collect::<Vec<_>>();
    let mut exile = wasm
        .game
        .objects_in_deterministic_order()
        .iter()
        .filter(|o| o.zone == Zone::Exile && o.owner == alice)
        .map(|o| o.name.to_string())
        .collect::<Vec<_>>();
    exile.sort();
    let mut expected_library = vec!["Plains".to_string(); 3];
    let mut expected_exile = vec![];
    if reveal {
        if let Some(kept) = keep {
            expected_library.push(kept.to_string());
        }
        expected_exile = ["Forest", "Island", "Mountain", "Swamp"]
            .into_iter()
            .filter(|name| Some(*name) != keep)
            .map(str::to_string)
            .collect();
    } else {
        expected_library.extend(["Swamp", "Mountain", "Island", "Forest"].map(str::to_string));
    }
    expected_exile.sort();
    let actual = json!({"error":error,"reveal_offered":offered.is_some(),"public_reveal":public_reveal,"reoffered_after_use":reoffered,"scheduled_delayed":delayed_before,"remaining_delayed":wasm.game.effect_store.delayed_triggers.len(),"library_bottom_to_top":library,"exile":exile,"source_still_hand":wasm.game.player(alice).unwrap().hand.contains(&source),"stack_length":wasm.game.stack.len()});
    let expected = json!({"error":null,"reveal_offered":true,"public_reveal":reveal,"reoffered_after_use":!reveal,"scheduled_delayed":usize::from(reveal),"remaining_delayed":0,"library_bottom_to_top":expected_library,"exile":expected_exile,"source_still_hand":true,"stack_length":0});
    Ok((
        expected,
        actual,
        json!({"opening_actions":format!("{actions:?}"),"opening_source":source.0,"first_upkeep_reached":upkeep,"choice_trace":dm.trace,"entry_route":"actual library draw7, advertised shell KeepOpeningHand/UsePregameAction/ContinuePregame methods, normal TurnRunner first upkeep and native priority. JS serialization/dispatch itself is not exercised."}),
    ))
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "canonical Devourer pregame audit"]
fn runtime_audit_devourer_pregame() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let data: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in data["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if ![
            "Devourer of Destiny",
            "Plains",
            "Island",
            "Mountain",
            "Forest",
            "Swamp",
        ]
        .contains(&n)
        {
            continue;
        }
        let (a, d) = ironsmith_registry_test::compile_builder_to_artifact(
            ironsmith_dynamic_compile::CompilerCardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(n),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(json!({"card":n,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        defs.insert(n.to_string(), d);
    }
    let mut rows = vec![];
    for (reveal, keep) in [
        (false, None),
        (true, None),
        (true, Some("Forest")),
        (true, Some("Mountain")),
    ] {
        let (e, a, evidence) = match run(&defs, reveal, keep) {
            Ok(r) => r,
            Err(error) => (Value::Null, json!({"error":error}), Value::Null),
        };
        let status = if e.is_null() {
            "fixture_or_pregame_error"
        } else if e == a {
            "expected_result_observed"
        } else if !a["error"].is_null() {
            "runtime_error"
        } else {
            "semantic_mismatch"
        };
        rows.push(json!({"card":"Devourer of Destiny","scenario":{"reveal":reveal,"keep":keep},"status":status,"expected":e,"actual":a,"evidence":evidence}));
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let out = json!({"scope":"Native WASM-session pregame action followed by real first upkeep; no delayed trigger injection. JS binding serialization and browser worker not replayed.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-wasm/src/wasm_game_impl/runtime_audit_devourer.rs")),"stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/devourer-pregame-reproductions.json"),
        serde_json::to_string_pretty(&out).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
