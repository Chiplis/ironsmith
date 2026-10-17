//! "Add mana equal to enchanted permanent's mana cost" (Elemental Resonance).
//!
//! Each mana symbol of the object's mana cost adds mana: a colored symbol its
//! color, generic N that much colorless, a Phyrexian symbol its color, {C}
//! and {S} colorless, and {X} nothing (X is 0 outside the stack, CR 107.3b).
//! A hybrid symbol adds one of its halves, chosen by the player getting the
//! mana (CR 107.4e); a monocolored hybrid {2/W} offers two colorless or {W}.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::target::ChooseSpec;

pub type AddManaEqualToManaCostEffect = ironsmith_core::AddManaEqualToManaCostEffect;

/// The mana one printed symbol can add, as alternatives (one entry per
/// choosable half). Each alternative is the full group of symbols it adds.
fn symbol_alternatives(pip: &[ManaSymbol]) -> Vec<Vec<ManaSymbol>> {
    let mut alternatives: Vec<Vec<ManaSymbol>> = Vec::new();
    for symbol in pip {
        let produced = match symbol {
            ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless => vec![*symbol],
            ManaSymbol::Snow => vec![ManaSymbol::Colorless],
            ManaSymbol::Generic(amount) => vec![ManaSymbol::Colorless; usize::from(*amount)],
            // A Phyrexian symbol's life half and {X} add no mana.
            ManaSymbol::Life(_) | ManaSymbol::X => continue,
        };
        if !alternatives.contains(&produced) {
            alternatives.push(produced);
        }
    }
    alternatives
}

/// Colored-symbols-only mode keeps only the choices that are one colored
/// mana symbol.
fn effect_symbol_alternatives(
    effect: &AddManaEqualToManaCostEffect,
    pip: &[ManaSymbol],
) -> Vec<Vec<ManaSymbol>> {
    let mut alternatives = symbol_alternatives(pip);
    if effect.colored_symbols_only {
        alternatives.retain(|group| {
            matches!(
                group.as_slice(),
                [ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green]
            )
        });
    }
    alternatives
}

/// Every object's mana symbols, in printed order, for the first object the
/// filter names as the effect resolves.
fn referenced_mana_cost_pips(
    game: &GameState,
    effect: &AddManaEqualToManaCostEffect,
    ctx: &ExecutionContext,
) -> Result<Vec<Vec<ManaSymbol>>, ExecutionError> {
    let objects = match crate::effects::helpers::resolve_objects_from_spec(
        game,
        &ChooseSpec::All(effect.filter.clone()),
        ctx,
    ) {
        Ok(objects) => objects,
        Err(error) if error.is_incomplete_execution() => return Err(error),
        Err(_) => Vec::new(),
    };
    let Some(cost) = objects
        .first()
        .and_then(|&id| game.object(id))
        .and_then(|object| object.mana_cost_owned())
    else {
        return Ok(Vec::new());
    };
    Ok(cost.pips().to_vec())
}

impl EffectExecutor for AddManaEqualToManaCostEffect {
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
        use super::choice_helpers::{
            choose_mana_symbols, credit_mana_symbols_from_context, mana_added_value_outputs,
        };
        let player = crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
        let mut symbols = Vec::new();
        for pip in referenced_mana_cost_pips(game, self, ctx)? {
            let alternatives = effect_symbol_alternatives(self, &pip);
            match alternatives.as_slice() {
                [] => {}
                [only] => symbols.extend(only.iter().copied()),
                _ => {
                    // Offer one representative symbol per half; a generic
                    // half is offered as colorless.
                    let offered: Vec<ManaSymbol> = alternatives
                        .iter()
                        .map(|group| group.first().copied().unwrap_or(ManaSymbol::Colorless))
                        .collect();
                    let chosen = choose_mana_symbols(
                        game,
                        ctx,
                        player,
                        1,
                        false,
                        &offered,
                        offered[0],
                    )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    let picked = chosen.first().copied().unwrap_or(offered[0]);
                    let index = offered
                        .iter()
                        .position(|symbol| *symbol == picked)
                        .unwrap_or(0);
                    symbols.extend(alternatives[index].iter().copied());
                }
            }
        }
        let receipt = credit_mana_symbols_from_context(game, player, symbols, ctx)?;
        Ok(mana_added_value_outputs(ctx, player, receipt))
    }

    fn producible_mana_symbols(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let ctx = ExecutionContext::new_default(source, controller);
        let symbols: Vec<ManaSymbol> = referenced_mana_cost_pips(game, self, &ctx)
            .ok()?
            .iter()
            .flat_map(|pip| effect_symbol_alternatives(self, pip).into_iter().flatten())
            .collect();
        (!symbols.is_empty()).then_some(symbols)
    }
}
