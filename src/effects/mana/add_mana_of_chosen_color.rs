//! Add mana of the chosen color effect implementation.

use crate::color::Color;
use crate::decisions::{ManaColorsSpec, make_decision};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
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

        let chosen = game
            .chosen_color(ctx.source)
            .unwrap_or(Color::Green);

        let selected = if let Some(fixed) = self.fixed_option {
            if fixed == chosen {
                fixed
            } else {
                let colors = vec![fixed, chosen];
                let spec = ManaColorsSpec::restricted(ctx.source, 1, true, colors.clone());
                let mut decision = make_decision(
                    game,
                    &mut ctx.decision_maker,
                    player_id,
                    Some(ctx.source),
                    spec,
                );
                decision.pop().unwrap_or(fixed)
            }
        } else {
            chosen
        };

        if let Some(p) = game.player_mut(player_id) {
            for _ in 0..amount {
                match selected {
                    Color::White => p.mana_pool.white += 1,
                    Color::Blue => p.mana_pool.blue += 1,
                    Color::Black => p.mana_pool.black += 1,
                    Color::Red => p.mana_pool.red += 1,
                    Color::Green => p.mana_pool.green += 1,
                }
            }
        }

        Ok(EffectOutcome::count(amount as i32))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

