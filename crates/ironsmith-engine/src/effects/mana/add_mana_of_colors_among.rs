//! Add one mana of each color among objects matching a filter.

use std::collections::HashMap;

use super::choice_helpers::{credit_mana_symbols_from_context, mana_added_count_outputs};
use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::object_query::for_each_candidate_id_for_filter;
use crate::snapshot::ObjectSnapshot;
use crate::tag::{SOURCE_EXILED_TAG, TagKey};
use crate::target::ObjectFilter;

pub type AddManaOfColorsAmongEffect = ironsmith_core::AddManaOfColorsAmongEffect;

impl EffectExecutor for AddManaOfColorsAmongEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::ColorsAmong {
            filter: &self.filter,
            choose_one: false,
            player: &self.player,
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let symbols = colors_among_for_execution(game, &self.filter, ctx, player_id)?;
        if symbols.is_empty() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }

        let symbols = credit_mana_symbols_from_context(game, player_id, symbols, ctx)?;
        let count = symbols.mana_count();
        Ok(mana_added_count_outputs(ctx, player_id, symbols, count))
    }

    fn producible_mana_symbols(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let symbols = colors_among_filter(game, &self.filter, source, controller);
        (!symbols.is_empty()).then_some(symbols)
    }
}

/// Execution and the shared mana planner use the same exact self-reference.
/// A sacrificed or phased source reads its retained colors; no snapshot is
/// incomplete evidence rather than an invented colorless result.
pub(super) fn colors_among_for_execution(
    game: &GameState,
    filter: &ObjectFilter,
    ctx: &ExecutionContext,
    controller: PlayerId,
) -> Result<Vec<ManaSymbol>, ExecutionError> {
    if !filter.is_source_only() {
        return Ok(colors_among_filter(game, filter, ctx.source, controller));
    }
    let colors = match game
        .try_current_characteristics(ctx.source)
        .map_err(ExecutionError::ContinuousDiscovery)?
    {
        Some(chars) => chars.colors,
        None => ctx
            .source_snapshot
            .as_ref()
            .filter(|snapshot| snapshot.object_id == ctx.source)
            .map(|snapshot| snapshot.colors)
            .ok_or_else(|| {
                ExecutionError::IncompleteEvidence(
                    "source-color mana requires its exact source or last-known snapshot".into(),
                )
            })?,
    };
    Ok(Color::ALL
        .into_iter()
        .filter(|color| colors.contains(*color))
        .map(ManaSymbol::from_color)
        .collect())
}

pub(super) fn colors_among_filter(
    game: &GameState,
    filter: &ObjectFilter,
    source: ObjectId,
    controller: PlayerId,
) -> Vec<ManaSymbol> {
    let mut tagged_objects = HashMap::new();
    let source_exiled = game
        .get_exiled_with_source_links(source)
        .iter()
        .filter_map(|id| {
            game.object(*id)
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        tagged_objects.insert(TagKey::from(SOURCE_EXILED_TAG), source_exiled);
    }

    let filter_ctx = game
        .filter_context_for(controller, Some(source))
        .with_tagged_objects(&tagged_objects);
    let mut colors = Vec::new();
    for_each_candidate_id_for_filter(game, filter, |id| {
        let Some(obj) = game.object(id) else {
            return;
        };
        if !filter.matches(obj, &filter_ctx, game) {
            return;
        }
        let color_set = match game.try_current_characteristics(id) {
            Ok(Some(chars)) => chars.colors,
            Ok(None) => return,
            Err(error) => {
                game.record_token_resource_failure(&ExecutionError::ContinuousDiscovery(error));
                return;
            }
        };
        push_color_if_present(&mut colors, color_set, Color::White, ManaSymbol::White);
        push_color_if_present(&mut colors, color_set, Color::Blue, ManaSymbol::Blue);
        push_color_if_present(&mut colors, color_set, Color::Black, ManaSymbol::Black);
        push_color_if_present(&mut colors, color_set, Color::Red, ManaSymbol::Red);
        push_color_if_present(&mut colors, color_set, Color::Green, ManaSymbol::Green);
    });
    colors
}

fn push_color_if_present(
    out: &mut Vec<ManaSymbol>,
    colors: crate::color::ColorSet,
    color: Color,
    symbol: ManaSymbol,
) {
    if colors.contains(color) && !out.contains(&symbol) {
        out.push(symbol);
    }
}
