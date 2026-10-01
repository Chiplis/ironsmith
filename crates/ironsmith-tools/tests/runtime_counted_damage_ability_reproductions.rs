//! Canonical counted-target ability audit; no engine edits.
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
    payload: &str,
    number: usize,
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let mut g = game();
    let name = defs[payload].0.name();
    let victim = CardDefinitionBuilder::new(CardId::new(), "Counted damage recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .build();
    let targets = (0..2)
        .map(|i| g.create_object_from_definition(&victim, PlayerId(i + 1), Zone::Battlefield))
        .collect::<Vec<_>>();
    let mut chosen = targets
        .iter()
        .take(number)
        .copied()
        .map(Target::Object)
        .collect::<Vec<_>>();
    if mode == "reverse" {
        chosen.reverse();
    }
    dm.targets = chosen.clone();
    dm.amounts = vec![];
    let mut q = TriggerQueue::new();
    let cost = match name {
        "Fire Shrine Keeper" => 1,
        "Rowan, Fearless Sparkmage" => 5,
        "Smoldering Werewolf" => 4,
        "Summon: Kujata" => 6,
        _ => return Err(format!("fixture unexpected source name {name}")),
    };
    dm.stage = "source_cast".into();
    cast_announce(&mut g, &defs[payload].0, cost, &mut q, dm)?;
    dm.stage = "source_resolution_and_actual_entry_chapter".into();
    one(&mut g, &mut q, dm)?;
    let source = find(&g, name)?;
    let lore = g.counter_count(source, CounterType::Lore);
    if name == "Summon: Kujata" && lore != 1 {
        return Err(format!(
            "fixture actual saga entry expected lore1 got{lore}"
        ));
    }
    let activation_cost = if name == "Fire Shrine Keeper" { 8 } else { 0 };
    let mut mana_paid = 0;
    let mut tap_paid = false;
    if matches!(name, "Fire Shrine Keeper" | "Rowan, Fearless Sparkmage") {
        if !g.stack.is_empty() {
            return Err("fixture source unexpectedly has pending entry ability".into());
        }
        g.turn.turn_number += 1;
        g.turn.phase = Phase::FirstMain;
        g.turn.step = None;
        g.turn.priority_player = Some(alice());
        g.remove_summoning_sickness(source);
        dm.stage = "actual_activation_announcement".into();
        let a=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,ability_index:1}if *id==source)).ok_or("fixture intended damage activation absent")?;
        let before = g.player(alice()).unwrap().mana_pool.total();
        announce(&mut g, a, &mut q, dm)?;
        mana_paid = before - g.player(alice()).unwrap().mana_pool.total();
        if mana_paid != activation_cost {
            return Err(format!(
                "fixture activation paid{mana_paid} expected{activation_cost}"
            ));
        }
        tap_paid = g.stack.last().is_some_and(|e| e.activation_cost_has_tap);
        if name == "Fire Shrine Keeper" && (!tap_paid || count(&g, name, Zone::Graveyard) != 1) {
            return Err("fixture Keeper tap/sacrifice payment incomplete".into());
        }
    }
    let entry = g
        .stack
        .last()
        .ok_or("fixture ability did not reach stack")?;
    if !entry.is_ability {
        return Err("fixture top of stack is not damage ability".into());
    }
    let announced = entry.targets == chosen;
    dm.trace.push(json!({"stage":"before_any_response","announced_targets":format!("{:?}",entry.targets),"expected_targets":format!("{chosen:?}"),"activation_paid":mana_paid,"activation_cost_has_tap":tap_paid,"source_graveyard":count(&g,name,Zone::Graveyard),"source_loyalty":g.counter_count(source,CounterType::Loyalty),"lore":lore}));
    let removed = if mode == "remove_first" {
        Some(0)
    } else if mode == "remove_second" {
        Some(1)
    } else {
        None
    };
    if let Some(i) = removed {
        dm.stage = "response_announce".into();
        dm.targets = vec![Target::Object(targets[i])];
        cast_announce(&mut g, &defs["Unsummon"].0, 1, &mut q, dm)?;
        dm.targets = chosen.clone();
        dm.stage = "response_resolution".into();
        one(&mut g, &mut q, dm)?;
    }
    dm.stage = "ability_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let actual = json!({"error":error,"announced_targets_match":announced,"damage":targets.iter().map(|id|g.damage_on(*id)).collect::<Vec<_>>(),"target_battlefield":targets.iter().map(|id|g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).collect::<Vec<_>>(),"source_battlefield":count(&g,name,Zone::Battlefield),"source_graveyard":count(&g,name,Zone::Graveyard),"source_loyalty":g.counter_count(source,CounterType::Loyalty),"source_lore":g.counter_count(source,CounterType::Lore),"activation_mana_paid":mana_paid,"tap_cost_paid":tap_paid,"distribution_callbacks":dm.trace.iter().filter(|v|v["choice"]=="distribution").count()});
    let amount = if matches!(name, "Fire Shrine Keeper" | "Summon: Kujata") {
        3
    } else {
        1
    };
    let expected = json!({"error":null,"announced_targets_match":true,"damage":(0..2).map(|i|if i<number&&Some(i)!=removed{amount}else{0}).collect::<Vec<_>>(),"target_battlefield":(0..2).map(|i|Some(i)!=removed).collect::<Vec<_>>(),"source_battlefield":usize::from(name!="Fire Shrine Keeper"),"source_graveyard":usize::from(name=="Fire Shrine Keeper"),"source_loyalty":if name=="Rowan, Fearless Sparkmage"{3}else{0},"source_lore":if name=="Summon: Kujata"{1}else{0},"activation_mana_paid":activation_cost,"tap_cost_paid":name=="Fire Shrine Keeper","distribution_callbacks":0});
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
    let input = p.join("counted-damage-ability-inputs.json");
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
    for payload in [
        "Fire Shrine Keeper",
        "Rowan, Fearless Sparkmage",
        "Smoldering Werewolf",
        "Smoldering Werewolf // Erupting Dreadwolf",
        "Summon: Kujata",
    ] {
        if !defs.contains_key(payload) {
            continue;
        }
        for (number, mode) in [
            (0, "stable"),
            (1, "stable"),
            (2, "stable"),
            (2, "reverse"),
            (2, "remove_first"),
            (2, "remove_second"),
        ] {
            eprintln!("COUNTED_DAMAGE_CASE {payload} {number} {mode}");
            let mut dm = Dm {
                targets: vec![],
                amounts: vec![],
                stage: "fixture".into(),
                trace: vec![],
                resolution_distributions: 0,
            };
            let result = run(&defs, payload, number, mode, &mut dm);
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
            rows.push(json!({"card":if payload.contains(" // "){"Smoldering Werewolf"}else{payload},"payload_name":payload,"scenario":{"target_count":number,"mode":mode},"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs[payload].1,"execution_trace":dm.trace}));
        }
    }
    let binary = std::env::current_exe().unwrap();
    let report = json!({"scope":"Four counted-target damage ability cards plus exact combined Smoldering Werewolf payload. Paid casts, actual ETB and entry chapter1, actual mana/tap/sacrifice/loyalty activation costs; zero/one/two targets, reversed order, either target removed through paid Unsummon.","limitations":"Mana and neutral high-toughness target creatures seeded. Later turn positioned explicitly for activated abilities. Rowan's cannot-block clause and later Kujata chapters are not certified. Combined Smoldering payload is executed separately but attributed to the same card, not a fifth name. No engine edits.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_counted_damage_ability_reproductions.rs")),"damage_runtime_sha256":hash(&root.join("crates/ironsmith-engine/src/effects/damage/deal_damage.rs")),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("counted-damage-ability-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("counted-damage-ability-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical counted-target damage expected-result reporter"]
fn report_counted_damage_abilities() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
