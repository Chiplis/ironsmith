//! Paid land-permission source / control-Aura probes from pinned full-corpus inputs.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, SelectObjectsContext, SelectOptionsContext,
    TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
fn hash(path: &std::path::Path) -> Value {
    let mut file = std::fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let sha256: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    json!({"path": path.display().to_string(), "sha256": sha256})
}

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn setup_players(players: usize) -> GameState {
    let mut g = GameState::new(
        ["Alice", "Bob", "Cara"][..players]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        20,
    );
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
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
        for p in (0..players).map(|p| PlayerId::from_index(p as u8)) {
            g.player_mut(p).unwrap().mana_pool.add(color, 30);
        }
    }
    g
}

struct ProbeDm {
    x: u32,
    kicks: u32,
    targets: Vec<Target>,
    accept: bool,
    recipient: usize,
    trace: Vec<Value>,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"boolean":c.description,"player":c.player.index(),"answer":self.accept}));
        self.accept
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        if c.is_x_value { self.x } else { c.min }
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if let Some(candidate) = c
            .candidates
            .iter()
            .find(|o| o.legal && o.name == "Cost witness")
        {
            vec![candidate.id]
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(json!({"objects":c.description,"player":c.player.index(),"offered":c.candidates.iter().map(|v|json!({"id":v.id.0,"legal":v.legal,"controller":g.controller_of_id(v.id).map(|p|p.index())})).collect::<Vec<_>>(),"selected":selected.iter().map(|v|v.0).collect::<Vec<_>>()}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let recipient = ["Alice", "Bob", "Cara"][self.recipient];
        let selected = c
            .options
            .iter()
            .find(|o| o.legal && o.description == recipient)
            .map(|o| vec![o.index])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(g, c));
        self.trace.push(json!({"options":c.description,"player":c.player.index(),"offered":c.options.iter().map(|o|o.description.clone()).collect::<Vec<_>>(),"selected":selected}));
        selected
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
}
fn resolve(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut ProbeDm) -> Result<(), String> {
    for _ in 0..32 {
        ironsmith::game_loop::check_and_apply_sbas_with(game, queue, dm)
            .map_err(|e| e.to_string())?;
        drain_pending_trigger_events(game, queue);
        put_triggers_on_stack_with_dm(game, queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        let mut priority = PriorityLoopState::new(game.players_in_game());
        priority.reset_for_new_priority_window(game);
        for _ in 0..game.players_in_game() {
            apply_priority_response_with_dm(
                game,
                queue,
                &mut priority,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Err("fixture priority resolution bound exceeded".into())
}
fn perform(
    game: &mut GameState,
    source: ObjectId,
    ability: Option<usize>,
    dm: &mut ProbeDm,
) -> Result<(TriggerQueue, u32), String> {
    let actor = game
        .controller_of_id(source)
        .ok_or("missing action source")?;
    let initial_stack_len = game.stack.len();
    game.turn.priority_player = Some(actor);
    let action = compute_legal_actions(game, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| match a {
            LegalAction::CastSpell { spell_id, .. } => ability.is_none() && *spell_id == source,
            LegalAction::ActivateAbility {
                source: id,
                ability_index,
            } => *id == source && Some(*ability_index) == ability,
            _ => false,
        })
        .ok_or("fixture intended legal action missing")?;
    let mana = game.player(actor).unwrap().mana_pool.total();
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if game.stack.len() > initial_stack_len {
            return Ok((queue, mana - game.player(actor).unwrap().mana_pool.total()));
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref c))
                if c.description.starts_with("Choose optional costs") =>
            {
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(if dm.kicks > 0 {
                        vec![(0, dm.kicks)]
                    } else {
                        vec![]
                    }),
                    dm,
                )
                .map_err(|e| e.to_string())?
            }
            GameProgress::NeedsDecisionCtx(ref c) if !matches!(c, DecisionContext::Priority(_)) => {
                apply_decision_context_with_dm(game, &mut queue, &mut state, c, dm)
                    .map_err(|e| e.to_string())?
            }
            other => return Err(format!("fixture action failed to reach stack: {other:?}")),
        };
    }
    Err("fixture action decision bound exceeded".into())
}

fn dm(x: u32, accept: bool) -> ProbeDm {
    ProbeDm {
        x,
        kicks: 0,
        targets: vec![],
        accept,
        recipient: 1,
        trace: vec![],
    }
}
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.zone == zone && o.name.contains(name))
        .count()
}
fn find(g: &GameState, name: &str, zone: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.zone == zone && o.name == name)
        .map(|o| o.id)
        .ok_or(format!("fixture {name} missing in {zone:?}"))
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    player: PlayerId,
    dm: &mut ProbeDm,
) -> Result<(TriggerQueue, u32), String> {
    let id = g.create_object_from_definition(def, player, Zone::Hand);
    perform(g, id, None, dm)
}
fn established(
    g: &mut GameState,
    def: &CardDefinition,
    player: PlayerId,
    expected: u32,
) -> Result<ObjectId, String> {
    let mut dm = dm(0, false);
    let (mut q, paid) = cast(g, def, player, &mut dm)?;
    if paid != expected {
        return Err(format!(
            "setup {} paid{paid} expected{expected}",
            def.name()
        ));
    }
    resolve(g, &mut q, &mut dm)?;
    find(g, def.name(), Zone::Battlefield)
}
fn card(name: &str, kind: CardType) -> CardDefinition {
    let b = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![kind]);
    if kind == CardType::Creature {
        b.power_toughness(PowerToughness::fixed(2, 6)).build()
    } else {
        b.build()
    }
}

