//! Actual self-sacrifice plus effect-cost order and outcome audit; no engine edits.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_attacker_declarations_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, Phase, PlayerId,
    PowerToughness, Step, Subtype, Zone,
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
    chosen_subtype: String,
    preferred_objects: Vec<ObjectId>,
    cost_order: String,
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
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = if c.description.starts_with("Secretly choose") {
            vec![
                c.options
                    .iter()
                    .find(|o| o.legal && o.description == self.chosen_subtype)
                    .expect("offered Oracle subtype")
                    .index,
            ]
        } else if self.stage == "response_resolve_Twiddle"
            && c.options
                .iter()
                .any(|o| o.legal && o.description == "Untap")
        {
            vec![
                c.options
                    .iter()
                    .find(|o| o.legal && o.description == "Untap")
                    .unwrap()
                    .index,
            ]
        } else if c.description.starts_with("Choose the next cost") {
            let mut options = c.options.iter().filter(|o| o.legal).collect::<Vec<_>>();
            options.sort_by_key(|o| {
                let text = o.description.to_lowercase();
                if text.starts_with("tap") {
                    0
                } else if text.starts_with("sacrifice this") {
                    if self.cost_order == "self_first" {
                        1
                    } else {
                        5
                    }
                } else if text.starts_with("choose") || text.contains("reveal") {
                    2
                } else {
                    3
                }
            });
            vec![options.first().expect("some payable cost").index]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace.push(json!({"stage":self.stage,"choice":"options","context":format!("{c:?}"),"selected":selected,"selected_descriptions":selected.iter().filter_map(|i|c.options.iter().find(|o|o.index==*i).map(|o|o.description.clone())).collect::<Vec<_>>()}));
        selected
    }
    fn decide_objects(&mut self, _: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = self
            .preferred_objects
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
            .collect::<Vec<_>>();
        assert!(
            selected.len() >= c.min && c.max.is_none_or(|m| selected.len() <= m),
            "chosen sacrifice count legal"
        );
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
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let state = c["source_state"].as_str().unwrap();
    let others = c["other_sacrifices"].as_u64().unwrap() as usize;
    let expected_available = state == "ready";
    let mut g = game();
    let mut q = TriggerQueue::new();
    let mut resources = vec![];
    for (i, power) in [2, 3].into_iter().enumerate() {
        let d = CardDefinitionBuilder::new(CardId::new(), format!("Sacrifice resource {i}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 8))
            .build();
        let zone = if matches!(n, "Emrakul's Evangel" | "Sword of the Ages") && i >= others {
            Zone::Library
        } else {
            Zone::Battlefield
        };
        resources.push(g.create_object_from_definition(&d, alice(), zone));
    }
    let excluded = CardDefinitionBuilder::new(CardId::new(), "Excluded Eldrazi")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi])
        .power_toughness(PowerToughness::fixed(4, 8))
        .build();
    let eld = g.create_object_from_definition(
        &excluded,
        if n == "Sword of the Ages" {
            PlayerId(1)
        } else {
            alice()
        },
        Zone::Battlefield,
    );
    let enemy = CardDefinitionBuilder::new(CardId::new(), "Opponent resource")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 8))
        .build();
    let opponent = g.create_object_from_definition(&enemy, PlayerId(1), Zone::Battlefield);
    let land = CardDefinitionBuilder::new(CardId::new(), "Deadfall land resource")
        .card_types(vec![CardType::Land])
        .build();
    let lands = if n == "Sandstone Deadfall" {
        (0..if state == "one_land" { 1 } else { 2 })
            .map(|_| g.create_object_from_definition(&land, alice(), Zone::Battlefield))
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    dm.stage = "actual_source_paid_cast".into();
    cast_announce(
        &mut g,
        &defs[n].0,
        match n {
            "A Killer Among Us" => 5,
            "Sword of the Ages" => 6,
            _ => 3,
        },
        &mut q,
        dm,
    )?;
    dm.stage = "source_spell_and_etb_resolution".into();
    finish(&mut g, &mut q, dm)?;
    let source = find(&g, n)?;
    let mut tokens = vec![];
    if n == "A Killer Among Us" {
        for name in ["Human", "Merfolk", "Goblin"] {
            let id = find(&g, name)?;
            if count(&g, name, Zone::Battlefield) != 1
                || !matches!(
                    g.object(id).unwrap().kind,
                    ironsmith::object::ObjectKind::Token
                )
                || g.calculated_power(id) != Some(1)
            {
                return Err("actual Killer token producer mismatch".into());
            }
            tokens.push(id);
        }
        let selected = match dm.chosen_subtype.as_str() {
            "Human" => Subtype::Human,
            "Merfolk" => Subtype::Merfolk,
            _ => Subtype::Goblin,
        };
        if g.secret_chosen_subtype(source, alice()) != Some(selected)
            || g.chosen_subtype(source).is_some()
        {
            return Err("actual secret subtype choice missing or public prematurely".into());
        }
        g.turn.turn_number += 1;
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(Step::DeclareAttackers);
        for id in &tokens {
            g.remove_summoning_sickness(*id);
        }
        let decl = tokens
            .iter()
            .map(|id| AttackerDeclaration {
                creature: *id,
                target: AttackTarget::Player(PlayerId(1)),
            })
            .collect::<Vec<_>>();
        let mut combat = CombatState::default();
        dm.stage = "actual_token_attack_declarations".into();
        if state != "no_attacker" {
            apply_attacker_declarations_with_dm(&mut g, &mut combat, &mut q, &decl, dm)
                .map_err(|e| e.to_string())?;
        }
        g.combat = Some(combat);
        finish(&mut g, &mut q, dm)?;
        let chosen_index = match dm.chosen_subtype.as_str() {
            "Human" => 0,
            "Merfolk" => 1,
            _ => 2,
        };
        let target_index = if c["matching_target"] == true {
            chosen_index
        } else {
            (chosen_index + 1) % 3
        };
        dm.targets = vec![Target::Object(tokens[target_index])];
    } else if n == "Sandstone Deadfall" {
        dm.preferred_objects = lands;
        g.turn.active_player = PlayerId(1);
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(Step::DeclareAttackers);
        g.remove_summoning_sickness(opponent);
        let mut combat = CombatState::default();
        dm.stage = "actual_opposing_attacker_declaration".into();
        apply_attacker_declarations_with_dm(
            &mut g,
            &mut combat,
            &mut q,
            &[AttackerDeclaration {
                creature: opponent,
                target: AttackTarget::Player(alice()),
            }],
            dm,
        )
        .map_err(|e| e.to_string())?;
        g.combat = Some(combat);
        finish(&mut g, &mut q, dm)?;
        dm.targets = vec![Target::Object(opponent)];
    } else {
        dm.preferred_objects = resources
            .iter()
            .copied()
            .take(c["other_sacrifices"].as_u64().unwrap() as usize)
            .collect();
        if n == "Sword of the Ages" {
            if !g.is_tapped(source) {
                return Err("Sword did not enter tapped".into());
            }
            if compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
                .iter()
                .any(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
            {
                return Err("tapped Sword unexpectedly activatable".into());
            }
            if state != "tapped" {
                response(&mut g, defs, "Twiddle", source, &mut q, dm)?;
            }
            if state != "tapped" && g.is_tapped(source) {
                return Err("actual Twiddle did not untap Sword".into());
            }
            dm.targets = vec![Target::Player(PlayerId(1))];
        } else {
            g.turn.turn_number += 1;
            if state != "summoning_sick" {
                g.remove_summoning_sickness(source);
            }
            if state == "tapped" {
                g.tap(source);
            }
        }
    }
    g.turn.priority_player = Some(alice());
    let legal_actions = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let activation = defs[n]
        .0
        .abilities
        .iter()
        .enumerate()
        .find_map(|(i, a)| {
            if let ironsmith::ability::AbilityKind::Activated(v) = &a.kind {
                Some((i, v))
            } else {
                None
            }
        })
        .ok_or("canonical activated ability missing")?;
    let diagnostics = json!({"stage":"activation_legality_probe","requested_other_sacrifices":others,"resource_zones":resources.iter().map(|id|format!("{:?}",g.object(*id).unwrap().zone)).collect::<Vec<_>>(),"source_id":source.0,"source_on_battlefield":g.object(source).is_some_and(|o|o.zone==Zone::Battlefield),"source_controller":g.current_controller(source).map(|p|p.0),"source_tapped":g.is_tapped(source),"source_summoning_sick":g.is_summoning_sick(source),"legal_actions":format!("{legal_actions:?}"),"total_cost_check":format!("{:?}",ironsmith::cost::can_pay_cost_with_reason(&g,source,alice(),&activation.1.mana_cost,ironsmith::costs::PaymentReason::ActivateAbility)),"component_checks":activation.1.mana_cost.costs().iter().map(|c|json!({"cost":format!("{c:?}").chars().take(180).collect::<String>(),"check":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*c.0,&g,&ironsmith::costs::CostCheckContext::new(source,alice()).with_reason(ironsmith::costs::PaymentReason::ActivateAbility)))})).collect::<Vec<_>>()});
    dm.trace.push(diagnostics);
    let Some(action) = legal_actions
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source))
    else {
        return Ok((
            json!({"error":null,"legal_activation_available":expected_available}),
            json!({"error":null,"legal_activation_available":false}),
        ));
    };
    if !expected_available {
        return Ok((
            json!({"error":null,"legal_activation_available":false}),
            json!({"error":null,"legal_activation_available":true}),
        ));
    }
    dm.stage = "actual_self_sacrifice_effect_cost_activation".into();
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut error = announce(&mut g, action, &mut q, dm).err();
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    dm.trace.push(json!({"stage":"after_activation_announcement","error":error,"mana_paid":paid,"source_graveyard":count(&g,n,Zone::Graveyard),"other_resource_graveyard":(0..2).map(|i|count(&g,&format!("Sacrifice resource {i}"),Zone::Graveyard)).collect::<Vec<_>>(),"lands_graveyard":count(&g,"Deadfall land resource",Zone::Graveyard),"stack":format!("{:?}",g.stack),"public_chosen_subtype":format!("{:?}",g.chosen_subtype(source))}));
    if error.is_none() {
        dm.stage = "activated_ability_resolution".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let actual;
    let expected;
    if n == "A Killer Among Us" {
        let target_index = match dm.targets[0] {
            Target::Object(id) => tokens.iter().position(|x| *x == id).unwrap(),
            _ => unreachable!(),
        };
        let boosted = c["matching_target"] == true;
        actual = json!({"error":error,"mana_paid":paid,"source_graveyard":count(&g,n,Zone::Graveyard),"token_counters":tokens.iter().map(|id|g.counter_count(*id,CounterType::PlusOnePlusOne)).collect::<Vec<_>>(),"token_power":tokens.iter().map(|id|g.calculated_power(*id)).collect::<Vec<_>>(),"token_deathtouch":tokens.iter().map(|id|g.current_has_static_ability_id(*id,ironsmith::static_abilities::StaticAbilityId::Deathtouch)).collect::<Vec<_>>()});
        expected = json!({"error":null,"mana_paid":0,"source_graveyard":1,"token_counters":(0..3).map(|i|if i==target_index&&boosted{3}else{0}).collect::<Vec<_>>(),"token_power":(0..3).map(|i|if i==target_index&&boosted{4}else{1}).collect::<Vec<_>>(),"token_deathtouch":(0..3).map(|i|i==target_index&&boosted).collect::<Vec<_>>()});
    } else if n == "Sandstone Deadfall" {
        actual = json!({"error":error,"mana_paid":paid,"source_graveyard":count(&g,n,Zone::Graveyard),"lands_graveyard":count(&g,"Deadfall land resource",Zone::Graveyard),"attacker_graveyard":count(&g,"Opponent resource",Zone::Graveyard),"attacker_battlefield":count(&g,"Opponent resource",Zone::Battlefield)});
        expected = json!({"error":null,"mana_paid":0,"source_graveyard":1,"lands_graveyard":2,"attacker_graveyard":1,"attacker_battlefield":0});
    } else {
        let sword = n == "Sword of the Ages";
        let expected_power = [0, 2, 5][others];
        actual = json!({"error":error,"mana_paid":paid,"source_graveyard":count(&g,n,Zone::Graveyard),"source_exile":count(&g,n,Zone::Exile),"other_graveyard":(0..2).map(|i|count(&g,&format!("Sacrifice resource {i}"),Zone::Graveyard)).collect::<Vec<_>>(),"other_exile":(0..2).map(|i|count(&g,&format!("Sacrifice resource {i}"),Zone::Exile)).collect::<Vec<_>>(),"other_battlefield":resources.iter().map(|id|g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).collect::<Vec<_>>(),"eldrazi_witness_battlefield":g.object(eld).is_some_and(|o|o.zone==Zone::Battlefield),"opponent_witness_battlefield":g.object(opponent).is_some_and(|o|o.zone==Zone::Battlefield),"eldrazi_horror_tokens":count(&g,"Eldrazi Horror",Zone::Battlefield),"token_stats":g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.name=="Eldrazi Horror")).map(|id|json!([g.calculated_power(*id),g.calculated_toughness(*id)])).collect::<Vec<_>>(),"bob_life":g.player(PlayerId(1)).unwrap().life});
        expected = json!({"error":null,"mana_paid":0,"source_graveyard":usize::from(!sword),"source_exile":usize::from(sword),"other_graveyard":(0..2).map(|i|usize::from(i<others&&!sword)).collect::<Vec<_>>(),"other_exile":(0..2).map(|i|usize::from(i<others&&sword)).collect::<Vec<_>>(),"other_battlefield":vec![false,false],"eldrazi_witness_battlefield":true,"opponent_witness_battlefield":true,"eldrazi_horror_tokens":if sword{0}else{others+1},"token_stats":if sword{vec![]}else{vec![json!([3,2]);others+1]},"bob_life":if sword{20-expected_power}else{20}});
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
    let input = p.join("self-sacrifice-effect-cost-inputs.json");
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
        eprintln!("SELF_SACRIFICE_EFFECT_COST {c}");
        let mut dm = Dm {
            accept: true,
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen_subtype: c["chosen_subtype"].as_str().unwrap().into(),
            preferred_objects: vec![],
            cost_order: c["cost_order"].as_str().unwrap().into(),
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
        "crates/ironsmith-engine/src/decision/legal_actions.rs",
        "crates/ironsmith-engine/src/game_loop/priority_state.rs",
        "crates/ironsmith-engine/src/cost.rs",
        "crates/ironsmith-engine/src/effects/player/reveal_chosen_subtype.rs",
        "crates/ironsmith-engine/src/effects/composition/choose_objects.rs",
    ];
    let report = json!({"scope":"Actual paid canonical A Killer Among Us, Emrakul's Evangel, Sword of the Ages and Sandstone Deadfall with actual automatic cost routes and explicit legality controls. Killer actual token creation, secret choices, attack declarations and matching/mismatching targets. Evangel and Sword zero/one/two other-resource states; token count or damage/exile consumers asserted only if legal activation is offered. Sword actual paid Twiddle untaps its printed tapped entry; Deadfall destroys an actual opposing attacker after sacrificing two owned lands.","limitations":"Mana and neutral2/3-power sacrifice resources, an Eldrazi and opponent resource seeded; source casts and effects use ordinary priority/SBAs. Summoning sickness cleared and combat positioned as explicit setup. No engine edits.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_self_sacrifice_effect_cost_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("self-sacrifice-effect-cost-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("self-sacrifice-effect-cost-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical self-sacrifice effect-cost expected-result reporter"]
fn report_self_sacrifice_effect_cost() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
