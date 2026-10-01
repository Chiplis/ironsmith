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
fn next_main(g: &mut GameState) {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    while g.turn.active_player != alice() {
        g.next_turn();
        ironsmith::turn::execute_untap_step(g);
    }
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(alice());
}
fn paid_land(
    g: &mut GameState,
    d: &CardDefinition,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let id = g.create_object_from_definition(d, alice(), Zone::Hand);
    let s = g.object(id).unwrap().stable_id;
    let a = compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==id))
        .ok_or("source land play absent")?;
    dm.stage = "actual_source_land_play".into();
    immediate(g, a, q, dm)?;
    finish(g, q, dm)?;
    Ok(s)
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
    let a=compute_legal_actions(g,dm.actor).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index:0,..}if *source==current(g,altar))).ok_or("Altar producer absent")?;
    announce(g, a, q, dm)?;
    finish(g, q, dm)?;
    dm.targets.clear();
    dm.chosen.clear();
    Ok(())
}
fn named_counter(g: &GameState, id: ObjectId, n: &str) -> u32 {
    g.object(id)
        .unwrap()
        .counters
        .iter()
        .filter(|(k, _)| k.description().eq_ignore_ascii_case(n))
        .map(|(_, v)| *v)
        .sum()
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let primary = n.split(" // ").next().unwrap();
    let mode = c["mode"].as_str().unwrap();
    let mut g = game();
    let mut q = TriggerQueue::new();
    for owner in [alice(), PlayerId(1)] {
        for _ in 0..8 {
            g.create_object_from_definition(&defs["Forest"].0, owner, Zone::Library);
        }
    }
    let source = if primary == "City of Shadows" {
        paid_land(&mut g, &defs[n].0, &mut q, dm)?
    } else {
        paid_cast(
            &mut g,
            defs,
            n,
            c["cost"].as_u64().unwrap() as u32,
            &mut q,
            dm,
        )?
    };
    let mut target = None;
    if ["Altar of Bhaal", "The Soul Stone"].contains(&primary) {
        let altar = paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?;
        let bear = paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut q, dm)?;
        sacrifice(&mut g, altar, bear, &mut q, dm)?;
        target = Some(bear);
    }
    let actor = if mode == "opposing_owner" {
        PlayerId(1)
    } else {
        alice()
    };
    set_actor(&mut g, dm, actor);
    let mut resources = vec![];
    if primary == "The Book of Vile Darkness" {
        if mode == "extra" {
            paid_cast(&mut g, defs, "Mirror Box", 3, &mut q, dm)?;
        }
        for name in ["Eye of Vecna", "Hand of Vecna"] {
            let missing = (mode == "missing_eye" && name == "Eye of Vecna")
                || (mode == "missing_hand" && name == "Hand of Vecna")
                || mode == "empty";
            if !missing {
                for _ in 0..if mode == "extra" { 2 } else { 1 } {
                    resources.push(paid_cast(
                        &mut g,
                        defs,
                        name,
                        if name == "Eye of Vecna" { 2 } else { 3 },
                        &mut q,
                        dm,
                    )?);
                }
            }
        }
    } else {
        let count = c["resources"].as_u64().unwrap() as usize;
        for _ in 0..count {
            dm.targets.clear();
            dm.chosen.clear();
            let mat = if mode == "wrong_type" {
                "Lotus Petal"
            } else if primary == "Food Chain" {
                "Grizzly Bears"
            } else {
                "Ornithopter"
            };
            let m = if primary == "Primordial Mist" && mode != "face_up" && mode != "wrong_zone" {
                let id =
                    g.create_object_from_definition(&defs["Ornithopter"].0, actor, Zone::Library);
                let stable = g.object(id).unwrap().stable_id;
                paid_cast(&mut g, defs, "Soul Summons", 2, &mut q, dm)?;
                stable
            } else if mode == "wrong_zone" && primary == "Primordial Mist" {
                let id = g.create_object_from_definition(&defs["Ornithopter"].0, actor, Zone::Hand);
                g.object(id).unwrap().stable_id
            } else {
                let m = paid_cast(
                    &mut g,
                    defs,
                    mat,
                    if mat == "Grizzly Bears" { 2 } else { 0 },
                    &mut q,
                    dm,
                )?;
                if mode == "wrong_zone" {
                    let altar = paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?;
                    sacrifice(&mut g, altar, m, &mut q, dm)?;
                }
                m
            };
            resources.push(m);
        }
    }
    set_actor(&mut g, dm, alice());
    if mode == "other_turn" {
        g.turn.active_player = PlayerId(1);
    }
    let sid = current(&g, source);
    g.remove_summoning_sickness(sid);
    dm.targets = if primary == "Altar of Bhaal" {
        vec![Target::Object(current(&g, target.unwrap()))]
    } else {
        vec![]
    };
    dm.chosen = resources.iter().map(|s| current(&g, *s)).collect();
    dm.chosen.push(sid);
    let index = c["ability_index"].as_u64().unwrap() as usize;
    if primary == "Food Chain" {
        g.player_mut(alice()).unwrap().mana_pool = Default::default();
    }
    let actions = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let action=actions.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index,..}|LegalAction::ActivateManaAbility{source,ability_index,..}if *source==sid&&*ability_index==index)).cloned();
    dm.trace.push(json!({"stage":"candidate_exact_path","ability_index":index,"cost_path":c["cost_path"],"other_cost_path":c["other_cost_path"],"legal_actions":format!("{actions:?}"),"material_ids":resources.iter().map(|s|current(&g,*s).0).collect::<Vec<_>>() }));
    let valid = c["valid"].as_bool().unwrap();
    if action.is_none() || !valid {
        return Ok((
            json!({"error":null,"legal_activation_available":valid}),
            json!({"error":null,"legal_activation_available":action.is_some()}),
        ));
    }
    let before = g.player(alice()).unwrap().mana_pool.total() as i64;
    let a = action.unwrap();
    dm.stage = "actual_candidate_activation".into();
    let mut error = if matches!(a, LegalAction::ActivateManaAbility { .. }) {
        immediate(&mut g, a, &mut q, dm)
    } else {
        announce(&mut g, a, &mut q, dm)
    }
    .err();
    let spent = before - g.player(alice()).unwrap().mana_pool.total() as i64;
    let zones = resources
        .iter()
        .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
        .collect::<Vec<_>>();
    let source_zone = format!("{:?}", g.object(current(&g, source)).unwrap().zone);
    dm.chosen.clear();
    if error.is_none() {
        dm.stage = "candidate_resolution".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let expected_zones = resources
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if primary == "The Book of Vile Darkness" {
                if mode != "extra" || i == 0 || i == 2 {
                    "Exile"
                } else {
                    "Battlefield"
                }
            } else if i == 0 {
                "Exile"
            } else {
                "Battlefield"
            }
        })
        .collect::<Vec<_>>();
    let mut expected = json!({"error":null,"legal_activation_available":true,"mana_net_spent":c["activation_net"],"cost_zones":expected_zones,"source_cost_zone":if primary=="The Book of Vile Darkness"{"Exile"}else{"Battlefield"}});
    let mut actual = json!({"error":error,"legal_activation_available":true,"mana_net_spent":spent,"cost_zones":zones,"source_cost_zone":source_zone});
    match primary {
        "Altar of Bhaal" => {
            expected["returned_target_zone"] = json!("Battlefield");
            actual["returned_target_zone"] = json!(format!(
                "{:?}",
                g.object(current(&g, target.unwrap())).unwrap().zone
            ));
            expected["returned_tapped"] = json!(false);
            actual["returned_tapped"] = json!(g.is_tapped(current(&g, target.unwrap())));
        }
        "City of Shadows" => {
            expected["storage_counters"] = json!(1);
            actual["storage_counters"] = json!(named_counter(&g, sid, "storage"));
            next_main(&mut g);
            g.player_mut(alice()).unwrap().mana_pool = Default::default();
            let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateManaAbility{source,ability_index:1,..}|LegalAction::ActivateAbility{source,ability_index:1,..}if *source==sid)).ok_or("City mana action absent")?;
            let e = immediate(&mut g, a, &mut q, dm).err();
            expected["mana_error"] = Value::Null;
            actual["mana_error"] = json!(e);
            expected["colorless_generated"] = json!(1);
            actual["colorless_generated"] = json!(g.player(alice()).unwrap().mana_pool.colorless);
        }
        "Food Chain" => {
            expected["generated_green"] = json!(3);
            actual["generated_green"] = json!(g.player(alice()).unwrap().mana_pool.green);
            dm.trace.push(json!({"stage":"food_chain_mana_observation","pool":format!("{:?}",g.player(alice()).unwrap().mana_pool),"restricted_units":format!("{:?}",g.player(alice()).unwrap().restricted_mana)}));
            // A failed mana producer cannot establish the downstream spending restriction.
            // Extra creatures also alter the legal-action predictor's potential mana.
        }
        "Primordial Mist" => {
            let m = resources[0];
            let id = current(&g, m);
            expected["exiled_face_down"] = json!(false);
            actual["exiled_face_down"] = json!(g.is_face_down(id));
            let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,from_zone:Zone::Exile,..}if *spell_id==id));
            expected["exiled_cast_available"] = json!(true);
            actual["exiled_cast_available"] = json!(a.is_some());
            if let Some(a) = a {
                dm.stage = "actual_manifested_card_cast_from_exile".into();
                let e = announce(&mut g, a, &mut q, dm)
                    .and_then(|_| finish(&mut g, &mut q, dm))
                    .err();
                expected["exiled_cast_error"] = Value::Null;
                actual["exiled_cast_error"] = json!(e);
                expected["final_material_zone"] = json!("Battlefield");
                actual["final_material_zone"] =
                    json!(format!("{:?}", g.object(current(&g, m)).unwrap().zone));
            }
        }
        "The Soul Stone" => {
            expected["harnessed"] = json!(true);
            actual["harnessed"] = json!(g.is_harnessed(sid));
            g.next_turn();
            ironsmith::turn::execute_untap_step(&mut g);
            while g.turn.active_player != alice() {
                g.next_turn();
                ironsmith::turn::execute_untap_step(&mut g);
            }
            ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
            let event = ironsmith::triggers::generate_step_trigger_events(&g)
                .ok_or("upkeep event absent")?;
            for t in ironsmith::triggers::check_triggers(&g, &event) {
                q.add(t);
            }
            for t in ironsmith::triggers::check_delayed_triggers(&mut g, &event) {
                q.add(t);
            }
            dm.targets = vec![Target::Object(current(&g, target.unwrap()))];
            dm.stage = "actual_harnessed_upkeep".into();
            let e = finish(&mut g, &mut q, dm).err();
            expected["upkeep_error"] = Value::Null;
            actual["upkeep_error"] = json!(e);
            expected["returned_target_zone"] = json!("Battlefield");
            actual["returned_target_zone"] = json!(format!(
                "{:?}",
                g.object(current(&g, target.unwrap())).unwrap().zone
            ));
        }
        "The Book of Vile Darkness" => {
            let token = g
                .battlefield
                .iter()
                .copied()
                .find(|id| g.object(*id).is_some_and(|o| o.name == "Vecna"));
            let p = if mode == "extra" { 9 } else { 8 };
            expected["vecna"] = json!({"pt":[p,p],"indestructible":true,"triggered_abilities":4});
            actual["vecna"]=token.map(|id|json!({"pt":[g.calculated_power(id),g.calculated_toughness(id)],"indestructible":g.current_has_static_ability_id(id,ironsmith::static_abilities::StaticAbilityId::Indestructible),"triggered_abilities":g.current_abilities(id).unwrap().iter().filter(|a|matches!(a.kind,ironsmith::ability::AbilityKind::Triggered(_))).count()})).unwrap_or(Value::Null);
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
    let input = p.join("single-exile-battlefield-inputs.json");
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
        eprintln!("SINGLE_EXILE_BATTLEFIELD {c}");
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
        "crates/ironsmith-engine/src/game_loop/priority_apply.rs",
        "crates/ironsmith-engine/src/effects/mana/add_mana_of_any_one_color.rs",
        "crates/ironsmith-engine/src/effects/composition/choose_objects.rs",
    ];
    let report = json!({"scope":"Eight exact battlefield single-exile paths across seven payload names (one alias). Actual paid sources, land play, creature/resource casts, and Soul Summons manifest producer. Book has two named-material paths proven by the same complete action with independently missing Eye/Hand controls. Exact outcomes and bounded follow-up mana, permission or harness-upkeep producers.","limitations":"Mana and initial library are setup; source sickness is cleared and turn/main positions are explicit. Food Chain clears old mana before activation so only its generated mana can pay follow-up spells. Actual Mirror Box allows two Eye/Hand copies in Book surplus control; its legendary creature anthem is included in expected9/9 Vecna. Primordial resources are actual paid Soul Summons manifests; no fabricated face-down objects or back faces.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_single_exile_battlefield_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("single-exile-battlefield-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("single-exile-battlefield-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical single-exile-battlefield expected-result reporter"]
fn report_single_exile_battlefield() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
