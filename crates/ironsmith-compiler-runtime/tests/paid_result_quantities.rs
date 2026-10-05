//! Frozen full-body scenarios, authored and UNRUN in the implementation-first campaign.
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{DistributeContext, NumberContext, SelectObjectsContext, TargetsContext};
use ironsmith::effect::{Effect, EffectId, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{AttackEventTarget, TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const COMPLETE: &[&str] = &["Bishop of Binding", "Essence Bottle", "Ooze Flux", "Vish Kal, Blood Arbiter"];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/paid_result_quantities.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text += &format!("Power/Toughness: {p}/{t}\n"); }
    text += row["oracle_text"].as_str().unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = Phase::FirstMain; game.turn.step = None;
    game.turn.active_player = A; game.turn.priority_player = Some(A); game
}
fn creature(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, p: i32, t: i32, subtype: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, format!("Type: Creature — {subtype}\nPower/Toughness: {p}/{t}"), false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn mana(game: &mut GameState, color: ManaSymbol, amount: u32) { game.player_mut(A).unwrap().mana_pool.add(color, amount); }
#[derive(Default)]
struct Choices { quantity: u32, target: Option<Target>, pick: Option<ObjectId>, distribution: Vec<(Target, u32)> }
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        assert!(!ctx.is_x_value, "a printed result variable is not announced mana X");
        assert!(self.quantity >= ctx.min && self.quantity <= ctx.max); self.quantity
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(ctx.requirements.iter().any(|r| r.legal_targets.contains(&target))); vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, ctx) }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(pick) = self.pick.filter(|id| ctx.candidates.iter().any(|c| c.id == *id && c.legal)) { vec![pick] }
        else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
    }
    fn decide_distribute(&mut self, _: &GameState, ctx: &DistributeContext) -> Vec<(Target, u32)> {
        assert_eq!(self.distribution.iter().map(|(_, n)| n).sum::<u32>(), ctx.total);
        assert!(self.distribution.iter().all(|(target, _)| ctx.targets.iter().any(|entry| entry.target == *target)));
        self.distribution.clone()
    }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { assert_eq!(game.stack.len(), 1); return; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        *game = game.clone(); state = state.clone();
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("announcement did not finish");
}
fn activation(game: &GameState, source: ObjectId, ordinal: usize) -> Option<LegalAction> {
    let index = game.current_abilities(source)?.iter().enumerate()
        .filter(|(_, ability)| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .nth(ordinal)?.0;
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index))
}
fn activate(game: &mut GameState, source: ObjectId, ordinal: usize, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = activation(game, source, ordinal).expect("real activation is legal"); announce(game, action, dm);
}
fn paid(game: &GameState, amount: u32) {
    let entry = game.stack.last().unwrap(); assert_eq!(entry.x_value, None);
    assert_eq!(entry.effect_outcomes[&EffectId::ACTIVATION_COUNTER_COST].instruction_result().count_or_zero(), i64::from(amount));
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) {
    execute_effect(game, &effect, &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
}
fn queue_event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) -> usize {
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) { queue.add(entry); }
    let count = queue.entries.len(); put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap(); count
}
fn ooze(game: &GameState) -> ObjectId {
    let tokens: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|o|
        o.kind == ObjectKind::Token && game.calculated_subtypes(*id).contains(&Subtype::Ooze))).collect();
    assert_eq!(tokens.len(), 1); let id = tokens[0];
    assert_eq!(game.current_controller(id), Some(A)); assert_eq!(game.object(id).unwrap().colors(), ColorSet::GREEN); id
}

#[test]
fn full_frozen_bodies_retain_all_abilities_and_do_not_leave_pending_quantities() {
    for name in COMPLETE { for definition in definitions(name) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(!debug.contains("PendingPriorEffectMetric"), "{name}: {debug}");
        let expected = if *name == "Ooze Flux" { 1 } else { 2 };
        let count = definition.abilities.iter().filter(|ability| match ability.kind {
            ironsmith::ability::AbilityKind::Triggered(_) => *name == "Bishop of Binding",
            ironsmith::ability::AbilityKind::Activated(_) => *name != "Bishop of Binding",
            _ => false,
        }).count();
        assert_eq!(count, expected, "{name}: every printed action survives");
    }}
}

