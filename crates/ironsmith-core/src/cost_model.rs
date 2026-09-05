use crate::tag::TagKeyWalk;

use crate::types::CardType;
use crate::{ColorSet, CounterType, ManaCost, ObjectFilter, Value};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum DynamicManaDisplayHint {
    Default,
    ManaEqualTo,
}

impl Default for DynamicManaDisplayHint {
    fn default() -> Self {
        Self::Default
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct DynamicManaCost {
    pub base: ManaCost,
    /// Resolve the base portion from the object whose spell or ability is
    /// being paid for. This preserves colored and hybrid pips, unlike a mana
    /// value expression, and intentionally fails for cards with no mana cost.
    pub source_mana_cost: bool,
    pub x_value: Option<Value>,
    pub additional_generic: Option<Value>,
    pub multiplier: Option<Value>,
    pub display_hint: DynamicManaDisplayHint,
}

impl DynamicManaCost {
    pub fn new(
        base: ManaCost,
        x_value: Option<Value>,
        additional_generic: Option<Value>,
        multiplier: Option<Value>,
        display_hint: DynamicManaDisplayHint,
    ) -> Self {
        Self {
            base,
            source_mana_cost: false,
            x_value,
            additional_generic,
            multiplier,
            display_hint,
        }
    }

    pub fn from_x(base: ManaCost, x_value: Value) -> Self {
        Self::new(
            base,
            Some(x_value),
            None,
            None,
            DynamicManaDisplayHint::Default,
        )
    }

    pub fn generic_equal_to(value: Value) -> Self {
        Self::new(
            ManaCost::new(),
            None,
            Some(value),
            None,
            DynamicManaDisplayHint::ManaEqualTo,
        )
    }

    pub fn from_source_mana_cost() -> Self {
        Self {
            base: ManaCost::new(),
            source_mana_cost: true,
            x_value: None,
            additional_generic: None,
            multiplier: None,
            display_hint: DynamicManaDisplayHint::Default,
        }
    }

    pub fn resolved_static_base(&self) -> Option<ManaCost> {
        if !self.source_mana_cost
            && self.x_value.is_none()
            && self.additional_generic.is_none()
            && self.multiplier.is_none()
        {
            return Some(self.base.clone());
        }
        None
    }

    pub fn display(&self) -> String {
        if self.source_mana_cost
            && self.x_value.is_none()
            && self.additional_generic.is_none()
            && self.multiplier.is_none()
        {
            return "its mana cost".to_string();
        }
        match self.display_hint {
            DynamicManaDisplayHint::ManaEqualTo => {
                if let Some(value) = self.additional_generic.as_ref() {
                    return format!("mana equal to {value:?}");
                }
            }
            DynamicManaDisplayHint::Default => {}
        }

        let mut text = if self.base.is_empty() {
            "{0}".to_string()
        } else {
            self.base.to_oracle()
        };
        if let Some(value) = self.x_value.as_ref() {
            text.push_str(&format!(", where X is {value:?}"));
        }
        if let Some(value) = self.additional_generic.as_ref() {
            text.push_str(&format!(" plus an additional {{{value:?}}}"));
        }
        if let Some(value) = self.multiplier.as_ref() {
            text.push_str(&format!(" for each {value:?}"));
        }
        text
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "costs retain typed object-filter and effect values inline across crate boundaries"
)]
#[derive(TagKeyWalk)]
pub enum Cost<E> {
    Mana(ManaCost),
    DynamicMana(DynamicManaCost),
    Tap,
    Untap,
    DiscardSource,
    SacrificeSelf,
    Sacrifice(ObjectFilter),
    Discard {
        count: u32,
        card_types: Vec<CardType>,
    },
    DiscardHand,
    RemoveCounters {
        counter_type: CounterType,
        count: u32,
    },
    AddCounters {
        counter_type: CounterType,
        count: u32,
    },
    RemoveAnyCountersFromSource {
        counter_type: Option<CounterType>,
        display_x: bool,
        remove_all: bool,
    },
    Energy(Value),
    Mill(Value),
    Life(Value),
    ExileSelf,
    ExileFromHand {
        count: u32,
        color_filter: Option<ColorSet>,
    },
    ExileFromGraveyard {
        count: u32,
        card_types: Vec<CardType>,
    },
    ReturnSelfToHand,
    Effect(E),
}

impl<E> Cost<E> {
    pub fn effect_ref(&self) -> Option<&E> {
        match self {
            Self::Effect(effect) => Some(effect),
            _ => None,
        }
    }

    pub fn validated_effect(effect: E) -> Self {
        Self::Effect(effect)
    }

    pub fn try_from_runtime_effect(effect: E) -> Result<Self, String> {
        Ok(Self::Effect(effect))
    }

    pub fn mana(mana_cost: ManaCost) -> Self {
        Self::Mana(mana_cost)
    }

    pub fn dynamic_mana(dynamic_mana: DynamicManaCost) -> Self {
        Self::DynamicMana(dynamic_mana)
    }

    pub fn discard_source() -> Self {
        Self::DiscardSource
    }

    pub fn sacrifice_self() -> Self {
        Self::SacrificeSelf
    }

    pub fn sacrifice(filter: ObjectFilter) -> Self {
        Self::Sacrifice(filter)
    }

    pub fn effect(effect: impl Into<E>) -> Self {
        Self::Effect(effect.into())
    }

    pub fn discard(count: u32, card_type: Option<CardType>) -> Self {
        Self::Discard {
            count,
            card_types: card_type.into_iter().collect(),
        }
    }

    pub fn discard_types(count: u32, card_types: Vec<CardType>) -> Self {
        Self::Discard { count, card_types }
    }

    pub fn remove_counters(counter_type: CounterType, count: u32) -> Self {
        Self::RemoveCounters {
            counter_type,
            count,
        }
    }

    pub fn add_counters(counter_type: CounterType, count: u32) -> Self {
        Self::AddCounters {
            counter_type,
            count,
        }
    }

    pub fn remove_any_counters_from_source(
        counter_type: Option<CounterType>,
        display_x: bool,
    ) -> Self {
        Self::RemoveAnyCountersFromSource {
            counter_type,
            display_x,
            remove_all: false,
        }
    }

    pub fn remove_all_counters_from_source(counter_type: Option<CounterType>) -> Self {
        Self::RemoveAnyCountersFromSource {
            counter_type,
            display_x: false,
            remove_all: true,
        }
    }

    pub fn discard_hand() -> Self {
        Self::DiscardHand
    }

    pub fn tap() -> Self {
        Self::Tap
    }

    pub fn untap() -> Self {
        Self::Untap
    }

    pub fn life(amount: impl Into<Value>) -> Self {
        Self::Life(amount.into())
    }

    pub fn energy(amount: impl Into<Value>) -> Self {
        Self::Energy(amount.into())
    }

    pub fn mill(count: impl Into<Value>) -> Self {
        Self::Mill(count.into())
    }

    pub fn exile_self() -> Self {
        Self::ExileSelf
    }

    pub fn exile_from_hand(count: u32, color_filter: Option<ColorSet>) -> Self {
        Self::ExileFromHand {
            count,
            color_filter,
        }
    }

    pub fn exile_from_graveyard(count: u32, card_types: Vec<CardType>) -> Self {
        Self::ExileFromGraveyard { count, card_types }
    }

    pub fn return_self_to_hand() -> Self {
        Self::ReturnSelfToHand
    }

    pub fn try_map_effect<E2, Error>(
        self,
        mut map_effect: impl FnMut(E) -> Result<E2, Error>,
    ) -> Result<Cost<E2>, Error> {
        Ok(match self {
            Self::Mana(mana) => Cost::Mana(mana),
            Self::DynamicMana(dynamic_mana) => Cost::DynamicMana(dynamic_mana),
            Self::Tap => Cost::Tap,
            Self::Untap => Cost::Untap,
            Self::DiscardSource => Cost::DiscardSource,
            Self::SacrificeSelf => Cost::SacrificeSelf,
            Self::Sacrifice(filter) => Cost::Sacrifice(filter),
            Self::Discard { count, card_types } => Cost::Discard { count, card_types },
            Self::DiscardHand => Cost::DiscardHand,
            Self::RemoveCounters {
                counter_type,
                count,
            } => Cost::RemoveCounters {
                counter_type,
                count,
            },
            Self::AddCounters {
                counter_type,
                count,
            } => Cost::AddCounters {
                counter_type,
                count,
            },
            Self::RemoveAnyCountersFromSource {
                counter_type,
                display_x,
                remove_all,
            } => Cost::RemoveAnyCountersFromSource {
                counter_type,
                display_x,
                remove_all,
            },
            Self::Energy(amount) => Cost::Energy(amount),
            Self::Mill(count) => Cost::Mill(count),
            Self::Life(amount) => Cost::Life(amount),
            Self::ExileSelf => Cost::ExileSelf,
            Self::ExileFromHand {
                count,
                color_filter,
            } => Cost::ExileFromHand {
                count,
                color_filter,
            },
            Self::ExileFromGraveyard { count, card_types } => {
                Cost::ExileFromGraveyard { count, card_types }
            }
            Self::ReturnSelfToHand => Cost::ReturnSelfToHand,
            Self::Effect(effect) => Cost::Effect(map_effect(effect)?),
        })
    }
}

impl<E> CostComponent for Cost<E>
where
    E: Clone + std::fmt::Debug + PartialEq,
{
    fn mana(mana_cost: ManaCost) -> Self {
        Self::Mana(mana_cost)
    }

    fn display(&self) -> String {
        match self {
            Self::Mana(cost) => cost.to_oracle(),
            Self::DynamicMana(cost) => cost.display(),
            Self::Tap => "{T}".to_string(),
            Self::Untap => "{Q}".to_string(),
            Self::DiscardSource => "discard this card".to_string(),
            Self::SacrificeSelf => "sacrifice this permanent".to_string(),
            Self::Sacrifice(_) => "sacrifice a permanent".to_string(),
            Self::Discard { count, .. } => format!("discard {count}"),
            Self::DiscardHand => "discard your hand".to_string(),
            Self::RemoveCounters {
                counter_type,
                count,
            } => format!("remove {count} {counter_type:?} counters"),
            Self::AddCounters {
                counter_type,
                count,
            } => format!("put {count} {counter_type:?} counters"),
            Self::RemoveAnyCountersFromSource {
                counter_type,
                display_x,
                remove_all,
            } => format!("remove any counters {counter_type:?} {display_x} {remove_all}"),
            Self::Energy(amount) => format!("pay {amount:?} energy"),
            Self::Mill(count) => format!("mill {count:?}"),
            Self::Life(amount) => format!("pay {amount:?} life"),
            Self::ExileSelf => "exile this card".to_string(),
            Self::ExileFromHand { count, .. } => format!("exile {count} card(s) from hand"),
            Self::ExileFromGraveyard { count, card_types } => {
                let type_text = describe_card_type_cost_list(card_types);
                let card_word = if *count == 1 { "card" } else { "cards" };
                format!("exile {count} {type_text}{card_word} from your graveyard")
            }
            Self::ReturnSelfToHand => "return this permanent to its owner's hand".to_string(),
            Self::Effect(_) => "effect".to_string(),
        }
    }

    fn sacrifice_filter(&self) -> Option<&ObjectFilter> {
        match self {
            Self::Sacrifice(filter) => Some(filter),
            _ => None,
        }
    }

    fn is_mana_cost(&self) -> bool {
        matches!(self, Self::Mana(_) | Self::DynamicMana(_))
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        match self {
            Self::Mana(cost) => Some(cost),
            _ => None,
        }
    }

    fn dynamic_mana_cost_ref(&self) -> Option<&DynamicManaCost> {
        match self {
            Self::DynamicMana(cost) => Some(cost),
            _ => None,
        }
    }

    fn exile_from_graveyard_details(&self) -> Option<(u32, &[CardType])> {
        match self {
            Self::ExileFromGraveyard { count, card_types } => Some((*count, card_types)),
            _ => None,
        }
    }

    fn is_loyalty_activation_cost(&self) -> bool {
        matches!(
            self,
            Self::RemoveCounters {
                counter_type: CounterType::Loyalty,
                ..
            } | Self::AddCounters {
                counter_type: CounterType::Loyalty,
                ..
            } | Self::RemoveAnyCountersFromSource {
                counter_type: Some(CounterType::Loyalty),
                ..
            }
        )
    }
}

impl<E> CoreCostComponent for Cost<E>
where
    E: Clone + std::fmt::Debug + PartialEq,
{
    fn tap_cost() -> Self {
        Self::Tap
    }
}

fn describe_card_type_cost_list(card_types: &[CardType]) -> String {
    match card_types {
        [] => String::new(),
        [one] => format!("{} ", one.to_string().to_ascii_lowercase()),
        [left, right] => format!(
            "{} and/or {} ",
            left.to_string().to_ascii_lowercase(),
            right.to_string().to_ascii_lowercase()
        ),
        _ => {
            let names = card_types
                .iter()
                .map(|card_type| card_type.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>();
            format!("{} ", names.join(", "))
        }
    }
}

pub trait CostComponent: Clone + std::fmt::Debug + PartialEq {
    fn mana(mana_cost: ManaCost) -> Self;

    fn display(&self) -> String;

    fn sacrifice_filter(&self) -> Option<&ObjectFilter> {
        None
    }

    fn is_mana_cost(&self) -> bool {
        false
    }

    fn requires_tap(&self) -> bool {
        false
    }

    fn life_amount(&self) -> Option<u32> {
        None
    }

    fn is_sacrifice_self(&self) -> bool {
        false
    }

    fn is_loyalty_activation_cost(&self) -> bool {
        false
    }

    fn exile_from_hand_details(&self) -> Option<(u32, Option<ColorSet>)> {
        None
    }

    fn exile_from_graveyard_details(&self) -> Option<(u32, &[CardType])> {
        None
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        None
    }

    fn dynamic_mana_cost_ref(&self) -> Option<&DynamicManaCost> {
        None
    }
}

pub trait CoreCostComponent: CostComponent {
    fn tap_cost() -> Self;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum TotalCostKind<C> {
    All(Vec<C>),
    OneOf(Vec<TotalCost<C>>),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct TotalCost<C> {
    kind: TotalCostKind<C>,
}

impl<C> TotalCost<C> {
    pub fn free() -> Self {
        Self::from_costs(vec![])
    }

    pub fn from_cost(cost: C) -> Self {
        Self::from_costs(vec![cost])
    }

    pub fn from_costs(costs: Vec<C>) -> Self {
        Self {
            kind: TotalCostKind::All(costs),
        }
    }

    pub fn one_of(branches: Vec<TotalCost<C>>) -> Self {
        Self {
            kind: TotalCostKind::OneOf(branches),
        }
    }

    pub fn kind(&self) -> &TotalCostKind<C> {
        &self.kind
    }

    pub fn as_all(&self) -> Option<&[C]> {
        match &self.kind {
            TotalCostKind::All(costs) => Some(costs),
            TotalCostKind::OneOf(_) => None,
        }
    }

    pub fn as_one_of(&self) -> Option<&[TotalCost<C>]> {
        match &self.kind {
            TotalCostKind::All(_) => None,
            TotalCostKind::OneOf(branches) => Some(branches),
        }
    }

    pub fn costs(&self) -> &[C] {
        self.as_all()
            .expect("TotalCost::costs called for an alternative cost")
    }

    pub fn try_map<C2, Error>(
        self,
        mut map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<TotalCost<C2>, Error> {
        self.try_map_with(&mut map_cost)
    }

    fn try_map_with<C2, Error>(
        self,
        map_cost: &mut impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<TotalCost<C2>, Error> {
        match self.kind {
            TotalCostKind::All(all) => {
                let mut costs = Vec::with_capacity(all.len());
                for cost in all {
                    costs.push(map_cost(cost)?);
                }
                Ok(TotalCost::from_costs(costs))
            }
            TotalCostKind::OneOf(branches) => {
                let mut mapped = Vec::with_capacity(branches.len());
                for branch in branches {
                    mapped.push(branch.try_map_with(map_cost)?);
                }
                Ok(TotalCost::one_of(mapped))
            }
        }
    }

    pub fn is_free(&self) -> bool {
        match &self.kind {
            TotalCostKind::All(costs) => costs.is_empty(),
            TotalCostKind::OneOf(branches) => branches.iter().any(Self::is_free),
        }
    }
}

impl<C> Default for TotalCost<C> {
    fn default() -> Self {
        Self::free()
    }
}

impl<C: CostComponent> TotalCost<C> {
    pub fn mana(mana_cost: ManaCost) -> Self {
        Self::from_cost(C::mana(mana_cost))
    }

    pub fn non_mana_costs(&self) -> impl Iterator<Item = &C> {
        self.costs().iter().filter(|cost| !cost.is_mana_cost())
    }

    pub fn has_non_mana_costs(&self) -> bool {
        match &self.kind {
            TotalCostKind::All(costs) => costs.iter().any(|cost| !cost.is_mana_cost()),
            TotalCostKind::OneOf(branches) => branches.iter().any(Self::has_non_mana_costs),
        }
    }

    pub fn has_loyalty_activation_cost(&self) -> bool {
        match &self.kind {
            TotalCostKind::All(costs) => {
                costs.iter().any(CostComponent::is_loyalty_activation_cost)
            }
            TotalCostKind::OneOf(branches) => {
                branches.iter().any(Self::has_loyalty_activation_cost)
            }
        }
    }

    pub fn display(&self) -> String {
        match &self.kind {
            TotalCostKind::All(costs) => {
                if costs.is_empty() {
                    return "Free".to_string();
                }
                let parts: Vec<String> = costs
                    .iter()
                    .map(|c| c.display())
                    .filter(|part| !part.trim().is_empty())
                    .collect();
                if parts.is_empty() {
                    return "Free".to_string();
                }
                parts.join(", ")
            }
            TotalCostKind::OneOf(branches) => {
                let parts: Vec<String> = branches.iter().map(Self::display).collect();
                if parts.is_empty() {
                    "Free".to_string()
                } else {
                    parts.join(" or ")
                }
            }
        }
    }

    pub fn mana_cost(&self) -> Option<&ManaCost> {
        match &self.kind {
            TotalCostKind::All(costs) => costs.iter().find_map(|c| c.mana_cost_ref()),
            TotalCostKind::OneOf(_) => None,
        }
    }

    pub fn dynamic_mana_cost(&self) -> Option<&DynamicManaCost> {
        match &self.kind {
            TotalCostKind::All(costs) => costs.iter().find_map(|c| c.dynamic_mana_cost_ref()),
            TotalCostKind::OneOf(_) => None,
        }
    }
}

impl<C: CostComponent> From<ManaCost> for TotalCost<C> {
    fn from(mana_cost: ManaCost) -> Self {
        Self::mana(mana_cost)
    }
}

impl<C> From<C> for TotalCost<C> {
    fn from(cost: C) -> Self {
        Self::from_cost(cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ManaSymbol;

    #[test]
    fn total_cost_all_display_and_free() {
        let cost: TotalCost<Cost<()>> = TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::life(Value::Fixed(3)),
        ]);

        assert!(!cost.is_free());
        assert_eq!(cost.display(), "{2}, pay Fixed(3) life");
        assert!(cost.has_non_mana_costs());
        assert_eq!(
            cost.mana_cost().map(ManaCost::to_oracle),
            Some("{2}".into())
        );
    }

    #[test]
    fn total_cost_one_of_display_and_introspection() {
        let cost: TotalCost<Cost<()>> = TotalCost::one_of(vec![
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Black])),
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)])),
        ]);

        assert!(!cost.is_free());
        assert!(cost.as_all().is_none());
        assert_eq!(cost.as_one_of().map(<[_]>::len), Some(2));
        assert_eq!(cost.display(), "{B} or {3}");
        assert!(cost.mana_cost().is_none());
    }

