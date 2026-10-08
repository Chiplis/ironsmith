//! Source-authored receipt/lifetime controls. Execution is intentionally UNRUN.
use super::*;
use crate::card::CardBuilder;
use crate::decision::{LegalAction, SelectFirstDecisionMaker};
use crate::effect::Effect;
use crate::effects::execute_effect;
use crate::events::cause::EventCause;
use crate::game_state::{Phase, StackEntry};
use crate::ids::{CardId, PlayerId};
use crate::types::CardType;
use ironsmith_core::{CounterExileGate, CounterExilePermission};

fn setup(kind: CardType) -> (GameState, ObjectId, ObjectId, PlayerId, PlayerId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = CardBuilder::new(CardId::new(), "Receipt target")
        .card_types(vec![kind])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(6)]))
        .build();
    let target = game.create_object_from_card(&card, bob, Zone::Stack);
    game.stack.push(StackEntry::new(target, bob));
    let source_card = CardBuilder::new(CardId::new(), "Counter source")
        .card_types(vec![CardType::Instant]).build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    (game, source, target, alice, bob)
}

fn counter(game: &mut GameState, source: ObjectId, target: ObjectId, player: PlayerId,
    gate: CounterExileGate, allow_land: bool) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, player, &mut dm);
    execute_effect(game, &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target))
        .with_exile_permission(CounterExilePermission { gate, allow_land })), &mut ctx).unwrap();
}

fn prices(game: &GameState, object: ObjectId, player: PlayerId) -> usize {
    game.effect_store.grant_registry.granted_alternative_casts_for_card(
        game, object, Zone::Exile, player).len()
}

fn offered(game: &GameState, object: ObjectId, player: PlayerId) -> bool {
    crate::decision::compute_legal_actions(game, player).unwrap().iter().any(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == object))
}

#[test]
fn exact_counter_receipt_grants_only_its_new_exile_to_its_controller() {
    for allow_land in [false, true] {
        let (mut game, source, target, alice, bob) = setup(CardType::Creature);
        let stable = game.object(target).unwrap().stable_id;
        let unrelated_card = CardBuilder::new(CardId::new(), "Old linked exile")
            .card_types(vec![CardType::Instant]).build();
        let unrelated = game.create_object_from_card(&unrelated_card, bob, Zone::Exile);
        game.add_exiled_with_source_link(source, unrelated);
        let stale_snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(target).unwrap(), &game);
        let unrelated_snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(unrelated).unwrap(), &game);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects("__it__", vec![stale_snapshot]);
        ctx.set_tagged_objects(ironsmith_core::SOURCE_EXILED_TAG, vec![unrelated_snapshot]);
        execute_effect(&mut game, &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target))
            .with_exile_permission(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land })), &mut ctx).unwrap();
        drop(ctx);
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(exiled, target);
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert!(!game.player(bob).unwrap().graveyard.contains(&exiled));
        assert_eq!(prices(&game, exiled, alice), 1);
        assert_eq!(prices(&game, exiled, bob), 0);
        assert_eq!(prices(&game, unrelated, alice), 0);
        assert!(game.effect_store.grant_registry.grants.iter().all(|grant|
            grant.target_id == Some(exiled) && grant.target_stable_id.is_none()));
        assert_eq!(game.effect_store.grant_registry.grants.len(), if allow_land { 2 } else { 1 });
        assert!(offered(&game, exiled, alice));
        assert!(!offered(&game, unrelated, alice));
        game.move_object(source, Zone::Graveyard, EventCause::effect()).unwrap();
        game.turn.turn_number += 3;
        assert_eq!(prices(&game, exiled, alice), 1, "source departure and later turns do not expire the exile incarnation");
        assert!(offered(&game, exiled, alice));
        game.turn.active_player = bob;
        assert!(!offered(&game, exiled, alice), "free casting does not grant flash");
    }
}

