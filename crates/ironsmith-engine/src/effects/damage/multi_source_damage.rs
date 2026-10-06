//! Simultaneous damage from independently evaluated, exactly bound sources.
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::damage::{checked_damage_amount, checked_damage_count};
use crate::events::processing::{
    SimultaneousDamageEvent, prepare_simultaneous_damage_assignments_with_scopes,
};
use crate::events::{DamageEvent, DamageTarget, Event, LifeGainEvent};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::DealDamageBySourcesEffect;

impl EffectExecutor for DealDamageBySourcesEffect {
    fn supports_damage_action_cohort(&self) -> bool {
        true
    }
    fn shares_iterated_damage_action(&self) -> bool {
        true
    }
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }
    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(super::deal_damage::prepare_damage_instruction(self.clone()))
    }
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| self.execute_bound_with_outputs(game, ctx),
        )
    }
}

impl super::deal_damage::DamageInstructionInputProvider for DealDamageBySourcesEffect {
    fn capture_damage_instruction(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<super::deal_damage::CapturedDamageInstructionPlan, ExecutionError> {
        self.resolve_damage_instruction_inputs(game, ctx)
            .map(|plan| plan.capture(ctx))
    }
}
trait ExecuteCapturedDamage {
    fn resolve_damage_instruction_inputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<super::deal_damage::DamageInstructionPlan, ExecutionError>;
    fn execute_bound_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError>;
}
impl ExecuteCapturedDamage for DealDamageBySourcesEffect {
    fn execute_bound_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.resolve_damage_instruction_inputs(game, ctx)?
            .execute_with_outputs(game, ctx)
    }
    fn resolve_damage_instruction_inputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<super::deal_damage::DamageInstructionPlan, ExecutionError> {
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
            return Ok(super::deal_damage::DamageInstructionPlan::Finished(
                if self.target.is_target() {
                    EffectOutcome::target_invalid()
                } else {
                    EffectOutcome::count(0)
                },
            ));
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
        Ok(super::deal_damage::DamageInstructionPlan::from_events(
            events,
        ))
    }
}

/// Replacement additions retain their participant's execution scope too.
struct AssignmentDamagePayload {
    assignment: usize,
    outputs: crate::effects::CompletedEffectOutputs,
}

struct ScopedDamagePrograms {
    assignments: Vec<usize>,
    context: std::sync::Arc<crate::effects::ExecutionContextCheckpoint>,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
}

impl ScopedDamagePrograms {
    /// Complete in the captured participant scope and retain the owner of each
    /// added-program receipt before the enclosing action projects its outcome.
    fn complete(
        self,
        game: &mut GameState,
        dm: &mut dyn crate::decision::DecisionMaker,
        original: EffectOutcome,
    ) -> Result<(EffectOutcome, Vec<AssignmentDamagePayload>), ExecutionError> {
        let mut participant = self.context.reborrow(dm);
        let completed = super::deal_damage::complete_damage_replacement_programs(
            game,
            &mut participant,
            original,
            self.programs,
        )?;
        if participant.decision_maker.awaiting_choice() {
            return Ok((EffectOutcome::count(0), Vec::new()));
        }
        let (original, additions) = completed.into_outputs();
        if additions.len() != self.assignments.len() {
            return Err(ExecutionError::InternalError(
                "damage additions lost assignment ownership".into(),
            ));
        }
        let additions = self
            .assignments
            .into_iter()
            .zip(additions)
            .map(|(assignment, outputs)| AssignmentDamagePayload {
                assignment,
                outputs,
            })
            .collect();
        Ok((original, additions))
    }
}

/// One assignment retains the complete execution scope of its authored
/// participant. Assignments from that participant share immutable context data.
#[derive(Clone)]
pub(super) struct CapturedDamageInput {
    event: SimultaneousDamageEvent,
    context: std::sync::Arc<crate::effects::ExecutionContextCheckpoint>,
}

impl CapturedDamageInput {
    pub(super) fn participant_scope(&self) -> crate::effects::EffectOutcomeScope {
        crate::effects::EffectOutcomeScope(self.context.clone())
    }
}

impl std::fmt::Debug for CapturedDamageInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapturedDamageInput")
            .field("event", &self.event)
            .finish_non_exhaustive()
    }
}

pub(super) fn capture_damage_inputs(
    events: Vec<SimultaneousDamageEvent>,
    ctx: &ExecutionContext,
) -> Vec<CapturedDamageInput> {
    let context = std::sync::Arc::new(crate::effects::ExecutionContextCheckpoint::capture(ctx));
    events
        .into_iter()
        .map(|event| CapturedDamageInput {
            event,
            context: context.clone(),
        })
        .collect()
}

