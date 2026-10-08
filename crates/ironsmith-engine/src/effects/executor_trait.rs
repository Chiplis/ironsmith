//! Effect executor trait for the modular effect system.
//!
//! This module defines the `EffectExecutor` trait that all effect implementations
//! must implement. Each effect type (damage, life, mana, etc.) implements this trait
//! with its own execution logic.

use std::any::Any;

use crate::costs::PaymentReason;
use crate::effect::{Effect, EffectMode, EffectOutcome, Value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::target::ChooseSpec;

/// Static choice-tag inputs and outputs of a cost instruction. These declarations
/// describe dependencies, not affordability or completed payment receipts.
#[derive(Debug, Clone, Default)]
pub struct CostChoiceBindings {
    pub required: Vec<crate::tag::TagKey>,
    pub published: Vec<crate::tag::TagKey>,
}

impl CostChoiceBindings {
    pub(crate) fn requiring(tag: crate::tag::TagKey) -> Self {
        Self {
            required: vec![tag],
            published: Vec::new(),
        }
    }

    pub(crate) fn from_filter(filter: &crate::target::ObjectFilter) -> Self {
        let Some(first) = filter.tagged_constraints.first() else {
            return Self::default();
        };
        if filter.tagged_constraints.iter().all(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == first.tag
        }) {
            Self::requiring(first.tag.clone())
        } else {
            Self::default()
        }
    }

    pub(crate) fn from_spec(spec: &ChooseSpec) -> Self {
        match spec.base() {
            ChooseSpec::Tagged(tag) => Self::requiring(tag.clone()),
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => Self::from_filter(filter),
            _ => Self::default(),
        }
    }

    /// Compose ordered declarations: only publications from preceding children
    /// satisfy a child's inputs. A later writer cannot erase an earlier need.
    pub(crate) fn append(&mut self, child: Self) {
        for tag in child.required {
            if !self.published.contains(&tag) && !self.required.contains(&tag) {
                self.required.push(tag);
            }
        }
        for tag in child.published {
            if !self.published.contains(&tag) {
                self.published.push(tag);
            }
        }
    }
}

/// Specification for a modal effect, used during spell casting per MTG rule 601.2b.
///
/// This contains the information needed to present mode choices to the player
/// during the casting process (before targets are chosen).
#[derive(Debug, Clone)]
pub struct ModalSpec {
    /// Descriptions of each available mode.
    pub mode_descriptions: Vec<String>,
    /// Maximum number of modes that can be chosen.
    pub max_modes: Value,
    /// Minimum number of modes that must be chosen.
    pub min_modes: Value,
    /// Whether the same mode can be chosen more than once.
    pub allow_repeated_modes: bool,
    /// Point costs for weighted modal choices. Unweighted modes use one point each.
    pub mode_point_costs: Vec<u32>,
    /// Whether the mode labels are mandatory casting-time additional costs
    /// (for example, Spree or Tiered).
    pub spree: bool,
    /// Additional mana cost associated with each mode.
    pub mode_additional_mana_costs: Vec<crate::mana::ManaCost>,
    /// Whether each selected mode must target a different player.
    pub distinct_player_targets_per_mode: bool,
    /// Alternate range enabled by a later optional-cost choice under CR 601.4.
    pub conditional_mode_range: Option<crate::effect::ConditionalModeRange>,
}

/// The supported runtime extension categories for effects.
///
/// These categories are intentionally broad. They describe the main execution
/// shape an effect participates in, which helps contributors choose the right
/// extension point when adding new runtime behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectExecutionCategory {
    /// A normal resolving effect that directly mutates game state and/or emits events.
    Standard,
    /// An effect that can legally participate in cost payment.
    CostExecutable,
    /// An effect whose primary purpose is to register delayed-trigger runtime state.
    DelayedTriggerRegistration,
    /// An effect whose primary purpose is to register replacement runtime state.
    ReplacementRegistration,
}

/// A selected input/output slot that a prepared program must retain even when
/// its value equals the enclosing participant's current binding. A value diff
/// alone cannot distinguish "written again" from "never produced".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedSelectionBinding {
    ObjectTag(crate::tag::TagKey),
    Outcome(crate::effect::EffectId),
}

/// Whether a target requirement can reuse an earlier compatible target slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetReusePolicy {
    /// Reuse a compatible target slot already declared by an earlier effect.
    ReuseCompatiblePrevious,
    /// Always declare a new target slot even if the spec matches an earlier one.
    AlwaysDeclareNew,
    /// Declare a synthetic target prelude now, but let exactly one later
    /// target-bearing effect consume that declaration.
    ///
    /// Lowering uses these preludes when Oracle names a target through a
    /// grammatical subject before the executable effect. The following effect
    /// may itself require a fresh tagged target in other contexts, so this
    /// one-shot bridge keeps the wrapper policy honest without duplicating the
    /// authored target.
    SyntheticPrelude,
}

/// Target selection metadata for a single effect.
#[derive(Debug, Clone, Copy)]
pub struct TargetSelectionProfile<'a> {
    pub spec: &'a ChooseSpec,
    /// Player assigned to make this target choice. `None` means the spell or
    /// ability controller uses the normal targeting flow.
    pub chooser: Option<&'a crate::target::PlayerFilter>,
    pub description: &'static str,
    pub min_targets: usize,
    pub max_targets: Option<usize>,
    pub count_value: Option<&'a crate::effect::Value>,
    /// Amount that must be divided among the selected targets during announcement.
    pub distribution_value: Option<&'a crate::effect::Value>,
    /// Minimum amount that must be assigned to each selected target.
    pub distribution_min_per_target: u32,
    pub reuse_policy: TargetReusePolicy,
}

/// Modal effect metadata used by target-selection planning.
#[derive(Debug, Clone, Copy)]
pub struct ModalEffectSpec<'a> {
    pub modes: &'a [EffectMode],
    pub max_modes: &'a Value,
    pub min_modes: &'a Value,
    pub allow_repeated_modes: bool,
    pub mode_point_costs: &'a [u32],
    pub spree: bool,
    pub mode_additional_mana_costs: &'a [crate::mana::ManaCost],
    pub disallow_previously_chosen_modes: bool,
    pub disallow_previously_chosen_modes_this_turn: bool,
    pub distinct_player_targets_per_mode: bool,
    pub conditional_mode_range: Option<&'a crate::effect::ConditionalModeRange>,
}

/// Trait for executing effects.
///
/// All modular effects implement this trait. Each effect is responsible for:
/// - Resolving any dynamic values (X, counts, etc.)
/// - Validating targets (if applicable)
/// - Mutating game state appropriately
/// - Returning an appropriate `EffectOutcome` (result + events)
///
/// # Example
///
/// ```ignore
/// use ironsmith::effects::EffectExecutor;
///
/// impl EffectExecutor for MyEffect {
///     fn execute(
///         &self,
///         game: &mut GameState,
///         ctx: &mut ExecutionContext,
///     ) -> Result<EffectOutcome, ExecutionError> {
///         // Implementation here
///         Ok(EffectOutcome::resolved())
///     }
/// }
/// ```
pub trait EffectExecutorClone {
    /// Clone this effect into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn EffectExecutor>;
}

