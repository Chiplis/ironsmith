//! Add one mana of each color among objects matching a filter.

use std::collections::HashMap;

use super::choice_helpers::{credit_mana_symbols_from_context, mana_added_count_outcome};
use crate::color::Color;
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::object_query::candidate_ids_for_filter;
use crate::snapshot::ObjectSnapshot;
use crate::tag::{SOURCE_EXILED_TAG, TagKey};
use crate::target::ObjectFilter;

pub type AddManaOfColorsAmongEffect = ironsmith_core::AddManaOfColorsAmongEffect;

impl EffectExecutor for AddManaOfColorsAmongEffect {
    fn directly_produces_mana(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let symbols = colors_among_filter(game, &self.filter, ctx.source, player_id);
        if symbols.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        credit_mana_symbols_from_context(game, player_id, symbols.iter().copied(), ctx);
        Ok(mana_added_count_outcome(
            ctx,
            player_id,
            symbols.clone(),
            symbols.len() as i32,
        ))
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

fn colors_among_filter(
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
    for id in candidate_ids_for_filter(game, filter) {
        let Some(obj) = game.object(id) else {
            continue;
        };
        if !filter.matches(obj, &filter_ctx, game) {
            continue;
        }
        let color_set = obj.colors();
        push_color_if_present(&mut colors, color_set, Color::White, ManaSymbol::White);
        push_color_if_present(&mut colors, color_set, Color::Blue, ManaSymbol::Blue);
        push_color_if_present(&mut colors, color_set, Color::Black, ManaSymbol::Black);
        push_color_if_present(&mut colors, color_set, Color::Red, ManaSymbol::Red);
        push_color_if_present(&mut colors, color_set, Color::Green, ManaSymbol::Green);
    }
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