#[test]
fn permanent_gate_changes_destination_not_target_legality() {
    for kind in [CardType::Creature, CardType::Artifact, CardType::Battle,
        CardType::Enchantment, CardType::Planeswalker, CardType::Instant, CardType::Sorcery] {
        let (mut game, source, target, alice, _) = setup(kind);
        let stable = game.object(target).unwrap().stable_id;
        counter(&mut game, source, target, alice, CounterExileGate::PermanentSpell, false);
        let moved = game.find_object_by_stable_id(stable).unwrap();
        let permanent = !matches!(kind, CardType::Instant | CardType::Sorcery);
        assert_eq!(game.object(moved).unwrap().zone, if permanent { Zone::Exile } else { Zone::Graveyard });
        assert!(!game.stack.iter().any(|entry| entry.object_id == target));
        assert_eq!(prices(&game, moved, alice), usize::from(permanent));
    }
}

#[test]
fn unsuccessful_counter_never_creates_permission_or_later_replacement() {
    for absent in [false, true] {
        let (mut game, source, target, alice, _) = setup(CardType::Creature);
        if absent {
            game.move_object(target, Zone::Hand, EventCause::effect()).unwrap();
        } else {
            game.object_mut(target).unwrap().abilities_mut().push(
                crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::uncounterable())
                    .in_zones(vec![Zone::Stack]));
        }
        counter(&mut game, source, target, alice, CounterExileGate::AnySpell, true);
        assert!(game.effect_store.grant_registry.grants.is_empty());
        if !absent {
            assert_eq!(game.object(target).unwrap().zone, Zone::Stack);
            let later = game.move_object(target, Zone::Graveyard, EventCause::effect()).unwrap();
            assert_eq!(game.object(later).unwrap().zone, Zone::Graveyard);
        }
    }
}

#[test]
fn prevented_or_redirected_counter_arrival_cannot_authorize_an_exile() {
    for prevent in [false, true] {
        let (mut game, source, target, alice, _) = setup(CardType::Creature);
        let stable = game.object(target).unwrap().stable_id;
        game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(target), Some(Zone::Stack), Some(Zone::Exile)),
                if prevent { crate::replacement::ReplacementAction::Prevent }
                else { crate::replacement::ReplacementAction::ChangeDestination(Zone::Hand) }));
        counter(&mut game, source, target, alice, CounterExileGate::AnySpell, true);
        assert!(game.effect_store.grant_registry.grants.is_empty());
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone, if prevent { Zone::Stack } else { Zone::Hand });
    }
}

#[test]
fn departure_reentry_and_stale_counter_target_cannot_rebind_free_price() {
    for intermediate in [Zone::Graveyard, Zone::Hand, Zone::Stack] {
        let (mut game, source, target, alice, _) = setup(CardType::Creature);
        let stable = game.object(target).unwrap().stable_id;
        counter(&mut game, source, target, alice, CounterExileGate::AnySpell, true);
        let first = game.find_object_by_stable_id(stable).unwrap();
        let gone = game.move_object(first, intermediate, EventCause::effect()).unwrap();
        let reentered = game.move_object(gone, Zone::Exile, EventCause::effect()).unwrap();
        assert_ne!(first, reentered);
        assert_eq!(prices(&game, reentered, alice), 0);
        assert!(!offered(&game, reentered, alice));
        // Even an unrelated fresh ordinary permission cannot inherit the price.
        game.effect_store.grant_registry.grant_play_from_to_card(reentered, Zone::Exile, alice,
            crate::grant_registry::PlayFromConstraints::default(),
            crate::grant_registry::GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
        counter(&mut game, source, target, alice, CounterExileGate::AnySpell, true);
        assert_eq!(prices(&game, reentered, alice), 0);
        assert!(!offered(&game, reentered, alice), "new normal permission still owes six mana");
        let count = game.effect_store.grant_registry.grants.len();
        let mut dm = SelectFirstDecisionMaker;
        let ctx = ExecutionContext::new(source, alice, &mut dm);
        grant_countered_exile_permission(&mut game, &ctx, first,
            CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: true });
        assert_eq!(game.effect_store.grant_registry.grants.len(), count, "stale exact arrival creates no grant");
    }
}

