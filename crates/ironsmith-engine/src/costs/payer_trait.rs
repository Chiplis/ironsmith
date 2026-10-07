//! CostPayer trait for the modular cost system.
//!
//! This module defines the `CostPayer` trait used by `Cost` and `CostEffect`.

use std::any::Any;
use std::collections::HashMap;

use crate::cost::CostPaymentError;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Opaque cached inputs for potential mana affordability. Cost adapters reuse
/// the supplied query without exposing the engine's internal derived view.
pub struct PotentialManaQuery<'view, 'game> {
    pub(crate) view: &'view crate::derived_view::DerivedGameView<'game>,
    pub(crate) payment: Option<&'view crate::mana_payment::ManaPaymentRequest>,
}

impl<'view, 'game> PotentialManaQuery<'view, 'game> {
    pub(crate) fn new(view: &'view crate::derived_view::DerivedGameView<'game>) -> Self {
        Self { view, payment: None }
    }
}

/// Why a cost is being paid.
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaymentReason {
    /// Casting a spell.
    CastSpell,
    /// Activating a non-mana ability.
    ActivateAbility,
    /// Activating a mana ability.
    ActivateManaAbility,
    /// Paying the cost to unlock a locked Room door.
    UnlockDoor,
    /// Turning a face-down permanent face up.
    TurnFaceUp,
    /// Paying one iteration of a cumulative upkeep cost.
    CumulativeUpkeep,
    /// Paying a cost during effect or triggered-ability resolution.
    Effect,
    /// Paying another special-action or generic engine cost.
    #[default]
    Other,
    /// The special action from hand, not the later spell cast.
    Foretell,
    /// Exact announcement. Legacy TurnFaceUp does not assert a method.
    TurnFaceUpWithMethod(ironsmith_core::ManaTurnFaceUpMethod),
    /// Frozen at announcement; source changes do not change the paid ability.
    ActivateAbilityWithKeyword { keyword: ironsmith_core::ActivatedAbilityKeyword, mana_ability: bool },
}

impl PaymentReason {
    pub fn activation(keyword: Option<ironsmith_core::ActivatedAbilityKeyword>, mana_ability: bool) -> Self {
        match keyword {
            Some(keyword) => Self::ActivateAbilityWithKeyword { keyword, mana_ability },
            None if mana_ability => Self::ActivateManaAbility,
            None => Self::ActivateAbility,
        }
    }
    pub fn is_mana_ability(self) -> bool {
        matches!(self, Self::ActivateManaAbility | Self::ActivateAbilityWithKeyword { mana_ability: true, .. })
    }
    pub fn is_non_mana_ability(self) -> bool {
        matches!(self, Self::ActivateAbility | Self::ActivateAbilityWithKeyword { mana_ability: false, .. })
    }
    pub fn is_ability(self) -> bool { self.is_mana_ability() || self.is_non_mana_ability() }

    pub fn is_cast_or_ability_payment(self) -> bool {
        matches!(
            self,
            Self::CastSpell | Self::ActivateAbility | Self::ActivateManaAbility | Self::ActivateAbilityWithKeyword { .. }
        )
    }

    pub fn mana_payment_purpose(self) -> crate::ability::ManaPaymentPurpose {
        match self {
            Self::CastSpell => crate::ability::ManaPaymentPurpose::CastSpell,
            Self::ActivateAbility => crate::ability::ManaPaymentPurpose::ActivateAbility,
            Self::ActivateManaAbility => crate::ability::ManaPaymentPurpose::ActivateManaAbility,
            Self::ActivateAbilityWithKeyword { mana_ability: true, .. } => crate::ability::ManaPaymentPurpose::ActivateManaAbility,
            Self::ActivateAbilityWithKeyword { mana_ability: false, .. } => crate::ability::ManaPaymentPurpose::ActivateAbility,
            Self::UnlockDoor => crate::ability::ManaPaymentPurpose::UnlockDoor,
            Self::TurnFaceUp | Self::TurnFaceUpWithMethod(_) => crate::ability::ManaPaymentPurpose::TurnFaceUp,
            Self::Foretell => crate::ability::ManaPaymentPurpose::Foretell,
            Self::CumulativeUpkeep => crate::ability::ManaPaymentPurpose::CumulativeUpkeep,
            Self::Effect => crate::ability::ManaPaymentPurpose::Effect,
            Self::Other => crate::ability::ManaPaymentPurpose::Other,
        }
    }
}

