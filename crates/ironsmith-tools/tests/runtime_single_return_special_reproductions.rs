//! Remaining single-object return costs: Aura, artifact, Elf, hand-land and combat hand source.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{
    DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};

#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    return_id: Option<ObjectId>,
    pay_tax: bool,
    put_id: Option<ObjectId>,
    decline: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        let yes = !self.decline && (c.player != PlayerId(1) || self.pay_tax);
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":yes}));
        yes
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let s = if c.description == "Choose a basic land type" {
            vec![
                c.options
                    .iter()
                    .find(|o| o.legal && o.description == "Mountain")
                    .expect("Mountain option")
                    .index,
            ]
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));
        s
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let preferred = if c.description.to_lowercase().contains("discard") {
            c.candidates
                .iter()
                .find(|o| o.legal && g.object(o.id).is_some_and(|o| o.name == "Serra Angel"))
                .map(|o| o.id)
        } else {
            self.put_id
                .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
                .or_else(|| {
                    self.return_id
                        .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
                })
        };
        let s = if let Some(id) = preferred {
            vec![id]
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(
            json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{s:?}")}),
        );
        s
    }
}
fn setup(players: usize, lands: usize) -> GameState {
    let mut g = GameState::new(
        ["Alice", "Bob", "Cara"][..players]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        20,
    );
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color, 12);
    }
    for _ in 0..lands {
        let def = CardDefinitionBuilder::new(CardId::new(), "Trigger sacrifice land")
            .card_types(vec![CardType::Land])
            .build();
        g.create_object_from_definition(&def, PlayerId(0), Zone::Battlefield);
    }
    g
}
fn announce(
    g: &mut GameState,
    def: &CardDefinition,
    actor: u8,
    dm: &mut Choices,
) -> Result<(TriggerQueue, Value), String> {
    g.turn.priority_player = Some(PlayerId(actor));
    let source = g.create_object_from_definition(def, PlayerId(actor), Zone::Hand);
    let action = compute_legal_actions(g, PlayerId(actor)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
        .ok_or("intended cast unavailable")?;
    let mana = g.player(PlayerId(actor)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none() && !g.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stopped:{progress:?}"));
        };
        if matches!(ctx, DecisionContext::Priority(_)) {
            return Err("announcement returned priority without spell".into());
        }
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    if g.stack.is_empty() {
        return Err("announcement budget".into());
    }
    let evidence = json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",g.stack.last().unwrap().targets),"resolution_error":null});
    if !g.stack.last().is_some_and(|entry| {
        g.object(entry.object_id)
            .is_some_and(|o| o.name == def.name())
    }) {
        return Err("intended spell did not reach top of stack".into());
    }
    Ok((q, evidence))
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: u8,
    dm: &mut Choices,
) -> Result<Value, String> {
    let (mut q, mut evidence) = announce(g, def, actor, dm)?;
    let mut state = PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        if let Err(error) = advance_priority_with_dm(g, &mut q, dm) {
            evidence["resolution_error"] = json!(error.to_string());
            return Ok(evidence);
        }
        if g.stack.is_empty() {
            return Ok(evidence);
        }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            if let Err(error) = apply_priority_response_with_dm(
                g,
                &mut q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            ) {
                evidence["resolution_error"] = json!(error.to_string());
                return Ok(evidence);
            }
        }
    }
    Err("resolution decision budget".into())
}

fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    for _ in 0..24 {
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        if g.stack.is_empty() {
            return Ok(());
        }
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
    }
    Err("finish budget".into())
}
fn activate(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    dm: &mut Choices,
) -> Result<Value, String> {
    let initial_stack_len = g.stack.len();
    g.turn.priority_player = Some(PlayerId(0));
    let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index)).ok_or("intended activation unavailable")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(action.clone()),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if st.pending_activation.is_none() && g.stack.len() > initial_stack_len {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("activation stopped:{progress:?}"));
        };
        if matches!(ctx, DecisionContext::Priority(_)) {
            return Err("activation returned priority without stack".into());
        }
        progress = apply_decision_context_with_dm(g, &mut q, &mut st, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    let paid = before - g.player(PlayerId(0)).unwrap().mana_pool.total();
    let announced_x = g.stack.last().and_then(|e| e.x_value);
    let source_present_at_announcement = g
        .object(source)
        .is_some_and(|o| o.zone == Zone::Battlefield);
    let error = finish(g, &mut q, dm).err();
    Ok(
        json!({"action":format!("{action:?}"),"mana_paid":paid,"resolution_error":error,"announced_x":announced_x,"source_present_at_announcement":source_present_at_announcement}),
    )
}

