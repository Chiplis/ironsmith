//! Canonical fixed multi-permanent tap costs with exact output and resource boundaries.
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
    targets: Vec<Target>,
    names: Vec<String>,
    x: u32,
    allow_optional: bool,
    choose_untap: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let s = self.x.max(c.min).min(c.max);
        self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"minimum":c.min,"maximum":c.max,"is_x_value":c.is_x_value,"requested":self.x,"selected":s}));
        s
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.trace.push(
            json!({"choice":"boolean","context":format!("{c:?}"),"selected":self.allow_optional}),
        );
        self.allow_optional
    }
    fn decide_colors(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        let s = vec![ironsmith::color::Color::Blue; c.count as usize];
        self.trace.push(
            json!({"choice":"colors","context":format!("{c:?}"),"selected":format!("{s:?}")}),
        );
        s
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let s = if let Some(o) = c.options.iter().find(|o| {
            o.legal
                && if self.choose_untap {
                    o.description.to_lowercase().contains("untap")
                } else {
                    o.description.to_lowercase().starts_with("tap ")
                }
        }) {
            vec![o.index]
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));
        s
    }
    fn decide_partition(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        let selected = c.cards.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        self.trace.push(json!({"choice":"partition_to_graveyard","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut s = vec![];
        for name in &self.names {
            for o in &c.candidates {
                if o.legal && g.object(o.id).is_some_and(|o| o.name == *name) && !s.contains(&o.id)
                {
                    s.push(o.id)
                }
            }
        }
        s.truncate(c.max.unwrap_or(s.len()));
        if s.len() < c.min {
            for o in &c.candidates {
                if o.legal && !s.contains(&o.id) {
                    s.push(o.id);
                    if s.len() >= c.min {
                        break;
                    }
                }
            }
        }
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":s.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string())})).collect::<Vec<_>>()}));
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
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color, 30);
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
    let entry = g
        .stack
        .iter()
        .find(|entry| {
            g.object(entry.object_id)
                .is_some_and(|o| o.name == def.name())
        })
        .ok_or("intended spell missing from stack")?;
    let evidence = json!({"spell":def.name(),"mana_paid":mana-g.player(PlayerId(actor)).unwrap().mana_pool.total(),"announced_targets":format!("{:?}",entry.targets),"resolution_error":null,"stack_names_at_announcement":g.stack.iter().filter_map(|e|g.object(e.object_id).map(|o|o.name.to_string())).collect::<Vec<_>>()});
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
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .iter()
        .filter(|o| o.name == name && o.zone == zone)
        .count()
}
fn action(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    mana: bool,
    dm: &mut Choices,
) -> Result<Value, String> {
    let a = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| match a {
            LegalAction::ActivateAbility {
                source: s,
                ability_index,
            } => !mana && *s == source && *ability_index == index,
            LegalAction::ActivateManaAbility {
                source: s,
                ability_index,
            } => mana && *s == source && *ability_index == index,
            _ => false,
        })
        .ok_or("intended activation unavailable")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(a.clone()),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        match progress {
            GameProgress::NeedsDecisionCtx(ref ctx)
                if !matches!(ctx, DecisionContext::Priority(_)) =>
            {
                progress = apply_decision_context_with_dm(g, &mut q, &mut st, ctx, dm)
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                let taps = g
                    .battlefield
                    .iter()
                    .filter(|id| g.is_tapped(**id))
                    .map(|id| id.0)
                    .collect::<Vec<_>>();
                let err = finish(g, &mut q, dm).err();
                return Ok(
                    json!({"action":format!("{a:?}"),"mana_change":g.player(PlayerId(0)).unwrap().mana_pool.total() as i64-before as i64,"resolution_error":err,"tapped_at_announcement":taps}),
                );
            }
        }
    }
    Err("activation decision budget".into())
}

const NAMES: [&str; 8] = [
    "Devout Chaplain",
    "Harmonized Trio",
    "Harmonized Trio // Brainstorm",
    "Lathril, Blade of the Elves",
    "Saradoc, Master of Buckland",
    "The Cabbage Merchant",
    "Whirler Rogue",
    "Skirk Fire Marshal",
];
fn fund(g: &mut GameState, actor: PlayerId) {
    for c in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(actor).unwrap().mana_pool.add(c, 30);
    }
}
fn next_main(g: &mut GameState) {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(g.turn.active_player);
    fund(g, g.turn.active_player);
}

