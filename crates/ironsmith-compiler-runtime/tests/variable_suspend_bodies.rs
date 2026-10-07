//! Four complete frozen cards. Scenarios are authored only; campaign execution is deferred.
use std::collections::VecDeque;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, NumberContext, TargetsContext};
use ironsmith::game_loop::{generate_and_queue_step_triggers, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Step;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::special_actions::{self, ActionError, SpecialAction};
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::{Subtype, Supertype};
use ironsmith::{CardId, CardType, GameState, ObjectId, Phase, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{StaticAbilityPayload, SuspendTime, TriggerKind, Value};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/variable_suspend_bodies.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards = fixtures();
    let row = cards.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name} direct: {}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name} artifact: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, kind: CardType, soldier: bool, basic: bool) -> ObjectId {
    let mut card = CardBuilder::new(CardId::new(), "Suspend witness")
        .card_types(vec![kind]).power_toughness(PowerToughness::fixed(2, 2));
    if soldier { card = card.subtypes(vec![Subtype::Soldier]); }
    if basic { card = card.supertypes(vec![Supertype::Basic]); }
    game.create_object_from_card(&card.build(), owner, zone)
}
fn library(game: &mut GameState, count: usize) {
    for _ in 0..count { object(game, A, Zone::Library, CardType::Artifact, false, false); }
}
fn fund(game: &mut GameState, name: &str, x: u32) {
    let (color, colored, generic) = match name {
        "Aeon Chronicler" => (ManaSymbol::Blue, 1, 3),
        "Benalish Commander" => (ManaSymbol::White, 2, 0),
        "Detritivore" => (ManaSymbol::Red, 1, 3),
        "Fungal Behemoth" => (ManaSymbol::Green, 2, 0),
        _ => unreachable!(),
    };
    let pool = &mut game.player_mut(A).unwrap().mana_pool;
    pool.add(color, colored);
    pool.add(ManaSymbol::Colorless, generic + x);
}
#[derive(Default)]
struct Choices {
    x: u32,
    pause_x: bool,
    pending: bool,
    number_calls: usize,
    accept: bool,
    targets: VecDeque<Target>,
    forbidden_targets: Vec<Target>,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        assert!(ctx.is_x_value);
        assert_eq!(ctx.min, 1);
        self.number_calls += 1;
        self.pending = self.pause_x;
        self.x
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.accept }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert!(ctx.requirements.iter().all(|requirement| self.forbidden_targets.iter().all(|target| !requirement.legal_targets.contains(target))), "excluded targets must remain illegal: {ctx:?}");
        if let Some(target) = self.targets.pop_front() {
            assert!(ctx.requirements.iter().all(|requirement| requirement.legal_targets.contains(&target)), "target must pass the real filter: {ctx:?}");
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
}
fn suspend(game: &mut GameState, source: ObjectId, choices: &mut Choices) -> ObjectId {
    let stable = game.object(source).unwrap().stable_id;
    special_actions::perform(SpecialAction::Suspend { card_id: source }, game, A, choices).unwrap();
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    exiled
}
fn queue(game: &mut GameState, choices: &mut impl DecisionMaker) -> usize {
    let mut triggers = TriggerQueue::new();
    put_triggers_on_stack_with_dm(game, &mut triggers, choices).unwrap();
    game.stack.len()
}
fn resolve_all(game: &mut GameState, choices: &mut impl DecisionMaker) {
    for _ in 0..16 {
        queue(game, choices);
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("bounded Suspend scenario did not settle");
}
fn remove(game: &mut GameState, source: ObjectId, kind: CounterType, amount: u32) {
    let (removed, event) = game.remove_counters(source, kind, amount, Some(source), Some(B)).unwrap();
    assert_eq!(removed, amount);
    game.queue_trigger_event(Default::default(), event);
}
fn pt(game: &GameState, source: ObjectId) -> (i32, i32) {
    (game.calculated_power(source).unwrap(), game.calculated_toughness(source).unwrap())
}
fn cda(definition: &CardDefinition) -> (&Value, &Value) {
    definition.abilities.iter().find_map(|ability| {
        let AbilityKind::Static(rule) = &ability.kind else { return None; };
        let StaticAbilityPayload::CharacteristicDefiningPt { power, toughness } = &rule.compiled_model()?.payload else { return None; };
        Some((power, toughness))
    }).expect("complete body retains its characteristic-defining ability")
}

#[test]
fn four_complete_frozen_cards_keep_suspend_minimum_counter_bodies_and_dynamic_stats() {
    assert_eq!(fixtures().len(), 4);
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(matches!(definition.alternative_casts.as_slice(), [ironsmith::AlternativeCastingMethod::Suspend { time: SuspendTime::X { minimum: 1 }, cost }] if cost.has_x()));
            let (power, toughness) = cda(&definition);
            assert_eq!(power, toughness);
            assert!(!matches!(power.unhinted(), Value::Fixed(_)), "{name} must retain the complete CDA");
            let body = definition.abilities.iter().find(|ability| {
                matches!(&ability.kind, AbilityKind::Triggered(ability)
                    if matches!(ability.trigger.compiled_model().map(|trigger| &trigger.kind), Some(TriggerKind::CounterRemovedFrom(trigger)) if !trigger.last))
            }).expect("body counter-removal trigger");
            assert_eq!(body.functional_zones, vec![Zone::Exile]);
            let AbilityKind::Triggered(body) = &body.kind else { unreachable!() };
            let TriggerKind::CounterRemovedFrom(trigger) = &body.trigger.compiled_model().unwrap().kind else { unreachable!() };
            assert!(trigger.filter.source);
            assert_eq!(trigger.filter.zone, Some(Zone::Exile));
            assert_eq!(trigger.counter_type, Some(CounterType::Time));
            assert!(!trigger.last && !trigger.one_or_more && !trigger.caused_by_source);
            assert!(!body.effects.all_effects_owned().is_empty());
            assert!(body.intervening_if.is_none(), "while-exiled qualifies the event, not the later resolution");
            assert_eq!(definition.abilities.iter().filter(|ability| matches!(&ability.kind, AbilityKind::Triggered(_))).count(), 3);
            assert!(definition.abilities.iter().all(|ability| match &ability.kind {
                AbilityKind::Static(rule) => rule.this_spell_x_minimum_value().is_none(),
                _ => true,
            }), "Suspend's X restriction must not prevent ordinary casts");
        }
    }
}