#[derive(Clone)]
struct SourceState {
    context: std::sync::Arc<crate::effects::ExecutionContextCheckpoint>,
    cause: crate::events::cause::EventCause,
    source: ObjectId,
    controller: PlayerId,
    snapshot: Option<ObjectSnapshot>,
    keywords: crate::rules::damage::SourceDamageKeywords,
}
impl SourceState {
    fn execution_context<'a>(
        &self,
        observation: crate::provenance::ProvNodeId,
        dm: &'a mut dyn crate::decision::DecisionMaker,
    ) -> ExecutionContext<'a> {
        let mut ctx = self.context.reborrow(dm);
        ctx.cause = self.cause.clone();
        ctx.source = self.source;
        ctx.controller = self.controller;
        ctx.source_snapshot = self.snapshot.clone();
        ctx.provenance = observation;
        ctx
    }
}
/// Every original damage assignment, consequence and observer belongs to this
/// one occurrence. Replacement-added instructions run only after all originals.
pub(super) fn commit_damage_batch_with_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    events: Vec<SimultaneousDamageEvent>,
    batch: Option<crate::provenance::ProvNodeId>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let prepared = prepare_damage_batch(game, parent, events, batch)?;
    complete_prepared_damage_batch_with_outputs(game, parent, prepared)
}

pub(super) fn commit_captured_damage_batch_with_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    inputs: Vec<CapturedDamageInput>,
    batch: Option<crate::provenance::ProvNodeId>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let mut scopes = Vec::<crate::effects::EffectOutcomeScope>::new();
    for input in &inputs {
        let scope = input.participant_scope();
        if !scopes
            .iter()
            .any(|existing| existing.same_instruction(&scope))
        {
            scopes.push(scope);
        }
    }
    let prepared = prepare_captured_damage_batch(game, parent, inputs, batch)?;
    let outputs = complete_prepared_damage_batch_with_outputs(game, parent, prepared)?;
    if !parent.decision_maker.awaiting_choice() {
        for scope in scopes {
            outputs.participant_output(&scope)?;
        }
    }
    Ok(outputs)
}

fn complete_prepared_damage_batch_with_outputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    prepared: Option<PreparedDamageBatch>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    match prepared {
        Some(mut prepared) => {
            // Ordinary damage already has a full-operation deferral owner.
            // Return preparation's suffix before committing originals so that
            // nested scopes and all later follow-ups retain their exact order.
            if game
                .effect_store
                .prevention_effects
                .follow_ups_are_deferred()
            {
                if let Some(assignments) = prepared.assignment_follow_ups.take() {
                    for follow_ups in assignments {
                        follow_ups.requeue(game);
                    }
                }
                if let Some(follow_ups) = prepared.follow_ups.take() {
                    follow_ups.requeue(game);
                }
            }
            prepared.complete_with_outputs(game, parent)
        }
        None => Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        )),
    }
}

/// Captured context identity names an authored damage instruction. Splits
/// and redirections retain that identity; separately captured instructions do
/// not merge merely because their source, target or visible bindings match.
struct DamageParticipants {
    contexts: Vec<std::sync::Arc<crate::effects::ExecutionContextCheckpoint>>,
    assignments: Vec<Vec<usize>>,
    owners: Vec<usize>,
}
impl DamageParticipants {
    fn from_sources(states: &[SourceState]) -> Self {
        let mut participants = Self {
            contexts: Vec::new(),
            assignments: Vec::new(),
            owners: Vec::with_capacity(states.len()),
        };
        for (index, state) in states.iter().enumerate() {
            let owner = participants
                .contexts
                .iter()
                .position(|context| std::sync::Arc::ptr_eq(context, &state.context))
                .unwrap_or_else(|| {
                    participants.contexts.push(state.context.clone());
                    participants.assignments.push(Vec::new());
                    participants.contexts.len() - 1
                });
            participants.assignments[owner].push(index);
            participants.owners.push(owner);
        }
        participants
    }
    fn owner(&self, assignment: usize) -> Result<usize, ExecutionError> {
        self.owners.get(assignment).copied().ok_or_else(|| {
            ExecutionError::InternalError("damage output lost its authored participant".into())
        })
    }
    /// A shared source receipt can become participant-owned only when all its
    /// contributing assignments belong to that one authored instruction.
    fn sole_contributor(
        &self,
        contributions: &[(usize, u32)],
    ) -> Result<Option<usize>, ExecutionError> {
        let Some((first, _)) = contributions.first() else {
            return Err(ExecutionError::InternalError(
                "damage shared output lost its contributions".into(),
            ));
        };
        let first = self.owner(*first)?;
        let mut sole = true;
        for (assignment, _) in contributions {
            sole &= self.owner(*assignment)? == first;
        }
        Ok(sole.then_some(first))
    }
}

type DamageParticipantResult = (
    std::sync::Arc<crate::effects::ExecutionContextCheckpoint>, crate::effects::CompletedEffectOutputs,
);

struct DamageAssignmentOriginal {
    recipient_before: Option<crate::effect::DamageRecipientBefore>,
    total: i64,
    prevented: bool,
    replaced: bool,
}

