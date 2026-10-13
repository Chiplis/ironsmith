//! Choose a land type and store it on the source object.

use crate::effects::composition::scoped_choice::{select_typed_option, SingleChoicePolicy};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::types::Subtype;
pub use ironsmith_core::ChooseLandTypeEffect;

fn land_type_options(exclude_basic: bool, basic_only: bool) -> Vec<Subtype> {
    Subtype::all_land_types()
        .iter()
        .copied()
        .filter(|subtype| {
            let basic = matches!(
                subtype,
                Subtype::Plains
                    | Subtype::Island
                    | Subtype::Swamp
                    | Subtype::Mountain
                    | Subtype::Forest
            );
            !(exclude_basic && basic) && (!basic_only || basic)
        })
        .collect()
}

impl EffectExecutor for ChooseLandTypeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let subtype_options = land_type_options(self.exclude_basic, self.basic_only);
        if subtype_options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let prompt = if self.exclude_basic {
            "Choose a nonbasic land type"
        } else if self.basic_only {
            "Choose a basic land type"
        } else {
            "Choose a land type"
        };
        let menu = subtype_options.iter().map(|value| (*value, value.to_string())).collect::<Vec<_>>();
        let Some(chosen) = select_typed_option(game, ctx, chooser, prompt, &menu, SingleChoicePolicy::First)? else {
            return Ok(EffectOutcome::count(0));
        };
        game.set_chosen_land_type(ctx.source, chosen);
        Ok(EffectOutcome::count(1))
    }
}
