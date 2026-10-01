//! Bounded small token-multiplier counterparts of imported capacity stress tests.
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
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, Effect, GameState, ObjectId, PlayerId, PowerToughness, Zone,
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
    accept: bool,
    target_name: Option<String>,
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
        let selected = SelectFirstDecisionMaker.decide_objects(g, c);
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
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        if self.target_name.as_deref() == Some("PLAYER_ALICE") {
            return vec![Target::Player(alice())];
        }
        if let Some(name) = &self.target_name {
            if c.requirements.len() == 1 {
                if let Some(t)=c.requirements[0].legal_targets.iter().find(|t|matches!(t,Target::Object(id)if g.object(*id).is_some_and(|o|o.name==*name))){return vec![*t];}
            }
        }
        SelectFirstDecisionMaker.decide_targets(g, c)
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
                    &PriorityResponse::OptionalCosts(vec![]),
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
        accept,
        target_name: None,
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
fn witness(name: &str, kind: CardType, cost: u8, with_x: bool) -> CardDefinition {
    let mut pips = vec![ManaSymbol::Generic(cost)];
    if with_x {
        pips.push(ManaSymbol::X);
    }
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(ManaCost::from_symbols(pips))
        .power_toughness(PowerToughness::fixed(3, 3))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build()
}

fn token_state(g: &GameState) -> Value {
    let ids: Vec<_> = g
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            g.object(*id)
                .is_some_and(|o| matches!(o.kind, ironsmith::object::ObjectKind::Token))
        })
        .collect();
    let count = |subtype: ironsmith::Subtype, p: PlayerId| {
        ids.iter()
            .filter(|id| {
                g.controller_of_id(**id) == Some(p)
                    && g.current_subtypes(**id)
                        .is_some_and(|s| s.contains(&subtype))
            })
            .count()
    };
    json!({"token_count":ids.len(),"alice_clue":count(ironsmith::Subtype::Clue,alice()),"alice_food":count(ironsmith::Subtype::Food,alice()),"alice_treasure":count(ironsmith::Subtype::Treasure,alice()),"alice_warrior":count(ironsmith::Subtype::Warrior,alice()),"bob_warrior":count(ironsmith::Subtype::Warrior,PlayerId::from_index(1)),"warriors_are_white_1_1_creatures":ids.iter().filter(|id|g.current_subtypes(**id).is_some_and(|s|s.contains(&ironsmith::Subtype::Warrior))).all(|id|g.calculated_power(*id)==Some(1)&&g.calculated_toughness(*id)==Some(1)&&g.current_card_types(*id).is_some_and(|v|v.contains(&CardType::Creature))&&g.current_colors(*id).is_some_and(|c|c.contains(ironsmith::Color::White)))})
}
fn scenario(
    defs: &HashMap<String, CardDefinition>,
    case: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup_players(2);
    let manufactor = case.starts_with("manufactor");
    let n = case.split('-').nth(1).unwrap().parse::<usize>().unwrap();
    let double = case.contains("double");
    let two_player = case.contains("players");
    let x = if case.contains("zero") { 0 } else { 2 };
    let start = std::time::Instant::now();
    let mut setup_paid = 0;
    for _ in 0..n {
        established(
            &mut g,
            &defs[if manufactor {
                "Academy Manufactor"
            } else {
                "Anointed Procession"
            }],
            alice(),
            if manufactor { 3 } else { 4 },
        )?;
        setup_paid += if manufactor { 3 } else { 4 };
    }
    if double {
        established(&mut g, &defs["Anointed Procession"], alice(), 4)?;
        setup_paid += 4;
    }
    let mut dm = dm(x, true);
    println!("stage: producer {case}");
    let (mut q, paid) = cast(
        &mut g,
        &defs[if manufactor {
            "Thraben Inspector"
        } else {
            "Secure the Wastes"
        }],
        alice(),
        &mut dm,
    )?;
    let mut error = resolve(&mut g, &mut q, &mut dm).err();
    let mut bob_paid = 0;
    if two_player && error.is_none() {
        let (mut q, p) = cast(
            &mut g,
            &defs["Secure the Wastes"],
            PlayerId::from_index(1),
            &mut dm,
        )?;
        bob_paid = p;
        error = resolve(&mut g, &mut q, &mut dm).err();
    }
    let tokens = token_state(&g);
    let m = if n == 0 { 0 } else { 3usize.pow(n as u32 - 1) } * if double { 2 } else { 1 };
    let clue = if manufactor {
        if n == 0 { 1 } else { m }
    } else {
        0
    };
    let food = if manufactor { m } else { 0 };
    let warrior = if manufactor {
        0
    } else {
        (x as usize) * 2usize.pow(n as u32)
    };
    let bw = if two_player { x as usize } else { 0 };
    let expected_tokens = json!({"token_count":clue+food*2+warrior+bw,"alice_clue":clue,"alice_food":food,"alice_treasure":food,"alice_warrior":warrior,"bob_warrior":bw,"warriors_are_white_1_1_creatures":true});
    let actual = json!({"resolution_error":error,"setup_paid":setup_paid,"producer_paid":paid,"bob_producer_paid":bob_paid,"tokens":tokens,"stack":g.stack.len()});
    let expected = json!({"resolution_error":null,"setup_paid":setup_paid,"producer_paid":if manufactor{1}else{x+1},"bob_producer_paid":if two_player{x+1}else{0},"tokens":expected_tokens,"stack":0});
    Ok((
        actual,
        expected,
        json!({"action_seconds":start.elapsed().as_secs_f64(),"choices":dm.trace,"tokens":g.battlefield.iter().filter_map(|id|g.object(*id)).filter(|o|matches!(o.kind,ironsmith::object::ObjectKind::Token)).map(|o|json!({"name":o.name.to_string(),"controller":g.controller_of_id(o.id).map(|p|p.index()),"types":g.current_card_types(o.id).map(|t|format!("{t:?}")),"subtypes":g.current_subtypes(o.id).map(|t|format!("{t:?}"))})).collect::<Vec<_>>()}),
    ))
}
#[test]
#[ignore = "small bounded production replacement/capacity outcomes, not MAGE arbitrary caps"]
fn report_token_capacity_reductions() {
    let case = std::env::var("CAPACITY_CASE").unwrap_or("manufactor-1".into());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_token_capacity_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names: Vec<_> = [
        "Academy Manufactor",
        "Anointed Procession",
        "Thraben Inspector",
        "Secure the Wastes",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut defs = HashMap::new();
    let mut compilation = vec![];
    for p in payloads.into_values().flatten() {
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p.parse_name.as_deref().unwrap_or(&p.name),
        );
        match ironsmith_registry::compile_builder_to_artifact(b, &p.parse_input, false) {
            Ok((a, d)) => {
                compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
                defs.insert(d.name().to_owned(), d);
            }
            Err(e) => compilation.push(json!({"card":p.name,"strict_compile_error":e.to_string()})),
        }
    }
    let result = scenario(&defs, &case);
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
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Bounded independently expected token counts/types from actual paid canonical producers and replacement sources. No injected token events, engine shims, or imported arbitrary500-token cap. Does not claim full imported large-scale capacity coverage.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":[{"card":if case.starts_with("manufactor"){"Academy Manufactor"}else{"Anointed Procession"},"scenario":case,"status":status,"expected":expected,"actual":actual,"diagnostics":diag}]});
    std::fs::write(
        root.join(format!("reports/runtime-audit/token-capacity-{case}.json")),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
