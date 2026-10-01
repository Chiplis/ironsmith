//! Strict paid-action reproductions for later imported MAGE runtime leads.
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
    resolve_stack_entry_with,
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
        if let Some(name) = &self.target_name {
            if c.requirements.len() == 1 {
                if let Some(t)=c.requirements[0].legal_targets.iter().find(|t|matches!(t,Target::Object(id)if g.object(*id).is_some_and(|o|o.name==*name))){return vec![*t];}
            }
        }
        SelectFirstDecisionMaker.decide_targets(g, c)
    }
}
fn resolve(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut ProbeDm) -> Result<(), String> {
    for _ in 0..24 {
        drain_pending_trigger_events(game, queue);
        put_triggers_on_stack_with_dm(game, queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
    }
    Err("fixture resolution bound exceeded".into())
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

fn phase(g: &mut GameState, combat: bool, dm: &mut ProbeDm) -> Result<Option<String>, String> {
    g.turn.phase = if combat {
        ironsmith::Phase::Combat
    } else {
        ironsmith::Phase::Beginning
    };
    g.turn.step = Some(if combat {
        ironsmith::Step::BeginCombat
    } else {
        ironsmith::Step::Upkeep
    });
    let event = ironsmith::triggers::generate_step_trigger_events(g)
        .ok_or("fixture phase event missing")?;
    let mut q = TriggerQueue::new();
    for trigger in ironsmith::triggers::check_triggers(g, &event) {
        q.add(trigger);
    }
    put_triggers_on_stack_with_dm(g, &mut q, dm).map_err(|e| e.to_string())?;
    Ok(resolve(g, &mut q, dm).err())
}
fn finish(actual: Value, dm: ProbeDm) -> (Value, Value) {
    (actual, json!({"choices":dm.trace}))
}
fn counters(g: &GameState, id: ObjectId, name: &'static str) -> u32 {
    g.object(id)
        .unwrap()
        .counters
        .get(&CounterType::Named(name.into()))
        .copied()
        .unwrap_or(0)
}
fn victory(d: &CardDefinition, players: usize, recipient: usize) -> Result<(Value, Value), String> {
    let mut g = setup_players(players);
    let id = established(
        &mut g,
        d,
        alice(),
        if d.name() == "Sol Ring" { 1 } else { 3 },
    )?;
    for p in 0..players {
        g.player_mut(PlayerId::from_index(p as u8))
            .unwrap()
            .mana_pool = Default::default();
    }
    g.turn.priority_player = Some(alice());
    let action=compute_legal_actions(&g,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateManaAbility{source,..}|LegalAction::ActivateAbility{source,..}if *source==id)).ok_or("fixture no legal Chimes activation")?;
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut dm = dm(0, true);
    dm.recipient = recipient;
    let mut error = None;
    match apply_priority_response_with_dm(
        &mut g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    ) {
        Err(e) => error = Some(e.to_string()),
        Ok(mut progress) => {
            for _ in 0..12 {
                match progress {
                    GameProgress::NeedsDecisionCtx(ref c)
                        if !matches!(c, DecisionContext::Priority(_)) =>
                    {
                        progress = match apply_decision_context_with_dm(
                            &mut g, &mut q, &mut state, c, &mut dm,
                        ) {
                            Ok(p) => p,
                            Err(e) => {
                                error = Some(e.to_string());
                                break;
                            }
                        };
                    }
                    _ => break,
                }
            }
            if error.is_none() {
                error = resolve(&mut g, &mut q, &mut dm).err();
            }
        }
    }
    Ok(finish(
        json!({"resolution_error":error,"source_tapped":g.is_tapped(id),"mana":(0..players).map(|p|g.player(PlayerId::from_index(p as u8)).unwrap().mana_pool.total()).collect::<Vec<_>>()}),
        dm,
    ))
}
fn upkeep_transfer(
    d: &CardDefinition,
    players: usize,
    accept: bool,
    available: bool,
    recipient: usize,
) -> Result<(Value, Value), String> {
    let mut g = setup_players(players);
    let rogue = d.name() == "Rogue Skycaptain";
    let id = established(&mut g, d, alice(), if rogue { 3 } else { 6 })?;
    let mut followers = vec![];
    if !rogue {
        let k = CardDefinitionBuilder::new(CardId::new(), "Kobolds of Kher Keep")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 1))
            .build();
        for p in [alice(), PlayerId::from_index(1)] {
            followers.push(g.create_object_from_definition(&k, p, Zone::Battlefield));
        }
    }
    if !available {
        g.player_mut(alice()).unwrap().mana_pool = Default::default();
    }
    let before = g.player(alice()).unwrap().mana_pool.total();
    let mut dm = dm(0, accept);
    dm.recipient = recipient;
    let error = phase(&mut g, false, &mut dm)?;
    let objects = std::iter::once(id).chain(followers).collect::<Vec<_>>();
    Ok(finish(
        json!({"resolution_error":error,"controllers":objects.iter().map(|i|g.controller_of_id(*i).map(|p|p.index())).collect::<Vec<_>>(),"tapped":objects.iter().map(|i|g.is_tapped(*i)).collect::<Vec<_>>(),"wage_counters":if rogue{counters(&g,id,"wage")}else{0},"mana_paid":before-g.player(alice()).unwrap().mana_pool.total()}),
        dm,
    ))
}
fn lacerator(d: &CardDefinition, lives: &[i32]) -> Result<(Value, Value), String> {
    let mut g = setup_players(lives.len() + 1);
    established(&mut g, d, alice(), 1)?;
    for (i, life) in lives.iter().enumerate() {
        g.player_mut(PlayerId::from_index((i + 1) as u8))
            .unwrap()
            .life = *life;
    }
    let mut dm = dm(0, true);
    let error = phase(&mut g, false, &mut dm)?;
    Ok(finish(
        json!({"resolution_error":error,"alice_life":g.player(alice()).unwrap().life}),
        dm,
    ))
}
fn kraken(
    d: &CardDefinition,
    players: usize,
    accept: bool,
    resources: usize,
    pretapped: bool,
) -> Result<(Value, Value), String> {
    let mut g = setup_players(players);
    let id = established(&mut g, d, alice(), 4)?;
    if pretapped {
        g.tap(id);
    }
    let c = witness("Kraken opponent tap witness", CardType::Creature, 1, false);
    let opponent = PlayerId::from_index((players - 1) as u8);
    let mut objects = vec![];
    for _ in 0..resources {
        objects.push(g.create_object_from_definition(&c, opponent, Zone::Battlefield));
    }
    let mut dm = dm(0, accept);
    dm.recipient = players - 1;
    let error = phase(&mut g, true, &mut dm)?;
    Ok(finish(
        json!({"resolution_error":error,"source_tapped":g.is_tapped(id),"opponent_tapped":objects.iter().filter(|id|g.is_tapped(**id)).count(),"fish":count(&g,"Fish",Zone::Battlefield)}),
        dm,
    ))
}
fn wishclaw(
    d: &CardDefinition,
    players: usize,
    library: bool,
    recipient: usize,
) -> Result<(Value, Value), String> {
    let mut g = setup_players(players);
    let id = established(&mut g, d, alice(), 2)?;
    if counters(&g, id, "wish") != 3 {
        return Err("fixture Talisman did not enter with3 wish counters".into());
    }
    let card = witness("Wishclaw library witness", CardType::Artifact, 1, false);
    if library {
        g.create_object_from_definition(&card, alice(), Zone::Library);
    }
    let mut dm = dm(0, true);
    dm.recipient = recipient;
    let ability = g
        .object(id)
        .unwrap()
        .abilities
        .iter()
        .position(|a| matches!(&a.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .ok_or("fixture no activated ability")?;
    let error = match perform(&mut g, id, Some(ability), &mut dm) {
        Ok((mut q, paid)) => {
            if paid != 1 {
                return Err(format!("fixture Wishclaw payment{paid}"));
            }
            resolve(&mut g, &mut q, &mut dm).err()
        }
        Err(e) if e.contains("fixture") => return Err(e),
        Err(e) => Some(e),
    };
    Ok(finish(
        json!({"resolution_error":error,"controller":g.controller_of_id(id).map(|p|p.index()),"source_tapped":g.is_tapped(id),"wish_counters":counters(&g,id,"wish"),"searched_cards_in_hand":count(&g,card.name(),Zone::Hand)}),
        dm,
    ))
}
fn record(
    rows: &mut Vec<Value>,
    name: &str,
    scenario: String,
    expected: Value,
    result: Result<(Value, Value), String>,
) {
    let (status, actual, diagnostics) = match result {
        Ok((v, t)) if v == expected => ("passed", v, t),
        Ok((v, t)) if !v["resolution_error"].is_null() => ("confirmed_resolution_failure", v, t),
        Ok((v, t)) => ("semantic_mismatch", v, t),
        Err(e) => (
            "execution_or_fixture_error",
            json!({"error":e}),
            Value::Null,
        ),
    };
    rows.push(json!({"card":name,"scenario":scenario,"status":status,"expected":expected,"actual":actual,"diagnostics":diagnostics}));
}
#[test]
#[ignore = "manual strict paid-action expected-result audit"]
fn report_player_choice_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_player_choice_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = [
        "Victory Chimes",
        "Reservoir Kraken",
        "Rogue Skycaptain",
        "Rohgahh of Kher Keep",
        "Vampire Lacerator",
        "Wishclaw Talisman",
        "Sol Ring",
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
        let (a, d) =
            ironsmith_registry::compile_builder_to_artifact(b, &p.parse_input, false).unwrap();
        compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        defs.insert(d.name().to_owned(), d);
    }
    let mut rows = vec![];
    record(
        &mut rows,
        "Sol Ring",
        "same legal mana-action fixture, fixed recipient/output control".into(),
        json!({"resolution_error":null,"source_tapped":true,"mana":[2,0]}),
        victory(&defs["Sol Ring"], 2, 0),
    );
    for (players, recipient) in [(2, 0), (2, 1), (3, 2)] {
        let mut mana = vec![0; players];
        mana[recipient] = 1;
        record(
            &mut rows,
            "Victory Chimes",
            format!("paid source; legal mana activation for player{recipient} of{players}"),
            json!({"resolution_error":null,"source_tapped":true,"mana":mana}),
            victory(&defs["Victory Chimes"], players, recipient),
        );
    }
    for name in ["Rogue Skycaptain", "Rohgahh of Kher Keep"] {
        for (players, accept, available, recipient) in [
            (2, true, true, 1),
            (2, false, true, 1),
            (2, true, false, 1),
            (3, false, true, 2),
        ] {
            let pay = accept && available;
            let rogue = name.starts_with("Rogue");
            let controllers = if rogue {
                vec![if pay { 0 } else { recipient }]
            } else {
                if pay {
                    vec![0, 0, 1]
                } else {
                    vec![recipient; 3]
                }
            };
            let expected = json!({"resolution_error":null,"controllers":controllers,"tapped":vec![!pay&&!rogue;if rogue{1}else{3}],"wage_counters":u32::from(rogue&&pay),"mana_paid":if pay{if rogue{2}else{3}}else{0}});
            record(
                &mut rows,
                name,
                format!(
                    "upkeep players={players} accept={accept} affordable={available} chosen={recipient}"
                ),
                expected,
                upkeep_transfer(&defs[name], players, accept, available, recipient),
            );
        }
    }
    for lives in [
        vec![20],
        vec![10],
        vec![9],
        vec![20, 10],
        vec![10, 20],
        vec![20, 20],
    ] {
        record(
            &mut rows,
            "Vampire Lacerator",
            format!("upkeep opponent lives{lives:?}"),
            json!({"resolution_error":null,"alice_life":if lives.iter().any(|v|*v<=10){20}else{19}}),
            lacerator(&defs["Vampire Lacerator"], &lives),
        );
    }
    for (players, accept, resources, pretapped) in [
        (2, true, 1, false),
        (2, false, 1, false),
        (2, true, 0, false),
        (2, true, 1, true),
        (3, true, 1, false),
    ] {
        let taps = accept && resources > 0 && !pretapped;
        record(
            &mut rows,
            "Reservoir Kraken",
            format!(
                "combat players={players} accept={accept} opponent_resources={resources} source_tapped={pretapped}"
            ),
            json!({"resolution_error":null,"source_tapped":pretapped||taps,"opponent_tapped":usize::from(taps),"fish":usize::from(taps)}),
            kraken(
                &defs["Reservoir Kraken"],
                players,
                accept,
                resources,
                pretapped,
            ),
        );
    }
    for (players, library, recipient) in [(2, true, 1), (2, false, 1), (3, true, 2)] {
        record(
            &mut rows,
            "Wishclaw Talisman",
            format!(
                "paid activation players={players} library_nonempty={library} chosen={recipient}"
            ),
            json!({"resolution_error":null,"controller":recipient,"source_tapped":true,"wish_counters":2,"searched_cards_in_hand":usize::from(library)}),
            wishclaw(&defs["Wishclaw Talisman"], players, library, recipient),
        );
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict canonical source artifacts and real paid source casts/activations. Phase cases use normal generated upkeep/combat events at explicit phase fixtures. Multiplayer chooser callbacks request the stated recipient; no target, chosen-player, or iterated-player context is injected. Zero-resource, payment/decline and threshold controls remain explicit.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/player-choice-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("wrote {} rows", report["rows"].as_array().unwrap().len());
}