/// Result of paying a cost.
#[derive(Debug, Clone, PartialEq)]
pub enum CostPaymentResult {
    /// Cost was paid successfully.
    Paid,
    /// Cost requires a choice from the player (e.g., which creature to sacrifice).
    /// Contains a description of the choice needed.
    NeedsChoice(String),
}

pub(crate) fn payment_event_cause(
    source: ObjectId,
    payer: PlayerId,
    reason: PaymentReason,
    requesting: Option<&crate::events::cause::EventCause>,
) -> crate::events::cause::EventCause {
    if reason == PaymentReason::Effect
        && let Some(cause) = requesting
    {
        let mut cause = cause.clone();
        cause.cause_type = crate::events::cause::CauseType::Cost;
        return cause;
    }
    crate::events::cause::EventCause::from_cost(source, payer)
}

/// Compose dependency-aware cost validation through the total-cost owner.
/// The checker retains speculative choice bindings between instructions; it
/// never executes a child action or asks for a payment choice.
pub(crate) fn check_effect_cost_program(
    effects: &[crate::effect::Effect],
    game: &GameState,
    execution: &mut crate::effects::ExecutionContext,
    reason: PaymentReason,
) -> Result<(), crate::effects::CostValidationError> {
    let components = effects
        .iter()
        .cloned()
        .map(crate::costs::Cost::try_effect)
        .collect::<Result<Vec<_>, _>>()
        .map_err(crate::effects::CostValidationError::Other)?;
    let total = crate::cost::TotalCost::from_costs(components);
    crate::special_actions::can_pay_total_cost_with_reason_in_context(
        game,
        execution.controller,
        execution.source,
        &total,
        reason,
        execution,
    )
    .map_err(|error| match error {
        CostPaymentError::ExecutionFailed(error) => crate::effects::CostValidationError::ExecutionFailed(error),
        error => crate::effects::CostValidationError::Other(error.to_string()),
    })
}

/// Captured inputs shared by cost preflight and live effect execution.
/// Capturing before borrowing the decision maker keeps both paths on the same
/// source, payment reason and value-resolution bindings.
pub(crate) struct CostExecutionBindings {
    source: ObjectId,
    payer: PlayerId,
    cause: crate::events::cause::EventCause,
    reason: PaymentReason,
    source_snapshot: Option<ObjectSnapshot>,
    prospective_cost_payment: bool,
    replacement: crate::effects::ReplacementExecutionContext,
    x_value: Option<u32>,
    chosen: Vec<crate::effects::ResolvedTarget>,
    announced: Vec<crate::effects::ResolvedTarget>,
    tags: HashMap<TagKey, Vec<ObjectSnapshot>>,
    outcomes: HashMap<crate::effect::EffectId, crate::effect::EffectOutcome>,
    provenance: ProvNodeId,
    inputs: Option<Box<crate::effects::PaymentExecutionInputs>>,
}

impl CostExecutionBindings {
    pub(crate) fn execution_context<'a>(
        self,
        decision_maker: &'a mut dyn crate::decision::DecisionMaker,
    ) -> crate::effects::ExecutionContext<'a> {
        let mut execution =
            crate::effects::ExecutionContext::new(self.source, self.payer, decision_maker)
                .with_cause(self.cause)
                .with_tagged_objects(self.tags)
                .with_cost_choice_targets(self.chosen)
                .with_provenance(self.provenance);
        if let Some(inputs) = self.inputs {
            inputs.restore_ref(&mut execution);
        }
        execution.source_snapshot = self.source_snapshot;
        execution.prospective_cost_payment = self.prospective_cost_payment;
        execution.replacement = self.replacement;
        execution.x_value = self.x_value;
        execution.effect_outcomes = self.outcomes;
        execution.announced_targets = Some(self.announced);
        execution.mana.payment_reason = Some(self.reason);
        execution
    }
}