#[test]
fn adventure_exception_preserves_its_own_stable_permission_not_the_counter_price() {
    let (mut game, source, old_target, alice, bob) = setup(CardType::Creature);
    game.move_object(old_target, Zone::Graveyard, EventCause::effect()).unwrap();
    let front_id = CardId::new();
    let back_id = CardId::new();
    let front = crate::cards::CardDefinitionBuilder::new(front_id, "Adventure creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(6)]))
        .other_face(back_id).other_face_name("Adventure journey")
        .build();
    let adventure = crate::cards::CardDefinitionBuilder::new(back_id, "Adventure journey")
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![crate::types::Subtype::Adventure])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(3)]))
        .other_face(front_id).other_face_name("Adventure creature")
        .build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&adventure);
    let target = game.create_object_from_definition(&front, bob, Zone::Stack);
    assert!(crate::decision::spell_has_adventure_half(&game, game.object(target).unwrap()));
    let stable = game.object(target).unwrap().stable_id;
    game.stack.push(StackEntry::new(target, bob));
    counter(&mut game, source, target, alice, CounterExileGate::AnySpell, false);
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(prices(&game, exiled, alice), 1);
    // An independent legitimate stable Adventure permission is a control for
    // the existing exception. The new counter owner must not modify it.
    game.effect_store.grant_registry.grant_to_stable_card(exiled, stable, Zone::Exile, alice,
        crate::grant::Grantable::PlayFrom,
        crate::grant_registry::GrantSource::Effect { source_id: source, expires_end_of_turn: u32::MAX });
    let stack = game.move_object(exiled, Zone::Stack, EventCause::effect()).unwrap();
    let returned = game.move_object(stack, Zone::Exile, EventCause::effect()).unwrap();
    assert_ne!(exiled, returned);
    assert_eq!(prices(&game, returned, alice), 0, "exact counter price ended when exile was left");
    assert!(game.effect_store.grant_registry.card_can_play_from_zone(&game, returned, Zone::Exile, alice),
        "existing Adventure stable permission remains legitimate");
    assert!(!offered(&game, returned, alice), "independent permission still owes the creature's mana cost");
}

#[derive(Debug, Clone)]
struct MoveArrivalAway(crate::ids::StableId);
impl EffectExecutor for MoveArrivalAway {
    fn execute(&self, game: &mut GameState, _: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        let exiled = game.find_object_by_stable_id(self.0).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        let hand = game.move_object(exiled, Zone::Hand, EventCause::effect()).unwrap();
        game.move_object(hand, Zone::Exile, EventCause::effect()).unwrap();
        Ok(EffectOutcome::resolved())
    }
}

#[test]
fn receipt_additions_cannot_transfer_price_to_a_later_exile_incarnation() {
    let (mut game, source, target, alice, _) = setup(CardType::Creature);
    let stable = game.object(target).unwrap().stable_id;
    game.effect_store.replacement_effects.add_one_shot_effect(
        crate::replacement::ReplacementEffect::with_matcher(source, alice,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                crate::target::ObjectFilter::specific(target), Some(Zone::Stack), Some(Zone::Exile)),
            crate::replacement::ReplacementAction::Additionally(vec![Effect::new(MoveArrivalAway(stable))])));
    counter(&mut game, source, target, alice, CounterExileGate::AnySpell, false);
    let current = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(current).unwrap().zone, Zone::Exile);
    assert_eq!(prices(&game, current, alice), 0);
    assert!(!offered(&game, current, alice));
}

#[derive(Default)]
struct PauseBoolean { pending: bool }
impl crate::decision::DecisionMaker for PauseBoolean {
    fn decide_boolean(&mut self, _: &GameState,
        _: &crate::decisions::context::BooleanContext) -> bool {
        self.pending = true;
        false
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}

#[test]
fn pending_or_failed_receipt_addition_rolls_back_price_and_counter_then_replays_once() {
    for fail in [false, true] {
        let (mut game, source, target, alice, _) = setup(CardType::Creature);
        let stable = game.object(target).unwrap().stable_id;
        let addition = if fail { Effect::lose_life(crate::effect::Value::X) }
            else { Effect::may(vec![Effect::gain_life(1)]) };
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(target), Some(Zone::Stack), Some(Zone::Exile)),
                crate::replacement::ReplacementAction::Additionally(vec![addition])));
        let mut dm = PauseBoolean::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let result = execute_effect(&mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target))
                .with_exile_permission(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: false })),
            &mut ctx);
        if fail { assert!(result.is_err()); } else {
            assert!(result.is_ok());
            assert!(ctx.decision_maker.awaiting_choice());
        }
        drop(ctx);
        assert_eq!(game.object(target).unwrap().zone, Zone::Stack);
        assert!(game.stack.iter().any(|entry| entry.object_id == target));
        assert!(game.effect_store.grant_registry.grants.is_empty());
        assert!(game.effect_store.replacement_effects.get_effect(replacement).is_some());
        if !fail {
            counter(&mut game, source, target, alice, CounterExileGate::AnySpell, false);
            let exiled = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(prices(&game, exiled, alice), 1);
            assert_eq!(game.player(alice).unwrap().life, 21);
        }
    }
}

