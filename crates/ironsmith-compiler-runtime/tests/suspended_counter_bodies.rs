//! Exact frozen full bodies, independent expected outcomes, and native paid actions.
//! Authored only: builds, compiler probes, and scenario execution are deferred.
use ironsmith::alternative_cast::{AlternativeCastingMethod, CastingMethod};
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, ManaPaymentContext, NumberContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, extract_target_requirements_from_program_with_modes,
    generate_and_queue_step_triggers, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::grant_registry::GrantSource;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::special_actions::{self, SpecialAction, TurnFaceUpMethod};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, Phase, PlayerId, Step, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::SuspendTime;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const NAMES: [&str; 4] = ["Fury Charm", "Shivan Sand-Mage", "Timebender", "Timecrafting"];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/suspended_counter_bodies.json.fixture")).unwrap();
    assert_eq!(rows.len(), 4);
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    compile_pair(name, text)
}

fn compile_pair(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}

fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.turn_number = 5;
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g
}

fn witness(g: &mut GameState, owner: PlayerId, zone: Zone, kind: CardType) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Counter witness")
        .card_types(vec![kind]).power_toughness(PowerToughness::fixed(2, 3)).build();
    g.create_object_from_card(&card, owner, zone)
}

fn fund(g: &mut GameState, color: ManaSymbol, colored: u32, generic: u32) {
    let pool = &mut g.player_mut(A).unwrap().mana_pool;
    pool.add(color, colored);
    pool.add(ManaSymbol::Colorless, generic);
}

#[derive(Default)]
struct Choices {
    mode: usize,
    x: u32,
    target: Option<ObjectId>,
    required: Vec<ObjectId>,
    forbidden: Vec<ObjectId>,
    illegal_modes: Vec<usize>,
    mode_calls: usize,
    number_calls: usize,
    target_calls: usize,
    cast_offers: usize,
    accept_cast: bool,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, g: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.to_ascii_lowercase().contains("mode") {
            assert_eq!((ctx.min, ctx.max), (1, 1));
            assert!(ctx.options.iter().any(|option| option.index == self.mode && option.legal), "{ctx:?}");
            for mode in &self.illegal_modes {
                assert!(ctx.options.iter().any(|option| option.index == *mode && !option.legal), "{ctx:?}");
            }
            self.mode_calls += 1;
            vec![self.mode]
        } else { SelectFirstDecisionMaker.decide_options(g, ctx) }
    }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        assert!(ctx.is_x_value);
        assert_eq!(ctx.min, 0);
        assert!(self.x <= ctx.max);
        self.number_calls += 1;
        self.x
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert_eq!(ctx.requirements.len(), 1, "one selected mode has one target: {ctx:?}");
        let req = &ctx.requirements[0];
        assert_eq!((req.min_targets, req.max_targets), (1, Some(1)));
        for id in &self.required {
            assert!(req.legal_targets.contains(&Target::Object(*id)), "missing legal target {id:?}: {ctx:?}");
        }
        for id in &self.forbidden {
            assert!(!req.legal_targets.contains(&Target::Object(*id)), "illegal target {id:?}: {ctx:?}");
        }
        let target = Target::Object(self.target.expect("explicit scenario target"));
        assert!(req.legal_targets.contains(&target), "{ctx:?}");
        self.target_calls += 1;
        vec![target]
    }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        assert!(ctx.description.to_ascii_lowercase().contains("cast"), "unexpected optional branch: {ctx:?}");
        self.cast_offers += 1;
        self.accept_cast
    }
    fn decide_mana_payment(&mut self, _: &GameState, ctx: &ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm { plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash }
    }
}

fn queue(g: &mut GameState, dm: &mut Choices) {
    put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
}

