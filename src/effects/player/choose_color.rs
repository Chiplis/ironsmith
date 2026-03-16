//! Choose a color and store it on the source object.

use crate::color::Color;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseColorEffect {
    pub chooser: PlayerFilter,
}

impl ChooseColorEffect {
    pub fn new(chooser: PlayerFilter) -> Self {
        Self { chooser }
    }

    fn color_options() -> [(Color, &'static str); 5] {
        [
            (Color::White, "White"),
            (Color::Blue, "Blue"),
            (Color::Black, "Black"),
            (Color::Red, "Red"),
            (Color::Green, "Green"),
        ]
    }
}

impl EffectExecutor for ChooseColorEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let options: Vec<SelectableOption> = Self::color_options()
            .iter()
            .enumerate()
            .map(|(idx, (_, label))| SelectableOption::new(idx, *label))
            .collect();
        let choice_ctx =
            SelectOptionsContext::new(chooser, Some(ctx.source), "Choose a color", options, 1, 1);
        let chosen = ctx
            .decision_maker
            .decide_options(game, &choice_ctx)
            .into_iter()
            .next()
            .unwrap_or(0);
        let (color, _) = Self::color_options()[chosen.min(4)];
        game.set_chosen_color(ctx.source, color);
        Ok(EffectOutcome::count(1))
    }
}
