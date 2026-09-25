//! "Note the type of mana spent to pay this activation cost" and "Add one
//! mana of this artifact's last noted type".

use super::choice_helpers::{credit_repeated_mana_symbol_from_context, mana_added_count_outcome};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;

pub type NoteActivationManaTypeEffect = ironsmith_core::NoteActivationManaTypeEffect;
pub type AddManaOfNotedTypeEffect = ironsmith_core::AddManaOfNotedTypeEffect;

const MANA_TYPES: [ManaSymbol; 6] = [
    ManaSymbol::White,
    ManaSymbol::Blue,
    ManaSymbol::Black,
    ManaSymbol::Red,
    ManaSymbol::Green,
    ManaSymbol::Colorless,
];

impl EffectExecutor for NoteActivationManaTypeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let payment = &ctx.mana.activation_payment;
        let mut spent = MANA_TYPES
            .iter()
            .copied()
            .filter(|symbol| payment.amount(*symbol) > 0);
        // A type is noted only when the payment had exactly one type of mana
        // (the printed cost is a single generic mana).
        let (Some(symbol), None) = (spent.next(), spent.next()) else {
            return Ok(EffectOutcome::count(0));
        };
        game.note_mana_type_for_source(ctx.source, symbol);
        Ok(EffectOutcome::count(1))
    }
}

impl EffectExecutor for AddManaOfNotedTypeEffect {
    fn directly_produces_mana(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        let Some(symbol) = game.noted_mana_type_for_source(ctx.source) else {
            return Ok(EffectOutcome::count(0));
        };
        if amount == 0 {
            return Ok(EffectOutcome::count(0));
        }
        let mana = credit_repeated_mana_symbol_from_context(game, player_id, symbol, amount, ctx);
        Ok(mana_added_count_outcome(
            ctx,
            player_id,
            mana,
            amount as i32,
        ))
    }

    fn producible_mana_symbols(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        Some(
            game.noted_mana_type_for_source(source)
                .into_iter()
                .collect(),
        )
    }
}
