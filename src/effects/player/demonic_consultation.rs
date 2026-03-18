use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct DemonicConsultationEffect {
    pub chosen_name_tag: TagKey,
}

impl DemonicConsultationEffect {
    pub fn new(chosen_name_tag: impl Into<TagKey>) -> Self {
        Self {
            chosen_name_tag: chosen_name_tag.into(),
        }
    }
}

impl EffectExecutor for DemonicConsultationEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chosen_name = ctx
            .tagged_objects
            .get(&self.chosen_name_tag)
            .and_then(|snapshots| snapshots.first())
            .map(|snapshot| snapshot.name.clone())
            .ok_or_else(|| ExecutionError::TagNotFound(self.chosen_name_tag.to_string()))?;

        let exile_top = |game: &mut GameState,
                         ctx: &mut ExecutionContext,
                         player: crate::ids::PlayerId|
         -> Option<crate::ids::ObjectId> {
            let top = game.player(player).and_then(|state| state.library.last().copied())?;
            let (new_id, final_zone) = game.move_object_with_commander_options(
                top,
                Zone::Exile,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            )?;
            (final_zone == Zone::Exile).then_some(new_id)
        };

        for _ in 0..6 {
            if exile_top(game, ctx, ctx.controller).is_none() {
                return Ok(EffectOutcome::resolved());
            }
        }

        while let Some(exiled_id) = exile_top(game, ctx, ctx.controller) {
            let is_match = game
                .object(exiled_id)
                .is_some_and(|obj| obj.name == chosen_name);
            if !is_match {
                continue;
            }

            let Some((_new_id, final_zone)) = game.move_object_with_commander_options(
                exiled_id,
                Zone::Hand,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) else {
                return Ok(EffectOutcome::resolved());
            };

            return Ok(EffectOutcome::count((final_zone == Zone::Hand) as i32));
        }

        Ok(EffectOutcome::resolved())
    }
}