/// Acknowledgement and the actual effect packet produced by a cost owner.
/// Effect-backed costs publish their observations at the payment boundary;
/// retaining this packet never requests another physical action/publication.
/// Owners with no effect packet retain their existing typed acknowledgement.
pub struct CostPaymentReceipt {
    pub result: CostPaymentResult,
    pub outputs: Option<crate::effects::CompletedEffectOutputs>,
}
impl CostPaymentReceipt {
    pub fn new(result: CostPaymentResult) -> Self {
        Self {
            result,
            outputs: None,
        }
    }
    pub(crate) fn from_outputs(outputs: crate::effects::CompletedEffectOutputs) -> Self {
        Self {
            result: CostPaymentResult::Paid,
            outputs: Some(outputs),
        }
    }
}

/// Context for cost payment operations.
///
/// Similar to ExecutionContext for effects, this provides the necessary
/// context for checking and paying costs.
pub struct CostContext<'dm> {
    /// The source object (permanent or spell whose cost is being paid).
    pub source: ObjectId,
    /// Last known characteristics of a departed source for resolution costs.
    pub source_snapshot: Option<ObjectSnapshot>,
    /// Execution-local replacement history and entry reservations of the
    /// instruction requesting this payment. Root costs start with no scope.
    pub(crate) replacement: crate::effects::ReplacementExecutionContext,
    /// The player paying the cost.
    pub payer: PlayerId,
    /// X value for variable costs.
    pub x_value: Option<u32>,
    /// Why this cost is being paid.
    pub reason: PaymentReason,
    /// Original requesting effect, when payment happens during its resolution.
    pub requesting_effect_cause: Option<crate::events::cause::EventCause>,
    /// Decision maker for player choices during cost payment.
    pub decision_maker: &'dm mut dyn crate::decision::DecisionMaker,
    /// Pre-chosen cards for costs that require card selection (e.g., ExileFromHand).
    /// When present, costs should use these instead of prompting for choice.
    pub pre_chosen_cards: Vec<ObjectId>,
    /// True only inside a cloned admission owner, never in actual payment.
    pub(crate) prospective_cost_payment: bool,
    pub announced_targets: Vec<crate::game_state::Target>,
    /// Tagged objects that persist across cost effects.
    ///
    /// This allows effects like "choose a creature, then sacrifice it" to work
    /// when both are cost effects. The first effect tags the chosen creature,
    /// and the second effect can reference it via the tag.
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    /// Outcomes of cost effects labeled with `WithIdEffect`.
    pub effect_outcomes: HashMap<crate::effect::EffectId, crate::effect::EffectOutcome>,
    /// Provenance parent node for events emitted while paying this cost.
    pub provenance: ProvNodeId,
    /// Some during an interactive cost transaction. The entries exclude ancestor
    /// mana abilities from funding themselves; special actions start with no exclusions.
    pub interactive_mana_exclusions: Option<Vec<ObjectId>>,
    /// Value inputs inherited by nested payments; instruction control remains local.
    pub(crate) execution_inputs: Option<Box<crate::effects::PaymentExecutionInputs>>,
    /// Exact resources reserved by other unpaid components.
    pub reserved_tap_sources: Vec<ObjectId>,
    /// Original sacrifice receipts, including a known empty result.
    pub completed_sacrifice: Option<Vec<ObjectSnapshot>>,
}

// Cost rollback owns every binding except the decision maker's prompt/answers.
// Exhaustive capture prevents a future input domain from escaping rollback.
macro_rules! cost_context_checkpoint {
    ($($field:ident: $field_type:ty,)* ) => {
        pub(crate) struct CostContextCheckpoint { $($field: $field_type,)* }
        impl CostContextCheckpoint {
            pub(crate) fn capture(ctx: &CostContext<'_>) -> Self {
                let CostContext { $($field,)* decision_maker: _ } = ctx;
                Self { $($field: $field.clone(),)* }
            }
            pub(crate) fn restore(self, ctx: &mut CostContext<'_>) {
                $(ctx.$field = self.$field;)*
            }
        }
    };
}
cost_context_checkpoint! {
    source: ObjectId,
    source_snapshot: Option<ObjectSnapshot>,
    replacement: crate::effects::ReplacementExecutionContext,
    payer: PlayerId,
    x_value: Option<u32>,
    reason: PaymentReason,
    requesting_effect_cause: Option<crate::events::cause::EventCause>,
    pre_chosen_cards: Vec<ObjectId>,
    prospective_cost_payment: bool,
    reserved_tap_sources: Vec<ObjectId>,
    completed_sacrifice: Option<Vec<ObjectSnapshot>>,
    announced_targets: Vec<crate::game_state::Target>,
    tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    effect_outcomes: HashMap<crate::effect::EffectId, crate::effect::EffectOutcome>,
    provenance: ProvNodeId,
    interactive_mana_exclusions: Option<Vec<ObjectId>>,
    execution_inputs: Option<Box<crate::effects::PaymentExecutionInputs>>,
}

