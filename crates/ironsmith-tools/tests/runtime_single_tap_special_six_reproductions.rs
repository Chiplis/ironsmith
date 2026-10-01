//! Six remaining special single-tap costs: actual producers, payment, effects and expiry.
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
    decline: bool,
    targets: Vec<Target>,
    branch: Option<usize>,
    resources: Vec<ObjectId>,
    discard_id: Option<ObjectId>,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_partition(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        self.trace
            .push(json!({"choice":"partition_keep_all","context":format!("{c:?}")}));
        vec![]
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
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":!self.decline}));
        !self.decline
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let s = if c.description.starts_with("Choose an activation cost") {
            let branch = self.branch.expect("explicit alternative selector");
            assert!(
                c.options.iter().any(|o| o.index == branch && o.legal),
                "fixture cannot select an unadvertised branch"
            );
            vec![branch]
        } else if c
            .options
            .iter()
            .any(|o| o.legal && o.description.to_lowercase().contains("first strike"))
        {
            vec![
                c.options
                    .iter()
                    .find(|o| o.legal && o.description.to_lowercase().contains("first strike"))
                    .unwrap()
                    .index,
            ]
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace.push(json!({"choice":"options","description":c.description,"options":c.options.iter().map(|o|json!({"index":o.index,"description":o.description,"legal":o.legal})).collect::<Vec<_>>(),"selected":s}));
        s
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut preferred = if c.description.to_lowercase().contains("discard") {
            self.discard_id.into_iter().collect::<Vec<_>>()
        } else {
            self.resources.clone()
        };
        if c.description.to_lowercase().contains("discard") {
            preferred.extend(
                c.candidates
                    .iter()
                    .filter(|o| o.legal && g.object(o.id).is_some_and(|x| x.name == "Plains"))
                    .map(|o| o.id),
            );
        }
        let preferred = preferred
            .into_iter()
            .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
            .take(c.max.unwrap_or(c.candidates.len()))
            .collect::<Vec<_>>();
        let s = if preferred.len() >= c.min && !preferred.is_empty() {
            preferred
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(json!({"choice":"objects","description":c.description,"context":format!("{c:?}"),"selected":s.iter().map(|id|id.0).collect::<Vec<_>>()}));
        s
    }
    fn decide_distribute(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        let s = self
            .resources
            .iter()
            .copied()
            .filter(|id| c.targets.iter().any(|t| t.target == Target::Object(*id)))
            .take(c.total as usize)
            .map(|id| (Target::Object(id), 1))
            .collect::<Vec<_>>();
        self.trace.push(json!({"choice":"distribute","context":format!("{c:?}"),"selected":format!("{s:?}"),"allocated_total":s.iter().map(|(_,n)|*n).sum::<u32>()}));
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

fn get_stable(g: &GameState, id: ObjectId) -> u64 {
    g.object(id).unwrap().stable_id.0.0
}
fn find_stable(g: &GameState, stable: u64) -> Option<ObjectId> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.stable_id.0.0 == stable)
        .map(|o| o.id)
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
fn intrinsic_index(
    g: &GameState,
    source: ObjectId,
    def: &CardDefinition,
    canonical: usize,
) -> Result<usize, String> {
    let ironsmith::ability::AbilityKind::Activated(expected) = &def.abilities[canonical].kind
    else {
        return Err("expected canonical activated ability".into());
    };
    let matches = g
        .current_abilities(source)
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter_map(|(i, a)| match &a.kind {
            ironsmith::ability::AbilityKind::Activated(a)
                if a.mana_cost == expected.mana_cost
                    && a.timing == expected.timing
                    && format!("{:?}", a.effects) == format!("{:?}", expected.effects) =>
            {
                Some(i)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "canonical intrinsic cost match not unique:{matches:?}"
        ));
    }
    Ok(matches[0])
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

fn check(checks: &mut Vec<Value>, name: &str, expected: Value, observed: Value) {
    checks.push(json!({"check":name,"expected":expected,"observed":observed}));
}
fn next_main(g: &mut GameState, actor: u8, turn: u32, h: &mut Vec<Value>) -> Result<(), String> {
    let mut r = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
    reach_main(g, &mut r, &mut TriggerQueue::new(), actor, turn, h)
}
fn combat_and_expiry(
    g: &mut GameState,
    attacker: ObjectId,
    blocker: Option<ObjectId>,
    h: &mut Vec<Value>,
) -> Result<Value, String> {
    let start = g.turn.turn_number;
    let mut r = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
    let mut q = TriggerQueue::new();
    let mut offered = None;
    let mut declared = false;
    let mut blocker_offered = None;
    for _ in 0..500 {
        match r.advance(g, &mut q).map_err(|e| e.to_string())? {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                if g.turn.turn_number > start
                    && g.turn.active_player == PlayerId(0)
                    && g.turn.phase == ironsmith::Phase::FirstMain
                {
                    h.push(json!({"producer":"TurnRunner actual combat and next own main","turn":g.turn.turn_number}));
                    return Ok(
                        json!({"attacker_offered":offered,"attack_declared":declared,"blocker_offered":blocker_offered,"expired_at_turn":g.turn.turn_number}),
                    );
                }
                ironsmith::game_loop::run_priority_loop_with(
                    g,
                    &mut q,
                    &mut ironsmith::decision::AutoPassDecisionMaker,
                )
                .map_err(|e| e.to_string())?;
                r.priority_done();
            }
            TurnAction::Decision(c) => match c {
                DecisionContext::Attackers(c) => {
                    if g.turn.turn_number == start {
                        let ok = c.attacker_options.iter().any(|o| o.creature == attacker);
                        offered = Some(ok);
                        h.push(json!({"producer":"actual attacker decision","context":format!("{c:?}"),"offered":ok}));
                        if ok {
                            r.respond_attackers(vec![ironsmith::decision::AttackerDeclaration {
                                creature: attacker,
                                target: ironsmith::combat_state::AttackTarget::Player(PlayerId(1)),
                            }]);
                            declared = true;
                        } else {
                            r.respond_attackers(vec![]);
                        }
                    } else {
                        r.respond_attackers(vec![]);
                    }
                }
                DecisionContext::Blockers(c) => {
                    if g.turn.turn_number == start {
                        if let Some(b) = blocker {
                            blocker_offered = Some(c.blocker_options.iter().any(|o| {
                                o.attacker == attacker
                                    && o.valid_blockers.iter().any(|(id, _)| *id == b)
                            }));
                        }
                        h.push(json!({"producer":"actual blocker decision","context":format!("{c:?}")}));
                    }
                    r.respond_blockers(vec![], c.player);
                }
                DecisionContext::SelectObjects(c) => {
                    let ids = ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, &c);
                    r.respond_discard(ids);
                }
                DecisionContext::Boolean(_) => r.respond_boolean(false),
                c => return Err(format!("combat turn decision {c:?}")),
            },
            TurnAction::TurnComplete => {
                g.next_turn();
                r = TurnRunner::new();
            }
            TurnAction::GameOver(w) => return Err(format!("combat game over {w:?}")),
        }
    }
    Err("combat turn bound".into())
}
fn special_trial(
    defs: &HashMap<String, CardDefinition>,
    case: &Value,
    variant: &str,
) -> Result<Value, String> {
    let n = case["card"].as_str().unwrap();
    let index = case["index"].as_u64().unwrap() as usize;
    let mut g = new_game(defs);
    let mut d = Choices::default();
    let mut h = vec![];
    let mut checks = vec![];
    let source = cast_creature(
        &mut g,
        defs,
        n,
        0,
        defs[n].card.mana_cost.as_ref().unwrap().mana_value(),
        &mut d,
        &mut h,
    )?;
    let stable = get_stable(&g, source);
    let mut source = source;
    let mut target = None;
    let mut blocker = None;
    let mut attacker = None;
    if n == "Arachnus Spinner" {
        fund(&mut g, 1);
        target = Some(cast_creature(
            &mut g,
            defs,
            "Fleetfeather Cockatrice",
            1,
            5,
            &mut d,
            &mut h,
        )?);
    }
    if n == "Zombie Trailblazer" && index == 0 {
        target = Some(land(&mut g, &defs["Forest"], 0, &mut d, &mut h)?);
    }
    if n == "Zombie Trailblazer" && index == 1 {
        attacker = Some(cast_creature(
            &mut g,
            defs,
            "Grizzly Bears",
            0,
            2,
            &mut d,
            &mut h,
        )?);
        target = attacker;
        next_main(&mut g, 1, 4, &mut h)?;
        fund(&mut g, 1);
        blocker = Some(cast_creature(
            &mut g,
            defs,
            "Grizzly Bears",
            1,
            2,
            &mut d,
            &mut h,
        )?);
        land(
            &mut g,
            &defs[if variant == "no_swamp" {
                "Plains"
            } else {
                "Swamp"
            }],
            1,
            &mut d,
            &mut h,
        )?;
        next_main(&mut g, 0, 5, &mut h)?;
        fund(&mut g, 0);
    }
    if n == "Vodalian War Machine" {
        next_main(&mut g, 0, 5, &mut h)?;
        fund(&mut g, 0);
    }
    if n == "Purple Pentapus" && variant != "wrong_zone" {
        d.targets = vec![Target::Object(source)];
        paid(&mut g, defs, "Murder", 0, 3, &mut d, &mut h)?;
        source = find_stable(&g, stable).ok_or("murdered Purple lost identity")?;
        if g.object(source).unwrap().zone != Zone::Graveyard {
            return Err("Murder did not produce source graveyard".into());
        }
    }
    let self_eligible = ["Arachnus Spinner", "Patron Wizard", "Zombie Trailblazer"].contains(&n);
    let count = if variant == "zero" {
        0
    } else if variant == "surplus" {
        2
    } else {
        1
    };
    let mut resources = vec![];
    for _ in 0..count {
        resources.push(cast_creature(
            &mut g,
            defs,
            if variant == "ineligible" {
                "Fervor"
            } else {
                "Universal Automaton"
            },
            0,
            if variant == "ineligible" { 3 } else { 1 },
            &mut d,
            &mut h,
        )?);
    }
    if variant == "tapped" || variant == "ineligible" {
        if self_eligible {
            tap(&mut g, defs, source, &mut d, &mut h)?;
        }
        if variant == "tapped" {
            for id in &resources {
                tap(&mut g, defs, *id, &mut d, &mut h)?;
            }
        }
    }
    let mut web_stable = None;
    if n == "Arachnus Spinner" && variant != "empty_search" {
        if variant == "graveyard" {
            d.targets = vec![Target::Object(target.unwrap())];
            paid(&mut g, defs, "Arachnus Web", 0, 3, &mut d, &mut h)?;
            let web = find(&g, "Arachnus Web", Zone::Battlefield)?;
            web_stable = Some(get_stable(&g, web));
            d.targets = vec![Target::Object(web)];
            paid(&mut g, defs, "Disenchant", 0, 2, &mut d, &mut h)?;
            if g.object(find_stable(&g, web_stable.unwrap()).unwrap())
                .unwrap()
                .zone
                != Zone::Graveyard
            {
                return Err("Disenchant did not put Web into graveyard".into());
            }
        } else {
            let web =
                g.create_object_from_definition(&defs["Arachnus Web"], PlayerId(0), Zone::Library);
            web_stable = Some(get_stable(&g, web));
        }
    }
    let web_before = web_stable.and_then(|s| find_stable(&g, s)).map(|id| id.0);
    let library_before = g
        .player(PlayerId(0))
        .unwrap()
        .library
        .iter()
        .map(|id| id.0)
        .collect::<Vec<_>>();
    let mut pending_stable = None;
    let mut bob_tax_before = 0;
    if n == "Patron Wizard" && variant != "no_pending_spell" {
        fund(&mut g, 1);
        d.targets = vec![Target::Player(PlayerId(0))];
        let (mut q, e) = announce(&mut g, &defs["Shock"], 1, &mut d)?;
        if e["mana_paid"] != 1 {
            return Err("Bob Shock paid wrong amount".into());
        }
        h.push(e);
        let spell = g.stack.last().unwrap().object_id;
        pending_stable = Some(get_stable(&g, spell));
        target = Some(spell);
        g.player_mut(PlayerId(1)).unwrap().mana_pool = Default::default();
        if variant == "pay_tax" || variant == "decline_funded_tax" {
            g.player_mut(PlayerId(1))
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 1);
        }
        bob_tax_before = g.player(PlayerId(1)).unwrap().mana_pool.total();
        priority(&mut g, &mut q, 0, &mut d)?;
    }
    d.resources = if !resources.is_empty() && variant != "ineligible" {
        vec![resources[0]]
    } else if self_eligible {
        vec![source]
    } else {
        vec![]
    };
    d.targets = target
        .map(|id| vec![Target::Object(id)])
        .unwrap_or_default();
    if n == "Patron Wizard" {
        d.decline = variant != "pay_tax";
    }
    let live = intrinsic_index(&g, source, &defs[n], index)?;
    let want = !(variant == "tapped"
        || variant == "ineligible"
        || variant == "wrong_zone"
        || variant == "no_pending_spell"
        || (variant == "zero" && !self_eligible));
    let before_taps = resources
        .iter()
        .map(|id| g.is_tapped(*id))
        .collect::<Vec<_>>();
    let source_tapped_before = g.is_tapped(source);
    let mana_before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let legal = offered(&g, source, live);
    check(
        &mut checks,
        "intended_activation_offered",
        json!(want),
        json!(legal),
    );
    let action = if legal {
        activate(&mut g, source, live, &mut d).unwrap_or_else(|e| json!({"announcement_error":e}))
    } else {
        Value::Null
    };
    let mut effects = Value::Null;
    if want && legal {
        check(
            &mut checks,
            "exact_printed_mana_paid",
            json!(if n == "Purple Pentapus" { 3 } else { 0 }),
            action["mana_paid"].clone(),
        );
        check(
            &mut checks,
            "resolution_completed",
            Value::Null,
            action
                .get("announcement_error")
                .or_else(|| action.get("resolution_error"))
                .cloned()
                .unwrap_or(Value::Null),
        );
        check(
            &mut checks,
            "chosen_resource_tap_and_surplus",
            json!(if resources.is_empty() {
                vec![]
            } else {
                (0..resources.len()).map(|i| i == 0).collect::<Vec<_>>()
            }),
            json!(
                resources
                    .iter()
                    .map(|id| g.is_tapped(*id))
                    .collect::<Vec<_>>()
            ),
        );
        if n != "Purple Pentapus" {
            check(
                &mut checks,
                "source_only_tapped_when_selected",
                json!(resources.is_empty()),
                json!(g.is_tapped(source)),
            );
        }
        match n {
            "Arachnus Spinner" => {
                let result = web_stable.and_then(|s| find_stable(&g, s));
                let attached = result.is_some_and(|id| {
                    g.object(id).is_some_and(|o| {
                        o.zone == Zone::Battlefield
                            && o.attached_to
                                == Some(ironsmith::object::AttachmentTarget::Object(
                                    target.unwrap(),
                                ))
                    })
                });
                let count = g
                    .battlefield
                    .iter()
                    .filter(|id| g.object(**id).is_some_and(|o| o.name == "Arachnus Web"))
                    .count();
                check(
                    &mut checks,
                    "named_aura_enters_attached_to_declared_target",
                    json!([
                        variant != "empty_search",
                        if variant == "empty_search" { 0 } else { 1 }
                    ]),
                    json!([attached, count]),
                );
                let after = g
                    .player(PlayerId(0))
                    .unwrap()
                    .library
                    .iter()
                    .map(|id| id.0)
                    .collect::<Vec<_>>();
                if variant != "graveyard" {
                    check(
                        &mut checks,
                        "library_search_shuffles_remaining_cards",
                        json!(true),
                        json!(
                            after
                                != library_before
                                    .iter()
                                    .filter(|id| web_before != Some(**id))
                                    .copied()
                                    .collect::<Vec<_>>()
                        ),
                    );
                }
                effects = json!({"web_object":result.map(|id|id.0),"attached":attached,"library_before":library_before,"library_after":after});
            }
            "Patron Wizard" => {
                let shock = pending_stable.and_then(|s| find_stable(&g, s));
                let should_pay = variant == "pay_tax";
                check(
                    &mut checks,
                    "tax_controller_pays_exactly_one_or_declines",
                    json!(if should_pay { 1 } else { 0 }),
                    json!(bob_tax_before - g.player(PlayerId(1)).unwrap().mana_pool.total()),
                );
                check(
                    &mut checks,
                    "real_pending_shock_resolves_only_when_tax_paid",
                    json!(if should_pay { 18 } else { 20 }),
                    json!(g.player(PlayerId(0)).unwrap().life),
                );
                check(
                    &mut checks,
                    "pending_spell_finishes_in_graveyard",
                    json!(true),
                    json!(
                        shock.is_some_and(|id| g
                            .object(id)
                            .is_some_and(|o| o.zone == Zone::Graveyard))
                    ),
                );
                effects = json!({"shock_object":shock.map(|id|id.0),"alice_life":g.player(PlayerId(0)).unwrap().life,"bob_mana_after":g.player(PlayerId(1)).unwrap().mana_pool.total()});
            }
            "Purple Pentapus" => {
                let returned = find_stable(&g, stable);
                check(
                    &mut checks,
                    "same_card_returns_tapped",
                    json!(true),
                    json!(returned.is_some_and(|id| {
                        g.object(id).is_some_and(|o| o.zone == Zone::Battlefield) && g.is_tapped(id)
                    })),
                );
                let partitions = d
                    .trace
                    .iter()
                    .filter(|e| e["choice"] == "partition_keep_all")
                    .count();
                check(
                    &mut checks,
                    "both_real_etbs_surveille",
                    json!(2),
                    json!(partitions),
                );
                effects =
                    json!({"returned_id":returned.map(|id|id.0),"partition_decisions":partitions});
            }
            "Vodalian War Machine" => {
                let perm = g.object_has_static_ability_id(
                    source,
                    ironsmith::static_abilities::StaticAbilityId::CanAttackAsThoughNoDefender,
                );
                check(
                    &mut checks,
                    "defender_retained_and_attack_permission_added",
                    json!([true, true]),
                    json!([
                        g.object_has_static_ability_id(
                            source,
                            ironsmith::static_abilities::StaticAbilityId::Defender
                        ),
                        perm
                    ]),
                );
                let combat = combat_and_expiry(&mut g, source, None, &mut h)?;
                check(
                    &mut checks,
                    "actual_combat_offers_and_declares_source",
                    json!([true, true]),
                    json!([combat["attacker_offered"], combat["attack_declared"]]),
                );
                check(
                    &mut checks,
                    "next_turn_attack_permission_expired",
                    json!(false),
                    json!(g.object_has_static_ability_id(
                        source,
                        ironsmith::static_abilities::StaticAbilityId::CanAttackAsThoughNoDefender
                    )),
                );
                effects = combat;
            }
            "Zombie Trailblazer" if index == 0 => {
                let land = target.unwrap();
                let types = g.calculated_subtypes(land);
                check(
                    &mut checks,
                    "forest_becomes_only_swamp",
                    json!([true, false]),
                    json!([
                        types.contains(&ironsmith::Subtype::Swamp),
                        types.contains(&ironsmith::Subtype::Forest)
                    ]),
                );
                let turn = g.turn.turn_number;
                next_main(&mut g, 1, turn + 1, &mut h)?;
                let expired = g.calculated_subtypes(land);
                check(
                    &mut checks,
                    "land_type_reverts_after_cleanup",
                    json!([false, true]),
                    json!([
                        expired.contains(&ironsmith::Subtype::Swamp),
                        expired.contains(&ironsmith::Subtype::Forest)
                    ]),
                );
                effects = json!({"land":land.0,"types_before_cleanup":format!("{types:?}"),"types_after_cleanup":format!("{expired:?}")});
            }
            "Zombie Trailblazer" => {
                let who = attacker.unwrap();
                check(
                    &mut checks,
                    "chosen_creature_gains_landwalk",
                    json!(true),
                    json!(g.object_has_static_ability_id(
                        who,
                        ironsmith::static_abilities::StaticAbilityId::Landwalk
                    )),
                );
                let combat = combat_and_expiry(&mut g, who, blocker, &mut h)?;
                check(
                    &mut checks,
                    "actual_creature_attack_declared",
                    json!([true, true]),
                    json!([combat["attacker_offered"], combat["attack_declared"]]),
                );
                check(
                    &mut checks,
                    "actual_blocker_eligibility_follows_defending_swamp",
                    json!(variant == "no_swamp"),
                    combat["blocker_offered"].clone(),
                );
                check(
                    &mut checks,
                    "landwalk_expires_after_turn",
                    json!(false),
                    json!(g.object_has_static_ability_id(
                        who,
                        ironsmith::static_abilities::StaticAbilityId::Landwalk
                    )),
                );
                effects = combat;
            }
            _ => return Err("unhandled special source".into()),
        }
    } else if !want {
        check(
            &mut checks,
            "unavailable_action_preserves_payment",
            json!([mana_before, source_tapped_before, before_taps]),
            json!([
                g.player(PlayerId(0)).unwrap().mana_pool.total(),
                g.is_tapped(source),
                resources
                    .iter()
                    .map(|id| g.is_tapped(*id))
                    .collect::<Vec<_>>()
            ]),
        );
    }
    let pass = checks.iter().all(|c| c["expected"] == c["observed"]);
    Ok(
        json!({"card":n,"ability_index":index,"scenario":variant,"status":if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"expected":{"action_offered":want,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["expected"]})).collect::<Vec<_>>()},"actual":{"action_offered":legal,"action":action,"effects":effects},"checks":checks,"fixture_evidence":{"canonical_index":index,"live_index":live,"source":source.0,"source_stable":stable,"resource_ids":resources.iter().map(|id|id.0).collect::<Vec<_>>(),"history":h,"decision_trace":d.trace}}),
    )
}
#[test]
#[ignore = "Six special single chosen-resource cost paths"]
fn report_single_tap_special_six() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/single-tap-special-six-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_single_tap_special_six_reproductions.rs"),
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
    let filter = std::env::var("SINGLE_TAP_SPECIAL_SIX_CARD").ok();
    let mut rows = vec![];
    for case in payloads["cases"].as_array().unwrap() {
        let name = case["card"].as_str().unwrap();
        if filter.as_ref().is_some_and(|s| s != name) {
            continue;
        }
        let mut variants = vec!["zero", "exact", "surplus", "tapped", "ineligible"];
        if name == "Arachnus Spinner" {
            variants.extend(["graveyard", "empty_search"]);
        }
        if name == "Patron Wizard" {
            variants.extend(["pay_tax", "decline_funded_tax", "no_pending_spell"]);
        }
        if name == "Purple Pentapus" {
            variants.push("wrong_zone");
        }
        if name == "Zombie Trailblazer" && case["index"] == 1 {
            variants.push("no_swamp");
        }
        for v in variants {
            println!("stage {name} {v}");
            match special_trial(&defs, case, v) {
                Ok(r) => rows.push(r),
                Err(e) => rows.push(
                    json!({"card":name,"scenario":v,"status":"fixture_error","actual":{"error":e}}),
                ),
            }
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"scope":"Six special single-tap cost paths: full paid canonical sources, real pending spell and graveyard producers, named Aura search/attachment, actual turn-runner combat and effect expiry. No unavailable action forced.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    let filename = std::env::var("SINGLE_TAP_SPECIAL_SIX_REPORT")
        .unwrap_or("single-tap-special-six-execution.json".into());
    std::fs::write(
        root.join("reports/runtime-audit").join(filename),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
