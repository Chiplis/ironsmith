//! Authored source regressions; intentionally unrun during the campaign gate.
use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::static_abilities::StaticAbility;

fn creature(game: &mut crate::game_state::GameState, zone: Zone) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Snapshot evidence creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    game.create_object_from_card(&card, PlayerId::from_index(0), zone)
}

fn add_numeric_cda(game: &mut crate::game_state::GameState, object: ObjectId, pair: Option<ironsmith_core::LinkedExilePair>) {
    let value = Value::SourceChosenNumber { if_unset: Some(0), pair };
    game.object_mut(object).unwrap().abilities_mut().push(Ability::static_ability(
        StaticAbility::characteristic_defining_pt(value.clone(), value),
    ));
    game.mark_continuous_state_dirty();
}

fn numeric_error(error: &ExecutionError) -> bool {
    matches!(error, ExecutionError::ContinuousDiscovery(
        crate::static_ability_processor::StaticEffectDiscoveryError::NumericChoiceEvidence { .. }
    ))
}

#[test]
fn checked_snapshot_rejects_incomplete_main_and_attachment_characteristics_without_a_meter() {
    for attached in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let host = creature(&mut game, Zone::Battlefield);
        let invalid = if attached {
            let child = creature(&mut game, Zone::Battlefield);
            game.object_mut(host).unwrap().attachments.push(child);
            child
        } else { host };
        // Missing immutable pair is unavailable evidence, not a never-chosen number.
        add_numeric_cda(&mut game, invalid, None);
        let snapshot = ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(host).unwrap(), &game);
        assert!(numeric_error(&snapshot.unwrap_err()));
        let cached = game.try_cached_object_snapshot_with_calculated_characteristics(game.object(host).unwrap());
        assert!(numeric_error(&cached.unwrap_err()));
        assert_eq!(game.object(host).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn checked_snapshots_preserve_known_never_chosen_zero_and_completed_nonzero_choices() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let host = creature(&mut game, Zone::Battlefield);
    let pair = ironsmith_core::LinkedExilePair {
        definition: ironsmith_core::LinkedExileDefinition([93; 32]), pair: 0,
    };
    add_numeric_cda(&mut game, host, Some(pair));
    let owner = crate::linked_exile::LinkedExileOwner {
        host, pair, acquisition: crate::linked_exile::LinkedExileAcquisition::Printed,
    };
    for chosen in [None, Some(0), Some(7)] {
        if let Some(chosen) = chosen { game.set_number_for_acquisition(owner.clone(), chosen).unwrap(); }
        let snapshot = ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(host).unwrap(), &game).unwrap();
        assert_eq!((snapshot.power, snapshot.toughness), (Some(chosen.unwrap_or(0) as i32), Some(chosen.unwrap_or(0) as i32)));
        assert_eq!(snapshot.numeric_choice_memory.as_ref().unwrap().get(&owner).map(|record| record.number), chosen);
    }
}

#[derive(Debug, Clone, Copy)]
enum CapturePath { Copy, Reveal, Destroy, ZoneProposal, Movement, Targets, Cast, Entry, Sba }

