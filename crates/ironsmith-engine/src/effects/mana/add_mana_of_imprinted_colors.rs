//! Add mana of imprinted card's colors effect implementation.
//!
//! Used by Chrome Mox and Pit of Offerings-style permanents to produce mana
//! based on the colors of the cards they exiled.

use crate::color::{Color, ColorSet};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub type AddManaOfImprintedColorsEffect = ironsmith_core::AddManaOfImprintedColorsEffect;

impl EffectExecutor for AddManaOfImprintedColorsEffect {
    fn mana_production(&self) -> Option<crate::mana_payment::program::ManaProduction<'_>> {
        use crate::mana_payment::program::ManaProduction;
        Some(ManaProduction::ImprintedColors)
    }

    fn directly_produces_mana(&self) -> bool {
        true
    }

    #[crate::effects::project_effect_outcome]
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        use super::production_resolution::{execute_production, ManaOutputProjection, ManaChoiceMenu};
        execute_production(self.mana_production().expect("mana production descriptor"),
            game, ctx, ManaOutputProjection::AuthoredCount, ManaChoiceMenu::AvailableColors)
    }

    fn producible_mana_symbols(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        let symbols: Vec<ManaSymbol> = linked_exiled_card_colors(game, source)
            .into_iter()
            .map(ManaSymbol::from_color)
            .collect();
        if symbols.is_empty() {
            return None;
        }
        Some(symbols)
    }
}

/// The distinct colors among the cards imprinted on, or exiled with,
/// `source`, in WUBRG order.
pub(super) fn linked_exiled_card_colors(
    game: &GameState,
    source: crate::ids::ObjectId,
) -> Vec<Color> {
    let imprinted = game.get_imprinted_cards(source);
    let exiled_with = game.get_exiled_with_source_links(source);
    let mut colors = ColorSet::COLORLESS;
    for &id in imprinted.iter().chain(exiled_with) {
        if !game.object(id).is_some_and(|object| object.zone == crate::Zone::Exile) {
            continue;
        }
        match game.try_current_characteristics(id) {
            Ok(Some(chars)) => colors = colors.union(chars.colors),
            Ok(None) => {}
            Err(error) => {
                game.record_token_resource_failure(&ExecutionError::ContinuousDiscovery(error));
                return Vec::new();
            }
        }
    }
    Color::ALL.into_iter().filter(|&color| colors.contains(color)).collect()
}
