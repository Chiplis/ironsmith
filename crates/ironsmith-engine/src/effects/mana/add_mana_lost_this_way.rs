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

/// The mana the producer behind `amount` removed, as symbols.
fn lost_mana_symbols(
    amount: &Value,
    ctx: &ExecutionContext,
) -> Result<Vec<ManaSymbol>, ExecutionError> {
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
        return Ok(Vec::new());
    };
    let mut symbols = Vec::new();
    for loss in outcome
        .events_of_type::<crate::events::ManaLostEvent>()
        .filter(|loss| loss.converted_to.is_none())
    {
        let pool = &loss.mana;
        for (count, symbol) in [
            (pool.white, ManaSymbol::White),
            (pool.blue, ManaSymbol::Blue),
            (pool.black, ManaSymbol::Black),
            (pool.red, ManaSymbol::Red),
            (pool.green, ManaSymbol::Green),
            (pool.colorless, ManaSymbol::Colorless),
        ] {
            symbols.extend(std::iter::repeat_n(symbol, count as usize));
        }
    }
    Ok(symbols)
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
        let symbols = lost_mana_symbols(&self.amount, ctx)?;
        let receipt = credit_mana_symbols_from_context(game, player, symbols, ctx)?;
        Ok(mana_added_value_outputs(ctx, player, receipt))
    }
}
