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
    opponent_name: String,
    keep: usize,
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
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"source_zone":g.object(c.source).map(|o|format!("{:?}",o.zone)),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let s = if let Some(o) = c
            .options
            .iter()
            .find(|o| o.legal && o.description.to_lowercase().contains("untap"))
        {
            vec![o.index]
        } else {
            ironsmith::decision::SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":s}));
        s
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut selected = vec![];
        let preferred = if c.player == PlayerId(1) {
            vec![self.opponent_name.clone()]
        } else {
            self.names.clone()
        };
        for name in preferred {
            for item in &c.candidates {
                if item.legal
                    && g.object(item.id).is_some_and(|o| o.name == name)
                    && !selected.contains(&item.id)
                {
                    selected.push(item.id);
                }
            }
        }
        let limit = if c.min == 0
            && c.max == Some(2)
            && c.candidates
                .iter()
                .any(|o| g.object(o.id).is_some_and(|o| o.zone == Zone::Battlefield))
        {
            self.keep
        } else {
            c.max.unwrap_or(selected.len())
        };
        selected.truncate(limit);
        if selected.len() < c.min {
            for item in &c.candidates {
                if item.legal && !selected.contains(&item.id) {
                    selected.push(item.id);
                    if selected.len() >= c.min {
                        break;
                    }
                }
            }
        }
        if selected.is_empty() && c.min > 0 {
            selected = ironsmith::decision::SelectFirstDecisionMaker.decide_objects(g, c);
        }
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":selected.iter().map(|id|json!({"id":id.0,"name":g.object(*id).map(|o|o.name.to_string())})).collect::<Vec<_>>()}));
        selected
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

const NAMES: [&str; 6] = [
    "Coin of Fate",
    "Great Hall of Starnheim",
    "Grove of the Guardian",
    "Kyscu Drake",
    "Magus of the Order",
    "Mount Doom",
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
fn summary(g: &GameState, name: &str) -> Value {
    use ironsmith::static_abilities::StaticAbilityId;
    let mut permanents=g.objects_in_deterministic_order().iter().filter(|o|o.zone==Zone::Battlefield&&o.card_types.contains(&CardType::Creature)).map(|o|{
  let c=g.calculated_characteristics(o.id).unwrap();
  json!({"name":if o.kind==ironsmith::object::ObjectKind::Token{"TOKEN".to_string()}else{o.name.to_string()},"power":c.power,"toughness":c.toughness,"tapped":g.is_tapped(o.id),"controller":c.controller.index(),"colors":c.colors,"types":c.card_types.iter().map(|t|format!("{t:?}")).collect::<Vec<_>>(),"subtypes":c.subtypes.iter().map(|x|format!("{x:?}")).collect::<Vec<_>>(),"flying":g.object_has_static_ability_id(o.id,StaticAbilityId::Flying),"vigilance":g.object_has_static_ability_id(o.id,StaticAbilityId::Vigilance)})
 }).collect::<Vec<_>>();
    permanents.sort_by_key(|v| v["name"].as_str().unwrap().to_owned());
    let mut graveyard = g
        .objects_in_deterministic_order()
        .iter()
        .filter(|o| o.zone == Zone::Graveyard && !["Murder", "Twiddle"].contains(&o.name.as_ref()))
        .map(|o| o.name.to_string())
        .collect::<Vec<_>>();
    graveyard.sort();
    let mut exile = g
        .objects_in_deterministic_order()
        .iter()
        .filter(|o| o.zone == Zone::Exile)
        .map(|o| o.name.to_string())
        .collect::<Vec<_>>();
    exile.sort();
    json!({"source_battlefield":find(g,name,Zone::Battlefield).is_ok(),"source_graveyard":find(g,name,Zone::Graveyard).is_ok(),"permanents":permanents,"graveyard":graveyard,"exile":exile,"monarch":g.monarch.map(|p|p.index()),"library_bottom":g.player(PlayerId(0)).unwrap().library.first().and_then(|id|g.object(*id)).map(|o|o.name.to_string()),"stack_length":g.stack.len()})
}
fn creature(
    name: &str,
    p: i32,
    t: i32,
    tapped: bool,
    subtypes: &[&str],
    flying: bool,
    vigilance: bool,
) -> Value {
    json!({"colors":match name{"Grizzly Bears"=>16,"Hill Giant"=>8,"Viashivan Dragon"=>24,"TOKEN"=>if p==4{1}else{17},_=>0},"types":["Creature"],"name":name,"power":p,"toughness":t,"tapped":tapped,"controller":0,"subtypes":subtypes,"flying":flying,"vigilance":vigilance})
}
fn run(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    variant: usize,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup(2, 0);
    let mut dm = Choices {
        targets: vec![],
        names: vec![],
        opponent_name: if variant == 0 {
            "Grizzly Bears"
        } else {
            "Hill Giant"
        }
        .into(),
        keep: variant,
        trace: vec![],
    };
    for _ in 0..4 {
        g.create_object_from_definition(&defs["Plains"], PlayerId(0), Zone::Library);
        g.create_object_from_definition(&defs["Plains"], PlayerId(1), Zone::Library);
    }
    if ["Kyscu Drake", "Magus of the Order"].contains(&name) {
        g.create_object_from_definition(&defs["Viashivan Dragon"], PlayerId(0), Zone::Library);
    }
    let is_land = [
        "Great Hall of Starnheim",
        "Grove of the Guardian",
        "Mount Doom",
    ]
    .contains(&name);
    let source_cast = if is_land {
        let r = land(&mut g, &defs[name], &mut dm)?;
        if r["mana_paid"] != 0 || r["enters_tapped"] != (name == "Great Hall of Starnheim") {
            return Err(format!("land entry mismatch:{r}"));
        }
        r
    } else {
        paid(
            &mut g,
            defs,
            name,
            &mut dm,
            if name == "Coin of Fate" { 2 } else { 4 },
        )?
    };
    let source = find(&g, name, Zone::Battlefield)?;
    let mut producers = vec![];
    let resources = match name {
        "Coin of Fate" => vec![("Grizzly Bears", 2), ("Hill Giant", 4)],
        "Great Hall of Starnheim" => vec![("Grizzly Bears", 2)],
        "Grove of the Guardian" => vec![("Grizzly Bears", 2), ("Hill Giant", 4)],
        "Kyscu Drake" => vec![("Spitting Drake", 4)],
        "Magus of the Order" => vec![("Grizzly Bears", 2)],
        "Mount Doom" => vec![
            ("Mox Amber", 0),
            ("Grizzly Bears", 2),
            ("Hill Giant", 4),
            ("Ornithopter", 0),
        ],
        _ => unreachable!(),
    };
    for (n, m) in resources {
        dm.targets.clear();
        producers.push(paid(&mut g, defs, n, &mut dm, m)?);
    }
    if name == "Coin of Fate" {
        for n in ["Grizzly Bears", "Hill Giant"] {
            dm.targets = vec![Target::Object(find(&g, n, Zone::Battlefield)?)];
            producers.push(paid(&mut g, defs, "Murder", &mut dm, 3)?);
        }
        dm.targets.clear();
    }
    if name == "Great Hall of Starnheim" {
        dm.targets = vec![Target::Object(source)];
        producers.push(paid(&mut g, defs, "Twiddle", &mut dm, 1)?);
        if g.is_tapped(source) {
            return Err("paid Twiddle did not untap land".into());
        }
        dm.targets.clear();
    }
    if name == "Magus of the Order" {
        g.next_turn();
        g.next_turn();
        ironsmith::turn::execute_untap_step(&mut g);
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
            g.player_mut(PlayerId(0)).unwrap().mana_pool.add(color, 10);
        }
    }
    dm.names = match name {
        "Coin of Fate" => vec!["Grizzly Bears", "Hill Giant"],
        "Great Hall of Starnheim" => vec!["Grizzly Bears"],
        "Grove of the Guardian" => vec!["Grizzly Bears", "Hill Giant"],
        "Kyscu Drake" => vec!["Spitting Drake", "Viashivan Dragon"],
        "Magus of the Order" => vec!["Grizzly Bears", "Viashivan Dragon"],
        "Mount Doom" => vec!["Mox Amber", "Grizzly Bears", "Hill Giant"],
        _ => unreachable!(),
    }
    .iter()
    .map(|x| x.to_string())
    .collect();
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
    let before = summary(&g, name);
    let mana_before = g.player(PlayerId(0)).unwrap().mana_pool.total();
    let activation = match index {
        Some(i) => {
            activate(&mut g, source, i, &mut dm).unwrap_or_else(|e| json!({"resolution_error":e}))
        }
        None => json!({"resolution_error":"intended activation unavailable"}),
    };
    let mut actual = summary(&g, name);
    actual["error"] = activation["resolution_error"].clone();
    actual["mana_paid"] = json!(mana_before - g.player(PlayerId(0)).unwrap().mana_pool.total());
    let mut gy = vec![name.to_string()];
    let mut expected_permanents = vec![];
    let mut monarch = Value::Null;
    let mut bottom = json!("Plains");
    match name {
        "Coin of Fate" => {
            monarch = json!(0);
            bottom = json!(dm.opponent_name);
            expected_permanents.push(if variant == 0 {
                creature("Hill Giant", 3, 3, true, &["Giant"], false, false)
            } else {
                creature("Grizzly Bears", 2, 2, true, &["Bear"], false, false)
            });
        }
        "Great Hall of Starnheim" => {
            gy.push("Grizzly Bears".into());
            expected_permanents.push(creature(
                "TOKEN",
                4,
                4,
                false,
                &["Angel", "Warrior"],
                true,
                true,
            ));
        }
        "Grove of the Guardian" => {
            expected_permanents.extend([
                creature("Grizzly Bears", 2, 2, true, &["Bear"], false, false),
                creature("Hill Giant", 3, 3, true, &["Giant"], false, false),
                creature("TOKEN", 8, 8, false, &["Elemental"], false, true),
            ]);
        }
        "Kyscu Drake" => {
            gy.push("Spitting Drake".into());
            expected_permanents.push(creature(
                "Viashivan Dragon",
                4,
                4,
                false,
                &["Dragon"],
                true,
                false,
            ));
        }
        "Magus of the Order" => {
            gy.push("Grizzly Bears".into());
            expected_permanents.push(creature(
                "Viashivan Dragon",
                4,
                4,
                false,
                &["Dragon"],
                true,
                false,
            ));
        }
        "Mount Doom" => {
            gy.push("Mox Amber".into());
            gy.push("Ornithopter".into());
            if variant > 0 {
                expected_permanents.push(creature(
                    "Grizzly Bears",
                    2,
                    2,
                    false,
                    &["Bear"],
                    false,
                    false,
                ));
            } else {
                gy.push("Grizzly Bears".into());
            }
            if variant > 1 {
                expected_permanents.push(creature(
                    "Hill Giant",
                    3,
                    3,
                    false,
                    &["Giant"],
                    false,
                    false,
                ));
            } else {
                gy.push("Hill Giant".into());
            }
        }
        _ => unreachable!(),
    };
    gy.sort();
    expected_permanents.sort_by_key(|v| v["name"].as_str().unwrap().to_owned());
    let cost = match name {
        "Coin of Fate" => 4,
        "Great Hall of Starnheim" => 3,
        "Grove of the Guardian" => 5,
        "Kyscu Drake" => 0,
        "Magus of the Order" => 1,
        "Mount Doom" => 7,
        _ => unreachable!(),
    };
    let expected = json!({"source_battlefield":false,"source_graveyard":true,"permanents":expected_permanents,"graveyard":gy,"exile":[],"monarch":monarch,"library_bottom":bottom,"stack_length":0,"error":null,"mana_paid":cost});
    Ok((
        expected,
        actual,
        json!({"source_cast":source_cast,"producers":producers,"before_activation":before,"advertised_actions":format!("{actions:?}"),"ability_index":index,"activation":activation,"choice_trace":dm.trace}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "sacrifice and effect cost sibling audit"]
fn report_sacrifice_cost_siblings() {
    let input = std::path::PathBuf::from(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap());
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        if !NAMES.contains(&n)
            && ![
                "Grizzly Bears",
                "Hill Giant",
                "Ornithopter",
                "Mox Amber",
                "Plains",
                "Murder",
                "Twiddle",
                "Spitting Drake",
                "Viashivan Dragon",
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
        let variants = if name == "Coin of Fate" {
            2
        } else if name == "Mount Doom" {
            3
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
    let r = json!({"scope":"Six same-branch self-sacrifice/effect-cost sources. Full canonical paid cards, actual land plays, actual graveyard producers, explicit cost/search/keep choices. Exact immediate outputs and cost resources only; no arbitrary payment-order coverage.","rows":rows,"artifacts":artifacts,"provenance":{"binary":binary,"binary_sha256":hash(&binary),"inventory_sha256":hash(&input),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_sacrifice_cost_sibling_reproductions.rs")),"runtime_stack_bytes":67108864}});
    std::fs::write(
        root.join("reports/runtime-audit/sacrifice-cost-sibling-reproductions.json"),
        serde_json::to_string_pretty(&r).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&r).unwrap());
}