fn action(g: &mut GameState, action: LegalAction, dm: &mut Choices) {
    g.turn.priority_player = Some(A);
    let mut state = PriorityLoopState::new(g.players.len());
    let mut triggers = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(g, &mut triggers, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(g, &mut triggers, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(g, &mut triggers, dm).unwrap();
}

fn cast(g: &mut GameState, id: ObjectId, method: CastingMethod, dm: &mut Choices) {
    action(g, LegalAction::CastSpell { spell_id: id, from_zone: Zone::Hand, casting_method: method }, dm);
}

fn resolve(g: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(g, dm).unwrap();
    queue(g, dm);
}

fn grant_definitions() -> [CardDefinition; 2] {
    compile_pair("Native Suspend grant witness",
        "Mana cost: {0}\nType: Instant\nTarget card in exile gains suspend until end of turn.")
}

fn grant_suspend(g: &mut GameState, grant: &CardDefinition, target: ObjectId) {
    let spell = g.create_object_from_definition(grant, A, Zone::Hand);
    let mut dm = Choices { target: Some(target), ..Default::default() };
    cast(g, spell, CastingMethod::Normal, &mut dm);
    resolve(g, &mut dm);
    assert_eq!(dm.target_calls, 1);
    assert_eq!(dm.mode_calls, 0);
    assert!(g.stack_is_empty());
    assert!(g.object(target).unwrap().alternative_casts.is_empty(),
        "real gains-suspend lowering grants abilities, not a casting-permission fixture");
}

/// Leave the card's real modal spell/trigger announced on the stack. Creature
/// sources reach it by a paid cast plus actual ETB or actual paid Morph action.
fn announce(g: &mut GameState, name: &str, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let source = g.create_object_from_definition(definition, A, Zone::Hand);
    let stable = g.object(source).unwrap().stable_id;
    let before = g.player(A).unwrap().mana_pool.total();
    assert_eq!(before, 0, "each scenario funds only the printed payment");
    match name {
        "Fury Charm" => fund(g, ManaSymbol::Red, 1, 1),
        "Shivan Sand-Mage" => fund(g, ManaSymbol::Red, 2, 2),
        "Timebender" => fund(g, ManaSymbol::Blue, 0, 3),
        "Timecrafting" => fund(g, ManaSymbol::Red, 1, dm.x),
        _ => unreachable!(),
    }
    cast(g, source, if name == "Timebender" { CastingMethod::FaceDown } else { CastingMethod::Normal }, dm);
    assert_eq!(g.player(A).unwrap().mana_pool.total(), 0, "{name}: full printed cost paid");
    assert_eq!(g.stack.len(), 1);
    if name == "Shivan Sand-Mage" || name == "Timebender" {
        assert_eq!(dm.target_calls, 0, "the creature spell has no targets");
        resolve(g, dm);
    }
    let source = g.find_object_by_stable_id(stable).unwrap();
    if name == "Timebender" {
        assert!(g.stack_is_empty(), "face-down entry does not turn the creature face up");
        assert!(g.is_face_down(source));
        assert_eq!((g.current_power(source), g.current_toughness(source)), (Some(2), Some(2)));
        fund(g, ManaSymbol::Blue, 1, 0);
        action(g, LegalAction::TurnFaceUp { creature_id: source, method: TurnFaceUpMethod::TurnFaceUpAbility }, dm);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0, "Morph pays exactly U");
        assert!(!g.is_face_down(source));
        assert_eq!((g.current_power(source), g.current_toughness(source)), (Some(1), Some(1)));
    }
    assert_eq!(g.stack.len(), 1);
    let entry = g.stack.last().unwrap();
    assert_eq!(entry.chosen_modes.as_deref(), Some([dm.mode].as_slice()));
    assert_eq!(entry.targets, vec![Target::Object(dm.target.unwrap())]);
    assert_eq!(dm.mode_calls, 1);
    assert_eq!(dm.target_calls, 1);
    assert_eq!(dm.number_calls, usize::from(name == "Timecrafting"));
    if name == "Timecrafting" { assert_eq!(entry.x_value, Some(dm.x)); }
    source
}

fn counter_modes(name: &str) -> Vec<(usize, bool)> {
    if name == "Fury Charm" { vec![(2, false)] } else { vec![(0, false), (1, true)] }
}

struct Targets {
    permanent: ObjectId,
    opponent_permanent: ObjectId,
    empty: ObjectId,
    charge_only: ObjectId,
    suspended: ObjectId,
    opponent_suspended: ObjectId,
    granted: ObjectId,
    opponent_granted: ObjectId,
    native_granted: ObjectId,
    opponent_native_granted: ObjectId,
    excluded: Vec<ObjectId>,
}

fn targets(g: &mut GameState, suspended_definition: &CardDefinition, grant: &CardDefinition) -> Targets {
    let permanent = witness(g, A, Zone::Battlefield, CardType::Artifact);
    let opponent_permanent = witness(g, B, Zone::Battlefield, CardType::Land);
    let empty = witness(g, B, Zone::Battlefield, CardType::Creature);
    let charge_only = witness(g, B, Zone::Battlefield, CardType::Artifact);
    g.add_counters(permanent, CounterType::Time, 5);
    g.add_counters(opponent_permanent, CounterType::Time, 5);
    g.add_counters(charge_only, CounterType::Charge, 5);
    let suspended = g.create_object_from_definition(suspended_definition, A, Zone::Exile);
    let opponent_suspended = g.create_object_from_definition(suspended_definition, B, Zone::Exile);
    let granted = witness(g, A, Zone::Exile, CardType::Artifact);
    let opponent_granted = witness(g, B, Zone::Exile, CardType::Artifact);
    for (id, recipient) in [(granted, A), (opponent_granted, B)] {
        g.effect_store.grant_registry.grant_alternative_cast_to_card(id, Zone::Exile, recipient,
            AlternativeCastingMethod::Suspend { time: SuspendTime::Fixed(4), cost: ManaCost::new() },
            GrantSource::Effect { source_id: permanent, expires_end_of_turn: g.turn.turn_number });
    }
    assert!(g.effect_store.grant_registry
        .granted_alternative_casts_for_card(g, opponent_granted, Zone::Exile, A).is_empty());
    assert_eq!(g.effect_store.grant_registry
        .granted_alternative_casts_for_card(g, opponent_granted, Zone::Exile, B).len(), 1);
    let native_granted = witness(g, A, Zone::Exile, CardType::Creature);
    let opponent_native_granted = witness(g, B, Zone::Exile, CardType::Creature);
    for id in [native_granted, opponent_native_granted] { grant_suspend(g, grant, id); }
    for id in [suspended, opponent_suspended, granted, opponent_granted, native_granted, opponent_native_granted] {
        g.add_counters(id, CounterType::Time, 5);
    }
    let no_suspend = witness(g, A, Zone::Exile, CardType::Artifact);
    g.add_counters(no_suspend, CounterType::Time, 5);
    let no_time = g.create_object_from_definition(suspended_definition, A, Zone::Exile);
    g.add_counters(no_time, CounterType::Charge, 5);
    let hidden = g.create_object_from_definition(suspended_definition, B, Zone::Exile);
    g.add_counters(hidden, CounterType::Time, 5);
    g.set_face_down(hidden);
    let mut excluded = vec![no_suspend, no_time, hidden];
    for zone in [Zone::Hand, Zone::Graveyard, Zone::Library, Zone::Command] {
        let id = g.create_object_from_definition(suspended_definition, B, zone);
        g.add_counters(id, CounterType::Time, 5);
        excluded.push(id);
    }
    g.take_pending_trigger_events();
    Targets { permanent, opponent_permanent, empty, charge_only, suspended, opponent_suspended,
        granted, opponent_granted, native_granted, opponent_native_granted, excluded }
}

#[test]
fn every_counter_mode_retains_both_domains_and_exactly_one_target_in_the_complete_body() {
    let suspend_defs = definitions("Shivan Sand-Mage");
    let grants = grant_definitions();
    for name in NAMES {
        for (route, definition) in definitions(name).into_iter().enumerate() {
            for (mode, put) in counter_modes(name) {
                // Every positive candidate is also selected in an independent game.
                for selected in 0..if put { 8 } else { 10 } {
                    let mut g = game();
                    let t = targets(&mut g, &suspend_defs[route], &grants[route]);
                    let mut legal = vec![t.permanent, t.opponent_permanent, t.suspended, t.opponent_suspended,
                        t.granted, t.opponent_granted, t.native_granted, t.opponent_native_granted];
                    let mut forbidden = t.excluded;
                    if put { forbidden.extend([t.empty, t.charge_only]); }
                    else { legal.extend([t.empty, t.charge_only]); }
                    let target = legal[selected];
                    let before = g.counter_count(target, CounterType::Time);
                    let mut dm = Choices { mode, x: 3, target: Some(target), required: legal.clone(), forbidden, ..Default::default() };
                    announce(&mut g, name, &definition, &mut dm);
                    resolve(&mut g, &mut dm);
                    let amount = if name == "Timecrafting" { 3 } else { 2 };
                    assert_eq!(g.counter_count(target, CounterType::Time), if put { before + amount } else { before.saturating_sub(amount) }, "{name}, mode {mode}");
                    for other in legal.into_iter().filter(|id| *id != target) {
                        assert_eq!(g.counter_count(other, CounterType::Time), if other == t.empty || other == t.charge_only { 0 } else { 5 });
                    }
                    assert_eq!(g.counter_count(t.charge_only, CounterType::Charge), 5);
                    assert!(g.stack_is_empty());
                    assert_eq!(dm.cast_offers, 0);
                }
            }
        }
    }
}

#[test]
fn fury_destroy_and_pump_are_separate_paid_modes_and_both_pump_parts_expire() {
    for definition in definitions("Fury Charm") {
        for mode in [0, 1] {
            let mut g = game();
            let artifact = witness(&mut g, B, Zone::Battlefield, CardType::Artifact);
            let creature = witness(&mut g, B, Zone::Battlefield, CardType::Creature);
            let target = if mode == 0 { artifact } else { creature };
            let identity = g.object(target).unwrap().stable_id;
            let other = if mode == 0 { creature } else { artifact };
            let mut dm = Choices { mode, target: Some(target), forbidden: vec![other], ..Default::default() };
            announce(&mut g, "Fury Charm", &definition, &mut dm);
            resolve(&mut g, &mut dm);
            if mode == 0 {
                let destroyed = g.find_object_by_stable_id(identity).unwrap();
                assert_eq!(g.object(destroyed).unwrap().zone, Zone::Graveyard);
                assert_eq!((g.current_power(creature), g.current_toughness(creature)), (Some(2), Some(3)));
            } else {
                assert_eq!((g.current_power(creature), g.current_toughness(creature)), (Some(3), Some(4)));
                assert!(g.current_has_static_ability_id(creature, StaticAbilityId::Trample));
                assert_eq!(g.object(artifact).unwrap().zone, Zone::Battlefield);
                ironsmith::turn::execute_cleanup_step(&mut g);
                g.refresh_continuous_state().unwrap();
                assert_eq!((g.current_power(creature), g.current_toughness(creature)), (Some(2), Some(3)));
                assert!(!g.current_has_static_ability_id(creature, StaticAbilityId::Trample));
            }
            assert!(g.stack_is_empty());
        }
    }
}

#[test]
fn timecrafting_pays_announced_x_once_including_zero_without_erasing_target_restrictions() {
    for definition in definitions("Timecrafting") {
        for (mode, put) in counter_modes("Timecrafting") {
            for x in [0, 1, 4, 9] {
                let mut g = game();
                let target = witness(&mut g, B, Zone::Battlefield, CardType::Artifact);
                let empty = witness(&mut g, A, Zone::Battlefield, CardType::Land);
                g.add_counters(target, CounterType::Time, 5);
                let mut dm = Choices { mode, x, target: Some(target),
                    required: if put { vec![target] } else { vec![target, empty] },
                    forbidden: if put { vec![empty] } else { vec![] }, ..Default::default() };
                announce(&mut g, "Timecrafting", &definition, &mut dm);
                resolve(&mut g, &mut dm);
                assert_eq!(g.counter_count(target, CounterType::Time), if put { 5 + x } else { 5u32.saturating_sub(x) });
                assert_eq!(g.counter_count(empty, CounterType::Time), 0);
                assert_eq!(dm.number_calls, 1);
                assert_eq!(dm.target_calls, 1, "X=0 still requires the announced legal target");
                assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
            }
        }
    }
}

#[test]
fn counterless_permanents_leave_only_removal_available_even_when_x_is_zero() {
    for name in NAMES {
        for definition in definitions(name) {
            let mut g = game();
            let target = witness(&mut g, B, Zone::Battlefield, CardType::Creature);
            let mut dm = Choices {
                mode: if name == "Fury Charm" { 2 } else { 0 },
                target: Some(target),
                illegal_modes: if name == "Fury Charm" { vec![0] } else { vec![1] },
                ..Default::default()
            };
            announce(&mut g, name, &definition, &mut dm);
            resolve(&mut g, &mut dm);
            assert_eq!(g.counter_count(target, CounterType::Time), 0);
            assert!(g.stack_is_empty());
        }
    }
}

#[test]
fn battlefield_rechecking_keeps_the_counter_qualifier_on_the_put_arm_only() {
    for name in NAMES {
        for definition in definitions(name) {
            for (mode, put) in counter_modes(name) {
                let mut g = game();
                let target = witness(&mut g, B, Zone::Battlefield, CardType::Artifact);
                g.add_counters(target, CounterType::Time, 1);
                let mut dm = Choices { mode, x: 3, target: Some(target), ..Default::default() };
                announce(&mut g, name, &definition, &mut dm);
                let (_, event) = g.remove_counters(target, CounterType::Time, 1, None, Some(B)).unwrap();
                g.queue_trigger_event(Default::default(), event);
                queue(&mut g, &mut dm);
                assert_eq!(g.stack.len(), 1);
                let entry = g.stack.last().unwrap();
                let program = entry.ability_effects.as_ref().or(definition.spell_effect.as_ref()).unwrap();
                let requirements = extract_target_requirements_from_program_with_modes(&g, program,
                    entry.controller, Some(entry.object_id), entry.chosen_modes.as_deref());
                assert_eq!(requirements.len(), 1);
                assert_eq!(requirements[0].legal_targets.contains(&Target::Object(target)), !put);
                resolve(&mut g, &mut dm);
                assert_eq!(g.counter_count(target, CounterType::Time), 0);
                assert!(g.stack_is_empty());
            }
        }
    }
}

#[test]
fn timebender_normal_blue_cast_has_no_face_up_trigger() {
    for definition in definitions("Timebender") {
        let mut g = game();
        let target = witness(&mut g, B, Zone::Battlefield, CardType::Artifact);
        g.add_counters(target, CounterType::Time, 5);
        let source = g.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = g.object(source).unwrap().stable_id;
        let mut dm = Choices { mode: 1, target: Some(target), ..Default::default() };
        fund(&mut g, ManaSymbol::Blue, 1, 0);
        cast(&mut g, source, CastingMethod::Normal, &mut dm);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut g, &mut dm);
        let entered = g.find_object_by_stable_id(stable).unwrap();
        assert_eq!((g.current_power(entered), g.current_toughness(entered)), (Some(1), Some(1)));
        assert!(!g.is_face_down(entered));
        assert_eq!(g.counter_count(target, CounterType::Time), 5);
        assert_eq!(dm.mode_calls, 0);
        assert_eq!(dm.target_calls, 0);
        assert!(g.stack_is_empty(), "normal ETB is not a turned-face-up event");
    }
}

