//! Add mana of imprinted card's colors effect implementation.
//!
//! Used by Chrome Mox and Pit of Offerings-style permanents to produce mana
//! based on the colors of the cards they exiled.

use super::choice_helpers::{
    choose_mana_colors, credit_mana_symbols_from_context, mana_added_count_outputs,
};
use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub type AddManaOfImprintedColorsEffect = ironsmith_core::AddManaOfImprintedColorsEffect;

impl EffectExecutor for AddManaOfImprintedColorsEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::ImprintedColors)
    }

    fn directly_produces_mana(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let source_id = ctx.source;
        let controller = ctx.controller;

        // "Any of the exiled card's/cards' colors": the cards this permanent
        // imprinted or exiled with its linked ability (CR 607.2a).
        let colors = linked_exiled_card_colors(game, source_id);
        if colors.is_empty() {
            // No linked card, or only colorless ones - can't produce mana.
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }

        let chosen_color = choose_mana_colors(
            game,
            ctx,
            controller,
            1,
            true,
            false,
            Some(&colors),
            colors[0],
        )?
        .into_iter()
        .next()
        .unwrap_or(colors[0]);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let symbol = ManaSymbol::from_color(chosen_color);
        let mana = credit_mana_symbols_from_context(game, controller, [symbol], ctx)?;

        Ok(mana_added_count_outputs(ctx, controller, mana, 1))
    }

    fn producible_mana_symbols(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let symbols: Vec<ManaSymbol> = linked_exiled_card_colors(game, source)
            .into_iter()
            .map(ManaSymbol::from_color)
            .collect();
        if symbols.is_empty() {
            return None;
        }
        Some(symbols)
    }
}

/// The distinct colors among the cards imprinted on, or exiled with,
/// `source`, in WUBRG order.
pub(super) fn linked_exiled_card_colors(
    game: &GameState,
    source: crate::ids::ObjectId,
) -> Vec<Color> {
    let imprinted = game.get_imprinted_cards(source);
    let exiled_with = game.get_exiled_with_source_links(source);
    [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ]
    .into_iter()
    .filter(|color| {
        imprinted.iter().chain(exiled_with).any(|&id| {
            game.object(id)
                .is_some_and(|object| object.colors().contains(*color))
        })
    })
    .collect()
}
