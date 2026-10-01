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

fn set_actor(g: &mut GameState, dm: &mut Dm, player: PlayerId) {
    dm.actor = player;
    g.turn.active_player = player;
    g.turn.priority_player = Some(player);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
}
fn sacrifice(
    g: &mut GameState,
    altar: ironsmith::ids::StableId,
    subject: ironsmith::ids::StableId,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    dm.chosen = vec![current(g, subject)];
    dm.targets = vec![Target::Player(PlayerId(2))];
    dm.stage = "actual_paid_altar_sacrifice".into();
    let a=compute_legal_actions(g,dm.actor).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index:0,..}if *source==current(g,altar))).ok_or("actual sacrifice action absent")?;
    announce(g, a, q, dm)?;
    finish(g, q, dm)?;
    assert_eq!(g.object(current(g, subject)).unwrap().zone, Zone::Graveyard);
    dm.chosen.clear();
    dm.targets.clear();
    Ok(())
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
    let source = paid_cast(
        &mut g,
        defs,
        n,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    let grave_source = ["Bone Dragon", "Despoiler of Souls", "Salvage Titan"].contains(&n);
    let mut materials = vec![];
    if n == "Say Its Name" {
        for _ in 0..total {
            materials.push(paid_cast(&mut g, defs, n, 2, &mut q, dm)?);
        }
    } else {
        let actor = PlayerId(c["resource_player"].as_u64().unwrap() as u8);
        set_actor(&mut g, dm, actor);
        let altar = paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?;
        if grave_source {
            sacrifice(&mut g, altar, source, &mut q, dm)?;
        }
        for _ in 0..total {
            let (name, cost) = if n == "Kethis, the Hidden Hand" {
                ("Isamaru, Hound of Konda", 1)
            } else {
                ("Ornithopter", 0)
            };
            let m = paid_cast(&mut g, defs, name, cost, &mut q, dm)?;
            sacrifice(&mut g, altar, m, &mut q, dm)?;
            materials.push(m);
        }
        if c["split"].as_bool().unwrap() {
            set_actor(&mut g, dm, alice());
            let altar2 = paid_cast(&mut g, defs, "Altar of Dementia", 2, &mut q, dm)?;
            let m = paid_cast(&mut g, defs, "Ornithopter", 0, &mut q, dm)?;
            sacrifice(&mut g, altar2, m, &mut q, dm)?;
            materials.push(m);
        }
    }
    set_actor(&mut g, dm, alice());
    if grave_source || n == "Say Its Name" {
        assert_eq!(g.object(current(&g, source)).unwrap().zone, Zone::Graveyard);
    }
    let source_id = current(&g, source);
    dm.targets.clear();
    dm.chosen = materials
        .iter()
        .take(req)
        .map(|s| current(&g, *s))
        .collect();
    let source_in_cost = n == "Salvage Titan" && total == 2;
    if source_in_cost || n == "Say Its Name" {
        dm.chosen.push(source_id);
    }
    let altanak = if n == "Say Its Name" {
        Some(g.create_object_from_definition(
            &defs["Altanak, the Thrice-Called"].0,
            alice(),
            Zone::Hand,
        ))
    } else {
        None
    };
    let actions = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let action=actions.iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index,..}if *source==source_id&&*ability_index==index)).cloned();
    let ability = match &defs[n].0.abilities[index].kind {
        ironsmith::ability::AbilityKind::Activated(a) => a,
        _ => return Err("wrong kind".into()),
    };
    dm.trace.push(json!({"stage":"candidate_path","ability_index":index,"cost_path":c["cost_path"],"source_zone":format!("{:?}",g.object(source_id).unwrap().zone),"legal_actions":format!("{actions:?}"),"gy_owners":materials.iter().map(|s|g.object(current(&g,*s)).unwrap().owner.0).collect::<Vec<_>>(),"total_cost_check":format!("{:?}",ironsmith::cost::can_pay_cost_with_reason(&g,source_id,alice(),&ability.mana_cost,ironsmith::costs::PaymentReason::ActivateAbility))}));
    let valid = c["valid"].as_bool().unwrap();
    if action.is_none() || !valid {
        return Ok((
            json!({"error":null,"legal_activation_available":valid}),
            json!({"error":null,"legal_activation_available":action.is_some()}),
        ));
    }
    dm.stage = "candidate_actual_activation".into();
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut error = announce(&mut g, action.unwrap(), &mut q, dm).err();
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    let cost_zones = materials
        .iter()
        .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
        .collect::<Vec<_>>();
    let source_cost_zone = format!("{:?}", g.object(current(&g, source)).unwrap().zone);
    if let Some(id) = altanak {
        dm.chosen = vec![id];
    } else {
        dm.chosen.clear();
    }
    if error.is_none() {
        dm.stage = "candidate_resolution".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let mut expected = json!({"error":null,"legal_activation_available":true,"mana_paid":c["activation_mana"],"cost_zones":(0..materials.len()).map(|i|if i<req{"Exile"}else{"Graveyard"}).collect::<Vec<_>>()});
    let mut actual = json!({"error":error,"legal_activation_available":true,"mana_paid":paid,"cost_zones":cost_zones});
    match n {
        "Bone Dragon" | "Despoiler of Souls" => {
            expected["source_zone"] = json!("Battlefield");
            actual["source_zone"] =
                json!(format!("{:?}", g.object(current(&g, source)).unwrap().zone));
            expected["source_tapped"] = json!(n == "Bone Dragon");
            actual["source_tapped"] = json!(g.is_tapped(current(&g, source)));
        }
        "Salvage Titan" => {
            expected["source_cost_zone"] =
                json!(if source_in_cost { "Exile" } else { "Graveyard" });
            actual["source_cost_zone"] = json!(source_cost_zone);
            expected["source_final_zone"] = json!(if source_in_cost { "Exile" } else { "Hand" });
            actual["source_final_zone"] =
                json!(format!("{:?}", g.object(current(&g, source)).unwrap().zone));
        }
        "Say Its Name" => {
            expected["source_final_zone"] = json!("Exile");
            actual["source_final_zone"] =
                json!(format!("{:?}", g.object(current(&g, source)).unwrap().zone));
            expected["altanak_battlefield"] = json!(true);
            actual["altanak_battlefield"] = json!(g.battlefield.iter().any(|id| {
                g.object(*id)
                    .is_some_and(|o| o.name == "Altanak, the Thrice-Called")
            }));
        }
        "Kethis, the Hidden Hand" => {
            if total > req {
                let id = current(&g, *materials.last().unwrap());
                g.turn.priority_player = Some(alice());
                dm.trace.push(json!({"stage":"post_kethis_grant","active_player":g.turn.active_player.0,"priority_player":format!("{:?}",g.turn.priority_player),"phase":format!("{:?}",g.turn.phase),"stack_len":g.stack.len(),"mana_pool":format!("{:?}",g.player(alice()).unwrap().mana_pool),"remaining_id":id.0,"current_abilities":format!("{:?}",g.current_abilities(id)),"legal_actions":format!("{:?}",compute_legal_actions(&g,alice()).expect("fixture has complete replacement state"))}));
                let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,from_zone:Zone::Graveyard,..}if *spell_id==id));
                expected["graveyard_cast_available"] = json!(true);
                actual["graveyard_cast_available"] = json!(a.is_some());
                if let Some(a) = a {
                    let b = g.player(alice()).unwrap().mana_pool.total();
                    dm.stage = "actual_granted_graveyard_cast".into();
                    let e = announce(&mut g, a, &mut q, dm)
                        .and_then(|_| finish(&mut g, &mut q, dm))
                        .err();
                    expected["granted_cast_error"] = Value::Null;
                    actual["granted_cast_error"] = json!(e);
                    expected["granted_cast_paid"] = json!(1);
                    actual["granted_cast_paid"] =
                        json!(b - g.player(alice()).unwrap().mana_pool.total());
                    expected["remaining_legend_zone"] = json!("Battlefield");
                    actual["remaining_legend_zone"] = json!(format!(
                        "{:?}",
                        g.object(current(&g, *materials.last().unwrap()))
                            .unwrap()
                            .zone
                    ));
                }
            }
        }
        "Night Soil" => {
            expected["saprolings"] = json!([[1, 1]]);
            actual["saprolings"] = json!(
                g.battlefield
                    .iter()
                    .filter(|id| g.object(**id).is_some_and(|o| o.name == "Saproling"))
                    .map(|id| vec![g.calculated_power(*id), g.calculated_toughness(*id)])
                    .collect::<Vec<_>>()
            );
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
    let input = p.join("fixed-exile-graveyard-inputs.json");
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
        eprintln!("FIXED_EXILE_GRAVEYARD {c}");
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
    let report = json!({"scope":"Six fixed-count exile cost paths: actual paid source and resource-producing sacrifices, self versus other graveyard cards, single-graveyard boundaries, and exact immediate outcomes. Canonical full costs; only engine-offered activations used.","limitations":"Mana, hand/library initial resources, and own-main timing are fixture setup. Sources and graveyard cards are actually paid and resolved/sacrificed normally. Night Soil includes opposing and split-graveyard controls. Say Its Name uses hand Altanak and declines its earlier optional return. Kethis tests immediate permission by actually paying for one surviving graveyard legend; duration expiry remains outside scope. Salvage Titan exact-three includes itself legally and must remain exiled.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_fixed_exile_graveyard_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("fixed-exile-graveyard-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("fixed-exile-graveyard-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical fixed-exile-graveyard expected-result reporter"]
fn report_fixed_exile_graveyard() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