#[test]
fn losing_the_last_time_counter_in_response_invalidates_the_exile_arm_before_resolution() {
    let suspend_defs = definitions("Shivan Sand-Mage");
    for name in NAMES {
        for (route, definition) in definitions(name).into_iter().enumerate() {
            for (mode, _) in counter_modes(name) {
                let mut g = game();
                let target = g.create_object_from_definition(&suspend_defs[route], B, Zone::Exile);
                g.add_counters(target, CounterType::Time, 1);
                let mut dm = Choices { mode, x: 3, target: Some(target), ..Default::default() };
                announce(&mut g, name, &definition, &mut dm);
                let (removed, event) = g.remove_counters(target, CounterType::Time, 1, None, Some(B)).unwrap();
                assert_eq!(removed, 1);
                g.queue_trigger_event(Default::default(), event);
                queue(&mut g, &mut dm);
                assert_eq!(g.stack.len(), 2, "one real last-counter trigger above the original mode");
                resolve(&mut g, &mut dm); // Opponent declines its cast; card stays in exile.
                assert_eq!(dm.cast_offers, 1);
                assert_eq!(g.stack.len(), 1);
                let entry = g.stack.last().unwrap();
                let program = entry.ability_effects.as_ref().or(definition.spell_effect.as_ref()).unwrap();
                let requirements = extract_target_requirements_from_program_with_modes(&g, program,
                    entry.controller, Some(entry.object_id), entry.chosen_modes.as_deref());
                assert_eq!(requirements.len(), 1);
                assert!(!requirements[0].legal_targets.contains(&Target::Object(target)));
                resolve(&mut g, &mut dm);
                assert_eq!(g.object(target).unwrap().zone, Zone::Exile);
                assert_eq!(g.counter_count(target, CounterType::Time), 0, "put cannot recreate suspended status");
                assert_eq!(dm.cast_offers, 1);
                assert!(g.stack_is_empty());
            }
        }
    }
}