#[test]
fn positive_suspend_x_is_paid_once_and_places_that_many_counters_without_persisting_spell_x() {
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            fund(&mut game, name, 3);
            let mut choices = Choices { x: 3, ..Default::default() };
            let exiled = suspend(&mut game, source, &mut choices);
            assert_eq!(choices.number_calls, 1);
            assert_eq!(game.counter_count(exiled, CounterType::Time), 3);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(game.object(exiled).unwrap().x_value.is_none());
            assert!(game.stack_is_empty(), "suspending is a special action");
        }
    }
}

#[test]
fn zero_unaffordable_and_pending_announcements_leave_the_full_card_and_mana_intact() {
    for definition in definitions("Aeon Chronicler") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        fund(&mut game, "Aeon Chronicler", 0);
        let action = SpecialAction::Suspend { card_id: source };
        assert_eq!(special_actions::can_perform_check(&action, &game, A), Err(ActionError::CantPayCost));
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
        let mana = game.player(A).unwrap().mana_pool.total();
        for invalid in [0, 4] {
            assert_eq!(special_actions::perform(action.clone(), &mut game, A, &mut Choices { x: invalid, ..Default::default() }), Err(ActionError::InvalidTarget));
            assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), mana);
            assert!(game.exile.is_empty());
        }
        let mut pending = Choices { x: 2, pause_x: true, ..Default::default() };
        special_actions::perform(action, &mut game, A, &mut pending).unwrap();
        assert!(pending.pending);
        assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana);
        assert!(game.exile.is_empty());
        let exiled = suspend(&mut game, source, &mut Choices { x: 2, ..Default::default() });
        assert_eq!(game.counter_count(exiled, CounterType::Time), 2);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
    }
}

