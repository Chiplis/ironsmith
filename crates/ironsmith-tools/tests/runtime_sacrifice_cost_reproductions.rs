//! Strict artifact, paid-cast sacrifice discount probes; stop before spell resolution.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
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
    wanted: usize,
    witness_ids: Vec<ObjectId>,
    choices: Vec<Value>,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        let answer = self.wanted > 0;
        self.choices
            .push(json!({"kind":"boolean","description":c.description,"answer":answer}));
        answer
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected: Vec<_> = c
            .candidates
            .iter()
            .filter(|o| self.witness_ids.contains(&o.id))
            .take(self.wanted)
            .map(|o| o.id)
            .collect();
        let answer = if selected.len() >= c.min {
            selected
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.choices.push(json!({"kind":"objects","description":c.description,"min":c.min,"max":c.max,"selected":answer.iter().map(|id|format!("{id:?}")).collect::<Vec<_>>() }));
        answer
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        SelectFirstDecisionMaker.decide_targets(g, c)
    }
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
        .ok_or("intended legal action unavailable")?;
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

fn probe(def: &CardDefinition, available: usize, wanted: usize) -> Value {
    let mut game = setup();
    let witness = CardDefinitionBuilder::new(CardId::new(), "Sacrifice audit witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let witness_ids = (0..available)
        .map(|_| game.create_object_from_definition(&witness, alice(), Zone::Battlefield))
        .collect();
    let source = game.create_object_from_definition(def, alice(), Zone::Hand);
    let mut dm = ProbeDm {
        wanted,
        witness_ids,
        choices: vec![],
    };
    let before = game.player(alice()).unwrap().mana_pool.total();
    let error = perform(&mut game, source, None, &mut dm).err();
    let count = |zone| {
        game.objects_in_deterministic_order()
            .into_iter()
            .filter(|o| o.name == "Sacrifice audit witness" && o.zone == zone)
            .count()
    };
    json!({"actual":{"cast_error":error,"reached_stack":game.stack.iter().any(|entry| game.object(entry.object_id).is_some_and(|o| o.name == def.name())),"mana_paid":before-game.player(alice()).unwrap().mana_pool.total(),"sacrificed":count(Zone::Graveyard),"witnesses_remaining":count(Zone::Battlefield)},"choices":dm.choices})
}
#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_sacrifice_cost_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_sacrifice_cost_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = [
        "Dargo, the Shipwrecker",
        "Rottenmouth Viper",
        "Awaken the Blood Avatar",
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
    for (name, base, discount) in [
        ("Dargo, the Shipwrecker", 7, 2),
        ("Rottenmouth Viper", 6, 1),
        ("Awaken the Blood Avatar", 8, 2),
    ] {
        for (available, wanted) in [(0, 0), (2, 0), (2, 1), (2, 2)] {
            let result = probe(&definitions[name], available, wanted);
            let expected = json!({"cast_error":null,"reached_stack":true,"mana_paid":base-discount*wanted,"sacrificed":wanted,"witnesses_remaining":available-wanted});
            let actual = &result["actual"];
            let status = if actual == &expected {
                "passed"
            } else if actual["cast_error"].is_null() {
                "semantic_mismatch"
            } else {
                "cast_failure_candidate"
            };
            rows.push(json!({"card":name,"scenario":format!("ordinary paid cast with {wanted} chosen sacrifices, {available} eligible witnesses"),"status":status,"expected":expected,"actual":actual,"decisions":result["choices"]}));
        }
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict canonical artifacts, ordinary legal casting with ample mana and zero/two eligible creature resources; explicit decline/one/two selections. Stops before spell resolution to isolate cast costs; no oracle fallback or assertion repair.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/sacrifice-cost-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("{}", serde_json::to_string(&report).unwrap());
}
