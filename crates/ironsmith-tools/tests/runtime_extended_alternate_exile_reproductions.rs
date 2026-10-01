//! Exact alternate Exile cost pairs with normal cast and resource boundary controls.
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
use ironsmith::{CardDefinition, CardId, GameState, ObjectId, Phase, PlayerId, Zone};
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
        self.trace.push(json!({"stage":self.stage,"choice":"number","context":format!("{c:?}"),"selected":self.x.max(c.min).min(c.max)}));
        self.x.max(c.min).min(c.max)
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
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let v = c["variant"].as_str().unwrap();
    let normal = v == "normal_paid";
    let payable = !["absent", "wrong_type", "required_minus_one"].contains(&v);
    let mut g = game();
    let mut q = TriggerQueue::new();
    for p in [alice(), PlayerId(1), PlayerId(2)] {
        for _ in 0..12 {
            g.create_object_from_definition(&defs["Forest"].0, p, Zone::Library);
        }
    }
    let witness = paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut q, dm)?;
    let mut materials = vec![];
    if n.ends_with("Shoal") {
        let resource = match n {
            "Blazing Shoal" => "Goblin Piker",
            "Disrupting Shoal" => "Coral Merfolk",
            "Nourishing Shoal" => "Grizzly Bears",
            _ => "Walking Corpse",
        };
        for _ in 0..if v == "absent" {
            0
        } else if v == "surplus" {
            2
        } else {
            1
        } {
            let id = g.create_object_from_definition(
                &defs[if v == "wrong_type" {
                    "Forest"
                } else {
                    resource
                }]
                .0,
                alice(),
                Zone::Hand,
            );
            materials.push(g.object(id).unwrap().stable_id);
        }
    } else {
        let count = if n == "Spinning Darkness" {
            if v == "required_minus_one" {
                1
            } else if v == "surplus" {
                3
            } else {
                2
            }
        } else if v == "absent" {
            0
        } else if v == "surplus" {
            2
        } else {
            1
        };
        for _ in 0..count {
            let name = if n == "Spinning Darkness" {
                "Dark Ritual"
            } else if v == "wrong_type" {
                "Forest"
            } else {
                "Grizzly Bears"
            };
            let id = g.create_object_from_definition(&defs[name].0, alice(), Zone::Hand);
            materials.push(g.object(id).unwrap().stable_id);
            if n == "Spinning Darkness" {
                g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Hand);
            }
        }
        let producer = paid_cast(&mut g, defs, "One with Nothing", 1, &mut q, dm)?;
        if n == "Spinning Darkness" {
            materials.push(producer)
        }
        if materials.iter().any(|s| zone(&g, *s) != "Graveyard") {
            return Err("actual discard resource producer failed".into());
        }
    }
    let mut target_spell = None;
    if n == "Disrupting Shoal" {
        set_actor(&mut g, dm, PlayerId(1));
        cast_announce(&mut g, &defs["Grizzly Bears"].0, 2, &mut q, dm)?;
        target_spell = g.stack.last().map(|e| e.object_id);
        set_actor(&mut g, dm, alice());
    }
    let id = g.create_object_from_definition(&defs[n].0, alice(), Zone::Hand);
    let source = g.object(id).unwrap().stable_id;
    let action=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==id&&if normal{matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Normal)}else{matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Alternative(0))}));
    let simple_expected =
        json!({"selected_method_offered":payable,"source_zone":"Hand","materials_exiled":0});
    let simple_actual = json!({"selected_method_offered":action.is_some(),"source_zone":zone(&g,source),"materials_exiled":0});
    dm.trace.push(json!({"stage":"before_selected_cast","materials":materials.iter().map(|s|json!({"name":g.object(current(&g,*s)).unwrap().name.to_string(),"zone":zone(&g,*s)})).collect::<Vec<_>>(),"action":format!("{action:?}")}));
    if action.is_none() || !payable {
        return Ok((simple_expected, simple_actual));
    }
    dm.stage = "announce_selected_alternate".into();
    dm.x = 2;
    dm.chosen = if n == "Spinning Darkness" {
        g.player(alice())
            .unwrap()
            .graveyard
            .iter()
            .rev()
            .filter(|id| materials.iter().any(|s| current(&g, *s) == **id))
            .take(3)
            .copied()
            .collect()
    } else {
        materials.iter().take(1).map(|s| current(&g, *s)).collect()
    };
    let intended_exiles = if normal { vec![] } else { dm.chosen.clone() };
    let intended_stable = intended_exiles
        .iter()
        .map(|id| g.object(*id).unwrap().stable_id)
        .collect::<Vec<_>>();
    dm.targets = match n {
        "Disrupting Shoal" => vec![Target::Object(target_spell.unwrap())],
        "Nourishing Shoal" | "Stalwart Valkyrie" => vec![],
        _ => vec![Target::Object(current(&g, witness))],
    };
    let before_mana = g.player(alice()).unwrap().mana_pool.total();
    let announcement_error = announce(&mut g, action.unwrap(), &mut q, dm).err();
    let paid = before_mana - g.player(alice()).unwrap().mana_pool.total();
    let expected_paid = if normal {
        if n == "Spinning Darkness" { 6 } else { 4 }
    } else if n == "Stalwart Valkyrie" {
        2
    } else {
        0
    };
    let expected_cost = json!({"announcement_error":null,"mana_paid":expected_paid,"exiled":intended_stable.len(),"chosen_exiles_match":true,"source_zone":"Stack"});
    let actual_cost = json!({"announcement_error":announcement_error,"mana_paid":paid,"exiled":materials.iter().filter(|s|zone(&g,**s)=="Exile").count(),"chosen_exiles_match":materials.iter().filter(|s|zone(&g,**s)=="Exile").all(|s|intended_stable.contains(s))&&intended_stable.iter().all(|s|zone(&g,*s)=="Exile"),"source_zone":zone(&g,source)});
    dm.trace.push(json!({"stage":"actual_selected_cost_result","expected":expected_cost,"actual":actual_cost}));
    if actual_cost != expected_cost {
        return Ok((expected_cost, actual_cost));
    }
    dm.stage = "resolve_selected_alternate".into();
    dm.chosen.clear();
    let error = finish(&mut g, &mut q, dm).err();
    let sid = current(&g, source);
    let wid = current(&g, witness);
    let expected_outcome = match n {
        "Blazing Shoal" => json!({"target_zone":"Battlefield","power":4,"toughness":2}),
        "Sickening Shoal" => json!({"target_zone":"Graveyard"}),
        "Nourishing Shoal" => json!({"alice_life":22}),
        "Disrupting Shoal" => json!({"bob_bears_battlefield":0,"bob_bears_graveyard":1}),
        "Stalwart Valkyrie" => json!({"source_zone":"Battlefield","power":3,"toughness":2}),
        "Spinning Darkness" => json!({"target_zone":"Graveyard","alice_life":23}),
        _ => unreachable!(),
    };
    let actual_outcome = match n {
        "Blazing Shoal" => {
            json!({"target_zone":zone(&g,witness),"power":g.current_power(wid),"toughness":g.current_toughness(wid)})
        }
        "Sickening Shoal" => json!({"target_zone":zone(&g,witness)}),
        "Nourishing Shoal" => json!({"alice_life":g.player(alice()).unwrap().life}),
        "Disrupting Shoal" => {
            json!({"bob_bears_battlefield":g.objects_in_deterministic_order().iter().filter(|o|o.owner==PlayerId(1)&&o.name=="Grizzly Bears"&&o.zone==Zone::Battlefield).count(),"bob_bears_graveyard":g.objects_in_deterministic_order().iter().filter(|o|o.owner==PlayerId(1)&&o.name=="Grizzly Bears"&&o.zone==Zone::Graveyard).count()})
        }
        "Stalwart Valkyrie" => {
            json!({"source_zone":zone(&g,source),"power":g.current_power(sid),"toughness":g.current_toughness(sid)})
        }
        "Spinning Darkness" => {
            json!({"target_zone":zone(&g,witness),"alice_life":g.player(alice()).unwrap().life})
        }
        _ => unreachable!(),
    };
    Ok((
        json!({"cost":expected_cost,"resolution_error":null,"outcome":expected_outcome}),
        json!({"cost":actual_cost,"resolution_error":error,"outcome":actual_outcome}),
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
    let input = p.join("extended-alternate-exile-inputs.json");
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
        eprintln!("ALTERNATE_EXILE_CASE {c}");
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
    let report = json!({"scope":"Six exact alternate Exile cost paths: four Shoals, Stalwart Valkyrie and Spinning Darkness. Canonical source/matching resource hands; graveyard materials through actual paid One with Nothing. Absent/wrong-type/exact/surplus and normal-paid controls, selected advertised method only. Full cost payment and independent outcome checks. No unavailable action forced.","limitations":"Initial hand source/materials and mana/library are fixtures. Graveyard resources arise through actual discard; target creatures and the counter target are paid canonical casts. Shoal X2 is requested using real announcement callback; actual chosen X is recorded. Negative missing-resource actions are inspected only, never forced. Top-three-black cost uses actual graveyard order with interleaved nonblack cards. No other cost-method branches certified.","provenance":{"binary":std::env::current_exe().unwrap(),"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_extended_alternate_exile_reproductions.rs")),"input_sha256":hash(&input),"seed":SEED,"unique_card_ids":true,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("extended-alternate-exile-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("extended-alternate-exile-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical Escape additional-cost expected-result reporter"]
fn report_extended_alternate_exile() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