/// Dense authored results and the owner of every actual damage report. Event
/// metadata belongs to the whole occurrence and survives subset projection.
struct DamageOriginalProjection {
    assignments: Vec<DamageAssignmentOriginal>,
    reported: Vec<TriggerEvent>,
    report_assignments: Vec<usize>,
}
impl DamageOriginalProjection {
    fn validate(&self, assignment_count: usize) -> Result<(), ExecutionError> {
        if self.assignments.len() != assignment_count
            || self.reported.len() != self.report_assignments.len()
            || self
                .report_assignments
                .iter()
                .any(|index| *index >= assignment_count)
        {
            return Err(ExecutionError::InternalError(
                "damage original lost assignment ownership".into(),
            ));
        }
        Ok(())
    }

    /// One projection owner serves the whole instruction and participant
    /// subsets. Counts/statuses are authored results; report occurrence totals
    /// and immutable event observations are never recalculated for a subset.
    fn project(
        &self,
        game: &GameState,
        selected: &[usize],
    ) -> Result<EffectOutcome, ExecutionError> {
        self.validate(self.assignments.len())?;
        let mut included = vec![false; self.assignments.len()];
        let mut total = 0u128;
        let mut any_prevented = false;
        let mut any_replaced = false;
        let mut recipients = Vec::new();
        for index in selected {
            let Some(included) = included.get_mut(*index) else {
                return Err(ExecutionError::InternalError(
                    "damage projection selected an unknown assignment".into(),
                ));
            };
            if *included {
                return Err(ExecutionError::InternalError(
                    "damage projection selected an assignment twice".into(),
                ));
            }
            *included = true;
            let assignment = &self.assignments[*index];
            total += assignment.total as u128;
            any_prevented |= assignment.prevented;
            any_replaced |= assignment.replaced;
            if let Some(recipient) = &assignment.recipient_before
                && !recipients.contains(recipient)
            {
                recipients.push(recipient.clone());
            }
        }
        let total = checked_damage_count(total, "simultaneous damage total")?;
        let mut outcome = if total == 0 && any_replaced {
            EffectOutcome::replaced()
        } else if total == 0 && any_prevented {
            EffectOutcome::prevented()
        } else {
            EffectOutcome::count(total)
        };
        for recipient in recipients {
            outcome = outcome.with_execution_fact(ExecutionFact::DamageRecipientBefore(recipient));
        }
        let reports = self
            .reported
            .iter()
            .zip(&self.report_assignments)
            .filter(|(_, index)| included[**index]);
        let affected = reports
            .clone()
            .filter_map(|(event, _)| event.downcast::<DamageEvent>())
            .filter_map(|event| match event.target {
                DamageTarget::Object(id) => Some(id),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !affected.is_empty() {
            outcome = outcome.with_affected_objects_from_game(game, affected);
        }
        for (event, _) in reports {
            if let Some(damage) = event.downcast::<DamageEvent>()
                && damage.excess_damage > 0
            {
                outcome = outcome
                    .with_execution_fact(ExecutionFact::ExcessDamageDealt)
                    .with_execution_fact(ExecutionFact::ExcessDamage(damage.excess_damage));
            }
            outcome = outcome.with_event(event.clone());
        }
        Ok(outcome)
    }
}

pub(super) struct PreparedDamageBatch {
    assignment_follow_ups: Option<Vec<crate::events::processing::CapturedPreventionFollowUps>>,
    follow_ups: Option<crate::events::processing::CapturedPreventionFollowUps>,
    originals: PreparedDamageOriginals,
    projection: DamageOriginalProjection,
    payloads: Vec<AssignmentDamagePayload>,
    programs: Vec<ScopedDamagePrograms>,
}

/// A lifelink receipt belongs to one source's whole simultaneous damage
/// event. Retain every contribution without duplicating that life-gain event.
enum DamageConsequenceOrigin {
    Assignment(usize),
    Lifelink(Vec<(usize, u32)>),
}
impl DamageConsequenceOrigin {
    fn validate(&self, assignment_count: usize) -> Result<(), ExecutionError> {
        let valid = match self {
            Self::Assignment(index) => *index < assignment_count,
            Self::Lifelink(contributions) => {
                !contributions.is_empty()
                    && contributions
                        .iter()
                        .all(|(index, _)| *index < assignment_count)
            }
        };
        if !valid {
            return Err(ExecutionError::InternalError(
                "damage consequence lost assignment ownership".into(),
            ));
        }
        Ok(())
    }
}
struct OwnedDamageConsequence {
    origin: DamageConsequenceOrigin,
    receipt: crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
}
struct CompletedDamageConsequence {
    origin: DamageConsequenceOrigin,
    outputs: crate::effects::CompletedEffectOutputs,
}

struct PreparedLifelinkConsequence {
    context_assignment: usize,
    contributions: Vec<(usize, u32)>,
    observation: crate::provenance::ProvNodeId,
    proposal: crate::events::processing::TraitEventResult,
}

struct PreparedDamageOriginals {
    states: Vec<SourceState>,
    prepared: Vec<(
        usize,
        crate::provenance::ProvNodeId,
        crate::rules::damage::PreparedDamageAssignment,
    )>,
    life_prepared: Vec<PreparedLifelinkConsequence>,
    batch: Option<crate::provenance::ProvNodeId>,
}

impl PreparedDamageOriginals {
    fn commit(
        self,
        game: &mut GameState,
        parent: &mut ExecutionContext,
    ) -> Result<Vec<OwnedDamageConsequence>, ExecutionError> {
        let Self {
            states,
            prepared,
            life_prepared,
            batch,
        } = self;
        // Validate ownership before the first original mutates the game.
        for (index, _, _) in &prepared {
            DamageConsequenceOrigin::Assignment(*index).validate(states.len())?;
        }
        for life in &life_prepared {
            DamageConsequenceOrigin::Assignment(life.context_assignment).validate(states.len())?;
            if life.contributions.is_empty()
                || life.contributions.iter().any(|(index, _)| {
                    *index >= states.len()
                        || states[*index].source != states[life.context_assignment].source
                })
            {
                return Err(ExecutionError::InternalError(
                    "lifelink consequence lost its source contributions".into(),
                ));
            }
        }
        crate::effects::composition::with_held_original_triggers(game, |game| {
            let mut receipts = Vec::new();
            let mut lost = 0u128;
            for (index, observation, plan) in prepared {
                let state = &states[index];
                let mut ctx = state.execution_context(observation, &mut *parent.decision_maker);
                let receipt =
                    crate::rules::damage::commit_prepared_damage_original_with_outputs(game, &mut ctx, plan)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(Vec::new());
                }
                lost += u128::from(receipt.original.life_lost);
                let mut receipt = crate::effects::SimultaneousEffectCommit {
                    outcome: receipt
                        .original
                        .consequence_outcome
                        .unwrap_or_else(|| crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved())),
                    completion: receipt.completion,
                };
                bind_consequence_batch(&mut receipt.outcome.outcome, batch);
                receipts.push(OwnedDamageConsequence {
                    origin: DamageConsequenceOrigin::Assignment(index),
                    receipt: crate::effects::composition::with_original_execution_context(
                        receipt, &ctx,
                    ),
                });
            }
            checked_damage_count(lost, "simultaneous damage-result life loss")?;
            let mut gained = 0u128;
            for PreparedLifelinkConsequence {
                context_assignment,
                contributions,
                observation,
                proposal,
            } in life_prepared
            {
                let state = &states[context_assignment];
                let mut ctx = state.execution_context(observation, &mut *parent.decision_maker);
                let mut receipt = crate::effects::life::life_change::commit_prepared_life_original_with_outputs(
                    game, &mut ctx, proposal,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(Vec::new());
                }
                gained += receipt
                    .outcome
                    .outcome
                    .events
                    .iter()
                    .filter_map(|event| event.downcast::<LifeGainEvent>())
                    .map(|event| u128::from(event.amount))
                    .sum::<u128>();
                bind_consequence_batch(&mut receipt.outcome.outcome, batch);
                receipts.push(OwnedDamageConsequence {
                    origin: DamageConsequenceOrigin::Lifelink(contributions),
                    receipt: crate::effects::composition::with_original_execution_context(
                        receipt, &ctx,
                    ),
                });
            }
            checked_damage_count(gained, "simultaneous lifelink life gain")?;
            Ok(receipts)
        })
    }
}

