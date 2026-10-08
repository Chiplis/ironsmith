//! "Prevent X of that damage" for the damage one wrapped instruction deals.
//!
//! CR 615.1 / 615.7: a prevention effect that applies to a specific amount
//! of damage. The shield exists only while the wrapped damage instruction
//! deals its damage (Errant Minion, Power Leak: "This Aura deals 2 damage to
//! that player. Prevent X of that damage, ..."), so an unused remainder never
//! reaches later damage.
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::combat::prevention_helpers::register_prevention_shield;
use crate::effects::helpers::resolve_value;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionTarget};
use crate::target::ChooseSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamagePortionEffect {
    /// How much of the wrapped damage is prevented.
    pub amount: Value,
    /// The damage instruction(s) whose damage the shield covers.
    pub effects: Vec<Effect>,
}

impl PreventDamagePortionEffect {
    pub fn new(amount: Value, effects: Vec<Effect>) -> Self {
        Self { amount, effects }
    }
}

impl EffectExecutor for PreventDamagePortionEffect {
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
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        let shield = (amount > 0).then(|| {
            register_prevention_shield(
                game,
                ctx,
                PreventionTarget::All,
                Some(amount),
                Until::EndOfTurn,
                DamageFilter::all(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
        });
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
        // The shield covers only "that damage": end it whether or not the
        // wrapped instruction used all of it.
        if let Some(shield) = shield {
            game.effect_store.prevention_effects.remove_shield(shield);
        }
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