#[test]
fn each_removal_body_casts_a_suspended_card_once_when_removing_its_last_counter() {
    let suspend_defs = definitions("Shivan Sand-Mage");
    for name in NAMES {
        for (route, definition) in definitions(name).into_iter().enumerate() {
            for initial in [1, 2] {
                let mut g = game();
                let target = g.create_object_from_definition(&suspend_defs[route], B, Zone::Exile);
                let stable = g.object(target).unwrap().stable_id;
                g.add_counters(target, CounterType::Time, initial);
                let followup = witness(&mut g, A, Zone::Battlefield, CardType::Artifact);
                g.add_counters(followup, CounterType::Time, 5);
                let mode = if name == "Fury Charm" { 2 } else { 0 };
                let mut dm = Choices { mode, x: 2, target: Some(target), accept_cast: true, ..Default::default() };
                announce(&mut g, name, &definition, &mut dm);
                resolve(&mut g, &mut dm);
                assert_eq!(g.counter_count(target, CounterType::Time), 0);
                assert_eq!(g.stack.len(), 1, "a multi-counter removal emits one last-counter cast trigger");
                assert!(g.stack[0].is_ability);
                assert_eq!(g.stack[0].controller, B);
                resolve(&mut g, &mut dm);
                assert_eq!(dm.cast_offers, 1);
                assert_eq!(g.stack.len(), 1);
                assert!(!g.stack[0].is_ability, "the optional cast creates a real spell");
                assert_eq!(g.stack[0].controller, B);
                // The newly cast full Shivan body also has its own real ETB mode.
                dm.mode = 1;
                dm.target = Some(followup);
                resolve(&mut g, &mut dm);
                assert_eq!(g.stack.len(), 1);
                let entered = g.find_object_by_stable_id(stable).unwrap();
                assert_eq!(g.object(entered).unwrap().zone, Zone::Battlefield);
                assert!(g.current_has_static_ability_id(entered, StaticAbilityId::Haste));
                resolve(&mut g, &mut dm);
                assert_eq!(g.counter_count(followup, CounterType::Time), 7);
                queue(&mut g, &mut dm);
                assert!(g.stack_is_empty());
                assert_eq!(dm.cast_offers, 1);
                assert_eq!(dm.mode_calls, 2);
                assert_eq!(g.player(B).unwrap().mana_pool.total(), 0, "Suspend cast pays no printed mana cost");
            }
        }
    }
}

