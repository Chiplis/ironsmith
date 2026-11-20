//! Choose one named option and store it on the source object.

use crate::effects::composition::scoped_choice::{select_typed_option, SingleChoicePolicy};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
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
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let menu = self.options.iter().map(|value| (value.clone(), value.clone())).collect::<Vec<_>>();
        let Some(chosen) = select_typed_option(game, ctx, chooser, "Choose one", &menu, SingleChoicePolicy::First)? else {
            return Ok(EffectOutcome::count(0));
        };
        let outcome = chosen.parse::<i32>().unwrap_or(1);
        // "Choose artifact or enchantment. Destroy all permanents of the
        // chosen type." A named option that is a card type is a card-type
        // choice, so later "the chosen type" filters can read it.
        if let Some(card_type) = crate::types::CardType::ALL
            .iter()
            .copied()
            .find(|card_type| chosen.eq_ignore_ascii_case(card_type.name()))
        {
            game.set_chosen_card_type(ctx.source, card_type);
        }
        game.set_chosen_named_option(ctx.source, chosen);
        Ok(EffectOutcome::count(outcome))
    }
}