#[test]
fn aeon_body_draws_for_each_removed_time_counter_and_ignores_other_counters_zones_and_sources() {
    for definition in definitions("Aeon Chronicler") {
        let mut game = game();
        library(&mut game, 6);
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        let other = object(&mut game, A, Zone::Exile, CardType::Creature, false, false);
        game.add_counters(source, CounterType::Time, 3);
        game.add_counters(source, CounterType::Charge, 2);
        game.add_counters(other, CounterType::Time, 2);
        game.take_pending_trigger_events();
        remove(&mut game, other, CounterType::Time, 1);
        remove(&mut game, source, CounterType::Charge, 1);
        assert_eq!(queue(&mut game, &mut Choices::default()), 0);
        remove(&mut game, source, CounterType::Time, 2);
        assert_eq!(queue(&mut game, &mut Choices::default()), 2);
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(B).unwrap().hand.len(), 0);
        assert_eq!(pt(&game, source), (2, 2));
        let source = game.move_object_by_effect(source, Zone::Battlefield).unwrap();
        game.add_counters(source, CounterType::Time, 2);
        game.take_pending_trigger_events();
        remove(&mut game, source, CounterType::Time, 1);
        assert_eq!(queue(&mut game, &mut Choices::default()), 0);
    }
}

#[test]
fn benalish_body_creates_actual_white_soldiers_and_the_cda_recounts_all_your_soldiers() {
    for definition in definitions("Benalish Commander") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        object(&mut game, B, Zone::Battlefield, CardType::Creature, true, false);
        object(&mut game, A, Zone::Graveyard, CardType::Creature, true, false);
        assert_eq!(pt(&game, source), (0, 0));
        game.add_counters(source, CounterType::Time, 3);
        game.take_pending_trigger_events();
        remove(&mut game, source, CounterType::Time, 2);
        assert_eq!(queue(&mut game, &mut Choices::default()), 2);
        resolve_all(&mut game, &mut Choices::default());
        let soldiers: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|object| object.owner == A && object.subtypes.contains(&Subtype::Soldier))).collect();
        assert_eq!(soldiers.len(), 2);
        for soldier in &soldiers {
            assert_eq!(pt(&game, *soldier), (1, 1));
            assert!(matches!(game.object(*soldier).unwrap().kind, ironsmith::object::ObjectKind::Token));
            assert!(game.object(*soldier).unwrap().colors().contains(ironsmith::color::Color::White));
        }
        assert_eq!(pt(&game, source), (2, 2));
        game.set_current_controller(soldiers[0], B).unwrap();
        assert_eq!(pt(&game, source), (1, 1));
        let source = game.move_object_by_effect(source, Zone::Battlefield).unwrap();
        assert_eq!(pt(&game, source), (2, 2), "the Commander itself is a Soldier");
    }
}

#[test]
fn detritivore_body_targets_nonbasic_lands_and_counts_all_opponents_graveyards() {
    for definition in definitions("Detritivore") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        let first = object(&mut game, B, Zone::Battlefield, CardType::Land, false, false);
        let second = object(&mut game, C, Zone::Battlefield, CardType::Land, false, false);
        let basic = object(&mut game, B, Zone::Battlefield, CardType::Land, false, true);
        object(&mut game, A, Zone::Graveyard, CardType::Land, false, false);
        object(&mut game, B, Zone::Graveyard, CardType::Land, false, true);
        object(&mut game, B, Zone::Graveyard, CardType::Creature, false, false);
        assert_eq!(pt(&game, source), (0, 0));
        game.add_counters(source, CounterType::Time, 3);
        game.take_pending_trigger_events();
        remove(&mut game, source, CounterType::Time, 2);
        let mut choices = Choices { targets: VecDeque::from([Target::Object(first), Target::Object(second)]), forbidden_targets: vec![Target::Object(basic)], ..Default::default() };
        assert_eq!(queue(&mut game, &mut choices), 2);
        resolve_all(&mut game, &mut choices);
        assert!(game.object(first).is_none() && game.object(second).is_none());
        assert_eq!(game.object(basic).unwrap().zone, Zone::Battlefield);
        assert_eq!(pt(&game, source), (2, 2));
        let card = *game.player(C).unwrap().graveyard.first().unwrap();
        game.move_object_by_effect(card, Zone::Exile).unwrap();
        assert_eq!(pt(&game, source), (1, 1));
    }
}

