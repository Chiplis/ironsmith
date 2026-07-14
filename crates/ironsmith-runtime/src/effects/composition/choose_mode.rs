//! ChooseMode effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::executor_trait::{ModalEffectSpec, ModalSpec};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub type ChooseModeEffect = ironsmith_core::ChooseModeEffect<crate::effect::Effect>;

impl EffectExecutor for ChooseModeEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for mode in &self.modes {
            for effect in &mode.effects {
                visitor(effect);
            }
        }
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
        if self.chooser.is_some() {
            return None;
        }
        Some(ModalSpec {
            mode_descriptions: self.modes.iter().map(|m| m.source_text.clone()).collect(),
            max_modes: self.choose_count.clone(),
            min_modes: self.min_choose_count.clone(),
            allow_repeated_modes: self.allow_repeated_modes,
            mode_point_costs: self.mode_point_costs.clone(),
            distinct_player_targets_per_mode: self.distinct_player_targets_per_mode,
        })
    }

    fn modal_effect_spec(&self) -> Option<ModalEffectSpec<'_>> {
        if self.chooser.is_some() {
            return None;
        }
        Some(ModalEffectSpec {
            modes: &self.modes,
            max_modes: &self.choose_count,
            min_modes: &self.min_choose_count,
            allow_repeated_modes: self.allow_repeated_modes,
            mode_point_costs: &self.mode_point_costs,
            disallow_previously_chosen_modes: self.disallow_previously_chosen_modes,
            disallow_previously_chosen_modes_this_turn: self
                .disallow_previously_chosen_modes_this_turn,
            distinct_player_targets_per_mode: self.distinct_player_targets_per_mode,
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
        match self.choose_count {
            Value::Fixed(_) => {}
            _ => {
                return Err(CostValidationError::Other(
                    "dynamic modal cost counts are not supported".to_string(),
                ));
            }
        }
        let min_modes = match &self.min_choose_count {
            Value::Fixed(value) => (*value).max(0) as usize,
            _ => {
                return Err(CostValidationError::Other(
                    "dynamic modal cost counts are not supported".to_string(),
                ));
            }
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
