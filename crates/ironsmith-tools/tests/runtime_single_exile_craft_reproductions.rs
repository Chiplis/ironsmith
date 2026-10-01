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
    actor: PlayerId,
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
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let mut picked = SelectFirstDecisionMaker.decide_targets(g, c);
        if c.requirements.len() == 1 {
            let r = &c.requirements[0];
            if self.targets.len() >= r.min_targets
                && r.max_targets.is_none_or(|m| self.targets.len() <= m)
                && self.targets.iter().all(|t| r.legal_targets.contains(t))
            {
                picked = self.targets.clone();
            }
        }
        self.trace.push(json!({"stage":self.stage,"choice":"targets","context":format!("{c:?}"),"selected":format!("{picked:?}")}));
        self.targets = picked.clone();
        picked
    }
    fn decide_distribute(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        let selected = c
            .targets
            .first()
            .map(|t| vec![(t.target, c.total)])
            .unwrap_or_default();
        self.trace.push(json!({"stage":self.stage,"choice":"distribution","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
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
        let mut selected = self
            .chosen
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
            .take(c.max.unwrap_or(usize::MAX))
            .collect::<Vec<_>>();
        if self.stage == "craft_resolution_with_canonical_metadata" && selected.len() < c.min {
            selected = SelectFirstDecisionMaker.decide_objects(g, c);
        }
        assert!(
            selected.len() >= c.min.min(c.candidates.iter().filter(|o| o.legal).count()),
            "selection fixture: {c:?}, requested={:?}, selected={selected:?}",
            self.chosen
        );
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
        for player in [alice(), PlayerId(1)] {
            g.player_mut(player).unwrap().mana_pool.add(s, 30);
        }
    }
    g
}
fn announce(
    g: &mut GameState,
    action: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(dm.actor);
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
    g.turn.priority_player = Some(dm.actor);
    let id = g.create_object_from_definition(d, dm.actor, Zone::Hand);
    let a = compute_legal_actions(g, dm.actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("fixture source cast absent")?;
    let before = g.player(dm.actor).unwrap().mana_pool.total();
    announce(g, a, q, dm)?;
    let paid = before - g.player(dm.actor).unwrap().mana_pool.total();
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

fn set_actor(g: &mut GameState, dm: &mut Dm, p: PlayerId) {
    dm.actor = p;
    g.turn.active_player = p;
    g.turn.priority_player = Some(p);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
}
fn immediate(
    g: &mut GameState,
    a: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(dm.actor);
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":dm.stage,"action":format!("{a:?}")}));
    let mut p =
        apply_priority_response_with_dm(g, q, &mut state, &PriorityResponse::PriorityAction(a), dm)
            .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if let GameProgress::NeedsDecisionCtx(c) = p {
            if matches!(c, DecisionContext::Priority(_)) {
                return Ok(());
            }
            p = apply_decision_context_with_dm(g, q, &mut state, &c, dm)
                .map_err(|e| e.to_string())?;
        } else {
            return Ok(());
        }
    }
    Err("immediate action bound".into())
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let primary = c["primary"].as_str().unwrap();
    let mode = c["mode"].as_str().unwrap();
    let count = c["resources"].as_u64().unwrap() as usize;
    let mut g = game();
    let mut q = TriggerQueue::new();
    for _ in 0..8 {
        g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Library);
    }
    if primary == "Idol of the Deep King" {
        dm.targets = vec![Target::Player(PlayerId(1))];
    }
    let source = paid_cast(
        &mut g,
        defs,
        n,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    if primary == "Master's Guide-Mural" {
        let token = g
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                g.object(*id).is_some_and(|o| {
                    o.name == "Golem" && matches!(o.kind, ironsmith::object::ObjectKind::Token)
                })
            })
            .ok_or("expected Mural ETB Golem absent")?;
        dm.targets = vec![Target::Object(token)];
        paid_cast(&mut g, defs, "Disenchant", 2, &mut q, dm)?;
        assert!(!g.battlefield.contains(&token));
    }
    let actor = if mode.starts_with("opposing") {
        PlayerId(1)
    } else {
        alice()
    };
    set_actor(&mut g, dm, actor);
    let material = if mode == "wrong_type" {
        if c["creature_craft"].as_bool().unwrap() {
            "Lotus Petal"
        } else {
            "Grizzly Bears"
        }
    } else {
        "Ornithopter"
    };
    let cost = if material == "Grizzly Bears" { 2 } else { 0 };
    let gy = mode.contains("graveyard");
    let mut materials = vec![];
    for _ in 0..count {
        dm.targets.clear();
        dm.chosen.clear();
        let m = if mode == "hand" {
            let id = g.create_object_from_definition(&defs[material].0, actor, Zone::Hand);
            g.object(id).unwrap().stable_id
        } else {
            paid_cast(&mut g, defs, material, cost, &mut q, dm)?
        };
        if gy {
            dm.targets = vec![Target::Object(current(&g, m))];
            paid_cast(&mut g, defs, "Disenchant", 2, &mut q, dm)?;
            assert_eq!(g.object(current(&g, m)).unwrap().zone, Zone::Graveyard);
        }
        materials.push(m);
    }
    set_actor(&mut g, dm, alice());
    if mode == "other_turn" {
        g.turn.active_player = PlayerId(1);
    }
    dm.targets.clear();
    dm.chosen = materials
        .first()
        .map(|s| vec![current(&g, *s)])
        .unwrap_or_default();
    let sid = current(&g, source);
    let index = c["ability_index"].as_u64().unwrap() as usize;
    let actions = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let action=actions.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index,..}if *source==sid&&*ability_index==index)).cloned();
    let ability = match &defs[n].0.abilities[index].kind {
        ironsmith::ability::AbilityKind::Activated(a) => a,
        _ => return Err("wrong candidate kind".into()),
    };
    dm.trace.push(json!({"stage":"candidate_exact_craft_path","ability_index":index,"cost_path":c["cost_path"],"mode":mode,"source_other_face":format!("{:?}",g.object(sid).unwrap().other_face),"legal_actions":format!("{actions:?}"),"material_zones":materials.iter().map(|s|format!("{:?}",g.object(current(&g,*s)).unwrap().zone)).collect::<Vec<_>>(),"cost_payability":format!("{:?}",ironsmith::cost::can_pay_cost_with_reason(&g,sid,alice(),&ability.mana_cost,ironsmith::costs::PaymentReason::ActivateAbility))}));
    let valid = c["valid"].as_bool().unwrap();
    if action.is_none() || !valid {
        return Ok((
            json!({"error":null,"legal_activation_available":valid}),
            json!({"error":null,"legal_activation_available":action.is_some()}),
        ));
    }
    dm.stage = "actual_craft_activation".into();
    let before = g.player(alice()).unwrap().mana_pool.total();
    let error = announce(&mut g, action.unwrap(), &mut q, dm).err();
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    let material_zones = materials
        .iter()
        .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
        .collect::<Vec<_>>();
    let source_zone = format!("{:?}", g.object(current(&g, source)).unwrap().zone);
    let expected = json!({"error":null,"legal_activation_available":true,"mana_paid":c["activation_mana"],"material_cost_zones":(0..count).map(|i|if i==0{"Exile"}else if gy{"Graveyard"}else{"Battlefield"}).collect::<Vec<_>>(),"source_cost_zone":"Exile","stacked":true});
    let actual = json!({"error":error,"legal_activation_available":true,"mana_paid":paid,"material_cost_zones":material_zones,"source_cost_zone":source_zone,"stacked":!g.stack.is_empty()});
    if error.is_none() {
        dm.stage = "craft_resolution_with_canonical_metadata".into();
        let resolution_error = finish(&mut g, &mut q, dm).err();
        dm.trace.push(json!({"stage":"craft_resolution_outcome_metadata_limited","error":resolution_error,"source_after":g.find_object_by_stable_id(source).and_then(|id|g.object(id)).map(|o|json!({"name":o.name.to_string(),"zone":format!("{:?}",o.zone),"other_face":format!("{:?}",o.other_face)})),"full_transform_semantics_tested":false}));
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
    let input = p.join("single-exile-craft-inputs.json");
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
        eprintln!("SINGLE_EXILE_CRAFT {c}");
        let mut dm = Dm {
            actor: alice(),
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
    let report = json!({"scope":"Twenty-four independently compiled Craft payload paths (twelve primary fronts plus twelve combined-name aliases). Exact single material exile costs, battlefield/graveyard resource alternatives, zero/exact/extra and wrong type/owner/zone/turn controls. Every source and permanent material paid normally; graveyard artifacts destroyed by actual paid Disenchant. Only advertised actions used.","limitations":"Mana, initial library Forests, and player main-phase placement are fixture setup. Master Guide-Mural token is actually destroyed before resource boundary checks so it cannot silently supply a cost. ETB abilities resolve normally, with no-target triggers naturally unable to stack. Resolution is attempted for paid Craft activations, but canonical front payloads lacking back-face metadata cannot establish correct transformation. No fabricated reverse face and no whole-card certification.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_single_exile_craft_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("single-exile-craft-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("single-exile-craft-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical single-exile-craft expected-result reporter"]
fn report_single_exile_craft() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
