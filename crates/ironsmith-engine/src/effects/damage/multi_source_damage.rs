//! Simultaneous damage from independently evaluated, exactly bound sources.
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::damage::{checked_damage_amount, checked_damage_count};
use crate::events::processing::{
    SimultaneousDamageEvent, process_simultaneous_damage_assignments_with_event_with_scope,
    with_deferred_prevention_follow_up_outcome,
};
use crate::events::{DamageEvent, DamageTarget, Event, LifeGainEvent};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::DealDamageBySourcesEffect;

impl EffectExecutor for DealDamageBySourcesEffect {
    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        (self.recipient_binding == ironsmith_core::DamageRecipientSetBinding::SharedSet)
            .then_some(&self.target)
    }
    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.get_target_spec().map(ChooseSpec::count)
    }
    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        self.sources
            .iter()
            .chain(self.get_target_spec())
            .cloned()
            .collect()
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = self.execute_bound(game, ctx);
        let pending = ctx.decision_maker.awaiting_choice();
        if result.is_err() || pending {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && pending);
            context_checkpoint.restore(ctx);
        }
        if pending && result.is_ok() {
            return Ok(EffectOutcome::count(0));
        }
        result
    }
}
trait ExecuteCapturedDamage {
    fn execute_bound(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;
}
impl ExecuteCapturedDamage for DealDamageBySourcesEffect {
    fn execute_bound(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Freeze one recipient set before any source's power, replacement or
        // result is executed. Each source uses this same set; no nested loop
        // executes a damage instruction independently.
        let each_source =
            self.recipient_binding == ironsmith_core::DamageRecipientSetBinding::EachSource;
        let proposed = if each_source {
            Vec::new()
        } else {
            match self.target.base() {
                ChooseSpec::Player(_)
                | ChooseSpec::EachPlayer(_)
                | ChooseSpec::SpecificPlayer(_)
                | ChooseSpec::SourceController
                | ChooseSpec::SourceOwner => {
                    match crate::effects::helpers::resolve_players_from_spec(
                        game,
                        &self.target,
                        ctx,
                    ) {
                        Ok(players) => players
                            .into_iter()
                            .filter(|player| {
                                game.player(*player)
                                    .is_some_and(|player| player.is_in_game())
                            })
                            .map(DamageTarget::Player)
                            .collect::<Vec<_>>(),
                        Err(ExecutionError::InvalidTarget) => Vec::new(),
                        Err(error) => return Err(error),
                    }
                }
                ChooseSpec::Object(_)
                | ChooseSpec::All(_)
                | ChooseSpec::Tagged(_)
                | ChooseSpec::SpecificObject(_)
                | ChooseSpec::Source
                | ChooseSpec::Iterated => {
                    match crate::effects::helpers::resolve_objects_from_spec(
                        game,
                        &self.target,
                        ctx,
                    ) {
                        Ok(objects) => objects
                            .into_iter()
                            .filter(|object| {
                                game.object(*object).is_some_and(|object| {
                                    object.zone == crate::zone::Zone::Battlefield
                                }) && !game.is_phased_out(*object)
                                    && super::deal_damage::object_can_be_dealt_damage(game, *object)
                            })
                            .map(DamageTarget::Object)
                            .collect::<Vec<_>>(),
                        Err(ExecutionError::InvalidTarget)
                        | Err(ExecutionError::TagNotFound(_)) => Vec::new(),
                        Err(error) => return Err(error),
                    }
                }
                _ => {
                    return Err(ExecutionError::UnresolvableValue(
                        "multi-source damage requires an object or player recipient set".into(),
                    ));
                }
            }
        };
        let mut recipients = Vec::new();
        for recipient in proposed {
            if !recipients.contains(&recipient) {
                recipients.push(recipient);
            }
        }
        if !each_source && recipients.is_empty() {
            return Ok(if self.target.is_target() {
                EffectOutcome::target_invalid()
            } else {
                EffectOutcome::count(0)
            });
        }
        let mut bindings = Vec::<(ObjectId, Option<ObjectSnapshot>)>::new();
        for group in &self.sources {
            if self.source_binding == ironsmith_core::DamageSourceSetBinding::CapturedIncarnations {
                let ChooseSpec::Tagged(tag) = group.base() else {
                    return Err(ExecutionError::UnresolvableValue(
                        "captured damage sources require an exact object-set receipt".into(),
                    ));
                };
                for captured in ctx.get_tagged_all(tag).into_iter().flatten() {
                    if captured.zone != crate::zone::Zone::Battlefield {
                        return Err(ExecutionError::UnresolvableValue(
                            "captured damage source was not a battlefield object".into(),
                        ));
                    }
                    if bindings.iter().any(|(id, _)| *id == captured.object_id) {
                        continue;
                    }
                    let snapshot = if let Some(object) =
                        game.object(captured.object_id).filter(|object| {
                            object.zone == crate::zone::Zone::Battlefield
                                && !game.is_phased_out(object.id)
                        }) {
                        ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
                    } else {
                        game.turn_store
                            .turn_history
                            .source_last_known_snapshot(captured.object_id)
                            .cloned()
                            .ok_or_else(|| {
                                ExecutionError::UnresolvableValue(
                                    "captured damage source has no exact last-known receipt".into(),
                                )
                            })?
                    };
                    bindings.push((captured.object_id, Some(snapshot)));
                }
                continue;
            }
            let objects = match crate::effects::helpers::resolve_objects_from_spec(game, group, ctx)
            {
                Ok(objects) => objects,
                Err(ExecutionError::InvalidTarget) | Err(ExecutionError::TagNotFound(_)) => {
                    Vec::new()
                }
                Err(error) => return Err(error),
            };
            for source in objects {
                if !bindings.iter().any(|(id, _)| *id == source)
                    && game
                        .object(source)
                        .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                    && !game.is_phased_out(source)
                {
                    bindings.push((
                        source,
                        game.object(source).map(|object| {
                            ObjectSnapshot::from_object_with_calculated_characteristics(
                                object, game,
                            )
                        }),
                    ));
                }
            }
        }
        let mut events = Vec::new();
        // Amounts are all read before a single damage/prevention consequence.
        for (source, snapshot) in bindings {
            let source_recipients = if each_source {
                if !game
                    .object(source)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                    || game.is_phased_out(source)
                    || !super::deal_damage::object_can_be_dealt_damage(game, source)
                {
                    continue;
                }
                vec![DamageTarget::Object(source)]
            } else {
                recipients.clone()
            };
            let old_source = ctx.source;
            let old_snapshot = ctx.source_snapshot.clone();
            ctx.source = source;
            ctx.source_snapshot = snapshot.clone();
            let amount = crate::effects::helpers::resolve_value(game, &self.amount, ctx);
            ctx.source = old_source;
            ctx.source_snapshot = old_snapshot;
            let amount = amount?.max(0) as u32;
            if amount > 0 {
                for recipient in &source_recipients {
                    events.push(SimultaneousDamageEvent {
                        source,
                        target: *recipient,
                        amount,
                        is_combat: false,
                        unpreventable: self.unpreventable,
                        cause: ctx.cause.clone(),
                        source_snapshot: snapshot.clone(),
                    });
                }
            }
        }
        if events.is_empty() {
            return Ok(EffectOutcome::count(0));
        }
        let source = ctx.source;
        let controller = ctx.controller;
        let cause = ctx.cause.clone();
        let provenance = ctx.provenance;
        let scope = ctx.replacement.clone();
        let batch = game.simultaneous_action_batch().unwrap_or_else(|| {
            game.alloc_child_event_provenance(provenance, crate::events::EventKind::Damage)
        });
        with_deferred_prevention_follow_up_outcome(game, ctx.decision_maker, |game, dm| {
            let mut parent = ExecutionContext::new(source, controller, dm)
                .with_cause(cause)
                .with_provenance(provenance);
            parent.replacement = scope;
            commit_damage_batch(game, &mut parent, events, Some(batch))
        })
    }
}

#[derive(Clone)]
struct SourceState {
    source: ObjectId,
    controller: PlayerId,
    snapshot: Option<ObjectSnapshot>,
    keywords: crate::rules::damage::SourceDamageKeywords,
}
/// Every original damage assignment, consequence and observer belongs to this
/// one occurrence. Replacement-added instructions run only after all originals.
pub(crate) fn commit_damage_batch(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    events: Vec<SimultaneousDamageEvent>,
    mut batch: Option<crate::provenance::ProvNodeId>,
) -> Result<EffectOutcome, ExecutionError> {
    // Capture the authored recipient, not a replacement redirect's new
    // destination. These scalars precede every original damage consequence.
    game.refresh_continuous_state()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let mut original_recipients = Vec::new();
    for event in &events {
        let receipt = match event.target {
            DamageTarget::Player(player) => {
                game.player(player)
                    .map(|state| crate::effect::DamageRecipientBefore::Player {
                        player,
                        life: state.life,
                    })
            }
            DamageTarget::Object(object) => game
                .try_current_characteristics(object)
                .map_err(ExecutionError::ContinuousDiscovery)?
                .map(|frame| crate::effect::DamageRecipientBefore::Object {
                    object,
                    was_creature: frame.card_types.contains(&crate::CardType::Creature),
                    loyalty: frame
                        .card_types
                        .contains(&crate::CardType::Planeswalker)
                        .then(|| {
                            game.object(object)
                                .and_then(|state| state.loyalty())
                                .unwrap_or(0)
                        }),
                }),
        };
        if let Some(receipt) = receipt
            && !original_recipients.contains(&receipt)
        {
            original_recipients.push(receipt);
        }
    }
    let processed = process_simultaneous_damage_assignments_with_event_with_scope(
        game,
        &events,
        parent.decision_maker,
        &parent.replacement,
    )?;
    if parent.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    if processed.len() != events.len() {
        return Err(ExecutionError::InternalError(
            "simultaneous damage lost an original assignment".into(),
        ));
    }
    let states = events
        .iter()
        .map(|event| {
            let snapshot = game
                .object(event.source)
                .filter(|_| !game.is_phased_out(event.source))
                .map(|object| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
                })
                .or_else(|| {
                    game.turn_store
                        .turn_history
                        .source_departure_snapshot(event.source)
                        .cloned()
                })
                .or_else(|| event.source_snapshot.clone());
            let controller = snapshot
                .as_ref()
                .map(|snapshot| snapshot.controller)
                .unwrap_or(parent.controller);
            let keywords =
                crate::rules::damage::source_damage_keywords(game, event.source, snapshot.as_ref());
            SourceState {
                source: event.source,
                controller,
                snapshot,
                keywords,
            }
        })
        .collect::<Vec<_>>();
    let any_prevented = processed.iter().any(|result| result.replacement_prevented);
    let any_replaced = processed
        .iter()
        .any(|result| result.payload_outcome.is_some());
    let mut prepared = Vec::new();
    let mut reported = Vec::new();
    let mut programs = Vec::new();
    let mut payloads = Vec::new();
    let mut lifelink = Vec::<(usize, u32)>::new();
    let mut total = 0i64;
    let mut capacities = std::collections::HashMap::<ObjectId, u32>::new();
    // Excess capacity is sampled before this event, across every real source.
    for result in &processed {
        for assignment in &result.assignments {
            if let DamageTarget::Object(target) = assignment.target {
                capacities.entry(target).or_insert_with(|| {
                    let creature = game
                        .current_has_card_type(target, crate::types::CardType::Creature)
                        .then(|| {
                            let damage = i64::from(game.damage_on(target));
                            (i64::from(game.calculated_toughness(target).unwrap_or(0)) - damage)
                                .max(0)
                                .min(i64::from(u32::MAX)) as u32
                        });
                    let loyalty = game
                        .current_has_card_type(target, crate::types::CardType::Planeswalker)
                        .then(|| {
                            game.object(target)
                                .and_then(|object| object.loyalty())
                                .unwrap_or(0)
                        });
                    let defense = game
                        .current_has_card_type(target, crate::types::CardType::Battle)
                        .then(|| {
                            game.object(target)
                                .and_then(|object| {
                                    object.counters.get(&crate::CounterType::Defense).copied()
                                })
                                .unwrap_or(0)
                        });
                    [creature, loyalty, defense]
                        .into_iter()
                        .flatten()
                        .min()
                        .unwrap_or(u32::MAX)
                });
            }
        }
    }
    for (index, result) in processed.iter().enumerate() {
        if states[index].keywords.has_deathtouch {
            for assignment in &result.assignments {
                if let DamageTarget::Object(target) = assignment.target
                    && assignment.amount > 0
                    && game.current_has_card_type(target, crate::types::CardType::Creature)
                {
                    if let Some(capacity) = capacities.get_mut(&target) {
                        *capacity = (*capacity).min(1);
                    }
                }
            }
        }
    }
    let mut dealt = std::collections::HashMap::<ObjectId, u64>::new();
    // Avoid aliasing the parent's decision-maker borrow while copying context.
    let cause = parent.cause.clone();
    let provenance = parent.provenance;
    let scope = parent.replacement.clone();
    for (index, result) in processed.into_iter().enumerate() {
        programs.extend(result.programs);
        if let Some(mut outcome) = result.payload_outcome {
            for event in &mut outcome.events {
                if let Some(batch) = batch {
                    *event = event.clone().with_simultaneous_batch(batch);
                }
            }
            payloads.push(outcome);
        }
        for assignment in result.assignments {
            let observation =
                game.alloc_child_event_provenance(provenance, crate::events::EventKind::Damage);
            let state = &states[index];
            let mut ctx =
                ExecutionContext::new(state.source, state.controller, &mut *parent.decision_maker)
                    .with_cause(cause.clone())
                    .with_provenance(observation);
            ctx.source_snapshot = state.snapshot.clone();
            ctx.replacement = scope.clone();
            let amount =
                checked_damage_count(u128::from(assignment.amount), "damage result count")?;
            let plan = crate::rules::damage::prepare_processed_damage_assignment(
                game,
                &mut ctx,
                assignment.target,
                assignment.amount,
                state.keywords,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if !plan.applied {
                continue;
            }
            total =
                checked_damage_count(total as u128 + amount as u128, "simultaneous damage total")?;
            let excess = if let DamageTarget::Object(target) = assignment.target {
                let prior = *dealt.get(&target).unwrap_or(&0);
                let next = prior + u64::from(assignment.amount);
                dealt.insert(target, next);
                let capacity = u64::from(*capacities.get(&target).unwrap_or(&u32::MAX));
                checked_damage_amount(
                    u128::from(next.saturating_sub(prior.max(capacity))),
                    "simultaneous excess damage",
                )?
            } else {
                0
            };
            let mut event = DamageEvent::with_cause(
                state.source,
                assignment.target,
                assignment.amount,
                events[index].is_combat,
                cause.clone(),
            )
            .with_excess_damage(excess);
            if let Some(snapshot) = plan.target_snapshot.clone() {
                event = event.with_target_snapshot(snapshot);
            }
            let mut event = TriggerEvent::new_with_provenance(event, observation);
            if let Some(batch) = batch {
                event = event.with_simultaneous_batch(batch);
            }
            if let Some(snapshot) = state.snapshot.clone() {
                event = event.with_source_snapshot(snapshot);
            }
            reported.push(event);
            if state.keywords.has_lifelink {
                if let Some((_, sum)) = lifelink
                    .iter_mut()
                    .find(|(existing, _)| states[*existing].source == state.source)
                {
                    *sum = checked_damage_amount(
                        u128::from(*sum) + u128::from(assignment.amount),
                        "lifelink damage total",
                    )?;
                } else {
                    lifelink.push((index, assignment.amount));
                }
            }
            prepared.push((index, observation, plan));
        }
    }
    // One original damage operation can split into several actual chunks
    // through partial redirection, then reconverge on one recipient. It is
    // still one occurrence for thresholds and per-recipient trigger counts.
    if batch.is_none() && reported.len() > 1 {
        let identity =
            game.alloc_child_event_provenance(provenance, crate::events::EventKind::Damage);
        batch = Some(identity);
        for event in &mut reported {
            *event = event.clone().with_simultaneous_batch(identity);
        }
        for outcome in &mut payloads {
            for event in &mut outcome.events {
                *event = event.clone().with_simultaneous_batch(identity);
            }
        }
    }
    // Retain occurrence totals on the original receipts even when an outer
    // simultaneous scope holds matching/publication until a later boundary.
    crate::events::damage::bind_received_damage_amounts(&mut reported);
    // Damage-trigger predicates observe the completed actual damage batch
    // before damage's life/counter results can remove qualified observers.
    crate::effects::runtime::capture_triggers_before_added_program(
        game,
        parent,
        None,
        reported.iter_mut(),
    )?;
    let mut life_prepared = Vec::new();
    for (index, amount) in lifelink {
        let observation =
            game.alloc_child_event_provenance(provenance, crate::events::EventKind::LifeGain);
        let state = &states[index];
        let mut ctx =
            ExecutionContext::new(state.source, state.controller, &mut *parent.decision_maker)
                .with_cause(cause.clone())
                .with_provenance(observation);
        ctx.source_snapshot = state.snapshot.clone();
        ctx.replacement = scope.clone();
        let proposal = crate::effects::life::life_change::prepare_life_change(
            game,
            &mut ctx,
            Event::new_with_provenance(
                LifeGainEvent::new(state.controller, amount).with_source(state.source),
                observation,
            ),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        life_prepared.push((index, observation, proposal));
    }
    let mut receipts = Vec::new();
    let mut life_receipts = Vec::new();
    game.effect_store.trigger_matching_holds += 1;
    for (index, observation, plan) in prepared {
        let state = &states[index];
        let mut ctx =
            ExecutionContext::new(state.source, state.controller, &mut *parent.decision_maker)
                .with_cause(cause.clone())
                .with_provenance(observation);
        ctx.source_snapshot = state.snapshot.clone();
        ctx.replacement = scope.clone();
        receipts.push((
            index,
            observation,
            crate::rules::damage::commit_prepared_damage_original(game, &mut ctx, plan)?,
        ));
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
    }
    for (index, observation, proposal) in life_prepared {
        let state = &states[index];
        let mut ctx =
            ExecutionContext::new(state.source, state.controller, &mut *parent.decision_maker)
                .with_cause(cause.clone())
                .with_provenance(observation);
        ctx.source_snapshot = state.snapshot.clone();
        ctx.replacement = scope.clone();
        life_receipts.push((
            index,
            observation,
            crate::effects::life::life_change::commit_prepared_life_original(
                game, &mut ctx, proposal,
            )?,
        ));
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
    }
    // Result replacements can amplify life amounts independently of actual
    // damage. Grouped event quantities must remain representable as well.
    checked_damage_count(
        receipts
            .iter()
            .map(|(_, _, receipt)| u128::from(receipt.original.life_lost))
            .sum(),
        "simultaneous damage-result life loss",
    )?;
    checked_damage_count(
        life_receipts
            .iter()
            .flat_map(|(_, _, receipt)| &receipt.outcome.events)
            .filter_map(|event| event.downcast::<LifeGainEvent>())
            .map(|event| u128::from(event.amount))
            .sum(),
        "simultaneous lifelink life gain",
    )?;
    game.effect_store.trigger_matching_holds -= 1;
    // Every continuation sees the common post-original state before any
    // earlier completion can remove a source or change an observed value.
    for (_, _, receipt) in &mut receipts {
        crate::rules::damage::freeze_damage_original(game, receipt)?;
    }
    for (_, _, receipt) in &mut life_receipts {
        if let Some(completion) = &mut receipt.completion {
            completion.freeze(game)?;
        }
    }

    for (_, _, receipt) in &mut receipts {
        if let Some(outcome) = &mut receipt.original.consequence_outcome {
            for event in &mut outcome.events {
                if let Some(batch) = batch {
                    *event = event.clone().with_simultaneous_batch(batch);
                }
            }
        }
    }
    for (_, _, receipt) in &mut life_receipts {
        for event in &mut receipt.outcome.events {
            if let Some(batch) = batch {
                *event = event.clone().with_simultaneous_batch(batch);
            }
        }
    }
    crate::effects::runtime::capture_triggers_before_added_program(
        game,
        parent,
        None,
        receipts
            .iter_mut()
            .flat_map(|(_, _, receipt)| {
                receipt
                    .original
                    .consequence_outcome
                    .iter_mut()
                    .flat_map(|outcome| outcome.events.iter_mut())
            })
            .chain(
                life_receipts
                    .iter_mut()
                    .flat_map(|(_, _, receipt)| receipt.outcome.events.iter_mut()),
            )
            .chain(
                payloads
                    .iter_mut()
                    .flat_map(|outcome| outcome.events.iter_mut()),
            ),
    )?;
    let affected = reported
        .iter()
        .filter_map(|event| event.downcast::<DamageEvent>())
        .filter_map(|event| match event.target {
            DamageTarget::Object(id) => Some(id),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut outcome = if total == 0 && any_replaced {
        EffectOutcome::replaced()
    } else if total == 0 && any_prevented {
        EffectOutcome::prevented()
    } else {
        EffectOutcome::count(total)
    };
    for recipient in original_recipients {
        outcome = outcome.with_execution_fact(ExecutionFact::DamageRecipientBefore(recipient));
    }
    if !affected.is_empty() {
        outcome = outcome.with_affected_objects_from_game(game, affected);
    }
    for event in reported {
        if let Some(damage) = event.downcast::<DamageEvent>()
            && damage.excess_damage > 0
        {
            outcome = outcome
                .with_execution_fact(ExecutionFact::ExcessDamageDealt)
                .with_execution_fact(ExecutionFact::ExcessDamage(damage.excess_damage));
        }
        outcome = outcome.with_event(event);
    }
    for (index, observation, receipt) in receipts {
        let state = &states[index];
        let mut ctx =
            ExecutionContext::new(state.source, state.controller, &mut *parent.decision_maker)
                .with_cause(cause.clone())
                .with_provenance(observation);
        ctx.source_snapshot = state.snapshot.clone();
        ctx.replacement = scope.clone();
        if let Some(consequence) =
            crate::rules::damage::complete_damage_original(game, &mut ctx, receipt)?
                .consequence_outcome
        {
            payloads.push(consequence);
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
    }
    for (index, observation, receipt) in life_receipts {
        let state = &states[index];
        let mut ctx =
            ExecutionContext::new(state.source, state.controller, &mut *parent.decision_maker)
                .with_cause(cause.clone())
                .with_provenance(observation);
        ctx.source_snapshot = state.snapshot.clone();
        ctx.replacement = scope.clone();
        payloads.push(if let Some(completion) = receipt.completion {
            completion.complete(game, &mut ctx, receipt.outcome)?
        } else {
            receipt.outcome
        });
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
    }
    outcome = EffectOutcome::aggregate_replacement_outcomes(outcome, payloads);
    super::deal_damage::finish_damage_replacement_programs(game, parent, outcome, programs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::execute_effect;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::PlayerFilter;
    fn setup() -> (GameState, ObjectId, ObjectId, PlayerId, PlayerId, PlayerId) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 30);
        let [a, b, c] = [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
        ];
        let card = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Source")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 4))
            .build();
        let first = game.create_object_from_definition(&card, a, crate::zone::Zone::Battlefield);
        let second = game.create_object_from_definition(&card, a, crate::zone::Zone::Battlefield);
        (game, first, second, a, b, c)
    }
    #[test]
    fn one_source_multi_recipient_life_results_all_commit_before_first_added_program() {
        let (mut game, source, _, a, b, c) = setup();
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                b,
                crate::events::WouldLoseLifeMatcher::you(),
                ReplacementAction::Additionally(vec![Effect::gain_life(crate::Value::LifeTotal(
                    PlayerFilter::Specific(c),
                ))]),
            ));
        let effect = Effect::new(crate::effects::DealDamageToRecipientsEffect {
            amount: crate::Value::Fixed(3),
            recipients: vec![ChooseSpec::SpecificPlayer(b), ChooseSpec::SpecificPlayer(c)],
        });
        let outcome = execute_effect(
            &mut game,
            &effect,
            &mut ExecutionContext::new_default(source, a),
        )
        .unwrap();
        assert_eq!(game.player(c).unwrap().life, 27);
        assert_eq!(
            game.player(b).unwrap().life,
            54,
            "added gain reads the already committed other life loss"
        );
        let damage = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<DamageEvent>())
            .collect::<Vec<_>>();
        assert_eq!(damage.len(), 2);
        assert!(damage.iter().all(|event| event.amount == 3));
    }
    #[test]
    fn multi_source_wide_total_preserves_every_original_and_receipt() {
        let (mut game, first, second, a, _, _) = setup();
        let recipient = game.create_object_from_definition(
            &crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Recipient")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(1, 4))
                .build(),
            a,
            crate::zone::Zone::Battlefield,
        );
        let sources = vec![
            ChooseSpec::SpecificObject(first),
            ChooseSpec::SpecificObject(second),
        ];
        let effect = Effect::new(DealDamageBySourcesEffect::new(
            sources,
            crate::Value::Fixed(i32::MAX),
            ChooseSpec::SpecificObject(recipient),
        ));
        let before = game.turn_store.turn_history.event_records.len();
        let result = execute_effect(
            &mut game,
            &effect,
            &mut ExecutionContext::new_default(first, a),
        );
        let outcome = result.unwrap();
        assert_eq!(outcome.count_or_zero(), i64::from(i32::MAX) * 2);
        assert_eq!(game.damage_on(recipient), i32::MAX as u32 * 2);
        assert_eq!(game.turn_store.turn_history.event_records.len(), before + 2);
        assert_eq!(game.effect_store.trigger_matching_holds, 0);
    }
}

#[cfg(test)]
mod recipient_set_tests {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::execute_effect;
    use crate::target::{ObjectFilter, PlayerFilter};
    #[test]
    fn each_source_and_each_player_are_one_captured_cartesian_damage_event() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let [a, b, c] = [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
        ];
        let card =
            crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Lifelink source")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 4))
                .with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::lifelink(),
                ))
                .build();
        let first = game.create_object_from_definition(&card, a, crate::zone::Zone::Battlefield);
        let second = game.create_object_from_definition(&card, a, crate::zone::Zone::Battlefield);
        let effect = Effect::new(DealDamageBySourcesEffect::new(
            vec![ChooseSpec::All(
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
            )],
            crate::Value::SourcePower,
            ChooseSpec::EachPlayer(PlayerFilter::Opponent),
        ));
        let outcome = execute_effect(
            &mut game,
            &effect,
            &mut ExecutionContext::new_default(first, a),
        )
        .unwrap();
        assert_eq!(game.player(a).unwrap().life, 28);
        assert_eq!(game.player(b).unwrap().life, 16);
        assert_eq!(game.player(c).unwrap().life, 16);
        let damage = outcome
            .events
            .iter()
            .filter(|event| event.downcast::<DamageEvent>().is_some())
            .collect::<Vec<_>>();
        assert_eq!(damage.len(), 4);
        let identities = damage
            .iter()
            .map(|event| event.provenance())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(
            identities.len(),
            4,
            "separate final assignments need separate observation identities"
        );
        let batch = damage[0].simultaneous_batch();
        assert!(batch.is_some());
        assert!(
            damage
                .iter()
                .all(|event| event.simultaneous_batch() == batch)
        );
        for source in [first, second] {
            assert_eq!(
                damage
                    .iter()
                    .filter(|event| event.downcast::<DamageEvent>().unwrap().source == source)
                    .count(),
                2
            );
        }
        let gain = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<LifeGainEvent>())
            .collect::<Vec<_>>();
        assert_eq!(gain.len(), 2);
        assert!(gain.iter().all(|event| event.amount == 4));
    }
}