fn tokens(g: &GameState, subtype: &str) -> Vec<ObjectId> {
    g.objects_in_deterministic_order()
        .iter()
        .filter(|o| {
            o.zone == Zone::Battlefield
                && o.kind == ironsmith::object::ObjectKind::Token
                && o.subtypes.iter().any(|t| format!("{t:?}") == subtype)
        })
        .map(|o| o.id)
        .collect()
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    extra: i32,
    state: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(3, 0);
    let mut dm = Choices {
        targets: vec![],
        names: vec![],
        x: 0,
        allow_optional: true,
        choose_untap: false,
        trace: vec![],
    };
    let mut producers = vec![];
    let actual_name = defs[name].name();
    let prepared = name.starts_with("Harmonized Trio");
    let tap_source = prepared || name == "Devout Chaplain" || name == "Lathril, Blade of the Elves";
    for _ in 0..8 {
        for p in [PlayerId(0), PlayerId(1), PlayerId(2)] {
            g.create_object_from_definition(&defs["Plains"], p, Zone::Library);
        }
    }
    next_main(&mut g);
    let target_name = if name == "Devout Chaplain" {
        "Ornithopter"
    } else {
        "Hill Giant"
    };
    let target_price = if name == "Devout Chaplain" { 0 } else { 4 };
    let target_cast = cast(&mut g, &defs[target_name], 1, &mut dm)?;
    if target_cast["mana_paid"] != target_price || !target_cast["resolution_error"].is_null() {
        return Err("opponent target cast failed".into());
    }
    let target = find(&g, target_name, Zone::Battlefield)?;
    let target_stable = g.object(target).unwrap().stable_id;
    while g.turn.active_player != PlayerId(0) {
        next_main(&mut g);
    }
    let price = if prepared {
        1
    } else {
        match name {
            "Devout Chaplain" | "The Cabbage Merchant" => 3,
            "Skirk Fire Marshal" => 5,
            _ => 4,
        }
    };
    let source_cast = paid(&mut g, defs, name, &mut dm, price)?;
    let source = find(&g, actual_name, Zone::Battlefield)?;
    if tap_source && state != "fresh" {
        next_main(&mut g);
        while g.turn.active_player != PlayerId(0) {
            next_main(&mut g);
        }
    }
    let required = match name {
        "Lathril, Blade of the Elves" => 10,
        "Skirk Fire Marshal" => 5,
        _ => 2,
    };
    let mut resources = match name {
        "Saradoc, Master of Buckland" => tokens(&g, "Halfling"),
        "Whirler Rogue" => tokens(&g, "Thopter"),
        "Skirk Fire Marshal" => vec![source],
        _ => vec![],
    };
    if name == "Saradoc, Master of Buckland" && resources.len() != 1 {
        return Err("actual Saradoc entry token wrong".into());
    }
    if name == "Whirler Rogue" {
        if resources.len() != 2 {
            return Err("actual Whirler entry tokens wrong".into());
        }
        if extra < 0 {
            dm.targets = vec![Target::Object(resources[0])];
            producers.push(paid(&mut g, defs, "Murder", &mut dm, 3)?);
            resources = tokens(&g, "Thopter");
            if resources.len() != 1 {
                return Err("actual Thopter death failed".into());
            }
        }
    }
    if name == "The Cabbage Merchant" {
        next_main(&mut g);
        for _ in 0..required + extra {
            dm.targets.clear();
            let e = cast(&mut g, &defs["Lotus Petal"], 1, &mut dm)?;
            if e["mana_paid"] != 0 || !e["resolution_error"].is_null() {
                return Err("opponent Petal cast failed".into());
            }
            producers.push(e);
        }
        resources = tokens(&g, "Food");
        while g.turn.active_player != PlayerId(0) {
            next_main(&mut g);
        }
        if resources.len() != (required + extra) as usize {
            return Err("Cabbage actual cast-trigger Food count wrong".into());
        }
    }
    let (producer, producer_price) = if prepared {
        ("Grizzly Bears", 2)
    } else {
        match name {
            "Devout Chaplain" => ("Fugitive Wizard", 1),
            "Lathril, Blade of the Elves" => ("Llanowar Elves", 1),
            "Saradoc, Master of Buckland" => ("Suntail Hawk", 1),
            "Whirler Rogue" => ("Ornithopter", 0),
            "Skirk Fire Marshal" => ("Goblin Piker", 2),
            _ => ("Grizzly Bears", 2),
        }
    };
    while resources.len() < (required + extra) as usize {
        let before = g.battlefield.clone();
        dm.targets.clear();
        producers.push(paid(&mut g, defs, producer, &mut dm, producer_price)?);
        if name == "Saradoc, Master of Buckland" {
            resources = tokens(&g, "Halfling");
        } else {
            let new = g
                .battlefield
                .iter()
                .filter(|id| !before.contains(id) && g.controller_of_id(**id) == Some(PlayerId(0)))
                .copied()
                .collect::<Vec<_>>();
            if new.len() != 1 {
                return Err("resource producer new object count wrong".into());
            }
            resources.push(new[0]);
        }
    }
    if extra < 0 && name != "Whirler Rogue" {
        producers.push(paid(&mut g, defs, "Bonesplitter", &mut dm, 1)?);
    }
    if state == "tapped" {
        dm.targets = vec![Target::Object(source)];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(source) {
            return Err("Twiddle source tap failed".into());
        }
    }
    let index = if prepared || name == "Devout Chaplain" {
        0
    } else {
        match name {
            "Lathril, Blade of the Elves" | "The Cabbage Merchant" => 2,
            _ => 1,
        }
    };
    let abilities = g.current_abilities(source).ok_or("abilities missing")?;
    let ability = match &abilities.get(index).ok_or("intended index missing")?.kind {
        ironsmith::ability::AbilityKind::Activated(a) => a,
        _ => return Err("intended index not activated".into()),
    };
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let mana = name == "The Cabbage Merchant";
    let offered = actions.iter().any(|a| match a {
        LegalAction::ActivateAbility {
            source: id,
            ability_index,
        } => !mana && *id == source && *ability_index == index,
        LegalAction::ActivateManaAbility {
            source: id,
            ability_index,
        } => mana && *id == source && *ability_index == index,
        _ => false,
    });
    let before = json!({"source_index":index,"source_sick":g.is_summoning_sick(source),"source_tapped":g.is_tapped(source),"prepared":g.is_prepared(source),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"sick":g.is_summoning_sick(*id),"tapped":g.is_tapped(*id)})).collect::<Vec<_>>(),"target_can_be_blocked":g.can_be_blocked(target),"cost":format!("{:?}",ability.mana_cost),"actions":format!("{actions:?}")});
    dm.targets = if name == "Devout Chaplain" || name == "Whirler Rogue" {
        vec![Target::Object(target)]
    } else {
        vec![]
    };
    dm.names = resources
        .iter()
        .map(|id| g.object(*id).unwrap().name.to_string())
        .collect();
    let valid = extra >= 0 && state == "ready";
    if !valid || !offered {
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace}),
        ));
    }
    let pool_before = g.player(PlayerId(0)).unwrap().mana_pool.clone();
    let activation = action(&mut g, source, index, mana, &mut dm)
        .unwrap_or_else(|e| json!({"resolution_error":e}));
    let target = g
        .objects_in_deterministic_order()
        .iter()
        .find(|o| o.stable_id == target_stable)
        .map(|o| o.id)
        .unwrap();
    let paid_taps = activation["tapped_at_announcement"]
        .as_array()
        .ok_or("tap snapshot unavailable")?;
    let resource_taps = resources
        .iter()
        .filter(|id| paid_taps.iter().any(|v| v.as_u64() == Some(id.0)))
        .count();
    let pool = &g.player(PlayerId(0)).unwrap().mana_pool;
    let mut actual = json!({"error":activation["resolution_error"],"source_zone":format!("{:?}",g.object(source).unwrap().zone),"source_tapped":g.is_tapped(source),"source_power":g.calculated_power(source),"source_toughness":g.calculated_toughness(source),"source_lifelink":g.object_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Lifelink),"source_damage":g.damage_on(source),"prepared":g.is_prepared(source),"target_zone":format!("{:?}",g.object(target).unwrap().zone),"target_can_be_blocked":g.can_be_blocked(target),"resources_tapped_at_announcement":resource_taps,"surplus_untapped_at_announcement":resources.len()-resource_taps,"dead_resources":resources.iter().filter(|id|!g.battlefield.contains(id)).count(),"mana_delta":pool.total()as i64-pool_before.total()as i64,"blue_delta":if mana{Some(pool.blue as i64-pool_before.blue as i64)}else{None},"life":g.players.iter().map(|p|p.life).collect::<Vec<_>>(),"stack_length":g.stack.len()});
    let (power, toughness) = if prepared {
        (1, 1)
    } else {
        match name {
            "Devout Chaplain" => (2, 2),
            "Lathril, Blade of the Elves" => (2, 3),
            "Saradoc, Master of Buckland" => (4, 4),
            _ => (2, 2),
        }
    };
    let mut expected = json!({"error":null,"source_zone":"Battlefield","source_tapped":tap_source||name=="Skirk Fire Marshal","source_power":power,"source_toughness":toughness,"source_lifelink":name=="Saradoc, Master of Buckland","source_damage":0,"prepared":prepared,"target_zone":if name=="Devout Chaplain"{"Exile"}else if name=="Skirk Fire Marshal"{"Graveyard"}else{"Battlefield"},"target_can_be_blocked":name!="Whirler Rogue","resources_tapped_at_announcement":required,"surplus_untapped_at_announcement":extra,"dead_resources":if name=="Skirk Fire Marshal"{required+extra-1}else{0},"mana_delta":if mana{1}else{0},"blue_delta":if mana{Some(1)}else{None},"life":if name=="Lathril, Blade of the Elves"{vec![30,10,10]}else if name=="Skirk Fire Marshal"{vec![10,10,10]}else{vec![20,20,20]},"stack_length":0});
    g.turn.phase = ironsmith::Phase::Ending;
    g.turn.step = Some(ironsmith::Step::Cleanup);
    ironsmith::turn::execute_cleanup_step(&mut g);
    actual["cleanup"] = json!({"source_power":g.calculated_power(source),"source_lifelink":g.object_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Lifelink),"target_can_be_blocked":g.can_be_blocked(target),"prepared":g.is_prepared(source)});
    expected["cleanup"] = json!({"source_power":if name=="Saradoc, Master of Buckland"{2}else{power},"source_lifelink":false,"target_can_be_blocked":true,"prepared":prepared});
    let prepared_observation=if prepared{Some(json!({"has_linked_prepare_spell":g.has_prepare_spell(source),"prepared_after_activation":actual["prepared"],"prepared_after_cleanup":actual["cleanup"]["prepared"],"scope":"Canonical frozen definition has no linked prepare-spell face. Only cost payment is certified; preparation and spell-copy behavior are unverified fixture limitations."}))}else{None};
    if prepared{actual.as_object_mut().unwrap().remove("prepared");expected.as_object_mut().unwrap().remove("prepared");actual["cleanup"].as_object_mut().unwrap().remove("prepared");expected["cleanup"].as_object_mut().unwrap().remove("prepared");}
    Ok((
        expected,
        actual,
        json!({"prepared_effect_scope":prepared_observation,"source_cast":source_cast,"target_cast":target_cast,"producers":producers,"before_activation":before,"activation":activation,"activation_dispatched":true,"choice_trace":dm.trace,"scope":"Canonical paid sources and resources, actual triggered tokens/death/mana. Exact cost snapshot before resolution retains payments even when Skirk kills resources. Harmonized Trio cost payments only: preparation and spell-copy effects require a linked face absent from this isolated fixture and are unverified."}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "fixed multi-permanent tap cost audit"]
fn report_special_tap_costs() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Plains",
                "Hill Giant",
                "Ornithopter",
                "Fugitive Wizard",
                "Grizzly Bears",
                "Llanowar Elves",
                "Suntail Hawk",
                "Lotus Petal",
                "Goblin Piker",
                "Bonesplitter",
                "Murder",
                "Twiddle",
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
        let mut cases = vec![(-1, "ready"), (0, "ready"), (1, "ready")];
        if name.starts_with("Harmonized Trio")
            || name == "Devout Chaplain"
            || name == "Lathril, Blade of the Elves"
        {
            cases.push((0, "fresh"));
            cases.push((0, "tapped"));
        }
        for (extra, state) in cases {
            let (status, expected, actual, evidence) = match run(&defs, name, extra, state) {
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
            rows.push(json!({"card":name,"scenario":{"extra_resources":extra,"state":state},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Eight fixed-multiple tap paths with source tap symbols, actual triggered resources, prepare-cost payments (linked preparation effects unverified), temporary effects and damage/death. Required-minus-one/exact/surplus plus fresh/tapped tap-symbol controls.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_special_tap_cost_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/special-tap-cost-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
