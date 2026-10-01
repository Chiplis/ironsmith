//! Opt-in sibling counter-mana audit with actual resource producers.
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
    target: Option<ObjectId>,
    colors: Vec<Color>,
    trace: Vec<Value>,
}
impl DecisionMaker for Dm {
    fn decide_targets(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        if let Some(id) = self.target {
            return vec![ironsmith::game_state::Target::Object(id)];
        }
        SelectFirstDecisionMaker.decide_targets(g, c)
    }
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
        let selected = if c.candidates.iter().any(|o| o.name == "Goat") {
            c.candidates
                .iter()
                .filter(|o| o.legal)
                .take(self.x as usize)
                .map(|o| o.id)
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
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
fn goats(g: &GameState) -> usize {
    g.battlefield
        .iter()
        .filter(|id| g.current_has_subtype(**id, ironsmith::Subtype::Goat))
        .count()
}
fn upkeep(g: &mut GameState, dm: &mut Dm) -> Result<(), String> {
    g.turn.turn_number += 1;
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::Beginning;
    g.turn.step = Some(ironsmith::Step::Upkeep);
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::generate_and_queue_step_triggers(g, &mut q);
    dm.trace.push(json!({"stage":"actual_own_upkeep"}));
    finish(g, &mut q, dm)
}
fn run(
    name: &str,
    defs: &HashMap<String, (CardDefinition, String)>,
    x: u32,
    resources: u32,
    control: bool,
    colors: &[Color],
    dm: &mut Dm,
) -> Result<Value, String> {
    let mut g = game();
    let d = &defs[name].0;
    let id = if d.card.card_types.contains(&CardType::Land) {
        let h = g.create_object_from_definition(d, alice(), Zone::Hand);
        let a = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::PlayLand{land_id,..}if *land_id==h))
            .ok_or("fixture no land play")?;
        perform(&mut g, a, dm)?;
        find(&g, name, Zone::Battlefield)?
    } else {
        cast(
            &mut g,
            d,
            if matches!(name, "Kyren Toy" | "Rasputin, the Oneiromancer") {
                3
            } else {
                4
            },
            dm,
        )?
    };
    let battery = name.ends_with("Mana Battery");
    let toy = name == "Kyren Toy";
    let pasture = name == "Springjack Pasture";
    let phase_source = matches!(
        name,
        "Bottomless Vault" | "Dwarven Hold" | "Hollow Trees" | "Icatian Store" | "Sand Silos"
    );
    let counter = if name == "Haruspex" {
        CounterType::PlusOnePlusOne
    } else if name == "Rasputin, the Oneiromancer" {
        CounterType::Dream
    } else if battery || toy {
        CounterType::Charge
    } else {
        CounterType::Storage
    };
    if name == "Rasputin, the Oneiromancer" {
        if g.counter_count(id, counter) != 2 {
            return Err("fixture Rasputin ETB must produce two dream counters".into());
        }
        dm.trace
            .push(json!({"stage":"actual_rasputin_etb","dream_counters":2}));
        if resources == 0 {
            next_turn(&mut g, id);
            dm.x = 2;
            activate(&mut g, id, 1, true, dm)?;
            if g.counter_count(id, counter) != 0 {
                return Err("fixture Rasputin emptying activation failed".into());
            }
        }
    }
    for n in 0..if name == "Rasputin, the Oneiromancer" {
        0
    } else {
        resources
    } {
        if name == "Haruspex" {
            let victim = CardDefinitionBuilder::new(CardId::new(), "Haruspex death witness")
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
                .build();
            let target = g.create_object_from_definition(&victim, alice(), Zone::Battlefield);
            dm.target = Some(target);
            g.turn.priority_player = Some(alice());
            let spell = g.create_object_from_definition(&defs["Murder"].0, alice(), Zone::Hand);
            let a = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
                .into_iter()
                .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
                .ok_or("fixture Murder cast absent")?;
            let before = g.player(alice()).unwrap().mana_pool.total();
            perform(&mut g, a, dm)?;
            let paid = before - g.player(alice()).unwrap().mana_pool.total();
            if paid != 3 {
                return Err("fixture Murder payment wrong".into());
            }
            dm.target = None;
            dm.trace
                .push(json!({"stage":"actual_paid_murder_death","mana_paid":paid}));
        } else if phase_source {
            if !g.is_tapped(id) {
                return Err("fixture upkeep storage land was not tapped".into());
            }
            upkeep(&mut g, dm)?;
        } else {
            next_turn(&mut g, id);
            let before = g.player(alice()).unwrap().mana_pool.total();
            activate(&mut g, id, if battery || toy { 0 } else { 1 }, false, dm)?;
            let paid = before - g.player(alice()).unwrap().mana_pool.total();
            let expected = if battery {
                2
            } else if toy || matches!(name, "Crucible of the Spirit Dragon" | "Mage-Ring Network") {
                1
            } else if pasture {
                4
            } else {
                0
            };
            if paid != expected {
                return Err(format!(
                    "fixture resource producer payment {paid}, expected {expected}"
                ));
            }
            dm.trace
                .push(json!({"stage":"actual_resource_activation","mana_paid":paid}));
        }
        let got = if pasture {
            goats(&g) as u32
        } else {
            g.counter_count(id, counter)
        };
        if got != n + 1 {
            return Err(format!(
                "fixture resource producer got {got}, expected {}",
                n + 1
            ));
        }
        dm.trace
            .push(json!({"stage":"resource_verified","count":got}));
    }
    next_turn(&mut g, id);
    g.player_mut(alice()).unwrap().mana_pool.empty();
    g.player_mut(alice()).unwrap().restricted_mana.clear();
    dm.x = x;
    dm.colors = colors.to_vec();
    let idx = if control {
        0
    } else if battery || toy || matches!(name, "Haruspex" | "Rasputin, the Oneiromancer") {
        1
    } else if phase_source {
        3
    } else {
        2
    };
    dm.trace.push(json!({"stage":"before_mana_activation","requested_x":x,"resources":resources,"mana":pool(&g)}));
    let legal = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state");
    let offered=legal.iter().any(|a|matches!(a,LegalAction::ActivateManaAbility{source,ability_index}if *source==id&&*ability_index==idx));
    dm.trace
        .push(json!({"stage":"mana_legality","offered":offered,"actions":format!("{legal:?}")}));
    let result = if offered {
        activate(&mut g, id, idx, true, dm)
    } else {
        Ok(())
    };
    let actual = json!({"legal_mana_activation_present":offered,"mana":pool(&g),"counters":g.counter_count(id,counter),"goats":goats(&g),"life":g.player(alice()).unwrap().life,"source_tapped":g.is_tapped(id),"restricted_mana":g.player(alice()).unwrap().restricted_mana.len()});
    dm.trace
        .push(json!({"stage":"after_activation_attempt","observed":actual}));
    result?;
    Ok(actual)
}
fn generate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reports/runtime-audit");
    let input: Value = serde_json::from_slice(
        &std::fs::read(root.join("mana-counter-sibling-inputs.json")).unwrap(),
    )
    .unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    let mut compile_rows = vec![];
    for (name, p) in input["cards"].as_object().unwrap() {
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p["parse_name"].as_str().unwrap_or(name),
        );
        let (a, d) = match ironsmith_registry::compile_builder_to_artifact(
            b,
            p["parse_input"].as_str().unwrap(),
            false,
        ) {
            Ok(v) => v,
            Err(e) => {
                compile_rows.push(json!({"card":name,"scenario":"strict canonical compilation before scenario","status":"compile_failed","outcome_category":"compile_unavailable","actual":{"error":e.to_string()},"expected":{"canonical_definition_available":true}}));
                continue;
            }
        };
        artifacts.push(json!({"card":name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
        defs.insert(name.clone(), (d, a.payload_checksum));
    }
    let mut rows = compile_rows;
    for name in input["cards"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|n| n.as_str() != "Murder" && defs.contains_key(n.as_str()))
    {
        let color = match name.as_str() {
            "Black Mana Battery" | "Bottomless Vault" | "Subterranean Hangar" => Some(Color::Black),
            "Blue Mana Battery" | "Sand Silos" | "Saprazzan Cove" => Some(Color::Blue),
            "Green Mana Battery" | "Hollow Trees" | "Rushwood Grove" => Some(Color::Green),
            "Red Mana Battery" | "Dwarven Hold" | "Mercadian Bazaar" => Some(Color::Red),
            "White Mana Battery" | "Icatian Store" | "Fountain of Cho" => Some(Color::White),
            "Crucible of the Spirit Dragon" | "Springjack Pasture" | "Haruspex" => {
                Some(Color::Blue)
            }
            _ => None,
        };
        let colors = if name == "Crucible of the Spirit Dragon" {
            vec![Color::Blue, Color::Red]
        } else {
            vec![color.unwrap_or(Color::Blue)]
        };
        let mut cases = if name == "Rasputin, the Oneiromancer" {
            vec![(1, 2, false), (2, 2, false), (0, 0, false)]
        } else {
            vec![(0, 2, false), (1, 2, false), (2, 2, false), (0, 0, false)]
        };
        if matches!(
            name.as_str(),
            "Crucible of the Spirit Dragon" | "Mage-Ring Network" | "Springjack Pasture"
        ) {
            cases.push((0, 0, true));
        }
        for (x, resources, control) in cases {
            eprintln!("MANA_COUNTER_CASE {name} x={x} resources={resources} control={control}");
            let mut dm = Dm {
                x: 0,
                target: None,
                colors: colors.clone(),
                trace: vec![],
            };
            let result = run(name, &defs, x, resources, control, &colors, &mut dm);
            let pasture = name == "Springjack Pasture";
            let bonus = u32::from(name.ends_with("Mana Battery") || name == "Kyren Toy");
            let output = if control { 0 } else { x + bonus };
            let expected_colors = if color.is_some() {
                (0..output as usize)
                    .map(|i| colors[i.min(colors.len() - 1)])
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            let unavailable = name == "Rasputin, the Oneiromancer" && resources == 0;
            let expected = json!({"legal_mana_activation_present":!unavailable,"mana":expected_pool(&expected_colors,if control{1}else if color.is_none(){output}else{0}),"counters":if pasture{0}else{resources-x},"goats":if pasture{resources-x}else{0},"life":20+if pasture{x}else{0},"source_tapped":!unavailable,"restricted_mana":if name=="Crucible of the Spirit Dragon"&&!control{x}else{0}});
            let (status, actual) = match result {
                Ok(a) if a == expected => ("expected_result_observed", a),
                Ok(a) => ("semantic_mismatch", a),
                Err(e) if e.contains("X value not set") || e.contains("Failed to pay cost") => {
                    ("action_or_choice_failed", json!({"error":e}))
                }
                Err(e) => ("execution_or_fixture_error", json!({"error":e})),
            };
            rows.push(json!({"card":name,"scenario":{"requested_x":x,"resources_from_actual_producers":resources,"fixed_mana_control":control},"expected":expected,"actual":actual,"status":status,"outcome_category":if status=="action_or_choice_failed"{"runtime_exception"}else if status=="semantic_mismatch"{"silent_wrong_result"}else{status},"artifact_checksum":defs[name].1,"execution_trace":dm.trace}));
        }
    }
    let report = json!({"scope":"Twenty-one other counter/removal/sacrifice mana-source candidates, actual paid casts and land plays, actual activated or upkeep resource producers, zero/positive removal and fixed-mana controls.","limitations":"Mana is seeded for source and resource-producer costs. Untaps and later turns are explicitly positioned; upkeep events use the normal producer. Requested X is submitted only when a normal decision asks. Crucible checks restricted-mana count but does not test spending restrictions. No engine changes.","provenance":{"binary":std::env::current_exe().unwrap(),"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"seed":SEED,"unique_card_ids":true,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        root.join("mana-counter-sibling-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("mana-counter-sibling-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical expected-result audit reporter"]
fn report_mana_counter_sibling_cases() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}
