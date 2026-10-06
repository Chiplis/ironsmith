//! Finite redirection is allocated when its replacement is selected, against
//! the current simultaneous proposals, never against the authored raw amounts.
//! Outrider en-Kor: Wizards' Time Spiral Remastered release notes (2021-03-23).
use super::*;
use crate::events::DamageEvent;

pub(super) fn needed(game: &GameState) -> bool {
    game.effect_store.replacement_effects.effects().iter().any(|effect| {
        game.effect_store.replacement_effects.is_one_shot(effect.id)
            && matches!(effect.replacement, ReplacementAction::RedirectDamageAmount { .. })
    })
}

struct Proposal {
    original: usize,
    event: Event,
    state: TraitEventProcessingState,
    additional: Vec<ReplacementEffect>,
    complete: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BudgetKey {
    Redirect(ReplacementEffectId),
    Prevention(crate::prevention::PreventionShieldId),
}

struct Candidate {
    members: Vec<(usize, ReplacementEffect)>,
    player: PlayerId,
    priority: ReplacementPriority,
    budget: Option<(BudgetKey, u32)>,
}

fn finite_budget(
    game: &GameState,
    effect: &ReplacementEffect,
    event: &Event,
) -> Option<(BudgetKey, u32)> {
    match &effect.replacement {
        ReplacementAction::RedirectDamageAmount { target, which, amount }
            if game.effect_store.replacement_effects.is_one_shot(effect.id)
                && *amount > 0 =>
        {
            // An impossible redirection neither receives nor spends a quota.
            application::resolve_trait_redirect_target(
                game, event, target, which, effect.controller,
            )?;
            Some((BudgetKey::Redirect(effect.id), *amount))
        }
        ReplacementAction::PreventWithShield { shield_id, .. } => {
            let damage = crate::events::downcast_event::<DamageEvent>(event.inner())?;
            if damage.is_unpreventable {
                return None;
            }
            let remaining = game.effect_store.prevention_effects.shields().iter()
                .find(|shield| shield.id == *shield_id)?.amount_remaining?;
            Some((BudgetKey::Prevention(*shield_id), remaining))
        }
        _ => None,
    }
}

fn candidates(
    game: &GameState,
    originals: &[SimultaneousDamageEvent],
    proposals: &mut [Proposal],
    scopes: &[&crate::effects::ReplacementExecutionContext],
    additional_ids: &mut std::collections::HashMap<ReplacementEffectKey, ReplacementEffectId>,
) -> Result<Vec<Candidate>, crate::effects::ExecutionError> {
    let mut candidates: Vec<Candidate> = Vec::new();
    for (index, proposal) in proposals.iter_mut().enumerate() {
        if proposal.complete || quantitative_event_has_been_removed(&proposal.event) {
            continue;
        }
        let damage = crate::events::downcast_event::<DamageEvent>(proposal.event.inner())
            .ok_or_else(|| crate::effects::ExecutionError::InternalError(
                "simultaneous damage lost its live proposal".into(),
            ))?;
        // Redirected recipients acquire their own shield counters. New shields
        // created by a replacement payload must likewise be discovered here.
        proposal.additional = damage_additional_replacements(
            game, damage.target, originals[proposal.original].source_snapshot.as_ref(), None, scopes[proposal.original],
        );
        for effect in &mut proposal.additional {
            let next_id = ReplacementEffectId(u64::MAX / 4 + additional_ids.len() as u64);
            effect.id = *additional_ids.entry(effect.application_key()).or_insert(next_id);
        }
        let player = proposal.event.inner().affected_player(game);
        for (effect, priority) in find_applicable_trait_replacements(
            game, &proposal.event, &proposal.state, &proposal.additional,
            originals[proposal.original].source_snapshot.as_ref(),
        )? {
            let budget = finite_budget(game, &effect, &proposal.event);
            if let Some((key, _)) = budget
                && let Some(existing) = candidates.iter_mut().find(|candidate| {
                    candidate.priority == priority
                        && candidate.budget.is_some_and(|(other, _)| other == key)
                })
            {
                existing.members.push((index, effect));
                if apnap_position(game, player) < apnap_position(game, existing.player) {
                    existing.player = player;
                }
            } else {
                candidates.push(Candidate {
                    members: vec![(index, effect)], player, priority, budget,
                });
            }
        }
    }
    for candidate in &mut candidates {
        candidate.members.sort_by_key(|(index, _)| {
            (apnap_position(game, proposals[*index].event.inner().affected_player(game)), *index)
        });
    }
    // A self-replacement must precede ordinary replacements of the containing
    // simultaneous event. Affected players make the remaining choices in APNAP.
    if let Some(priority) = candidates.iter().map(|candidate| candidate.priority).min() {
        candidates.retain(|candidate| candidate.priority == priority);
    }
    if let Some(position) = candidates.iter()
        .map(|candidate| apnap_position(game, candidate.player)).min()
    {
        candidates.retain(|candidate| apnap_position(game, candidate.player) == position);
    }
    Ok(candidates)
}

fn choose_candidate(
    game: &GameState,
    proposals: &[Proposal],
    candidates: &[Candidate],
    dm: &mut dyn DecisionMaker,
) -> Result<Option<usize>, crate::effects::ExecutionError> {
    if candidates.len() == 1 {
        return Ok(Some(0));
    }
    let options = candidates.iter().enumerate().map(|(index, candidate)| {
        let (proposal, effect) = &candidate.members[0];
        let source = proposals[*proposal].event.inner().source_object();
        let name = source.and_then(|id| game.current_name(id))
            .unwrap_or_else(|| "the damage source".into());
        let description = replacement_effect_choice_description(game, effect);
        let description = if candidate.budget.is_some() {
            format!("{description}: allocate simultaneous damage")
        } else {
            format!("{description}: damage from {name}")
        };
        crate::decisions::specs::ReplacementOption::new(index, effect.source, description)
            .with_related_objects(source.into_iter().collect())
    }).collect();
    let answer = crate::decisions::make_decision(
        game, dm, candidates[0].player, None,
        crate::decisions::specs::ReplacementSpec::new(options),
    );
    if dm.awaiting_choice() {
        return Ok(None);
    }
    match answer.as_slice() {
        [index] if *index < candidates.len() => Ok(Some(*index)),
        _ => Err(crate::effects::ExecutionError::InternalError(
            "simultaneous damage replacement must name exactly one offered choice".into(),
        )),
    }
}

fn allocate(
    game: &GameState,
    proposals: &[Proposal],
    candidate: &Candidate,
    dm: &mut dyn DecisionMaker,
) -> Result<Option<Vec<u32>>, crate::effects::ExecutionError> {
    let Some((key, capacity)) = candidate.budget else {
        return Ok(Some(vec![0; candidate.members.len()]));
    };
    let amounts = candidate.members.iter().map(|(index, _)| {
        crate::events::downcast_event::<DamageEvent>(proposals[*index].event.inner())
            .map(|event| event.amount)
            .ok_or_else(|| crate::effects::ExecutionError::InternalError(
                "damage allocation lost its proposal".into(),
            ))
    }).collect::<Result<Vec<_>, _>>()?;
    let total = amounts.iter().map(|amount| u128::from(*amount)).sum::<u128>();
    let mut remaining = u128::from(capacity).min(total) as u32;
    let mut quotas = Vec::with_capacity(amounts.len());
    for (position, (index, effect)) in candidate.members.iter().enumerate() {
        let later = amounts[position + 1..].iter()
            .map(|amount| u128::from(*amount)).sum::<u128>();
        let minimum = u128::from(remaining).saturating_sub(later) as u32;
        let maximum = amounts[position].min(remaining);
        let chosen = if minimum == maximum {
            minimum
        } else {
            let source = proposals[*index].event.inner().source_object();
            let source_name = source.and_then(|source| game.current_name(source))
                .unwrap_or_else(|| "the damage source".into());
            let shield_name = game.current_name(effect.source)
                .unwrap_or_else(|| "the replacement".into());
            let kind = match key {
                BudgetKey::Redirect(_) => "redirected",
                BudgetKey::Prevention(_) => "prevented",
            };
            let spec = crate::decisions::NumberSpec::range(
                effect.source, minimum, maximum,
                format!("Choose how much {kind} damage from {source_name} uses {shield_name}"),
            );
            let chosen = crate::decisions::make_decision_with_fallback(
                game, dm, proposals[*index].event.inner().affected_player(game),
                Some(effect.source), spec, crate::decision::FallbackStrategy::Maximum,
            );
            if dm.awaiting_choice() {
                return Ok(None);
            }
            if chosen < minimum || chosen > maximum {
                return Err(crate::effects::ExecutionError::InternalError(
                    "simultaneous damage allocation is outside the offered range".into(),
                ));
            }
            chosen
        };
        quotas.push(chosen);
        remaining -= chosen;
    }
    Ok(Some(quotas))
}

fn append_result(into: &mut ProcessedDamageResult, addition: ProcessedDamageResult) {
    into.assignments.extend(addition.assignments);
    into.replacement_prevented |= addition.replacement_prevented;
    into.programs.extend(addition.programs);
    into.original_payloads.extend(addition.original_payloads);
    if let Some(payload) = addition.payload_outcome {
        into.payload_outcome = Some(crate::effect::EffectOutcome::aggregate(
            into.payload_outcome.take().into_iter().chain(std::iter::once(payload)),
        ));
    }
}

fn finish_proposal(
    game: &mut GameState,
    proposal: &mut Proposal,
    originals: &[SimultaneousDamageEvent],
    result: TraitEventResult,
    scope: &crate::effects::ReplacementExecutionContext,
    dm: &mut dyn DecisionMaker,
) -> Result<ProcessedDamageResult, crate::effects::ExecutionError> {
    proposal.complete = true;
    let result = retain_additional_programs(result, &mut proposal.state);
    let original = &originals[proposal.original];
    finish_processed_damage_result(
        game, original.source, original.cause.clone(), original.source_snapshot.as_ref(),
        proposal.event.provenance(), dm, scope, result, std::mem::take(&mut proposal.state),
    )
}

/// Retain each prevention addition's original assignment and captured scope.
/// Redirected branches share the original owner; separate assignments do not.
fn with_follow_up_owner<T>(
    game: &mut GameState,
    scope: &crate::effects::ReplacementExecutionContext,
    original: usize,
    owners: &mut Vec<usize>,
    body: impl FnOnce(&mut GameState) -> Result<T, crate::effects::ExecutionError>,
) -> Result<T, crate::effects::ExecutionError> {
    let before = game.effect_store.prevention_effects.pending_follow_up_count();
    game.effect_store.prevention_effects.begin_follow_up_replacement_scope(scope);
    let result = body(game);
    game.effect_store.prevention_effects.end_follow_up_replacement_scope();
    let value = result?;
    let after = game.effect_store.prevention_effects.pending_follow_up_count();
    let added = after.checked_sub(before).ok_or_else(||
        crate::effects::ExecutionError::InternalError(
            "damage proposal consumed another assignment's prevention follow-ups".into(),
        ))?;
    owners.extend(std::iter::repeat_n(original, added));
    Ok(value)
}

pub(super) fn process(
    game: &mut GameState,
    originals: &[SimultaneousDamageEvent],
    dm: &mut dyn DecisionMaker,
    scopes: &[&crate::effects::ReplacementExecutionContext],
) -> Result<(Vec<ProcessedDamageResult>, Vec<usize>), DamageProcessingError> {
    let mut proposals = Vec::with_capacity(originals.len());
    for (original, item) in originals.iter().enumerate() {
        let scope = scopes[original];
        proposals.push(Proposal {
            original,
            event: prepare_damage_proposal(
                game, item.source, item.target, item.amount, item.is_combat,
                item.unpreventable, item.cause.clone(), item.source_snapshot.as_ref(),
            ),
            state: TraitEventProcessingState {
                applied_effects: scope.suppressed_replacement_effects.iter()
                    .copied().filter(|id| id.0 < u64::MAX / 4).collect(),
                applied_effect_keys: scope.suppressed_replacement_effect_keys.clone(),
                ..Default::default()
            },
            additional: Vec::new(),
            complete: false,
        });
    }
    let mut results = vec![ProcessedDamageResult {
        assignments: Vec::new(), replacement_prevented: false,
        payload_outcome: None, original_payloads: Vec::new(), programs: Vec::new(),
    }; originals.len()];
    let mut failed_source = originals[0].source;
    let mut additional_ids = std::collections::HashMap::new();
    let mut follow_up_owners = Vec::new();
    let operation = (|| -> Result<_, crate::effects::ExecutionError> {
        loop {
            game.update_cant_effects();
            game.update_replacement_effects()
                .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
            let choices = candidates(game, originals, &mut proposals, scopes, &mut additional_ids)?;
            if choices.is_empty() {
                break;
            }
            let Some(chosen) = choose_candidate(game, &proposals, &choices, dm)? else {
                return Ok((results, follow_up_owners));
            };
            let candidate = &choices[chosen];
            let Some(quotas) = allocate(game, &proposals, candidate, dm)? else {
                return Ok((results, follow_up_owners));
            };
            for ((index, original_effect), quota) in candidate.members.iter().zip(quotas) {
                let proposal = &mut proposals[*index];
                let original = &originals[proposal.original];
                let scope = scopes[proposal.original];
                failed_source = original.source;
                let mut effect = original_effect.clone();
                if candidate.budget.is_some() {
                    // The chosen replacement applies to this entire simultaneous
                    // subset even when another source receives all its capacity.
                    if quota == 0 {
                        mark_applied_replacement_choice(&mut proposal.state, original_effect);
                        continue;
                    }
                    match &mut effect.replacement {
                        ReplacementAction::RedirectDamageAmount { amount, .. } => *amount = quota,
                        ReplacementAction::PreventWithShield { max_amount, .. } => *max_amount = Some(quota),
                        _ => unreachable!("only finite damage replacements have quotas"),
                    }
                }
                let original_index = proposal.original;
                let remainders = with_follow_up_owner(game, scope, original_index, &mut follow_up_owners, |game| {
                    proposal.state.increment();
                    let applied = apply_trait_replacement_retaining_damage_branches(
                        game, proposal.event.clone(), &effect, &mut proposal.state,
                    )?;
                    // Per-branch ephemeral prevention descriptors retain their own
                    // original identity even though the application uses a quota.
                    mark_applied_replacement_choice(&mut proposal.state, original_effect);
                    consume_one_shot_if_applied(game, effect.id, &applied);
                    let remainders = std::mem::take(&mut proposal.state.damage_remainders);
                    match applied {
                        TraitApplyResult::Modified(event) | TraitApplyResult::Unchanged(event) => {
                            proposal.event = event;
                        }
                        TraitApplyResult::Prevented => {
                            let completed = finish_proposal(
                                game, proposal, originals, TraitEventResult::Prevented, scope, dm,
                            )?;
                            append_result(&mut results[proposal.original], completed);
                        }
                        TraitApplyResult::Replaced(effects) => {
                            let result = TraitEventResult::Replaced {
                                context: Box::new(ReplacementEventContext::new(
                                    game, proposal.event.clone(), &proposal.state,
                                )),
                                effects, effect_id: effect.id,
                                replacement: effect.replacement.clone(),
                                source: effect.source, controller: effect.controller,
                            };
                            let completed = finish_proposal(game, proposal, originals, result, scope, dm)?;
                            append_result(&mut results[proposal.original], completed);
                        }
                        TraitApplyResult::NeedsInteraction { .. } => {
                            return Err(crate::effects::ExecutionError::InternalError(
                                "damage replacement requested an unsupported interaction".into(),
                            ));
                        }
                    }
                    Ok(remainders)
                })?;
                if dm.awaiting_choice() {
                    return Ok((results, follow_up_owners));
                }
                let original_index = proposal.original;
                if let Some(damage) = crate::events::downcast_event::<DamageEvent>(proposal.event.inner())
                    && damage.remainder.is_some()
                {
                    let mut damage = damage.clone();
                    damage.remainder = None;
                    proposal.event = proposal.event.rewrap(damage);
                }
                let remainder_envelope = proposal.event.clone();
                for remainder in remainders {
                    let event = remainder_envelope.rewrap(remainder.event);
                    // The split-time history, not the primary branch's later
                    // history, determines what may still apply to the remainder.
                    proposals.push(Proposal {
                        original: original_index,
                        additional: Vec::new(),
                        event,
                        state: TraitEventProcessingState {
                            applied_effects: remainder.applied_effects.into_iter()
                                .filter(|id| id.0 < u64::MAX / 4).collect(),
                            applied_effect_keys: remainder.applied_effect_keys,
                            ..Default::default()
                        },
                        complete: false,
                    });
                }
            }
        }
        for proposal in &mut proposals {
            if !proposal.complete {
                failed_source = originals[proposal.original].source;
                let result = TraitEventResult::Proceed(proposal.event.clone());
                let original = proposal.original;
                let completed = with_follow_up_owner(
                    game, scopes[original], original, &mut follow_up_owners,
                    |game| finish_proposal(game, proposal, originals, result, scopes[original], dm),
                )?;
                append_result(&mut results[proposal.original], completed);
                if dm.awaiting_choice() {
                    return Ok((results, follow_up_owners));
                }
            }
        }
        Ok((results, follow_up_owners))
    })();
    operation.map_err(|error| DamageProcessingError { source: failed_source, error })
}
