//! Checked current/retained type evidence; restored native contracts, UNRUN.
use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::continuous::Modification;
use crate::effect::{Condition, Effect, Until, Value};
use crate::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor, ExecutionError, ResolvedTarget, execute_effect};
use crate::triggers::{verify_intervening_if_checked, verify_intervening_if_at_resolution_checked};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn game() -> GameState { GameState::new(vec!["A".into(), "B".into()], 20) }
fn card(game: &mut GameState, zone: Zone, types: Vec<CardType>) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(crate::CardId::new(), "Reference").card_types(types)
        .power_toughness(PowerToughness::fixed(1, 1)).build(), A, zone)
}
fn set_types(game: &mut GameState, object: ObjectId, types: Vec<CardType>) {
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(object), Modification::SetCardTypes(types), Until::EndOfTurn)
        .execute(game, &mut EffectContext::new_default(object, A)).unwrap();
}
fn tagged(tag: &str) -> ObjectFilter { ObjectFilter::default().shares_card_type_with_tagged(tag) }
fn live(negative: bool) -> ObjectFilter {
    let relation = if negative { ObjectCharacteristicRelation::shares_none(vec![ObjectCharacteristic::CardType], ObjectFilter::permanent()) }
        else { ObjectCharacteristicRelation::shares(vec![ObjectCharacteristic::CardType], ObjectFilter::permanent()) }.excluding_candidate();
    let mut filter = ObjectFilter::default(); filter.characteristic_relations.push(relation); filter
}
fn event(event: impl crate::events::GameEventType + 'static) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(event, crate::provenance::ProvNodeId::default())
}
#[test]
fn source_and_linked_exile_types_are_current_while_arbitrary_tags_remain_retained() {
    let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Land]);
    let candidate = card(&mut game, Zone::Battlefield, vec![CardType::Artifact]);
    let mut ctx = game.filter_context_for(A, Some(source));
    ctx.tagged_objects.insert("historical".into(), vec![ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game)]);
    set_types(&mut game, source, vec![CardType::Artifact]);
    assert!(tagged(crate::tag::SOURCE_OBJECT_TAG).matches(game.object(candidate).unwrap(), &ctx, &game));
    assert!(!tagged("historical").matches(game.object(candidate).unwrap(), &ctx, &game));
    let moved = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let returned = game.move_object_by_effect(moved, Zone::Battlefield).unwrap();
    assert_eq!(game.current_card_types(returned).unwrap(), vec![CardType::Land]);
    assert!(tagged(crate::tag::SOURCE_OBJECT_TAG).matches(game.object(candidate).unwrap(), &ctx, &game));
    let exiled = card(&mut game, Zone::Exile, vec![CardType::Land]); game.add_exiled_with_source_link(source, exiled);
    assert!(!tagged(crate::tag::SOURCE_EXILED_TAG).matches(game.object(candidate).unwrap(), &ctx, &game));
    set_types(&mut game, exiled, vec![CardType::Artifact]);
    assert!(tagged(crate::tag::SOURCE_EXILED_TAG).matches(game.object(candidate).unwrap(), &ctx, &game));
    let hand = game.move_object_by_effect(exiled, Zone::Hand).unwrap(); game.move_object_by_effect(hand, Zone::Exile).unwrap();
    assert!(!tagged(crate::tag::SOURCE_EXILED_TAG).matches(game.object(candidate).unwrap(), &ctx, &game));
}
#[test]
fn live_comparison_excludes_the_candidate_and_reads_copied_types_and_phasing() {
    let mut game = game(); let candidate = card(&mut game, Zone::Battlefield, vec![CardType::Artifact]);
    let donor = card(&mut game, Zone::Battlefield, vec![CardType::Creature]); let ctx = game.filter_context_for(A, Some(candidate));
    assert!(!live(false).matches(game.object(candidate).unwrap(), &ctx, &game));
    assert!(live(true).matches(game.object(candidate).unwrap(), &ctx, &game));
    let values = crate::snapshot::CopiableValues::from_object(game.object(candidate).unwrap());
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(donor), Modification::CopyOf {
        target_id: candidate, copiable_values: Box::new(values), preserve_source_abilities: false,
        name_override: None, name_override_surface: None, add_supertypes: vec![],
    }, Until::EndOfTurn).execute(&mut game, &mut EffectContext::new_default(donor, A)).unwrap();
    assert!(live(false).matches(game.object(candidate).unwrap(), &ctx, &game));
    assert!(!live(true).matches(game.object(candidate).unwrap(), &ctx, &game));
    game.phase_out(donor); assert!(live(true).matches(game.object(candidate).unwrap(), &ctx, &game));
}
#[test]
fn unavailable_type_evidence_cannot_commit_outer_negation_or_zero_counts_and_recovers() {
    for filter in [live(false), live(true), tagged(crate::tag::SOURCE_OBJECT_TAG), tagged(crate::tag::SOURCE_EXILED_TAG)] { for counted in [false, true] {
        let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Land]);
        let candidate = card(&mut game, Zone::Battlefield, vec![CardType::Artifact]);
        let exiled = card(&mut game, Zone::Exile, vec![CardType::Land]); game.add_exiled_with_source_link(source, exiled);
        let pool = game.player(B).unwrap().mana_pool.clone(); game.player_mut(B).unwrap().mana_pool.blue = u32::MAX;
        let effect = if counted { Effect::gain_life(Value::Add(Box::new(Value::Fixed(1)), Box::new(Value::Count(filter.clone())))) }
            else { Effect::conditional_only(Condition::Not(Box::new(Condition::TargetMatches(filter.clone()))), vec![Effect::gain_life(7)]) };
        let mut ctx = EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Object(candidate)]);
        assert!(execute_effect(&mut game, &effect, &mut ctx).unwrap_err().is_incomplete_execution());
        assert_eq!(game.player(A).unwrap().life, 20); assert!(game.take_pending_trigger_events().is_empty());
        game.player_mut(B).unwrap().mana_pool = pool; execute_effect(&mut game, &Effect::gain_life(1), &mut ctx).unwrap();
        assert_eq!(game.player(A).unwrap().life, 21);
    }}
}
#[test]
fn known_typeless_source_and_empty_linked_set_differ_from_missing_source_evidence() {
    let mut game = game(); let target = card(&mut game, Zone::Battlefield, vec![CardType::Artifact]);
    let source = ObjectId::from_raw(999_991); let mut ctx = EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Object(target)]);
    let effect = Effect::conditional_only(Condition::Not(Box::new(Condition::TargetMatches(tagged(crate::tag::SOURCE_OBJECT_TAG)))), vec![Effect::gain_life(2)]);
    assert!(matches!(execute_effect(&mut game, &effect, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
    let mut known = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(target).unwrap(), &game);
    known.object_id = source; known.card_types.clear(); ctx.source_snapshot = Some(known);
    execute_effect(&mut game, &effect, &mut ctx).unwrap(); assert_eq!(game.player(A).unwrap().life, 22);
    let empty = Effect::conditional_only(Condition::Not(Box::new(Condition::TargetMatches(tagged(crate::tag::SOURCE_EXILED_TAG)))), vec![Effect::gain_life(2)]);
    execute_effect(&mut game, &empty, &mut ctx).unwrap(); assert_eq!(game.player(A).unwrap().life, 24);
}
#[test]
fn checked_admission_and_resolution_share_source_but_use_distinct_characteristic_instants() {
    let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
    let linked = card(&mut game, Zone::Exile, vec![CardType::Land]); game.add_exiled_with_source_link(source, linked);
    let land = card(&mut game, Zone::Battlefield, vec![CardType::Land]);
    let event = event(crate::events::LandPlayedEvent::with_current_snapshot(land, B, Zone::Hand, Zone::Battlefield, &game).unwrap());
    let condition = Condition::TaggedObjectMatches("triggering".into(), tagged(crate::tag::SOURCE_EXILED_TAG));
    assert!(verify_intervening_if_checked(&game, &condition, A, &event, source, None, None).unwrap());
    set_types(&mut game, land, vec![CardType::Artifact]);
    assert!(verify_intervening_if_checked(&game, &condition, A, &event, source, None, None).unwrap());
    assert!(!verify_intervening_if_at_resolution_checked(&game, &condition, A, &event, source, None, None).unwrap());
    let moved = game.move_object_by_effect(land, Zone::Graveyard).unwrap(); assert_eq!(game.current_card_types(moved).unwrap(), vec![CardType::Land]);
    assert!(!verify_intervening_if_at_resolution_checked(&game, &condition, A, &event, source, None, None).unwrap());
    assert_eq!(event.trigger_player(), Some(B));
}
#[test]
fn legacy_land_actor_notice_cannot_invent_a_completed_characteristic_frame() {
    use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
    for resolution in [false, true] { for negative in [false, true] {
        let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
        let land = card(&mut game, Zone::Battlefield, vec![CardType::Land]);
        let notice = event(crate::events::LandPlayedEvent::new(land, B, Zone::Hand));
        set_types(&mut game, land, vec![CardType::Artifact]);
        let actor_only = crate::triggers::PlayerPlaysLandTrigger::new(PlayerFilter::Opponent, ObjectFilter::default());
        assert!(actor_only.matches(&notice, &TriggerContext::for_source(source, A, &game)));
        let predicate = Condition::TaggedObjectMatches("triggering".into(), ObjectFilter::artifact());
        let condition = if negative { Condition::Not(Box::new(predicate)) } else { predicate };
        let result = if resolution { verify_intervening_if_at_resolution_checked(&game, &condition, A, &notice, source, None, None) }
            else { verify_intervening_if_checked(&game, &condition, A, &notice, source, None, None) };
        assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))), "mode={resolution} negative={negative}: {result:?}");
    }}
}
#[test]
fn existing_sigil_current_flag_rechecks_one_one_but_past_tense_stays_on_event_lki() {
    let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
    let entrant = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
    let mut entered = crate::events::EnterBattlefieldEvent::new(entrant, Zone::Hand);
    entered.completed_snapshot = Some(ObjectSnapshot::from_object_with_calculated_characteristics(game.object(entrant).unwrap(), &game));
    let event = event(entered); let filter = ObjectFilter { power: Some(Comparison::Equal(1)), toughness: Some(Comparison::Equal(1)), ..Default::default() };
    let current = Condition::TaggedObjectMatches("triggering".into(), filter.clone());
    let past = Condition::TaggedObjectMatchedLastKnown("triggering".into(), filter);
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(entrant), Modification::ModifyPowerToughness { power: 1, toughness: 1 }, Until::EndOfTurn)
        .execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
    assert!(verify_intervening_if_checked(&game, &current, A, &event, source, None, None).unwrap());
    assert!(!verify_intervening_if_at_resolution_checked(&game, &current, A, &event, source, None, None).unwrap());
    assert!(verify_intervening_if_at_resolution_checked(&game, &past, A, &event, source, None, None).unwrap());
}
#[test]
fn negative_characteristic_gates_reject_missing_destination_and_wrong_identity_or_zone_receipts() {
    for resolution in [false, true] { for malformed in [0, 1, 2] {
        let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
        let object = card(&mut game, if malformed == 2 { Zone::Graveyard } else { Zone::Battlefield }, vec![CardType::Land]);
        let event = match malformed {
            0 => event(crate::events::ZoneChangeEvent::with_cause(object, Zone::Hand, Zone::Battlefield, crate::events::EventCause::effect(), None)),
            _ => {
                let mut played = crate::events::LandPlayedEvent::new(if malformed == 1 { ObjectId::from_raw(998_444) } else { object }, B, Zone::Hand);
                played.snapshot = Some(ObjectSnapshot::from_object_with_calculated_characteristics(game.object(object).unwrap(), &game));
                played.completed_destination = Some(Zone::Battlefield);
                event(played)
            }
        };
        let condition = Condition::Not(Box::new(Condition::TaggedObjectMatches("triggering".into(), ObjectFilter::land())));
        let result = if resolution { verify_intervening_if_at_resolution_checked(&game, &condition, A, &event, source, None, None) }
            else { verify_intervening_if_checked(&game, &condition, A, &event, source, None, None) };
        assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))), "mode={resolution} malformed={malformed}: {result:?}");
    }}
}
#[test]
fn trusted_destination_mapping_allows_current_resolution_but_admission_still_needs_its_snapshot() {
    for resolution in [false, true] {
        let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
        let entrant = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
        let mut change = crate::events::ZoneChangeEvent::with_cause(ObjectId::from_raw(988_111), Zone::Hand, Zone::Battlefield, crate::events::EventCause::effect(), None);
        change.result_objects = vec![entrant]; let event = event(change);
        let condition = Condition::TaggedObjectMatches("triggering".into(), ObjectFilter::creature());
        if resolution { assert!(verify_intervening_if_at_resolution_checked(&game, &condition, A, &event, source, None, None).unwrap()); }
        else { assert!(matches!(verify_intervening_if_checked(&game, &condition, A, &event, source, None, None), Err(ExecutionError::IncompleteEvidence(_)))); }
    }
}
#[test]
fn explicit_redirected_play_destination_is_valid_and_uses_exact_departure_characteristics() {
    let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
    let linked = card(&mut game, Zone::Exile, vec![CardType::Land]); game.add_exiled_with_source_link(source, linked);
    let played = card(&mut game, Zone::Graveyard, vec![CardType::Land]);
    let notice = crate::events::LandPlayedEvent::with_current_snapshot(played, B, Zone::Exile, Zone::Graveyard, &game).unwrap();
    assert_eq!(notice.completed_destination, Some(Zone::Graveyard)); let event = event(notice);
    let condition = Condition::TaggedObjectMatches("triggering".into(), tagged(crate::tag::SOURCE_EXILED_TAG));
    assert!(verify_intervening_if_checked(&game, &condition, A, &event, source, None, None).unwrap());
    assert!(verify_intervening_if_at_resolution_checked(&game, &condition, A, &event, source, None, None).unwrap());
    set_types(&mut game, played, vec![CardType::Artifact]); game.move_object_by_effect(played, Zone::Hand).unwrap();
    assert!(!verify_intervening_if_at_resolution_checked(&game, &condition, A, &event, source, None, None).unwrap());
}
#[test]
fn singular_entry_reference_capture_preserves_current_departure_and_phasing_types() {
    for transition in [0, 1, 2] {
        let mut game = game(); let source = card(&mut game, Zone::Battlefield, vec![CardType::Creature]);
        let entrant = card(&mut game, Zone::Battlefield, vec![CardType::Artifact]);
        let mut entered = crate::events::EnterBattlefieldEvent::new(entrant, Zone::Hand);
        entered.completed_snapshot = Some(ObjectSnapshot::from_object_with_calculated_characteristics(game.object(entrant).unwrap(), &game));
        let event = event(entered); set_types(&mut game, entrant, vec![CardType::Enchantment]);
        if transition == 1 { game.move_object_by_effect(entrant, Zone::Graveyard).unwrap(); }
        if transition == 2 { game.phase_out(entrant); }
        let mut ctx = EffectContext::new_default(source, A).with_triggering_event(event);
        crate::effects::TagTriggeringObjectEffect::new("triggering").execute(&mut game, &mut ctx).unwrap();
        assert_eq!(ctx.get_tagged("triggering").unwrap().object_id, entrant);
        assert_eq!(ctx.get_tagged("triggering").unwrap().card_types, vec![CardType::Enchantment]);
        let observed = card(&mut game, Zone::Library, vec![CardType::Land]);
        ctx.set_tagged_objects("looked", vec![ObjectSnapshot::from_object(game.object(observed).unwrap(), &game)]);
        assert_eq!(ctx.get_tagged("triggering").unwrap().object_id, entrant);
    }
}