fn play_land(
    g: &mut GameState,
    name: &str,
    id: ObjectId,
    dm: &mut ProbeDm,
) -> Result<(bool, Value), String> {
    let actor = PlayerId(1);
    if g.object(id).map(|o| o.zone) != Some(Zone::Hand) {
        return Err(format!("land checkpoint {name} no longer in hand"));
    }
    g.turn.priority_player = Some(actor);
    let actions = compute_legal_actions(g, actor).expect("fixture has complete replacement state");
    let action = actions
        .into_iter()
        .find(|a| matches!(a, LegalAction::PlayLand {land_id} if *land_id==id));
    let offered = action.is_some();
    let mut progress = Value::Null;
    if let Some(action) = action {
        let mut q = TriggerQueue::new();
        let mut state = PriorityLoopState::new(g.players_in_game());
        let p = apply_priority_response_with_dm(
            g,
            &mut q,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            dm,
        )
        .map_err(|e| e.to_string())?;
        progress = json!(format!("{p:?}"));
        resolve(g, &mut q, dm)?;
    }
    let in_battlefield = count(g, name, Zone::Battlefield);
    if offered && in_battlefield != 1 {
        return Err(format!("offered land {name} did not reach battlefield"));
    }
    Ok((
        offered,
        json!({"name":name,"id_before":id.0,"offered":offered,"battlefield":in_battlefield,"progress":progress,"lands_played":g.player(actor).unwrap().lands_played_this_turn}),
    ))
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    payload: &Value,
    mode: &str,
) -> Result<(Value, Value, Value), String> {
    let name = payload["name"].as_str().unwrap();
    let cost = payload["expected_cast_mana_at_x0"].as_u64().unwrap() as u32;
    let extra = payload["additional_land_plays"].as_u64().unwrap() as usize;
    let mut g = setup_players(2);
    // Libraries support the source's real optional/mandatory draw and mill effects.
    let library = card("Library witness", CardType::Artifact);
    for player in [alice(), PlayerId(1)] {
        for _ in 0..20 {
            g.create_object_from_definition(&library, player, Zone::Library);
        }
    }
    let owner = if mode == "owned" {
        PlayerId(1)
    } else {
        alice()
    };
    g.turn.active_player = owner;
    g.turn.priority_player = Some(owner);
    let global_permission = mode.ends_with("-global");
    if global_permission {
        established(&mut g, &defs["Exploration"], owner, 1)?;
    }
    let source = established(&mut g, &defs[name], owner, cost)?;
    g.turn.active_player = PlayerId(1);
    g.turn.priority_player = Some(PlayerId(1));
    let mut d = dm(0, false);
    // All resources are installed before the Aura action. None are added after
    // its resolution and before controller-sensitive decisions.
    if name == "Flubs, the Fool" {
        let collateral = card("Discard collateral", CardType::Artifact);
        for _ in 0..8 {
            g.create_object_from_definition(&collateral, PlayerId(1), Zone::Hand);
        }
    }
    let mut hand_lands = vec![];
    for index in 0..extra + 2 {
        let name = format!("Land checkpoint {index}");
        let id =
            g.create_object_from_definition(&card(&name, CardType::Land), PlayerId(1), Zone::Hand);
        hand_lands.push((name, id));
    }
    for player in [alice(), PlayerId(1)] {
        g.create_object_from_definition(
            &card("Return-cost land", CardType::Land),
            player,
            Zone::Battlefield,
        );
    }
    let mut q = TriggerQueue::new();
    resolve(&mut g, &mut q, &mut d)?;
    let mut aura = Value::Null;
    if mode != "owned" {
        d.targets = vec![Target::Object(source)];
        let aura_name = if mode.starts_with("confiscate") {
            "Confiscate"
        } else if mode == "act-of-treason" {
            "Act of Treason"
        } else {
            "Control Magic"
        };
        let (mut q, paid) = cast(&mut g, &defs[aura_name], PlayerId(1), &mut d)?;
        let expected = if mode.starts_with("confiscate") {
            6
        } else if mode == "act-of-treason" {
            3
        } else {
            4
        };
        if paid != expected {
            return Err(format!("aura paid {paid} expected {expected}"));
        }
        resolve(&mut g, &mut q, &mut d)?;
        if mode == "act-of-treason" {
            aura = json!({"card":aura_name,"mana_paid":paid,"graveyard_count":count(&g,aura_name,Zone::Graveyard)});
        } else {
            let aura_id = find(&g, aura_name, Zone::Battlefield)?;
            let attachment = format!("{:?}", g.object(aura_id).unwrap().attached_to);
            if g.object(aura_id).unwrap().attached_to
                != Some(ironsmith::object::AttachmentTarget::Object(source))
            {
                return Err(format!("Aura attached incorrectly: {attachment}"));
            }
            aura = json!({"card":aura_name,"id":aura_id.0,"mana_paid":paid,"attached_to":attachment,"controller":g.controller_of_id(aura_id).map(|p|p.index())});
        }
    }
    let current = g.controller_of_id(source).map(|p| p.index());
    let calculated = g
        .calculated_characteristics(source)
        .map(|c| c.controller.index());
    let before = json!({"current_controller":current,"calculated_controller":calculated,"source_owner":g.object(source).unwrap().owner.index(),"continuous_effects":format!("{:?}",g.all_continuous_effects())});
    let mut control_actions = Value::Null;
    if name == "Mina and Denn, Wildborn" {
        let flags = [alice(), PlayerId(1)]
            .iter()
            .map(|p| {
                compute_legal_actions(&g, *p).expect("fixture has complete replacement state")
                    .iter()
                    .any(|a| matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))
            })
            .collect::<Vec<_>>();
        control_actions = json!({"source_activation_legal_for_alice":flags[0],"source_activation_legal_for_bob":flags[1]});
    }
    let bob_library_before = g.player(PlayerId(1)).unwrap().library.len();
    if name == "Aesi, Tyrant of Gyre Strait" {
        d.accept = true;
    }
    let mut offered = vec![];
    let mut lands = vec![];
    d.targets = vec![Target::Object(source)];
    for (land_name, land_id) in hand_lands {
        let (legal, trace) = play_land(&mut g, &land_name, land_id, &mut d)?;
        offered.push(legal);
        lands.push(trace);
    }
    let actual = json!({"controller_before_land_plays":current,"calculated_controller_before_land_plays":calculated,"lands_offered":offered,"lands_played":g.player(PlayerId(1)).unwrap().lands_played_this_turn,"control_sensitive_actions":control_actions,"aesi_bob_cards_drawn":if name=="Aesi, Tyrant of Gyre Strait"{json!(bob_library_before-g.player(PlayerId(1)).unwrap().library.len())}else{Value::Null}});
    let expected = json!({"controller_before_land_plays":1,"calculated_controller_before_land_plays":1,"lands_offered":(0..extra+2).map(|i|i<extra+1).collect::<Vec<_>>(),"lands_played":extra+1,"control_sensitive_actions":if name=="Mina and Denn, Wildborn"{json!({"source_activation_legal_for_alice":false,"source_activation_legal_for_bob":true})}else{Value::Null},"aesi_bob_cards_drawn":if name=="Aesi, Tyrant of Gyre Strait"{json!(extra+1)}else{Value::Null}});
    Ok((
        actual,
        expected,
        json!({"source":source.0,"source_mana_paid":cost,"global_permission":global_permission,"aura":aura,"before":before,"land_checkpoints":lands,"decisions":d.trace}),
    ))
}
fn strip_card_id(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(card) = v.get_mut("card").and_then(Value::as_object_mut) {
        card.remove("id");
    }
    v
}
#[test]
#[ignore = "strict paid controller and extra-land-permission outcomes"]
fn report_land_play_control() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/land-play-control-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_land_play_control_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut compilation = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let name = p["name"].as_str().unwrap();
        if name.contains(" // ") {
            continue;
        }
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p["parse_name"].as_str().unwrap_or(name),
        );
        match ironsmith_registry::compile_builder_to_artifact(
            b,
            p["parse_input"].as_str().unwrap(),
            false,
        ) {
            Ok((a, d)) => {
                let definition = serde_json::to_value(&a.payload.definition).unwrap();
                compilation.push(json!({"card":name,"artifact_checksum":a.payload_checksum,"frozen_artifact_checksum":p["frozen_artifact_checksum"],"definition_matches_frozen_except_unique_card_id":strip_card_id(&definition)==strip_card_id(&p["frozen_definition"]),"definition":definition}));
                defs.insert(name.to_string(), d);
            }
            Err(e) => compilation.push(json!({"card":name,"strict_compile_error":e.to_string()})),
        }
    }
    let mut rows = vec![];
    let mut excluded = vec![];
    let filter = std::env::var("LAND_CONTROL_CARDS").ok();
    for p in payloads["cards"].as_array().unwrap() {
        let name = p["name"].as_str().unwrap();
        if ["Control Magic", "Confiscate", "Act of Treason"].contains(&name) {
            continue;
        }
        let why = if name.contains(" // ") {
            Some("combined alias")
        } else if p["conditional"].as_bool().unwrap() {
            Some("conditional source needs independent prerequisite fixture")
        } else if filter
            .as_ref()
            .is_some_and(|f| !f.split('|').any(|v| v == name))
        {
            Some("selected pilot subset")
        } else {
            None
        };
        if let Some(why) = why {
            excluded.push(json!({"card":name,"reason":why}));
            continue;
        }
        let creature = p["frozen_definition"]["card"]["card_types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t == "Creature");
        let mut modes = if creature {
            vec!["owned", "control-magic", "confiscate", "act-of-treason"]
        } else {
            vec!["owned", "confiscate"]
        };
        if name == "Typhoid Rats" {
            modes.extend(["control-magic-global", "confiscate-global"]);
        }
        for mode in modes {
            println!("stage: {name} / {mode}");
            let result = trial(&defs, p, mode);
            let (status, actual, expected, diag) = match result {
                Ok((v, e, t)) => {
                    if v == e {
                        ("passed", v, e, t)
                    } else {
                        ("semantic_mismatch", v, e, t)
                    }
                }
                Err(e) => (
                    "execution_or_fixture_error",
                    json!({"error":e}),
                    Value::Null,
                    Value::Null,
                ),
            };
            rows.push(json!({"card":name,"scenario":mode,"status":status,"actual":actual,"expected":expected,"diagnostics":diag}));
        }
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Canonical source paid from hand. Either Bob owns the source, or Bob normally casts a targeting control Aura on Alice's source. Actual Bob hand-land plays verify permission follows controller. Only unique CardId is excluded in strict frozen-definition parity. No engine edits.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows,"excluded":excluded});
    let output = std::env::var("LAND_CONTROL_OUTPUT")
        .unwrap_or("reports/runtime-audit/land-play-control-execution.json".into());
    std::fs::write(
        root.join(output),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
