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
    /// Whether the mode labels are mandatory Spree additional costs.
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

/// Whether a target requirement can reuse an earlier compatible target slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetReusePolicy {
    /// Reuse a compatible target slot already declared by an earlier effect.
    ReuseCompatiblePrevious,
    /// Always declare a new target slot even if the spec matches an earlier one.
    AlwaysDeclareNew,
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

/// A fully determined part of one simultaneous multi-player action.
///
/// Implementations are prepared for every affected player against the same
/// immutable game state. `commit` must apply only the already-determined
/// mutation: it must not ask a new question or recalculate a value from game
/// state changed by an earlier proposal in the same batch.
pub trait SimultaneousEffectProposal: std::fmt::Debug + Send {
    fn commit(self: Box<Self>, game: &mut GameState) -> Result<EffectOutcome, ExecutionError>;
}

pub trait EffectExecutor:
    std::fmt::Debug + Any + Send + Sync + EffectExecutorClone + 'static
{
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

    /// Whether this effect can prepare an immutable proposal for a generic
    /// simultaneous each-player action (CR 101.4, 608.2f).
    fn supports_simultaneous_player_action(&self) -> bool {
        false
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

    /// Get the target specification for this effect, if it has one.
    ///
    /// Used for target selection during spell/ability resolution.
    /// Returns `None` for effects that don't require targeting.
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

    /// Visit immediately nested runtime effects, if this effect is a wrapper or
    /// composition effect.
    ///
    /// Implementations should expose only direct children. Recursive traversal is
    /// provided by the default capability helpers below.
    fn visit_child_effects(&self, _visitor: &mut dyn FnMut(&Effect)) {}

    /// Whether this effect is a resolution prelude that only prepares context
    /// for following effects, such as tagging an object for a self-replacement.
    fn is_resolution_prelude(&self) -> bool {
        false
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
    /// Not enough cards to exile
    NotEnoughCards,
    /// Cannot sacrifice required permanent
    CannotSacrifice,
    /// Generic error with message
    Other(String),
}

/// Additional behavior required for effects that can be used as costs.
pub trait CostExecutableEffect: EffectExecutor {
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
    fn target_only_profiles_force_new_target_slots() {
        let effect = crate::effect::Effect::new(crate::effects::TargetOnlyEffect::new(
            crate::target::ChooseSpec::AnyTarget,
        ));
        let profile = effect
            .target_selection_profile()
            .expect("target-only effect should expose target profile");

        assert_eq!(profile.spec, &crate::target::ChooseSpec::AnyTarget);
        assert_eq!(profile.reuse_policy, TargetReusePolicy::AlwaysDeclareNew);
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
