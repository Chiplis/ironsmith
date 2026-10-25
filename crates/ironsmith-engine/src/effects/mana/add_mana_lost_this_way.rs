//! "you add the mana lost this way" (Drain Power).
//!
//! The amount names the producer that removed the mana (a lose-all-unspent-
//! mana instruction); its unconverted mana-lost events supply exactly which
//! mana is added, type for type.

use crate::effect::{EffectOutcome, Value};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;

pub type AddManaLostThisWayEffect = ironsmith_core::AddManaLostThisWayEffect;

type LostUnit = (
    crate::player::ManaSourceProvenance,
    Option<crate::ability::RestrictedManaUnit>,
);

/// The mana the producer behind `amount` removed, as symbols. Tracked units
/// come first, in the order of the returned unit list.
fn lost_mana(
    amount: &Value,
    ctx: &ExecutionContext,
) -> Result<(Vec<ManaSymbol>, Vec<LostUnit>), ExecutionError> {
    let Value::PriorEffectMetric { effect_id, query } = amount.unhinted() else {
        return Err(ExecutionError::UnresolvableValue(
            "the mana lost this way requires its mana-loss producer".into(),
        ));
    };
    if query.action != Some(ironsmith_core::PriorEffectAction::ManaLost) {
        return Err(ExecutionError::UnresolvableValue(
            "the mana lost this way requires a mana-loss metric".into(),
        ));
    }
    let Some(outcome) = ctx.get_outcome(*effect_id) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut units: Vec<LostUnit> = Vec::new();
    let mut untracked = Vec::new();
    for loss in outcome
        .events_of_type::<crate::events::ManaLostEvent>()
        .filter(|loss| loss.converted_to.is_none())
    {
        let mut pool = loss.mana.clone();
        for unit in &loss.lost_units {
            if pool.remove(unit.0.symbol, 1) {
                units.push(unit.clone());
            }
        }
        for (count, symbol) in [
            (pool.white, ManaSymbol::White),
            (pool.blue, ManaSymbol::Blue),
            (pool.black, ManaSymbol::Black),
            (pool.red, ManaSymbol::Red),
            (pool.green, ManaSymbol::Green),
            (pool.colorless, ManaSymbol::Colorless),
        ] {
            untracked.extend(std::iter::repeat_n(symbol, count as usize));
        }
    }
    let mut symbols: Vec<ManaSymbol> = units.iter().map(|unit| unit.0.symbol).collect();
    symbols.extend(untracked);
    Ok((symbols, units))
}

impl EffectExecutor for AddManaLostThisWayEffect {
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
        use super::choice_helpers::{credit_mana_symbols_from_context, mana_added_value_outputs};
        let player = crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
        let (symbols, units) = lost_mana(&self.amount, ctx)?;
        let before = game
            .player(player)
            .map_or(0, |data| data.mana_source_provenance.len());
        let receipt = credit_mana_symbols_from_context(game, player, symbols.clone(), ctx)?;
        // CR 106.13: the same mana moves to the new pool. Its original
        // producer, snapshot (snow, card types) and spending restrictions
        // stay with it; this effect is not its producer.
        if receipt.original_committed && receipt.mana == symbols && !units.is_empty() {
            game.with_player_mana_mut(player, |data| {
                if data.mana_source_provenance.len() != before + symbols.len() {
                    return;
                }
                for (offset, (provenance, restriction)) in units.into_iter().enumerate() {
                    let entry = &mut data.mana_source_provenance[before + offset];
                    if entry.symbol != provenance.symbol || entry.restricted {
                        continue;
                    }
                    entry.source = provenance.source;
                    entry.snapshot = provenance.snapshot;
                    if let Some(mut restriction) = restriction {
                        restriction.symbol = entry.symbol;
                        entry.restricted = true;
                        data.restricted_mana.push(restriction);
                    }
                }
            });
        }
        Ok(mana_added_value_outputs(ctx, player, receipt))
    }
}
