//! Own-tap-symbol plus single chosen resource: exact canonical source, actual aging and resources.
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

fn checkpoint(g: &GameState, source: ObjectId, resources: &[ObjectId]) -> Value {
    json!({"source_tapped":g.is_tapped(source),"mana":g.player(PlayerId(0)).unwrap().mana_pool.total(),"life":[g.player(PlayerId(0)).unwrap().life,g.player(PlayerId(1)).unwrap().life],"hand":g.player(PlayerId(0)).unwrap().hand.iter().map(|id|id.0).collect::<Vec<_>>(),"library":g.player(PlayerId(0)).unwrap().library.iter().map(|id|id.0).collect::<Vec<_>>(),"source_plus":g.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),"resources":resources.iter().map(|id|json!({"id":id.0,"tapped":g.is_tapped(*id),"plus":g.counter_count(*id,ironsmith::CounterType::PlusOnePlusOne)})).collect::<Vec<_>>(),"tokens":g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.kind==ironsmith::object::ObjectKind::Token)).map(|id|id.0).collect::<Vec<_>>(),"stack":g.stack.len()})
}
fn source_trial(
    defs: &HashMap<String, CardDefinition>,
    case: &Value,
    variant: &str,
) -> Result<Value, String> {
    let name = case["card"].as_str().unwrap();
    let mut g = new_game(defs);
    let mut d = Choices::default();
    let mut h = vec![];
    let target = if ["Spawnbinder Mage", "Wanderbrine Trapper", "Revelsong Horn"].contains(&name) {
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
    if name == "Heap Gate" {
        paid(&mut g, defs, "Exploration", 0, 1, &mut d, &mut h)?;
    }
    let def = &defs[name];
    let source = if def.card.card_types.contains(&CardType::Land) {
        land(&mut g, def, 0, &mut d, &mut h)?
    } else {
        cast_creature(
            &mut g,
            defs,
            name,
            0,
            def.card.mana_cost.as_ref().unwrap().mana_value(),
            &mut d,
            &mut h,
        )?
    };
    if variant != "fresh" {
        let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
        reach_main(&mut g, &mut runner, &mut TriggerQueue::new(), 0, 5, &mut h)?;
    }
    fund(&mut g, 0);
    let count = match variant {
        "zero" => 0,
        "surplus" => 2,
        _ => 1,
    };
    let resource_name = match name {
        "Heap Gate" => "Basilisk Gate",
        "Network Terminal" => "Ornithopter",
        _ => "Universal Automaton",
    };
    let mut resources = vec![];
    for _ in 0..count {
        resources.push(if resource_name == "Basilisk Gate" {
            land(&mut g, &defs[resource_name], 0, &mut d, &mut h)?
        } else {
            cast_creature(
                &mut g,
                defs,
                resource_name,
                0,
                defs[resource_name]
                    .card
                    .mana_cost
                    .as_ref()
                    .unwrap()
                    .mana_value(),
                &mut d,
                &mut h,
            )?
        });
    }
    if variant == "source_tapped" {
        tap(&mut g, defs, source, &mut d, &mut h)?;
    }
    if variant == "all_tapped" {
        for id in resources.clone() {
            tap(&mut g, defs, id, &mut d, &mut h)?;
        }
        if !g.is_tapped(source) {
            tap(&mut g, defs, source, &mut d, &mut h)?;
        }
    }
    if ["Akoum Flameseeker", "Network Terminal"].contains(&name) {
        let id = g.create_object_from_definition(&defs["Serra Angel"], PlayerId(0), Zone::Hand);
        d.discard_id = Some(id);
    }
    if name == "Stoneforge Acolyte" {
        for card in ["Forest", "Island", "Mountain", "Shuko"] {
            g.create_object_from_definition(&defs[card], PlayerId(0), Zone::Library);
        }
    }
    let index = case["index"].as_u64().unwrap() as usize;
    let live = intrinsic_index(&g, source, def, index)?;
    d.resources = resources.first().copied().into_iter().collect();
    d.targets = if let Some(id) = target {
        vec![Target::Object(id)]
    } else if ["Zada's Commando", "Zulaport Chainmage"].contains(&name) {
        vec![Target::Player(PlayerId(1))]
    } else {
        vec![]
    };
    let want = match variant {
        "zero" | "source_tapped" | "all_tapped" => false,
        "fresh" => !g.current_has_card_type(source, CardType::Creature) && !g.is_tapped(source),
        _ => true,
    };
    let before = checkpoint(&g, source, &resources);
    let legal = offered(&g, source, live);
    // Unexpectedly advertised negative cases are followed through their own legal-action entry point; no missing action is forced.
    let action = if legal {
        activate(&mut g, source, live, &mut d).unwrap_or_else(|e| json!({"announcement_error":e}))
    } else {
        Value::Null
    };
    let after = checkpoint(&g, source, &resources);
    let mut checks =
        vec![json!({"check":"intended_activation_offered","expected":want,"observed":legal})];
    let transform = name.starts_with("Chosen of Markov") || name.starts_with("Town Gossipmonger");
    if want && legal {
        let mana = if [
            "Heap Gate",
            "Network Terminal",
            "Revelsong Horn",
            "Selesnya Evangel",
            "Wanderbrine Trapper",
        ]
        .contains(&name)
        {
            1
        } else if ["Stardew Valley", "The Shire"].contains(&name) {
            2
        } else {
            0
        };
        checks.extend([json!({"check":"exact_mana_cost_paid","expected":mana,"observed":action["mana_paid"]}),json!({"check":"source_tap_symbol_paid","expected":true,"observed":g.is_tapped(source)}),json!({"check":"one_distinct_chosen_resource_tapped","expected":[1,count-1],"observed":[resources.iter().filter(|id|g.is_tapped(**id)).count(),resources.iter().filter(|id|!g.is_tapped(**id)).count()]})]);
        if !transform {
            checks.push(json!({"check":"resolution_completed","expected":null,"observed":action.get("announcement_error").or_else(||action.get("resolution_error")).cloned().unwrap_or(Value::Null)}));
            match name{
    "Akoum Flameseeker"|"Network Terminal"=>{checks.push(json!({"check":"draw_discard_exact_card_and_counts","expected":[before["hand"].as_array().unwrap().len(),before["library"].as_array().unwrap().len()-1,true],"observed":[g.player(PlayerId(0)).unwrap().hand.len(),g.player(PlayerId(0)).unwrap().library.len(),g.player(PlayerId(0)).unwrap().graveyard.iter().any(|id|g.object(*id).is_some_and(|o|o.name=="Serra Angel"))]}));},
    "Malakir Soothsayer"=>checks.push(json!({"check":"draw_one_lose_one","expected":[before["hand"].as_array().unwrap().len()+1,before["library"].as_array().unwrap().len()-1,19],"observed":[g.player(PlayerId(0)).unwrap().hand.len(),g.player(PlayerId(0)).unwrap().library.len(),g.player(PlayerId(0)).unwrap().life]})),
    "Ondu War Cleric"=>checks.push(json!({"check":"gain_two","expected":22,"observed":g.player(PlayerId(0)).unwrap().life})),
    "Zada's Commando"=>checks.push(json!({"check":"opponent_takes_one_damage","expected":19,"observed":g.player(PlayerId(1)).unwrap().life})),
    "Zulaport Chainmage"=>checks.push(json!({"check":"opponent_loses_two","expected":18,"observed":g.player(PlayerId(1)).unwrap().life})),
    "Spawnbinder Mage"|"Wanderbrine Trapper"=>checks.push(json!({"check":"opponent_target_tapped","expected":true,"observed":g.is_tapped(target.unwrap())})),
    "Revelsong Horn"=>checks.push(json!({"check":"target_plus_one_one","expected":[4,4],"observed":[g.calculated_power(target.unwrap()),g.calculated_toughness(target.unwrap())]})),
    "Munda's Vanguard"=>{let mut observed=vec![g.counter_count(source,ironsmith::CounterType::PlusOnePlusOne)];observed.extend(resources.iter().map(|id|g.counter_count(*id,ironsmith::CounterType::PlusOnePlusOne)));checks.push(json!({"check":"one_counter_on_each_own_creature","expected":vec![1;resources.len()+1],"observed":observed}));},
    "Stoneforge Acolyte"=>{let bottom=g.player(PlayerId(0)).unwrap().library.iter().take(3).map(|id|g.object(*id).unwrap().name.to_string()).collect::<std::collections::BTreeSet<_>>();checks.push(json!({"check":"equipment_hand_remainder_bottom","expected":[true,vec!["Forest","Island","Mountain"],before["library"].as_array().unwrap().len()-1],"observed":[g.player(PlayerId(0)).unwrap().hand.iter().any(|id|g.object(*id).is_some_and(|o|o.name=="Shuko")),bottom,g.player(PlayerId(0)).unwrap().library.len()]}));},
    "Drana's Chosen"|"Selesnya Evangel"|"Heap Gate"|"Stardew Valley"|"The Shire"=>{let tokens=g.battlefield.iter().copied().filter(|id|g.object(*id).is_some_and(|o|o.kind==ironsmith::object::ObjectKind::Token)).collect::<Vec<_>>();checks.push(json!({"check":"one_real_token_created","expected":1,"observed":tokens.len()}));if tokens.len()==1{let id=tokens[0];let expected_name=if name=="Drana's Chosen"{"Zombie"}else if name=="Selesnya Evangel"{"Saproling"}else if name=="Heap Gate"{"Treasure"}else{"Food"};checks.push(json!({"check":"correct_token_name_and_tapped_state","expected":[expected_name,name=="Drana's Chosen"],"observed":[g.object(id).unwrap().name.to_string(),g.is_tapped(id)]}));if name=="Drana's Chosen"||name=="Selesnya Evangel"{checks.push(json!({"check":"token_exact_power_toughness_color","expected":[if name=="Drana's Chosen"{2}else{1},if name=="Drana's Chosen"{2}else{1},format!("{:?}",if name=="Drana's Chosen"{ironsmith::color::ColorSet::BLACK}else{ironsmith::color::ColorSet::GREEN})],"observed":[g.calculated_power(id),g.calculated_toughness(id),format!("{:?}",g.current_colors(id).unwrap())]}));}else{checks.push(json!({"check":"token_is_artifact","expected":true,"observed":g.current_has_card_type(id,CardType::Artifact)}));}}},
    _=>return Err("unexpected source effect".into())
   }
        }
    } else if !want {
        checks.push(
            json!({"check":"unpayable_negative_preserves_game","expected":before,"observed":after}),
        );
    }
    let pass = checks.iter().all(|c| c["expected"] == c["observed"]);
    Ok(
        json!({"card":name,"scenario":variant,"status":if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"scope":if transform{"cost_only_transform_excluded_without_linked_metadata"}else{"cost_and_listed_effect"},"expected":{"action_offered":want,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["expected"]})).collect::<Vec<_>>()},"actual":{"action_offered":legal,"action":action,"before":before,"after":after},"checks":checks,"fixture_evidence":{"canonical_index":index,"live_index":live,"source":source.0,"resource_ids":resources.iter().map(|id|id.0).collect::<Vec<_>>(),"resource_name":resource_name,"resource_fresh":resources.iter().map(|id|g.is_summoning_sick(*id)).collect::<Vec<_>>(),"history":h,"decision_trace":d.trace}}),
    )
}
#[test]
#[ignore = "Twenty own-tap-symbol and single chosen-resource cost paths"]
fn report_single_tap_source_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/single-tap-source-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_single_tap_source_reproductions.rs"),
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
    let filter = std::env::var("SINGLE_TAP_SOURCE_CARD").ok();
    let mut rows = vec![];
    for case in payloads["cases"].as_array().unwrap() {
        let name = case["card"].as_str().unwrap();
        if filter.as_ref().is_some_and(|s| s != name) {
            continue;
        }
        let variants = [
            "zero",
            "exact",
            "surplus",
            "fresh",
            "source_tapped",
            "all_tapped",
        ];
        for v in variants {
            println!("stage {name} {v}");
            match source_trial(&defs, case, v) {
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
    let report = json!({"scope":"Twenty own-tap-symbol plus single-object cost paths. Full canonical paid sources/resources, actual land plays and real turns/untaps, precise resource/mana/effect outcomes, zero/exact/surplus/fresh/source-tapped/all-tapped controls. Unexpectedly advertised negative actions followed through payment; no unavailable action forced. Transform effect semantics excluded without linked metadata.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    let filename = std::env::var("SINGLE_TAP_SOURCE_REPORT")
        .unwrap_or("single-tap-source-execution.json".into());
    std::fs::write(
        root.join("reports/runtime-audit").join(filename),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
