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
const COMPLETE: &[&str] = &["Bishop of Binding", "Essence Bottle", "Ooze Flux", "Vish Kal, Blood Arbiter", "Malevolent Witchkite", "Sawblade Skinripper", "Voracious Brood"];

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
struct Choices { quantity: u32, target: Option<Target>, pick: Option<ObjectId>, distribution: Vec<(Target, u32)>, objects: Option<Vec<ObjectId>>, forbidden: Vec<ObjectId> }
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
        if let Some(objects) = &self.objects {
            assert!(objects.iter().all(|id| ctx.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal)));
            assert!(self.forbidden.iter().all(|id| !ctx.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal)));
            return objects.clone();
        }
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
        let expected = if matches!(*name, "Ooze Flux" | "Malevolent Witchkite" | "Voracious Brood") { 1 } else { 2 };
        let count = definition.abilities.iter().filter(|ability| match ability.kind {
            ironsmith::ability::AbilityKind::Triggered(_) => matches!(*name, "Bishop of Binding" | "Malevolent Witchkite" | "Sawblade Skinripper" | "Voracious Brood"),
            ironsmith::ability::AbilityKind::Activated(_) => !matches!(*name, "Bishop of Binding" | "Malevolent Witchkite" | "Voracious Brood"),
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
    assert_eq!(queue_event(&mut game, event, &mut dm), 1);
    assert!(matches!(resolve_stack_entry_with(&mut game, &mut dm),
        Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ironsmith::effects::ExecutionError::IncompleteEvidence(_)))));
    assert_eq!(game.player(A).unwrap().hand.len(), 0, "unknown pairing rolls back the whole resolution");
    assert_eq!(game.current_power(recipient), Some(2));
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

#[test]
fn witchkite_full_entry_counts_only_actual_selected_sacrifices_across_the_printed_union() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Malevolent Witchkite") { for scenario in 0..5 {
        let mut game = game();
        let artifact = compile_to_runtime_definition("Artifact resource", "Type: Artifact", false).unwrap();
        let enchantment = compile_to_runtime_definition("Enchantment resource", "Type: Enchantment", false).unwrap();
        let own_artifact = game.create_object_from_definition(&artifact, A, Zone::Battlefield);
        let own_enchantment = game.create_object_from_definition(&enchantment, A, Zone::Battlefield);
        let foreign = game.create_object_from_definition(&artifact, B, Zone::Battlefield);
        let hand = game.create_object_from_definition(&enchantment, A, Zone::Hand);
        let plain = creature(&mut game, A, Zone::Battlefield, "Nontoken creature", 2, 3, "Elf");
        let added = creature(&mut game, A, Zone::Battlefield, "Replacement-only sacrifice", 9, 9, "Elf");
        let token_definition = compile_to_runtime_definition("Saproling", "Type: Creature — Saproling\nPower/Toughness: 1/1", false).unwrap();
        apply(&mut game, own_artifact, Effect::new(ironsmith::effects::CreateTokenEffect::you(token_definition, 1)));
        let token = *game.battlefield.iter().find(|id| game.object(**id).unwrap().kind == ObjectKind::Token).unwrap();
        for _ in 0..8 { creature(&mut game, A, Zone::Library, "Draw witness", 1, 1, "Elf"); }
        let source = game.create_object_from_definition(&definition, A, Zone::Hand); let stable = game.object(source).unwrap().stable_id;
        let added_sacrifice = Effect::new(ironsmith::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(added)));
        let replacement = match scenario {
            2 => Some(ReplacementAction::Prevent),
            3 => Some(ReplacementAction::Instead(vec![added_sacrifice])),
            4 => Some(ReplacementAction::Additionally(vec![added_sacrifice])),
            _ => None,
        };
        if let Some(action) = replacement {
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(plain, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(own_artifact), Some(Zone::Battlefield), Some(Zone::Graveyard)), action));
        }
        mana(&mut game, ManaSymbol::Black, 2); mana(&mut game, ManaSymbol::Colorless, 4);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
        let mut dm = Choices::default(); announce(&mut game, action, &mut dm); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let source = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.object_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Flying));
        let mut queue = TriggerQueue::new(); drain_pending_trigger_events(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap(); assert_eq!(game.stack.len(), 1);
        dm.objects = Some(if scenario == 0 { Vec::new() } else { vec![own_artifact, own_enchantment, token] });
        dm.forbidden = vec![foreign, hand, plain, source, added];
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let actual = match scenario { 0 => 0, 2 | 3 => 2, _ => 3 };
        assert_eq!(game.player(A).unwrap().library.len(), 8 - actual);
        assert_eq!(game.player(A).unwrap().hand.len(), 1 + actual, "selected count and added sacrifice are not the draw count");
        assert!(game.object(foreign).is_some() && game.object(plain).is_some() && game.object(hand).is_some());
    }}
}