#[cfg(test)]
mod amplified_result_limit_tests {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::execute_effect;
    use crate::target::PlayerFilter;
    #[test]
    fn amplified_life_loss_sum_retains_wide_aggregate_damage_receipts() {
        let mut game = GameState::new(
            vec!["Alice".into(), "Bob".into(), "Charlie".into()],
            i32::MAX,
        );
        let [a, b, c] = [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
        ];
        let source = game.create_object_from_definition(
            &crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Amplified loss")
                .card_types(vec![crate::types::CardType::Artifact])
                .build(),
            a,
            crate::zone::Zone::Battlefield,
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                a,
                crate::events::WouldLoseLifeMatcher::new(PlayerFilter::Any),
                crate::replacement::ReplacementAction::Modify(
                    crate::replacement::EventModification::Multiply(i32::MAX as u32),
                ),
            ),
        );
        let effect = Effect::new(crate::effects::DealDamageToRecipientsEffect {
            amount: crate::Value::Fixed(1),
            recipients: vec![ChooseSpec::SpecificPlayer(b), ChooseSpec::SpecificPlayer(c)],
        });
        let before = game.turn_store.turn_history.event_records.len();
        let result = execute_effect(
            &mut game,
            &effect,
            &mut ExecutionContext::new_default(source, a),
        );
        let outcome = result.expect("wide aggregate life loss is representable");
        assert_eq!(outcome.count_or_zero(), 2);
        assert_eq!(game.player(b).unwrap().life, 0);
        assert_eq!(game.player(c).unwrap().life, 0);
        let losses = game
            .turn_store
            .turn_history
            .projected_records()
            .skip(before)
            .filter_map(|record| record.event.downcast::<crate::events::LifeLossEvent>())
            .map(|event| u64::from(event.amount))
            .sum::<u64>();
        assert_eq!(losses, 2 * i32::MAX as u64);
        assert_eq!(game.effect_store.trigger_matching_holds, 0);
    }
}

