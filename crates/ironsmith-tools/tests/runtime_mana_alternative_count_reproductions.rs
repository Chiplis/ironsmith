//! Opt-in evidence for repeated-symbol mana alternatives; no engine modifications.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::color::Color;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    ColorsContext, DecisionContext, SelectObjectsContext, SelectOptionsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardDefinition, CardId, CardType, GameState, PlayerId, Zone};
use serde_json::{Value, json};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
struct Dm {
    requested: Vec<Color>,
    trace: Vec<Value>,
}
impl DecisionMaker for Dm {
    fn decide_colors(&mut self, _: &GameState, c: &ColorsContext) -> Vec<Color> {
        let selected = (0..c.count as usize)
            .map(|i| self.requested[i.min(self.requested.len() - 1)])
            .collect::<Vec<_>>();
        self.trace.push(json!({"choice":"mana_colors","context":format!("{c:?}"),"requested":format!("{:?}",self.requested),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_objects(
        &mut self,
        g: &GameState,
        c: &SelectObjectsContext,
    ) -> Vec<ironsmith::ObjectId> {
        let selected = SelectFirstDecisionMaker.decide_objects(g, c);
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
}
fn symbol(c: Color) -> ManaSymbol {
    match c {
        Color::White => ManaSymbol::White,
        Color::Blue => ManaSymbol::Blue,
        Color::Black => ManaSymbol::Black,
        Color::Red => ManaSymbol::Red,
        Color::Green => ManaSymbol::Green,
    }
}
fn pool(g: &GameState) -> Value {
    let p = &g.player(PlayerId(0)).unwrap().mana_pool;
    json!({"white":p.white,"blue":p.blue,"black":p.black,"red":p.red,"green":p.green,"colorless":p.colorless})
}
fn expected_pool(colors: &[Color], colorless: bool) -> Value {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    for c in colors {
        g.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(symbol(*c), 1)
    }
    if colorless {
        g.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1)
    }
    pool(&g)
}
fn perform(g: &mut GameState, action: LegalAction, dm: &mut Dm) -> Result<(), String> {
    let mut q = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":"actual_legal_action","action":format!("{action:?}")}));
    let mut result = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..16 {
        match result {
            GameProgress::NeedsDecisionCtx(ref c) if !matches!(c, DecisionContext::Priority(_)) => {
                result = apply_decision_context_with_dm(g, &mut q, &mut state, c, dm)
                    .map_err(|e| e.to_string())?
            }
            _ => {
                return if g.stack.is_empty() {
                    Ok(())
                } else {
                    Err("mana ability unexpectedly put an entry on stack".into())
                };
            }
        }
    }
    Err("mana activation decision budget".into())
}
fn run(
    d: &CardDefinition,
    requested: Vec<Color>,
    payment: Option<ManaSymbol>,
    colorless: bool,
    fuel: bool,
    dm: &mut Dm,
) -> Result<Value, String> {
    let alice = PlayerId(0);
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(SEED);
    g.turn.turn_number = 3;
    g.turn.active_player = alice;
    g.turn.priority_player = Some(alice);
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    let source = g.create_object_from_definition(d, alice, Zone::Battlefield);
    g.remove_summoning_sickness(source);
    if fuel {
        let f = CardDefinitionBuilder::new(CardId::new(), "Mana audit exile fuel")
            .card_types(vec![CardType::Instant])
            .build();
        g.create_object_from_definition(&f, alice, Zone::Hand);
    }
    if let Some(m) = payment {
        g.player_mut(alice).unwrap().mana_pool.add(m, 1);
    }
    let actions = compute_legal_actions(&g, alice).expect("fixture has complete replacement state")
        .into_iter()
        .filter(|a| matches!(a,LegalAction::ActivateManaAbility{source:id,..}if *id==source))
        .collect::<Vec<_>>();
    dm.trace.push(json!({"stage":"before_activation","mana":pool(&g),"available_mana_actions":format!("{actions:?}")}));
    let action = if colorless {
        actions.first()
    } else {
        actions.last()
    }
    .cloned()
    .ok_or("expected legal mana activation absent")?;
    dm.requested = requested;
    perform(&mut g, action, dm)?;
    let exiled = g
        .exile
        .iter()
        .filter(|id| {
            g.object(**id)
                .is_some_and(|o| o.name == "Mana audit exile fuel")
        })
        .count();
    Ok(
        json!({"mana":pool(&g),"source_tapped":g.is_tapped(source),"exiled_fuel":exiled,"hand":g.player(alice).unwrap().hand.len()}),
    )
}
fn generate_report() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reports/runtime-audit");
    let inputs: Value = serde_json::from_slice(
        &std::fs::read(root.join("repeated-mana-alternative-inputs.json")).unwrap(),
    )
    .unwrap();
    let mut defs = HashMap::new();
    let mut compilation = vec![];
    for (name, p) in inputs["cards"].as_object().unwrap() {
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p["parse_name"].as_str().unwrap_or(name),
        );
        let (a, d) = ironsmith_registry::compile_builder_to_artifact(
            b,
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compilation.push(json!({"card":name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        defs.insert(name.clone(), (d, a.payload_checksum));
    }
    let pairs = [
        ("Cascade Bluffs", Color::Blue, Color::Red),
        ("Fetid Heath", Color::White, Color::Black),
        ("Fire-Lit Thicket", Color::Red, Color::Green),
        ("Flooded Grove", Color::Green, Color::Blue),
        ("Graven Cairns", Color::Black, Color::Red),
        ("Mystic Gate", Color::White, Color::Blue),
        ("Rugged Prairie", Color::Red, Color::White),
        ("Sunken Ruins", Color::Blue, Color::Black),
        ("Twilight Mire", Color::Black, Color::Green),
        ("Wooded Bastion", Color::Green, Color::White),
    ];
    let mut scenarios: Vec<(&str, Vec<Color>, Option<ManaSymbol>, bool, bool)> = vec![];
    for (name, a, b) in pairs {
        for output in [vec![a, a], vec![a, b], vec![b, b]] {
            scenarios.push((name, output, Some(symbol(a)), false, false));
        }
        scenarios.push((name, vec![], None, true, false));
    }
    for color in [Color::Black, Color::Green] {
        scenarios.push(("Cadaverous Bloom", vec![color, color], None, false, true));
        scenarios.push(("Khalni Gem", vec![color, color], None, false, false));
    }
    scenarios.push((
        "Relic of Sauron",
        vec![Color::Blue, Color::Black],
        None,
        false,
        false,
    ));
    scenarios.push((
        "Relic of Sauron",
        vec![Color::Red, Color::Red],
        None,
        false,
        false,
    ));
    scenarios.push((
        "Dimir Signet",
        vec![Color::Blue, Color::Black],
        Some(ManaSymbol::Colorless),
        false,
        false,
    ));
    scenarios.push((
        "Shimmering Grotto",
        vec![Color::Green],
        Some(ManaSymbol::Colorless),
        false,
        false,
    ));
    scenarios.push(("Shimmering Grotto", vec![], None, true, false));
    let mut rows = vec![];
    for (name, colors, payment, colorless, fuel) in scenarios {
        let (d, checksum) = &defs[name];
        let expected = json!({"mana":expected_pool(&colors,colorless),"source_tapped":name!="Cadaverous Bloom","exiled_fuel":usize::from(fuel),"hand":0});
        let mut dm = Dm {
            requested: vec![],
            trace: vec![],
        };
        let scenario = json!({"requested_output":format!("{colors:?}"),"colorless_control":colorless,"initial_payment_mana":format!("{payment:?}"),"exile_fuel":fuel});
        let result = run(d, colors, payment, colorless, fuel, &mut dm);
        let (status, actual) = match result {
            Ok(actual) if actual == expected => ("expected_result_observed", actual),
            Ok(actual) => ("semantic_mismatch", actual),
            Err(e) => ("execution_or_fixture_error", json!({"error":e})),
        };
        rows.push(json!({"card":name,"scenario":scenario,"expected":expected,"actual":actual,"status":status,"outcome_category":if status=="semantic_mismatch"{"silent_wrong_result"}else if status=="expected_result_observed"{"expected_result_observed"}else{"unreviewed"},"artifact_checksum":checksum,"execution_trace":dm.trace}));
    }
    let report = json!({"scope":"All eleven canonical payloads matching repeated mana symbols in alternatives, plus numeric two-mana, fixed two-symbol, and one-mana boundary controls. Actual legal mana activations, costs, color choices, and resulting pool.","limitations":"Mana sources are established battlefield fixtures, not cast this turn; ETB and casting costs not exercised. Exactly enough initial mana is supplied for each filter cost. Mixed two-color requests cannot be entered in a one-color-count prompt: the DM submits the first requested color, and the complete resulting pool is compared to the printed valid output. Colorless controls select the first printed mana ability; colored checks select the last legal mana ability. No engine changes.","provenance":{"binary":std::env::current_exe().unwrap(),"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"seed":SEED,"unique_card_ids":true,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        root.join("repeated-mana-alternative-artifacts.json"),
        serde_json::to_string_pretty(&compilation).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("repeated-mana-alternative-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}
#[test]
#[ignore = "opt-in expected-result audit reporter; passing only means report generated"]
fn report_repeated_mana_alternatives() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate_report)
        .unwrap()
        .join()
        .unwrap();
}