#[test]
fn fungal_body_is_optional_and_the_cda_sums_only_plus_one_counters_on_your_creatures() {
    for definition in definitions("Fungal Behemoth") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        let own = object(&mut game, A, Zone::Battlefield, CardType::Creature, false, false);
        let other = object(&mut game, A, Zone::Battlefield, CardType::Creature, false, false);
        let enemy = object(&mut game, B, Zone::Battlefield, CardType::Creature, false, false);
        let artifact = object(&mut game, A, Zone::Battlefield, CardType::Artifact, false, false);
        game.add_counters(own, CounterType::PlusOnePlusOne, 2);
        game.add_counters(other, CounterType::PlusOnePlusOne, 3);
        game.add_counters(own, CounterType::Charge, 4);
        game.add_counters(enemy, CounterType::PlusOnePlusOne, 7);
        game.add_counters(artifact, CounterType::PlusOnePlusOne, 9);
        assert_eq!(pt(&game, source), (5, 5));
        game.add_counters(source, CounterType::Time, 4);
        game.take_pending_trigger_events();
        remove(&mut game, source, CounterType::Time, 1);
        let mut decline = Choices { targets: VecDeque::from([Target::Object(enemy)]), ..Default::default() };
        resolve_all(&mut game, &mut decline);
        assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 7);
        remove(&mut game, source, CounterType::Time, 2);
        let mut accept = Choices { accept: true, targets: VecDeque::from([Target::Object(own), Target::Object(own)]), ..Default::default() };
        assert_eq!(queue(&mut game, &mut accept), 2);
        resolve_all(&mut game, &mut accept);
        assert_eq!(game.counter_count(own, CounterType::PlusOnePlusOne), 4);
        assert_eq!(pt(&game, source), (7, 7));
        game.phase_out(other);
        assert_eq!(pt(&game, source), (4, 4));
        game.phase_in(other);
        game.set_current_controller(own, B).unwrap();
        assert_eq!(pt(&game, source), (3, 3));
    }
}

#[test]
fn upkeep_and_last_counter_cast_keep_the_body_trigger_and_grant_haste() {
    for definition in definitions("Aeon Chronicler") {
        let mut game = game();
        library(&mut game, 6);
        object(&mut game, A, Zone::Hand, CardType::Artifact, false, false);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        fund(&mut game, "Aeon Chronicler", 1);
        let exiled = suspend(&mut game, source, &mut Choices { x: 1, ..Default::default() });
        let stable = game.object(exiled).unwrap().stable_id;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        let mut triggers = TriggerQueue::new();
        generate_and_queue_step_triggers(&mut game, &mut triggers);
        let mut choices = Choices { accept: true, ..Default::default() };
        put_triggers_on_stack_with_dm(&mut game, &mut triggers, &mut choices).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.counter_count(exiled, CounterType::Time), 0);
        assert_eq!(queue(&mut game, &mut choices), 2, "last-counter cast and draw are separate triggers");
        resolve_all(&mut game, &mut choices);
        let entered = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(pt(&game, entered), (2, 2));
        assert!(game.current_has_static_ability_id(entered, ironsmith::static_abilities::StaticAbilityId::Haste));
        assert!(game.object(entered).unwrap().x_value.is_none_or(|x| x == 0));
    }
}

