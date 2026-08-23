//! Add mana of any color/type that lands matching a filter could produce.

use super::choice_helpers::{
    choose_mana_symbols, credit_mana_symbols_from_context, mana_added_count_outcome,
};
use crate::ability::{AbilityKind, ActivatedAbility, ActivatedAbilityRuntimeExt as _};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::object::Object;
use crate::target::{ObjectFilter, PlayerFilter};
pub use ironsmith_core::ManaTypeSource;

/// Effect that adds mana constrained by a set of land-produced mana types.
///
/// This models text like:
/// - "Add one mana of any color that a land an opponent controls could produce."
/// - "Add one mana of any type that a Gate you control could produce."
/// - "That player adds one mana of any type that land produced."
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
    /// Whether to inspect potential land abilities or the actual triggering event.
    pub mana_type_source: ManaTypeSource,
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
            mana_type_source: ManaTypeSource::MatchingLandsCouldProduce,
        }
    }

    pub fn from_triggering_event(
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
            mana_type_source: ManaTypeSource::TriggeringEventProduced,
        }
    }
}

impl EffectExecutor for AddManaOfLandProducedTypesEffect {
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
        if amount == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let available = match self.mana_type_source {
            ManaTypeSource::MatchingLandsCouldProduce => {
                collect_available_mana_symbols(game, ctx, &self.land_filter)
            }
            ManaTypeSource::TriggeringEventProduced => {
                collect_triggering_event_mana_symbols(game, ctx, &self.land_filter)
            }
        };
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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        let chosen_symbols = credit_mana_symbols_from_context(game, player_id, chosen_symbols, ctx);

        Ok(mana_added_count_outcome(
            ctx,
            player_id,
            chosen_symbols,
            amount as i32,
        ))
    }
}

fn collect_triggering_event_mana_symbols(
    game: &GameState,
    ctx: &ExecutionContext,
    source_filter: &ObjectFilter,
) -> Vec<ManaSymbol> {
    let Some(event) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::ManaAddedEvent>())
    else {
        return Vec::new();
    };

    // A captured snapshot is the characteristics of the mana source at the
    // moment it produced mana. Prefer it to the possibly changed live object.
    let filter_ctx = ctx.filter_context(game);
    let source_matches = if let Some(snapshot) = event.snapshot.as_ref() {
        source_filter.matches_snapshot(snapshot, &filter_ctx, game)
    } else if let Some(source) = game.object(event.source) {
        source_filter.matches(source, &filter_ctx, game)
    } else {
        false
    };
    if !source_matches {
        return Vec::new();
    }

    let mut symbols = event
        .mana
        .iter()
        .copied()
        .filter(|symbol| {
            matches!(
                symbol,
                ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green
                    | ManaSymbol::Colorless
            )
        })
        .collect::<Vec<_>>();
    symbols.sort_by_key(|symbol| canonical_symbol_order(*symbol));
    symbols.dedup();
    symbols
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

        let abilities = game
            .current_abilities(perm_id)
            .unwrap_or_else(|| perm.abilities_vec());
        for ability in &abilities {
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.is_runtime_mana_ability(game, perm.id, game.controller_of(perm)) {
                continue;
            }
            if !mana_ability_condition_met(game, perm, mana_ability) {
                continue;
            }

            for symbol in
                mana_ability.inferred_mana_symbols(game, perm.id, game.controller_of(perm))
            {
                push_symbol_if_addable(&mut symbols, symbol);
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
                controller: game.controller_of(source),
                source: source.id,
                defending_player: None,
                attacking_player: None,
                filter_source: Some(source.id),
                iterated_player: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::events::mana::ManaProductionProvenance;
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn triggering_event_mode_adds_only_a_type_the_land_actually_produced() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let land = CardDefinitionBuilder::new(CardId::new(), "Abilityless Land")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_definition(&land, bob, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(land_id).expect("land should exist"), &game);

        // Resolve from LKI to prove this does not recompute what the current
        // battlefield object could produce.
        game.remove_object(land_id);
        let event = crate::events::ManaAddedEvent::new(land_id, bob, bob, vec![ManaSymbol::Red])
            .with_snapshot(Some(snapshot))
            .with_production_provenance(ManaProductionProvenance::TappedSourceForMana)
            .into_trigger_event();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let effect = AddManaOfLandProducedTypesEffect::from_triggering_event(
            1,
            PlayerFilter::IteratedPlayer,
            ObjectFilter::land(),
            true,
            false,
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("actual produced-type effect should resolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(bob).expect("Bob should exist").mana_pool.red, 1);
        assert_eq!(
            game.player(bob).expect("Bob should exist").mana_pool.green,
            0
        );
    }

    #[test]
    fn triggering_event_mode_respects_type_and_source_filter_restrictions() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let land = CardDefinitionBuilder::new(CardId::new(), "Ordinary Land")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        let snapshot =
            ObjectSnapshot::from_object(game.object(land_id).expect("land should exist"), &game);
        let event =
            crate::events::ManaAddedEvent::new(land_id, alice, alice, vec![ManaSymbol::Colorless])
                .with_snapshot(Some(snapshot))
                .with_production_provenance(ManaProductionProvenance::TappedSourceForMana)
                .into_trigger_event();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let effect = AddManaOfLandProducedTypesEffect::from_triggering_event(
            1,
            PlayerFilter::You,
            ObjectFilter::land(),
            false,
            false,
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("color-restricted produced-type effect should resolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            game.player(alice)
                .expect("Alice should exist")
                .mana_pool
                .colorless,
            0
        );
    }
}
