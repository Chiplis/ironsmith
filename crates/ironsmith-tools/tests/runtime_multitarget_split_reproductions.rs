//! Exact outcomes for two-player draws, split effects, returns, and optional copying.
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

const NAMES: [&str; 6] = [
    "Biomantic Mastery",
    "Fleeting Reflection",
    "Hostile Takeover",
    "Once and Future",
    "Withdraw",
    "Twiddle",
];
fn creature(name: &str, p: i32, t: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(p, t))
        .build()
}
fn trial(
    defs: &HashMap<String, CardDefinition>,
    name: &str,
    mode: usize,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup_players(2);
    let bob = PlayerId(1);
    let mut dm = dm(0, mode > 0);
    let mut ids = vec![];
    if name == "Biomantic Mastery" {
        for _ in 0..2 {
            g.create_object_from_definition(
                &creature("Alice creature", 2, 6),
                alice(),
                Zone::Battlefield,
            );
        }
        for _ in 0..3 {
            g.create_object_from_definition(
                &creature("Bob creature", 3, 7),
                bob,
                Zone::Battlefield,
            );
        }
        for _ in 0..12 {
            g.create_object_from_definition(
                &creature("Library witness", 1, 1),
                alice(),
                Zone::Library,
            );
        }
        dm.targets = if mode == 0 {
            vec![Target::Player(alice()), Target::Player(bob)]
        } else {
            vec![Target::Player(bob), Target::Player(alice())]
        };
    } else if name == "Once and Future" {
        for i in 0..2 {
            ids.push(g.create_object_from_definition(
                &creature(&format!("Target witness {i}"), 2, 6),
                alice(),
                Zone::Graveyard,
            ));
        }
        dm.targets = ids[..if mode % 2 == 0 { 1 } else { 2 }]
            .iter()
            .copied()
            .map(Target::Object)
            .collect();
        g.player_mut(alice()).unwrap().mana_pool = Default::default();
        g.player_mut(alice())
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, if mode >= 2 { 4 } else { 1 });
        if mode < 2 {
            g.player_mut(alice())
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 3);
        }
    } else {
        for i in 0..3 {
            ids.push(g.create_object_from_definition(
                &creature(&format!("Target witness {i}"), 2 + i, 6 + i),
                if i == 0 { alice() } else { bob },
                Zone::Battlefield,
            ));
        }
        let n = if name == "Hostile Takeover" {
            mode
        } else if name == "Fleeting Reflection" {
            1 + mode
        } else {
            2
        };
        dm.targets = ids[..n].iter().copied().map(Target::Object).collect();
        if name == "Fleeting Reflection" {
            let saved = dm.targets.clone();
            dm.targets = vec![Target::Object(ids[0])];
            dm.accept = true;
            let (mut q, paid) = cast(&mut g, &defs["Twiddle"], alice(), &mut dm)?;
            if paid != 1 {
                return Err("tap setup mana mismatch".into());
            }
            resolve(&mut g, &mut q, &mut dm)?;
            if !g.is_tapped(ids[0]) {
                return Err("actual Twiddle failed to tap source".into());
            }
            dm.targets = saved;
        }
    }
    let before_life = g.player(alice()).unwrap().life;
    let bob_mana = g.player(bob).unwrap().mana_pool.total();
    let (mut q, paid) = cast(&mut g, &defs[name], alice(), &mut dm)
        .map_err(|e| format!("{e}; choices={:?}", dm.trace))?;
    let announced_mana = g
        .object(g.stack.last().unwrap().object_id)
        .map(|o| format!("{:?}", o.mana_spent_to_cast));
    let error = resolve(&mut g, &mut q, &mut dm).err();
    let actual;
    let expected;
    if name == "Biomantic Mastery" {
        actual = json!({"resolution_error":error,"paid":paid,"alice_hand":g.player(alice()).unwrap().hand.len(),"bob_hand":g.player(bob).unwrap().hand.len(),"alice_library":g.player(alice()).unwrap().library.len()});
        expected =
            json!({"resolution_error":null,"paid":7,"alice_hand":5,"bob_hand":0,"alice_library":7});
    } else if name == "Once and Future" {
        let states:Vec<_>=(0..2).map(|i|{let label=format!("Target witness {i}");json!({"hand":count(&g,&label,Zone::Hand),"graveyard":count(&g,&label,Zone::Graveyard),"library":count(&g,&label,Zone::Library)})}).collect();
        let second = mode % 2 == 1;
        let adamant = mode >= 2;
        actual = json!({"resolution_error":error,"paid":paid,"states":states,"source_exile":count(&g,name,Zone::Exile)});
        expected = json!({"resolution_error":null,"paid":4,"states":[{"hand":1,"graveyard":0,"library":0},{"hand":usize::from(second&&adamant),"graveyard":usize::from(!second),"library":usize::from(second&&!adamant)}],"source_exile":1});
    } else {
        let states:Vec<_>=ids.iter().enumerate().map(|(i,id)|{let label=format!("Target witness {i}");let live=g.object(*id).is_some_and(|o|o.zone==Zone::Battlefield);json!({"battlefield":live,"hand":count(&g,&label,Zone::Hand),"graveyard":count(&g,&label,Zone::Graveyard),"power":if live{g.calculated_power(*id)}else{None},"toughness":if live{g.calculated_toughness(*id)}else{None},"damage":g.damage_on(*id),"tapped":g.is_tapped(*id),"hexproof":live&&g.object_has_static_ability_id(*id,ironsmith::static_abilities::StaticAbilityId::Hexproof)})}).collect();
        let ws:Vec<_>=(0..3).map(|i|{let dead=name=="Hostile Takeover"&&mode>0&&i==0;let hand=name=="Withdraw"&&(i==0||(i==1&&mode==0));let live=!dead&&!hand;let(p,t)=if name=="Hostile Takeover"&&mode==2&&i==1{(4,4)}else if name=="Fleeting Reflection"&&mode==1&&i==0{(3,7)}else{(2+i,6+i)};json!({"battlefield":live,"hand":usize::from(hand),"graveyard":usize::from(dead),"power":if live{Some(p)}else{None},"toughness":if live{Some(t)}else{None},"damage":if live&&name=="Hostile Takeover"{3}else{0},"tapped":false,"hexproof":name=="Fleeting Reflection"&&i==0})}).collect();
        actual = json!({"resolution_error":error,"paid":paid,"bob_mana_paid":bob_mana-g.player(bob).unwrap().mana_pool.total(),"states":states});
        expected = json!({"resolution_error":null,"paid":if name=="Hostile Takeover"{5}else{2},"bob_mana_paid":usize::from(name=="Withdraw"&&mode>0),"states":ws});
    }
    Ok((
        actual,
        expected,
        json!({"choices":dm.trace,"mode":mode,"announced_mana_spent":announced_mana,"life_change":g.player(alice()).unwrap().life-before_life,"stack":g.stack.len()}),
    ))
}
#[test]
#[ignore = "strict split-target expected board outcomes"]
fn report_multitarget_split() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_multitarget_split_reproductions.rs"),
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
        if std::env::var("MULTITARGET_CARD").is_ok_and(|n| n != *name) || !defs.contains_key(*name)
        {
            continue;
        }
        for mode in 0..match *name {
            "Hostile Takeover" => 3,
            "Once and Future" => 4,
            _ => 2,
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
    let report = json!({"scope":"Strict normal paid casts with explicit target callbacks and independent draw/zone/stat/damage outcomes. Optional first/second targets, adamant mana colors, Withdraw payment accept/decline, and actual Twiddle tapping before copy/untap. No engine shims or target-count-only success claims.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":rows});
    std::fs::write(
        root.join(if std::env::var("MULTITARGET_CARD").is_ok() {
            "reports/runtime-audit/multitarget-split-once-mana-execution.json"
        } else {
            "reports/runtime-audit/multitarget-split-execution.json"
        }),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
