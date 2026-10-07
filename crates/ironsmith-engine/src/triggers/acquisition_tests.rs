//! Source-only contracts for definition/acquisition retention. Not executed.
use super::*;
use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::card::CardBuilder;
use crate::continuous::{CalculatedAbilities, ContinuousEffect, Modification};
use crate::effect::{Effect, Until};
use crate::game_state::GameState;
use crate::ids::{CardId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::{Trigger, TriggerEvent, compute_trigger_identity};
use crate::zone::Zone;
use ironsmith_core::{Color, ColorSet, TextChange};

fn definition(stamp: u8, trigger: Trigger, amount: i32) -> Ability {
    Ability::triggered(trigger, crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life(amount)])
        .with_trigger_definition(LinkedExileDefinition([stamp; 32])))
}
fn colored_trigger() -> Trigger {
    Trigger::spell_cast(Some(ObjectFilter { colors: Some(ColorSet::BLACK), ..Default::default() }), PlayerFilter::You)
}
fn triggered(ability: &Ability) -> &TriggeredAbility {
    let AbilityKind::Triggered(triggered) = &ability.kind else { panic!("triggered fixture"); };
    triggered
}
fn bind(ability: Ability, host: u64, origin: AbilityOrigin) -> Ability {
    let mut ability = ability;
    bind_ability(&mut ability, Some(ObjectId::from_raw(host)), &origin);
    ability
}
fn rewrite(ability: &TriggeredAbility, from: Color, to: Color) -> TriggeredAbility {
    crate::continuous::text_change_programs::rewrite_triggered_ability_words(
        ability, TextChange::color(from, to).unwrap(),
    ).unwrap()
}
fn filter(ability: &TriggeredAbility) -> Option<ColorSet> {
    ability.trigger.downcast_ref::<crate::triggers::SpellCastTrigger>().unwrap()
        .filter.as_ref().unwrap().colors
}
fn body(abilities: Vec<Ability>) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Acquisition witness")
        .card_types(vec![crate::types::CardType::Enchantment]).build();
    let source = game.create_object_from_card(&card, PlayerId::from_index(0), Zone::Battlefield);
    game.object_mut(source).unwrap().abilities = abilities.into();
    (game, source)
}
fn current(game: &GameState, source: ObjectId) -> TriggeredAbility {
    triggered(&game.calculated_characteristics(source).unwrap().abilities[0]).clone()
}
fn change(game: &mut GameState, source: ObjectId, from: Color, to: Color, duration: Until) {
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
        source, PlayerId::from_index(0), vec![source],
        Modification::RewriteText(TextChange::color(from, to).unwrap()),
    ).until(duration));
    game.refresh_continuous_state().unwrap();
}

#[test]
fn ordered_rewrites_keep_identity_and_prior_capture_and_separate_shared_matchers() {
    let matcher = colored_trigger();
    let first = bind(definition(1, matcher.clone(), 1), 10, AbilityOrigin::Printed(0));
    let second = bind(definition(2, matcher, 2), 10, AbilityOrigin::Printed(1));
    assert_eq!(triggered(&first).trigger.runtime_matcher_identity(), triggered(&second).trigger.runtime_matcher_identity());
    let original = triggered(&first).clone();
    let identity = compute_trigger_identity(&original);
    let blue = rewrite(&original, Color::Black, Color::Blue);
    let blue_again = rewrite(&original, Color::Black, Color::Blue);
    let green = rewrite(&blue, Color::Blue, Color::Green);
    let independent = rewrite(triggered(&second), Color::Black, Color::Blue);
    assert_eq!(filter(&original), Some(ColorSet::BLACK));
    assert_eq!(filter(&blue), Some(ColorSet::BLUE));
    assert_eq!(filter(&green), Some(ColorSet::GREEN));
    for rewritten in [&blue, &blue_again, &green] {
        assert_eq!(compute_trigger_identity(rewritten), identity);
    }
    assert_eq!(blue.trigger.runtime_matcher_identity(), blue_again.trigger.runtime_matcher_identity());
    assert_ne!(compute_trigger_identity(&independent), identity, "shared matcher caches must not borrow another program's acquisition");
    assert_eq!(compute_trigger_identity(&independent), compute_trigger_identity(triggered(&second)));
}

#[test]
fn equal_printed_slots_independent_registered_grants_and_incarnations_are_distinct() {
    let authored = definition(3, colored_trigger(), 1);
    let first = bind(authored.clone(), 20, AbilityOrigin::Printed(0));
    let second = bind(authored.clone(), 20, AbilityOrigin::Printed(1));
    let reincarnated = bind(authored.clone(), 21, AbilityOrigin::Printed(0));
    let mut grants = crate::continuous::ContinuousEffectManager::new();
    let effect = ContinuousEffect::from_resolution(ObjectId::from_raw(99), PlayerId::from_index(0),
        vec![ObjectId::from_raw(20)], Modification::AddAbilityGeneric(authored.clone()));
    grants.add_effect(effect.clone()); grants.add_effect(effect);
    let grant = |index: usize| bind(authored.clone(), 20, AbilityOrigin::Effect {
        effect: (&grants.effects()[index]).into(), slot: 0,
    });
    let grant_a = grant(0); let grant_b = grant(1);
    let identities = [&first, &second, &reincarnated, &grant_a, &grant_b].map(|ability| compute_trigger_identity(triggered(ability)));
    assert_eq!(identities.into_iter().collect::<std::collections::HashSet<_>>().len(), 5);
    assert_eq!(compute_trigger_identity(&rewrite(triggered(&grant_a), Color::Black, Color::Blue)), identities[3]);
}

