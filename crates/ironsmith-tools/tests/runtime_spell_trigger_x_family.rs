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
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, Effect, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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
fn setup() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
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
        for p in [alice(), PlayerId::from_index(1)] {
            g.player_mut(p).unwrap().mana_pool.add(color, 30);
        }
    }
    g
}

struct ProbeDm {
    x: u32,
    accept: bool,
    target_name: Option<String>,
    object_choices: Vec<Value>,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.accept
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        if c.is_x_value { self.x } else { c.min }
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = SelectFirstDecisionMaker.decide_objects(g, c);
        self.object_choices.push(json!({"description":c.description,"min":c.min,"max":c.max,
            "candidates":c.candidates.iter().map(|v|json!({"id":v.id.0,"name":g.object(v.id).map(|o|o.name.to_string()),"legal":v.legal})).collect::<Vec<_>>(),
            "selected":selected.iter().map(|id|id.0).collect::<Vec<_>>()}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(g, c)
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
        object_choices: vec![],
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

fn scenario(def: &CardDefinition, mana_cost: u32, x: Option<u32>) -> Result<Value, String> {
    let mut g = setup();
    let source = established(&mut g, def, alice(), mana_cost)?;
    let filler = CardDefinitionBuilder::new(CardId::new(), "Spell X library witness")
        .card_types(vec![CardType::Artifact])
        .build();
    for _ in 0..5 {
        g.create_object_from_definition(&filler, alice(), Zone::Library);
    }
    let spell = witness(
        "Spell X family audit sorcery",
        CardType::Sorcery,
        1,
        x.is_some(),
    );
    let mut dm = dm(x.unwrap_or(0), true);
    let (mut q, paid) = cast(&mut g, &spell, alice(), &mut dm)?;
    if paid != 1 + x.unwrap_or(0) {
        return Err(format!("wrong spell payment {paid}"));
    }
    let error = resolve(&mut g, &mut q, &mut dm).err();
    let counters = g
        .object(source)
        .ok_or("source disappeared")?
        .counters
        .get(&ironsmith::object::CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);
    Ok(
        json!({"resolution_error":error,"mana_paid":paid,"source_counters":counters,
        "library_cards_in_hand":count(&g,filler.name(),Zone::Hand)}),
    )
}
fn kiora(def: &CardDefinition, kraken: bool, mana_cost: u8) -> Result<(Value, Value), String> {
    let mut g = setup();
    established(&mut g, def, alice(), 5)?;
    let filler = CardDefinitionBuilder::new(CardId::new(), "Kiora free spell witness")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    for _ in 0..6 {
        g.create_object_from_definition(&filler, alice(), Zone::Library);
    }
    let spell = CardDefinitionBuilder::new(CardId::new(), "Kiora cast witness")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![if kraken {
            ironsmith::Subtype::Kraken
        } else {
            ironsmith::Subtype::Human
        }])
        .power_toughness(PowerToughness::fixed(3, 3))
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(mana_cost)]))
        .build();
    let mut dm = dm(0, true);
    let (mut q, paid) = cast(&mut g, &spell, alice(), &mut dm)?;
    if paid != u32::from(mana_cost) {
        return Err(format!("wrong Kraken payment {paid}"));
    }
    let triggers = q.entries.len() + g.stack.iter().filter(|entry| entry.is_ability).count();
    let error = resolve(&mut g, &mut q, &mut dm).err();
    Ok((
        json!({"resolution_error":error,"mana_paid":paid,
        "free_artifacts":count(&g,filler.name(),Zone::Battlefield),
        "library_remaining":count(&g,filler.name(),Zone::Library)}),
        json!({"triggers_before_resolution":triggers,"object_choices":dm.object_choices}),
    ))
}
#[test]
#[ignore = "manual strict paid-cast expected-result audit"]
fn report_spell_trigger_x_family() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_spell_trigger_x_family.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = [
        "Geometer's Arthropod",
        "Nev, the Practical Dean",
        "Kiora, Sovereign of the Deep",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut rows = vec![];
    let mut compilation = vec![];
    for p in payloads.into_values().flatten() {
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p.parse_name.as_deref().unwrap_or(&p.name),
        );
        let (a, d) =
            ironsmith_registry::compile_builder_to_artifact(builder, &p.parse_input, false)
                .unwrap();
        compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        if p.name.starts_with("Kiora,") {
            for (kraken, mana_cost) in [(false, 4), (true, 2), (true, 4)] {
                let expected = json!({"resolution_error":null,"mana_paid":mana_cost,
                    "free_artifacts":usize::from(kraken),"library_remaining":6-usize::from(kraken)});
                let (status, actual, diagnostics) = match kiora(&d, kraken, mana_cost) {
                    Ok((v, trace)) if v == expected => ("passed", v, trace),
                    Ok((v, trace)) if !v["resolution_error"].is_null() => {
                        ("confirmed_resolution_failure", v, trace)
                    }
                    Ok((v, trace)) => ("semantic_mismatch", v, trace),
                    Err(e) => (
                        "execution_or_fixture_error",
                        json!({"error":e}),
                        Value::Null,
                    ),
                };
                rows.push(json!({"card":p.name,"scenario":format!("paid hand cast kraken={kraken}, mana_value={mana_cost}"),"status":status,"expected":expected,"actual":actual,"diagnostics":diagnostics}));
            }
            continue;
        }
        for x in [None, Some(0), Some(2), Some(4)] {
            let nev = p.name.starts_with("Nev,");
            let expected = json!({"resolution_error":null,"mana_paid":1+x.unwrap_or(0),
                "source_counters":if nev{x.unwrap_or(0)}else{0},
                "library_cards_in_hand":usize::from(!nev&&x.unwrap_or(0)>0)});
            let (status, actual) = match scenario(&d, if nev { 3 } else { 2 }, x) {
                Ok(v) if v == expected => ("passed", v),
                Ok(v) if !v["resolution_error"].is_null() => ("confirmed_resolution_failure", v),
                Ok(v) => ("semantic_mismatch", v),
                Err(e) => ("execution_or_fixture_error", json!({"error":e})),
            };
            rows.push(json!({"card":p.name,"scenario":format!("legal source then paid spell X{x:?}"),"status":status,"expected":expected,"actual":actual}));
        }
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict canonical source artifacts, actual legally paid source and neutral X/non-X spell casts; five available library cards for Geometer. First qualifying X spell after casting Nev. Kiora sees a legally paid Kraken/Human hand cast with six eligible one-mana artifacts in the library. Normal generated spell-cast triggers; no fallback or state-repair assertions.",
        "provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/spell-trigger-x-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("wrote {} rows", report["rows"].as_array().unwrap().len());
}