impl PreparedDamageBatch {
    fn complete_with_outputs(
        self,
        game: &mut GameState,
        parent: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let mut completed =
            crate::effects::composition::execute_simultaneous_originals_with_outputs(
                game,
                parent,
                false,
                |game, parent| Ok(vec![self.commit_original(game, parent)?]),
                |_, _, _| {
                    Ok(crate::effects::composition::OriginalTriggerObservation::OwnerPublished)
                },
            )?;
        if parent.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        completed.pop().ok_or_else(|| {
            ExecutionError::InternalError("damage coordinator lost its original receipt".into())
        })
    }

    pub(super) fn commit_original(
        self,
        game: &mut GameState,
        parent: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        let Self {
            assignment_follow_ups,
            follow_ups,
            originals,
            projection,
            payloads,
            programs,
        } = self;
        let assignment_count = originals.states.len();
        let participants = DamageParticipants::from_sources(&originals.states);
        projection.validate(assignment_count)?;
        if assignment_follow_ups
            .as_ref()
            .is_some_and(|slots| slots.len() != assignment_count)
            || payloads
                .iter()
                .any(|payload| payload.assignment >= assignment_count)
            || programs.iter().any(|group| {
                group.assignments.len() != group.programs.len()
                    || group
                        .assignments
                        .iter()
                        .any(|index| *index >= assignment_count)
            })
        {
            return Err(ExecutionError::InternalError(
                "prepared damage lost replacement assignment ownership".into(),
            ));
        }
        let receipts = originals.commit(game, parent)?;
        if parent.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                EffectOutcome::count(0),
            ));
        }
        let primary = projection.project(game, &(0..assignment_count).collect::<Vec<_>>())?;
        let participant_primaries = participants
            .assignments
            .iter()
            .map(|assignments| projection.project(game, assignments))
            .collect::<Result<Vec<_>, _>>()?;
        let outcome = EffectOutcome::aggregate_replacement_outcomes(
            primary.clone(),
            payloads
                .iter()
                .map(|payload| payload.outputs.outcome.clone())
                .chain(receipts.iter().map(|owned| owned.receipt.outcome.outcome.clone())),
        );
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome,
            completion: Some(Box::new(DamageBatchCompletion {
                assignment_count,
                participants,
                participant_primaries,
                primary,
                receipts,
                payloads,
                programs,
                follow_ups,
                assignment_follow_ups,
            })),
        })
    }
}

