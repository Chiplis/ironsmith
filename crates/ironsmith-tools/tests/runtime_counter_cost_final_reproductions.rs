//! Canonical counter-removal costs with actual producers and explicit full payment choices.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{
    CountersContext, DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
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
    mode: usize,
    prefer_remove: bool,
    targets: Vec<Target>,
    counter_objects: Vec<ObjectId>,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_proliferate(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ProliferateContext,
    ) -> ironsmith::decisions::specs::ProliferateResponse {
        let selected = self
            .counter_objects
            .iter()
            .copied()
            .filter(|id| {
                c.eligible_permanents
                    .iter()
                    .any(|(candidate, _)| candidate == id)
            })
            .collect::<Vec<_>>();
        self.trace.push(json!({"choice":"proliferate","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        ironsmith::decisions::specs::ProliferateResponse {
            permanents: selected,
            players: vec![],
        }
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
    fn decide_number(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let selected = self.amount.max(c.min).min(c.max);
        self.trace
            .push(json!({"choice":"number","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","source_zone":g.object(c.source).map(|o|format!("{:?}",o.zone)),"source_counters":g.object(c.source).map(|o|format!("{:?}",o.counters)),"context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let mode_word = if self.mode == 0 {
            "charge counters"
        } else {
            "+1/+1 counters"
        };
        let s = if c
            .options
            .iter()
            .any(|o| o.description.to_lowercase().contains("remove"))
            && c.options.iter().any(|o| o.description.contains("{3}"))
        {
            c.options
                .iter()
                .find(|o| {
                    o.legal
                        && if self.prefer_remove {
                            o.description.to_lowercase().contains("remove")
                        } else {
                            o.description.contains("{3}")
                        }
                })
                .map(|o| vec![o.index])
                .unwrap()
        } else if c
            .options
            .iter()
            .any(|o| o.legal && o.description.to_lowercase().contains("untap"))
        {
            c.options
                .iter()
                .find(|o| o.legal && o.description.to_lowercase().contains("untap"))
                .map(|o| vec![o.index])
                .unwrap()
        } else if c
            .options
            .iter()
            .any(|o| o.description.contains("charge counters"))
            && c.options
                .iter()
                .any(|o| o.description.contains("+1/+1 counters"))
        {
            c.options
                .iter()
                .find(|o| o.legal && o.description.contains(mode_word))
                .map(|o| vec![o.index])
                .unwrap()
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));
        s
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let preferred: Vec<_> = if c.description.to_lowercase().contains("search") {
            c.candidates
                .iter()
                .filter(|o| o.legal && g.object(o.id).is_some_and(|o| o.name == "Plains"))
                .take(1)
                .map(|o| o.id)
                .collect()
        } else {
            self.counter_objects
                .iter()
                .copied()
                .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
                .collect()
        };
        let s = if preferred.is_empty() {
            ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, c)
        } else {
            preferred
                .into_iter()
                .take(c.max.unwrap_or(self.counter_objects.len()))
                .collect()
        };
        self.trace.push(
            json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{s:?}")}),
        );
        s
    }
    fn decide_distribute(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        let mut needed = c.total;
        let mut chosen = vec![];
        for id in &self.counter_objects {
            if c.targets.iter().any(|t| t.target == Target::Object(*id)) {
                let available = g
                    .object(*id)
                    .map(|o| o.counters.values().copied().sum::<u32>())
                    .unwrap_or(0);
                let take = available.min(needed);
                if take > 0 {
                    chosen.push((Target::Object(*id), take));
                    needed -= take;
                }
            }
        }
        self.trace.push(json!({"choice":"counter-object-distribution","context":format!("{c:?}"),"selected":format!("{chosen:?}"),"unavailable_required":needed}));
        chosen
    }
    fn decide_counters(
        &mut self,
        _: &GameState,
        c: &CountersContext,
    ) -> Vec<(ironsmith::CounterType, u32)> {
        let mut needed = c.max_total;
        let mut s = vec![];
        for (kind, n) in &c.available_counters {
            let take = needed.min(*n);
            if take > 0 {
                s.push((*kind, take));
                needed -= take;
            }
        }
        self.trace.push(json!({"choice":"counters","context":format!("{c:?}"),"selected":format!("{s:?}"),"unavailable_required":needed}));
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
    let mut evidence = json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",g.stack.last().unwrap().targets),"resolution_error":null});
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
        if st.pending_activation.is_none() && !g.stack.is_empty() {
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
    let error = finish(g, &mut q, dm).err();
    Ok(json!({"action":format!("{action:?}"),"mana_paid":paid,"resolution_error":error}))
}
fn find_cost_index(def: &CardDefinition, ordinal: usize) -> Result<usize, String> {
    def.abilities
        .iter()
        .enumerate()
        .filter(|(_, a)| match &a.kind {
            ironsmith::ability::AbilityKind::Activated(a) => a.mana_cost.costs().iter().any(|c| {
                c.effect_ref().is_some_and(|e| {
                    e.downcast_ref::<ironsmith::effects::RemoveAnyCountersAmongEffect>()
                        .is_some()
                })
            }),
            _ => false,
        })
        .nth(ordinal)
        .map(|(i, _)| i)
        .ok_or("intended counter cost index absent".into())
}
const NAMES: [&str; 4] = [
    "Duchess, Wayward Tavernkeep",
    "Gavel of the Righteous",
    "Glen Elendra Guardian",
    "The Filigree Sylex",
];
fn find(g: &GameState, name: &str, zone: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .iter()
        .find(|o| o.name == name && o.zone == zone)
        .map(|o| o.id)
        .ok_or(format!("{name} absent in {zone:?}"))
}
fn paid(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    dm: &mut Choices,
    mana: u32,
) -> Result<Value, String> {
    let e = cast(g, &defs[name], 0, dm)?;
    if e["mana_paid"] != mana || !e["resolution_error"].is_null() {
        return Err(format!("{name} cast/payment mismatch:{e}"));
    }
    Ok(e)
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    variant: usize,
) -> Result<(Value, Value, Value), String> {
    use ironsmith::CounterType;
    let mut g = setup(3, 0);
    let wd = CardDefinitionBuilder::new(CardId::new(), "Final counter resource")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(5, 8))
        .build();
    let witness = g.create_object_from_definition(&wd, PlayerId(0), Zone::Battlefield);
    for owner in [PlayerId(0), PlayerId(1)] {
        for _ in 0..4 {
            g.create_object_from_definition(&defs["Plains"], owner, Zone::Library);
        }
    }
    g.player_mut(PlayerId(1))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 2);
    let duchess = name.starts_with("Duchess");
    let gavel = name.starts_with("Gavel");
    let guardian = name.starts_with("Glen");
    let sylex = name == "The Filigree Sylex";
    let mut dm = Choices {
        amount: 2,
        mode: 0,
        prefer_remove: variant == 0,
        targets: vec![],
        counter_objects: vec![],
        trace: vec![],
    };
    let source_cast = paid(
        &mut g,
        defs,
        name,
        &mut dm,
        if duchess {
            4
        } else if guardian {
            3
        } else {
            2
        },
    )?;
    let source = find(&g, name, Zone::Battlefield)?;
    let mut producers = vec![];
    let mut resource = source;
    let mut stack_target = None;
    if duchess {
        producers.push(paid(&mut g, defs, "Quest for Ancient Secrets", &mut dm, 1)?);
        resource = find(&g, "Quest for Ancient Secrets", Zone::Battlefield)?;
        dm.targets = vec![Target::Player(PlayerId(1))];
        producers.push(paid(&mut g, defs, "Shock", &mut dm, 1)?);
        if g.counter_count(resource, CounterType::Quest) != 1 {
            return Err("real Quest graveyard trigger did not produce one counter".into());
        }
    }
    if gavel {
        producers.push(paid(&mut g, defs, "Moxite Refinery", &mut dm, 2)?);
        let refinery = find(&g, "Moxite Refinery", Zone::Battlefield)?;
        dm.targets = vec![Target::Object(witness), Target::Object(witness)];
        producers.push(paid(&mut g, defs, "Common Bond", &mut dm, 3)?);
        if g.counter_count(witness, CounterType::PlusOnePlusOne) != 2 {
            return Err("Common Bond resource absent".into());
        }
        dm.counter_objects = vec![witness];
        dm.targets = vec![Target::Object(source)];
        let idx = find_cost_index(&defs["Moxite Refinery"], 0)?;
        let e = activate(&mut g, refinery, idx, &mut dm)?;
        if e["mana_paid"] != 2 || g.counter_count(source, CounterType::Charge) != 2 {
            return Err(format!("Moxite real charge producer failed:{e}"));
        }
        producers.push(e);
    }
    if guardian {
        if g.counter_count(source, CounterType::MinusOneMinusOne) != 1 {
            return Err("Guardian ETB minus counter absent".into());
        }
        let actor = if variant == 0 { 1 } else { 0 };
        dm.targets = vec![Target::Player(PlayerId(if actor == 0 { 1 } else { 0 }))];
        let (mut q, e) = announce(&mut g, &defs["Shock"], actor, &mut dm)?;
        if e["mana_paid"] != 1 {
            return Err("Shock stack producer payment".into());
        }
        stack_target = g.stack.last().map(|e| e.object_id);
        producers.push(e);
        if actor == 1 {
            let mut st = PriorityLoopState::new(g.players_in_game());
            for _ in 0..2 {
                apply_priority_response_with_dm(
                    &mut g,
                    &mut q,
                    &mut st,
                    &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                    &mut dm,
                )
                .map_err(|e| e.to_string())?;
            }
        }
        if g.turn.priority_player != Some(PlayerId(0)) || g.stack.is_empty() {
            return Err("Shock response priority not established".into());
        }
    }
    if sylex && variant == 0 {
        let idx = defs[name]
            .abilities
            .iter()
            .enumerate()
            .find_map(|(i, a)| {
                matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
            })
            .unwrap();
        for count in 1..=10 {
            dm.targets = vec![];
            let e = activate(&mut g, source, idx, &mut dm)?;
            if e["mana_paid"] != 0 || g.counter_count(source, CounterType::Oil) != count {
                return Err(format!("Sylex real oil producer failed at{count}:{e}"));
            }
            producers.push(e);
            dm.targets = vec![Target::Object(source)];
            producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
            if g.is_tapped(source) {
                return Err("Twiddle did not untap Sylex".into());
            }
        }
    }
    if sylex && variant == 1 {
        producers.push(paid(&mut g, defs, "Meldweb Strider", &mut dm, 5)?);
        resource = find(&g, "Meldweb Strider", Zone::Battlefield)?;
        producers.push(paid(&mut g, defs, "Darksteel Myr", &mut dm, 3)?);
        let myr = find(&g, "Darksteel Myr", Zone::Battlefield)?;
        dm.counter_objects = vec![resource];
        for count in 2..=10 {
            dm.targets = vec![Target::Object(myr)];
            producers.push(paid(&mut g, defs, "Volt Charge", &mut dm, 3)?);
            if g.counter_count(resource, CounterType::Oil) != count {
                return Err(format!("Volt Charge did not establish {count} oil"));
            }
        }
    }
    dm.counter_objects = vec![resource];
    dm.targets = if gavel {
        vec![Target::Object(witness)]
    } else if guardian {
        vec![Target::Object(stack_target.unwrap())]
    } else if sylex {
        vec![Target::Player(PlayerId(1))]
    } else {
        vec![]
    };
    let advertised = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let index = if gavel {
        advertised
            .iter()
            .find_map(|a| match a {
                LegalAction::ActivateAbility {
                    source: s,
                    ability_index,
                } if *s == source => Some(*ability_index),
                _ => None,
            })
            .ok_or(format!("Gavel has no advertised activation:{advertised:?}"))?
    } else {
        find_cost_index(&defs[name], 0)?
    };
    let before_activation = json!({"source_zone":g.object(source).map(|o|format!("{:?}",o.zone)),"source_oil":g.counter_count(source,CounterType::Oil),"resource_zone":g.object(resource).map(|o|format!("{:?}",o.zone)),"resource_oil":g.counter_count(resource,CounterType::Oil),"resource_charge":g.counter_count(resource,CounterType::Charge),"mana_available":g.player(PlayerId(0)).unwrap().mana_pool.total(),"advertised_actions":format!("{advertised:?}")});
    let activation = match activate(&mut g, source, index, &mut dm) {
        Ok(a) => a,
        Err(e) => json!({"action_error":e,"resolution_error":e,"mana_paid":null}),
    };
    let paid_mana = if duchess {
        1
    } else if guardian {
        2
    } else if gavel && variant == 1 {
        3
    } else {
        0
    };
    if activation["action_error"].is_null() && activation["mana_paid"] != paid_mana {
        return Err(format!(
            "activation paid mismatch expected{paid_mana}:{activation}; choices:{:?}",
            dm.trace
        ));
    }
    let token_names: Vec<_> = g
        .battlefield
        .iter()
        .filter_map(|id| {
            g.object(*id)
                .filter(|o| o.kind == ironsmith::object::ObjectKind::Token)
                .map(|o| o.name.to_string())
        })
        .collect();
    let actual = json!({"error":activation["resolution_error"],"source_battlefield":g.object(source).is_some_and(|o|o.zone==Zone::Battlefield),"source_graveyard":g.objects_in_deterministic_order().iter().any(|o|o.name==name&&o.zone==Zone::Graveyard),"source_minus":g.counter_count(source,CounterType::MinusOneMinusOne),"source_oil":g.counter_count(source,CounterType::Oil),"source_charge":g.counter_count(source,CounterType::Charge),"resource_oil":g.counter_count(resource,CounterType::Oil),"resource_quest":g.counter_count(resource,CounterType::Quest),"witness_power":g.calculated_power(witness),"witness_toughness":g.calculated_toughness(witness),"attached":g.object(source).and_then(|o|o.attached_to)==Some(ironsmith::object::AttachmentTarget::Object(witness)),"alice_life":g.player(PlayerId(0)).unwrap().life,"bob_life":g.player(PlayerId(1)).unwrap().life,"alice_hand_plains":g.objects_in_deterministic_order().iter().filter(|o|o.name=="Plains"&&o.zone==Zone::Hand&&o.owner==PlayerId(0)).count(),"bob_hand_plains":g.objects_in_deterministic_order().iter().filter(|o|o.name=="Plains"&&o.zone==Zone::Hand&&o.owner==PlayerId(1)).count(),"shock_graveyard":g.objects_in_deterministic_order().iter().filter(|o|o.name=="Shock"&&o.zone==Zone::Graveyard).count(),"tokens":token_names});
    let bonus = if gavel {
        if variant == 0 { 1 } else { 2 }
    } else {
        0
    };
    let expected = json!({"error":null,"source_battlefield":!sylex,"source_graveyard":sylex,"source_minus":0,"source_oil":0,"source_charge":bonus,"resource_oil":0,"resource_quest":0,"witness_power":5+bonus,"witness_toughness":8+bonus,"attached":gavel,"alice_life":20,"bob_life":if duchess{18}else if sylex{10}else{20},"alice_hand_plains":if guardian&&variant==1{1}else{0},"bob_hand_plains":if guardian&&variant==0{1}else{0},"shock_graveyard":if duchess||guardian{1}else{0},"tokens":if duchess{vec!["Junk"]}else{vec![]}});
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"producers":producers,"before_activation":before_activation,"activation":activation,"choice_trace":dm.trace,"ability_index":index}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "final counter-cost legal producer audit"]
fn report_counter_cost_final() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Common Bond",
                "Plains",
                "Quest for Ancient Secrets",
                "Shock",
                "Twiddle",
                "Moxite Refinery",
                "Meldweb Strider",
                "Darksteel Myr",
                "Volt Charge",
            ]
            .contains(&n)
        {
            continue;
        }
        let (a, d) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(n),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        artifacts.push(
            json!({"card":n,"checksum":a.payload_checksum,"definition":a.payload.definition}),
        );
        defs.insert(n.to_string(), d);
    }
    let mut rows = vec![];
    for name in NAMES {
        let variants = if [
            "Gavel of the Righteous",
            "Glen Elendra Guardian",
            "The Filigree Sylex",
        ]
        .contains(&name)
        {
            2
        } else {
            1
        };
        for variant in 0..variants {
            let (status, expected, actual, evidence) = match run(&defs, name, variant) {
                Ok((e, a, f)) => (
                    if e == a {
                        "expected_result_observed"
                    } else if !a["error"].is_null() {
                        "execution_failed"
                    } else {
                        "semantic_mismatch"
                    },
                    e,
                    a,
                    f,
                ),
                Err(e) => (
                    "fixture_or_producer_error",
                    Value::Null,
                    json!({"error":e}),
                    Value::Null,
                ),
            };
            rows.push(json!({"card":name,"scenario":{"variant":variant},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Real paid canonical sources and producers: Duchess Quest+Shock counter; Gavel CommonBond+Moxite charge counters, alternate equip payment; Guardian own/opponent actual pending Shock; Sylex ten real tap-for-oil activations with ten paid Twiddles, then full remove/sacrifice cost. Immediate result scope only.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_counter_cost_final_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/counter-cost-final-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
