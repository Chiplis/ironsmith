//! Keyword action event emission effect.
//!
//! Some rules text triggers on a keyword action (e.g., "when you cycle this card").
//! This effect provides a generic way to emit a KeywordActionEvent as part of an
//! effect/cost pipeline so triggers can observe it.

use std::collections::HashMap;
use crate::ids::{ObjectId, PlayerId};

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::filter::ObjectFilterExt as _;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::EmitKeywordActionEffect;

use super::keyword_programs::forage_payments;

/// Record that the resolving spell or ability made one or more players guess
/// or group cards or permanents into piles this turn (CR 700.3). Several pile
/// or guess instructions of one resolution count once.
pub(crate) fn record_pile_or_guess_instruction(game: &mut GameState, ctx: &ExecutionContext) {
    game.turn_store
        .turn_history
        .pile_or_guess_instructions_this_turn
        .insert((
            ctx.source,
            ctx.resolution_object_id_floor,
            ctx.trigger_identity,
            ctx.ability_index,
        ));
}

fn snapshot_from_memory(_game: &GameState, snapshot: &ObjectSnapshot) -> ObjectSnapshot {
    snapshot.clone()
}

fn object_tags_from_config(
    effect: &EmitKeywordActionEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<HashMap<TagKey, Vec<ObjectSnapshot>>, ExecutionError> {
    let mut tags: HashMap<TagKey, Vec<ObjectSnapshot>> = HashMap::new();
    for config in &effect.object_tags {
        // An instruction that never ran named no objects.
        let Some(outcome) = ctx.get_outcome(config.effect_id) else {
            continue;
        };
        let memories = if config.use_affected_memory {
            outcome.affected_object_memory()
        } else {
            outcome.chosen_object_memory()
        };
        let Some(memories) = memories else {
            continue;
        };
        let snapshots = memories
            .iter()
            .map(|memory| snapshot_from_memory(game, memory))
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            tags.entry(config.tag.clone())
                .or_default()
                .extend(snapshots);
        }
    }
    Ok(tags)
}

impl EffectExecutor for EmitKeywordActionEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // Pile separation is turn-history instrumentation, not a
                // trigger-visible keyword action.
                if self.action == KeywordActionKind::GroupIntoPiles {
                    record_pile_or_guess_instruction(game, ctx);
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if matches!(
                    self.action,
                    KeywordActionKind::Forage
                        | KeywordActionKind::AssembleContraption
                        | KeywordActionKind::Planeswalk
                        | KeywordActionKind::SetSchemeInMotion
                        | KeywordActionKind::AbandonScheme
                ) {
                    return super::keyword_programs::execute_keyword_program_with_outputs(
                        game,
                        ctx,
                        self.action,
                        self.amount,
                    );
                }
                if self.action == KeywordActionKind::Harness
                    && !super::keyword_programs::commit_harness(game, ctx.source)
                {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                KeywordActionCompletion(self.clone()).execute_child_with_outputs(game, ctx)
            },
        )
    }

    fn cost_description(&self) -> Option<String> {
        if self.action == KeywordActionKind::Forage {
            return Some("Forage".into());
        }
        // Internal scaffolding effect used to emit trigger-visible events from costs.
        // This should not show up as part of the printed/visible cost.
        Some(String::new())
    }
}

