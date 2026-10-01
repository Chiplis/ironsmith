//! Exact Escape cost-pair and outcome audit with actual paid discard producers.
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
        let selected = if (self.stage.starts_with("resolve_escape")
            || self.stage == "cast_prepared_spell"
            || self.stage == "resolve_Jadzi, Steward of Fate")
            && c.min > 0
        {
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
    g.mark_main_phase_started();
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
fn zone(g: &GameState, id: ironsmith::ids::StableId) -> String {
    format!("{:?}", g.object(current(g, id)).unwrap().zone)
}
fn plus(g: &GameState, id: ObjectId) -> u32 {
    g.object(id)
        .map(|o| {
            o.counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}
fn expected_pt(n: &str, surplus: bool) -> Option<(i32, i32, u32)> {
    Some(match n {
        "Alex Wilder, Runaway" => (3, 3, 0),
        "Bloodbraid Challenger" => (4, 3, 0),
        "Chainweb Aracnir" => (4, 5, 3),
        "Charred Graverobber" => (4, 2, 1),
        "Kroxa, Titan of Death's Hunger"
        | "Phlage, Titan of Fire's Fury"
        | "Uro, Titan of Nature's Wrath" => (6, 6, 0),
        "Loathsome Chimera" => (5, 2, 1),
        "Nethergoyf" => {
            if surplus {
                (1, 2, 0)
            } else {
                (0, 1, 0)
            }
        }
        "Ox of Agonas" => (5, 3, 1),
        "Phoenix of Ash" => (3, 3, 1),
        "Underworld Charger" => (5, 5, 2),
        "Underworld Rage-Hound" => (4, 2, 1),
        "Voracious Typhon" => (7, 7, 3),
        "Woe Strider" => (5, 4, 2),
        _ => return None,
    })
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let available = c["available"].as_u64().unwrap() as usize;
    let required = c["required"].as_u64().unwrap() as usize;
    let payable = c["variant"] != "required_minus_one";
    let surplus = c["variant"] == "surplus";
    let mut g = game();
    let mut q = TriggerQueue::new();
    for p in [alice(), PlayerId(1), PlayerId(2)] {
        for _ in 0..24 {
            g.create_object_from_definition(&defs["Forest"].0, p, Zone::Library);
        }
    }
    let witness = paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut q, dm)?;
    if c["variant"] == "normal_hand" {
        let source = paid_cast(&mut g, defs, n, 2, &mut q, dm)?;
        let id = current(&g, source);
        return Ok((
            json!({"source_zone":"Battlefield","power":1,"toughness":3,"haste":false}),
            json!({"source_zone":zone(&g,source),"power":g.current_power(id),"toughness":g.current_toughness(id),"haste":g.current_has_static_ability_id(id,ironsmith::static_abilities::StaticAbilityId::Haste)}),
        ));
    }
    let mut target_gy = None;
    if ["Cling to Dust", "From the Catacombs"].contains(&n) {
        set_actor(&mut g, dm, PlayerId(1));
        let id = g.create_object_from_definition(&defs["Grizzly Bears"].0, PlayerId(1), Zone::Hand);
        let s = g.object(id).unwrap().stable_id;
        paid_cast(&mut g, defs, "One with Nothing", 1, &mut q, dm)?;
        if zone(&g, s) != "Graveyard" {
            return Err("opponent discard producer failed".into());
        }
        target_gy = Some(s);
        set_actor(&mut g, dm, alice());
    }
    let id = g.create_object_from_definition(&defs[n].0, alice(), Zone::Hand);
    let source = g.object(id).unwrap().stable_id;
    let mut resources = vec![];
    for i in 0..available - 1 {
        let resource = if n == "Nethergoyf" && i == 1 {
            "Ornithopter"
        } else {
            "Forest"
        };
        let id = g.create_object_from_definition(&defs[resource].0, alice(), Zone::Hand);
        resources.push(g.object(id).unwrap().stable_id);
    }
    let discard = paid_cast(&mut g, defs, "One with Nothing", 1, &mut q, dm)?;
    resources.insert(0, discard);
    if zone(&g, source) != "Graveyard" || resources.iter().any(|id| zone(&g, *id) != "Graveyard") {
        return Err("actual discard did not populate required graveyard".into());
    }
    dm.stage = "escape_legality".into();
    let sid = current(&g, source);
    let action=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,from_zone:Zone::Graveyard,casting_method:ironsmith::alternative_cast::CastingMethod::Alternative(0),..}if *spell_id==sid));
    let before_life = g.players.iter().map(|p| p.life).collect::<Vec<_>>();
    let before_mana = g.player(alice()).unwrap().mana_pool.total();
    dm.trace.push(json!({"stage":"actual_discard_producer_state","source_zone":zone(&g,source),"resources":resources.iter().map(|s|json!({"card":g.object(current(&g,*s)).unwrap().name.to_string(),"zone":zone(&g,*s)})).collect::<Vec<_>>(),"advertised_escape_action":format!("{action:?}")}));
    let expected_available = json!({"escape_offered":payable,"mana_paid":0,"resources_exiled":0,"source_zone":"Graveyard"});
    if action.is_none() {
        return Ok((
            expected_available,
            json!({"escape_offered":false,"mana_paid":0,"resources_exiled":0,"source_zone":zone(&g,source)}),
        ));
    }
    if !payable {
        return Ok((
            expected_available,
            json!({"escape_offered":true,"mana_paid":0,"resources_exiled":0,"source_zone":zone(&g,source)}),
        ));
    }
    dm.stage = "announce_escape".into();
    dm.chosen = resources
        .iter()
        .take(required)
        .map(|s| current(&g, *s))
        .collect();
    dm.targets = match n {
        "Cling to Dust" | "From the Catacombs" => {
            vec![Target::Object(current(&g, target_gy.unwrap()))]
        }
        "Escape Velocity" | "Mogis's Favor" | "Sentinel's Eyes" | "Run for Your Life"
        | "Sleep of the Dead" => vec![Target::Object(current(&g, witness))],
        "Fruit of Tizerus" | "Sweet Oblivion" | "Phlage, Titan of Fire's Fury" => {
            vec![Target::Player(PlayerId(1))]
        }
        _ => vec![],
    };
    let announcement_error = announce(&mut g, action.unwrap(), &mut q, dm).err();
    let paid = before_mana - g.player(alice()).unwrap().mana_pool.total();
    let exiled = resources
        .iter()
        .filter(|s| zone(&g, **s) == "Exile")
        .count();
    let remaining = resources
        .iter()
        .filter(|s| zone(&g, **s) == "Graveyard")
        .count();
    let expected_cost = json!({"escape_offered":true,"announcement_error":null,"mana_paid":c["mana"],"resources_exiled":required,"resources_remaining":available-required,"source_zone":"Stack"});
    let actual_cost = json!({"escape_offered":true,"announcement_error":announcement_error,"mana_paid":paid,"resources_exiled":exiled,"resources_remaining":remaining,"source_zone":zone(&g,source)});
    dm.trace
        .push(json!({"stage":"escape_cost_result","expected":expected_cost,"actual":actual_cost}));
    if actual_cost != expected_cost {
        return Ok((expected_cost, actual_cost));
    }
    dm.stage = "resolve_escape".into();
    dm.chosen.clear();
    let resolution_error = finish(&mut g, &mut q, dm).err();
    let id = current(&g, source);
    let permanent = expected_pt(n, surplus).is_some()
        || n == "Elspeth, Sun's Nemesis"
        || ["Escape Velocity", "Mogis's Favor", "Sentinel's Eyes"].contains(&n);
    let expected_hand = match n {
        "Glimpse of Freedom" | "Uro, Titan of Nature's Wrath" => Some(1),
        "Ox of Agonas" => Some(3),
        _ => None,
    };
    let mut expected_life = before_life.clone();
    match n {
        "Cling to Dust" | "Uro, Titan of Nature's Wrath" => expected_life[0] += 3,
        "Phlage, Titan of Fire's Fury" => {
            expected_life[0] += 3;
            expected_life[1] -= 3
        }
        "Fruit of Tizerus" => expected_life[1] -= 2,
        "Kroxa, Titan of Death's Hunger" => {
            expected_life[1] -= 3;
            expected_life[2] -= 3
        }
        _ => {}
    }
    let pt = expected_pt(n, surplus)
        .map(|(p, t, c)| json!({"power":p,"toughness":t,"plus_one_counters":c}));
    let actual_pt=pt.as_ref().map(|_|json!({"power":g.current_power(id),"toughness":g.current_toughness(id),"plus_one_counters":plus(&g,id)}));
    let host_pt = match n {
        "Escape Velocity" => Some((3, 2)),
        "Mogis's Favor" => Some((4, 1)),
        "Sentinel's Eyes" => Some((3, 3)),
        _ => None,
    };
    let other_expected = match n {
        "Alex Wilder, Runaway" => json!({"haste":true}),
        "Cling to Dust" => json!({"target_zone":"Exile"}),
        "From the Catacombs" => {
            json!({"target_zone":"Battlefield","target_controller":0,"corpse_counters":1})
        }
        "Sweet Oblivion" => json!({"bob_graveyard":4}),
        "Sleep of the Dead" => json!({"target_tapped":true}),
        "Run for Your Life" => json!({"target_haste":true}),
        "Woe Strider" => json!({"tokens":1,"token_pt":[[0,1]]}),
        "Satyr's Cunning" => json!({"tokens":1,"token_pt":[[1,1]]}),
        "Elspeth, Sun's Nemesis" => json!({"loyalty":5}),
        _ => Value::Null,
    };
    let tokens = g
        .objects_in_deterministic_order()
        .into_iter()
        .filter(|o| {
            o.zone == Zone::Battlefield && matches!(o.kind, ironsmith::object::ObjectKind::Token)
        })
        .map(|o| json!([g.current_power(o.id), g.current_toughness(o.id)]))
        .collect::<Vec<_>>();
    let other_actual = match n {
        "Alex Wilder, Runaway" => {
            json!({"haste":g.current_has_static_ability_id(id,ironsmith::static_abilities::StaticAbilityId::Haste)})
        }
        "Cling to Dust" => json!({"target_zone":zone(&g,target_gy.unwrap())}),
        "From the Catacombs" => {
            let t = current(&g, target_gy.unwrap());
            json!({"target_zone":zone(&g,target_gy.unwrap()),"target_controller":g.current_controller(t).map(|p|p.0),"corpse_counters":g.object(t).unwrap().counters.iter().filter(|(k,_)|k.description().eq_ignore_ascii_case("corpse")).map(|(_,v)|*v).sum::<u32>()})
        }
        "Sweet Oblivion" => json!({"bob_graveyard":g.player(PlayerId(1)).unwrap().graveyard.len()}),
        "Sleep of the Dead" => json!({"target_tapped":g.is_tapped(current(&g,witness))}),
        "Run for Your Life" => {
            json!({"target_haste":g.current_has_static_ability_id(current(&g,witness),ironsmith::static_abilities::StaticAbilityId::Haste)})
        }
        "Woe Strider" | "Satyr's Cunning" => json!({"tokens":tokens.len(),"token_pt":tokens}),
        "Elspeth, Sun's Nemesis" => {
            json!({"loyalty":g.object(id).unwrap().counters.get(&CounterType::Loyalty).copied().unwrap_or(0)})
        }
        _ => Value::Null,
    };
    Ok((
        json!({"cost":expected_cost,"resolution_error":null,"source_zone":if permanent{"Battlefield"}else{"Graveyard"},"source_pt":pt,"hand_size":expected_hand,"life":expected_life,"host_pt":host_pt,"other":other_expected}),
        json!({"cost":actual_cost,"resolution_error":resolution_error,"source_zone":zone(&g,source),"source_pt":actual_pt,"hand_size":expected_hand.map(|_|g.player(alice()).unwrap().hand.len()),"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"host_pt":host_pt.map(|_|(g.current_power(current(&g,witness)),g.current_toughness(current(&g,witness)))),"other":other_actual}),
    ))
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
    let input = p.join("extended-escape-inputs.json");
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
            Err(e) => {
                rows.push(
                    json!({"card":n,"status":"compile_failed","actual":{"error":e.to_string()}}),
                );
            }
        }
    }
    for c in data["cases"].as_array().unwrap() {
        let n = c["card"].as_str().unwrap();
        eprintln!("ESCAPE_CASE {c}");
        let mut dm = Dm {
            actor: alice(),
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: false,
            x: 0,
            plot: false,
            option_text: String::new(),
        };
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&defs, c, &mut dm)));
        let (status, expected, actual) = match result {
            Ok(Ok((e, a))) => (
                if e == a {
                    "expected_result_observed"
                } else if !a["announcement_error"].is_null() || !a["resolution_error"].is_null() {
                    "resolution_failed"
                } else {
                    "semantic_mismatch"
                },
                e,
                a,
            ),
            Ok(Err(e)) => (
                "execution_or_fixture_error",
                Value::Null,
                json!({"error":e}),
            ),
            Err(_) => (
                "panicked",
                Value::Null,
                json!({"error":"fixture or runtime panic; inspect isolated row trace"}),
            ),
        };
        rows.push(json!({"card":n,"scenario":c,"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs.get(n).map(|x|&x.1),"execution_trace":dm.trace}));
    }
    let report = json!({"scope":"Exact27 canonical Escape additional-cost paths. Actual paid One with Nothing discards source and initial hand resources; source is never inserted directly in graveyard. Required-minus-one/exact/surplus resources, including Nethergoyf distinct-card-type boundary. Normally advertised actual Escape actions, exact cost payment and independently derived scoped resolution outcomes. No missing action is forced.","limitations":"Initial mana, hand cards and libraries are fixtures. Every source and resource enters graveyard through actual discard. Only the stated cost and resolution fields are checked: Chainweb has no opposing flyer; Charred has no remaining own outlaw target; Bloodbraid library has only lands; Uro optional land is declined; ongoing haste/block/untap restrictions and planeswalker activations are not certified. Negative count cases only inspect legality; incorrectly advertised impossible actions are not dispatched.","provenance":{"binary":std::env::current_exe().unwrap(),"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_extended_escape_reproductions.rs")),"input_sha256":hash(&input),"seed":SEED,"unique_card_ids":true,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("extended-escape-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("extended-escape-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical Escape additional-cost expected-result reporter"]
fn report_extended_escape() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
