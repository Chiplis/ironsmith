//! Boundary probes prompted by numeric conditions missing from scalar wire fields.
//! Some numbers are correctly encoded in strings or named static abilities.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, generate_and_queue_step_triggers, resolve_stack_entry_with,
};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn filler(kind: CardType) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Numeric boundary fixture")
        .card_types(vec![kind]);
    if kind == CardType::Creature {
        builder
            .power_toughness(ironsmith::PowerToughness::fixed(2, 3))
            .build()
    } else {
        builder.build()
    }
}
fn finish(game: &mut GameState, queue: &mut TriggerQueue) -> Result<(), String> {
    let mut dm = SelectFirstDecisionMaker;
    for _ in 0..24 {
        advance_priority_with_dm(game, queue, &mut dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        resolve_stack_entry_with(game, &mut dm).map_err(|e| e.to_string())?;
    }
    Err("resolution budget".into())
}
fn upkeep(game: &mut GameState, player: PlayerId) -> Result<(), String> {
    // Seed a reachable start-of-upkeep board; ETB and linked-face transitions are separate coverage.
    game.effect_store.pending_trigger_events.clear();
    game.turn.phase = ironsmith::Phase::Beginning;
    game.turn.step = Some(ironsmith::Step::Upkeep);
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(game, &mut queue);
    finish(game, &mut queue)
}
fn shadowborn(
    def: &CardDefinition,
    creatures: usize,
    noncreatures: usize,
    your_upkeep: bool,
) -> Value {
    let mut game = setup();
    game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    game.create_object_from_definition(&filler(CardType::Creature), PlayerId(0), Zone::Battlefield);
    for _ in 0..creatures {
        game.create_object_from_definition(
            &filler(CardType::Creature),
            PlayerId(0),
            Zone::Graveyard,
        );
    }
    for _ in 0..noncreatures {
        game.create_object_from_definition(
            &filler(CardType::Instant),
            PlayerId(0),
            Zone::Graveyard,
        );
    }
    let error = upkeep(&mut game, PlayerId(if your_upkeep { 0 } else { 1 })).err();
    json!({"resolution_error":error,"remaining_creatures":game.battlefield.len()})
}
fn stabwhisker(def: &CardDefinition, hand: usize, your_upkeep: bool) -> Value {
    let mut game = setup();
    game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    for _ in 0..hand {
        game.create_object_from_definition(&filler(CardType::Instant), PlayerId(1), Zone::Hand);
    }
    let error = upkeep(&mut game, PlayerId(if your_upkeep { 0 } else { 1 })).err();
    json!({"resolution_error":error,"alice_life":game.player(PlayerId(0)).unwrap().life,"bob_life":game.player(PlayerId(1)).unwrap().life})
}
fn magus(def: &CardDefinition, hand: usize) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    game.remove_summoning_sickness(source);
    for _ in 0..hand {
        game.create_object_from_definition(&filler(CardType::Instant), PlayerId(0), Zone::Hand);
    }
    for _ in 0..3 {
        game.create_object_from_definition(&filler(CardType::Instant), PlayerId(0), Zone::Library);
    }
    let action = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:id,..} if *id==source));
    let legal = action.is_some();
    let mut error = None;
    if let Some(action) = action {
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
        for _ in 0..16 {
            if state.pending_activation.is_none() && !game.stack.is_empty() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                return Err(format!("activation stalled:{progress:?}"));
            };
            progress =
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
                    .map_err(|e| e.to_string())?;
        }
        if game.stack.is_empty() {
            return Err("activation failed to reach stack".into());
        }
        error = finish(&mut game, &mut queue).err();
    }
    Ok(
        json!({"resolution_error":error,"draw_ability_legal":legal,"hand_size":game.player(PlayerId(0)).unwrap().hand.len(),"source_tapped":game.is_tapped(source)}),
    )
}
fn winter(def: &CardDefinition, count: usize) -> Value {
    let mut game = setup();
    game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    for kind in [
        CardType::Creature,
        CardType::Artifact,
        CardType::Land,
        CardType::Instant,
        CardType::Sorcery,
        CardType::Enchantment,
        CardType::Planeswalker,
        CardType::Battle,
    ]
    .into_iter()
    .take(count)
    {
        game.create_object_from_definition(&filler(kind), PlayerId(0), Zone::Graveyard);
    }
    game.update_cant_effects();
    json!({"resolution_error":null,"alice_max_hand":game.player(PlayerId(0)).unwrap().max_hand_size,"bob_max_hand":game.player(PlayerId(1)).unwrap().max_hand_size})
}
fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: Value,
    expected: Value,
    result: Result<Value, String>,
) {
    let (status, actual) = match result {
        Ok(actual) => (
            if actual == expected {
                "expected_result_observed"
            } else if !actual["resolution_error"].is_null() {
                "resolution_failed"
            } else {
                "semantic_mismatch"
            },
            actual,
        ),
        Err(error) => ("fixture_or_announcement_error", json!({"error":error})),
    };
    rows.push(json!({"card":card,"scenario":scenario,"expected":expected,"actual":actual,"status":status}));
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "report generator, not a card correctness test"]
fn report_numeric_predicates() {
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Shadowborn Demon",
        "Stabwhisker the Odious",
        "Magus of the Library",
        "Winter, Misanthropic Guide",
    ];
    let mut definitions = HashMap::new();
    let mut artifacts = Vec::new();
    for payload in input["cards"].as_array().unwrap() {
        let name = payload["name"].as_str().unwrap();
        if !names.contains(&name) {
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
    for (creatures, noncreatures, yours) in [
        (0, 0, true),
        (1, 0, true),
        (5, 0, true),
        (6, 0, true),
        (7, 0, true),
        (5, 4, true),
        (6, 4, true),
        (0, 0, false),
    ] {
        record(
            &mut rows,
            "Shadowborn Demon",
            json!({"graveyard_creatures":creatures,"graveyard_noncreatures":noncreatures,"your_upkeep":yours}),
            json!({"resolution_error":null,"remaining_creatures":if yours && creatures<6 {1}else{2}}),
            Ok(shadowborn(
                &definitions["Shadowborn Demon"],
                creatures,
                noncreatures,
                yours,
            )),
        );
    }
    for hand in [0, 1, 2, 3, 4] {
        for yours in [false, true] {
            record(
                &mut rows,
                "Stabwhisker the Odious",
                json!({"opponent_hand":hand,"your_upkeep":yours,"setup":"strict back face on battlefield; flip transition not covered"}),
                json!({"resolution_error":null,"alice_life":20,"bob_life":20-if yours{0}else{3usize.saturating_sub(hand)}}),
                Ok(stabwhisker(
                    &definitions["Stabwhisker the Odious"],
                    hand,
                    yours,
                )),
            );
        }
    }
    for hand in [0, 6, 7, 8] {
        record(
            &mut rows,
            "Magus of the Library",
            json!({"initial_hand":hand,"setup":"untapped creature controlled since before current turn; actual legal activation"}),
            json!({"resolution_error":null,"draw_ability_legal":hand==7,"hand_size":hand+usize::from(hand==7),"source_tapped":hand==7}),
            magus(&definitions["Magus of the Library"], hand),
        );
    }
    for count in [0, 3, 4, 5, 7, 8] {
        record(
            &mut rows,
            "Winter, Misanthropic Guide",
            json!({"graveyard_distinct_card_types":count,"setup":"derived maximum hand size; draw trigger and cleanup not covered"}),
            json!({"resolution_error":null,"alice_max_hand":7,"bob_max_hand":if count>=4{(7-count as i32).max(0)}else{7}}),
            Ok(winter(&definitions["Winter, Misanthropic Guide"], count)),
        );
    }
    let binary = std::env::current_exe().unwrap();
    let report = json!({"scope":"Strict canonical artifact numeric boundary probes. A passing scenario does not certify other card abilities or branches. Seeded legal battlefield/zone configurations use engine-generated upkeep events; Magus uses normal action and payment.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory)}});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/numeric-predicate-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
