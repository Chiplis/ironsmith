//! Fixed additional choose/WithId-sacrifice costs through actual paid casting.
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
        .find(|id| !prior.contains(id) && g.object(*id).is_some_and(|o| o.name == defs[n].name()))
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

fn announce_existing(
    g: &mut GameState,
    def: &CardDefinition,
    actor: u8,
    source: ObjectId,
    dm: &mut Choices,
) -> Result<(TriggerQueue, Value), String> {
    g.turn.priority_player = Some(PlayerId(actor));
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
    let evidence = json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",g.stack.last().unwrap().targets),"resolution_error":null,"stack_card":g.stack.last().unwrap().object_id.0,"resources_after_announcement":dm.resources.iter().map(|id|json!({"original_id":id.0,"on_battlefield":g.battlefield.contains(id),"object":g.object(*id).map(|o|format!("{:?}",o.zone))})).collect::<Vec<_>>()});
    if !g.stack.last().is_some_and(|entry| {
        g.object(entry.object_id)
            .is_some_and(|o| o.name == def.name())
    }) {
        return Err("intended spell did not reach top of stack".into());
    }
    Ok((q, evidence))
}

fn check(v: &mut Vec<Value>, n: &str, e: Value, o: Value) {
    v.push(json!({"check":n,"expected":e,"observed":o}));
}
fn next_own_main(g: &mut GameState, actor: u8, h: &mut Vec<Value>) -> Result<(), String> {
    let start = g.turn.turn_number;
    let mut r = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
    reach_main(g, &mut r, &mut TriggerQueue::new(), actor, start + 1, h)
}
fn produce(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    actor: u8,
    d: &mut Choices,
    h: &mut Vec<Value>,
) -> Result<ObjectId, String> {
    fund(g, actor);
    if defs[name].card.card_types.contains(&CardType::Land) {
        let probe = g.create_object_from_definition(&defs[name], PlayerId(actor), Zone::Hand);
        let can = compute_legal_actions(g, PlayerId(actor)).expect("fixture has complete replacement state")
            .iter()
            .any(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==probe));
        // The probe remains an ordinary duplicate hand card. It grants no permissions or resources.
        if !can {
            next_own_main(g, actor, h)?;
            fund(g, actor);
        }
        land(g, &defs[name], actor, d, h)
    } else {
        cast_creature(
            g,
            defs,
            name,
            actor,
            defs[name].card.mana_cost.as_ref().unwrap().mana_value(),
            d,
            h,
        )
    }
}