impl<T> EffectExecutorClone for T
where
    T: EffectExecutor + Clone + 'static,
{
    fn clone_boxed(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

/// Opaque identity and captured context for one authored instruction.
/// Cloning a scope retains that identity; independently captured instructions
/// remain distinct even when their visible inputs happen to be equal.
#[derive(Clone)]
pub struct EffectOutcomeScope(
    pub(crate) std::sync::Arc<crate::effects::ExecutionContextCheckpoint>,
);
impl std::fmt::Debug for EffectOutcomeScope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EffectOutcomeScope")
            .finish_non_exhaustive()
    }
}
impl EffectOutcomeScope {
    pub fn same_instruction(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

/// One authored participant's complete result in its captured execution scope.
/// This is a projection of the aggregate history, not another physical action.
pub struct ScopedEffectOutcome {
    pub scope: EffectOutcomeScope,
    /// Nested results remain owned by this authored participant.
    pub outputs: CompletedEffectOutputs,
}

/// A contributing quantity retains its owner's scope. Its unit is defined by
/// the semantic action owner (for example actual damage contributing lifelink).
#[derive(Clone)]
pub struct EffectOutcomeContribution {
    pub scope: EffectOutcomeScope,
    pub amount: u32,
}
#[derive(Clone)]
pub enum SharedOutcomeOwnership {
    /// The child owner already published its chronological observations.
    /// Retain this packet as an alternative view, never another parent history.
    Published,
    /// Retained child packet without a declared participant association.
    Batch,
    /// One shared output observed by the named authored participants.
    Participants(Vec<EffectOutcomeScope>),
    Contributions(Vec<EffectOutcomeContribution>),
}

/// A single shared action result must be published once, rather than copied
/// into every contributing participant's result.
pub struct SharedEffectOutcome {
    pub ownership: SharedOutcomeOwnership,
    /// The shared action owns its aggregate and any nested projections. Those
    /// children are alternatives within this receipt, never extra peer outputs.
    pub outputs: SharedEffectOutputView,
}

/// Immutable reference to one already-published completion. Cloning this
/// handle preserves its actual packet and authored scope identities; it cannot
/// execute an action or publish another chronological history.
#[derive(Clone)]
pub struct PublishedEffectOutputs(std::sync::Arc<CompletedEffectOutputs>);

impl std::fmt::Debug for PublishedEffectOutputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublishedEffectOutputs")
            .field("participants", &self.0.participants.len())
            .field("shared", &self.0.shared.len())
            .finish_non_exhaustive()
    }
}

impl PublishedEffectOutputs {
    pub(crate) fn retain(outputs: CompletedEffectOutputs) -> Self {
        Self(std::sync::Arc::new(outputs))
    }

    pub(crate) fn same_completion(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }

    /// Metadata aliases retain one producer, never one packet per copy.
    pub(crate) fn append_distinct(
        retained: &mut Vec<Self>,
        incoming: impl IntoIterator<Item = Self>,
    ) {
        for owner in incoming {
            if !retained.iter().any(|prior| prior.same_completion(&owner)) {
                retained.push(owner);
            }
        }
    }
}

/// A shared child's mutable routing projection. An already-published child
/// also retains its immutable producer packet; parent observation annotations
/// affect only this view, never the producer or another parent's view.
pub struct SharedEffectOutputView {
    outputs: CompletedEffectOutputs,
    published_owner: Option<PublishedEffectOutputs>,
}

impl From<CompletedEffectOutputs> for SharedEffectOutputView {
    fn from(outputs: CompletedEffectOutputs) -> Self {
        Self {
            outputs,
            published_owner: None,
        }
    }
}

impl std::ops::Deref for SharedEffectOutputView {
    type Target = CompletedEffectOutputs;
    fn deref(&self) -> &Self::Target {
        &self.outputs
    }
}

impl std::ops::DerefMut for SharedEffectOutputView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.outputs
    }
}

impl SharedEffectOutputView {
    fn from_published(owner: PublishedEffectOutputs) -> Self {
        Self {
            outputs: owner.0.clone_projection(),
            published_owner: Some(owner),
        }
    }

    fn clone_projection(&self) -> Self {
        Self {
            outputs: self.outputs.clone_projection(),
            published_owner: self.published_owner.clone(),
        }
    }
}

/// Completion keeps its authoritative chronological aggregate alongside any
/// owner-supplied participant/shared projections. Projections are alternatives
/// for routing and binding; concatenating them with the aggregate duplicates
/// event history. Empty projections mean the owner exposes only its aggregate.
pub struct CompletedEffectOutputs {
    /// True only when the owner accounts for every child through participant
    /// or shared projections. Aggregate-only children make group coverage partial.
    pub projections_complete: bool,
    pub outcome: EffectOutcome,
    pub participants: Vec<ScopedEffectOutcome>,
    pub shared: Vec<SharedEffectOutcome>,
}
impl std::fmt::Debug for CompletedEffectOutputs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompletedEffectOutputs")
            .field("projections_complete", &self.projections_complete)
            .field("outcome", &self.outcome)
            .field("participant_count", &self.participants.len())
            .field("shared_count", &self.shared.len())
            .finish()
    }
}

impl CompletedEffectOutputs {
    /// Copy an alternative routing view of existing completed data. Scope
    /// Arcs and event identities are retained; no original, continuation or
    /// publisher is cloned, and this operation never appends event history.
    pub(crate) fn clone_projection(&self) -> Self {
        Self {
            projections_complete: self.projections_complete,
            outcome: self.outcome.clone(),
            participants: self
                .participants
                .iter()
                .map(|participant| ScopedEffectOutcome {
                    scope: participant.scope.clone(),
                    outputs: participant.outputs.clone_projection(),
                })
                .collect(),
            shared: self
                .shared
                .iter()
                .map(|shared| SharedEffectOutcome {
                    ownership: shared.ownership.clone(),
                    outputs: shared.outputs.clone_projection(),
                })
                .collect(),
        }
    }

    /// Resolve a view supplied by this owner, without falling back to its
    /// collective aggregate or borrowing a nested instruction's result.
    pub(crate) fn participant_output(
        &self,
        scope: &EffectOutcomeScope,
    ) -> Result<&CompletedEffectOutputs, ExecutionError> {
        let mut matches = self
            .participants
            .iter()
            .filter(|participant| participant.scope.same_instruction(scope));
        let participant = matches.next().ok_or_else(|| {
            ExecutionError::InternalError("completed action lost its authored participant".into())
        })?;
        if matches.next().is_some() {
            return Err(ExecutionError::InternalError(
                "completed action has ambiguous authored participant outputs".into(),
            ));
        }
        Ok(&participant.outputs)
    }

    /// Bind one authored result with its declared shared observations. This
    /// view is an alternative to the owner's history, never another action.
    /// Opaque retained children have no inferred participant association.
    pub(crate) fn participant_view(
        &self,
        scope: &EffectOutcomeScope,
    ) -> Result<EffectOutcome, ExecutionError> {
        let primary = self.participant_output(scope)?.outcome.clone();
        let related = self.shared.iter().filter(|shared| match &shared.ownership {
            SharedOutcomeOwnership::Batch | SharedOutcomeOwnership::Published => false,
            SharedOutcomeOwnership::Participants(scopes) => {
                scopes.iter().any(|related| related.same_instruction(scope))
            }
            SharedOutcomeOwnership::Contributions(contributions) => contributions
                .iter()
                .any(|contribution| contribution.scope.same_instruction(scope)),
        });
        Ok(EffectOutcome::aggregate_replacement_outcomes(
            primary,
            related.map(|shared| shared.outputs.outcome.clone()),
        ))
    }

    /// The parent has already included this child's aggregate exactly once.
    /// Retain its complete packet as an alternative view under its own owner,
    /// without adding chronological history or inferring parent participants.
    pub(crate) fn append_owned_child(mut self, child: Self) -> Self {
        self.retain_owned_child(child);
        self
    }

    /// Borrowing form for context-restoring loops and closures. The caller
    /// owns aggregate projection; the actual child retains its own routing.
    pub(crate) fn retain_owned_child(&mut self, child: Self) {
        self.projections_complete &= child.projections_complete;
        self.retain_batch_children([child]);
    }

    /// Preserve retained child ownership while an enclosing composition owner
    /// applies its aggregate result projection and observation annotations.
    pub(crate) fn project_aggregate(mut self, outcome: EffectOutcome) -> Self {
        self.outcome = outcome;
        self.synchronize_observations();
        self
    }

    /// Retain a cost/root-action packet whose observations are already owned by
    /// its publisher. Do not concatenate its events into the parent's history.
    pub(crate) fn retain_published_children(&mut self, children: impl IntoIterator<Item = Self>) {
        self.retain_published_references(children.into_iter().map(PublishedEffectOutputs::retain));
    }