impl std::fmt::Debug for CostContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CostContext")
            .field("source", &self.source)
            .field("payer", &self.payer)
            .field("x_value", &self.x_value)
            .field("reason", &self.reason)
            .field("pre_chosen_cards", &self.pre_chosen_cards)
            .field(
                "tagged_objects",
                &self.tagged_objects.keys().collect::<Vec<_>>(),
            )
            .field(
                "effect_outcomes",
                &self.effect_outcomes.keys().collect::<Vec<_>>(),
            )
            .field("provenance", &self.provenance)
            .finish()
    }
}

impl<'dm> CostContext<'dm> {
    pub(crate) fn checkpoint(&self) -> CostContextCheckpoint {
        CostContextCheckpoint::capture(self)
    }

    /// Create a new cost context with a decision maker.
    pub fn new(
        source: ObjectId,
        payer: PlayerId,
        decision_maker: &'dm mut dyn crate::decision::DecisionMaker,
    ) -> Self {
        Self {
            source,
            payer,
            source_snapshot: None,
            replacement: crate::effects::ReplacementExecutionContext::default(),
            x_value: None,
            reason: PaymentReason::Other,
            requesting_effect_cause: None,
            decision_maker,
            pre_chosen_cards: Vec::new(),
            prospective_cost_payment: false,
            announced_targets: Vec::new(),
            tagged_objects: HashMap::new(),
            effect_outcomes: HashMap::new(),
            provenance: ProvNodeId::default(),
            interactive_mana_exclusions: None,
            execution_inputs: None,
            reserved_tap_sources: Vec::new(),
            completed_sacrifice: None,
        }
    }