fn trial(
    defs: &HashMap<String, CardDefinition>,
    c: &Value,
    variant: &str,
) -> Result<Value, String> {
    let n = c["card"].as_str().unwrap();
    let def = &defs[n];
    let required = c["required"].as_u64().unwrap() as usize;
    let variable = c["variable"] == true;
    let target_kind = c["target"].as_str().unwrap();
    let mut g = new_game(defs);
    let mut d = Choices {
        decline: true,
        ..Default::default()
    };
    let mut h = vec![];
    let mut checks = vec![];
    g.turn.active_player = PlayerId(1);
    g.turn.priority_player = Some(PlayerId(1));
    fund(&mut g, 1);
    for _ in 0..4 {
        g.create_object_from_definition(&defs["Plains"], PlayerId(1), Zone::Hand);
    }
    let mut targets = vec![];
    let damage_target = if target_kind == "creature"
        || target_kind == "two_creatures"
        || c["damage_resource"] == true
    {
        Some(produce(&mut g, defs, "Wall of Stone", 1, &mut d, &mut h)?)
    } else {
        None
    };
    if target_kind == "creature" || target_kind == "two_creatures" {
        targets.push(Target::Object(damage_target.unwrap()));
    }
    if target_kind == "two_creatures" {
        targets.push(Target::Object(produce(
            &mut g,
            defs,
            "Wall of Stone",
            1,
            &mut d,
            &mut h,
        )?));
    }
    let resource_name = if variant == "alternative_type" {
        c["alternative_resource"].as_str().unwrap()
    } else if variant == "wrong_type" {
        c["wrong_resource"].as_str().unwrap()
    } else {
        c["resource"].as_str().unwrap()
    };
    let mut opposing = vec![];
    if variant == "opponent_only" {
        for _ in 0..required {
            opposing.push(produce(&mut g, defs, resource_name, 1, &mut d, &mut h)?);
        }
    }
    next_own_main(&mut g, 0, &mut h)?;
    fund(&mut g, 0);
    for name in [
        "Wall of Stone",
        "Centaur Courser",
        "Colossal Dreadmaw",
        "Serra Angel",
        "Ornithopter",
    ] {
        g.create_object_from_definition(&defs[name], PlayerId(0), Zone::Library);
    }
    if target_kind == "grave_creature" || target_kind == "grave_artifact" {
        let card = if target_kind == "grave_creature" {
            "Grizzly Bears"
        } else {
            "Mind Stone"
        };
        let id = produce(&mut g, defs, card, 0, &mut d, &mut h)?;
        let stable = get_stable(&g, id);
        d.targets = vec![Target::Object(id)];
        paid(
            &mut g,
            defs,
            if target_kind == "grave_creature" {
                "Murder"
            } else {
                "Disenchant"
            },
            0,
            if target_kind == "grave_creature" {
                3
            } else {
                2
            },
            &mut d,
            &mut h,
        )?;
        let id = find_stable(&g, stable).ok_or("graveyard producer identity lost")?;
        if g.object(id).unwrap().zone != Zone::Graveyard {
            return Err("graveyard producer did not reach graveyard".into());
        }
        targets.push(Target::Object(id));
    }
    let count = match variant {
        "zero" | "opponent_only" => 0,
        "insufficient" => required - 1,
        "surplus" => required + 1,
        _ => required,
    };
    let mut resources = vec![];
    for _ in 0..count {
        resources.push(produce(&mut g, defs, resource_name, 0, &mut d, &mut h)?);
    }
    if c["damage_resource"] == true && !matches!(variant, "wrong_type" | "no_damage" | "zero") {
        for id in &resources {
            d.targets = vec![Target::Object(*id), Target::Object(damage_target.unwrap())];
            paid(&mut g, defs, "Rabid Bite", 0, 2, &mut d, &mut h)?;
            if !g.source_dealt_damage_this_turn(*id) {
                return Err("actual Bite did not establish resource dealt damage this turn".into());
            }
        }
    }
    let mut source = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let source_stable = get_stable(&g, source);
    if c["origin"] == "Graveyard" && variant != "wrong_zone" {
        d.targets.clear();
        d.discard_id = Some(source);
        paid(&mut g, defs, "Faithless Looting", 0, 1, &mut d, &mut h)?;
        source = find_stable(&g, source_stable).ok_or("discarded source identity lost")?;
        if g.object(source).unwrap().zone != Zone::Graveyard {
            return Err("normal Looting did not discard source".into());
        }
        d.discard_id = None;
    }
    let resource_snapshot=resources.iter().map(|id|json!({"id":id.0,"stable":get_stable(&g,*id),"name":g.object(*id).unwrap().name.to_string(),"zone":format!("{:?}",g.object(*id).unwrap().zone),"controller":g.current_controller(*id).map(|p|p.0),"power":g.calculated_power(*id),"mana_value":g.object(*id).unwrap().mana_cost.as_ref().map_or(0,|m|m.mana_value()),"dealt_damage_this_turn":g.source_dealt_damage_this_turn(*id)})).collect::<Vec<_>>();
    if target_kind == "spell" {
        fund(&mut g, 1);
        d.targets = vec![Target::Player(PlayerId(0))];
        let (mut q, e) = announce(&mut g, &defs["Shock"], 1, &mut d)?;
        if e["mana_paid"] != 1 {
            return Err("pending Shock cost wrong".into());
        }
        h.push(e);
        targets = vec![Target::Object(g.stack.last().unwrap().object_id)];
        priority(&mut g, &mut q, 0, &mut d)?;
    }
    if target_kind == "player" {
        targets = vec![Target::Player(PlayerId(1))];
    }
    d.targets = targets.clone();
    d.resources = resources
        .iter()
        .copied()
        .take(if variable { count } else { required })
        .collect();
    g.player_mut(PlayerId(0)).unwrap().mana_pool = Default::default();
    for pip in def.card.mana_cost.as_ref().unwrap().pips() {
        let symbol = pip[0];
        match symbol {
            ManaSymbol::Generic(v) => g
                .player_mut(PlayerId(0))
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, u32::from(v)),
            ManaSymbol::X => {}
            _ => g.player_mut(PlayerId(0)).unwrap().mana_pool.add(symbol, 1),
        }
    }
    let mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let origin = g.object(source).unwrap().zone;
    let want = if variable {
        true
    } else {
        !matches!(
            variant,
            "zero" | "insufficient" | "wrong_type" | "opponent_only" | "no_damage" | "wrong_zone"
        )
    };
    let all_actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered = all_actions
        .iter()
        .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source));
    let total_check = format!(
        "{:?}",
        ironsmith::cost::can_pay_cost_with_reason(
            &g,
            source,
            PlayerId(0),
            &g.object(source).unwrap().additional_cost,
            ironsmith::costs::PaymentReason::CastSpell
        )
    );
    let component_checks = g
        .object(source)
        .unwrap()
        .additional_cost
        .costs()
        .iter()
        .map(|cost| {
            format!(
                "{:?}",
                ironsmith::costs::can_pay_with_check_context(
                    &*cost.0,
                    &g,
                    &ironsmith::costs::CostCheckContext::new(source, PlayerId(0))
                        .with_reason(ironsmith::costs::PaymentReason::CastSpell)
                )
            )
        })
        .collect::<Vec<_>>();
    check(&mut checks, "cast_offered", json!(want), json!(offered));
    // This preliminary gate harness intentionally stops at an advertised source action.
    // Such a row remains unreviewed and requires a separate paid-outcome follow-through;
    // no available source action is dispatched using an underspecified choice policy.
    let state = json!({"source_zone":format!("{origin:?}"),"source_stable":source_stable,"active":g.turn.active_player.0,"priority":g.turn.priority_player.map(|p|p.0),"turn":g.turn.turn_number,"phase":format!("{:?}",g.turn.phase),"available_mana":format!("{:?}",g.player(PlayerId(0)).unwrap().mana_pool),"stack":g.stack.iter().map(|e|json!({"id":e.object_id.0,"name":g.object(e.object_id).map(|o|o.name.to_string())})).collect::<Vec<_>>(),"targets":format!("{targets:?}"),"resources":resource_snapshot,"opposing_resources":opposing.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"controller":g.current_controller(*id).map(|p|p.0)})).collect::<Vec<_>>()});
    let ordinary_control = if !offered && want {
        if target_kind == "spell" {
            let a = cast(&mut g, &defs["Dispel"], 0, &mut d)?;
            if a["mana_paid"] != 1 || !a["resolution_error"].is_null() {
                return Err(format!("Dispel same-state control failed {a}"));
            }
            Some(a)
        } else {
            let id = cast_creature(&mut g, defs, "Ornithopter", 0, 0, &mut d, &mut h)?;
            Some(
                json!({"mana_paid":0,"resolution_error":null,"normal_zero_mana_creature_cast_reaches_battlefield":g.battlefield.contains(&id),"object":id.0}),
            )
        }
    } else {
        None
    };
    let pass = checks.iter().all(|v| v["expected"] == v["observed"]);
    Ok(
        json!({"card":n,"scenario":variant,"status":if offered{"offered_requires_followthrough"}else if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"expected":{"cast_offered":want,"intended_origin":format!("{origin:?}"),"required_count":if variable{count}else{required},"resource_count":count,"printed_mana_funded":mana},"actual":{"cast_offered":offered,"source_action_dispatched":false,"total_cost_check":total_check,"component_checks":component_checks,"ordinary_control":ordinary_control},"checks":checks,"fixture_evidence":{"path":c["path"],"consumer_path":c["consumer_path"],"state_at_probe":state,"history":h,"decision_trace":d.trace}}),
    )
}
#[test]
#[ignore = "Explicit fixed additional choose/WithId cost family audit"]
fn report_additional_withid_special() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/additional-withid-special-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join(
            "crates/ironsmith-tools/tests/runtime_additional_withid_special_reproductions.rs",
        ),
    ];
    let before = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect::<Vec<_>>();
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
    let filter = std::env::var("ADDITIONAL_WITHID_CARD").ok();
    let mut rows = vec![];
    for c in payloads["cases"].as_array().unwrap() {
        let n = c["card"].as_str().unwrap();
        if filter.as_ref().is_some_and(|v| v != n) {
            continue;
        }
        let mut variants = vec!["zero", "exact", "surplus", "opponent_only"];
        if c["wrong_resource"].is_string() {
            variants.push("wrong_type");
        }
        if c["required"].as_u64().unwrap() > 1 {
            variants.push("insufficient");
        }
        if c["alternative_resource"].is_string() {
            variants.push("alternative_type");
        }
        if c["origin"] == "Graveyard" {
            variants.push("wrong_zone");
        }
        if c["damage_resource"] == true {
            variants.retain(|v| *v != "opponent_only");
            variants.push("no_damage");
        }
        for v in variants {
            println!("stage {n} {v}");
            rows.push(match trial(&defs, c, v) {
                Ok(row) => row,
                Err(e) => {
                    json!({"card":n,"scenario":v,"status":"fixture_error","actual":{"error":e}})
                }
            });
        }
    }
    let after = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect::<Vec<_>>();
    let out = std::env::var("ADDITIONAL_WITHID_REPORT")
        .unwrap_or("additional-withid-special-execution.json".into());
    std::fs::write(root.join("reports/runtime-audit").join(out),serde_json::to_string_pretty(&json!({"scope":"47 special additional WithId costs, first legal-action gate with actual resources and production timing/targets. Available actions stay explicitly unreviewed pending exact downstream fixture; absent actions are never forced. Same-state normal cast controls run on intended-legal missing actions.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}})).unwrap()+"\n").unwrap();
}