#[test]
fn plural_or_tagged_rider_target_is_rejected_before_countering() {
    let mut source_filter = crate::target::ObjectFilter::spell();
    source_filter.source = true;
    let mut nested_tag_filter = crate::target::ObjectFilter::spell();
    nested_tag_filter.any_of.push(crate::target::ObjectFilter::default().match_tagged(
        "stale", crate::target::TaggedOpbjectRelation::IsTaggedObject));
    for target_spec in [ChooseSpec::All(crate::target::ObjectFilter::spell()),
        ChooseSpec::Object(source_filter),
        ChooseSpec::target(ChooseSpec::Object(nested_tag_filter)),
        ChooseSpec::Tagged("stale".into())] {
        let (mut game, source, target, alice, _) = setup(CardType::Creature);
        game.stack.push(StackEntry::new(source, alice));
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(target).unwrap(), &game);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects("stale", vec![snapshot]);
        let result = execute_effect(&mut game, &Effect::new(CounterEffect::new(target_spec)
            .with_exile_permission(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: true })),
            &mut ctx);
        assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))));
        assert_eq!(game.object(target).unwrap().zone, Zone::Stack);
        assert_eq!(game.object(source).unwrap().zone, Zone::Stack);
        assert_eq!(game.stack.len(), 2);
        assert!(game.effect_store.grant_registry.grants.is_empty());
    }
}

#[test]
fn permanent_gate_uses_stack_face_before_exile_reveals_the_card() {
    let (mut game, source, target, alice, _) = setup(CardType::Instant);
    let stable = game.object(target).unwrap().stable_id;
    assert!(game.set_face_down(target));
    assert!(game.current_card_types(target).unwrap().contains(&CardType::Creature));
    counter(&mut game, source, target, alice, CounterExileGate::PermanentSpell, false);
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    assert!(game.object(exiled).unwrap().card_types.contains(&CardType::Instant));
    assert_eq!(prices(&game, exiled, alice), 1,
        "the permanent spell on the stack qualified even though the revealed exile card is an instant");
}

#[test]
fn independently_exiled_nonpermanent_has_counter_receipt_but_no_gated_permission() {
    for kind in [CardType::Instant, CardType::Sorcery] {
        let (mut game, source, target, alice, _) = setup(kind);
        let stable = game.object(target).unwrap().stable_id;
        game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(target), Some(Zone::Stack), Some(Zone::Graveyard)),
                crate::replacement::ReplacementAction::ChangeDestination(Zone::Exile)));
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = CounterEffect::new(ChooseSpec::SpecificObject(target))
            .with_exile_permission(CounterExilePermission { gate: CounterExileGate::PermanentSpell, allow_land: false })
            .execute(&mut game, &mut ctx).unwrap();
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert!(outcome.events.iter().any(|event| event.downcast::<crate::events::SpellCounteredEvent>().is_some()),
            "the original counter committed, but its printed permanent gate did not apply");
        assert_eq!(prices(&game, exiled, alice), 0);
        assert!(!offered(&game, exiled, alice));
    }
}

