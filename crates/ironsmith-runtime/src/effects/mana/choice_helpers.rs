//! Shared helpers for mana color choice and mana pool crediting.

use crate::color::Color;
use crate::decisions::{ManaColorsSpec, ask_choose_one, make_decision};
use crate::effect::{EffectOutcome, OutcomeValue};
use crate::effects::ExecutionContext;
use crate::events::ManaAddedEvent;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::ids::PlayerId;
use crate::mana::ManaSymbol;
use crate::static_abilities::StaticAbilityId;
use crate::types::Subtype;

pub(crate) fn apply_land_two_or_more_mana_replacement(
    game: &GameState,
    ctx: &ExecutionContext,
    mana: Vec<ManaSymbol>,
) -> Vec<ManaSymbol> {
    if mana.len() < 2 {
        return mana;
    }
    let view = crate::derived_view::DerivedGameView::new(game);
    let source = ctx.source;
    if !view.object_has_card_type(source, crate::types::CardType::Land) {
        return mana;
    }
    let tapped_for_mana = ctx.ability_index.is_some_and(|idx| {
        let Some(object) = game.object(source) else {
            return false;
        };
        let Some(ability) = object.abilities.get(idx) else {
            return false;
        };
        let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
            return false;
        };
        activated.has_tap_cost()
    });
    if !tapped_for_mana {
        return mana;
    }
    let replacement_active = game.battlefield.iter().any(|&perm_id| {
        view.object_has_static_ability_id(
            perm_id,
            StaticAbilityId::LandsProduceOneColorlessInsteadIfProduceTwoOrMore,
        )
    });
    if replacement_active {
        vec![ManaSymbol::Colorless]
    } else {
        mana
    }
}

pub(crate) fn mana_added_value_outcome(
    ctx: &ExecutionContext,
    player_id: PlayerId,
    mana: Vec<ManaSymbol>,
) -> EffectOutcome {
    mana_added_outcome_with_value(ctx, player_id, mana.clone(), OutcomeValue::ManaAdded(mana))
}

pub(crate) fn mana_added_count_outcome(
    ctx: &ExecutionContext,
    player_id: PlayerId,
    mana: Vec<ManaSymbol>,
    count: i32,
) -> EffectOutcome {
    mana_added_outcome_with_value(ctx, player_id, mana, OutcomeValue::Count(count))
}

pub(crate) fn mana_added_outcome_with_value(
    ctx: &ExecutionContext,
    player_id: PlayerId,
    mana: Vec<ManaSymbol>,
    value: OutcomeValue,
) -> EffectOutcome {
    let outcome = EffectOutcome::from_value(value);
    if mana.is_empty() {
        outcome
    } else {
        outcome.with_event(ManaAddedEvent::trigger_event(
            ctx.source,
            ctx.controller,
            player_id,
            mana,
        ))
    }
}

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

    let effective_available = match (available_colors, ctx.mana.mana_color_restriction.as_deref()) {
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
    if ctx.decision_maker.awaiting_choice() {
        return Vec::new();
    }

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

pub(crate) fn credit_mana_symbols_from_context<I>(
    game: &mut GameState,
    player_id: PlayerId,
    symbols: I,
    ctx: &ExecutionContext,
) -> Vec<ManaSymbol>
where
    I: IntoIterator<Item = ManaSymbol>,
{
    let symbols = apply_land_two_or_more_mana_replacement(game, ctx, symbols.into_iter().collect());
    credit_mana_symbols_with_context(
        game,
        player_id,
        symbols.iter().copied(),
        Some(ctx.source),
        &ctx.mana.mana_usage_restrictions,
        ctx.mana.mana_source_chosen_creature_type,
    );
    symbols
}

fn credit_mana_symbols_with_context<I>(
    game: &mut GameState,
    player_id: PlayerId,
    symbols: I,
    source: Option<ObjectId>,
    restrictions: &[crate::ability::ManaUsageRestriction],
    source_chosen_creature_type: Option<Subtype>,
) where
    I: IntoIterator<Item = ManaSymbol>,
{
    if let Some(player) = game.player_mut(player_id) {
        for symbol in symbols {
            if restrictions.is_empty() {
                player.mana_pool.add(symbol, 1);
            } else {
                player.add_restricted_mana(crate::ability::RestrictedManaUnit {
                    symbol,
                    source: source.unwrap_or(ObjectId::from_raw(0)),
                    source_chosen_creature_type,
                    restrictions: restrictions.to_vec(),
                });
            }
        }
    }
}

pub(crate) fn credit_repeated_mana_symbol_from_context(
    game: &mut GameState,
    player_id: PlayerId,
    symbol: ManaSymbol,
    count: u32,
    ctx: &ExecutionContext,
) -> Vec<ManaSymbol> {
    let symbols = apply_land_two_or_more_mana_replacement(
        game,
        ctx,
        std::iter::repeat_n(symbol, count as usize).collect(),
    );
    credit_mana_symbols_with_context(
        game,
        player_id,
        symbols.iter().copied(),
        Some(ctx.source),
        &ctx.mana.mana_usage_restrictions,
        ctx.mana.mana_source_chosen_creature_type,
    );
    symbols
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
        )
        .unwrap_or(default_symbol);
        if ctx.decision_maker.awaiting_choice() {
            return Vec::new();
        }
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
            )
            .unwrap_or(default_symbol);
            if ctx.decision_maker.awaiting_choice() {
                return Vec::new();
            }
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
