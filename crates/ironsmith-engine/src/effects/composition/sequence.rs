//! Sequence effect implementation.
//!
//! Runs a list of effects in order and exposes the terminal outcome.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, rebase_target_scope};
use crate::game_state::GameState;

/// Failure policy belongs to the authored program, not the child action owner.
#[derive(Clone, Copy)]
enum ProgramFailurePolicy {
    Continue,
    Stop,
}

/// An ordered program retains its selected instruction and acknowledged child
/// packets. Its scope belongs to the caller; it never rebases targets or groups
/// unrelated programs merely because their instruction positions match.
pub(crate) struct OrderedProgramCursor<'effects> {
    effects: std::borrow::Cow<'effects, [Effect]>,
    next: usize,
    selected: Option<usize>,
    children: Vec<crate::effects::CompletedEffectOutputs>,
    failure_policy: ProgramFailurePolicy,
    skip_pending_entry: bool,
    stopped: bool,
    observe_replacements: bool,
}

impl std::fmt::Debug for OrderedProgramCursor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderedProgramCursor")
            .field("next", &self.next)
            .field("selected", &self.selected)
            .field("stopped", &self.stopped)
            .finish_non_exhaustive()
    }
}

impl<'effects> OrderedProgramCursor<'effects> {
    fn new(
        effects: std::borrow::Cow<'effects, [Effect]>,
        failure_policy: ProgramFailurePolicy,
        skip_pending_entry: bool,
    ) -> Self {
        Self {
            effects,
            next: 0,
            selected: None,
            children: Vec::new(),
            failure_policy,
            skip_pending_entry,
            stopped: false,
            observe_replacements: false,
        }
    }

    pub(crate) fn replacement(effects: std::borrow::Cow<'effects, [Effect]>) -> Self {
        let mut cursor = Self::new(effects, ProgramFailurePolicy::Continue, false);
        cursor.observe_replacements = true;
        cursor
    }

    fn select_effect(&mut self, ctx: &ExecutionContext) -> Result<Option<&Effect>, ExecutionError> {
        if self.selected.is_some() {
            return Err(ExecutionError::InternalError(
                "ordered program selected another instruction before acknowledgement".into(),
            ));
        }
        if self.stopped
            || ctx.resolution_stopped()
            || (self.skip_pending_entry && ctx.decision_maker.awaiting_choice())
        {
            return Ok(None);
        }
        let Some(effect) = self.effects.get(self.next) else {
            return Ok(None);
        };
        self.selected = Some(self.next);
        self.next += 1;
        Ok(Some(effect))
    }

    fn into_children(self) -> Vec<crate::effects::CompletedEffectOutputs> {
        self.children
    }

    fn completed(self, pending: bool) -> crate::effects::CompletedEffectOutputs {
        let mut outputs = if self.observe_replacements {
            let children = self.children;
            let aggregate =
                EffectOutcome::aggregate(children.iter().map(|outputs| outputs.outcome.clone()));
            let mut outputs =
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved());
            for child in children {
                outputs.retain_owned_child(child);
            }
            outputs.project_aggregate(aggregate)
        } else {
            crate::effects::CompletedEffectOutputs::from_children(
                self.children,
                EffectOutcome::aggregate,
            )
        };
        if pending {
            outputs.projections_complete = false;
        }
        outputs
    }
}