    /// Metadata and prospective copies may share producer handles. This parent
    /// retains those exact published packets with its own annotation view;
    /// their observations are never another part of the parent's history.
    pub(crate) fn retain_published_references(
        &mut self,
        children: impl IntoIterator<Item = PublishedEffectOutputs>,
    ) {
        for owner in children {
            if self.shared.iter().any(|shared| {
                matches!(shared.ownership, SharedOutcomeOwnership::Published)
                    && shared
                        .outputs
                        .published_owner
                        .as_ref()
                        .is_some_and(|prior| prior.same_completion(&owner))
            }) {
                continue;
            }
            self.projections_complete &= owner.0.projections_complete;
            self.shared.push(SharedEffectOutcome {
                ownership: SharedOutcomeOwnership::Published,
                outputs: SharedEffectOutputView::from_published(owner),
            });
        }
    }

    /// Retain each additional programme's result exactly once with batch
    /// ownership, and propagate its observation annotations into prior outputs.
    pub(crate) fn append_batch_program_outputs(
        mut self,
        completed: crate::effects::replacement::CompletedReplacementPrograms,
    ) -> Self {
        let (original, additions) = completed.into_outputs();
        self.outcome = original;
        self.append_replacement_outputs(additions)
    }

    /// Retain completed replacement packets as alternate routing views of the
    /// same observations, preserving the enclosing authored instruction result.
    pub(crate) fn append_replacement_outputs(
        mut self,
        replacements: impl IntoIterator<Item = Self>,
    ) -> Self {
        let replacements = replacements.into_iter().collect::<Vec<_>>();
        self.outcome = EffectOutcome::aggregate_replacement_outcomes(
            self.outcome,
            replacements.iter().map(|outputs| outputs.outcome.clone()),
        );
        self.retain_batch_children(replacements);
        self
    }

    /// The aggregate already includes this child once. Keep its routing
    /// alternatives inside one owned receipt rather than flattening them.
    pub(crate) fn retain_batch_children(&mut self, children: impl IntoIterator<Item = Self>) {
        self.shared
            .extend(children.into_iter().map(|outputs| SharedEffectOutcome {
                ownership: SharedOutcomeOwnership::Batch,
                outputs: outputs.into(),
            }));
        self.synchronize_observations();
    }

    /// Result projection includes each child once; routing alternatives retain
    /// the actual packets without claiming coverage for the enclosing primary.
    pub(crate) fn with_primary_result(
        primary: EffectOutcome,
        children: impl IntoIterator<Item = Self>,
    ) -> Self {
        Self::from_children(children, |outcomes| {
            EffectOutcome::aggregate_with_primary_result(primary, outcomes)
        })
    }

    /// The caller owns its aggregate contract; actual children remain alternate
    /// views of the same history, never additional chronological observations.
    pub(crate) fn from_children(
        children: impl IntoIterator<Item = Self>,
        project: impl FnOnce(Vec<EffectOutcome>) -> EffectOutcome,
    ) -> Self {
        let children: Vec<_> = children.into_iter().collect();
        let aggregate = project(children.iter().map(|child| child.outcome.clone()).collect());
        let mut outputs = Self::aggregate_only(aggregate);
        outputs.retain_batch_children(children);
        outputs
    }

    pub(crate) fn append_batch_completion_outputs(mut self, completion: Self) -> Self {
        self.outcome = EffectOutcome::aggregate_with_primary_result(
            self.outcome,
            [completion.outcome.clone()],
        );
        self.retain_batch_children([completion]);
        self
    }
    pub(crate) fn synchronize_observations(&mut self) {
        // Composition loops may collect projections before projecting their
        // aggregate. With no observed events there is nothing to inherit;
        // avoid rescanning every prior receipt on each iteration.
        if self.outcome.events.is_empty() {
            return;
        }
        for participant in &mut self.participants {
            crate::effects::composition::inherit_original_observations(
                &mut participant.outputs.outcome,
                &self.outcome.events,
            );
            participant.outputs.synchronize_observations();
        }
        for shared in &mut self.shared {
            crate::effects::composition::inherit_original_observations(
                &mut shared.outputs.outcome,
                &self.outcome.events,
            );
            shared.outputs.synchronize_observations();
        }
    }
    pub fn aggregate_only(outcome: EffectOutcome) -> Self {
        Self {
            projections_complete: false,
            outcome,
            participants: Vec::new(),
            shared: Vec::new(),
        }
    }
    pub fn into_outcome(self) -> EffectOutcome {
        self.outcome
    }
}

/// An authored view of shared damage, with receipts owned by its enclosing
/// branch. The view never owns the physical damage history. Selection preludes
/// and completion acknowledgements transfer to the cohort exactly once.
pub struct DamageActionBinding {
    pub outcome: EffectOutcome,
    pub(crate) preludes: Vec<CompletedEffectOutputs>,
    pub(crate) completion_facts: Vec<crate::effect::ExecutionFact>,
}

impl DamageActionBinding {
    pub fn from_outcome(outcome: EffectOutcome) -> Self {
        Self {
            outcome,
            preludes: Vec::new(),
            completion_facts: Vec::new(),
        }
    }

    pub(crate) fn project(
        mut self,
        project: impl FnOnce(EffectOutcome) -> Result<EffectOutcome, ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        self.outcome = project(self.outcome)?;
        Ok(self)
    }

    pub(crate) fn from_bindings(
        bindings: Vec<Self>,
        project: impl FnOnce(Vec<EffectOutcome>) -> EffectOutcome,
    ) -> Self {
        let mut preludes = Vec::new();
        let mut completion_facts = Vec::new();
        let outcomes = bindings
            .into_iter()
            .map(|binding| {
                preludes.extend(binding.preludes);
                completion_facts.extend(binding.completion_facts);
                binding.outcome
            })
            .collect();
        Self {
            outcome: project(outcomes),
            preludes,
            completion_facts,
        }
    }

    /// Preserve the damage owner's direct participant routing. Prelude packets
    /// are retained as owned alternatives after their already-observed history
    /// is prefixed once; no selection or original observation runs again.
    pub(crate) fn transfer_owned_outputs(
        self,
        owner: &mut CompletedEffectOutputs,
    ) -> EffectOutcome {
        if !self.preludes.is_empty() {
            let observations = EffectOutcome::aggregate(
                self.preludes
                    .iter()
                    .map(|outputs| outputs.outcome.clone())
                    .chain(std::iter::once(owner.outcome.clone())),
            );
            owner.outcome = owner
                .outcome
                .clone()
                .with_authoritative_observations(observations);
            owner.retain_batch_children(self.preludes);
            // Coverage for the newly composed instruction has not been proven.
            owner.projections_complete = false;
        }
        owner.outcome.execution_facts.extend(self.completion_facts);
        self.outcome
    }
}

/// Whether an owner exposes its original work separately from additions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalPhaseStatus {
    /// The existing completion still combines both phases. No readiness proof.
    Combined,
    /// The owner can finish and retain its original work without its additions.
    Retained,
    /// The original phase is complete; this owner contains only additions.
    Complete,
}

/// Completion programs are frozen only after every original proposal commits,
/// then executed after the simultaneous action has closed. The owner preserves
/// each participant's execution context and rolls back the whole instruction
/// if completion pauses for a decision or fails.
pub trait SimultaneousEffectCompletion: Send {
    /// Unmigrated owners retain their combined completion contract explicitly.
    fn original_phase_status(&self) -> OriginalPhaseStatus {
        OriginalPhaseStatus::Combined
    }

