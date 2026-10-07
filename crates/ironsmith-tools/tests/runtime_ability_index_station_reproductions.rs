//! Paid conditional-index transitions caused by costs and current-turn counter history.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
struct Choices {
    accept: bool,
    target: Option<Target>,
    trace: Vec<Value>,
    resources: Vec<ObjectId>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"decision":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let a = if let Some(o) = c
            .options
            .iter()
            .find(|o| o.legal && o.description == "Untap")
        {
            vec![o.index]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"decision":"options","context":format!("{c:?}"),"answer":a}));
        a
    }
    fn decide_objects(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let preferred: Vec<_> = self
            .resources
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
            .collect();
        let a = if preferred.len() >= c.min && !preferred.is_empty() {
            preferred
                .into_iter()
                .take(c.max.unwrap_or(self.resources.len()))
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(
            json!({"decision":"objects","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
        a
    }
    fn decide_targets(&mut self, _g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let a = c
            .requirements
            .iter()
            .filter_map(|r| {
                self.target
                    .filter(|t| r.legal_targets.contains(t))
                    .or_else(|| r.legal_targets.first().copied())
            })
            .collect::<Vec<_>>();
        self.trace.push(
            json!({"decision":"targets","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
        a
    }
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: PlayerId,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<u32, String> {
    eprintln!("AUDIT_STAGE cast {}", def.name());
    g.turn.priority_player = Some(actor);
    let id = g.create_object_from_definition(def, actor, Zone::Hand);
    let action = compute_legal_actions(g, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or_else(|| format!("{} normal cast unavailable", def.name()))?;
    let before = g.player(actor).unwrap().mana_pool.total();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace.push(json!({"stage":"legal_cast","card":def.name(),"actor":actor.index(),"action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_cast.is_none()
            && g.stack.iter().any(|e| {
                !e.is_ability && g.object(e.object_id).is_some_and(|o| o.name == def.name())
            })
        {
            let paid = before - g.player(actor).unwrap().mana_pool.total();
            dm.trace.push(json!({"stage":"cast_complete","card":def.name(),"paid":paid,"stack":format!("{:?}",g.stack)}));
            return Ok(paid);
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("cast announcement bound".into())
}
fn resolve_one(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    state.reset_for_new_priority_window(g);
    // The final pass invokes the production resolver, which retains the trigger
    // queue and therefore produces dedicated ETB events as well as zone changes.
    for _ in 0..g.players_in_game() {
        let progress = apply_priority_response_with_dm(
            g,
            q,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .map_err(|e| e.to_string())?;
        if let GameProgress::NeedsDecisionCtx(ctx) = &progress {
            if !matches!(
                ctx,
                ironsmith::decisions::context::DecisionContext::Priority(_)
            ) {
                return Err(format!(
                    "priority resolution requires unhandled decision:{ctx:?}"
                ));
            }
        }
    }
    Ok(())
}
fn drain(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    drain_pending_trigger_events(g, q);
    put_triggers_on_stack_with_dm(g, q, dm).map_err(|e| e.to_string())
}
fn resolve_all(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    for _ in 0..24 {
        drain(g, q, dm)?;
        if g.stack.is_empty() {
            return Ok(());
        }
        resolve_one(g, q, dm)?;
    }
    Err("bounded resolution did not empty stack".into())
}
fn announce(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    g.turn.priority_player = Some(PlayerId(0));
    eprintln!("AUDIT_STAGE activate {action:?}");
    let old = g.stack.len();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":"activation_action","action":format!("{action:?}")}));
    let mut p = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_activation.is_none() && g.stack.len() > old {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = p else {
            return Err(format!("activation stalled:{p:?}"));
        };
        p = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("activation bound".into())
}

fn mana(g: &mut GameState) {
    for p in [PlayerId(0), PlayerId(1)] {
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(symbol, 12);
        }
    }
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or_else(|| format!("{name} absent from battlefield"))
}

fn advance_turn(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    for e in ironsmith::turn::execute_draw_step_with(g, dm).unwrap() {
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t);
        }
    }
    resolve_all(g, q, dm)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(PlayerId(0));
    mana(g);
    Ok(())
}
fn announce_mana(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if let GameProgress::NeedsDecisionCtx(ctx) = progress {
            if matches!(
                ctx,
                ironsmith::decisions::context::DecisionContext::Priority(_)
            ) {
                return Ok(());
            }
            progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
                .map_err(|e| e.to_string())?;
        } else {
            return Ok(());
        }
    }
    Err("mana announcement bound".into())
}

fn activation(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    source: ObjectId,
    last: bool,
) -> Result<Value, String> {
    g.turn.priority_player = Some(PlayerId(0));
    let actions: Vec<_> = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .filter(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
        .collect();
    dm.trace.push(
        json!({"stage":"advertised_actions","all":format!("{actions:?}"),"select_last_pump":last}),
    );
    let a = if last {
        actions.last().cloned()
    } else {
        actions.first().cloned()
    };
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total() as i64;
    let mut error = None;
    let mut resolution_error = None;
    if let Some(action) = a.clone() {
        dm.trace.push(json!({"stage":"advertised_source_action","action":format!("{action:?}"),"abilities":format!("{:?}",g.current_abilities(source))}));
        error = if matches!(action, LegalAction::ActivateManaAbility { .. }) {
            announce_mana(g, q, dm, action).err()
        } else {
            announce(g, q, dm, action).err()
        };
        if error.is_none() {
            resolution_error = resolve_all(g, q, dm).err();
        }
    }
    Ok(
        json!({"offered":a.is_some(),"announcement_error":error,"resolution_error":resolution_error,"mana_paid":before-g.player(PlayerId(0)).unwrap().mana_pool.total() as i64,"remaining_stack":g.stack.len()}),
    )
}
fn run(
    def: &CardDefinition,
    hill: &CardDefinition,
    giganto: &CardDefinition,
    mode: usize,
) -> Result<Value, String> {
    use ironsmith::static_abilities::StaticAbilityId as K;
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704855);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    mana(&mut g);
    let mut q = TriggerQueue::new();
    let mut dm = Choices {
        accept: false,
        target: Some(Target::Player(PlayerId(1))),
        trace: vec![],
        resources: vec![],
    };
    let source_paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
    if source_paid != 5 {
        return Err("Wrong Crusher payment".into());
    }
    resolve_all(&mut g, &mut q, &mut dm)?;
    let source = find(&g, def.name())?;
    if g.player(PlayerId(1)).unwrap().life != 17 {
        return Err("Actual ETB did not deal three to Bob".into());
    }
    let mut resources: Vec<ObjectId> = vec![];
    let count = if mode == 2 {
        2
    } else if mode == 0 {
        0
    } else {
        1
    };
    for _ in 0..count {
        let creature = if mode == 3 { giganto } else { hill };
        dm.target = None;
        let paid = cast(&mut g, creature, PlayerId(0), &mut q, &mut dm)?;
        if paid != creature.card.mana_cost.as_ref().unwrap().mana_value() {
            return Err("Wrong station resource payment".into());
        }
        resolve_all(&mut g, &mut q, &mut dm)?;
        let id = *g
            .battlefield
            .iter()
            .find(|id| {
                g.object(**id).is_some_and(|o| o.name == creature.name())
                    && !resources.contains(*id)
            })
            .ok_or("paid station resource missing")?;
        resources.push(id);
    }
    for id in &resources {
        dm.resources = vec![*id];
        let a = activation(&mut g, &mut q, &mut dm, source, false)?;
        if a != json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":0,"remaining_stack":0})
        {
            return Err(format!("Actual station activation failed: {a}"));
        }
    }
    let expected_counters = [0, 4, 8, 10][mode];
    let creature_before = g.current_has_card_type(source, CardType::Creature);
    let flying_before = g.object_has_static_ability_id(source, K::Flying);
    let counters = g.counter_count(source, ironsmith::CounterType::Charge);
    let pump = activation(&mut g, &mut q, &mut dm, source, true)?;
    let expected = json!({"activation":{"offered":mode>=2,"announcement_error":null,"resolution_error":null,"mana_paid":if mode>=2{2}else{0},"remaining_stack":0},"charge_counters_before_pump":expected_counters,"charge_counters_after_station":expected_counters,"creature_before":expected_counters>=8,"creature_after":expected_counters>=8,"flying_before":expected_counters>=8,"flying_after":expected_counters>=8,"pump_applied_power":if mode>=2{Some(3)}else{None},"toughness":if mode>=2{Some(5)}else{None},"station_resources_tapped":count});
    let actual = json!({"activation":pump,"charge_counters_before_pump":counters,"charge_counters_after_station":g.counter_count(source,ironsmith::CounterType::Charge),"creature_before":creature_before,"creature_after":g.current_has_card_type(source,CardType::Creature),"flying_before":flying_before,"flying_after":g.object_has_static_ability_id(source,K::Flying),"pump_applied_power":g.calculated_power(source),"toughness":g.calculated_toughness(source),"station_resources_tapped":resources.iter().filter(|id|g.is_tapped(**id)).count()});
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":{"mode":mode,"source_cast_paid":source_paid,"station_counter_expectation_from_printed_power":expected_counters,"note":"CR721.2b: the pump is inside the eight-counter station striation. Below-threshold cases must not advertise or announce that ability. The fixture dispatches only an actually advertised pump; no missing action is forced."},"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_station_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_ability_index_station_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = ["Debris Field Crusher"];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names
        .into_iter()
        .chain(["Rumbling Baloth", "Gigantosaurus"])
    {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(name),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name, def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_STATIC_CARD").is_ok_and(|n| n != name) {
            continue;
        }
        let count = 4;
        for mode in 0..count {
            if std::env::var("AUDIT_STATIC_MODE").is_ok_and(|n| n != mode.to_string()) {
                continue;
            }
            eprintln!("AUDIT_CASE {name} {mode}");
            let (status, out) = match run(
                &defs[name],
                &defs["Rumbling Baloth"],
                &defs["Gigantosaurus"],
                mode,
            ) {
                Ok(out) => {
                    let status = if out["actual"]["activation"]["announcement_error"].is_string() {
                        "action_or_choice_failed"
                    } else if out["actual"]["activation"]["resolution_error"].is_string() {
                        "resolution_failed"
                    } else if out["expected"] == out["actual"] {
                        "expected_result_observed"
                    } else {
                        "semantic_mismatch"
                    };
                    (status, out)
                }
                Err(e) => (
                    "fixture_or_execution_error",
                    json!({"expected":null,"actual":{"error":e}}),
                ),
            };
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Paid canonical source and real printed producer actions, actual turn/untap transitions as required, resource-complete advertised activation. Exact condition characteristics, costs and outcome checked; linked-face and unrelated abilities remain outside scope."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    let out = std::env::var("AUDIT_STATIC_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            root.join("reports/runtime-audit/ability-index-station-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}