fn find(g: &GameState, n: &str, z: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.name == n && o.zone == z)
        .map(|o| o.id)
        .ok_or(format!("missing {n} in {z:?}"))
}
fn paid(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    actor: u8,
    cost: u32,
    d: &mut Choices,
    history: &mut Vec<Value>,
) -> Result<(), String> {
    let p = cast(g, &defs[name], actor, d)?;
    if p["mana_paid"] != cost || !p["resolution_error"].is_null() {
        return Err(format!("producer failed {p}"));
    }
    history.push(p);
    Ok(())
}

fn fund(g: &mut GameState, p: u8) {
    for c in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(p)).unwrap().mana_pool.add(c, 12);
    }
}
fn reach_main(
    g: &mut GameState,
    r: &mut TurnRunner,
    q: &mut TriggerQueue,
    actor: u8,
    min_turn: u32,
    h: &mut Vec<Value>,
) -> Result<(), String> {
    for _ in 0..500 {
        match r.advance(g, q).map_err(|e| e.to_string())? {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                if g.turn.active_player == PlayerId(actor)
                    && g.turn.phase == ironsmith::Phase::FirstMain
                    && g.turn.turn_number >= min_turn
                {
                    h.push(json!({"producer":"TurnRunner reached real first main priority","turn":g.turn.turn_number,"active":actor}));
                    return Ok(());
                }
                ironsmith::game_loop::run_priority_loop_with(
                    g,
                    q,
                    &mut ironsmith::decision::AutoPassDecisionMaker,
                )
                .map_err(|e| e.to_string())?;
                r.priority_done();
            }
            TurnAction::Decision(c) => match c {
                DecisionContext::Attackers(_) => {
                    r.respond_attackers(vec![]);
                }
                DecisionContext::Blockers(c) => {
                    r.respond_blockers(vec![], c.player);
                }
                DecisionContext::SelectObjects(c) => {
                    let ids = ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, &c);
                    r.respond_discard(ids);
                }
                DecisionContext::Boolean(_) => {
                    r.respond_boolean(false);
                }
                c => return Err(format!("turn decision {c:?}")),
            },
            TurnAction::TurnComplete => {
                g.next_turn();
                *r = TurnRunner::new();
            }
            TurnAction::GameOver(w) => return Err(format!("game over {w:?}")),
        }
    }
    Err("turn budget".into())
}
fn priority(g: &mut GameState, q: &mut TriggerQueue, p: u8, d: &mut Choices) -> Result<(), String> {
    if g.turn.priority_player == Some(PlayerId(p)) {
        return Ok(());
    }
    let mut st = PriorityLoopState::new(g.players_in_game());
    for _ in 0..g.players_in_game() {
        apply_priority_response_with_dm(
            g,
            q,
            &mut st,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            d,
        )
        .map_err(|e| e.to_string())?;
        if g.turn.priority_player == Some(PlayerId(p)) {
            return Ok(());
        }
    }
    Err("priority unavailable".into())
}
fn offered(g: &GameState, s: ObjectId, index: usize) -> bool {
    compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").iter().any(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index}if *source==s&&*ability_index==index))
}