fn sacrifice_with_observations(game: &mut GameState, source: ObjectId, player: PlayerId, target: ObjectId) {
    let effect = Effect::new(ironsmith::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(target)));
    let outcome = execute_effect(game, &effect, &mut EffectContext::new(source, player, &mut SelectFirstDecisionMaker)).unwrap();
    for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
}
fn end_step(game: &mut GameState, player: PlayerId, dm: &mut Choices) -> usize {
    game.turn.phase = Phase::Ending; game.turn.step = Some(ironsmith::game_state::Step::End); game.turn.active_player = player;
    let mut queue = TriggerQueue::new(); ironsmith::game_loop::generate_and_queue_step_triggers(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap(); game.stack.len()
}

#[test]
fn sawblade_full_body_uses_current_turn_actual_sacrifice_history_not_the_threshold_or_source_counters() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Sawblade Skinripper") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.object_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Menace));
        mana(&mut game, ManaSymbol::Colorless, 2);
        assert!(activation(&game, source, 0).is_none(), "another permanent is required");
        let original = creature(&mut game, A, Zone::Battlefield, "Original payment", 1, 2, "Elf");
        let enchantment = compile_to_runtime_definition("Enchantment resource", "Type: Enchantment", false).unwrap();
        let added = game.create_object_from_definition(&enchantment, A, Zone::Battlefield);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(original), Some(Zone::Battlefield), Some(Zone::Graveyard)),
            ReplacementAction::Additionally(vec![Effect::new(ironsmith::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(added)))])));
        let mut dm = Choices { pick: Some(original), target: Some(Target::Player(B)), ..Default::default() };
        activate(&mut game, source, 0, &mut dm); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        let foreign = creature(&mut game, B, Zone::Battlefield, "Opponent sacrifice", 1, 2, "Elf");
        sacrifice_with_observations(&mut game, source, B, foreign);
        assert_eq!(end_step(&mut game, B, &mut dm), 0, "only your end step");
        assert_eq!(end_step(&mut game, A, &mut dm), 1);
        let land = compile_to_runtime_definition("Later land", "Type: Land", false).unwrap();
        let later = game.create_object_from_definition(&land, A, Zone::Battlefield);
        sacrifice_with_observations(&mut game, source, A, later);
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().life, 17, "two payment-time sacrifices plus the later one; foreign sacrifice and threshold do not contribute");
        game.next_turn();
        assert_eq!(end_step(&mut game, A, &mut dm), 0, "previous-turn sacrifice history is gone");
    }
}

#[test]
fn sawblade_prevented_or_wholly_replaced_payment_does_not_satisfy_the_end_step_history_gate() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Sawblade Skinripper") { for prevent in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let enchantment = compile_to_runtime_definition("Selected enchantment", "Type: Enchantment", false).unwrap();
        let selected = game.create_object_from_definition(&enchantment, A, Zone::Battlefield);
        let replacement = if prevent { ReplacementAction::Prevent } else { ReplacementAction::Instead(vec![Effect::gain_life(1)]) };
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(selected), Some(Zone::Battlefield), Some(Zone::Graveyard)), replacement));
        mana(&mut game, ManaSymbol::Colorless, 2);
        let mut dm = Choices { pick: Some(selected), ..Default::default() }; activate(&mut game, source, 0, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1, "the modified cost is paid");
        assert!(game.object(selected).is_some()); assert_eq!(end_step(&mut game, A, &mut dm), 0);
    }}
}

