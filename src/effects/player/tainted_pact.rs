use std::collections::HashSet;

use crate::decisions::context::BooleanContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct TaintedPactEffect;

impl EffectExecutor for TaintedPactEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut seen_names = HashSet::new();

        loop {
            let Some(top_card) = game
                .player(ctx.controller)
                .and_then(|player| player.library.last().copied())
            else {
                return Ok(EffectOutcome::resolved());
            };

            let Some((exiled_id, final_zone)) = game.move_object_with_commander_options(
                top_card,
                Zone::Exile,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) else {
                return Ok(EffectOutcome::resolved());
            };
            if final_zone != Zone::Exile {
                return Ok(EffectOutcome::resolved());
            }

            let Some(name) = game.object(exiled_id).map(|obj| obj.name.clone()) else {
                return Ok(EffectOutcome::resolved());
            };
            if !seen_names.insert(name.clone()) {
                return Ok(EffectOutcome::resolved());
            }

            let choice_ctx = BooleanContext::new(
                ctx.controller,
                Some(exiled_id),
                format!("Put {name} into your hand?"),
            );
            let put_into_hand = ctx.decision_maker.decide_boolean(game, &choice_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if !put_into_hand {
                continue;
            }

            let Some((_new_id, hand_zone)) = game.move_object_with_commander_options(
                exiled_id,
                Zone::Hand,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) else {
                return Ok(EffectOutcome::resolved());
            };
            return Ok(EffectOutcome::count((hand_zone == Zone::Hand) as i32));
        }
    }
}