    /// Transfer the requesting instruction's captured inputs into a payment
    /// frame before borrowing its decision maker. Announced targets remain
    /// distinct from any preselected cost objects.
    pub(crate) fn from_execution_context(
        source: ObjectId,
        payer: PlayerId,
        reason: PaymentReason,
        execution: &'dm mut crate::effects::ExecutionContext<'_>,
    ) -> Self {
        let announced_targets = execution
            .announced_targets
            .as_deref()
            .unwrap_or(&execution.targets)
            .iter()
            .map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => {
                    crate::game_state::Target::Object(*id)
                }
                crate::effects::ResolvedTarget::Player(player) => {
                    crate::game_state::Target::Player(*player)
                }
            })
            .collect();
        let pre_chosen_cards = if execution.targets_are_cost_choices {
            execution
                .targets
                .iter()
                .filter_map(|target| match target {
                    crate::effects::ResolvedTarget::Object(id) => Some(*id),
                    crate::effects::ResolvedTarget::Player(_) => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            source,
            payer,
            reason,
            source_snapshot: execution.source_snapshot.clone(),
            prospective_cost_payment: execution.prospective_cost_payment,
            reserved_tap_sources: Vec::new(),
            completed_sacrifice: None,
            replacement: execution.replacement.clone(),
            x_value: execution.x_value,
            requesting_effect_cause: Some(execution.cause.clone()),
            pre_chosen_cards,
            announced_targets,
            tagged_objects: execution.tagged_objects.clone(),
            effect_outcomes: execution.effect_outcomes.clone(),
            provenance: execution.provenance,
            interactive_mana_exclusions: None,
            execution_inputs: Some(Box::new(crate::effects::PaymentExecutionInputs::capture(
                execution,
            ))),
            decision_maker: &mut *execution.decision_maker,
        }
    }

    pub(crate) fn execution_bindings(&self) -> CostExecutionBindings {
        CostExecutionBindings {
            source: self.source,
            payer: self.payer,
            cause: self.event_cause(),
            reason: self.reason,
            source_snapshot: self.source_snapshot.clone(),
            prospective_cost_payment: self.prospective_cost_payment,
            replacement: self.replacement.clone(),
            x_value: self.x_value,
            chosen: self
                .pre_chosen_cards
                .iter()
                .copied()
                .map(crate::effects::ResolvedTarget::Object)
                .collect(),
            announced: self
                .announced_targets
                .iter()
                .map(|target| match target {
                    crate::game_state::Target::Object(id) => {
                        crate::effects::ResolvedTarget::Object(*id)
                    }
                    crate::game_state::Target::Player(player) => {
                        crate::effects::ResolvedTarget::Player(*player)
                    }
                })
                .collect(),
            tags: self.tagged_objects.clone(),
            outcomes: self.effect_outcomes.clone(),
            provenance: self.provenance,
            inputs: self.execution_inputs.clone(),
        }
    }

    /// Read-only feasibility has no decision authority. An unannounced X uses
    /// zero for the initial offer; payment retains the actual announcement.
    pub(crate) fn with_execution_context<T>(
        &self,
        query: impl FnOnce(&mut crate::effects::ExecutionContext) -> T,
    ) -> T {
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        let mut execution = self
            .execution_bindings()
            .execution_context(&mut decision_maker);
        execution.x_value.get_or_insert(0);
        query(&mut execution)
    }

    /// Set the X value.
    pub fn with_x(mut self, x: u32) -> Self {
        self.x_value = Some(x);
        self
    }

    /// Set the payment reason.
    pub fn with_reason(mut self, reason: PaymentReason) -> Self {
        self.reason = reason;
        self
    }

    /// Keep the requesting effect's captured source and controller while
    /// preserving the fact that the resulting action pays a cost.
    pub fn event_cause(&self) -> crate::events::cause::EventCause {
        payment_event_cause(
            self.source,
            self.payer,
            self.reason,
            self.requesting_effect_cause.as_ref(),
        )
    }

    /// Set pre-chosen cards for costs that require card selection.
    pub fn with_pre_chosen_cards(mut self, cards: Vec<ObjectId>) -> Self {
        self.pre_chosen_cards = cards;
        self
    }

    /// Set provenance parent for emitted events.
    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.provenance = provenance;
        self
    }
}

/// A context for checking costs without a decision maker.
///
/// This is used by query functions (like `can_pay_cost`, `compute_legal_actions`)
/// that need to check if costs CAN be paid but don't actually pay them.
/// Since `can_pay()` implementations never use the decision_maker field,
/// this is safe for all read-only cost checking operations.
pub struct CostCheckContext {
    /// The source object (permanent or spell whose cost is being checked).
    pub source: ObjectId,
    /// The player whose cost payment is being checked.
    pub payer: PlayerId,
    /// X value for variable costs.
    pub x_value: Option<u32>,
    /// Why this cost would be paid.
    pub reason: PaymentReason,
    /// Pre-chosen cards (usually empty for checking).
    pub pre_chosen_cards: Vec<ObjectId>,
}

impl CostCheckContext {
    /// Create a new cost check context.
    pub fn new(source: ObjectId, payer: PlayerId) -> Self {
        Self {
            source,
            payer,
            x_value: None,
            reason: PaymentReason::Other,
            pre_chosen_cards: Vec::new(),
        }
    }

    /// Set the X value.
    pub fn with_x(mut self, x: u32) -> Self {
        self.x_value = Some(x);
        self
    }

    /// Set the payment reason.
    pub fn with_reason(mut self, reason: PaymentReason) -> Self {
        self.reason = reason;
        self
    }