#[test]
fn essence_bottle_both_activations_read_only_actual_elixir_payment_and_accept_known_zero() {
    for definition in definitions("Essence Bottle") { for count in [0, 3] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let kind = CounterType::Named("elixir".into()); let mut dm = Choices::default();
        mana(&mut game, ManaSymbol::Colorless, 3); activate(&mut game, source, 0, &mut dm);
        assert!(game.is_tapped(source)); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert_eq!(game.counter_count(source, kind), 1);
        game.remove_counters(source, kind, 1, None, None); game.add_counters(source, kind, count);
        game.add_counters(source, CounterType::Charge, 9); game.untap(source);
        activate(&mut game, source, 1, &mut dm); paid(&game, count); assert_eq!(game.counter_count(source, kind), 0);
        assert_eq!(game.counter_count(source, CounterType::Charge), 9);
        let activation_id = game.stack.last().unwrap().ability_id.unwrap();
        apply(&mut game, source, Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(activation_id))));
        assert_eq!(game.stack.len(), 2); paid(&game, count);
        apply(&mut game, source, Effect::move_to_zone(ChooseSpec::Source, Zone::Graveyard, false));
        game = game.clone();
        for _ in 0..2 { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        assert_eq!(game.player(A).unwrap().life, 20 + 4 * count as i32, "the copy retains the same actual payment without paying again");
    }}
}

#[test]
fn ooze_flux_distributed_payment_restricts_kind_control_and_zone_and_keeps_actual_result() {
    use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    for definition in definitions("Ooze Flux") { for modified in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = creature(&mut game, A, Zone::Battlefield, "First payer", 1, 3, "Elf");
        let second = creature(&mut game, A, Zone::Battlefield, "Second payer", 1, 3, "Elf");
        let foreign = creature(&mut game, B, Zone::Battlefield, "Foreign", 1, 3, "Elf");
        let buried = creature(&mut game, A, Zone::Graveyard, "Buried", 1, 3, "Elf");
        for id in [first, second, foreign, buried] { game.add_counters(id, CounterType::PlusOnePlusOne, 4); }
        game.add_counters(first, CounterType::Charge, 8);
        if modified { game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::counters::matchers::WouldRemoveCountersMatcher::any(), ReplacementAction::Modify(EventModification::Subtract(1)))); }
        mana(&mut game, ManaSymbol::Green, 1); mana(&mut game, ManaSymbol::Colorless, 1);
        let mut dm = Choices { quantity: 3, distribution: vec![(Target::Object(first), 2), (Target::Object(second), 1)], ..Default::default() };
        activate(&mut game, source, 0, &mut dm); let actual = if modified { 2 } else { 3 }; paid(&game, actual);
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne) + game.counter_count(second, CounterType::PlusOnePlusOne), 8 - actual);
        assert_eq!(game.counter_count(foreign, CounterType::PlusOnePlusOne), 4); assert_eq!(game.counter_count(buried, CounterType::PlusOnePlusOne), 4);
        assert_eq!(game.counter_count(first, CounterType::Charge), 8); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        apply(&mut game, source, Effect::move_to_zone(ChooseSpec::Source, Zone::Graveyard, false));
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap(); let token = ooze(&game);
        assert_eq!((game.current_power(token), game.current_toughness(token)), (Some(actual as i32), Some(actual as i32)));
    }}
}

#[test]
fn ooze_flux_cannot_pay_zero_or_borrow_foreign_noncreature_or_wrong_kind_counters() {
    for definition in definitions("Ooze Flux") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        mana(&mut game, ManaSymbol::Green, 1); mana(&mut game, ManaSymbol::Colorless, 1);
        let own = creature(&mut game, A, Zone::Battlefield, "Own", 1, 3, "Elf");
        let foreign = creature(&mut game, B, Zone::Battlefield, "Foreign", 1, 3, "Elf");
        game.add_counters(own, CounterType::Charge, 5); game.add_counters(source, CounterType::PlusOnePlusOne, 5);
        game.add_counters(foreign, CounterType::PlusOnePlusOne, 5);
        assert!(activation(&game, source, 0).is_none()); assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
        assert_eq!(game.counter_count(foreign, CounterType::PlusOnePlusOne), 5);
    }
}