struct DamageBatchCompletion {
    assignment_count: usize,
    participants: DamageParticipants,
    participant_primaries: Vec<EffectOutcome>,
    assignment_follow_ups: Option<Vec<crate::events::processing::CapturedPreventionFollowUps>>,
    follow_ups: Option<crate::events::processing::CapturedPreventionFollowUps>,
    primary: EffectOutcome,
    receipts: Vec<OwnedDamageConsequence>,
    payloads: Vec<AssignmentDamagePayload>,
    programs: Vec<ScopedDamagePrograms>,
}

impl crate::effects::SimultaneousEffectCompletion for DamageBatchCompletion {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        for owned in &mut self.receipts {
            let receipt = &mut owned.receipt;
            crate::effects::outcome_recording::complete_outcome(
                game,
                None,
                None,
                &mut receipt.outcome.outcome,
                Vec::new(),
            );
            if let Some(completion) = &mut receipt.completion {
                completion.freeze(game)?;
            }
        }
        Ok(())
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        for owned in &mut self.receipts {
            let receipt = &mut owned.receipt;
            crate::effects::composition::inherit_original_observations(
                &mut receipt.outcome.outcome,
                &original.events,
            );
            if let Some(completion) = &mut receipt.completion {
                completion.observe_original(game, ctx, &mut receipt.outcome.outcome)?;
            }
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        crate::effects::runtime::capture_triggers_before_added_program(
            game,
            ctx,
            None,
            self.receipts
                .iter_mut()
                .flat_map(|owned| owned.receipt.outcome.outcome.events.iter_mut())
                .chain(
                    self.payloads
                        .iter_mut()
                        .flat_map(|payload| payload.outputs.outcome.events.iter_mut()),
                ),
        )?;
        for owned in &self.receipts {
            let receipt = &owned.receipt;
            crate::effects::composition::inherit_original_observations(
                original,
                &receipt.outcome.outcome.events,
            );
        }
        for payload in &self.payloads {
            crate::effects::composition::inherit_original_observations(
                original,
                &payload.outputs.outcome.events,
            );
        }
        Ok(())
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        match (*self).complete_owned(game, ctx, original)? {
            Some(completed) => completed.into_effect_outputs(),
            None => Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            )),
        }
    }
}

/// Shared outputs are published once, with their contribution ownership.
/// They cannot be copied into every participant's authored result.
enum SharedDamageOutput {
    Consequence(CompletedDamageConsequence),
    BatchPrevention(crate::effects::CompletedEffectOutputs),
}
impl SharedDamageOutput {
    fn synchronize_observations(&mut self, events: &[TriggerEvent]) {
        match self {
            Self::Consequence(completed) => {
                crate::effects::composition::inherit_original_observations(
                    &mut completed.outputs.outcome,
                    events,
                );
                completed.outputs.synchronize_observations();
            }
            Self::BatchPrevention(outputs) => {
                crate::effects::composition::inherit_original_observations(
                    &mut outputs.outcome,
                    events,
                );
                outputs.synchronize_observations();
            }
        }
    }
}

