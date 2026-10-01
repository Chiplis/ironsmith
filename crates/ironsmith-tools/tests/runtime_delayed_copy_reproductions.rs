//! Opt-in expected-result probes for delayed copying. A passing Rust test means
//! the report was emitted; inspect row statuses for actual engine correctness.

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::cost::TotalCost;
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
use ironsmith::{CardDefinition, CardId, CardType, Effect, GameState, ObjectId, PlayerId, Zone};
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
    for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Colorless] {
        game.player_mut(alice()).unwrap().mana_pool.add(color, 30);
    }
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

fn gain_spell(cost: u8, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(
        CardId::new(),
        format!("Delayed copy fixture {card_type:?} {cost}"),
    )
    .card_types(vec![card_type])
    .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(cost)]))
    .with_spell_effect(vec![Effect::gain_life(1)])
    .build()
}

fn activate_life(game: &mut GameState, cost: u8) -> Result<Value, String> {
    let ability = CardDefinitionBuilder::new(
        CardId::new(),
        format!("Delayed copy ability fixture {cost}"),
    )
    .card_types(vec![CardType::Artifact])
    .with_activated(
        TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(cost)])),
        vec![Effect::gain_life(1)],
    )
    .build();
    let source = game.create_object_from_definition(&ability, alice(), Zone::Battlefield);
    let (_, spent) = cast_or_activate(game, source, false)?;
    if spent != u32::from(cost) {
        return Err(format!("activation spent {spent}, expected {cost}"));
    }
    let before = game.player(alice()).unwrap().life;
    resolve_all(game)?;
    Ok(json!({"mana_spent": spent, "life_gained": game.player(alice()).unwrap().life - before}))
}

fn dynaheir(definition: &CardDefinition, costs: &[u8], arm: bool) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    if arm {
        cast_or_activate(&mut game, source, false)?;
        if !game.is_tapped(source) {
            return Err("Dynaheir activation did not pay tap cost".into());
        }
        // The activation itself must resolve successfully to establish the delayed trigger.
        if let Err(error) = resolve_all(&mut game) {
            return Ok(
                json!({"arming_resolution_error": error, "tap_cost_paid": true,
                "later_activations": "not exercised because arming resolution failed"}),
            );
        }
    }
    let outcomes = costs
        .iter()
        .map(|cost| activate_life(&mut game, *cost))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({"arming_resolution_error": null, "outcomes": outcomes}))
}

fn sea_gate(
    definition: &CardDefinition,
    spells: &[(u8, CardType)],
    cast_source: bool,
) -> Result<Value, String> {
    let mut game = setup();
    if cast_source {
        let source = game.create_object_from_definition(definition, alice(), Zone::Hand);
        let before_mana = game.player(alice()).unwrap().mana_pool.total();
        let (stack_id, _) = cast_or_activate(&mut game, source, true)?;
        let entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == stack_id)
            .ok_or("Sea Gate spell missing")?;
        if entry.optional_costs_paid.was_kicked() {
            return Err("fixture unexpectedly kicked Sea Gate".into());
        }
        if before_mana - game.player(alice()).unwrap().mana_pool.total() != 2 {
            return Err("Sea Gate spell did not pay its normal two-mana cost".into());
        }
        if let Err(error) = resolve_all(&mut game) {
            return Ok(json!({"source_resolution_error": error, "later_spells": "not exercised"}));
        }
        if !game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|card| card.name == definition.name())
        }) {
            return Err("Sea Gate did not enter from its actual spell cast".into());
        }
    }
    let mut outcomes = Vec::new();
    for (cost, card_type) in spells {
        let spell = gain_spell(*cost, *card_type);
        let source = game.create_object_from_definition(&spell, alice(), Zone::Hand);
        let before = game.player(alice()).unwrap().life;
        let mana = game.player(alice()).unwrap().mana_pool.total();
        cast_or_activate(&mut game, source, true)?;
        let spent = mana - game.player(alice()).unwrap().mana_pool.total();
        if spent != u32::from(*cost) {
            return Err(format!("spell spent {spent}, expected {cost}"));
        }
        resolve_all(&mut game)?;
        outcomes.push(json!({"mana_value": cost, "type": format!("{card_type:?}"),
            "life_gained": game.player(alice()).unwrap().life - before}));
    }
    Ok(json!({"source_resolution_error": null, "outcomes": outcomes}))
}

fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: Value,
    expected: Value,
    result: Result<Value, String>,
) {
    let (status, actual) = match result {
        Ok(actual) if actual == expected => ("passed", actual),
        Ok(actual) => ("semantic_mismatch", actual),
        Err(error) => ("execution_or_fixture_error", json!({"error": error})),
    };
    rows.push(json!({"card": card, "scenario": scenario, "status": status, "expected": expected, "actual": actual}));
}

#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_delayed_copy_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_delayed_copy_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|path| hash(path)).collect();
    let names = ["Dynaheir, Invoker Adept", "Sea Gate Stormcaller"]
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
            (definition.name().to_owned(), definition)
        })
        .collect();
    let mut rows = Vec::new();
    for (costs, arm) in [
        (vec![4], false),
        (vec![4], true),
        (vec![3, 4], true),
        (vec![4, 4], true),
    ] {
        let mut used = false;
        let outcomes: Vec<_> = costs
            .iter()
            .map(|cost| {
                let copies = usize::from(arm && !used && *cost >= 4);
                if copies > 0 {
                    used = true;
                }
                json!({"mana_spent": cost, "life_gained": 1 + copies})
            })
            .collect();
        let name = &names[0];
        record(
            &mut rows,
            name,
            json!({"activation_mana_costs": costs, "arm": arm}),
            json!({"arming_resolution_error": null, "outcomes": outcomes}),
            dynaheir(&definitions[name], &costs, arm),
        );
    }
    for (spells, cast_source) in [
        (vec![(1, CardType::Instant)], false),
        (vec![(1, CardType::Instant)], true),
        (vec![(2, CardType::Sorcery)], true),
        (vec![(3, CardType::Instant), (1, CardType::Instant)], true),
        (vec![(1, CardType::Instant), (1, CardType::Instant)], true),
    ] {
        let mut used = false;
        let outcomes: Vec<_> = spells.iter().map(|(cost, card_type)| {
            let copies = usize::from(cast_source && !used && *cost <= 2);
            if copies > 0 { used = true; }
            json!({"mana_value": cost, "type": format!("{card_type:?}"), "life_gained": 1 + copies})
        }).collect();
        let name = &names[1];
        record(
            &mut rows,
            name,
            json!({"cast_source": cast_source, "spells": outcomes}),
            json!({"source_resolution_error": null, "outcomes": outcomes}),
            sea_gate(&definitions[name], &spells, cast_source),
        );
    }
    let after: Vec<_> = paths.iter().map(|path| hash(path)).collect();
    let report = json!({"scope": "Actual legal casts and activations; canonical source cards, neutral life-gain fixture spells/abilities; normal unkicked Sea Gate only; failures do not repair or continue failed resolution",
        "provenance": {"compiler": "fresh in-process compilation from cards.json", "before": before, "after": after,
            "artifacts_unchanged": before == after}, "rows": rows});
    let output = root.join("reports/runtime-audit/delayed-copy-execution.json");
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    std::fs::write(&output, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("{}", report);
}