#[test]
fn replacement_owned_exile_movements_are_not_original_counter_arrival_evidence() {
    let (mut game, source, target, alice, bob) = setup(CardType::Creature);
    let target_stable = game.object(target).unwrap().stable_id;
    let unrelated_card = CardBuilder::new(CardId::new(), "Replacement-owned unrelated card")
        .card_types(vec![CardType::Instant]).build();
    let unrelated = game.create_object_from_card(&unrelated_card, bob, Zone::Hand);
    let unrelated_stable = game.object(unrelated).unwrap().stable_id;
    // Source trace: ReplacementAction::Instead yields TraitApplyResult::Replaced
    // (events/processing/application.rs); its movements execute as separate
    // instructions. The counter's original Proceed receipt is absent. This is
    // a provenance control, not a claim about every possible rules replacement.
    game.effect_store.replacement_effects.add_one_shot_effect(
        crate::replacement::ReplacementEffect::with_matcher(source, alice,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                crate::target::ObjectFilter::specific(target), Some(Zone::Stack), Some(Zone::Exile)),
            crate::replacement::ReplacementAction::Instead(vec![
                Effect::new(crate::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(unrelated), Zone::Exile, false)),
                Effect::new(crate::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(target), Zone::Exile, false)),
            ])));
    let mut ctx = ExecutionContext::new_default(source, alice);
    let outcome = CounterEffect::new(ChooseSpec::SpecificObject(target))
        .with_exile_permission(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: true })
        .execute(&mut game, &mut ctx).unwrap();
    assert!(!outcome.events.iter().any(|event| event.downcast::<crate::events::SpellCounteredEvent>().is_some()),
        "this synthetic Instead program moved the original separately; it supplied no committed counter original");
    for stable in [target_stable, unrelated_stable] {
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert_eq!(prices(&game, exiled, alice), 0);
        assert!(!offered(&game, exiled, alice));
    }
    assert!(game.effect_store.grant_registry.grants.is_empty());
}

#[derive(Debug, Clone)]
struct NonconvergingCounterCharacteristics;
impl crate::static_abilities::StaticAbilityKind for NonconvergingCounterCharacteristics {
    fn id(&self) -> crate::static_abilities::StaticAbilityId {
        crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
    }
    fn display(&self) -> String { "Counter snapshot discovery failure fixture".into() }
    fn generate_effects(&self, source: ObjectId, controller: PlayerId, game: &GameState)
        -> Vec<crate::continuous::ContinuousEffect> {
        let crate::ability::AbilityKind::Static(parent) = &game.object(source).unwrap().abilities[0].kind
            else { panic!("fixture parent missing") };
        vec![crate::continuous::ContinuousEffect::new(source, controller,
            crate::continuous::EffectTarget::Source,
            crate::continuous::Modification::AddAbility(parent.clone()))]
    }
}

#[test]
fn checked_counter_characteristic_failure_rolls_back_without_original_or_grant() {
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        let (mut game, source, target, alice, bob) = setup(CardType::Creature);
        let host = CardBuilder::new(CardId::new(), "Unbounded characteristic host")
            .card_types(vec![CardType::Artifact]).build();
        let host = game.create_object_from_card(&host, alice, Zone::Battlefield);
        game.object_mut(host).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::new(NonconvergingCounterCharacteristics)));
        assert!(matches!(crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
            game.object(target).unwrap(), &game), Err(ExecutionError::ContinuousDiscovery(_))),
            "the fixture specifically cannot supply the checked pre-counter characteristics");
        let ids = game.next_object_id_counter();
        let revision = game.effect_store.continuous_effects.revision();
        let replacements = game.effect_store.replacement_effects.effects().len();
        let provenance = game.provenance_graph().node_count();
        let stack = format!("{:?}", game.stack);
        let exile = game.exile.clone();
        let graveyard = game.player(bob).unwrap().graveyard.clone();
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let instruction = CounterEffect::new(ChooseSpec::SpecificObject(target))
            .with_exile_permission(CounterExilePermission { gate: CounterExileGate::PermanentSpell, allow_land: false });
        let result = instruction.execute(&mut game, &mut ctx);
        assert!(matches!(result, Err(ExecutionError::ContinuousDiscovery(_))), "{result:?}");
        assert_eq!(game.object(target).unwrap().zone, Zone::Stack);
        assert_eq!(format!("{:?}", game.stack), stack);
        assert_eq!(game.exile, exile);
        assert_eq!(game.player(bob).unwrap().graveyard, graveyard);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), ids);
        assert_eq!(game.provenance_graph().node_count(), provenance);
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert_eq!(game.effect_store.replacement_effects.effects().len(), replacements);
        assert!(game.effect_store.grant_registry.grants.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(!ctx.decision_maker.awaiting_choice());
    }).unwrap().join().unwrap();
}
