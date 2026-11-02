//! Tests for the shared current-rule and immutable physical-action admission owner.
use super::*;
use crate::{Ability, CardDefinitionBuilder, CardId, CardType, Effect};
use crate::effect::{Restriction, Until};
use crate::effects::{CantEffect, ExecutionContext, execute_effect};
use crate::static_abilities::{CompiledStaticAbility, StaticAbility};
const A: PlayerId = PlayerId(0);
fn restriction() -> Restriction {
    Restriction::MoveBetweenZones { from: Zone::Graveyard, to: None, initial_state: None, cards_only: true }
}
fn guard(game: &mut GameState) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Origin-zone guardian")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::restriction(restriction(), "Cards cannot leave this origin".into())))
        .build();
    game.create_object_from_definition(&definition, A, Zone::Battlefield)
}
fn card(game: &mut GameState, zone: Zone) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Ordinary zone card")
        .card_types(vec![CardType::Artifact]).build();
    // Composite movement retains each physical source's complete definition.
    // Supply this fixture's graph explicitly rather than falling through to
    // the production catalog's parser-backed legacy built-in definitions.
    game.register_linked_face_definition(&definition);
    game.create_object_from_definition(&definition, A, zone)
}
#[test]
fn direct_moves_observe_provider_entry_departure_and_ability_loss_without_external_refresh() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let target = card(&mut game, Zone::Graveyard);
    assert!(!game.zone_move_is_prohibited(target, Zone::Hand));
    let provider = guard(&mut game);
    assert!(game.zone_move_is_prohibited(target, Zone::Hand), "new provider is active without priming its tracker");
    assert!(game.move_object_by_effect(target, Zone::Hand).is_none());
    let land = CardDefinitionBuilder::new(CardId::new(), "Land permission witness").card_types(vec![CardType::Land]).build();
    let land = game.create_object_from_definition(&land, A, Zone::Graveyard);
    assert!(crate::effects::zones::land_play_restriction_applies(&game, A, land).unwrap(),
        "land-play permission cannot override origin-zone prohibition either");
    game.move_object_by_effect(provider, Zone::Exile).unwrap();
    let moved = game.move_object_by_effect(target, Zone::Hand).expect("departed provider cannot leave a stale lock");
    assert_eq!(game.object(moved).unwrap().zone, Zone::Hand);
    let target = card(&mut game, Zone::Graveyard);
    let provider = guard(&mut game);
    assert!(game.zone_move_is_prohibited(target, Zone::Exile));
    game.object_mut(provider).unwrap().abilities_mut().clear();
    assert!(game.move_object_by_effect(target, Zone::Exile).is_some(), "removed ability cannot leave a stale lock");
}
#[test]
fn end_of_turn_rule_expires_and_card_copy_is_not_a_physical_card() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let source = card(&mut game, Zone::Battlefield);
    let target = card(&mut game, Zone::Graveyard);
    let effect = Effect::new(CantEffect::new(restriction(), Until::EndOfTurn));
    execute_effect(&mut game, &effect, &mut ExecutionContext::new_default(source, A)).unwrap();
    assert!(game.zone_move_is_prohibited(target, Zone::Exile));
    let copy_id = game.new_object_id();
    let mut copy = crate::object::Object::spell_copy_of(game.object(target).unwrap(), copy_id, A);
    copy.zone = Zone::Graveyard;
    game.add_object(copy);
    assert!(!game.zone_move_is_prohibited(copy_id, Zone::Stack));
    assert!(game.move_object_by_effect(copy_id, Zone::Stack).is_some(), "707.12 card-copy casting is not physical-card movement");
    crate::turn::execute_cleanup_step(&mut game);
    assert!(game.effect_store.restriction_effects.is_empty(), "the EOT instance itself expired");
    assert!(game.effect_store.cant_effects.cant_move_between_zones.is_empty(), "rebuilding admission must discard the expired rule's cached row");
    assert!(!game.zone_move_is_prohibited(target, Zone::Exile));
    assert!(game.move_object_by_effect(target, Zone::Exile).is_some());
}
#[test]
fn a_prohibited_entry_never_runs_as_entry_effects_and_the_same_fixture_runs_when_allowed() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let provider = guard(&mut game);
    let program = crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life(4)]);
    let entry_rule = CompiledStaticAbility::as_enters_effect_program(program, "this permanent", false, false, None);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Entry-program witness")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::from_model(entry_rule))).build();
    let entrant = game.create_object_from_definition(&definition, A, Zone::Graveyard);
    let next_id = game.next_object_id_counter();
    let result = game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
    assert!(matches!(result.original, crate::events::processing::EventOutcome::NotApplicable | crate::events::processing::EventOutcome::Prevented));
    assert!(!result.pending); assert!(result.programs.is_empty()); assert!(result.published_outputs.is_empty());
    assert_eq!(game.player(A).unwrap().life, 20, "forbidden entry cannot spend or execute its as-entry program");
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Graveyard); assert_eq!(game.next_object_id_counter(), next_id);
    game.move_object_by_effect(provider, Zone::Exile).unwrap();
    crate::tests::test_helpers::enter_fixture(&mut game, entrant, "same entry program now admitted");
    assert_eq!(game.player(A).unwrap().life, 24, "the positive control proves the real as-entry program was present");
}
#[test]
fn simultaneous_originals_retain_admission_when_a_provider_enters_or_departs() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let provider = guard(&mut game);
    let other = card(&mut game, Zone::Graveyard);
    let opened = game.open_simultaneous_action();
    game.move_object_by_effect(provider, Zone::Exile).unwrap();
    assert!(game.move_object_by_effect(other, Zone::Hand).is_none(), "a source departing in this same physical action was initially active");
    game.close_simultaneous_action(opened);
    assert!(game.move_object_by_effect(other, Zone::Hand).is_some());
    let provider = guard(&mut game);
    let provider = game.move_object_by_effect(provider, Zone::Graveyard).unwrap();
    let other = card(&mut game, Zone::Graveyard);
    let resumed_original = card(&mut game, Zone::Graveyard);
    let opened = game.open_simultaneous_action();
    game.move_object_by_effect(provider, Zone::Battlefield).unwrap();
    assert!(game.move_object_by_effect(other, Zone::Battlefield).is_some(), "a newly entering provider cannot deny its same-action original sibling");
    let frame = game.capture_simultaneous_original_frame();
    game.close_simultaneous_action(opened);
    assert!(game.zone_move_is_prohibited(resumed_original, Zone::Hand));
    game.with_simultaneous_original_frame(&frame, |game| -> Result<(), ()> {
        assert!(!game.zone_move_is_prohibited(resumed_original, Zone::Hand), "the retained original owns its captured admission, not added-program providers"); Ok(())
    }).unwrap();
    assert!(game.zone_move_is_prohibited(resumed_original, Zone::Hand), "caller admission is restored after the original frame");
}

