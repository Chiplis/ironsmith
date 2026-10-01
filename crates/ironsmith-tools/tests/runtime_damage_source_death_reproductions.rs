//! Actual fixed death/leave damage and post-death source-removal audit; no engine edits.
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
        self.trace.push(json!({"stage":self.stage,"choice":"boolean","description":c.description,"accept":if self.stage=="actual_source_paid_cast" {false}else{self.accept}}));
        if self.stage == "actual_source_paid_cast" {
            false
        } else {
            self.accept
        }
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
        let selected = if self.stage == "actual_source_paid_cast"
            && c.min == 0
            && c.description.starts_with("Choose optional costs")
        {
            vec![]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
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
fn in_grave(g: &GameState, n: &str) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.name == n && o.zone == Zone::Graveyard)
        .map(|o| o.id)
        .ok_or("fixture source graveyard card absent".into())
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let recipients = c["recipients"].as_str().unwrap();
    let amount = c["amount"].as_u64().unwrap() as u32;
    let mut g = game();
    let mut q = TriggerQueue::new();
    let ground = CardDefinitionBuilder::new(CardId::new(), "Death ground witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .build();
    let flying = CardDefinitionBuilder::new(CardId::new(), "Death flying witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .flying()
        .build();
    let ids = [
        g.create_object_from_definition(&ground, alice(), Zone::Battlefield),
        g.create_object_from_definition(&ground, PlayerId(1), Zone::Battlefield),
        g.create_object_from_definition(&flying, PlayerId(2), Zone::Battlefield),
    ];
    let draw = CardDefinitionBuilder::new(CardId::new(), "Death Cremate draw witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    g.create_object_from_definition(&draw, alice(), Zone::Library);
    dm.targets = if recipients == "creature" {
        vec![Target::Object(ids[1])]
    } else if recipients == "player" {
        vec![Target::Player(PlayerId(1))]
    } else {
        vec![]
    };
    dm.stage = "actual_source_paid_cast".into();
    cast_announce(
        &mut g,
        &defs[n].0,
        c["cast_cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    dm.stage = "source_spell_resolution".into();
    finish(&mut g, &mut q, dm)?;
    let source = find(&g, n)?;
    if count(&g, n, Zone::Battlefield) != 1 {
        return Err("fixture unexpected source copy/offspring".into());
    }
    response(
        &mut g,
        defs,
        if c["noncreature_artifact"].as_bool().unwrap() {
            "Disenchant"
        } else {
            "Murder"
        },
        source,
        &mut q,
        dm,
    )?;
    let grave = in_grave(&g, n)?;
    let e = g
        .stack
        .last()
        .ok_or("fixture actual death/leave trigger absent")?;
    if !e.is_ability || e.targets != dm.targets {
        return Err("fixture death trigger targets mismatch".into());
    }
    if g.players.iter().any(|p| p.life != 20) || ids.iter().any(|id| g.damage_on(*id) != 0) {
        return Err("fixture death damage happened before response window".into());
    }
    dm.trace.push(json!({"stage":"queued_actual_death_trigger","source_battlefield_id":source.0,"source_graveyard_id":grave.0,"announced_targets":format!("{:?}",e.targets),"source_snapshot":e.source_snapshot.as_ref().map(|s|json!({"id":s.object_id.0,"power":s.power,"toughness":s.toughness,"zone":format!("{:?}",s.zone)})),"oracle_amount":amount,"oracle_scope":recipients}));
    match mode {
        "exile_source" => response(&mut g, defs, "Cremate", grave, &mut q, dm)?,
        "remove_target" => response(&mut g, defs, "Unsummon", ids[1], &mut q, dm)?,
        _ => {}
    }
    dm.stage = "ability_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let does_damage = (!c["optional"].as_bool().unwrap() || dm.accept) && mode != "remove_target";
    let actual = json!({"error":error,"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"creature_damage":ids.iter().map(|id|g.damage_on(*id)).collect::<Vec<_>>(),"creature_battlefield":ids.iter().map(|id|g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).collect::<Vec<_>>(),"source_battlefield":count(&g,n,Zone::Battlefield),"source_graveyard":count(&g,n,Zone::Graveyard),"source_exile":count(&g,n,Zone::Exile),"clue_tokens":count(&g,"Clue",Zone::Battlefield),"draw_witness_hand":count(&g,"Death Cremate draw witness",Zone::Hand)});
    let mut life = vec![20, 20, 20];
    if mode != "remove_target" {
        life[0] += c["gain_life"].as_i64().unwrap() as i32;
    }
    if does_damage {
        match recipients {
            "you" => life[0] -= amount as i32,
            "player" => life[1] -= amount as i32,
            "opponents" => {
                life[1] -= amount as i32;
                life[2] -= amount as i32;
            }
            _ => {}
        }
    }
    let damage = (0..3)
        .map(|i| {
            if does_damage
                && match recipients {
                    "creature" => i == 1,
                    "nonlegendary_creatures" => true,
                    "nonflying_creatures" => i < 2,
                    _ => false,
                }
            {
                amount
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    let expected = json!({"error":null,"life":life,"creature_damage":damage,"creature_battlefield":[true,mode!="remove_target",true],"source_battlefield":0,"source_graveyard":usize::from(mode!="exile_source"),"source_exile":usize::from(mode=="exile_source"),"clue_tokens":c["clue_tokens"],"draw_witness_hand":usize::from(mode=="exile_source")});
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
    let input = p.join("damage-source-death-inputs.json");
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
        if !defs.contains_key(n) {
            continue;
        }
        let mut modes = vec!["graveyard_present", "exile_source"];
        if c["recipients"] == "creature" {
            modes.push("remove_target");
        }
        let accepts = if c["optional"].as_bool().unwrap() {
            vec![false, true]
        } else {
            vec![true]
        };
        for accept in accepts {
            for mode in &modes {
                eprintln!("DEATH_DAMAGE_LKI {n} {mode} accept={accept}");
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
        "crates/ironsmith-engine/src/effects/composition/tag_triggering_object.rs",
        "crates/ironsmith-engine/src/effects/damage/deal_damage.rs",
    ];
    let report = json!({"scope":"26 fixed death/leave damage sources actually cast and killed by paid Murder/Disenchant. Actual queued trigger targets checked; paid Cremate removes its post-death source object. Graveyard-present, optional decline/accept and actual-target-removal controls. Oracle-derived exact damage/lifegain and Clue counts.","limitations":"Mana, neutral2/12 nonlegendary ground/flying creatures and one draw witness seeded. Normal cast costs; offspring declined, echo/cycling/evoke/other abilities not certified. Piru expectation includes21 lifelink life for7 damage to3 creatures. Damage event identity after zone changes is not certified. No engine edits.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_damage_source_death_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("damage-source-death-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("damage-source-death-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical fixed death damage expected-result reporter"]
fn report_damage_source_death() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