    /// Create a temporary CostContext for use with can_pay checking.
    ///
    /// This is safe because can_pay implementations never use decision_maker.
    /// The returned context uses a dummy decision maker that would panic if
    /// any actual decisions were attempted (which should never happen in can_pay).
    pub fn as_cost_context<'a>(
        &self,
        dm: &'a mut dyn crate::decision::DecisionMaker,
    ) -> CostContext<'a> {
        CostContext {
            source: self.source,
            source_snapshot: None,
            replacement: crate::effects::ReplacementExecutionContext::default(),
            payer: self.payer,
            x_value: self.x_value,
            reason: self.reason,
            requesting_effect_cause: None,
            decision_maker: dm,
            pre_chosen_cards: self.pre_chosen_cards.clone(),
            prospective_cost_payment: false,
            announced_targets: Vec::new(),
            tagged_objects: HashMap::new(),
            effect_outcomes: HashMap::new(),
            provenance: ProvNodeId::default(),
            interactive_mana_exclusions: None,
            execution_inputs: None,
            reserved_tap_sources: Vec::new(),
            completed_sacrifice: None,
        }
    }
}

/// Check if a cost can be paid using a check-only context.
///
/// This is a convenience function for query operations that don't have
/// access to a decision maker. Since `can_pay` never uses the decision_maker,
/// this is safe.
pub fn can_pay_with_check_context(
    cost: &dyn CostPayer,
    game: &crate::game_state::GameState,
    ctx: &CostCheckContext,
) -> Result<(), crate::cost::CostPaymentError> {
    // Create a temporary AutoPass decision maker just for the check
    let mut auto_dm = crate::decision::CliDecisionMaker;
    let cost_ctx = ctx.as_cost_context(&mut auto_dm);
    cost.can_pay(game, &cost_ctx)
}

/// Check if a cost can potentially be paid using a check-only context.
pub fn can_potentially_pay_with_check_context(
    cost: &dyn CostPayer,
    game: &crate::game_state::GameState,
    ctx: &CostCheckContext,
) -> Result<(), crate::cost::CostPaymentError> {
    let mut auto_dm = crate::decision::CliDecisionMaker;
    let cost_ctx = ctx.as_cost_context(&mut auto_dm);
    cost.can_potentially_pay(game, &cost_ctx)
}

/// Trait for paying costs.
///
/// All modular costs implement this trait. Each cost is responsible for:
/// - Checking if it can be paid (right now)
/// - Checking if it could potentially be paid (with untapped mana sources)
/// - Actually paying the cost
/// - Providing display text
///
/// # Example
///
/// ```ignore
/// use ironsmith::costs::CostPayer;
///
/// #[derive(Debug, Clone)]
/// struct MyCost;
///
/// impl CostPayer for MyCost {
///     fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
///         // Validate whether the cost can be paid
///         Ok(())
///     }
///
///     fn pay(&self, game: &mut GameState, ctx: &mut CostContext) -> Result<CostPaymentResult, CostPaymentError> {
///         // Mutate the game state to pay the cost
///         Ok(CostPaymentResult::Paid)
///     }
///
///     fn display(&self) -> String {
///         "My cost".to_string()
///     }
/// }
/// ```
impl CostContext<'_> {
    pub(crate) fn capture_execution_context(
        &mut self,
    ) -> crate::effects::ExecutionContextCheckpoint {
        let bindings = self.execution_bindings();
        let execution = bindings.execution_context(&mut *self.decision_maker);
        crate::effects::ExecutionContextCheckpoint::capture(&execution)
    }
}

pub trait CostPayerClone {
    /// Clone this cost into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn CostPayer>;
}

impl<T> CostPayerClone for T
where
    T: CostPayer + Clone + 'static,
{
    fn clone_boxed(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }
}

pub trait CostPayer: std::fmt::Debug + Send + Sync + CostPayerClone + Any {
    /// Read-only nominal X exported by this owner for later prepared components.
    /// Actual publication remains with original payment acknowledgement.
    fn payment_x_from_prepared_payment(
        &self,
        _proposal: &dyn crate::effects::SimultaneousEffectProposal,
        _execution: &crate::effects::ExecutionContext,
    ) -> Result<Option<u32>, CostPaymentError> {
        Ok(None)
    }

    /// Validate the nominal payment receipt through this cost's semantic owner.
    /// Ordinary and prepared adapters use the same policy; actual replacement
    /// actions need not equal the requested payment. Validation only inspects
    /// the receipt and must not execute another action or publish observations.
    fn validate_payment_outcome(
        &self,
        _outcome: &crate::effect::EffectOutcome,
    ) -> Result<(), CostPaymentError> {
        Ok(())
    }

