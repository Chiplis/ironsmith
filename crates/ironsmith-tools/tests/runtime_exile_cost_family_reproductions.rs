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
use ironsmith::{CardDefinition, CardId, GameState, ObjectId, Phase, PlayerId, Zone};
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
        let selected = self
            .chosen
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
            .take(c.max.unwrap_or(usize::MAX))
            .collect::<Vec<_>>();
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
    let payload = c["card"].as_str().unwrap();
    let n = payload.split(" // ").next().unwrap();
    let mut g = game();
    let mut q = TriggerQueue::new();
    let source_stable = paid_cast(
        &mut g,
        defs,
        payload,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    let source = current(&g, source_stable);
    let craft = n == "Ore-Rich Stalactite" || n == "Sunbird Standard" || n == "Braided Net";
    let index = c["ability_index"].as_u64().unwrap_or(1) as usize;
    let mut target = None;
    if n == "Fabrication Foundry" {
        let stable = paid_cast(&mut g, defs, "Ornithopter", 0, &mut q, dm)?;
        dm.targets = vec![Target::Object(current(&g, stable))];
        paid_cast(&mut g, defs, "Disenchant", 2, &mut q, dm)?;
        dm.targets.clear();
        target = Some(stable);
    }
    let mut resources = vec![];
    let zone = c["material_zone"].as_str().unwrap();
    for v in c["materials"].as_array().unwrap() {
        let card = v.as_str().unwrap();
        let is_spell = card == "Shock" || card == "Opt";
        if card == "Shock" {
            dm.targets = vec![Target::Player(PlayerId(1))];
        }
        if card == "Opt" {
            g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Library);
        }
        let cost = match card {
            "Ornithopter" => 0,
            "Shock" | "Opt" | "Sol Ring" => 1,
            "Mind Stone" | "Grizzly Bears" => 2,
            "Draco" => 16,
            "Metalwork Colossus" => 11,
            _ => unreachable!(),
        };
        let stable = paid_cast(&mut g, defs, card, cost, &mut q, dm)?;
        dm.targets.clear();
        if zone == "Graveyard" && !is_spell {
            dm.targets = vec![Target::Object(current(&g, stable))];
            paid_cast(&mut g, defs, "Disenchant", 2, &mut q, dm)?;
            dm.targets.clear();
        }
        let expected = if is_spell || zone == "Graveyard" {
            Zone::Graveyard
        } else {
            Zone::Battlefield
        };
        if g.object(current(&g, stable)).unwrap().zone != expected {
            return Err(format!("resource {card} wrong zone"));
        }
        resources.push(stable);
    }
    dm.chosen = resources.iter().map(|s| current(&g, *s)).collect();
    if let Some(t) = target {
        dm.targets = vec![Target::Object(current(&g, t))];
    }
    if c["sorcery_window"] == false {
        g.next_turn();
        ironsmith::turn::execute_untap_step(&mut g);
        ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        for sym in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            g.player_mut(alice()).unwrap().mana_pool.add(sym, 30);
        }
    }
    g.turn.priority_player = Some(alice());
    let legal = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let action = legal
        .iter()
        .find(
            |a| matches!(a,LegalAction::ActivateAbility{source:s,ability_index:i,..}if *s==source && *i==index),
        )
        .cloned();
    let ability = match &defs[payload].0.abilities[index].kind {
        ironsmith::ability::AbilityKind::Activated(a) => a,
        _ => return Err("ability1 not activated".into()),
    };
    let resource_states=resources.iter().map(|s|{let o=g.object(current(&g,*s)).unwrap();json!({"id":o.id.0,"name":o.name.to_string(),"zone":format!("{:?}",o.zone),"mana_value":o.mana_cost.as_ref().map(|m|m.mana_value()).unwrap_or(0),"owner":o.owner.0,"controller":g.current_controller(o.id).map(|p|p.0)})}).collect::<Vec<_>>();
    dm.trace.push(json!({"stage":"actual_activation_legality","legal_actions":format!("{legal:?}"),"ability_index":index,"source_untapped":!g.is_tapped(source),"source_current_zone":format!("{:?}",g.object(source).unwrap().zone),"own_main_phase":g.turn.active_player==alice()&&g.turn.phase==Phase::FirstMain,"stack_empty":g.stack.is_empty(),"resources":resource_states,"card_other_face":format!("{:?}",defs[payload].0.card.other_face),"total_cost_check":format!("{:?}",ironsmith::cost::can_pay_cost_with_reason(&g,source,alice(),&ability.mana_cost,ironsmith::costs::PaymentReason::ActivateAbility)),"component_checks":ability.mana_cost.costs().iter().map(|cost|json!({"cost":format!("{cost:?}").chars().take(250).collect::<String>(),"check":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*cost.0,&g,&ironsmith::costs::CostCheckContext::new(source,alice()).with_reason(ironsmith::costs::PaymentReason::ActivateAbility)))})).collect::<Vec<_>>()}));
    let expected_legal = c["valid"].as_bool().unwrap();
    if action.is_none() || !expected_legal {
        return Ok((
            json!({"error":null,"legal_activation_available":expected_legal}),
            json!({"error":null,"legal_activation_available":action.is_some()}),
        ));
    }
    dm.stage = "actual_cost_payment".into();
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut error = announce(&mut g, action.unwrap(), &mut q, dm).err();
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    let source_zone = format!("{:?}", g.object(current(&g, source_stable)).unwrap().zone);
    let material_zones = resources
        .iter()
        .map(|s| format!("{:?}", g.object(current(&g, *s)).unwrap().zone))
        .collect::<Vec<_>>();
    let expected_paid = if craft {
        c["activation_mana"].as_u64().unwrap_or(5) as u32
    } else if n == "Fabrication Foundry" {
        3
    } else {
        0
    };
    dm.trace.push(json!({"stage":"actual_costs_paid","mana_paid":paid,"source_zone":source_zone,"material_zones":material_zones,"stack":format!("{:?}",g.stack),"error":error}));
    if craft {
        return Ok((
            json!({"error":null,"legal_activation_available":true,"mana_paid":expected_paid,"source_zone_after_cost":"Exile","material_zones_after_cost":vec!["Exile";resources.len()],"stacked":true}),
            json!({"error":error,"legal_activation_available":true,"mana_paid":paid,"source_zone_after_cost":source_zone,"material_zones_after_cost":material_zones,"stacked":!g.stack.is_empty()}),
        ));
    }
    if error.is_none() {
        dm.stage = "ability_resolution".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let mut expected = json!({"error":null,"legal_activation_available":true,"mana_paid":expected_paid,"source_zone_after_cost":"Battlefield","material_zones_after_cost":vec!["Exile";resources.len()]});
    let mut actual = json!({"error":error,"legal_activation_available":true,"mana_paid":paid,"source_zone_after_cost":source_zone,"material_zones_after_cost":material_zones});
    if let Some(t) = target {
        expected["returned_target_zone"] = json!("Battlefield");
        actual["returned_target_zone"] =
            json!(format!("{:?}", g.object(current(&g, t)).unwrap().zone));
    } else {
        expected["source_power_after_emblem"] = json!(9);
        actual["source_power_after_emblem"] = json!(g.calculated_power(source));
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
    let input = p.join("exile-cost-family-inputs.json");
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
        eprintln!("EXILE_COST_FAMILY {c}");
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
    let report = json!({"scope":"Non-single ChooseObjects followed by tagged ExileEffect activation costs: actual full canonical source casts, paid resource casts and Disenchant graveyard producers. Explicit source ability1 legal discovery; missing actions never forced. Craft positives stop after actual announcement/cost payment because frozen definitions omit back-face metadata.","limitations":"Mana is seeded and Opt draw witnesses seeded. Graveyard and battlefield cost resources otherwise obtained through actual paid canonical casts and paid Disenchant or spell completion. Craft transformed-face correctness is untested. Existing Foundry and Capitoline findings are additional family evidence, not new names.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_exile_cost_family_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("exile-cost-family-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("exile-cost-family-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical exile-cost-family expected-result reporter"]
fn report_exile_cost_family() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
