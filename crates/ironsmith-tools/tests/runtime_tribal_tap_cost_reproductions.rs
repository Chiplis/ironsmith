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
    "Ancestor's Prophet",
    "Captivating Vampire",
    "Gilt-Leaf Archdruid",
    "Voice of the Woods",
    "Supreme Inquisitor",
    "Hand of Justice",
    "Catapult Master",
    "Gravespawn Sovereign",
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
fn land(
    g: &mut GameState,
    def: &CardDefinition,
    actor: PlayerId,
    dm: &mut Choices,
) -> Result<Value, String> {
    let id = g.create_object_from_definition(def, actor, Zone::Hand);
    let a = compute_legal_actions(g, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id,..}if *land_id==id))
        .ok_or("land action unavailable")?;
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(a.clone()),
        dm,
    )
    .map_err(|e| e.to_string())?;
    finish(g, &mut q, dm)?;
    Ok(json!({"action":format!("{a:?}"),"actor":actor.index(),"card":def.name()}))
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    extra: i32,
    state: &str,
    exile_count: usize,
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
    let mut target_stable = None;
    for actor in [PlayerId(0), PlayerId(1), PlayerId(2)] {
        if name == "Supreme Inquisitor" && actor == PlayerId(1) {
            for n in [
                "Plains", "Plains", "Plains", "Plains", "Plains", "Island", "Mountain", "Swamp",
            ] {
                g.create_object_from_definition(&defs[n], actor, Zone::Library);
            }
        } else {
            for _ in 0..40 {
                g.create_object_from_definition(&defs["Plains"], actor, Zone::Library);
            }
        }
    }
    next_main(&mut g);
    if [
        "Captivating Vampire",
        "Hand of Justice",
        "Catapult Master",
        "Gravespawn Sovereign",
    ]
    .contains(&name)
    {
        let e = cast(&mut g, &defs["Hill Giant"], 1, &mut dm)?;
        if e["mana_paid"] != 4 || !e["resolution_error"].is_null() {
            return Err("paid Bob target failed".into());
        }
        producers.push(e);
        let id = find(&g, "Hill Giant", Zone::Battlefield)?;
        target_stable = Some(g.object(id).unwrap().stable_id);
    }
    if name == "Gilt-Leaf Archdruid" {
        let e = cast(&mut g, &defs["Exploration"], 1, &mut dm)?;
        if e["mana_paid"] != 1 || !e["resolution_error"].is_null() {
            return Err("paid Bob Exploration failed".into());
        }
        producers.push(e);
        producers.push(land(&mut g, &defs["Island"], PlayerId(1), &mut dm)?);
        producers.push(land(&mut g, &defs["Forest"], PlayerId(1), &mut dm)?);
    }
    next_main(&mut g);
    if name == "Gilt-Leaf Archdruid" {
        producers.push(land(&mut g, &defs["Mountain"], PlayerId(2), &mut dm)?);
    }
    next_main(&mut g);
    if name == "Gilt-Leaf Archdruid" {
        producers.push(land(&mut g, &defs["Plains"], PlayerId(0), &mut dm)?);
    }
    if name == "Gravespawn Sovereign" {
        dm.targets = vec![Target::Object(find(&g, "Hill Giant", Zone::Battlefield)?)];
        producers.push(paid(&mut g, defs, "Murder", &mut dm, 3)?);
    }
    dm.targets.clear();
    let source_cast = paid(
        &mut g,
        defs,
        name,
        &mut dm,
        match name {
            "Captivating Vampire" => 3,
            "Hand of Justice" | "Gravespawn Sovereign" => 6,
            _ => 5,
        },
    )?;
    let source = find(&g, name, Zone::Battlefield)?;
    let required = if name == "Gilt-Leaf Archdruid" {
        7
    } else if name == "Hand of Justice" {
        3
    } else {
        5
    };
    let mut resources = if name == "Hand of Justice" {
        vec![]
    } else {
        vec![source]
    };
    let (producer, price) = match name {
        "Ancestor's Prophet" => ("Devoted Caretaker", 1),
        "Captivating Vampire" => ("Vampire Noble", 3),
        "Gilt-Leaf Archdruid" | "Voice of the Woods" => ("Llanowar Elves", 1),
        "Supreme Inquisitor" => ("Fugitive Wizard", 1),
        "Hand of Justice" => ("Savannah Lions", 1),
        "Catapult Master" => ("Elite Vanguard", 1),
        "Gravespawn Sovereign" => ("Walking Corpse", 2),
        _ => unreachable!(),
    };
    while resources.len() < (required + extra) as usize {
        producers.push(paid(&mut g, defs, producer, &mut dm, price)?);
        let id = g
            .objects_in_deterministic_order()
            .iter()
            .filter(|o| {
                o.name == producer && o.zone == Zone::Battlefield && !resources.contains(&o.id)
            })
            .map(|o| o.id)
            .next()
            .ok_or("resource absent")?;
        resources.push(id);
    }
    if extra < 0 {
        producers.push(paid(&mut g, defs, "Ornithopter", &mut dm, 0)?);
    }
    if name == "Hand of Justice" && state != "fresh" {
        for _ in 0..3 {
            next_main(&mut g);
        }
    }
    if state == "tapped" {
        dm.targets = vec![Target::Object(source)];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(source) {
            return Err("source Twiddle did not tap".into());
        }
    }
    let target = target_stable.and_then(|stable| {
        g.objects_in_deterministic_order()
            .iter()
            .find(|o| o.stable_id == stable)
            .map(|o| o.id)
    });
    dm.targets = if ["Gilt-Leaf Archdruid", "Supreme Inquisitor"].contains(&name) {
        vec![Target::Player(PlayerId(1))]
    } else {
        target
            .map(|id| vec![Target::Object(id)])
            .unwrap_or_default()
    };
    dm.names = vec![producer.into()];
    if name != "Hand of Justice" {
        dm.names.insert(0, name.into());
    }
    if name == "Supreme Inquisitor" {
        if exile_count == 5 {
            dm.names.push("Plains".into());
        } else if exile_count == 1 {
            dm.names.push("Island".into());
        }
    }
    let abilities = g.current_abilities(source).ok_or("abilities absent")?;
    let (index, ability) = abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| match &a.kind {
            ironsmith::ability::AbilityKind::Activated(x) => Some((i, x)),
            _ => None,
        })
        .last()
        .ok_or("ability absent")?;
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered=actions.iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    let before = json!({"source_ability_index":index,"source_sick":g.is_summoning_sick(source),"source_tapped":g.is_tapped(source),"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"sick":g.is_summoning_sick(*id),"tapped":g.is_tapped(*id)})).collect::<Vec<_>>(),"actions":format!("{actions:?}"),"cost":format!("{:?}",ability.mana_cost),"alice_life":g.player(PlayerId(0)).unwrap().life});
    let valid = extra >= 0 && !(name == "Hand of Justice" && state != "ready");
    if !valid || !offered {
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace,"scope":"Unpayable and unavailable actions never forced. An advertised action in an insufficient-resource state is only a legality diagnostic."}),
        ));
    }
    let before_life = g.player(PlayerId(0)).unwrap().life;
    let before_mana = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation = action(&mut g, source, index, false, &mut dm)
        .unwrap_or_else(|e| json!({"resolution_error":e}));
    let target = target_stable.and_then(|stable| {
        g.objects_in_deterministic_order()
            .iter()
            .find(|o| o.stable_id == stable)
            .map(|o| o.id)
    });
    let target_summary=target.map(|id|{let o=g.object(id).unwrap();json!({"zone":format!("{:?}",o.zone),"controller":g.controller_of_id(id).map(|p|p.index()),"owner":o.owner.index(),"is_vampire":g.calculated_characteristics(id).is_some_and(|c|c.subtypes.iter().any(|t|format!("{t:?}")=="Vampire")),"power":g.calculated_power(id),"toughness":g.calculated_toughness(id)})});
    let tokens=g.objects_in_deterministic_order().iter().filter(|o|o.kind==ironsmith::object::ObjectKind::Token&&o.zone==Zone::Battlefield).map(|o|{let c=g.calculated_characteristics(o.id).unwrap();json!({"power":c.power,"toughness":c.toughness,"colors":c.colors,"types":c.card_types.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"subtypes":c.subtypes.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"trample":g.object_has_static_ability_id(o.id,ironsmith::static_abilities::StaticAbilityId::Trample),"controller":c.controller.index()})}).collect::<Vec<_>>();
    let actual = json!({"error":activation["resolution_error"],"mana_paid":before_mana-g.player(PlayerId(0)).unwrap().mana_pool.total(),"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"source_tapped":g.is_tapped(source),"alice_life_gain":g.player(PlayerId(0)).unwrap().life-before_life,"target":target_summary,"tokens":tokens,"land_controller_counts":(0u8..3).map(|i|g.objects_in_deterministic_order().iter().filter(|o|o.zone==Zone::Battlefield&&o.card_types.contains(&CardType::Land)&&g.controller_of_id(o.id)==Some(PlayerId(i))).count()).collect::<Vec<_>>(),"bob_library_count":g.player(PlayerId(1)).unwrap().library.len(),"exiled_plains":count(&g,"Plains",Zone::Exile),"exiled_island":count(&g,"Island",Zone::Exile),"stack_length":g.stack.len()});
    let expected_target=target_stable.map(|_|json!({"zone":if name=="Hand of Justice"{"Graveyard"}else if name=="Catapult Master"{"Exile"}else{"Battlefield"},"controller":if ["Captivating Vampire","Gravespawn Sovereign"].contains(&name){0}else{1},"owner":1,"is_vampire":name=="Captivating Vampire","power":if name=="Captivating Vampire"{4}else{3},"toughness":if name=="Captivating Vampire"{4}else{3}}));
    let expected = json!({"error":null,"mana_paid":0,"tapped_resources":required,"untapped_resources":extra,"source_tapped":true,"alice_life_gain":if name=="Ancestor's Prophet"{10}else{0},"target":expected_target,"tokens":if name=="Voice of the Woods"{vec![json!({"power":7,"toughness":7,"colors":16,"types":["Creature"],"subtypes":["Elemental"],"trample":true,"controller":0})]}else{vec![]},"land_controller_counts":if name=="Gilt-Leaf Archdruid"{vec![3,0,1]}else{vec![0,0,0]},"bob_library_count":if name=="Supreme Inquisitor"{8-exile_count}else{40},"exiled_plains":if name=="Supreme Inquisitor"&&exile_count==5{5}else{0},"exiled_island":if name=="Supreme Inquisitor"&&exile_count==1{1}else{0},"stack_length":0});
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
fn report_tribal_tap_costs() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Plains",
                "Island",
                "Forest",
                "Mountain",
                "Swamp",
                "Ornithopter",
                "Hill Giant",
                "Devoted Caretaker",
                "Vampire Noble",
                "Llanowar Elves",
                "Fugitive Wizard",
                "Savannah Lions",
                "Elite Vanguard",
                "Walking Corpse",
                "Exploration",
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
        let mut cases = vec![(-1, "ready", 5), (0, "ready", 5), (1, "ready", 5)];
        if name == "Hand of Justice" {
            cases.extend([(0, "fresh", 5), (0, "tapped", 5)]);
        }
        if name == "Supreme Inquisitor" {
            cases.extend([(0, "ready", 0), (0, "ready", 1)]);
        }
        for (extra, state, exile_count) in cases {
            let (status, expected, actual, evidence) =
                match run(&defs, name, extra, state, exile_count) {
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
            rows.push(json!({"card":name,"scenario":{"extra_resources":extra,"state":state,"exile_count":if name=="Supreme Inquisitor"{Some(exile_count)}else{None}},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Eight tribal fixed-multiple tap cost paths with actual paid sources/resources and targets. Required-minus-one/exact/surplus, ineligible Ornithopter decoys, actual lands/control transfer, actual death/reanimation, exact token characteristics. Hand of Justice uses real turns/untap and paid Twiddle for tap-source controls; unpayable actions never dispatched. Supreme Inquisitor0/1/5 search choices.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_tribal_tap_cost_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/tribal-tap-cost-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