#[test]
fn shivan_pays_suspend_four_red_upkeeps_then_casts_and_runs_either_etb_mode_with_haste() {
    for definition in definitions("Shivan Sand-Mage") {
        for mode in [0, 1] {
            let mut g = game();
            let target = witness(&mut g, B, Zone::Battlefield, CardType::Artifact);
            g.add_counters(target, CounterType::Time, 5);
            let source = g.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = g.object(source).unwrap().stable_id;
            let mut dm = Choices { mode, target: Some(target), accept_cast: true, ..Default::default() };
            fund(&mut g, ManaSymbol::Red, 1, 0);
            special_actions::perform(SpecialAction::Suspend { card_id: source }, &mut g, A, &mut dm).unwrap();
            let exiled = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!(g.object(exiled).unwrap().zone, Zone::Exile);
            assert_eq!(g.counter_count(exiled, CounterType::Time), 4);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
            assert!(g.stack_is_empty());
            assert_eq!(dm.number_calls, 0, "Suspend 4 is fixed");
            for remaining in [3, 2, 1, 0] {
                g.turn.turn_number += 1;
                g.turn.phase = Phase::Beginning;
                g.turn.step = Some(Step::Upkeep);
                g.turn.active_player = B;
                let mut triggers = TriggerQueue::new();
                generate_and_queue_step_triggers(&mut g, &mut triggers);
                assert!(triggers.entries.is_empty(), "opponent upkeep does not remove your time counter");
                g.turn.active_player = A;
                generate_and_queue_step_triggers(&mut g, &mut triggers);
                assert_eq!(triggers.entries.len(), 1);
                put_triggers_on_stack_with_dm(&mut g, &mut triggers, &mut dm).unwrap();
                resolve(&mut g, &mut dm);
                assert_eq!(g.counter_count(exiled, CounterType::Time), remaining);
                assert_eq!(g.stack.len(), usize::from(remaining == 0));
            }
            resolve(&mut g, &mut dm); // Optional last-counter cast.
            assert_eq!(dm.cast_offers, 1);
            assert_eq!(g.stack.len(), 1);
            assert!(!g.stack[0].is_ability);
            resolve(&mut g, &mut dm); // Creature enters and chooses its ETB mode/target.
            assert_eq!(dm.mode_calls, 1);
            assert_eq!(dm.target_calls, 1);
            resolve(&mut g, &mut dm);
            assert_eq!(g.counter_count(target, CounterType::Time), if mode == 0 { 3 } else { 7 });
            let entered = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!((g.current_power(entered), g.current_toughness(entered)), (Some(3), Some(2)));
            assert!(g.current_has_static_ability_id(entered, StaticAbilityId::Haste));
            assert!(g.stack_is_empty());
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
            g.set_current_controller(entered, B).unwrap();
            assert!(!g.current_has_static_ability_id(entered, StaticAbilityId::Haste), "Suspend haste lasts only while continuously controlled");
        }
    }
}

