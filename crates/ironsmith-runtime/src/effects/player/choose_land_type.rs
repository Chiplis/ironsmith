//! Choose a land type and store it on the source object.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::types::Subtype;
pub use ironsmith_core::ChooseLandTypeEffect;

fn land_type_options(exclude_basic: bool) -> Vec<Subtype> {
    Subtype::all_land_types()
        .iter()
        .copied()
        .filter(|subtype| {
            !exclude_basic
                || !matches!(
                    subtype,
                    Subtype::Plains
                        | Subtype::Island
                        | Subtype::Swamp
                        | Subtype::Mountain
                        | Subtype::Forest
                )
        })
        .collect()
}

impl EffectExecutor for ChooseLandTypeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let subtype_options = land_type_options(self.exclude_basic);
        if subtype_options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let options = subtype_options
            .iter()
            .enumerate()
            .map(|(idx, subtype)| SelectableOption::new(idx, subtype.to_string()))
            .collect::<Vec<_>>();
        let prompt = if self.exclude_basic {
            "Choose a nonbasic land type"
        } else {
            "Choose a land type"
        };
        let choice_ctx =
            SelectOptionsContext::new(chooser, Some(ctx.source), prompt, options, 1, 1);
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(chosen) = selected
            .into_iter()
            .next()
            .filter(|idx| *idx < subtype_options.len())
        else {
            return Ok(EffectOutcome::count(0));
        };

        game.set_chosen_land_type(ctx.source, subtype_options[chosen]);
        Ok(EffectOutcome::count(1))
    }
}
