//! Actual fixed attack/block-trigger damage with paid source removal; no engine edits.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration};
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, DistributeContext, SelectObjectsContext, SelectOptionsContext,
    TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_attacker_declarations_with_dm, apply_blocker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, PowerToughness, Step,
    Subtype, Zone,
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
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let recipients = c["recipients"].as_str().unwrap();
    let producer = c["producer"].as_str().unwrap();
    let amount = c["amount"].as_u64().unwrap() as u32;
    let mut g = game();
    let mut q = TriggerQueue::new();
    let ground = CardDefinitionBuilder::new(CardId::new(), "Combat ground witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .build();
    let lifelink = CardDefinitionBuilder::new(CardId::new(), "Combat lifelink witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .lifelink()
        .build();
    let zombie = CardDefinitionBuilder::new(CardId::new(), "Combat Zombie witness")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(2, 12))
        .build();
    let flying = CardDefinitionBuilder::new(CardId::new(), "Combat flying witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .flying()
        .build();
    let ids = [
        g.create_object_from_definition(
            if c["zombie_recipient"] == true {
                &zombie
            } else if c["recipient_lifelink"] == true {
                &lifelink
            } else {
                &ground
            },
            PlayerId(1),
            Zone::Battlefield,
        ),
        g.create_object_from_definition(&flying, PlayerId(1), Zone::Battlefield),
        g.create_object_from_definition(&ground, PlayerId(2), Zone::Battlefield),
    ];
    dm.targets = if recipients == "target" {
        vec![Target::Object(ids[0])]
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
        return Err("fixture unexpected source copy".into());
    }
    g.turn.turn_number += 1;
    g.turn.active_player = if producer == "blocks" {
        PlayerId(1)
    } else {
        alice()
    };
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::DeclareAttackers);
    let attacker = if producer == "blocks" { ids[0] } else { source };
    let defender = if producer == "blocks" {
        alice()
    } else {
        PlayerId(1)
    };
    g.remove_summoning_sickness(attacker);
    g.remove_summoning_sickness(source);
    let mut combat = CombatState::default();
    dm.stage = "actual_attacker_declaration".into();
    apply_attacker_declarations_with_dm(
        &mut g,
        &mut combat,
        &mut q,
        &[AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(defender),
        }],
        dm,
    )
    .map_err(|e| e.to_string())?;
    g.combat = Some(combat);
    if producer != "attack" {
        g.turn.step = Some(Step::DeclareBlockers);
        let mut combat = g.combat.take().unwrap();
        let blocker = if producer == "blocks" { source } else { ids[0] };
        dm.stage = "actual_blocker_declaration".into();
        apply_blocker_declarations(
            &mut g,
            &mut combat,
            &mut q,
            &[BlockerDeclaration {
                blocker,
                blocking: attacker,
            }],
            defender,
        )
        .map_err(|e| e.to_string())?;
        g.combat = Some(combat);
    }
    dm.stage = "queue_actual_combat_trigger".into();
    advance_priority_with_dm(&mut g, &mut q, dm).map_err(|e| e.to_string())?;
    let e = g
        .stack
        .last()
        .ok_or("fixture actual combat trigger absent")?;
    if !e.is_ability || e.targets != dm.targets {
        return Err(format!(
            "fixture combat trigger targets mismatch: {:?}, expected {:?}",
            e.targets, dm.targets
        ));
    }
    if g.players.iter().any(|p| p.life != 20) || ids.iter().any(|id| g.damage_on(*id) != 0) {
        return Err("fixture combat-trigger damage happened before response window".into());
    }
    dm.trace.push(json!({"stage":"queued_actual_combat_trigger","producer":producer,"source_battlefield_id":source.0,"recipient_ids":ids.iter().map(|id|id.0).collect::<Vec<_>>(),"attacker":attacker.0,"defender":defender.0,"announced_targets":format!("{:?}",e.targets),"source_snapshot":e.source_snapshot.as_ref().map(|s|json!({"id":s.object_id.0,"power":s.power,"toughness":s.toughness,"zone":format!("{:?}",s.zone)})),"oracle_amount":amount,"oracle_scope":recipients}));
    match mode {
        "bounce_source" => response(&mut g, defs, "Unsummon", source, &mut q, dm)?,
        "destroy_source" => response(&mut g, defs, "Murder", source, &mut q, dm)?,
        "remove_recipient" => response(&mut g, defs, "Unsummon", ids[0], &mut q, dm)?,
        _ => {}
    }
    dm.stage = "ability_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let does_damage =
        (!c["optional"].as_bool().unwrap() || dm.accept) && mode != "remove_recipient";
    let mut source_ids = g
        .turn_store
        .turn_history
        .event_records
        .iter()
        .chain(g.turn_store.turn_history.staged_event_records.iter())
        .filter_map(|r| r.event.downcast::<ironsmith::events::DamageEvent>())
        .filter(|e| e.amount > 0)
        .map(|e| e.source.0)
        .collect::<Vec<_>>();
    source_ids.sort();
    source_ids.dedup();
    let mut source_names = source_ids
        .iter()
        .map(|id| {
            g.object(ObjectId(*id))
                .map(|o| o.name.to_string())
                .unwrap_or_else(|| "unavailable_source_identity".into())
        })
        .collect::<Vec<_>>();
    source_names.sort();
    source_names.dedup();
    dm.trace.push(json!({"stage":"resolved_damage_source_identity_observation","object_ids":source_ids,"names":source_names,"comparison":"Cross-zone object IDs recorded but not independently treated as rules defects; compare source card identity and observable outcomes."}));
    let actual = json!({"error":error,"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"creature_damage":ids.iter().map(|id|g.damage_on(*id)).collect::<Vec<_>>(),"creature_battlefield":ids.iter().map(|id|g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).collect::<Vec<_>>(),"source_battlefield":count(&g,n,Zone::Battlefield),"source_graveyard":count(&g,n,Zone::Graveyard),"source_hand":count(&g,n,Zone::Hand),"damage_event_source_names":source_names});
    let mut life = vec![20, 20, 20];
    if mode != "remove_recipient" {
        life[0] += c["gain_life"].as_i64().unwrap() as i32;
    }
    if does_damage {
        match recipients {
            "defender" => life[1] -= amount as i32,
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
                    "target" | "blockers" | "defender_nonflying" => i == 0,
                    "defender_creatures" => i < 2,
                    _ => false,
                }
            {
                amount
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    let expected = json!({"error":null,"life":life,"creature_damage":damage,"creature_battlefield":[mode!="remove_recipient",true,true],"source_battlefield":usize::from(mode=="source_present"||mode=="remove_recipient"),"source_graveyard":usize::from(mode=="destroy_source"),"source_hand":usize::from(mode=="bounce_source"),"damage_event_source_names":if does_damage{vec![n]}else{vec![]}});
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
    let input = p.join("damage-source-combat-inputs.json");
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
        let mut modes = vec!["source_present", "bounce_source", "destroy_source"];
        if c["recipients"] == "target" || c["recipients"] == "blockers" {
            modes.push("remove_recipient");
        }
        let accepts = if c["optional"].as_bool().unwrap() {
            vec![false, true]
        } else {
            vec![true]
        };
        for accept in accepts {
            for mode in &modes {
                eprintln!("COMBAT_DAMAGE_LKI {n} {mode} accept={accept}");
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
                rows.push(json!({"card":n,"scenario":{"mode":mode,"accept_optional":accept,"producer":c["producer"],"zombie_recipient":c["zombie_recipient"],"recipient_lifelink":c["recipient_lifelink"]},"status":status,"oracle_expectation":c,"expected":expected,"actual":actual,"artifact_checksum":defs[n].1,"execution_trace":dm.trace}));
            }
        }
    }
    let binary = std::env::current_exe().unwrap();
    let files = [
        "crates/ironsmith-engine/src/effects/composition/execute_with_source.rs",
        "crates/ironsmith-engine/src/effects/composition/tag_triggering_object.rs",
        "crates/ironsmith-engine/src/effects/damage/deal_damage.rs",
    ];
    let report = json!({"scope":"16 fixed attack/block damage sources actually cast, then real combat declarations produce queued abilities. Paid Unsummon/Murder removes source before resolution. Source-present, recipient-removal, optional decline/accept and Purifying Dragon Zombie/non-Zombie controls; independently expected exact damage, gain and damage-event source card identity. Acolyte lifelink-blocker witness verifies gameplay consequences of wrong source role.","limitations":"Mana and neutral2/12 creatures seeded. Turn positioned at combat after paid cast with summoning sickness cleared. Only queued stack resolves; combat damage and stale combat-member cleanup are outside scope. No engine edits.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_damage_source_combat_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("damage-source-combat-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("damage-source-combat-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical fixed combat-trigger damage expected-result reporter"]
fn report_damage_source_combat() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