impl crate::effects::ActionProgramCursor for OrderedProgramCursor<'_> {
    fn next_action(
        &mut self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<crate::effects::ProgramAction>, ExecutionError> {
        // Only the owned declaration crosses a staged boundary. Synchronous
        // execution below borrows the actual definition instead of cloning it.
        // Empty identity explicitly provides no shared-action grouping proof.
        Ok(self
            .select_effect(ctx)?
            .cloned()
            .map(crate::effects::ProgramAction::new))
    }

    fn select_execution_instruction(
        &mut self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::ProgramInstructionSelection<'_>, ExecutionError> {
        let dispatch_while_pending = !self.skip_pending_entry;
        let instruction = self.select_effect(ctx)?;
        Ok(crate::effects::ProgramInstructionSelection::borrowed(
            instruction,
            dispatch_while_pending,
        ))
    }
    fn accept_action(
        &mut self,
        outputs: crate::effects::CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        self.selected.take().ok_or_else(|| {
            ExecutionError::InternalError(
                "ordered program acknowledged an instruction it did not select".into(),
            )
        })?;
        let failed = outputs.outcome.status.is_failure();
        self.children.push(outputs);
        if failed && matches!(self.failure_policy, ProgramFailurePolicy::Stop) {
            self.stopped = true;
        }
        Ok(())
    }
    fn accept_action_with_context(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        outputs: crate::effects::CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        self.accept_action(outputs)?;
        // Qualification still peeks at the next authored definition after a
        // failed child. Pending input skips it; future operands stay unevaluated.
        if self.observe_replacements && !ctx.decision_maker.awaiting_choice() {
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                ctx,
                self.effects.get(self.next),
                self.children
                    .iter_mut()
                    .flat_map(|outputs| outputs.outcome.events.iter_mut()),
            )?;
        }
        Ok(())
    }

    fn ends_action_unit(&self) -> bool {
        true
    }

    fn finish(self: Box<Self>) -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        if self.selected.is_some() || (!self.stopped && self.next < self.effects.len()) {
            return Err(ExecutionError::InternalError(
                "ordered program finished before its selected instructions completed".into(),
            ));
        }
        Ok(crate::effects::ProgramCompletion::new(
            (*self).completed(false),
        ))
    }

    fn finish_pending(
        self: Box<Self>,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        Ok((*self).completed(true))
    }

    fn finish_stopped(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        Ok(crate::effects::ProgramCompletion::new(
            (*self).completed(false),
        ))
    }
}

/// Own the ordered child execution once. Callers retain their target/context,
/// aggregate and transaction contracts; each child keeps its action identity.
fn execute_program_children_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    failure_policy: ProgramFailurePolicy,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    execute_ordered_program_cursor_with_outputs(
        game,
        ctx,
        OrderedProgramCursor::new(std::borrow::Cow::Borrowed(effects), failure_policy, true),
        purpose,
    )
}

/// Replacement programs retain their existing pending-entry dispatch contract.
/// Their caller owns per-event observation mode and the final result projection.
pub(crate) fn execute_observed_replacement_cursor_with_outputs<'cursor>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    cursor: Box<dyn super::ActionProgramCursor + 'cursor>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    super::action_program::execute_action_program_with_outputs(
        cursor,
        game,
        ctx,
        crate::effects::EffectExecutionPurpose::Action,
    )
}

fn execute_ordered_program_cursor_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut cursor: OrderedProgramCursor,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    // Ordinary Sequence callers own their aggregate and pending-prefix policy.
    // Take actual acknowledged children without invoking a different finalizer.
    super::action_program::run_program_cursor(&mut cursor, game, ctx, purpose)?;
    Ok(cursor.into_children())
}

/// Execute an ordinary ordered program in its caller-owned scope. Authored
/// failures do not stop subsequent instructions; a pending choice always does.
/// The enclosing compound owns rollback and the aggregate result contract.
pub(crate) fn execute_ordered_children_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    execute_ordered_children_for_purpose(
        game,
        ctx,
        effects,
        crate::effects::EffectExecutionPurpose::Action,
    )
}

/// Share ordered execution while the caller explicitly chooses payment semantics.
pub(super) fn execute_ordered_children_for_purpose(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    execute_program_children_with_outputs(
        game,
        ctx,
        effects,
        ProgramFailurePolicy::Continue,
        purpose,
    )
}

/// Run a checked program in its caller-owned target/context scope. Payment
/// and failure programs stop after the first failed or pending child; ordinary
/// authored SequenceEffect retains its separate target and result contracts.
pub(crate) fn execute_checked_program_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    execute_checked_program_for_purpose(
        game,
        ctx,
        effects,
        crate::effects::EffectExecutionPurpose::Action,
    )
}

pub(super) fn execute_checked_payment_program_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    execute_checked_program_for_purpose(
        game,
        ctx,
        effects,
        crate::effects::EffectExecutionPurpose::Payment,
    )
}

