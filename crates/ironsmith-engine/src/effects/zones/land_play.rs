//! One land action owner for root permissions and resolving instructions.

use crate::effect::EffectOutcome;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{EventOutcome, PreparedEventOutcome};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::{EntryCommitResult, GameState, LibraryTopAnnouncement};
use crate::ids::{ObjectId, PlayerId};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Preserve the existing caller's publication boundary relative to land history.
#[derive(Clone, Copy)]
pub(crate) enum LandPlayObservationTiming {
    BeforeHistory,
    AfterHistory,
}

#[derive(Clone, Copy)]
pub(crate) enum LandPlayObservationKind {
    Entry,
    Played,
}

#[derive(Clone)]
pub(crate) enum LandPlayAuthorization {
    SelectedPermission {
        back_face: bool,
        /// An already opened exile action must retain its exact unqualified grant.
        opened_permission: Option<crate::alternative_cast::GrantSelection>,
    },
    /// The resolving instruction supplies permission to play this exact object.
    /// It does not supply another land allowance or permission on another turn.
    ResolvingInstruction {
        from_zone: Zone,
        temporary_copy: bool,
    },
}

/// Entry adapters retain their existing proposal/discovery contracts. Both
/// receipts complete through the ordinary zone receipt owner, after land action
/// bookkeeping, and never rerun the original or replacement matching.
enum LandPlayEntryReceipt {
    Root(EntryCommitResult),
    Contextual(super::BattlefieldEntryReceipt),
}

impl LandPlayEntryReceipt {
    fn subject(&self, game: &GameState) -> Option<ObjectId> {
        let id = match self {
            Self::Root(receipt) => match &receipt.original {
                EventOutcome::Proceed(entry) => entry.new_id,
                _ => return None,
            },
            Self::Contextual(receipt) => match receipt.outcome {
                super::BattlefieldEntryOutcome::Moved(id) => id,
                super::BattlefieldEntryOutcome::Redirected(ref change) => change.new_object_id?,
                _ => return None,
            },
        };
        game.object(id).map(|_| id)
    }

    fn original_summary(&self) -> EffectOutcome {
        match self {
            Self::Root(receipt) => match &receipt.original {
                EventOutcome::Proceed(entry) => EffectOutcome::with_objects(vec![entry.new_id]),
                EventOutcome::Prevented => EffectOutcome::prevented(),
                EventOutcome::Replaced => EffectOutcome::replaced(),
                EventOutcome::NotApplicable => EffectOutcome::target_invalid(),
            },
            Self::Contextual(receipt) => match &receipt.outcome {
                super::BattlefieldEntryOutcome::Moved(id) => EffectOutcome::with_objects(vec![*id]),
                super::BattlefieldEntryOutcome::Redirected(change) => {
                    EffectOutcome::with_objects(change.new_object_ids.clone())
                }
                super::BattlefieldEntryOutcome::Prevented => EffectOutcome::impossible(),
            },
        }
    }

    fn into_zone_receipt_with_outputs(
        self,
        game: &mut GameState,
        card: ObjectId,
    ) -> Result<
        (
            PreparedEventOutcome<super::AppliedZoneChange>,
            Vec<crate::effects::PublishedEffectOutputs>,
        ),
        ExecutionError,
    > {
        match self {
            Self::Contextual(receipt) => {
                let ((_, receipt), outputs) = receipt.into_zone_receipt_with_outputs();
                Ok((receipt, outputs))
            }
            Self::Root(receipt) => {
                let published = receipt.published_outputs;
                let original = receipt.original.map(|entry| entry.new_id);
                let receipt = super::promote_committed_zone_change_receipt(
                    game,
                    card,
                    PreparedEventOutcome {
                        original,
                        programs: receipt.programs,
                    },
                )?;
                Ok((receipt, published))
            }
        }
    }
}