#[test]
fn vish_kal_uses_sacrifice_lki_then_paid_counter_receipt_even_when_source_or_target_leaves() {
    for definition in definitions("Vish Kal, Blood Arbiter") { for target_leaves in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = creature(&mut game, A, Zone::Battlefield, "Sacrificed", 2, 3, "Elf");
        let stable = game.object(victim).unwrap().stable_id;
        apply(&mut game, source, Effect::pump(3, 3, ChooseSpec::SpecificObject(victim), Until::EndOfTurn));
        let mut dm = Choices { pick: Some(victim), ..Default::default() }; activate(&mut game, source, 0, &mut dm);
        let grave = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard);
        let returned = game.move_object_by_game_rule(grave, Zone::Battlefield).unwrap();
        game.add_counters(returned, CounterType::PlusOnePlusOne, 20);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 5);
        let target = creature(&mut game, B, Zone::Battlefield, "Debuffed", 8, 9, "Soldier");
        dm.target = Some(Target::Object(target)); activate(&mut game, source, 1, &mut dm); paid(&game, 5);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
        if target_leaves { game.move_object_by_game_rule(target, Zone::Hand).unwrap(); }
        apply(&mut game, source, Effect::move_to_zone(ChooseSpec::Source, Zone::Graveyard, false));
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        if !target_leaves { assert_eq!((game.current_power(target), game.current_toughness(target)), (Some(3), Some(4))); }
        assert!(game.stack.is_empty());
    }}
}

#[test]
fn bishop_entry_links_exact_opponent_and_attack_reads_exile_characteristic_until_departure() {
    for definition in definitions("Bishop of Binding") { for scenario in 0..3 {
        let link_leaves = scenario == 1;
        let mut game = game(); let victim = creature(&mut game, B, Zone::Battlefield, "Exiled victim", 3, 7, "Soldier");
        let victim_stable = game.object(victim).unwrap().stable_id;
        let recipient = creature(&mut game, A, Zone::Battlefield, "Vampire recipient", 2, 4, "Vampire");
        let source = game.create_object_from_definition(&definition, A, Zone::Hand); let stable = game.object(source).unwrap().stable_id;
        mana(&mut game, ManaSymbol::White, 1); mana(&mut game, ManaSymbol::Colorless, 3);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
        let mut dm = Choices { target: Some(Target::Object(victim)), ..Default::default() }; announce(&mut game, action, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); let source = game.find_object_by_stable_id(stable).unwrap();
        let mut queue = TriggerQueue::new(); drain_pending_trigger_events(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap(); assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); let linked = game.find_object_by_stable_id(victim_stable).unwrap();
        assert_eq!(game.object(linked).unwrap().zone, Zone::Exile); assert_eq!(game.get_exiled_with_source_links(source), &[linked]);
        dm.target = Some(Target::Object(recipient));
        let event = TriggerEvent::new_with_provenance(ironsmith::events::combat::CreatureAttackedEvent::new(source, AttackEventTarget::Player(B)), Default::default());
        assert_eq!(queue_event(&mut game, event, &mut dm), 1);
        if link_leaves { game.move_object_by_game_rule(linked, Zone::Hand).unwrap(); }
        if scenario == 2 { assert!(game.set_face_down(linked)); }
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let power = if scenario == 0 { 5 } else { 2 }; assert_eq!(game.current_power(recipient), Some(power));
        apply(&mut game, source, Effect::move_to_zone(ChooseSpec::Source, Zone::Graveyard, false));
        let current = game.find_object_by_stable_id(victim_stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone, if link_leaves { Zone::Hand } else { Zone::Battlefield });
    }}
}

#[test]
fn linked_power_reference_never_consumes_an_unrelated_prior_draw() {
    let text = "Type: Creature — Vampire\nPower/Toughness: 1/1\nWhenever this creature attacks, draw seven cards. Target Vampire gets +X/+X until end of turn, where X is the power of the exiled card.";
    let definition = compile_to_runtime_definition("Linked reference witness", text, false).unwrap();
    let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let recipient = creature(&mut game, A, Zone::Battlefield, "Vampire", 2, 4, "Vampire");
    for _ in 0..8 { creature(&mut game, A, Zone::Library, "Library witness", 1, 1, "Elf"); }
    let mut dm = Choices { target: Some(Target::Object(recipient)), ..Default::default() };
    let event = TriggerEvent::new_with_provenance(ironsmith::events::combat::CreatureAttackedEvent::new(source, AttackEventTarget::Player(B)), Default::default());
    assert_eq!(queue_event(&mut game, event, &mut dm), 1); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(game.player(A).unwrap().hand.len(), 7); assert_eq!(game.current_power(recipient), Some(2));
}

