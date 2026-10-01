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
        SelectFirstDecisionMaker.decide_objects(g, c)
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
fn zaxara(def: &CardDefinition, x: Option<u32>) -> Result<Value, String> {
    let mut g = setup();
    established(&mut g, def, alice(), 4)?;
    let w = witness("X trigger audit spell", CardType::Sorcery, 1, x.is_some());
    let mut dm = dm(x.unwrap_or(0), true);
    let (mut q, paid) = cast(&mut g, &w, alice(), &mut dm)?;
    let error = resolve(&mut g, &mut q, &mut dm).err();
    let hydras: Vec<_> = g
        .objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.zone == Zone::Battlefield && o.name.contains("Hydra"))
        .map(|o| {
            o.counters
                .get(&ironsmith::object::CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0)
        })
        .collect();
    Ok(json!({"resolution_error":error,"mana_paid":paid,"hydra_counters":hydras}))
}
fn queller(
    defs: &HashMap<String, CardDefinition>,
    processed: bool,
    accept: bool,
) -> Result<Value, String> {
    let mut g = setup();
    let w = witness("Queller audit sorcery", CardType::Sorcery, 2, false);
    let mut dm = dm(0, accept);
    let (_q, paid) = cast(&mut g, &w, alice(), &mut dm)?;
    if paid != 2 {
        return Err("witness cast payment incorrect".into());
    }
    dm.target_name = Some(w.name().to_owned());
    let (mut q, paid) = cast(&mut g, &defs["Spell Queller"], alice(), &mut dm)?;
    if paid != 3 {
        return Err("Queller payment incorrect".into());
    }
    resolve(&mut g, &mut q, &mut dm)?;
    if count(&g, w.name(), Zone::Exile) != 1 {
        return Err("Queller did not establish linked exile".into());
    }
    if processed {
        dm.target_name = Some(w.name().to_owned());
        let (mut q, paid) = cast(&mut g, &defs["Pull from Eternity"], alice(), &mut dm)?;
        if paid != 1 {
            return Err("Pull payment incorrect".into());
        }
        resolve(&mut g, &mut q, &mut dm)?;
        if count(&g, w.name(), Zone::Graveyard) != 1 {
            return Err("Pull did not move linked exile card".into());
        }
    }
    dm.target_name = Some("Spell Queller".into());
    let (mut q, paid) = cast(&mut g, &defs["Murder"], alice(), &mut dm)?;
    if paid != 3 {
        return Err("Murder payment incorrect".into());
    }
    let error = resolve(&mut g, &mut q, &mut dm).err();
    Ok(
        json!({"resolution_error":error,"queller_graveyard":count(&g,"Spell Queller",Zone::Graveyard),"witness_exile":count(&g,w.name(),Zone::Exile),"witness_graveyard":count(&g,w.name(),Zone::Graveyard),"alice_life":g.player(alice()).unwrap().life}),
    )
}
fn conqueror(
    defs: &HashMap<String, CardDefinition>,
    accept: bool,
    destroy: bool,
    present: bool,
) -> Result<Value, String> {
    let mut g = setup();
    let bob = PlayerId::from_index(1);
    if present {
        g.turn.active_player = bob;
        established(&mut g, &defs["Charismatic Conqueror"], bob, 2)?;
        g.turn.turn_number += 1;
        g.turn.active_player = alice();
    }
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Opponent entry audit witness")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    let mut dm = dm(0, accept);
    let (mut q, paid) = cast(&mut g, &artifact, alice(), &mut dm)?;
    if paid != 1 {
        return Err("artifact payment incorrect".into());
    }
    // The public single-entry helper omits dedicated EnterBattlefield events.
    // Passing priority uses the production resolver with its trigger queue.
    let mut priority = PriorityLoopState::new(g.players_in_game());
    priority.reset_for_new_priority_window(&mut g);
    for _ in 0..g.players_in_game() {
        apply_priority_response_with_dm(
            &mut g,
            &mut q,
            &mut priority,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            &mut dm,
        )
        .map_err(|e| format!("artifact entry via priority failed: {e}"))?;
    }
    drain_pending_trigger_events(&mut g, &mut q);
    put_triggers_on_stack_with_dm(&mut g, &mut q, &mut dm).map_err(|e| e.to_string())?;
    if g.stack.len() != usize::from(present) {
        return Err(format!("expected {} Conqueror triggers, found {}", usize::from(present), g.stack.len()));
    }
    if destroy {
        dm.target_name = Some(artifact.name().into());
        let (_q, paid) = cast(&mut g, &defs["Shatter"], alice(), &mut dm)?;
        if paid != 2 {
            return Err("Shatter payment incorrect".into());
        }
        resolve_stack_entry_with(&mut g, &mut dm).map_err(|e| format!("Shatter failed: {e}"))?;
        if count(&g, artifact.name(), Zone::Graveyard) != 1 {
            return Err("Shatter did not destroy witness".into());
        }
    }
    let error = resolve(&mut g, &mut q, &mut dm).err();
    let tapped = find(&g, artifact.name(), Zone::Battlefield)
        .ok()
        .map(|id| g.is_tapped(id));
    Ok(
        json!({"resolution_error":error,"vampires":count(&g,"Vampire",Zone::Battlefield),"artifact_tapped":tapped,"artifact_graveyard":count(&g,artifact.name(),Zone::Graveyard)}),
    )
}
fn record(
    rows: &mut Vec<Value>,
    name: &str,
    label: &str,
    expected: Value,
    result: Result<Value, String>,
) {
    let (status, actual) = match result {
        Ok(v) if v == expected => ("passed", v),
        Ok(v) if !v["resolution_error"].is_null() => ("confirmed_resolution_failure", v),
        Ok(v) => ("semantic_mismatch", v),
        Err(e) => ("execution_or_fixture_error", json!({"error":e})),
    };
    rows.push(
        json!({"card":name,"scenario":label,"status":status,"expected":expected,"actual":actual}),
    );
}
#[test]
#[ignore = "manual expected-result audit; inspect row statuses"]
fn report_mage_followup_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_mage_followup_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = [
        "Zaxara, the Exemplary",
        "Spell Queller",
        "Charismatic Conqueror",
        "Pull from Eternity",
        "Murder",
        "Shatter",
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
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p.parse_name.as_deref().unwrap_or(&p.name),
        );
        let (a, d) =
            ironsmith_registry::compile_builder_to_artifact(builder, &p.parse_input, false)
                .unwrap();
        compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        defs.insert(d.name().to_owned(), d);
    }
    let mut rows = vec![];
    for x in [None, Some(0), Some(2), Some(4)] {
        record(
            &mut rows,
            "Zaxara, the Exemplary",
            &format!("legally cast source4 then spell X{x:?}"),
            json!({"resolution_error":null,"mana_paid":1+x.unwrap_or(0),"hydra_counters":x.into_iter().collect::<Vec<_>>()}),
            zaxara(&defs["Zaxara, the Exemplary"], x),
        );
    }
    for (processed, accept) in [(false, false), (false, true), (true, true)] {
        record(
            &mut rows,
            "Spell Queller",
            &format!("actual exile, processed={processed}, acceptfreecast={accept}"),
            json!({"resolution_error":null,"queller_graveyard":1,"witness_exile":usize::from(!processed&&!accept),"witness_graveyard":usize::from(processed||accept),"alice_life":if !processed&&accept{21}else{20}}),
            queller(&defs, processed, accept),
        );
    }
    for (accept, destroy, present) in [
        (true, false, true),
        (false, false, true),
        (true, true, true),
        (true, false, false),
    ] {
        record(
            &mut rows,
            "Charismatic Conqueror",
            &format!(
                "opponent artifact entry; accepttap={accept}, destroybeforetrigger={destroy}, present={present}"
            ),
            json!({"resolution_error":null,"vampires":usize::from(present&&(!accept||destroy)),"artifact_tapped":if destroy{None}else{Some(present&&accept)},"artifact_graveyard":usize::from(destroy)}),
            conqueror(&defs, accept, destroy, present),
        );
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict source/support artifacts and actual paid casts. Zaxara sees real X/nonX spells; Hydra counters measured before SBA. Queller exiles an actual spell, Pull legally removes its linked exile card, Murder creates its normal leave event. Conqueror sees a real opponent artifact entry with tap/decline or Shatter-before-resolution. No oracle fallback, unavailable action forcing, or assertion repairs.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/mage-followup-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("wrote {} rows", report["rows"].as_array().unwrap().len());
}