fn recorded_effect(game: &mut GameState, source: ObjectId, effect: Effect, dm: &mut Choices) {
    let outcome = execute_effect(game, &effect, &mut EffectContext::new(source, A, dm)).unwrap();
    for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
    let mut queue = TriggerQueue::new(); drain_pending_trigger_events(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}

#[test]
fn voracious_brood_full_body_counts_only_owned_creature_cards_and_captures_each_matching_batch() {
    for definition in definitions("Voracious Brood") {
        let mut game = game();
        for _ in 0..2 { creature(&mut game, A, Zone::Graveyard, "Old graveyard creature", 1, 2, "Elf"); }
        creature(&mut game, B, Zone::Graveyard, "Opponent graveyard creature", 1, 2, "Elf");
        let artifact = compile_to_runtime_definition("Noncreature card", "Type: Artifact", false).unwrap();
        game.create_object_from_definition(&artifact, A, Zone::Graveyard);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand); let stable = game.object(source).unwrap().stable_id;
        mana(&mut game, ManaSymbol::Green, 1); mana(&mut game, ManaSymbol::Colorless, 2);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
        let mut dm = Choices::default(); announce(&mut game, action, &mut dm); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let source = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
        let first = creature(&mut game, A, Zone::Library, "Milled creature one", 1, 2, "Elf");
        let first_stable = game.object(first).unwrap().stable_id;
        creature(&mut game, A, Zone::Library, "Milled creature two", 1, 2, "Elf");
        game.create_object_from_definition(&artifact, A, Zone::Library);
        recorded_effect(&mut game, source, Effect::new(ironsmith::effects::MillEffect::you(3)), &mut dm);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].event_value_amount, Some(2), "the batch amount is the matched creature-card subset, not all milled cards");
        let trigger = game.stack[0].ability_id.unwrap();
        apply(&mut game, source, Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(trigger))));
        assert_eq!(game.stack.len(), 2); assert_eq!(game.stack[1].event_value_amount, Some(2));
        let first_grave = game.find_object_by_stable_id(first_stable).unwrap(); game.move_object_by_game_rule(first_grave, Zone::Exile).unwrap();
        for _ in 0..2 { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 6, "copies retain the captured two-card event count after a card leaves");

        creature(&mut game, A, Zone::Battlefield, "Owned one", 1, 2, "Elf");
        creature(&mut game, A, Zone::Battlefield, "Owned two", 1, 2, "Elf");
        let stolen = creature(&mut game, B, Zone::Battlefield, "Foreign-owned", 1, 2, "Elf");
        game.set_current_controller(stolen, A);
        assert_eq!(game.current_controller(stolen), Some(A));
        assert_eq!(game.object(stolen).unwrap().owner, B);
        let token = compile_to_runtime_definition("Saproling", "Type: Creature — Saproling\nPower/Toughness: 1/1", false).unwrap();
        apply(&mut game, source, Effect::new(ironsmith::effects::CreateTokenEffect::you(token, 1)));
        let mut filter = ironsmith::target::ObjectFilter::creature().you_control(); filter.other = true;
        recorded_effect(&mut game, source, Effect::destroy_all(filter), &mut dm);
        assert_eq!(game.stack.len(), 1); assert_eq!(game.stack[0].event_value_amount, Some(2), "tokens and opponent-owned cards do not enter your creature-card batch");
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 8);
        let hand = creature(&mut game, A, Zone::Hand, "Hand-origin creature", 1, 2, "Elf");
        recorded_effect(&mut game, source, Effect::move_to_zone(ChooseSpec::SpecificObject(hand), Zone::Graveyard, false), &mut dm);
        assert_eq!(game.stack.len(), 1); assert_eq!(game.stack[0].event_value_amount, Some(1));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 9);
        let noncreature = game.create_object_from_definition(&artifact, A, Zone::Hand);
        recorded_effect(&mut game, source, Effect::move_to_zone(ChooseSpec::SpecificObject(noncreature), Zone::Graveyard, false), &mut dm);
        assert!(game.stack.is_empty());
    }
}