    /// Finish original work and return the actual packet with its still-retained
    /// additions. Only owners advertising Retained are dispatched here.
    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        _original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        Err(ExecutionError::InternalError(
            "retained original phase has no completion owner".into(),
        ))
    }

    /// Consume the actual original packet at an original-only phase boundary.
    /// The compatibility default preserves the scalar callback and retains the
    /// incoming packet once after successful completion. An overriding owner
    /// must preserve or project its actual children without republishing them.
    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let mut completed =
            self.complete_original_phase_with_outputs(game, ctx, original.outcome.clone())?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        completed.outcome.retain_owned_child(original);
        Ok(completed)
    }

    /// Run only the non-draw prefix, retaining the owner's actual draw and tail.
    /// Existing completion owners without a draw boundary finish normally.
    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(SimultaneousEffectCommit::finished)
    }

    /// Transfer the actual prefix packet through its owner's draw boundary.
    /// The compatibility default retains the incoming packet once after the
    /// existing scalar callback. Pending/stop policy stays with native callers;
    /// this entry point adds no new suspension gate or draw advancement.
    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let mut prepared =
            self.prepare_draw_boundary_with_outputs(game, ctx, original.outcome.clone())?;
        prepared.outcome.retain_owned_child(original);
        Ok(prepared)
    }

    fn prepare_draw_boundary(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.prepare_draw_boundary_with_outputs(game, ctx, original)
            .map(SimultaneousEffectCommit::into_aggregate)
    }

    /// Observe frozen originals before any participant executes additions.
    /// Scoped and compound completions preserve this phase for their children.
    /// This phase must not execute a deferred program or a physical action.
    fn observe_original(
        &mut self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        _original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        Ok(())
    }

    /// Retain authored projections when the action owner supplies them.
    /// Existing completion owners expose their aggregate without inventing
    /// participant ownership. Decorators must forward this contract explicitly
    /// when their child exposes projections.
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.complete(game, ctx, original)
            .map(CompletedEffectOutputs::aggregate_only)
    }

    /// Consume the actual original packet during full completion. The default
    /// retains the existing scalar callback's output and the incoming packet
    /// exactly once. Native composition owners may consume that packet directly
    /// to preserve child ownership through their own result projection.
    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let mut outputs = self.complete_with_outputs(game, ctx, original.outcome.clone())?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        outputs.retain_owned_child(original);
        Ok(outputs)
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError>;
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError>;
}

/// Original receipts may carry scalar compatibility outcomes or retained
/// outputs. Both expose the same authoritative aggregate to phase owners.
pub trait OriginalEffectOutput {
    /// Construct an explicit aggregate-only fallback, without claiming child coverage.
    fn from_aggregate(outcome: EffectOutcome) -> Self;
    fn aggregate(&self) -> &EffectOutcome;
    fn aggregate_mut(&mut self) -> &mut EffectOutcome;
    fn into_outputs(self) -> CompletedEffectOutputs;
}
impl OriginalEffectOutput for EffectOutcome {
    fn from_aggregate(outcome: EffectOutcome) -> Self {
        outcome
    }
    fn aggregate(&self) -> &EffectOutcome {
        self
    }
    fn aggregate_mut(&mut self) -> &mut EffectOutcome {
        self
    }
    fn into_outputs(self) -> CompletedEffectOutputs {
        CompletedEffectOutputs::aggregate_only(self)
    }
}
impl OriginalEffectOutput for CompletedEffectOutputs {
    fn from_aggregate(outcome: EffectOutcome) -> Self {
        Self::aggregate_only(outcome)
    }
    fn aggregate(&self) -> &EffectOutcome {
        &self.outcome
    }
    fn aggregate_mut(&mut self) -> &mut EffectOutcome {
        &mut self.outcome
    }
    fn into_outputs(mut self) -> CompletedEffectOutputs {
        self.synchronize_observations();
        self
    }
}

pub struct SimultaneousEffectCommit<Output = EffectOutcome> {
    pub outcome: Output,
    pub completion: Option<Box<dyn SimultaneousEffectCompletion>>,
}
impl<Output> SimultaneousEffectCommit<Output> {
    pub fn finished(outcome: Output) -> Self {
        Self {
            outcome,
            completion: None,
        }
    }
}
impl SimultaneousEffectCommit {
    pub(crate) fn into_retained(self) -> SimultaneousEffectCommit<CompletedEffectOutputs> {
        SimultaneousEffectCommit {
            outcome: self.outcome.into_outputs(),
            completion: self.completion,
        }
    }
}
impl SimultaneousEffectCommit<CompletedEffectOutputs> {
    pub(crate) fn into_aggregate(self) -> SimultaneousEffectCommit {
        SimultaneousEffectCommit {
            outcome: self.outcome.into_outcome(),
            completion: self.completion,
        }
    }
}

/// A fully determined part of one simultaneous multi-player action.
///
/// Implementations are prepared for every affected player against the same
/// immutable game state. `commit` must apply only the already-determined
/// mutation: it must not ask a new question or recalculate a value from game
/// state changed by an earlier proposal in the same batch.
pub trait SimultaneousEffectProposal: std::fmt::Debug + Send {
    /// Frozen assignments for a shared damage action, after preparation.
    /// None preserves a scheduling boundary that this proposal cannot join.
    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        None
    }

    /// Decorate this instruction's binding view after the shared action has
    /// completed. The returned view must never be published as extra history.
    fn bind_damage_action(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        _owner: &CompletedEffectOutputs,
    ) -> Result<DamageActionBinding, ExecutionError> {
        Err(ExecutionError::InternalError(
            "prepared instruction has no shared damage binding contract".into(),
        ))
    }
    /// Accepted nominal life payment, before replacements alter its actions.
    /// The batch owner checks shared team affordability once (CR 119.4a).
    fn declared_life_payment(&self) -> Option<(crate::ids::PlayerId, u32)> {
        None
    }

    /// All nominal payments owned by a compound proposal. A single action
    /// retains its existing declaration; adapters forward complete child lists.
    fn declared_life_payments(&self) -> Vec<(crate::ids::PlayerId, u32)> {
        self.declared_life_payment().into_iter().collect()
    }

    /// A captured instruction may contain simultaneous original actions.
    /// Standalone execution asks its owner; decorators forward this boundary.
    /// Enclosing simultaneous programs may impose their own wider grouping.
    fn has_simultaneous_originals(&self) -> bool {
        false
    }

    /// A single payment owner may export its captured nominal quantity.
    /// Compounds must not infer one quantity by summing unrelated resource units.
    fn nominal_payment_quantity(&self) -> Option<u64> {
        None
    }

    /// Nominal resource claims owned by this prepared program. Legacy life
    /// owners retain their declarations; compound scopes forward full claims.
    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.declared_life_payments()
            .into_iter()
            .map(|(player, amount)| crate::effects::PaymentResourceClaim::Life { player, amount })
            .collect()
    }

    /// Resolve mutable preflight and selection for every participant before
    /// any participant runs a replacement program or commits an original.
    fn prepare_selection(
        &mut self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        Ok(())
    }

    /// Resolve a prepared proposal's replacement choices against the shared
    /// pre-mutation world. Owners run this for every participant before any
    /// commit; immutable choice-free proposals need no further preparation.
    fn prepare_original(
        &mut self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        Ok(())
    }

    /// Seal an instruction that has not been collected into a shared owner.
    /// Coordinators call this after contribution collection and before any
    /// sibling original commits. Owners already sealed during preparation
    /// need no additional work; scopes must forward the child's phase.
    /// Repeated calls must retain the same prepared action without consuming
    /// replacement choices or one-shot resources again.
    fn seal_original(
        &mut self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        Ok(())
    }

    /// Separate original mutations from replacement-added programs when the
    /// proposal has them. Existing choice-free proposals finish in one phase.
    /// Retain packets supplied by the original owner without inventing a
    /// continuation for an already finished action. Legacy proposals remain
    /// explicit aggregate-only adapters until they expose retained originals.
    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.commit_original(game, ctx)
            .map(SimultaneousEffectCommit::into_retained)
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit(game, ctx)
            .map(SimultaneousEffectCommit::finished)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;
}

/// A proposal that defers a choice-free per-player effect to commit time.
///
/// Correct for effects whose behavior involves no decisions and whose
/// matching set is scoped to the iterated player (e.g. "each player returns
/// all creature cards from their graveyard"): earlier players' commits cannot
/// change what this player's action does.
#[derive(Debug)]
pub struct DeferredPlayerActionProposal {
    pub effect: crate::effect::Effect,
    pub iterated_player: Option<crate::ids::PlayerId>,
}

impl SimultaneousEffectProposal for DeferredPlayerActionProposal {
    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let effect = self.effect;
        ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
            crate::effects::execute_effect_with_outputs(game, &effect, ctx)
                .map(SimultaneousEffectCommit::finished)
        })
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(|receipt| receipt.outcome.into_outcome())
    }
}

