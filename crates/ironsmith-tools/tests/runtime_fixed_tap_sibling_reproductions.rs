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
    "Aphetto Grifter",
    "Nullmage Shepherd",
    "Ghirapur Aether Grid",
    "Skystrike Officer",
    "Diversionary Tactics",
    "Cloudgoat Ranger",
    "Benthicore",
    "Cryptic Gateway",
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
fn advance_turn(g: &mut GameState) {
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
    mixed: bool,
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
    for _ in 0..6 {
        for actor in [PlayerId(0), PlayerId(1)] {
            g.create_object_from_definition(&defs["Plains"], actor, Zone::Library);
        }
    }
    let mut target = None;
    if name == "Diversionary Tactics" {
        advance_turn(&mut g);
        let e = cast(&mut g, &defs["Grizzly Bears"], 1, &mut dm)?;
        if e["mana_paid"] != 2 || !e["resolution_error"].is_null() {
            return Err("Bob target paidcast failed".into());
        }
        producers.push(e);
        target = Some(find(&g, "Grizzly Bears", Zone::Battlefield)?);
        advance_turn(&mut g);
    }
    let source_cast = paid(
        &mut g,
        defs,
        name,
        &mut dm,
        match name {
            "Aphetto Grifter" | "Ghirapur Aether Grid" | "Skystrike Officer" => 3,
            "Nullmage Shepherd" | "Diversionary Tactics" => 4,
            "Cloudgoat Ranger" | "Cryptic Gateway" => 5,
            "Benthicore" => 7,
            _ => unreachable!(),
        },
    )?;
    let source = find(&g, name, Zone::Battlefield)?;
    let required = match name {
        "Nullmage Shepherd" => 4,
        "Skystrike Officer" | "Cloudgoat Ranger" => 3,
        _ => 2,
    };
    let source_resource =
        ["Aphetto Grifter", "Nullmage Shepherd", "Skystrike Officer"].contains(&name);
    let mut resources = if source_resource {
        vec![source]
    } else {
        vec![]
    };
    let (producer, price) = match name {
        "Aphetto Grifter" => ("Fugitive Wizard", 1),
        "Skystrike Officer" => ("Elite Vanguard", 1),
        "Cloudgoat Ranger" => ("Kithkin Zephyrnaut", 3),
        "Benthicore" => ("Merfolk of the Pearl Trident", 1),
        "Cryptic Gateway" => ("Llanowar Elves", 1),
        _ => ("Ornithopter", 0),
    };
    if ["Cloudgoat Ranger", "Benthicore"].contains(&name) {
        resources = g
            .objects_in_deterministic_order()
            .iter()
            .filter(|o| {
                o.kind == ironsmith::object::ObjectKind::Token
                    && o.zone == Zone::Battlefield
                    && g.controller_of_id(o.id) == Some(PlayerId(0))
            })
            .map(|o| o.id)
            .collect();
        if resources.len() != required as usize {
            return Err(format!(
                "canonical source token producer count mismatch {} != {required}",
                resources.len()
            ));
        }
        if extra < 0 {
            dm.targets = vec![Target::Object(resources[0])];
            producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
            if !g.is_tapped(resources[0]) {
                return Err("resource Twiddle did not tap".into());
            }
        }
    }
    let want = (required
        + extra.max(if ["Cloudgoat Ranger", "Benthicore"].contains(&name) {
            0
        } else {
            -1
        })) as usize;
    while resources.len() < want {
        let n = if mixed && resources.len() == 1 {
            "Grizzly Bears"
        } else {
            producer
        };
        let price = if n == "Grizzly Bears" { 2 } else { price };
        producers.push(paid(&mut g, defs, n, &mut dm, price)?);
        let id = g
            .objects_in_deterministic_order()
            .iter()
            .filter(|o| {
                o.name == n
                    && o.zone == Zone::Battlefield
                    && g.controller_of_id(o.id) == Some(PlayerId(0))
                    && !resources.contains(&o.id)
            })
            .map(|o| o.id)
            .next()
            .ok_or("paid resource missing")?;
        resources.push(id);
    }
    if ["Aphetto Grifter", "Nullmage Shepherd"].contains(&name) {
        producers.push(paid(&mut g, defs, "Clock of Omens", &mut dm, 4)?);
        target = Some(find(&g, "Clock of Omens", Zone::Battlefield)?);
    }
    if name == "Benthicore" {
        dm.targets = vec![Target::Object(source)];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if !g.is_tapped(source) {
            return Err("source Twiddle did not tap".into());
        }
    }
    if extra < 0
        && [
            "Aphetto Grifter",
            "Skystrike Officer",
            "Cloudgoat Ranger",
            "Benthicore",
            "Ghirapur Aether Grid",
        ]
        .contains(&name)
    {
        let decoy = if name == "Ghirapur Aether Grid" {
            "Grizzly Bears"
        } else {
            "Ornithopter"
        };
        producers.push(paid(
            &mut g,
            defs,
            decoy,
            &mut dm,
            if decoy == "Grizzly Bears" { 2 } else { 0 },
        )?);
    }
    if ["Cloudgoat Ranger", "Benthicore"].contains(&name) {
        for id in resources
            .iter()
            .filter(|id| g.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token)
        {
            let c = g.calculated_characteristics(*id).unwrap();
            let subtypes = c
                .subtypes
                .iter()
                .map(|s| format!("{s:?}"))
                .collect::<Vec<_>>();
            let expected = if name == "Cloudgoat Ranger" {
                ["Kithkin", "Soldier"]
            } else {
                ["Merfolk", "Wizard"]
            };
            if c.power != Some(1)
                || c.toughness != Some(1)
                || !c.card_types.contains(&CardType::Creature)
                || !expected.iter().all(|s| subtypes.iter().any(|t| t == s))
                || c.colors
                    != if name == "Cloudgoat Ranger" {
                        ironsmith::color::ColorSet::WHITE
                    } else {
                        ironsmith::color::ColorSet::BLUE
                    }
            {
                return Err("actual source token characteristics mismatch".into());
            }
        }
    }
    if name == "Cryptic Gateway" {
        g.create_object_from_definition(&defs["Elvish Mystic"], PlayerId(0), Zone::Hand);
    }
    dm.names = resources
        .iter()
        .filter_map(|id| g.object(*id).map(|o| o.name.to_string()))
        .collect();
    dm.names.push("Elvish Mystic".into());
    dm.targets = if name == "Ghirapur Aether Grid" {
        vec![Target::Player(PlayerId(1))]
    } else {
        target
            .map(|id| vec![Target::Object(id)])
            .unwrap_or_default()
    };
    let abilities = g.current_abilities(source).ok_or("no current abilities")?;
    let (index, _) = abilities
        .iter()
        .enumerate()
        .filter(|(_, a)| matches!(&a.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .last()
        .ok_or("no activated ability")?;
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered=actions.iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    let before = json!({"resources":resources.iter().map(|id|json!({"id":id.0,"name":g.object(*id).unwrap().name.to_string(),"tapped":g.is_tapped(*id),"fresh":g.is_summoning_sick(*id),"characteristics":format!("{:?}",g.calculated_characteristics(*id))})).collect::<Vec<_>>(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"source_index":index,"available_actions":format!("{actions:?}"),"target":target.map(|id|json!({"id":id.0,"name":g.object(id).unwrap().name.to_string(),"tapped":g.is_tapped(id),"controller":g.controller_of_id(id).map(|p|p.index())}))});
    if extra < 0 || !offered {
        return Ok((
            json!({"activation_available":extra>=0}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace}),
        ));
    }
    let mana_before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation = action(&mut g, source, index, false, &mut dm)
        .unwrap_or_else(|e| json!({"resolution_error":e}));
    let actual = json!({"error":activation["resolution_error"],"mana_paid":mana_before-g.player(PlayerId(0)).unwrap().mana_pool.total(),"tapped_resources":resources.iter().filter(|id|g.is_tapped(**id)).count(),"untapped_resources":resources.iter().filter(|id|!g.is_tapped(**id)).count(),"target_battlefield":target.map(|id|g.object(id).is_some_and(|o|o.zone==Zone::Battlefield)),"target_tapped":target.map(|id|g.is_tapped(id)),"clock_graveyard":count(&g,"Clock of Omens",Zone::Graveyard),"bob_life":g.player(PlayerId(1)).unwrap().life,"plains_hand":count(&g,"Plains",Zone::Hand),"mystic_hand":count(&g,"Elvish Mystic",Zone::Hand),"mystic_battlefield":count(&g,"Elvish Mystic",Zone::Battlefield),"source_power":g.calculated_power(source),"source_toughness":g.calculated_toughness(source),"source_flying":g.object_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Flying),"source_shroud":g.object_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Shroud),"source_tapped":g.is_tapped(source),"stack_length":g.stack.len()});
    let (power, toughness) = match name {
        "Aphetto Grifter" => (Some(1), Some(1)),
        "Nullmage Shepherd" => (Some(2), Some(4)),
        "Skystrike Officer" => (Some(2), Some(3)),
        "Cloudgoat Ranger" => (Some(5), Some(3)),
        "Benthicore" => (Some(5), Some(5)),
        _ => (None, None),
    };
    let expected = json!({"error":null,"mana_paid":0,"tapped_resources":required,"untapped_resources":extra,"target_battlefield":target.map(|_|name!="Nullmage Shepherd"),"target_tapped":target.map(|_|name!="Nullmage Shepherd"),"clock_graveyard":if name=="Nullmage Shepherd"{1}else{0},"bob_life":if name=="Ghirapur Aether Grid"{19}else{20},"plains_hand":if name=="Skystrike Officer"{1}else{0},"mystic_hand":if name=="Cryptic Gateway"&&mixed{1}else{0},"mystic_battlefield":if name=="Cryptic Gateway"&&!mixed{1}else{0},"source_power":power,"source_toughness":toughness,"source_flying":(["Cloudgoat Ranger","Skystrike Officer"].contains(&name)),"source_shroud":name=="Benthicore","source_tapped":source_resource,"stack_length":0});
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
fn report_fixed_tap_siblings() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Plains",
                "Ornithopter",
                "Fugitive Wizard",
                "Elite Vanguard",
                "Kithkin Zephyrnaut",
                "Merfolk of the Pearl Trident",
                "Llanowar Elves",
                "Elvish Mystic",
                "Grizzly Bears",
                "Clock of Omens",
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
        for (extra, mixed) in
            [(-1, false), (0, false), (1, false)]
                .into_iter()
                .chain(if name == "Cryptic Gateway" {
                    Some((0, true))
                } else {
                    None
                })
        {
            let (status, expected, actual, evidence) = match run(&defs, name, extra, mixed) {
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
            rows.push(json!({"card":name,"scenario":{"extra_resources":extra,"mixed_types":mixed},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Eight fixed multi-tap sibling paths with canonical paid sources/resources, natural ETB tokens, real target spells and explicit choices. Required-minus-one/exact/extra resources. Cryptic Gateway also two unlike tapped creature types; no unavailable action forced.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_fixed_tap_sibling_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/fixed-tap-sibling-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