/// Pure completion publication, separated from executable keyword programs.
/// The compatibility façade retains the existing compiler vocabulary.
#[derive(Debug, Clone)]
struct KeywordActionCompletion(EmitKeywordActionEffect);
impl EffectExecutor for KeywordActionCompletion {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let config = &self.0;
        let object_tags = object_tags_from_config(config, game, ctx)?;
        if config.action == KeywordActionKind::Exploit {
            // CR 702.110b: "when this exploits a creature" looks back in time.
            // A creature that exploited itself is gone by now, so carry its
            // last-known information for the source filter and its own trigger.
            let on_battlefield = game
                .object(ctx.source)
                .filter(|object| object.zone == crate::zone::Zone::Battlefield);
            let source_snapshot = on_battlefield
                .map(|object| game.cached_object_snapshot_with_calculated_characteristics(object))
                .or_else(|| ctx.source_snapshot.clone());
            // Only a source sacrificed by this exploit ability gets lookback.
            // An ETB-time snapshot must not revive an ability whose source left
            // before this sacrifice (CR 603.10a, 702.110b).
            let exploited_itself = object_tags
                .get(crate::tag::EXPLOITED_TAG)
                .is_some_and(|objects| objects.iter().any(|object| object.object_id == ctx.source));
            let lookback = if on_battlefield.is_none() && exploited_itself {
                source_snapshot.iter().cloned().collect()
            } else {
                Vec::new()
            };
            let event = TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(config.action, ctx.controller, ctx.source, config.amount)
                    .with_object_tags(object_tags)
                    .with_snapshot(source_snapshot),
                ctx.provenance,
            )
            .with_lookback_source_snapshots(lookback);
            return PublishKeywordActionCompletion(event).execute_child_with_outputs(game, ctx);
        }
        // CR 702.29: a cycling ability's announced X (paid as part of the
        // cycling cost) is the X of its "when you cycle this card" trigger.
        let x_value = (config.action == KeywordActionKind::Cycle)
            .then_some(ctx.x_value)
            .flatten();
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(config.action, ctx.controller, ctx.source, config.amount)
                .with_object_tags(object_tags)
                .with_x_value(x_value),
            ctx.provenance,
        );
        let event = if config.action == KeywordActionKind::Cycle {
            event.with_lookback_source_snapshots(ctx.source_snapshot.iter()
                .filter(|snapshot| snapshot.zone == crate::zone::Zone::Hand)
                .cloned().collect())
        } else { event };
        PublishKeywordActionCompletion(event).execute_child_with_outputs(game, ctx)
    }
}

/// Publish an already authored completion without executing the action again or
/// reconstructing its subjects from the instruction source. The caller freezes
/// action-specific observations at the semantic completion boundary.
#[derive(Debug, Clone)]
pub(super) struct PublishKeywordActionCompletion(pub(super) TriggerEvent);

impl EffectExecutor for PublishKeywordActionCompletion {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        observe_keyword_action_completion_with_outputs(game, self.0.clone())
    }
}

/// Shared completion observation owner for recorded effects and game-loop
/// actions. Root actions retain their own queue/transaction boundary; neither
/// adapter reconstructs occurrence identity, lifecycle state or grouping.
pub(crate) fn observe_keyword_action_completion(
    game: &mut GameState,
    event: TriggerEvent,
) -> Result<EffectOutcome, ExecutionError> {
    observe_keyword_action_completion_with_outputs(game, event).map(|outputs| outputs.outcome)
}

pub(crate) fn observe_keyword_action_completion_with_outputs(
    game: &mut GameState,
    event: TriggerEvent,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if event.downcast::<KeywordActionEvent>().is_none() {
        return Err(ExecutionError::InternalError(
            "keyword completion requires a keyword action observation".into(),
        ));
    }
    let parent = event.provenance();
    let event = crate::effects::observe_action_completion(game, event, Some(parent))?;
    Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
        EffectOutcome::resolved().with_event(event),
    ))
}

pub(crate) fn complete_keyword_action(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: KeywordActionEvent,
) -> Result<EffectOutcome, ExecutionError> {
    publish_keyword_action_completion(
        game,
        ctx,
        TriggerEvent::new_with_provenance(event, ctx.provenance),
    )
}

/// Publish an authored observation with its original parent, snapshots and
/// simultaneous group. Cost and resolution adapters need those identities even
/// when their publication context no longer names the original instruction.
pub(crate) fn publish_keyword_action_completion(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: TriggerEvent,
) -> Result<EffectOutcome, ExecutionError> {
    publish_keyword_action_completion_receipt(game, ctx, event)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// The recorded completion leaf retains its packet through scalar adapters.
pub(crate) fn publish_keyword_action_completion_receipt(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: TriggerEvent,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if event.downcast::<KeywordActionEvent>().is_none() {
        return Err(ExecutionError::InternalError(
            "keyword completion requires a keyword action observation".into(),
        ));
    }
    PublishKeywordActionCompletion(event).execute_child_with_outputs(game, ctx)
}

/// Keep the action body's result independent of its completion notification.
/// Both remain recorded children; publication must not add to a count or replace
/// a chosen/affected-object result supplied by the action's owner.
pub(crate) fn complete_keyword_action_with_result(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: EffectOutcome,
    event: KeywordActionEvent,
) -> Result<EffectOutcome, ExecutionError> {
    publish_keyword_action_completion_with_result(
        game,
        ctx,
        original,
        TriggerEvent::new_with_provenance(event, ctx.provenance),
    )
}

/// Retain the body owner's outputs while publishing its one completion.
pub(crate) fn complete_keyword_action_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: crate::effects::CompletedEffectOutputs,
    event: KeywordActionEvent,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    publish_keyword_action_completion_with_outputs(
        game,
        ctx,
        original,
        TriggerEvent::new_with_provenance(event, ctx.provenance),
    )
}

