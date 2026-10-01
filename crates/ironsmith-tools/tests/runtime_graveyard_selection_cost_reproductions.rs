//! Adjacent variable/fixed graveyard-selection costs through actual paid producers.
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
        let yes = true;
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

const NAMES: [&str; 3] = ["Corpseweft", "Painbringer", "Battlefield Scrounger"];
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
fn next_main(g: &mut GameState, d: &mut Choices, history: &mut Vec<Value>) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    let mut q = TriggerQueue::new();
    if let Some(e) = ironsmith::triggers::generate_step_trigger_events(g) {
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t)
        }
    }
    finish(g, &mut q, d)?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    for e in ironsmith::turn::execute_draw_step_with(g, d) {
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t)
        }
    }
    finish(g, &mut q, d)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(g.turn.active_player);
    for c in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(g.turn.active_player)
            .unwrap()
            .mana_pool
            .add(c, 12);
    }
    history.push(json!({"producer":"actual next turn, untap, upkeep, draw, first main","active_player":g.turn.active_player.index(),"turn":g.turn.turn_number}));
    Ok(())
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    variant: usize,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(2, 0);
    for p in 0..2 {
        for _ in 0..20 {
            g.create_object_from_definition(&defs["Plains"], PlayerId(p), Zone::Library);
        }
    }
    let resources = if name == "Battlefield Scrounger" {
        6 + variant
    } else {
        variant.min(2)
    };
    let fresh = name == "Painbringer" && variant == 3;
    let tapped = name == "Painbringer" && variant == 4;
    let mut d = Choices {
        amount: resources as u32,
        actual_x: None,
        targets: vec![],
        reveal_ids: vec![],
        revealed: Default::default(),
        trace: vec![],
    };
    let mut history = vec![];
    paid(
        &mut g,
        defs,
        name,
        0,
        match name {
            "Corpseweft" => 3,
            "Painbringer" => 4,
            _ => 5,
        },
        &mut d,
        &mut history,
    )?;
    let source = find(&g, name, Zone::Battlefield)?;
    if name == "Painbringer" && !fresh {
        next_main(&mut g, &mut d, &mut history)?;
        next_main(&mut g, &mut d, &mut history)?;
        if g.is_summoning_sick(source) {
            return Err("source remains summoning sick after own next turn".into());
        }
    }
    let mut target = None;
    if name == "Painbringer" {
        paid(&mut g, defs, "Hill Giant", 0, 4, &mut d, &mut history)?;
        target = Some(find(&g, "Hill Giant", Zone::Battlefield)?);
    }
    for _ in 0..resources {
        if name == "Corpseweft" {
            d.targets.clear();
            paid(&mut g, defs, "Grizzly Bears", 0, 2, &mut d, &mut history)?;
            let bear = find(&g, "Grizzly Bears", Zone::Battlefield)?;
            d.targets = vec![Target::Object(bear)];
            paid(&mut g, defs, "Murder", 0, 3, &mut d, &mut history)?;
        } else {
            d.targets = vec![Target::Player(PlayerId(1))];
            paid(&mut g, defs, "Shock", 0, 1, &mut d, &mut history)?;
        }
    }
    if tapped {
        d.targets = vec![Target::Object(source)];
        paid(&mut g, defs, "Twiddle", 0, 1, &mut d, &mut history)?;
        if !g.is_tapped(source) {
            return Err("Twiddle did not tap source".into());
        }
    }
    let expected_gy = if name == "Corpseweft" {
        resources * 2
    } else {
        resources + usize::from(tapped)
    };
    let gy = g.player(PlayerId(0)).unwrap().graveyard.len();
    if gy != expected_gy {
        return Err(format!("graveyard count {gy}!={expected_gy}"));
    }
    if g.player(PlayerId(1)).unwrap().life <= 0 {
        return Err("graveyard producer killed Bob".into());
    }
    let intended_legal = match name {
        "Corpseweft" => resources > 0,
        "Painbringer" => !fresh && !tapped,
        _ => gy >= 7,
    };
    d.targets = target.map(Target::Object).into_iter().collect();
    d.trace.clear();
    d.reveal_ids = g
        .player(PlayerId(0))
        .unwrap()
        .graveyard
        .iter()
        .copied()
        .filter(|id| name != "Corpseweft" || g.object(*id).unwrap().name == "Grizzly Bears")
        .take(if name == "Battlefield Scrounger" {
            3
        } else {
            resources
        })
        .collect();
    let index = 0;
    g.turn.priority_player = Some(PlayerId(0));
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let offered=actions.iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index}if *s==source&&*ability_index==index));
    let ability = g
        .current_ability(source, index)
        .ok_or("ability index missing")?;
    let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind else {
        return Err("index not activated".into());
    };
    let ctx = ironsmith::costs::CostCheckContext::new(source, PlayerId(0))
        .with_reason(ironsmith::costs::PaymentReason::ActivateAbility);
    let diagnostic = json!({"source":source.0,"source_name":name,"ability_index":index,"source_zone":format!("{:?}",g.object(source).unwrap().zone),"controller":g.current_controller(source).map(|p|p.index()),"source_tapped":g.is_tapped(source),"source_summoning_sick":g.is_summoning_sick(source),"graveyard_count":gy,"bob_life":g.player(PlayerId(1)).unwrap().life,"stack_len":g.stack.len(),"active_player":g.turn.active_player.index(),"priority_player":g.turn.priority_player.map(|p|p.index()),"phase":format!("{:?}",g.turn.phase),"mana_pool":format!("{:?}",g.player(PlayerId(0)).unwrap().mana_pool),"legal_actions":format!("{actions:?}"),"component_checks":activated.mana_cost.costs().iter().map(|c|json!({"cost":format!("{c:?}").chars().take(220).collect::<String>(),"check":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*c.0,&g,&ctx))})).collect::<Vec<_>>()});
    let before = board(&g);
    let mana_before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let (action, error) = if offered && intended_legal {
        match activate(&mut g, source, index, &mut d) {
            Ok(a) => (a, None),
            Err(e) => (Value::Null, Some(e)),
        }
    } else {
        (Value::Null, None)
    };
    let actual = json!({"action_offered":offered,"error":error,"resolution_error":action["resolution_error"],"mana_paid":mana_before-g.player(PlayerId(0)).unwrap().mana_pool.total(),"objects_unchanged":board(&g)==before,"graveyard_after":g.player(PlayerId(0)).unwrap().graveyard.len(),"library_after":g.player(PlayerId(0)).unwrap().library.len(),"source_power":g.calculated_power(source),"source_toughness":g.calculated_toughness(source),"source_tapped":g.is_tapped(source),"target_power":target.and_then(|id|g.calculated_power(id)),"target_toughness":target.and_then(|id|g.calculated_toughness(id))});
    let expected = json!({"action_offered":intended_legal,"reason":if fresh{"fresh creature cannot pay tap cost"}else if tapped{"tapped creature cannot pay tap cost"}else if name=="Corpseweft"&&resources==0{"one-or-more creature cost has no resource"}else if name=="Battlefield Scrounger"&&gy<7{"threshold not met"}else{"printed requirements and resources satisfied"}});
    Ok((
        actual,
        expected,
        json!({"paid_source_and_producers":history,"legality_diagnostic":diagnostic,"activation":action,"decisions":d.trace,"known_unpayable_not_dispatched":!intended_legal,"objects_before":before,"objects_after":board(&g),"desired_cost_cards":format!("{:?}",d.reveal_ids),"downstream_required_if_available":match name{"Corpseweft"=>format!("exile {resources} creature cards; tapped {} / {} Zombie Horror",2*resources,2*resources),"Painbringer"=>format!("exile {resources} graveyard cards; Hill Giant -{resources}/-{resources} until EOT"),_=>"bottom exactly3 graveyard cards; source6/6; second activation unavailable this turn".into()}}),
    ))
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
#[ignore = "strict paid variable and fixed graveyard-selection cost gates"]
fn report_graveyard_selection_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/graveyard-selection-cost-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_graveyard_selection_cost_reproductions.rs"),
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
        for variant in 0..if name == "Painbringer" { 5 } else { 3 } {
            println!("stage: {name} variant{variant}");
            let (status, actual, expected, evidence) = match trial(&defs, name, variant) {
                Ok((a, e, t)) => (
                    if a["action_offered"] == e["action_offered"] {
                        if a["action_offered"] == true {
                            "offered_needs_downstream_review"
                        } else {
                            "expected_unavailable_negative"
                        }
                    } else {
                        "legal_action_mismatch"
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
    let report = json!({"scope":"Corpseweft0/1/2 actual killed creatures; Painbringer0/1/2 actual paid Shocks, real next-turn untap and fresh/tapped negatives; Scrounger6/7/8 actual paid Shocks after normal source cast. Availability-first, never force unavailable action; downstream outcomes remain unexecuted if blocked.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/graveyard-selection-cost-final-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
