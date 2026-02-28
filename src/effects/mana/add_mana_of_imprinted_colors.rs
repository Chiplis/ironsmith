//! Add mana of imprinted card's colors effect implementation.
//!
//! Used by Chrome Mox to produce mana based on the colors of the exiled card.

use super::choice_helpers::{choose_mana_colors, credit_mana_symbols};
use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;

/// Effect that adds one mana of any of the imprinted card's colors.
///
/// If no card is imprinted, or the imprinted card is colorless, this does nothing.
/// If the imprinted card has multiple colors, the player chooses which color.
///
/// # Example
///
/// ```ignore
/// // Chrome Mox's mana ability
/// let effect = AddManaOfImprintedColorsEffect::new();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfImprintedColorsEffect;

impl AddManaOfImprintedColorsEffect {
    /// Create a new add mana of imprinted colors effect.
    pub fn new() -> Self {
        Self
    }
}

impl Default for AddManaOfImprintedColorsEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectExecutor for AddManaOfImprintedColorsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = ctx.source;
        let controller = ctx.controller;

        // Get the imprinted cards
        let imprinted = game.get_imprinted_cards(source_id).to_vec();

        if imprinted.is_empty() {
            // No imprinted card - can't produce mana
            return Ok(EffectOutcome::count(0));
        }

        // Get colors from the first imprinted card (Chrome Mox only imprints one)
        let imprinted_id = imprinted[0];
        let colors: Vec<Color> = game
            .object(imprinted_id)
            .map(|obj| {
                let color_set = obj.colors();
                let mut colors = Vec::new();
                if color_set.contains(Color::White) {
                    colors.push(Color::White);
                }
                if color_set.contains(Color::Blue) {
                    colors.push(Color::Blue);
                }
                if color_set.contains(Color::Black) {
                    colors.push(Color::Black);
                }
                if color_set.contains(Color::Red) {
                    colors.push(Color::Red);
                }
                if color_set.contains(Color::Green) {
                    colors.push(Color::Green);
                }
                colors
            })
            .unwrap_or_default();

        if colors.is_empty() {
            // Imprinted card is colorless - can't produce mana
            return Ok(EffectOutcome::count(0));
        }

        let chosen_color =
            choose_mana_colors(game, ctx, controller, 1, true, Some(&colors), colors[0])
                .into_iter()
                .next()
                .unwrap_or(colors[0]);
        credit_mana_symbols(game, controller, [ManaSymbol::from_color(chosen_color)]);

        Ok(EffectOutcome::count(1))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn producible_mana_symbols(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let imprinted_id = *game.get_imprinted_cards(source).first()?;
        let color_set = game.object(imprinted_id)?.colors();

        let mut symbols = Vec::new();
        if color_set.contains(Color::White) {
            symbols.push(ManaSymbol::White);
        }
        if color_set.contains(Color::Blue) {
            symbols.push(ManaSymbol::Blue);
        }
        if color_set.contains(Color::Black) {
            symbols.push(ManaSymbol::Black);
        }
        if color_set.contains(Color::Red) {
            symbols.push(ManaSymbol::Red);
        }
        if color_set.contains(Color::Green) {
            symbols.push(ManaSymbol::Green);
        }
        if symbols.is_empty() {
            return None;
        }
        Some(symbols)
    }
}
