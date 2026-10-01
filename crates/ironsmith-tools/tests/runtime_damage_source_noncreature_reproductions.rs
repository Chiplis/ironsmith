//! Actual land/artifact/enchantment entry damage source-removal audit; no engine edits.
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
            "Wrecking Ball" => 4,
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
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    case: &Value,
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let name = case["card"].as_str().unwrap();
    let recipients = case["recipients"].as_str().unwrap();
    let amount = case["amount"].as_i64().unwrap() as i32;
    let is_land = case["land"].as_bool().unwrap();
    let zero = mode.starts_with("zero");
    let mut g = game();
    let mut q = TriggerQueue::new();
    let ground = CardDefinitionBuilder::new(CardId::new(), "ETB ground target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .build();
    let flying = CardDefinitionBuilder::new(CardId::new(), "ETB flying target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .flying()
        .build();
    let vigilant = CardDefinitionBuilder::new(CardId::new(), "ETB vigilant flying control")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .flying()
        .vigilance()
        .build();
    let ids = [
        g.create_object_from_definition(&ground, alice(), Zone::Battlefield),
        g.create_object_from_definition(&flying, PlayerId(1), Zone::Battlefield),
        g.create_object_from_definition(&ground, PlayerId(1), Zone::Battlefield),
        g.create_object_from_definition(&vigilant, PlayerId(2), Zone::Battlefield),
    ];
    let library = CardDefinitionBuilder::new(CardId::new(), "ETB draw witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    for _ in 0..2 {
        g.create_object_from_definition(&library, alice(), Zone::Library);
    }
    let target_index = match recipients {
        "any" | "creature" | "flying_target" => Some(1),
        "own_creature" => Some(0),
        _ => None,
    };
    dm.targets = if zero {
        vec![]
    } else if let Some(i) = target_index {
        vec![Target::Object(ids[i])]
    } else if recipients == "player" {
        vec![Target::Player(PlayerId(1))]
    } else {
        vec![]
    };
    let announced_targets = dm.targets.clone();
    if is_land {
        let h = g.create_object_from_definition(&defs[name].0, alice(), Zone::Hand);
        let a = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::PlayLand{land_id,..}if *land_id==h))
            .ok_or("fixture legal land play absent")?;
        dm.stage = "actual_land_play".into();
        let mana_before = g.player(alice()).unwrap().mana_pool.total();
        immediate_action(&mut g, a, &mut q, dm)?;
        if g.player(alice()).unwrap().mana_pool.total() != mana_before {
            return Err("fixture land play unexpectedly consumed mana".into());
        }
    } else {
        dm.stage = "actual_source_paid_cast".into();
        cast_announce(
            &mut g,
            &defs[name].0,
            case["cast_cost"].as_u64().unwrap() as u32,
            &mut q,
            dm,
        )?;
        dm.stage = "normal_source_spell_resolution".into();
        one(&mut g, &mut q, dm)?;
    }
    let source = find(&g, name)?;
    if is_land && g.is_tapped(source) != case["enters_tapped"].as_bool().unwrap() {
        return Err("fixture land entry tapped state disagrees with Oracle".into());
    }
    let entry = g.stack.last().ok_or("fixture actual ETB trigger missing")?;
    if !entry.is_ability || entry.targets != announced_targets {
        return Err("fixture ETB trigger or targets do not match".into());
    }
    if g.players.iter().any(|p| p.life != 20)
        || ids.iter().any(|id| g.damage_on(*id) != 0)
        || count(&g, "ETB draw witness", Zone::Hand) != 0
    {
        return Err("fixture ETB effects ran before response window".into());
    }
    dm.trace.push(json!({"stage":"queued_actual_ETB_before_response","source":source.0,"source_snapshot":entry.source_snapshot.as_ref().map(|s|json!({"id":s.object_id.0,"power":s.power,"toughness":s.toughness})),"announced_targets":format!("{:?}",entry.targets),"oracle_expected_amount":amount,"oracle_recipient_scope":recipients,"source_tapped":g.is_tapped(source),"land_play":is_land,"zero_targets":zero}));
    match mode {
        "bounce_source" | "zero_bounce" => response(&mut g, defs, "Boomerang", source, &mut q, dm)?,
        "destroy_source" => response(
            &mut g,
            defs,
            if is_land {
                "Wrecking Ball"
            } else {
                "Disenchant"
            },
            source,
            &mut q,
            dm,
        )?,
        "remove_target" => response(
            &mut g,
            defs,
            "Unsummon",
            ids[target_index.ok_or("fixture no object target to remove")?],
            &mut q,
            dm,
        )?,
        _ => {}
    }
    if let Some(e) = g.stack.last() {
        dm.trace.push(json!({"stage":"prebound_trigger_tags","tags":e.tagged_objects.iter().map(|(tag,snapshots)|json!({"tag":format!("{tag:?}"),"snapshots":snapshots.iter().map(|s|json!({"id":s.object_id.0,"zone":format!("{:?}",s.zone),"power":s.power,"currently_live":g.object(s.object_id).is_some()})).collect::<Vec<_>>()})).collect::<Vec<_>>()}));
    }
    dm.stage = "ability_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let actual = json!({"error":error,"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"creature_damage":ids.iter().map(|id|g.damage_on(*id)).collect::<Vec<_>>(),"creature_battlefield":ids.iter().map(|id|g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).collect::<Vec<_>>(),"source_battlefield":count(&g,name,Zone::Battlefield),"source_hand":count(&g,name,Zone::Hand),"source_graveyard":count(&g,name,Zone::Graveyard),"draw_witness_hand":count(&g,"ETB draw witness",Zone::Hand)});
    let mut life = vec![20, 20, 20];
    life[0] += if mode == "remove_target" {
        0
    } else {
        case["gain_life"].as_i64().unwrap() as i32
    };
    match recipients {
        "you" => life[0] -= amount,
        "player" => life[1] -= amount,
        "opponents" => {
            life[1] -= amount;
            life[2] -= amount;
        }
        _ => {}
    }
    let damage = (0..4)
        .map(|i| {
            let receives = match recipients {
                "any" | "creature" | "flying_target" | "own_creature" => {
                    Some(i) == target_index && mode != "remove_target" && !zero
                }
                "keyword_filtered_creatures" => i < 3,
                "all_other_creatures" => true,
                "opponent_creatures" => i > 0,
                "nonflying_creatures" => i == 0 || i == 2,
                "flying_creatures" => i == 1 || i == 3,
                _ => false,
            };
            if receives { amount } else { 0 }
        })
        .collect::<Vec<_>>();
    let expected = json!({"error":null,"life":life,"creature_damage":damage,"creature_battlefield":(0..4).map(|i|!(mode=="remove_target"&&Some(i)==target_index)).collect::<Vec<_>>(),"source_battlefield":usize::from(!matches!(mode,"bounce_source"|"destroy_source"|"zero_bounce")),"source_hand":usize::from(matches!(mode,"bounce_source"|"zero_bounce")),"source_graveyard":usize::from(mode=="destroy_source"),"draw_witness_hand":case["draw"]});
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
    let input = p.join("damage-source-noncreature-inputs.json");
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
    for case in data["cases"].as_array().unwrap() {
        let name = case["card"].as_str().unwrap();
        if !defs.contains_key(name) {
            continue;
        }
        let mut modes = vec!["present", "bounce_source", "destroy_source"];
        if matches!(
            case["recipients"].as_str().unwrap(),
            "any" | "creature" | "flying_target" | "own_creature"
        ) {
            modes.push("remove_target");
        }
        if case["optional_target"].as_bool().unwrap() {
            modes.extend(["zero_present", "zero_bounce"]);
        }
        for mode in modes {
            eprintln!("DAMAGE_SOURCE_NONCREATURE {name} {mode}");
            let mut dm = Dm {
                targets: vec![],
                amounts: vec![],
                stage: "fixture".into(),
                trace: vec![],
                resolution_distributions: 0,
            };
            let result = run(&defs, case, mode, &mut dm);
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
            rows.push(json!({"card":name,"scenario":mode,"status":status,"oracle_expectation":case,"expected":expected,"actual":actual,"artifact_checksum":defs[name].1,"execution_trace":dm.trace}));
        }
    }
    let binary = std::env::current_exe().unwrap();
    let files = [
        "crates/ironsmith-engine/src/effects/composition/execute_with_source.rs",
        "crates/ironsmith-engine/src/effects/composition/tag_triggering_object.rs",
        "crates/ironsmith-engine/src/effects/damage/deal_damage.rs",
    ];
    let report = json!({"scope":"Eleven damage Deserts and25 manually Oracle-reviewed artifact/enchantment fixed-damage entry sources. Actual land play or normal paid casts, queued entry triggers, paid Boomerang/Wrecking Ball/Disenchant removal, object-target-removal and zero-target controls. Expected damage and recipient scopes come from canonical Oracle independently of IR.","limitations":"Mana, neutral2/12 ground/flying/vigilant creatures and neutral library cards seeded. Normal front-face casts only. Equipment is unattached; Fires of Mount Doom equipment destruction and The Black Arrow Dragon clause not certified. Optional targets are explicitly zero/one. No engine edits or blanket promotions.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_damage_source_noncreature_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("damage-source-noncreature-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("damage-source-noncreature-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical noncreature entry source-removal Oracle outcome reporter"]
fn report_damage_source_noncreature() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
