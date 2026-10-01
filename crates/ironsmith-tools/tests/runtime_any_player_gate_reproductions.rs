//! Authored restrictions on activations usable by either player.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    for player in [PlayerId(0), PlayerId(1)] {
        for symbol in [ManaSymbol::Red, ManaSymbol::Blue, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 20);
        }
        let land = CardDefinitionBuilder::new(CardId::new(), "Any-player discard land")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_definition(&land, player, Zone::Hand);
        for _ in 0..8 {
            let card = CardDefinitionBuilder::new(CardId::new(), "Any-player library card")
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_definition(&card, player, Zone::Library);
        }
    }
    game
}
fn announce(game: &mut GameState, player: PlayerId, action: LegalAction) -> Result<Value, String> {
    game.turn.priority_player = Some(player);
    let is_mana = matches!(action, LegalAction::ActivateManaAbility { .. });
    let before = game.stack.len();
    let mana = game.player(player).unwrap().mana_pool.total();
    let selected = format!("{action:?}");
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none()
            && state.pending_activation.is_none()
            && ((is_mana && game.player(player).unwrap().mana_pool.total() > mana)
                || (!is_mana && game.stack.len() > before))
        {
            return Ok(
                json!({"action":selected,"stack_growth":game.stack.len()-before,"mana_delta":game.player(player).unwrap().mana_pool.total() as i64-mana as i64}),
            );
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm)
            .map_err(|e| e.to_string())?;
    }
    Err("announcement budget".into())
}
fn index(def: &CardDefinition) -> Result<usize, String> {
    def.abilities
        .iter()
        .position(|a| match &a.kind {
            AbilityKind::Activated(a) => a
                .additional_restrictions
                .iter()
                .any(|s| s.to_lowercase().starts_with("any player may")),
            _ => false,
        })
        .ok_or("missing any-player activation".into())
}
fn run(
    def: &CardDefinition,
    active: u8,
    activator: u8,
    window: &str,
    storm_stack: bool,
) -> Result<(Value, Value), String> {
    let mut game = setup();
    let mut source = game.create_object_from_definition(
        def,
        PlayerId(0),
        if def.name() == "Lightning Storm" {
            Zone::Hand
        } else {
            Zone::Battlefield
        },
    );
    let stable = game.object(source).unwrap().stable_id;
    let ability_index = index(def)?;
    let mut evidence = json!({"source_owner":0,"active_player":active,"activator":activator,"window":window,"storm_on_stack":storm_stack,"scope":"actual announcement only; ability resolution outside scope"});
    if def.name() == "Lightning Storm" && storm_stack {
        let cast = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==source))
            .ok_or("storm cast unavailable")?;
        evidence["producer_cast"] = announce(&mut game, PlayerId(0), cast)?;
        source = game
            .find_object_by_stable_id(stable)
            .ok_or("storm missing after cast")?;
        if game.object(source).unwrap().zone != Zone::Stack {
            return Err("storm did not reach stack".into());
        }
    }
    if def.name() == "Mana Cache" {
        game.add_counters(source, CounterType::Charge, 3);
    }
    game.turn.active_player = PlayerId(active);
    game.turn.priority_player = Some(PlayerId(activator));
    match window {
        "main" => {
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
        }
        "draw" => {
            game.turn.phase = ironsmith::Phase::Beginning;
            game.turn.step = Some(ironsmith::Step::Draw);
        }
        "upkeep" => {
            game.turn.phase = ironsmith::Phase::Beginning;
            game.turn.step = Some(ironsmith::Step::Upkeep);
        }
        "end" => {
            game.turn.phase = ironsmith::Phase::Ending;
            game.turn.step = Some(ironsmith::Step::End);
        }
        _ => return Err("unknown window".into()),
    }
    game.effect_store.pending_trigger_events.clear();
    let action=compute_legal_actions(&game,PlayerId(activator)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,ability_index:i}|LegalAction::ActivateManaAbility{source:id,ability_index:i} if *id==source && *i==ability_index));
    let legal = action.is_some();
    let mut announced = false;
    if let Some(action) = action {
        evidence["activation"] = announce(&mut game, PlayerId(activator), action)?;
        announced = true;
    }
    evidence["source_zone"] = json!(format!("{:?}", game.object(source).unwrap().zone));
    evidence["charge_counters_after"] = json!(game.counter_count(source, CounterType::Charge));
    Ok((
        json!({"activation_offered":legal,"activation_announced":announced}),
        evidence,
    ))
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "report generator, not a card correctness test"]
fn report_any_player_gates() {
    let cases = vec![
        ("Mana Cache", 0, 0, "main", false, true),
        ("Mana Cache", 0, 1, "main", false, false),
        ("Mana Cache", 1, 1, "main", false, true),
        ("Mana Cache", 1, 1, "end", false, false),
        ("Mana Cache", 0, 0, "upkeep", false, true),
        ("Well of Knowledge", 0, 0, "draw", false, true),
        ("Well of Knowledge", 0, 1, "draw", false, false),
        ("Well of Knowledge", 1, 1, "draw", false, true),
        ("Well of Knowledge", 0, 0, "main", false, false),
        ("Well of Knowledge", 1, 1, "main", false, false),
        ("Lightning Storm", 0, 0, "main", false, false),
        ("Lightning Storm", 0, 1, "main", false, false),
        ("Lightning Storm", 0, 0, "main", true, true),
        ("Lightning Storm", 0, 1, "main", true, true),
    ];
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let mut definitions = HashMap::new();
    let mut artifacts = Vec::new();
    for payload in input["cards"].as_array().unwrap() {
        let name = payload["name"].as_str().unwrap();
        if !cases.iter().any(|r| r.0 == name) {
            continue;
        }
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            payload["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum}));
        definitions.insert(name.to_string(), def);
    }
    let mut rows = Vec::new();
    for (name, active, activator, window, stack, allowed) in cases {
        let expected = json!({"activation_offered":allowed,"activation_announced":allowed});
        let (status, actual, evidence) =
            match run(&definitions[name], active, activator, window, stack) {
                Ok((actual, evidence)) => (
                    if actual == expected {
                        "expected_result_observed"
                    } else {
                        "semantic_mismatch"
                    },
                    actual,
                    evidence,
                ),
                Err(error) => (
                    "fixture_or_announcement_error",
                    json!({"error":error}),
                    Value::Null,
                ),
            };
        rows.push(json!({"card":name,"scenario":{"active_player":active,"activating_player":activator,"window":window,"storm_on_stack":stack},"expected":expected,"actual":actual,"fixture_evidence":evidence,"status":status}));
    }
    let binary = std::env::current_exe().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = json!({"scope":"Strict artifact conditional permissions. Established board and legal priority windows; actual paid Lightning Storm cast and actual activations. Effect correctness not certified.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory)}});
    std::fs::write(
        root.join("reports/runtime-audit/any-player-gate-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