#[derive(Debug, Clone)]
struct MutateThenCapture { target: ObjectId, path: CapturePath }
impl EffectExecutor for MutateThenCapture {
    fn execute(&self, game: &mut crate::game_state::GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError>
    {
        // The original state was complete at admission. Become incomplete after
        // a real prefix mutation, so the production owner must restore it.
        game.player_mut(ctx.controller).unwrap().life += 5;
        ctx.tag_object("prefix", ObjectSnapshot::from_object(game.object(self.target).unwrap(), game));
        add_numeric_cda(game, self.target, None);
        match self.path {
            CapturePath::Copy => crate::effects::CreateTokenCopyEffect::one(
                crate::target::ChooseSpec::SpecificObject(self.target),
            ).execute(game, ctx),
            CapturePath::Reveal => crate::effects::LookAtHandEffect::reveal(
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::You),
            ).execute(game, ctx),
            CapturePath::Destroy => crate::effects::DestroyEffect::with_spec(
                crate::target::ChooseSpec::SpecificObject(self.target),
            ).execute(game, ctx),
            CapturePath::ZoneProposal => {
                crate::events::processing::process_zone_change_full(game, self.target, Zone::Battlefield, Zone::Graveyard, ctx.cause.clone())?;
                Ok(EffectOutcome::resolved())
            }
            CapturePath::Movement => {
                // Legacy Option mutation is permitted to return absence only
                // while the exact error reaches the enclosing execution root.
                assert!(game.move_object_by_effect(self.target, Zone::Graveyard).is_none());
                Ok(EffectOutcome::resolved())
            }
            CapturePath::Targets => {
                ctx.targets.push(crate::effects::ResolvedTarget::Object(self.target));
                ctx.try_snapshot_targets(game)?;
                Ok(EffectOutcome::resolved())
            }
            CapturePath::Cast => {
                crate::events::SpellCastEvent::try_from_completed_cast(self.target, ctx.controller, Zone::Hand, game)?;
                Ok(EffectOutcome::resolved())
            }
            CapturePath::Entry => {
                crate::events::processing::process_etb_with_event_and_dm(game, self.target, Zone::Hand, ctx.decision_maker)?;
                Ok(EffectOutcome::resolved())
            }
            CapturePath::Sba => {
                crate::rules::state_based::apply_state_based_actions_with(game, ctx.decision_maker)?;
                Ok(EffectOutcome::resolved())
            }
        }
    }
}

#[test]
fn incomplete_snapshot_evidence_reaches_production_owners_and_rolls_back_the_whole_effect() {
    for path in [CapturePath::Copy, CapturePath::Reveal, CapturePath::Destroy,
        CapturePath::ZoneProposal, CapturePath::Movement, CapturePath::Targets,
        CapturePath::Cast, CapturePath::Entry, CapturePath::Sba]
    {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let zone = match path { CapturePath::Reveal | CapturePath::Entry => Zone::Hand, CapturePath::Cast => Zone::Stack, _ => Zone::Battlefield };
        let target = creature(&mut game, zone);
        let mut ctx = ExecutionContext::new_default(target, PlayerId::from_index(0));
        let before = game.clone();
        let error = crate::effects::execute_effect(&mut game,
            &Effect::new(MutateThenCapture { target, path }), &mut ctx).unwrap_err();
        assert!(numeric_error(&error), "{path:?}: {error:?}");
        assert_eq!(game.player(ctx.controller).unwrap().life, before.player(ctx.controller).unwrap().life, "{path:?}");
        assert_eq!(game.object(target).unwrap().zone, zone, "{path:?}");
        assert_eq!(game.battlefield, before.battlefield, "{path:?}");
        assert_eq!(game.objects_map().len(), before.objects_map().len(), "{path:?}");
        assert!(ctx.get_tagged("prefix").is_none(), "{path:?}");
        assert!(ctx.target_snapshots.is_empty(), "{path:?}");
        let snapshot = ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(target).unwrap(), &game).unwrap();
        assert_eq!((snapshot.power, snapshot.toughness), (Some(2), Some(2)), "failed capture must not poison retry: {path:?}");
    }
}

#[test]
fn known_frame_capture_rejects_numeric_error_before_copying_provisional_values() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let host = creature(&mut game, Zone::Battlefield);
    let mut chars = game.try_current_characteristics(host).unwrap().unwrap();
    chars.numeric_choice_error = Some("missing native numeric acquisition history");
    chars.power = Some(0);
    chars.toughness = Some(0);
    let result = ObjectSnapshot::try_from_object_with_known_characteristics(game.object(host).unwrap(), &game, Some(&chars));
    assert!(numeric_error(&result.unwrap_err()));
    let missing = ObjectSnapshot::try_from_object_with_known_characteristics(game.object(host).unwrap(), &game, None);
    assert!(matches!(missing, Err(ExecutionError::ContinuousDiscovery(
        crate::static_ability_processor::StaticEffectDiscoveryError::UnavailableCharacteristics { object }
    )) if object == host));
}