#[test]
fn first_discovery_text_change_and_expiry_share_once_per_turn_and_resolution_history() {
    let mut authored = definition(4, colored_trigger(), 1);
    let AbilityKind::Triggered(ability) = &mut authored.kind else { unreachable!() };
    ability.intervening_if = Some(crate::ConditionExpr::MaxTimesEachTurn(1));
    let (mut game, source) = body(vec![authored]);
    let view = crate::derived_view::DerivedGameView::new(&game);
    let fast = view.abilities_rc(source).unwrap();
    let original = triggered(&fast[0]).clone();
    let identity = compute_trigger_identity(&original);
    drop(view);
    assert_eq!(identity, compute_trigger_identity(&current(&game, source)), "fast discovery binds before any text edit");
    game.record_trigger_fired(source, identity);
    game.record_do_this_action(source, identity);
    game.record_triggered_ability_resolved(source, identity);
    change(&mut game, source, Color::Black, Color::Blue, Until::EndOfTurn);
    let rewritten = current(&game, source);
    let rewritten_identity = compute_trigger_identity(&rewritten);
    assert_eq!(rewritten_identity, identity);
    assert_eq!(game.trigger_fire_count_this_turn(source, rewritten_identity), 1);
    assert_eq!(game.do_this_action_count_this_turn(source, rewritten_identity), 1);
    assert_eq!(game.triggered_ability_resolution_count_this_turn(source, rewritten_identity), 1);
    let event = TriggerEvent::new(crate::events::spells::SpellCastEvent::new(
        source, PlayerId::from_index(0), Zone::Hand,
    ), Default::default());
    assert!(!crate::triggers::verify_intervening_if(&game, rewritten.intervening_if.as_ref().unwrap(),
        PlayerId::from_index(0), &event, source, Some(rewritten_identity), None));
    assert_eq!(filter(&original), Some(ColorSet::BLACK));
    assert_eq!(filter(&rewritten), Some(ColorSet::BLUE));
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state().unwrap();
    let expired = current(&game, source);
    assert_eq!(filter(&expired), Some(ColorSet::BLACK));
    assert_eq!(compute_trigger_identity(&expired), identity);
    assert_eq!(game.trigger_fire_count_this_turn(source, identity), 1);
}

#[test]
fn copied_definition_rebinds_acquisition_and_face_replacement_changes_definition() {
    let (mut game, source) = body(vec![definition(5, colored_trigger(), 1)]);
    let original = current(&game, source);
    let copied = bind(Ability { kind: AbilityKind::Triggered(original.clone()), functional_zones: vec![Zone::Battlefield] },
        source.0 + 1000, AbilityOrigin::Printed(0));
    assert_eq!(triggered(&copied).effects.retained_trigger_definition(), original.effects.retained_trigger_definition());
    assert_ne!(compute_trigger_identity(triggered(&copied)), compute_trigger_identity(&original));
    game.object_mut(source).unwrap().abilities = vec![definition(6, colored_trigger(), 1)].into();
    game.refresh_continuous_state().unwrap();
    let changed_face = compute_trigger_identity(&current(&game, source));
    assert_ne!(changed_face, compute_trigger_identity(&original));
    let departed = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let returned = game.move_object_by_effect(departed, Zone::Battlefield).unwrap();
    assert_ne!(source, returned);
    assert_ne!(compute_trigger_identity(&current(&game, returned)), changed_face);
}

#[test]
fn layer_one_copy_keeps_frozen_definition_after_donor_changes_and_leaves() {
    let (mut game, donor) = body(vec![definition(10, colored_trigger(), 1)]);
    let values = crate::snapshot::CopiableValues::from_object(game.object(donor).unwrap());
    let receiver = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Receiving host")
        .card_types(vec![crate::types::CardType::Enchantment]).build(), PlayerId::from_index(0), Zone::Battlefield);
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
        donor, PlayerId::from_index(0), vec![receiver], Modification::CopyOf {
            target_id: donor, copiable_values: Box::new(values), preserve_source_abilities: false,
            name_override: None, name_override_surface: None, add_supertypes: vec![],
        },
    ).until(Until::Forever));
    let donor_identity = compute_trigger_identity(&current(&game, donor));
    let original_copy = current(&game, receiver);
    let copy_identity = compute_trigger_identity(&original_copy);
    assert_ne!(copy_identity, donor_identity);
    game.record_trigger_fired(receiver, copy_identity);
    change(&mut game, receiver, Color::Black, Color::Blue, Until::Forever);
    assert_eq!(compute_trigger_identity(&current(&game, receiver)), copy_identity);
    assert_eq!(filter(&original_copy), Some(ColorSet::BLACK));
    assert_eq!(filter(&current(&game, receiver)), Some(ColorSet::BLUE));
    game.object_mut(donor).unwrap().abilities = vec![definition(11, colored_trigger(), 9)].into();
    game.move_object_by_effect(donor, Zone::Graveyard).unwrap();
    game.refresh_continuous_state().unwrap();
    let after_departure = current(&game, receiver);
    assert_eq!(after_departure.effects.retained_trigger_definition(), Some(LinkedExileDefinition([10; 32])));
    assert_eq!(compute_trigger_identity(&after_departure), copy_identity);
    assert_eq!(game.trigger_fire_count_this_turn(receiver, copy_identity), 1);
}

