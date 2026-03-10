//! ChooseMode effect implementation.

use crate::effect::{EffectMode, EffectOutcome, Value};
use crate::effects::executor_trait::ModalSpec;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Effect that presents modal choices to the player.
///
/// For modal spells like "Choose one —" or "Choose one or more —".
///
/// # Fields
///
/// * `modes` - Available mode choices
/// * `choose_count` - Maximum number of modes to choose
/// * `min_choose_count` - Minimum modes to choose (defaults to choose_count if None)
///
/// # Example
///
/// ```ignore
/// // "Choose one —"
/// let effect = ChooseModeEffect::choose_one(vec![
///     EffectMode::new("Deal 3 damage to any target", vec![Effect::deal_damage(3, ...)]),
///     EffectMode::new("Gain 3 life", vec![Effect::gain_life(3)]),
/// ]);
///
/// // "Choose one or both —"
/// let effect = ChooseModeEffect::choose_up_to(2, 1, vec![...]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseModeEffect {
    /// Available mode choices.
    pub modes: Vec<EffectMode>,
    /// Maximum number of modes to choose.
    pub choose_count: Value,
    /// Minimum modes to choose. If None, defaults to choose_count (exact choice).
    pub min_choose_count: Option<Value>,
    /// Whether the same mode can be chosen more than once.
    pub allow_repeated_modes: bool,
    /// Whether chosen modes are disallowed for future activations of the same ability.
    pub disallow_previously_chosen_modes: bool,
    /// Whether "previously chosen" tracking resets each turn.
    pub disallow_previously_chosen_modes_this_turn: bool,
}

impl ChooseModeEffect {
    /// Create a new ChooseMode effect.
    pub fn new(
        modes: Vec<EffectMode>,
        choose_count: Value,
        min_choose_count: Option<Value>,
    ) -> Self {
        Self {
            modes,
            choose_count,
            min_choose_count,
            allow_repeated_modes: false,
            disallow_previously_chosen_modes: false,
            disallow_previously_chosen_modes_this_turn: false,
        }
    }

    /// Create a "choose one" modal effect.
    pub fn choose_one(modes: Vec<EffectMode>) -> Self {
        Self::new(modes, Value::Fixed(1), None)
    }

    /// Create a "choose X" modal effect with exact count required.
    pub fn choose_exactly(count: impl Into<Value>, modes: Vec<EffectMode>) -> Self {
        Self::new(modes, count.into(), None)
    }

    /// Create a "choose up to X" or "choose one or more" modal effect.
    pub fn choose_up_to(
        max: impl Into<Value>,
        min: impl Into<Value>,
        modes: Vec<EffectMode>,
    ) -> Self {
        Self::new(modes, max.into(), Some(min.into()))
    }

    /// Allow selecting the same mode more than once.
    pub fn with_repeated_modes(mut self) -> Self {
        self.allow_repeated_modes = true;
        self
    }

    /// Require each activation to choose a mode that has not been chosen before.
    pub fn with_previously_unchosen_modes_only(mut self) -> Self {
        self.disallow_previously_chosen_modes = true;
        self.disallow_previously_chosen_modes_this_turn = false;
        self
    }

    /// Require each activation this turn to choose a mode not chosen earlier this turn.
    pub fn with_previously_unchosen_modes_only_this_turn(mut self) -> Self {
        self.disallow_previously_chosen_modes = true;
        self.disallow_previously_chosen_modes_this_turn = true;
        self
    }
}

impl EffectExecutor for ChooseModeEffect {
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
        super::choose_mode_runtime::run_choose_mode(self, game, ctx)
    }

    fn get_modal_spec(&self) -> Option<ModalSpec> {
        Some(ModalSpec {
            mode_descriptions: self.modes.iter().map(|m| m.description.clone()).collect(),
            max_modes: self.choose_count.clone(),
            min_modes: self
                .min_choose_count
                .clone()
                .unwrap_or_else(|| self.choose_count.clone()),
        })
    }
}

impl CostExecutableEffect for ChooseModeEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        let max_modes = match self.choose_count {
            Value::Fixed(value) => value.max(0) as usize,
            _ => {
                return Err(CostValidationError::Other(
                    "dynamic modal cost counts are not supported".to_string(),
                ));
            }
        };
        let min_modes = match &self.min_choose_count {
            Some(Value::Fixed(value)) => (*value).max(0) as usize,
            Some(_) => {
                return Err(CostValidationError::Other(
                    "dynamic modal cost counts are not supported".to_string(),
                ));
            }
            None => max_modes,
        };

        let legal_mode_count = self
            .modes
            .iter()
            .filter(|mode| {
                mode.effects.iter().all(|effect| {
                    effect.0.as_cost_executable().is_some_and(|_| {
                        effect
                            .0
                            .can_execute_as_cost(game, source, controller)
                            .is_ok()
                    })
                })
            })
            .count();

        if legal_mode_count >= min_modes {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "not enough legal cost options available".to_string(),
            ))
        }
    }
}