const NAMES: [(&str, usize); 5] = [
    ("Krasis Incubation", 3),
    ("Master Transmuter", 0),
    ("Urban Retreat", 2),
    ("Wirewood Symbiote", 0),
    ("Zareth San, the Trickster", 1),
];
fn get_stable(g: &GameState, id: ObjectId) -> u64 {
    g.object(id).unwrap().stable_id.0.0
}
fn find_stable(g: &GameState, stable: u64) -> Option<ObjectId> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.stable_id.0.0 == stable)
        .map(|o| o.id)
}
fn snapshot(g: &GameState, source_stable: u64, target: Option<ObjectId>) -> Value {
    let source = find_stable(g, source_stable);
    let state = |id: ObjectId| json!({"id":id.0,"stable":get_stable(g,id),"name":g.object(id).unwrap().name.to_string(),"zone":format!("{:?}",g.object(id).unwrap().zone),"tapped":g.is_tapped(id),"power":g.calculated_power(id),"toughness":g.calculated_toughness(id),"plus_counters":g.object(id).unwrap().counters.get(&ironsmith::CounterType::PlusOnePlusOne).copied().unwrap_or(0),"owner":g.object(id).unwrap().owner.index(),"controller":g.current_controller(id).map(|p|p.index()),"attacking":g.combat.as_ref().is_some_and(|c|ironsmith::combat_state::is_attacking(c,id)),"attack_target":g.combat.as_ref().and_then(|c|ironsmith::combat_state::get_attack_target(c,id)).map(|x|format!("{x:?}"))});
    json!({"source":source.map(state),"target":target.and_then(|id|g.object(id).map(|_|state(id))),"objects":g.objects_in_deterministic_order().iter().filter(|o|o.zone!=Zone::Library).map(|o|state(o.id)).collect::<Vec<_>>(),"combat":format!("{:?}",g.combat),"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"stack_len":g.stack.len()})
}
fn diag(g: &GameState, s: ObjectId, index: usize) -> Value {
    let a = g.current_ability(s, index).unwrap();
    let ironsmith::ability::AbilityKind::Activated(a) = &a.kind else {
        panic!("activated expected")
    };
    let c = ironsmith::costs::CostCheckContext::new(s, PlayerId(0))
        .with_reason(ironsmith::costs::PaymentReason::ActivateAbility);
    json!({"source":s.0,"ability_index":index,"active":g.turn.active_player.index(),"phase":format!("{:?}",g.turn.phase),"priority":g.turn.priority_player.map(|p|p.index()),"turn":g.turn.turn_number,"mana":format!("{:?}",g.player(PlayerId(0)).unwrap().mana_pool),"component_checks":a.mana_cost.costs().iter().map(|cost|json!({"cost":format!("{cost:?}"),"check":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*cost.0,g,&c))})).collect::<Vec<_>>()})
}

