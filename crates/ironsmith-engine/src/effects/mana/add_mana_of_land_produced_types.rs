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
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::LandProducedTypes {
            amount: &self.amount,
            player: &self.player,
            filter: &self.land_filter,
            allow_colorless: self.allow_colorless,
            same_type: self.same_type,
            source: self.mana_type_source,
        })
    }

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
                collect_triggering_event_mana_symbols(game, ctx, &self.land_filter)?
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
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        let chosen_symbols =
            credit_mana_symbols_from_context(game, player_id, chosen_symbols, ctx)?;

        Ok(mana_added_count_outcome(
            ctx,
            player_id,
            chosen_symbols,
            amount as i32,
        ))
    }
}

pub(super) fn collect_triggering_event_mana_symbols(
    game: &GameState,
    ctx: &ExecutionContext,
    source_filter: &ObjectFilter,
) -> Result<Vec<ManaSymbol>, ExecutionError> {
    let Some(event) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::ManaAddedEvent>())
    else {
        return Err(ExecutionError::IncompleteEvidence("produced mana types require the exact triggering production event".into()));
    };

    // This is an event-time comparison. A later live object cannot fill a
    // missing production receipt, even if it still has the same identity.
    let snapshot = event.snapshot.as_ref().ok_or_else(|| ExecutionError::IncompleteEvidence(
        "produced mana types require the event-time source snapshot".into(),
    ))?;
    if snapshot.object_id != event.source {
        return Err(ExecutionError::IncompleteEvidence("mana production source snapshot belongs to a different object".into()));
    }
    let filter_ctx = ctx.filter_context(game);
    if !source_filter.matches_snapshot(snapshot, &filter_ctx, game) {
        return Ok(Vec::new());
    }
    if event.mana.iter().any(|symbol| !matches!(symbol,
                ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green
                    | ManaSymbol::Colorless)) {
        return Err(ExecutionError::IncompleteEvidence("mana production receipt contains an unresolved mana symbol".into()));
    }
    let mut symbols = event.mana.clone();
    symbols.sort_by_key(|symbol| canonical_symbol_order(*symbol));
    symbols.dedup();
    Ok(symbols)
}

pub(super) fn collect_available_mana_symbols(
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
                    recipient: None,
                    ..Default::default()
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

pub(super) fn is_allowed_symbol(symbol: ManaSymbol, allow_colorless: bool) -> bool {
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

#[cfg(test)]
mod retained_production_evidence_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::{CardId, CardType, Zone};
    #[test]
    fn absent_mismatched_and_known_empty_production_receipts_stay_distinct() {
        for case in 0..5 {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let player = crate::PlayerId(0);
            let definition = CardBuilder::new(CardId::new(), "Production source").card_types(vec![CardType::Land]).build();
            let source = game.create_object_from_card(&definition, player, Zone::Battlefield);
            let mut ctx = ExecutionContext::new_default(source, player);
            if case > 0 {
                let mut snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
                if case == 2 { snapshot.object_id = crate::ObjectId::from_raw(999_991); }
                let event = crate::events::ManaAddedEvent::new(source, player, player,
                    if case == 3 { Vec::new() } else { vec![ManaSymbol::Blue] })
                    .with_snapshot((case != 1).then_some(snapshot));
                ctx = ctx.with_triggering_event(event.into_trigger_event());
            }
            let effect = AddManaOfLandProducedTypesEffect::from_triggering_event(1, PlayerFilter::You, ObjectFilter::land(), true, false);
            let projection = effect.mana_production().unwrap().resolve(&game, &ctx);
            if case < 3 {
                assert!(matches!(projection, Err(ExecutionError::IncompleteEvidence(_))));
                assert!(matches!(effect.execute(&mut game, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
                assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
            } else {
                projection.unwrap();
                if case == 4 { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                effect.execute(&mut game, &mut ctx).unwrap();
                assert_eq!(game.player(player).unwrap().mana_pool.blue, u32::from(case == 4));
            }
        }
    }
}