#[test]
fn actual_granted_suspend_casts_for_its_owner_once_and_has_no_printed_method_record() {
    for (route, grant) in grant_definitions().into_iter().enumerate() {
        for owner in [A, B] {
            let mut g = game();
            let target = witness(&mut g, owner, Zone::Exile, CardType::Creature);
            let stable = g.object(target).unwrap().stable_id;
            grant_suspend(&mut g, &grant, target);
            g.add_counters(target, CounterType::Time, 2);
            let fury = definitions("Fury Charm").into_iter().nth(route).unwrap();
            let mut dm = Choices { mode: 2, target: Some(target), accept_cast: true, ..Default::default() };
            announce(&mut g, "Fury Charm", &fury, &mut dm);
            resolve(&mut g, &mut dm);
            assert_eq!(g.stack.len(), 1);
            assert_eq!(g.stack[0].controller, owner);
            assert!(g.stack[0].is_ability);
            resolve(&mut g, &mut dm);
            assert_eq!(dm.cast_offers, 1);
            assert_eq!(g.stack.len(), 1);
            assert!(!g.stack[0].is_ability);
            assert_eq!(g.stack[0].controller, owner);
            resolve(&mut g, &mut dm);
            let entered = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!(g.object(entered).unwrap().zone, Zone::Battlefield);
            assert_eq!(g.current_controller(entered), Some(owner));
            assert!(g.current_has_static_ability_id(entered, StaticAbilityId::Haste));
            assert!(g.stack_is_empty());
            assert_eq!(dm.cast_offers, 1);
        }
    }
}