#[test]
fn recognized_result_references_reject_dropped_symbols_and_incomplete_counter_descriptors() {
    for binding in [
        "the power {U} of the exiled card", "the power of the exiled card {U}",
        "the power: of the exiled card", "the power of the exiled card:",
        "the power {U} of the exiled cards", "the power of the exiled cards {U}",
        "the power: of the exiled cards", "the power of the exiled cards:",
        "the number of charge {U} counters removed this way", "the number of charge counters removed this way {U}",
        "the number of charge: counters removed this way", "the number of charge counters removed this way:",
        "the number of bogus charge counters removed this way",
    ] {
        let text = format!("Type: Creature\nPower/Toughness: 1/1\nRemove all charge counters from this creature: Target creature gets +X/+X until end of turn, where X is {binding}.");
        assert!(compile_to_runtime_definition("Malformed result witness", &text, false).is_err(), "{text}");
        assert!(compile_to_artifact("Malformed result witness", &text, false).is_err(), "artifact: {text}");
    }
    let text = "Type: Creature\nPower/Toughness: 1/1\nRemove all +1/+1 counters from this creature: Target creature gets +X/+X until end of turn, where X is the number of +1/+1 counters removed this way.";
    assert!(compile_to_runtime_definition("Typed counter witness", text, false).is_ok());
    let mismatch = text.replacen("number of +1/+1 counters", "number of charge counters", 1);
    assert!(compile_to_runtime_definition("Typed counter witness", &mismatch, false).is_err());
}

#[test]
fn vish_kal_actual_sacrifice_is_distinct_from_paid_selection_and_replacement_added_sacrifices() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Vish Kal, Blood Arbiter") { for scenario in 0..4 {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = creature(&mut game, A, Zone::Battlefield, "Original selected", 3, 4, "Elf");
        let added = creature(&mut game, A, Zone::Battlefield, "Replacement-only", 11, 12, "Elf");
        let stable = game.object(original).unwrap().stable_id;
        apply(&mut game, source, Effect::pump(2, 2, ChooseSpec::SpecificObject(original), Until::EndOfTurn));
        let sacrifice_added = Effect::new(ironsmith::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(added)));
        let action = match scenario {
            0 => ReplacementAction::Prevent,
            1 => ReplacementAction::ChangeDestination(Zone::Exile),
            2 => ReplacementAction::Instead(vec![sacrifice_added]),
            _ => ReplacementAction::Additionally(vec![sacrifice_added]),
        };
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(original), Some(Zone::Battlefield), Some(Zone::Graveyard)), action));
        let mut dm = Choices { pick: Some(original), ..Default::default() };
        activate(&mut game, source, 0, &mut dm);
        let entry = game.stack.last().unwrap();
        let selected = ironsmith_core::tag::SacrificeCostTag::Selected(0).key();
        let actual = ironsmith_core::tag::SacrificeCostTag::OriginalResult(0).key();
        assert_eq!(entry.tagged_objects[&selected][0].stable_id, stable);
        assert_eq!(entry.tagged_objects[&actual].len(), usize::from(scenario == 1 || scenario == 3));
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone, match scenario { 1 => Zone::Exile, 3 => Zone::Graveyard, _ => Zone::Battlefield });
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), if scenario == 1 || scenario == 3 { 5 } else { 0 });
    }}
}

#[test]
fn tom_full_body_draws_only_the_original_sacrificed_creature_power_then_always_discards() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Tom, Bert, and William") { for scenario in 0..5 {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = creature(&mut game, A, Zone::Battlefield, "Original selected", 3, 4, "Elf");
        let added = creature(&mut game, A, Zone::Battlefield, "Replacement-only", 11, 12, "Elf");
        creature(&mut game, A, Zone::Hand, "Initial hand card", 1, 1, "Elf");
        for _ in 0..8 { creature(&mut game, A, Zone::Library, "Library card", 1, 1, "Elf"); }
        apply(&mut game, source, Effect::pump(2, 2, ChooseSpec::SpecificObject(original), Until::EndOfTurn));
        let added_sacrifice = Effect::new(ironsmith::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(added)));
        let action = match scenario {
            0 => None,
            1 => Some(ReplacementAction::ChangeDestination(Zone::Exile)),
            2 => Some(ReplacementAction::Prevent),
            3 => Some(ReplacementAction::Instead(vec![added_sacrifice])),
            _ => Some(ReplacementAction::Additionally(vec![added_sacrifice])),
        };
        if let Some(action) = action {
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(original), Some(Zone::Battlefield), Some(Zone::Graveyard)), action));
        }
        mana(&mut game, ManaSymbol::Colorless, 1);
        let mut dm = Choices { pick: Some(original), ..Default::default() }; activate(&mut game, source, 0, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "modified sacrifice remains a completed payment");
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let drawn = if scenario == 2 || scenario == 3 { 0 } else { 5 };
        assert_eq!(game.player(A).unwrap().library.len(), 8 - drawn);
        assert_eq!(game.player(A).unwrap().hand.len(), drawn, "even a zero-card draw is followed by discard");
    }}
}
