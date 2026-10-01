//! Strict artifact, legal action probes for remaining missing effect-outcome families.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker,
    compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm, drain_pending_trigger_events,
    put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, Effect, GameState, ObjectId, PlayerId, PowerToughness,
    Subtype, Zone,
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
fn bob() -> PlayerId {
    PlayerId::from_index(1)
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
        g.player_mut(alice()).unwrap().mana_pool.add(color, 30);
    }
    g
}
struct ProbeDm {
    decline: bool,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(g, c)
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let target = Target::Player(bob());
        if c.requirements.len() == 1 && c.requirements[0].legal_targets.contains(&target) {
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(g, c)
        }
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
    game.turn.priority_player = Some(alice());
    let action = compute_legal_actions(game, alice()).expect("fixture has complete replacement state")
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
    let mana = game.player(alice()).unwrap().mana_pool.total();
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
        if !game.stack.is_empty() {
            return Ok((
                queue,
                mana - game.player(alice()).unwrap().mana_pool.total(),
            ));
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
fn neutral(cost: u8, kind: CardType, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(cost)]))
        .power_toughness(PowerToughness::fixed(3, 3))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build()
}
fn count(game: &GameState, name: &str, zone: Zone) -> usize {
    game.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.name == name && o.zone == zone)
        .count()
}
fn cast_permanent(
    game: &mut GameState,
    definition: &CardDefinition,
    expected: u32,
) -> Result<ObjectId, String> {
    let id = game.create_object_from_definition(definition, alice(), Zone::Hand);
    let mut dm = ProbeDm { decline: false };
    let (mut q, paid) = perform(game, id, None, &mut dm)?;
    if paid != expected {
        return Err(format!("fixture permanent paid{paid},expected{expected}"));
    }
    resolve(game, &mut q, &mut dm)?;
    game.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name == definition.name() && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .ok_or("fixture permanent did not enter".into())
}
fn kaya(def: &CardDefinition, exiled: usize) -> Result<Value, String> {
    let mut game = setup();
    let source = cast_permanent(&mut game, def, 3)?;
    game.object_mut(source)
        .unwrap()
        .counters
        .insert(CounterType::Loyalty, 5);
    let card = neutral(1, CardType::Instant, "Exiled audit card");
    for _ in 0..exiled {
        game.create_object_from_definition(&card, bob(), Zone::Exile);
    }
    game.create_object_from_definition(&card, alice(), Zone::Exile);
    let mut dm = ProbeDm { decline: false };
    let (mut q, paid) = perform(&mut game, source, Some(2), &mut dm)?;
    if paid != 0 {
        return Err("loyalty ability unexpectedly paid mana".into());
    }
    let loyalty = game
        .object(source)
        .and_then(|o| o.counters.get(&CounterType::Loyalty))
        .copied()
        .unwrap_or(0);
    if loyalty != 0 {
        return Err(format!("loyalty cost not paid: {loyalty}"));
    }
    let target = game
        .stack
        .last()
        .ok_or("Kaya stack missing")?
        .targets
        .clone();
    if target != vec![Target::Player(bob())] {
        return Err(format!("Kaya target incorrect: {target:?}"));
    }
    let error = resolve(&mut game, &mut q, &mut dm).err();
    Ok(
        json!({"resolution_error":error,"alice_life":game.player(alice()).unwrap().life,"bob_life":game.player(bob()).unwrap().life,"loyalty_paid":5}),
    )
}
fn cage(def: &CardDefinition, populated: bool) -> Result<Value, String> {
    let mut game = setup();
    if populated {
        game.create_object_from_definition(
            &neutral(1, CardType::Artifact, "Cage artifact"),
            alice(),
            Zone::Battlefield,
        );
        game.create_object_from_definition(
            &neutral(2, CardType::Creature, "Cage creature"),
            bob(),
            Zone::Battlefield,
        );
    }
    let source = cast_permanent(&mut game, def, 3)?;
    let exiled = game.exile.len();
    if exiled != if populated { 2 } else { 0 } {
        return Err(format!("Cage ETB did not establish linked exile: {exiled}"));
    }
    let mut dm = ProbeDm { decline: false };
    let (mut q, paid) = perform(&mut game, source, Some(1), &mut dm)?;
    if paid != 8 {
        return Err(format!("Cage activation paid {paid}"));
    }
    let error = resolve(&mut game, &mut q, &mut dm).err();
    Ok(
        json!({"resolution_error":error,"exile_before":exiled,"exile_after":game.exile.len(),"graveyards":game.players.iter().map(|p|p.graveyard.len()).collect::<Vec<_>>(),"robots":count(&game,"Robot",Zone::Battlefield),"cage_battlefield":count(&game,def.name(),Zone::Battlefield)}),
    )
}
fn surtland(def: &CardDefinition, giant: bool, decline: bool) -> Result<Value, String> {
    let mut game = setup();
    let source = cast_permanent(&mut game, def, 5)?;
    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Flinger sacrifice witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3));
    if giant {
        builder = builder.subtypes(vec![Subtype::Giant]);
    }
    game.create_object_from_definition(&builder.build(), alice(), Zone::Battlefield);
    game.remove_summoning_sickness(source);
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut q = TriggerQueue::new();
    let mut dm = ProbeDm { decline };
    apply_attacker_declarations_with_dm(
        &mut game,
        &mut combat,
        &mut q,
        &[AttackerDeclaration {
            creature: source,
            target: AttackTarget::Player(bob()),
        }],
        &mut dm,
    )
    .map_err(|e| e.to_string())?;
    game.combat = Some(combat);
    let error = resolve(&mut game, &mut q, &mut dm).err();
    Ok(
        json!({"resolution_error":error,"bob_life":game.player(bob()).unwrap().life,"witness_graveyard":count(&game,"Flinger sacrifice witness",Zone::Graveyard),"witness_battlefield":count(&game,"Flinger sacrifice witness",Zone::Battlefield)}),
    )
}
fn tellah(def: &CardDefinition, cost: u8, creature: bool) -> Result<Value, String> {
    let mut game = setup();
    cast_permanent(&mut game, def, 5)?;
    let filler = neutral(1, CardType::Instant, "Tellah library witness");
    for _ in 0..12 {
        game.create_object_from_definition(&filler, alice(), Zone::Library);
    }
    let spell = neutral(
        cost,
        if creature {
            CardType::Creature
        } else {
            CardType::Sorcery
        },
        "Tellah cast witness",
    );
    let id = game.create_object_from_definition(&spell, alice(), Zone::Hand);
    let mut dm = ProbeDm { decline: false };
    let (mut q, paid) = perform(&mut game, id, None, &mut dm)?;
    if paid != u32::from(cost) {
        return Err(format!("Tellah witness paid {paid}, expected{cost}"));
    }
    let error = resolve(&mut game, &mut q, &mut dm).err();
    Ok(
        json!({"resolution_error":error,"hero_tokens":count(&game,"Hero",Zone::Battlefield),"drawn":game.player(alice()).unwrap().hand.len(),"tellah_battlefield":count(&game,def.name(),Zone::Battlefield),"bob_life":game.player(bob()).unwrap().life,"mana_paid":paid}),
    )
}
fn record(
    rows: &mut Vec<Value>,
    name: &str,
    scenario: &str,
    expected: Value,
    result: Result<Value, String>,
) {
    let (status, actual) = match result {
        Ok(a) if a == expected => ("passed", a),
        Ok(a) if !a["resolution_error"].is_null() => ("confirmed_resolution_failure", a),
        Ok(a) => ("semantic_mismatch", a),
        Err(e) => ("execution_or_fixture_error", json!({"error":e})),
    };
    rows.push(json!({"card":name,"scenario":scenario,"status":status,"expected":expected,"actual":actual}));
}
#[test]
#[ignore = "manual expected-result audit; inspect JSON row statuses"]
fn report_remaining_outcome_execution() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_remaining_outcome_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = [
        "Kaya, Orzhov Usurper",
        "Pinnacle Starcage",
        "Surtland Flinger",
        "Tellah, Great Sage",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut definitions = HashMap::new();
    let mut compilation = Vec::new();
    for p in payloads.into_values().flatten() {
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p.parse_name.as_deref().unwrap_or(&p.name),
        );
        let (a, d) =
            ironsmith_registry::compile_builder_to_artifact(builder, &p.parse_input, false)
                .unwrap();
        compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        definitions.insert(d.name().to_owned(), d);
    }
    let mut rows = Vec::new();
    for n in [0, 2, 5] {
        record(
            &mut rows,
            "Kaya, Orzhov Usurper",
            &format!("minus5 with opponent exile {n}; ownexile1 excluded"),
            json!({"resolution_error":null,"alice_life":20+n,"bob_life":20-n,"loyalty_paid":5}),
            kaya(&definitions["Kaya, Orzhov Usurper"], n),
        );
    }
    for populated in [false, true] {
        record(
            &mut rows,
            "Pinnacle Starcage",
            if populated {
                "paid8 afterETBexiled2"
            } else {
                "paid8 withoutlinkedcards"
            },
            json!({"resolution_error":null,"exile_before":if populated{2}else{0},"exile_after":0,"graveyards":if populated{[2,1]}else{[1,0]},"robots":if populated{2}else{0},"cage_battlefield":0}),
            cage(&definitions["Pinnacle Starcage"], populated),
        );
    }
    for (giant, decline, damage) in [(false, false, 3), (true, false, 6), (true, true, 0)] {
        record(
            &mut rows,
            "Surtland Flinger",
            &format!("legalattack;giant{giant};decline{decline}"),
            json!({"resolution_error":null,"bob_life":20-damage,"witness_graveyard":usize::from(!decline),"witness_battlefield":usize::from(decline)}),
            surtland(&definitions["Surtland Flinger"], giant, decline),
        );
    }
    for (cost, creature) in [(3, false), (4, false), (8, false), (8, true)] {
        record(
            &mut rows,
            "Tellah, Great Sage",
            &format!("paid{cost};creature{creature}"),
            json!({"resolution_error":null,"hero_tokens":usize::from(!creature),"drawn":if !creature&&cost>=4{2}else{0},"tellah_battlefield":if !creature&&cost>=8{0}else{1},"bob_life":if !creature&&cost>=8{12}else{20},"mana_paid":cost}),
            tellah(&definitions["Tellah, Great Sage"], cost, creature),
        );
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict canonical artifact source permanents legally cast with exact mana. Kaya pays actual minus5 loyalty; Cage ETB establishes actual linked exile beforepaid8; Surtland legal attacker declaration and explicit sacrifice/decline; Tellah actual qualifying/nonqualifying paidcasts. No oracle fallback or assertion repairs.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    let output = root.join("reports/runtime-audit/remaining-outcome-execution.json");
    std::fs::write(
        &output,
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
    println!("{}", serde_json::to_string(&report).unwrap());
}