/// Preserve an action's primary result when its completion carries captured
/// provenance, snapshots or a group whose original scope has already closed.
pub(crate) fn publish_keyword_action_completion_with_result(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: EffectOutcome,
    event: TriggerEvent,
) -> Result<EffectOutcome, ExecutionError> {
    publish_keyword_action_completion_with_outputs(
        game,
        ctx,
        crate::effects::CompletedEffectOutputs::aggregate_only(original),
        event,
    )
    .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Aggregate and retained callers share publication and primary projection.
pub(crate) fn publish_keyword_action_completion_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: crate::effects::CompletedEffectOutputs,
    event: TriggerEvent,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let completion = publish_keyword_action_completion_receipt(game, ctx, event)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    Ok(original.append_batch_completion_outputs(completion))
}

impl CostExecutableEffect for EmitKeywordActionEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(
            self,
            game,
            source,
            controller,
            crate::costs::PaymentReason::Other,
        )
    }
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::effects::CostValidationError> {
        if self.action == KeywordActionKind::Forage
            && super::payable_choice::payable_action_indices(
                game,
                source,
                controller,
                &forage_payments(reason == crate::costs::PaymentReason::CastSpell),
            ).is_empty()
        {
            return Err(crate::effects::CostValidationError::NotEnoughCards);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::ColorSet;
    use crate::effect::EffectId;
    use crate::ids::{ObjectId, PlayerId, StableId};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn forwards_affected_object_memory_as_event_object_tag() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(10);
        let sacrificed = ObjectId::from_raw(20);
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect_id = EffectId(7);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::resolved().with_affected_object_memory(vec![{
                let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                    sacrificed,
                    StableId::from(sacrificed),
                    bob,
                    bob,
                    Zone::Battlefield,
                );
                snapshot.name = "Sacrificed Creature".to_string();
                snapshot.power = Some(3);
                snapshot.toughness = Some(5);
                snapshot.linked_face_mana_value = Some((2) as u32);
                snapshot.card_types = vec![CardType::Creature];
                snapshot.colors = ColorSet::default();
                snapshot.subtypes = Vec::new();
                snapshot.is_token = true;
                snapshot
            }]),
        );

        let effect = EmitKeywordActionEffect::new(crate::events::KeywordActionKind::Exploit, 1)
            .with_affected_object_memory_tag(effect_id, crate::tag::EXPLOITED_TAG);
        let outcome = effect.execute(&mut game, &mut ctx).expect("event emitted");
        let event = outcome.events.first().expect("keyword action event");
        let keyword = event
            .downcast::<KeywordActionEvent>()
            .expect("keyword action payload");
        let snapshots = keyword
            .object_tags
            .get(crate::tag::EXPLOITED_TAG)
            .expect("exploited tag");

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].object_id, sacrificed);
        assert_eq!(snapshots[0].controller, bob);
        assert_eq!(snapshots[0].zone, Zone::Battlefield);
        assert_eq!(snapshots[0].toughness, Some(5));
    }
}


