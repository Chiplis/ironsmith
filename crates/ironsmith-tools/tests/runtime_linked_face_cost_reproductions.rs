//! Canonical paid rest-tag consumers with independent exact zone/order expectations.

#[path = "support/canonical_linked_fixture.rs"]
mod canonical_linked_fixture;
use canonical_linked_fixture::LinkedFamily;
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
    option_text: String,
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
        let selected = if !self.option_text.is_empty()
            && !c.description.contains("Confirm mana")
            && c.options
                .iter()
                .any(|o| o.legal && o.description.to_lowercase().contains(&self.option_text))
        {
            vec![
                c.options
                    .iter()
                    .find(|o| o.legal && o.description.to_lowercase().contains(&self.option_text))
                    .unwrap()
                    .index,
            ]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace.push(json!({"stage":self.stage,"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        self.trace.push(json!({"stage":self.stage,"choice":"number","context":format!("{c:?}"),"selected":self.x}));
        self.x
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if self.stage == "cast_prepared_spell" && c.min > 0 {
            c.candidates
                .iter()
                .filter(|o| o.legal)
                .take(c.min)
                .map(|o| o.id)
                .collect::<Vec<_>>()
        } else {
            self.chosen
                .iter()
                .copied()
                .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
                .take(c.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>()
        };
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
fn prepared_cast(
    g: &mut GameState,
    sid: ObjectId,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<Value, String> {
    let hand = g.player(alice()).unwrap().hand.len();
    let mana = g.player(alice()).unwrap().mana_pool.total();
    let copies = g
        .objects_in_deterministic_order()
        .into_iter()
        .filter(|o| g.prepared_spell_source(o.id) == Some(sid))
        .map(|o| o.id)
        .collect::<Vec<_>>();
    let action=compute_legal_actions(g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,from_zone:Zone::Exile,..}if copies.contains(spell_id)));
    let Some(a) = action else {
        return Ok(json!({"available":false,"prepared":g.is_prepared(sid)}));
    };
    dm.stage = "cast_prepared_spell".into();
    dm.chosen.clear();
    dm.targets.clear();
    let e = announce(g, a, q, dm).and_then(|_| finish(g, q, dm)).err();
    Ok(
        json!({"available":true,"error":e,"paid":mana-g.player(alice()).unwrap().mana_pool.total(),"prepared":g.is_prepared(sid),"hand_delta":g.player(alice()).unwrap().hand.len()as i32-hand as i32,"pests":g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.name=="Pest")).count()}),
    )
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    family: &LinkedFamily,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let primary = n.split(" // ").next().unwrap();
    let mode = c["mode"].as_str().unwrap();
    let llu = primary == "Lluwen, Exchange Student";
    let trio = primary == "Harmonized Trio";
    let mut g = game();
    let mut q = TriggerQueue::new();
    family.register(&mut g);
    let mut linked_defs = defs.clone();
    for (n, d) in &family.definitions {
        linked_defs.insert(n.clone(), d.clone());
    }
    for _ in 0..8 {
        g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Library);
    }
    if trio {
        for _ in 0..2 {
            g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Hand);
        }
    }
    let source = paid_cast(
        &mut g,
        &linked_defs,
        c["subject"].as_str().unwrap(),
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    let mut sid = current(&g, source);
    if llu && mode != "already_prepared" {
        let obs = prepared_cast(&mut g, sid, &mut q, dm)?;
        dm.trace
            .push(json!({"stage":"actual_initial_prepared_cast","observed":obs}));
        if obs
            != json!({"available":true,"error":null,"paid":1,"prepared":false,"hand_delta":0,"pests":1})
            && mode != "reprepare_after_cast"
        {
            return Ok((
                json!({"initial_prepared_cast":{"available":true,"error":null,"paid":1,"prepared":false,"hand_delta":0,"pests":1}}),
                json!({"initial_prepared_cast":obs}),
            ));
        }
    }
    if !llu && !trio {
        let mat = paid_cast(&mut g, &linked_defs, "Ornithopter", 0, &mut q, dm)?;
        dm.chosen = vec![current(&g, mat)];
        let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index:2,..}if *source==sid)).ok_or("Sawblades battlefield craft unavailable")?;
        dm.stage = "actual_paid_craft".into();
        let before = g.player(alice()).unwrap().mana_pool.total();
        announce(&mut g, a, &mut q, dm)?;
        finish(&mut g, &mut q, dm)?;
        sid = current(&g, source);
        let observed = json!({"source_name":g.object(sid).unwrap().name.to_string(),"source_zone":format!("{:?}",g.object(sid).unwrap().zone),"paid":before-g.player(alice()).unwrap().mana_pool.total(),"material_zone":format!("{:?}",g.object(current(&g,mat)).unwrap().zone)});
        let expected = json!({"source_name":"Bladewheel Chariot","source_zone":"Battlefield","paid":4,"material_zone":"Exile"});
        dm.trace
            .push(json!({"stage":"actual_craft_result","expected":expected,"observed":observed}));
        if observed != expected {
            return Ok((json!({"craft":expected}), json!({"craft":observed})));
        }
        dm.chosen.clear();
    }
    let actor = if mode == "opposing_owner" {
        PlayerId(1)
    } else {
        alice()
    };
    set_actor(&mut g, dm, actor);
    let count = c["resources"].as_u64().unwrap() as usize;
    let mut resources = vec![];
    let altar = if llu && count > 0 && mode != "wrong_zone" && mode != "wrong_type" {
        Some(paid_cast(
            &mut g,
            &linked_defs,
            "Altar of Dementia",
            2,
            &mut q,
            dm,
        )?)
    } else {
        None
    };
    for i in 0..count {
        let mat = if llu {
            if mode == "wrong_type" {
                "Shock"
            } else {
                "Grizzly Bears"
            }
        } else if trio {
            if mode == "wrong_type" {
                "Darksteel Relic"
            } else {
                "Grizzly Bears"
            }
        } else if mode == "wrong_type" {
            "Grizzly Bears"
        } else if mode == "one_tapped" && i == 0 {
            "Mind Stone"
        } else {
            "Darksteel Relic"
        };
        let m = if mode == "wrong_zone" {
            let id = g.create_object_from_definition(&defs[mat].0, actor, Zone::Hand);
            g.object(id).unwrap().stable_id
        } else {
            dm.targets = if mat == "Shock" {
                vec![Target::Player(PlayerId(2))]
            } else {
                vec![]
            };
            let m = paid_cast(
                &mut g,
                &linked_defs,
                mat,
                match mat {
                    "Grizzly Bears" | "Mind Stone" => 2,
                    "Shock" => 1,
                    _ => 0,
                },
                &mut q,
                dm,
            )?;
            dm.targets.clear();
            if let Some(alt) = altar {
                sacrifice(&mut g, alt, m, &mut q, dm)?;
            }
            if mat == "Mind Stone" {
                let a=compute_legal_actions(&g,actor).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateManaAbility{source,..}if *source==current(&g,m))).ok_or("Mind Stone tap producer unavailable")?;
                immediate(&mut g, a, &mut q, dm)?;
            }
            m
        };
        resources.push(m);
    }
    set_actor(&mut g, dm, alice());
    if mode == "other_turn" {
        g.turn.active_player = PlayerId(1);
    }
    g.remove_summoning_sickness(sid);
    dm.chosen = resources.iter().map(|s| current(&g, *s)).collect();
    let index = c["ability_index"].as_u64().unwrap() as usize;
    let actions = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let a=actions.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index,..}if *source==sid&&*ability_index==index)).cloned();
    dm.trace.push(json!({"stage":"linked_candidate","has_prepare_spell":g.has_prepare_spell(sid),"prepared":g.is_prepared(sid),"legal_actions":format!("{actions:?}"),"ability_index":index}));
    let valid = c["valid"].as_bool().unwrap();
    if a.is_none() || !valid {
        if trio && mode == "one_creature" && a.is_some() {
            dm.stage = "advertised_but_insufficient_total_cost".into();
            let error = announce(&mut g, a.clone().unwrap(), &mut q, dm).err();
            dm.trace.push(json!({"stage":"insufficient_cost_attempt","error":error,"source_tapped":g.is_tapped(sid),"resource_tapped":resources.iter().map(|s|g.is_tapped(current(&g,*s))).collect::<Vec<_>>() }));
        }
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":a.is_some()}),
        ));
    }
    let before = g.player(alice()).unwrap().mana_pool.total();
    dm.stage = "linked_activation".into();
    let mut error = announce(&mut g, a.unwrap(), &mut q, dm).err();
    dm.chosen.clear();
    if error.is_none() {
        error = finish(&mut g, &mut q, dm).err();
    }
    let mut expected = json!({"error":null,"activation_available":true,"mana_paid":0});
    let mut actual = json!({"error":error,"activation_available":true,"mana_paid":before-g.player(alice()).unwrap().mana_pool.total()});
    if llu {
        expected["cost_zones"] = json!(
            resources
                .iter()
                .enumerate()
                .map(|(i, _)| if i == 0 { "Exile" } else { "Graveyard" })
                .collect::<Vec<_>>()
        );
        actual["cost_zones"] = json!(
            resources
                .iter()
                .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
                .collect::<Vec<_>>()
        );
    } else {
        expected["cost_tapped"] = json!(
            resources
                .iter()
                .enumerate()
                .map(|(i, _)| i < 2)
                .collect::<Vec<_>>()
        );
        actual["cost_tapped"] = json!(
            resources
                .iter()
                .map(|s| g.is_tapped(current(&g, *s)))
                .collect::<Vec<_>>()
        );
        expected["source_tapped"] = json!(trio);
        actual["source_tapped"] = json!(g.is_tapped(sid));
    }
    if llu || trio {
        expected["prepared"] = json!(true);
        actual["prepared"] = json!(g.is_prepared(sid));
        expected["prepared_copy_count"] = json!(1);
        actual["prepared_copy_count"] = json!(
            g.objects_in_deterministic_order()
                .into_iter()
                .filter(|o| g.prepared_spell_source(o.id) == Some(sid))
                .count()
        );
        let e = json!({"available":true,"error":null,"paid":1,"prepared":false,"hand_delta":if trio{1}else{0},"pests":if trio{0}else if mode=="already_prepared"{1}else{2}});
        expected["spell_cast"] = e;
        actual["spell_cast"] = prepared_cast(&mut g, sid, &mut q, dm)?;
    } else {
        expected["creature"] = json!(true);
        actual["creature"] =
            json!(g.current_has_card_type(sid, ironsmith::types::CardType::Creature));
        expected["pt"] = json!([5, 5]);
        actual["pt"] = json!([g.current_power(sid), g.current_toughness(sid)]);
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
    let input = p.join("linked-face-cost-inputs.json");
    let data: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let families = data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["group"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|n| (n.to_string(), LinkedFamily::from_catalog(&root, n).unwrap()))
        .collect::<HashMap<_, _>>();
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
        eprintln!("LINKED_FACE_COST {c}");
        let mut dm = Dm {
            actor: alice(),
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: c["reverse"].as_bool().unwrap_or(false),
            x: c["x"].as_u64().unwrap_or(0) as u32,
            plot: c["plot"].as_bool().unwrap_or(false),
            option_text: String::new(),
        };
        let result = run(&defs, &families[c["group"].as_str().unwrap()], c, &mut dm);
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
        "crates/ironsmith-engine/src/game_loop/priority_mana.rs",
        "crates/ironsmith-engine/src/game_state/object_state_and_events.rs",
        "crates/ironsmith-engine/src/effects/mana/add_mana_of_any_one_color.rs",
        "crates/ironsmith-engine/src/effects/composition/choose_objects.rs",
    ];
    let report = json!({"scope":"Canonical cards.json linked-face materialization and actual paid prepare/craft producer checks. Lluwen and Harmonized Trio plus aliases; Bladewheel through actual paid Sawblades and battlefield-material craft. Four linkage metadata fields and artifact paired IDs derived from real catalog, no behavior edits. Strict frozen pre-link definitions and linked artifacts preserved separately.","limitations":"Native production artifact materialization and GameState linked-face cache registration exercised; JS binding and WASM batch-remapping route not replayed. Initial mana/hand/library and source sickness are setup. No direct transform, set_prepared, clear_prepared or effect-context injection. Source cards, cost resources and prepared spells all use actual paid legal actions.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_linked_face_cost_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"helper_sha256":hash(&root.join("crates/ironsmith-tools/tests/support/canonical_linked_fixture.rs")),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows,"linkage_families":families.values().map(|f|json!({"metadata":f.metadata_record,"unlinked_artifacts":f.unlinked_artifacts,"linked_artifacts":f.linked_artifacts})).collect::<Vec<_>>()});
    std::fs::write(
        p.join("linked-face-cost-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("linked-face-cost-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical linked-face-cost expected-result reporter"]
fn report_linked_face_cost() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
