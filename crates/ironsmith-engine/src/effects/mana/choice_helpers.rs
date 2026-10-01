//! Shared helpers for mana color choice and mana pool crediting.

use crate::color::Color;
use crate::decisions::{ManaColorsSpec, ask_choose_one, make_decision};
use crate::effect::{EffectOutcome, OutcomeValue};
use crate::effects::ExecutionContext;
use crate::events::ManaAddedEvent;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::mana::ManaSymbol;
use crate::snapshot::ObjectSnapshot;

/// Complete receipt for a resolved mana operation. Additional instruction
/// outcomes never substitute for the original instruction's mana quantity.
pub(crate) struct ManaCreditReceipt {
    mana: Vec<ManaSymbol>,
    original_committed: bool,
    outcome: EffectOutcome,
}

impl ManaCreditReceipt {
    pub(crate) fn mana_count(&self) -> i32 { self.mana.len() as i32 }
}

pub(crate) fn mana_added_value_outcome(
    _ctx: &ExecutionContext,
    _player_id: PlayerId,
    mut receipt: ManaCreditReceipt,
) -> EffectOutcome {
    receipt.outcome.set_value(OutcomeValue::ManaAdded(receipt.mana));
    receipt.outcome
}

pub(crate) fn mana_added_count_outcome(
    _ctx: &ExecutionContext,
    _player_id: PlayerId,
    mut receipt: ManaCreditReceipt,
    count: i32,
) -> EffectOutcome {
    receipt.outcome.set_value(OutcomeValue::Count(if receipt.original_committed { count } else { 0 }));
    receipt.outcome
}

/// Choose one or more mana colors through the decision system with stable
/// fallback behavior and length normalization.
pub(crate) fn choose_mana_colors(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    count: u32,
    same_color: bool,
    distinct_colors: bool,
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

    // An exact output selected in the payment plan is already a choice. Manual
    // activations keep their ordinary prompt, and distinct-color instructions
    // still ask when one color cannot satisfy them.
    if ctx.mana.mana_color_restriction.is_some()
        && let Some([color]) = effective_available.as_deref()
        && (same_color || !distinct_colors || count == 1)
    {
        return vec![*color; count as usize];
    }

    let spec = if let Some(colors) = effective_available.as_deref() {
        if colors.is_empty() {
            return vec![fallback; count as usize];
        }
        if distinct_colors && !same_color {
            ManaColorsSpec::restricted_different_colors(ctx.source, count, colors.to_vec())
        } else {
            ManaColorsSpec::restricted(ctx.source, count, same_color, colors.to_vec())
        }
    } else if distinct_colors && !same_color {
        ManaColorsSpec::different_colors(ctx.source, count)
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

    if distinct_colors && !same_color {
        let available = effective_available.as_deref().unwrap_or(&Color::ALL);
        let mut distinct = Vec::with_capacity(count as usize);
        for color in chosen {
            if !distinct.contains(&color) {
                distinct.push(color);
            }
        }
        while distinct.len() < count as usize {
            if let Some(color) = available
                .iter()
                .copied()
                .find(|color| !distinct.contains(color))
            {
                distinct.push(color);
            } else {
                distinct.push(fallback);
            }
        }
        chosen = distinct;
    } else {
        while chosen.len() < count as usize {
            chosen.push(fallback);
        }
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
    ctx: &mut ExecutionContext,
) -> Result<ManaCreditReceipt, crate::effects::ExecutionError>
where
    I: IntoIterator<Item = ManaSymbol>,
{
    use crate::effects::ExecutionError;
    use crate::events::processing::process_trait_event_with_execution_context;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(ManaCreditReceipt { mana: Vec::new(), original_committed: false, outcome: EffectOutcome::count(0) });
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(ManaCreditReceipt { mana: Vec::new(), original_committed: false, outcome: EffectOutcome::count(0) });
        }
        let mana = symbols.into_iter().collect::<Vec<_>>();
        if mana.is_empty() {
            return Ok(ManaCreditReceipt { mana, original_committed: true, outcome: EffectOutcome::count(0) });
        }
        let snapshot = ctx.source_snapshot.clone().or_else(|| game.object(ctx.source)
            .map(|object| ObjectSnapshot::from_object(object, game)));
        let event = crate::events::Event::new_with_provenance(
            ManaAddedEvent::new(ctx.source, ctx.controller, player_id, mana)
                .with_production_provenance(ctx.mana.production_provenance)
                .with_snapshot(snapshot), ctx.provenance);
        let result = process_trait_event_with_execution_context(game, event, ctx)?;
        let mut primary_mana = Vec::new();
        let mut primary_committed = false;
        let outcome = crate::effects::replacement::execute_event_expansion(game, ctx, result, |game, ctx, original| {
            let receipt = commit_mana_result(game, ctx, original)?;
            primary_mana = receipt.mana;
            primary_committed = receipt.original_committed;
            Ok(receipt.outcome)
        })?;
        Ok(ManaCreditReceipt { mana: primary_mana, original_committed: primary_committed, outcome })
    })();
    let pending = ctx.decision_maker.awaiting_choice();
    if pending || result.is_err() {
        game.restore_execution_checkpoint(checkpoint, result.is_ok() && pending);
        context_checkpoint.restore(ctx);
    }
    if pending && result.is_ok() { return Ok(ManaCreditReceipt { mana: Vec::new(), original_committed: false, outcome: EffectOutcome::count(0) }); }
    result
}

