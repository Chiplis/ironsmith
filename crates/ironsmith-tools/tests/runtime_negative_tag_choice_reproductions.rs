//! Negative-tag non-target choosing and copy-route controls from frozen canonical inputs.
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
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
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
    kicks: u32,
    targets: Vec<Target>,
    accept: bool,
    preferred_object: Option<String>,
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
        let selected = if let Some(candidate) = c
            .candidates
            .iter()
            .find(|o| o.legal && self.preferred_object.as_ref().is_some_and(|s| o.name == *s))
        {
            vec![candidate.id]
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(json!({"objects":c.description,"player":c.player.index(),"offered":c.candidates.iter().map(|v|json!({"id":v.id.0,"legal":v.legal,"controller":g.controller_of_id(v.id).map(|p|p.index())})).collect::<Vec<_>>(),"selected":selected.iter().map(|v|v.0).collect::<Vec<_>>()}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        if c.description.starts_with("Choose mode")
            && c.options.iter().any(|o| o.index == 1 && o.legal)
        {
            self.trace
                .push(json!({"mode_context":format!("{c:?}"),"selected":[1]}));
            return vec![1];
        }
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
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        self.trace.push(json!({"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
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
                    &PriorityResponse::OptionalCosts(if dm.kicks > 0 {
                        vec![(0, dm.kicks)]
                    } else {
                        vec![]
                    }),
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
        kicks: 0,
        targets: vec![],
        accept,
        preferred_object: None,
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
fn card(name: &str, kind: CardType) -> CardDefinition {
    let b = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![kind]);
    if kind == CardType::Creature {
        b.power_toughness(PowerToughness::fixed(2, 6)).build()
    } else {
        b.build()
    }
}

fn characteristics(g: &GameState, id: ObjectId) -> Value {
    json!({"name":g.current_name(id),"power":g.calculated_power(id),"toughness":g.calculated_toughness(id)})
}
fn tokens(g: &GameState) -> Vec<Value> {
    let mut out = g
        .objects_in_deterministic_order()
        .into_iter()
        .filter(|o| {
            o.zone == Zone::Battlefield && matches!(o.kind, ironsmith::object::ObjectKind::Token)
        })
        .map(|o| characteristics(g, o.id))
        .collect::<Vec<_>>();
    out.sort_by_key(|v| v["name"].as_str().unwrap_or("").to_string());
    out
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    n: usize,
    accept: bool,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup_players(2);
    for player in [alice(), PlayerId(1)] {
        for _ in 0..12 {
            g.create_object_from_definition(
                &card("Library witness", CardType::Artifact),
                player,
                Zone::Library,
            );
        }
    }
    let mut d = dm(0, accept);
    let mut q = TriggerQueue::new();
    let (mut paid, mut setup_paid) = (0, 0);
    let (mut actual, mut expected) = (Value::Null, Value::Null);
    if name == "Brudiclad, Telchor Engineer" {
        if n > 0 {
            let (mut qq, p) = cast(&mut g, &defs["Krenko's Command"], alice(), &mut d)?;
            setup_paid += p;
            resolve(&mut g, &mut qq, &mut d)?;
        }
        let source = established(&mut g, &defs[name], alice(), 6)?;
        setup_paid += 6;
        d.preferred_object = Some("Goblin".into());
        ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        let event = ironsmith::triggers::generate_step_trigger_events(&g)
            .ok_or("missing beginning-combat event")?;
        let triggers = ironsmith::triggers::check_triggers(&g, &event);
        if triggers.len() != 1 {
            return Err(format!(
                "expected one Brudiclad trigger; got {}",
                triggers.len()
            ));
        }
        for t in triggers {
            q.add(t);
        }
        let error = resolve(&mut g, &mut q, &mut d).err();
        let mut want = vec![];
        if n > 0 {
            for _ in 0..2 {
                want.push(json!({"name":"Goblin","power":1,"toughness":1}));
            }
        }
        want.push(if n > 0 && accept {
            json!({"name":"Goblin","power":1,"toughness":1})
        } else {
            json!({"name":"Phyrexian Myr","power":2,"toughness":1})
        });
        want.sort_by_key(|v| v["name"].as_str().unwrap().to_string());
        actual = json!({"resolution_error":error,"tokens":tokens(&g),"source":characteristics(&g,source)});
        expected = json!({"resolution_error":null,"tokens":want,"source":{"name":name,"power":4,"toughness":4}});
    } else if name == "Loki, Lord of Misrule" || name == "Sakashima's Will" {
        let source = if name == "Loki, Lord of Misrule" {
            setup_paid += 4;
            Some(established(&mut g, &defs[name], alice(), 4)?)
        } else {
            None
        };
        let chosen = if name == "Sakashima's Will" || n > 0 {
            setup_paid += 2;
            established(&mut g, &defs["Grizzly Bears"], alice(), 2)?
        } else {
            source.unwrap()
        };
        let other = if n > 0 {
            setup_paid += 1;
            Some(established(&mut g, &defs["Typhoid Rats"], alice(), 1)?)
        } else {
            None
        };
        d.preferred_object = Some("Grizzly Bears".into());
        if let Some(source) = source {
            d.targets = vec![Target::Object(source)];
            let (mut qq, p) = cast(&mut g, &defs["Crimson Wisps"], alice(), &mut d)?;
            setup_paid += p;
            resolve(&mut g, &mut qq, &mut d)?;
            d.targets = vec![Target::Object(chosen)];
            let (qq, p) = perform(&mut g, source, Some(0), &mut d)?;
            q = qq;
            paid = p;
        } else {
            let (qq, p) = cast(&mut g, &defs[name], alice(), &mut d)?;
            q = qq;
            paid = p;
        }
        let error = resolve(&mut g, &mut q, &mut d).err();
        let ids = source
            .into_iter()
            .chain(std::iter::once(chosen))
            .chain(other)
            .collect::<std::collections::BTreeSet<_>>();
        let states = ids
            .iter()
            .map(|id| characteristics(&g, *id))
            .collect::<Vec<_>>();
        let want = if n == 0 && name == "Loki, Lord of Misrule" {
            json!({"name":name,"power":3,"toughness":4})
        } else {
            json!({"name":"Grizzly Bears","power":2,"toughness":2})
        };
        actual = json!({"resolution_error":error,"paid":paid,"states":states,"source_tapped":source.map(|s|g.is_tapped(s))});
        expected = json!({"resolution_error":null,"paid":if source.is_some(){1}else{4},"states":ids.iter().map(|_|want.clone()).collect::<Vec<_>>(),"source_tapped":source.map(|_|true)});
    } else if name == "Watchers of the Dead" {
        let source = established(&mut g, &defs[name], alice(), 2)?;
        setup_paid += 2;
        for _ in 0..n {
            d.targets = vec![Target::Player(alice())];
            let (mut qq, p) = cast(&mut g, &defs["Shock"], PlayerId(1), &mut d)?;
            setup_paid += p;
            resolve(&mut g, &mut qq, &mut d)?;
        }
        if g.player(PlayerId(1)).unwrap().graveyard.len() != n {
            return Err("graveyard producer did not leave expected Shock count".into());
        }
        d.targets.clear();
        let (mut qq, p) = perform(&mut g, source, Some(0), &mut d)?;
        paid = p;
        let error = resolve(&mut g, &mut qq, &mut d).err();
        actual = json!({"resolution_error":error,"paid":paid,"bob_graveyard":g.player(PlayerId(1)).unwrap().graveyard.len(),"exiled_shocks":count(&g,"Shock",Zone::Exile),"exiled_watchers":count(&g,name,Zone::Exile)});
        expected = json!({"resolution_error":null,"paid":0,"bob_graveyard":n.min(2),"exiled_shocks":n.saturating_sub(2),"exiled_watchers":1});
    }
    Ok((
        actual,
        expected,
        json!({"setup_mana_paid":setup_paid,"action_mana_paid":paid,"decisions":d.trace,"stack":g.stack.len()}),
    ))
}
fn normalized_definition(value: &Value, path: &str, ignored: &mut Vec<String>) -> Value {
    match value {
        Value::Object(o) => {
            let mut out = serde_json::Map::new();
            for (k, v) in o {
                if k == "id"
                    && path.ends_with("/card")
                    && o.contains_key("card_types")
                    && o.contains_key("name")
                {
                    ignored.push(format!("{path}/id"));
                    continue;
                }
                out.insert(
                    k.clone(),
                    normalized_definition(v, &format!("{path}/{k}"), ignored),
                );
            }
            Value::Object(out)
        }
        Value::Array(a) => Value::Array(
            a.iter()
                .enumerate()
                .map(|(i, v)| normalized_definition(v, &format!("{path}/{i}"), ignored))
                .collect(),
        ),
        _ => value.clone(),
    }
}
#[test]
#[ignore = "strict paid negative-tag route probes"]
fn report_negative_tag_choices() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/negative-tag-choice-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_negative_tag_choice_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut compilation = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let name = p["name"].as_str().unwrap();
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p["parse_name"].as_str().unwrap_or(name),
        );
        match ironsmith_registry::compile_builder_to_artifact(
            b,
            p["parse_input"].as_str().unwrap(),
            false,
        ) {
            Ok((a, d)) => {
                let definition = serde_json::to_value(&a.payload.definition).unwrap();
                let (mut ignored, mut ignored_frozen) = (vec![], vec![]);
                let parity = normalized_definition(&definition, "/definition", &mut ignored)
                    == normalized_definition(
                        &p["frozen_definition"],
                        "/definition",
                        &mut ignored_frozen,
                    );
                compilation.push(json!({"card":name,"artifact_checksum":a.payload_checksum,"frozen_artifact_checksum":p["frozen_artifact_checksum"],"definition_matches_frozen_except_unique_card_ids":parity,"ignored_card_id_paths":ignored,"definition":definition}));
                defs.insert(name.to_owned(), d);
            }
            Err(e) => compilation.push(json!({"card":name,"strict_compile_error":e.to_string()})),
        }
    }
    let mut rows = vec![];
    for name in [
        "Brudiclad, Telchor Engineer",
        "Loki, Lord of Misrule",
        "Sakashima's Will",
        "Watchers of the Dead",
    ] {
        for n in if name == "Watchers of the Dead" {
            vec![0, 1, 2, 4]
        } else {
            vec![0, 2]
        } {
            for accept in if name == "Brudiclad, Telchor Engineer" {
                vec![false, true]
            } else {
                vec![true]
            } {
                println!("stage: {name} resources{n} accept{accept}");
                let (status, actual, expected, diagnostics) = match trial(&defs, name, n, accept) {
                    Ok((v, e, t)) => {
                        if v == e {
                            ("passed", v, e, t)
                        } else {
                            ("semantic_mismatch", v, e, t)
                        }
                    }
                    Err(e) => (
                        "execution_or_fixture_error",
                        json!({"error":e}),
                        Value::Null,
                        Value::Null,
                    ),
                };
                rows.push(json!({"card":name,"scenario":{"resources":n,"accept":accept},"status":status,"actual":actual,"expected":expected,"diagnostics":diagnostics}));
            }
        }
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Frozen canonical legal paid copies/negative-tag graveyard chooser. Brudiclad actual beginning-combat event and paid Krenko token creation, Loki paid haste producer then blue/tap activation, Sakashima explicitly chosen second mode, Watchers opponent graveyard cards from actually paid Shock spells and actual source-exile activation cost. Wicked Guardian is independently reproduced by sibling report; not rerun here.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/negative-tag-choice-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
