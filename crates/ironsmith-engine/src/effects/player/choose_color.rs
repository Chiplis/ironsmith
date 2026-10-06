//! Choose a color and store it on the source object.

use crate::color::Color;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
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
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        // A prepared mana activation may carry this exact stored-color
        // decision. Manual execution retains its ordinary options prompt.
        let choice = crate::mana_payment::ManaProductionChoice {
            purpose: crate::mana_payment::ManaChoicePurpose::StoredColor,
            source: ctx.source, player: chooser,
            available: Color::ALL.into_iter().map(crate::mana::ManaSymbol::from_color).collect(),
            count: 1, same_type: true, distinct: false,
        };
        if let Some(output) = ctx.decision_maker.planned_mana_output(game, &choice)
            .map_err(ExecutionError::InternalError)? {
            if !choice.accepts(&output) {
                return Err(ExecutionError::InternalError("invalid prepared stored-color choice".into()));
            }
            let color = match output[0] {
                crate::mana::ManaSymbol::White => Color::White,
                crate::mana::ManaSymbol::Blue => Color::Blue,
                crate::mana::ManaSymbol::Black => Color::Black,
                crate::mana::ManaSymbol::Red => Color::Red,
                crate::mana::ManaSymbol::Green => Color::Green,
                _ => return Err(ExecutionError::InternalError("non-color in stored-color witness".into())),
            };
            game.set_chosen_color(ctx.source, color);
            return Ok(EffectOutcome::count(1).with_execution_fact(crate::effect::ExecutionFact::ChosenColor(color)));
        }
        let options: Vec<SelectableOption> = Self::color_options()
            .iter()
            .enumerate()
            .map(|(idx, (_, label))| SelectableOption::new(idx, *label))
            .collect();
        let choice_ctx =
            SelectOptionsContext::new(chooser, Some(ctx.source), "Choose a color", options, 1, 1);
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(chosen) = selected
            .into_iter()
            .next()
            .filter(|idx| *idx < Self::color_options().len())
        else {
            return Ok(EffectOutcome::count(0));
        };
        let (color, _) = Self::color_options()[chosen];
        game.set_chosen_color(ctx.source, color);
        Ok(EffectOutcome::count(1).with_execution_fact(crate::effect::ExecutionFact::ChosenColor(color)))
    }
}
