//! Canonical-card expected-result audit. Reporter success is not gameplay success.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

struct Decisions {
    accept: bool,
}
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.accept
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
fn cast(game: &mut GameState, source: ObjectId, dm: &mut impl DecisionMaker) -> Result<(), String> {
    let action = compute_legal_actions(game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == source))
        .ok_or("fixture has no legal cast")?;
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
        if state.pending_cast.is_none() && !game.stack.is_empty() {
            drain_pending_trigger_events(game, &mut queue);
            put_triggers_on_stack_with_dm(game, &mut queue, dm).map_err(|e| e.to_string())?;
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast stalled: {progress:?}"));
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("cast decision bound exceeded".into())
}
fn resolve_all(game: &mut GameState, dm: &mut dyn DecisionMaker) -> Result<usize, String> {
    let mut queue = TriggerQueue::new();
    let mut count = 0;
    for _ in 0..32 {
        drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(count);
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
        count += 1;
    }
    Err("resolution bound exceeded".into())
}
fn color_count(def: &CardDefinition, colors: usize, accept: bool) -> Result<Value, String> {
    let mut game = setup();
    game.create_object_from_definition(def, PlayerId(0), Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Count audit library filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..24 {
        game.create_object_from_definition(&filler, PlayerId(0), Zone::Library);
    }
    for _ in 0..2 {
        game.create_object_from_definition(&filler, PlayerId(0), Zone::Hand);
    }
    let symbols = if colors == 0 {
        vec![ManaSymbol::Generic(1)]
    } else {
        vec![
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ][..colors]
            .to_vec()
    };
    let spell = CardDefinitionBuilder::new(CardId::new(), "Count audit colored spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(symbols))
        .build();
    let spell_id = game.create_object_from_definition(&spell, PlayerId(0), Zone::Hand);
    if game.current_colors(spell_id).map(|v| v.count()) != Some(colors as u32) {
        return Err("fixture colors incorrect".into());
    }
    let mut dm = Decisions { accept };
    cast(&mut game, spell_id, &mut dm)?;
    let stack_before = game.stack.len();
    let resolution = resolve_all(&mut game, &mut dm);
    Ok(
        json!({"resolution_error":resolution.err(),"life":game.player(PlayerId(0)).unwrap().life,
        "hand":game.player(PlayerId(0)).unwrap().hand.len(),"library":game.player(PlayerId(0)).unwrap().library.len(),
        "stack_before_resolution":stack_before}),
    )
}
fn tend(
    def: &CardDefinition,
    forest: &CardDefinition,
    lands: usize,
    treefolk: usize,
    overlap: bool,
) -> Result<Value, String> {
    let mut game = setup();
    let tree = CardDefinitionBuilder::new(CardId::new(), "Count audit Treefolk")
        .card_types(if overlap {
            vec![CardType::Land, CardType::Creature]
        } else {
            vec![CardType::Creature]
        })
        .subtypes(vec![Subtype::Treefolk])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
    for _ in 0..lands {
        game.create_object_from_definition(forest, PlayerId(0), Zone::Battlefield);
    }
    for _ in 0..treefolk {
        game.create_object_from_definition(&tree, PlayerId(0), Zone::Battlefield);
    }
    game.create_object_from_definition(forest, PlayerId(0), Zone::Library);
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    let mut dm = SelectFirstDecisionMaker;
    cast(&mut game, source, &mut dm)?;
    let resolution = resolve_all(&mut game, &mut dm);
    Ok(
        json!({"resolution_error":resolution.err(),"tokens":game.battlefield.iter().filter(|id| game.object(**id).is_some_and(|o| o.kind == ironsmith::object::ObjectKind::Token)).count(),
        "permanents":game.battlefield.len(),"library":game.player(PlayerId(0)).unwrap().library.len()}),
    )
}
fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: Value,
    expected: Value,
    result: Result<Value, String>,
    scope: &str,
) {
    let (status, actual) = match result {
        Ok(actual) if actual == expected => ("expected_result_observed", actual),
        Ok(actual) => ("semantic_mismatch", actual),
        Err(error) => ("fixture_or_announcement_error", json!({"error":error})),
    };
    rows.push(json!({"card":card,"scenario":scenario,"expected":expected,"actual":actual,"status":status,"scope":scope}));
}
fn fingerprint(path: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
#[ignore = "report generation only; inspect outcome statuses"]
fn report_count_semantics() {
    let names = [
        "Ancient Cornucopia",
        "Moonveil Regent",
        "Tend the Sprigs",
        "Forest",
    ]
    .map(str::to_string);
    let path = ironsmith_tools::default_cards_path();
    let payloads =
        ironsmith_tools::load_card_payloads_by_names(path.to_str().unwrap(), &names).unwrap();
    let definitions: HashMap<_, _> = payloads
        .into_values()
        .flatten()
        .map(|payload| {
            let definition =
                ironsmith_tools::compile_runtime_definition_from_payload(&payload).unwrap();
            (definition.card.name.clone(), definition)
        })
        .collect();
    let binary = std::env::current_exe().unwrap();
    let binary_before = fingerprint(&binary);
    let mut rows = Vec::new();
    for name in ["Ancient Cornucopia", "Moonveil Regent"] {
        for (colors, accept) in [
            (0, true),
            (1, true),
            (2, true),
            (3, true),
            (5, true),
            (3, false),
        ] {
            let is_cornucopia = name == "Ancient Cornucopia";
            let expected = json!({"resolution_error":null,"life":20 + if is_cornucopia && accept { colors } else {0},
                "hand":if !is_cornucopia && accept {colors} else {2},
                "library":24 - if !is_cornucopia && accept {colors} else {0},
                "stack_before_resolution":if is_cornucopia && colors == 0 {1} else {2}});
            record(
                &mut rows,
                name,
                json!({"spell_colors":colors,"accept_optional":accept}),
                expected,
                color_count(&definitions[name], colors, accept),
                "actual engine-offered spell cast and paid mana; cast event, triggered ability, optional choice, hand/library/life outcome",
            );
        }
    }
    for (lands, treefolk, overlap) in [
        (0, 0, false),
        (1, 1, false),
        (5, 1, false),
        (6, 0, false),
        (6, 1, false),
        (0, 6, false),
        (0, 6, true),
        (0, 3, true),
    ] {
        let tokens = usize::from(lands + treefolk + 1 >= 7);
        let expected = json!({"resolution_error":null,"tokens":tokens,"permanents":lands+treefolk+1+tokens,"library":0});
        record(
            &mut rows,
            "Tend the Sprigs",
            json!({"lands_before":lands,"treefolk_before":treefolk,"treefolk_are_also_lands":overlap}),
            expected,
            tend(
                &definitions["Tend the Sprigs"],
                &definitions["Forest"],
                lands,
                treefolk,
                overlap,
            ),
            "actual paid legal spell cast, basic land search then union-count condition, including land-creature overlap counted once",
        );
    }
    let report = json!({"rows":rows,"provenance":{"binary":binary,"binary_sha256":binary_before,"binary_unchanged":binary_before==fingerprint(&binary),"cards_sha256":fingerprint(&path)},
        "scope":"bounded expected-result count/color scenarios; not corpus completeness"});
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join("reports/runtime-audit/count-semantic-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