fn execute_checked_program_for_purpose(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let children = execute_program_children_with_outputs(
        game,
        ctx,
        effects,
        ProgramFailurePolicy::Stop,
        purpose,
    )?;
    Ok(crate::effects::CompletedEffectOutputs::from_children(
        children,
        EffectOutcome::aggregate,
    ))
}

/// Effect that executes multiple effects in sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceEffect {
    /// Effects to execute in order.
    pub effects: Vec<Effect>,
    /// Whether these effects were printed as one coordinated Oracle clause.
    pub surface: ironsmith_core::SequenceSurface,
    /// Optional authored label on a numeric result-table row.
    pub result_label: Option<String>,
}

impl SequenceEffect {
    /// Create a new SequenceEffect.
    pub fn new(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::Sequential,
            result_label: None,
        }
    }

    pub fn sentence_leading_then(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::SentenceLeadingThen,
            result_label: None,
        }
    }

    pub fn comma_then(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::CommaThen,
            result_label: None,
        }
    }

    pub fn repeated_comma_then(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::RepeatedCommaThen,
            result_label: None,
        }
    }

    pub fn coordinated(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::Coordinated,
            result_label: None,
        }
    }

    pub fn coordinated_with_leading_duration(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::CoordinatedLeadingDuration,
            result_label: None,
        }
    }

    pub fn result_conjunction(effects: Vec<Effect>, leading_duration: bool) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::ResultConjunction { leading_duration },
            result_label: None,
        }
    }

    pub fn result_labeled(effects: Vec<Effect>, label: impl Into<String>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::Sequential,
            result_label: Some(label.into()),
        }
    }
}

impl EffectExecutor for SequenceEffect {
    fn supports_replacement_draw_continuation(&self) -> bool {
        self.effects
            .iter()
            .all(crate::effects::replacement::replacement_effect_supported)
    }
    fn prepare_replacement_draw_continuation_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let cursor = self.select_prepared_action_program(game, ctx)?;
        super::object_iteration::prepare_iteration_continuation(cursor, game, ctx)
    }

    fn supports_prepared_action_program(&self) -> bool {
        self.effects
            .iter()
            .all(super::action_program::action_program_child_is_prepared)
    }
    fn select_prepared_action_program(
        &self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        Ok(Some(sequence_cursor(self, ctx)))
    }

    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        let mut bindings = crate::effects::CostChoiceBindings::default();
        for effect in &self.effects {
            bindings.append(effect.0.cost_choice_bindings());
        }
        bindings
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        self.effects
            .iter()
            .all(|effect| effect.0.as_cost_executable().is_some())
            .then_some(self as &dyn CostExecutableEffect)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
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
        execute_sequence_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Action,
        )
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.effects])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.effects], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}

fn execute_sequence_with_outputs(
    sequence: &SequenceEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::tokens::execute_resource_transaction_with_pending_value(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            super::action_program::execute_action_program_with_outputs(
                sequence_cursor(sequence, ctx),
                game,
                ctx,
                purpose,
            )
        },
    )
}

