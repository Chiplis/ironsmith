//! Bind actual-color activation history before the ordinary payment solver.
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::{GameState, ObjectId};
use ironsmith_core::mana::ManaSpendingRestriction;
pub(crate) fn needs_binding(cost: &TotalCost) -> bool {
    match cost.kind() {
        ironsmith_core::TotalCostKind::OneOf(branches) => branches.iter().any(needs_binding),
        ironsmith_core::TotalCostKind::All(parts) => parts.iter().any(|part| {
            part.mana_cost_ref().is_some_and(|mana| {
                mana.spending_restrictions()
                    .contains(&ManaSpendingRestriction::UnspentActivationColors)
            }) || part.dynamic_mana_cost_ref().is_some_and(|cost| {
                cost.base
                    .spending_restrictions()
                    .contains(&ManaSpendingRestriction::UnspentActivationColors)
            })
        }),
    }
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
    fn resolved(cost: &TotalCost, colors: crate::color::ColorSet) -> TotalCost {
        match cost.kind() {
            ironsmith_core::TotalCostKind::OneOf(branches) => TotalCost::one_of(
                branches
                    .iter()
                    .map(|branch| resolved(branch, colors))
                    .collect(),
            ),
            ironsmith_core::TotalCostKind::All(parts) => TotalCost::from_costs(
                parts
                    .iter()
                    .map(|part| {
                        if let Some(mana) = part.mana_cost_ref() {
                            Cost::mana(mana.clone().resolve_activation_history_colors(colors))
                        } else if let Some(dynamic) = part.dynamic_mana_cost_ref() {
                            let mut dynamic = dynamic.clone();
                            dynamic.base = dynamic.base.resolve_activation_history_colors(colors);
                            Cost::dynamic_mana(dynamic)
                        } else {
                            part.clone()
                        }
                    })
                    .collect(),
            ),
        }
    }
    resolved(cost, colors)
}
