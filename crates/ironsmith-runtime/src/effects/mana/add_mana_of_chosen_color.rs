//! Add mana of the chosen color effect implementation.

use super::choice_helpers::{
    choose_mana_colors, credit_repeated_mana_symbol_from_context, mana_added_count_outcome,
};
use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub type AddManaOfChosenColorEffect = ironsmith_core::AddManaOfChosenColorEffect;

impl EffectExecutor for AddManaOfChosenColorEffect {
    fn directly_produces_mana(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        if amount == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let chosen = game.chosen_color(ctx.source).unwrap_or(Color::Green);

        let selected = if let Some(fixed) = self.fixed_option {
            if fixed == chosen {
                fixed
            } else {
                let options = [fixed, chosen];
                choose_mana_colors(game, ctx, player_id, 1, true, false, Some(&options), fixed)
                    .into_iter()
                    .next()
                    .unwrap_or(fixed)
            }
        } else {
            chosen
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        let symbol = ManaSymbol::from_color(selected);
        credit_repeated_mana_symbol_from_context(game, player_id, symbol, amount, ctx);
        let mana = std::iter::repeat_n(symbol, amount as usize).collect::<Vec<_>>();

        Ok(mana_added_count_outcome(
            ctx,
            player_id,
            mana,
            amount as i32,
        ))
    }

    fn producible_mana_symbols(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let chosen = game.chosen_color(source).unwrap_or(Color::Green);
        let mut symbols = vec![ManaSymbol::from_color(chosen)];
        if let Some(fixed) = self.fixed_option {
            let fixed_symbol = ManaSymbol::from_color(fixed);
            if !symbols.contains(&fixed_symbol) {
                symbols.push(fixed_symbol);
            }
        }
        Some(symbols)
    }
}
