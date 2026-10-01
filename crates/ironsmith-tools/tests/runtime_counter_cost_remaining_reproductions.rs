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
    targets: Vec<Target>,
    counter_objects: Vec<ObjectId>,
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
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let mode_word = if self.mode == 0 {
            "charge counter"
        } else {
            "+1/+1 counter"
        };
        let s = if c
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
            .any(|o| o.description.contains("charge counter"))
            && c.options
                .iter()
                .any(|o| o.description.contains("+1/+1 counter"))
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
    let counters_before = counter_snapshot(g);
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
    let counters_after_announcement = counter_snapshot(g);
    let error = finish(g, &mut q, dm).err();
    Ok(
        json!({"action":format!("{action:?}"),"mana_paid":paid,"resolution_error":error,"counters_before":counters_before,"counters_after_announcement":counters_after_announcement}),
    )
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
fn turn_cycle(g: &mut GameState) {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    while g.turn.active_player != PlayerId(0) {
        g.next_turn();
        ironsmith::turn::execute_untap_step(g);
    }
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(PlayerId(0));
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
}

const NAMES: [&str; 4] = [
    "Tayam, Luminous Enigma",
    "Tekuthal, Inquiry Dominus",
    "Xavier Sal, Infested Captain",
    "Power Conduit",
];
fn counter_snapshot(g: &GameState) -> Value {
    json!(
        g.battlefield
            .iter()
            .filter_map(|id| g.object(*id).map(|o| {
                let mut counts = o
                    .counters
                    .iter()
                    .map(|(k, n)| json!({"kind":format!("{k:?}"),"count":n}))
                    .collect::<Vec<_>>();
                counts.sort_by_key(|v| v["kind"].as_str().unwrap().to_string());
                json!({"id":id.0,"name":o.name.to_string(),"counters":counts})
            }))
            .collect::<Vec<_>>()
    )
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or(format!("missing battlefield {name}"))
}
fn paid_cast(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    expected: u32,
    dm: &mut Choices,
    history: &mut Vec<Value>,
) -> Result<(), String> {
    let e = cast(g, &defs[name], 0, dm)?;
    if e["mana_paid"] != expected || !e["resolution_error"].is_null() {
        return Err(format!("producer {name} failed: {e}"));
    }
    history.push(e);
    Ok(())
}
fn offered(g: &GameState, source: ObjectId, index: usize) -> bool {
    compute_legal_actions(g,PlayerId(0)).expect("fixture has complete replacement state").iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index))
}
fn total(g: &GameState, id: ObjectId) -> u32 {
    g.object(id).map(|o| o.counters.values().sum()).unwrap_or(0)
}
fn plus(g: &GameState, id: ObjectId) -> u32 {
    g.counter_count(id, ironsmith::CounterType::PlusOnePlusOne)
}
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .iter()
        .filter(|o| o.name == name && o.zone == zone)
        .count()
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    variant: usize,
) -> Result<(Value, Value, Value), String> {
    use ironsmith::CounterType;
    let mut g = setup(2, 0);
    for _ in 0..3 {
        g.create_object_from_definition(&defs["Plains"], PlayerId(0), Zone::Library);
    }
    let mut dm = Choices {
        amount: 2,
        mode: 0,
        targets: vec![],
        counter_objects: vec![],
        trace: vec![],
    };
    let mut history = vec![];
    let mut acts = vec![];
    let cost = if name == "Tayam, Luminous Enigma" || name == "Tekuthal, Inquiry Dominus" {
        4
    } else if name == "Xavier Sal, Infested Captain" {
        3
    } else {
        2
    };
    // Tayam's exact-two/zero controls establish Bears before its static entry grant exists.
    if name == "Tayam, Luminous Enigma" && variant >= 2 {
        paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut dm, &mut history)?;
    }
    paid_cast(&mut g, defs, name, cost, &mut dm, &mut history)?;
    let source = find(&g, name)?;
    let index = find_cost_index(&defs[name], 0)?;
    let (expected, actual);
    let resource_payable = if name == "Power Conduit" {
        variant < 3
    } else {
        variant < 2
    };
    if name == "Tayam, Luminous Enigma" {
        if variant < 2 {
            paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut dm, &mut history)?;
        }
        let bear = find(&g, "Grizzly Bears")?;
        let rat = if variant == 1 {
            paid_cast(&mut g, defs, "Typhoid Rats", 1, &mut dm, &mut history)?;
            Some(find(&g, "Typhoid Rats")?)
        } else {
            None
        };
        if variant < 3 {
            dm.targets = vec![Target::Object(bear), Target::Object(rat.unwrap_or(bear))];
            paid_cast(&mut g, defs, "Common Bond", 3, &mut dm, &mut history)?;
        }
        let before = counter_snapshot(&g);
        let expected_before = if variant == 0 {
            3
        } else if variant == 1 {
            4
        } else if variant == 2 {
            2
        } else {
            0
        };
        if total(&g, bear) + rat.map(|r| total(&g, r)).unwrap_or(0) != expected_before {
            return Err(format!("Tayam producer counters wrong:{before}"));
        }
        dm.counter_objects = std::iter::once(bear).chain(rat).collect();
        dm.targets.clear();
        let legal = offered(&g, source, index);
        if legal && resource_payable {
            acts.push(activate(&mut g, source, index, &mut dm)?);
        }
        let errors = acts
            .iter()
            .filter_map(|e| e["resolution_error"].as_str())
            .collect::<Vec<_>>();
        actual = json!({"offered":legal,"errors":errors,"activation_mana":acts.iter().map(|a|a["mana_paid"].as_u64().unwrap()).sum::<u64>(),"bear_counters":total(&g,bear),"rat_counters":rat.map(|r|total(&g,r)),"library_plains":count(&g,"Plains",Zone::Library),"graveyard_plains":count(&g,"Plains",Zone::Graveyard),"battlefield_plains":count(&g,"Plains",Zone::Battlefield)});
        expected = json!({"offered":variant<2,"errors":[],"activation_mana":if variant<2{3}else{0},"bear_counters":if variant<2{0}else{expected_before},"rat_counters":rat.map(|_|1),"library_plains":if variant<2{0}else{3},"graveyard_plains":if variant<2{2}else{0},"battlefield_plains":if variant<2{1}else{0}});
    } else if name == "Tekuthal, Inquiry Dominus" {
        paid_cast(&mut g, defs, "Ornithopter", 0, &mut dm, &mut history)?;
        let orn = find(&g, "Ornithopter")?;
        let bear = if variant == 1 {
            paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut dm, &mut history)?;
            Some(find(&g, "Grizzly Bears")?)
        } else {
            None
        };
        let target = if variant == 3 { source } else { orn };
        dm.targets = vec![
            Target::Object(target),
            Target::Object(bear.unwrap_or(target)),
        ];
        for _ in 0..if variant == 2 { 1 } else { 2 } {
            paid_cast(&mut g, defs, "Common Bond", 3, &mut dm, &mut history)?;
        }
        dm.counter_objects = std::iter::once(target).chain(bear).collect();
        dm.targets.clear();
        let before = counter_snapshot(&g);
        let legal = offered(&g, source, index);
        if legal && resource_payable {
            acts.push(activate(&mut g, source, index, &mut dm)?);
        }
        let indestructible = g.counter_count(source, CounterType::Indestructible);
        let source_plus = plus(&g, source);
        let orn_plus = plus(&g, orn);
        let bear_plus = bear.map(|b| plus(&g, b));
        dm.targets = vec![Target::Object(source)];
        paid_cast(&mut g, defs, "Murder", 3, &mut dm, &mut history)?;
        actual = json!({"offered":legal,"errors":acts.iter().filter_map(|e|e["resolution_error"].as_str()).collect::<Vec<_>>(),"activation_mana":acts.iter().map(|a|a["mana_paid"].as_u64().unwrap()).sum::<u64>(),"indestructible_before_murder":indestructible,"source_plus_before_murder":source_plus,"ornithopter_plus":orn_plus,"bear_plus":bear_plus,"source_survives_murder":g.object(source).is_some_and(|o|o.zone==Zone::Battlefield),"life":g.player(PlayerId(0)).unwrap().life});
        expected = json!({"offered":variant<2,"errors":[],"activation_mana":if variant<2{3}else{0},"indestructible_before_murder":if variant<2{1}else{0},"source_plus_before_murder":if variant==3{4}else{0},"ornithopter_plus":if variant==0{1}else if variant==2{2}else{0},"bear_plus":bear.map(|_|1),"source_survives_murder":variant<2,"life":20});
        history.push(json!({"counter_resource_before_activation":before}));
    } else if name == "Xavier Sal, Infested Captain" {
        let mut resource = source;
        if variant == 0 {
            paid_cast(
                &mut g,
                defs,
                "Ghave, Guru of Spores",
                5,
                &mut dm,
                &mut history,
            )?;
            resource = find(&g, "Ghave, Guru of Spores")?;
            dm.counter_objects = vec![resource];
            let gi = find_cost_index(&defs["Ghave, Guru of Spores"], 0)?;
            let e = activate(&mut g, resource, gi, &mut dm)?;
            if e["mana_paid"] != 1
                || !e["resolution_error"].is_null()
                || plus(&g, resource) != 4
                || count(&g, "Saproling", Zone::Battlefield) != 1
            {
                return Err(format!("Ghave populate producer failed:{e}"));
            }
            history.push(e);
        } else if variant == 1 || variant == 3 {
            paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut dm, &mut history)?;
            resource = find(&g, "Grizzly Bears")?;
        }
        if variant == 1 || variant == 2 {
            dm.targets = vec![Target::Object(resource), Target::Object(resource)];
            paid_cast(&mut g, defs, "Common Bond", 3, &mut dm, &mut history)?;
        }
        turn_cycle(&mut g);
        dm.counter_objects = vec![resource];
        dm.targets.clear();
        let legal = offered(&g, source, index);
        if legal && resource_payable {
            acts.push(activate(&mut g, source, index, &mut dm)?);
        }
        actual = json!({"offered":legal,"errors":acts.iter().filter_map(|e|e["resolution_error"].as_str()).collect::<Vec<_>>(),"activation_mana":acts.iter().map(|a|a["mana_paid"].as_u64().unwrap()).sum::<u64>(),"resource_plus":plus(&g,resource),"saprolings":count(&g,"Saproling",Zone::Battlefield),"source_tapped":g.is_tapped(source)});
        expected = json!({"offered":variant<2,"errors":[],"activation_mana":0,"resource_plus":if variant==0{3}else if variant==1{1}else if variant==2{2}else{0},"saprolings":if variant==0{2}else{0},"source_tapped":variant<2});
    } else {
        paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut dm, &mut history)?;
        let bear = find(&g, "Grizzly Bears")?;
        if variant < 3 {
            dm.targets = vec![Target::Object(bear), Target::Object(bear)];
            paid_cast(&mut g, defs, "Common Bond", 3, &mut dm, &mut history)?;
        }
        dm.counter_objects = vec![bear];
        dm.mode = if variant == 1 { 1 } else { 0 };
        dm.targets = vec![Target::Object(if variant == 1 { bear } else { source })];
        let legal = offered(&g, source, index);
        if legal && resource_payable {
            acts.push(activate(&mut g, source, index, &mut dm)?);
        }
        if variant == 2 {
            dm.targets = vec![Target::Object(source)];
            paid_cast(&mut g, defs, "Twiddle", 1, &mut dm, &mut history)?;
            if g.is_tapped(source) {
                return Err("Twiddle did not untap conduit".into());
            }
            dm.counter_objects = vec![source];
            dm.mode = 1;
            dm.targets = vec![Target::Object(bear)];
            acts.push(activate(&mut g, source, index, &mut dm)?);
        }
        actual = json!({"offered":legal,"errors":acts.iter().filter_map(|e|e["resolution_error"].as_str()).collect::<Vec<_>>(),"activation_mana":acts.iter().map(|a|a["mana_paid"].as_u64().unwrap()).sum::<u64>(),"bear_plus":plus(&g,bear),"source_charge":g.counter_count(source,CounterType::Charge),"source_tapped":g.is_tapped(source)});
        expected = json!({"offered":variant<3,"errors":[],"activation_mana":0,"bear_plus":if variant==0{1}else if variant<3{2}else{0},"source_charge":if variant==0{1}else{0},"source_tapped":variant<3});
    }
    Ok((
        expected,
        actual,
        json!({"normal_paid_sources_and_producers":history,"activations":acts,"choices":dm.trace,"ability_index":index,"source":source.0,"fixture_cost_resources_sufficient":resource_payable}),
    ))
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
#[ignore = "remaining canonical counter-removal costs"]
fn report_counter_cost_remaining() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/counter-cost-remaining-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_counter_cost_remaining_reproductions.rs"),
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
        let (mut ids, mut fids) = (vec![], vec![]);
        let parity = normalize(&definition, "/definition", &mut ids)
            == normalize(&p["frozen_definition"], "/definition", &mut fids);
        compilation.push(json!({"card":n,"artifact_checksum":a.payload_checksum,"frozen_artifact_checksum":p["frozen_artifact_checksum"],"definition_matches_frozen_except_unique_card_ids":parity,"ignored_id_paths":ids,"definition":definition}));
        defs.insert(n.to_string(), d);
    }
    let mut rows = vec![];
    for name in NAMES {
        for variant in 0..4 {
            println!("stage: {name} variant{variant}");
            let (status, expected, actual, evidence) = match run(&defs, name, variant) {
                Ok((e, a, f)) => (
                    if e == a {
                        "expected_result_observed"
                    } else if !a["errors"].as_array().unwrap().is_empty() {
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
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"scope":"Four full canonical counter-cost sources through real paid casts and exact ordinary cost decisions. Actual Common Bond and ETB counter producers; full object distributions and counter counts chosen. Tekuthal paid Murder indestructibility; Xavier actual Ghave paid token producer and own turn/untap; Power Conduit both modes and self-charge recycle through paid Twiddle. No engine modifications. Negative-resource branches query legal actions without forcing unavailable activations.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/counter-cost-remaining-final-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