/// Complete receipts and the enclosing chronological projection are retained
/// together. The aggregate is authoritative for ordinary damage; cohort callers
/// can consume assignment results and shared outputs without executing again.
struct CompletedDamageBatch {
    contribution_contexts: Vec<std::sync::Arc<crate::effects::ExecutionContextCheckpoint>>,
    outcome: EffectOutcome,
    participants: Vec<DamageParticipantResult>,
    shared: Vec<SharedDamageOutput>,
}
impl CompletedDamageBatch {
    fn into_effect_outputs(self) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self {
            outcome,
            participants,
            shared,
            contribution_contexts,
        } = self;
        let participants = participants
            .into_iter()
            .map(|(context, outputs)| crate::effects::ScopedEffectOutcome {
                scope: crate::effects::EffectOutcomeScope(context),
                outputs,
            })
            .collect::<Vec<_>>();
        let shared = shared
            .into_iter()
            .map(|shared| match shared {
                SharedDamageOutput::Consequence(CompletedDamageConsequence { origin, outputs }) => {
                    let DamageConsequenceOrigin::Lifelink(contributions) = origin else {
                        return Err(ExecutionError::InternalError(
                            "assignment damage consequence reached shared output projection".into(),
                        ));
                    };
                    let contributions = contributions
                        .into_iter()
                        .map(|(index, amount)| {
                            let context =
                                contribution_contexts.get(index).cloned().ok_or_else(|| {
                                    ExecutionError::InternalError(
                                        "shared damage output lost a contributing context".into(),
                                    )
                                })?;
                            Ok(crate::effects::EffectOutcomeContribution {
                                scope: crate::effects::EffectOutcomeScope(context),
                                amount,
                            })
                        })
                        .collect::<Result<Vec<_>, ExecutionError>>()?;
                    Ok(crate::effects::SharedEffectOutcome {
                        ownership: crate::effects::SharedOutcomeOwnership::Contributions(
                            contributions,
                        ),
                        outputs,
                    })
                }
                SharedDamageOutput::BatchPrevention(outputs) => {
                    Ok(crate::effects::SharedEffectOutcome {
                        ownership: crate::effects::SharedOutcomeOwnership::Participants(
                            participants
                                .iter()
                                .map(|participant| participant.scope.clone())
                                .collect(),
                        ),
                        outputs,
                    })
                }
            })
            .collect::<Result<Vec<_>, ExecutionError>>()?;
        Ok(crate::effects::CompletedEffectOutputs {
            projections_complete: true,
            outcome,
            participants,
            shared,
        })
    }
}

