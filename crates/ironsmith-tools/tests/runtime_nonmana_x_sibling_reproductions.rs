//! Fixed-mana/nonmana-X sibling activations with real paid sources and resource producers.
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

const NAMES: [&str; 8] = [
    "Champion of Stray Souls",
    "Chatterfang, Squirrel General",
    "Grim Hireling",
    "Insatiable Frugivore",
    "Krav, the Unredeemed",
    "Ruthless Technomancer",
    "Taigam, Sidisi's Hand",
    "Winter, Cursed Rider",
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
    for _ in 0..12 {
        g.create_object_from_definition(&defs["Plains"], PlayerId(0), Zone::Library);
    }
    let funded = variant > 0;
    let requested = if name == "Ruthless Technomancer" {
        if variant == 2 { 2 } else { 1 }
    } else if variant == 2 {
        2
    } else {
        0
    };
    let mut d = Choices {
        amount: requested,
        actual_x: None,
        targets: vec![Target::Player(PlayerId(0))],
        reveal_ids: vec![],
        revealed: Default::default(),
        trace: vec![],
    };
    let mut history = vec![];
    let source_mana = match name {
        "Champion of Stray Souls" => 6,
        "Chatterfang, Squirrel General" => 3,
        "Krav, the Unredeemed" | "Taigam, Sidisi's Hand" => 5,
        "Winter, Cursed Rider" => 2,
        _ => 4,
    };
    paid(&mut g, defs, name, 0, source_mana, &mut d, &mut history)?;
    let source = find(&g, name, Zone::Battlefield)?;
    if matches!(
        name,
        "Champion of Stray Souls" | "Taigam, Sidisi's Hand" | "Winter, Cursed Rider"
    ) {
        d.targets.clear();
        paid(&mut g, defs, "Fervor", 0, 3, &mut d, &mut history)?;
    }
    let mut target = None;
    if matches!(
        name,
        "Chatterfang, Squirrel General"
            | "Grim Hireling"
            | "Taigam, Sidisi's Hand"
            | "Winter, Cursed Rider"
    ) {
        d.targets.clear();
        paid(&mut g, defs, "Hill Giant", 0, 4, &mut d, &mut history)?;
        target = Some(find(&g, "Hill Giant", Zone::Battlefield)?);
    }
    // Sacrifice/return costs get real graveyard targets even in the no-resource controls.
    if matches!(name, "Champion of Stray Souls" | "Ruthless Technomancer") {
        for _ in 0..2 {
            d.targets.clear();
            paid(&mut g, defs, "Ornithopter", 0, 0, &mut d, &mut history)?;
            let id = find(&g, "Ornithopter", Zone::Battlefield)?;
            d.targets = vec![Target::Object(id)];
            paid(&mut g, defs, "Murder", 0, 3, &mut d, &mut history)?;
        }
    }
    if funded {
        match name {
            "Champion of Stray Souls" | "Krav, the Unredeemed" => {
                for _ in 0..2 {
                    d.targets.clear();
                    paid(&mut g, defs, "Grizzly Bears", 0, 2, &mut d, &mut history)?;
                }
            }
            "Chatterfang, Squirrel General" => {
                d.targets.clear();
                paid(
                    &mut g,
                    defs,
                    "Chatter of the Squirrel",
                    0,
                    1,
                    &mut d,
                    &mut history,
                )?;
            }
            "Grim Hireling" => {
                for _ in 0..2 {
                    d.targets.clear();
                    paid(&mut g, defs, "Strike It Rich", 0, 1, &mut d, &mut history)?;
                }
            }
            "Insatiable Frugivore" => {
                d.targets.clear();
                paid(&mut g, defs, "Gilded Goose", 0, 1, &mut d, &mut history)?;
            }
            "Ruthless Technomancer" => {
                for _ in 0..2 {
                    d.targets.clear();
                    paid(&mut g, defs, "Ornithopter", 0, 0, &mut d, &mut history)?;
                }
            }
            "Taigam, Sidisi's Hand" => {
                for _ in 0..2 {
                    d.targets = vec![Target::Player(PlayerId(1))];
                    paid(&mut g, defs, "Shock", 0, 1, &mut d, &mut history)?;
                }
            }
            "Winter, Cursed Rider" => {
                for _ in 0..2 {
                    d.targets.clear();
                    paid(&mut g, defs, "Ornithopter", 0, 0, &mut d, &mut history)?;
                    let id = find(&g, "Ornithopter", Zone::Battlefield)?;
                    d.targets = vec![Target::Object(id)];
                    paid(&mut g, defs, "Murder", 0, 3, &mut d, &mut history)?;
                }
            }
            _ => unreachable!(),
        }
    }
    let resources = match name {
        "Champion of Stray Souls" | "Krav, the Unredeemed" => {
            count(&g, "Grizzly Bears", Zone::Battlefield)
        }
        "Chatterfang, Squirrel General" => count(&g, "Squirrel", Zone::Battlefield),
        "Grim Hireling" => count(&g, "Treasure", Zone::Battlefield),
        "Insatiable Frugivore" => count(&g, "Food", Zone::Battlefield),
        "Ruthless Technomancer" => count(&g, "Ornithopter", Zone::Battlefield),
        "Taigam, Sidisi's Hand" => count(&g, "Shock", Zone::Graveyard),
        "Winter, Cursed Rider" => count(&g, "Ornithopter", Zone::Graveyard),
        _ => 0,
    };
    let expected_resources = if name == "Insatiable Frugivore" {
        if funded { 2 } else { 1 }
    } else if funded {
        2
    } else {
        0
    };
    if resources != expected_resources {
        return Err(format!(
            "producer resource count {resources} != {expected_resources}"
        ));
    }
    let capacity = resources
        + if matches!(
            name,
            "Chatterfang, Squirrel General" | "Krav, the Unredeemed"
        ) {
            1
        } else {
            0
        };
    let index = match name {
        "Champion of Stray Souls" => 0,
        "Grim Hireling" | "Insatiable Frugivore" | "Ruthless Technomancer" => 1,
        _ => 2,
    };
    d.targets = if name == "Krav, the Unredeemed" {
        vec![Target::Player(PlayerId(0))]
    } else if name == "Ruthless Technomancer" {
        vec![Target::Object(find(&g, "Ornithopter", Zone::Graveyard)?)]
    } else {
        target.map(Target::Object).into_iter().collect()
    };
    d.actual_x = None;
    d.trace.clear();
    let objects_before = board(&g);
    let mana_before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    g.turn.priority_player = Some(PlayerId(0));
    let offered=compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state").iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    let ability = g
        .current_ability(source, index)
        .ok_or("missing intended current ability")?;
    let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind else {
        return Err("intended index not activated".into());
    };
    let context = ironsmith::costs::CostCheckContext::new(source, PlayerId(0))
        .with_reason(ironsmith::costs::PaymentReason::ActivateAbility);
    let diagnostic = json!({"source":source.0,"source_name":g.object(source).unwrap().name.to_string(),"ability_index":index,"source_controller":g.current_controller(source).map(|p|p.index()),"source_tapped":g.is_tapped(source),"source_summoning_sick":g.is_summoning_sick(source),"source_has_haste":g.current_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Haste),"phase":format!("{:?}",g.turn.phase),"active_player":g.turn.active_player.index(),"priority_player":g.turn.priority_player.map(|p|p.index()),"mana_pool":format!("{:?}",g.player(PlayerId(0)).unwrap().mana_pool),"stack_len":g.stack.len(),"legal_actions":format!("{:?}",compute_legal_actions(&g,PlayerId(0)).expect("fixture has complete replacement state")),"component_checks":activated.mana_cost.costs().iter().map(|c|json!({"cost":format!("{c:?}").chars().take(200).collect::<String>(),"check":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*c.0,&g,&context)),"references_cost_x":c.effect_ref().map(|e|e.references_cost_x()),"nonmana_x_capacity":c.effect_ref().and_then(|e|e.max_cost_x(&g,source,PlayerId(0)))})).collect::<Vec<_>>()});
    let mut action = Value::Null;
    let mut error = None;
    if capacity > 0 || name != "Ruthless Technomancer" {
        match activate(&mut g, source, index, &mut d) {
            Ok(v) => action = v,
            Err(e) => error = Some(e),
        }
    }
    let number = d
        .trace
        .iter()
        .find(|r| r["choice"] == "number" && r["is_x_value"] == true);
    let min = if name == "Ruthless Technomancer" {
        1
    } else {
        0
    };
    let actual = json!({"action_offered":offered,"error":error,"x_min":number.map(|n|n["min"].clone()),"x_max":number.map(|n|n["max"].clone()),"announced_x":action["announced_x"],"resource_count_before":resources,"objects_unchanged":board(&g)==objects_before,"mana_paid":mana_before-g.player(PlayerId(0)).unwrap().mana_pool.total(),"resolution_error":action["resolution_error"]});
    let expected = if name == "Ruthless Technomancer" && capacity == 0 {
        json!({"action_payable":false,"resource_capacity":0,"dispatch_required":false})
    } else {
        json!({"action_offered":true,"x_min":min,"x_max":capacity,"requested_x_available":requested>=min&&requested<=capacity as u32,"positive_x_scope":"Actual choice bounds before payment; intended positive effect not assumed executed."})
    };
    Ok((
        actual,
        expected,
        json!({"paid_source_and_producers":history,"activation":action,"decisions":d.trace,"requested_x":requested,"expected_resources":expected_resources,"expected_capacity":capacity,"objects_before":objects_before,"objects_after":board(&g),"known_unpayable_not_dispatched":name=="Ruthless Technomancer"&&capacity==0,"legality_diagnostic":diagnostic}),
    ))
}
fn board(g: &GameState) -> Vec<Value> {
    g.objects_in_deterministic_order().iter().map(|o|json!({"id":o.id.0,"name":o.name.to_string(),"zone":format!("{:?}",o.zone),"owner":o.owner.index(),"power":g.calculated_power(o.id),"toughness":g.calculated_toughness(o.id),"counters":format!("{:?}",o.counters)})).collect()
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
#[ignore = "strict paid fixed-mana/nonmana-X bounds"]
fn report_nonmana_x_siblings() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/nonmana-x-sibling-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_nonmana_x_sibling_reproductions.rs"),
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
        for variant in 0..3 {
            println!("stage: {name} variant{variant}");
            let (status, actual, expected, evidence) = match trial(&defs, name, variant) {
                Ok((a, e, t)) => (
                    if t["known_unpayable_not_dispatched"] == true {
                        "known_unpayable_diagnostic"
                    } else if a["error"].is_null()
                        && a["x_min"] == e["x_min"]
                        && a["x_max"] == e["x_max"]
                    {
                        "expected_choice_bounds_observed"
                    } else {
                        "activation_unavailable_or_choice_bound_mismatch"
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
    let report = json!({"scope":"Eight fixed-mana/nonmana-X siblings. Canonical paid sources and resource producers. Inspect normal activation X bounds, never send an out-of-bounds response; retained X0 completion is a secondary observation. Known-unpayable Ruthless negative is not dispatched.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/nonmana-x-sibling-final-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
