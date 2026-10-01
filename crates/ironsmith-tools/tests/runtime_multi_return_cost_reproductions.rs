//! Canonical counter-removal costs with actual producers and explicit full payment choices.
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
fn land(g: &mut GameState, def: &CardDefinition, dm: &mut Choices) -> Result<Value, String> {
    let id = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let action = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id,..}if *land_id==id))
        .ok_or("land play not advertised")?;
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    apply_priority_response_with_dm(
        g,
        &mut q,
        &mut st,
        &PriorityResponse::PriorityAction(action.clone()),
        dm,
    )
    .map_err(|e| e.to_string())?;
    finish(g, &mut q, dm)?;
    let id = find(g, def.name(), Zone::Battlefield)?;
    Ok(
        json!({"action":format!("{action:?}"),"mana_paid":before-g.player(PlayerId(0)).unwrap().mana_pool.total(),"enters_tapped":g.is_tapped(id)}),
    )
}

fn cycle(g: &mut GameState) {
    g.next_turn();
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(PlayerId(0));
    for c in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(c, 30);
    }
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

const NAMES: [&str; 6] = [
    "Flooded Shoreline",
    "Multani, Yavimaya's Avatar",
    "Pearl Lake Ancient",
    "Soratami Mirror-Mage",
    "Soratami Seer",
    "Uyo, Silent Prophet",
];
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    extra: i32,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(2, 0);
    let mut dm = Choices {
        targets: vec![],
        names: vec![],
        x: 0,
        allow_optional: true,
        choose_untap: true,
        trace: vec![],
    };
    let mut producers = vec![];
    for _ in 0..6 {
        g.create_object_from_definition(&defs["Plains"], PlayerId(0), Zone::Library);
        g.create_object_from_definition(&defs["Plains"], PlayerId(1), Zone::Library);
    }
    let required = if ["Pearl Lake Ancient", "Soratami Mirror-Mage"].contains(&name) {
        3
    } else {
        2
    };
    let resources = (required + extra) as usize;
    for i in 0..resources {
        if i > 0 {
            cycle(&mut g);
        }
        producers.push(land(&mut g, &defs["Island"], &mut dm)?);
    }
    let is_bounce = ["Flooded Shoreline", "Soratami Mirror-Mage"].contains(&name);
    let multani = name.starts_with("Multani");
    let seer = name == "Soratami Seer";
    let uyo = name.starts_with("Uyo");
    let pearl = name.starts_with("Pearl");
    let mut victim = None;
    if is_bounce {
        producers.push(paid(&mut g, defs, "Hill Giant", &mut dm, 4)?);
        victim = Some(find(&g, "Hill Giant", Zone::Battlefield)?);
    }
    let source_cast = paid(
        &mut g,
        defs,
        name,
        &mut dm,
        match name {
            "Flooded Shoreline" => 2,
            "Multani, Yavimaya's Avatar" => 6,
            "Pearl Lake Ancient" => 7,
            "Soratami Mirror-Mage" => 4,
            "Soratami Seer" => 5,
            "Uyo, Silent Prophet" => 6,
            _ => unreachable!(),
        },
    )?;
    let mut source = find(&g, name, Zone::Battlefield)?;
    if multani {
        dm.targets = vec![Target::Object(source)];
        producers.push(paid(&mut g, defs, "Murder", &mut dm, 3)?);
        source = find(&g, name, Zone::Graveyard)?;
    }
    let mut original_queue = None;
    if uyo {
        dm.targets = vec![Target::Player(PlayerId(1))];
        let (q, e) = announce(&mut g, &defs["Shock"], 0, &mut dm)?;
        if e["mana_paid"] != 1 {
            return Err("actual original Shock payment mismatch".into());
        }
        producers.push(e);
        original_queue = Some(q);
        dm.targets = vec![Target::Object(g.stack.last().unwrap().object_id)];
        dm.allow_optional = false;
    } else {
        dm.targets = victim
            .map(|id| vec![Target::Object(id)])
            .unwrap_or_default();
    }
    dm.names = vec!["Island".into()];
    let actions = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state");
    let index = actions
        .iter()
        .filter_map(|a| match a {
            LegalAction::ActivateAbility {
                source: s,
                ability_index,
            } if *s == source => Some(*ability_index),
            _ => None,
        })
        .max();
    let costs = defs[name]
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            ironsmith::ability::AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .last()
        .ok_or("no printed ability")?;
    let before = json!({"land_battlefield":count(&g,"Island",Zone::Battlefield),"land_hand":count(&g,"Island",Zone::Hand),"source_zone":g.object(source).map(|o|format!("{:?}",o.zone)),"source_power":g.calculated_power(source),"source_toughness":g.calculated_toughness(source),"available_mana":g.player(PlayerId(0)).unwrap().mana_pool.total(),"advertised_actions":format!("{actions:?}"),"component_checks":costs.mana_cost.costs().iter().map(|c|json!({"cost":format!("{c:?}").chars().take(160).collect::<String>(),"check":format!("{:?}",ironsmith::costs::can_pay_with_check_context(&*c.0,&g,&ironsmith::costs::CostCheckContext::new(source,PlayerId(0)).with_reason(ironsmith::costs::PaymentReason::ActivateAbility)))})).collect::<Vec<_>>()});
    if extra < 0 {
        return Ok((
            json!({"activation_available":false}),
            json!({"activation_available":index.is_some()}),
            json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace}),
        ));
    }
    if index.is_none() {
        return Ok((
            json!({"activation_available":true}),
            json!({"activation_available":false}),
            json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation_dispatched":false,"choice_trace":dm.trace,"unreached_scope":"No X or cost selection, return cost or downstream bounce/draw/copy effect was forced. Original pending spell remains on stack in Uyo cases."}),
        ));
    }
    let mana_before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation = match index {
        Some(i) => action(&mut g, source, i, false, &mut dm)
            .unwrap_or_else(|e| json!({"resolution_error":e})),
        None => json!({"resolution_error":"intended activation unavailable"}),
    };
    if let Some(mut q) = original_queue {
        finish(&mut g, &mut q, &mut dm)?;
    }
    let actual = json!({"error":activation["resolution_error"],"mana_paid":mana_before-g.player(PlayerId(0)).unwrap().mana_pool.total(),"island_battlefield":count(&g,"Island",Zone::Battlefield),"island_hand":count(&g,"Island",Zone::Hand),"island_graveyard":count(&g,"Island",Zone::Graveyard),"plains_hand":count(&g,"Plains",Zone::Hand),"source_battlefield":count(&g,name,Zone::Battlefield),"source_hand":count(&g,name,Zone::Hand),"source_graveyard":count(&g,name,Zone::Graveyard),"giant_battlefield":count(&g,"Hill Giant",Zone::Battlefield),"giant_hand":count(&g,"Hill Giant",Zone::Hand),"bob_life":g.player(PlayerId(1)).unwrap().life,"stack_length":g.stack.len()});
    let mana = match name {
        "Flooded Shoreline" => 2,
        "Multani, Yavimaya's Avatar" => 2,
        "Pearl Lake Ancient" => 0,
        "Soratami Mirror-Mage" => 3,
        "Soratami Seer" => 4,
        "Uyo, Silent Prophet" => 2,
        _ => unreachable!(),
    };
    let expected = json!({"error":null,"mana_paid":mana,"island_battlefield":extra,"island_hand":if seer{0}else{required},"island_graveyard":if seer{required}else{0},"plains_hand":if seer{required}else{0},"source_battlefield":if multani||pearl{0}else{1},"source_hand":if multani||pearl{1}else{0},"source_graveyard":0,"giant_battlefield":0,"giant_hand":if is_bounce{1}else{0},"bob_life":if uyo{16}else{20},"stack_length":0});
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"activation":activation,"ability_index":index,"activation_dispatched":index.is_some(),"choice_trace":dm.trace}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "fixed multi-land return cost audit"]
fn report_multi_return_costs() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && !["Island", "Plains", "Hill Giant", "Murder", "Shock"].contains(&n)
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
        for extra in [-1, 0, 1] {
            let (status, expected, actual, evidence) = match run(&defs, name, extra) {
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
            rows.push(json!({"card":name,"scenario":{"extra_lands":extra},"status":status,"expected":expected,"actual":actual,"fixture_evidence":evidence}));
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let r = json!({"scope":"Six fixed multiple-return-land costs: real land plays and paid canonical sources. One insufficient-resource legality control and two legally payable states per card. Missing-action cases stop at availability; later costs and effects remain unexecuted. Actual pending Shock target for Uyo, actual Multani death via Murder. No forced unadvertised actions.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_multi_return_cost_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/multi-return-cost-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
