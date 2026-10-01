//! Paid-cast X context probes from imported MAGE leads.
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
    x: u32,
    kicked: bool,
    copy_name: Option<String>,
    accept_copy: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        let answer = if c.is_x_value { self.x } else { c.min };
        self.trace.push(json!({"kind":"number","description":c.description,"is_x":c.is_x_value,"answer":answer}));
        answer
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"kind":"boolean","description":c.description,"answer":self.accept_copy}));
        self.accept_copy
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let answer = self
            .copy_name
            .as_ref()
            .and_then(|name| {
                c.candidates
                    .iter()
                    .find(|o| o.legal && g.object(o.id).is_some_and(|object| object.name == *name))
            })
            .map(|o| vec![o.id])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_objects(g, c));
        self.trace.push(json!({"kind":"objects","description":c.description,"selected":answer.iter().map(|id|format!("{id:?}")).collect::<Vec<_>>()}));
        answer
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        if c.description == "Choose which replacement effect to apply" && !self.accept_copy {
            self.trace.push(json!({"kind":"replacement_decline","description":c.description,"answer":[0],"options":c.options.iter().map(|o|json!({"index":o.index,"description":o.description})).collect::<Vec<_>>() }));
            return vec![0];
        }
        let answer = self
            .copy_name
            .as_ref()
            .and_then(|name| c.options.iter().find(|o| o.description.contains(name)))
            .map(|o| vec![o.index])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(g, c));
        self.trace
            .push(json!({"kind":"options","description":c.description,"answer":answer}));
        answer
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        SelectFirstDecisionMaker.decide_targets(g, c)
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
                dm.trace.push(json!({"kind":"optional_costs","description":c.description,"answer":if dm.kicked{vec![(0,1)]}else{vec![]}}));
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(if dm.kicked { vec![(0, 1)] } else { vec![] }),
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

