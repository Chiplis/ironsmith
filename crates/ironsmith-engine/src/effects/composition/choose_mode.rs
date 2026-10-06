//! ChooseMode effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::executor_trait::{ModalEffectSpec, ModalSpec};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub type ChooseModeEffect = ironsmith_core::ChooseModeEffect<crate::effect::Effect>;

impl EffectExecutor for ChooseModeEffect {
    fn supports_prepared_action_program(&self) -> bool {
        self.common_prefix_effects
            .iter()
            .chain(self.modes.iter().flat_map(|mode| mode.effects.iter()))
            .all(super::action_program::action_program_child_is_prepared)
    }

    fn select_prepared_action_program(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        Ok(
            super::choose_mode_runtime::select_mode_program(self, game, ctx)?.map(|program| {
                super::choose_mode_runtime::selected_mode_cursor(self, program, ctx)
            }),
        )
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // The mode choice happens at commit; earlier read-only choosers in the
        // same action unit (e.g. pile splitting) already ran for every player.
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for effect in &self.common_prefix_effects {
            visitor(effect);
        }
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

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::choose_mode_runtime::run_choose_mode_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Action,
        )
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
            spree: self.spree,
            mode_additional_mana_costs: self.mode_additional_mana_costs.clone(),
            distinct_player_targets_per_mode: self.distinct_player_targets_per_mode,
            conditional_mode_range: self.conditional_mode_range.clone(),
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
            spree: self.spree,
            mode_additional_mana_costs: &self.mode_additional_mana_costs,
            disallow_previously_chosen_modes: self.disallow_previously_chosen_modes,
            disallow_previously_chosen_modes_this_turn: self
                .disallow_previously_chosen_modes_this_turn,
            distinct_player_targets_per_mode: self.distinct_player_targets_per_mode,
            conditional_mode_range: self.conditional_mode_range.as_ref(),
        })
    }
}

impl CostExecutableEffect for ChooseModeEffect {
    fn execute_payment_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::choose_mode_runtime::run_choose_mode_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Payment,
        )
    }

    fn payment_bindings_are_owned_by_children(&self) -> bool {
        true
    }

    fn canonical_cost_effect(&self) -> Option<crate::effect::Effect> {
        let mut replacement = self.clone();
        let mut changed = false;
        if let Some(effects) = crate::effects::canonical_cost_children(&self.common_prefix_effects)
        {
            replacement.common_prefix_effects = effects;
            changed = true;
        }
        for mode in &mut replacement.modes {
            if let Some(effects) = crate::effects::canonical_cost_children(&mode.effects) {
                mode.effects = effects;
                changed = true;
            }
        }
        changed.then(|| crate::effect::Effect::new(replacement))
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

    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
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

        if min_modes == 0 {
            return crate::costs::check_effect_cost_program(
                &self.common_prefix_effects,
                game,
                ctx,
                reason,
            );
        }
        let mut legal_mode_count = 0;
        for mode in &self.modes {
            let program = self
                .common_prefix_effects
                .iter()
                .chain(&mode.effects)
                .cloned()
                .collect::<Vec<_>>();
            // The shared cost query owner isolates speculative bindings.
            let result = crate::costs::check_effect_cost_program(&program, game, ctx, reason);
            if result.is_ok() {
                legal_mode_count += 1;
            }
        }

        if legal_mode_count >= min_modes {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "not enough legal cost options available".to_string(),
            ))
        }
    }
}
