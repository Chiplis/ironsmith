//! Actual damage, tap and death-event LKI audit; no engine edits.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    DecisionContext, DistributeContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
struct Dm {
    targets: Vec<Target>,
    amounts: Vec<u32>,
    stage: String,
    trace: Vec<Value>,
    resolution_distributions: usize,
}
impl DecisionMaker for Dm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        assert_eq!(c.requirements.len(), 1, "fixture one printed target group");
        let r = &c.requirements[0];
        assert!(
            self.targets.len() >= r.min_targets
                && r.max_targets.is_none_or(|m| self.targets.len() <= m),
            "fixture target count legal"
        );
        assert!(
            self.targets.iter().all(|t| r.legal_targets.contains(t)),
            "fixture all targets legal"
        );
        self.trace.push(json!({"stage":self.stage,"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_distribute(&mut self, _: &GameState, c: &DistributeContext) -> Vec<(Target, u32)> {
        let chosen = self
            .targets
            .iter()
            .copied()
            .zip(self.amounts.iter().copied())
            .collect::<Vec<_>>();
        if self.stage == "ability_resolution" {
            self.resolution_distributions += 1;
        }
        self.trace.push(json!({"stage":self.stage,"choice":"distribution","context":format!("{c:?}"),"selected":format!("{chosen:?}"),"selected_total":self.amounts.iter().sum::<u32>(),"all_selected_targets_offered":chosen.iter().all(|(t,_)|c.targets.iter().any(|v|v.target==*t))}));
        chosen
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace.push(json!({"stage":self.stage,"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = c
            .candidates
            .iter()
            .find(|x| x.legal && x.name == "Arc Mage discard witness")
            .map(|x| vec![x.id])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_objects(g, c));
        self.trace.push(json!({"stage":self.stage,"choice":"objects","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
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
        g.player_mut(alice()).unwrap().mana_pool.add(s, 12);
    }
    g
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or(format!("fixture source {name} absent"))
}
fn count(g: &GameState, name: &str, z: Zone) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.name == name && o.zone == z)
        .count()
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
fn in_zone(g: &GameState, name: &str, z: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name == name && o.zone == z)
        .map(|o| o.id)
        .ok_or(format!("fixture {name} absent from {z:?}"))
}
fn response(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    spell: &str,
    target: ObjectId,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    let saved = dm.targets.clone();
    let amounts = dm.amounts.clone();
    dm.stage = format!("response_announce_{spell}");
    dm.targets = vec![Target::Object(target)];
    dm.amounts = vec![];
    cast_announce(
        g,
        &defs[spell].0,
        match spell {
            "Murder" => 3,
            "Boomerang" => 2,
            _ => 1,
        },
        q,
        dm,
    )?;
    dm.targets = saved;
    dm.amounts = amounts;
    dm.stage = format!("response_resolve_{spell}");
    one(g, q, dm)
}
fn immediate_action(
    g: &mut GameState,
    a: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":dm.stage,"actual_action":format!("{a:?}")}));
    let mut progress =
        apply_priority_response_with_dm(g, q, &mut state, &PriorityResponse::PriorityAction(a), dm)
            .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if let GameProgress::NeedsDecisionCtx(c) = progress {
            if !matches!(c, DecisionContext::Priority(_)) {
                progress = apply_decision_context_with_dm(g, q, &mut state, &c, dm)
                    .map_err(|e| e.to_string())?;
                continue;
            }
        }
        check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("immediate action budget".into())
}
fn zones(g: &GameState, name: &str) -> Value {
    json!({"battlefield":count(g,name,Zone::Battlefield),"hand":count(g,name,Zone::Hand),"graveyard":count(g,name,Zone::Graveyard),"exile":count(g,name,Zone::Exile)})
}
fn expected_zone(zone: &str) -> Value {
    json!({"battlefield":usize::from(zone=="battlefield"),"hand":usize::from(zone=="hand"),"graveyard":usize::from(zone=="graveyard"),"exile":usize::from(zone=="exile")})
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    name: &str,
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let mut g = game();
    let mut q = TriggerQueue::new();
    let draw = CardDefinitionBuilder::new(CardId::new(), "Cremate draw witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    g.create_object_from_definition(&draw, alice(), Zone::Library);
    dm.targets = vec![Target::Player(PlayerId(1))];
    let owner = if name == "City of Brass" {
        let h = g.create_object_from_definition(&defs[name].0, alice(), Zone::Hand);
        let a = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::PlayLand{land_id,..}if *land_id==h))
            .ok_or("fixture legal land play absent")?;
        dm.stage = "actual_land_play".into();
        immediate_action(&mut g, a, &mut q, dm)?;
        find(&g, name)?
    } else {
        dm.stage = "source_paid_cast".into();
        cast_announce(
            &mut g,
            &defs[name].0,
            if name == "Brash Taunter" { 5 } else { 3 },
            &mut q,
            dm,
        )?;
        dm.stage = "normal_source_resolution".into();
        finish(&mut g, &mut q, dm)?;
        find(&g, name)?
    };
    let event_name = if name == "Spiteful Sliver" {
        "Sliver Construct"
    } else {
        name
    };
    let event_object = if name == "Spiteful Sliver" {
        dm.stage = "separate_sliver_paid_cast".into();
        cast_announce(&mut g, &defs[event_name].0, 3, &mut q, dm)?;
        finish(&mut g, &mut q, dm)?;
        find(&g, event_name)?
    } else {
        owner
    };
    let mut before_power = g.calculated_power(event_object);
    let mut expected_owner_zone = "battlefield";
    let mut expected_event_zone = "battlefield";
    let amount;
    if name == "City of Brass" {
        amount = usize::from(mode != "untapped_control") as i32;
        if mode != "untapped_control" {
            let before = g.player(alice()).unwrap().mana_pool.total();
            let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateManaAbility{source,ability_index:1}if *source==owner)).ok_or("fixture City mana ability absent")?;
            dm.stage = "actual_city_mana_activation".into();
            immediate_action(&mut g, a, &mut q, dm)?;
            let after = g.player(alice()).unwrap().mana_pool.total();
            if !g.is_tapped(owner) || after != before + 1 {
                return Err(format!(
                    "fixture City not tapped or mana not added {before} -> {after}"
                ));
            }
            dm.trace.push(json!({"stage":"actual_city_tap","tapped":g.is_tapped(owner),"mana_added":after-before}));
        }
    } else if name == "Balduvian Berserker" {
        if mode.starts_with("buffed") {
            response(&mut g, defs, "Giant Growth", owner, &mut q, dm)?;
            before_power = g.calculated_power(owner);
            if before_power != Some(4) {
                return Err("fixture Growth power not four".into());
            }
        }
        amount = if mode.starts_with("buffed") { 4 } else { 1 };
        response(&mut g, defs, "Murder", owner, &mut q, dm)?;
        if count(&g, name, Zone::Graveyard) != 1 {
            return Err("fixture actual death absent".into());
        }
        expected_owner_zone = "graveyard";
        expected_event_zone = "graveyard";
    } else {
        amount = if mode.starts_with("lethal") || mode == "three_damage_control" {
            3
        } else {
            1
        };
        response(
            &mut g,
            defs,
            if amount == 3 {
                "Lightning Bolt"
            } else {
                "Flame Jab"
            },
            event_object,
            &mut q,
            dm,
        )?;
        if mode.starts_with("lethal") {
            expected_event_zone = "graveyard";
            if event_name == name {
                expected_owner_zone = "graveyard";
            }
        }
    }
    if mode != "untapped_control" {
        let e = g
            .stack
            .last()
            .ok_or("fixture expected damage trigger absent")?;
        if !e.is_ability {
            return Err("fixture expected triggered ability on stack".into());
        }
        if name != "City of Brass" && e.targets != dm.targets {
            return Err("fixture trigger targets not announced as requested".into());
        }
        if g.players.iter().any(|p| p.life != 20) {
            return Err("fixture damage trigger resolved before response window".into());
        }
        if !matches!(name, "City of Brass" | "Balduvian Berserker") {
            let event = e
                .triggering_event
                .as_ref()
                .and_then(|v| v.downcast::<ironsmith::events::DamageEvent>())
                .ok_or("fixture expected actual damage event")?;
            if event.amount != amount as u32
                || event.target != ironsmith::events::DamageTarget::Object(event_object)
            {
                return Err("fixture wrong actual producer amount or recipient".into());
            }
            dm.trace.push(json!({"stage":"actual_damage_producer_verified","amount":event.amount,"recipient":format!("{:?}",event.target),"target_snapshot":event.target_snapshot.as_ref().map(|s|json!({"id":s.object_id.0,"power":s.power,"zone":format!("{:?}",s.zone)}))}));
        }
        dm.trace.push(json!({"stage":"damage_trigger_before_response","owner":owner.0,"event_object":event_object.0,"event_object_power_before_event":before_power,"announced_targets":format!("{:?}",e.targets),"triggering_event":format!("{:?}",e.triggering_event),"source_snapshot":e.source_snapshot.as_ref().map(|s|json!({"id":s.object_id.0,"power":s.power,"zone":format!("{:?}",s.zone)}))}));
    } else if !g.stack.is_empty() {
        return Err("fixture unexpected untapped City trigger".into());
    }
    match mode {
        "bounce_source" | "tap_bounce" => {
            response(
                &mut g,
                defs,
                if name == "City of Brass" {
                    "Boomerang"
                } else {
                    "Unsummon"
                },
                owner,
                &mut q,
                dm,
            )?;
            expected_owner_zone = "hand";
            expected_event_zone = "hand";
        }
        "destroy_source" => {
            response(&mut g, defs, "Murder", owner, &mut q, dm)?;
            expected_owner_zone = "graveyard";
            expected_event_zone = "graveyard";
        }
        "bounce_granter" => {
            response(&mut g, defs, "Unsummon", owner, &mut q, dm)?;
            expected_owner_zone = "hand";
        }
        "bounce_damaged" => {
            response(&mut g, defs, "Unsummon", event_object, &mut q, dm)?;
            expected_event_zone = "hand";
        }
        "destroy_damaged" => {
            response(&mut g, defs, "Murder", event_object, &mut q, dm)?;
            expected_event_zone = "graveyard";
        }
        "lethal_then_exile" | "unbuffed_exile" | "buffed_exile" => {
            let id = in_zone(&g, event_name, Zone::Graveyard)?;
            response(&mut g, defs, "Cremate", id, &mut q, dm)?;
            expected_event_zone = "exile";
            if event_name == name {
                expected_owner_zone = "exile";
            }
        }
        _ => {}
    }
    if let Some(e) = g.stack.last() {
        dm.trace.push(json!({"stage":"prebound_stack_tags","tags":e.tagged_objects.iter().map(|(tag,objects)|json!({"tag":format!("{tag:?}"),"objects":objects.iter().map(|o|json!({"id":o.object_id.0,"zone":format!("{:?}",o.zone),"power":o.power,"current_zone":g.object(o.object_id).map(|v|format!("{:?}",v.zone))})).collect::<Vec<_>>()})).collect::<Vec<_>>()}));
    }
    dm.stage = "ability_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let actual = json!({"error":error,"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"owner_zones":zones(&g,name),"event_object_zones":zones(&g,event_name),"draw_witness_hand":count(&g,"Cremate draw witness",Zone::Hand)});
    let expected = json!({"error":null,"life":if name=="City of Brass"{vec![20-amount,20,20]}else{vec![20,20-amount,20]},"owner_zones":expected_zone(expected_owner_zone),"event_object_zones":expected_zone(expected_event_zone),"draw_witness_hand":usize::from(mode.ends_with("exile"))});
    Ok((expected, actual))
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn generate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let p = root.join("reports/runtime-audit");
    let input = p.join("damage-source-event-inputs.json");
    let data: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    let mut rows = vec![];
    for (name, v) in data["cards"].as_object().unwrap() {
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            v["parse_name"].as_str().unwrap_or(name),
        );
        match ironsmith_registry::compile_builder_to_artifact(
            b,
            v["parse_input"].as_str().unwrap(),
            false,
        ) {
            Ok((a, d)) => {
                artifacts.push(json!({"card":name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
                defs.insert(name.clone(), (d, a.payload_checksum));
            }
            Err(e) => rows.push(
                json!({"card":name,"status":"compile_failed","actual":{"error":e.to_string()}}),
            ),
        }
    }
    for name in [
        "Boros Reckoner",
        "Brash Taunter",
        "Spiteful Sliver",
        "City of Brass",
        "Balduvian Berserker",
    ] {
        if !defs.contains_key(name) {
            continue;
        }
        let modes = match name {
            "Boros Reckoner" => vec![
                "present",
                "bounce_source",
                "destroy_source",
                "lethal",
                "lethal_then_exile",
            ],
            "Brash Taunter" => vec!["present", "bounce_source", "three_damage_control"],
            "Spiteful Sliver" => vec![
                "present",
                "bounce_granter",
                "bounce_damaged",
                "destroy_damaged",
                "lethal",
                "lethal_then_exile",
            ],
            "City of Brass" => vec!["untapped_control", "tap", "tap_bounce"],
            _ => vec!["unbuffed", "unbuffed_exile", "buffed", "buffed_exile"],
        };
        for mode in modes {
            eprintln!("DAMAGE_SOURCE_EVENT_CASE {name} {mode}");
            let mut dm = Dm {
                targets: vec![],
                amounts: vec![],
                stage: "fixture".into(),
                trace: vec![],
                resolution_distributions: 0,
            };
            let result = run(&defs, name, mode, &mut dm);
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
            rows.push(json!({"card":name,"scenario":mode,"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs[name].1,"execution_trace":dm.trace}));
        }
    }
    let binary = std::env::current_exe().unwrap();
    let files = [
        "crates/ironsmith-engine/src/effects/composition/execute_with_source.rs",
        "crates/ironsmith-engine/src/effects/helpers/value_eval/context.rs",
        "crates/ironsmith-engine/src/effects/composition/tag_triggering_object.rs",
    ];
    let report = json!({"scope":"Actual paid damage/death/tapped-mana producers; damaged-object tag, granted-ability owner versus damaged object, and last-known-power on death. Exact opponent/controller life and source zones checked.","limitations":"Seeded mana pools and Cremate draw witness. Source creatures, Sliver Construct and all damage/removal spells actually paid and normally resolved. City played from hand and mana ability actually activated. No engine edits. Untested abilities and unexecuted typed candidates are not certified.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_damage_source_event_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("damage-source-event-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("damage-source-event-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical damage-event source-LKI expected-result reporter"]
fn report_damage_source_events() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
