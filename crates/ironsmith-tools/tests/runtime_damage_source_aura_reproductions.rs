//! Actual Aura attachment, damage-source attribution and optional entry damage source-LKI audit; no engine edits.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, DistributeContext, SelectObjectsContext, SelectOptionsContext,
    TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
struct Dm {
    accept: bool,
    targets: Vec<Target>,
    amounts: Vec<u32>,
    stage: String,
    trace: Vec<Value>,
    resolution_distributions: usize,
}
impl DecisionMaker for Dm {
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace.push(json!({"stage":self.stage,"choice":"boolean","description":c.description,"accept":self.accept}));
        self.accept
    }
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
fn response(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    spell: &str,
    target: ObjectId,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    let saved = dm.targets.clone();
    dm.stage = format!("response_announce_{spell}");
    dm.targets = vec![Target::Object(target)];
    cast_announce(
        g,
        &defs[spell].0,
        match spell {
            "Murder" => 3,
            "Boomerang" | "Disenchant" => 2,
            _ => 1,
        },
        q,
        dm,
    )?;
    dm.targets = saved;
    dm.stage = format!("response_resolve_{spell}");
    one(g, q, dm)
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let name = c["card"].as_str().unwrap();
    let aura = c["aura"].as_bool().unwrap();
    let targeted = c["trigger_target"].as_bool().unwrap();
    let optional = c["optional"].as_bool().unwrap();
    let amount = c["amount"].as_u64().unwrap() as u32;
    let mut g = game();
    let mut q = TriggerQueue::new();
    let hostdef = CardDefinitionBuilder::new(CardId::new(), "Aura attachment host")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .build();
    let host = g.create_object_from_definition(&hostdef, alice(), Zone::Battlefield);
    let victimdef = CardDefinitionBuilder::new(CardId::new(), "Separate flying damage target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .flying()
        .build();
    let victim = g.create_object_from_definition(&victimdef, PlayerId(1), Zone::Battlefield);
    let draw = CardDefinitionBuilder::new(CardId::new(), "Optional draw witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    g.create_object_from_definition(&draw, alice(), Zone::Library);
    dm.targets = if aura {
        vec![Target::Object(host)]
    } else {
        vec![]
    };
    dm.stage = "actual_source_paid_cast".into();
    cast_announce(
        &mut g,
        &defs[name].0,
        c["cast_cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    if aura && g.stack.last().unwrap().targets != vec![Target::Object(host)] {
        return Err("fixture Aura cast target not attachment host".into());
    }
    dm.targets = if targeted {
        vec![Target::Object(victim)]
    } else {
        vec![]
    };
    dm.stage = "normal_source_resolution".into();
    one(&mut g, &mut q, dm)?;
    let source = find(&g, name)?;
    if aura
        && g.object(source).unwrap().attached_to
            != Some(ironsmith::object::AttachmentTarget::Object(host))
    {
        return Err("fixture Aura not actually attached to declared host".into());
    }
    let e = g.stack.last().ok_or("fixture ETB trigger missing")?;
    if !e.is_ability || e.targets != dm.targets {
        return Err("fixture ETB target declaration mismatch".into());
    }
    if g.damage_on(host) != 0
        || g.damage_on(victim) != 0
        || count(&g, "Optional draw witness", Zone::Hand) != 0
    {
        return Err("fixture entry effect happened before response".into());
    }
    dm.trace.push(json!({"stage":"before_response","source":source.0,"aura_attachment":g.object(source).unwrap().attached_to.map(|t|format!("{t:?}")),"host":host.0,"separate_damage_target":victim.0,"announced_trigger_targets":format!("{:?}",e.targets),"optional_accept":dm.accept}));
    match mode {
        "bounce_source" => response(&mut g, defs, "Boomerang", source, &mut q, dm)?,
        "destroy_source" => response(
            &mut g,
            defs,
            if aura { "Disenchant" } else { "Murder" },
            source,
            &mut q,
            dm,
        )?,
        "bounce_host" => response(&mut g, defs, "Unsummon", host, &mut q, dm)?,
        "remove_damage_target" => response(&mut g, defs, "Unsummon", victim, &mut q, dm)?,
        _ => {}
    }
    dm.stage = "ability_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let source_present = !matches!(mode, "bounce_source" | "destroy_source" | "bounce_host");
    let receives = (!optional || dm.accept)
        && mode != "remove_damage_target"
        && (targeted || mode != "bounce_host");
    let mut damage_sources=g.turn_store.turn_history.event_records.iter().chain(g.turn_store.turn_history.staged_event_records.iter()).filter_map(|r|r.event.downcast::<ironsmith::events::DamageEvent>()).filter(|e|e.amount>0 && matches!(e.target,ironsmith::events::DamageTarget::Object(id)if id==host||id==victim)).map(|e|e.source.0).collect::<Vec<_>>();
    damage_sources.sort();
    damage_sources.dedup();
    let actual = json!({"error":error,"damage_source_ids":damage_sources,"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"host_damage":g.damage_on(host),"separate_target_damage":g.damage_on(victim),"host_battlefield":g.object(host).is_some_and(|o|o.zone==Zone::Battlefield),"separate_target_battlefield":g.object(victim).is_some_and(|o|o.zone==Zone::Battlefield),"source_battlefield":count(&g,name,Zone::Battlefield),"source_hand":count(&g,name,Zone::Hand),"source_graveyard":count(&g,name,Zone::Graveyard),"aura_attached_to_host":g.object(source).is_some_and(|o|o.attached_to==Some(ironsmith::object::AttachmentTarget::Object(host))),"draw_witness_hand":count(&g,"Optional draw witness",Zone::Hand)});
    let expected = json!({"error":null,"damage_source_ids":if receives{vec![source.0]}else{vec![]},"life":[20,20,20],"host_damage":if receives&&!targeted{amount}else{0},"separate_target_damage":if receives&&targeted{amount}else{0},"host_battlefield":mode!="bounce_host","separate_target_battlefield":mode!="remove_damage_target","source_battlefield":usize::from(source_present),"source_hand":usize::from(mode=="bounce_source"),"source_graveyard":usize::from(matches!(mode,"destroy_source"|"bounce_host")),"aura_attached_to_host":aura&&source_present,"draw_witness_hand":usize::from(name=="Wicked Guardian"&&dm.accept)});
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
    let input = p.join("damage-source-aura-inputs.json");
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
    for c in data["cases"].as_array().unwrap() {
        let n = c["card"].as_str().unwrap();
        if !defs.contains_key(n) {
            continue;
        }
        let mut modes = vec!["present", "bounce_source", "destroy_source"];
        if c["aura"].as_bool().unwrap() {
            modes.push("bounce_host");
        }
        if c["trigger_target"].as_bool().unwrap() {
            modes.push("remove_damage_target");
        }
        let accepts = if c["optional"].as_bool().unwrap() {
            vec![false, true]
        } else {
            vec![true]
        };
        for accept in accepts {
            for mode in &modes {
                eprintln!("AURA_DAMAGE_LKI {n} {mode} accept={accept}");
                let mut dm = Dm {
                    accept,
                    targets: vec![],
                    amounts: vec![],
                    stage: "fixture".into(),
                    trace: vec![],
                    resolution_distributions: 0,
                };
                let result = run(&defs, c, mode, &mut dm);
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
                rows.push(json!({"card":n,"scenario":{"mode":mode,"accept_optional":accept},"status":status,"oracle_expectation":c,"expected":expected,"actual":actual,"artifact_checksum":defs[n].1,"execution_trace":dm.trace}));
            }
        }
    }
    let binary = std::env::current_exe().unwrap();
    let files = [
        "crates/ironsmith-engine/src/effects/composition/execute_with_source.rs",
        "crates/ironsmith-engine/src/effects/composition/tag_attached_to_source.rs",
        "crates/ironsmith-engine/src/effects/composition/tag_triggering_object.rs",
    ];
    let report = json!({"scope":"Five actually paid Aura casts with verified attachment host independent from the ETB target; two optional ETB damage creatures with explicit decline/accept. Paid source bounce/destroy, Aura host-removal SBAs, and separate damage-target-removal controls.","limitations":"Neutral2/12 host/flying recipient, mana and optional draw witness seeded. Expected amounts and optionality manually taken from canonical Oracle text, not compiled IR. Damage event source identity checked independently from marked damage. Other abilities/upkeeps not certified. No engine edits.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_damage_source_aura_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("damage-source-aura-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("damage-source-aura-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical Aura and optional entry damage expected-result reporter"]
fn report_damage_source_aura() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
