//! Intrinsic unattach costs: Captain America and Sunforger, with exact paid Equipment sources.
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
    distribution_count: usize,
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
    fn decide_distribute(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        let selected = self
            .targets
            .iter()
            .copied()
            .filter(|t| c.targets.iter().any(|x| x.target == *t))
            .take(self.distribution_count)
            .collect::<Vec<_>>();
        let n = selected.len() as u32;
        let result = if n > 0 && c.total >= n * c.min_per_target {
            selected
                .iter()
                .enumerate()
                .map(|(i, t)| (*t, c.total / n + u32::from((i as u32) < c.total % n)))
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        self.trace.push(json!({"choice":"distribute","context":format!("{c:?}"),"total":c.total,"min_per_target":c.min_per_target,"selected":format!("{result:?}"),"allocated_total":result.iter().map(|(_,n)|*n).sum::<u32>()}));
        result
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

use ironsmith::static_abilities::StaticAbilityId;

const NAMES: [(&str, usize); 2] = [("Captain America, First Avenger", 0), ("Sunforger", 1)];
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

fn equip(
    g: &mut GameState,
    def: &CardDefinition,
    eq: ObjectId,
    canonical_index: usize,
    cost: u32,
    host: ObjectId,
    d: &mut Choices,
    h: &mut Vec<Value>,
) -> Result<(), String> {
    let ironsmith::ability::AbilityKind::Activated(expected) = &def.abilities[canonical_index].kind
    else {
        return Err("canonical equip index not activated".into());
    };
    let indices = g
        .current_abilities(eq)
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter_map(|(i, a)| match &a.kind {
            ironsmith::ability::AbilityKind::Activated(a)
                if a.mana_cost == expected.mana_cost
                    && a.effects.flattened_default_effects().iter().any(|e| {
                        e.downcast_ref::<ironsmith::effects::AttachObjectsEffect>()
                            .is_some()
                    }) =>
            {
                Some(i)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if indices.len() != 1 {
        return Err(format!(
            "exact current equip match ambiguous or missing:{indices:?}"
        ));
    }
    let index = indices[0];
    d.targets = vec![Target::Object(host)];
    let ev=activate(g,eq,index,d).map_err(|e|format!("equip actual index{index} canonical index{canonical_index} failed:{e}; advertised:{:?}",compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state")))?;
    if ev["mana_paid"] != cost || !ev["resolution_error"].is_null() {
        return Err(format!("equip payment/resolution:{ev}"));
    }
    if g.object(eq).unwrap().attached_to != Some(ironsmith::object::AttachmentTarget::Object(host))
    {
        return Err("normal equip did not attach intended Equipment to intended host".into());
    }
    h.push(json!({"producer":"normal paid equip","equipment":eq.0,"host":host.0,"canonical_equipment_equip_index":canonical_index,"actual_equipment_equip_index":index,"evidence":ev}));
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
fn snapshot(
    g: &GameState,
    eq_stable: u64,
    host: ObjectId,
    target: ObjectId,
    library_card: Option<u64>,
) -> Value {
    let eq = find_stable(g, eq_stable);
    json!({"equipment":eq.map(|id|json!({"id":id.0,"zone":format!("{:?}",g.object(id).unwrap().zone),"attached_to_host":g.object(id).unwrap().attached_to==Some(ironsmith::object::AttachmentTarget::Object(host)),"controller":g.current_controller(id).map(|p|p.index())})),"host":{"id":host.0,"tapped":g.is_tapped(host),"summoning_sick":g.is_summoning_sick(host),"power":g.calculated_power(host),"toughness":g.calculated_toughness(host)},"target":{"zone":g.object(target).map(|o|format!("{:?}",o.zone)),"damage":g.damage_on(target)},"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"library":g.player(PlayerId(0)).unwrap().library.iter().map(|id|get_stable(g,*id)).collect::<Vec<_>>(),"selected_library_card_zone":library_card.and_then(|s|find_stable(g,s)).map(|id|format!("{:?}",g.object(id).unwrap().zone)),"random_operation_count":g.irreversible_random_count(),"stack_len":g.stack.len()})
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    canonical_index: usize,
    v: usize,
) -> Result<Value, String> {
    let mut g = new_game(defs);
    g.turn.active_player = PlayerId(1);
    g.turn.priority_player = Some(PlayerId(1));
    fund(&mut g, 1);
    let mut d = Choices::default();
    let mut h = vec![];
    let mut q = TriggerQueue::new();
    let mut r = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
    let target = cast_creature(&mut g, defs, "Hill Giant", 1, 4, &mut d, &mut h)?;
    reach_main(&mut g, &mut r, &mut q, 0, 4, &mut h)?;
    fund(&mut g, 0);
    let captain = name == "Captain America, First Avenger";
    let host = cast_creature(
        &mut g,
        defs,
        if captain { name } else { "Grizzly Bears" },
        0,
        if captain { 3 } else { 2 },
        &mut d,
        &mut h,
    )?;
    let eq_name = if captain { "Heartseeker" } else { "Sunforger" };
    let equipment = cast_creature(
        &mut g,
        defs,
        eq_name,
        0,
        if captain { 4 } else { 3 },
        &mut d,
        &mut h,
    )?;
    let eq_stable = get_stable(&g, equipment);
    if v != 0 {
        equip(
            &mut g,
            &defs[eq_name],
            equipment,
            if captain { 3 } else { 2 },
            if captain { 5 } else { 3 },
            host,
            &mut d,
            &mut h,
        )?;
    }
    let tapped = if captain { v == 4 } else { v == 2 };
    if tapped {
        tap(&mut g, defs, host, &mut d, &mut h)?;
    }
    let mut search_card = None;
    let valid_shock = !captain && v <= 2;
    if !captain && v != 6 {
        let card = match v {
            0..=2 => "Shock",
            3 => "Cancel",
            4 => "Fireblast",
            _ => "Stone Rain",
        };
        let id = g.create_object_from_definition(&defs[card], PlayerId(0), Zone::Library);
        search_card = Some(get_stable(&g, id));
        d.put_id = Some(id);
        h.push(json!({"fixture_library_card":card,"stable":search_card,"printed_oracle_eligible":valid_shock}));
    }
    priority(&mut g, &mut q, 0, &mut d)?;
    let source = if captain { host } else { equipment };
    let index = intrinsic_index(&g, source, &defs[name], canonical_index)?;
    let target_count = if captain {
        if v == 0 || v == 4 { 1 } else { v }
    } else {
        1
    };
    d.targets = vec![Target::Player(PlayerId(1))];
    if captain && target_count >= 2 {
        d.targets.push(Target::Object(target));
    }
    if captain && target_count >= 3 {
        d.targets.push(Target::Player(PlayerId(0)));
    }
    d.distribution_count = target_count;
    d.return_id = Some(equipment);
    d.trace.clear();
    let before = snapshot(&g, eq_stable, host, target, search_card);
    let is_offered = offered(&g, source, index);
    let want = v > 0;
    let a = g.current_ability(source, index).unwrap();
    let ironsmith::ability::AbilityKind::Activated(a) = &a.kind else {
        return Err("selected intrinsic ability not activated".into());
    };
    let ctx = ironsmith::costs::CostCheckContext::new(source, PlayerId(0))
        .with_reason(ironsmith::costs::PaymentReason::ActivateAbility);
    let diagnostic = json!({"canonical_source_ability_index":canonical_index,"selected_live_source_ability_index":index,"source":source.0,"current_ability":format!("{a:?}"),"component_checks":a.mana_cost.costs().iter().map(|c|json!({"cost":format!("{c:?}"),"check":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*c.0,&g,&ctx))})).collect::<Vec<_>>(),"active":g.turn.active_player.index(),"phase":format!("{:?}",g.turn.phase),"priority":g.turn.priority_player.map(|p|p.index()),"legal_actions":format!("{:?}",compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state"))});
    let mana_before_resolution = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut action = if want && is_offered {
        match activate(&mut g, source, index, &mut d) {
            Ok(x) => x,
            Err(e) => json!({"error":e}),
        }
    } else {
        Value::Null
    };
    if want && is_offered {
        action["total_mana_after_resolution"] =
            json!(mana_before_resolution - g.player(PlayerId(0)).unwrap().mana_pool.total());
    }
    let after = snapshot(&g, eq_stable, host, target, search_card);
    let mut checks =
        vec![json!({"check":"intended_activation_offered","expected":want,"observed":is_offered})];
    if want && is_offered {
        checks.extend([json!({"check":"total_mana_paid_including_nested_cast","expected":if captain{3}else{2},"observed":action["total_mana_after_resolution"]}),json!({"check":"resolution_error","expected":null,"observed":action.get("error").or_else(||action.get("resolution_error")).cloned().unwrap_or(Value::Null)}),json!({"check":"equipment_unattached_battlefield","expected":["Battlefield",false],"observed":[after["equipment"]["zone"],after["equipment"]["attached_to_host"]]}),json!({"check":"host_returns_to_base_power_toughness","expected":if captain{vec![4,4]}else{vec![2,2]},"observed":[after["host"]["power"],after["host"]["toughness"]]}),json!({"check":"no_tap_cost_on_host","expected":before["host"]["tapped"],"observed":after["host"]["tapped"]})]);
        if captain {
            let (a, b, hill) = match target_count {
                1 => (20, 16, 0),
                2 => (20, 18, 2),
                _ => (19, 18, 1),
            };
            checks.push(json!({"check":"equipment_mana_value_four_distributed_damage","expected":[a,b,hill],"observed":[after["life"][0],after["life"][1],after["target"]["damage"]]}));
        } else {
            checks.push(json!({"check":"free_eligible_spell_or_no_match","expected":[if valid_shock{18}else{20},if valid_shock{"Graveyard"}else if search_card.is_some(){"Library"}else{"none"}],"observed":[after["life"][1],after["selected_library_card_zone"].as_str().unwrap_or("none")]}));
            checks.push(json!({"check":"one_real_shuffle","expected":1,"observed":after["random_operation_count"].as_u64().unwrap()-before["random_operation_count"].as_u64().unwrap()}));
            checks.push(json!({"check":"library_card_count","expected":before["library"].as_array().unwrap().len()-usize::from(valid_shock),"observed":after["library"].as_array().unwrap().len()}));
        }
    }
    let pass = checks.iter().all(|c| c["expected"] == c["observed"]);
    Ok(
        json!({"card":name,"scenario":if captain{match v{0=>"unattached_equipment",1=>"fresh_host_divide_among_one",2=>"fresh_host_divide_among_two",3=>"fresh_host_divide_among_three",_=>"tapped_host_divide_among_one"}}else{match v{0=>"unattached_equipment",1=>"fresh_host_valid_shock",2=>"tapped_host_valid_shock",3=>"only_ineligible_blue_instant",4=>"only_ineligible_mana_value_six",5=>"only_ineligible_red_sorcery",_=>"no_instant_in_library"}},"status":if pass{"expected_outcome_passed"}else{"outcome_mismatch"},"checks":checks,"expected":{"action_offered":want,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["expected"]})).collect::<Vec<_>>()},"actual":{"action_offered":is_offered,"action":action,"before":before,"after":after,"checks":checks.iter().map(|c|json!({"check":c["check"],"value":c["observed"]})).collect::<Vec<_>>()},"fixture_evidence":{"history":h,"diagnostic":diagnostic,"decision_trace":d.trace}}),
    )
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
#[ignore = "intrinsic unattach costs, full canonical sources and actual paid equip"]
fn report_unattach_intrinsic_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/unattach-cost-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_unattach_intrinsic_reproductions.rs"),
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
        for variant in 0..if name == "Captain America, First Avenger" {
            5
        } else {
            7
        } {
            println!("stage: {name} variant{variant}");
            match trial(&defs,name,index,variant){Ok(r)=>rows.push(r),Err(e)=>rows.push(json!({"card":name,"scenario":{"variant":variant},"status":"fixture_error","actual":{"error":e}}))}
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
    let path_coverage=inventory["rows"].as_array().unwrap().iter().filter(|p|p["consumer_kind"]=="UnattachObjectsEffect"&&NAMES.iter().any(|(n,_)|p["card"]==*n)).map(|p|json!({"card":p["card"],"path":p["path"],"consumer_path":p["consumer_path"],"source_rows":rows.iter().enumerate().filter(|(_,r)|r["card"]==p["card"]).map(|(i,_)|i).collect::<Vec<_>>()})).collect::<Vec<_>>();
    let report = json!({"scope":"Two intrinsic unattach paths: full paid Captain or Sunforger, actual paid equip, fresh and actually tapped host positives, unattached negatives; distribution and eligible/ineligible library fixtures prepared. First gate only when action unavailable, no forced action.","rows":rows,"path_coverage":path_coverage,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/unattach-intrinsic-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
