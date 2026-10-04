//! Actual Flitterwing counter-removal cost and delayed combat-draw audit; no engine edits.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, CountersContext, DecisionContext, DistributeContext, SelectObjectsContext,
    SelectOptionsContext, TargetsContext,
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
    CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, Phase, PlayerId,
    PowerToughness, Step, Zone,
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
    stage: String,
    trace: Vec<Value>,
    counter_objects: Vec<ObjectId>,
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
    fn decide_distribute(&mut self, g: &GameState, c: &DistributeContext) -> Vec<(Target, u32)> {
        let mut needed = c.total;
        let mut chosen = vec![];
        for id in &self.counter_objects {
            if c.targets.iter().any(|t| t.target == Target::Object(*id)) {
                let available = g
                    .object(*id)
                    .map(|o| o.counters.values().copied().sum::<u32>())
                    .unwrap_or(0);
                let take = available.min(needed);
                if take > 0 {
                    chosen.push((Target::Object(*id), take));
                    needed -= take;
                }
            }
        }
        self.trace.push(json!({"stage":self.stage,"choice":"counter_object_distribution","context":format!("{c:?}"),"selected":format!("{chosen:?}"),"unavailable_required":needed}));
        chosen
    }
    fn decide_counters(&mut self, _: &GameState, c: &CountersContext) -> Vec<(CounterType, u32)> {
        let mut needed = c.max_total;
        let mut chosen = vec![];
        for (kind, n) in &c.available_counters {
            let take = u32::try_from(needed.min(u64::from(*n))).unwrap();
            if take > 0 {
                chosen.push((*kind, take));
                needed -= u64::from(take);
            }
        }
        self.trace.push(json!({"stage":self.stage,"choice":"counter_kinds","context":format!("{c:?}"),"selected":format!("{chosen:?}"),"unavailable_required":needed}));
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
    mode: &str,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = "Flitterwing Nuisance";
    let mut g = game();
    let mut q = TriggerQueue::new();
    let creature = CardDefinitionBuilder::new(CardId::new(), "Flitterwing combat witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 12))
        .build();
    let first = g.create_object_from_definition(&creature, alice(), Zone::Battlefield);
    let second = g.create_object_from_definition(&creature, alice(), Zone::Battlefield);
    let other = g.create_object_from_definition(&creature, PlayerId(1), Zone::Battlefield);
    let pw_def = CardDefinitionBuilder::new(CardId::new(), "Flitterwing planeswalker witness")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(10)
        .build();
    let pw = g.create_object_from_definition(&pw_def, PlayerId(1), Zone::Battlefield);
    let draw = CardDefinitionBuilder::new(CardId::new(), "Flitterwing draw witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    for _ in 0..6 {
        g.create_object_from_definition(&draw, alice(), Zone::Library);
    }
    dm.stage = "actual_source_paid_cast".into();
    cast_announce(&mut g, &defs[n].0, 1, &mut q, dm)?;
    dm.stage = "source_spell_resolution".into();
    finish(&mut g, &mut q, dm)?;
    let source = find(&g, n)?;
    let before_counter = g.counter_count(source, CounterType::MinusOneMinusOne);
    if before_counter != 1 || g.calculated_power(source) != Some(1) {
        return Err(format!(
            "actual Flitterwing entry expected1 counter/1power, got{before_counter}/{:?}",
            g.calculated_power(source)
        ));
    }
    let mut activation_paid = 0;
    if mode != "no_activation" {
        dm.counter_objects = vec![source];
        dm.stage = "actual_paid_counter_removal_activation".into();
        let actions = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
        let action = actions
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
            .ok_or("Flitterwing counter-removal activation unavailable")?;
        let mana = g.player(alice()).unwrap().mana_pool.total();
        announce(&mut g, action, &mut q, dm)?;
        activation_paid = mana - g.player(alice()).unwrap().mana_pool.total();
        dm.trace.push(json!({"stage":"activation_announced","mana_paid":activation_paid,"source_counter_before":before_counter,"source_counter_after":g.counter_count(source,CounterType::MinusOneMinusOne),"stack_size":g.stack.len()}));
        if activation_paid != 3 || g.counter_count(source, CounterType::MinusOneMinusOne) != 0 {
            return Err("Flitterwing full activation payment mismatch".into());
        }
        dm.stage = "delayed_trigger_creation".into();
        finish(&mut g, &mut q, dm)?;
    }
    if mode == "source_bounced_after_activation" {
        response(&mut g, defs, "Unsummon", source, &mut q, dm)?;
    }
    let mut life = vec![20, 20, 20];
    let mut loyalty = 10;
    if mode == "noncombat_creature_player" {
        dm.targets = vec![Target::Player(PlayerId(1))];
        dm.stage = "actual_noncombat_producer_paid_cast".into();
        cast_announce(&mut g, &defs["Prodigal Pyromancer"].0, 3, &mut q, dm)?;
        finish(&mut g, &mut q, dm)?;
        let pyro = find(&g, "Prodigal Pyromancer")?;
        g.remove_summoning_sickness(pyro);
        let action = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==pyro))
            .ok_or("Pyromancer actual tap activation unavailable")?;
        dm.stage = "actual_noncombat_creature_activation".into();
        announce(&mut g, action, &mut q, dm)?;
        if !g.is_tapped(pyro) {
            return Err("actual Pyromancer tap cost unpaid".into());
        }
        finish(&mut g, &mut q, dm)?;
        life[1] = 19;
    } else {
        let actor = if mode == "other_controller_combat" {
            PlayerId(1)
        } else {
            alice()
        };
        let defender = if actor == PlayerId(1) {
            PlayerId(2)
        } else {
            PlayerId(1)
        };
        let attackers = match mode {
            "self_combat_player" => vec![source],
            "two_owned_combat" => vec![first, second],
            "other_controller_combat" => vec![other],
            _ => vec![first],
        };
        g.turn.active_player = actor;
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(Step::DeclareAttackers);
        for id in &attackers {
            g.remove_summoning_sickness(*id);
        }
        let target = if mode == "own_combat_planeswalker" {
            AttackTarget::Planeswalker(pw)
        } else {
            AttackTarget::Player(defender)
        };
        let declarations = attackers
            .iter()
            .map(|id| AttackerDeclaration {
                creature: *id,
                target: target.clone(),
            })
            .collect::<Vec<_>>();
        let mut combat = CombatState::default();
        dm.stage = "actual_combat_declarations".into();
        apply_attacker_declarations_with_dm(&mut g, &mut combat, &mut q, &declarations, dm)
            .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        finish(&mut g, &mut q, dm)?;
        g.turn.step = Some(Step::DeclareBlockers);
        apply_blocker_declarations(&mut g, &mut combat, &mut q, &[], defender)
            .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        finish(&mut g, &mut q, dm)?;
        g.turn.step = Some(Step::CombatDamage);
        dm.stage = "actual_combat_damage".into();
        let events = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
            &mut g, &combat, false, dm,
        )
        .map_err(|e| e.to_string())?;
        dm.trace
            .push(json!({"stage":"actual_combat_damage_result","events":format!("{events:?}")}));
        ironsmith::game_loop::queue_combat_damage_triggers(&mut g, &events, &mut q);
        if mode == "own_combat_planeswalker" {
            loyalty = 8;
        } else {
            life[defender.0 as usize] -= 2 * attackers.len() as i32;
        }
    }
    dm.stage = "delayed_draw_resolution".into();
    let error = finish(&mut g, &mut q, dm).err();
    let draws = match mode {
        "other_controller_combat" | "noncombat_creature_player" | "no_activation" => 0,
        "two_owned_combat" => 2,
        _ => 1,
    };
    let source_bounced = mode == "source_bounced_after_activation";
    let actual = json!({"error":error,"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"draw_witness_hand":count(&g,"Flitterwing draw witness",Zone::Hand),"draw_witness_library":count(&g,"Flitterwing draw witness",Zone::Library),"planeswalker_loyalty":g.counter_count(pw,CounterType::Loyalty),"source_battlefield":count(&g,n,Zone::Battlefield),"source_hand":count(&g,n,Zone::Hand),"source_minus_counters":g.counter_count(source,CounterType::MinusOneMinusOne),"activation_mana_paid":activation_paid});
    let expected = json!({"error":null,"life":life,"draw_witness_hand":draws,"draw_witness_library":6-draws,"planeswalker_loyalty":loyalty,"source_battlefield":usize::from(!source_bounced),"source_hand":usize::from(source_bounced),"source_minus_counters":usize::from(mode=="no_activation"),"activation_mana_paid":if mode=="no_activation"{0}else{3}});
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
    let input = p.join("flitterwing-delayed-draw-inputs.json");
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
        let n = "Flitterwing Nuisance";
        let mode = c["mode"].as_str().unwrap();
        eprintln!("FLITTERWING_DELAYED_DRAW {mode}");
        let mut dm = Dm {
            accept: true,
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            counter_objects: vec![],
        };
        let result = run(&defs, mode, &mut dm);
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
        rows.push(json!({"card":n,"scenario":{"mode":mode},"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs[n].1,"execution_trace":dm.trace}));
    }
    let binary = std::env::current_exe().unwrap();
    let files = [
        "crates/ironsmith-engine/src/effects/composition/execute_with_source.rs",
        "crates/ironsmith-engine/src/effects/composition/tag_triggering_object.rs",
        "crates/ironsmith-engine/src/effects/damage/deal_damage.rs",
    ];
    let report = json!({"scope":"Canonical paid Flitterwing Nuisance enters with its actual -1/-1 counter; legal2U/remove1 activation uses explicit counter object distribution and counter-kind decisions. Real combat and normal paid Prodigal Pyromancer activation test subsequent delayed draws, with controller/noncombat/no-activation negatives and a source-removal control.","limitations":"Mana, neutral2/12 creatures, a10-loyalty planeswalker and library witnesses seeded. Combat positioned explicitly after paid setup and summoning sickness cleared. Normal priority and SBA resolution; no manually fabricated damage events or delayed abilities. Other printed paths not certified. No engine edits.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_flitterwing_delayed_draw_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("flitterwing-delayed-draw-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("flitterwing-delayed-draw-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical Flitterwing delayed draw expected-result reporter"]
fn report_flitterwing_delayed_draw() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