// Linked-ability ownership scenarios. Authored source only; never executed in
// this campaign. Each frozen Bishop body is exercised directly and by artifact.
fn bishop_waiting_for_entry(game: &mut GameState, definition: &CardDefinition, victim: ObjectId, dm: &mut Choices) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(source).unwrap().stable_id;
    mana(game, ManaSymbol::White, 1); mana(game, ManaSymbol::Colorless, 3);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
    dm.target = Some(Target::Object(victim)); announce(game, action, dm);
    resolve_stack_entry_with(game, dm).unwrap();
    let source = game.find_object_by_stable_id(stable).unwrap();
    let mut queue = TriggerQueue::new(); drain_pending_trigger_events(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert!(game.stack[0].linked_exile_owner.is_some());
    source
}
fn bishop_attack(game: &mut GameState, source: ObjectId, recipient: ObjectId, dm: &mut Choices) -> usize {
    dm.target = Some(Target::Object(recipient));
    queue_event(game, TriggerEvent::new_with_provenance(
        ironsmith::events::combat::CreatureAttackedEvent::new(source, AttackEventTarget::Player(B)),
        Default::default()), dm)
}
fn borrow_bishop_triggers(game: &mut GameState, host: ObjectId, donor: ObjectId) {
    use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(host, A,
        EffectTarget::Specific(host), Modification::CopyTriggeredAbilities {
            filter: ironsmith::target::ObjectFilter::specific(donor),
            exclude_source_name: false, exclude_source_id: true,
        }));
    game.refresh_continuous_state().unwrap();
}

