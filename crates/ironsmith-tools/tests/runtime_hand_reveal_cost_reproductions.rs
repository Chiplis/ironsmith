//! Forecast and own-turn hand-reveal costs, driven through actual turns and priority.
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
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":true}));
        true
    }
    fn decide_colors(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<Color> {
        let colors = vec![Color::Red];
        self.trace.push(
            json!({"choice":"colors","context":format!("{c:?}"),"selected":format!("{colors:?}")}),
        );
        colors
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let x = if c.description == "Choose one or more colors" {
            vec![
                c.options
                    .iter()
                    .find(|o| o.legal && o.description == "Red")
                    .expect("red color option")
                    .index,
            ]
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":x}));
        x
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let x = ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, c);
        self.trace.push(
            json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{x:?}")}),
        );
        x
    }
    fn view_cards(
        &mut self,
        g: &GameState,
        viewer: PlayerId,
        ids: &[ObjectId],
        c: &ironsmith::decisions::context::ViewCardsContext,
    ) {
        self.trace.push(json!({"choice":"view_cards","viewer":viewer.index(),"public":c.public,"zone":format!("{:?}",c.zone),"ids":ids.iter().map(|id|id.0).collect::<Vec<_>>(),"names":ids.iter().map(|id|g.object(*id).map(|o|o.name.to_string())).collect::<Vec<_>>() }));
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

const NAMES: [(&str, usize, u32); 11] = [
    ("Govern the Guildless", 0, 2),
    ("Piercing Rays", 0, 3),
    ("Plumes of Peace", 1, 2),
    ("Pride of the Clouds", 2, 4),
    ("Proclamation of Rebirth", 0, 6),
    ("Sky Hussar", 2, 0),
    ("Skyscribing", 0, 3),
    ("Spirit en-Dal", 1, 2),
    ("Steeling Stance", 0, 1),
    ("Tetzimoc, Primal Death", 1, 1),
    ("Writ of Passage", 1, 2),
];
fn fund(g: &mut GameState) {
    for c in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(c, 12);
    }
}
fn priority(g: &mut GameState, q: &mut TriggerQueue, d: &mut Choices) -> Result<(), String> {
    if g.turn.priority_player == Some(PlayerId(0)) {
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
        if g.turn.priority_player == Some(PlayerId(0)) {
            return Ok(());
        }
    }
    Err("Alice priority unavailable".into())
}
fn reach_upkeep(
    g: &mut GameState,
    r: &mut TurnRunner,
    q: &mut TriggerQueue,
    d: &mut Choices,
    actor: u8,
    min_turn: u32,
    h: &mut Vec<Value>,
) -> Result<(), String> {
    for _ in 0..500 {
        match r.advance(g, q).map_err(|e| e.to_string())? {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                if g.turn.active_player == PlayerId(actor)
                    && g.turn.step == Some(ironsmith::Step::Upkeep)
                    && g.turn.turn_number >= min_turn
                {
                    finish(g, q, d)?;
                    priority(g, q, d)?;
                    h.push(json!({"producer":"TurnRunner reached real upkeep priority","turn":g.turn.turn_number,"active":g.turn.active_player.index(),"step":format!("{:?}",g.turn.step),"runner_state":r.state().sync_name()}));
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
            TurnAction::Decision(ctx) => match ctx {
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
                c => return Err(format!("unhandled turn decision:{c:?}")),
            },
            TurnAction::TurnComplete => {
                g.next_turn();
                *r = TurnRunner::new();
            }
            TurnAction::GameOver(w) => return Err(format!("game over {w:?}")),
        }
    }
    Err("turn runner budget".into())
}
fn offered(g: &GameState, source: ObjectId, index: usize) -> bool {
    compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index))
}
fn snapshot(g: &GameState, source: ObjectId, target: Option<ObjectId>) -> Value {
    json!({"source_in_hand":g.object(source).is_some_and(|o|o.zone==Zone::Hand),"source_revealed_until_upkeep_end":g.is_hand_card_revealed_until_upkeep_ends(source),
 "hand_sizes":g.players.iter().map(|p|p.hand.len()).collect::<Vec<_>>(),"library_sizes":g.players.iter().map(|p|p.library.len()).collect::<Vec<_>>(),
 "target":target.map(|id|json!({"id":id.0,"zone":g.object(id).map(|o|format!("{:?}",o.zone)),"tapped":g.is_tapped(id),"power":g.calculated_power(id),"toughness":g.calculated_toughness(id),"colors":format!("{:?}",g.current_colors(id)),"red_only":g.current_colors(id)==Some(ironsmith::color::ColorSet::RED),"shadow":g.object_has_static_ability_id(id,StaticAbilityId::Shadow),"cannot_be_blocked":!g.can_be_blocked(id),"counter_map":g.object(id).map(|o|format!("{:?}",o.counters)),"prey":g.object(id).map(|o|o.counters.iter().filter(|(kind,_)|kind.description()=="prey").map(|(_,n)|*n).sum::<u32>())})),
 "creatures":g.objects_in_deterministic_order().iter().filter(|o|o.zone==Zone::Battlefield&&o.card_types.contains(&CardType::Creature)).map(|o|json!({"id":o.id.0,"name":o.name.to_string(),"controller":g.current_controller(o.id).map(|p|p.index()),"is_token":matches!(o.kind,ironsmith::object::ObjectKind::Token),"subtypes":format!("{:?}",o.subtypes),"tapped":g.is_tapped(o.id),"power":g.calculated_power(o.id),"toughness":g.calculated_toughness(o.id),"flying":g.object_has_static_ability_id(o.id,StaticAbilityId::Flying),"white_blue":g.current_colors(o.id)==Some(ironsmith::color::ColorSet::WHITE.union(ironsmith::color::ColorSet::BLUE))})).collect::<Vec<_>>()})
}
fn diagnostics(g: &GameState, source: ObjectId, index: usize) -> Value {
    let a = g.current_ability(source, index).unwrap();
    let ironsmith::ability::AbilityKind::Activated(a) = &a.kind else {
        panic!("activation expected")
    };
    let c = ironsmith::costs::CostCheckContext::new(source, PlayerId(0))
        .with_reason(ironsmith::costs::PaymentReason::ActivateAbility);
    json!({"source":source.0,"source_zone":format!("{:?}",g.object(source).unwrap().zone),"turn":g.turn.turn_number,"active":g.turn.active_player.index(),"priority":g.turn.priority_player.map(|p|p.index()),"phase":format!("{:?}",g.turn.phase),"step":format!("{:?}",g.turn.step),"mana_pool":format!("{:?}",g.player(PlayerId(0)).unwrap().mana_pool),"component_checks":a.mana_cost.costs().iter().map(|cost|json!({"cost":format!("{cost:?}"),"result":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*cost.0,g,&c))})).collect::<Vec<_>>()})
}
fn probe(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    name: &str,
    scenario: &str,
    want: bool,
    cost: u32,
    target: Option<ObjectId>,
    d: &mut Choices,
    h: &[Value],
) -> Value {
    d.targets = target.map(Target::Object).into_iter().collect();
    d.trace.clear();
    let before = snapshot(g, source, target);
    let diag = diagnostics(g, source, index);
    let is_offered = offered(g, source, index);
    let action = if is_offered && want {
        match activate(g, source, index, d) {
            Ok(v) => v,
            Err(e) => json!({"error":e}),
        }
    } else {
        Value::Null
    };
    let after = snapshot(g, source, target);
    let reveal_viewers: Vec<_> = d
        .trace
        .iter()
        .filter(|v| {
            v["choice"] == "view_cards"
                && v["public"] == true
                && v["zone"] == "Hand"
                && v["ids"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!(source.0)))
        })
        .map(|v| v["viewer"].clone())
        .collect();
    let mut checks =
        vec![json!({"check":"intended_activation_offered","expected":want,"observed":is_offered})];
    if scenario == "next_own_upkeep_reset" {
        checks.push(json!({"check":"prior_upkeep_reveal_expired","expected":false,"observed":before["source_revealed_until_upkeep_end"]}));
        let expired = match name {
            "Govern the Guildless" => Some((
                "prior_color_change_expired",
                json!("Some(ColorSet(16))"),
                before["target"]["colors"].clone(),
            )),
            "Spirit en-Dal" => Some((
                "prior_shadow_expired",
                json!(false),
                before["target"]["shadow"].clone(),
            )),
            "Steeling Stance" => Some((
                "prior_pump_expired",
                json!([2, 2]),
                json!([before["target"]["power"], before["target"]["toughness"]]),
            )),
            "Writ of Passage" => Some((
                "prior_blocking_restriction_expired",
                json!(false),
                before["target"]["cannot_be_blocked"].clone(),
            )),
            _ => None,
        };
        if let Some((check, expected, observed)) = expired {
            checks.push(json!({"check":check,"expected":expected,"observed":observed}));
        }
    }
    if want && is_offered {
        checks.push(json!({"check":"mana_paid","expected":cost,"observed":action["mana_paid"]}));
        checks.push(json!({"check":"resolution_error","expected":null,"observed":action.get("error").or_else(||action.get("resolution_error")).cloned().unwrap_or(Value::Null)}));
        checks.push(json!({"check":"source_stays_in_hand","expected":true,"observed":after["source_in_hand"]}));
        checks.push(json!({"check":"source_publicly_revealed_to_both_players","expected":[0,1],"observed":reveal_viewers}));
        if name != "Tetzimoc, Primal Death" {
            checks.push(json!({"check":"source_remains_revealed_during_upkeep","expected":true,"observed":after["source_revealed_until_upkeep_end"]}));
        }
        let (label, expected, observed) = match name {
            "Govern the Guildless" => (
                "target_red_only",
                json!(true),
                after["target"]["red_only"].clone(),
            ),
            "Piercing Rays" | "Plumes of Peace" => (
                "target_tapped",
                json!(true),
                after["target"]["tapped"].clone(),
            ),
            "Pride of the Clouds" => {
                let birds = |v: &Value| {
                    v["creatures"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|x| {
                            x["name"] == "Bird"
                                && x["controller"] == 0
                                && x["is_token"] == true
                                && x["power"] == 1
                                && x["toughness"] == 1
                                && x["flying"] == true
                                && x["white_blue"] == true
                        })
                        .count()
                };
                (
                    "correct_bird_tokens",
                    json!(birds(&before) + 1),
                    json!(birds(&after)),
                )
            }
            "Proclamation of Rebirth" => {
                let lions = |v: &Value| {
                    v["creatures"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|x| x["name"] == "Savannah Lions" && x["controller"] == 0)
                        .count()
                };
                (
                    "returned_lions",
                    json!(lions(&before) + 1),
                    json!(lions(&after)),
                )
            }
            "Sky Hussar" => (
                "alice_hand_delta",
                json!(1),
                json!(
                    after["hand_sizes"][0].as_i64().unwrap()
                        - before["hand_sizes"][0].as_i64().unwrap()
                ),
            ),
            "Skyscribing" => (
                "both_hand_deltas",
                json!([1, 1]),
                json!(
                    (0..2)
                        .map(|p| after["hand_sizes"][p].as_i64().unwrap()
                            - before["hand_sizes"][p].as_i64().unwrap())
                        .collect::<Vec<_>>()
                ),
            ),
            "Spirit en-Dal" => (
                "target_shadow",
                json!(true),
                after["target"]["shadow"].clone(),
            ),
            "Steeling Stance" => (
                "target_power_toughness",
                json!([3, 3]),
                json!([after["target"]["power"], after["target"]["toughness"]]),
            ),
            "Tetzimoc, Primal Death" => (
                "prey_counter_increment",
                json!(before["target"]["prey"].as_u64().unwrap() + 1),
                after["target"]["prey"].clone(),
            ),
            "Writ of Passage" => (
                "target_unblockable",
                json!(true),
                after["target"]["cannot_be_blocked"].clone(),
            ),
            _ => unreachable!(),
        };
        checks.push(json!({"check":label,"expected":expected,"observed":observed}));
        if name == "Sky Hussar" {
            let taps = |v: &Value| {
                v["creatures"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|x| x["tapped"] == true)
                    .count()
            };
            checks.push(json!({"check":"two_creatures_tapped_as_cost","expected":taps(&before)+2,"observed":taps(&after)}));
        }
    }
    let pass = checks.iter().all(|c| c["expected"] == c["observed"]);
    json!({"card":name,"scenario":scenario,"status":if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"checks":checks,"expected":{"action_offered":want,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["expected"]})).collect::<Vec<_>>()},"actual":{"action_offered":is_offered,"action":action,"before":before,"after":after,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["observed"]})).collect::<Vec<_>>()},"fixture_evidence":{"paid_producers":h,"diagnostic":diag,"decision_trace":d.trace}})
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    index: usize,
    cost: u32,
    variant: usize,
) -> Result<Vec<Value>, String> {
    let mut g = setup(2, 0);
    for p in 0..2 {
        for _ in 0..30 {
            g.create_object_from_definition(&defs["Plains"], PlayerId(p), Zone::Library);
        }
    }
    let mut d = Choices::default();
    let mut h = vec![];
    let mut targets = vec![];
    let negative = variant == 3;
    match name {
        "Pride of the Clouds" | "Skyscribing" => {}
        "Sky Hussar" => {
            if negative {
                paid(&mut g, defs, "Savannah Lions", 0, 1, &mut d, &mut h)?;
            } else {
                for _ in 0..2 {
                    paid(&mut g, defs, "Raise the Alarm", 0, 2, &mut d, &mut h)?;
                }
            }
        }
        "Proclamation of Rebirth" => {
            let creature = if negative {
                "Grizzly Bears"
            } else {
                "Savannah Lions"
            };
            for _ in 0..if negative { 1 } else { 2 } {
                d.targets.clear();
                paid(
                    &mut g,
                    defs,
                    creature,
                    0,
                    if negative { 2 } else { 1 },
                    &mut d,
                    &mut h,
                )?;
                let id = find(&g, creature, Zone::Battlefield)?;
                d.targets = vec![Target::Object(id)];
                paid(&mut g, defs, "Murder", 0, 3, &mut d, &mut h)?;
            }
            targets = g
                .player(PlayerId(0))
                .unwrap()
                .graveyard
                .iter()
                .copied()
                .filter(|id| g.object(*id).unwrap().name == creature)
                .collect();
        }
        _ => {
            let creature = if name == "Writ of Passage" && negative {
                "Hill Giant"
            } else {
                "Grizzly Bears"
            };
            for _ in 0..if name == "Piercing Rays" && !negative {
                2
            } else {
                1
            } {
                d.targets.clear();
                paid(
                    &mut g,
                    defs,
                    creature,
                    0,
                    if creature == "Hill Giant" { 4 } else { 2 },
                    &mut d,
                    &mut h,
                )?;
            }
            targets = g
                .battlefield
                .iter()
                .copied()
                .filter(|id| g.object(*id).is_some_and(|o| o.name == creature))
                .collect();
        }
    }
    let source = g.create_object_from_definition(&defs[name], PlayerId(0), Zone::Hand);
    let mut r = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
    let mut q = TriggerQueue::new();
    if variant != 1 {
        reach_upkeep(
            &mut g,
            &mut r,
            &mut q,
            &mut d,
            if variant == 2 { 1 } else { 0 },
            4,
            &mut h,
        )?;
    }
    fund(&mut g);
    if negative && name == "Piercing Rays" {
        d.targets = vec![Target::Object(targets[0])];
        paid(&mut g, defs, "Twiddle", 0, 1, &mut d, &mut h)?;
        if !g.is_tapped(targets[0]) {
            return Err("Twiddle did not tap target".into());
        }
    }
    priority(&mut g, &mut q, &mut d)?;
    let scenario = match variant {
        0 => "own_upkeep_first",
        1 => "own_main",
        2 => "opponent_upkeep",
        _ => "own_upkeep_insufficient_resource_or_target",
    };
    let want = !negative && (variant == 0 || (variant == 1 && name == "Tetzimoc, Primal Death"));
    let mut rows = vec![probe(
        &mut g,
        source,
        index,
        name,
        scenario,
        want,
        cost,
        targets.first().copied(),
        &mut d,
        &h,
    )];
    if want
        && rows[0]["actual"]["action_offered"] == true
        && rows[0]["actual"]["action"]["resolution_error"].is_null()
        && rows[0]["actual"]["action"].get("error").is_none()
    {
        let second_target = if targets.len() > 1 {
            targets[1]
        } else {
            targets.first().copied().unwrap_or(source)
        };
        priority(&mut g, &mut q, &mut d)?;
        rows.push(probe(
            &mut g,
            source,
            index,
            name,
            &format!("{scenario}_repeat"),
            name == "Tetzimoc, Primal Death",
            cost,
            if targets.is_empty() {
                None
            } else {
                Some(second_target)
            },
            &mut d,
            &h,
        ));
        if variant == 0 && name != "Tetzimoc, Primal Death" {
            let turn = g.turn.turn_number + 2;
            reach_upkeep(&mut g, &mut r, &mut q, &mut d, 0, turn, &mut h)?;
            fund(&mut g);
            let next_target = if name == "Proclamation of Rebirth" {
                Some(second_target)
            } else {
                targets.first().copied()
            };
            rows.push(probe(
                &mut g,
                source,
                index,
                name,
                "next_own_upkeep_reset",
                true,
                cost,
                next_target,
                &mut d,
                &h,
            ));
        }
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
#[ignore = "strict fixed hand-reveal costs, actual turns and priority"]
fn report_hand_reveal_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/hand-reveal-cost-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_hand_reveal_cost_reproductions.rs"),
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
    for (name, index, cost) in NAMES {
        for variant in 0..if [
            "Sky Hussar",
            "Piercing Rays",
            "Proclamation of Rebirth",
            "Writ of Passage",
        ]
        .contains(&name)
        {
            4
        } else {
            3
        } {
            println!("stage: {name} variant{variant}");
            match trial(&defs,name,index,cost,variant){Ok(r)=>rows.extend(r),Err(e)=>rows.push(json!({"card":name,"scenario":{"variant":variant},"status":"fixture_error","actual":{"error":e}}))}
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"scope":"Ten forecast hand activations and Tetzimoc: normal paid support sources, actual TurnRunner upkeep, own main and opposing upkeep restrictions, repeat/next-upkeep reset, resource negatives. Never force absent actions.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/hand-reveal-cost-final-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
