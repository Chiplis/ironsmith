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

const NAMES: [&str; 5] = [
    "Cryptic Cruiser",
    "Oracle of Dust",
    "Void Attendant",
    "Leashling",
    "Penance",
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
fn yield_priority(g: &mut GameState, to: PlayerId, dm: &mut Choices) -> Result<(), String> {
    if g.turn.priority_player == Some(to) {
        return Ok(());
    }
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(LegalAction::PassPriority),
        dm,
    )
    .map_err(|e| e.to_string())?;
    if g.turn.priority_player != Some(to) {
        return Err("priority did not pass to intended player".into());
    }
    Ok(())
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    resources: usize,
    own_exile: bool,
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
    let processor = !["Leashling", "Penance"].contains(&name);
    let mut target = None;
    for _ in 0..6 {
        for actor in [PlayerId(0), PlayerId(1)] {
            g.create_object_from_definition(
                &defs[if processor { "Plains" } else { "Mountain" }],
                actor,
                Zone::Library,
            );
        }
    }
    if processor {
        next_main(&mut g);
        if name == "Cryptic Cruiser" {
            let e = cast(&mut g, &defs["Grizzly Bears"], 1, &mut dm)?;
            if e["mana_paid"] != 2 || !e["resolution_error"].is_null() {
                return Err(format!("Bob target cast: {e}"));
            }
            producers.push(e);
            target = Some(find(&g, "Grizzly Bears", Zone::Battlefield)?);
        }
        if own_exile {
            next_main(&mut g);
        }
        for _ in 0..resources {
            let owner = if own_exile { 0 } else { 1 };
            let e = cast(&mut g, &defs["Ornithopter"], owner, &mut dm)?;
            if e["mana_paid"] != 0 || !e["resolution_error"].is_null() {
                return Err(format!("resource cast: {e}"));
            }
            producers.push(e);
            let id = find(&g, "Ornithopter", Zone::Battlefield)?;
            dm.targets = vec![Target::Object(id)];
            producers.push(paid(&mut g, defs, "Swords to Plowshares", &mut dm, 1)?);
        }
        if !own_exile {
            next_main(&mut g);
        }
    } else {
        for _ in 0..resources {
            g.create_object_from_definition(&defs["Island"], PlayerId(0), Zone::Library);
        }
        for _ in 0..resources {
            producers.push(paid(&mut g, defs, "Reach Through Mists", &mut dm, 1)?);
        }
    }
    dm.targets.clear();
    let source_cast = paid(
        &mut g,
        defs,
        name,
        &mut dm,
        match name {
            "Cryptic Cruiser" => 4,
            "Oracle of Dust" => 5,
            "Void Attendant" | "Penance" => 3,
            "Leashling" => 6,
            _ => unreachable!(),
        },
    )?;
    let source = find(&g, name, Zone::Battlefield)?;
    if name == "Penance" {
        fund(&mut g, PlayerId(1));
        yield_priority(&mut g, PlayerId(1), &mut dm)?;
        dm.targets = vec![Target::Player(PlayerId(0))];
        let (_q, e) = announce(&mut g, &defs["Shock"], 1, &mut dm)?;
        if e["mana_paid"] != 1 {
            return Err("pending Shock payment mismatch".into());
        }
        producers.push(e);
        // The caster receives priority after announcing Shock. Pass it normally to Alice.
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(g.players_in_game());
        apply_priority_response_with_dm(
            &mut g,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            &mut dm,
        )
        .map_err(|e| e.to_string())?;
        if g.turn.priority_player != Some(PlayerId(0)) || g.stack.len() != 1 {
            return Err("pending Shock priority handoff failed".into());
        }
        producers.push(json!({"action":"Bob passes priority to Alice with his paid Shock pending","stack_length":g.stack.len()}));
    }
    dm.names = vec![
        if processor {
            "Ornithopter".into()
        } else {
            "Island".into()
        },
        "Shock".into(),
    ];
    dm.targets = target
        .map(|id| vec![Target::Object(id)])
        .unwrap_or_default();
    let current = g.current_abilities(source).ok_or("abilities absent")?;
    let (index, ability) = current
        .iter()
        .enumerate()
        .filter_map(|(i, a)| match &a.kind {
            ironsmith::ability::AbilityKind::Activated(x) => Some((i, x)),
            _ => None,
        })
        .last()
        .ok_or("activated ability absent")?;
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered=actions.iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    let before = json!({"source_ability_index":index,"hand_islands":count(&g,"Island",Zone::Hand),"exiled_ornithopters":g.objects_in_deterministic_order().iter().filter(|o|o.name=="Ornithopter"&&o.zone==Zone::Exile).map(|o|json!({"id":o.id.0,"owner":o.owner.index()})).collect::<Vec<_>>(),"library_bottom_to_top":g.player(PlayerId(0)).unwrap().library.iter().filter_map(|id|g.object(*id).map(|o|o.name.to_string())).collect::<Vec<_>>(),"actions":format!("{actions:?}"),"component_checks":ability.mana_cost.costs().iter().map(|c|format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*c.0,&g,&ironsmith::costs::CostCheckContext::new(source,PlayerId(0)).with_reason(ironsmith::costs::PaymentReason::ActivateAbility)))).collect::<Vec<_>>()});
    let valid = resources > 0 && !own_exile;
    if !valid || !offered {
        return Ok((
            json!({"activation_available":valid}),
            json!({"activation_available":offered}),
            json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace,"scope":"Unavailable action is not forced; later movement/effects unexecuted, including pending Shock in Penance zero-hand case."}),
        ));
    }
    let mana_before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation = action(&mut g, source, index, false, &mut dm)
        .unwrap_or_else(|e| json!({"resolution_error":e}));
    let mana_paid = mana_before - g.player(PlayerId(0)).unwrap().mana_pool.total();
    let life_after = g.player(PlayerId(0)).unwrap().life;
    let mut followup = None;
    let mut token_before = None;
    let mut token_after = None;
    let mut scion_mana = None;
    if name == "Penance" {
        yield_priority(&mut g, PlayerId(1), &mut dm)?;
        dm.targets = vec![Target::Player(PlayerId(0))];
        let e = cast(&mut g, &defs["Shock"], 1, &mut dm)?;
        if e["mana_paid"] != 1 {
            return Err("second Shock payment mismatch".into());
        }
        followup = Some(e);
    }
    if name == "Void Attendant" {
        let tokens = g
            .objects_in_deterministic_order()
            .iter()
            .filter(|o| {
                o.kind == ironsmith::object::ObjectKind::Token && o.zone == Zone::Battlefield
            })
            .map(|o| o.id)
            .collect::<Vec<_>>();
        token_before=Some(tokens.iter().map(|id|{let c=g.calculated_characteristics(*id).unwrap();json!({"power":c.power,"toughness":c.toughness,"colors":c.colors,"types":c.card_types.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"subtypes":c.subtypes.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"controller":c.controller.index()})}).collect::<Vec<_>>());
        if let Some(id) = tokens.first() {
            let before = g.player(PlayerId(0)).unwrap().mana_pool.colorless;
            let ability = g.current_abilities(*id).unwrap();
            let idx = ability
                .iter()
                .enumerate()
                .find(|(_, a)| a.is_mana_ability())
                .map(|(i, _)| i)
                .ok_or("Scion mana ability absent")?;
            followup = Some(
                action(&mut g, *id, idx, true, &mut dm)
                    .unwrap_or_else(|e| json!({"resolution_error":e})),
            );
            scion_mana = Some(g.player(PlayerId(0)).unwrap().mana_pool.colorless - before);
        }
        token_after = Some(
            g.objects_in_deterministic_order()
                .iter()
                .filter(|o| {
                    o.kind == ironsmith::object::ObjectKind::Token && o.zone == Zone::Battlefield
                })
                .count(),
        );
    }
    let actual = json!({"error":activation["resolution_error"],"mana_paid":mana_paid,"source_battlefield":count(&g,name,Zone::Battlefield),"source_hand":count(&g,name,Zone::Hand),"island_hand":count(&g,"Island",Zone::Hand),"library_top":g.player(PlayerId(0)).unwrap().library.last().and_then(|id|g.object(*id).map(|o|o.name.to_string())),"alice_library_count":g.player(PlayerId(0)).unwrap().library.len(),"ornithopter_exile":count(&g,"Ornithopter",Zone::Exile),"bob_ornithopter_graveyard":g.objects_in_deterministic_order().iter().filter(|o|o.name=="Ornithopter"&&o.zone==Zone::Graveyard&&o.owner==PlayerId(1)).count(),"target_tapped":target.map(|id|g.is_tapped(id)),"plains_graveyard":count(&g,"Plains",Zone::Graveyard),"plains_hand":count(&g,"Plains",Zone::Hand),"alice_life_after_activation":life_after,"alice_life_after_followup":g.player(PlayerId(0)).unwrap().life,"scion_before":token_before,"scion_after":token_after,"scion_colorless_added":scion_mana,"followup_error":followup.as_ref().map(|f|f["resolution_error"].clone()),"stack_length":g.stack.len()});
    let expected = json!({"error":null,"mana_paid":match name{"Cryptic Cruiser"=>3,"Oracle of Dust"|"Void Attendant"=>2,_=>0},"source_battlefield":if name=="Leashling"{0}else{1},"source_hand":if name=="Leashling"{1}else{0},"island_hand":if processor{0}else{resources-1},"library_top":if processor{"Plains"}else{"Island"},"alice_library_count":if name=="Oracle of Dust"{5}else if processor{6}else{7},"ornithopter_exile":if processor{resources-1}else{0},"bob_ornithopter_graveyard":if processor{1}else{0},"target_tapped":target.map(|_|true),"plains_graveyard":if name=="Oracle of Dust"{1}else{0},"plains_hand":0,"alice_life_after_activation":20,"alice_life_after_followup":if name=="Penance"{18}else{20},"scion_before":if name=="Void Attendant"{Some(vec![json!({"power":1,"toughness":1,"colors":0,"types":["Creature"],"subtypes":["Eldrazi","Scion"],"controller":0})])}else{None},"scion_after":if name=="Void Attendant"{Some(0)}else{None},"scion_colorless_added":if name=="Void Attendant"{Some(1)}else{None},"followup_error":if ["Penance","Void Attendant"].contains(&name){Some(Value::Null)}else{None},"stack_length":0});
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation":activation,"followup":followup,"activation_dispatched":true,"choice_trace":dm.trace}),
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
fn report_single_move_costs() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Plains",
                "Mountain",
                "Island",
                "Ornithopter",
                "Grizzly Bears",
                "Swords to Plowshares",
                "Reach Through Mists",
                "Shock",
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
        for (resources, own_exile) in [(0, false), (1, false), (2, false)].into_iter().chain(
            if !["Leashling", "Penance"].contains(&name) {
                Some((1, true))
            } else {
                None
            },
        ) {
            let (status, expected, actual, evidence) = match run(&defs, name, resources, own_exile)
            {
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
            rows.push(json!({"card":name,"scenario":{"resources":resources,"own_exile":own_exile},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Five single-object move costs: actual paid sources, actual opponent exile producers via paid Swords/Ornithopter, actual Reach Through Mists hand resources.0/1/2 resources and opponent-ownership negatives. Canonical exact outcomes, pending Shock prevention and second-Shock expiration, Scion sacrifice mana. Unavailable actions never forced.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_single_move_cost_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/single-move-cost-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
