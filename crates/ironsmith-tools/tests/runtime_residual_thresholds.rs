//! Expected-result boundary probes for the completed corpus text screen.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::NumberContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, generate_and_queue_step_triggers, resolve_stack_entry_with,
};
use ironsmith::game_state::StackEntry;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Subtype, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Default)]
struct Decisions {
    x: u32,
    choices: Vec<Value>,
}
impl DecisionMaker for Decisions {
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        let chosen = self.x.clamp(ctx.min, ctx.max);
        self.choices.push(json!({"description":ctx.description,"min":ctx.min,"max":ctx.max,"chosen":chosen,"is_x":ctx.is_x_value}));
        chosen
    }
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(71757432704846);
    game.turn.turn_number = 3;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for player in [PlayerId(0), PlayerId(1)] {
        game.player_mut(player)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 32);
    }
    game
}
fn cast(
    game: &mut GameState,
    id: ObjectId,
    player: PlayerId,
    dm: &mut Decisions,
) -> Result<Value, String> {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    let before = game.player(player).unwrap().mana_pool.total();
    let action = compute_legal_actions(game, player).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==id))
        .ok_or("fixture has no legal cast")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_cast.is_none() && !game.stack.is_empty() {
            let entry = game.stack.last().unwrap();
            return Ok(
                json!({"mana_spent":before-game.player(player).unwrap().mana_pool.total(),"stack_x":entry.x_value,"choices":dm.choices}),
            );
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast stalled: {progress:?}"));
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("cast decision budget".into())
}
fn finish(
    game: &mut GameState,
    queue: &mut TriggerQueue,
    dm: &mut Decisions,
) -> Result<(), String> {
    for _ in 0..32 {
        advance_priority_with_dm(game, queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
    }
    Err("resolution budget".into())
}
fn valley(def: &CardDefinition, lands: usize, your_upkeep: bool) -> Result<(Value, Value), String> {
    let mut game = setup();
    game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    let land = CardDefinitionBuilder::new(CardId::new(), "Boundary land")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 1..lands {
        game.create_object_from_definition(&land, PlayerId(0), Zone::Battlefield);
    }
    game.effect_store.pending_trigger_events.clear();
    game.turn.phase = ironsmith::Phase::Beginning;
    game.turn.step = Some(ironsmith::Step::Upkeep);
    game.turn.active_player = PlayerId(if your_upkeep { 0 } else { 1 });
    game.turn.priority_player = Some(game.turn.active_player);
    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut queue);
    let error = finish(&mut game, &mut queue, &mut Decisions::default()).err();
    Ok((
        json!({"resolution_error":error,"life":game.player(PlayerId(0)).unwrap().life}),
        json!({"lands":lands,"your_upkeep":your_upkeep,"source_setup":"seeded permanent, actual upkeep trigger generation; entry replacement not tested"}),
    ))
}
fn trudge(def: &CardDefinition, x: u32, noncast: bool) -> Result<(Value, Value), String> {
    let mut game = setup();
    let id = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let stable = game.object(id).unwrap().stable_id;
    let mut dm = Decisions {
        x,
        ..Default::default()
    };
    let evidence = if noncast {
        let producer = CardDefinitionBuilder::new(CardId::new(), "Boundary noncast entry")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_definition(&producer, PlayerId(0), Zone::Battlefield);
        game.push_to_stack(StackEntry::ability(
            source,
            PlayerId(0),
            vec![ironsmith::Effect::new(
                ironsmith::effects::PutOntoBattlefieldEffect::new(
                    ironsmith::target::ChooseSpec::SpecificObject(id),
                    false,
                    ironsmith::target::PlayerFilter::You,
                ),
            )],
        ));
        json!({"method":"ordinary PutOntoBattlefieldEffect from hand; X should be zero"})
    } else {
        let evidence = cast(&mut game, id, PlayerId(0), &mut dm)?;
        if evidence["stack_x"] != json!(x) || evidence["mana_spent"] != json!(x + 1) {
            return Err(format!("requested X/cost mismatch: {evidence}"));
        }
        evidence
    };
    let error = finish(&mut game, &mut TriggerQueue::new(), &mut dm).err();
    let current = game
        .find_object_by_stable_id(stable)
        .ok_or("source disappeared")?;
    let object = game.object(current).unwrap();
    Ok((
        json!({"resolution_error":error,"zone":format!("{:?}",object.zone),"tapped":game.is_tapped(current),"stun_counters":game.counter_count(current,CounterType::Stun)}),
        evidence,
    ))
}
fn velocipede(
    def: &CardDefinition,
    mv: u32,
    kind: &str,
    controller: usize,
    source_itself: bool,
) -> Result<(Value, Value), String> {
    let mut game = setup();
    if !source_itself {
        game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    }
    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Boundary entering permanent")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mv.try_into().unwrap(),
        )]));
    builder = if kind == "creature" {
        builder
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 3))
    } else if kind == "vehicle" {
        builder
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Vehicle])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 3))
    } else {
        builder.card_types(vec![CardType::Artifact])
    };
    let incoming = builder.build();
    let player = PlayerId(controller as u8);
    let id = game.create_object_from_definition(
        if source_itself { def } else { &incoming },
        player,
        Zone::Hand,
    );
    let stable = game.object(id).unwrap().stable_id;
    let mut dm = Decisions::default();
    let evidence = cast(&mut game, id, player, &mut dm)?;
    let error = finish(&mut game, &mut TriggerQueue::new(), &mut dm).err();
    let current = game
        .find_object_by_stable_id(stable)
        .ok_or("source disappeared")?;
    Ok((
        json!({"resolution_error":error,"zone":format!("{:?}",game.object(current).unwrap().zone),"plus_one_counters":game.counter_count(current,CounterType::PlusOnePlusOne)}),
        evidence,
    ))
}
fn record(
    rows: &mut Vec<Value>,
    name: &str,
    scenario: Value,
    expected: Value,
    result: Result<(Value, Value), String>,
) {
    let (status, actual, evidence) = match result {
        Ok((actual, evidence)) => (
            if actual == expected {
                "expected_result_observed"
            } else if !actual["resolution_error"].is_null() {
                "resolution_failed"
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
    rows.push(json!({"card":name,"scenario":scenario,"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "report generator, not a card correctness test"]
fn report_residual_thresholds() {
    let inventory = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let input: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Sheltered Valley",
        "Slumbering Trudge",
        "Thunderous Velocipede",
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
    let binary = std::env::current_exe().unwrap();
    let before = hash(&binary);
    let mut rows = Vec::new();
    for lands in [1, 2, 3, 4, 5] {
        for yours in [false, true] {
            record(
                &mut rows,
                "Sheltered Valley",
                json!({"lands":lands,"your_upkeep":yours}),
                json!({"resolution_error":null,"life":20+usize::from(yours&&lands<=3)}),
                valley(&definitions["Sheltered Valley"], lands, yours),
            );
        }
    }
    for x in [0, 1, 2, 3, 4] {
        record(
            &mut rows,
            "Slumbering Trudge",
            json!({"cast_x":x}),
            json!({"resolution_error":null,"zone":"Battlefield","tapped":x<=2,"stun_counters":3u32.saturating_sub(x)}),
            trudge(&definitions["Slumbering Trudge"], x, false),
        );
    }
    record(
        &mut rows,
        "Slumbering Trudge",
        json!({"noncast_entry":true}),
        json!({"resolution_error":null,"zone":"Battlefield","tapped":true,"stun_counters":3}),
        trudge(&definitions["Slumbering Trudge"], 0, true),
    );
    for (mv, kind, controller, source_itself) in [
        (0, "creature", 0, false),
        (4, "creature", 0, false),
        (5, "creature", 0, false),
        (4, "vehicle", 0, false),
        (5, "vehicle", 0, false),
        (4, "artifact", 0, false),
        (5, "creature", 1, false),
        (3, "vehicle", 0, true),
    ] {
        let expected = if controller != 0 || source_itself || kind == "artifact" {
            0
        } else if mv <= 4 {
            1
        } else {
            3
        };
        record(
            &mut rows,
            "Thunderous Velocipede",
            json!({"incoming_mana_value":mv,"kind":kind,"controller":controller,"source_itself":source_itself}),
            json!({"resolution_error":null,"zone":"Battlefield","plus_one_counters":expected}),
            velocipede(
                &definitions["Thunderous Velocipede"],
                mv,
                kind,
                controller,
                source_itself,
            ),
        );
    }
    let report = json!({"scope":"Strict artifact boundary probes; actual paid casts, upkeep trigger generation, or explicitly identified generic noncast entry. No whole-card certification.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":before,"binary_unchanged":before==hash(&binary),"inventory":inventory,"inventory_sha256":hash(&inventory)}});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/residual-threshold-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