impl DamageBatchCompletion {
    fn complete_owned(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<Option<CompletedDamageBatch>, ExecutionError> {
        let Self {
            assignment_count,
            participants,
            mut participant_primaries,
            mut primary,
            receipts,
            mut payloads,
            programs,
            follow_ups,
            assignment_follow_ups,
        } = self;
        crate::effects::composition::inherit_original_observations(&mut primary, &original.events);
        for payload in &mut payloads {
            crate::effects::composition::inherit_original_observations(
                &mut payload.outputs.outcome,
                &original.events,
            );
        }
        let mut completed_receipts = Vec::with_capacity(receipts.len());
        for OwnedDamageConsequence {
            origin,
            mut receipt,
        } in receipts
        {
            crate::effects::composition::inherit_original_observations(
                &mut receipt.outcome.outcome,
                &original.events,
            );
            let outputs = crate::effects::composition::complete_committed_original_with_outputs(game, ctx, receipt)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            completed_receipts.push(CompletedDamageConsequence { origin, outputs });
        }
        for consequence in &completed_receipts {
            consequence.origin.validate(assignment_count)?;
        }
        if participant_primaries.len() != participants.contexts.len() {
            return Err(ExecutionError::InternalError(
                "damage completion lost authored participant results".into(),
            ));
        }
        let mut participant_payloads = (0..participant_primaries.len())
            .map(|_| Vec::new())
            .collect::<Vec<Vec<crate::effects::CompletedEffectOutputs>>>();
        let mut shared = Vec::new();
        let mut outcome = EffectOutcome::aggregate_replacement_outcomes(
            primary,
            payloads
                .iter()
                .map(|payload| payload.outputs.outcome.clone())
                .chain(
                    completed_receipts
                        .iter()
                        .map(|owned| owned.outputs.outcome.clone()),
                ),
        );
        for payload in payloads {
            participant_payloads[participants.owner(payload.assignment)?].push(payload.outputs);
        }
        for consequence in completed_receipts {
            let owner = match &consequence.origin {
                DamageConsequenceOrigin::Assignment(index) => Some(participants.owner(*index)?),
                DamageConsequenceOrigin::Lifelink(contributions) => {
                    participants.sole_contributor(contributions)?
                }
            };
            if let Some(owner) = owner {
                participant_payloads[owner].push(consequence.outputs);
            } else {
                shared.push(SharedDamageOutput::Consequence(consequence));
            }
        }
        for program_group in programs {
            let (original, additions) =
                program_group.complete(game, ctx.decision_maker, outcome)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            outcome = EffectOutcome::aggregate_replacement_outcomes(
                original,
                additions
                    .iter()
                    .map(|payload| payload.outputs.outcome.clone()),
            );
            for addition in additions {
                participant_payloads[participants.owner(addition.assignment)?]
                    .push(addition.outputs);
            }
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        for (index, follow_ups) in assignment_follow_ups.into_iter().flatten().enumerate() {
            let completed = follow_ups.complete_with_outputs(game, ctx.decision_maker)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            outcome = EffectOutcome::aggregate_replacement_outcomes(
                outcome,
                completed.iter().map(|outputs| outputs.outcome.clone()),
            );
            participant_payloads[participants.owner(index)?].extend(completed);
        }
        if let Some(follow_ups) = follow_ups {
            let completed = follow_ups.complete_with_outputs(game, ctx.decision_maker)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            outcome = EffectOutcome::aggregate_replacement_outcomes(
                outcome,
                completed.iter().map(|outputs| outputs.outcome.clone()),
            );
            if participant_payloads.len() == 1 {
                participant_payloads[0].extend(completed);
            } else {
                shared.extend(
                    completed
                        .into_iter()
                        .map(SharedDamageOutput::BatchPrevention),
                );
            }
        }
        // Later programme observation can annotate earlier events in the
        // aggregate. Transfer those annotations to every retained receipt too.
        for primary in &mut participant_primaries {
            crate::effects::composition::inherit_original_observations(primary, &outcome.events);
        }
        for payload in participant_payloads.iter_mut().flatten() {
            crate::effects::composition::inherit_original_observations(
                &mut payload.outcome,
                &outcome.events,
            );
            payload.synchronize_observations();
        }
        for output in &mut shared {
            output.synchronize_observations(&outcome.events);
        }
        let contribution_contexts = participants
            .owners
            .iter()
            .map(|owner| participants.contexts[*owner].clone())
            .collect();
        let participants = participants
            .contexts
            .into_iter()
            .zip(participant_primaries.into_iter().zip(participant_payloads))
            .map(|(context, (primary, payloads))| {
                let aggregate = EffectOutcome::aggregate_replacement_outcomes(
                    primary,
                    payloads.iter().map(|outputs| outputs.outcome.clone()),
                );
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::resolved(),
                );
                outputs.retain_batch_children(payloads);
                (context, outputs.project_aggregate(aggregate))
            })
            .collect();
        Ok(Some(CompletedDamageBatch {
            contribution_contexts,
            outcome,
            participants,
            shared,
        }))
    }
}
/// Prepare replacements and retain prevention follow-ups without executing
/// them. The full captured parent context survives the decision-maker reborrow.
pub(super) fn prepare_damage_batch(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    events: Vec<SimultaneousDamageEvent>,
    batch: Option<crate::provenance::ProvNodeId>,
) -> Result<Option<PreparedDamageBatch>, ExecutionError> {
    let inputs = capture_damage_inputs(events, parent);
    prepare_captured_damage_batch(game, parent, inputs, batch)
}

/// Prepared participants can contribute their own complete context without
/// reconstructing damage or consequence semantics in the enclosing adapter.
pub(super) fn prepare_captured_damage_batch(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    inputs: Vec<CapturedDamageInput>,
    batch: Option<crate::provenance::ProvNodeId>,
) -> Result<Option<PreparedDamageBatch>, ExecutionError> {
    let scopes = inputs
        .iter()
        .map(CapturedDamageInput::participant_scope)
        .collect();
    let captured = crate::effects::ExecutionContextCheckpoint::capture(parent);
    let source = parent.source;
    let controller = parent.controller;
    let mut completed_context = None;
    let result = crate::events::processing::capture_deferred_prevention_follow_ups(
        game,
        parent.decision_maker,
        |game, dm| {
            let mut ctx = ExecutionContext::new(source, controller, dm);
            captured.restore_ref(&mut ctx);
            let result = prepare_damage_batch_inputs(game, &mut ctx, inputs, batch);
            completed_context = Some(crate::effects::ExecutionContextCheckpoint::capture(&ctx));
            result
        },
    );
    if let Some(completed) = completed_context {
        completed.restore(parent);
    }
    let (prepared, follow_ups) = result?;
    Ok(prepared.map(|mut prepared| {
        prepared.follow_ups = Some(follow_ups.with_participants(scopes));
        prepared
    }))
}

fn prepare_damage_batch_inputs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    inputs: Vec<CapturedDamageInput>,
    mut batch: Option<crate::provenance::ProvNodeId>,
) -> Result<Option<PreparedDamageBatch>, ExecutionError> {
    let (events, contexts): (Vec<_>, Vec<_>) = inputs
        .into_iter()
        .map(|input| (input.event, input.context))
        .unzip();
    let replacement_scopes = contexts
        .iter()
        .map(|context| context.replacement_scope())
        .collect::<Vec<_>>();
    // Capture the authored recipient, not a replacement redirect's new
    // destination. These scalars precede every original damage consequence.
    game.refresh_continuous_state()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let mut original_assignments = Vec::with_capacity(events.len());
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
        original_assignments.push(DamageAssignmentOriginal {
            recipient_before: receipt,
            total: 0,
            prevented: false,
            replaced: false,
        });
    }
    let (processed, assignment_follow_ups) = prepare_simultaneous_damage_assignments_with_scopes(
        game,
        &events,
        parent.decision_maker,
        &replacement_scopes,
    )?
    .into_parts();
    if parent.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    if processed.len() != events.len() {
        return Err(ExecutionError::InternalError(
            "simultaneous damage lost an original assignment".into(),
        ));
    }
    if assignment_follow_ups.len() != contexts.len() {
        return Err(ExecutionError::InternalError(
            "simultaneous damage lost an assignment prevention scope".into(),
        ));
    }
    let assignment_follow_ups = assignment_follow_ups
        .into_iter()
        .zip(&contexts)
        .map(|(follow_ups, context)| {
            follow_ups.with_participants(vec![crate::effects::EffectOutcomeScope(context.clone())])
        })
        .collect();
    let states = events
        .iter()
        .enumerate()
        .map(|(index, event)| {
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
                .unwrap_or(contexts[index].controller());
            let keywords =
                crate::rules::damage::source_damage_keywords(game, event.source, snapshot.as_ref());
            SourceState {
                context: contexts[index].clone(),
                cause: event.cause.clone(),
                source: event.source,
                controller,
                snapshot,
                keywords,
            }
        })
        .collect::<Vec<_>>();
    for (original, result) in original_assignments.iter_mut().zip(&processed) {
        original.prevented = result.replacement_prevented;
        original.replaced = result.payload_outcome.is_some();
    }
    let mut prepared = Vec::new();
    let mut reported = Vec::new();
    let mut report_assignments = Vec::new();
    let mut programs = Vec::<ScopedDamagePrograms>::new();
    let mut payloads = Vec::new();
    let mut lifelink = Vec::<(usize, u32, Vec<(usize, u32)>)>::new();
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
    let provenance = parent.provenance;
    for (index, result) in processed.into_iter().enumerate() {
        if !result.programs.is_empty() {
            if let Some(last) = programs.last_mut()
                && std::sync::Arc::ptr_eq(&last.context, &contexts[index])
            {
                last.assignments
                    .extend(std::iter::repeat_n(index, result.programs.len()));
                last.programs.extend(result.programs);
            } else {
                programs.push(ScopedDamagePrograms {
                    assignments: vec![index; result.programs.len()],
                    context: contexts[index].clone(),
                    programs: result.programs,
                });
            }
        }
        if let Some(mut outcome) = result.payload_outcome {
            for event in &mut outcome.events {
                if let Some(batch) = batch {
                    *event = event.clone().with_simultaneous_batch(batch);
                }
            }
            payloads.push(AssignmentDamagePayload {
                assignment: index,
                outputs: crate::effects::CompletedEffectOutputs::aggregate_only(outcome),
            });
        }
        for assignment in result.assignments {
            let observation = game.alloc_child_event_provenance(
                states[index].context.provenance(),
                crate::events::EventKind::Damage,
            );
            let state = &states[index];
            let mut ctx = state.execution_context(observation, &mut *parent.decision_maker);
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
                return Ok(None);
            }
            if !plan.applied {
                continue;
            }
            total =
                checked_damage_count(total as u128 + amount as u128, "simultaneous damage total")?;
            original_assignments[index].total = checked_damage_count(
                original_assignments[index].total as u128 + amount as u128,
                "damage assignment total",
            )?;
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
                state.cause.clone(),
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
            report_assignments.push(index);
            if state.keywords.has_lifelink {
                if let Some((_, sum, contributions)) = lifelink
                    .iter_mut()
                    .find(|(existing, _, _)| states[*existing].source == state.source)
                {
                    *sum = checked_damage_amount(
                        u128::from(*sum) + u128::from(assignment.amount),
                        "lifelink damage total",
                    )?;
                    contributions.push((index, assignment.amount));
                } else {
                    lifelink.push((index, assignment.amount, vec![(index, assignment.amount)]));
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
            for event in &mut outcome.outputs.outcome.events {
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
    for (index, amount, contributions) in lifelink {
        let observation = game.alloc_child_event_provenance(
            states[index].context.provenance(),
            crate::events::EventKind::LifeGain,
        );
        let state = &states[index];
        let mut ctx = state.execution_context(observation, &mut *parent.decision_maker);
        let proposal = crate::effects::life::life_change::prepare_life_change(
            game,
            &mut ctx,
            Event::new_with_provenance(
                LifeGainEvent::new(state.controller, amount).with_source(state.source),
                observation,
            ),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        life_prepared.push(PreparedLifelinkConsequence {
            context_assignment: index,
            contributions,
            observation,
            proposal,
        });
    }
    Ok(Some(PreparedDamageBatch {
        assignment_follow_ups: Some(assignment_follow_ups),
        follow_ups: None,
        originals: PreparedDamageOriginals {
            states,
            prepared,
            life_prepared,
            batch,
        },
        projection: DamageOriginalProjection {
            assignments: original_assignments,
            reported,
            report_assignments,
        },
        payloads,
        programs,
    }))
}

fn bind_consequence_batch(
    outcome: &mut EffectOutcome,
    batch: Option<crate::provenance::ProvNodeId>,
) {
    if let Some(batch) = batch {
        for event in &mut outcome.events {
            *event = event.clone().with_simultaneous_batch(batch);
        }
    }
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
