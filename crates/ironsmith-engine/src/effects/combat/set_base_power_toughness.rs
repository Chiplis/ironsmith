//! Set base power/toughness effect implementation.

use crate::card::PtValue;
use crate::continuous::{EffectTarget, Modification, PtSublayer};
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::helpers::{resolve_single_object_for_effect, resolve_value};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::types::CardType;
pub use ironsmith_core::SetBasePowerToughnessEffect;

/// Effect that sets a creature's base power and toughness.
///
/// Creates a continuous effect in layer 7b ("setting" sublayer).
impl EffectExecutor for SetBasePowerToughnessEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let base_power = resolve_value(game, &self.power, ctx)?;
        let base_toughness = resolve_value(game, &self.toughness, ctx)?;

        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;

        let target = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;
        if !target.has_card_type(CardType::Creature) {
            return Ok(EffectOutcome::target_invalid());
        }
        if matches!(self.duration, Until::Forever) {
            let target = game
                .object_mut(target_id)
                .ok_or(ExecutionError::ObjectNotFound(target_id))?;
            target.base_power = Some(PtValue::Fixed(base_power));
            target.base_toughness = Some(PtValue::Fixed(base_toughness));
            game.refresh_continuous_state();
            return Ok(EffectOutcome::resolved());
        }

        let apply = ApplyContinuousEffect::new(
            EffectTarget::Specific(target_id),
            Modification::SetPowerToughness {
                power: Value::Fixed(base_power),
                toughness: Value::Fixed(base_toughness),
                sublayer: PtSublayer::Setting,
            },
            self.duration.clone(),
        );
        execute_effect(game, &Effect::new(apply), ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature"
    }
}
