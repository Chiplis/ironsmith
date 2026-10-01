//! Strict paid-action reductions of Coat, Control Magic, and Futurist outcome leads.
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
        let selected = SelectFirstDecisionMaker.decide_objects(g, c);
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
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        if self.target_name.as_deref() == Some("PLAYER_ALICE") {
            return vec![Target::Player(alice())];
        }
        if let Some(name) = &self.target_name {
            if c.requirements.len() == 1 {
                if let Some(t)=c.requirements[0].legal_targets.iter().find(|t|matches!(t,Target::Object(id)if g.object(*id).is_some_and(|o|o.name==*name))){return vec![*t];}
            }
        }
        SelectFirstDecisionMaker.decide_targets(g, c)
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

fn dm(x: u32, accept: bool) -> ProbeDm {
    ProbeDm {
        x,
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

fn creature(name: &str, subtype: ironsmith::Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}
fn inspect(g: &GameState, id: ObjectId) -> Value {
    json!({"power":g.calculated_power(id),"toughness":g.calculated_toughness(id),"controller":g.controller_of_id(id).map(|p|p.index()),"tapped":g.is_tapped(id),"subtypes":g.current_subtypes(id).map(|v|v.iter().map(|s|format!("{s:?}")).collect::<Vec<_>>()),"unblockable":g.object_has_static_ability_id(id,ironsmith::static_abilities::StaticAbilityId::Unblockable)})
}
fn coat_case(
    defs: &HashMap<String, CardDefinition>,
    case: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup_players(2);
    let a = creature("Human witness", ironsmith::Subtype::Human);
    let b = creature("Zombie witness", ironsmith::Subtype::Zombie);
    let source = if case.contains("changeling") {
        &defs["Woodland Changeling"]
    } else {
        &a
    };
    let one = g.create_object_from_definition(source, alice(), Zone::Battlefield);
    let two = g.create_object_from_definition(
        if case.contains("different") {
            &b
        } else {
            source
        },
        alice(),
        Zone::Battlefield,
    );
    let mut dm = dm(0, false);
    let (mut q, paid) = cast(&mut g, &defs["Coat of Arms"], alice(), &mut dm)?;
    let error = resolve(&mut g, &mut q, &mut dm).err();
    let power = if case.contains("different") { 2 } else { 3 };
    Ok((
        json!({"resolution_error":error,"paid":paid,"one_power":g.calculated_power(one),"two_power":g.calculated_power(two),"one_toughness":g.calculated_toughness(one),"two_toughness":g.calculated_toughness(two),"coat_battlefield":count(&g,"Coat of Arms",Zone::Battlefield)}),
        json!({"resolution_error":null,"paid":5,"one_power":power,"two_power":power,"one_toughness":power,"two_toughness":power,"coat_battlefield":1}),
        json!({"one":inspect(&g,one),"two":inspect(&g,two),"choices":dm.trace}),
    ))
}
fn control_case(
    defs: &HashMap<String, CardDefinition>,
    case: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup_players(2);
    let bob = PlayerId::from_index(1);
    let caster = if case.contains("bob") { bob } else { alice() };
    let owner = if case.contains("self") {
        caster
    } else if caster == bob {
        alice()
    } else {
        bob
    };
    g.turn.active_player = caster;
    g.turn.priority_player = Some(caster);
    let target = if case.contains("directgitrog") {
        g.create_object_from_definition(&defs["The Gitrog Monster"], owner, Zone::Battlefield)
    } else if case.contains("gitrog") || case.contains("paidwitness") {
        g.turn.active_player = owner;
        let basic = creature("Control witness", ironsmith::Subtype::Human);
        let id = established(
            &mut g,
            if case.contains("gitrog") {
                &defs["The Gitrog Monster"]
            } else {
                &basic
            },
            owner,
            if case.contains("gitrog") { 5 } else { 2 },
        )?;
        g.turn.active_player = caster;
        g.turn.priority_player = Some(caster);
        id
    } else {
        g.create_object_from_definition(
            &creature("Control witness", ironsmith::Subtype::Human),
            owner,
            Zone::Battlefield,
        )
    };
    let mut dm = dm(0, false);
    dm.target_name = Some(
        if case.contains("gitrog") {
            "The Gitrog Monster"
        } else {
            "Control witness"
        }
        .into(),
    );
    let (mut q, paid) = cast(&mut g, &defs["Control Magic"], caster, &mut dm)?;
    let error = resolve(&mut g, &mut q, &mut dm).err();
    let aura = find(&g, "Control Magic", Zone::Battlefield).ok();
    Ok((
        json!({"resolution_error":error,"paid":paid,"aura_battlefield":aura.is_some(),"attached_to_target":aura.and_then(|id|g.object(id)).and_then(|o|o.attached_to)==Some(ironsmith::object::AttachmentTarget::Object(target)),"target_controller":g.controller_of_id(target).map(|p|p.index())}),
        json!({"resolution_error":null,"paid":4,"aura_battlefield":true,"attached_to_target":true,"target_controller":caster.index()}),
        json!({"choices":dm.trace,"caster":caster.index(),"owner":owner.index(),"target":inspect(&g,target),"derived_controller":g.calculated_characteristics(target).map(|v|v.controller.index()),"control_effects":g.all_continuous_effects().iter().filter(|e|matches!(e.modification,ironsmith::continuous::Modification::ChangeController(_))).map(|e|format!("{e:?}")).collect::<Vec<_>>(),"controller_after_reads":g.controller_of_id(target).map(|p|p.index()),"aura":aura.and_then(|id|g.object(id)).map(|o|json!({"owner":o.owner.index(),"controller":g.controller_of_id(o.id).map(|p|p.index()),"attached_to":o.attached_to.map(|id|format!("{id:?}"))}))}),
    ))
}
fn futurist_case(
    defs: &HashMap<String, CardDefinition>,
    case: &str,
) -> Result<(Value, Value, Value), String> {
    let mut g = setup_players(2);
    let source = established(&mut g, &defs["Futurist Operative"], alice(), 4)?;
    let mut dm = dm(0, true);
    dm.target_name = Some("Futurist Operative".into());
    let mut paid = 0;
    let mut error = None;
    if case != "futurist-untapped" {
        let (mut q, p) = cast(&mut g, &defs["Twiddle"], alice(), &mut dm)?;
        paid += p;
        error = resolve(&mut g, &mut q, &mut dm).err();
    }
    let tapped_snapshot = inspect(&g, source);
    if case == "futurist-retap-control" {
        let a = compute_legal_actions(&g, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find_map(|a| match a {
                LegalAction::ActivateAbility {
                    source: id,
                    ability_index,
                } if id == source => Some(ability_index),
                _ => None,
            })
            .ok_or("Futurist activation missing")?;
        let (mut q, p) = perform(&mut g, source, Some(a), &mut dm)?;
        paid += p;
        error = resolve(&mut g, &mut q, &mut dm).err();
    }
    let tapped = case == "futurist-tapped";
    let expected_state = json!({"power":if tapped{1}else{3},"toughness":if tapped{1}else{4},"controller":0,"tapped":tapped,"subtypes":if tapped{vec!["Human","Citizen"]}else{vec!["Human","Ninja"]},"unblockable":tapped});
    let mut state = inspect(&g, source);
    if let Some(a) = state["subtypes"].as_array_mut() {
        a.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    }
    let mut expected_state = expected_state;
    if let Some(a) = expected_state["subtypes"].as_array_mut() {
        a.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    }
    Ok((
        json!({"resolution_error":error,"followup_paid":paid,"state":state}),
        json!({"resolution_error":null,"followup_paid":if case=="futurist-untapped"{0}else if tapped{1}else{4},"state":expected_state}),
        json!({"choices":dm.trace,"after_twiddle":tapped_snapshot}),
    ))
}
#[test]
#[ignore = "manual expected outcomes, paid actions and strict canonical artifacts"]
fn report_mage_outcome_followups() {
    let case = std::env::var("FOLLOWUP_CASE").unwrap_or("coat-same".into());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        std::env::current_exe().unwrap(),
        ironsmith_tools::default_cards_path(),
        root.join("crates/ironsmith-tools/tests/runtime_mage_outcome_followups.rs"),
    ];
    let before: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let names: Vec<_> = [
        "Coat of Arms",
        "Woodland Changeling",
        "Control Magic",
        "Futurist Operative",
        "Twiddle",
        "The Gitrog Monster",
    ]
    .map(str::to_owned)
    .to_vec();
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
    let result = if case.starts_with("coat") {
        coat_case(&defs, &case)
    } else if case.starts_with("control") {
        control_case(&defs, &case)
    } else {
        futurist_case(&defs, &case)
    };
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
    let after: Vec<_> = paths.iter().map(|p| hash(p)).collect();
    let report = json!({"scope":"Strict canonical normal paid actions with tiny neutral boards and expected game outcomes, no engine shims. Synthetic subtype witnesses are 2/2 creatures with only the stated type.","provenance":{"before":before,"after":after,"artifacts_unchanged":before==after},"compilation":compilation,"rows":[{"card":if case.starts_with("coat"){"Coat of Arms"}else if case.starts_with("control"){"Control Magic"}else{"Futurist Operative"},"scenario":case,"status":status,"expected":expected,"actual":actual,"diagnostics":diag}]});
    std::fs::write(
        root.join(format!("reports/runtime-audit/mage-followup-{case}.json")),
        serde_json::to_string_pretty(&report).unwrap() + "\n",
    )
    .unwrap();
}