/// Query against the caller's observed/proposed face, shared with root legality.
pub(crate) fn land_play_restriction_applies(
    game: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> Result<bool, ExecutionError> {
    let object = game
        .object(card)
        .ok_or(ExecutionError::ObjectNotFound(card))?;
    Ok(game
        .effect_store
        .cant_effects
        .cant_play_land_filters
        .get(&player)
        .is_some_and(|restrictions| {
            restrictions.iter().any(|restriction| {
                let context = game
                    .filter_context_for_combat(
                        restriction.controller,
                        restriction.source,
                        None,
                        None,
                    )
                    .with_iterated_player(restriction.iterated_player.or(Some(player)))
                    .with_tagged_objects(&restriction.tagged_objects);
                restriction.filter.matches(object, &context, game)
            })
        }))
}

fn record_land_play(game: &mut GameState, player: PlayerId) {
    if let Some(player) = game.player_mut(player) {
        player.record_land_play();
    }
}

/// A successful original phase consumes one land allowance even when its entry
/// is modified. Only an actual battlefield entrant supplies the parent play
/// observation; replacement-owned actions retain their own observations.
pub(crate) fn execute_land_play_program<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    card: ObjectId,
    player: PlayerId,
    authorization: LandPlayAuthorization,
    timing: LandPlayObservationTiming,
    observe: impl FnMut(
        &mut GameState,
        &mut ExecutionContext<'a>,
        ObjectId,
        LandPlayObservationKind,
        TriggerEvent,
    ) -> Result<(), ExecutionError>,
) -> Result<EffectOutcome, ExecutionError> {
    execute_land_play_program_with_outputs(game, ctx, card, player, authorization, timing, observe)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn execute_land_play_program_with_outputs<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    card: ObjectId,
    player: PlayerId,
    authorization: LandPlayAuthorization,
    timing: LandPlayObservationTiming,
    mut observe: impl FnMut(
        &mut GameState,
        &mut ExecutionContext<'a>,
        ObjectId,
        LandPlayObservationKind,
        TriggerEvent,
    ) -> Result<(), ExecutionError>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let root = matches!(
                authorization,
                LandPlayAuthorization::SelectedPermission { .. }
            );
            let actual_from = game
                .object(card)
                .ok_or(ExecutionError::ObjectNotFound(card))?
                .zone;
            if root {
                game.begin_library_top_announcement(LibraryTopAnnouncement::Land(card));
            }
            let selected_entry_definition = if let LandPlayAuthorization::SelectedPermission { back_face, .. } = &authorization {
                crate::special_actions::apply_land_play_face(game, card, *back_face)
            } else { None };
            let checked = game
                .continuous_query_snapshot()
                .map_err(ExecutionError::ContinuousDiscovery)?;
            let legal = checked.is_active_player(player)
                && checked
                    .player(player)
                    .is_some_and(|player| player.can_play_land())
                && !land_play_restriction_applies(&checked, player, card)?;
            if !legal {
                return if root {
                    Err(ExecutionError::Impossible(
                        "land action is prohibited or has no turn allowance".into(),
                    ))
                } else {
                    Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::impossible(),
                    ))
                };
            }
            let permission = if let LandPlayAuthorization::SelectedPermission {
                opened_permission: Some(permission),
                ..
            } = &authorization
            {
                crate::special_actions::opened_land_play_permission(game, player, card, permission)?
            } else if root {
                crate::special_actions::choose_land_play_permission(
                    game,
                    player,
                    card,
                    &mut ctx.decision_maker,
                )?
            } else {
                crate::special_actions::LandPlayPermissionReceipt::default()
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            permission.reserve(game, player)?;
            game.reserve_next_land_play_timing(player, card);
            let receipt = if root {
                let receipt = game
                    .move_object_with_etb_processing_with_cause_and_entry_options_and_controller(
                        card,
                        Zone::Battlefield,
                        ctx.cause.clone(),
                        &mut ctx.decision_maker,
                        Some(player),
                        permission.enters_tapped,
                        true,
                        selected_entry_definition,
                    )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if receipt.pending {
                    return Err(ExecutionError::InternalError(
                        "land entry reported pending without an outstanding choice".into(),
                    ));
                }
                if matches!(receipt.original, EventOutcome::NotApplicable) {
                    return Err(ExecutionError::ObjectNotFound(card));
                }
                LandPlayEntryReceipt::Root(receipt)
            } else {
                let entry = super::move_to_battlefield_with_options(
                    game,
                    ctx,
                    card,
                    super::BattlefieldEntryOptions::specific(player, permission.enters_tapped),
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                LandPlayEntryReceipt::Contextual(entry.ok_or_else(|| {
                    ExecutionError::InternalError(
                        "land entry lost its receipt without pending input".into(),
                    )
                })?)
            };
            if matches!(
                authorization,
                LandPlayAuthorization::ResolvingInstruction {
                    temporary_copy: true,
                    ..
                }
            ) && matches!(&receipt, LandPlayEntryReceipt::Contextual(entry) if entry.outcome == super::BattlefieldEntryOutcome::Prevented)
            {
                game.remove_object(card);
            }
            let from_zone = match authorization {
                LandPlayAuthorization::ResolvingInstruction { from_zone, .. } => from_zone,
                _ => actual_from,
            };
            let completed_play = receipt
                .subject(game)
                .map(|subject| {
                    let destination = game
                        .object(subject)
                        .ok_or(ExecutionError::ObjectNotFound(subject))?
                        .zone;
                    crate::events::LandPlayedEvent::with_current_snapshot(
                        subject,
                        player,
                        from_zone,
                        destination,
                        game,
                    )
                    .map(|event| (subject, event))
                })
                .transpose()?;
            if matches!(timing, LandPlayObservationTiming::AfterHistory) {
                record_land_play(game, player);
            }
            if let Some((subject, completed_play)) = completed_play {
                // The contextual entry owner already freezes and queues its ETB.
                // Root callers publish their entry here at their existing boundary.
                if let LandPlayEntryReceipt::Root(entry) = &receipt
                    && let EventOutcome::Proceed(entry) = &entry.original
                    && game
                        .object(subject)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
                {
                    let provenance = game
                        .provenance_graph_mut()
                        .alloc_root_event(crate::events::EventKind::EnterBattlefield);
                    let event = super::battlefield_entry_observation(
                        game,
                        subject,
                        actual_from,
                        entry.enters_tapped,
                        provenance,
                        Vec::new(),
                    )?;
                    let mut event = game.ensure_trigger_event_provenance(event);
                    game.freeze_completed_entry_events(std::iter::once(&mut event))?;
                    observe(game, ctx, subject, LandPlayObservationKind::Entry, event)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                }
                let event = crate::effects::observe_action_completion(
                    game,
                    TriggerEvent::new_with_provenance(
                        completed_play,
                        crate::provenance::ProvNodeId::default(),
                    ),
                    if root { None } else { Some(ctx.provenance) },
                )?;
                if event.snapshot().is_none() {
                    return Err(ExecutionError::InternalError(
                        "completed land play has no immutable subject snapshot".into(),
                    ));
                }
                observe(game, ctx, subject, LandPlayObservationKind::Played, event)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
            }
            if matches!(timing, LandPlayObservationTiming::BeforeHistory) {
                record_land_play(game, player);
            }
            let original = receipt.original_summary();
            let (receipt, published) = receipt.into_zone_receipt_with_outputs(game, card)?;
            let mut outputs = super::finish_zone_change_receipts_with_outputs(
                game,
                ctx,
                original,
                vec![(card, receipt)],
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            outputs.retain_published_references(published);
            if root {
                let outcome = &mut outputs.outcome;
                crate::effects::retain_unmatched_outcome_events(game, &mut outcome.events);
                for event in std::mem::take(&mut outcome.events) {
                    game.queue_trigger_event(event.provenance(), event);
                }
                game.finish_library_top_announcement(LibraryTopAnnouncement::Land(card));
            }
            permission.complete(game);
            outputs.synchronize_observations();
            Ok(outputs)
        },
    )
}

pub(crate) fn play_land_from_resolving_effect_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card: ObjectId,
    player: PlayerId,
    from_zone: Zone,
    temporary_copy: bool,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let result = execute_land_play_program_with_outputs(
        game,
        ctx,
        card,
        player,
        LandPlayAuthorization::ResolvingInstruction {
            from_zone,
            temporary_copy,
        },
        LandPlayObservationTiming::BeforeHistory,
        |game, _, _, _, event| {
            game.queue_trigger_event(event.provenance(), event);
            Ok(())
        },
    )?;
    // A copy that cannot even start the action belongs to its caller's proposal,
    // not to the game. Do not strand that provisional object in the command zone.
    if temporary_copy && !ctx.decision_maker.awaiting_choice() && game.object(card).is_some() {
        game.remove_object(card);
    }
    Ok(result)
}
