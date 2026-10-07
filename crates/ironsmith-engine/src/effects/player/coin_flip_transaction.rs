//! One authored simultaneous coin batch. Replacement-only physical coins are
//! discarded before completed history, receipt ordinals, or triggers exist.
use crate::decisions::ask_choose_one;
use crate::effect::CoinFlipResult;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilterExt as _;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use ironsmith_core::{CoinFace, CoinFlipKind, StaticAbilityPayload};

use super::FlipCoinEffect;

pub(super) fn flip_batch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effect: &FlipCoinEffect,
    player: PlayerId,
    count: u32,
    instruction_start: u32,
) -> Result<Option<Vec<CoinFlipResult>>, ExecutionError> {
    if count == 0 {
        return Ok(Some(Vec::new()));
    }
    game.turn_store.turn_history.check_completed_coin_flip_capacity(player, count as usize)?;
    let checked = game.continuous_query_snapshot().map_err(ExecutionError::ContinuousDiscovery)?;
    let mut multiplicity = 1u32;
    let mut first_batch_wins = false;
    for &source in &checked.battlefield {
        if checked.is_phased_out(source) {
            continue;
        }
        let Some(controller) = checked.current_controller(source) else { continue };
        let filter_context = checked.filter_context_for(controller, Some(source));
        // Every active ability occurrence is a replacement, even when its
        // model is shared with another occurrence. These bounded replacements
        // commute: every extra-coin occurrence doubles every original coin.
        for ability in checked.current_abilities(source).unwrap_or_default() {
            let crate::ability::AbilityKind::Static(ability) = ability.kind else { continue };
            let Some(model) = ability.compiled_model() else { continue };
            match &model.payload {
                StaticAbilityPayload::ExtraCoinIgnoreOne { player: filter }
                    if filter.matches_player(player, &filter_context) => {
                    multiplicity = multiplicity.checked_mul(2).ok_or(
                        ExecutionError::ResourceLimitExceeded {
                            resource: "physical replacement coins per original flip",
                            requested: u128::from(multiplicity) * 2,
                            maximum: u32::MAX as u128,
                        },
                    )?;
                }
                StaticAbilityPayload::FirstCoinBatchHeadsWin { player: filter }
                    if filter.matches_player(player, &filter_context) => {
                    first_batch_wins = game.turn_store.turn_history.completed_coin_flip_count(player) == 0;
                }
                _ => {}
            }
        }
    }
    let physical = count.checked_mul(multiplicity).ok_or(
        ExecutionError::ResourceLimitExceeded {
            resource: "physical replacement coin batch",
            requested: u128::from(count) * u128::from(multiplicity),
            maximum: u32::MAX as u128,
        },
    )?;
    let mut flips = Vec::new();
    flips.try_reserve_exact(physical as usize).map_err(|_| ExecutionError::ResourceAllocationFailed {
        resource: "physical replacement coin batch", requested: physical as usize,
    })?;
    // All calls precede the simultaneous physical results. On replay, neither
    // a partial call list nor earlier physical results can escape this owner.
    for _ in 0..physical {
        let call = if effect.kind == CoinFlipKind::Called {
            let options = [("Heads".to_string(), CoinFace::Heads), ("Tails".to_string(), CoinFace::Tails)];
            let call = ask_choose_one(game, &mut ctx.decision_maker, player, ctx.source, &options);
            if ctx.decision_maker.awaiting_choice() { return Ok(None); }
            Some(call.ok_or_else(|| ExecutionError::UnresolvableValue("a called coin flip requires a call".into()))?)
        } else { None };
        flips.push(CoinFlipResult {
            player, face: CoinFace::Heads, call, winner: None, loser: None,
            turn_ordinal: 0, instruction_ordinal: 0, associated_player: None,
        });
    }
    let forced_winner = effect.forced_winner.as_ref()
        .map(|filter| crate::effects::helpers::resolve_player_filter(game, filter, ctx)).transpose()?;
    let forced_loser = effect.forced_loser.as_ref()
        .map(|filter| crate::effects::helpers::resolve_player_filter(game, filter, ctx)).transpose()?;
    for flip in &mut flips {
        let mut faces = [CoinFace::Heads, CoinFace::Tails];
        game.shuffle_slice(&mut faces);
        let natural = game.take_forced_coin_flip().unwrap_or(faces[0]);
        flip.face = if first_batch_wins { CoinFace::Heads } else { effect.forced_face.unwrap_or(natural) };
        if first_batch_wins {
            // This explicit result modification can give a face-only flip a
            // winner, unlike an ordinary heads result (FIN release notes).
            flip.winner = Some(player);
        } else if effect.forced_winner.is_some() || effect.forced_loser.is_some() {
            flip.winner = forced_winner;
            flip.loser = forced_loser;
        } else if let Some(call) = flip.call {
            if call == flip.face { flip.winner = Some(player); } else { flip.loser = Some(player); }
        }
    }
    // Nested Thumb replacements finish inside out. Each pair contributes one
    // retained result; ignored coins never become events or receipt entries.
    let mut width = multiplicity;
    while width > 1 {
        let mut write = 0;
        for read in (0..flips.len()).step_by(2) {
            let options = [
                (format!("Keep {:?} ({})", flips[read].face, result_label(flips[read])), 0usize),
                (format!("Keep {:?} ({})", flips[read + 1].face, result_label(flips[read + 1])), 1usize),
            ];
            let choice = ask_choose_one(game, &mut ctx.decision_maker, player, ctx.source, &options);
            if ctx.decision_maker.awaiting_choice() { return Ok(None); }
            let choice = choice.ok_or_else(|| ExecutionError::UnresolvableValue("a replacement coin result must be kept".into()))?;
            flips[write] = flips[read + choice];
            write += 1;
        }
        flips.truncate(write);
        width /= 2;
    }
    let first = game.turn_store.turn_history.record_completed_coin_flips(player, flips.len())?;
    for (index, flip) in flips.iter_mut().enumerate() {
        flip.turn_ordinal = first + index as u32;
        flip.instruction_ordinal = instruction_start + index as u32;
    }
    game.mark_continuous_state_dirty();
    Ok(Some(flips))
}

fn result_label(flip: CoinFlipResult) -> &'static str {
    if flip.winner == Some(flip.player) { "win" }
    else if flip.loser == Some(flip.player) { "loss" }
    else { "no winner or loser" }
}
