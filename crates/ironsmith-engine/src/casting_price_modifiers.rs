//! Additional casting prices supplied by active battlefield abilities.
use crate::cost::{OptionalCostRef, TotalCost};
use crate::costs::Cost;
use crate::filter::ObjectFilterExt as _;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::Object;
use crate::{GameState, PlayerId};
use ironsmith_core::StaticAbilityPayload as Payload;

#[derive(Default)]
pub(crate) struct CastingPriceModifiers {
    pub(crate) replacements: Vec<(OptionalCostRef, TotalCost)>,
    discounts: Vec<(Vec<String>, u32, bool)>,
}

impl CastingPriceModifiers {
    pub(crate) fn for_spell(game: &GameState, spell: &Object, caster: PlayerId) -> Self {
        let mut spell = spell.clone();
        if let Some(chars) = game.current_characteristics(spell.id) {
            spell.name = chars.name;
            spell.card_types = chars.card_types;
            spell.subtypes = chars.subtypes;
            spell.supertypes = chars.supertypes;
            spell.color_override = Some(chars.colors);
        }
        let mut modifiers = Self::default();
        let view = crate::derived_view::DerivedGameView::new(game);
        for source in game
            .battlefield
            .iter()
            .copied()
            .filter(|id| !game.is_phased_out(*id))
        {
            let Some(abilities) = view.static_abilities_rc(source) else {
                continue;
            };
            let Some(owner) = game.object(source) else {
                continue;
            };
            for ability in abilities.iter() {
                let Some(model) = ability.compiled_model() else {
                    continue;
                };
                let payload = match &model.payload {
                    Payload::Conditional { ability, .. } => &ability.payload,
                    payload => payload,
                };
                let matches = |filter: &crate::ObjectFilter| {
                    if !ability.is_active(game, source) {
                        return false;
                    }
                    let context = game
                        .filter_context_for(game.controller_of(owner), Some(source))
                        .with_caster(Some(caster))
                        .with_prospective_cast(spell.id);
                    let mut filter = filter.clone();
                    filter.zone = None;
                    filter.matches_non_recursive(&spell, &context, game)
                };
                match payload {
                    Payload::OptionalCostPaymentAlternative {
                        filter,
                        reference,
                        replacement,
                    } if matches(filter) => modifiers
                        .replacements
                        .push((reference.clone(), replacement.clone())),
                    Payload::NamedCastingPriceReduction {
                        filter,
                        keywords,
                        maximum,
                        optional,
                    } if matches(filter) => {
                        modifiers
                            .discounts
                            .push((keywords.clone(), *maximum, *optional))
                    }
                    _ => {}
                }
            }
        }
        modifiers
    }

    /// Preserve announcement order: discount amount first, then authored payment order.
    pub(crate) fn named_prices<'a>(
        &self,
        keyword: &str,
        branches: impl Iterator<Item = &'a [Cost]>,
    ) -> Vec<TotalCost> {
        let (mut required, mut optional) = (0u32, 0u32);
        for (keywords, amount, may) in &self.discounts {
            if keywords
                .iter()
                .any(|name| name.eq_ignore_ascii_case(keyword))
            {
                let total = if *may { &mut optional } else { &mut required };
                *total = total.saturating_add(*amount);
            }
        }
        let prices: Vec<_> = branches.map(NamedPrice::new).collect();
        let limit = required.saturating_add(optional);
        let cap = if limit == 0 {
            0
        } else {
            prices
                .iter()
                .map(|price| price.maximum_discount(limit))
                .max()
                .unwrap_or(0)
        };
        (required.min(cap)..=cap)
            .flat_map(|amount| prices.iter().map(move |price| price.discounted(amount)))
            .collect()
    }
}

struct NamedPrice {
    mana: ManaCost,
    first_mana: Option<usize>,
    other: Vec<Cost>,
}

impl NamedPrice {
    fn new(parts: &[Cost]) -> Self {
        let mut mana = ManaCost::new();
        let mut other = Vec::new();
        let mut first_mana = None;
        for part in parts {
            if let Some(payment) = part.mana_cost_ref() {
                first_mana.get_or_insert(other.len());
                mana = crate::decision::add_mana_cost(&mana, payment);
            } else {
                other.push(part.clone());
            }
        }
        Self {
            mana,
            first_mana,
            other,
        }
    }

    fn maximum_discount(&self, limit: u32) -> u32 {
        // X is announced later. Until then every provider-supported amount is distinct.
        if self.mana.has_x() {
            return limit;
        }
        self.mana
            .pips()
            .iter()
            .map(|pip| {
                pip.iter()
                    .filter_map(|symbol| match symbol {
                        ManaSymbol::Generic(n) => Some(u32::from(*n)),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0)
            })
            .fold(0, u32::saturating_add)
            .min(limit)
    }

    fn discounted(&self, amount: u32) -> TotalCost {
        let mut parts = self.other.clone();
        if let Some(position) = self.first_mana {
            // A local discount waits for the receiving spell's symbol announcements.
            let payment = if amount > 0
                && (self.mana.has_x() || self.mana.pips().iter().any(|pip| pip.len() > 1))
            {
                let mut recipe = ironsmith_core::DynamicManaCost::new(
                    self.mana.clone(),
                    None,
                    None,
                    None,
                    ironsmith_core::DynamicManaDisplayHint::Default,
                );
                recipe.generic_reduction = amount;
                Cost::dynamic_mana(recipe)
            } else {
                Cost::mana(self.mana.reduce_generic(amount))
            };
            parts.insert(position, payment);
        }
        TotalCost::from_costs(parts)
    }
}