fn commit_mana_result(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    result: crate::events::processing::TraitEventResult,
) -> Result<ManaCreditReceipt, crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    use crate::events::processing::TraitEventResult;
    match result {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                let resolved = crate::events::downcast_event::<ManaAddedEvent>(event.inner())
                    .ok_or_else(|| ExecutionError::InternalError("mana replacement returned an incompatible event".into()))?;
                let player = game.player_mut(resolved.player).ok_or(ExecutionError::PlayerNotFound(resolved.player))?;
                for symbol in resolved.mana.iter().copied() {
                    if ctx.mana.mana_usage_restrictions.is_empty() {
                        player.add_unrestricted_mana_with_retention(symbol, resolved.source, resolved.snapshot.clone(), ctx.mana.retention);
                    } else {
                        player.add_restricted_mana_with_snapshot_and_retention(crate::ability::RestrictedManaUnit {
                            symbol, source: resolved.source,
                            source_chosen_creature_type: ctx.mana.mana_source_chosen_creature_type,
                            restrictions: ctx.mana.mana_usage_restrictions.clone(),
                        }, resolved.snapshot.clone(), ctx.mana.retention);
                    }
                }
                let outcome = if resolved.mana.is_empty() { EffectOutcome::count(0) } else {
                    EffectOutcome::count(resolved.mana.len() as i32).with_event(
                        crate::triggers::TriggerEvent::new_with_provenance(resolved.clone(), event.provenance()))
                };
                Ok(ManaCreditReceipt { mana: resolved.mana.clone(), original_committed: true, outcome })
            }
            TraitEventResult::Replaced { effects, source, controller, context, .. } => {
                let mut outcome = crate::effects::replacement::execute_replacement_payload(game, ctx, &effects, source, controller, &context, None)?;
                let mut original = EffectOutcome::replaced();
                original.set_value(OutcomeValue::Count(0));
                let outcome = EffectOutcome::aggregate_replacement_outcomes(original, [outcome]);
                Ok(ManaCreditReceipt { mana: Vec::new(), original_committed: false, outcome })
            }
            TraitEventResult::Prevented => Ok(ManaCreditReceipt { mana: Vec::new(), original_committed: false, outcome: EffectOutcome::prevented() }),
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                if ctx.decision_maker.awaiting_choice() {
                    Ok(ManaCreditReceipt { mana: Vec::new(), original_committed: false, outcome: EffectOutcome::count(0) })
                } else { Err(ExecutionError::InternalError("mana replacement suspended without a captured decision".into())) }
            }
        TraitEventResult::Expanded { .. } => Err(ExecutionError::InternalError("mana commit requires a flattened replacement result".into())),
    }
}

pub(crate) fn credit_repeated_mana_symbol_from_context(
    game: &mut GameState,
    player_id: PlayerId,
    symbol: ManaSymbol,
    count: u32,
    ctx: &mut ExecutionContext,
) -> Result<ManaCreditReceipt, crate::effects::ExecutionError> {
    credit_mana_symbols_from_context(game, player_id, std::iter::repeat_n(symbol, count as usize), ctx)
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
