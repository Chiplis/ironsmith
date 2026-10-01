//! Opt-in actual-payment boundary probes for mana restrictions.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;

fn hash(path: &std::path::Path) -> Value {
    let mut file = std::fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let sha256: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    json!({"path": path.display().to_string(), "sha256": sha256})
}

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;

    game
}

fn cast_or_activate(
    game: &mut GameState,
    id: ObjectId,
    cast: bool,
) -> Result<(ObjectId, u32), String> {
    let stable = game.object(id).ok_or("action source missing")?.stable_id;
    game.turn.priority_player = Some(alice());
    let action = compute_legal_actions(game, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|action| match action {
            LegalAction::CastSpell { spell_id, .. } => cast && *spell_id == id,
            LegalAction::ActivateAbility { source, .. } => !cast && *source == id,
            _ => false,
        })
        .ok_or("fixture has no matching legal action")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|error| error.to_string())?;
    for _ in 0..24 {
        if let Some(result) = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.stable_id == stable)
            })
            .map(|entry| (entry.object_id, entry.mana_spent_on_activation.total()))
        {
            drain_pending_trigger_events(game, &mut queue);
            put_triggers_on_stack_with_dm(game, &mut queue, &mut dm)
                .map_err(|error| error.to_string())?;
            return Ok(result);
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref context))
                if context.description.starts_with("Choose optional costs") =>
            {
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(vec![]),
                    &mut dm,
                )
                .map_err(|error| error.to_string())?
            }
            GameProgress::NeedsDecisionCtx(ref context)
                if !matches!(context, DecisionContext::Priority(_)) =>
            {
                apply_decision_context_with_dm(game, &mut queue, &mut state, context, &mut dm)
                    .map_err(|error| error.to_string())?
            }
            ref other => return Err(format!("fixture action did not reach the stack: {other:?}")),
        };
    }
    Err("fixture exceeded action decision bound".into())
}

fn resolve_all(game: &mut GameState) -> Result<usize, String> {
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut resolved = 0;
    for _ in 0..24 {
        drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, &mut dm)
            .map_err(|error| error.to_string())?;
        if game.stack.is_empty() {
            return Ok(resolved);
        }
        resolve_stack_entry_with(game, &mut dm).map_err(|error| error.to_string())?;
        resolved += 1;
    }
    Err("fixture exceeded resolution bound".into())
}

fn activate_mana(game: &mut GameState, id: ObjectId) -> Result<Value, String> {
    let action = compute_legal_actions(game, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|action| matches!(action, LegalAction::ActivateManaAbility { source,.. } if *source==id))
        .ok_or("fixture mana ability is not legal")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|error| error.to_string())?;
    for _ in 0..20 {
        match progress {
            GameProgress::NeedsDecisionCtx(ref context)
                if !matches!(context, DecisionContext::Priority(_)) =>
            {
                progress =
                    apply_decision_context_with_dm(game, &mut queue, &mut state, context, &mut dm)
                        .map_err(|error| error.to_string())?;
            }
            _ => break,
        }
    }
    if !game.is_tapped(id) || !game.stack.is_empty() {
        return Err(format!(
            "fixture mana activation did not complete immediately: {progress:?}"
        ));
    }
    let player = game.player(alice()).unwrap();
    Ok(
        json!({"produced_mana":player.mana_pool.total(),"restricted_units":format!("{:?}",player.restricted_mana),"restricted_unit_count":player.restricted_mana.len()}),
    )
}

fn scenario(
    definition: &CardDefinition,
    cost: u8,
    creature: bool,
    has_x: bool,
    ability: bool,
    extra: u32,
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    // Helga's eligible cast trigger draws; sufficient neutral cards avoid deck-out noise.
    let land = CardDefinitionBuilder::new(CardId::new(), "Mana restriction audit filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..8 {
        game.create_object_from_definition(&land, alice(), Zone::Library);
    }
    let produced = activate_mana(&mut game, source)?;
    let wanted = if definition.name().starts_with("Helga") {
        1
    } else {
        2
    };
    if produced["produced_mana"] != wanted {
        return Err(format!("unexpected source mana: {produced}"));
    }
    game.player_mut(alice())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, extra);
    let mut symbols = vec![ManaSymbol::Generic(cost)];
    if has_x {
        symbols.push(ManaSymbol::X);
    }
    let mana = ManaCost::from_symbols(symbols);
    let builder = CardDefinitionBuilder::new(CardId::new(), "Mana restriction payment witness")
        .card_types(vec![if ability {
            CardType::Artifact
        } else if creature {
            CardType::Creature
        } else {
            CardType::Sorcery
        }]);
    let witness = if ability {
        builder
            .with_activated(
                ironsmith::TotalCost::mana(mana),
                vec![ironsmith::Effect::gain_life(1)],
            )
            .build()
    } else {
        builder
            .mana_cost(mana)
            .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
            .with_spell_effect(vec![ironsmith::Effect::gain_life(1)])
            .build()
    };
    let id = game.create_object_from_definition(
        &witness,
        alice(),
        if ability {
            Zone::Battlefield
        } else {
            Zone::Hand
        },
    );
    let pool_before = game.player(alice()).unwrap().mana_pool.total();
    let available = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .any(|action| match action {
            LegalAction::CastSpell { spell_id, .. } => !ability && spell_id == id,
            LegalAction::ActivateAbility { source, .. } => ability && source == id,
            _ => false,
        });
    if !available {
        return Ok(
            json!({"source_mana":produced,"payment_completed":false,"legal_action_present":false,"paid":0,"error":null}),
        );
    }
    match cast_or_activate(&mut game, id, !ability) {
        Ok(_) => {
            let paid = pool_before - game.player(alice()).unwrap().mana_pool.total();
            if paid < u32::from(cost) || paid <= extra {
                return Err(format!(
                    "fixture did not consume produced mana: paid {paid}, unrestricted {extra}"
                ));
            }
            let error = resolve_all(&mut game).err();
            Ok(
                json!({"source_mana":produced,"payment_completed":true,"legal_action_present":true,"paid":paid,"error":error}),
            )
        }
        Err(error) => Ok(
            json!({"source_mana":produced,"payment_completed":false,"legal_action_present":true,"paid":pool_before-game.player(alice()).unwrap().mana_pool.total(),"error":error}),
        ),
    }
}

