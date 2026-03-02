//! Shared helpers for mana color choice and mana pool crediting.

use crate::color::Color;
use crate::decisions::{ManaColorsSpec, ask_choose_one, make_decision};
use crate::executor::ExecutionContext;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::mana::ManaSymbol;

/// Choose one or more mana colors through the decision system with stable
/// fallback behavior and length normalization.
pub(crate) fn choose_mana_colors(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    count: u32,
    same_color: bool,
    available_colors: Option<&[Color]>,
    default_color: Color,
) -> Vec<Color> {
    if count == 0 {
        return Vec::new();
    }

    let effective_available = match (available_colors, ctx.mana_color_restriction.as_deref()) {
        (Some(effect_colors), Some(ctx_colors)) => Some(
            effect_colors
                .iter()
                .copied()
                .filter(|color| ctx_colors.contains(color))
                .collect::<Vec<_>>(),
        ),
        (Some(effect_colors), None) => Some(effect_colors.to_vec()),
        (None, Some(ctx_colors)) => Some(ctx_colors.to_vec()),
        (None, None) => None,
    };

    let fallback = effective_available
        .as_deref()
        .and_then(|colors| colors.first().copied())
        .unwrap_or(default_color);

    let spec = if let Some(colors) = effective_available.as_deref() {
        if colors.is_empty() {
            return vec![fallback; count as usize];
        }
        ManaColorsSpec::restricted(ctx.source, count, same_color, colors.to_vec())
    } else {
        ManaColorsSpec::any_color(ctx.source, count, same_color)
    };

    let mut chosen = make_decision(
        game,
        &mut ctx.decision_maker,
        player_id,
        Some(ctx.source),
        spec,
    );

    if let Some(available) = effective_available.as_deref() {
        chosen.retain(|color| available.contains(color));
    }

    while chosen.len() < count as usize {
        chosen.push(fallback);
    }
    chosen.truncate(count as usize);

    if same_color && let Some(first) = chosen.first().copied() {
        chosen.fill(first);
    }

    chosen
}

/// Credit mana symbols to a player's mana pool.
pub(crate) fn credit_mana_symbols<I>(game: &mut GameState, player_id: PlayerId, symbols: I)
where
    I: IntoIterator<Item = ManaSymbol>,
{
    if let Some(player) = game.player_mut(player_id) {
        for symbol in symbols {
            player.mana_pool.add(symbol, 1);
        }
    }
}

/// Credit one repeated mana symbol to a player's mana pool.
pub(crate) fn credit_repeated_mana_symbol(
    game: &mut GameState,
    player_id: PlayerId,
    symbol: ManaSymbol,
    count: u32,
) {
    if let Some(player) = game.player_mut(player_id) {
        player.mana_pool.add(symbol, count);
    }
}

/// Choose one or more mana symbols through the decision system with stable
/// fallback behavior and length normalization.
pub(crate) fn choose_mana_symbols(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    count: u32,
    same_symbol: bool,
    available_symbols: &[ManaSymbol],
    default_symbol: ManaSymbol,
) -> Vec<ManaSymbol> {
    if count == 0 {
        return Vec::new();
    }
    if available_symbols.is_empty() {
        return vec![default_symbol; count as usize];
    }

    let choices = available_symbols
        .iter()
        .map(|symbol| (mana_symbol_oracle(*symbol), *symbol))
        .collect::<Vec<_>>();

    let mut chosen = Vec::new();
    if same_symbol {
        let selected = ask_choose_one(
            game,
            &mut ctx.decision_maker,
            player_id,
            ctx.source,
            &choices,
        );
        let fallback = if available_symbols.contains(&selected) {
            selected
        } else {
            default_symbol
        };
        chosen.resize(count as usize, fallback);
    } else {
        for _ in 0..count {
            let selected = ask_choose_one(
                game,
                &mut ctx.decision_maker,
                player_id,
                ctx.source,
                &choices,
            );
            chosen.push(if available_symbols.contains(&selected) {
                selected
            } else {
                default_symbol
            });
        }
    }

    while chosen.len() < count as usize {
        chosen.push(default_symbol);
    }
    chosen.truncate(count as usize);
    chosen
}

fn mana_symbol_oracle(symbol: ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "{W}".to_string(),
        ManaSymbol::Blue => "{U}".to_string(),
        ManaSymbol::Black => "{B}".to_string(),
        ManaSymbol::Red => "{R}".to_string(),
        ManaSymbol::Green => "{G}".to_string(),
        ManaSymbol::Colorless => "{C}".to_string(),
        _ => "{?}".to_string(),
    }
}