    /// Export an authored cost X through the same owner in ordinary and
    /// prepared execution. No value means this payment did not announce X.
    fn payment_x_from_outcome(
        &self,
        _outcome: &crate::effect::EffectOutcome,
        _execution: &crate::effects::ExecutionContext,
    ) -> Result<Option<u32>, CostPaymentError> {
        Ok(None)
    }

    /// Preserve owner-specific payment outputs before subsequent cost frames.
    /// Ordinary and prepared adapters invoke this after nominal validation and
    /// X export; the hook may update bindings but cannot mutate the world.
    fn finalize_payment_bindings(
        &self,
        _game: &GameState,
        _outcome: &crate::effect::EffectOutcome,
        _execution: &mut crate::effects::ExecutionContext,
        _payment_x: Option<u32>,
    ) -> Result<(), CostPaymentError> {
        Ok(())
    }

    /// Whether this payer can separate original payment from deferred additions
    /// while preserving affordability and nominal acknowledgement. Total-cost
    /// composition checks every component before preparing any of them.
    fn supports_prepared_payment(&self) -> bool {
        false
    }

    /// Prepare a payment in the total owner's payer/reason/cause scope.
    /// Returning a proposal must not execute payment or replacement additions;
    /// its original and completion phases own those actions and receipts.
    fn prepare_simultaneous_payment(
        &self,
        _game: &GameState,
        _execution: &mut crate::effects::ExecutionContext,
    ) -> Result<
        Option<Box<dyn crate::effects::SimultaneousEffectProposal>>,
        crate::effects::ExecutionError,
    > {
        Ok(None)
    }

    /// Check if this cost can be paid RIGHT NOW.
    ///
    /// For mana costs, this checks if the mana is in the pool.
    /// For tap costs, this checks if the permanent is untapped.
    /// For sacrifice costs, this checks if valid targets exist.
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError>;

    /// Check if this cost COULD potentially be paid.
    ///
    /// For mana costs, this includes untapped mana sources.
    /// For non-mana costs, this typically equals `can_pay()`.
    ///
    /// This is used for UI to show actions that could be afforded after
    /// tapping mana sources.
    fn can_potentially_pay(
        &self,
        game: &GameState,
        ctx: &CostContext,
    ) -> Result<(), CostPaymentError> {
        // Default implementation: same as can_pay
        self.can_pay(game, ctx)
    }

    /// Potential affordability using an existing derived view. Non-mana
    /// owners retain their normal query; mana owners can reuse the caller's
    /// source-discovery cache without taking over cost traversal.
    fn can_potentially_pay_with_query(
        &self,
        game: &GameState,
        ctx: &CostContext,
        _query: &crate::costs::PotentialManaQuery<'_, '_>,
    ) -> Result<(), CostPaymentError> {
        self.can_potentially_pay(game, ctx)
    }

    /// Actually pay the cost, mutating game state.
    ///
    /// # Errors
    ///
    /// Returns an error if the cost cannot be paid.
    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError>;

    /// Retain actual outputs without changing legacy owners' payment contract.
    /// Absence is explicit; do not reconstruct packets from mutable history.
    fn pay_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentReceipt, CostPaymentError> {
        self.pay(game, ctx).map(CostPaymentReceipt::new)
    }

    /// Clone this cost into a boxed trait object.
    fn clone_box(&self) -> Box<dyn CostPayer> {
        CostPayerClone::clone_boxed(self)
    }

    /// Human-readable display text for this cost.
    ///
    /// Examples: "{T}", "Pay 2 life", "{2}{W}", "Sacrifice a creature"
    fn display(&self) -> String;

    /// Returns true if this is a mana cost.
    ///
    /// Used for separating mana costs from other costs in display.
    fn is_mana_cost(&self) -> bool {
        false
    }

    /// Returns true if this cost requires tapping the source.
    ///
    /// Used for display formatting ("Tap X" vs "X (cost description)").
    fn requires_tap(&self) -> bool {
        false
    }

    /// Returns true if this cost requires untapping the source.
    fn requires_untap(&self) -> bool {
        false
    }

    /// Returns true if this is a life payment cost.
    fn is_life_cost(&self) -> bool {
        false
    }

    /// Returns the life amount if this is a life payment cost.
    fn life_amount(&self) -> Option<u32> {
        None
    }

