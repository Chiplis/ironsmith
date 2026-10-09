//! A die-result table row that fixes X for the program it governs.
//!
//! "Roll a d20. ... You may cast up to X ... . 1—9 | X is one. 10—19 | X is
//! two. 20 | X is three." (Wand of Wonder): the row the result selects runs
//! the governed program with X equal to its value (CR 706.2, 107.3).
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct BindXValueEffect {
    pub value: Value,
    pub effects: Vec<Effect>,
}

impl BindXValueEffect {
    pub fn new(value: Value, effects: Vec<Effect>) -> Self {
        Self { value, effects }
    }
}

impl EffectExecutor for BindXValueEffect {
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
        let value = resolve_value(game, &self.value, ctx)?.max(0) as u32;
        let saved_x = ctx.x_value;
        ctx.x_value = Some(value);
        let mut outcomes = Vec::with_capacity(self.effects.len());
        let mut failure = None;
        for effect in &self.effects {
            match execute_effect(game, effect, ctx) {
                Ok(outcome) => outcomes.push(outcome),
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
            if ctx.decision_maker.awaiting_choice() {
                break;
            }
        }
        ctx.x_value = saved_x;
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }
    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.effects])
    }
    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}
