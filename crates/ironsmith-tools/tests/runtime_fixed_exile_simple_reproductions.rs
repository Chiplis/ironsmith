//! Canonical paid rest-tag consumers with independent exact zone/order expectations.

use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, OrderContext, SelectObjectsContext,
    SelectOptionsContext, TargetsContext, ViewCardsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CounterType, GameState, ObjectId, Phase, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
struct Dm {
    targets: Vec<Target>,
    stage: String,
    trace: Vec<Value>,
    chosen: Vec<ObjectId>,
    reverse: bool,
    x: u32,
    plot: bool,
}
impl DecisionMaker for Dm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace.push(json!({"stage":self.stage,"choice":"boolean","context":format!("{c:?}"),"selected":self.plot}));
        self.plot
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        assert_eq!(c.requirements.len(), 1);
        let r = &c.requirements[0];
        assert!(
            self.targets.len() >= r.min_targets
                && r.max_targets.is_none_or(|m| self.targets.len() <= m)
        );
        assert!(self.targets.iter().all(|t| r.legal_targets.contains(t)));
        self.trace.push(json!({"stage":self.stage,"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace.push(json!({"stage":self.stage,"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        self.trace.push(json!({"stage":self.stage,"choice":"number","context":format!("{c:?}"),"selected":self.x}));
        self.x
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if c.player == PlayerId(1) {
            SelectFirstDecisionMaker.decide_objects(g, c)
        } else {
            self.chosen
                .iter()
                .copied()
                .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
                .take(c.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>()
        };
        assert!(selected.len() >= c.min.min(c.candidates.iter().filter(|o| o.legal).count()));
        self.trace.push(json!({"stage":self.stage,"choice":"objects","context":format!("{c:?}"),"selected":selected.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}));
        selected
    }
    fn decide_order(&mut self, g: &GameState, c: &OrderContext) -> Vec<ObjectId> {
        let mut selected = c.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        if c.description.to_lowercase().contains("library") {
            selected.sort_by_key(|id| {
                g.object(*id)
                    .map(|o| o.name.to_string())
                    .unwrap_or_default()
            });
            if self.reverse {
                selected.reverse();
            }
        }
        self.trace.push(json!({"stage":self.stage,"choice":"order","context":format!("{c:?}"),"selected":selected.iter().map(|id|g.object(*id).map(|o|o.name.to_string())).collect::<Vec<_>>()}));
        selected
    }
    fn view_cards(
        &mut self,
        g: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        c: &ViewCardsContext,
    ) {
        self.trace.push(json!({"stage":self.stage,"choice":"view","context":format!("{c:?}"),"viewer":viewer.0,"cards":cards.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}));
    }
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    g.set_random_seed(SEED);
    g.turn.turn_number = 3;
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    for s in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(alice()).unwrap().mana_pool.add(s, 30);
    }
    g
}
fn announce(
    g: &mut GameState,
    action: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
    let mut state = PriorityLoopState::new(g.players_in_game());
    let initial = g.stack.len();
    dm.trace
        .push(json!({"stage":dm.stage,"action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none()
            && state.pending_activation.is_none()
            && g.stack.len() > initial
        {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(c) = progress else {
            return Err(format!("announcement stopped:{progress:?}"));
        };
        if matches!(c, DecisionContext::Priority(_)) {
            return Err("announcement returned priority before ability/spell stacked".into());
        }
        progress =
            apply_decision_context_with_dm(g, q, &mut state, &c, dm).map_err(|e| e.to_string())?;
    }
    Err("announcement budget".into())
}
fn one(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
    advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
    let mut state = PriorityLoopState::new(g.players_in_game());
    state.reset_for_new_priority_window(g);
    for _ in 0..g.players_in_game() {
        apply_priority_response_with_dm(
            g,
            q,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .map_err(|e| e.to_string())?;
    }
    check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
    advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
    Ok(())
}
fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    for _ in 0..24 {
        check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        if g.stack.is_empty() {
            return Ok(());
        }
        one(g, q, dm)?;
    }
    Err("resolution budget".into())
}
fn cast_announce(
    g: &mut GameState,
    d: &CardDefinition,
    cost: u32,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
    let id = g.create_object_from_definition(d, alice(), Zone::Hand);
    let a = compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("fixture source cast absent")?;
    let before = g.player(alice()).unwrap().mana_pool.total();
    announce(g, a, q, dm)?;
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    dm.trace
        .push(json!({"stage":"actual_paid_cast","card":d.name(),"paid":paid}));
    if paid != cost {
        return Err(format!(
            "fixture {} expected castcost {cost}, paid {paid}",
            d.name()
        ));
    }
    Ok(())
}

fn paid_cast(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    n: &str,
    cost: u32,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let before = g
        .objects_in_deterministic_order()
        .into_iter()
        .map(|o| o.stable_id)
        .collect::<Vec<_>>();
    dm.stage = format!("paid_cast_{n}");
    let definition = &defs
        .get(n)
        .ok_or_else(|| format!("canonical helper did not compile: {n}"))?
        .0;
    cast_announce(g, definition, cost, q, dm)?;
    dm.stage = format!("resolve_{n}");
    finish(g, q, dm)?;
    g.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name == defs[n].0.name() && !before.contains(&o.stable_id))
        .map(|o| o.stable_id)
        .ok_or(format!("cast resource {n} absent after resolution"))
}
fn current(g: &GameState, id: ironsmith::ids::StableId) -> ObjectId {
    g.find_object_by_stable_id(id)
        .expect("current resource incarnation")
}

fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let mut g = game();
    let mut q = TriggerQueue::new();
    let total = c["resources"].as_u64().unwrap() as usize;
    let required = c["required"].as_u64().unwrap() as usize;
    let index = c["ability_index"].as_u64().unwrap() as usize;
    let source_stable = paid_cast(
        &mut g,
        defs,
        n,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    let source = current(&g, source_stable);
    let altar_stable = paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?;
    let altar = current(&g, altar_stable);
    let mut resources = vec![];
    for i in 0..total {
        dm.targets.clear();
        dm.chosen.clear();
        let stable = paid_cast(&mut g, defs, "Ornithopter", 0, &mut q, dm)?;
        let id = current(&g, stable);
        dm.chosen = vec![id];
        dm.targets = vec![Target::Player(PlayerId(1))];
        dm.stage = format!("actual_resource_sacrifice_{i}");
        let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index:0,..}if *s==altar)).ok_or("resource Altar activation absent")?;
        announce(&mut g, a, &mut q, dm)?;
        finish(&mut g, &mut q, dm)?;
        assert_eq!(g.object(current(&g, stable)).unwrap().zone, Zone::Graveyard);
        resources.push(stable);
    }
    assert_eq!(
        g.player(alice()).unwrap().graveyard.len(),
        total,
        "only deliberate eligible resources in own graveyard"
    );
    dm.chosen = resources
        .iter()
        .take(required)
        .map(|s| current(&g, *s))
        .collect();
    dm.targets.clear();
    g.remove_summoning_sickness(source);
    let outcome = c["outcome"].as_str().unwrap();
    if ["discard", "damage2", "lose3"].contains(&outcome) {
        dm.targets = vec![Target::Player(PlayerId(1))];
    }
    if outcome == "discard" {
        g.create_object_from_definition(&defs["Forest"].0, PlayerId(1), Zone::Hand);
    }
    let target = if outcome == "return" {
        let stable = *resources.last().unwrap();
        dm.targets = vec![Target::Object(current(&g, stable))];
        Some(stable)
    } else {
        None
    };
    let legal = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let action=legal.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index:i,..}if *s==source&&*i==index)).cloned();
    let ability = match &defs[n].0.abilities[index].kind {
        ironsmith::ability::AbilityKind::Activated(a) => a,
        _ => return Err("wrong ability kind".into()),
    };
    dm.trace.push(json!({"stage":"candidate_cost_path","ability_index":index,"cost_path":c["cost_path"],"resource_count":total,"chosen_cost_ids":dm.chosen.iter().map(|id|id.0).collect::<Vec<_>>(),"graveyard_count":g.player(alice()).unwrap().graveyard.len(),"legal_actions":format!("{legal:?}"),"source_tapped":g.is_tapped(source),"total_cost_check":format!("{:?}",ironsmith::cost::can_pay_cost_with_reason(&g,source,alice(),&ability.mana_cost,ironsmith::costs::PaymentReason::ActivateAbility))}));
    let valid = c["valid"].as_bool().unwrap();
    if action.is_none() || !valid {
        return Ok((
            json!({"error":null,"legal_activation_available":valid}),
            json!({"error":null,"legal_activation_available":action.is_some()}),
        ));
    }
    dm.stage = "actual_candidate_activation".into();
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut error = announce(&mut g, action.unwrap(), &mut q, dm).err();
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    let cost_zones = resources
        .iter()
        .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
        .collect::<Vec<_>>();
    dm.trace.push(json!({"stage":"cost_committed","mana_paid":paid,"resource_zones":cost_zones,"source_tapped":g.is_tapped(source),"error":error}));
    if error.is_none() {
        dm.stage = "candidate_resolution".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let mut expected = json!({"error":null,"legal_activation_available":true,"mana_paid":c["activation_mana"],"cost_zones":(0..total).map(|i|if i<required{"Exile"}else{"Graveyard"}).collect::<Vec<_>>(),"source_tapped":c["tap"]});
    let mut actual = json!({"error":error,"legal_activation_available":true,"mana_paid":paid,"cost_zones":cost_zones,"source_tapped":g.is_tapped(source)});
    match outcome {
        "Bear" | "Skeleton" | "Wolf" | "Zombie" => {
            expected["tokens"] = json!([{ "name":outcome,"power":c["power"],"toughness":c["toughness"],"tapped":c["token_tapped"]}]);
            actual["tokens"]=json!(g.battlefield.iter().filter_map(|id|g.object(*id).filter(|o|matches!(o.kind,ironsmith::object::ObjectKind::Token)).map(|o|json!({"name":o.name.to_string(),"power":g.calculated_power(*id),"toughness":g.calculated_toughness(*id),"tapped":g.is_tapped(*id)}))).collect::<Vec<_>>());
        }
        "discard" => {
            expected["bob_hand"] = json!(0);
            actual["bob_hand"] = json!(g.player(PlayerId(1)).unwrap().hand.len());
            expected["bob_graveyard"] = json!(1);
            actual["bob_graveyard"] = json!(g.player(PlayerId(1)).unwrap().graveyard.len());
        }
        "damage2" | "lose3" => {
            expected["bob_life"] = json!(if outcome == "damage2" { 18 } else { 17 });
            actual["bob_life"] = json!(g.player(PlayerId(1)).unwrap().life);
        }
        "pump" => {
            expected["source_pt"] = json!([c["power"], c["toughness"]]);
            actual["source_pt"] =
                json!([g.calculated_power(source), g.calculated_toughness(source)]);
        }
        "first_strike" | "flying" => {
            let keyword = if outcome == "flying" {
                ironsmith::static_abilities::StaticAbilityId::Flying
            } else {
                ironsmith::static_abilities::StaticAbilityId::FirstStrike
            };
            expected["gained_keyword"] = json!(true);
            actual["gained_keyword"] = json!(g.current_has_static_ability_id(source, keyword));
        }
        "indestructible" => {
            expected["indestructible_counters"] = json!(1);
            actual["indestructible_counters"] =
                json!(g.counter_count(source, CounterType::Indestructible));
        }
        "return" => {
            expected["target_final_zone"] = json!(if total == required { "Exile" } else { "Hand" });
            actual["target_final_zone"] = json!(format!(
                "{:?}",
                g.object(current(&g, target.unwrap())).unwrap().zone
            ));
        }
        _ => unreachable!(),
    }
    Ok((expected, actual))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn generate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let p = root.join("reports/runtime-audit");
    let input = p.join("fixed-exile-simple-inputs.json");
    let data: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    let mut rows = vec![];
    for (n, v) in data["cards"].as_object().unwrap() {
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            v["parse_name"].as_str().unwrap_or(n),
        );
        match ironsmith_registry::compile_builder_to_artifact(
            b,
            v["parse_input"].as_str().unwrap(),
            false,
        ) {
            Ok((a, d)) => {
                artifacts.push(json!({"card":n,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
                defs.insert(n.clone(), (d, a.payload_checksum));
            }
            Err(e) => rows
                .push(json!({"card":n,"status":"compile_failed","actual":{"error":e.to_string()}})),
        }
    }
    for c in data["cases"].as_array().unwrap() {
        let n = c["card"].as_str().unwrap();
        eprintln!("FIXED_EXILE_SIMPLE {c}");
        let mut dm = Dm {
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: c["reverse"].as_bool().unwrap_or(false),
            x: c["x"].as_u64().unwrap_or(0) as u32,
            plot: c["plot"].as_bool().unwrap_or(false),
        };
        let result = run(&defs, c, &mut dm);
        let (status, expected, actual) = match result {
            Ok((e, a)) => (
                if a == e {
                    "expected_result_observed"
                } else if !a["error"].is_null() {
                    "resolution_failed"
                } else {
                    "semantic_mismatch"
                },
                e,
                a,
            ),
            Err(e) => (
                "execution_or_fixture_error",
                Value::Null,
                json!({"error":e}),
            ),
        };
        rows.push(json!({"card":n,"scenario":c,"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs[n].1,"execution_trace":dm.trace}));
    }
    let binary = std::env::current_exe().unwrap();
    let files = [
        "crates/ironsmith-engine/src/effects/zones/move_to_zone.rs",
        "crates/ironsmith-engine/src/effects/cards/look_at_top.rs",
        "crates/ironsmith-engine/src/effects/composition/choose_objects.rs",
    ];
    let report = json!({"scope":"Fourteen fixed-count exile-cost paths with required-minus-one/exact/extra resources and independent immediate outcomes. Every graveyard resource is a full paid Ornithopter sacrificed through an actual paid Altar of Dementia activation; zero power ensures no unrelated mill resources. Canonical source fully paid, correct actual ability selected; no missing action forced.","limitations":"Mana and Bob discard-hand witness seeded. Source summoning sickness cleared as explicit elapsed-turn setup. Normal priority/SBAs, actual cost choices and exact committed zones. Cabal Inquisitor also covers threshold6/7/8. Surgeon exact2 case legally exiles its targeted card as cost, so target is expected to remain exiled; extra3 preserves target and returns it. Temporary effect expiry and secondary token abilities are outside this immediate-outcome scope.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_fixed_exile_simple_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("fixed-exile-simple-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("fixed-exile-simple-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical fixed-exile-simple expected-result reporter"]
fn report_fixed_exile_simple() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
