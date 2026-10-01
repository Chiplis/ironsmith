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
fn pt(g: &GameState, id: ObjectId) -> Value {
    json!([g.current_power(id), g.current_toughness(id)])
}
fn keyword(g: &GameState, id: ObjectId, k: ironsmith::static_abilities::StaticAbilityId) -> bool {
    g.current_has_static_ability_id(id, k)
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
    // Source enters with its full canonical definition through normal paid priority resolution.
    let source = if c["land"].as_bool().unwrap() {
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
    if primary == "Great Arashin City" {
        next_main(&mut g);
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
    }
    let mut witnesses = vec![];
    let mut cost_tokens = vec![];
    if primary == "Screams of the Damned" {
        for p in [alice(), PlayerId(1)] {
            set_actor(&mut g, dm, p);
            witnesses.push(paid_cast(&mut g, defs, "Hill Giant", 4, &mut q, dm)?);
        }
    }
    set_actor(&mut g, dm, alice());
    if mode == "additional_construct" {
        paid_cast(&mut g, defs, "Memnite", 0, &mut q, dm)?;
    }
    if primary == "Tawnos, Solemn Survivor" {
        paid_cast(&mut g, defs, "Servo Exhibition", 2, &mut q, dm)?;
        cost_tokens = g
            .battlefield
            .iter()
            .copied()
            .filter(|id| g.object(*id).is_some_and(|o| o.name == "Servo"))
            .collect();
        if cost_tokens.len() != 2 {
            return Err("Servo producer did not produce two tokens".into());
        }
    }
    let actor = if mode == "opposing_owner" {
        PlayerId(1)
    } else {
        alice()
    };
    set_actor(&mut g, dm, actor);
    let count = c["resources"].as_u64().unwrap() as usize;
    let material = c["material"].as_str().unwrap();
    let mut resources = vec![];
    let needs_altar = count > 0
        && mode != "wrong_zone"
        && defs[material]
            .0
            .card
            .card_types
            .contains(&ironsmith::types::CardType::Creature);
    let altar = if needs_altar {
        Some(paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?)
    } else {
        None
    };
    for _ in 0..count {
        dm.chosen.clear();
        dm.targets.clear();
        let m = if mode == "wrong_zone" {
            let id = g.create_object_from_definition(&defs[material].0, actor, Zone::Hand);
            g.object(id).unwrap().stable_id
        } else if material == "Forest" {
            let id = g.create_object_from_definition(&defs[material].0, actor, Zone::Library);
            let st = g.object(id).unwrap().stable_id;
            dm.plot = false;
            paid_cast(&mut g, defs, "Satyr Wayfinder", 2, &mut q, dm)?;
            if g.object(current(&g, st)).unwrap().zone != Zone::Graveyard {
                return Err("Wayfinder did not mill land with optional choice declined".into());
            }
            st
        } else {
            if material == "Shock" {
                dm.targets = vec![Target::Player(PlayerId(2))];
            }
            let st = paid_cast(
                &mut g,
                defs,
                material,
                c["material_cost"].as_u64().unwrap() as u32,
                &mut q,
                dm,
            )?;
            dm.targets.clear();
            if let Some(altar) = altar {
                sacrifice(&mut g, altar, st, &mut q, dm)?;
            } else if material == "Mind Stone" {
                dm.targets = vec![Target::Object(current(&g, st))];
                paid_cast(&mut g, defs, "Disenchant", 2, &mut q, dm)?;
                dm.targets.clear();
            }
            st
        };
        resources.push(m);
    }
    set_actor(&mut g, dm, alice());
    let sid = current(&g, source);
    g.remove_summoning_sickness(sid);
    if mode == "other_turn" {
        g.turn.active_player = PlayerId(1);
    }
    if primary == "Ellie and Alan, Paleontologists" {
        for card in ["Forest", "Llanowar Elves", "Hill Giant", "Forest"] {
            g.create_object_from_definition(&defs[card].0, alice(), Zone::Library);
        }
        dm.plot = mode != "discover_to_hand";
        if mode == "discover_to_hand" {
            dm.option_text = "hand".into();
        }
    }
    if primary == "Disciple of the Ring" {
        dm.option_text = "+1/+1".into();
    }
    if primary == "Lluwen, Exchange Student" {
        dm.trace.push(json!({"stage":"canonical_prepare_metadata","has_prepare_spell":g.has_prepare_spell(sid),"prepared":g.is_prepared(sid)}));
    }
    if primary == "Titans' Nest" {
        g.player_mut(alice()).unwrap().mana_pool = Default::default();
        g.player_mut(alice()).unwrap().mana_pool.add(
            if mode == "forbidden_x" {
                ManaSymbol::Green
            } else {
                ManaSymbol::Blue
            },
            1,
        );
    }
    dm.chosen = cost_tokens.clone();
    dm.chosen.extend(resources.iter().map(|s| current(&g, *s)));
    dm.x = c["x"].as_u64().unwrap_or(0) as u32;
    let index = c["ability_index"].as_u64().unwrap() as usize;
    let actions = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let action=actions.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index,..}|LegalAction::ActivateManaAbility{source,ability_index,..}if *source==sid&&*ability_index==index)).cloned();
    dm.trace.push(json!({"stage":"candidate_exact_path","ability_index":index,"cost_path":c["cost_path"],"legal_actions":format!("{actions:?}"),"materials":resources.iter().map(|s|json!({"id":current(&g,*s).0,"zone":format!("{:?}",g.object(current(&g,*s)).unwrap().zone)})).collect::<Vec<_>>() }));
    let valid = c["valid"].as_bool().unwrap();
    if action.is_none() || !valid {
        return Ok((
            json!({"error":null,"legal_activation_available":valid}),
            json!({"error":null,"legal_activation_available":action.is_some()}),
        ));
    }
    let token_before = g.battlefield.clone();
    let before = g.player(alice()).unwrap().mana_pool.total() as i64;
    let life_before = g.players.iter().map(|p| p.life).collect::<Vec<_>>();
    let action = action.unwrap();
    dm.stage = "actual_candidate_activation".into();
    let mut error = if matches!(action, LegalAction::ActivateManaAbility { .. }) {
        immediate(&mut g, action, &mut q, dm)
    } else {
        announce(&mut g, action, &mut q, dm)
    }
    .err();
    let zones = resources
        .iter()
        .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
        .collect::<Vec<_>>();
    let cost_tokens_gone = cost_tokens.iter().all(|id| !g.battlefield.contains(id));
    dm.chosen.clear();
    if error.is_none() {
        dm.stage = "candidate_resolution".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let spent = before - g.player(alice()).unwrap().mana_pool.total() as i64;
    let mut expected = json!({"error":null,"legal_activation_available":true,"mana_net_spent":c["activation_net"],"cost_zones":resources.iter().enumerate().map(|(i,_)|if i==0{"Exile"}else{"Graveyard"}).collect::<Vec<_>>()});
    let mut actual = json!({"error":error,"legal_activation_available":true,"mana_net_spent":spent,"cost_zones":zones});
    let tokens = g
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            !token_before.contains(id)
                && g.object(*id)
                    .is_some_and(|o| matches!(o.kind, ironsmith::object::ObjectKind::Token))
        })
        .collect::<Vec<_>>();
    match primary {
        "Disciple of the Ring" => {
            expected["source_pt"] = json!([4, 5]);
            actual["source_pt"] = pt(&g, sid);
        }
        "Hostile Desert" => {
            expected["source_pt"] = json!([3, 4]);
            actual["source_pt"] = pt(&g, sid);
            expected["land_creature_elemental"] = json!(true);
            actual["land_creature_elemental"] = json!(
                g.current_has_card_type(sid, ironsmith::types::CardType::Land)
                    && g.current_has_card_type(sid, ironsmith::types::CardType::Creature)
                    && g.current_has_subtype(sid, ironsmith::types::Subtype::Elemental)
            );
        }
        "Great Arashin City" | "Moorland Haunt" => {
            expected["tokens"] =
                json!([{ "name":"Spirit","pt":[1,1],"flying":primary=="Moorland Haunt"}]);
            actual["tokens"]=json!(tokens.iter().map(|id|json!({"name":g.object(*id).unwrap().name.to_string(),"pt":pt(&g,*id),"flying":keyword(&g,*id,ironsmith::static_abilities::StaticAbilityId::Flying)})).collect::<Vec<_>>());
        }
        "Dollhouse of Horrors" => {
            let x = if mode == "additional_construct" { 2 } else { 1 };
            expected["tokens"] = json!([{"name":"Grizzly Bears","pt":[x,x],"artifact":true,"construct":true,"haste":true}]);
            actual["tokens"]=json!(tokens.iter().map(|id|json!({"name":g.object(*id).unwrap().name.to_string(),"pt":pt(&g,*id),"artifact":g.current_has_card_type(*id,ironsmith::types::CardType::Artifact),"construct":g.current_has_subtype(*id,ironsmith::types::Subtype::Construct),"haste":keyword(&g,*id,ironsmith::static_abilities::StaticAbilityId::Haste)})).collect::<Vec<_>>());
        }
        "Ellie and Alan, Paleontologists" => {
            let elf = g
                .objects_in_deterministic_order()
                .into_iter()
                .find(|o| o.name == "Llanowar Elves")
                .map(|o| format!("{:?}", o.zone));
            expected["discovered_elf_zone"] = json!(if mode == "discover_to_hand" {
                "Hand"
            } else {
                "Battlefield"
            });
            actual["discovered_elf_zone"] = json!(elf);
            expected["remaining_library_size"] = json!(3);
            actual["remaining_library_size"] = json!(g.player(alice()).unwrap().library.len());
        }
        "Osgir, the Reconstructor" => {
            expected["tokens"] = json!(vec![material; 2]);
            actual["tokens"] = json!(
                tokens
                    .iter()
                    .map(|id| g.object(*id).unwrap().name.to_string())
                    .collect::<Vec<_>>()
            );
        }
        "Tawnos, Solemn Survivor" => {
            expected["sacrificed_artifact_tokens"] = json!(true);
            actual["sacrificed_artifact_tokens"] = json!(cost_tokens_gone);
            expected["copies"] = json!([{"name":material,"artifact":true,"pt":if material=="Ornithopter"{json!([0,2])}else{json!([2,2])}}]);
            actual["copies"]=json!(tokens.iter().map(|id|json!({"name":g.object(*id).unwrap().name.to_string(),"artifact":g.current_has_card_type(*id,ironsmith::types::CardType::Artifact),"pt":pt(&g,*id)})).collect::<Vec<_>>());
        }
        "Lluwen, Exchange Student" => {
            if g.has_prepare_spell(sid) {
                expected["prepared"] = json!(true);
                actual["prepared"] = json!(g.is_prepared(sid));
            } else {
                dm.trace.push(json!({"stage":"consumer_scope","status":"metadata_limited","reason":"No prepare spell in full canonical source or combined alias; no fabricated spell face or unprepare state. Exile cost committed and ability resolved; preparation consumer not certified."}));
            }
        }
        "Rubble Rouser" => {
            expected["life_delta"] = json!([0, 1, 1]);
            actual["life_delta"] = json!(
                g.players
                    .iter()
                    .enumerate()
                    .map(|(i, p)| life_before[i] - p.life)
                    .collect::<Vec<_>>()
            );
        }
        "Screams of the Damned" => {
            expected["life_delta"] = json!([1, 1, 1]);
            actual["life_delta"] = json!(
                g.players
                    .iter()
                    .enumerate()
                    .map(|(i, p)| life_before[i] - p.life)
                    .collect::<Vec<_>>()
            );
            expected["witness_damage"] = json!([1, 1]);
            actual["witness_damage"] = json!(
                witnesses
                    .iter()
                    .map(|s| g.damage_on(current(&g, *s)))
                    .collect::<Vec<_>>()
            );
        }
        "Titans' Nest" => {
            expected["colorless_generated"] = json!(1);
            actual["colorless_generated"] = json!(g.player(alice()).unwrap().mana_pool.colorless);
            if mode.starts_with("forbidden_") {
                let spell = if mode == "forbidden_x" {
                    "Stream of Life"
                } else {
                    "Mind Stone"
                };
                dm.x = 1;
                dm.targets = if mode == "forbidden_x" {
                    vec![Target::Player(alice())]
                } else {
                    vec![]
                };
                let id = g.create_object_from_definition(&defs[spell].0, alice(), Zone::Hand);
                let act = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
                    .into_iter()
                    .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id));
                expected["forbidden_cast_completed"] = json!(false);
                actual["forbidden_cast_completed"] = json!(false);
                if let Some(a) = act {
                    dm.stage = "actual_forbidden_restricted_mana_spell".into();
                    let before = g.player(alice()).unwrap().mana_pool.total();
                    let e = announce(&mut g, a, &mut q, dm)
                        .and_then(|_| finish(&mut g, &mut q, dm))
                        .err();
                    dm.trace.push(json!({"stage":"forbidden_cast_result","spell":spell,"paid":before-g.player(alice()).unwrap().mana_pool.total(),"error":e,"alice_life":g.player(alice()).unwrap().life}));
                    actual["forbidden_cast_completed"] = json!(e.is_none());
                }
            } else if resources.len() == 1 {
                g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Library);
                let id = g.create_object_from_definition(
                    &defs["Omen of the Sea"].0,
                    alice(),
                    Zone::Hand,
                );
                let act = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
                    .into_iter()
                    .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id));
                expected["colored_cast_available"] = json!(true);
                actual["colored_cast_available"] = json!(act.is_some());
                if let Some(a) = act {
                    dm.stage = "actual_restricted_mana_spell".into();
                    let e = announce(&mut g, a, &mut q, dm)
                        .and_then(|_| finish(&mut g, &mut q, dm))
                        .err();
                    expected["colored_cast_error"] = Value::Null;
                    actual["colored_cast_error"] = json!(e);
                }
            }
        }
        _ => return Err("unknown card".into()),
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
    let input = p.join("single-exile-remaining-inputs.json");
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
        eprintln!("SINGLE_EXILE_REMAINING {c}");
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
    let report = json!({"scope":"Thirteen remaining exact single-exile graveyard paths, actual paid sources or legal land plays; actual cast/sacrifice/destroy/mill resource producers. Zero/exact/surplus, owner/zone/type and sorcery timing boundaries. Independent printed outcomes; selected modal pump/discover alternatives, token-copy and mana restrictions, no forced unavailable activation.","limitations":"Mana and initial hand/library setup, clearing source sickness, and explicit main phases are fixture setup. Paid canonical resources become graveyard cards through Altar sacrifices, resolved spells, Disenchant or Satyr Wayfinder. Lluwen prepare metadata limitation is observed separately without fabricating a spell face. Only selected Disciple pump mode and Titans colored/noncolored spending boundary covered; duration expiry and unrelated abilities outside scope.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_single_exile_remaining_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("single-exile-remaining-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("single-exile-remaining-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical single-exile-remaining expected-result reporter"]
fn report_single_exile_remaining() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