pub use ironsmith_core::PerformNamedActionEffect;
fn retained_named_action_objects(spec:&crate::target::ChooseSpec,game:&GameState,ctx:&ExecutionContext)->Result<Option<Vec<ObjectSnapshot>>, ExecutionError> {
    match spec {
        crate::target::ChooseSpec::Tagged(tag)=>ctx.tagged_objects.get(tag).cloned().map(Some)
            .ok_or_else(|| ExecutionError::IncompleteEvidence("named action has no captured object collection".into())),
        crate::target::ChooseSpec::SurfaceHinted {spec,..}=>retained_named_action_objects(spec,game,ctx),
        crate::target::ChooseSpec::Object(filter) | crate::target::ChooseSpec::All(filter)
            if filter.any_of.is_empty() => {
                // A qualified retained reference (for example, enchanted
                // creature) filters the prelude's immutable snapshot packet.
                // Re-querying its live ID after the authored sacrifice would
                // lose the host or observe replacement-added type changes.
                let mut anchors = filter.tagged_constraints.iter().filter(|constraint|
                    constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject);
                let Some(anchor) = anchors.next() else { return Ok(None); };
                if anchors.next().is_some() { return Ok(None); }
                let snapshots = ctx.get_tagged_all(&anchor.tag).ok_or_else(||
                    ExecutionError::IncompleteEvidence("named action has no retained qualified reference".into()))?;
                let filter_ctx = ctx.filter_context(game);
                Ok(Some(snapshots.iter().filter(|snapshot|
                    filter.matches_snapshot(snapshot, &filter_ctx, game)).cloned().collect()))
            }
        _=>Ok(None),
    }
}

