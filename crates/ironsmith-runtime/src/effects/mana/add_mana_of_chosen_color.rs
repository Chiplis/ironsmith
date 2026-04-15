//! Add mana of the chosen color effect implementation.

use super::choice_helpers::{choose_mana_colors, credit_repeated_mana_symbol_from_context};
use crate::color::Color;
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::target::PlayerFilter;

/// Effect that adds mana of a previously chosen color.
///
/// If `fixed_option` is provided, the player chooses between that fixed color
/// and the chosen color (used for "Add {B} or one mana of the chosen color.").
#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfChosenColorEffect {
    /// Number of mana to add.
    pub amount: Value,
    /// Which player receives the mana.
    pub player: PlayerFilter,
    /// Optional fixed color alternative.
    pub fixed_option: Option<Color>,
}

impl AddManaOfChosenColorEffect {
    /// Create a new add mana of chosen color effect.
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
            fixed_option: None,
        }
    }

    /// Create a new add mana effect with a fixed-color alternative.
    pub fn with_fixed_option(amount: impl Into<Value>, player: PlayerFilter, fixed: Color) -> Self {
        Self {
            amount: amount.into(),
            player,
            fixed_option: Some(fixed),
        }
    }
}

impl EffectExecutor for AddManaOfChosenColorEffect {
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
                choose_mana_colors(game, ctx, player_id, 1, true, Some(&options), fixed)
                    .into_iter()
                    .next()
                    .unwrap_or(fixed)
            }
        } else {
            chosen
        };

        credit_repeated_mana_symbol_from_context(
            game,
            player_id,
            ManaSymbol::from_color(selected),
            amount,
            ctx,
        );

        Ok(EffectOutcome::count(amount as i32))
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
