//! Single tap direct-effect costs: actual resources, self eligibility, precise outcomes.
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

fn effect_trial(
    defs: &HashMap<String, CardDefinition>,
    case: &Value,
    variant: &str,
) -> Result<Value, String> {
    let name = case["card"].as_str().unwrap();
    let mode = case["mode"].as_str().unwrap();
    let resource_name = case["resource"].as_str().unwrap();
    let index = case["index"].as_u64().unwrap() as usize;
    let mut g = new_game(defs);
    let mut d = Choices::default();
    let mut h = vec![];
    let target = if ["tap", "targetpump20", "prevent1", "bounce"].contains(&mode) {
        fund(&mut g, 1);
        Some(cast_creature(
            &mut g,
            defs,
            "Fleetfeather Cockatrice",
            1,
            5,
            &mut d,
            &mut h,
        )?)
    } else {
        None
    };
    let cost = defs[name].card.mana_cost.as_ref().unwrap().mana_value();
    let source = cast_creature(&mut g, defs, name, 0, cost, &mut d, &mut h)?;
    let count = match variant {
        "zero" => 0,
        "surplus" => 2,
        _ => 1,
    };
    let mut resources = vec![];
    for _ in 0..count {
        fund(&mut g, 0);
        let rn = if variant == "ineligible" {
            "Fervor"
        } else {
            resource_name
        };
        let cost = defs[rn].card.mana_cost.as_ref().unwrap().mana_value();
        resources.push(cast_creature(&mut g, defs, rn, 0, cost, &mut d, &mut h)?);
    }
    if variant == "tapped" || variant == "ineligible" {
        for id in resources.clone() {
            if variant == "tapped" {
                fund(&mut g, 0);
                tap(&mut g, defs, id, &mut d, &mut h)?;
            }
        }
        if g.current_has_card_type(source, CardType::Creature)
            || g.current_has_card_type(source, CardType::Artifact)
        {
            fund(&mut g, 0);
            tap(&mut g, defs, source, &mut d, &mut h)?;
        }
    }
    let selected = resources
        .first()
        .copied()
        .or_else(|| (case["source_self"] == true).then_some(source));
    let want = if variant == "tapped" || variant == "ineligible" {
        false
    } else if variant == "zero" {
        case["source_self"] == true
    } else {
        true
    };
    let live = intrinsic_index(&g, source, &defs[name], index)?;
    d.resources = selected.into_iter().collect();
    d.targets = match mode {
        "tap" | "targetpump20" | "prevent1" | "bounce" => vec![Target::Object(target.unwrap())],
        "mill" | "damage1" => vec![Target::Player(PlayerId(1))],
        "prevent2" => vec![Target::Player(PlayerId(0))],
        _ => vec![],
    };
    let before = json!({"source_power":g.calculated_power(source),"source_toughness":g.calculated_toughness(source),"source_tapped":g.is_tapped(source),"resource_tapped":resources.iter().map(|id|g.is_tapped(*id)).collect::<Vec<_>>(),"resource_power":selected.and_then(|id|g.calculated_power(id)),"hand":g.player(PlayerId(0)).unwrap().hand.len(),"own_library":g.player(PlayerId(0)).unwrap().library.len(),"opponent_library":g.player(PlayerId(1)).unwrap().library.len(),"life":[g.player(PlayerId(0)).unwrap().life,g.player(PlayerId(1)).unwrap().life],"plan":g.counter_count(source,ironsmith::CounterType::Named("plan".into()))});
    let legal = offered(&g, source, live);
    let action = if legal && want {
        activate(&mut g, source, live, &mut d).unwrap_or_else(|e| json!({"announcement_error":e}))
    } else {
        Value::Null
    };
    let mut checks =
        vec![json!({"check":"intended_activation_offered","expected":want,"observed":legal})];
    if want && legal {
        let fixed_mana = if name == "Black Oak of Odunos" {
            1
        } else if name == "Volrath's Gardens" {
            2
        } else {
            0
        };
        checks.extend([json!({"check":"exact_fixed_mana_and_resolution","expected":[fixed_mana,null],"observed":[action["mana_paid"],action["resolution_error"]]}),json!({"check":"chosen_resource_tapped","expected":true,"observed":g.is_tapped(selected.unwrap())}),json!({"check":"unselected_surplus_untapped","expected":false,"observed":resources.iter().skip(1).any(|id|g.is_tapped(*id))}),json!({"check":"source_tapped_by_cost_or_effect_only","expected":selected==Some(source)||mode=="indestructible_tap","observed":g.is_tapped(source)})]);
        use ironsmith::static_abilities::StaticAbilityId as K;
        let pt = |dp: i64, dt: i64| json!({"check":"power_toughness_delta","expected":[before["source_power"].as_i64().unwrap()+dp,before["source_toughness"].as_i64().unwrap()+dt],"observed":[g.calculated_power(source),g.calculated_toughness(source)]});
        match mode{
   "tap"=>checks.push(json!({"check":"target_tapped","expected":true,"observed":g.is_tapped(target.unwrap())})),
   "draw"=>checks.push(json!({"check":"one_card_drawn","expected":[before["hand"].as_u64().unwrap()+1,before["own_library"].as_u64().unwrap()-1],"observed":[g.player(PlayerId(0)).unwrap().hand.len(),g.player(PlayerId(0)).unwrap().library.len()]})),
   "mill"=>checks.push(json!({"check":"opponent_mills_exactly_one","expected":before["opponent_library"].as_u64().unwrap()-1,"observed":g.player(PlayerId(1)).unwrap().library.len()})),
   "pump11"=>checks.push(pt(1,1)),"pump21"=>checks.push(pt(2,1)),"pump_power"=>checks.push(pt(before["resource_power"].as_i64().unwrap(),0)),
   "targetpump20"=>checks.push(json!({"check":"target_plus_two_zero","expected":[5,3],"observed":[g.calculated_power(target.unwrap()),g.calculated_toughness(target.unwrap())]})),
   "bounce"=>checks.push(json!({"check":"target_returned_to_owners_hand","expected":true,"observed":g.player(PlayerId(1)).unwrap().hand.iter().any(|id|g.object(*id).is_some_and(|o|o.name=="Fleetfeather Cockatrice"))})),
   "unblockable"=>checks.push(json!({"check":"source_cannot_be_blocked","expected":false,"observed":g.can_be_blocked(source)})),
   "damage1"=>checks.push(json!({"check":"opponent_takes_one","expected":19,"observed":g.player(PlayerId(1)).unwrap().life})),
   "life2"=>checks.push(json!({"check":"controller_gains_two","expected":22,"observed":g.player(PlayerId(0)).unwrap().life})),
   "flying"|"deathtouch"|"firststrike"|"indestructible_tap"=>checks.push(json!({"check":"printed_keyword_granted","expected":true,"observed":g.object_has_static_ability_id(source,match mode{"flying"=>K::Flying,"deathtouch"=>K::Deathtouch,"firststrike"=>K::FirstStrike,_=>K::Indestructible})})),
   "plan"=>checks.push(json!({"check":"mill_one_and_one_plan","expected":[before["own_library"].as_u64().unwrap()-1,before["plan"].as_u64().unwrap()+1],"observed":[g.player(PlayerId(0)).unwrap().library.len(),g.counter_count(source,ironsmith::CounterType::Named("plan".into()))]})),
   "prevent1"|"prevent2"=>{fund(&mut g,0);d.targets=if mode=="prevent1"{vec![Target::Object(target.unwrap())]}else{vec![Target::Player(PlayerId(0))]};paid(&mut g,defs,"Shock",0,1,&mut d,&mut h)?;checks.push(if mode=="prevent1"{json!({"check":"one_of_two_damage_prevented","expected":1,"observed":g.damage_on(target.unwrap())})}else{json!({"check":"two_damage_prevented","expected":20,"observed":g.player(PlayerId(0)).unwrap().life})});},
   _=>return Err("unknown expectation".into())
  }
    } else if !want && !legal {
        checks.push(json!({"check":"unavailable_cost_does_not_tap_resources","expected":before["resource_tapped"],"observed":resources.iter().map(|id|g.is_tapped(*id)).collect::<Vec<_>>()}));
    }
    let pass = checks.iter().all(|c| c["expected"] == c["observed"]);
    Ok(
        json!({"card":name,"scenario":variant,"status":if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"expected":{"action_offered":want,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["expected"]})).collect::<Vec<_>>()},"actual":{"action_offered":legal,"action":action,"before":before,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["observed"]})).collect::<Vec<_>>()},"checks":checks,"fixture_evidence":{"history":h,"decision_trace":d.trace,"canonical_index":index,"live_index":live,"source":source.0,"selected_resource":selected.map(|id|id.0),"resource_ids":resources.iter().map(|id|id.0).collect::<Vec<_>>(),"source_self_allowed_by_oracle":case["source_self"],"resource_name":resource_name,"resource_fresh":resources.iter().map(|id|g.is_summoning_sick(*id)).collect::<Vec<_>>()}}),
    )
}
#[test]
#[ignore = "Single tap direct-effect cost outcomes and actual resource controls"]
fn report_single_tap_effect_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/single-tap-effect-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_single_tap_effect_reproductions.rs"),
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
    let filter = std::env::var("SINGLE_TAP_EFFECT_CARD").ok();
    let mut rows = vec![];
    for case in payloads["cases"].as_array().unwrap() {
        let name = case["card"].as_str().unwrap();
        if filter.as_ref().is_some_and(|s| s != name) {
            continue;
        }
        let variants = ["zero", "exact", "surplus", "tapped", "ineligible"];
        for v in variants {
            println!("stage {name} {v}");
            match effect_trial(&defs, case, v) {
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
    let report = json!({"scope":"Twenty-nine direct-effect single tap paths, canonical paid sources and resources. Source-self eligibility follows Oracle; no synthetic resource/state repair. Exact and surplus resources, real tapped and ineligible controls; independent effect expectations.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    let filename = std::env::var("SINGLE_TAP_EFFECT_REPORT")
        .unwrap_or("single-tap-effect-execution.json".into());
    std::fs::write(
        root.join("reports/runtime-audit").join(filename),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
