//! Alternative tap-cost selectors: exact typed Waterbend branches, canonical paid sources.
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
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":true}));
        true
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let s = if c.description.starts_with("Choose an activation cost") {
            let branch = self.branch.expect("explicit alternative selector");
            assert!(
                c.options.iter().any(|o| o.index == branch && o.legal),
                "fixture cannot select an unadvertised branch"
            );
            vec![branch]
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
fn activate_branch(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    dm: &mut Choices,
) -> Result<Value, String> {
    let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    let Some(action) = action else {
        return Ok(json!({"action_offered":false,"branch_offered":null,"resolved":false}));
    };
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    let mut selectors = vec![];
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(action.clone()),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..96 {
        if st.pending_activation.is_none() && !g.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("activation stopped:{progress:?}"));
        };
        if let DecisionContext::SelectOptions(c) = &ctx {
            if c.description.starts_with("Choose an activation cost") {
                let branch = dm.branch.unwrap();
                let legal = c.options.iter().any(|o| o.index == branch && o.legal);
                selectors.push(json!({"description":c.description,"desired_index":branch,"desired_legal":legal,"options":c.options.iter().map(|o|json!({"index":o.index,"description":o.description,"legal":o.legal})).collect::<Vec<_>>()}));
                if !legal {
                    return Ok(
                        json!({"action_offered":true,"branch_offered":false,"selector":selectors,"resolved":false,"mana_paid":before-g.player(PlayerId(0)).unwrap().mana_pool.total(),"first_gate":"alternative_cost_selector"}),
                    );
                }
            }
        }
        if matches!(ctx, DecisionContext::Priority(_)) {
            return Err("activation returned priority without stack".into());
        }
        progress = match apply_decision_context_with_dm(g, &mut q, &mut st, &ctx, dm) {
            Ok(p) => p,
            Err(e) => {
                return Ok(
                    json!({"action_offered":true,"branch_offered":true,"selector":selectors,"resolved":false,"announcement_error":e.to_string(),"first_gate":"cost_payment","mana_paid":before-g.player(PlayerId(0)).unwrap().mana_pool.total()}),
                );
            }
        };
    }
    if st.pending_activation.is_some() {
        return Err("activation decision budget".into());
    }
    let paid = before - g.player(PlayerId(0)).unwrap().mana_pool.total();
    let error = finish(g, &mut q, dm).err();
    Ok(
        json!({"action_offered":true,"branch_offered":true,"selector":selectors,"resolved":error.is_none(),"mana_paid":paid,"resolution_error":error,"first_gate":if error.is_some(){"resolution"}else{"completed"}}),
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
                if a.mana_cost == expected.mana_cost && a.timing == expected.timing =>
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

const SOURCES: [(&str, usize, u32, u32); 14] = [
    ("Aang's Iceberg", 2, 3, 3),
    ("Aang, Swift Savior", 3, 8, 3),
    ("Aang, Swift Savior // Aang and La, Ocean's Fury", 3, 8, 3),
    ("Avatar Kuruk", 1, 20, 0),
    ("Flexible Waterbender", 1, 3, 4),
    ("Foggy Swamp Vinebender", 1, 5, 4),
    ("Geyser Leaper", 1, 4, 5),
    ("Giant Koi", 0, 3, 6),
    ("Invasion Submersible", 1, 3, 3),
    ("Katara, Bending Prodigy", 1, 6, 3),
    ("Ruthless Waterbender", 0, 2, 2),
    ("Water Tribe Rallier", 0, 5, 2),
    ("Waterbender Ascension", 1, 4, 2),
    ("Watery Grasp", 1, 5, 1),
];
fn state(g: &GameState, stable: u64, resources: &[ObjectId], target: Option<ObjectId>) -> Value {
    let source = find_stable(g, stable);
    json!({"source":source.and_then(|id|g.object(id).map(|o|json!({"id":id.0,"name":o.name.to_string(),"zone":format!("{:?}",o.zone),"tapped":g.is_tapped(id),"fresh":g.is_summoning_sick(id),"power":g.calculated_power(id),"toughness":g.calculated_toughness(id),"counters":format!("{:?}",o.counters),"card_types":format!("{:?}",o.card_types)}))),
 "resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string()),"tapped":g.is_tapped(*id),"fresh":g.is_summoning_sick(*id),"artifact":g.current_has_card_type(*id,CardType::Artifact),"creature":g.current_has_card_type(*id,CardType::Creature)})).collect::<Vec<_>>(),
 "untapped_eligible":g.battlefield.iter().filter(|id|g.current_controller(**id)==Some(PlayerId(0))&&!g.is_tapped(**id)&&(g.current_has_card_type(**id,CardType::Creature)||g.current_has_card_type(**id,CardType::Artifact))).map(|id|id.0).collect::<Vec<_>>(),
 "hand":g.player(PlayerId(0)).unwrap().hand.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string()})).collect::<Vec<_>>(),
 "library_len":g.player(PlayerId(0)).unwrap().library.len(),"graveyard":g.player(PlayerId(0)).unwrap().graveyard.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>(),
 "target_zone":target.and_then(|id|g.object(id).map(|o|format!("{:?}",o.zone))),"extra_turns":g.turn_store.extra_turns.iter().map(|p|p.index()).collect::<Vec<_>>(),"random_count":g.irreversible_random_count(),"mana":g.player(PlayerId(0)).unwrap().mana_pool.total()})
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    canonical: usize,
    n: u32,
    cast_cost: u32,
    branch: u32,
    variant: &str,
) -> Result<Value, String> {
    let mut g = new_game(defs);
    let mut d = Choices::default();
    let mut history = vec![];
    let mut resources = vec![];
    // Initial hand/library and mana are explicit fixtures. Every battlefield source/resource is a real paid cast.
    let count = if variant == "noneligible" {
        n as usize
    } else if variant == "no_resources" {
        0
    } else {
        branch as usize
    };
    for i in 0..count {
        fund(&mut g, 0);
        let (rn, cost) = if variant == "noneligible" {
            ("Fervor", 3)
        } else {
            match i % 3 {
                0 => ("Grizzly Bears", 2),
                1 => ("Tormod's Crypt", 0),
                _ => ("Ornithopter", 0),
            }
        };
        resources.push(cast_creature(
            &mut g,
            defs,
            rn,
            0,
            cost,
            &mut d,
            &mut history,
        )?);
    }
    if variant == "tapped_resources" {
        for id in resources.clone() {
            fund(&mut g, 0);
            tap(&mut g, defs, id, &mut d, &mut history)?;
        }
    }
    let target = if name == "Waterbender Ascension" || name == "Watery Grasp" {
        fund(&mut g, 1);
        Some(cast_creature(
            &mut g,
            defs,
            "Spectral Sailor",
            1,
            1,
            &mut d,
            &mut history,
        )?)
    } else {
        None
    };
    fund(&mut g, 0);
    d.targets.clear();
    let source = if name == "Avatar Kuruk" {
        let hand = g.create_object_from_definition(&defs[name], PlayerId(0), Zone::Hand);
        let stable = get_stable(&g, hand);
        d.discard_id = Some(hand);
        paid(
            &mut g,
            defs,
            "Faithless Looting",
            0,
            1,
            &mut d,
            &mut history,
        )?;
        let gy = find_stable(&g, stable).ok_or("Kuruk vanished")?;
        if g.object(gy).unwrap().zone != Zone::Graveyard {
            return Err("paid Looting did not discard Kuruk".into());
        }
        d.discard_id = None;
        d.targets = vec![Target::Object(gy)];
        paid(&mut g, defs, "Reanimate", 0, 1, &mut d, &mut history)?;
        let id = find_stable(&g, stable).ok_or("reanimated Kuruk vanished")?;
        if g.object(id).unwrap().zone != Zone::Battlefield {
            return Err("paid Reanimate did not put Kuruk on battlefield".into());
        }
        id
    } else {
        let prior = g.battlefield.to_vec();
        d.targets = target.into_iter().map(Target::Object).collect();
        paid(&mut g, defs, name, 0, cast_cost, &mut d, &mut history)?;
        g.battlefield
            .iter()
            .copied()
            .find(|id| {
                !prior.contains(id) && g.object(*id).is_some_and(|o| o.name == defs[name].name())
            })
            .ok_or("paid source missing battlefield")?
    };
    // Guaranteed independent draw/loot/search fixtures, installed before the audited activation.
    g.create_object_from_definition(&defs["Serra Angel"], PlayerId(0), Zone::Hand);
    if name == "Water Tribe Rallier" {
        g.create_object_from_definition(&defs["Grizzly Bears"], PlayerId(0), Zone::Library);
    }
    let stable = get_stable(&g, source);
    let live = intrinsic_index(&g, source, &defs[name], canonical)?;
    d.targets = if name == "Waterbender Ascension" {
        target.into_iter().map(Target::Object).collect()
    } else {
        vec![]
    };
    d.resources = resources.clone();
    d.branch = Some(branch as usize);
    d.discard_id = Some(*g.player(PlayerId(0)).unwrap().hand.last().unwrap());
    // Fund only the printed remainder for positive branches; controls fund all mana to expose illegal alternatives without forcing them.
    g.player_mut(PlayerId(0)).unwrap().mana_pool = Default::default();
    let mana = if variant == "positive" { n - branch } else { n };
    g.player_mut(PlayerId(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, mana);
    g.turn.priority_player = Some(PlayerId(0));
    let before = state(&g, stable, &resources, target);
    let expected_legal = variant == "positive";
    let current = g.current_abilities(source).unwrap();
    let ironsmith::ability::AbilityKind::Activated(a) = &current[live].kind else {
        return Err("live index not activated".into());
    };
    let ironsmith_core::TotalCostKind::OneOf(branches) = a.mana_cost.kind() else {
        return Err("canonical cost not OneOf".into());
    };
    if branches.len() != n as usize + 1 {
        return Err("unexpected number of typed alternatives".into());
    }
    let branch_text = branches[branch as usize].display();
    let diagnostic = json!({"canonical_index":canonical,"live_index":live,"branch_index":branch,"typed_branch_display":branch_text,"branch_count":branches.len(),"legal_actions":format!("{:?}",compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state"))});
    let trace_start = d.trace.len();
    let action = match activate_branch(&mut g, source, live, &mut d) {
        Ok(a) => a,
        Err(e) => json!({"announcement_error":e,"resolved":false}),
    };
    let after = state(&g, stable, &resources, target);
    let branch_offered = action["branch_offered"].as_bool().unwrap_or(false);
    let mut checks = vec![
        json!({"check":"desired_alternative_offered","expected":expected_legal,"observed":branch_offered}),
    ];
    let mut outcome_scope = "first_cost_gate_only";
    if expected_legal && branch_offered {
        checks.push(json!({"check":"printed_mana_remainder_paid","expected":n-branch,"observed":action["mana_paid"]}));
        checks.push(json!({"check":"exact_chosen_resources_tapped","expected":branch,"observed":resources.iter().filter(|id|g.is_tapped(**id)).count()}));
        checks.push(json!({"check":"unselected_source_remains_untapped","expected":false,"observed":g.is_tapped(source)}));
        if !name.starts_with("Aang, Swift Savior") {
            checks.push(json!({"check":"resolved_without_exception","expected":true,"observed":action["resolved"]}));
            if action["resolved"] == true {
                outcome_scope = "cost_and_specific_effect";
                match name {
     "Aang's Iceberg"=>checks.push(json!({"check":"source_sacrificed","expected":"Graveyard","observed":after["source"]["zone"]})),
     "Avatar Kuruk"=>checks.push(json!({"check":"one_extra_turn","expected":[0],"observed":after["extra_turns"]})),
     "Flexible Waterbender"=>checks.push(json!({"check":"base_power_toughness","expected":[5,2],"observed":[after["source"]["power"],after["source"]["toughness"]]})),
     "Foggy Swamp Vinebender"=>checks.push(json!({"check":"one_counter_power_toughness","expected":[5,4],"observed":[after["source"]["power"],after["source"]["toughness"]]})),
     "Geyser Leaper"=>{checks.push(json!({"check":"draw_one_discard_one","expected":[before["hand"].as_array().unwrap().len(),before["library_len"].as_u64().unwrap()-1],"observed":[after["hand"].as_array().unwrap().len(),after["library_len"].as_u64().unwrap()]}));checks.push(json!({"check":"explicit_discard_serra_angel","expected":true,"observed":after["graveyard"].as_array().unwrap().contains(&json!("Serra Angel"))}));},
     "Giant Koi"=>checks.push(json!({"check":"cannot_be_blocked","expected":false,"observed":g.can_be_blocked(source)})),
     "Invasion Submersible"=>{checks.push(json!({"check":"creature_three_three","expected":[true,3,3],"observed":[g.current_has_card_type(source,CardType::Creature),g.calculated_power(source),g.calculated_toughness(source)]}));},
     "Katara, Bending Prodigy"=>checks.push(json!({"check":"draw_one","expected":[before["hand"].as_array().unwrap().len()+1,before["library_len"].as_u64().unwrap()-1],"observed":[after["hand"].as_array().unwrap().len(),after["library_len"].as_u64().unwrap()]})),
     "Ruthless Waterbender"=>checks.push(json!({"check":"plus_one_one","expected":[2,4],"observed":[after["source"]["power"],after["source"]["toughness"]]})),
     "Water Tribe Rallier"=>checks.push(json!({"check":"eligible_bear_to_hand_other_three_bottom","expected":[before["hand"].as_array().unwrap().len()+1,before["library_len"].as_u64().unwrap()-1,true],"observed":[after["hand"].as_array().unwrap().len(),after["library_len"].as_u64().unwrap(),after["hand"].as_array().unwrap().iter().any(|r|r["name"]=="Grizzly Bears")]})),
     "Waterbender Ascension"=>checks.push(json!({"check":"target_cannot_be_blocked","expected":false,"observed":g.can_be_blocked(target.unwrap())})),
     "Watery Grasp"=>checks.push(json!({"check":"enchanted_creature_into_owners_library","expected":["Graveyard",31,1],"observed":[after["source"]["zone"],g.player(PlayerId(1)).unwrap().library.len(),after["random_count"].as_u64().unwrap()-before["random_count"].as_u64().unwrap()]})),
     _=>return Err("unexpected source".into())
    }
                if name == "Avatar Kuruk" || name == "Invasion Submersible" {
                    fund(&mut g, 0);
                    checks.push(json!({"check":"exhaust_once_unavailable_after_resolution","expected":false,"observed":offered(&g,source,live)}));
                }
            }
        } else {
            outcome_scope = "cost_only_no_linked_backface_in_canonical_payload";
        }
    }
    if !expected_legal && !branch_offered {
        checks.push(json!({"check":"no_payment_for_rejected_branch","expected":[before["mana"],before["resources"]],"observed":[after["mana"],after["resources"]]}));
    }
    let pass = checks.iter().all(|c| c["expected"] == c["observed"]);
    Ok(
        json!({"card":name,"scenario":format!("branch_{branch}_{variant}"),"branch":branch,"waterbend":n,"variant":variant,"status":if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"outcome_scope":outcome_scope,"checks":checks,"expected":{"branch_offered":expected_legal,"mana_paid":if expected_legal{Some(n-branch)}else{None},"tap_count":if expected_legal{Some(branch)}else{None}},"actual":{"action":action,"before":before,"after":after},"fixture_evidence":{"history":history,"diagnostic":diagnostic,"decision_trace":d.trace,"activation_trace_start":trace_start}}),
    )
}
#[test]
#[ignore = "full canonical alternative tap-cost branches and exact legal selectors"]
fn report_alternative_tap_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/alternative-tap-cost-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_alternative_tap_cost_reproductions.rs"),
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
    let filter = std::env::var("ALTERNATIVE_TAP_CARD").ok();
    let mut rows = vec![];
    for (name, index, n, cost) in SOURCES {
        if filter.as_ref().is_some_and(|f| f != name) {
            continue;
        }
        for branch in 0..=n {
            println!("stage {name} branch{branch}");
            match trial(&defs,name,index,n,cost,branch,"positive"){Ok(r)=>rows.push(r),Err(e)=>rows.push(json!({"card":name,"scenario":format!("branch_{branch}_positive"),"branch":branch,"variant":"positive","status":"fixture_error","actual":{"error":e}}))}
        }
        for variant in ["no_resources", "noneligible", "tapped_resources"] {
            println!("stage {name} {variant}");
            match trial(&defs,name,index,n,cost,n,variant){Ok(r)=>rows.push(r),Err(e)=>rows.push(json!({"card":name,"scenario":format!("branch_{n}_{variant}"),"branch":n,"variant":variant,"status":"fixture_error","actual":{"error":e}}))}
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"scope":"All 79 inventoried OneOf Tap consumer paths plus full-mana alternatives and nonresource/tapped controls. Every selected branch must be advertised legal. Full canonical paid sources and resources; Avatar Kuruk actual paid discard and reanimation. Aang transform effect excluded without canonical backface metadata.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    let filename = std::env::var("ALTERNATIVE_TAP_REPORT")
        .unwrap_or("alternative-tap-cost-execution.json".into());
    std::fs::write(
        root.join("reports/runtime-audit").join(filename),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
