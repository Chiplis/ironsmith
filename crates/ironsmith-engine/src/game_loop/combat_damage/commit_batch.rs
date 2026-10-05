//! One owner for every original combat damage/result assignment.
use super::*;
use crate::rules::damage::{
    commit_prepared_damage_original, complete_damage_original, freeze_damage_original,
    prepare_processed_damage_assignment,
};

struct Original {
    index: usize,
    source: ObjectId,
    controller: PlayerId,
    snapshot: crate::snapshot::ObjectSnapshot,
    cause: crate::events::cause::EventCause,
    damage: crate::rules::damage::PreparedDamageAssignment,
    toxic: Option<crate::effects::counters::PreparedCounterPlacement>,
}
struct Receipt {
    index: usize,
    source: ObjectId,
    controller: PlayerId,
    snapshot: crate::snapshot::ObjectSnapshot,
    cause: crate::events::cause::EventCause,
    damage: crate::rules::damage::DamageAssignmentReceipt,
    toxic: Option<crate::effects::SimultaneousEffectCommit>,
}
fn target(target: EventDamageTarget) -> DamageEventTarget {
    match target {
        EventDamageTarget::Player(player) => DamageEventTarget::Player(player),
        EventDamageTarget::Object(object) => DamageEventTarget::Object(object),
    }
}
fn append(
    outcome: &mut Option<crate::effect::EffectOutcome>,
    addition: Option<crate::effect::EffectOutcome>,
) {
    if let Some(addition) = addition {
        *outcome = Some(crate::effect::EffectOutcome::aggregate(
            outcome.take().into_iter().chain(std::iter::once(addition)),
        ));
    }
}
// The caller owns the whole step checkpoint, including scope/hold rollback.
pub(super) fn commit_combat_damage_batch(
    game: &mut GameState,
    planned: Vec<PlannedCombatDamage>,
    processed: Vec<crate::events::processing::ProcessedDamageResult>,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    let capacities = CombatExcessCapacities::before_damage(
        game,
        processed
            .iter()
            .filter(|result| !result.replacement_prevented)
            .flat_map(|result| &result.assignments)
            .filter_map(|assignment| match assignment.target {
                EventDamageTarget::Object(object) => Some(object),
                _ => None,
            }),
    );
    let mut events = Vec::new();
    let mut originals = Vec::new();
    let mut additions = Vec::new();
    let mut lifelink = CombatLifelinkTotals::default();
    let mut toxic_occurrences = std::collections::HashSet::new();
    for (plan, result) in planned.into_iter().zip(processed) {
        let base = events.len();
        events.push(CombatDamageEvent {
            damage_receipt: None,
            source_snapshot: Some(plan.source_snapshot.clone()),
            target_snapshot: None,
            source: plan.source,
            target: target(plan.target),
            amount: 0,
            life_lost: 0,
            consequence_outcome: result.payload_outcome,
            result: plan.result.clone(),
            lifelink_outcome: None,
        });
        if !result.programs.is_empty() {
            additions.push((
                base,
                plan.source,
                plan.controller,
                plan.source_snapshot.clone(),
                plan.cause.clone(),
                result.programs,
            ));
        }
        if result.replacement_prevented {
            continue;
        }
        let keywords = crate::rules::damage::SourceDamageKeywords {
            has_deathtouch: plan.result.has_deathtouch,
            has_infect: plan.result.has_infect,
            has_wither: plan.result.has_wither,
            has_lifelink: plan.result.has_lifelink,
        };
        for assignment in result.assignments {
            let mut ctx = ExecutionContext::new(plan.source, plan.controller, &mut *dm)
                .with_cause(plan.cause.clone());
            ctx.source_snapshot = Some(plan.source_snapshot.clone());
            let prepared = prepare_processed_damage_assignment(
                game,
                &mut ctx,
                assignment.target,
                assignment.amount,
                keywords,
            )
            .map_err(|error| CombatDamageAssignmentError::execution(plan.source, error))?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Vec::new());
            }
            if !prepared.applied {
                continue;
            }
            let index = if assignment.target == plan.target {
                base
            } else {
                let index = events.len();
                events.push(CombatDamageEvent {
                    damage_receipt: None,
                    source_snapshot: Some(plan.source_snapshot.clone()),
                    target_snapshot: None,
                    source: plan.source,
                    target: target(assignment.target),
                    amount: 0,
                    life_lost: 0,
                    consequence_outcome: None,
                    result: plan.result.clone(),
                    lifelink_outcome: None,
                });
                index
            };
            events[index].amount = crate::events::damage::checked_damage_amount(
                u128::from(events[index].amount) + u128::from(assignment.amount),
                "combat damage receipt total",
            )
            .map_err(|error| CombatDamageAssignmentError::execution(plan.source, error))?;
            events[index].target_snapshot = prepared.target_snapshot.clone();
            lifelink
                .record(
                    plan.source,
                    plan.controller,
                    keywords.has_lifelink,
                    assignment.amount,
                    base,
                )
                .map_err(|error| CombatDamageAssignmentError::execution(plan.source, error))?;
            let toxic = if let EventDamageTarget::Player(player) = assignment.target
                && toxic_occurrences.insert((plan.source, player))
            {
                prepare_combat_toxic(game, plan.source, &plan.source_snapshot, player, dm)
                    .map_err(|error| CombatDamageAssignmentError::execution(plan.source, error))?
            } else {
                None
            };
            if dm.awaiting_choice() {
                return Ok(Vec::new());
            }
            originals.push(Original {
                index,
                source: plan.source,
                controller: plan.controller,
                snapshot: plan.source_snapshot.clone(),
                cause: plan.cause.clone(),
                damage: prepared,
                toxic,
            });
        }
    }
    let prepared_lifelink = lifelink.prepare(game, &mut events, dm)?;
    if dm.awaiting_choice() {
        return Ok(Vec::new());
    }
    crate::events::damage::checked_damage_count(
        events.iter().map(|event| u128::from(event.amount)).sum(),
        "simultaneous combat damage total",
    )
    .map_err(|error| {
        CombatDamageAssignmentError::execution(
            events
                .first()
                .map_or(ObjectId::from_raw(0), |event| event.source),
            error,
        )
    })?;
    capacities.assign_excess(&mut events);
    // CR 120.4b precedes the life/counter results in 120.4c. No result program
    // can remove the observer or alter this complete damage occurrence.
    capture_combat_consequence_triggers(game, &mut events, dm)?;
    let mut receipts = Vec::new();
    let result_batch = events
        .iter()
        .filter_map(|event| {
            event
                .damage_receipt
                .as_ref()
                .and_then(|receipt| receipt.simultaneous_batch())
        })
        .next();
    let opened = result_batch
        .map(|batch| game.open_simultaneous_action_with_batch(batch))
        .unwrap_or(false);
    game.effect_store.trigger_matching_holds += 1;
    for original in originals {
        let Original {
            index,
            source,
            controller,
            snapshot,
            cause,
            damage,
            toxic,
        } = original;
        let mut ctx = ExecutionContext::new(source, controller, &mut *dm).with_cause(cause.clone());
        ctx.source_snapshot = Some(snapshot.clone());
        let damage = commit_prepared_damage_original(game, &mut ctx, damage)
            .map_err(|error| CombatDamageAssignmentError::execution(source, error))?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        let toxic = toxic
            .map(|prepared| {
                crate::effects::counters::commit_prepared_counter_original(game, &mut ctx, prepared)
            })
            .transpose()
            .map_err(|error| CombatDamageAssignmentError::execution(source, error))?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        events[index].life_lost = crate::events::damage::checked_damage_amount(
            u128::from(events[index].life_lost) + u128::from(damage.original.life_lost),
            "combat life-loss receipt total",
        )
        .map_err(|error| CombatDamageAssignmentError::execution(source, error))?;
        receipts.push(Receipt {
            index,
            source,
            controller,
            snapshot,
            cause,
            damage,
            toxic,
        });
    }
    let mut lifelink_receipts = commit_prepared_combat_lifelink(game, prepared_lifelink, dm)?;
    if dm.awaiting_choice() {
        return Ok(Vec::new());
    }
    crate::events::damage::checked_damage_count(
        events.iter().map(|event| u128::from(event.life_lost)).sum(),
        "simultaneous combat life-loss total",
    )
    .map_err(|error| {
        CombatDamageAssignmentError::execution(
            events
                .first()
                .map_or(ObjectId::from_raw(0), |event| event.source),
            error,
        )
    })?;
    game.effect_store.trigger_matching_holds -= 1;
    game.close_simultaneous_action(opened);
    if let Some(batch) = result_batch {
        for receipt in &mut receipts {
            for event in receipt
                .damage
                .original
                .consequence_outcome
                .iter_mut()
                .flat_map(|outcome| outcome.events.iter_mut())
                .chain(
                    receipt
                        .toxic
                        .iter_mut()
                        .flat_map(|toxic| toxic.outcome.events.iter_mut()),
                )
            {
                if matches!(
                    event.kind(),
                    crate::events::EventKind::MarkersChanged | crate::events::EventKind::LifeLoss
                ) && event.simultaneous_batch().is_none()
                {
                    *event = event.clone().with_simultaneous_batch(batch);
                }
            }
        }
    }
    for event in &events {
        if let DamageEventTarget::Player(player) = event.target {
            if let Some(identity) = game.commander_identity(event.source) {
                let prior = game
                    .player(player)
                    .and_then(|player| player.commander_damage.get(&identity))
                    .copied()
                    .unwrap_or(0);
                crate::events::damage::checked_damage_amount(
                    u128::from(prior) + u128::from(event.amount),
                    "commander damage total",
                )
                .map_err(|error| CombatDamageAssignmentError::execution(event.source, error))?;
            }
            game.record_commander_damage(player, event.source, event.amount);
        }
    }
    if let Some((source, controller)) = receipts
        .first()
        .map(|receipt| (receipt.source, receipt.controller))
    {
        let ctx = ExecutionContext::new(source, controller, &mut *dm);
        crate::effects::capture_triggers_before_added_program(
            game,
            &ctx,
            None,
            receipts
                .iter_mut()
                .flat_map(|receipt| {
                    receipt
                        .damage
                        .original
                        .consequence_outcome
                        .iter_mut()
                        .flat_map(|outcome| outcome.events.iter_mut())
                        .chain(
                            receipt
                                .toxic
                                .iter_mut()
                                .flat_map(|toxic| toxic.outcome.events.iter_mut()),
                        )
                })
                .chain(
                    lifelink_receipts
                        .iter_mut()
                        .flat_map(|(_, _, _, _, receipt)| receipt.outcome.events.iter_mut()),
                ),
        )
        .map_err(|error| CombatDamageAssignmentError::execution(source, error))?;
    }
    // Every original damage result, including lifelink, is now committed.
    // Every completion sees the same complete original result frame, including
    // lifelink, before any replacement-added program begins.
    for receipt in &mut receipts {
        freeze_damage_original(game, &mut receipt.damage)
            .map_err(|error| CombatDamageAssignmentError::execution(receipt.source, error))?;
        if let Some(completion) = receipt
            .toxic
            .as_mut()
            .and_then(|toxic| toxic.completion.as_mut())
        {
            completion
                .freeze(game)
                .map_err(|error| CombatDamageAssignmentError::execution(receipt.source, error))?;
        }
    }
    for (source, _, _, _, receipt) in &mut lifelink_receipts {
        if let Some(completion) = &mut receipt.completion {
            completion
                .freeze(game)
                .map_err(|error| CombatDamageAssignmentError::execution(*source, error))?;
        }
    }
    complete_combat_lifelink(game, &mut events, lifelink_receipts, dm)?;
    if dm.awaiting_choice() {
        return Ok(Vec::new());
    }
    for receipt in receipts {
        let Receipt {
            index,
            source,
            controller,
            snapshot,
            cause,
            damage,
            toxic,
        } = receipt;
        let mut ctx = ExecutionContext::new(source, controller, &mut *dm).with_cause(cause);
        ctx.source_snapshot = Some(snapshot);
        let completed = complete_damage_original(game, &mut ctx, damage)
            .map_err(|error| CombatDamageAssignmentError::execution(source, error))?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        append(
            &mut events[index].consequence_outcome,
            completed.consequence_outcome,
        );
        if let Some(toxic) = toxic {
            let outcome = if let Some(completion) = toxic.completion {
                completion
                    .complete(game, &mut ctx, toxic.outcome)
                    .map_err(|error| CombatDamageAssignmentError::execution(source, error))?
            } else {
                toxic.outcome
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Vec::new());
            }
            append(&mut events[index].consequence_outcome, Some(outcome));
        }
    }
    finish_combat_damage_additions(game, &mut events, additions, dm)?;
    if dm.awaiting_choice() {
        return Ok(Vec::new());
    }
    game.refresh_continuous_state().map_err(|error| {
        CombatDamageAssignmentError::execution(
            events
                .first()
                .map(|event| event.source)
                .unwrap_or(ObjectId::from_raw(0)),
            crate::effects::ExecutionError::ContinuousDiscovery(error),
        )
    })?;
    Ok(events)
}