#[test]
fn numeric_reselection_invalidates_cached_memory_without_changing_layer_descriptors() {
    for leaves_game in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let host = creature(&mut game, Zone::Battlefield);
        let pair = ironsmith_core::LinkedExilePair {
            definition: ironsmith_core::LinkedExileDefinition([94; 32]), pair: 0,
        };
        let owner = crate::linked_exile::LinkedExileOwner {
            host, pair, acquisition: crate::linked_exile::LinkedExileAcquisition::Printed,
        };
        game.set_number_for_acquisition(owner.clone(), 2).unwrap();
        let admitted = game.try_cached_object_snapshot_with_calculated_characteristics(game.object(host).unwrap()).unwrap();
        assert_eq!(admitted.numeric_choice_memory.as_ref().unwrap()[&owner].number, 2);
        let effects_revision = game.effect_store.continuous_effects.revision();
        game.set_number_for_acquisition(owner.clone(), 5).unwrap();
        let updated = game.try_cached_object_snapshot_with_calculated_characteristics(game.object(host).unwrap()).unwrap();
        assert_eq!(game.effect_store.continuous_effects.revision(), effects_revision,
            "the test changes choice evidence without adding or changing continuous effects");
        assert_eq!(updated.numeric_choice_memory.as_ref().unwrap()[&owner].number, 5);
        assert_eq!(updated.numeric_choice_memory.as_ref().unwrap()[&owner].public_group, 0);
        if leaves_game {
            assert!(game.leave_game(PlayerId::from_index(0)).unwrap());
        } else {
            let later = game.move_object_by_effect(host, Zone::Graveyard).unwrap();
            game.set_number_for_acquisition(crate::linked_exile::LinkedExileOwner { host: later, ..owner.clone() }, 7).unwrap();
        }
        assert_eq!(game.number_for_acquisition(&owner, Some(&admitted)).unwrap(), Some(5),
            "cached true departure retains the latest choice, before the admitted snapshot and without following the new incarnation");
    }
}

#[test]
fn checked_trigger_discovery_without_a_meter_returns_numeric_failure_instead_of_empty_success() {
    let mut game=crate::tests::test_helpers::setup_two_player_game();
    let host=creature(&mut game,Zone::Battlefield);
    game.object_mut(host).unwrap().abilities_mut().push(Ability::triggered(
        crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),vec![Effect::gain_life(1)]));
    add_numeric_cda(&mut game,host,None);
    let event=crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfUpkeepEvent::new(PlayerId::from_index(0)),Default::default());
    assert!(game.token_resource_failure().is_none());
    let error=crate::triggers::check_triggers_checked(&game,&event).unwrap_err();
    assert!(numeric_error(&error));
    assert!(game.token_resource_failure().is_none(),"read-only discovery owns a private failure scope");

    let mut game=crate::tests::test_helpers::setup_two_player_game();
    let host=creature(&mut game,Zone::Battlefield);
    let mut filter=crate::target::ObjectFilter::default();
    filter.mana_value=Some(crate::filter::Comparison::EqualExpr(Box::new(Value::SourceChosenNumber{if_unset:Some(0),pair:None})));
    game.object_mut(host).unwrap().abilities_mut().push(Ability::triggered(
        crate::triggers::Trigger::spell_cast(Some(filter),crate::target::PlayerFilter::Any),vec![Effect::gain_life(1)]));
    let spell=creature(&mut game,Zone::Stack);
    let event=crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::SpellCastEvent::try_from_completed_cast(spell,PlayerId::from_index(0),Zone::Hand,&game).unwrap(),Default::default());
    assert!(matches!(crate::triggers::check_triggers_checked(&game,&event),Err(ExecutionError::IncompleteEvidence(_))),
        "a missing numeric matcher owner is an error even when characteristic capture itself is complete");
    assert!(game.token_resource_failure().is_none());
}
