//! Opt-in expected-result probes for simultaneous player effects. A passing Rust test means
//! the report was emitted; inspect row statuses for actual engine correctness.

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
use ironsmith::mana::ManaSymbol;
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
    for color in [ManaSymbol::Black, ManaSymbol::Blue, ManaSymbol::Colorless] {
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

fn bob() -> PlayerId {
    PlayerId::from_index(1)
}

fn filler(card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(
        CardId::new(),
        format!("Simultaneous audit filler {card_type:?}"),
    )
    .card_types(vec![card_type])
    .build()
}

fn count_named(game: &GameState, name: &str, zone: Zone, owner: PlayerId) -> usize {
    game.objects_in_deterministic_order()
        .into_iter()
        .filter(|object| object.name == name && object.zone == zone && object.owner == owner)
        .count()
}

fn cast_paid(game: &mut GameState, definition: &CardDefinition, mana: u32) -> Result<(), String> {
    let id = game.create_object_from_definition(definition, alice(), Zone::Hand);
    let before = game.player(alice()).unwrap().mana_pool.total();
    cast_or_activate(game, id, true)?;
    let paid = before - game.player(alice()).unwrap().mana_pool.total();
    if paid != mana {
        return Err(format!("fixture cast paid {paid}, expected {mana}"));
    }
    Ok(())
}

fn draw_spell(definition: &CardDefinition) -> Result<Value, String> {
    let mut game = setup();
    let land = filler(CardType::Land);
    for player in [alice(), bob()] {
        for _ in 0..12 {
            game.create_object_from_definition(&land, player, Zone::Library);
        }
        game.create_object_from_definition(&land, player, Zone::Hand);
        for _ in 0..2 {
            game.create_object_from_definition(&land, player, Zone::Graveyard);
        }
    }
    cast_paid(&mut game, definition, 3)?;
    let error = resolve_all(&mut game).err();
    Ok(json!({"resolution_error": error,
        "hands": game.players.iter().map(|player| player.hand.len()).collect::<Vec<_>>(),
        "graveyard_filler": [count_named(&game, land.name(), Zone::Graveyard, alice()), count_named(&game, land.name(), Zone::Graveyard, bob())],
        "stack_size": game.stack.len()}))
}

fn executioner(
    definitions: &HashMap<String, CardDefinition>,
    protected: bool,
) -> Result<Value, String> {
    let mut game = setup();
    for player in [alice(), bob()] {
        game.create_object_from_definition(
            &definitions["Grizzly Bears"],
            player,
            Zone::Battlefield,
        );
    }
    if protected {
        game.create_object_from_definition(
            &definitions["Sigarda, Host of Herons"],
            bob(),
            Zone::Battlefield,
        );
    }
    cast_paid(&mut game, &definitions["Merciless Executioner"], 3)?;
    let error = resolve_all(&mut game).err();
    Ok(json!({"resolution_error": error,
        "battlefield_counts": ([alice(), bob()].map(|player| game.battlefield.iter().filter(|id| game.current_controller(**id) == Some(player)).count())),
        "graveyard_counts": game.players.iter().map(|player| player.graveyard.len()).collect::<Vec<_>>() }))
}

fn tempt(definitions: &HashMap<String, CardDefinition>) -> Result<Value, String> {
    let mut game = setup();
    game.create_object_from_definition(
        &definitions["Merciless Executioner"],
        alice(),
        Zone::Graveyard,
    );
    game.create_object_from_definition(
        &definitions["Sigarda, Host of Herons"],
        bob(),
        Zone::Graveyard,
    );
    cast_paid(&mut game, &definitions["Tempt with Immortality"], 5)?;
    let error = resolve_all(&mut game).err();
    Ok(json!({"resolution_error": error,
        "executioner_battlefield": count_named(&game, "Merciless Executioner", Zone::Battlefield, alice()),
        "executioner_graveyard": count_named(&game, "Merciless Executioner", Zone::Graveyard, alice()),
        "sigarda_battlefield": count_named(&game, "Sigarda, Host of Herons", Zone::Battlefield, bob()),
        "sigarda_graveyard": count_named(&game, "Sigarda, Host of Herons", Zone::Graveyard, bob()),
        "tempt_graveyard": count_named(&game, "Tempt with Immortality", Zone::Graveyard, alice())}))
}

fn sticktwister(definition: &CardDefinition, types: usize) -> Result<Value, String> {
    let mut game = setup();
    game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    for card_type in [
        CardType::Land,
        CardType::Instant,
        CardType::Sorcery,
        CardType::Enchantment,
    ]
    .into_iter()
    .take(types)
    {
        game.create_object_from_definition(&filler(card_type), alice(), Zone::Graveyard);
    }
    game.turn.phase = ironsmith::Phase::Ending;
    game.turn.step = Some(ironsmith::Step::End);
    let event = ironsmith::triggers::generate_step_trigger_events(&game)
        .ok_or("fixture generated no end-step event")?;
    let mut queue = TriggerQueue::new();
    for entry in ironsmith::triggers::check_triggers(&game, &event) {
        queue.add(entry);
    }
    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker)
        .map_err(|error| error.to_string())?;
    let error = resolve_all(&mut game).err();
    Ok(json!({"resolution_error": error, "opponent_life": game.player(bob()).unwrap().life}))
}

fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: &str,
    expected: Value,
    result: Result<Value, String>,
) {
    let (status, actual) = match result {
        Ok(actual) if actual == expected => ("passed", actual),
        Ok(actual) if !actual["resolution_error"].is_null() => {
            ("confirmed_resolution_failure", actual)
        }
        Ok(actual) => ("semantic_mismatch", actual),
        Err(error) => ("execution_or_fixture_error", json!({"error": error})),
    };
    rows.push(json!({"card": card, "scenario": scenario, "status": status, "expected": expected, "actual": actual}));
}

#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_simultaneous_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_simultaneous_action_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|path| hash(path)).collect();
    let names = [
        "Day's Undoing",
        "Divination",
        "Merciless Executioner",
        "Sigarda, Host of Herons",
        "Tempt with Immortality",
        "Grizzly Bears",
        "Osseous Sticktwister",
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
            (definition.name().to_owned(), definition)
        })
        .collect();
    let mut rows = Vec::new();
    record(
        &mut rows,
        "Divination",
        "single-player draw control",
        json!({"resolution_error": null, "hands": [3,1], "graveyard_filler": [2,2], "stack_size":0}),
        draw_spell(&definitions["Divination"]),
    );
    record(
        &mut rows,
        "Day's Undoing",
        "own-turn actual legal cast; sufficient libraries",
        json!({"resolution_error": null, "hands": [7,7], "graveyard_filler": [0,0], "stack_size":0}),
        draw_spell(&definitions["Day's Undoing"]),
    );
    for protected in [false, true] {
        record(
            &mut rows,
            "Merciless Executioner",
            if protected {
                "opponent controls Sigarda"
            } else {
                "each player controls a creature"
            },
            json!({"resolution_error": null, "battlefield_counts": [1, if protected {2} else {0}], "graveyard_counts": [1, if protected {0} else {1}]}),
            executioner(&definitions, protected),
        );
    }
    record(
        &mut rows,
        "Tempt with Immortality",
        "return Executioner and opposing Sigarda; both accept",
        json!({"resolution_error": null, "executioner_battlefield": 0, "executioner_graveyard": 1, "sigarda_battlefield": 1, "sigarda_graveyard": 0, "tempt_graveyard":1}),
        tempt(&definitions),
    );
    for types in [3, 4] {
        record(
            &mut rows,
            "Osseous Sticktwister",
            if types == 3 {
                "three graveyard card types; no delirium"
            } else {
                "four graveyard card types; opponent has nothing to sacrifice or discard"
            },
            json!({"resolution_error": null, "opponent_life": if types == 3 {20} else {18}}),
            sticktwister(&definitions["Osseous Sticktwister"], types),
        );
    }
    let after: Vec<_> = paths.iter().map(|path| hash(path)).collect();
    let report = json!({"scope":"Actual legal casts with checked paid mana and normal stack/event resolution; Osseous uses engine-generated end-step event; no state repair after failures; source cards freshly compiled from canonical corpus",
        "provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"rows":rows});
    let output = root.join("reports/runtime-audit/simultaneous-action-execution.json");
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    std::fs::write(&output, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("{report}");
}
