//! Strict paid-action audit of counter-dependent trigger outcomes; JSON contains verdicts.
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

fn activated(g: &GameState, id: ObjectId) -> Result<usize, String> {
    compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find_map(|a| match a {
            LegalAction::ActivateAbility {
                source,
                ability_index,
            } if source == id => Some(ability_index),
            _ => None,
        })
        .ok_or("fixture activation missing".into())
}
fn coat(defs: &HashMap<String, CardDefinition>, case: &str) -> Result<(Value, Value), String> {
    let mut g = setup_players(2);
    let mut ids = vec![];
    let names = if case == "coat-two" {
        vec!["Woodland Changeling", "Woodland Changeling"]
    } else {
        vec![
            "Mistform Ultimus",
            "Lord of Atlantis",
            "Lord of the Unreal",
            "Woodland Changeling",
            "Woodland Changeling",
            "Mutavault",
            "Smuggler's Copter",
        ]
    };
    for name in &names {
        ids.push(g.create_object_from_definition(&defs[*name], alice(), Zone::Battlefield));
    }
    let start = std::time::Instant::now();
    println!("stage: coat cast {case}");
    if case != "coat-absent" {
        established(&mut g, &defs["Coat of Arms"], alice(), 5)?;
    }
    let mut dm = dm(0, true);
    if case != "coat-two" {
        for id in [ids[5], ids[6]] {
            println!("stage: activate {:?} {case}", g.object(id).unwrap().name);
            let ability = activated(&g, id)?;
            let (mut q, _) = perform(&mut g, id, Some(ability), &mut dm)?;
            resolve(&mut g, &mut q, &mut dm)?;
        }
    }
    println!("stage: inspect characteristics {case}");
    let creatures:Vec<_>=ids.iter().zip(names).map(|(id,name)|json!({"name":name,"power":g.calculated_power(*id),"toughness":g.calculated_toughness(*id),"creature":g.current_card_types(*id).is_some_and(|v|v.contains(&CardType::Creature))})).collect();
    Ok((
        json!({"creatures":creatures,"stack":g.stack.len()}),
        json!({"choices":dm.trace,"action_seconds":start.elapsed().as_secs_f64()}),
    ))
}
fn gitrog(defs: &HashMap<String, CardDefinition>, case: &str) -> Result<(Value, Value), String> {
    let mut g = setup_players(2);
    let bob = PlayerId::from_index(1);
    let library = witness("Gitrog library witness", CardType::Artifact, 1, false);
    for p in [alice(), bob] {
        for _ in 0..12 {
            g.create_object_from_definition(&library, p, Zone::Library);
        }
    }
    let source = established(&mut g, &defs["The Gitrog Monster"], alice(), 5)?;
    let mut dm = dm(0, true);
    let borrowed = case.contains("borrowed");
    if borrowed {
        g.turn.active_player = bob;
        dm.target_name = Some("The Gitrog Monster".into());
        let (mut q, paid) = cast(&mut g, &defs["Control Magic"], bob, &mut dm)?;
        if paid != 4 {
            return Err("Control Magic paid wrong amount".into());
        }
        resolve(&mut g, &mut q, &mut dm)?;
        if g.controller_of_id(source) != Some(bob) {
            return Err("Control Magic did not change source control".into());
        }
        g.turn.active_player = alice();
        g.turn.priority_player = Some(alice());
    }
    let land_count = if case.ends_with('0') {
        0
    } else if case.ends_with('2') {
        2
    } else {
        1
    };
    let land_owner = if case.contains("bobland") {
        bob
    } else {
        alice()
    };
    let discard = case.contains("discard");
    for _ in 0..land_count {
        g.create_object_from_definition(
            &defs["Dryad Arbor"],
            land_owner,
            if discard {
                Zone::Hand
            } else {
                Zone::Battlefield
            },
        );
    }
    let start = std::time::Instant::now();
    dm.target_name = if discard {
        Some("PLAYER_ALICE".into())
    } else {
        None
    };
    println!("stage: Gitrog producer {case}");
    let (mut q, paid) = cast(
        &mut g,
        &defs[if discard { "Mind Rot" } else { "Damnation" }],
        alice(),
        &mut dm,
    )?;
    let error = resolve(&mut g, &mut q, &mut dm).err();
    println!("stage: Gitrog producer resolved {case}");
    let actual = json!({"resolution_error":error,"producer_paid":paid,"gitrog_battlefield":count(&g,"The Gitrog Monster",Zone::Battlefield),"lands_graveyard":count(&g,"Dryad Arbor",Zone::Graveyard),"alice_hand":g.player(alice()).unwrap().hand.len(),"bob_hand":g.player(bob).unwrap().hand.len(),"stack":g.stack.len()});
    Ok((
        actual,
        json!({"choices":dm.trace,"action_seconds":start.elapsed().as_secs_f64(),"controller_before_producer":if borrowed{1}else{0},"land_owner":land_owner.index()}),
    ))
}
#[test]
#[ignore = "manual bounded native reductions of MAGE layer and Gitrog timeouts"]
fn report_mage_layer_gitrog() {
    let case = std::env::var("MAGE_NATIVE_CASE").unwrap_or("coat-two".into());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_mage_layer_gitrog_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = if case.starts_with("coat") {
        vec![
            "Coat of Arms",
            "Mistform Ultimus",
            "Lord of Atlantis",
            "Lord of the Unreal",
            "Woodland Changeling",
            "Mutavault",
            "Smuggler's Copter",
        ]
    } else {
        vec![
            "The Gitrog Monster",
            "Damnation",
            "Dryad Arbor",
            "Control Magic",
            "Mind Rot",
        ]
    };
    let names: Vec<_> = names.into_iter().map(str::to_owned).collect();
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
    let expected = if case.starts_with("coat") {
        let entries = if case == "coat-two" {
            vec![("Woodland Changeling", 3), ("Woodland Changeling", 3)]
        } else {
            let v = if case == "coat-absent" {
                [5, 2, 2, 4, 4, 4, 3]
            } else {
                [10, 6, 6, 9, 9, 9, 3]
            };
            [
                "Mistform Ultimus",
                "Lord of Atlantis",
                "Lord of the Unreal",
                "Woodland Changeling",
                "Woodland Changeling",
                "Mutavault",
                "Smuggler's Copter",
            ]
            .into_iter()
            .zip(v)
            .collect()
        };
        json!({"creatures":entries.into_iter().map(|(n,v)|json!({"name":n,"power":v,"toughness":v,"creature":true})).collect::<Vec<_>>(),"stack":0})
    } else {
        let count = if case.ends_with('0') {
            0
        } else if case.ends_with('2') {
            2
        } else {
            1
        };
        let borrowed = case.contains("borrowed");
        let bobland = case.contains("bobland");
        let discard = case.contains("discard");
        json!({"resolution_error":null,"producer_paid":if discard{3}else{4},"gitrog_battlefield":usize::from(discard),"lands_graveyard":count,"alice_hand":usize::from(discard&&count>0&&!borrowed),"bob_hand":usize::from(discard&&count>0&&borrowed&&bobland),"stack":0})
    };
    let result = if defs.len() != names.len() {
        Err("strict artifact unavailable for at least one source/support card".into())
    } else if case.starts_with("coat") {
        coat(&defs, &case)
    } else {
        gitrog(&defs, &case)
    };
    let (status, actual, diag) = match result {
        Ok((v, t)) if v == expected => ("passed", v, t),
        Ok((v, t)) => ("semantic_mismatch", v, t),
        Err(e) => (
            "execution_or_fixture_error",
            json!({"error":e}),
            Value::Null,
        ),
    };
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Small strict native counterparts to imported MAGE timeouts. Actual paid Coat cast, actual Mutavault activation/Copter crew; or actual paid Gitrog, optional paid Control Magic transfer, and actual Damnation/Mind Rot producers. Normal priority, SBA and real graveyard/draw events. No full imported turn progression claim.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":[{"card":if case.starts_with("coat"){"Coat of Arms"}else{"The Gitrog Monster"},"scenario":case,"status":status,"expected":expected,"actual":actual,"diagnostics":diag}]});
    std::fs::write(
        root.join(format!("reports/runtime-audit/mage-native-{case}.json")),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
