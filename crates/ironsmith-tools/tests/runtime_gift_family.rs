//! Gift announcement/context audit using strict serialized canonical artifacts.
//! Set AUDIT_RUNTIME_INVENTORY to a saved all-face canonical inventory.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::game_loop::{
    CastStage, PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::StackEntry;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Default)]
struct Decisions {
    choices: Vec<Value>,
}
impl DecisionMaker for Decisions {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let chosen = SelectFirstDecisionMaker.decide_options(game, ctx);
        self.choices.push(json!({"description":ctx.description,"selected":chosen,"options":ctx.options.iter().map(|o|json!({"index":o.index,"description":o.description,"legal":o.legal})).collect::<Vec<_>>() }));
        chosen
    }
}
fn scenario(
    def: &CardDefinition,
    promise: bool,
    needs_stack_target: bool,
) -> Result<(Value, Value), String> {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(71757432704846);
    game.turn.turn_number = 3;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    let creature = CardDefinitionBuilder::new(CardId::new(), "Gift audit creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 3))
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    let land = CardDefinitionBuilder::new(CardId::new(), "Gift audit land")
        .card_types(vec![CardType::Land])
        .build();
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Gift audit artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let enchantment = CardDefinitionBuilder::new(CardId::new(), "Gift audit enchantment")
        .card_types(vec![CardType::Enchantment])
        .build();
    for player in [PlayerId(0), PlayerId(1)] {
        for item in [&creature, &land, &artifact, &enchantment] {
            game.create_object_from_definition(item, player, Zone::Battlefield);
            game.create_object_from_definition(item, player, Zone::Hand);
            game.create_object_from_definition(item, player, Zone::Graveyard);
            for _ in 0..8 {
                game.create_object_from_definition(item, player, Zone::Library);
            }
        }
        for _ in 0..3 {
            game.create_object_from_definition(&creature, player, Zone::Graveyard);
        }
        for mana in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(mana, 12);
        }
    }
    if needs_stack_target {
        let id = game.create_object_from_definition(&creature, PlayerId(1), Zone::Stack);
        game.push_to_stack(StackEntry::new(id, PlayerId(1)));
    }
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    game.effect_store.pending_trigger_events.clear();
    let action = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==source))
        .ok_or("fixture has no legal cast")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Decisions::default();
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    let mut chose_gift = false;
    for _ in 0..64 {
        if state.pending_cast.is_none()
            && game
                .stack
                .last()
                .is_some_and(|entry| entry.controller == PlayerId(0))
        {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stalled: {progress:?}"));
        };
        if state
            .pending_cast
            .as_ref()
            .is_some_and(|p| matches!(p.stage, CastStage::ChoosingOptionalCosts))
        {
            let indices: Vec<_> = game
                .object(state.pending_cast.as_ref().unwrap().spell_id)
                .unwrap()
                .optional_costs
                .iter()
                .enumerate()
                .filter(|(_, cost)| format!("{cost:?}").contains("Gift"))
                .map(|(i, _)| (i, 1))
                .collect();
            if indices.len() != 1 {
                return Err(format!(
                    "fixture expected exactly one Gift optional cost, got {indices:?}"
                ));
            }
            let chosen = if promise { indices } else { Vec::new() };
            chose_gift = true;
            progress = apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::OptionalCosts(chosen),
                &mut dm,
            )
            .map_err(|e| e.to_string())?;
        } else {
            progress =
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
                    .map_err(|e| e.to_string())?;
        }
    }
    let entry = game.stack.last().ok_or("spell did not reach stack")?;
    let promised = entry.optional_costs_paid.was_paid_label("Gift");
    if !chose_gift || promised != promise || entry.controller != PlayerId(0) {
        return Err(format!(
            "gift fixture announcement mismatch: selected={chose_gift}, expected={promise}, actual={promised}"
        ));
    }
    let evidence = json!({"optional_gift_was_paid":promised,"stack_chosen_player":entry.chosen_player.map(|p|p.index()),
        "object_chosen_player":game.chosen_player(entry.object_id).map(|p|p.index()),
        "stack_targets":format!("{:?}",entry.targets),"choices":dm.choices,"remaining_mana":game.player(PlayerId(0)).unwrap().mana_pool.total()});
    let mut resolution_error = None;
    let mut resolved = 0;
    for _ in 0..32 {
        drain_pending_trigger_events(&mut game, &mut queue);
        if let Err(e) = put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm) {
            resolution_error = Some(e.to_string());
            break;
        }
        if game.stack.is_empty() {
            break;
        }
        if let Err(e) = resolve_stack_entry_with(&mut game, &mut dm) {
            resolution_error = Some(e.to_string());
            break;
        }
        resolved += 1;
    }
    if resolution_error.is_none() && !game.stack.is_empty() {
        return Err(format!("resolution budget exhausted with {} entries remaining", game.stack.len()));
    }
    Ok((
        json!({"resolution_error":resolution_error}),
        json!({"announcement":evidence,"resolved_entries":resolved,"remaining_stack":game.stack.len(),
        "alice_life":game.player(PlayerId(0)).unwrap().life,"bob_life":game.player(PlayerId(1)).unwrap().life}),
    ))
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "report generator; passing does not certify gameplay"]
fn report_gift_family() {
    let inventory = std::path::PathBuf::from(
        std::env::var("AUDIT_RUNTIME_INVENTORY")
            .expect("set AUDIT_RUNTIME_INVENTORY to canonical saved inventory.json"),
    );
    let data: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let binary = std::env::current_exe().unwrap();
    let binary_before = hash(&binary);
    let mut rows = Vec::new();
    let mut selected = 0;
    for payload in data["cards"].as_array().unwrap() {
        let oracle = payload["oracle_text"].as_str().unwrap();
        if !oracle
            .lines()
            .any(|line| line.starts_with("Gift a ") || line.starts_with("Gift an "))
        {
            continue;
        }
        selected += 1;
        let name = payload["name"].as_str().unwrap();
        let parsed = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                payload["parse_name"].as_str().unwrap_or(name),
            ),
            payload["parse_input"].as_str().unwrap(),
            false,
        );
        let (artifact, definition) = match parsed {
            Ok(value) => value,
            Err(error) => {
                rows.push(json!({"card":name,"status":"compile_or_materialization_failed","actual":{"error":error.to_string()}}));
                continue;
            }
        };
        for promise in [false, true] {
            let expected = json!({"resolution_error":null});
            let (status, actual, evidence) =
                match scenario(
                    &definition,
                    promise,
                    oracle.contains("target spell") || oracle.contains("Counter target"),
                ) {
                    Ok((actual, evidence)) if actual == expected => {
                        ("resolution_completed", actual, evidence)
                    }
                    Ok((actual, evidence)) => ("resolution_failed", actual, evidence),
                    Err(error) => (
                        "fixture_or_announcement_error",
                        json!({"error":error}),
                        Value::Null,
                    ),
                };
            rows.push(json!({"card":name,"status":status,"scenario":{"promise_gift":promise},"expected":expected,"actual":actual,"fixture_evidence":evidence,
                "artifact_checksum":artifact.payload_checksum,"scope":"strict serialized canonical artifact; engine-offered paid cast, explicitly accept/decline Gift, recorded recipient choices and stack context, bounded spell/trigger resolution; outcome semantics and follow-up turns not certified"}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(inventory.parent().unwrap().join("manifest.json")).unwrap(),
    )
    .unwrap();
    let report = json!({"rows":rows,"selected_cards":selected,"provenance":{"binary":binary,"binary_sha256":binary_before,"binary_unchanged":binary_before==hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory),"cards_sha256":manifest["cards_sha256"]}});
    std::fs::write(
        root.join("reports/runtime-audit/gift-family-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