#[test]
fn sequential_return_instructions_see_a_new_provider_but_one_batched_return_does_not() {
    use crate::effects::{MoveToZoneEffect, SequenceEffect};
    use crate::target::ChooseSpec;
    for batched in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let provider = guard(&mut game);
        let provider = game.move_object_by_effect(provider, Zone::Graveyard).unwrap();
        let provider_stable = game.object(provider).unwrap().stable_id;
        let actor = card(&mut game, Zone::Battlefield);
        let other = card(&mut game, Zone::Graveyard);
        let other_stable = game.object(other).unwrap().stable_id;
        let effect = if batched {
            Effect::new(MoveToZoneEffect::new(ChooseSpec::Tagged("returnees".into()), Zone::Battlefield, false))
        } else {
            Effect::new(SequenceEffect::new(vec![
                Effect::new(MoveToZoneEffect::new(ChooseSpec::SpecificObject(provider), Zone::Battlefield, false)),
                Effect::new(MoveToZoneEffect::new(ChooseSpec::SpecificObject(other), Zone::Battlefield, false)),
            ]))
        };
        let snapshots = [provider, other].into_iter().map(|id| crate::snapshot::ObjectSnapshot::from_object_id(&game, id).unwrap()).collect();
        let mut ctx = ExecutionContext::new_default(actor, A); ctx.set_tagged_objects("returnees", snapshots);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();
        let provider = game.find_object_by_stable_id(provider_stable).unwrap();
        let other = game.find_object_by_stable_id(other_stable).unwrap();
        assert_eq!(game.object(provider).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(other).unwrap().zone, if batched { Zone::Battlefield } else { Zone::Graveyard },
            "one physical entry batch differs from two sequential source instructions");
    }
}
#[test]
fn current_continuous_ability_loss_and_its_expiration_change_the_same_admission_owner() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let provider = guard(&mut game);
    let target = card(&mut game, Zone::Graveyard);
    assert!(game.zone_move_is_prohibited(target, Zone::Hand));
    let effect = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
        crate::target::ChooseSpec::SpecificObject(provider), crate::continuous::Modification::RemoveAllAbilities, Until::EndOfTurn));
    execute_effect(&mut game, &effect, &mut ExecutionContext::new_default(provider, A)).unwrap();
    assert!(!game.zone_move_is_prohibited(target, Zone::Hand));
    assert!(game.move_object_by_effect(target, Zone::Hand).is_some());
    let target = card(&mut game, Zone::Graveyard);
    crate::turn::execute_cleanup_step(&mut game);
    assert!(game.zone_move_is_prohibited(target, Zone::Hand));
    assert!(game.move_object_by_effect(target, Zone::Hand).is_none());
}
#[test]
fn a_composite_original_checks_every_component_before_any_entry_program_or_removal() {
    use crate::effects::zones::{BattlefieldEntryOptions, BattlefieldEntryOutcome};
    for protected_component in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let actor = card(&mut game, Zone::Battlefield);
        let primary = card(&mut game, Zone::Command);
        let sibling = card(&mut game, Zone::Command);
        if protected_component { game.set_face_down(sibling); }
        game.capture_game_start_object_states();
        assert_eq!(game.initial_object_state(primary).unwrap(), Some((Zone::Command, false)));
        assert_eq!(game.initial_object_state(sibling).unwrap(), Some((Zone::Command, protected_component)));
        let restriction = Restriction::MoveBetweenZones {
            from: Zone::Command, to: None, initial_state: Some((Zone::Command, true)), cards_only: true,
        };
        execute_effect(&mut game, &Effect::new(CantEffect::new(restriction, Until::EndOfTurn)),
            &mut ExecutionContext::new_default(actor, A)).unwrap();
        assert!(!game.zone_move_is_prohibited(primary, Zone::Battlefield));
        assert_eq!(game.zone_move_is_prohibited(sibling, Zone::Battlefield), protected_component);
        let entry_rule = CompiledStaticAbility::as_enters_effect_program(
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life(4)]), "this permanent", false, false, None);
        let combined = CardDefinitionBuilder::new(CardId::new(), "Composite entry witness")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::static_ability(StaticAbility::from_model(entry_rule))).build();
        let options = BattlefieldEntryOptions::preserve(false).with_composite_entry(combined, vec![primary, sibling]);
        let mut ctx = ExecutionContext::new_default(actor, A);
        let receipts = crate::effects::zones::move_to_battlefield_batch_with_options(&mut game, &mut ctx,
            vec![(primary, options)]).unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(matches!(receipts[0].outcome, BattlefieldEntryOutcome::Prevented), protected_component);
        crate::effects::zones::finish_battlefield_entry_receipts_with_outputs(
            &mut game, &mut ctx, crate::effect::EffectOutcome::resolved(), receipts).unwrap();
        assert_eq!(game.player(A).unwrap().life, if protected_component { 20 } else { 24 });
        if protected_component {
            assert_eq!(game.object(primary).unwrap().zone, Zone::Command);
            assert_eq!(game.object(sibling).unwrap().zone, Zone::Command);
            assert!(game.is_face_down(sibling));
            assert_eq!(game.battlefield.len(), 1, "neither card becomes a partial composite");
        } else {
            assert!(game.object(primary).is_none()); assert!(game.object(sibling).is_none());
            assert_eq!(game.battlefield.len(), 2, "positive control actually committed the complete composite and its entry program");
        }
    }
}

#[test]
fn rebuilding_current_zone_admission_replaces_provider_rows_instead_of_accumulating_them() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let provider = guard(&mut game);
    let target = card(&mut game, Zone::Graveyard);
    game.update_cant_effects();
    assert_eq!(game.effect_store.cant_effects.cant_move_between_zones.len(), 1);
    game.update_cant_effects();
    assert_eq!(game.effect_store.cant_effects.cant_move_between_zones.len(), 1, "one current provider contributes one current rule after each rebuild");
    game.object_mut(provider).unwrap().abilities_mut().clear();
    game.update_cant_effects();
    assert!(game.effect_store.cant_effects.cant_move_between_zones.is_empty());
    assert!(!game.zone_move_is_prohibited(target, Zone::Exile));
    assert!(game.move_object_by_effect(target, Zone::Exile).is_some());
}
