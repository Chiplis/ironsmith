//! Canonical divided-damage ability audit; a passing reporter does not certify card semantics.
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
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    name: &str,
    split: &[u32],
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let mut g = game();
    let witness = CardDefinitionBuilder::new(CardId::new(), "Distribution creature witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .build();
    let creatures = (0..5)
        .map(|i| g.create_object_from_definition(&witness, PlayerId(1 + i % 2), Zone::Battlefield))
        .collect::<Vec<_>>();
    let walker = CardDefinitionBuilder::new(CardId::new(), "Distribution planeswalker witness")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(10)
        .build();
    let pw = g.create_object_from_definition(&walker, PlayerId(1), Zone::Battlefield);
    if g.counter_count(pw, CounterType::Loyalty) != 10 {
        return Err("fixture planeswalker loyalty not initialized".into());
    }
    let targets = if mode == "mixed" {
        vec![
            Target::Object(creatures[0]),
            if name == "Dragonlord Atarka" {
                Target::Object(pw)
            } else {
                Target::Player(PlayerId(1))
            },
        ]
    } else {
        creatures
            .iter()
            .take(split.len())
            .map(|id| Target::Object(*id))
            .collect()
    };
    dm.targets = targets.clone();
    dm.amounts = split.to_vec();
    let original = targets
        .iter()
        .copied()
        .zip(split.iter().copied())
        .collect::<Vec<_>>();
    let triggered = matches!(
        name,
        "Bogardan Hellkite" | "Dragonlord Atarka" | "Gandalf, Spark Starter" | "Inferno Titan"
    );
    let cost = match name {
        "Bogardan Hellkite" => 8,
        "Dragonlord Atarka" => 7,
        "Gandalf, Spark Starter" | "Inferno Titan" | "Ral, Caller of Storms" => 6,
        "Samut, the Tested" => 4,
        _ => 3,
    };
    let mut q = TriggerQueue::new();
    dm.stage = "source_cast".into();
    cast_announce(&mut g, &defs[name].0, cost, &mut q, dm)?;
    dm.stage = "source_resolution_and_trigger_announcement".into();
    one(&mut g, &mut q, dm)?;
    let source = find(&g, name)?;
    let activation_cost = if name == "Arc Mage" { 3 } else { 0 };
    let mut activation_mana_paid = 0;
    if !triggered {
        if !g.stack.is_empty() {
            return Err("fixture unexpected source ETB stack entry".into());
        }
        g.turn.turn_number += 1;
        g.turn.active_player = alice();
        g.turn.priority_player = Some(alice());
        g.turn.phase = Phase::FirstMain;
        g.turn.step = None;
        g.remove_summoning_sickness(source);
        if name == "Arc Mage" {
            let fuel = CardDefinitionBuilder::new(CardId::new(), "Arc Mage discard witness")
                .card_types(vec![CardType::Sorcery])
                .build();
            g.create_object_from_definition(&fuel, alice(), Zone::Hand);
        }
        let index = if matches!(name, "Ral, Caller of Storms" | "Samut, the Tested") {
            1
        } else {
            0
        };
        let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&*ability_index==index)).ok_or("fixture printed damage activation absent")?;
        let before = g.player(alice()).unwrap().mana_pool.total();
        dm.stage = "ability_announcement".into();
        announce(&mut g, a, &mut q, dm)?;
        activation_mana_paid = before - g.player(alice()).unwrap().mana_pool.total();
        if activation_mana_paid != activation_cost {
            return Err(format!(
                "fixture activation paid{activation_mana_paid} expected{activation_cost}"
            ));
        }
    }
    let entry = g
        .stack
        .last()
        .ok_or("fixture damage ability did not reach stack")?;
    if !entry.is_ability {
        return Err("fixture top stack entry is not ability".into());
    }
    let stored = entry
        .target_distributions
        .iter()
        .flat_map(|d| d.allocations.clone())
        .collect::<Vec<_>>();
    let announcement_matches = stored == original;
    dm.trace.push(json!({"stage":"before_any_response","targets":format!("{:?}",entry.targets),"stored_allocations":format!("{stored:?}"),"expected_allocations":format!("{original:?}"),"source_tapped":g.is_tapped(source),"source_graveyard":count(&g,name,Zone::Graveyard),"source_loyalty":g.counter_count(source,CounterType::Loyalty),"activation_mana_paid":activation_mana_paid,"discarded":count(&g,"Arc Mage discard witness",Zone::Graveyard)}));
    if mode.starts_with("remove_") {
        dm.stage = "response".into();
        dm.targets = vec![Target::Object(if mode == "remove_source" {
            source
        } else {
            creatures[0]
        })];
        dm.amounts = vec![];
        cast_announce(&mut g, &defs["Unsummon"].0, 1, &mut q, dm)?;
        one(&mut g, &mut q, dm)?;
        dm.targets = targets.clone();
        dm.amounts = split.to_vec();
    }
    dm.stage = "ability_resolution".into();
    let result = finish(&mut g, &mut q, dm);
    let error = result.err();
    let actual = json!({"error":error,"announced_distribution_matches":announcement_matches,"resolution_distribution_prompts":dm.resolution_distributions,"creature_damage":creatures.iter().map(|id|g.damage_on(*id)).collect::<Vec<_>>(),"creature_battlefield":creatures.iter().map(|id|g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).collect::<Vec<_>>(),"life":[g.player(PlayerId(1)).unwrap().life,g.player(PlayerId(2)).unwrap().life],"witness_loyalty":g.counter_count(pw,CounterType::Loyalty),"source_battlefield":count(&g,name,Zone::Battlefield),"source_graveyard":count(&g,name,Zone::Graveyard),"source_hand":count(&g,name,Zone::Hand),"source_tapped":g.is_tapped(source),"source_loyalty":g.counter_count(source,CounterType::Loyalty),"discarded":count(&g,"Arc Mage discard witness",Zone::Graveyard),"activation_mana_paid":activation_mana_paid});
    let mut damage = vec![0u32; 5];
    let mut life = vec![20i32; 2];
    let mut pw_loyalty = 10;
    for (target, amount) in original {
        if let Some(i) = creatures
            .iter()
            .position(|id| Target::Object(*id) == target)
        {
            if !matches!(mode, "remove_one" | "remove_all") || i != 0 {
                damage[i] += amount;
            }
        } else if target == Target::Object(pw) {
            pw_loyalty -= amount;
        } else if target == Target::Player(PlayerId(1)) {
            life[0] -= amount as i32;
        }
    }
    let mut on_battlefield = vec![true; 5];
    if matches!(mode, "remove_one" | "remove_all") {
        on_battlefield[0] = false;
    }
    let expected = json!({"error":null,"announced_distribution_matches":true,"resolution_distribution_prompts":0,"creature_damage":damage,"creature_battlefield":on_battlefield,"life":life,"witness_loyalty":pw_loyalty,"source_battlefield":usize::from(name!="Mogg Mob"&&mode!="remove_source"),"source_graveyard":usize::from(name=="Mogg Mob"),"source_hand":usize::from(mode=="remove_source"),"source_tapped":name=="Arc Mage"&&mode!="remove_source","source_loyalty":if matches!(name,"Ral, Caller of Storms"|"Samut, the Tested"){2}else{0},"discarded":usize::from(name=="Arc Mage"),"activation_mana_paid":activation_cost});
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
    let input = p.join("ability-damage-distribution-inputs.json");
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
        "Bogardan Hellkite",
        "Dragonlord Atarka",
        "Gandalf, Spark Starter",
        "Inferno Titan",
        "Arc Mage",
        "Mogg Mob",
        "Ral, Caller of Storms",
        "Samut, the Tested",
    ] {
        if !defs.contains_key(name) {
            continue;
        }
        let total = if matches!(name, "Bogardan Hellkite" | "Dragonlord Atarka") {
            5
        } else if matches!(name, "Arc Mage" | "Samut, the Tested") {
            2
        } else {
            3
        };
        let splits = match total {
            5 => vec![
                vec![5],
                vec![4, 1],
                vec![1, 4],
                vec![2, 2, 1],
                vec![1, 1, 1, 1, 1],
            ],
            3 => vec![vec![3], vec![2, 1], vec![1, 2], vec![1, 1, 1]],
            _ => vec![vec![2], vec![1, 1]],
        };
        let mut scenarios = splits
            .into_iter()
            .map(|s| ("stable", s))
            .collect::<Vec<_>>();
        scenarios.extend([
            ("mixed", vec![1, total - 1]),
            ("remove_one", vec![1, total - 1]),
            ("remove_all", vec![total]),
        ]);
        if !matches!(
            name,
            "Mogg Mob" | "Ral, Caller of Storms" | "Samut, the Tested"
        ) {
            scenarios.push(("remove_source", vec![1, total - 1]));
        }
        for (mode, split) in scenarios {
            eprintln!("ABILITY_DISTRIBUTION_CASE {name} {mode} {split:?}");
            let mut dm = Dm {
                targets: vec![],
                amounts: vec![],
                stage: "fixture".into(),
                trace: vec![],
                resolution_distributions: 0,
            };
            let result = run(&defs, name, &split, mode, &mut dm);
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
            rows.push(json!({"card":name,"scenario":{"distribution":split,"mode":mode},"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs[name].1,"execution_trace":dm.trace}));
        }
    }
    let binary = std::env::current_exe().unwrap();
    let runtime_source = root.join("crates/ironsmith-engine/src/game_loop/sba_triggers.rs");
    let report = json!({"scope":"Eight canonical divided-damage triggered/activated cards. Paid casts and activation costs; true ETB producers through normal priority; exact announced allocation and target-by-target results; legal paid Unsummon response removes a target or source.","limitations":"Mana and high-toughness neutral targets are seeded. Arc Mage uses an explicitly positioned later turn with summoning sickness removed. Inferno Titan tests its ETB trigger, not attack trigger. Loyalty, discard, sacrifice and mana costs are asserted. No engine changes.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_ability_damage_distribution_reproductions.rs")),"trigger_runtime_source_sha256":hash(&runtime_source),"inventory_input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("ability-damage-distribution-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("ability-damage-distribution-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical divided damage expected-result reporter"]
fn report_ability_damage_distribution() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