/// A context-only resolution prelude has one binding owner shared by real
/// execution and applicability queries. Binding reads the current world and
/// retained evidence without executing actions or requesting decisions.
/// Its scalar result belongs to ordinary execution; a query discards it.
pub trait ResolutionPreludeBinding {
    fn bind_resolution_prelude(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;
}

pub trait EffectExecutor:
    std::fmt::Debug + Any + Send + Sync + EffectExecutorClone + 'static
{
    /// The authored primitive action whose original result this executor produces.
    /// Composition executors preserve their children's independently recorded actions.
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        None
    }

    /// Execute this effect, mutating the game state and returning the outcome.
    ///
    /// # Arguments
    ///
    /// * `game` - The mutable game state to modify
    /// * `ctx` - The execution context containing source, controller, targets, etc.
    ///
    /// # Returns
    ///
    /// * `Ok(EffectOutcome)` - The outcome (result + events) of executing the effect
    /// * `Err(ExecutionError)` - If the effect could not be executed
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;

    /// Retain outputs supplied by the semantic owner during ordinary execution.
    /// Aggregate-only owners explicitly expose partial projection coverage.
    /// Overrides share their execution body with `execute`; they must not
    /// prepare or commit the action a second time to recover its outputs.
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.execute(game, ctx)
            .map(CompletedEffectOutputs::aggregate_only)
    }

    /// Native action owners retain dynamically introduced draws and their tails.
    fn supports_replacement_draw_continuation(&self) -> bool {
        false
    }

    fn prepare_replacement_draw_continuation_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::finished)
    }

    fn prepare_replacement_draw_continuation(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.prepare_replacement_draw_continuation_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }

    /// Whether this effect can prepare an immutable proposal for a generic
    /// simultaneous each-player action (CR 101.4, 608.2f).
    /// Selected authored programs have a separate cursor capability. The
    /// coordinator invokes it only for Action execution; payment declaration
    /// and acknowledgement remain with the existing TotalCost owner.
    fn supports_prepared_action_program(&self) -> bool {
        false
    }

    fn select_prepared_action_program(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        Err(ExecutionError::Impossible(
            "effect has no selected action-program owner".into(),
        ))
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        false
    }

    /// This action must complete each participant occurrence separately.
    /// Wrappers inherit the contract from their children; the action owner
    /// declares it, rather than callers recognizing particular keywords.
    fn requires_sequential_player_actions(&self) -> bool {
        let mut sequential = false;
        self.visit_child_effects(&mut |child| {
            sequential |= child.0.requires_sequential_player_actions();
        });
        sequential
    }

    /// Whether executing this effect once per player can only observe game
    /// state and update execution-context metadata/outcomes. Such effects may
    /// run in APNAP order without an immutable mutation proposal because no
    /// player's execution can change the game state seen by another player.
    fn is_read_only_simultaneous_player_action(&self) -> bool {
        false
    }

    /// Prepare this player's part of a simultaneous action without mutating
    /// game state. The composition layer collects every proposal in APNAP order
    /// before committing the batch atomically.
    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        Err(ExecutionError::InternalError(
            "effect advertised simultaneous preparation without implementing it".to_string(),
        ))
    }

    /// Whether object iteration can retain this program inside one shared
    /// damage occurrence boundary. This does not make arbitrary sequences
    /// simultaneous: only damage owners and wrappers that preserve the same
    /// action boundary opt in. Source and tag decorators forward the contract.
    fn shares_iterated_damage_action(&self) -> bool {
        self.transparent_child_effect()
            .is_some_and(|effect| effect.0.shares_iterated_damage_action())
    }

    /// Explicit shared-assignment and scoped-result routing capability.
    /// Transparent shape alone does not prove that a decorator forwards it.
    fn supports_damage_action_cohort(&self) -> bool {
        false
    }

    /// Resolve the nominal event for a single replacement-original action.
    /// This query must not mutate the world, process replacements, or commit
    /// the action. Event-family owners prepare the returned proposal before
    /// any sibling original commits. None means no event-only contract.
    /// Decorators do not inherit this automatically: returning just an event
    /// must not discard their result/tag/source scope or completion metadata.
    fn replacement_original_event(
        &self,
        _game: &GameState,
        _ctx: &ExecutionContext,
    ) -> Result<Option<crate::events::Event>, ExecutionError> {
        Ok(None)
    }

    /// The primary runtime category for this effect.
    ///
    /// Most effects are `Standard`. Effects whose main job is to register
    /// delayed triggers or replacement effects should override this.
    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::Standard
    }

    /// The runtime categories this effect participates in.
    ///
    /// By default this reports the primary category plus `CostExecutable` when
    /// the effect opts into `CostExecutableEffect`. This is intended for
    /// introspection, contributor guidance, and future tooling around effect
    /// extension points.
    fn execution_categories(&self) -> Vec<EffectExecutionCategory> {
        let mut categories = vec![self.primary_execution_category()];
        if self.as_cost_executable().is_some()
            && !categories.contains(&EffectExecutionCategory::CostExecutable)
        {
            categories.push(EffectExecutionCategory::CostExecutable);
        }
        categories
    }

    /// Clone this effect into a boxed trait object.
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        EffectExecutorClone::clone_boxed(self)
    }

    /// Execute a composed child with the ordinary instruction recording
    /// contract. Prepared commits and cost validation use their own APIs.
    fn execute_child(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let child = crate::effect::Effect::from_boxed_executor(self.clone_box());
        crate::effects::execute_effect(game, &child, ctx)
    }

    /// The same ordinary instruction gateway, retaining child ownership.
    fn execute_child_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let child = crate::effect::Effect::from_boxed_executor(self.clone_box());
        crate::effects::execute_effect_with_outputs(game, &child, ctx)
    }

    /// Get the target specification for this effect, if it has one.
    ///
    /// Used for target selection during spell/ability resolution.
    /// Returns `None` for effects that don't require targeting.
    /// Cost selection dependencies owned by this instruction. Decorators
    /// forward declarations without stripping their execution/query scopes.
    /// Compound owners must compose the authored child order themselves.
    fn cost_choice_bindings(&self) -> CostChoiceBindings {
        if let Some(child) = self.transparent_child_effect() {
            return child.0.cost_choice_bindings();
        }
        let mut bindings = CostChoiceBindings::default();
        for spec in self
            .get_target_spec()
            .cloned()
            .into_iter()
            .chain(self.decision_related_object_specs())
        {
            bindings.append(CostChoiceBindings::from_spec(&spec));
        }
        bindings
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.transparent_child_effect()
            .and_then(|effect| effect.0.get_target_spec())
    }

    /// Return structured object specs that are useful to preview when this
    /// effect appears as one option in a player decision.
    ///
    /// This is display metadata only: it must not mutate game state or make any
    /// choices. By default, targeted effects preview their target spec. Effects
    /// that affect a proven set through an `ObjectFilter` can expose that set as
    /// `ChooseSpec::All(filter)`.
    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        if let Some(effect) = self.transparent_child_effect() {
            return effect.0.decision_related_object_specs();
        }
        self.get_target_spec().cloned().into_iter().collect()
    }

    /// Return object ids that are useful to preview when this effect appears as
    /// one option in a player decision.
    ///
    /// This centralizes object preview resolution so individual effects only
    /// need to expose structured specs, not duplicate object lookup logic.
    fn related_object_ids_for_decision(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Option<Vec<ObjectId>> {
        let specs = self.decision_related_object_specs();
        if specs.is_empty() {
            return None;
        }

        let mut ids = Vec::new();
        for spec in specs {
            if let Some(mut spec_ids) =
                crate::effects::helpers::preview_object_ids_for_choose_spec(game, &spec, ctx)
            {
                ids.append(&mut spec_ids);
            }
        }
        ids.sort();
        ids.dedup();
        Some(ids)
    }

    /// Get a human-readable description of what this effect targets.
    ///
    /// Used for UI/logging during target selection.
    fn target_description(&self) -> &'static str {
        if let Some(effect) = self.transparent_child_effect() {
            return effect.0.target_description();
        }
        "target"
    }

    /// Get the target count for this effect, if it has one.
    ///
    /// Used for determining min/max targets during target selection.
    /// Returns `None` to use default (exactly 1 target).
    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.transparent_child_effect()
            .and_then(|effect| effect.0.get_target_count())
    }

    /// Value divided among this effect's targets during announcement, if any.
    fn get_target_distribution_value(&self) -> Option<&Value> {
        self.transparent_child_effect()
            .and_then(|effect| effect.0.get_target_distribution_value())
    }

    /// Minimum amount assigned to each target in an announced division.
    fn target_distribution_min_per_target(&self) -> u32 {
        self.transparent_child_effect()
            .map_or(1, |effect| effect.0.target_distribution_min_per_target())
    }

    /// Whether this target requirement should reuse a compatible earlier target.
    fn target_reuse_policy(&self) -> TargetReusePolicy {
        self.transparent_child_effect()
            .map_or(TargetReusePolicy::ReuseCompatiblePrevious, |effect| {
                effect.0.target_reuse_policy()
            })
    }

    /// Player assigned to make this effect's target choice, when Oracle says
    /// someone other than the spell or ability controller chooses it.
    fn target_chooser(&self) -> Option<&crate::target::PlayerFilter> {
        self.transparent_child_effect()
            .and_then(|effect| effect.0.target_chooser())
    }

    /// Structured target selection metadata for this effect.
    fn target_selection_profile(&self) -> Option<TargetSelectionProfile<'_>> {
        let spec = self.get_target_spec()?;
        let spec_count = spec.count();
        let (min_targets, max_targets) = if let Some(target_count) = self.get_target_count() {
            (target_count.min, target_count.max)
        } else if spec_count != crate::effect::ChoiceCount::default() {
            (spec_count.min, spec_count.max)
        } else {
            (1, Some(1))
        };

        Some(TargetSelectionProfile {
            spec,
            chooser: self.target_chooser(),
            description: self.target_description(),
            min_targets,
            max_targets,
            count_value: spec.count_value(),
            distribution_value: self.get_target_distribution_value(),
            distribution_min_per_target: self.target_distribution_min_per_target(),
            reuse_policy: self.target_reuse_policy(),
        })
    }

    /// Get the modal specification for this effect, if it's a modal effect.
    ///
    /// Per MTG rule 601.2b, modes must be chosen during spell casting (before targets).
    /// This method returns the information needed to present mode choices to the player.
    /// Returns `None` for non-modal effects.
    fn get_modal_spec(&self) -> Option<ModalSpec> {
        self.transparent_child_effect()
            .and_then(|effect| effect.0.get_modal_spec())
    }

    /// Return modal child-effect metadata for target-selection planning.
    fn modal_effect_spec(&self) -> Option<ModalEffectSpec<'_>> {
        self.transparent_child_effect()
            .and_then(|effect| effect.modal_effect_spec())
    }

    /// Get the modal specification with game context, allowing conditional evaluation.
    ///
    /// For compositional effects like ConditionalEffect, this method allows evaluating
    /// the condition at cast time to determine which branch's modal spec to use.
    /// For example, Akroma's Will wraps ChooseModeEffect in a ConditionalEffect that
    /// checks if you control a commander - this method evaluates that condition and
    /// returns the appropriate modal spec.
    ///
    /// Default implementation delegates to `get_modal_spec()`.
    fn get_modal_spec_with_context(
        &self,
        _game: &GameState,
        _controller: PlayerId,
        _source: ObjectId,
    ) -> Option<ModalSpec> {
        self.get_modal_spec()
    }

    /// Returns this effect as a cost-capable trait object when it can legally
    /// participate in cost payment.
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        None
    }

    /// If this is a "pay life" effect, returns the amount.
    ///
    /// Used for checking if alternative cost effects can be paid.
    fn pay_life_amount(&self) -> Option<u32> {
        None
    }

    /// If this is an "exile from hand as cost" effect, returns (count, color_filter).
    ///
    /// Used for checking if alternative cost effects can be paid.
    fn exile_from_hand_cost_info(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        None
    }

    /// Check if this effect can be executed as a cost.
    ///
    /// This is used for non-mana cost components in mana abilities and alternative casting costs.
    /// Returns Ok(()) if the cost can be paid, or Err with a reason if not.
    ///
    /// Default implementation returns Ok(()) (effect can always be executed).
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        self.can_execute_as_cost_with_reason(game, source, controller, PaymentReason::Other)
    }

    /// Check if this effect can be executed as a cost for a specific payment reason.
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        reason: PaymentReason,
    ) -> Result<(), CostValidationError> {
        if let Some(cost_effect) = self.as_cost_executable() {
            return CostExecutableEffect::can_execute_as_cost_with_reason(
                cost_effect,
                game,
                source,
                controller,
                reason,
            );
        }
        Ok(())
    }

    /// Returns true if this is a "tap source" cost effect.
    ///
    /// Used for checking summoning sickness restrictions.
    fn is_tap_source_cost(&self) -> bool {
        false
    }

    /// Returns true if this is an "untap source" cost effect.
    fn is_untap_source_cost(&self) -> bool {
        false
    }

    /// Returns true if this is a "sacrifice source" cost effect.
    fn is_sacrifice_source_cost(&self) -> bool {
        false
    }

    /// Returns a human-readable description of this effect when used as a cost.
    ///
    /// Used for displaying alternative casting costs like "Pay 1 life, exile a blue card".
    /// Returns None if no description is available, in which case a generic display is used.
    fn cost_description(&self) -> Option<String> {
        None
    }

    /// Return the semantically transparent child effect for wrappers whose
    /// metadata should be inherited from a single inner effect.
    fn transparent_child_effect(&self) -> Option<&Effect> {
        None
    }

    /// Legacy shape prechecks may unwrap metadata decorators only while the
    /// child's cost inputs keep the same meaning. Context scopes override this
    /// boundary so their own contextual cost query runs before leaf inspection.
    fn transparent_cost_precheck_child_effect(&self) -> Option<&Effect> {
        self.transparent_child_effect()
    }

    /// Visit immediately nested runtime effects, if this effect is a wrapper or
    /// composition effect.
    ///
    /// Implementations should expose only direct children. Recursive traversal is
    /// provided by the default capability helpers below.
    fn visit_child_effects(&self, _visitor: &mut dyn FnMut(&Effect)) {}

    /// Typed Suspend casting identity for this program's current source.
    /// Only executors which perform that cast, or wrappers which preserve its
    /// execution source, opt in. The generic child visitor also visits granted,
    /// copied, deferred and source-rebound programs and is not safe here.
    fn contains_current_source_suspend_cast(&self) -> bool {
        false
    }

    /// Object inputs acquired by this instruction itself, excluding possible
    /// future/deferred children exposed only for previews or target planning.
    /// Native composite action owners declare their own inputs explicitly.
    fn own_preflight_object_specs(&self) -> Vec<ChooseSpec> {
        let mut has_children = false;
        self.visit_child_effects(&mut |_| has_children = true);
        if has_children {
            Vec::new()
        } else {
            self.get_target_spec().cloned().into_iter().collect()
        }
    }

    /// Direct role use, excluding optional/conditional children until they run.
    fn directly_mentions_player_filter(&self, needle: &crate::target::PlayerFilter) -> bool {
        let mut has_children = false;
        self.visit_child_effects(&mut |_| has_children = true);
        !has_children
            && self
                .get_target_spec()
                .is_some_and(|spec| spec.mentions_player_filter(needle))
    }
    fn mentions_player_filter(&self, needle: &crate::target::PlayerFilter) -> bool {
        let mut found = self.directly_mentions_player_filter(needle);
        self.visit_child_effects(&mut |effect| found |= effect.0.mentions_player_filter(needle));
        found
    }

    /// Visit complete definitions directly owned by this executor. Composition
    /// traversal remains the caller's responsibility through child effects.
    fn visit_card_definitions(&self, _visitor: &mut dyn FnMut(&crate::cards::CardDefinition)) {}

    /// Whether this instruction selects object references for a following
    /// action. Its selection payload must not replace that action's numeric
    /// result. Read-only preparation is a separate contract checked by callers.
    fn is_object_selection_prelude(&self) -> bool {
        self.transparent_child_effect()
            .is_some_and(|effect| effect.0.is_object_selection_prelude())
    }

    /// Declare bindings owned by this selection instruction or its decorators.
    /// Prepared adapters retain these slots even if no value diff was observed;
    /// they do not classify concrete chooser or annotation effect types.
    fn visit_prepared_selection_bindings(&self, visitor: &mut dyn FnMut(PreparedSelectionBinding)) {
        if let Some(effect) = self.transparent_child_effect() {
            effect.0.visit_prepared_selection_bindings(visitor);
        }
    }

    /// Expose the context-only binding owner when this effect is a resolution
    /// prelude. A boolean declaration alone cannot authorize speculative action
    /// execution; applicability calls this read-only owner directly.
    fn as_resolution_prelude(&self) -> Option<&dyn ResolutionPreludeBinding> {
        None
    }

    /// Whether this effect can consume an X value when used as a cost.
    fn references_cost_x(&self) -> bool {
        self.transparent_child_effect()
            .is_some_and(|effect| effect.references_cost_x())
    }

    /// Maximum legal X value for this effect when used as a cost.
    fn max_cost_x(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> Option<u32> {
        self.transparent_child_effect()
            .and_then(|effect| effect.max_cost_x(game, source, controller))
    }

    /// Complete, side-effect-free mana production semantics for the compact
    /// evaluator. This is deliberately opt-in: a capability hint is not proof
    /// that executing an effect only produces mana. Wrappers must preserve
    /// restrictions, choices and other effects rather than forwarding blindly.
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        None
    }

    /// Returns true when this effect is directly capable of adding mana.
    ///
    /// This is intentionally context-free, so compiler/runtime classification can
    /// distinguish "may add mana" from "we can infer exact symbols right now".
    fn directly_produces_mana(&self) -> bool {
        false
    }

    /// Returns true if this effect or any nested child effect can add mana.
    fn contains_mana_production(&self) -> bool {
        if self.directly_produces_mana() {
            return true;
        }

        let mut found = false;
        self.visit_child_effects(&mut |effect| {
            if !found && effect.contains_mana_production() {
                found = true;
            }
        });
        found
    }

    /// Returns true if this effect can add mana in the given game context.
    ///
    /// This recurses through composition effects via `visit_child_effects`.
    fn could_produce_mana(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> bool {
        if self.directly_produces_mana()
            || self
                .producible_mana_symbols(game, source, controller)
                .is_some_and(|symbols| !symbols.is_empty())
        {
            return true;
        }

        let mut found = false;
        self.visit_child_effects(&mut |effect| {
            if !found && effect.could_produce_mana(game, source, controller) {
                found = true;
            }
        });
        found
    }

    /// Returns mana symbols this effect can produce when used as a mana ability payload.
    ///
    /// This is a best-effort capability hook used by inference effects such as
    /// "add one mana of any type that a land could produce". Implementations
    /// should return all possible symbols for the given source/controller context.
    fn producible_mana_symbols(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        None
    }

    /// Collect all inferable mana symbols from this effect subtree.
    fn collect_producible_mana_symbols(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        out: &mut Vec<ManaSymbol>,
    ) {
        if let Some(symbols) = self.producible_mana_symbols(game, source, controller) {
            out.extend(symbols);
        }
        self.visit_child_effects(&mut |effect| {
            effect.collect_producible_mana_symbols(game, source, controller, out);
        });
    }

    /// Downcast support for effect introspection.
    fn as_any(&self) -> &dyn Any
    where
        Self: Sized,
    {
        self
    }
}

