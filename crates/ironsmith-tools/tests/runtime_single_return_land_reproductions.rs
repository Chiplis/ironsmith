//! Single-object land-return costs through paid canonical sources and actual land plays.
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

use ironsmith::color::Color;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};

#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    return_id: Option<ObjectId>,
    pay_tax: bool,
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
        let yes = c.player != PlayerId(1) || self.pay_tax;
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
            self.return_id
                .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
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

const NAMES: [(&str, usize, u32, u32); 15] = [
    ("Floodbringer", 1, 2, 2),
    ("Meloku the Clouded Mirror", 1, 5, 1),
    ("Mina and Denn, Wildborn", 1, 4, 2),
    ("Moonbow Illusionist", 1, 3, 2),
    ("Oboro Breezecaller", 1, 2, 2),
    ("Oboro Envoy", 1, 4, 2),
    ("Quirion Ranger", 0, 1, 0),
    ("Scryb Ranger", 4, 2, 0),
    ("Soramaro, First to Dream", 2, 6, 4),
    ("Soratami Cloudskater", 1, 2, 2),
    ("Soratami Mindsweeper", 1, 4, 2),
    ("Soratami Mirror-Guard", 1, 4, 2),
    ("Soratami Rainshaper", 1, 3, 3),
    ("Soratami Savant", 1, 4, 3),
    ("Wonderscape Sage", 1, 2, 0),
];
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
fn land(
    g: &mut GameState,
    def: &CardDefinition,
    p: u8,
    d: &mut Choices,
    h: &mut Vec<Value>,
) -> Result<ObjectId, String> {
    let id = g.create_object_from_definition(def, PlayerId(p), Zone::Hand);
    let stable = g.object(id).unwrap().stable_id;
    let mut q = TriggerQueue::new();
    priority(g, &mut q, p, d)?;
    let action = compute_legal_actions(g, PlayerId(p)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==id))
        .ok_or("land play unavailable")?;
    let mut st = PriorityLoopState::new(g.players_in_game());
    apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(action),
        d,
    )
    .map_err(|e| e.to_string())?;
    finish(g, &mut q, d)?;
    let id = g
        .objects_in_deterministic_order()
        .iter()
        .find(|o| o.stable_id == stable && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .ok_or("played land absent")?;
    h.push(json!({"producer":"normal legal land play","name":def.name(),"actor":p,"object":id.0,"stable_id":stable.0.0,"turn":g.turn.turn_number}));
    Ok(id)
}
fn offered(g: &GameState, s: ObjectId, index: usize) -> bool {
    compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").iter().any(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index}if *source==s&&*ability_index==index))
}
fn snapshot(g: &GameState, source: ObjectId, target: Option<ObjectId>) -> Value {
    let zoneids = |p: u8, z: Zone| {
        g.objects_in_deterministic_order()
            .iter()
            .filter(|o| o.zone == z && o.owner == PlayerId(p))
            .map(|o| json!({"id":o.id.0,"stable":o.stable_id.0.0,"name":o.name.to_string()}))
            .collect::<Vec<_>>()
    };
    json!({"source_present":g.object(source).is_some_and(|o|o.zone==Zone::Battlefield),"source_tapped":g.is_tapped(source),"source_summoning_sick":g.is_summoning_sick(source),"source_has_haste":g.object_has_static_ability_id(source,StaticAbilityId::Haste),"hand":[zoneids(0,Zone::Hand),zoneids(1,Zone::Hand)],"graveyard":[zoneids(0,Zone::Graveyard),zoneids(1,Zone::Graveyard)],"library_sizes":g.players.iter().map(|p|p.library.len()).collect::<Vec<_>>(),"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"target":target.map(|id|json!({"id":id.0,"name":g.object(id).map(|o|o.name.to_string()),"tapped":g.is_tapped(id),"power":g.calculated_power(id),"toughness":g.calculated_toughness(id),"subtypes":format!("{:?}",g.current_subtypes(id)),"trample":g.object_has_static_ability_id(id,StaticAbilityId::Trample),"shroud":g.object_has_static_ability_id(id,StaticAbilityId::Shroud),"can_be_blocked":g.can_be_blocked(id)})),
 "own_lands":g.battlefield.iter().filter(|id|g.current_controller(**id)==Some(PlayerId(0))&&g.current_has_card_type(**id,CardType::Land)).map(|id|json!({"id":id.0,"stable":g.object(*id).unwrap().stable_id.0.0,"name":g.object(*id).unwrap().name.to_string()})).collect::<Vec<_>>(),
 "illusions":g.objects_in_deterministic_order().iter().filter(|o|o.zone==Zone::Battlefield&&o.kind==ironsmith::object::ObjectKind::Token&&g.current_controller(o.id)==Some(PlayerId(0))&&g.current_has_subtype(o.id,ironsmith::types::Subtype::Illusion)&&g.calculated_power(o.id)==Some(1)&&g.calculated_toughness(o.id)==Some(1)&&g.current_colors(o.id)==Some(ironsmith::color::ColorSet::BLUE)&&g.object_has_static_ability_id(o.id,StaticAbilityId::Flying)).count(),"stack_len":g.stack.len()})
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
    s: ObjectId,
    index: usize,
    name: &str,
    cost: u32,
    resources: usize,
    target: Option<ObjectId>,
    stable: Option<u64>,
    want: bool,
    scenario: &str,
    d: &mut Choices,
    h: &[Value],
    wonders_nonbasic: bool,
) -> Value {
    d.trace.clear();
    let before = snapshot(g, s, target);
    let diagnostic = diag(g, s, index);
    let yes = offered(g, s, index);
    let mana = g.player(PlayerId(1)).unwrap().mana_pool.total();
    let action = if yes && want {
        match activate(g, s, index, d) {
            Ok(x) => x,
            Err(e) => json!({"error":e}),
        }
    } else {
        Value::Null
    };
    let after = snapshot(g, s, target);
    let mut checks = vec![json!({"check":"activation_offered","expected":want,"observed":yes})];
    if yes && want {
        checks.extend([json!({"check":"mana_paid","expected":cost,"observed":action["mana_paid"]}),json!({"check":"resolution_error","expected":null,"observed":action.get("error").or_else(||action.get("resolution_error")).cloned().unwrap_or(Value::Null)}),json!({"check":"one_selected_land_returned_to_hand","expected":true,"observed":stable.is_some_and(|id|after["hand"][0].as_array().unwrap().iter().any(|o|o["stable"]==id))}),json!({"check":"remaining_lands","expected":resources-1,"observed":after["own_lands"].as_array().unwrap().len()})]);
        let handdelta = after["hand"][0].as_array().unwrap().len() as i64
            - before["hand"][0].as_array().unwrap().len() as i64;
        let libdelta = before["library_sizes"][0].as_i64().unwrap()
            - after["library_sizes"][0].as_i64().unwrap();
        let (label, expect, actual) = match name {
            "Floodbringer" => (
                "target_land_tapped",
                json!(true),
                after["target"]["tapped"].clone(),
            ),
            "Meloku the Clouded Mirror" => (
                "correct_illusion_tokens",
                json!(before["illusions"].as_u64().unwrap() + 1),
                after["illusions"].clone(),
            ),
            "Mina and Denn, Wildborn" => (
                "target_trample",
                json!(true),
                after["target"]["trample"].clone(),
            ),
            "Moonbow Illusionist" => (
                "target_basic_land_type",
                json!("Some([Mountain])"),
                after["target"]["subtypes"].clone(),
            ),
            "Oboro Breezecaller" | "Quirion Ranger" | "Scryb Ranger" => (
                "target_untapped",
                json!(false),
                after["target"]["tapped"].clone(),
            ),
            "Oboro Envoy" => (
                "target_power_toughness",
                json!([
                    3 - before["hand"][0].as_array().unwrap().len() as i64 - 1,
                    3
                ]),
                json!([after["target"]["power"], after["target"]["toughness"]]),
            ),
            "Soramaro, First to Dream" => (
                "draw_and_return_hand_delta",
                json!([2, 1]),
                json!([handdelta, libdelta]),
            ),
            "Soratami Cloudskater" => (
                "return_draw_discard_deltas",
                json!([1, 1, 1]),
                json!([
                    handdelta,
                    libdelta,
                    after["graveyard"][0].as_array().unwrap().len() as i64
                        - before["graveyard"][0].as_array().unwrap().len() as i64
                ]),
            ),
            "Soratami Mindsweeper" => (
                "target_player_milled_two",
                json!([2, 2]),
                json!([
                    before["library_sizes"][1].as_i64().unwrap()
                        - after["library_sizes"][1].as_i64().unwrap(),
                    after["graveyard"][1].as_array().unwrap().len() as i64
                        - before["graveyard"][1].as_array().unwrap().len() as i64
                ]),
            ),
            "Soratami Mirror-Guard" => (
                "target_cannot_be_blocked",
                json!(false),
                after["target"]["can_be_blocked"].clone(),
            ),
            "Soratami Rainshaper" => (
                "target_shroud",
                json!(true),
                after["target"]["shroud"].clone(),
            ),
            "Soratami Savant" => (
                "paid_or_countered_pending_shock",
                json!([
                    if d.pay_tax { 18 } else { 20 },
                    if d.pay_tax { 3 } else { 0 },
                    0
                ]),
                json!([
                    after["life"][0].as_i64().unwrap(),
                    (mana - g.player(PlayerId(1)).unwrap().mana_pool.total()) as i64,
                    g.stack.len() as i64
                ]),
            ),
            "Wonderscape Sage" => (
                "return_draw_conditional_discard_deltas",
                json!([
                    if wonders_nonbasic { 2 } else { 1 },
                    1,
                    if wonders_nonbasic { 0 } else { 1 },
                    true
                ]),
                json!([
                    handdelta,
                    libdelta,
                    after["graveyard"][0].as_array().unwrap().len() as i64
                        - before["graveyard"][0].as_array().unwrap().len() as i64,
                    after["source_tapped"]
                ]),
            ),
            _ => unreachable!(),
        };
        checks.push(json!({"check":label,"expected":expect,"observed":actual}));
    }
    let pass = checks.iter().all(|c| c["expected"] == c["observed"]);
    json!({"card":name,"scenario":scenario,"resources":resources,"status":if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"checks":checks,"expected":{"action_offered":want,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["expected"]})).collect::<Vec<_>>()},"actual":{"action_offered":yes,"action":action,"before":before,"after":after,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["observed"]})).collect::<Vec<_>>()},"fixture_evidence":{"history":h,"diagnostic":diagnostic,"decision_trace":d.trace}})
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    index: usize,
    castcost: u32,
    cost: u32,
    variant: usize,
) -> Result<Vec<Value>, String> {
    let resources = variant.min(2);
    let resources = if variant >= 3 { 1 } else { resources };
    let mut g = setup(2, 0);
    g.turn.active_player = PlayerId(1);
    g.turn.priority_player = Some(PlayerId(1));
    for p in 0..2 {
        for _ in 0..30 {
            g.create_object_from_definition(&defs["Plains"], PlayerId(p), Zone::Library);
        }
    }
    let mut d = Choices::default();
    let mut h = vec![];
    let island = land(&mut g, &defs["Island"], 1, &mut d, &mut h)?;
    let mut r = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
    let mut q = TriggerQueue::new();
    reach_main(&mut g, &mut r, &mut q, 0, 4, &mut h)?;
    fund(&mut g, 0);
    // Real hand resources also keep the characteristic-defining Soramaro alive throughout setup.
    for _ in 0..2 {
        g.create_object_from_definition(&defs["Serra Angel"], PlayerId(0), Zone::Hand);
    }
    paid(&mut g, defs, "Exploration", 0, 1, &mut d, &mut h)?;
    let nonbasic = name == "Wonderscape Sage" && variant == 3;
    let fresh = name == "Wonderscape Sage" && variant == 4;
    let tapped = name == "Wonderscape Sage" && variant == 5;
    let mut lands = vec![];
    for _ in 0..resources {
        lands.push(land(
            &mut g,
            &defs[if nonbasic { "Cloudpost" } else { "Forest" }],
            0,
            &mut d,
            &mut h,
        )?);
    }
    if name == "Wonderscape Sage" && !fresh {
        paid(&mut g, defs, "Fervor", 0, 3, &mut d, &mut h)?;
    }
    paid(&mut g, defs, name, 0, castcost, &mut d, &mut h)?;
    let source = find(&g, name, Zone::Battlefield)?;
    let mut target = None;
    if [
        "Mina and Denn, Wildborn",
        "Oboro Envoy",
        "Quirion Ranger",
        "Scryb Ranger",
        "Soratami Mirror-Guard",
        "Soratami Rainshaper",
    ]
    .contains(&name)
    {
        let n = if name == "Oboro Envoy" {
            "Hill Giant"
        } else {
            "Grizzly Bears"
        };
        paid(
            &mut g,
            defs,
            n,
            0,
            if n == "Hill Giant" { 4 } else { 2 },
            &mut d,
            &mut h,
        )?;
        target = Some(find(&g, n, Zone::Battlefield)?);
    }
    if ["Floodbringer", "Moonbow Illusionist", "Oboro Breezecaller"].contains(&name) {
        target = Some(island);
    }
    if ["Oboro Breezecaller", "Quirion Ranger", "Scryb Ranger"].contains(&name) || tapped {
        let id = if tapped { source } else { target.unwrap() };
        d.targets = vec![Target::Object(id)];
        paid(&mut g, defs, "Twiddle", 0, 1, &mut d, &mut h)?;
        if !g.is_tapped(id) {
            return Err("Twiddle did not tap intended target".into());
        }
    }
    d.targets = target.map(Target::Object).into_iter().collect();
    if name == "Soratami Mindsweeper" {
        d.targets = vec![Target::Player(PlayerId(1))];
    }
    if name == "Soratami Savant" {
        fund(&mut g, 1);
        priority(&mut g, &mut q, 1, &mut d)?;
        d.targets = vec![Target::Player(PlayerId(0))];
        let (_pending, ev) = announce(&mut g, &defs["Shock"], 1, &mut d)?;
        if ev["mana_paid"] != 1 {
            return Err("Shock payment wrong".into());
        }
        h.push(ev);
        let spell = g.stack.last().unwrap().object_id;
        d.targets = vec![Target::Object(spell)];
        d.pay_tax = variant == 3;
    }
    priority(&mut g, &mut q, 0, &mut d)?;
    d.return_id = lands.first().copied();
    let stable = d.return_id.map(|id| g.object(id).unwrap().stable_id.0.0);
    let want = resources > 0 && !fresh && !tapped;
    let scenario = if nonbasic {
        "nonbasic_locus"
    } else if fresh {
        "fresh_source_without_haste"
    } else if tapped {
        "tapped_source"
    } else if name == "Soratami Savant" && variant == 3 {
        "counter_tax_paid"
    } else {
        match resources {
            0 => "zero_resources",
            1 => "exact_resource",
            _ => "surplus_resources",
        }
    };
    let row = probe(
        &mut g, source, index, name, cost, resources, target, stable, want, scenario, &mut d, &h,
        nonbasic,
    );
    let completed = row["actual"]["action_offered"] == true
        && row["actual"]["action"]["resolution_error"].is_null()
        && row["actual"]["action"].get("error").is_none();
    let mut rows = vec![row];
    if resources == 2 && ["Quirion Ranger", "Scryb Ranger"].contains(&name) && completed {
        priority(&mut g, &mut q, 0, &mut d)?;
        d.return_id = Some(lands[1]);
        let stable = Some(g.object(lands[1]).unwrap().stable_id.0.0);
        rows.push(probe(
            &mut g,
            source,
            index,
            name,
            cost,
            1,
            target,
            stable,
            false,
            "same_turn_repeat_with_remaining_forest",
            &mut d,
            &h,
            false,
        ));
        let next = g.turn.turn_number + 2;
        reach_main(&mut g, &mut r, &mut q, 0, next, &mut h)?;
        fund(&mut g, 0);
        d.targets = target.map(Target::Object).into_iter().collect();
        paid(&mut g, defs, "Twiddle", 0, 1, &mut d, &mut h)?;
        if !g.is_tapped(target.unwrap()) {
            return Err("reset Twiddle did not tap target".into());
        }
        rows.push(probe(
            &mut g,
            source,
            index,
            name,
            cost,
            1,
            target,
            stable,
            true,
            "next_own_turn_reset",
            &mut d,
            &h,
            false,
        ));
    }
    Ok(rows)
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
#[ignore = "single land-return costs, paid sources and actual land plays"]
fn report_single_return_land_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/single-return-cost-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_single_return_land_reproductions.rs"),
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
    for (name, index, castcost, cost) in NAMES {
        for variant in 0..if name == "Wonderscape Sage" {
            6
        } else if name == "Soratami Savant" {
            4
        } else {
            3
        } {
            println!("stage: {name} variant{variant}");
            match trial(&defs,name,index,castcost,cost,variant){Ok(r)=>rows.extend(r),Err(e)=>rows.push(json!({"card":name,"scenario":{"variant":variant},"status":"fixture_error","actual":{"error":e}}))}
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
    let path_coverage=inventory["rows"].as_array().unwrap().iter().filter(|p|p["count_shape"]=="single"&&p["consumer_kind"]=="ReturnToHandEffect"&&NAMES.iter().any(|(n,_,_,_)|p["card"]==*n)).map(|p|json!({"card":p["card"],"path":p["path"],"consumer_path":p["consumer_path"],"source_rows":rows.iter().enumerate().filter(|(_,r)|r["card"]==p["card"]).map(|(i,_)|i).collect::<Vec<_>>()})).collect::<Vec<_>>();
    let report = json!({"scope":"Fifteen single-land-return activated costs. Full paid source, actual paid Exploration and legal land plays, real pending spell for countering, cost/resource/timing negatives. Never force unavailable actions.","rows":rows,"path_coverage":path_coverage,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/single-return-land-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
