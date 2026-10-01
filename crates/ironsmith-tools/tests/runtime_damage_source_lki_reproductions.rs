//! Actual-producer source-LKI damage-family audit; no engine edits.
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
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, Phase, PlayerId,
    PowerToughness, Zone,
};
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
fn replenish(g: &mut GameState) {
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
        if spell == "Murder" { 3 } else { 1 },
        q,
        dm,
    )?;
    dm.targets = saved;
    dm.amounts = amounts;
    dm.stage = format!("response_resolve_{spell}");
    one(g, q, dm)
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    name: &str,
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let mut g = game();
    let victim = CardDefinitionBuilder::new(CardId::new(), "LKI damage target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .flying()
        .build();
    let targets = (0..2)
        .map(|i| g.create_object_from_definition(&victim, PlayerId(i + 1), Zone::Battlefield))
        .collect::<Vec<_>>();
    let walker = CardDefinitionBuilder::new(CardId::new(), "LKI planeswalker witness")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(10)
        .build();
    let pw = g.create_object_from_definition(&walker, PlayerId(1), Zone::Battlefield);
    if g.counter_count(pw, CounterType::Loyalty) != 10 {
        return Err("fixture planeswalker loyalty not initialized".into());
    }
    let library = CardDefinitionBuilder::new(CardId::new(), "Cremate draw witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    g.create_object_from_definition(&library, alice(), Zone::Library);
    if name == "Ureni, the Song Unending" {
        for _ in 0..3 {
            let land = CardDefinitionBuilder::new(CardId::new(), "Ureni count land")
                .card_types(vec![CardType::Land])
                .build();
            g.create_object_from_definition(&land, alice(), Zone::Battlefield);
        }
    }
    let distributed = matches!(name, "Fury" | "Ureni, the Song Unending" | "Gang of Devils");
    let total = if name == "Fury" { 4 } else { 3 };
    dm.targets = if name == "Goblin Chainwhirler" {
        vec![]
    } else if distributed {
        targets.iter().copied().map(Target::Object).collect()
    } else {
        vec![Target::Object(targets[0])]
    };
    dm.amounts = if distributed {
        vec![1, total - 1]
    } else {
        vec![]
    };
    let cost = match name {
        "Flametongue Kavu" => 4,
        "Chainweb Aracnir" => 1,
        "Fury" | "Scourge of Valkas" | "Terror of the Peaks" => 5,
        "Ureni, the Song Unending" => 8,
        "Gang of Devils" => 6,
        "Goblin Chainwhirler" => 3,
        _ => return Err("unknown fixture card".into()),
    };
    let mut q = TriggerQueue::new();
    dm.stage = "source_cast".into();
    cast_announce(&mut g, &defs[name].0, cost, &mut q, dm)?;
    dm.stage = "actual_source_ETB".into();
    one(&mut g, &mut q, dm)?;
    let owner = find(&g, name)?;
    let mut event_object = owner;
    if matches!(name, "Scourge of Valkas" | "Terror of the Peaks") {
        finish(&mut g, &mut q, dm)?;
        if name == "Scourge of Valkas" && g.damage_on(targets[0]) != 1 {
            return Err("fixture Scourge own ETB did not deal one damage".into());
        }
        ironsmith::turn::execute_cleanup_step(&mut g);
        g.turn.turn_number += 1;
        g.turn.phase = Phase::FirstMain;
        g.turn.step = None;
        g.turn.active_player = alice();
        g.turn.priority_player = Some(alice());
        replenish(&mut g);
        let entering = if name == "Scourge of Valkas" {
            "Dragon Hatchling"
        } else {
            "Lantern Kami"
        };
        dm.stage = "event_object_cast".into();
        cast_announce(
            &mut g,
            &defs[entering].0,
            if entering == "Dragon Hatchling" { 2 } else { 1 },
            &mut q,
            dm,
        )?;
        dm.stage = "actual_event_object_ETB".into();
        one(&mut g, &mut q, dm)?;
        event_object = find(&g, entering)?;
    }
    if name == "Gang of Devils" {
        if !g.stack.is_empty() {
            return Err("fixture unexpected pre-death stack".into());
        }
        response(&mut g, defs, "Murder", owner, &mut q, dm)?;
        if count(&g, name, Zone::Graveyard) != 1 {
            return Err("fixture source death failed".into());
        }
    }
    let entry = g.stack.last().ok_or("fixture damage trigger absent")?;
    if !entry.is_ability {
        return Err("fixture expected damage trigger on top".into());
    }
    let stored = entry
        .target_distributions
        .iter()
        .flat_map(|d| d.allocations.clone())
        .collect::<Vec<_>>();
    let announced = if distributed {
        stored
            == dm
                .targets
                .iter()
                .copied()
                .zip(dm.amounts.iter().copied())
                .collect::<Vec<_>>()
    } else {
        true
    };
    dm.trace.push(json!({"stage":"damage_trigger_before_response","ability_source":entry.object_id.0,"damage_source_event_object":event_object.0,"announced_targets":format!("{:?}",entry.targets),"allocations":format!("{stored:?}"),"source_snapshot":entry.source_snapshot.as_ref().map(|s|json!({"id":s.object_id.0,"zone":format!("{:?}",s.zone),"power":s.power,"toughness":s.toughness}))}));
    if mode.contains("buff") {
        response(&mut g, defs, "Giant Growth", owner, &mut q, dm)?;
        if g.calculated_power(owner) != Some(4) {
            return Err("fixture Chainweb buff did not produce power4".into());
        }
        dm.trace.push(
            json!({"stage":"actual_source_power_after_growth","power":g.calculated_power(owner)}),
        );
    }
    match mode {
        "bounce_owner" | "buff_then_bounce" => {
            response(&mut g, defs, "Unsummon", owner, &mut q, dm)?
        }
        "destroy_owner" => response(
            &mut g,
            defs,
            if name == "Ureni, the Song Unending" {
                "Pongify"
            } else {
                "Murder"
            },
            owner,
            &mut q,
            dm,
        )?,
        "bounce_event_object" => response(&mut g, defs, "Unsummon", event_object, &mut q, dm)?,
        "destroy_event_object" => response(&mut g, defs, "Murder", event_object, &mut q, dm)?,
        "exile_dead_source" => {
            let grave = in_zone(&g, name, Zone::Graveyard)?;
            response(&mut g, defs, "Cremate", grave, &mut q, dm)?;
        }
        "remove_target" => response(&mut g, defs, "Unsummon", targets[0], &mut q, dm)?,
        _ => {}
    }
    if let Some(event) = g
        .stack
        .last()
        .and_then(|e| e.triggering_event.as_ref())
        .and_then(|e| e.downcast::<ironsmith::events::zones::ZoneChangeEvent>())
    {
        dm.trace.push(json!({"stage":"actual_trigger_event_before_resolution","from":format!("{:?}",event.from),"to":format!("{:?}",event.to),"event_objects":event.objects.iter().map(|id|id.0).collect::<Vec<_>>(),"explicit_result_objects":event.result_objects.iter().map(|id|json!({"id":id.0,"current_zone":g.object(*id).map(|o|format!("{:?}",o.zone))})).collect::<Vec<_>>(),"event_snapshot":event.snapshot.as_ref().map(|v|json!({"id":v.object_id.0,"zone":format!("{:?}",v.zone),"power":v.power}))}));
    }
    if let Some(entry) = g.stack.last() {
        dm.trace.push(json!({"stage":"prebound_stack_tags","tags":entry.tagged_objects.iter().map(|(tag,objects)|json!({"tag":format!("{tag:?}"),"objects":objects.iter().map(|o|json!({"id":o.object_id.0,"zone":format!("{:?}",o.zone),"power":o.power,"current_zone":g.object(o.object_id).map(|v|format!("{:?}",v.zone))})).collect::<Vec<_>>()})).collect::<Vec<_>>()}));
    }
    dm.stage = "ability_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let event_name = if name == "Scourge of Valkas" {
        "Dragon Hatchling"
    } else if name == "Terror of the Peaks" {
        "Lantern Kami"
    } else {
        name
    };
    let actual = json!({"error":error,"announced_distribution_matches":announced,"resolution_distribution_prompts":dm.resolution_distributions,"target_damage":targets.iter().map(|id|g.damage_on(*id)).collect::<Vec<_>>(),"target_battlefield":targets.iter().map(|id|g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).collect::<Vec<_>>(),"opponent_life":[g.player(PlayerId(1)).unwrap().life,g.player(PlayerId(2)).unwrap().life],"planeswalker_loyalty":g.counter_count(pw,CounterType::Loyalty),"owner_battlefield":count(&g,name,Zone::Battlefield),"owner_hand":count(&g,name,Zone::Hand),"owner_graveyard":count(&g,name,Zone::Graveyard),"owner_exile":count(&g,name,Zone::Exile),"event_object_battlefield":count(&g,event_name,Zone::Battlefield),"draw_witness_hand":count(&g,"Cremate draw witness",Zone::Hand)});
    let chain = name == "Goblin Chainwhirler";
    let removed_owner = matches!(mode, "bounce_owner" | "buff_then_bounce" | "destroy_owner");
    let bounced_owner = matches!(mode, "bounce_owner" | "buff_then_bounce");
    let removed_event = matches!(mode, "bounce_event_object" | "destroy_event_object");
    let death = name == "Gang of Devils";
    let mut damage = if distributed {
        vec![1, total - 1]
    } else if chain {
        vec![1, 1]
    } else {
        vec![
            match name {
                "Flametongue Kavu" => 4,
                "Chainweb Aracnir" => {
                    if mode.contains("buff") {
                        4
                    } else {
                        1
                    }
                }
                "Scourge of Valkas" => {
                    if removed_owner || removed_event {
                        1
                    } else {
                        2
                    }
                }
                "Terror of the Peaks" => 1,
                _ => 0,
            },
            0,
        ]
    };
    if mode == "remove_target" {
        damage[0] = 0;
    }
    let expected = json!({"error":null,"announced_distribution_matches":true,"resolution_distribution_prompts":0,"target_damage":damage,"target_battlefield":[mode!="remove_target",true],"opponent_life":if chain{vec![19,19]}else{vec![20,20]},"planeswalker_loyalty":if chain{9}else{10},"owner_battlefield":usize::from(!removed_owner&&!death),"owner_hand":usize::from(bounced_owner),"owner_graveyard":usize::from(mode=="destroy_owner"||(death&&mode!="exile_dead_source")),"owner_exile":usize::from(mode=="exile_dead_source"),"event_object_battlefield":usize::from(if event_name==name{!removed_owner&&!death}else{!removed_event}),"draw_witness_hand":usize::from(mode=="exile_dead_source")});
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
    let input = p.join("damage-source-lki-inputs.json");
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
        "Flametongue Kavu",
        "Chainweb Aracnir",
        "Fury",
        "Ureni, the Song Unending",
        "Gang of Devils",
        "Scourge of Valkas",
        "Terror of the Peaks",
        "Goblin Chainwhirler",
    ] {
        if !defs.contains_key(name) {
            continue;
        }
        let modes = if name == "Gang of Devils" {
            vec!["present", "exile_dead_source", "remove_target"]
        } else if matches!(name, "Scourge of Valkas" | "Terror of the Peaks") {
            vec![
                "present",
                "bounce_owner",
                "bounce_event_object",
                "destroy_event_object",
                "remove_target",
            ]
        } else if name == "Chainweb Aracnir" {
            vec![
                "present",
                "bounce_owner",
                "destroy_owner",
                "remove_target",
                "buff_present",
                "buff_then_bounce",
            ]
        } else {
            vec!["present", "bounce_owner", "destroy_owner", "remove_target"]
        };
        for mode in modes {
            eprintln!("DAMAGE_LKI_CASE {name} {mode}");
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
        "crates/ironsmith-engine/src/effects/composition/tag_triggering_object.rs",
        "crates/ironsmith-engine/src/effects/composition/execute_with_source.rs",
        "crates/ironsmith-engine/src/effects/damage/deal_distributed_damage.rs",
        "crates/ironsmith-engine/src/effects/helpers.rs",
    ];
    let report = json!({"scope":"Actual paid source spells and real ETB/death producers; source-present, bounce/destroy, separate entering-object vs trigger-owner, dynamic source-power and target-removal controls for eight typed-source representatives.","limitations":"Mana, high-toughness flying targets, planeswalker and Ureni's three counting lands are seeded. Scourge setup trigger is resolved then an actual cleanup clears damage before a paid Dragon Hatchling cast. Chainweb uses normal cast, Fury uses normal cast; escape/evoke not certified. No engine edits. Candidate scan is separate and does not promote unexecuted matches.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_damage_source_lki_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("damage-source-lki-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("damage-source-lki-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical source-LKI expected-result audit reporter"]
fn report_damage_source_lki() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