struct SequenceCursor {
    effects: Vec<Effect>,
    coordinated: bool,
    next: usize,
    child_assignments: Option<Vec<crate::game_state::TargetAssignment>>,
    chosen_modes: Option<Vec<usize>>,
    consumed_modal_selection: bool,
    coordinated_target_state: crate::game_loop::CoordinatedTargetState,
    assignment_cursor: usize,
    active_scope: Option<(
        Vec<crate::effects::ResolvedTarget>,
        Vec<crate::game_state::TargetAssignment>,
    )>,
    outputs: crate::effects::CompletedEffectOutputs,
    outcomes: Vec<EffectOutcome>,
    events: Vec<crate::events::RawEvent>,
    facts: Vec<crate::effect::ExecutionFact>,
    unit_ends: Vec<usize>,
}
impl std::fmt::Debug for SequenceCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SequenceCursor")
            .field("next", &self.next)
            .finish_non_exhaustive()
    }
}
fn sequence_cursor(
    sequence: &SequenceEffect,
    ctx: &ExecutionContext,
) -> Box<dyn crate::effects::ActionProgramCursor> {
    let mut outputs =
        crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0));
    outputs.projections_complete = true;
    Box::new(SequenceCursor {
        effects: sequence.effects.clone(),
        coordinated: sequence.surface.is_coordinated(),
        next: 0,
        // Presentation-only one-child sequences inherit their caller's scope.
        child_assignments: (sequence.effects.len() > 1 && !ctx.target_assignments.is_empty())
            .then(|| ctx.target_assignments.clone()),
        chosen_modes: ctx.chosen_modes.clone(),
        consumed_modal_selection: false,
        coordinated_target_state: crate::game_loop::CoordinatedTargetState::default(),
        assignment_cursor: 0,
        active_scope: None,
        outputs,
        outcomes: Vec::new(),
        events: Vec::new(),
        facts: Vec::new(),
        unit_ends: super::action_units::partition_action_units(
            &sequence.effects,
            |_| None,
            |_, _| true,
        )
        .into_iter()
        .filter_map(|unit| unit.last().copied())
        .collect(),
    })
}
impl SequenceCursor {
    fn completed(mut self, pending: bool) -> crate::effects::CompletedEffectOutputs {
        let Some(_) = self.outcomes.last() else {
            return crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0));
        };
        if pending {
            self.outputs.projections_complete = false;
        }
        let mut outcome = EffectOutcome::aggregate_terminal(self.outcomes);
        outcome.events = self.events;
        outcome.execution_facts = self.facts;
        self.outputs.project_aggregate(outcome)
    }
}
impl crate::effects::ActionProgramCursor for SequenceCursor {
    fn finish_stopped(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        Ok(crate::effects::ProgramCompletion::new(
            (*self).completed(false),
        ))
    }
    fn next_action(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<crate::effects::ProgramAction>, ExecutionError> {
        if ctx.resolution_stopped() {
            return Ok(None);
        }
        let Some(effect) = self.effects.get(self.next) else {
            return Ok(None);
        };
        if self.next > 0 {
            crate::effects::match_triggers_at_instruction_boundary(
                game,
                ctx,
                Some(effect),
                self.events.iter(),
            )?;
        }
        let assignment_count = if self.child_assignments.is_some() {
            if self.coordinated {
                crate::game_loop::count_target_selection_slots_for_coordinated_child(
                    effect,
                    self.chosen_modes.as_deref(),
                    &mut self.consumed_modal_selection,
                    &mut self.coordinated_target_state,
                )
            } else {
                crate::game_loop::count_target_selection_slots_for_isolated_effect(
                    effect,
                    self.chosen_modes.as_deref(),
                    &mut self.consumed_modal_selection,
                )
            }
        } else {
            0
        };
        let assignments_exhausted = self
            .child_assignments
            .as_ref()
            .is_some_and(|assignments| self.assignment_cursor >= assignments.len())
            && self.active_scope.is_some();
        if assignment_count > 0 && !assignments_exhausted {
            let assignments = self
                .child_assignments
                .as_ref()
                .expect("child assignments checked above");
            let end = self
                .assignment_cursor
                .saturating_add(assignment_count)
                .min(assignments.len());
            let selected = assignments[self.assignment_cursor..end].to_vec();
            self.assignment_cursor = end;
            self.active_scope = Some(rebase_target_scope(&ctx.targets, &selected));
        }
        let index = self.next;
        self.next += 1;
        Ok(Some(crate::effects::ProgramAction {
            native: None,
            effect: effect.clone(),
            identity: vec![index],
            scope: crate::effects::ProgramActionScope {
                targets: self.active_scope.clone(),
                public_search_reveal_tag: Some(super::choose_objects_runtime::revealed_search_tag(
                    effect,
                    self.effects.get(index + 1),
                )),
                pending_entry_attachment: Some(
                    crate::effects::permanents::entry_attachment_for_move(
                        effect,
                        self.effects.get(index + 1),
                    ),
                ),
                ..Default::default()
            },
        }))
    }
    fn accept_action(
        &mut self,
        child: crate::effects::CompletedEffectOutputs,
    ) -> Result<(), ExecutionError> {
        let outcome = child.outcome.clone();
        let outputs = std::mem::replace(
            &mut self.outputs,
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        );
        self.outputs = outputs.append_owned_child(child);
        self.events.extend(outcome.events.clone());
        self.facts.extend(outcome.execution_facts.clone());
        self.outcomes.push(outcome);
        Ok(())
    }
    fn ends_action_unit(&self) -> bool {
        self.next
            .checked_sub(1)
            .is_some_and(|index| self.unit_ends.contains(&index))
    }
    fn continues_past_illegal_targets(&self) -> bool {
        true
    }
    fn finish(self: Box<Self>) -> Result<crate::effects::ProgramCompletion, ExecutionError> {
        Ok(crate::effects::ProgramCompletion::new(
            (*self).completed(false),
        ))
    }
    fn finish_pending(
        self: Box<Self>,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        Ok((*self).completed(true))
    }
}

impl CostExecutableEffect for SequenceEffect {
    fn execute_payment_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        execute_sequence_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Payment,
        )
    }

    fn payment_bindings_are_owned_by_children(&self) -> bool {
        true
    }

    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        crate::costs::check_effect_cost_program(&self.effects, game, ctx, reason)
    }

    fn canonical_cost_effect(&self) -> Option<crate::effect::Effect> {
        let effects = crate::effects::canonical_cost_children(&self.effects)?;
        let mut replacement = self.clone();
        replacement.effects = effects;
        Some(crate::effect::Effect::new(replacement))
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
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
    ) -> Result<(), CostValidationError> {
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        let mut execution =
            ExecutionContext::new(source, controller, &mut decision_maker).with_x(0);
        CostExecutableEffect::can_execute_as_cost_with_context(self, game, &mut execution, reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::BooleanContext;
    use crate::effect::{ChoiceCount, Until, Value};
    use crate::effects::ResolvedTarget;
    use crate::effects::continuous::RuntimeModification;
    use crate::game_state::TargetAssignment;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::effects::execute_effect;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[derive(Clone, Debug)]
    struct PendingChoiceEffect;

    impl EffectExecutor for PendingChoiceEffect {
        fn execute(
            &self,
            game: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            let prompt = BooleanContext::new(ctx.controller, Some(ctx.source), "pause");
            ctx.decision_maker.decide_boolean(game, &prompt);
            Ok(EffectOutcome::count(0))
        }
    }

    #[derive(Clone, Debug)]
    struct PreventedEffect;

    impl EffectExecutor for PreventedEffect {
        fn execute(
            &self,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            Ok(EffectOutcome::prevented())
        }
    }

    #[derive(Default)]
    struct CapturingDecisionMaker {
        pending: bool,
    }

    impl DecisionMaker for CapturingDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }

        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.pending = true;
            false
        }
    }

    #[test]
    fn sequence_forwards_inner_target_spec() {
        let effect = SequenceEffect::new(vec![
            Effect::gain_life(1),
            Effect::counter(ChooseSpec::target_spell()),
        ]);

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn sentence_leading_then_preserves_each_sequential_target_slot() {
        let effect = Effect::new(SequenceEffect::sentence_leading_then(vec![
            Effect::new(crate::effects::TargetOnlyEffect::explicit(
                ChooseSpec::target(ChooseSpec::Object(crate::filter::ObjectFilter::creature())),
            )),
            Effect::new(crate::effects::TargetOnlyEffect::explicit(
                ChooseSpec::target(ChooseSpec::Object(crate::filter::ObjectFilter::artifact())),
            )),
        ]));
        let mut consumed_modal_selection = false;

        assert_eq!(
            crate::game_loop::count_target_selection_slots_for_isolated_effect(
                &effect,
                None,
                &mut consumed_modal_selection,
            ),
            2
        );
    }

    #[test]
    fn sequence_exposes_terminal_summary_for_multiple_meaningful_results() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = SequenceEffect::new(vec![Effect::gain_life(1), Effect::gain_life(2)])
            .execute(&mut game, &mut ctx)
            .expect("sequence should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            result
                .events_of_type::<crate::events::LifeGainEvent>()
                .count(),
            2,
            "terminal summary selection must retain events from earlier steps"
        );
    }

    #[test]
    fn direct_and_dispatched_sequences_keep_all_original_object_results_and_all_replacement_actions() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        for dispatched in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let source = create_creature(&mut game, "Source", alice);
            let first = create_creature(&mut game, "First original", alice);
            let second = create_creature(&mut game, "Second original", alice);
            let added = create_creature(&mut game, "Replacement-only", alice);
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(crate::ObjectFilter::specific(first), Some(Zone::Battlefield), Some(Zone::Graveyard)),
                ReplacementAction::Additionally(vec![Effect::destroy(ChooseSpec::SpecificObject(added))])));
            let sequence = SequenceEffect::coordinated(vec![
                Effect::destroy(ChooseSpec::SpecificObject(first)),
                Effect::destroy(ChooseSpec::SpecificObject(second)),
            ]);
            let mut ctx = ExecutionContext::new_default(source, alice);
            let outcome = if dispatched { execute_effect(&mut game, &Effect::new(sequence), &mut ctx) }
                else { sequence.execute(&mut game, &mut ctx) }.unwrap();
            assert_eq!(outcome.instruction_result().count_or_zero(), 1, "terminal original summary is retained");
            let memory = outcome.affected_object_memory().unwrap();
            assert_eq!(memory.iter().map(|object| object.object_id).collect::<Vec<_>>(), vec![first, second]);
            assert!(game.object(first).is_none() && game.object(second).is_none() && game.object(added).is_none());
            let id = crate::effect::EffectId(17); ctx.effect_outcomes.insert(id, outcome);
            let quantity = Value::PriorEffectMetric { effect_id: id,
                query: ironsmith_core::PriorEffectMetricQuery::new(ironsmith_core::EffectMetricSource::AffectedObjects,
                    ironsmith_core::EffectMetric::TotalManaValue).with_filter(crate::ObjectFilter::creature())
                    .with_action(ironsmith_core::PriorEffectAction::Destroyed) };
            assert_eq!(crate::effects::helpers::resolve_value_wide(&game, &quantity, &ctx).unwrap(), 4);
        }
    }

    #[test]
    fn native_sequence_pending_and_resource_error_restore_prefix_receipts_before_replay() {
        for dispatched in [false, true] { for pending in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0); let source = create_creature(&mut game, "Source", alice);
            let suffix = if pending { Effect::new(PendingChoiceEffect) } else {
                Effect::new(crate::effects::CreateTokenEffect::you(crate::cards::tokens::treasure_token_definition(), 1))
            };
            if !pending { game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits { max_created_tokens: 0, ..Default::default() }); }
            let sequence = SequenceEffect::new(vec![Effect::with_id(11, Effect::gain_life(3)), suffix]);
            let mut dm = CapturingDecisionMaker::default();
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ctx.effect_outcomes.insert(crate::effect::EffectId(99), EffectOutcome::count(7));
            let before = game.next_object_id_counter(); let history = game.turn_store.turn_history.event_records.len();
            let result = if dispatched { execute_effect(&mut game, &Effect::new(sequence.clone()), &mut ctx) }
                else { sequence.execute(&mut game, &mut ctx) };
            if pending { assert!(result.is_ok() && ctx.decision_maker.awaiting_choice()); }
            else { assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded { .. })), "{result:?}"); }
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.next_object_id_counter(), before); assert_eq!(game.turn_store.turn_history.event_records.len(), history);
            assert_eq!(ctx.effect_outcomes.len(), 1); assert_eq!(ctx.effect_outcomes[&crate::effect::EffectId(99)].count_or_zero(), 7);
            assert!(!game.effect_store.has_pending_trigger_work());
            drop(ctx); game.set_token_creation_limits(Default::default());
            let mut replay = ExecutionContext::new_default(source, alice);
            sequence.execute(&mut game, &mut replay).unwrap();
            assert_eq!(game.player(alice).unwrap().life, 23, "prefix executes once after the suspended/failed attempt rolls back");
            assert_eq!(replay.effect_outcomes[&crate::effect::EffectId(11)].count_or_zero(), 3);
        }}
    }

    #[test]
    fn sequence_preserves_full_resolution_stop_with_terminal_original_receipts() {
        #[derive(Debug, Clone)] struct Stop;
        impl EffectExecutor for Stop {
            fn execute(&self, _: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
                ctx.stop_resolution(); Ok(EffectOutcome::count(7))
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Source", alice); let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = SequenceEffect::new(vec![Effect::gain_life(2), Effect::new(Stop), Effect::gain_life(9)])
            .execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.resolution_stopped()); assert_eq!(game.player(alice).unwrap().life, 22);
        assert_eq!(outcome.instruction_result().count_or_zero(), 7);
    }

    #[test]
    fn sequence_stops_before_later_effects_when_inner_effect_needs_choice() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let starting_life = game.player(alice).expect("Alice exists").life;
        let source = game.new_object_id();
        let mut dm = CapturingDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        SequenceEffect::new(vec![Effect::new(PendingChoiceEffect), Effect::gain_life(3)])
            .execute(&mut game, &mut ctx)
            .expect("sequence should surface the pending choice");

        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(
            game.player(alice).expect("Alice exists").life,
            starting_life,
            "later sequence effects must not run before the pending choice is answered"
        );
    }

    #[test]
    fn ordered_sequences_continue_after_a_prevented_child() {
        let alice = PlayerId::from_index(0);

        let mut coordinated_game = crate::tests::test_helpers::setup_two_player_game();
        let source = coordinated_game.new_object_id();
        let mut coordinated_ctx = ExecutionContext::new_default(source, alice);
        let coordinated =
            SequenceEffect::coordinated(vec![Effect::new(PreventedEffect), Effect::gain_life(3)]);
        let outcome = coordinated
            .execute(&mut coordinated_game, &mut coordinated_ctx)
            .expect("coordinated sequence should resolve");
        assert_eq!(
            coordinated_game.player(alice).expect("Alice").life,
            23,
            "a prevented sibling must not suppress an independent coordinated action"
        );
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);

        let mut sequential_game = crate::tests::test_helpers::setup_two_player_game();
        let source = sequential_game.new_object_id();
        let mut sequential_ctx = ExecutionContext::new_default(source, alice);
        let sequential =
            SequenceEffect::new(vec![Effect::new(PreventedEffect), Effect::gain_life(3)]);
        let outcome = sequential
            .execute(&mut sequential_game, &mut sequential_ctx)
            .expect("sequential sequence should resolve");
        assert_eq!(
            sequential_game.player(alice).expect("Alice").life,
            23,
            "instruction order does not require the preceding instruction to succeed"
        );
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn coordinated_runtime_effects_use_independent_equal_target_assignments() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Blue Dragon", alice);
        let first = create_creature(&mut game, "First Target", alice);
        let second = create_creature(&mut game, "Second Target", alice);
        let third = create_creature(&mut game, "Third Target", alice);
        let spec = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::up_to(1));
        let pump = |amount, tag: &'static str| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec.clone(),
                    RuntimeModification::ModifyPowerToughness {
                        power: Value::Fixed(amount),
                        toughness: Value::Fixed(0),
                    },
                    Until::YourNextTurn,
                )
                .require_creature_target(),
            )
            .tag(tag)
        };
        let sequence = SequenceEffect::coordinated(vec![
            pump(-3, "first"),
            pump(-2, "second"),
            pump(-1, "third"),
        ]);
        let mut consumed_modal_selection = false;
        assert_eq!(
            crate::game_loop::count_target_selection_slots_for_isolated_effect(
                &Effect::new(sequence.clone()),
                None,
                &mut consumed_modal_selection,
            ),
            3,
            "the runtime planner must retain all three target words"
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![
                ResolvedTarget::Object(first),
                ResolvedTarget::Object(second),
                ResolvedTarget::Object(third),
            ])
            .with_target_assignments(vec![
                TargetAssignment {
                    spec: spec.clone(),
                    range: 0..1,
                },
                TargetAssignment {
                    spec: spec.clone(),
                    range: 1..2,
                },
                TargetAssignment { spec, range: 2..3 },
            ]);

        sequence
            .execute(&mut game, &mut ctx)
            .expect("execute coordinated pumps");

        assert_eq!(game.calculated_power(first), Some(-1));
        assert_eq!(game.calculated_power(second), Some(0));
        assert_eq!(game.calculated_power(third), Some(1));
    }
}