    #[test]
    fn total_cost_try_map_recurses_through_alternatives() {
        let cost: TotalCost<Cost<&'static str>> = TotalCost::one_of(vec![
            TotalCost::from_cost(Cost::effect("a")),
            TotalCost::from_cost(Cost::effect("b")),
        ]);

        let mapped = cost
            .try_map(|component| component.try_map_effect(|effect| Ok::<_, ()>(effect.len())))
            .unwrap();

        assert_eq!(
            mapped,
            TotalCost::one_of(vec![
                TotalCost::from_cost(Cost::effect(1usize)),
                TotalCost::from_cost(Cost::effect(1usize)),
            ])
        );
    }

    #[test]
    fn dynamic_mana_is_a_cost_component() {
        let dynamic =
            DynamicManaCost::from_x(ManaCost::from_symbols(vec![ManaSymbol::X]), Value::Fixed(4));
        let cost: Cost<()> = Cost::dynamic_mana(dynamic.clone());
        assert!(cost.is_mana_cost());
        assert_eq!(cost.dynamic_mana_cost_ref(), Some(&dynamic));
    }

    #[test]
    fn behold_optional_cost_refs_preserve_subtype_discriminators() {
        let dragon = OptionalCostRef::with_discriminator(OptionalCostKind::Behold, "Dragon");
        let generic = OptionalCostRef::new(OptionalCostKind::Behold);
        let goblin = OptionalCostRef::with_discriminator(OptionalCostKind::Behold, "Goblin");

        assert_eq!(dragon.display_label(), "Behold Dragon");
        assert_eq!(OptionalCostRef::from_label("Behold Dragon"), dragon);
        assert!(dragon.matches_query(&generic));
        assert!(dragon.matches_query(&dragon));
        assert!(!dragon.matches_query(&goblin));
    }
}

