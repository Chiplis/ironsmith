//! Paid Snow Devil / Intrepid Ace static combat predicate outcome probes.
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
            .find(|o| o.legal && o.name == "Cost witness")
        {
            vec![candidate.id]
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
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

fn paid(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    actor: PlayerId,
    cost: u32,
    d: &mut ProbeDm,
    trace: &mut Vec<Value>,
) -> Result<Option<ObjectId>, String> {
    let (mut q, p) = cast(g, &defs[name], actor, d)?;
    if p != cost {
        return Err(format!("{name} paid{p} expected{cost}"));
    }
    resolve(g, &mut q, d)?;
    let id = g.battlefield.iter().copied().find(|id| {
        g.object(*id)
            .is_some_and(|o| o.name == name && o.owner == actor)
    });
    trace.push(json!({"stage":"paid_cast","card":name,"player":actor.index(),"mana_paid":p,"battlefield_id":id.map(|id|id.0)}));
    Ok(id)
}
fn play_snow(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    actor: PlayerId,
    d: &mut ProbeDm,
    trace: &mut Vec<Value>,
) -> Result<(), String> {
    let id = g.create_object_from_definition(&defs["Snow-Covered Island"], actor, Zone::Hand);
    g.turn.priority_player = Some(actor);
    let a = compute_legal_actions(g, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==id))
        .ok_or("snow land play unavailable")?;
    let mut q = TriggerQueue::new();
    let mut st = PriorityLoopState::new(g.players_in_game());
    apply_priority_response_with_dm(g, &mut q, &mut st, &PriorityResponse::PriorityAction(a), d)
        .map_err(|e| e.to_string())?;
    resolve(g, &mut q, d)?;
    let id = g
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            g.object(*id)
                .is_some_and(|o| o.name == "Snow-Covered Island" && o.owner == actor)
        })
        .ok_or("played snow land missing")?;
    trace.push(json!({"stage":"actual_land_play","player":actor.index(),"object":id.0,"supertypes":format!("{:?}",g.current_supertypes(id))}));
    Ok(())
}
fn next_main(g: &mut GameState, d: &mut ProbeDm, trace: &mut Vec<Value>) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    let mut q = TriggerQueue::new();
    if let Some(e) = ironsmith::triggers::generate_step_trigger_events(g) {
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t);
        }
    }
    resolve(g, &mut q, d)?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    let events = ironsmith::turn::execute_draw_step_with(g, d);
    for e in events {
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t);
        }
    }
    resolve(g, &mut q, d)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(g.turn.active_player);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(g.turn.active_player)
            .unwrap()
            .mana_pool
            .add(color, 20);
    }
    trace.push(json!({"stage":"real_next_turn_main","active":g.turn.active_player.index(),"phase":format!("{:?}",g.turn.phase)}));
    Ok(())
}
fn characteristics(g: &GameState, id: ObjectId) -> Value {
    use ironsmith::static_abilities::StaticAbilityId;
    json!({"power":g.calculated_power(id),"toughness":g.calculated_toughness(id),"flying":g.object_has_static_ability_id(id,StaticAbilityId::Flying),"first_strike":g.object_has_static_ability_id(id,StaticAbilityId::FirstStrike),"attacking":g.combat.as_ref().is_some_and(|c|ironsmith::combat_state::is_attacking(c,id)),"blocking":g.combat.as_ref().is_some_and(|c|ironsmith::combat_state::is_blocking(c,id))})
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    role: &str,
    snow: usize,
) -> Result<(Value, Value, Value), String> {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration};
    let mut g = setup_players(2);
    for p in [alice(), PlayerId(1)] {
        for _ in 0..6 {
            g.create_object_from_definition(
                &card("Library witness", CardType::Artifact),
                p,
                Zone::Library,
            );
        }
    }
    let mut d = dm(0, true);
    let mut trace = vec![];
    let host_name = if name == "Snow Devil" {
        "Grizzly Bears"
    } else {
        name
    };
    let host = paid(
        &mut g,
        defs,
        host_name,
        alice(),
        if name == "Snow Devil" { 2 } else { 1 },
        &mut d,
        &mut trace,
    )?
    .ok_or("host missing")?;
    if name == "Snow Devil" {
        d.targets = vec![Target::Object(host)];
        let aura =
            paid(&mut g, defs, name, alice(), 2, &mut d, &mut trace)?.ok_or("Aura missing")?;
        if g.object(aura).unwrap().attached_to
            != Some(ironsmith::object::AttachmentTarget::Object(host))
        {
            return Err("Snow Devil wrong attachment".into());
        }
    }
    if snow == 1 {
        play_snow(&mut g, defs, alice(), &mut d, &mut trace)?;
    }
    let before = characteristics(&g, host);
    let mut opponent = None;
    let mut combat = CombatState::default();
    let mut q = TriggerQueue::new();
    if role != "neither" {
        let (attacker, defender, actor) = if role == "attacking" {
            (host, PlayerId(1), alice())
        } else {
            next_main(&mut g, &mut d, &mut trace)?;
            if snow == 2 {
                play_snow(&mut g, defs, PlayerId(1), &mut d, &mut trace)?;
            }
            let enemy_name = if name == "Snow Devil" {
                "Grizzly Bears"
            } else {
                "Hill Giant"
            };
            let enemy = paid(
                &mut g,
                defs,
                enemy_name,
                PlayerId(1),
                if name == "Snow Devil" { 2 } else { 4 },
                &mut d,
                &mut trace,
            )?
            .ok_or("opponent creature missing")?;
            opponent = Some(enemy);
            (enemy, alice(), PlayerId(1))
        };
        d.targets = vec![Target::Object(attacker)];
        paid(&mut g, defs, "Crimson Wisps", actor, 1, &mut d, &mut trace)?;
        ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
        let attacks = [AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(defender),
        }];
        ironsmith::game_loop::apply_attacker_declarations_with_dm(
            &mut g,
            &mut combat,
            &mut q,
            &attacks,
            &mut d,
        )
        .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        resolve(&mut g, &mut q, &mut d)?;
        ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
        let blocks = if role == "blocking" {
            vec![BlockerDeclaration {
                blocker: host,
                blocking: attacker,
            }]
        } else {
            vec![]
        };
        ironsmith::game_loop::apply_blocker_declarations(
            &mut g,
            &mut combat,
            &mut q,
            &blocks,
            defender,
        )
        .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        resolve(&mut g, &mut q, &mut d)?;
        trace.push(json!({"stage":"legal_combat_declarations","attacks":format!("{attacks:?}"),"blocks":format!("{blocks:?}"),"active":actor.index()}));
    }
    let during = characteristics(&g, host);
    if role != "neither" {
        ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
        for first in [true, false] {
            let events = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
                &mut g, &combat, first, &mut d,
            )
            .map_err(|e| format!("combat damage:{e:?}"))?;
            trace.push(json!({"stage":"actual_combat_damage","first_strike_step":first,"events":format!("{events:?}")}));
            resolve(&mut g, &mut q, &mut d)?;
        }
    }
    let flying = name == "Snow Devil";
    let first = flying && role == "blocking" && snow == 1;
    let ace = name == "Intrepid Ace";
    let expected_before = json!({"power":if ace{4}else{2},"toughness":if ace{1}else{2},"flying":flying,"first_strike":false,"attacking":false,"blocking":false});
    let expected_during = json!({"power":if ace&&["attacking","blocking"].contains(&role){2}else if ace{4}else{2},"toughness":if ace{1}else{2},"flying":flying,"first_strike":first,"attacking":role=="attacking","blocking":role=="blocking"});
    let actual = json!({"before":before,"during":during,"host_survives":g.object(host).is_some_and(|o|o.zone==Zone::Battlefield),"opponent_survives":opponent.map(|id|g.object(id).is_some_and(|o|o.zone==Zone::Battlefield)),"life":[g.player(alice()).unwrap().life,g.player(PlayerId(1)).unwrap().life]});
    let expected = json!({"before":expected_before,"during":expected_during,"host_survives":role!="blocking"||first,"opponent_survives":opponent.map(|_|role!="blocking"||ace),"life":[if role=="neither_in_combat"{17}else{20},if role=="attacking"{18}else{20}]});
    Ok((
        actual,
        expected,
        json!({"producers_and_combat":trace,"decisions":d.trace,"host":host.0,"opponent":opponent.map(|id|id.0)}),
    ))
}
fn normalize(v: &Value, p: &str, ids: &mut Vec<String>) -> Value {
    match v {
        Value::Object(o) => {
            let mut r = serde_json::Map::new();
            for (k, v) in o {
                if k == "id"
                    && p.ends_with("/card")
                    && o.contains_key("card_types")
                    && o.contains_key("name")
                {
                    ids.push(format!("{p}/id"));
                    continue;
                }
                r.insert(k.clone(), normalize(v, &format!("{p}/{k}"), ids));
            }
            Value::Object(r)
        }
        Value::Array(a) => Value::Array(
            a.iter()
                .enumerate()
                .map(|(i, v)| normalize(v, &format!("{p}/{i}"), ids))
                .collect(),
        ),
        _ => v.clone(),
    }
}
#[test]
#[ignore = "paid static combat predicate semantics"]
fn report_static_combat_predicates() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("reports/runtime-audit/static-combat-predicate-frozen-inputs.json");
    let paths = [
        std::env::current_exe().unwrap(),
        input.clone(),
        root.join("crates/ironsmith-tools/tests/runtime_static_combat_predicate_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let payloads: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut compilation = vec![];
    for p in payloads["cards"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap();
        let (a, d) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(n),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        let definition = serde_json::to_value(&a.payload.definition).unwrap();
        let (mut ids, mut fi) = (vec![], vec![]);
        let parity = normalize(&definition, "/definition", &mut ids)
            == normalize(&p["frozen_definition"], "/definition", &mut fi);
        compilation.push(json!({"card":n,"artifact_checksum":a.payload_checksum,"frozen_artifact_checksum":p["frozen_artifact_checksum"],"definition_matches_frozen_except_unique_card_ids":parity,"ignored_id_paths":ids,"definition":definition}));
        defs.insert(n.to_string(), d);
    }
    let mut rows = vec![];
    for name in ["Snow Devil", "Intrepid Ace"] {
        for snow in if name == "Snow Devil" {
            vec![0, 1, 2]
        } else {
            vec![0]
        } {
            for role in ["neither", "attacking", "blocking", "neither_in_combat"] {
                if name == "Snow Devil"
                    && (role == "neither_in_combat" || (snow == 2 && role != "blocking"))
                {
                    continue;
                }
                println!("stage: {name} snow{snow} role{role}");
                let (status, actual, expected, evidence) = match trial(&defs, name, role, snow) {
                    Ok((a, e, t)) => (
                        if a == e {
                            "expected_result_observed"
                        } else {
                            "semantic_mismatch"
                        },
                        a,
                        e,
                        t,
                    ),
                    Err(e) => (
                        "fixture_or_execution_error",
                        json!({"error":e}),
                        Value::Null,
                        Value::Null,
                    ),
                };
                rows.push(json!({"card":name,"scenario":{"role":role,"snow_land_controller":if snow==0{Value::Null}else{json!(snow-1)}},"status":status,"actual":actual,"expected":expected,"fixture_evidence":evidence}));
            }
        }
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Full canonical paid sources/Aura plus actual Snow-Covered Island land plays, ordinary target/priority resolution, paid Crimson Wisps granting attacker haste, actual attacker/blocker declarations and combat damage. Snapshot exact precombat/during-combat P/T, flying/first strike, attack/block roles; independent survival/life outcomes. No synthetic combat flags, counters, or granted card abilities.","rows":rows,"compilation":compilation,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after}});
    std::fs::write(
        root.join("reports/runtime-audit/static-combat-predicate-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
