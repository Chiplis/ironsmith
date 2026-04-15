use crate::{CostComponent, ManaCost, TotalCost, Zone};

#[derive(Debug, Clone, PartialEq)]
pub enum TrapCondition {
    OpponentCastSpells { count: u32 },
    OpponentSearchedLibrary,
    OpponentCreatureEntered,
    CreatureDealtDamageToYou,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AlternativeCastRequirements {
    pub exile_from_graveyard: u32,
    pub discard_from_hand: u32,
}

fn compose_total_cost<C: CostComponent>(
    mana_cost: Option<ManaCost>,
    additional_costs: Vec<C>,
) -> TotalCost<C> {
    let mut components = if let Some(mana_cost) = mana_cost {
        vec![C::mana(mana_cost)]
    } else {
        Vec::new()
    };
    components.extend(additional_costs);
    TotalCost::from_costs(components)
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlternativeCastingMethod<E, C, Cond> {
    Dash { cost: ManaCost },
    Warp { cost: ManaCost },
    Plot { cost: ManaCost },
    Suspend { cost: ManaCost, time: u32 },
    Disturb { cost: ManaCost },
    Overload { cost: ManaCost, effects: Vec<E> },
    Flashback { total_cost: TotalCost<C> },
    Harmonize { total_cost: TotalCost<C> },
    JumpStart,
    Escape { cost: Option<ManaCost>, exile_count: u32 },
    Madness { cost: ManaCost },
    Miracle { cost: ManaCost },
    Foretell { cost: ManaCost },
    Composed {
        name: &'static str,
        total_cost: TotalCost<C>,
        condition: Option<Cond>,
    },
    MindbreakTrap {
        name: &'static str,
        cost: ManaCost,
        condition: TrapCondition,
    },
    Bestow { total_cost: TotalCost<C> },
}

impl<E, C, Cond> AlternativeCastingMethod<E, C, Cond>
where
    E: Clone,
    C: CostComponent,
    Cond: Clone,
{
    pub fn cast_from_zone(&self) -> Zone {
        match self {
            Self::Dash { .. } => Zone::Hand,
            Self::Warp { .. } => Zone::Hand,
            Self::Plot { .. } | Self::Suspend { .. } => Zone::Exile,
            Self::Flashback { .. }
            | Self::Harmonize { .. }
            | Self::JumpStart
            | Self::Escape { .. }
            | Self::Disturb { .. } => Zone::Graveyard,
            Self::Madness { .. } | Self::Foretell { .. } => Zone::Exile,
            Self::Miracle { .. }
            | Self::Overload { .. }
            | Self::Composed { .. }
            | Self::MindbreakTrap { .. }
            | Self::Bestow { .. } => Zone::Hand,
        }
    }

    pub fn exiles_after_resolution(&self) -> bool {
        matches!(
            self,
            Self::Flashback { .. } | Self::Harmonize { .. } | Self::JumpStart | Self::Escape { .. }
        )
    }

    pub fn mana_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Dash { cost } => Some(cost),
            Self::Warp { cost } => Some(cost),
            Self::Plot { cost } => Some(cost),
            Self::Suspend { cost, .. } => Some(cost),
            Self::Disturb { cost } => Some(cost),
            Self::Overload { cost, .. } => Some(cost),
            Self::Flashback { total_cost } => total_cost.mana_cost(),
            Self::Harmonize { total_cost } => total_cost.mana_cost(),
            Self::JumpStart => None,
            Self::Escape { cost, .. } => cost.as_ref(),
            Self::Madness { cost } => Some(cost),
            Self::Miracle { cost } => Some(cost),
            Self::Foretell { cost } => Some(cost),
            Self::MindbreakTrap { cost, .. } => Some(cost),
            Self::Composed { total_cost, .. } => total_cost.mana_cost(),
            Self::Bestow { total_cost } => total_cost.mana_cost(),
        }
    }

    pub fn non_mana_costs(&self) -> Vec<C> {
        fn non_mana_components<C: CostComponent>(total_cost: &TotalCost<C>) -> Vec<C> {
            total_cost.non_mana_costs().cloned().collect()
        }

        match self {
            Self::Flashback { total_cost } => non_mana_components(total_cost),
            Self::Harmonize { total_cost } => non_mana_components(total_cost),
            Self::Composed { total_cost, .. } => non_mana_components(total_cost),
            Self::Bestow { total_cost } => non_mana_components(total_cost),
            _ => Vec::new(),
        }
    }

