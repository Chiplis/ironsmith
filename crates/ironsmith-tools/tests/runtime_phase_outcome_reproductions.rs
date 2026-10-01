//! Expected-result phase trigger probes; passing test means report emitted, not semantic correctness.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_priority_response_with_dm,
    drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, PlayerId, Zone};
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
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}
fn setup() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g
}
fn filler(kind: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), format!("Phase outcome audit {kind:?}"))
        .card_types(vec![kind])
        .build()
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

fn phase(game: &mut GameState, upkeep: bool) -> Result<Option<String>, String> {
    game.turn.phase = if upkeep {
        ironsmith::Phase::Beginning
    } else {
        ironsmith::Phase::Ending
    };
    game.turn.step = Some(if upkeep {
        ironsmith::Step::Upkeep
    } else {
        ironsmith::Step::End
    });
    let event = ironsmith::triggers::generate_step_trigger_events(game)
        .ok_or("fixture step event missing")?;
    let mut queue = TriggerQueue::new();
    for trigger in ironsmith::triggers::check_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker)
        .map_err(|e| e.to_string())?;
    Ok(resolve_all(game).err())
}
fn lily(definition: &CardDefinition, counters: u32) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    game.object_mut(source)
        .unwrap()
        .add_counters(CounterType::PlusOnePlusOne, counters);
    let error = phase(&mut game, true)?;
    Ok(
        json!({"resolution_error":error,"counters":game.object(source).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0),"life":game.player(alice()).unwrap().life}),
    )
}
fn monsoon(
    definitions: &HashMap<String, CardDefinition>,
    untapped: usize,
    present: bool,
) -> Result<Value, String> {
    let mut game = setup();
    if present {
        game.create_object_from_definition(&definitions["Monsoon"], alice(), Zone::Battlefield);
    }
    let mut islands = Vec::new();
    for _ in 0..untapped {
        islands.push(game.create_object_from_definition(
            &definitions["Island"],
            alice(),
            Zone::Battlefield,
        ));
    }
    let already =
        game.create_object_from_definition(&definitions["Island"], alice(), Zone::Battlefield);
    game.tap(already);
    let opposite =
        game.create_object_from_definition(&definitions["Island"], bob(), Zone::Battlefield);
    let error = phase(&mut game, false)?;
    Ok(
        json!({"resolution_error":error,"active_life":game.player(alice()).unwrap().life,"newly_tapped":islands.iter().filter(|id|game.is_tapped(**id)).count(),"already_tapped":game.is_tapped(already),"opponent_island_tapped":game.is_tapped(opposite)}),
    )
}
fn mog(definition: &CardDefinition, have_hands: bool) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    let land = filler(CardType::Land);
    for player in [alice(), bob()] {
        for _ in 0..5 {
            game.create_object_from_definition(&land, player, Zone::Library);
        }
    }
    if have_hands {
        game.create_object_from_definition(&filler(CardType::Creature), alice(), Zone::Hand);
        game.create_object_from_definition(&filler(CardType::Sorcery), bob(), Zone::Hand);
    }
    let error = phase(&mut game, false)?;
    let tokens: Vec<_> = game
        .objects_in_deterministic_order()
        .into_iter()
        .filter(|object| object.zone == Zone::Battlefield && object.name == "Moogle")
        .collect();
    Ok(
        json!({"resolution_error":error,"hands":game.players.iter().map(|p|p.hand.len()).collect::<Vec<_>>(),"graveyards":game.players.iter().map(|p|p.graveyard.len()).collect::<Vec<_>>(),"moogle_tokens":tokens.len(),"source_counters":game.object(source).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0),"token_counters":tokens.iter().map(|o|o.counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0)).sum::<u32>()}),
    )
}
fn valakut(
    definitions: &HashMap<String, CardDefinition>,
    play_land: bool,
) -> Result<Value, String> {
    let mut game = setup();
    game.create_object_from_definition(
        &definitions["Valakut Exploration"],
        alice(),
        Zone::Battlefield,
    );
    let filler = filler(CardType::Instant);
    for _ in 0..5 {
        game.create_object_from_definition(&filler, alice(), Zone::Library);
    }
    if play_land {
        let land = game.create_object_from_definition(&definitions["Island"], alice(), Zone::Hand);
        let action = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==land))
            .ok_or("fixture land play not legal")?;
        let mut q = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        apply_priority_response_with_dm(
            &mut game,
            &mut q,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut SelectFirstDecisionMaker,
        )
        .map_err(|e| e.to_string())?;
        put_triggers_on_stack_with_dm(&mut game, &mut q, &mut SelectFirstDecisionMaker)
            .map_err(|e| e.to_string())?;
        if let Err(error) = resolve_all(&mut game) {
            return Ok(json!({"resolution_error":error,"stage":"landfall"}));
        }
        if game.exile.len() != 1 {
            return Err(format!(
                "fixture landfall did not exile exactly one card: {}",
                game.exile.len()
            ));
        }
    }
    let exile_before = game.exile.len();
    let error = phase(&mut game, false)?;
    Ok(
        json!({"resolution_error":error,"exile_before_end":exile_before,"exile_after_end":game.exile.len(),"graveyard_after_end":game.player(alice()).unwrap().graveyard.len(),"opponent_life":game.player(bob()).unwrap().life}),
    )
}
fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: &str,
    expected: Value,
    result: Result<Value, String>,
) {
    let (status, actual) = match result {
        Ok(a) if a == expected => ("passed", a),
        Ok(a) if !a["resolution_error"].is_null() => ("confirmed_resolution_failure", a),
        Ok(a) => ("semantic_mismatch", a),
        Err(e) => ("execution_or_fixture_error", json!({"error":e})),
    };
    rows.push(json!({"card":card,"scenario":scenario,"status":status,"expected":expected,"actual":actual}));
}
#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_phase_outcome_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_phase_outcome_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = [
        "Lily Bowen, Raging Grandma",
        "Mog, Moogle Warrior",
        "Monsoon",
        "Valakut Exploration",
        "Island",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut definitions = HashMap::new();
    let mut compilation = Vec::new();
    for p in payloads.into_values().flatten() {
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p.parse_name.as_deref().unwrap_or(&p.name),
        );
        let (artifact, definition) =
            ironsmith_registry::compile_builder_to_artifact(builder, &p.parse_input, false)
                .unwrap();
        compilation.push(json!({"card":p.name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        definitions.insert(definition.name().to_owned(), definition);
    }
    let mut rows = Vec::new();
    for (counters, expected_counters, life) in [(2, 4, 20), (16, 32, 20), (17, 1, 36)] {
        record(
            &mut rows,
            "Lily Bowen, Raging Grandma",
            &format!("upkeep with {counters} counters"),
            json!({"resolution_error":null,"counters":expected_counters,"life":life}),
            lily(&definitions["Lily Bowen, Raging Grandma"], counters),
        );
    }
    for (untapped, present, newly, life) in [(0, true, 0, 20), (2, true, 2, 18), (2, false, 0, 20)]
    {
        record(
            &mut rows,
            "Monsoon",
            &format!("end step: {untapped} untapped Islands, source present={present}"),
            json!({"resolution_error":null,"active_life":life,"newly_tapped":newly,"already_tapped":true,"opponent_island_tapped":false}),
            monsoon(&definitions, untapped, present),
        );
    }
    for have in [false, true] {
        record(
            &mut rows,
            "Mog, Moogle Warrior",
            if have {
                "each player discards; creature plus noncreature"
            } else {
                "both hands empty"
            },
            json!({"resolution_error":null,"hands":if have{[1,1]}else{[0,0]},"graveyards":if have{[1,1]}else{[0,0]},"moogle_tokens":usize::from(have),"source_counters":u32::from(have),"token_counters":u32::from(have)}),
            mog(&definitions["Mog, Moogle Warrior"], have),
        );
    }
    for play in [false, true] {
        record(
            &mut rows,
            "Valakut Exploration",
            if play {
                "legal land play, landfall exile, then end step"
            } else {
                "end step without linked exiles"
            },
            json!({"resolution_error":null,"exile_before_end":usize::from(play),"exile_after_end":0,"graveyard_after_end":usize::from(play),"opponent_life":if play{19}else{20}}),
            valakut(&definitions, play),
        );
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict canonical artifact phase triggers. Engine-generated phase events enter normal trigger matcher and stack resolution; Valakut uses a legal land play before end step. Fixture counters and initial permanents represent established game state, not assertion repairs.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    let output = root.join("reports/runtime-audit/phase-outcome-execution.json");
    std::fs::write(
        &output,
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("{}", serde_json::to_string(&report).unwrap());
}
