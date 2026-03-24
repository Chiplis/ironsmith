//! Choose one named option and store it on the source object.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNamedOptionEffect {
    pub chooser: PlayerFilter,
    pub options: Vec<String>,
}

impl ChooseNamedOptionEffect {
    pub fn new(chooser: PlayerFilter, options: Vec<String>) -> Self {
        Self { chooser, options }
    }
}

impl EffectExecutor for ChooseNamedOptionEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let options = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, option)| SelectableOption::new(idx, option.clone()))
            .collect::<Vec<_>>();
        let choice_ctx = SelectOptionsContext::new(
            chooser,
            Some(ctx.source),
            "Choose one",
            options,
            1,
            1,
        );
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(chosen) = selected
            .into_iter()
            .next()
            .filter(|idx| *idx < self.options.len())
        else {
            return Ok(EffectOutcome::count(0));
        };
        game.set_chosen_named_option(ctx.source, self.options[chosen].clone());
        Ok(EffectOutcome::count(1))
    }
}
