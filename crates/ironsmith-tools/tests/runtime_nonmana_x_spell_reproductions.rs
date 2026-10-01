//! Nonmana-X spell cost contexts through normal casting and actual resource producers.
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

struct Choices {
    amount: u32,
    actual_x: Option<u32>,
    targets: Vec<Target>,
    reveal_ids: Vec<ObjectId>,
    revealed: std::collections::BTreeSet<u64>,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        let yes = false;
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":yes}));
        yes
    }
    fn decide_number(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let selected = self.amount.max(c.min).min(c.max);
        if c.is_x_value {
            self.actual_x = Some(selected);
        }
        self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"selected":selected,"min":c.min,"max":c.max,"requested":self.amount,"is_x_value":c.is_x_value,"source_on_battlefield":c.source.is_some_and(|id|g.object(id).is_some_and(|o|o.zone==Zone::Battlefield)),"hand":g.player(c.player).unwrap().hand.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string()),"colors":format!("{:?}",g.current_colors(*id))})).collect::<Vec<_>>() }));
        selected
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let selected = if g
            .object(c.source)
            .is_some_and(|o| o.name == "Martyr of Bones")
        {
            self.targets
                .iter()
                .copied()
                .take(self.actual_x.unwrap_or(0) as usize)
                .collect()
        } else {
            self.targets.clone()
        };
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{selected:?}"),"requirement_bounds":c.requirements.iter().map(|r|json!({"min":r.min_targets,"max":r.max_targets})).collect::<Vec<_>>() }));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = c
            .options
            .iter()
            .find(|o| o.legal && o.description.to_lowercase().contains("reveal"))
            .map(|o| vec![o.index])
            .unwrap_or_else(|| ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c));
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if c.description.to_lowercase().contains("reveal") {
            self.reveal_ids
                .iter()
                .copied()
                .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
                .take(c.max.unwrap_or(self.reveal_ids.len()))
                .collect()
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"source_present":c.source.is_some_and(|id|g.object(id).is_some()),"selected":format!("{selected:?}")}));
        selected
    }
    fn view_cards(
        &mut self,
        g: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        c: &ironsmith::decisions::context::ViewCardsContext,
    ) {
        if c.public && c.zone == Zone::Hand {
            for id in cards {
                self.revealed.insert(id.0);
            }
        }
        self.trace.push(json!({"choice":"public_reveal","context":format!("{c:?}"),"viewer":viewer.index(),"cards":cards.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string()),"zone":g.object(*id).map(|o|format!("{:?}",o.zone))})).collect::<Vec<_>>() }));
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
fn cast_existing(
    g: &mut GameState,
    source: ObjectId,
    zone: Zone,
    d: &mut Choices,
) -> Result<Value, String> {
    g.turn.priority_player = Some(PlayerId(0));
    let action=compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,from_zone,..}if *spell_id==source&&*from_zone==zone)).ok_or("existing cast unavailable")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(action.clone()),
        d,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if st.pending_cast.is_none() && !g.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast stopped:{progress:?}"));
        };
        if matches!(ctx, DecisionContext::Priority(_)) {
            return Err("cast returned priority without stack".into());
        }
        progress = apply_decision_context_with_dm(g, &mut q, &mut st, &ctx, d)
            .map_err(|e| e.to_string())?;
    }
    if g.stack.is_empty() {
        return Err("cast announcement budget".into());
    }
    let x = g.stack.last().and_then(|e| e.x_value);
    let mana = before - g.player(PlayerId(0)).unwrap().mana_pool.total();
    let error = finish(g, &mut q, d).err();
    Ok(
        json!({"action":format!("{action:?}"),"mana_paid":mana,"announced_x":x,"resolution_error":error}),
    )
}
fn land(g: &mut GameState, def: &CardDefinition, d: &mut Choices) -> Result<Value, String> {
    let id = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    g.turn.priority_player = Some(PlayerId(0));
    let action = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==id))
        .ok_or("land play unavailable")?;
    let mut q = TriggerQueue::new();
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
    if count(g, def.name(), Zone::Battlefield) != 1 {
        return Err("land did not reach battlefield".into());
    }
    Ok(json!({"producer":"normal land play","card":def.name(),"battlefield_count":1}))
}
const NAMES: [&str; 7] = [
    "Devastating Summons",
    "Eliminate the Competition",
    "Firecat Blitz",
    "Flash of Insight",
    "Immoral Bargain",
    "Summons of Saruman",
    "Tectonic Split",
];
fn find(g: &GameState, n: &str, z: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.name == n && o.zone == z)
        .map(|o| o.id)
        .ok_or(format!("missing {n} in {z:?}"))
}
fn count(g: &GameState, n: &str, z: Zone) -> usize {
    g.objects_in_deterministic_order()
        .iter()
        .filter(|o| o.name == n && o.zone == z)
        .count()
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
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    variant: usize,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(2, 0);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(1)).unwrap().mana_pool.add(color, 20);
    }
    for _ in 0..20 {
        g.create_object_from_definition(&defs["Plains"], PlayerId(0), Zone::Library);
    }
    let flashback = matches!(
        name,
        "Firecat Blitz" | "Flash of Insight" | "Summons of Saruman"
    );
    let land_resource = matches!(
        name,
        "Devastating Summons" | "Firecat Blitz" | "Tectonic Split"
    );
    let funded = variant > 0;
    let desired = if name == "Tectonic Split" {
        0
    } else if variant == 2 {
        if land_resource { 1 } else { 2 }
    } else {
        0
    };
    let mut d = Choices {
        amount: 0,
        actual_x: None,
        targets: vec![],
        reveal_ids: vec![],
        revealed: Default::default(),
        trace: vec![],
    };
    let mut history = vec![];
    let source = if flashback {
        paid(&mut g, defs, name, 0, 2, &mut d, &mut history)?;
        find(&g, name, Zone::Graveyard)?
    } else {
        g.create_object_from_definition(&defs[name], PlayerId(0), Zone::Hand)
    };
    if matches!(name, "Eliminate the Competition" | "Immoral Bargain") {
        for _ in 0..2 {
            d.targets.clear();
            paid(&mut g, defs, "Hill Giant", 0, 4, &mut d, &mut history)?;
        }
    }
    if funded {
        if land_resource {
            history.push(land(&mut g, &defs["Mountain"], &mut d)?);
        } else {
            for _ in 0..2 {
                let producer = if matches!(name, "Eliminate the Competition" | "Immoral Bargain") {
                    "Grizzly Bears"
                } else if name == "Flash of Insight" {
                    "Opt"
                } else {
                    "Shock"
                };
                d.targets = if producer == "Shock" {
                    vec![Target::Player(PlayerId(1))]
                } else {
                    vec![]
                };
                paid(
                    &mut g,
                    defs,
                    producer,
                    0,
                    if producer == "Grizzly Bears" { 2 } else { 1 },
                    &mut d,
                    &mut history,
                )?;
            }
        }
    }
    let resource_name = if land_resource {
        "Mountain"
    } else if matches!(name, "Eliminate the Competition" | "Immoral Bargain") {
        "Grizzly Bears"
    } else if name == "Flash of Insight" {
        "Opt"
    } else {
        "Shock"
    };
    let resource_zone = if land_resource || resource_name == "Grizzly Bears" {
        Zone::Battlefield
    } else {
        Zone::Graveyard
    };
    let resources = count(&g, resource_name, resource_zone);
    let expected_resources = if funded {
        if land_resource { 1 } else { 2 }
    } else {
        0
    };
    if resources != expected_resources {
        return Err(format!(
            "producer resources {resources}!={expected_resources}"
        ));
    }
    if flashback && count(&g, name, Zone::Graveyard) != 1 {
        return Err("primary paid source did not reach graveyard".into());
    }
    d.amount = desired;
    d.actual_x = None;
    d.trace.clear();
    d.targets = if matches!(name, "Eliminate the Competition" | "Immoral Bargain") {
        g.objects_in_deterministic_order()
            .iter()
            .filter(|o| {
                o.name == "Hill Giant" && o.zone == Zone::Battlefield && o.owner == PlayerId(0)
            })
            .take(desired as usize)
            .map(|o| Target::Object(o.id))
            .collect()
    } else {
        vec![]
    };
    g.turn.priority_player = Some(PlayerId(0));
    let zone = if flashback {
        Zone::Graveyard
    } else {
        Zone::Hand
    };
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered=actions.iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,from_zone,..}if *spell_id==source&&*from_zone==zone));
    let ctx = ironsmith::costs::CostCheckContext::new(source, PlayerId(0))
        .with_x(desired)
        .with_reason(ironsmith::costs::PaymentReason::CastSpell);
    let costs = if flashback {
        defs[name]
            .alternative_casts
            .iter()
            .flat_map(|a| a.non_mana_costs())
            .collect::<Vec<_>>()
    } else {
        defs[name].additional_cost.costs().to_vec()
    };
    let diagnostic = json!({"source":source.0,"zone":format!("{zone:?}"),"mana_pool":format!("{:?}",g.player(PlayerId(0)).unwrap().mana_pool),"active_player":g.turn.active_player.index(),"priority_player":g.turn.priority_player.map(|p|p.index()),"phase":format!("{:?}",g.turn.phase),"stack_len":g.stack.len(),"legal_actions":format!("{actions:?}"),"component_checks":costs.iter().map(|c|json!({"cost":format!("{c:?}").chars().take(220).collect::<String>(),"check":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*c.0,&g,&ctx)),"nonmana_x_capacity":c.effect_ref().and_then(|e|e.max_cost_x(&g,source,PlayerId(0)))})).collect::<Vec<_>>()});
    let lib_before = stable_zone(&g, Zone::Library);
    let hand_before = stable_zone(&g, Zone::Hand);
    let grave_before = stable_zone(&g, Zone::Graveyard);
    let before_mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let before_board = board(&g);
    let (action, error) = if offered {
        match cast_existing(&mut g, source, zone, &mut d) {
            Ok(a) => (a, None),
            Err(e) => (Value::Null, Some(e)),
        }
    } else {
        (Value::Null, None)
    };
    let lib_after = stable_zone(&g, Zone::Library);
    let hand_after = stable_zone(&g, Zone::Hand);
    let grave_after = stable_zone(&g, Zone::Graveyard);
    let newly_hand = hand_after
        .iter()
        .filter(|id| !hand_before.contains(id))
        .copied()
        .collect::<Vec<_>>();
    let order_ok = if name == "Flash of Insight" && desired == 2 {
        let top = &lib_before[lib_before.len() - 2..];
        let mut expected_bottom = top
            .iter()
            .filter(|id| !newly_hand.contains(id))
            .copied()
            .collect::<Vec<_>>();
        expected_bottom.extend_from_slice(&lib_before[..lib_before.len() - 2]);
        newly_hand.len() == 1 && top.contains(&newly_hand[0]) && lib_after == expected_bottom
    } else if name == "Summons of Saruman" && desired == 2 {
        lib_after == lib_before[..lib_before.len() - 2]
            && lib_before[lib_before.len() - 2..]
                .iter()
                .all(|id| grave_after.contains(id))
    } else {
        lib_after == lib_before
    };
    let army=g.objects_in_deterministic_order().iter().find(|o|o.name=="Orc Army"&&o.zone==Zone::Battlefield).map(|o|json!({"power":g.calculated_power(o.id),"toughness":g.calculated_toughness(o.id),"plus_one_counters":g.counter_count(o.id,ironsmith::CounterType::PlusOnePlusOne)}));
    let outcome = json!({"mana_paid":before_mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"announced_x":action["announced_x"],"resolution_error":action["resolution_error"],"source_exile":count(&g,name,Zone::Exile),"resource_exile":count(&g,resource_name,Zone::Exile),"library_delta":lib_after.len() as i32-lib_before.len() as i32,"new_hand_cards":newly_hand.len(),"new_plains_in_graveyard":grave_after.iter().filter(|id|!grave_before.contains(id)).filter(|id|g.objects_in_deterministic_order().iter().any(|o|o.stable_id.0.0==**id&&o.name=="Plains")).count(),"army":army,"library_order_matches_card_rule":order_ok});
    let expected_outcome = if name == "Flash of Insight" || name == "Summons of Saruman" {
        json!({"mana_paid":if name=="Flash of Insight"{2}else{5},"announced_x":desired,"resolution_error":null,"source_exile":1,"resource_exile":desired,"library_delta":if desired==0{0}else if name=="Flash of Insight"{-1}else{-2},"new_hand_cards":if name=="Flash of Insight"&&desired>0{1}else{0},"new_plains_in_graveyard":if name=="Summons of Saruman"{desired}else{0},"army":if name=="Summons of Saruman"&&desired>0{json!({"power":2,"toughness":2,"plus_one_counters":2})}else{Value::Null},"library_order_matches_card_rule":true})
    } else {
        Value::Null
    };
    let actual = json!({"cast_offered":offered,"error":error,"resolution_error":action["resolution_error"],"mana_paid":before_mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"announced_x":action["announced_x"],"objects_unchanged":board(&g)==before_board,"resource_count_before":resources,"resource_count_after":count(&g,resource_name,resource_zone),"source_hand":count(&g,name,Zone::Hand),"source_graveyard":count(&g,name,Zone::Graveyard),"source_exile":count(&g,name,Zone::Exile),"source_battlefield":count(&g,name,Zone::Battlefield),"objects_after_probe":board(&g),"outcome":outcome,"expected_outcome":expected_outcome});
    let probe_choices = d.trace.clone();
    // Same-state fixed-cost cast and flashback controls verify usable priority/mana/zone permission.
    d.amount = 0;
    d.actual_x = None;
    d.targets = vec![Target::Player(PlayerId(1))];
    d.trace.clear();
    let before_life = g.player(PlayerId(1)).unwrap().life;
    paid(&mut g, defs, "Firebolt", 0, 1, &mut d, &mut history)?;
    let bolt = find(&g, "Firebolt", Zone::Graveyard)?;
    let flash_control = cast_existing(&mut g, bolt, Zone::Graveyard, &mut d)?;
    if flash_control["mana_paid"] != 5
        || !flash_control["resolution_error"].is_null()
        || count(&g, "Firebolt", Zone::Exile) != 1
        || g.player(PlayerId(1)).unwrap().life != before_life - 4
    {
        return Err(format!(
            "fixed Firebolt normal/flashback control failed:{flash_control}"
        ));
    }
    let expected = json!({"cast_offered":true,"intended_cast_origin":format!("{zone:?}"),"requested_x":if name=="Tectonic Split"{Value::Null}else{json!(desired)},"required_sacrifice_count":if name=="Tectonic Split"{json!((resources+1)/2)}else{Value::Null},"resources_available":resources});
    Ok((
        actual,
        expected,
        json!({"paid_source_and_resource_producers":history,"legality_diagnostic":diagnostic,"probe_action":action,"probe_decisions":probe_choices,"requested_x":desired,"expected_resources":expected_resources,"objects_before_probe":before_board,"control":{"fixed_firebolt_flashback":flash_control,"damage_to_bob":4,"exiled":true,"choices":d.trace}}),
    ))
}
fn stable_zone(g: &GameState, z: Zone) -> Vec<u64> {
    let p = g.player(PlayerId(0)).unwrap();
    let ids = match z {
        Zone::Library => p.library.iter().copied().collect::<Vec<_>>(),
        Zone::Hand => p.hand.to_vec(),
        Zone::Graveyard => p.graveyard.to_vec(),
        _ => vec![],
    };
    ids.iter()
        .map(|id| g.object(*id).unwrap().stable_id.0.0)
        .collect()
}
fn board(g: &GameState) -> Vec<Value> {
    g.objects_in_deterministic_order().iter().map(|o|json!({"id":o.id.0,"name":o.name.to_string(),"zone":format!("{:?}",o.zone),"owner":o.owner.index(),"power":g.calculated_power(o.id),"toughness":g.calculated_toughness(o.id)})).collect()
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
#[ignore = "strict paid nonmana-X spell contexts"]
fn report_nonmana_x_spells() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/nonmana-x-spell-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_nonmana_x_spell_reproductions.rs"),
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
    for name in NAMES {
        for variant in 0..if name == "Tectonic Split" { 2 } else { 3 } {
            println!("stage: {name} variant{variant}");
            let (status, actual, expected, evidence) = match trial(&defs, name, variant) {
                Ok((a, e, t)) => (
                    if a["cast_offered"] == false {
                        "expected_legal_cast_unavailable"
                    } else if !a["error"].is_null() || !a["resolution_error"].is_null() {
                        "execution_failed"
                    } else {
                        "cast_executed_needs_outcome_review"
                    },
                    a,
                    e,
                    t,
                ),
                Err(e) => (
                    "fixture_or_announcement_error",
                    json!({"error":e}),
                    Value::Null,
                    Value::Null,
                ),
            };
            rows.push(json!({"card":name,"scenario":{"variant":variant},"status":status,"actual":actual,"expected":expected,"fixture_evidence":evidence}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"scope":"Seven nonmana-X spell cost contexts. Four normal casts including Tectonic Split derived count, three actual normal-paid-to-graveyard flashback sources. Resources from paid spells or actual land plays. Same-state actual paid Firebolt normal+flashback controls. Absent actions are never dispatched.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/nonmana-x-spell-final-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