/// Which authored reference to a verified alternative casting method should
/// be used when describing a later "that cost was paid" condition.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub enum AlternativeCostReferenceSurface {
    ManaCost,
    NamedCost,
    ThatCost,
}

/// A condition reference correlated with an actual alternative casting
/// method on the same card.
///
/// The mana string is canonicalized from a typed `ManaCost` at construction;
/// callers cannot smuggle arbitrary oracle text into this executable key.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub struct AlternativeCostReference {
    method_name: String,
    mana_cost: Option<String>,
    surface: AlternativeCostReferenceSurface,
}

impl AlternativeCostReference {
    fn new(
        method_name: impl Into<String>,
        mana_cost: Option<&ManaCost>,
        surface: AlternativeCostReferenceSurface,
    ) -> Self {
        Self {
            method_name: method_name.into(),
            mana_cost: mana_cost.map(ManaCost::to_oracle),
            surface,
        }
    }

    pub fn by_mana_cost(method_name: impl Into<String>, mana_cost: &ManaCost) -> Self {
        Self::new(
            method_name,
            Some(mana_cost),
            AlternativeCostReferenceSurface::ManaCost,
        )
    }

    pub fn by_name(method_name: impl Into<String>, mana_cost: Option<&ManaCost>) -> Self {
        Self::new(
            method_name,
            mana_cost,
            AlternativeCostReferenceSurface::NamedCost,
        )
    }