#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_mana_restriction_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_mana_restriction_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|path| hash(path)).collect();
    let names = ["Helga, Skittish Seer", "Troyan, Gutsy Explorer"]
        .map(str::to_owned)
        .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut definitions = HashMap::new();
    let mut compilation = Vec::new();
    for payload in payloads.into_values().flatten() {
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            payload.parse_name.as_deref().unwrap_or(&payload.name),
        );
        match ironsmith_registry::compile_builder_to_artifact(builder,&payload.parse_input,false){
            Ok((artifact,definition))=>{compilation.push(json!({"card":payload.name,"status":"strict_artifact_materialized","artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));definitions.insert(definition.name().to_owned(),definition);},
            Err(error)=>compilation.push(json!({"card":payload.name,"status":"compile_or_materialization_failed","error":error.to_string()})),
        }
    }
    let mut rows = Vec::new();
    for (name, label, cost, creature, has_x, ability, extra, allowed) in [
        (
            "Helga, Skittish Seer",
            "creature MV3 forbidden",
            3,
            true,
            false,
            false,
            2,
            false,
        ),
        (
            "Helga, Skittish Seer",
            "creature MV4 eligible",
            4,
            true,
            false,
            false,
            3,
            true,
        ),
        (
            "Helga, Skittish Seer",
            "noncreature MV4 forbidden",
            4,
            false,
            false,
            false,
            3,
            false,
        ),
        (
            "Helga, Skittish Seer",
            "creature X eligible",
            1,
            true,
            true,
            false,
            0,
            true,
        ),
        (
            "Helga, Skittish Seer",
            "noncreature X forbidden",
            1,
            false,
            true,
            false,
            0,
            false,
        ),
        (
            "Helga, Skittish Seer",
            "activated ability forbidden",
            1,
            false,
            false,
            true,
            0,
            false,
        ),
        (
            "Troyan, Gutsy Explorer",
            "spell MV4 forbidden",
            4,
            false,
            false,
            false,
            2,
            false,
        ),
        (
            "Troyan, Gutsy Explorer",
            "spell MV5 eligible",
            5,
            false,
            false,
            false,
            3,
            true,
        ),
        (
            "Troyan, Gutsy Explorer",
            "spell X eligible",
            2,
            false,
            true,
            false,
            0,
            true,
        ),
        (
            "Troyan, Gutsy Explorer",
            "activated ability forbidden",
            2,
            false,
            false,
            true,
            0,
            false,
        ),
    ] {
        let result = definitions
            .get(name)
            .ok_or_else(|| "strict compilation failed".to_string())
            .and_then(|definition| scenario(definition, cost, creature, has_x, ability, extra));
        let (status, actual) = match result {
            Ok(actual)
                if actual["payment_completed"] == allowed
                    && (!allowed || actual["error"].is_null()) =>
            {
                ("passed", actual)
            }
            Ok(actual) => ("semantic_mismatch", actual),
            Err(error) => ("execution_or_fixture_error", json!({"error":error})),
        };
        rows.push(json!({"card":name,"scenario":label,"expected_payment_allowed":allowed,"status":status,"actual":actual}));
    }
    let after: Vec<_> = paths.iter().map(|path| hash(path)).collect();
    let report = json!({"scope":"Strict canonical artifact mana abilities legally activated, then isolated neutral spell/ability witnesses consume their generated mana. Positive and forbidden-payment boundaries measured; no oracle fallback or assertion repairs.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    let output = root.join("reports/runtime-audit/mana-restriction-execution.json");
    std::fs::write(
        &output,
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("{}", serde_json::to_string(&report).unwrap());
}
