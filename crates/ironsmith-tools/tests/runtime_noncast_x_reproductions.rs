//! Noncast entry probes for a screened X-value family; completion is not whole-card correctness.
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
    decline: bool,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(g, c)
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

fn probe(def: &CardDefinition) -> Result<Value, String> {
    let mut game = setup();
    let witness = CardDefinitionBuilder::new(CardId::new(), "Noncast X audit filler")
        .card_types(vec![CardType::Creature, CardType::Artifact])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    for p in [alice(), bob()] {
        game.create_object_from_definition(&witness, p, Zone::Battlefield);
        for _ in 0..8 {
            game.create_object_from_definition(&witness, p, Zone::Library);
        }
    }
    let card = game.create_object_from_definition(def, alice(), Zone::Hand);
    let put = CardDefinitionBuilder::new(CardId::new(), "Noncast entry audit fixture")
        .card_types(vec![CardType::Artifact])
        .with_activated(
            ironsmith::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
            vec![Effect::put_onto_battlefield(
                ironsmith::target::ChooseSpec::SpecificObject(card),
                false,
                ironsmith::target::PlayerFilter::You,
            )],
        )
        .build();
    let source = game.create_object_from_definition(&put, alice(), Zone::Battlefield);
    let mut dm = ProbeDm { decline: false };
    let (mut q, paid) = perform(&mut game, source, Some(0), &mut dm)?;
    if paid != 1 {
        return Err(format!("fixture paid{paid} not1"));
    }
    let error = resolve(&mut game, &mut q, &mut dm).err();
    let source_object = game
        .objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name == def.name());
    Ok(
        json!({"resolution_error":error,"fixture_mana_paid":paid,"source_zone":source_object.map(|o|format!("{:?}",o.zone)),"source_x":source_object.and_then(|o|o.x_value),"source_counters":source_object.map(|o|format!("{:?}",o.counters)),"stack_remaining":game.stack.len()}),
    )
}
#[test]
#[ignore = "manual screened family audit; inspect report classifications"]
fn report_noncast_x_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let screen = root.join("reports/runtime-audit/noncast-x-candidates.json");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_noncast_x_reproductions.rs"),
        screen.clone(),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let screen: Value = serde_json::from_str(&std::fs::read_to_string(screen).unwrap()).unwrap();
    let names: Vec<String> = screen["card_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut payloads: Vec<_> = payloads.into_values().flatten().collect();
    payloads.sort_by(|a, b| a.name.cmp(&b.name));
    let mut compilation = vec![];
    let mut rows = vec![];
    let mut visited = std::collections::HashSet::new();
    for p in payloads {
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p.parse_name.as_deref().unwrap_or(&p.name),
        );
        let (a, d) = match ironsmith_registry::compile_builder_to_artifact(
            builder,
            &p.parse_input,
            false,
        ) {
            Ok(v) => v,
            Err(e) => {
                rows.push(json!({"card":p.name,"status":"strict_artifact_unavailable","error":e.to_string()}));
                continue;
            }
        };
        if !visited.insert(d.name().to_owned()) {
            rows.push(json!({"card":p.name,"canonical_name":d.name(),"status":"duplicate_face_alias_not_reexecuted"}));
            continue;
        }
        let aura = a
            .payload
            .definition
            .card
            .subtypes
            .iter()
            .any(|t| format!("{t:?}") == "Aura");
        compilation.push(json!({"card":d.name(),"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        if aura {
            rows.push(
                json!({"card":d.name(),"status":"fixture_excluded_aura_attachment_required"}),
            );
            continue;
        }
        eprintln!("noncast X probe started: {}", d.name());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| probe(&d)));
        let (status, actual) = match result {
            Ok(Ok(v)) if !v["resolution_error"].is_null() => ("runtime_exception_candidate", v),
            Ok(Ok(v)) => ("execution_completed_no_outcome_claim", v),
            Ok(Err(e)) => ("fixture_error", json!({"error":e})),
            Err(_) => (
                "panic_candidate",
                json!({"error":"panic in noncast entry fixture"}),
            ),
        };
        rows.push(json!({"card":d.name(),"status":status,"expected":{"resolution_error":null},"actual":actual}));
        std::fs::write(
            root.join("reports/runtime-audit/noncast-x-progress.json"),
            serde_json::to_string_pretty(&rows).unwrap(),
        )
        .unwrap();
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Screened X-base-cost self-ETB/entry-static family. Strict definitions enter from hand through an actually legal, paid generic fixture activation that puts the named permanent onto the battlefield. No spell was cast, so X defaults to zero. Runtime exceptions are candidates pending oracle/context review; successful completion is not an expected-outcome or whole-card pass. Aura attachment cases excluded. No state-based-actions claim.","rules_source":"https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt","rules":["107.3g","107.3j","107.3m"],"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/noncast-x-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!(
        "noncast family report: {} rows",
        report["rows"].as_array().unwrap().len()
    );
}
