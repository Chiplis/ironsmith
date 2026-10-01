//! Real stack targets and turn transitions for authored activation restrictions.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, Effect, GameState, ObjectId, PlayerId, Zone,
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
    target: Option<Target>,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_targets(&mut self, g: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let selected = if let Some(target) = &self.target {
            ctx.requirements
                .iter()
                .filter(|r| r.legal_targets.contains(target))
                .map(|_| target.clone())
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_targets(g, ctx)
        };
        self.trace.push(json!({"kind":"target_choice","context":format!("{ctx:?}"),"selected":format!("{selected:?}")}));
        selected
    }
}
fn announce(g: &mut GameState, action: LegalAction, dm: &mut Choices) -> Result<(), String> {
    let old = g.stack.len();
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"kind":"legal_action","action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && g.stack.len() > old
        {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(g, &mut queue, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("announcement bound".into())
}
fn cast(g: &mut GameState, def: &CardDefinition, dm: &mut Choices) -> Result<ObjectId, String> {
    let id = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    g.turn.priority_player = Some(PlayerId(0));
    let action = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==id))
        .ok_or("normal paid cast unavailable")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    announce(g, action, dm)?;
    let stack_id = g
        .stack
        .iter()
        .find(|entry| {
            g.object(entry.object_id)
                .is_some_and(|o| o.zone == Zone::Stack && o.name == def.name())
        })
        .ok_or("cast did not create actual spell object")?
        .object_id;
    dm.trace.push(json!({"kind":"paid_cast","card":def.name(),"mana_spent":before-g.player(PlayerId(0)).unwrap().mana_pool.total(),"stack_object":stack_id.0,"spells_cast_this_turn":g.turn_store.turn_history.spells_cast_by_player(PlayerId(0))}));
    Ok(stack_id)
}
fn process_triggers(g: &mut GameState, dm: &mut Choices, leave: usize) -> Result<(), String> {
    let mut q = TriggerQueue::new();
    for _ in 0..24 {
        drain_pending_trigger_events(g, &mut q);
        put_triggers_on_stack_with_dm(g, &mut q, dm).map_err(|e| e.to_string())?;
        if g.stack.len() <= leave {
            return Ok(());
        }
        resolve_stack_entry_with(g, dm).map_err(|e| e.to_string())?;
    }
    Err("setup trigger resolution bound".into())
}
fn scenario(def: &CardDefinition, n: usize) -> Result<(Value, Value), String> {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704850);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    let mut dm = Choices {
        target: None,
        trace: vec![],
    };
    g.player_mut(PlayerId(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 50);
    let instant = CardDefinitionBuilder::new(CardId::new(), "Stack gate setup instant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    for _ in 0..8 {
        g.create_object_from_definition(&instant, PlayerId(0), Zone::Library);
    }
    let index=def.abilities.iter().position(|a|matches!(&a.kind,AbilityKind::Activated(a) if a.additional_restrictions.iter().any(|r|r.contains("only if")))).ok_or("conditional activated ability not found")?;
    let source;
    if def.name() == "Rocket Launcher" {
        cast(&mut g, def, &mut dm)?;
        process_triggers(&mut g, &mut dm, 0)?;
        source = *g
            .battlefield
            .iter()
            .find(|id| g.object(**id).is_some_and(|o| o.name == def.name()))
            .ok_or("Rocket cast did not enter battlefield")?;
        dm.trace
            .push(json!({"kind":"source_entered","turn":g.turn.turn_number,"source":source.0}));
        for _ in 0..n {
            g.next_turn();
            ironsmith::turn::execute_untap_step(&mut g);
            dm.trace.push(json!({"kind":"next_turn","turn":g.turn.turn_number,"active_player":g.turn.active_player.index()}));
        }
        // Each next_turn performs the real turn-start history update. The test
        // observes the later first-main priority window without another cast.
        g.turn.phase = ironsmith::Phase::FirstMain;
        g.turn.step = None;
        g.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 5);
        dm.target = Some(Target::Player(PlayerId(1)));
    } else {
        source = g.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
        g.remove_summoning_sickness(source);
        g.effect_store.pending_trigger_events.clear();
        if def.name() == "Kitsa, Otterball Elite" {
            g.add_counters(source, CounterType::PlusOnePlusOne, n as u32);
        } else {
            for _ in 0..n {
                cast(&mut g, &instant, &mut dm)?;
                process_triggers(&mut g, &mut dm, 0)?;
            }
        }
        let target = cast(&mut g, &instant, &mut dm)?;
        process_triggers(&mut g, &mut dm, 1)?;
        if g.stack.len() != 1 || g.stack[0].object_id != target {
            return Err("actual target spell not retained on stack".into());
        }
        if def.name() == "Kitsa, Otterball Elite"
            && g.calculated_power(source) != Some(n as i32 + 2)
        {
            return Err(format!(
                "prowess setup power mismatch:{:?}",
                g.calculated_power(source)
            ));
        }
        if def.name() == "Stella Lee, Wild Card"
            && g.turn_store.turn_history.spells_cast_by_player(PlayerId(0)) != n as u32 + 1
        {
            return Err("spell history setup mismatch".into());
        }
        dm.target = Some(Target::Object(target));
    }
    g.turn.priority_player = Some(PlayerId(0));
    let mut evidence = json!({"ability_index":index,"source":source.0,"source_power":g.calculated_power(source),"source_tapped":g.is_tapped(source),"turn":g.turn.turn_number,"active_player":g.turn.active_player.index(),"spells_cast_this_turn":g.turn_store.turn_history.spells_cast_by_player(PlayerId(0)),"turn_history":format!("{:?}",g.turn_store.turn_history),"source_state":format!("{:?}",g.object(source)),"stack_before":format!("{:?}",g.stack),"target":format!("{:?}",dm.target)});
    let action=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index:i} if *s==source&&*i==index));
    let offered = action.is_some();
    let mut announced = false;
    if let Some(action) = action {
        let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
        announce(&mut g, action, &mut dm)?;
        announced = true;
        evidence["activation_mana_spent"] =
            json!(before - g.player(PlayerId(0)).unwrap().mana_pool.total());
        evidence["source_tapped_after"] = json!(g.is_tapped(source));
        evidence["stack_after"] = json!(format!("{:?}", g.stack));
    }
    evidence["trace"] = json!(dm.trace);
    Ok((
        json!({"activation_offered":offered,"activation_announced":announced}),
        evidence,
    ))
}
#[test]
#[ignore = "audit observations; no whole-card correctness assertion"]
fn report_stack_history_gates() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_stack_history_gate_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let data: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let mut rows = vec![];
    let mut compilation = vec![];
    for name in [
        "Kitsa, Otterball Elite",
        "Stella Lee, Wild Card",
        "Rocket Launcher",
    ] {
        let p = data["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compilation.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        let cases: Vec<_> = match name {
            "Kitsa, Otterball Elite" => vec![(0, false), (1, true), (2, true)],
            "Stella Lee, Wild Card" => vec![(0, false), (1, false), (2, true), (3, true)],
            _ => vec![(0, false), (1, false), (2, true)],
        };
        for (n, allowed) in cases {
            let expected = json!({"activation_offered":allowed,"activation_announced":allowed});
            let (status, actual, evidence) = match scenario(&def, n) {
                Ok((a, e)) => (
                    if a == expected {
                        "expected_result_observed"
                    } else {
                        "semantic_mismatch"
                    },
                    a,
                    e,
                ),
                Err(e) => (
                    "fixture_or_execution_error",
                    json!({"error":e}),
                    Value::Null,
                ),
            };
            rows.push(json!({"card":name,"scenario":{"parameter":n,"parameter_meaning":match name{"Kitsa, Otterball Elite"=>"plus-one counters before one actual noncreature cast and resolved prowess","Stella Lee, Wild Card"=>"actual prior spells cast and resolved before final target instant cast",_=>"actual turn transitions after paid source cast"}},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence,"artifact_checksum":artifact.payload_checksum,"scope":"Normal legal-action discovery and completed activation announcement with valid target and paid costs. Kitsa/Stella use historical established sources and actual paid instant casts. Rocket is actually cast, then zero/one/two engine turn transitions occur. Activated ability resolution is outside this gate test."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704850_u64}});
    std::fs::write(
        root.join("reports/runtime-audit/stack-history-gate-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "wrote {} stack and history cases",
        report["rows"].as_array().unwrap().len()
    );
}