/// Error returned when a cost effect cannot be paid.
#[derive(Debug, Clone, PartialEq)]
pub enum CostValidationError {
    /// Source is already tapped
    AlreadyTapped,
    /// Source is already untapped
    AlreadyUntapped,
    /// Creature has summoning sickness (can't tap)
    SummoningSickness,
    /// Not enough life to pay
    NotEnoughLife,
    /// Not enough energy to pay
    NotEnoughEnergy,
    /// Not enough cards to exile
    NotEnoughCards,
    /// Cannot sacrifice required permanent
    CannotSacrifice,
    /// Checked execution failure retains its typed rollback contract.
    ExecutionFailed(ExecutionError),
    /// Generic error with message
    Other(String),
}

/// Preserve cost-program children while converting their declared payment representations.
pub(crate) fn canonical_cost_children(effects: &[Effect]) -> Option<Vec<Effect>> {
    let mut changed = false;
    let children = effects
        .iter()
        .map(|effect| {
            if let Some(replacement) = effect
                .0
                .as_cost_executable()
                .and_then(|cost| cost.canonical_cost_effect())
            {
                changed = true;
                replacement
            } else {
                effect.clone()
            }
        })
        .collect();
    changed.then_some(children)
}

/// Additional behavior required for effects that can be used as costs.
pub trait CostExecutableEffect: EffectExecutor {
    /// State changed by a written object-selection payment. This metadata
    /// supports source-symbol reservation, not legality or affordability.
    /// Source-rebinding scopes must stop forwarding the caller-source claim.
    fn cost_choice_tap_state(&self) -> Option<bool> {
        self.transparent_child_effect()
            .and_then(|child| child.0.as_cost_executable())
            .and_then(|cost| cost.cost_choice_tap_state())
    }

