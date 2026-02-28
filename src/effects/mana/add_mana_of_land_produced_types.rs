//! Add mana of any color/type that lands matching a filter could produce.

use super::choice_helpers::{choose_mana_symbols, credit_mana_symbols};
use crate::ability::{AbilityKind, ActivatedAbility};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::object::Object;
use crate::target::{ObjectFilter, PlayerFilter};

/// Effect that adds mana constrained to what matching lands could produce.
///
/// This models text like:
/// - "Add one mana of any color that a land an opponent controls could produce."
/// - "Add one mana of any type that a Gate you control could produce."
#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfLandProducedTypesEffect {
    /// Number of mana to add.
    pub amount: Value,
    /// Which player receives the mana.
    pub player: PlayerFilter,
    /// Lands to inspect for producible mana.
    pub land_filter: ObjectFilter,
    /// Whether colorless mana is allowed ("any type" vs "any color").
    pub allow_colorless: bool,
    /// If true, all mana must be the same type.
    pub same_type: bool,
}

impl AddManaOfLandProducedTypesEffect {
    pub fn new(
        amount: impl Into<Value>,
        player: PlayerFilter,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            land_filter,
            allow_colorless,
            same_type,
        }
    }
}

impl EffectExecutor for AddManaOfLandProducedTypesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        if amount == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let available = collect_available_mana_symbols(game, ctx, &self.land_filter);
        let available = available
            .into_iter()
            .filter(|symbol| is_allowed_symbol(*symbol, self.allow_colorless))
            .collect::<Vec<_>>();
        if available.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let chosen_symbols = choose_mana_symbols(
            game,
            ctx,
            player_id,
            amount,
            self.same_type,
            &available,
            available[0],
        );

        credit_mana_symbols(game, player_id, chosen_symbols);

        Ok(EffectOutcome::count(amount as i32))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

fn collect_available_mana_symbols(
    game: &GameState,
    ctx: &ExecutionContext,
    land_filter: &ObjectFilter,
) -> Vec<ManaSymbol> {
    let mut symbols = Vec::new();
    let filter_ctx = ctx.filter_context(game);
    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        if !perm.is_land() || !land_filter.matches(perm, &filter_ctx, game) {
            continue;
        }

        for ability in &perm.abilities {
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.is_mana_ability() {
                continue;
            }
            if !mana_ability_condition_met(game, perm, mana_ability) {
                continue;
            }

            for symbol in mana_ability.mana_symbols() {
                push_symbol_if_addable(&mut symbols, *symbol);
            }
            for effect in &mana_ability.effects {
                infer_symbols_from_mana_effect(
                    game,
                    perm.id,
                    perm.controller,
                    effect,
                    &mut symbols,
                );
            }
        }
    }

    symbols.sort_by_key(|symbol| canonical_symbol_order(*symbol));
    symbols.dedup();
    symbols
}

fn mana_ability_condition_met(
    game: &GameState,
    source: &Object,
    mana_ability: &ActivatedAbility,
) -> bool {
    mana_ability
        .activation_condition
        .as_ref()
        .is_none_or(|condition| {
            let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
                controller: source.controller,
                source: source.id,
                filter_source: Some(source.id),
                triggering_event: None,
                trigger_identity: None,
                ability_index: None,
                options: crate::condition_eval::ExternalEvaluationOptions {
                    // For mana-production inference we only care about what colors can be
                    // produced, not whether the ability is currently activatable by timing/limits.
                    ignore_timing: true,
                    ignore_activation_limits: true,
                },
            };
            crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
        })
}

fn infer_symbols_from_mana_effect(
    game: &GameState,
    source: crate::ids::ObjectId,
    land_controller: crate::ids::PlayerId,
    effect: &crate::effect::Effect,
    out: &mut Vec<ManaSymbol>,
) {
    if let Some(inferred) = effect.producible_mana_symbols(game, source, land_controller) {
        for symbol in inferred {
            push_symbol_if_addable(out, symbol);
        }
    }
}

fn push_symbol_if_addable(out: &mut Vec<ManaSymbol>, symbol: ManaSymbol) {
    if matches!(
        symbol,
        ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless
    ) {
        out.push(symbol);
    }
}

fn is_allowed_symbol(symbol: ManaSymbol, allow_colorless: bool) -> bool {
    match symbol {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green => true,
        ManaSymbol::Colorless => allow_colorless,
        _ => false,
    }
}

fn canonical_symbol_order(symbol: ManaSymbol) -> usize {
    match symbol {
        ManaSymbol::White => 0,
        ManaSymbol::Blue => 1,
        ManaSymbol::Black => 2,
        ManaSymbol::Red => 3,
        ManaSymbol::Green => 4,
        ManaSymbol::Colorless => 5,
        _ => 100,
    }
}
