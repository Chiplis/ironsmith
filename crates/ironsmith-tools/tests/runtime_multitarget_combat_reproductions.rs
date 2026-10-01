//! Exact target outcomes from paid activations and actual combat declarations.
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
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
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
    target_name: Option<String>,
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
        target_name: None,
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

const NAMES: [&str; 7] = [
    "Drooling Groodion",
    "Garruk, Savage Herald",
    "Feral Contest",
    "Monstrous Step",
    "Falling Timber",
    "Twiddle",
    "Shock",
];
fn creature(name: &str, p: i32, t: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(p, t))
        .build()
}
fn cast_resolve(
    g: &mut GameState,
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    dm: &mut ProbeDm,
) -> Result<(u32, Option<String>), String> {
    let (mut q, p) =
        cast(g, &defs[name], alice(), dm).map_err(|e| format!("{e};trace={:?}", dm.trace))?;
    Ok((p, resolve(g, &mut q, dm).err()))
}
fn attack(
    g: &mut GameState,
    ids: &[ObjectId],
    dm: &mut ProbeDm,
) -> Result<ironsmith::combat_state::CombatState, String> {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    for id in ids {
        g.remove_summoning_sickness(*id);
    }
    g.turn.phase = ironsmith::Phase::Combat;
    g.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut q = TriggerQueue::new();
    let declarations: Vec<_> = ids
        .iter()
        .map(|id| AttackerDeclaration {
            creature: *id,
            target: AttackTarget::Player(PlayerId(1)),
        })
        .collect();
    ironsmith::game_loop::apply_attacker_declarations_with_dm(
        g,
        &mut combat,
        &mut q,
        &declarations,
        dm,
    )
    .map_err(|e| e.to_string())?;
    g.combat = Some(combat.clone());
    resolve(g, &mut q, dm)?;
    Ok(combat)
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    mode: usize,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup_players(2);
    let mut dm = dm(0, true);
    let mut ids = vec![];
    let falling = name == "Falling Timber";
    for i in 0..3 {
        ids.push(g.create_object_from_definition(
            &creature(&format!("Target witness {i}"), 2 + i, 6 + i),
            if i == 0 || falling {
                alice()
            } else {
                PlayerId(1)
            },
            Zone::Battlefield,
        ));
    }
    let mut setup_paid = 0;
    let source = if name == "Drooling Groodion" || name == "Garruk, Savage Herald" {
        setup_paid = 6;
        Some(established(&mut g, &defs[name], alice(), 6)?)
    } else {
        None
    };
    if name == "Drooling Groodion" {
        g.create_object_from_definition(
            &creature("Cost witness", 1, 1),
            alice(),
            Zone::Battlefield,
        );
    }
    if falling {
        let d = CardDefinitionBuilder::new(CardId::new(), "Cost witness")
            .card_types(vec![CardType::Land])
            .build();
        g.create_object_from_definition(&d, alice(), Zone::Battlefield);
    }
    let tapped = (name == "Feral Contest" && mode == 1) || (name == "Monstrous Step" && mode == 2);
    if tapped {
        dm.targets = vec![Target::Object(ids[1])];
        let (p, e) = cast_resolve(&mut g, defs, "Twiddle", &mut dm)?;
        setup_paid += p;
        if e.is_some() || !g.is_tapped(ids[1]) {
            return Err("actual Twiddle did not tap forced blocker".into());
        }
    }
    let n = if (name == "Monstrous Step" && mode == 0) || (falling && mode == 0) {
        1
    } else {
        2
    };
    dm.targets = ids[..n].iter().copied().map(Target::Object).collect();
    dm.kicks = u32::from(falling && mode == 1);
    let (paid, error) = if let Some(source) = source {
        let ability = if name == "Drooling Groodion" { 0 } else { 1 };
        let (mut q, p) = perform(&mut g, source, Some(ability), &mut dm)
            .map_err(|e| format!("{e};trace={:?}", dm.trace))?;
        (p, resolve(&mut g, &mut q, &mut dm).err())
    } else if falling && mode == 2 {
        (0, None)
    } else {
        cast_resolve(&mut g, defs, name, &mut dm)?
    };
    let mut combat_diag = Value::Null;
    let actual;
    let expected;
    if falling {
        dm.targets = vec![Target::Object(ids[0])];
        dm.kicks = 0;
        let (p, e) = cast_resolve(&mut g, defs, "Shock", &mut dm)?;
        setup_paid += p;
        if e.is_some() {
            return Err(format!("noncombat control failed {e:?}"));
        }
        let mut combat = attack(&mut g, &ids, &mut dm)?;
        g.turn.step = Some(ironsmith::Step::DeclareBlockers);
        let mut q = TriggerQueue::new();
        ironsmith::game_loop::apply_blocker_declarations(
            &mut g,
            &mut combat,
            &mut q,
            &[],
            PlayerId(1),
        )
        .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        g.turn.step = Some(ironsmith::Step::CombatDamage);
        let events = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
            &mut g, &combat, false, &mut dm,
        )
        .map_err(|e| format!("{e:?}"))?;
        resolve(&mut g, &mut q, &mut dm)?;
        actual = json!({"resolution_error":error,"paid":paid,"noncombat_damage":g.damage_on(ids[0]),"bob_life":g.player(PlayerId(1)).unwrap().life,"cost_sacrificed":count(&g,"Cost witness",Zone::Graveyard)});
        expected = json!({"resolution_error":null,"paid":if mode==2{0}else{3},"noncombat_damage":2,"bob_life":match mode{0=>13,1=>16,_=>11},"cost_sacrificed":usize::from(mode==1)});
        combat_diag = json!({"damage_events":format!("{events:?}")});
    } else if name == "Feral Contest" || name == "Monstrous Step" {
        let mut combat = attack(&mut g, &[ids[0]], &mut dm)?;
        g.turn.step = Some(ironsmith::Step::DeclareBlockers);
        let mut q = TriggerQueue::new();
        let mut clone_g = g.clone();
        let mut clone_combat = combat.clone();
        let empty = ironsmith::game_loop::apply_blocker_declarations(
            &mut clone_g,
            &mut clone_combat,
            &mut q,
            &[],
            PlayerId(1),
        );
        let empty_error = empty.err().map(|e| e.to_string());
        let required = name == "Feral Contest" || mode > 0;
        let legal = if tapped {
            vec![]
        } else {
            vec![ironsmith::decision::BlockerDeclaration {
                blocker: ids[1],
                blocking: ids[0],
            }]
        };
        let blocker_error = ironsmith::game_loop::apply_blocker_declarations(
            &mut g,
            &mut combat,
            &mut q,
            &legal,
            PlayerId(1),
        )
        .err()
        .map(|e| e.to_string());
        actual = json!({"resolution_error":error,"paid":paid,"power":g.calculated_power(ids[0]),"toughness":g.calculated_toughness(ids[0]),"plus":g.counter_count(ids[0],CounterType::PlusOnePlusOne),"empty_blocks_legal":empty_error.is_none(),"intended_block_error":blocker_error});
        expected = json!({"resolution_error":null,"paid":if name=="Feral Contest"{4}else{5},"power":if name=="Feral Contest"{3}else{9},"toughness":if name=="Feral Contest"{7}else{13},"plus":usize::from(name=="Feral Contest"),"empty_blocks_legal":!required||tapped,"intended_block_error":null});
        combat_diag =
            json!({"empty_block_error":empty_error,"forced_blocker_tapped":g.is_tapped(ids[1])});
    } else {
        let states:Vec<_>=ids.iter().map(|id|json!({"power":g.calculated_power(*id),"toughness":g.calculated_toughness(*id),"damage":g.damage_on(*id)})).collect();
        let grood = name == "Drooling Groodion";
        actual = json!({"resolution_error":error,"paid":paid,"states":states,"cost_sacrificed":count(&g,"Cost witness",Zone::Graveyard),"source_loyalty":source.map(|id|g.counter_count(id,CounterType::Loyalty)).unwrap_or(0)});
        expected = json!({"resolution_error":null,"paid":if grood{4}else{0},"states":if grood{vec![json!({"power":4,"toughness":8,"damage":0}),json!({"power":1,"toughness":5,"damage":0}),json!({"power":4,"toughness":8,"damage":0})]}else{vec![json!({"power":2,"toughness":6,"damage":0}),json!({"power":3,"toughness":7,"damage":2}),json!({"power":4,"toughness":8,"damage":0})]},"cost_sacrificed":usize::from(grood),"source_loyalty":if grood{0}else{3}});
    }
    Ok((
        actual,
        expected,
        json!({"choices":dm.trace,"setup_paid":setup_paid,"combat":combat_diag}),
    ))
}
#[test]
#[ignore = "strict multi-target combat and activation outcomes"]
fn report_multitarget_combat() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_multitarget_combat_reproductions.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names = NAMES.map(str::to_owned).to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut defs = HashMap::new();
    let mut compilation = vec![];
    for p in payloads.into_values().flatten() {
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            p.parse_name.as_deref().unwrap_or(&p.name),
        );
        match ironsmith_registry::compile_builder_to_artifact(b, &p.parse_input, false) {
            Ok((a, d)) => {
                compilation.push(json!({"card":p.name,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
                defs.insert(d.name().to_owned(), d);
            }
            Err(e) => compilation.push(json!({"card":p.name,"strict_compile_error":e.to_string()})),
        }
    }
    let mut rows = vec![];
    for name in &NAMES[..5] {
        if !defs.contains_key(*name) {
            continue;
        }
        for mode in 0..match *name {
            "Falling Timber" | "Monstrous Step" => 3,
            "Feral Contest" => 2,
            _ => 1,
        } {
            println!("stage: {name} mode{mode}");
            let result = trial(&defs, name, mode);
            let (status, actual, expected, diag) = match result {
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
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":expected,"actual":actual,"diagnostics":diag}));
        }
    }
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict paid multi-target permanents/activations or spells, explicit targeting, real creature sacrifice/loyalty costs. Real attacks/block declarations, legal empty-block negative controls on cloned state, actual paid Twiddle unavailable-blocker controls, actual combat damage plus noncombat Shock prevention control. Setup creatures represent an established battlefield and have summoning sickness removed before attack.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join("reports/runtime-audit/multitarget-combat-execution.json"),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