#[cfg(test)]
mod captured_incarnation_tests {
    use super::*;
    use crate::effect::{Effect, Until};
    use crate::effects::execute_effect;
    fn perform(
        game: &mut GameState,
        parent: ObjectId,
        controller: PlayerId,
        effect: Effect,
    ) -> EffectOutcome {
        execute_effect(
            game,
            &effect,
            &mut ExecutionContext::new_default(parent, controller),
        )
        .unwrap()
    }
    #[test]
    fn captured_damage_sources_use_current_or_actual_last_known_power_and_controller_without_following_a_blink()
     {
        for mode in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let parent = game.create_object_from_definition(
                &crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Parent")
                    .card_types(vec![crate::types::CardType::Artifact])
                    .build(),
                a,
                crate::zone::Zone::Battlefield,
            );
            let source = game.create_object_from_definition(
                &crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Old source")
                    .card_types(vec![crate::types::CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(2, 4))
                    .with_ability(crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::lifelink(),
                    ))
                    .build(),
                b,
                crate::zone::Zone::Battlefield,
            );
            let recipient = game.create_object_from_definition(
                &crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Recipient")
                    .card_types(vec![crate::types::CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(1, 50))
                    .build(),
                a,
                crate::zone::Zone::Battlefield,
            );
            let captured = ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(),
                &game,
            );
            perform(
                &mut game,
                parent,
                a,
                Effect::pump(5, 0, ChooseSpec::SpecificObject(source), Until::EndOfTurn),
            );
            perform(
                &mut game,
                parent,
                a,
                Effect::new(crate::effects::GainControlEffect::new(
                    ChooseSpec::SpecificObject(source),
                    Until::EndOfTurn,
                )),
            );
            let mut returned = None;
            if mode == 1 {
                perform(
                    &mut game,
                    parent,
                    a,
                    Effect::exile(ChooseSpec::SpecificObject(source)),
                );
                let exile = *game.exile.last().unwrap();
                perform(
                    &mut game,
                    parent,
                    a,
                    Effect::new(
                        crate::effects::MoveToZoneEffect::new(
                            ChooseSpec::SpecificObject(exile),
                            crate::zone::Zone::Battlefield,
                            false,
                        )
                        .under_owner_control(),
                    ),
                );
                returned = game.battlefield.iter().copied().find(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name == "Old source")
                });
                assert_ne!(returned, Some(source));
                assert_eq!(game.calculated_power(returned.unwrap()), Some(2));
            } else if mode == 2 {
                game.phase_out(source);
            }
            let floor = game.new_object_id();
            let mut ctx = ExecutionContext::new_default(parent, a);
            ctx.resolution_object_id_floor = Some(floor);
            ctx.set_tagged_objects("captured", vec![captured]);
            let effect = DealDamageBySourcesEffect::new(
                vec![ChooseSpec::Tagged("captured".into())],
                crate::Value::SourcePower,
                ChooseSpec::SpecificObject(recipient),
            )
            .with_source_binding(ironsmith_core::DamageSourceSetBinding::CapturedIncarnations);
            let outcome =
                execute_effect(&mut game, &Effect::new(effect.clone()), &mut ctx).unwrap();
            assert_eq!(
                game.damage_on(recipient),
                7,
                "mode {mode}: capture-time power 2 cannot replace current/departure power 7"
            );
            assert_eq!(game.player(a).unwrap().life, 27);
            assert_eq!(game.player(b).unwrap().life, 20);
            let damage = outcome
                .events
                .iter()
                .filter_map(|event| event.downcast::<DamageEvent>())
                .collect::<Vec<_>>();
            assert_eq!(damage.len(), 1);
            assert_eq!(damage[0].source, source);
            if let Some(returned) = returned {
                assert_ne!(damage[0].source, returned);
                assert_eq!(game.current_controller(returned), Some(b));
            }
            if mode != 0 {
                let strict = effect
                    .clone()
                    .with_source_binding(ironsmith_core::DamageSourceSetBinding::LiveMembers);
                execute_effect(&mut game, &Effect::new(strict), &mut ctx).unwrap();
                assert_eq!(
                    game.damage_on(recipient),
                    7,
                    "live-member mode cannot use a missing or phased target"
                );
                game.turn_store.turn_history.clear_for_new_turn();
                let missing = execute_effect(&mut game, &Effect::new(effect), &mut ctx);
                assert!(
                    matches!(missing, Err(ExecutionError::UnresolvableValue(_))),
                    "missing LKI must not fall back to the earlier capture"
                );
                assert_eq!(game.damage_on(recipient), 7);
            }
        }
    }
}