#[test]
fn missing_definition_or_host_is_a_checked_hold_but_a_quoted_definition_can_be_rewritten() {
    let unstamped = Ability::triggered(colored_trigger(), vec![Effect::gain_life(1)]);
    assert!(crate::continuous::text_change_programs::rewrite_triggered_ability_words(
        triggered(&unstamped), TextChange::color(Color::Black, Color::Blue).unwrap(),
    ).is_err());
    let authored = definition(7, colored_trigger(), 1);
    let quoted = rewrite(triggered(&authored), Color::Black, Color::Blue);
    assert!(quoted.trigger.acquired_identity(quoted.effects.retained_trigger_definition()).is_none());
    let acquired = bind(authored, 70, AbilityOrigin::Printed(0));
    let orphaned = CalculatedAbilities::from(vec![acquired]);
    assert!(triggered(&orphaned[0]).trigger.acquired_identity(triggered(&orphaned[0]).effects.retained_trigger_definition()).is_none());
    let (mut game, source) = body(vec![unstamped]);
    let old = current(&game, source);
    assert_eq!(compute_trigger_identity(&old), compute_trigger_identity(&current(&game, source)));
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
        source, PlayerId::from_index(0), vec![source],
        Modification::RewriteText(TextChange::color(Color::Black, Color::Blue).unwrap()),
    ));
    assert_eq!(game.calculated_characteristics(source).unwrap().text_change_error,
        Some(crate::continuous::text_changes::TextChangeDomainError::TriggeredAbility));
}

#[test]
fn immutable_native_snapshots_keep_acquisition_and_history_across_game_clone() {
    let (mut game, source) = body(vec![definition(8, colored_trigger(), 1)]);
    let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
    let captured = triggered(&snapshot.abilities[0]).clone();
    let identity = compute_trigger_identity(&captured);
    game.record_trigger_fired(source, identity);
    let saved = game.clone();
    change(&mut game, source, Color::Black, Color::Blue, Until::Forever);
    assert_eq!(compute_trigger_identity(&current(&game, source)), identity);
    assert_eq!(filter(&captured), Some(ColorSet::BLACK));
    game = saved;
    assert_eq!(filter(&current(&game, source)), Some(ColorSet::BLACK));
    assert_eq!(compute_trigger_identity(&current(&game, source)), identity);
    assert_eq!(game.trigger_fire_count_this_turn(source, identity), 1);
    assert_eq!(snapshot.ability_origins.as_ref().unwrap()[0], AbilityOrigin::Printed(0));
    let mut claim = snapshot.clone();
    claim.strip_to_public_claim_form();
    assert!(claim.abilities.is_empty());
    assert!(claim.ability_origins.is_none());
}

#[test]
fn a_body_edit_does_not_reset_the_first_event_history_owner() {
    let mut authored = definition(9, Trigger::spell_cast(None, PlayerFilter::You), 1);
    let AbilityKind::Triggered(ability) = &mut authored.kind else { unreachable!() };
    ability.intervening_if = Some(crate::ConditionExpr::FirstTimeThisTurn);
    ability.choices = vec![crate::target::ChooseSpec::Object(ObjectFilter {
        colors: Some(ColorSet::BLACK), ..Default::default()
    })];
    let (mut game, source) = body(vec![authored]);
    let original = current(&game, source);
    let identity = compute_trigger_identity(&original);
    let event = |id| TriggerEvent::new(crate::events::spells::SpellCastEvent::new(
        ObjectId::from_raw(id), PlayerId::from_index(0), Zone::Hand,
    ), Default::default());
    let earlier = event(10000);
    let later = event(10001);
    game.record_turn_history_event(&earlier);
    change(&mut game, source, Color::Black, Color::Blue, Until::Forever);
    let changed = current(&game, source);
    let context = crate::triggers::TriggerContext::for_source(source, PlayerId::from_index(0), &game)
        .with_trigger_identity(compute_trigger_identity(&changed));
    assert_eq!(compute_trigger_identity(&changed), identity);
    assert!(!crate::triggers::check::first_time_this_turn_event(&game, &changed, &later, &context));
    assert!(!crate::triggers::check::first_time_this_turn_event(&game, &original, &later, &context));
    let crate::target::ChooseSpec::Object(filter) = &changed.choices[0] else { unreachable!() };
    assert_eq!(filter.colors, Some(ColorSet::BLUE));
}
