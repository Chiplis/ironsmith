//! Opt-in canonical outcome reports; a passing reporter does not mean card correctness.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::StackEntry;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Default)]
struct Decisions {
    trace: Vec<Value>,
}
impl DecisionMaker for Decisions {
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.trace.push(json!({"description":ctx.description,"minimum":ctx.min,"maximum":ctx.max,
            "candidates":ctx.candidates.iter().map(|c|json!({"name":game.object(c.id).map(|o|o.name.to_string()),"legal":c.legal})).collect::<Vec<_>>()}));
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(71757432704846);
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ] {
        game.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(symbol, 12);
    }
    game
}
fn announce(
    game: &mut GameState,
    action: LegalAction,
    dm: &mut impl DecisionMaker,
) -> Result<(), String> {
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
    for _ in 0..32 {
        if state.pending_cast.is_none()
            && state.pending_activation.is_none()
            && !game.stack.is_empty()
        {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stalled: {progress:?}"));
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("announcement bound exceeded".into())
}
fn triad(def: &CardDefinition, values: &[u32]) -> Result<(Value, Value), String> {
    let mut game = setup();
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    for (i, value) in values.iter().copied().enumerate() {
        let artifact =
            CardDefinitionBuilder::new(CardId::new(), &format!("Threshold audit artifact {i}"))
                .card_types(vec![CardType::Artifact])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
                    value as u8,
                )]))
                .build();
        game.create_object_from_definition(&artifact, PlayerId(0), Zone::Graveyard);
    }
    let action = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..} if *s==source));
    let mut dm = Decisions::default();
    let mut announce_error = None;
    let mut resolution_error = None;
    let offered = action.is_some();
    if let Some(action) = action {
        if let Err(e) = announce(&mut game, action, &mut dm) {
            announce_error = Some(e)
        } else {
            resolution_error = resolve_stack_entry_with(&mut game, &mut dm)
                .err()
                .map(|e| e.to_string());
        }
    }
    Ok((
        json!({"ability_offered":offered,"announcement_error":announce_error,"resolution_error":resolution_error,
        "exiled":game.exile.len(),"graveyard_remaining":game.player(PlayerId(0)).unwrap().graveyard.len(),"command_objects":game.command_zone.len()}),
        json!({"object_decisions":dm.trace}),
    ))
}
fn trumpet(def: &CardDefinition, mana_value: u32) -> Result<(Value, Value), String> {
    let mut game = setup();
    let target = CardDefinitionBuilder::new(CardId::new(), "Threshold audit target spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value as u8,
        )]))
        .build();
    let target_id = game.create_object_from_definition(&target, PlayerId(1), Zone::Stack);
    game.push_to_stack(StackEntry::new(target_id, PlayerId(1)));
    let filler = CardDefinitionBuilder::new(CardId::new(), "Threshold audit library creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
        .build();
    for _ in 0..8 {
        game.create_object_from_definition(&filler, PlayerId(0), Zone::Library);
    }
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let action = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==source))
        .ok_or("no legal counterspell cast")?;
    let mut dm = SelectFirstDecisionMaker;
    announce(&mut game, action, &mut dm)?;
    if game.stack.len() != 2
        || game.stack.last().unwrap().targets
            != vec![ironsmith::game_state::Target::Object(target_id)]
    {
        return Err(format!(
            "counterspell target fixture incorrect: {:?}",
            game.stack.last()
        ));
    }
    let resolution_error = resolve_stack_entry_with(&mut game, &mut dm)
        .err()
        .map(|e| e.to_string());
    Ok((
        json!({"resolution_error":resolution_error,"target_countered":game.player(PlayerId(1)).unwrap().graveyard.len()==1,
        "recruit_occurred":game.player(PlayerId(0)).unwrap().library.len()!=8}),
        json!({"library_remaining":game.player(PlayerId(0)).unwrap().library.len(),"hand":game.player(PlayerId(0)).unwrap().hand.len(),"tokens":game.battlefield.len()}),
    ))
}
fn row(
    rows: &mut Vec<Value>,
    name: &str,
    scenario: Value,
    expected: Value,
    result: Result<(Value, Value), String>,
    scope: &str,
) {
    let (status, actual, evidence) = match result {
        Ok((actual, evidence)) if actual == expected => {
            ("expected_result_observed", actual, evidence)
        }
        Ok((actual, evidence)) => ("semantic_mismatch", actual, evidence),
        Err(e) => (
            "fixture_or_announcement_error",
            json!({"error":e}),
            Value::Null,
        ),
    };
    rows.push(json!({"card":name,"scenario":scenario,"expected":expected,"actual":actual,"fixture_evidence":evidence,"status":status,"scope":scope}));
}
fn hash(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "report generation only; inspect expected/actual rows"]
fn report_threshold_casts() {
    let path = ironsmith_tools::default_cards_path();
    let names = ["The Capitoline Triad", "Sound the Trumpets"].map(str::to_string);
    let definitions: HashMap<_, _> =
        ironsmith_tools::load_card_payloads_by_names(path.to_str().unwrap(), &names)
            .unwrap()
            .into_values()
            .flatten()
            .map(|payload| {
                let definition =
                    ironsmith_tools::compile_runtime_definition_from_payload(&payload).unwrap();
                (payload.name, definition)
            })
            .collect();
    let binary = std::env::current_exe().unwrap();
    let binary_before = hash(&binary);
    let mut rows = Vec::new();
    for values in [
        vec![],
        vec![29],
        vec![30],
        vec![10, 10, 10],
        vec![15, 15],
        vec![15],
    ] {
        let payable = values.iter().sum::<u32>() >= 30;
        let expected = json!({"ability_offered":payable,"announcement_error":null,"resolution_error":null,"exiled":if payable {values.len()}else{0},
            "graveyard_remaining":if payable {0}else{values.len()},"command_objects":usize::from(payable)});
        row(
            &mut rows,
            "The Capitoline Triad",
            json!({"historic_graveyard_mana_values":values}),
            expected,
            triad(&definitions["The Capitoline Triad"], &values),
            "engine-offered activation, actual selection/cost payment, exiled/graveyard cards and created emblem; known threshold-boundary fixtures",
        );
    }
    for mana_value in [0, 1, 2, 3, 6] {
        row(
            &mut rows,
            "Sound the Trumpets",
            json!({"target_spell_mana_value":mana_value}),
            json!({"resolution_error":null,"target_countered":true,"recruit_occurred":mana_value<=2}),
            trumpet(&definitions["Sound the Trumpets"], mana_value),
            "actual paid targeted cast and resolution; simple opponent spell seeded on stack, observable recruit draw compared at mana-value boundary",
        );
    }
    let report = json!({"rows":rows,"provenance":{"binary":binary,"binary_sha256":binary_before,"binary_unchanged":binary_before==hash(&binary),"cards_sha256":hash(&path)},"scope":"eleven bounded expected-result scenarios; not corpus completeness"});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/threshold-cast-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