#[cfg(test)]
mod zipped_recipient_tests {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::execute_effect;
    use crate::target::ObjectFilter;

    #[test]
    fn zipped_sources_make_one_self_assignment_each_and_skip_nonpositive_amounts() {
        for second_power in [-2, 0, 5] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let mut sources = Vec::new();
            for (name, power, controller) in [("First", 2, a), ("Second", second_power, b)] {
                sources.push(
                    game.create_object_from_definition(
                        &crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), name)
                            .card_types(vec![crate::types::CardType::Creature])
                            .power_toughness(crate::card::PowerToughness::fixed(power, 50))
                            .with_ability(crate::ability::Ability::static_ability(
                                crate::static_abilities::StaticAbility::lifelink(),
                            ))
                            .build(),
                        controller,
                        crate::zone::Zone::Battlefield,
                    ),
                );
            }
            let damage = DealDamageBySourcesEffect::new(
                vec![ChooseSpec::All(ObjectFilter::creature())],
                crate::Value::SourcePower,
                ChooseSpec::SpecificObject(ObjectId(9999)), // Unused in EachSource mode.
            )
            .with_recipient_binding(ironsmith_core::DamageRecipientSetBinding::EachSource);
            assert!(damage.get_target_spec().is_none());
            assert!(damage.get_target_count().is_none());
            let outcome = execute_effect(
                &mut game,
                &Effect::new(damage),
                &mut ExecutionContext::new_default(sources[0], a),
            )
            .unwrap();
            assert_eq!(game.damage_on(sources[0]), 2);
            assert_eq!(game.damage_on(sources[1]), second_power.max(0) as u32);
            assert_eq!(game.player(a).unwrap().life, 22);
            assert_eq!(game.player(b).unwrap().life, 20 + second_power.max(0));
            let events = outcome
                .events
                .iter()
                .filter(|event| event.downcast::<DamageEvent>().is_some())
                .collect::<Vec<_>>();
            assert_eq!(events.len(), if second_power > 0 { 2 } else { 1 });
            assert!(events[0].simultaneous_batch().is_some());
            for event in &events {
                let damage = event.downcast::<DamageEvent>().unwrap();
                assert_eq!(damage.target, DamageTarget::Object(damage.source));
                assert_eq!(event.simultaneous_batch(), events[0].simultaneous_batch());
            }
            if events.len() == 2 {
                assert_ne!(events[0].provenance(), events[1].provenance());
            }
        }
    }

    #[test]
    fn empty_zipped_set_needs_no_source_or_recipient() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let damage = DealDamageBySourcesEffect::new(
            vec![ChooseSpec::All(ObjectFilter::creature())],
            crate::Value::SourcePower,
            ChooseSpec::Source,
        )
        .with_recipient_binding(ironsmith_core::DamageRecipientSetBinding::EachSource);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(damage),
            &mut ExecutionContext::new_default(ObjectId(9999), PlayerId::from_index(0)),
        )
        .unwrap();
        assert!(outcome.events.is_empty());
    }
}