    pub fn total_cost(&self) -> Option<&TotalCost<C>> {
        match self {
            Self::Flashback { total_cost } => Some(total_cost),
            Self::Harmonize { total_cost } => Some(total_cost),
            Self::Composed { total_cost, .. } => Some(total_cost),
            Self::Bestow { total_cost } => Some(total_cost),
            _ => None,
        }
    }

    pub fn cast_condition(&self) -> Option<&Cond> {
        match self {
            Self::Composed { condition, .. } => condition.as_ref(),
            _ => None,
        }
    }

    pub fn with_cast_condition(mut self, condition: Cond) -> Self {
        if let Self::Composed {
            condition: existing_condition,
            ..
        } = &mut self
        {
            *existing_condition = Some(condition);
        }
        self
    }

    pub fn exile_from_hand_requirement(&self) -> Option<(u32, Option<crate::ColorSet>)> {
        if let Some(total_cost) = self.total_cost() {
            for component in total_cost.non_mana_costs() {
                if let Some(info) = component.exile_from_hand_details() {
                    return Some(info);
                }
            }
        }
        None
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Dash { .. } => "Dash",
            Self::Warp { .. } => "Warp",
            Self::Plot { .. } => "Plot",
            Self::Suspend { .. } => "Suspend",
            Self::Disturb { .. } => "Disturb",
            Self::Overload { .. } => "Overload",
            Self::Flashback { .. } => "Flashback",
            Self::Harmonize { .. } => "Harmonize",
            Self::JumpStart => "Jump-start",
            Self::Escape { .. } => "Escape",
            Self::Madness { .. } => "Madness",
            Self::Miracle { .. } => "Miracle",
            Self::Foretell { .. } => "Foretell",
            Self::Composed { name, .. } => name,
            Self::MindbreakTrap { name, .. } => name,
            Self::Bestow { .. } => "Bestow",
        }
    }

    pub fn trap(name: &'static str, cost: ManaCost, condition: TrapCondition) -> Self {
        Self::MindbreakTrap {
            name,
            cost,
            condition,
        }
    }

    pub fn trap_condition(&self) -> Option<&TrapCondition> {
        match self {
            Self::MindbreakTrap { condition, .. } => Some(condition),
            _ => None,
        }
    }

    pub fn alternative_cost(
        name: &'static str,
        mana_cost: Option<ManaCost>,
        additional_costs: Vec<C>,
    ) -> Self {
        Self::Composed {
            name,
            total_cost: compose_total_cost(mana_cost, additional_costs),
            condition: None,
        }
    }

    pub fn alternative_cost_with_condition(
        name: &'static str,
        mana_cost: Option<ManaCost>,
        additional_costs: Vec<C>,
        condition: Cond,
    ) -> Self {
        Self::Composed {
            name,
            total_cost: compose_total_cost(mana_cost, additional_costs),
            condition: Some(condition),
        }
    }

    pub fn requirements(&self) -> AlternativeCastRequirements {
        match self {
            Self::JumpStart => AlternativeCastRequirements {
                discard_from_hand: 1,
                ..Default::default()
            },
            Self::Escape { exile_count, .. } => AlternativeCastRequirements {
                exile_from_graveyard: *exile_count,
                ..Default::default()
            },
            _ => AlternativeCastRequirements::default(),
        }
    }

    pub fn is_composed_cost(&self) -> bool {
        matches!(self, Self::Composed { .. })
    }

    pub fn is_miracle(&self) -> bool {
        matches!(self, Self::Miracle { .. })
    }

    pub fn plot_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Plot { cost } => Some(cost),
            _ => None,
        }
    }

    pub fn suspend_spec(&self) -> Option<(u32, &ManaCost)> {
        match self {
            Self::Suspend { cost, time } => Some((*time, cost)),
            _ => None,
        }
    }

    pub fn disturb_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Disturb { cost } => Some(cost),
            _ => None,
        }
    }

    pub fn overload_effects(&self) -> Option<&[E]> {
        match self {
            Self::Overload { effects, .. } => Some(effects.as_slice()),
            _ => None,
        }
    }

    pub fn miracle_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Miracle { cost } => Some(cost),
            _ => None,
        }
    }

    pub fn is_madness(&self) -> bool {
        matches!(self, Self::Madness { .. })
    }

    pub fn madness_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Madness { cost } => Some(cost),
            _ => None,
        }
    }

    pub fn is_bestow(&self) -> bool {
        matches!(self, Self::Bestow { .. })
    }
}