impl EffectExecutor for PerformNamedActionEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        // Only an exact predecessor receipt can publish completion in a
        // payment program. A bare label is not an independently payable cost.
        self.prior_objects.is_some().then_some(self)
    }
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        super::execute_transaction(game, ctx, || EffectOutcome::count(0), |game, ctx| {
            let retained=self.objects.as_ref().map(|objects|retained_named_action_objects(objects,game,ctx)).transpose()?.flatten();
            let ids = match &self.objects {
                Some(objects) if retained.is_none() => crate::effects::helpers::resolve_objects_for_effect(game, ctx, objects)?,
                Some(_) => Vec::new(),
                None => Vec::new(),
            };
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let objects = if let Some((effect_id, action)) = self.prior_objects {
                let receipt = ctx.get_outcome(effect_id).ok_or_else(||
                    ExecutionError::IncompleteEvidence("named action has no original predecessor receipt".into()))?;
                let objects: Vec<_> = receipt.action_objects(action,None).into_iter().map(|snapshot| {
                    // A tap result identifies the changed object before the
                    // transition; participant events observe its completed state.
                    if action==crate::effect::PriorEffectAction::Tapped {
                        receipt.events.iter().filter_map(|event|event.downcast::<crate::events::PermanentTappedEvent>())
                            .find(|event|event.permanent==snapshot.object_id).and_then(|event|event.snapshot.clone()).unwrap_or_else(||snapshot.clone())
                    }else{snapshot.clone()}
                }).collect();
                if objects.is_empty() {
                    let original = receipt.instruction_result();
                    if original.status != crate::effect::OutcomeStatus::Succeeded { return Ok(EffectOutcome::count(0)); }
                    if original.value.as_count() != Some(0) && !original.value.objects().is_some_and(|objects| objects.is_empty()) {
                        return Err(ExecutionError::IncompleteEvidence("named action predecessor has no authoritative empty object result".into()));
                    }
                }
                objects
            } else if let Some(retained)=retained {retained}
            else { ids.into_iter().map(|id| ObjectSnapshot::from_object_id(game, id)
                .ok_or_else(|| ExecutionError::IncompleteEvidence("named action lost its selected object snapshot".into())))
                .collect::<Result<Vec<_>, _>>()? };
            // A known empty set is a completed amount-zero operation. Its
            // source history still advances once; observer bounds are explicit.
            let amount = if self.objects.is_some() || self.prior_objects.is_some() {u32::try_from(objects.len()).map_err(|_| ExecutionError::Impossible("named action object count exceeds its numeric vocabulary".into()))?} else {1};
            let participant_sources = self.prior_objects.and_then(|(id, _)| ctx.get_outcome(id)).and_then(|outcome| {
                let mut found = false; let mut sources = Vec::new(); let mut seen = std::collections::HashSet::new();
                for fact in outcome.instruction_result().execution_facts() {
                    let crate::effect::ExecutionFact::OriginalZoneMoveObservers {objects:originals,sources:observers}=fact else{continue};
                    if !originals.iter().any(|id|objects.iter().any(|object|object.object_id==*id)){continue;}
                    found=true;
                    for snapshot in observers {if seen.insert(snapshot.object_id){sources.push(snapshot.clone());}}
                }
                if !found { for event in &outcome.events {
                    let Some(change) = event.downcast::<crate::events::ZoneChangeEvent>() else { continue; };
                    if !change.objects.iter().any(|id| objects.iter().any(|object| object.object_id == *id))
                        && !change.snapshots().iter().any(|snapshot| objects.iter().any(|object| object.object_id == snapshot.object_id)) { continue; }
                    found = true;
                    for snapshot in event.lookback_source_snapshots() {
                        if seen.insert(snapshot.object_id) { sources.push(snapshot.clone()); }
                    }
                }
                }
                found.then_some(sources)
            });
            if std::env::var_os("IRONSMITH_TRACE_NAMED_PARTICIPANTS").is_some() {
                eprintln!("named-participant producer label={} prior={:?} objects={:?} observers={:?}", self.label, self.prior_objects, objects.iter().take(8).map(|s|(s.object_id,s.zone,s.abilities.len())).collect::<Vec<_>>(), participant_sources.as_ref().map(|sources|sources.iter().take(8).map(|s|(s.object_id,s.zone,s.abilities.len())).collect::<Vec<_>>()));
                if let Some((id,_))=self.prior_objects && let Some(receipt)=ctx.get_outcome(id) {eprintln!("named-participant receipt status={:?} facts={:?}", receipt.instruction_result().status, receipt.instruction_result().execution_facts().iter().take(8).map(std::mem::discriminant).collect::<Vec<_>>());}
            }
            // A completed predecessor may cite its captured related source
            // even after that source leaves. This records an actual relation;
            // an ordinary imperative performer still requires its current BF
            // incarnation and never receives this exception.
            let retained_performers=if self.prior_objects.is_some() {
                self.performer.as_ref().map(|spec|retained_named_action_objects(spec,game,ctx)).transpose()?.flatten()
            }else{None};
            let mut performer_memory=std::collections::HashMap::new();
            let mut staged=0u32;
            let performers=if let Some(retained)=retained_performers {
                let receipt=self.prior_objects.and_then(|(id,_)|ctx.get_outcome(id));
                let mut ids=Vec::new();
                for captured in retained.into_iter().filter(|snapshot|snapshot.zone==crate::zone::Zone::Battlefield) {
                    if performer_memory.contains_key(&captured.object_id){continue;}
                    let original=receipt.and_then(|receipt|receipt.events.iter().flat_map(|event|event.lookback_source_snapshots()).find(|snapshot|snapshot.object_id==captured.object_id)).cloned().unwrap_or(captured);
                    ids.push(original.object_id);performer_memory.insert(original.object_id,original);
                }
                ids
            }else if let Some(spec)=&self.performer {
                let ids=match crate::effects::helpers::resolve_objects_for_effect(game,ctx,spec) {
                    Ok(ids)=>ids,Err(ExecutionError::InvalidTarget)=>return Ok(EffectOutcome::target_invalid()),Err(error)=>return Err(error),
                };
                if ctx.decision_maker.awaiting_choice(){return Ok(EffectOutcome::count(0));}
                let mut seen=std::collections::HashSet::new();let mut performers=Vec::new();
                for id in ids.into_iter().filter(|id|seen.insert(*id)) {
                    if ctx.replacement.entry_counter_source==Some(id) {
                        let payment=game.object(id).map(|object|object.mana_spent_to_cast.clone()).unwrap_or_default();
                        ctx.replacement.entry_named_actions.push((self.label.clone(),payment));
                        staged=staged.checked_add(1).ok_or_else(||ExecutionError::Impossible("staged action count overflow".into()))?;
                    }else if !game.is_phased_out(id) && game.object(id).is_some_and(|object|object.zone==crate::zone::Zone::Battlefield) {performers.push(id);}
                }
                performers
            }else{vec![ctx.source]};
            if performers.is_empty() {
                let count=i32::try_from(staged).map_err(|_|ExecutionError::Impossible("staged action count exceeds outcome vocabulary".into()))?;
                return Ok(EffectOutcome::count(count));
            }
            // Complete every selected actor before event-time observers inspect
            // the instruction. Each actor owns one action, regardless of the
            // affected-object packet's cardinality.
            for &performer in &performers {
                if performer_memory.get(&performer).is_none_or(|snapshot|game.find_object_by_stable_id(snapshot.stable_id)==Some(performer)) {game.record_named_action_with_objects(performer,&self.label,&objects)?;}
            }
            let mut completed_performers = Vec::new();
            for &performer in &performers {
                let snapshot=performer_memory.get(&performer).cloned().or_else(||ObjectSnapshot::from_object_id(game,performer))
                    .or_else(||(performer==ctx.source).then(||ctx.source_snapshot.clone()).flatten())
                    .ok_or_else(||ExecutionError::IncompleteEvidence("named action lacks its performer snapshot".into()))?;
                let player=if self.performer.is_some(){snapshot.controller}else{ctx.controller};
                completed_performers.push((performer, snapshot, player));
            }
            // Freeze actual receipt controllers before player-history-dependent
            // continuous effects can change a later performer's controller.
            // A retained departed performer still completed a player operation;
            // the preceding incarnation guard only limits object-local state.
            game.record_player_named_actions(completed_performers.iter().map(|(_, _, player)| (*player, self.label.as_str())))?;
            let original_lookback=self.prior_objects.and_then(|(id,_)|ctx.get_outcome(id)).map(|receipt| {
                let mut seen=std::collections::HashSet::new();
                receipt.events.iter().filter(|event|event.inner().snapshots().iter().any(|snapshot|objects.iter().any(|object|object.object_id==snapshot.object_id)))
                    .flat_map(|event|event.lookback_source_snapshots()).filter(|snapshot|seen.insert(snapshot.object_id)).cloned().collect::<Vec<_>>()
            }).filter(|sources|!sources.is_empty());
            let lookback=if let Some(sources)=original_lookback{sources}else{game.try_trigger_source_lookback_snapshots()?};
            let participant_sources=participant_sources.or_else(||self.prior_objects.map(|_|lookback.clone()));
            let mut events=Vec::new();let mut affected=Vec::new();
            for (performer, prior_snapshot, player) in completed_performers {
                // Live actor characteristics include every completed player's
                // history update; retained predecessor actors keep their exact
                // immutable packet and never bind a replacement incarnation.
                let source = performer_memory.get(&performer).cloned()
                    .or_else(||ObjectSnapshot::from_object_id(game,performer))
                    .unwrap_or(prior_snapshot);
                if self.performer.is_some(){affected.push(source.clone());}
                let mut event=crate::events::NamedActionEvent::new(self.label.clone(),player,performer,source,objects.clone(),amount,ctx.cause.clone())
                    .with_mana_spent(ctx.mana.activation_payment.clone()).with_combat_phase(matches!(game.turn.phase, crate::game_state::Phase::Combat).then_some(game.turn_store.combat_phases_started_this_turn)).with_complete_source_lookback();
                if let Some(sources) = &participant_sources { event = event.with_participant_source_lookback(sources.clone()); }
                let provenance=game.alloc_child_event_provenance(ctx.provenance,crate::events::EventKind::KeywordAction);
                events.push(TriggerEvent::new_with_provenance(event,provenance).with_lookback_source_snapshots(lookback.clone()));
            }
            if self.performer.is_none(){affected=objects;}
            let affected_ids=affected.iter().map(|snapshot|snapshot.object_id).collect();
            let count=u64::try_from(performers.len()).ok().and_then(|count|count.checked_mul(u64::from(amount))).and_then(|count|count.checked_add(u64::from(staged)))
                .and_then(|count|i32::try_from(count).ok()).ok_or_else(||ExecutionError::Impossible("named action count exceeds outcome vocabulary".into()))?;
            Ok(EffectOutcome::count(count).with_affected_objects(affected_ids).with_affected_object_memory(affected).with_events(events))
        })
    }
    fn get_target_spec(&self)->Option<&crate::target::ChooseSpec> {self.performer.as_ref().or(self.objects.as_ref())}

}

impl CostExecutableEffect for PerformNamedActionEffect {
    fn can_execute_as_cost(&self, _game: &GameState, _source: ObjectId, _controller: PlayerId)
        -> Result<(), crate::effects::CostValidationError> {
        if self.prior_objects.is_none() {
            return Err(crate::effects::CostValidationError::Other("a named payment completion requires an exact prior operation receipt".into()));
        }
        // Affordability belongs to the physical predecessor cost. During live
        // payment execute reads only its actual packet; an empty/declined
        // predecessor produces no event and no completed-object count.
        Ok(())
    }
}