    /// Additional per-object eligibility for a choice consumed by this cost.
    /// This is a read-only filter, not proof that a whole selection pays the
    /// cost. None leaves eligibility to the complete cost query. Decorators
    /// retain their input scopes; implementations must not ask for choices or
    /// publish speculative bindings.
    fn cost_choice_candidate_is_eligible(
        &self,
        game: &GameState,
        execution: &mut ExecutionContext,
        reason: PaymentReason,
        tag: &crate::tag::TagKey,
        object: ObjectId,
    ) -> Option<bool> {
        self.transparent_child_effect()
            .and_then(|child| child.0.as_cost_executable())
            .and_then(|cost| {
                cost.cost_choice_candidate_is_eligible(game, execution, reason, tag, object)
            })
    }

    /// Ordered/conditional cost programs compose payment children explicitly.
    /// Ordinary actions and replacement payloads retain ordinary dispatch.
    fn execute_payment_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.execute_with_outputs(game, ctx)
    }

    /// Unprepared compounds may forward binding acknowledgement to their
    /// actual payment children. Captured proposals acknowledge at the parent.
    fn payment_bindings_are_owned_by_children(&self) -> bool {
        false
    }

    /// A captured cost may provide nominal X for preparing following components
    /// without publishing a payment receipt or executing the action.
    fn payment_x_from_prepared_payment(
        &self,
        proposal: &dyn SimultaneousEffectProposal,
        execution: &ExecutionContext,
    ) -> Result<Option<u32>, CostValidationError> {
        if let Some(child) = self.transparent_child_effect()
            && let Some(cost) = child.0.as_cost_executable()
        {
            return cost.payment_x_from_prepared_payment(proposal, execution);
        }
        Ok(None)
    }

    /// Prepare this cost's nominal payment owner, distinct from an ordinary
    /// action that may report only its actual physical result. Decorators
    /// forward through their own scoped proposals and the recorded gateway.
    fn prepare_simultaneous_payment(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        self.prepare_simultaneous_player_action(game, ctx)
    }

    /// Whether this cost owns a complete prepared-original payment contract.
    /// This is stronger than ordinary simultaneous execution: the proposal
    /// must retain nominal acknowledgement, resource declarations and deferred
    /// additions without paying the cost during preparation. Transparent
    /// wrappers retain their own proposal scopes while forwarding capability.
    fn supports_prepared_payment(&self) -> bool {
        self.supports_simultaneous_player_action()
            && self.transparent_child_effect().is_some_and(|effect| {
                effect
                    .0
                    .as_cost_executable()
                    .is_some_and(|cost| cost.supports_prepared_payment())
            })
    }

    /// Confirm that the prepared proposal retains this owner's accepted
    /// nominal payment. Decorators inspect their scoped proposal through the
    /// child contract, rather than having totals infer acceptance from a
    /// particular resource (life, mana, counters, or selected objects).
    fn accepts_prepared_payment(&self, proposal: &dyn SimultaneousEffectProposal) -> bool {
        self.transparent_child_effect().is_some_and(|effect| {
            effect
                .0
                .as_cost_executable()
                .is_some_and(|cost| cost.accepts_prepared_payment(proposal))
        })
    }

    /// Validate this owner's nominal payment result after ordinary execution.
    /// Replacements may change the physical action without invalidating an
    /// accepted cost. Owners that require a receipt or acknowledge failure
    /// define that policy here; the cost bridge does not classify effect types.
    /// Transparent decorators retain their metadata and forward the policy.
    fn validate_payment_outcome(&self, outcome: &EffectOutcome) -> Result<(), CostValidationError> {
        if let Some(effect) = self.transparent_child_effect()
            && let Some(cost) = effect.0.as_cost_executable()
        {
            return cost.validate_payment_outcome(outcome);
        }
        if outcome.instruction_result().status == crate::effect::OutcomeStatus::Impossible {
            return Err(CostValidationError::Other(
                "effect payment was not acknowledged".into(),
            ));
        }
        Ok(())
    }

    /// Export X chosen by this cost, from its nominal receipt or announced
    /// inputs. Physical changes and replacement-child results cannot redefine
    /// the authored quantity. Adapters preserve an already-bound parent X.
    fn payment_x_from_outcome(
        &self,
        outcome: &EffectOutcome,
        execution: &ExecutionContext,
    ) -> Result<Option<u32>, CostValidationError> {
        if let Some(effect) = self.transparent_child_effect()
            && let Some(cost) = effect.0.as_cost_executable()
        {
            return cost.payment_x_from_outcome(outcome, execution);
        }
        Ok(None)
    }

    /// Finalize this owner's cost-specific context bindings after its receipt
    /// and authored X are available. The world is read-only: this boundary may
    /// validate/publish bindings, but must not execute another physical action.
    /// Transparent decorators retain their scopes and forward the policy.
    fn finalize_payment_bindings(
        &self,
        game: &GameState,
        outcome: &EffectOutcome,
        execution: &mut ExecutionContext,
        payment_x: Option<u32>,
    ) -> Result<(), crate::cost::CostPaymentError> {
        if let Some(effect) = self.transparent_child_effect()
            && let Some(cost) = effect.0.as_cost_executable()
        {
            return cost.finalize_payment_bindings(game, outcome, execution, payment_x);
        }
        Ok(())
    }

    /// Canonical representation of this instruction when it pays a cost.
    /// Cost factories invoke this contract; ordinary execution and replacement
    /// programs keep their authored effects. Composition adapters preserve
    /// their own metadata while delegating conversion of cost children.
    fn canonical_cost_effect(&self) -> Option<Effect> {
        None
    }

    /// Validate the same captured bindings that live payment will execute.
    /// Legacy validators remain available while their owners migrate; adapters
    /// must forward this contract instead of reconstructing a partial context.
    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        reason: PaymentReason,
    ) -> Result<(), CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(
            self,
            game,
            ctx.source,
            ctx.controller,
            reason,
        )
    }

    /// Check whether this effect can be paid in a cost context.
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError>;

    /// Check whether this effect can be paid in a cost context for a specific reason.
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        _reason: PaymentReason,
    ) -> Result<(), CostValidationError> {
        CostExecutableEffect::can_execute_as_cost(self, game, source, controller)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test effect that always resolves.
    #[derive(Debug, Clone)]
    struct TestEffect;

    impl EffectExecutor for TestEffect {
        fn execute(
            &self,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            Ok(EffectOutcome::resolved())
        }
    }

    #[derive(Debug, Clone)]
    struct ReplacementTestEffect;

    impl EffectExecutor for ReplacementTestEffect {
        fn execute(
            &self,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            Ok(EffectOutcome::resolved())
        }

        fn primary_execution_category(&self) -> EffectExecutionCategory {
            EffectExecutionCategory::ReplacementRegistration
        }
    }

    #[derive(Debug, Clone)]
    struct CostTestEffect;

    impl EffectExecutor for CostTestEffect {
        fn execute(
            &self,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            Ok(EffectOutcome::resolved())
        }

        fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
            Some(self)
        }
    }

    impl CostExecutableEffect for CostTestEffect {
        fn can_execute_as_cost(
            &self,
            _game: &GameState,
            _source: ObjectId,
            _controller: PlayerId,
        ) -> Result<(), CostValidationError> {
            Ok(())
        }
    }

    #[test]
    fn test_effect_executor_trait_is_object_safe() {
        // This test verifies that EffectExecutor can be used as a trait object
        let effect: Box<dyn EffectExecutor> = Box::new(TestEffect);
        assert!(format!("{:?}", effect).contains("TestEffect"));
    }

    #[test]
    fn standard_effect_reports_standard_category() {
        let effect = TestEffect;
        assert_eq!(
            effect.execution_categories(),
            vec![EffectExecutionCategory::Standard]
        );
    }

    #[test]
    fn replacement_effect_reports_replacement_category() {
        let effect = ReplacementTestEffect;
        assert_eq!(
            effect.execution_categories(),
            vec![EffectExecutionCategory::ReplacementRegistration]
        );
    }

    #[test]
    fn cost_effect_reports_cost_capability() {
        let effect = CostTestEffect;
        assert_eq!(
            effect.execution_categories(),
            vec![
                EffectExecutionCategory::Standard,
                EffectExecutionCategory::CostExecutable,
            ]
        );
    }

    #[test]
    fn transparent_wrappers_inherit_target_and_modal_profiles() {
        let targeted = crate::effect::Effect::with_id(
            17,
            crate::effect::Effect::deal_damage(1, crate::target::ChooseSpec::AnyTarget),
        );
        let profile = targeted
            .target_selection_profile()
            .expect("with-id wrapper should expose inner target profile");
        assert_eq!(profile.spec, &crate::target::ChooseSpec::AnyTarget);
        assert_eq!(profile.min_targets, 1);
        assert_eq!(profile.max_targets, Some(1));

        let modal = crate::effect::Effect::with_id(
            18,
            crate::effect::Effect::choose_one(vec![crate::effect::EffectMode::new(
                "Deal damage",
                vec![crate::effect::Effect::deal_damage(
                    1,
                    crate::target::ChooseSpec::AnyTarget,
                )],
            )]),
        );
        let modal_spec = modal
            .modal_effect_spec()
            .expect("with-id wrapper should expose inner modal profile");
        assert_eq!(modal_spec.modes.len(), 1);
        assert_eq!(modal_spec.min_modes, &crate::effect::Value::Fixed(1));
        assert_eq!(modal_spec.max_modes, &crate::effect::Value::Fixed(1));
    }

    #[test]
    fn target_only_profiles_distinguish_synthetic_and_authored_declarations() {
        let synthetic = crate::effect::Effect::new(crate::effects::TargetOnlyEffect::new(
            crate::target::ChooseSpec::AnyTarget,
        ));
        let synthetic_profile = synthetic
            .target_selection_profile()
            .expect("target-only effect should expose target profile");
        assert_eq!(
            synthetic_profile.reuse_policy,
            TargetReusePolicy::SyntheticPrelude
        );

        let authored = crate::effect::Effect::new(crate::effects::TargetOnlyEffect::explicit(
            crate::target::ChooseSpec::AnyTarget,
        ));
        let authored_profile = authored
            .target_selection_profile()
            .expect("authored target-only effect should expose target profile");
        assert_eq!(
            authored_profile.reuse_policy,
            TargetReusePolicy::AlwaysDeclareNew
        );
    }

    #[test]
    fn resolution_prelude_and_cost_x_hooks_delegate_through_wrappers() {
        assert!(crate::effect::Effect::tag_triggering_object("triggering").is_resolution_prelude());
        assert!(crate::effect::Effect::tag_attached_to_source("attached").is_resolution_prelude());
        assert!(
            crate::effect::Effect::new(crate::effects::TaggedEffect::new(
                "context",
                crate::effect::Effect::new(crate::effects::SequenceEffect::new(Vec::new())),
            ))
            .is_resolution_prelude()
        );

        let cost = crate::effect::Effect::with_id(
            19,
            crate::effect::Effect::new(crate::effects::SacrificeEffect::you(
                crate::filter::ObjectFilter::creature(),
                crate::effect::Value::X,
            )),
        );
        assert!(cost.references_cost_x());
    }
}