    /// Returns true if this is a sacrifice self cost.
    fn is_sacrifice_self(&self) -> bool {
        false
    }

    /// Returns true if this is a sacrifice (other permanent) cost.
    fn is_sacrifice(&self) -> bool {
        false
    }

    /// Returns the sacrifice filter if this is a sacrifice cost.
    fn sacrifice_filter(&self) -> Option<&crate::filter::ObjectFilter> {
        None
    }

    /// Returns true if this is a discard cost.
    fn is_discard(&self) -> bool {
        false
    }

    /// Returns the discard details (count, optional card type) if this is a discard cost.
    fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        None
    }

    /// Returns true if this is an exile from hand cost.
    fn is_exile_from_hand(&self) -> bool {
        false
    }

    /// Returns the exile from hand details (count, color filter) if applicable.
    fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        None
    }

    /// Returns the exile from graveyard details (count, allowed card types) if applicable.
    fn exile_from_graveyard_details(&self) -> Option<(u32, &[crate::types::CardType])> {
        None
    }

    /// Returns true if this is a remove counters cost.
    fn is_remove_counters(&self) -> bool {
        false
    }

    /// Returns the mana cost if this is a mana payment cost.
    fn mana_cost(&self) -> Option<&crate::mana::ManaCost> {
        None
    }

    /// Returns true if this cost requires player interaction/choice.
    ///
    /// Immediate costs (tap, untap, life, remove counters, sacrifice self) return false.
    /// Costs needing selection (mana payment, sacrifice target) return true.
    fn needs_player_choice(&self) -> bool {
        false
    }

    /// Returns how this cost should be processed during cost payment.
    ///
    /// This determines the game loop's handling:
    /// - `Immediate`: Pay directly via `pay()`
    /// - `ManaPayment`: Use mana payment UI
    /// - `SacrificeTarget`: Use target selection UI
    /// - `DiscardCards`: Use card selection UI
    /// - `ExileFromHand`: Use card selection UI
    /// - `InlineWithTriggers`: Handle inline for trigger detection
    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        // Default: immediate payment
        crate::costs::CostProcessingMode::Immediate
    }

    /// Returns the backing effect when this cost is effect-backed.
    ///
    /// Default is `None` for non-effect costs.
    fn effect_ref(&self) -> Option<&crate::effect::Effect> {
        None
    }

    /// Downcast support for staged cost handling.
    fn as_any(&self) -> &dyn Any;
}

// Implement Clone for Box<dyn CostPayer>
impl Clone for Box<dyn CostPayer> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test cost that always succeeds.
    #[derive(Debug, Clone)]
    struct TestCost;

    impl CostPayer for TestCost {
        fn can_pay(&self, _game: &GameState, _ctx: &CostContext) -> Result<(), CostPaymentError> {
            Ok(())
        }

        fn pay(
            &self,
            _game: &mut GameState,
            _ctx: &mut CostContext,
        ) -> Result<CostPaymentResult, CostPaymentError> {
            Ok(CostPaymentResult::Paid)
        }

        fn display(&self) -> String {
            "Test".to_string()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_cost_payer_trait_is_object_safe() {
        // This test verifies that CostPayer can be used as a trait object
        let cost: Box<dyn CostPayer> = Box::new(TestCost);
        assert!(format!("{:?}", cost).contains("TestCost"));
        assert_eq!(cost.display(), "Test");
    }

    #[test]
    fn test_box_dyn_cost_payer_clone() {
        let cost: Box<dyn CostPayer> = Box::new(TestCost);
        let cloned = cost.clone();
        assert!(format!("{:?}", cloned).contains("TestCost"));
    }

    #[test]
    fn test_cost_context_creation() {
        let source = ObjectId::from_raw(1);
        let payer = PlayerId::from_index(0);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, payer, &mut dm);

        assert_eq!(ctx.source, source);
        assert_eq!(ctx.payer, payer);
        assert!(ctx.x_value.is_none());
    }

    #[test]
    fn test_cost_context_with_x() {
        let source = ObjectId::from_raw(1);
        let payer = PlayerId::from_index(0);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, payer, &mut dm).with_x(5);

        assert_eq!(ctx.x_value, Some(5));
    }
}