#[test]
fn bishop_borrowed_plain_exile_neither_boosts_the_linked_reader_nor_returns_on_departure() {
    use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
    for definition in definitions("Bishop of Binding") {
        let mut game = game();
        let victim = creature(&mut game, B, Zone::Battlefield, "Legitimate victim", 4, 6, "Soldier");
        let victim_stable = game.object(victim).unwrap().stable_id;
        let unrelated = creature(&mut game, B, Zone::Battlefield, "Other exile", 9, 9, "Elf");
        let unrelated_stable = game.object(unrelated).unwrap().stable_id;
        let recipient = creature(&mut game, A, Zone::Battlefield, "Recipient", 2, 4, "Vampire");
        let mut dm = Choices::default();
        let source = bishop_waiting_for_entry(&mut game, &definition, victim, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let donor = compile_to_runtime_definition("Borrowed plain exile donor",
            "Type: Creature\nPower/Toughness: 1/1\n{0}: Exile target creature.", false).unwrap();
        let donor = game.create_object_from_definition(&donor, A, Zone::Exile);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A,
            EffectTarget::Specific(source), Modification::CopyActivatedAbilities {
                filter: ironsmith::target::ObjectFilter::specific(donor), counter: None,
                include_mana: true, only_loyalty: false, exclude_source_name: false,
                exclude_source_id: true, force_once_each_turn: false,
            }));
        game.refresh_continuous_state().unwrap();
        dm.target = Some(Target::Object(unrelated)); activate(&mut game, source, 0, &mut dm);
        assert!(game.stack.last().unwrap().linked_exile_owner.is_none());
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.get_exiled_with_source_links(source).len(), 2);
        assert_eq!(bishop_attack(&mut game, source, recipient, &mut dm), 1);
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.current_power(recipient), Some(6), "the borrowed ability's 9 power is unrelated");
        apply(&mut game, source, Effect::move_to_zone(ChooseSpec::Source, Zone::Hand, false));
        assert_eq!(game.object(game.find_object_by_stable_id(victim_stable).unwrap()).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(game.find_object_by_stable_id(unrelated_stable).unwrap()).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn bishop_copied_entry_uses_the_original_pair_and_sums_only_live_victim_incarnations() {
    for definition in definitions("Bishop of Binding") { for mode in 0..4 {
        let mut game = game();
        let first = creature(&mut game, B, Zone::Battlefield, "First victim", 3, 4, "Soldier");
        let second = creature(&mut game, B, Zone::Battlefield, "Second victim", 5, 6, "Soldier");
        let first_stable = game.object(first).unwrap().stable_id;
        let second_stable = game.object(second).unwrap().stable_id;
        let recipient = creature(&mut game, A, Zone::Battlefield, "Recipient", 2, 4, "Vampire");
        let mut dm = Choices::default();
        let source = bishop_waiting_for_entry(&mut game, &definition, first, &mut dm);
        let source_stable = game.object(source).unwrap().stable_id;
        let owner = game.stack.last().unwrap().linked_exile_owner.clone().unwrap();
        let entry_id = game.stack.last().unwrap().target_id();
        apply(&mut game, source, Effect::copy_spell(ChooseSpec::SpecificObject(entry_id)));
        assert_eq!(game.stack.len(), 2);
        assert_eq!(game.stack.last().unwrap().linked_exile_owner.as_ref(), Some(&owner));
        let copy_id = game.stack.last().unwrap().target_id();
        apply(&mut game, source, Effect::new(ironsmith::effects::RetargetStackObjectEffect::new(
            ChooseSpec::SpecificObject(copy_id)).with_mode(ironsmith::effects::RetargetMode::OneToFixed(
                ChooseSpec::SpecificObject(second)))));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.linked_exile_pair_members(&owner).unwrap().len(), 2);
        assert_eq!(bishop_attack(&mut game, source, recipient, &mut dm), 1);
        let first = game.find_object_by_stable_id(first_stable).unwrap();
        let second = game.find_object_by_stable_id(second_stable).unwrap();
        match mode {
            1 => { // A blink of the victim cannot restore its recorded incarnation.
                let hand = game.move_object_by_game_rule(first, Zone::Hand).unwrap();
                game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
            }
            2 => { game.set_face_down(second); }
            3 => { // A pending reader retains the old source while duration returns run.
                apply(&mut game, source, Effect::move_to_zone(ChooseSpec::Source, Zone::Hand, false));
                let hand = game.find_object_by_stable_id(source_stable).unwrap();
                game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap();
            }
            _ => {}
        }
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.current_power(recipient), Some(match mode { 0 => 10, 1 => 7, 2 => 5, _ => 2 }));
    }}
}