#[test]
fn granted_suspend_uses_current_abilities_and_disappears_on_expiry_loss_or_concealment() {
    use ironsmith::continuous::Modification;
    use ironsmith::effect::Until;
    use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
    use ironsmith::target::ChooseSpec;
    for (route, grant) in grant_definitions().into_iter().enumerate() {
        for loss in ["cleanup", "abilities", "face down"] {
            let mut g = game();
            let target = witness(&mut g, B, Zone::Exile, CardType::Creature);
            grant_suspend(&mut g, &grant, target);
            g.add_counters(target, CounterType::Time, 5);
            let legal = witness(&mut g, A, Zone::Battlefield, CardType::Artifact);
            match loss {
                "cleanup" => { ironsmith::turn::execute_cleanup_step(&mut g); }
                "abilities" => {
                    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(target),
                        Modification::RemoveAllAbilities, Until::EndOfTurn)
                        .execute(&mut g, &mut EffectContext::new_default(legal, A)).unwrap();
                }
                "face down" => { assert!(g.set_face_down(target)); }
                _ => unreachable!(),
            }
            g.refresh_continuous_state().unwrap();
            let fury = definitions("Fury Charm").into_iter().nth(route).unwrap();
            let mut dm = Choices { mode: 2, target: Some(legal), forbidden: vec![target], ..Default::default() };
            announce(&mut g, "Fury Charm", &fury, &mut dm);
            resolve(&mut g, &mut dm);
            assert_eq!(g.counter_count(target, CounterType::Time), 5, "{loss}");
        }
    }
}

