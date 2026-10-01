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

fn immediate(
    g: &mut GameState,
    a: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
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
    Err("immediate action budget".into())
}
fn actual_land(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    n: &str,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let id = g.create_object_from_definition(&defs[n].0, alice(), Zone::Hand);
    let stable = g.object(id).unwrap().stable_id;
    let a = compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==id))
        .ok_or("land play missing")?;
    dm.stage = "actual_land_play".into();
    immediate(g, a, q, dm)?;
    finish(g, q, dm)?;
    assert!(g.is_tapped(current(g, stable)));
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    while g.turn.active_player != alice() {
        g.next_turn();
        ironsmith::turn::execute_untap_step(g);
    }
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(alice());
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
    Ok(stable)
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
    let req = c["required"].as_u64().unwrap() as usize;
    let index = c["ability_index"].as_u64().unwrap() as usize;
    let mechtitan = n.starts_with("Mechtitan Core");
    let land = n == "Mines of Moria" || n == "Sunken Palace";
    let mut source = if n == "Sunken Palace" {
        None
    } else if land {
        Some(actual_land(&mut g, defs, n, &mut q, dm)?)
    } else {
        Some(paid_cast(
            &mut g,
            defs,
            n,
            c["cost"].as_u64().unwrap() as u32,
            &mut q,
            dm,
        )?)
    };
    let altar = if !mechtitan {
        Some(paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?)
    } else {
        None
    };
    let mut materials = vec![];
    for _ in 0..total {
        dm.targets.clear();
        dm.chosen.clear();
        let material = c["material"].as_str().unwrap_or("Ornithopter");
        let m = paid_cast(&mut g, defs, material, 0, &mut q, dm)?;
        if material == "Lotus Petal" {
            let mid = current(&g, m);
            let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index:0,..}|LegalAction::ActivateManaAbility{source,ability_index:0,..}if *source==mid)).ok_or("Lotus Petal resource producer unavailable")?;
            dm.stage = "actual_lotus_petal_mana_sacrifice".into();
            immediate(&mut g, a, &mut q, dm)?;
            finish(&mut g, &mut q, dm)?;
            assert_eq!(g.object(current(&g, m)).unwrap().zone, Zone::Graveyard);
        } else if let Some(altar) = altar {
            dm.chosen = vec![current(&g, m)];
            dm.targets = vec![Target::Player(PlayerId(1))];
            dm.stage = "actual_graveyard_resource_sacrifice".into();
            let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index:0,..}if *source==current(&g,altar))).ok_or("Altar resource action absent")?;
            announce(&mut g, a, &mut q, dm)?;
            finish(&mut g, &mut q, dm)?;
            assert_eq!(g.object(current(&g, m)).unwrap().zone, Zone::Graveyard);
        }
        materials.push(m);
    }
    if source.is_none() {
        source = Some(actual_land(&mut g, defs, n, &mut q, dm)?);
    }
    let source = source.unwrap();
    if let Some(mode) = c["spend_mode"].as_str() {
        g.player_mut(alice()).unwrap().mana_pool = Default::default();
        if mode == "basic_mana" {
            let sid = current(&g, source);
            let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateManaAbility{source,ability_index:1,..}if *source==sid)).ok_or("basic mana control absent")?;
            dm.stage = "actual_palace_basic_mana_control".into();
            immediate(&mut g, a, &mut q, dm)?;
        } else {
            g.player_mut(alice())
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 1);
        }
        for _ in 0..4 {
            g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Library);
        }
        dm.targets.clear();
        dm.chosen.clear();
        let before = g.player(alice()).unwrap().hand.len();
        let error = paid_cast(&mut g, defs, "Opt", 1, &mut q, dm).err();
        return Ok((
            json!({"error":null,"drawn_cards":1,"palace_exile_ability_activated":false}),
            json!({"error":error,"drawn_cards":g.player(alice()).unwrap().hand.len()-before,"palace_exile_ability_activated":false}),
        ));
    }
    dm.targets.clear();
    dm.chosen.clear();
    let target = if n == "Zombie Assassin" {
        Some(paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut q, dm)?)
    } else {
        None
    };
    let source_id = current(&g, source);
    g.remove_summoning_sickness(source_id);
    dm.chosen = materials
        .iter()
        .take(req)
        .map(|s| current(&g, *s))
        .collect();
    dm.chosen.push(source_id);
    if let Some(t) = target {
        dm.targets = vec![Target::Object(current(&g, t))];
    }
    if n == "Sunken Palace" {
        g.player_mut(alice()).unwrap().mana_pool = Default::default();
        g.player_mut(alice())
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        g.player_mut(alice())
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
    }
    let legal = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let action=legal.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index,..}|LegalAction::ActivateManaAbility{source,ability_index,..}if *source==source_id&&*ability_index==index)).cloned();
    dm.trace.push(json!({"stage":"candidate_cost_path","cost_path":c["cost_path"],"ability_index":index,"legal_actions":format!("{legal:?}"),"source_tapped":g.is_tapped(source_id),"source_zone":format!("{:?}",g.object(source_id).unwrap().zone),"resource_zones":materials.iter().map(|s|format!("{:?}",g.object(current(&g,*s)).unwrap().zone)).collect::<Vec<_>>()}));
    let valid = c["valid"].as_bool().unwrap();
    if action.is_none() || !valid {
        return Ok((
            json!({"error":null,"legal_activation_available":valid}),
            json!({"error":null,"legal_activation_available":action.is_some()}),
        ));
    }
    let a = action.unwrap();
    let mana = matches!(a, LegalAction::ActivateManaAbility { .. });
    dm.stage = "actual_candidate_activation".into();
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut error = if mana {
        immediate(&mut g, a, &mut q, dm)
    } else {
        announce(&mut g, a, &mut q, dm)
    }
    .err();
    let delta = before as i64 - g.player(alice()).unwrap().mana_pool.total() as i64;
    let cost_zones = materials
        .iter()
        .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
        .collect::<Vec<_>>();
    let source_cost_zone = format!("{:?}", g.object(current(&g, source)).unwrap().zone);
    let cost_tapped = g.is_tapped(source_id);
    dm.chosen.clear();
    if error.is_none() {
        dm.stage = "candidate_resolution".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let mut expected = json!({"error":null,"legal_activation_available":true,"mana_net_spent":c["activation_net"],"cost_zones":(0..total).map(|i|if i<req{"Exile"}else if mechtitan{"Battlefield"}else{"Graveyard"}).collect::<Vec<_>>(),"source_cost_zone":if mechtitan||n=="Zombie Assassin"{"Exile"}else if n=="Coin of Fate"{"Graveyard"}else{"Battlefield"}});
    let mut actual = json!({"error":error,"legal_activation_available":true,"mana_net_spent":delta,"cost_zones":cost_zones,"source_cost_zone":source_cost_zone});
    if land {
        expected["source_tapped"] = json!(true);
        actual["source_tapped"] = json!(cost_tapped);
    }
    if mechtitan {
        let token = g
            .battlefield
            .iter()
            .copied()
            .find(|id| g.object(*id).is_some_and(|o| o.name == "Mechtitan"));
        expected["token"] =
            json!({"pt":[10,10],"color_count":5,"keywords":[true,true,true,true,true]});
        actual["token"]=token.map(|id|json!({"pt":[g.calculated_power(id),g.calculated_toughness(id)],"color_count":g.object(id).unwrap().colors().count(),"keywords":([ironsmith::static_abilities::StaticAbilityId::Flying,ironsmith::static_abilities::StaticAbilityId::Vigilance,ironsmith::static_abilities::StaticAbilityId::Trample,ironsmith::static_abilities::StaticAbilityId::Lifelink,ironsmith::static_abilities::StaticAbilityId::Haste].iter().map(|k|g.current_has_static_ability_id(id,*k)).collect::<Vec<_>>())})).unwrap_or(Value::Null);
        if let Some(token) = token {
            dm.targets = vec![Target::Object(token)];
            dm.stage = "actual_paid_token_disenchant".into();
            let e = paid_cast(&mut g, defs, "Disenchant", 2, &mut q, dm).err();
            expected["token_removal_error"] = Value::Null;
            actual["token_removal_error"] = json!(e);
            expected["materials_after_token_leaves"] = json!(
                (0..total)
                    .map(|i| json!({"zone":"Battlefield","tapped":i<req}))
                    .collect::<Vec<_>>()
            );
            actual["materials_after_token_leaves"]=json!(materials.iter().map(|s|{let id=current(&g,*s);json!({"zone":format!("{:?}",g.object(id).unwrap().zone),"tapped":g.is_tapped(id)})}).collect::<Vec<_>>());
            expected["core_final_zone"] = json!("Exile");
            actual["core_final_zone"] =
                json!(format!("{:?}", g.object(current(&g, source)).unwrap().zone));
        }
    } else if n == "Mines of Moria" {
        expected["treasures"] = json!(2);
        actual["treasures"] = json!(
            g.battlefield
                .iter()
                .filter(|id| g.object(**id).is_some_and(|o| o.name == "Treasure"))
                .count()
        );
    } else if n == "Sunken Palace" {
        expected["generated_blue"] = json!(1);
        actual["generated_blue"] = json!(g.player(alice()).unwrap().mana_pool.blue);
        let hand_before = g.player(alice()).unwrap().hand.len();
        for _ in 0..4 {
            g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Library);
        }
        dm.targets.clear();
        dm.chosen.clear();
        dm.stage = "actual_opt_spending_generated_mana".into();
        let e = paid_cast(&mut g, defs, "Opt", 1, &mut q, dm).err();
        expected["opt_error"] = Value::Null;
        actual["opt_error"] = json!(e);
        expected["drawn_cards"] = json!(2);
        actual["drawn_cards"] = json!(g.player(alice()).unwrap().hand.len() - hand_before);
    } else if n == "Zombie Assassin" {
        expected["target_final_zone"] = json!("Graveyard");
        actual["target_final_zone"] = json!(format!(
            "{:?}",
            g.object(current(&g, target.unwrap())).unwrap().zone
        ));
    } else if n == "Coin of Fate" {
        expected["material_final_zones"] = json!(
            (0..total)
                .map(|i| if i == 0 {
                    "Library"
                } else if i == 1 {
                    "Battlefield"
                } else {
                    "Graveyard"
                })
                .collect::<Vec<_>>()
        );
        actual["material_final_zones"] = json!(
            materials
                .iter()
                .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
                .collect::<Vec<_>>()
        );
        expected["returned_tapped"] = json!(true);
        actual["returned_tapped"] = json!(g.is_tapped(current(&g, materials[1])));
        expected["monarch"] = json!(0);
        actual["monarch"] = json!(g.monarch.map(|p| p.0));
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
    let input = p.join("fixed-exile-remaining-inputs.json");
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
        eprintln!("FIXED_EXILE_REMAINING {c}");
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
    let report = json!({"scope":"Six payload paths, including one combined-name alias: required-minus-one/exact/extra controls. Actual source cast/land play, actual paid Ornithopter resources, graveyard via actual paid Altar sacrifice. Exact costs/outcomes; Mechtitan token destroyed with actual paid Disenchant, Sunken mana actually spent on Opt.","limitations":"Mana and initial hand/library are fixture setup. Lands enter through legal PlayLand and next_turn/untap-step cycles. Zombie Assassin sickness cleared for elapsed-turn setup. Normal priority and SBA resolution. Mechtitan alias is an independent payload but no extra primary name. Coin opponent always chooses the first exiled resource. Sunken Palace tested with exactly U+colorless before activation, leaving only generated U to spend on Opt.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_fixed_exile_remaining_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("fixed-exile-remaining-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("fixed-exile-remaining-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical fixed-exile-remaining expected-result reporter"]
fn report_fixed_exile_remaining() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
