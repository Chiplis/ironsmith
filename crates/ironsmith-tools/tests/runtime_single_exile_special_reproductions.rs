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
        let selected = self
            .chosen
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
            .take(c.max.unwrap_or(usize::MAX))
            .collect::<Vec<_>>();
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
fn sacrifice(
    g: &mut GameState,
    altar: ironsmith::ids::StableId,
    m: ironsmith::ids::StableId,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    dm.chosen = vec![current(g, m)];
    dm.targets = vec![Target::Player(PlayerId(2))];
    dm.stage = "actual_resource_sacrifice".into();
    let a=compute_legal_actions(g,dm.actor).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index:0,..}if *source==current(g,altar))).ok_or("Altar resource producer absent")?;
    announce(g, a, q, dm)?;
    finish(g, q, dm)?;
    dm.targets.clear();
    dm.chosen.clear();
    Ok(())
}
fn petal(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let m = paid_cast(g, defs, "Lotus Petal", 0, q, dm)?;
    let id = current(g, m);
    let a=compute_legal_actions(g,dm.actor).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateManaAbility{source,ability_index:0,..}|LegalAction::ActivateAbility{source,ability_index:0,..}if *source==id)).ok_or("Petal resource producer absent")?;
    immediate(g, a, q, dm)?;
    finish(g, q, dm)?;
    Ok(m)
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let mode = c["mode"].as_str().unwrap();
    let outcome = c["outcome"].as_str().unwrap();
    let count = c["resources"].as_u64().unwrap() as usize;
    let mut g = game();
    let mut q = TriggerQueue::new();
    if ["forests", "draw"].contains(&outcome) {
        for _ in 0..4 {
            g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Library);
        }
    }
    let target = if ["aura_pump", "prevent", "destroy"].contains(&outcome) {
        Some(paid_cast(&mut g, defs, "Hill Giant", 4, &mut q, dm)?)
    } else {
        None
    };
    if outcome == "aura_pump" {
        dm.targets = vec![Target::Object(current(&g, target.unwrap()))];
    }
    let source = paid_cast(
        &mut g,
        defs,
        n,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    dm.targets.clear();
    let altar = paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?;
    if c["source_graveyard"].as_bool().unwrap_or(false) {
        sacrifice(&mut g, altar, source, &mut q, dm)?;
    }
    let actor = if mode == "opposing_owner" {
        PlayerId(1)
    } else {
        alice()
    };
    set_actor(&mut g, dm, actor);
    let producer = if actor != alice() {
        paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?
    } else {
        altar
    };
    let mat = c["material"].as_str().unwrap();
    let mut resources = vec![];
    for _ in 0..count {
        dm.targets.clear();
        dm.chosen.clear();
        let m = if mat == "Shock" && mode == "wrong_zone" {
            let id = g.create_object_from_definition(&defs[mat].0, actor, Zone::Hand);
            g.object(id).unwrap().stable_id
        } else if mat == "Lotus Petal" && mode != "wrong_zone" {
            petal(&mut g, defs, &mut q, dm)?
        } else {
            if mat == "Shock" {
                dm.targets = vec![Target::Player(PlayerId(2))];
            }
            let m = paid_cast(
                &mut g,
                defs,
                mat,
                c["material_cost"].as_u64().unwrap() as u32,
                &mut q,
                dm,
            )?;
            if mode != "wrong_zone" && mat != "Shock" {
                sacrifice(&mut g, producer, m, &mut q, dm)?;
            }
            m
        };
        resources.push(m);
    }
    if mode == "noncreature_on_top" {
        let m = petal(&mut g, defs, &mut q, dm)?;
        resources.push(m);
    }
    set_actor(&mut g, dm, alice());
    dm.targets.clear();
    dm.chosen.clear();
    let mut fungi = vec![];
    if outcome == "fungus_counters" {
        for owner in [alice(), PlayerId(1)] {
            set_actor(&mut g, dm, owner);
            fungi.push(paid_cast(&mut g, defs, "Utopia Mycon", 1, &mut q, dm)?);
        }
        set_actor(&mut g, dm, alice());
    }
    let source_id = current(&g, source);
    g.remove_summoning_sickness(source_id);
    let chosen_index = if c["top"].as_bool().unwrap() {
        if mode == "noncreature_on_top" && !c["any_card"].as_bool().unwrap_or(false) {
            count - 1
        } else {
            resources.len().saturating_sub(1)
        }
    } else {
        0
    };
    dm.chosen = resources
        .get(chosen_index)
        .map(|s| vec![current(&g, *s)])
        .unwrap_or_default();
    if ["prevent", "destroy"].contains(&outcome) {
        dm.targets = vec![Target::Object(current(&g, target.unwrap()))];
    }
    let index = c["ability_index"].as_u64().unwrap() as usize;
    let actions = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let action=actions.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index,..}if *source==source_id&&*ability_index==index)).cloned();
    dm.trace.push(json!({"stage":"candidate_exact_path","cost_path":c["cost_path"],"ability_index":index,"legal_actions":format!("{actions:?}"),"source_zone":format!("{:?}",g.object(source_id).unwrap().zone),"graveyard_order":g.player(alice()).unwrap().graveyard.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string()})).collect::<Vec<_>>(),"chosen_resource":dm.chosen.iter().map(|id|id.0).collect::<Vec<_>>()}));
    let valid = c["valid"].as_bool().unwrap();
    if action.is_none() || !valid {
        return Ok((
            json!({"error":null,"legal_activation_available":valid}),
            json!({"error":null,"legal_activation_available":action.is_some()}),
        ));
    }
    let before = g.player(alice()).unwrap().mana_pool.total();
    dm.stage = "actual_candidate_activation".into();
    let mut error = announce(&mut g, action.unwrap(), &mut q, dm).err();
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    let zones = resources
        .iter()
        .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
        .collect::<Vec<_>>();
    if outcome == "forests" {
        dm.chosen = g
            .player(alice())
            .unwrap()
            .library
            .iter()
            .copied()
            .take(2)
            .collect();
    } else {
        dm.chosen.clear();
    }
    if error.is_none() {
        dm.stage = "candidate_resolution".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let mut expected = json!({"error":null,"legal_activation_available":true,"mana_paid":c["activation_mana"],"cost_zones":(0..resources.len()).map(|i|if i==chosen_index{"Exile"}else{"Graveyard"}).collect::<Vec<_>>()});
    let mut actual = json!({"error":error,"legal_activation_available":true,"mana_paid":paid,"cost_zones":zones});
    match outcome {
        "pump" => {
            expected["source_pt"] = c["pt"].clone();
            actual["source_pt"] = json!([
                g.calculated_power(current(&g, source)),
                g.calculated_toughness(current(&g, source))
            ]);
        }
        "aura_pump" => {
            let id = current(&g, target.unwrap());
            expected["enchanted_pt"] = json!([4, 4]);
            actual["enchanted_pt"] = json!([g.calculated_power(id), g.calculated_toughness(id)]);
        }
        "prevent" => {
            dm.targets = vec![Target::Object(current(&g, target.unwrap()))];
            let e = paid_cast(&mut g, defs, "Shock", 1, &mut q, dm).err();
            expected["damage_spell_error"] = Value::Null;
            actual["damage_spell_error"] = json!(e);
            expected["damage_marked"] = json!(1);
            actual["damage_marked"] = json!(g.damage_on(current(&g, target.unwrap())));
        }
        "regenerate" => {
            dm.targets = vec![Target::Object(current(&g, source))];
            let e = paid_cast(&mut g, defs, "Murder", 3, &mut q, dm).err();
            expected["destruction_spell_error"] = Value::Null;
            actual["destruction_spell_error"] = json!(e);
            expected["source_zone"] = json!("Battlefield");
            actual["source_zone"] =
                json!(format!("{:?}", g.object(current(&g, source)).unwrap().zone));
            expected["source_tapped"] = json!(true);
            actual["source_tapped"] = json!(g.is_tapped(current(&g, source)));
        }
        "toughness_counters" => {
            expected["toughness_counters"] = c["counter_amount"].clone();
            actual["toughness_counters"] =
                json!(g.counter_count(current(&g, source), CounterType::PlusZeroPlusOne));
        }
        "forests" => {
            expected["forests"] = json!([true, true]);
            actual["forests"] = json!(
                g.battlefield
                    .iter()
                    .filter(|id| g.object(**id).is_some_and(|o| o.name == "Forest"))
                    .map(|id| g.is_tapped(*id))
                    .collect::<Vec<_>>()
            );
        }
        "destroy" => {
            expected["target_zone"] = json!("Graveyard");
            actual["target_zone"] = json!(format!(
                "{:?}",
                g.object(current(&g, target.unwrap())).unwrap().zone
            ));
        }
        "fungus_counters" => {
            expected["fungus_spore_counters"] = json!([1, 1]);
            actual["fungus_spore_counters"] = json!(
                fungi
                    .iter()
                    .map(|s| g
                        .object(current(&g, *s))
                        .unwrap()
                        .counters
                        .iter()
                        .filter(|(kind, _)| kind.description().eq_ignore_ascii_case("spore"))
                        .map(|(_, count)| *count)
                        .sum::<u32>())
                    .collect::<Vec<_>>()
            );
        }
        "return" => {
            let id = current(&g, source);
            expected["source_zone"] = json!("Battlefield");
            actual["source_zone"] = json!(format!("{:?}", g.object(id).unwrap().zone));
            expected["source_tapped"] = c["return_tapped"].clone();
            actual["source_tapped"] = json!(g.is_tapped(id));
            expected["source_plus_counters"] = c["return_counters"].clone();
            actual["source_plus_counters"] =
                json!(g.counter_count(id, CounterType::PlusOnePlusOne));
        }
        "draw" => {
            expected["hand"] = json!(1);
            actual["hand"] = json!(g.player(alice()).unwrap().hand.len());
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
    let input = p.join("single-exile-special-inputs.json");
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
        eprintln!("SINGLE_EXILE_SPECIAL {c}");
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
    let report = json!({"scope":"Fourteen single-exile paths: top-card/top-creature ordering, subtype/owner filters, source-in-graveyard return, mana-value counters, regeneration and prevention consumers. Zero/exact/extra and relevant wrong-resource controls. Actual paid sources/resources/producers and follow-up destruction/damage spells.","limitations":"Mana, initial hand/library and active-player main phase are fixture setup. Source sickness cleared for elapsed-turn availability. All creatures are paid and sacrificed through paid Altar; Lotus Petal sacrifices itself, Shock resolves. Top controls add a noncreature above two creature cards. Regeneration tested by paid Murder, prevention by paid Shock. Thelon tests actual paid Fungus witnesses for both players. Duration expiry and irrelevant printed abilities are outside scope.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_single_exile_special_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("single-exile-special-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("single-exile-special-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical single-exile-special expected-result reporter"]
fn report_single_exile_special() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