#[test]
fn retained_native_suspend_bodies_preserve_typed_identity_without_labels_or_new_fields() {
    use ironsmith::ability::{AbilityKind, PresentationKeyword, PresentationLabel};
    use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_ability, restore_runtime_ability};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::CastSourceEffect;
    for restored in [false, true] {
        let mut legacy = if restored {
            definitions("Shivan Sand-Mage").into_iter().nth(1).unwrap()
        } else {
            CardDefinitionBuilder::new(CardId::new(), "Retained native Suspend grant")
                .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 3))
                .suspend(4, ManaCost::new()).build()
        };
        // This is the pre-correction costless grant carrier: only two executable
        // abilities, no marker, no alternative method, and no surface labels.
        legacy.alternative_casts.clear();
        legacy.abilities.retain(|ability| ability.functional_zones == [Zone::Exile]);
        assert_eq!(legacy.abilities.len(), 2);
        for ability in &mut legacy.abilities {
            let AbilityKind::Triggered(triggered) = &mut ability.kind else { panic!("Suspend trigger"); };
            triggered.presentation_label = None;
        }
        if restored {
            legacy.abilities = legacy.abilities.into_iter().map(|ability| {
                let encoded = encode_runtime_ability(ability).unwrap();
                let serialized = serde_json::to_vec(&encoded).unwrap();
                restore_runtime_ability(serde_json::from_slice(&serialized).unwrap()).unwrap()
            }).collect();
        }
        let mut lookalike = legacy.clone();
        lookalike.card.id = CardId::new();
        for ability in &mut lookalike.abilities {
            let AbilityKind::Triggered(triggered) = &mut ability.kind else { unreachable!() };
            triggered.presentation_label = Some(PresentationLabel::Keyword(PresentationKeyword::Suspend));
            triggered.effects = vec![ironsmith::Effect::may(vec![ironsmith::Effect::new(
                CastSourceEffect::new().without_paying_mana_cost().require_exile(),
            )])].into();
        }
        let mut g = game();
        let target = g.create_object_from_definition(&legacy, B, Zone::Exile);
        let mislabeled = g.create_object_from_definition(&lookalike, B, Zone::Exile);
        g.add_counters(target, CounterType::Time, 5);
        g.add_counters(mislabeled, CounterType::Time, 5);
        let fury = definitions("Fury Charm").into_iter().nth(usize::from(restored)).unwrap();
        let mut dm = Choices { mode: 2, target: Some(target), forbidden: vec![mislabeled], ..Default::default() };
        announce(&mut g, "Fury Charm", &fury, &mut dm);
        resolve(&mut g, &mut dm);
        assert_eq!(g.counter_count(target, CounterType::Time), 3);
        assert_eq!(g.counter_count(mislabeled, CounterType::Time), 5);
    }
}

#[test]
fn another_ability_or_execution_source_cannot_supply_the_outer_cards_suspend_identity() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::continuous::Modification;
    use ironsmith::effect::Until;
    use ironsmith::effects::{ApplyContinuousEffect, CastSourceEffect, ExecuteWithSourceEffect, ScheduleDelayedTriggerEffect};
    use ironsmith::target::{ChooseSpec, PlayerFilter};
    for (route, definition) in definitions("Shivan Sand-Mage").into_iter().enumerate() {
        for scope in ["granted ability", "delayed program", "different source"] {
            let mut g = game();
            let legal = witness(&mut g, A, Zone::Battlefield, CardType::Artifact);
            let recipient = witness(&mut g, B, Zone::Exile, CardType::Creature);
            let mut outer = definition.clone();
            outer.card.id = CardId::new();
            outer.alternative_casts.clear();
            outer.abilities.retain(|ability| ability.functional_zones == [Zone::Exile]);
            assert_eq!(outer.abilities.len(), 2);
            let inner_suspend_trigger = outer.abilities[1].clone();
            let cast_as_suspend = ironsmith::Effect::new(CastSourceEffect::new()
                .without_paying_mana_cost().require_exile().cast_as_suspend());
            let nested = match scope {
                "granted ability" => ironsmith::Effect::new(ApplyContinuousEffect::with_spec(
                    ChooseSpec::SpecificObject(recipient),
                    Modification::AddAbilityGeneric(inner_suspend_trigger), Until::EndOfTurn,
                )),
                "delayed program" => ironsmith::Effect::new(ScheduleDelayedTriggerEffect::new(
                    ironsmith::triggers::Trigger::beginning_of_end_step(PlayerFilter::You),
                    vec![cast_as_suspend], true, vec![], PlayerFilter::You,
                )),
                "different source" => ironsmith::Effect::new(ExecuteWithSourceEffect::new(
                    ChooseSpec::SpecificObject(recipient), cast_as_suspend,
                )),
                _ => unreachable!(),
            };
            for ability in &mut outer.abilities {
                let AbilityKind::Triggered(triggered) = &mut ability.kind else { unreachable!() };
                triggered.effects = vec![ironsmith::Effect::may(vec![nested.clone()])].into();
            }
            let outer = g.create_object_from_definition(&outer, B, Zone::Exile);
            g.add_counters(outer, CounterType::Time, 5);
            let fury = definitions("Fury Charm").into_iter().nth(route).unwrap();
            let mut dm = Choices { mode: 2, target: Some(legal), forbidden: vec![outer], ..Default::default() };
            announce(&mut g, "Fury Charm", &fury, &mut dm);
            resolve(&mut g, &mut dm);
            assert_eq!(g.counter_count(outer, CounterType::Time), 5, "{scope}");
            assert_eq!(g.object(recipient).unwrap().zone, Zone::Exile);
            assert_eq!(dm.cast_offers, 0);
        }
    }
}
