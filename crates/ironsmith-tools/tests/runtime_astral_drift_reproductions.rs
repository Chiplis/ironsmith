//! Opt-in canonical runtime evidence. Passing means reports were generated.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, BlockerDeclaration, DecisionMaker, GameProgress, LegalAction,
    SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, PartitionContext, SelectObjectsContext, SelectOptionsContext, ViewCardsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_attacker_declarations_with_dm, apply_blocker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, PowerToughness, Step,
    Subtype, Zone,
};
use serde_json::{Value, json};

use std::collections::HashMap;

const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
fn bob() -> PlayerId {
    PlayerId(1)
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.set_random_seed(SEED);
    game.turn.turn_number = 3;
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for player in [alice(), bob()] {
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 12);
        }
    }
    game
}
fn creature(name: &str, blue: bool, flying: bool) -> CardDefinition {
    let mut b = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 6));
    if blue {
        b = b.color_indicator(ironsmith::color::ColorSet::BLUE);
    }
    if flying {
        b = b.flying();
    }
    b.build()
}
struct Choices {
    target: Option<ObjectId>,
    accept: bool,
    trace: Vec<Value>,
    planar_viewed: usize,
}
impl DecisionMaker for Choices {
    fn decide_targets(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        let selected = self
            .target
            .map(ironsmith::game_state::Target::Object)
            .into_iter()
            .filter(|target| {
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            })
            .collect::<Vec<_>>();
        self.trace.push(json!({"choice":"targets","context":format!("{ctx:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
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
    fn decide_partition(&mut self, _: &GameState, c: &PartitionContext) -> Vec<ObjectId> {
        self.trace
            .push(json!({"choice":"surveil_keep_all","context":format!("{c:?}")}));
        Vec::new()
    }
    fn view_cards(
        &mut self,
        _: &GameState,
        _: PlayerId,
        cards: &[ObjectId],
        ctx: &ViewCardsContext,
    ) {
        if ctx.description.contains("planar") {
            self.planar_viewed += cards.len();
        }
        self.trace.push(json!({"choice":"view_cards","context":format!("{ctx:?}"),"cards":format!("{cards:?}")}));
    }
}
fn announce(
    game: &mut GameState,
    action: LegalAction,
    dm: &mut impl DecisionMaker,
) -> Result<TriggerQueue, String> {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..24 {
        if !game.stack.is_empty()
            && state.pending_activation.is_none()
            && state.pending_cast.is_none()
        {
            progress.map_err(|e| e.to_string())?;
            return Ok(queue);
        }
        let context = match progress.map_err(|e| e.to_string())? {
            GameProgress::NeedsDecisionCtx(c) => c,
            p => return Err(format!("announcement stopped: {p:?}")),
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm);
    }
    Err("announcement exceeded24 decisions".into())
}
fn cast(
    game: &mut GameState,
    def: &CardDefinition,
    dm: &mut impl DecisionMaker,
) -> Result<TriggerQueue, String> {
    game.turn.priority_player = Some(alice());
    let id = game.create_object_from_definition(def, alice(), Zone::Hand);
    let action = compute_legal_actions(game, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or_else(|| format!("no legal cast of {}", def.name()))?;
    announce(game, action, dm)
}
fn finish(
    game: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut impl DecisionMaker,
) -> Result<(), String> {
    for step in 0..32 {
        eprintln!(
            "ASTRAL_STAGE finish step={step} stack_len={}",
            game.stack.len()
        );
        advance_priority_with_dm(game, q, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
    }
    Err("resolution exceeded32 steps".into())
}
fn attack(
    game: &mut GameState,
    attackers: &[ObjectId],
    defender: PlayerId,
) -> Result<TriggerQueue, String> {
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    for id in attackers {
        game.remove_summoning_sickness(*id);
    }
    let declarations = attackers
        .iter()
        .map(|id| AttackerDeclaration {
            creature: *id,
            target: AttackTarget::Player(defender),
        })
        .collect::<Vec<_>>();
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations_with_dm(
        game,
        &mut combat,
        &mut queue,
        &declarations,
        &mut SelectFirstDecisionMaker,
    )
    .map_err(|e| e.to_string())?;
    game.combat = Some(combat);
    Ok(queue)
}
fn block(
    game: &mut GameState,
    queue: &mut TriggerQueue,
    blocker: ObjectId,
    attacker: ObjectId,
    defender: PlayerId,
) -> Result<(), String> {
    game.turn.step = Some(Step::DeclareBlockers);
    let mut combat = game.combat.take().ok_or("missing combat")?;
    let result = apply_blocker_declarations(
        game,
        &mut combat,
        queue,
        &[BlockerDeclaration {
            blocker,
            blocking: attacker,
        }],
        defender,
    )
    .map_err(|e| e.to_string());
    game.combat = Some(combat);
    result
}
fn compile(names: &[String]) -> HashMap<String, (CardDefinition, String)> {
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit/astral-drift-inputs.json");
    let document: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    names
        .iter()
        .map(|name| {
            let p = &document["cards"][name];
            let builder = ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(name),
            );
            let (a, d) = ironsmith_registry::compile_builder_to_artifact(
                builder,
                p["parse_input"].as_str().unwrap(),
                false,
            )
            .unwrap();
            (name.clone(), (d, a.payload_checksum))
        })
        .collect()
}

fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: Value,
    expected: Value,
    result: Result<Value, String>,
    checksum: &str,
    trace: Vec<Value>,
) {
    let (status, actual) = match result {
        Ok(a) if a == expected => ("expected_result_observed", a),
        Ok(a) => ("semantic_mismatch", a),
        Err(e) if e.starts_with("Resolution failed:") => ("resolution_failed", json!({"error":e})),
        Err(e) => ("execution_or_fixture_error", json!({"error":e})),
    };
    rows.push(json!({"card":card,"scenario":scenario,"expected":expected,"actual":actual,"status":status,"artifact_checksum":checksum,"seed":SEED,"execution_trace":trace}));
    if status == "semantic_mismatch" {
        rows.last_mut().unwrap()["outcome_category"] = json!("silent_wrong_result");
    }
    if status == "resolution_failed" {
        rows.last_mut().unwrap()["outcome_category"] = json!("runtime_exception");
    }
}
fn report(name: &str, scope: &str, limitations: &str, rows: Vec<Value>) {
    let path = std::env::current_exe().unwrap();
    let binary_hash = std::env::var("AUDIT_BINARY_SHA256")
        .expect("isolating driver supplies independently verified executable hash");
    let report = json!({"scope":scope,"limitations":limitations,"provenance":{"binary":path,"binary_sha256":binary_hash,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","seed":SEED,"unique_card_ids":true},"rows":rows});
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit")
        .join(name);
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

#[test]
#[ignore = "bounded per-scenario reporter; completion is not semantic correctness"]
fn report_astral_drift_case() {
    let case = std::env::var("ASTRAL_CASE").expect("ASTRAL_CASE selects one isolated scenario");
    let (other, branch) = case
        .split_once('_')
        .expect("own_accept or other_decline etc");
    let other = other == "other";
    let accept = branch == "accept";
    let shroud = branch == "shroud";
    let empty = branch == "empty";
    let names = vec!["Astral Drift".to_owned(), "Forgotten Cave".to_owned()];
    eprintln!("ASTRAL_STAGE canonical_compile");
    let defs = compile(&names);
    let (def, checksum) = &defs["Astral Drift"];
    let mut game = game();
    if other {
        game.create_object_from_definition(def, alice(), Zone::Battlefield);
    }
    let cycle_def = if other {
        &defs["Forgotten Cave"].0
    } else {
        def
    };
    let cycle = game.create_object_from_definition(cycle_def, alice(), Zone::Hand);
    let cycle_stable = game.object(cycle).unwrap().stable_id;
    let target = if empty {
        None
    } else {
        let mut b = CardDefinitionBuilder::new(CardId::new(), "Astral target owned by Bob")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 6));
        if shroud {
            b = b.shroud();
        }
        let id = game.create_object_from_definition(&b.build(), bob(), Zone::Battlefield);
        game.set_current_controller(id, alice());
        Some(id)
    };
    let target_stable = target.map(|id| game.object(id).unwrap().stable_id);
    for n in 0..3 {
        game.create_object_from_definition(
            &creature(&format!("Cycling draw buffer {n}"), false, false),
            alice(),
            Zone::Library,
        );
    }
    let mut dm = Choices {
        target,
        accept,
        trace: Vec::new(),
        planar_viewed: 0,
    };
    let mut before_end = Value::Null;
    let mut mana_paid = 0;
    let mut stage = "discover";
    let result: Result<Value, String> = (|| {
        eprintln!("ASTRAL_STAGE legal_action_discovery");
        let actions = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .filter(|a| matches!(a,LegalAction::ActivateAbility{source,..}if *source==cycle))
            .collect::<Vec<_>>();
        dm.trace.push(json!({"stage":"discovery","actions":format!("{actions:?}"),"cycled_card":cycle_def.name()}));
        if actions.len() != 1 {
            return Err(format!(
                "expected one legal printed cycling activation, found {}",
                actions.len()
            ));
        }
        let action = actions[0].clone();
        let before = game.player(alice()).unwrap().mana_pool.total();
        stage = "announcement";
        eprintln!("ASTRAL_STAGE cycling_announcement {action:?}");
        let mut queue = announce(&mut game, action, &mut dm)?;
        mana_paid = before.saturating_sub(game.player(alice()).unwrap().mana_pool.total());
        stage = "cycling_and_trigger_resolution";
        eprintln!("ASTRAL_STAGE resolve_cycling_and_actual_trigger");
        finish(&mut game, &mut queue, &mut dm)?;
        let current_target = target_stable.and_then(|id| game.find_object_by_stable_id(id));
        before_end = json!({"target_zone":current_target.and_then(|id|game.object(id)).map(|o|format!("{:?}",o.zone)),"delayed_triggers":game.effect_store.delayed_triggers.len(),"drawn_cards":game.player(alice()).unwrap().hand.len()});
        dm.trace.push(json!({"stage":"after_cycling_before_end","state":before_end,"delayed":format!("{:?}",game.effect_store.delayed_triggers)}));
        stage = "next_end_step";
        eprintln!("ASTRAL_STAGE next_end_step");
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::End);
        ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
        finish(&mut game, &mut queue, &mut dm)?;
        let target_now = target_stable.and_then(|id| game.find_object_by_stable_id(id));
        let cycled_now = game.find_object_by_stable_id(cycle_stable);
        Ok(
            json!({"mana_paid":mana_paid,"cycled_card_in_graveyard":cycled_now.and_then(|id|game.object(id)).is_some_and(|o|o.zone==Zone::Graveyard),"before_end":before_end,"after_end_target_zone":target_now.and_then(|id|game.object(id)).map(|o|format!("{:?}",o.zone)),"after_end_target_controller":target_now.and_then(|id|game.current_controller(id)).map(|p|p.0),"hand_count":game.player(alice()).unwrap().hand.len(),"library_count":game.player(alice()).unwrap().library.len(),"delayed_triggers_remaining":game.effect_store.delayed_triggers.len(),"stack_empty":game.stack.is_empty()}),
        )
    })();
    dm.trace.push(json!({"last_stage":stage,"mana_paid":mana_paid,"before_end":before_end,"stack":format!("{:?}",game.stack),"target_snapshot":target_stable.and_then(|id|game.find_object_by_stable_id(id)).and_then(|id|game.object(id)).map(|o|format!("zone={:?}, owner={:?}",o.zone,o.owner))}));
    let mut rows = Vec::new();
    record(
        &mut rows,
        "Astral Drift",
        json!({"case":case,"cycle_card":cycle_def.name(),"drift_on_battlefield":other,"accept_optional_exile":accept,"target_has_shroud":shroud,"no_creatures":empty,"target_owner":"Bob","target_initial_controller":"Alice"}),
        json!({"mana_paid":if other{1}else{3},"cycled_card_in_graveyard":true,"before_end":{"target_zone":if empty{None}else{Some(if accept{"Exile"}else{"Battlefield"})},"delayed_triggers":usize::from(accept),"drawn_cards":1},"after_end_target_zone":if empty{None}else{Some("Battlefield")},"after_end_target_controller":if empty{None}else{Some(if accept{1}else{0})},"hand_count":1,"library_count":2,"delayed_triggers_remaining":0,"stack_empty":true}),
        result,
        checksum,
        dm.trace,
    );
    std::fs::create_dir_all(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../reports/runtime-audit/astral-drift-cases"),
    )
    .unwrap();
    report(
        &format!("astral-drift-cases/{case}.json"),
        "One bounded actual printed cycling activation and produced Astral Drift trigger, then next end step",
        "Battlefield Drift is an established source. Target is an established creature owned by Bob but controlled by Alice; accepted exile should return it under Bob. Empty/shroud controls offer no legal creature target. This reporter is externally isolated and timeout-bounded; report completion is not semantic correctness.",
        rows,
    );
}
