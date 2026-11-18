//! Bind actual-color activation history before the ordinary payment solver.
use crate::cost::TotalCost;
use crate::{GameState, ObjectId};
use ironsmith_core::mana::ManaSpendingRestriction;
pub(crate) fn needs_binding(cost: &TotalCost) -> bool {
    cost.payment_branches().flatten().any(|part| {
        part.mana_cost_ref()
            .or_else(|| part.dynamic_mana_cost_ref().map(|cost| &cost.base))
            .is_some_and(|mana| mana.spending_restrictions().contains(&ManaSpendingRestriction::UnspentActivationColors))
    })
}
pub(crate) fn bind(
    game: &GameState,
    source: ObjectId,
    index: Option<usize>,
    cost: &TotalCost,
) -> TotalCost {
    let identity = index.and_then(|index| {
        game.current_characteristics(source).and_then(|chars| {
            let origin = chars.abilities.origin(index)?.clone();
            let crate::ability::AbilityKind::Activated(ability) = &chars.abilities.get(index)?.kind
            else {
                return None;
            };
            Some((origin, ability.effects.activation_definition))
        })
    });
    let colors = game.unused_activation_mana_colors(
        source,
        identity.as_ref().map(|(origin, _)| origin),
        identity.as_ref().and_then(|(_, definition)| *definition),
    );
    crate::cost::map_base_mana(cost, |mana| {
        Some(mana.clone().resolve_activation_history_colors(colors))
    })
}
