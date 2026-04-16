use crate::mana::ManaCost;

#[derive(Debug, Clone, PartialEq)]
pub enum Cost {
    Mana(ManaCost),
    Tap,
    Untap,
    DiscardSource,
    SacrificeSelf,
    Sacrifice(crate::target::ObjectFilter),
    Discard {
        count: u32,
        card_types: Vec<crate::types::CardType>,
    },
    DiscardHand,
    RemoveCounters {
        counter_type: crate::object::CounterType,
        count: u32,
    },
    AddCounters {
        counter_type: crate::object::CounterType,
        count: u32,
    },
    RemoveAnyCountersFromSource {
        counter_type: Option<crate::object::CounterType>,
        display_x: bool,
    },
    Energy(crate::effect::Value),
    Mill(crate::effect::Value),
    Life(crate::effect::Value),
    ExileSelf,
    ExileFromHand {
        count: u32,
        color_filter: Option<crate::color::ColorSet>,
    },
    ReturnSelfToHand,
    Effect(crate::effect::Effect),
    Placeholder(String),
}

impl ironsmith_core::CostComponent for Cost {
    fn mana(mana_cost: ManaCost) -> Self {
        Self::Mana(mana_cost)
    }

    fn display(&self) -> String {
        match self {
            Self::Mana(cost) => cost.to_oracle(),
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
            } => format!("remove any counters {counter_type:?} {display_x}"),
            Self::Energy(amount) => format!("pay {amount:?} energy"),
            Self::Mill(count) => format!("mill {count:?}"),
            Self::Life(amount) => format!("pay {amount:?} life"),
            Self::ExileSelf => "exile this card".to_string(),
            Self::ExileFromHand { count, .. } => format!("exile {count} card(s) from hand"),
            Self::ReturnSelfToHand => "return this permanent to its owner's hand".to_string(),
            Self::Effect(_) => "effect".to_string(),
            Self::Placeholder(text) => text.clone(),
        }
    }

    fn is_mana_cost(&self) -> bool {
        matches!(self, Self::Mana(_))
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        match self {
            Self::Mana(cost) => Some(cost),
            _ => None,
        }
    }
}

impl ironsmith_core::CoreCostComponent for Cost {
    fn tap_cost() -> Self {
        Self::Tap
    }
}

impl Cost {
    pub fn effect_ref(&self) -> Option<&crate::effect::Effect> {
        match self {
            Self::Effect(effect) => Some(effect),
            _ => None,
        }
    }

    pub fn validated_effect(effect: crate::effect::Effect) -> Self {
        Self::Effect(effect)
    }

    pub fn try_from_runtime_effect(effect: crate::effect::Effect) -> Result<Self, String> {
        Ok(Self::Effect(effect))
    }

    pub fn mana(mana_cost: ManaCost) -> Self {
        Self::Mana(mana_cost)
    }

    pub fn discard_source() -> Self {
        Self::DiscardSource
    }

    pub fn sacrifice_self() -> Self {
        Self::SacrificeSelf
    }

    pub fn sacrifice(filter: crate::target::ObjectFilter) -> Self {
        Self::Sacrifice(filter)
    }

    pub fn effect(effect: impl Into<crate::effect::Effect>) -> Self {
        Self::Effect(effect.into())
    }

    pub fn discard(count: u32, card_type: Option<crate::types::CardType>) -> Self {
        Self::Discard {
            count,
            card_types: card_type.into_iter().collect(),
        }
    }

    pub fn discard_types(count: u32, card_types: Vec<crate::types::CardType>) -> Self {
        Self::Discard { count, card_types }
    }

    pub fn remove_counters(counter_type: crate::object::CounterType, count: u32) -> Self {
        Self::RemoveCounters {
            counter_type,
            count,
        }
    }

    pub fn add_counters(counter_type: crate::object::CounterType, count: u32) -> Self {
        Self::AddCounters {
            counter_type,
            count,
        }
    }

    pub fn remove_any_counters_from_source(
        counter_type: Option<crate::object::CounterType>,
        display_x: bool,
    ) -> Self {
        Self::RemoveAnyCountersFromSource {
            counter_type,
            display_x,
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

    pub fn life(amount: impl Into<crate::effect::Value>) -> Self {
        Self::Life(amount.into())
    }

    pub fn energy(amount: impl Into<crate::effect::Value>) -> Self {
        Self::Energy(amount.into())
    }

    pub fn mill(count: impl Into<crate::effect::Value>) -> Self {
        Self::Mill(count.into())
    }

    pub fn exile_self() -> Self {
        Self::ExileSelf
    }

    pub fn exile_from_hand(count: u32, color_filter: Option<crate::color::ColorSet>) -> Self {
        Self::ExileFromHand {
            count,
            color_filter,
        }
    }

    pub fn return_self_to_hand() -> Self {
        Self::ReturnSelfToHand
    }
}
