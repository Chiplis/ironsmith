//! ChooseMode effect implementation.

use crate::decisions::{ModesSpec, make_decision, specs::ModeOption};
use crate::effect::{EffectMode, EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::executor_trait::ModalSpec;
use crate::effects::helpers::resolve_value;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::targeting::compute_legal_targets;

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

    /// Check if a mode is legal to choose.
    ///
    /// A mode is illegal if any of its effects requires targets but no legal targets exist.
    fn check_mode_legal(
        game: &GameState,
        mode: &EffectMode,
        controller: PlayerId,
        source: crate::ids::ObjectId,
    ) -> bool {
        for effect in &mode.effects {
            if let Some(target_spec) = effect.0.get_target_spec() {
                let legal_targets =
                    compute_legal_targets(game, target_spec, controller, Some(source));
                // If effect requires targets (min > 0) and none exist, mode is illegal
                if legal_targets.is_empty() {
                    // Check if the effect actually requires at least one target
                    // Most effects require at least one target unless explicitly "up to"
                    // For now, assume any target spec means at least one target is needed
                    return false;
                }
            }
        }
        true
    }
}

impl EffectExecutor for ChooseModeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let max_modes = resolve_value(game, &self.choose_count, ctx)?.max(0) as usize;
        let min_modes = match &self.min_choose_count {
            Some(min_val) => resolve_value(game, min_val, ctx)?.max(0) as usize,
            None => max_modes,
        };

        if self.modes.is_empty() || max_modes == 0 {
            return Ok(EffectOutcome::resolved());
        }

        // Per MTG rule 601.2b, modes are chosen during casting (before targets).
        // Check if modes were pre-chosen during the casting process.
        let chosen_indices: Vec<usize> = if let Some(ref pre_chosen) = ctx.chosen_modes {
            // Use pre-chosen modes from casting
            pre_chosen.clone()
        } else {
            // Fallback: prompt for modes (for direct effect execution in tests or abilities)
            // Convert EffectModes to ModeOptions for the decision
            // Check legality of each mode based on whether targets are available
            let mode_options: Vec<ModeOption> = self
                .modes
                .iter()
                .enumerate()
                .map(|(i, mode)| {
                    ModeOption::with_legality(
                        i,
                        mode.description.clone(),
                        Self::check_mode_legal(game, mode, ctx.controller, ctx.source),
                    )
                })
                .collect();

            // Count how many modes are legal
            let legal_mode_count = mode_options.iter().filter(|m| m.legal).count();

            // If there aren't enough legal modes to meet minimum requirements, fail
            if legal_mode_count < min_modes {
                return Err(ExecutionError::Impossible(
                    "Not enough legal modes available".to_string(),
                ));
            }

            // Ask player which modes to choose using the spec-based system
            let spec = ModesSpec::new(ctx.source, mode_options, min_modes, max_modes);
            make_decision(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            )
        };

        // Filter to valid indices (within bounds)
        let valid_chosen_indices: Vec<usize> = chosen_indices
            .into_iter()
            .filter(|&i| i < self.modes.len())
            .collect();

        // Execute the chosen modes in order and aggregate outcomes
        let mut outcomes = Vec::new();
        for &idx in &valid_chosen_indices {
            if let Some(mode) = self.modes.get(idx) {
                for effect in &mode.effects {
                    outcomes.push(execute_effect(game, effect, ctx)?);
                }
            }
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_mode(description: &str, effects: Vec<Effect>) -> EffectMode {
        EffectMode {
            description: description.to_string(),
            effects,
        }
    }

    #[test]
    fn test_choose_one_auto_first() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        let effect = ChooseModeEffect::choose_one(vec![
            make_mode("Gain 5 life", vec![Effect::gain_life(5)]),
            make_mode("Gain 1 life", vec![Effect::gain_life(1)]),
        ]);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Without decision maker, auto-selects first mode
        assert_eq!(game.player(alice).unwrap().life, initial_life + 5);
    }

    #[test]
    fn test_choose_mode_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ChooseModeEffect::choose_one(vec![]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_choose_mode_zero_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        let effect = ChooseModeEffect::new(
            vec![make_mode("Gain 5 life", vec![Effect::gain_life(5)])],
            Value::Fixed(0),
            None,
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        // No modes should execute
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_choose_up_to_auto_min() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        // Use AutoPassDecisionMaker to auto-select minimum count
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let initial_life = game.player(alice).unwrap().life;

        // Choose one or both (min 1, max 2)
        let effect = ChooseModeEffect::choose_up_to(
            2,
            1,
            vec![
                make_mode("Gain 3 life", vec![Effect::gain_life(3)]),
                make_mode("Gain 2 life", vec![Effect::gain_life(2)]),
            ],
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        // With AutoPassDecisionMaker, auto-selects first min (1) modes
        assert_eq!(game.player(alice).unwrap().life, initial_life + 3);
    }

    #[test]
    fn test_choose_mode_clone_box() {
        let effect =
            ChooseModeEffect::choose_one(vec![make_mode("Test", vec![Effect::gain_life(1)])]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ChooseModeEffect"));
    }
}