fn probe(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    name: &str,
    want: bool,
    cost: u32,
    resource: Option<ObjectId>,
    target: Option<ObjectId>,
    put: Option<ObjectId>,
    scenario: &str,
    d: &mut Choices,
    h: &[Value],
) -> Value {
    let source_stable = get_stable(g, source);
    let returned_stable = resource.map(|id| get_stable(g, id));
    let put_stable = put.map(|id| get_stable(g, id));
    let before = snapshot(g, source_stable, target);
    let diagnostic = diag(g, source, index);
    let yes = offered(g, source, index);
    d.trace.clear();
    d.return_id = resource;
    d.put_id = put;
    d.targets = if name == "Wirewood Symbiote" {
        target.map(Target::Object).into_iter().collect()
    } else {
        vec![]
    };
    let action = if yes && want {
        match activate(g, source, index, d) {
            Ok(x) => x,
            Err(e) => json!({"error":e}),
        }
    } else {
        Value::Null
    };
    let after = snapshot(g, source_stable, target);
    let mut checks = vec![json!({"check":"activation_offered","expected":want,"observed":yes})];
    if yes && want {
        let resource_zone = returned_stable
            .and_then(|s| find_stable(g, s))
            .map(|id| format!("{:?}", g.object(id).unwrap().zone));
        checks.extend([json!({"check":"mana_paid","expected":cost,"observed":action["mana_paid"]}),json!({"check":"resolution_error","expected":null,"observed":action.get("error").or_else(||action.get("resolution_error")).cloned().unwrap_or(Value::Null)}),json!({"check":"chosen_cost_object_returned","expected":"Hand","observed":resource_zone})]);
        let target_stable = target.and_then(|id| g.object(id).map(|o| o.stable_id.0.0));
        let untouched = before["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| {
                let stable = o["stable"].as_u64().unwrap();
                stable != source_stable
                    && Some(stable) != returned_stable
                    && Some(stable) != put_stable
                    && Some(stable) != target_stable
            })
            .all(|o| {
                after["objects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|a| a["stable"] == o["stable"])
                    == Some(o)
            });
        checks.push(
            json!({"check":"unselected_objects_preserved","expected":true,"observed":untouched}),
        );
        checks.push(json!({"check":"no_extra_objects_created_or_lost","expected":before["objects"].as_array().unwrap().len(),"observed":after["objects"].as_array().unwrap().len()}));
        if name == "Master Transmuter" && returned_stable != Some(source_stable) {
            checks.push(json!({"check":"source_tapped_as_cost","expected":true,"observed":after["source"]["tapped"]}));
        }
        let (label, expect, actual) = match name {
            "Krasis Incubation" => (
                "enchanted_creature_two_counters_after_aura_return",
                json!([4, 4, 2]),
                json!([
                    after["target"]["power"],
                    after["target"]["toughness"],
                    after["target"]["plus_counters"]
                ]),
            ),
            "Master Transmuter" => {
                let put_state = put_stable.and_then(|s| find_stable(g, s)).map(|id| {
                    json!([
                        format!("{:?}", g.object(id).unwrap().zone),
                        g.current_controller(id).map(|p| p.index())
                    ])
                });
                (
                    "selected_hand_artifact_optional_entry",
                    json!([if d.decline { "Hand" } else { "Battlefield" }, 0]),
                    json!(put_state),
                )
            }
            "Urban Retreat" => (
                "source_enters_from_hand_tapped",
                json!(["Battlefield", true]),
                json!([after["source"]["zone"], after["source"]["tapped"]]),
            ),
            "Wirewood Symbiote" => (
                "target_untapped",
                json!(false),
                after["target"]["tapped"].clone(),
            ),
            "Zareth San, the Trickster" => (
                "source_enters_tapped_attacking_same_player",
                json!(["Battlefield", true, true, "Player(PlayerId(1))"]),
                json!([
                    after["source"]["zone"],
                    after["source"]["tapped"],
                    after["source"]["attacking"],
                    after["source"]["attack_target"]
                ]),
            ),
            _ => unreachable!(),
        };
        checks.push(json!({"check":label,"expected":expect,"observed":actual}));
    }
    let pass = checks.iter().all(|c| c["expected"] == c["observed"]);
    json!({"card":name,"scenario":scenario,"status":if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"checks":checks,"expected":{"action_offered":want,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["expected"]})).collect::<Vec<_>>()},"actual":{"action_offered":yes,"action":action,"before":before,"after":after,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["observed"]})).collect::<Vec<_>>()},"fixture_evidence":{"history":h,"diagnostic":diagnostic,"decision_trace":d.trace}})
}
fn new_game(defs: &HashMap<String, CardDefinition>) -> GameState {
    let mut g = setup(2, 0);
    for p in 0..2 {
        for _ in 0..30 {
            g.create_object_from_definition(&defs["Plains"], PlayerId(p), Zone::Library);
        }
    }
    g
}
fn cast_creature(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    n: &str,
    p: u8,
    cost: u32,
    d: &mut Choices,
    h: &mut Vec<Value>,
) -> Result<ObjectId, String> {
    let prior: Vec<_> = g.battlefield.to_vec();
    d.targets.clear();
    paid(g, defs, n, p, cost, d, h)?;
    g.battlefield
        .iter()
        .copied()
        .find(|id| !prior.contains(id) && g.object(*id).is_some_and(|o| o.name == n))
        .ok_or(format!("new {n} absent"))
}
fn tap(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    id: ObjectId,
    d: &mut Choices,
    h: &mut Vec<Value>,
) -> Result<(), String> {
    d.targets = vec![Target::Object(id)];
    paid(g, defs, "Twiddle", 0, 1, d, h)?;
    if !g.is_tapped(id) {
        return Err("actual Twiddle did not tap intended object".into());
    }
    Ok(())
}
fn combat(
    g: &mut GameState,
    r: &mut TurnRunner,
    q: &mut TriggerQueue,
    attackers: &[ObjectId],
    block: Option<ObjectId>,
    d: &mut Choices,
    h: &mut Vec<Value>,
) -> Result<(), String> {
    for _ in 0..100 {
        match r.advance(g, q).map_err(|e| e.to_string())? {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                if matches!(r.state(), TurnState::DeclareBlockersPriority) {
                    finish(g, q, d)?;
                    priority(g, q, 0, d)?;
                    h.push(json!({"producer":"actual TurnRunner attacks and blocks, post-block priority","attacks":attackers.iter().map(|id|id.0).collect::<Vec<_>>(),"blocker":block.map(|id|id.0),"combat":format!("{:?}",g.combat)}));
                    return Ok(());
                }
                ironsmith::game_loop::run_priority_loop_with(
                    g,
                    q,
                    &mut ironsmith::decision::AutoPassDecisionMaker,
                )
                .map_err(|e| e.to_string())?;
                r.priority_done();
            }
            TurnAction::Decision(c) => match c {
                DecisionContext::Attackers(_) => {
                    r.respond_attackers(
                        attackers
                            .iter()
                            .map(|id| ironsmith::decision::AttackerDeclaration {
                                creature: *id,
                                target: ironsmith::combat_state::AttackTarget::Player(PlayerId(1)),
                            })
                            .collect(),
                    );
                }
                DecisionContext::Blockers(c) => {
                    r.respond_blockers(
                        block
                            .map(|id| ironsmith::decision::BlockerDeclaration {
                                blocker: id,
                                blocking: attackers[0],
                            })
                            .into_iter()
                            .collect(),
                        c.player,
                    );
                }
                c => return Err(format!("combat decision {c:?}")),
            },
            a => return Err(format!("combat stopped {a:?}")),
        }
    }
    Err("combat budget".into())
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    index: usize,
    v: usize,
) -> Result<Vec<Value>, String> {
    let mut g = new_game(defs);
    let mut d = Choices::default();
    let mut h = vec![];
    let mut q = TriggerQueue::new();
    let mut r = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
    if name == "Zareth San, the Trickster" {
        g.turn.active_player = PlayerId(1);
        g.turn.priority_player = Some(PlayerId(1));
        fund(&mut g, 1);
        let blocker = cast_creature(&mut g, defs, "Grizzly Bears", 1, 2, &mut d, &mut h)?;
        reach_main(&mut g, &mut r, &mut q, 0, 4, &mut h)?;
        fund(&mut g, 0);
        paid(&mut g, defs, "Fervor", 0, 3, &mut d, &mut h)?;
        let mut attackers = vec![];
        let resources = if v >= 3 { 1 } else { v };
        for _ in 0..resources.max(1) {
            attackers.push(cast_creature(
                &mut g,
                defs,
                if resources == 0 {
                    "Grizzly Bears"
                } else {
                    "Merfolk Looter"
                },
                0,
                2,
                &mut d,
                &mut h,
            )?);
        }
        let source = g.create_object_from_definition(&defs[name], PlayerId(0), Zone::Hand);
        if v != 4 {
            combat(
                &mut g,
                &mut r,
                &mut q,
                &attackers,
                if v == 3 { Some(blocker) } else { None },
                &mut d,
                &mut h,
            )?;
        }
        fund(&mut g, 0);
        priority(&mut g, &mut q, 0, &mut d)?;
        let resource = if resources > 0 {
            Some(attackers[0])
        } else {
            None
        };
        return Ok(vec![probe(
            &mut g,
            source,
            index,
            name,
            resources > 0 && v < 3,
            4,
            resource,
            None,
            None,
            match v {
                0 => "no_attacking_rogue",
                1 => "one_unblocked_attacking_rogue",
                2 => "two_unblocked_attacking_rogues",
                3 => "blocked_attacking_rogue",
                _ => "rogue_not_attacking_in_main",
            },
            &mut d,
            &h,
        )]);
    }
    if name == "Krasis Incubation" {
        let target = cast_creature(&mut g, defs, "Grizzly Bears", 0, 2, &mut d, &mut h)?;
        let source = if v == 0 {
            g.create_object_from_definition(&defs[name], PlayerId(0), Zone::Hand)
        } else {
            d.targets = vec![Target::Object(target)];
            paid(&mut g, defs, name, 0, 4, &mut d, &mut h)?;
            find(&g, name, Zone::Battlefield)?
        };
        if v == 2 {
            let other = cast_creature(&mut g, defs, "Grizzly Bears", 0, 2, &mut d, &mut h)?;
            d.targets = vec![Target::Object(other)];
            paid(&mut g, defs, name, 0, 4, &mut d, &mut h)?;
        }
        priority(&mut g, &mut q, 0, &mut d)?;
        return Ok(vec![probe(
            &mut g,
            source,
            index,
            name,
            v > 0,
            3,
            Some(source),
            Some(target),
            None,
            match v {
                0 => "source_still_in_hand",
                1 => "one_attached_source_aura",
                _ => "two_attached_auras_source_identity",
            },
            &mut d,
            &h,
        )]);
    }
    if name == "Master Transmuter" {
        let fresh = v == 3;
        let tapped = v == 4;
        let decline = v == 5;
        let others = if v < 3 { v } else { 0 };
        if !fresh {
            paid(&mut g, defs, "Fervor", 0, 3, &mut d, &mut h)?;
        }
        let source = cast_creature(&mut g, defs, name, 0, 4, &mut d, &mut h)?;
        let mut artifacts = vec![];
        for _ in 0..others {
            artifacts.push(cast_creature(
                &mut g,
                defs,
                "Ornithopter",
                0,
                0,
                &mut d,
                &mut h,
            )?);
        }
        let put = g.create_object_from_definition(&defs["Ornithopter"], PlayerId(0), Zone::Hand);
        if tapped {
            tap(&mut g, defs, source, &mut d, &mut h)?;
        }
        priority(&mut g, &mut q, 0, &mut d)?;
        d.decline = decline;
        return Ok(vec![probe(
            &mut g,
            source,
            index,
            name,
            !fresh && !tapped,
            1,
            Some(artifacts.first().copied().unwrap_or(source)),
            None,
            Some(put),
            match v {
                0 => "no_other_artifact_return_source",
                1 => "one_other_artifact",
                2 => "two_other_artifacts",
                3 => "fresh_source_without_haste",
                4 => "tapped_source",
                _ => "return_source_decline_artifact_entry",
            },
            &mut d,
            &h,
        )]);
    }
    if name == "Wirewood Symbiote" {
        let source = cast_creature(&mut g, defs, name, 0, 1, &mut d, &mut h)?;
        let target = cast_creature(&mut g, defs, "Grizzly Bears", 0, 2, &mut d, &mut h)?;
        let mut elves = vec![];
        for _ in 0..v {
            elves.push(cast_creature(
                &mut g,
                defs,
                "Llanowar Elves",
                0,
                1,
                &mut d,
                &mut h,
            )?);
        }
        tap(&mut g, defs, target, &mut d, &mut h)?;
        priority(&mut g, &mut q, 0, &mut d)?;
        let row = probe(
            &mut g,
            source,
            index,
            name,
            v > 0,
            0,
            elves.first().copied(),
            Some(target),
            None,
            match v {
                0 => "no_elf",
                1 => "one_elf",
                _ => "two_elves",
            },
            &mut d,
            &h,
        );
        let completed = row["actual"]["action_offered"] == true
            && row["actual"]["action"]["resolution_error"].is_null()
            && row["actual"]["action"].get("error").is_none();
        let mut rows = vec![row];
        if v == 2 && completed {
            priority(&mut g, &mut q, 0, &mut d)?;
            rows.push(probe(
                &mut g,
                source,
                index,
                name,
                false,
                0,
                Some(elves[1]),
                Some(target),
                None,
                "same_turn_repeat_with_remaining_elf",
                &mut d,
                &h,
            ));
            let next = g.turn.turn_number + 2;
            reach_main(&mut g, &mut r, &mut q, 0, next, &mut h)?;
            fund(&mut g, 0);
            tap(&mut g, defs, target, &mut d, &mut h)?;
            rows.push(probe(
                &mut g,
                source,
                index,
                name,
                true,
                0,
                Some(elves[1]),
                Some(target),
                None,
                "next_turn_activation_reset",
                &mut d,
                &h,
            ));
        }
        return Ok(rows);
    }
    if name == "Urban Retreat" {
        let resources = if v >= 3 { 1 } else { v };
        let mut creatures = vec![];
        for _ in 0..resources {
            let id = cast_creature(&mut g, defs, "Grizzly Bears", 0, 2, &mut d, &mut h)?;
            if v != 3 {
                tap(&mut g, defs, id, &mut d, &mut h)?;
            }
            creatures.push(id);
        }
        let source = g.create_object_from_definition(&defs[name], PlayerId(0), Zone::Hand);
        if v == 4 {
            reach_main(&mut g, &mut r, &mut q, 1, 4, &mut h)?;
            fund(&mut g, 0);
        }
        if v == 5 {
            fund(&mut g, 1);
            priority(&mut g, &mut q, 1, &mut d)?;
            d.targets = vec![Target::Player(PlayerId(0))];
            let (_pending, e) = announce(&mut g, &defs["Shock"], 1, &mut d)?;
            if e["mana_paid"] != 1 {
                return Err("pending Shock payment".into());
            }
            h.push(e);
        }
        priority(&mut g, &mut q, 0, &mut d)?;
        return Ok(vec![probe(
            &mut g,
            source,
            index,
            name,
            resources > 0 && v < 3,
            2,
            creatures.first().copied(),
            None,
            None,
            match v {
                0 => "no_tapped_creature",
                1 => "one_tapped_creature",
                2 => "two_tapped_creatures",
                3 => "only_untapped_creature",
                4 => "opponent_main",
                _ => "own_main_stack_not_empty",
            },
            &mut d,
            &h,
        )]);
    }
    unreachable!()
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn normalize(v: &Value, p: &str, ids: &mut Vec<String>) -> Value {
    match v {
        Value::Object(o) => {
            let mut r = serde_json::Map::new();
            for (k, v) in o {
                if k == "id"
                    && p.ends_with("/card")
                    && o.contains_key("card_types")
                    && o.contains_key("name")
                {
                    ids.push(format!("{p}/id"));
                    continue;
                }
                r.insert(k.clone(), normalize(v, &format!("{p}/{k}"), ids));
            }
            Value::Object(r)
        }
        Value::Array(a) => Value::Array(
            a.iter()
                .enumerate()
                .map(|(i, v)| normalize(v, &format!("{p}/{i}"), ids))
                .collect(),
        ),
        _ => v.clone(),
    }
}
#[test]
#[ignore = "single special return costs, actual paid resources and combat"]
fn report_single_return_special_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/single-return-special-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_single_return_special_reproductions.rs"),
    ];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut compilation = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        let (a, d) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(n),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        let definition = serde_json::to_value(&a.payload.definition).unwrap();
        let (mut ids, mut fi) = (vec![], vec![]);
        let parity = normalize(&definition, "/definition", &mut ids)
            == normalize(&p["frozen_definition"], "/definition", &mut fi);
        compilation.push(json!({"card":n,"artifact_checksum":a.payload_checksum,"frozen_artifact_checksum":p["frozen_artifact_checksum"],"definition_matches_frozen_except_unique_card_ids":parity,"ignored_id_paths":ids,"definition":definition}));
        defs.insert(n.to_string(), d);
    }
    let mut rows = vec![];
    for (name, index) in NAMES {
        for variant in 0..match name {
            "Master Transmuter" | "Urban Retreat" => 6,
            "Zareth San, the Trickster" => 5,
            _ => 3,
        } {
            println!("stage: {name} variant{variant}");
            match trial(&defs,name,index,variant){Ok(r)=>rows.extend(r),Err(e)=>rows.push(json!({"card":name,"scenario":{"variant":variant},"status":"fixture_error","actual":{"error":e}}))}
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inventory: Value = serde_json::from_slice(
        &std::fs::read(root.join("reports/runtime-audit/choose-consume-cost-candidates.json"))
            .unwrap(),
    )
    .unwrap();
    let path_coverage=inventory["rows"].as_array().unwrap().iter().filter(|p|p["count_shape"]=="single"&&p["consumer_kind"]=="ReturnToHandEffect"&&NAMES.iter().any(|(n,_)|p["card"]==*n)).map(|p|json!({"card":p["card"],"path":p["path"],"consumer_path":p["consumer_path"],"source_rows":rows.iter().enumerate().filter(|(_,r)|r["card"]==p["card"]).map(|(i,_)|i).collect::<Vec<_>>()})).collect::<Vec<_>>();
    let report = json!({"scope":"Five remaining single-object return cost paths: actual paid Aura/artifact/Elf/tapped creature sources and resources, actual TurnRunner Rogue combat, hand abilities and timing/untapped/blocked negatives. Never force unavailable actions.","rows":rows,"path_coverage":path_coverage,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/single-return-special-final-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
