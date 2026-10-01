//! Opt-in canonical mana-X activation evidence. Passing means a report was generated.
use ironsmith::CounterType;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::color::Color;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, ColorsContext, DecisionContext, NumberContext, SelectObjectsContext,
    SelectOptionsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, Zone};
use serde_json::{Value, json};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
struct Dm {
    x: u32,
    colors: Vec<Color>,
    trace: Vec<Value>,
}
impl DecisionMaker for Dm {
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        let selected = if c.is_x_value || c.description.to_lowercase().contains("counter") {
            self.x
        } else {
            c.min
        };
        self.trace.push(json!({"choice":"number","context":format!("{c:?}"),"requested_x":self.x,"selected":selected}));
        selected
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"selected":true}));
        true
    }
    fn decide_colors(&mut self, _: &GameState, c: &ColorsContext) -> Vec<Color> {
        let selected = (0..c.count as usize)
            .map(|i| self.colors[i.min(self.colors.len() - 1)])
            .collect::<Vec<_>>();
        self.trace.push(json!({"choice":"colors","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = SelectFirstDecisionMaker.decide_options(g, c);
        self.trace
            .push(json!({"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = SelectFirstDecisionMaker.decide_objects(g, c);
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(SEED);
    g.turn.turn_number = 3;
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    for s in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(alice()).unwrap().mana_pool.add(s, 12);
    }
    g
}
fn pool(g: &GameState) -> Value {
    let p = &g.player(alice()).unwrap().mana_pool;
    json!({"white":p.white,"blue":p.blue,"black":p.black,"red":p.red,"green":p.green,"colorless":p.colorless})
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
fn expected_pool(colors: &[Color], colorless: u32) -> Value {
    let mut g = game();
    g.player_mut(alice()).unwrap().mana_pool.empty();
    for c in colors {
        g.player_mut(alice()).unwrap().mana_pool.add(symbol(*c), 1);
    }
    g.player_mut(alice())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, colorless);
    pool(&g)
}
fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    for _ in 0..32 {
        check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        if g.stack.is_empty() {
            return Ok(());
        }
        state.reset_for_new_priority_window(g);
        for _ in 0..g.players_in_game() {
            apply_priority_response_with_dm(
                g,
                q,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Err("fixture resolution budget exceeded".into())
}
fn perform(g: &mut GameState, a: LegalAction, dm: &mut Dm) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace.push(
        json!({"stage":"actual_legal_action","action":format!("{a:?}"),"mana_before":pool(g)}),
    );
    let mut result = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(a),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        match result {
            GameProgress::NeedsDecisionCtx(ref c) if !matches!(c, DecisionContext::Priority(_)) => {
                result = apply_decision_context_with_dm(g, &mut q, &mut state, c, dm)
                    .map_err(|e| e.to_string())?;
            }
            _ => return finish(g, &mut q, dm),
        }
    }
    Err("fixture announcement budget exceeded".into())
}
fn find(g: &GameState, name: &str, zone: Zone) -> Result<ObjectId, String> {
    g.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name == name && o.zone == zone)
        .map(|o| o.id)
        .ok_or(format!("fixture missing {name} in {zone:?}"))
}
fn count(g: &GameState, name: &str, zone: Zone) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.name == name && o.zone == zone)
        .count()
}
fn cast(
    g: &mut GameState,
    d: &CardDefinition,
    expected: u32,
    dm: &mut Dm,
) -> Result<ObjectId, String> {
    g.turn.priority_player = Some(alice());
    let id = g.create_object_from_definition(d, alice(), Zone::Hand);
    let a = compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("fixture cast action absent")?;
    let before = g.player(alice()).unwrap().mana_pool.total();
    perform(g, a, dm)?;
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
    if paid != expected {
        return Err(format!(
            "fixture {} cast paid {paid}, expected {expected}",
            d.name()
        ));
    }
    dm.trace
        .push(json!({"stage":"actual_paid_cast","card":d.name(),"mana_paid":paid}));
    find(g, d.name(), Zone::Battlefield)
}
fn activate(
    g: &mut GameState,
    id: ObjectId,
    index: usize,
    mana: bool,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
    let actions = compute_legal_actions(g, alice()).expect("fixture has complete replacement state");
    let a = actions
        .iter()
        .find(|a| match a {
            LegalAction::ActivateManaAbility {
                source,
                ability_index,
            } => mana && *source == id && *ability_index == index,
            LegalAction::ActivateAbility {
                source,
                ability_index,
            } => !mana && *source == id && *ability_index == index,
            _ => false,
        })
        .cloned()
        .ok_or_else(|| format!("fixture intended action absent; actions={actions:?}"))?;
    perform(g, a, dm)
}
fn next_turn(g: &mut GameState, id: ObjectId) {
    g.turn.turn_number += 1;
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.untap(id);
    g.remove_summoning_sickness(id);
}
fn run(
    name: &str,
    defs: &HashMap<String, (CardDefinition, String)>,
    x: u32,
    counters: u32,
    control: bool,
    colors: &[Color],
    dm: &mut Dm,
) -> Result<Value, String> {
    let mut g = game();
    let d = &defs[name].0;
    let id;
    if d.card.card_types.contains(&CardType::Land) {
        id = g.create_object_from_definition(d, alice(), Zone::Hand);
        let a = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::PlayLand{land_id,..}if *land_id==id))
            .ok_or("fixture no land play")?;
        perform(&mut g, a, dm)?;
        let live = find(&g, name, Zone::Battlefield)?;
        for n in 0..counters {
            next_turn(&mut g, live);
            let before = g.player(alice()).unwrap().mana_pool.total();
            activate(&mut g, live, 1, false, dm)?;
            let paid = before - g.player(alice()).unwrap().mana_pool.total();
            if paid != 1 || g.counter_count(live, CounterType::Storage) != n + 1 {
                return Err("fixture storage producer failed".into());
            }
            dm.trace
                .push(json!({"stage":"actual_storage_added","mana_paid":paid,"counters":n+1}));
        }
    } else {
        let price = if name == "Petalmane Baku" { 2 } else { 1 };
        id = cast(&mut g, d, price, dm)?;
        if name == "Wizard's Rockets" && !g.is_tapped(id) {
            return Err("fixture Rockets did not enter tapped".into());
        }
        if name == "Petalmane Baku" {
            for n in 0..counters {
                cast(&mut g, &defs["Lantern Kami"].0, 1, dm)?;
                if g.counter_count(id, CounterType::Ki) != n + 1 {
                    return Err("fixture Spirit cast did not produce ki counter".into());
                }
                dm.trace
                    .push(json!({"stage":"actual_spirit_cast_ki","counters":n+1}));
            }
        }
    }
    let id = find(&g, name, Zone::Battlefield)?;
    next_turn(&mut g, id);
    let neutral = CardDefinitionBuilder::new(CardId::new(), "X mana draw witness")
        .card_types(vec![CardType::Sorcery])
        .build();
    g.create_object_from_definition(&neutral, alice(), Zone::Library);
    g.player_mut(alice()).unwrap().mana_pool.empty();
    let payment = if control {
        0
    } else if name == "Wizard's Rockets" {
        x
    } else {
        1
    };
    g.player_mut(alice())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, payment);
    dm.x = x;
    dm.colors = colors.to_vec();
    let counter = if name == "Petalmane Baku" {
        CounterType::Ki
    } else {
        CounterType::Storage
    };
    dm.trace.push(json!({"stage":"before_mana_activation","requested_x":x,"counters":g.counter_count(id,counter),"mana":pool(&g),"source_tapped":g.is_tapped(id)}));
    let idx = if control {
        0
    } else if d.card.card_types.contains(&CardType::Land) {
        2
    } else {
        1
    };
    let result = activate(&mut g, id, idx, true, dm);
    let actual = json!({"mana":pool(&g),"counters":g.counter_count(id,counter),"source_tapped":g.is_tapped(id),"source_battlefield":count(&g,name,Zone::Battlefield),"source_graveyard":count(&g,name,Zone::Graveyard),"drawn":count(&g,"X mana draw witness",Zone::Hand)});
    dm.trace
        .push(json!({"stage":"after_activation_attempt","observed":actual}));
    result?;
    Ok(actual)
}
fn generate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reports/runtime-audit");
    let input: Value = serde_json::from_slice(
        &std::fs::read(root.join("mana-x-announcement-inputs.json")).unwrap(),
    )
    .unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    for (name, p) in input["cards"].as_object().unwrap() {
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
        artifacts.push(json!({"card":name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        defs.insert(name.clone(), (d, a.payload_checksum));
    }
    let mut cases = vec![];
    for (name, a, b) in [
        ("Calciform Pools", Color::White, Color::Blue),
        ("Dreadship Reef", Color::Blue, Color::Black),
        ("Fungal Reaches", Color::Red, Color::Green),
        ("Molten Slagheap", Color::Black, Color::Red),
        ("Saltcrusted Steppe", Color::Green, Color::White),
    ] {
        for x in 0..=2 {
            cases.push((name, x, 2, false, vec![a, b]));
        }
        cases.push((name, 0, 0, false, vec![a]));
        cases.push((name, 0, 0, true, vec![a]));
    }
    for x in 0..=2 {
        cases.push(("Petalmane Baku", x, 2, false, vec![Color::Blue]));
        cases.push((
            "Wizard's Rockets",
            x,
            0,
            false,
            vec![Color::Blue, Color::Red],
        ));
    }
    cases.push(("Petalmane Baku", 0, 0, false, vec![Color::Blue]));
    cases.push(("Llanowar Elves", 0, 0, true, vec![Color::Green]));
    let mut rows = vec![];
    for (name, x, counters, control, colors) in cases {
        eprintln!("MANA_X_CASE {name} x={x} counters={counters} control={control}");
        let mut dm = Dm {
            x: 0,
            colors: colors.clone(),
            trace: vec![],
        };
        let result = run(name, &defs, x, counters, control, &colors, &mut dm);
        let output = if control && name == "Llanowar Elves" {
            vec![Color::Green]
        } else if control {
            vec![]
        } else {
            (0..x as usize)
                .map(|i| colors[i.min(colors.len() - 1)])
                .collect()
        };
        let rockets = name == "Wizard's Rockets";
        let expected = json!({"mana":expected_pool(&output,u32::from(control&&name!="Llanowar Elves")),"counters":if rockets {0}else{counters-x},"source_tapped":control,"source_battlefield":usize::from(!rockets),"source_graveyard":usize::from(rockets),"drawn":usize::from(rockets)});
        let (status, actual) = match result {
            Ok(a) if a == expected => ("expected_result_observed", a),
            Ok(a) => ("semantic_mismatch", a),
            Err(e) if e.contains("X value not set") => {
                ("action_or_choice_failed", json!({"error":e}))
            }
            Err(e) => ("execution_or_fixture_error", json!({"error":e})),
        };
        rows.push(json!({"card":name,"scenario":{"requested_x":x,"counters_from_actual_producers":counters,"fixed_mana_control":control,"requested_colors":format!("{colors:?}")},"expected":expected,"actual":actual,"status":status,"outcome_category":if status=="action_or_choice_failed"{"runtime_exception"}else if status=="semantic_mismatch"{"silent_wrong_result"}else{status},"artifact_checksum":defs[name].1,"execution_trace":dm.trace}));
    }
    let report = json!({"scope":"All seven unbound-X-at-announcement candidates: real storage-counter activations, paid Spirit casts for ki counters, actual artifact cast and land play; zero/positive X and fixed-mana controls.","limitations":"Mana and neutral library cards seeded. Later main phases and untaps are positioned explicitly between paid counter activations, not a complete turn simulation. X is supplied only when a normal number decision asks; requested_x does not claim the engine exposed that choice. Activation errors stop resolution; after-error state is diagnostic, not completed cost payment. No engine changes.","provenance":{"binary":std::env::current_exe().unwrap(),"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"seed":SEED,"unique_card_ids":true,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        root.join("mana-x-announcement-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("mana-x-announcement-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical expected-result audit reporter"]
fn report_mana_x_announcement_cases() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