#[test]
fn variable_suspend_uses_counter_replacements_and_rolls_back_a_pending_placement_transaction() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    struct PendingPlacement { pending: bool }
    impl DecisionMaker for PendingPlacement {
        fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
            assert_eq!(ctx.min, 1);
            2
        }
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            self.pending = true;
            false
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    for definition in definitions("Aeon Chronicler") {
        for pending in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let witness = object(&mut game, A, Zone::Battlefield, CardType::Artifact, false, false);
            let replacement = if pending {
                ReplacementAction::Instead(vec![
                    ironsmith::Effect::gain_life(2),
                    ironsmith::Effect::may(vec![ironsmith::Effect::gain_life(3)]),
                ])
            } else { ReplacementAction::Double };
            let replacement_id = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    witness, A,
                    ironsmith::events::counters::matchers::WouldPutCountersMatcher::new(
                        ironsmith_core::ObjectFilter::default().in_zone(Zone::Exile),
                        Some(CounterType::Time),
                    ),
                    replacement,
                ),
            );
            fund(&mut game, "Aeon Chronicler", 2);
            let mana = game.player(A).unwrap().mana_pool.total();
            if pending {
                let mut choices = PendingPlacement { pending: false };
                special_actions::perform(SpecialAction::Suspend { card_id: source }, &mut game, A, &mut choices).unwrap();
                assert!(choices.pending);
                assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
                assert!(game.exile.is_empty());
                assert_eq!(game.player(A).unwrap().mana_pool.total(), mana);
                assert_eq!(game.player(A).unwrap().life, 20);
                assert!(game.effect_store.replacement_effects.get_effect(replacement_id).is_some());
                let exiled = suspend(&mut game, source, &mut Choices { x: 2, accept: true, ..Default::default() });
                assert_eq!(game.counter_count(exiled, CounterType::Time), 0);
                assert_eq!(game.player(A).unwrap().life, 25);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            } else {
                let exiled = suspend(&mut game, source, &mut Choices { x: 2, ..Default::default() });
                assert_eq!(game.counter_count(exiled, CounterType::Time), 4);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "replacement changes counters, not the announced payment");
            }
        }
    }
}

#[test]
fn a_counter_body_already_on_the_stack_survives_its_source_leaving_exile() {
    for definition in definitions("Aeon Chronicler") {
        let mut game = game();
        library(&mut game, 3);
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        game.add_counters(source, CounterType::Time, 2);
        game.take_pending_trigger_events();
        remove(&mut game, source, CounterType::Time, 1);
        assert_eq!(queue(&mut game, &mut Choices::default()), 1);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}

#[test]
fn fungal_two_creature_counter_total_reports_a_checked_scalar_limit_and_recovers() {
    use ironsmith::static_ability_processor::StaticEffectDiscoveryError;
    for definition in definitions("Fungal Behemoth") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Exile);
        let first = object(&mut game, A, Zone::Battlefield, CardType::Creature, false, false);
        let second = object(&mut game, A, Zone::Battlefield, CardType::Creature, false, false);
        game.add_counters(first, CounterType::PlusOnePlusOne, 1_500_000_000);
        game.add_counters(second, CounterType::PlusOnePlusOne, 1_000_000_000);
        // Each witness's own 2+counter P/T is representable; only the CDA's
        // cross-object sum exceeds i32. That must not wrap or panic first.
        assert!(matches!(game.refresh_continuous_state(), Err(StaticEffectDiscoveryError::ScalarRange {
            resource: "characteristic-defining scalar", value: 2_500_000_000,
        })));
        assert!(matches!(game.try_current_characteristics(source), Err(StaticEffectDiscoveryError::ScalarRange {
            resource: "characteristic-defining scalar", value: 2_500_000_000,
        })));
        assert!(ironsmith::decision::compute_legal_actions(&game, A).is_err());
        game.object_mut(second).unwrap().counters.insert(CounterType::PlusOnePlusOne, 1);
        game.refresh_continuous_state().unwrap();
        let complete = game.try_current_characteristics(source).unwrap().unwrap();
        assert_eq!((complete.power, complete.toughness), (Some(1_500_000_001), Some(1_500_000_001)));
    }
}