    pub fn as_that_cost(method_name: impl Into<String>, mana_cost: Option<&ManaCost>) -> Self {
        Self::new(
            method_name,
            mana_cost,
            AlternativeCostReferenceSurface::ThatCost,
        )
    }

    /// Canonical storage key recorded when the corresponding casting method
    /// is selected. Query matching intentionally ignores this storage surface.
    pub fn paid_marker(method_name: impl Into<String>, mana_cost: Option<&ManaCost>) -> Self {
        Self::new(
            method_name,
            mana_cost,
            AlternativeCostReferenceSurface::NamedCost,
        )
    }

    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    pub fn mana_cost_text(&self) -> Option<&str> {
        self.mana_cost.as_deref()
    }

    pub fn surface(&self) -> AlternativeCostReferenceSurface {
        self.surface
    }

    fn matches_query(&self, query: &Self) -> bool {
        match query.surface {
            AlternativeCostReferenceSurface::ManaCost => {
                query.mana_cost.is_some() && self.mana_cost == query.mana_cost
            }
            AlternativeCostReferenceSurface::NamedCost => {
                self.method_name.eq_ignore_ascii_case(&query.method_name)
            }
            AlternativeCostReferenceSurface::ThatCost => {
                self.method_name.eq_ignore_ascii_case(&query.method_name)
                    && self.mana_cost == query.mana_cost
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub enum OptionalCostKind {
    Kicker,
    Multikicker,
    Replicate,
    Buyback,
    Entwine,
    Squad,
    Offspring,
    Bargain,
    Conspire,
    Gift,
    Behold,
    Waterbend,
    CastDuringYourMainPhase,
    Escape,
    Blitz,
    Evoke,
    Madness,
    Suspend,
    CompleatedLifePaid,
    GrantedConspire,
    Tribute,
    Surge,
    Spectacle,
    Additional,
    /// A later condition referring to a verified alternative casting method.
    AlternativeCast(AlternativeCostReference),
    CustomUnsupported(String),
}

impl OptionalCostKind {
    pub fn from_label(label: &str) -> Self {
        let trimmed = label.trim();
        let lower = trimmed.to_ascii_lowercase();
        match lower.as_str() {
            "kicker" => Self::Kicker,
            "multikicker" => Self::Multikicker,
            "replicate" => Self::Replicate,
            "buyback" => Self::Buyback,
            "entwine" => Self::Entwine,
            "squad" => Self::Squad,
            "offspring" => Self::Offspring,
            "bargain" => Self::Bargain,
            "conspire" => Self::Conspire,
            "gift" => Self::Gift,
            "behold" => Self::Behold,
            "waterbend" => Self::Waterbend,
            "castduringyourmainphase" => Self::CastDuringYourMainPhase,
            "escape" => Self::Escape,
            "blitz" => Self::Blitz,
            "evoke" => Self::Evoke,
            "madness" => Self::Madness,
            "suspend" => Self::Suspend,
            "compleatedlifepaid" => Self::CompleatedLifePaid,
            "granted conspire" => Self::GrantedConspire,
            "tribute" => Self::Tribute,
            "surge" => Self::Surge,
            "spectacle" => Self::Spectacle,
            "additional" | "additional cost" => Self::Additional,
            _ if lower.starts_with("kicker ") => Self::Kicker,
            _ if lower.starts_with("gift ") => Self::Gift,
            _ if lower.starts_with("conspire") => Self::Conspire,
            _ if lower.starts_with("behold ") => Self::Behold,
            _ if lower.starts_with("waterbend ") => Self::Waterbend,
            _ if lower.starts_with("as an additional cost to cast this spell, you may behold ") => {
                Self::Behold
            }
            _ if lower.starts_with("as an additional cost to cast this spell, you may ") => {
                Self::Additional
            }
            _ => Self::CustomUnsupported(trimmed.to_string()),
        }
    }

    pub fn canonical_label(&self) -> &str {
        match self {
            Self::Kicker => "Kicker",
            Self::Multikicker => "Multikicker",
            Self::Replicate => "Replicate",
            Self::Buyback => "Buyback",
            Self::Entwine => "Entwine",
            Self::Squad => "Squad",
            Self::Offspring => "Offspring",
            Self::Bargain => "Bargain",
            Self::Conspire => "Conspire",
            Self::Gift => "Gift",
            Self::Behold => "Behold",
            Self::Waterbend => "Waterbend",
            Self::CastDuringYourMainPhase => "CastDuringYourMainPhase",
            Self::Escape => "Escape",
            Self::Blitz => "Blitz",
            Self::Evoke => "Evoke",
            Self::Madness => "Madness",
            Self::Suspend => "Suspend",
            Self::CompleatedLifePaid => "CompleatedLifePaid",
            Self::GrantedConspire => "Granted Conspire",
            Self::Tribute => "Tribute",
            Self::Surge => "Surge",
            Self::Spectacle => "Spectacle",
            Self::Additional => "Additional",
            Self::AlternativeCast(reference) => reference.method_name(),
            Self::CustomUnsupported(label) => label.as_str(),
        }
    }

    pub fn is_query_for(&self, stored: &Self) -> bool {
        self == stored
            || matches!((self, stored),
                (Self::AlternativeCast(query), Self::AlternativeCast(stored))
                    if stored.matches_query(query)
            )
            || matches!(
                (self, stored),
                (Self::Kicker, Self::Kicker)
                    | (Self::Gift, Self::Gift)
                    | (Self::Conspire, Self::Conspire)
                    | (Self::Behold, Self::Behold)
                    | (
                        Self::Additional,
                        Self::Additional | Self::Behold | Self::Waterbend
                    )
            )
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub struct OptionalCostRef {
    pub kind: OptionalCostKind,
    pub discriminator: Option<String>,
}

impl OptionalCostRef {
    pub fn new(kind: OptionalCostKind) -> Self {
        Self {
            kind,
            discriminator: None,
        }
    }

    pub fn with_discriminator(kind: OptionalCostKind, discriminator: impl Into<String>) -> Self {
        let discriminator = discriminator.into();
        Self {
            kind,
            discriminator: (!discriminator.trim().is_empty()).then_some(discriminator),
        }
    }

    pub fn from_label(label: &str) -> Self {
        let trimmed = label.trim();
        let kind = OptionalCostKind::from_label(trimmed);
        let discriminator = match &kind {
            OptionalCostKind::Kicker => trimmed
                .strip_prefix("Kicker ")
                .or_else(|| trimmed.strip_prefix("kicker "))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            OptionalCostKind::Gift => trimmed
                .strip_prefix("Gift ")
                .or_else(|| trimmed.strip_prefix("gift "))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            OptionalCostKind::Waterbend => trimmed
                .strip_prefix("Waterbend ")
                .or_else(|| trimmed.strip_prefix("waterbend "))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            OptionalCostKind::Behold => trimmed
                .strip_prefix("Behold ")
                .or_else(|| trimmed.strip_prefix("behold "))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            OptionalCostKind::CustomUnsupported(_) => Some(trimmed.to_string()),
            _ => None,
        };
        Self {
            kind,
            discriminator,
        }
    }

    pub fn matches_query(&self, query: &Self) -> bool {
        if !query.kind.is_query_for(&self.kind) {
            return false;
        }
        match query.discriminator.as_deref() {
            Some(query_discriminator) => {
                self.discriminator.as_deref() == Some(query_discriminator)
                    || matches!(query.kind, OptionalCostKind::CustomUnsupported(_))
            }
            None => true,
        }
    }

    pub fn display_label(&self) -> String {
        if let OptionalCostKind::AlternativeCast(reference) = &self.kind {
            return match reference.surface() {
                AlternativeCostReferenceSurface::ManaCost => reference
                    .mana_cost_text()
                    .unwrap_or("alternative")
                    .to_string(),
                AlternativeCostReferenceSurface::NamedCost => reference.method_name().to_string(),
                AlternativeCostReferenceSurface::ThatCost => "That".to_string(),
            };
        }
        match self.discriminator.as_deref() {
            Some(discriminator)
                if matches!(
                    self.kind,
                    OptionalCostKind::Kicker
                        | OptionalCostKind::Gift
                        | OptionalCostKind::Behold
                        | OptionalCostKind::Waterbend
                ) =>
            {
                format!("{} {discriminator}", self.kind.canonical_label())
            }
            _ => self.kind.canonical_label().to_string(),
        }
    }

    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        self.display_label().eq_ignore_ascii_case(other)
            || self.kind.canonical_label().eq_ignore_ascii_case(other)
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.display_label().starts_with(prefix)
    }

    pub fn to_ascii_lowercase(&self) -> String {
        self.display_label().to_ascii_lowercase()
    }

    pub fn strip_prefix(&self, prefix: &str) -> Option<&str> {
        match (&self.kind, prefix) {
            (OptionalCostKind::Kicker, "Kicker ") => self.discriminator.as_deref(),
            (OptionalCostKind::Gift, "Gift ") => self.discriminator.as_deref(),
            (OptionalCostKind::Behold, "Behold ") => self.discriminator.as_deref(),
            (OptionalCostKind::Waterbend, "Waterbend ") => self.discriminator.as_deref(),
            (OptionalCostKind::CustomUnsupported(label), _) => label.strip_prefix(prefix),
            _ => None,
        }
    }
}

impl From<&str> for OptionalCostRef {
    fn from(label: &str) -> Self {
        Self::from_label(label)
    }
}

impl From<String> for OptionalCostRef {
    fn from(label: String) -> Self {
        Self::from_label(&label)
    }
}

impl From<&String> for OptionalCostRef {
    fn from(label: &String) -> Self {
        Self::from_label(label)
    }
}

impl From<&OptionalCostRef> for OptionalCostRef {
    fn from(value: &OptionalCostRef) -> Self {
        value.clone()
    }
}

impl From<OptionalCostKind> for OptionalCostRef {
    fn from(kind: OptionalCostKind) -> Self {
        Self::new(kind)
    }
}

impl std::fmt::Display for OptionalCostRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display_label())
    }
}

impl PartialEq<&str> for OptionalCostRef {
    fn eq(&self, other: &&str) -> bool {
        self.display_label() == *other || self.kind.canonical_label() == *other
    }
}

impl PartialEq<str> for OptionalCostRef {
    fn eq(&self, other: &str) -> bool {
        self.display_label() == other || self.kind.canonical_label() == other
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct OptionalCost<C> {
    pub kind: OptionalCostKind,
    pub reference: OptionalCostRef,
    pub source_label: String,
    pub cost: TotalCost<C>,
    pub repeatable: bool,
    pub returns_to_hand: bool,
}

impl<C> OptionalCost<C> {
    fn typed(kind: OptionalCostKind, source_label: impl Into<String>, cost: TotalCost<C>) -> Self {
        let source_label = source_label.into();
        let reference = OptionalCostRef::from_label(&source_label);
        Self {
            kind,
            reference,
            source_label,
            cost,
            repeatable: false,
            returns_to_hand: false,
        }
    }

    pub fn cost_ref(&self) -> OptionalCostRef {
        self.reference.clone()
    }

    pub fn display_label(&self) -> String {
        self.cost_ref().display_label()
    }

    pub fn kicker(cost: TotalCost<C>) -> Self {
        Self::typed(OptionalCostKind::Kicker, "Kicker", cost)
    }

    pub fn multikicker(cost: TotalCost<C>) -> Self {
        Self::typed(OptionalCostKind::Multikicker, "Multikicker", cost).repeatable()
    }

    pub fn replicate(cost: TotalCost<C>) -> Self {
        Self::typed(OptionalCostKind::Replicate, "Replicate", cost).repeatable()
    }

    pub fn buyback(cost: TotalCost<C>) -> Self {
        Self::typed(OptionalCostKind::Buyback, "Buyback", cost).returns_to_hand()
    }

    pub fn entwine(cost: TotalCost<C>) -> Self {
        Self::typed(OptionalCostKind::Entwine, "Entwine", cost)
    }

    pub fn squad(cost: TotalCost<C>) -> Self {
        Self::typed(OptionalCostKind::Squad, "Squad", cost).repeatable()
    }

    pub fn offspring(cost: TotalCost<C>) -> Self {
        Self::typed(OptionalCostKind::Offspring, "Offspring", cost)
    }

    pub fn custom(label: impl Into<String>, cost: TotalCost<C>) -> Self {
        let label = label.into();
        Self::typed(OptionalCostKind::from_label(&label), label, cost)
    }

    pub fn repeatable(mut self) -> Self {
        self.repeatable = true;
        self
    }

    pub fn returns_to_hand(mut self) -> Self {
        self.returns_to_hand = true;
        self
    }

    pub fn try_map<C2, Error>(
        self,
        map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<OptionalCost<C2>, Error> {
        Ok(OptionalCost {
            kind: self.kind,
            reference: self.reference,
            source_label: self.source_label,
            cost: self.cost.try_map(map_cost)?,
            repeatable: self.repeatable,
            returns_to_hand: self.returns_to_hand,
        })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
pub struct OptionalCostsPaid {
    pub costs: Vec<(OptionalCostRef, u32)>,
    /// Cast-proposal provenance: this spell was announced while its controller
    /// could normally cast a sorcery (their main phase, with an empty stack).
    /// This is a timing fact, not an optional cost.
    pub cast_at_sorcery_timing: bool,
}

impl OptionalCostsPaid {
    pub fn new(num_optional_costs: usize) -> Self {
        Self {
            costs: vec![(OptionalCostRef::from(""), 0); num_optional_costs],
            cast_at_sorcery_timing: false,
        }
    }

    pub fn from_costs<C>(costs: &[OptionalCost<C>]) -> Self {
        Self {
            costs: costs.iter().map(|c| (c.cost_ref(), 0)).collect(),
            cast_at_sorcery_timing: false,
        }
    }

    pub fn any_paid(&self) -> bool {
        self.costs.iter().any(|(_, n)| *n > 0)
    }

    pub fn was_paid(&self, index: usize) -> bool {
        self.costs.get(index).map(|(_, n)| *n > 0).unwrap_or(false)
    }

    pub fn was_paid_label(&self, label: impl Into<OptionalCostRef>) -> bool {
        let query = label.into();
        self.costs
            .iter()
            .any(|(stored, n)| stored.matches_query(&query) && *n > 0)
    }

    pub fn times_paid(&self, index: usize) -> u32 {
        self.costs.get(index).map(|(_, n)| *n).unwrap_or(0)
    }

    pub fn times_paid_label(&self, label: impl Into<OptionalCostRef>) -> u32 {
        let query = label.into();
        self.costs
            .iter()
            .filter(|(stored, _)| stored.matches_query(&query))
            .map(|(_, n)| *n)
            .sum()
    }

    pub fn pay(&mut self, index: usize) {
        if let Some((_, times)) = self.costs.get_mut(index) {
            *times += 1;
        }
    }

    pub fn pay_times(&mut self, index: usize, times: u32) {
        if let Some((_, t)) = self.costs.get_mut(index) {
            *t += times;
        }
    }

    pub fn pay_label(&mut self, label: impl Into<OptionalCostRef>) {
        let label = label.into();
        if let Some((_, times)) = self.costs.iter_mut().find(|(stored, _)| *stored == label) {
            *times += 1;
        }
    }

    pub fn mark_label_paid(&mut self, label: impl Into<OptionalCostRef>) {
        let label = label.into();
        if let Some((_, times)) = self.costs.iter_mut().find(|(stored, _)| *stored == label) {
            *times += 1;
        } else {
            self.costs.push((label, 1));
        }
    }

    pub fn mark_cast_at_sorcery_timing(&mut self) {
        self.cast_at_sorcery_timing = true;
    }

    pub fn was_cast_at_sorcery_timing(&self) -> bool {
        self.cast_at_sorcery_timing
    }

    pub fn was_kicked(&self) -> bool {
        self.was_paid_label("Kicker") || self.was_paid_label("Multikicker")
    }

    pub fn kick_count(&self) -> u32 {
        self.times_paid_label("Kicker") + self.times_paid_label("Multikicker")
    }

    pub fn was_bought_back(&self) -> bool {
        self.was_paid_label("Buyback")
    }

    pub fn was_entwined(&self) -> bool {
        self.was_paid_label("Entwine")
    }
}

#[cfg(test)]
mod alternative_cost_reference_tests {
    use super::*;
    use crate::ManaSymbol;

    fn mastery_cost() -> ManaCost {
        ManaCost::from_symbols(vec![ManaSymbol::Generic(2), ManaSymbol::Blue])
    }

    #[test]
    fn selected_alternative_cost_matches_typed_name_mana_and_that_references() {
        let cost = mastery_cost();
        let stored = OptionalCostRef::new(OptionalCostKind::AlternativeCast(
            AlternativeCostReference::paid_marker("Sneak", Some(&cost)),
        ));
        let mut paid = OptionalCostsPaid::default();
        paid.mark_label_paid(stored);

        for query in [
            AlternativeCostReference::by_name("Sneak", Some(&cost)),
            AlternativeCostReference::by_mana_cost("Parsed alternative cost", &cost),
            AlternativeCostReference::as_that_cost("Sneak", Some(&cost)),
        ] {
            assert!(
                paid.was_paid_label(OptionalCostRef::new(OptionalCostKind::AlternativeCast(
                    query
                )))
            );
        }
    }

    #[test]
    fn alternative_cost_reference_near_misses_do_not_cross_correlate() {
        let cost = mastery_cost();
        let other_cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(3), ManaSymbol::Blue]);
        let mut paid = OptionalCostsPaid::default();
        paid.mark_label_paid(OptionalCostRef::new(OptionalCostKind::AlternativeCast(
            AlternativeCostReference::paid_marker("Sneak", Some(&cost)),
        )));

        assert!(
            !paid.was_paid_label(OptionalCostRef::new(OptionalCostKind::AlternativeCast(
                AlternativeCostReference::by_name("Mastery", Some(&cost),)
            ),))
        );
        assert!(
            !paid.was_paid_label(OptionalCostRef::new(OptionalCostKind::AlternativeCast(
                AlternativeCostReference::by_mana_cost("Sneak", &other_cost,)
            ),))
        );
    }
}
