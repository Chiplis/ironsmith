//! Strict canonical cast-trigger counter amount audit.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, SelectObjectsContext, SelectOptionsContext,
    TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, Effect, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
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
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(alice()).unwrap().mana_pool.add(color, 30);
    }
    g
}
struct ProbeDm {
    decline: bool,
    x: u32,
    choices: Vec<Value>,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        if c.is_x_value { self.x } else { c.min }
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected: Vec<_> = c
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.legal && g.controller_of_id(candidate.id) == Some(c.player)
            })
            .take(self.x as usize)
            .map(|o| o.id)
            .collect();
        self.choices.push(json!({"player":format!("{:?}",c.player),"description":c.description,"min":c.min,"max":c.max,"selected":selected.iter().map(|id|format!("{id:?}")).collect::<Vec<_>>(),"offered_controllers":c.candidates.iter().map(|candidate|g.controller_of_id(candidate.id).map(|o|format!("{o:?}"))).collect::<Vec<_>>()}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let target = Target::Player(bob());
        if c.requirements.len() == 1 && c.requirements[0].legal_targets.contains(&target) {
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(g, c)
        }
    }
}
fn resolve(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut ProbeDm) -> Result<(), String> {
    for _ in 0..24 {
        drain_pending_trigger_events(game, queue);
        put_triggers_on_stack_with_dm(game, queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
    }
    Err("fixture resolution bound exceeded".into())
}
fn perform(
    game: &mut GameState,
    source: ObjectId,
    ability: Option<usize>,
    dm: &mut ProbeDm,
) -> Result<(TriggerQueue, u32), String> {
    game.turn.priority_player = Some(alice());
    let action = compute_legal_actions(game, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| match a {
            LegalAction::CastSpell { spell_id, .. } => ability.is_none() && *spell_id == source,
            LegalAction::ActivateAbility {
                source: id,
                ability_index,
            } => *id == source && Some(*ability_index) == ability,
            _ => false,
        })
        .ok_or("fixture intended legal action missing")?;
    let mana = game.player(alice()).unwrap().mana_pool.total();
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
    for _ in 0..24 {
        if !game.stack.is_empty() {
            return Ok((
                queue,
                mana - game.player(alice()).unwrap().mana_pool.total(),
            ));
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref c))
                if c.description.starts_with("Choose optional costs") =>
            {
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(vec![]),
                    dm,
                )
                .map_err(|e| e.to_string())?
            }
            GameProgress::NeedsDecisionCtx(ref c) if !matches!(c, DecisionContext::Priority(_)) => {
                apply_decision_context_with_dm(game, &mut queue, &mut state, c, dm)
                    .map_err(|e| e.to_string())?
            }
            other => return Err(format!("fixture action failed to reach stack: {other:?}")),
        };
    }
    Err("fixture action decision bound exceeded".into())
}

