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
                let err = finish(g, &mut q, dm).err();
                return Ok(
                    json!({"action":format!("{a:?}"),"mana_change":g.player(PlayerId(0)).unwrap().mana_pool.total() as i64-before as i64,"resolution_error":err}),
                );
            }
        }
    }
    Err("activation decision budget".into())
}

const NAMES: [&str; 8] = [
    "Deathless Ancient",
    "Gangrenous Goliath",
    "Summon the School",
    "Skirsdag High Priest",
    "Myr Turbine",
    "Gallows at Willow Hill",
    "Keeper of the Nine Gales",
    "Tradewind Rider",
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
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    extra: i32,
    state: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(2, 0);
    let mut dm = Choices {
        targets: vec![],
        names: vec![],
        x: 0,
        allow_optional: true,
        choose_untap: false,
        trace: vec![],
    };
    let mut producers = vec![];
    let grave = [
        "Deathless Ancient",
        "Gangrenous Goliath",
        "Summon the School",
    ]
    .contains(&name);
    let tap_creature = [
        "Skirsdag High Priest",
        "Keeper of the Nine Gales",
        "Tradewind Rider",
    ]
    .contains(&name);
    let mut target_stable = None;
    for _ in 0..8 {
        for actor in [PlayerId(0), PlayerId(1)] {
            g.create_object_from_definition(&defs["Plains"], actor, Zone::Library);
        }
    }
    if !grave && name != "Myr Turbine" {
        next_main(&mut g);
        let e = cast(&mut g, &defs["Hill Giant"], 1, &mut dm)?;
        if e["mana_paid"] != 4 || !e["resolution_error"].is_null() {
            return Err("Bob target cast failed".into());
        }
        producers.push(e);
        target_stable = Some(
            g.object(find(&g, "Hill Giant", Zone::Battlefield)?)
                .unwrap()
                .stable_id,
        );
        next_main(&mut g);
    }
    if name == "Myr Turbine" {
        g.create_object_from_definition(&defs["Myr Enforcer"], PlayerId(0), Zone::Library);
    }
    let source_cast = paid(
        &mut g,
        defs,
        name,
        &mut dm,
        match name {
            "Deathless Ancient" => 6,
            "Gangrenous Goliath" | "Myr Turbine" => 5,
            "Summon the School" | "Tradewind Rider" => 4,
            "Skirsdag High Priest" => 2,
            _ => 3,
        },
    )?;
    if grave && name != "Summon the School" {
        dm.targets = vec![Target::Object(find(&g, name, Zone::Battlefield)?)];
        producers.push(paid(&mut g, defs, "Murder", &mut dm, 3)?);
    }
    let source = find(
        &g,
        name,
        if grave {
            Zone::Graveyard
        } else {
            Zone::Battlefield
        },
    )?;
    if tap_creature && state != "fresh" {
        next_main(&mut g);
        next_main(&mut g);
    }
    let required = match name {
        "Deathless Ancient" | "Gangrenous Goliath" | "Gallows at Willow Hill" => 3,
        "Summon the School" => 4,
        "Myr Turbine" => 5,
        _ => 2,
    };
    let (producer, price) = match name {
        "Deathless Ancient" => ("Vampire Noble", 3),
        "Gangrenous Goliath" => ("Devoted Caretaker", 1),
        "Summon the School" => ("Merfolk of the Pearl Trident", 1),
        "Myr Turbine" => ("Myr Moonvessel", 1),
        "Gallows at Willow Hill" => ("Fugitive Wizard", 1),
        "Keeper of the Nine Gales" => ("Storm Crow", 2),
        _ => ("Grizzly Bears", 2),
    };
    let mut resources = if name == "Summon the School" {
        g.objects_in_deterministic_order()
            .iter()
            .filter(|o| {
                o.zone == Zone::Battlefield && o.kind == ironsmith::object::ObjectKind::Token
            })
            .map(|o| o.id)
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    if name == "Summon the School" && resources.len() != 2 {
        return Err("Summon actual token count wrong".into());
    }
    while resources.len() < (required + extra) as usize {
        dm.targets.clear();
        producers.push(paid(&mut g, defs, producer, &mut dm, price)?);
        let id = g
            .objects_in_deterministic_order()
            .iter()
            .filter(|o| {
                o.name == producer
                    && o.zone == Zone::Battlefield
                    && g.controller_of_id(o.id) == Some(PlayerId(0))
                    && !resources.contains(&o.id)
            })
            .map(|o| o.id)
            .next()
            .ok_or("resource missing")?;
        resources.push(id);
    }
    if extra < 0 {
        producers.push(paid(&mut g, defs, "Bonesplitter", &mut dm, 1)?);
    }
    if name == "Skirsdag High Priest" && state != "no_death" {
        dm.targets = vec![Target::Object(find(&g, "Hill Giant", Zone::Battlefield)?)];
        producers.push(paid(&mut g, defs, "Murder", &mut dm, 3)?);
    }
    if state == "tapped" {
        dm.targets = vec![Target::Object(source)];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(source) {
            return Err("Twiddle did not tap source".into());
        }
    }
    let target = target_stable.and_then(|s| {
        g.objects_in_deterministic_order()
            .iter()
            .find(|o| o.stable_id == s)
            .map(|o| o.id)
    });
    dm.targets = if [
        "Gallows at Willow Hill",
        "Keeper of the Nine Gales",
        "Tradewind Rider",
    ]
    .contains(&name)
    {
        target
            .map(|id| vec![Target::Object(id)])
            .unwrap_or_default()
    } else {
        vec![]
    };
    dm.names = resources
        .iter()
        .filter_map(|id| g.object(*id).map(|o| o.name.to_string()))
        .collect();
    dm.names.push("Myr Enforcer".into());
    let abilities = g
        .current_abilities(source)
        .ok_or("source abilities absent")?;
    let (index, ability) = abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| match &a.kind {
            ironsmith::ability::AbilityKind::Activated(x) => Some((i, x)),
            _ => None,
        })
        .last()
        .ok_or("source activation absent")?;
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered=actions.iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    if name == "Skirsdag High Priest"
        && g.turn_store.turn_history.total_creatures_died_this_turn()
            != if state == "no_death" { 0 } else { 1 }
    {
        return Err("Morbid fixture death history mismatch".into());
    }
    let before = json!({"creatures_died_this_turn":g.turn_store.turn_history.total_creatures_died_this_turn(),"source_index":index,"source_zone":format!("{:?}",g.object(source).unwrap().zone),"source_tapped":g.is_tapped(source),"source_sick":g.is_summoning_sick(source),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"sick":g.is_summoning_sick(*id),"tapped":g.is_tapped(*id)})).collect::<Vec<_>>(),"actions":format!("{actions:?}"),"cost":format!("{:?}",ability.mana_cost)});
    let valid = extra >= 0 && state != "tapped" && !(tap_creature && state == "fresh");
    if !valid || !offered {
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace,"scope":"Unpayable or unavailable action never forced; advertised insufficient-resource cases are only legality diagnostics."}),
        ));
    }
    let before_mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation = action(&mut g, source, index, false, &mut dm)
        .unwrap_or_else(|e| json!({"resolution_error":e}));
    let target = target_stable.and_then(|s| {
        g.objects_in_deterministic_order()
            .iter()
            .find(|o| o.stable_id == s)
            .map(|o| o.id)
    });
    let tokens=g.objects_in_deterministic_order().iter().filter(|o|o.kind==ironsmith::object::ObjectKind::Token&&o.zone==Zone::Battlefield).map(|o|{let c=g.calculated_characteristics(o.id).unwrap();json!({"power":c.power,"toughness":c.toughness,"colors":c.colors,"types":c.card_types.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"subtypes":c.subtypes.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"flying":g.object_has_static_ability_id(o.id,ironsmith::static_abilities::StaticAbilityId::Flying),"controller":c.controller.index()})}).collect::<Vec<_>>();
    let tutor=g.objects_in_deterministic_order().iter().find(|o|o.name=="Myr Enforcer"&&o.zone==Zone::Battlefield).map(|o|json!({"power":g.calculated_power(o.id),"toughness":g.calculated_toughness(o.id),"controller":g.controller_of_id(o.id).map(|p|p.index()),"is_myr":o.subtypes.iter().any(|t|format!("{t:?}")=="Myr"),"is_artifact":o.card_types.contains(&CardType::Artifact),"is_creature":o.card_types.contains(&CardType::Creature)}));
    let mut actual = json!({"error":activation["resolution_error"],"mana_paid":before_mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"source_hand":count(&g,name,Zone::Hand),"source_graveyard":count(&g,name,Zone::Graveyard),"source_battlefield":count(&g,name,Zone::Battlefield),"source_tapped":if grave{None}else{Some(g.is_tapped(source))},"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"target_zone":target.map(|id|format!("{:?}",g.object(id).unwrap().zone)),"tokens":tokens,"tutored_myr":tutor,"stack_length":g.stack.len(),"post_return_hand_activation":null});
    if grave && count(&g, name, Zone::Hand) == 1 {
        next_main(&mut g);
        next_main(&mut g);
        let hand = find(&g, name, Zone::Hand)?;
        actual["post_return_hand_activation"] = json!(
            compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
                .iter()
                .any(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==hand))
        );
    }
    if state == "no_death" {
        return Ok((
            json!({"activation_available":false,"error":null,"source_tapped":false,"tapped_resources":0,"demon_tokens":0}),
            json!({"activation_available":offered,"error":actual["error"],"source_tapped":actual["source_tapped"],"tapped_resources":actual["tapped_resources"],"demon_tokens":g.objects_in_deterministic_order().iter().filter(|o|o.zone==Zone::Battlefield&&o.kind==ironsmith::object::ObjectKind::Token&&o.subtypes.iter().any(|t|format!("{t:?}")=="Demon")).count()}),
            json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation":activation,"activation_dispatched":true,"complete_observed_state":actual,"choice_trace":dm.trace,"scope":"Payable action advertised and dispatched through normal priority despite zero recorded creature deaths. This tests the missing activation restriction, not an insufficient-resource payment."}),
        ));
    }
    let expected_tokens = match name {
        "Summon the School" => vec![
            json!({"power":1,"toughness":1,"colors":2,"types":["Creature"],"subtypes":["Merfolk","Wizard"],"flying":false,"controller":0});
            2
        ],
        "Skirsdag High Priest" => vec![
            json!({"power":5,"toughness":5,"colors":4,"types":["Creature"],"subtypes":["Demon"],"flying":true,"controller":0}),
        ],
        "Gallows at Willow Hill" => vec![
            json!({"power":1,"toughness":1,"colors":1,"types":["Creature"],"subtypes":["Spirit"],"flying":true,"controller":1}),
        ],
        _ => vec![],
    };
    let expected = json!({"error":null,"mana_paid":if name=="Gallows at Willow Hill"{3}else{0},"source_hand":if grave{1}else{0},"source_graveyard":0,"source_battlefield":if grave{0}else{1},"source_tapped":if grave{None}else{Some(true)},"tapped_resources":required,"untapped_resources":extra,"target_zone":target_stable.map(|_|if ["Gallows at Willow Hill","Skirsdag High Priest"].contains(&name){"Graveyard"}else{"Hand"}),"tokens":expected_tokens,"tutored_myr":if name=="Myr Turbine"{Some(json!({"power":4,"toughness":4,"controller":0,"is_myr":true,"is_artifact":true,"is_creature":true}))}else{None},"stack_length":0,"post_return_hand_activation":if grave{Some(false)}else{None}});
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation":activation,"activation_dispatched":true,"choice_trace":dm.trace}),
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
fn report_mixed_tap_costs() {
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
                "Vampire Noble",
                "Devoted Caretaker",
                "Merfolk of the Pearl Trident",
                "Myr Moonvessel",
                "Myr Enforcer",
                "Fugitive Wizard",
                "Storm Crow",
                "Grizzly Bears",
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
        if [
            "Skirsdag High Priest",
            "Keeper of the Nine Gales",
            "Tradewind Rider",
        ]
        .contains(&name)
        {
            cases.push((0, "fresh"));
        }
        if ![
            "Deathless Ancient",
            "Gangrenous Goliath",
            "Summon the School",
        ]
        .contains(&name)
        {
            cases.push((0, "tapped"));
        }
        if name == "Skirsdag High Priest" {
            cases.push((0, "no_death"));
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
    let r = json!({"scope":"Eight multi-tap graveyard-return and tap-source cost paths. Actual paid source/resource/target spells, actual deaths for graveyard/Morbid, actual source turn/untap and paid Twiddle. Required-minus-one/exact/surplus and zone/fresh/tapped/morbid controls. Invalid offered actions never dispatched; exact token/tutor/return outcomes checked.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_mixed_tap_cost_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/mixed-tap-cost-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