#[test]
fn borrowed_pairs_keep_independent_donor_and_grant_acquisitions_on_one_host() {
    for definition in definitions("Bishop of Binding") {
        let mut game = game();
        let host = creature(&mut game, A, Zone::Battlefield, "Borrowing host", 1, 1, "Vampire");
        let donor1 = game.create_object_from_definition(&definition, A, Zone::Exile);
        let donor2 = game.create_object_from_definition(&definition, A, Zone::Exile);
        borrow_bishop_triggers(&mut game, host, donor1);
        borrow_bishop_triggers(&mut game, host, donor2);
        let first = creature(&mut game, B, Zone::Battlefield, "First victim", 3, 4, "Elf");
        let second = creature(&mut game, B, Zone::Battlefield, "Second victim", 7, 8, "Elf");
        let recipient = creature(&mut game, A, Zone::Battlefield, "Recipient", 2, 4, "Vampire");
        let event = TriggerEvent::new_with_provenance(
            ironsmith::events::EnterBattlefieldEvent::new(host, Zone::Hand), Default::default());
        let entries = check_triggers(&game, &event); assert_eq!(entries.len(), 2);
        assert_ne!(entries[0].linked_exile_owner, entries[1].linked_exile_owner);
        let mut dm = Choices::default();
        for (entry, victim) in entries.into_iter().zip([first, second]) {
            let mut queue = TriggerQueue::new(); queue.add(entry);
            dm.target = Some(Target::Object(victim));
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        assert_eq!(bishop_attack(&mut game, host, recipient, &mut dm), 2);
        let top = game.stack.last().unwrap().linked_exile_owner.clone().unwrap();
        let members = game.linked_exile_pair_members(&top).unwrap(); assert_eq!(members.len(), 1);
        let amount = game.current_power(members[0]).unwrap();
        // Donor departure after admission cannot redirect a pending reader.
        let donor1 = game.move_object_by_game_rule(donor1, Zone::Hand).unwrap();
        let donor1 = game.move_object_by_game_rule(donor1, Zone::Exile).unwrap();
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.current_power(recipient), Some(2 + amount));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.current_power(recipient), Some(12));
        borrow_bishop_triggers(&mut game, host, donor1);
        assert_eq!(bishop_attack(&mut game, host, recipient, &mut dm), 2);
        let owners: Vec<_> = game.stack.iter().map(|entry| entry.linked_exile_owner.as_ref().unwrap()).collect();
        assert_eq!(owners.iter().filter(|owner| game.linked_exile_pair_members(owner).unwrap().is_empty()).count(), 1,
            "new donor incarnation and acquisition start without the old victim");
    }
}

#[test]
fn paired_producer_missing_admission_evidence_rolls_back_before_exile_and_recovers() {
    for definition in definitions("Bishop of Binding") {
        let mut game = game();
        let victim = creature(&mut game, B, Zone::Battlefield, "Victim", 3, 4, "Elf");
        let mut dm = Choices::default();
        let source = bishop_waiting_for_entry(&mut game, &definition, victim, &mut dm);
        let exact = game.clone();
        game.stack.last_mut().unwrap().linked_exile_owner = None;
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut dm),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ironsmith::effects::ExecutionError::IncompleteEvidence(_)))));
        assert_eq!(game.object(victim).unwrap().zone, Zone::Battlefield);
        assert!(game.get_exiled_with_source_links(source).is_empty());
        game = exact; resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.get_exiled_with_source_links(source).len(), 1);
    }
}

#[test]
fn compiler_linking_rejects_ambiguous_extra_bodies_and_reused_local_card_ids_do_not_bind_definitions() {
    let bodies = [
        "When this creature enters, exile target creature an opponent controls until this creature leaves the battlefield.\nWhenever this creature attacks, target Vampire gets +X/+X until end of turn, where X is the power of the exiled card.",
        "When this creature enters, exile target creature an opponent controls until this creature leaves the battlefield.\nWhenever this creature attacks, target Vampire gets +X/+X until end of turn, where X is the toughness of the exiled card.",
    ];
    let mut pairs = Vec::new();
    for body in bodies {
        let text = format!("Type: Creature — Vampire\nPower/Toughness: 1/1\n{body}");
        let definition = compile_to_runtime_definition("Same caller-local identifier", &text, false).unwrap();
        let member_pairs: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(ability) => ability.effects.linked_exile_pair,
            _ => None,
        }).collect();
        assert_eq!(member_pairs.len(), 2); assert_eq!(member_pairs[0], member_pairs[1]);
        pairs.push(member_pairs[0]);
        let ambiguous = compile_to_runtime_definition("Two printed producer scopes",
            format!("{text}\nWhenever this creature dies, exile target card from a graveyard."), false).unwrap();
        assert!(ambiguous.abilities.iter().all(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(ability) => ability.effects.linked_exile_pair.is_none(),
            _ => true,
        }));
    }
    assert_ne!(pairs[0].definition, pairs[1].definition);
}