fn witness(name: &str, cost: u8, kind: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(cost)]))
        .power_toughness(PowerToughness::fixed(3, 3))
        .build()
}
fn probe(
    def: &CardDefinition,
    x: u32,
    resources: [usize; 2],
    library_mvs: [u8; 2],
) -> Result<Value, String> {
    let mut game = setup();
    let resource = witness("Counter audit sacrifice witness", 1, CardType::Creature);
    for (i, p) in [alice(), bob()].into_iter().enumerate() {
        for _ in 0..resources[i] {
            game.create_object_from_definition(&resource, p, Zone::Battlefield);
        }
        game.create_object_from_definition(
            &witness(
                &format!("Revealed top {i}"),
                library_mvs[i],
                CardType::Sorcery,
            ),
            p,
            Zone::Library,
        );
    }
    let source = game.create_object_from_definition(def, alice(), Zone::Hand);
    let mut dm = ProbeDm {
        decline: false,
        x,
        choices: vec![],
    };
    let (mut q, paid) = perform(&mut game, source, None, &mut dm)?;
    let cast_x = game.stack.last().and_then(|entry| entry.x_value);
    let error = resolve(&mut game, &mut q, &mut dm).err();
    let entering = game
        .objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name == def.name() && o.zone == Zone::Battlefield);
    let counters = entering
        .and_then(|o| o.counters.get(&CounterType::PlusOnePlusOne))
        .copied();
    let sacrificed: Vec<_> = [alice(), bob()]
        .into_iter()
        .map(|p| {
            game.objects_in_deterministic_order()
                .into_iter()
                .filter(|o| o.name == resource.name() && o.owner == p && o.zone == Zone::Graveyard)
                .count()
        })
        .collect();
    Ok(
        json!({"resolution_error":error,"mana_paid":paid,"cast_x":cast_x,"entered":entering.is_some(),"plus_one_counters":counters.unwrap_or(0),"sacrificed":sacrificed,"choice_trace":dm.choices}),
    )
}
fn record(
    rows: &mut Vec<Value>,
    name: &str,
    label: &str,
    expected: Value,
    result: Result<Value, String>,
) {
    let mut trace = Value::Null;
    let result = result.map(|mut v| {
        trace = v
            .as_object_mut()
            .unwrap()
            .remove("choice_trace")
            .unwrap_or(Value::Null);
        v
    });
    let (status, actual) = match result {
        Ok(v) if v == expected => ("passed", v),
        Ok(v) if !v["resolution_error"].is_null() => ("confirmed_resolution_failure", v),
        Ok(v) => ("semantic_mismatch", v),
        Err(e) => ("execution_or_fixture_error", json!({"error":e})),
    };
    rows.push(
        json!({"card":name,"scenario":label,"status":status,"expected":expected,"actual":actual,"decisions":trace}),
    );
}
#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_cast_counter_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_cast_counter_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = ["Gluttonous Hellkite", "Naya Soulbeast", "Triskelion"]
        .map(str::to_owned)
        .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut definitions = HashMap::new();
    let mut compilation = vec![];
    for p in payloads.into_values().flatten() {
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p.parse_name.as_deref().unwrap_or(&p.name),
        );
        let (a, d) =
            ironsmith_registry::compile_builder_to_artifact(builder, &p.parse_input, false)
                .unwrap();
        compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        definitions.insert(d.name().to_owned(), d);
    }
    let mut rows = vec![];
    for (x, resources, sacrificed) in [
        (0, [2, 2], [0, 0]),
        (1, [2, 2], [1, 1]),
        (2, [1, 2], [1, 2]),
    ] {
        let n = "Gluttonous Hellkite";
        record(
            &mut rows,
            n,
            &format!("paidcast X={x}, creatures {resources:?}"),
            json!({"resolution_error":null,"mana_paid":3+2*x,"cast_x":x,"entered":true,"plus_one_counters":2*sacrificed.iter().sum::<usize>(),"sacrificed":sacrificed}),
            probe(&definitions[n], x, resources, [1, 1]),
        );
    }
    for mvs in [[2, 3], [0, 4]] {
        let n = "Naya Soulbeast";
        record(
            &mut rows,
            n,
            &format!("paid8 cast, revealed mana values {mvs:?}"),
            json!({"resolution_error":null,"mana_paid":8,"cast_x":null,"entered":true,"plus_one_counters":mvs.iter().sum::<u8>(),"sacrificed":[0,0]}),
            probe(&definitions[n], 0, [0, 0], mvs),
        );
    }
    record(
        &mut rows,
        "Triskelion",
        "paid6 fixed three-entry-counter control",
        json!({"resolution_error":null,"mana_paid":6,"cast_x":null,"entered":true,"plus_one_counters":3,"sacrificed":[0,0]}),
        probe(&definitions["Triskelion"], 0, [0, 0], [1, 1]),
    );
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict canonical artifacts, actual paid casts and normal cast triggers; Gluttonous X=0/1/2 with sufficient/asymmetric sacrifice resources, Naya actual library mana values. Counter amounts measured on entered permanent.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/cast-counter-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("{}", serde_json::to_string(&report).unwrap());
}