fn dm(x: u32, kicked: bool) -> ProbeDm {
    ProbeDm {
        x,
        kicked,
        copy_name: None,
        accept_copy: false,
        trace: vec![],
    }
}
fn count(g: &GameState, pattern: &str) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.zone == Zone::Battlefield && o.name.contains(pattern))
        .count()
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    dm: &mut ProbeDm,
) -> Result<(u32, Option<u32>, String), String> {
    let id = g.create_object_from_definition(def, alice(), Zone::Hand);
    let (mut q, paid) = perform(g, id, None, dm)?;
    let entry = g.stack.last().ok_or("missing cast entry")?;
    let x = entry.x_value;
    let optional = format!("{:?}", entry.optional_costs_paid);
    let error = resolve(g, &mut q, dm).err();
    Ok((
        paid,
        x,
        error.unwrap_or_else(|| format!("optional_costs={optional}")),
    ))
}
fn verdeloth(def: &CardDefinition, kicked: bool, x: u32) -> Result<Value, String> {
    let mut game = setup();
    let mut dm = dm(x, kicked);
    let (paid, cast_x, info) = cast(&mut game, def, &mut dm)?;
    let error = if info.starts_with("optional_costs=") {
        None
    } else {
        Some(info.clone())
    };
    Ok(
        json!({"actual":{"resolution_error":error,"mana_paid":paid,"cast_x":cast_x,"saprolings":count(&game,"Saproling"),"verdeloth":count(&game,"Verdeloth")},"cast_metadata":info,"decisions":dm.trace}),
    )
}
fn skydiver(def: &CardDefinition, kicked: bool, x: u32) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let witness = CardDefinitionBuilder::new(CardId::new(), "X kicker artifact witness")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            x.max(1) as u8
        )]))
        .build();
    let target = game.create_object_from_definition(&witness, bob, Zone::Battlefield);
    let mut dm = dm(x, kicked);
    let (paid, cast_x, info) = cast(&mut game, def, &mut dm)?;
    let error = if info.starts_with("optional_costs=") {
        None
    } else {
        Some(info.clone())
    };
    Ok(
        json!({"actual":{"resolution_error":error,"mana_paid":paid,"cast_x":cast_x,"target_controlled_by_alice":game.controller_of_id(target)==Some(alice()),"skydiver":count(&game,"Thieving Skydiver")},"cast_metadata":info,"decisions":dm.trace}),
    )
}
fn defenders(
    def: &CardDefinition,
    clone: Option<&CardDefinition>,
    x: u32,
    accept_copy: bool,
) -> Result<Value, String> {
    let mut game = setup();
    let mut caster = dm(x, false);
    let (paid, cast_x, info) = cast(&mut game, def, &mut caster)?;
    if !info.starts_with("optional_costs=") {
        return Ok(
            json!({"actual":{"stage":"original cast","resolution_error":info,"mana_paid":paid,"cast_x":cast_x,"tokens":count(&game,"Astartes"),"defenders":count(&game,"Defenders of Humanity")},"decisions":caster.trace}),
        );
    }
    let mut total_paid = paid;
    let mut error = None;
    if let Some(clone) = clone {
        caster.copy_name = Some("Defenders of Humanity".into());
        caster.accept_copy = accept_copy;
        caster.x = 0;
        let (clone_paid, _, info) = cast(&mut game, clone, &mut caster)?;
        total_paid += clone_paid;
        if !info.starts_with("optional_costs=") {
            error = Some(info);
        }
    }
    Ok(
        json!({"actual":{"stage":if clone.is_some(){"copy cast"}else{"original cast"},"resolution_error":error,"mana_paid":total_paid,"cast_x":cast_x,"tokens":count(&game,"Astartes"),"defenders":count(&game,"Defenders of Humanity")},"decisions":caster.trace}),
    )
}
fn record(
    rows: &mut Vec<Value>,
    name: &str,
    label: &str,
    expected: Value,
    result: Result<Value, String>,
) {
    let (status, actual, extra) = match result {
        Ok(v) => {
            let actual = v["actual"].clone();
            let status = if actual == expected {
                "passed"
            } else if !actual["resolution_error"].is_null() {
                "confirmed_resolution_failure"
            } else {
                "semantic_mismatch"
            };
            (status, actual, v)
        }
        Err(e) => (
            "execution_or_fixture_error",
            json!({"error":e}),
            Value::Null,
        ),
    };
    rows.push(json!({"card":name,"scenario":label,"status":status,"expected":expected,"actual":actual,"details":extra}));
}
#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_x_context_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_x_context_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = [
        "Verdeloth the Ancient",
        "Defenders of Humanity",
        "Clever Impersonator",
        "Thieving Skydiver",
    ]
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
    for (kicked, x) in [(false, 0), (true, 0), (true, 2)] {
        record(
            &mut rows,
            "Verdeloth the Ancient",
            &format!("paidcast kicker={kicked}, X={x}"),
            json!({"resolution_error":null,"mana_paid":6+if kicked{x}else{0},"cast_x":if kicked{Some(x)}else{None},"saprolings":if kicked{x}else{0},"verdeloth":1}),
            verdeloth(&definitions["Verdeloth the Ancient"], kicked, x),
        );
    }
    for (kicked, x) in [(false, 0), (true, 1), (true, 2)] {
        record(
            &mut rows,
            "Thieving Skydiver",
            &format!("kicker={kicked}, X={x}, opponent artifact MV{}", x.max(1)),
            json!({"resolution_error":null,"mana_paid":2+if kicked{x}else{0},"cast_x":if kicked{Some(x)}else{None},"target_controlled_by_alice":kicked,"skydiver":1}),
            skydiver(&definitions["Thieving Skydiver"], kicked, x),
        );
    }
    for x in [0, 1, 3] {
        record(
            &mut rows,
            "Defenders of Humanity",
            &format!("original paid cast X={x}"),
            json!({"stage":"original cast","resolution_error":null,"mana_paid":3+x,"cast_x":x,"tokens":x,"defenders":1}),
            defenders(&definitions["Defenders of Humanity"], None, x, false),
        );
    }
    for accept in [false, true] {
        record(
            &mut rows,
            "Clever Impersonator",
            &format!("paid4 after actual Defenders X1, copy={accept}"),
            json!({"stage":"copy cast","resolution_error":null,"mana_paid":8,"cast_x":1,"tokens":1,"defenders":if accept{2}else{1}}),
            defenders(
                &definitions["Defenders of Humanity"],
                Some(&definitions["Clever Impersonator"]),
                1,
                accept,
            ),
        );
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict paid casts, explicit kicker/X and copy choices. Verdeloth is the actual failing source in RosheenMeandererManaX imported tests; no Rosheen defect inferred. Clever Impersonator copies an actually cast Defenders permanent; X decisions do not carry into a permanent copy.","rules_source":"https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt","rules":["107.3a","107.3j","107.3m","707.2"],"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/x-context-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("{}", serde_json::to_string(&report).unwrap());
}
