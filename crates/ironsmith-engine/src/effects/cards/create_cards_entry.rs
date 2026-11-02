//! Originless real-card battlefield creation through native entry admission.
//! This never creates a token or claims that a pre-existing card changed zones.
use crate::cards::CardDefinition;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::events::{CreateCardsEvent, EnterBattlefieldEvent};
use crate::events::processing::EventOutcome;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::object::AttachmentTarget;
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

/// The outer creation producer already resolved its amount/definition and
/// reserved the generated-object budget. This owner consumes one materialization
/// slot per actual provisional real card, prepares every entry, then commits.
pub(crate) fn execute_real_card_battlefield_creation(
    game: &mut GameState, ctx: &mut ExecutionContext,
    definition: &CardDefinition, creation: &CreateCardsEvent,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    if creation.zone != Zone::Battlefield || !creation.selection_finalized {
        return Err(ExecutionError::IncompleteEvidence("battlefield creation requires a finalized creation descriptor".into()));
    }
    let lookback = game.try_trigger_source_lookback_snapshots()?;
    let mut entries = Vec::new();
    for _ in 0..creation.amount {
        game.commit_token_resource_slot()?;
        let object = game.stage_originless_card_entry(definition, creation.clone());
        entries.push((object, None));
    }
    // All proposals/as-enters choices precede every original creation arrival.
    for (object, entry) in &mut entries {
        let physical_admission_forbidden = game.object(*object).is_none_or(|object|
            !object.card_types.iter().any(|kind| kind.is_permanent_type()))
            || game.is_planar_card(*object) || game.is_vanguard_card(*object)
            || game.is_scheme_card(*object) || game.is_conspiracy_card(*object);
        let result = if physical_admission_forbidden {
            crate::events::processing::EtbEventResult { prevented: true, ..Default::default() }
        } else {
            crate::events::processing::process_originless_card_entry_with_dm(
                game, *object, creation.clone(), &mut ctx.decision_maker,
            )?
        };
        if ctx.decision_maker.awaiting_choice() { return Ok(pending()); }
        let Some(mut prepared) = game.prepare_etb_entry_with_controller_and_dm(
            *object, result, Some(creation.player), &mut ctx.decision_maker,
        )? else {
            if ctx.decision_maker.awaiting_choice() { return Ok(pending()); }
            return Err(ExecutionError::IncompleteEvidence("originless entry preparation has no retained verdict".into()));
        };
        let mut single = creation.clone(); single.amount = 1;
        prepared.creation = Some(single);
        if !prepared.result.prevented {
            let prospective = game.prepared_entry_attachment_game_state(*object, &prepared, Some(creation.player))?
                .ok_or_else(|| ExecutionError::IncompleteEvidence("originless entry lost its native prospective state".into()))?;
            // Admission uses the resolved permanent, including copy/type choices.
            if prospective.card_cannot_enter_battlefield(*object) {
                prepared.result.prevented = true;
            } else if prospective.calculated_subtypes(*object).contains(&crate::types::Subtype::Aura) {
                let destinations = game.battlefield.iter().copied().map(AttachmentTarget::Object)
                    .chain(game.players.iter().filter(|player| player.is_in_game()).map(|player| AttachmentTarget::Player(player.id)))
                    .collect::<Vec<_>>();
                prepared.entry_attachment = crate::effects::permanents::choose_attachment_destination(
                    &prospective, ctx, *object, &destinations,
                )?;
                prepared.entry_attachment_specified = true;
                if ctx.decision_maker.awaiting_choice() { return Ok(pending()); }
            }
        }
        *entry = Some(prepared);
    }
    let batch = game.open_simultaneous_action();
    let mut arrivals = Vec::new(); let mut battlefield = Vec::new();
    let mut notifications = Vec::new(); let mut programs = Vec::new(); let mut published = Vec::new();
    for (object, prepared) in entries {
        let prepared = prepared.ok_or_else(|| ExecutionError::IncompleteEvidence("creation lost a prepared original".into()))?;
        let committed = game.commit_prepared_originless_card_entry(
            object, prepared, creation.clone(), ctx.cause.clone(), &mut ctx.decision_maker,
        )?;
        if committed.pending || ctx.decision_maker.awaiting_choice() {
            game.close_simultaneous_action(batch); return Ok(pending());
        }
        programs.extend(committed.programs);
        crate::effects::PublishedEffectOutputs::append_distinct(&mut published, committed.published_outputs);
        match committed.original {
            EventOutcome::Proceed(result) => {
                let actual = game.object(result.new_id).ok_or(ExecutionError::ObjectNotFound(result.new_id))?;
                if game.is_originless_card_entry_pending(result.new_id) {
                    return Err(ExecutionError::IncompleteEvidence("creation reported an unmaterialized arrival".into()));
                }
                arrivals.push(result.new_id);
                if actual.zone == Zone::Battlefield {
                    battlefield.push(result.new_id);
                    game.add_battlefield_put_with_source_link(ctx.source, result.new_id);
                    notifications.push(crate::effects::zones::battlefield_creation_observation(
                        game, result.new_id, creation.clone(), result.enters_tapped,
                        ctx.provenance, lookback.clone(),
                    )?);
                }
            }
            EventOutcome::Prevented | EventOutcome::Replaced | EventOutcome::NotApplicable => {
                if game.is_originless_card_entry_pending(object) { game.remove_object(object); }
            }
        }
    }
    if !crate::effects::zones::order_simultaneous_creation_entry_timestamps(game, ctx, &battlefield)? {
        game.close_simultaneous_action(batch); return Ok(pending());
    }
    game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
    game.close_simultaneous_action(batch);
    // Freeze after the entire actual creation batch, before deferred additions.
    game.freeze_completed_entry_events(notifications.iter_mut())?;
    for event in notifications { game.queue_trigger_event(ctx.provenance, event); }
    game.mark_hidden_cards_publicly_revealed(&arrivals);
    let objects = arrivals.iter().map(|id| {
        let object = game.object(*id).ok_or(ExecutionError::ObjectNotFound(*id))?;
        ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game).map_err(Into::into)
    }).collect::<Result<Vec<_>, ExecutionError>>()?;
    let original = EffectOutcome::count(arrivals.len() as i64).with_execution_facts([
        ExecutionFact::ResultObjects(arrivals.clone()), ExecutionFact::AffectedObjects(arrivals),
        ExecutionFact::ResultObjectMemory(objects.clone()), ExecutionFact::AffectedObjectMemory(objects.clone()),
        ExecutionFact::PlayerAffectedObjectMemory(vec![(creation.player, objects)]),
    ]);
    let mut outputs = CompletedEffectOutputs::aggregate_only(original);
    outputs.retain_published_references(published);
    crate::effects::replacement::complete_replacement_programs_with_original_outputs(
        game, ctx, outputs, move |game, ctx, original| {
            crate::effects::replacement::complete_deferred_replacement_programs_with_bindings(
                game, ctx, original, programs, |_, context, receipt| {
                    let object = crate::events::downcast_event::<EnterBattlefieldEvent>(context.event.inner()).map(|entry| entry.object);
                    let exact = receipt.result_object_memory().unwrap_or_default().iter()
                        .filter(|snapshot| Some(snapshot.object_id) == object).cloned().collect::<Vec<_>>();
                    Ok(crate::effects::replacement::ReplacementProgramBindings {
                        targets: None, object_tags: vec![("it".into(), exact.clone()), ("__it__".into(), exact)],
                    })
                },
            )
        },
    )
}
fn pending() -> CompletedEffectOutputs { CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ability, CardDefinitionBuilder, CardId};
    use crate::ids::PlayerId;
    use crate::object::{AuraAttachmentFilter, ObjectKind};
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};
    use crate::replacement::{ReplacementAction, ReplacementEffect};

    fn definition(name: &str) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build()
    }
    fn descriptor(source: ObjectId, player: PlayerId, amount: u32) -> CreateCardsEvent {
        CreateCardsEvent { operation_label: Some("sample generation".into()), card_name: "supplied real identity".into(), amount,
            player, source, zone: Zone::Battlefield, selection_finalized: true, replacement_source: None }
    }
    fn execute(game: &mut GameState, ctx: &mut ExecutionContext, definition: &CardDefinition, creation: &CreateCardsEvent)
        -> Result<CompletedEffectOutputs, ExecutionError>
    {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game, ctx, pending, |game, ctx| {
                let (_, meter) = game.begin_token_resource_scope();
                let _guard = crate::effects::tokens::resources::TokenInstructionGuard::enter(meter)?;
                game.reserve_token_creation(u128::from(creation.amount))?;
                execute_real_card_battlefield_creation(game, ctx, definition, creation)
            },
        )
    }
    fn setup() -> (GameState, PlayerId, ObjectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&CardDefinitionBuilder::new(CardId::new(), "generation instruction")
            .card_types(vec![CardType::Artifact]).build(), player, Zone::Battlefield);
        (game, player, source)
    }
    #[test]
    fn uncreated_card_has_no_zone_membership_or_legal_movement_and_only_projects_an_entry() {
        use crate::filter::ObjectFilterExt as _;
        let (mut game, player, source) = setup();
        let creation = descriptor(source, player, 1);
        let pending_id = game.stage_originless_card_entry(&definition("supplied real identity"), creation.clone());
        assert!(game.is_originless_card_entry_pending(pending_id));
        assert!(!game.battlefield.contains(&pending_id)); assert!(!game.can_be_sacrificed(pending_id));
        let mut battlefield = ObjectFilter::creature(); battlefield.zone = Some(Zone::Battlefield);
        let context = game.filter_context_for(player, Some(source));
        assert!(!battlefield.matches(game.object(pending_id).unwrap(), &context, &game));
        assert!(game.move_object(pending_id, Zone::Graveyard, crate::events::cause::EventCause::effect()).is_none());
        assert!(matches!(crate::events::processing::process_zone_change_full(&mut game, pending_id,
            Zone::Battlefield, Zone::Graveyard, crate::events::cause::EventCause::effect()).unwrap(),
            crate::events::processing::TraitEventResult::Prevented));
        let event = EnterBattlefieldEvent::originless_creation(pending_id, creation);
        let prospective = event.try_prospective_game_state(&game).unwrap().unwrap();
        assert!(!prospective.is_originless_card_entry_pending(pending_id));
        assert!(battlefield.matches(prospective.object(pending_id).unwrap(), &context, &prospective));
        game.validate_zone_consistency().unwrap();
        game.remove_object(pending_id); assert!(game.take_pending_trigger_events().is_empty());
        game.validate_zone_consistency().unwrap();
    }
    #[test]
    fn real_cards_enter_through_native_replacements_without_any_old_zone_or_movement() {
        let (mut game, player, source) = setup();
        let mut definition = definition("supplied real identity");
        definition.abilities.push(Ability::static_ability(StaticAbility::enters_tapped_ability()));
        definition.ability_labels.push("This creature enters tapped.".into());
        let mut ctx = ExecutionContext::new_default(source, player);
        let out = execute(&mut game, &mut ctx, &definition, &descriptor(source, player, 2)).unwrap();
        let ids = out.outcome.output_objects(); assert_eq!(ids.len(), 2); assert_eq!(out.outcome.as_count(), Some(2));
        for id in ids {
            assert_eq!(game.object(*id).unwrap().kind, ObjectKind::Card);
            assert!(game.is_tapped(*id)); assert!(!game.is_originless_card_entry_pending(*id));
        }
        let events = game.take_pending_trigger_events();
        assert!(!events.iter().any(|event| event.kind() == crate::events::EventKind::ZoneChange));
        let entries = events.iter().filter_map(|event| crate::events::downcast_event::<EnterBattlefieldEvent>(event.inner())).collect::<Vec<_>>();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|entry| entry.from.is_none() && entry.creation.as_ref().is_some_and(|creation| creation.amount == 1 && creation.selection_finalized)));
        assert_eq!(out.outcome.result_object_memory().unwrap().len(), 2);
        assert!(out.outcome.affected_object_memory().unwrap().iter().all(|object| object.kind == ObjectKind::Card));
        let first = ids[0];
        let moved = game.move_object(first, Zone::Graveyard, crate::events::cause::EventCause::effect()).unwrap();
        assert_eq!(game.object(moved).unwrap().kind, ObjectKind::Card, "a real created card persists after leaving the battlefield");
        assert!(game.player(player).unwrap().graveyard.contains(&moved));
        game.validate_zone_consistency().unwrap();
    }
    #[test]
    fn entry_redirection_creates_in_final_zone_without_reapplying_creation_or_moving_a_card() {
        let (mut game, player, source) = setup();
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, player, crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(ObjectFilter::creature()),
            ReplacementAction::ChangeDestination(Zone::Exile),
        ));
        let mut ctx = ExecutionContext::new_default(source, player);
        let out = execute(&mut game, &mut ctx, &definition("supplied real identity"), &descriptor(source, player, 1)).unwrap();
        let [created] = out.outcome.output_objects() else { panic!("one redirected creation"); };
        assert_eq!(game.object(*created).unwrap().zone, Zone::Exile);
        assert_eq!(game.object(*created).unwrap().kind, ObjectKind::Card);
        assert_eq!(out.outcome.result_object_memory().unwrap()[0].object_id, *created);
        assert!(!game.take_pending_trigger_events().iter().any(|event| matches!(event.kind(), crate::events::EventKind::ZoneChange | crate::events::EventKind::EnterBattlefield)));
        game.validate_zone_consistency().unwrap();
    }
    #[test]
    fn prevented_creation_has_no_arrival_or_unindexed_object() {
        let (mut game, player, source) = setup();
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, player, crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(ObjectFilter::creature()), ReplacementAction::Prevent,
        ));
        let mut ctx = ExecutionContext::new_default(source, player);
        let out = execute(&mut game, &mut ctx, &definition("supplied real identity"), &descriptor(source, player, 1)).unwrap();
        assert_eq!(out.outcome.as_count(), Some(0)); assert!(out.outcome.output_objects().is_empty());
        assert_eq!(game.battlefield, vec![source]); assert_eq!(game.objects_map().len(), 1);
        assert!(game.take_pending_trigger_events().is_empty()); game.validate_zone_consistency().unwrap();
    }
    #[test]
    fn impossible_aura_is_not_created_and_does_not_move_to_a_graveyard() {
        let (mut game, player, source) = setup();
        let enchant = AuraAttachmentFilter::Object(ObjectFilter::creature());
        let aura = CardDefinitionBuilder::new(CardId::new(), "supplied unattached Aura")
            .card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura]).enchants(enchant.clone())
            .with_ability(Ability::static_ability(StaticAbility::enchant(enchant))).build();
        let mut ctx = ExecutionContext::new_default(source, player);
        let out = execute(&mut game, &mut ctx, &aura, &descriptor(source, player, 1)).unwrap();
        assert_eq!(out.outcome.as_count(), Some(0)); assert_eq!(game.objects_map().len(), 1);
        assert!(game.player(player).unwrap().graveyard.is_empty()); assert!(game.take_pending_trigger_events().is_empty());
        game.validate_zone_consistency().unwrap();
    }
    #[test]
    fn suspended_as_enters_choice_rolls_back_all_real_cards_and_entry_choices() {
        struct PauseSecond { calls: usize, pending: bool }
        impl crate::decision::DecisionMaker for PauseSecond {
            fn decide_colors(&mut self, _: &GameState, _: &crate::decisions::context::ColorsContext) -> Vec<crate::color::Color> {
                self.calls += 1; self.pending = self.calls == 2;
                if self.pending { Vec::new() } else { vec![crate::color::Color::Blue] }
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        let (mut game, player, source) = setup();
        let mut definition = definition("supplied real identity");
        definition.abilities.push(Ability::static_ability(StaticAbility::choose_color_as_enters(None, "As this creature enters, choose a color.".into())));
        definition.ability_labels.push("As this creature enters, choose a color.".into());
        let mut decisions = PauseSecond { calls: 0, pending: false };
        let mut ctx = ExecutionContext::new(source, player, &mut decisions);
        let out = execute(&mut game, &mut ctx, &definition, &descriptor(source, player, 2)).unwrap();
        assert_eq!(out.outcome.as_count(), Some(0)); assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(game.battlefield, vec![source]); assert_eq!(game.objects_map().len(), 1);
        assert!(game.take_pending_trigger_events().is_empty()); game.validate_zone_consistency().unwrap();
        drop(ctx); assert_eq!(decisions.calls, 2);
    }
}
