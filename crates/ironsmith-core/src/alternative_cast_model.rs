use crate::{CostComponent, ManaCost, PowerToughness, TotalCost, Zone};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TrapCondition {
    OpponentCastSpells { count: u32 },
    OpponentSearchedLibrary,
    OpponentCreatureEntered,
    CreatureDealtDamageToYou,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum AlternativeCastingMethod<E, C, Cond> {
    Dash {
        cost: ManaCost,
    },
    Blitz {
        total_cost: TotalCost<C>,
    },
    Warp {
        cost: ManaCost,
    },
    Plot {
        cost: ManaCost,
    },
    Suspend {
        cost: ManaCost,
        time: u32,
    },
    Disturb {
        cost: ManaCost,
    },
    Overload {
        cost: ManaCost,
        effects: Vec<E>,
    },
    /// CR 702.148: an alternative cost whose stack text omits every
    /// square-bracketed segment before modes and targets are announced.
    Cleave {
        cost: ManaCost,
        effects: Vec<E>,
    },
    Awaken {
        amount: u32,
        cost: ManaCost,
        effects: Vec<E>,
    },
    Flashback {
        total_cost: TotalCost<C>,
    },
    Harmonize {
        total_cost: TotalCost<C>,
    },
    Retrace {
        total_cost: TotalCost<C>,
    },
    JumpStart {
        additional_cost: TotalCost<C>,
    },
    Escape {
        cost: Option<ManaCost>,
        exile_count: u32,
        additional_cost: TotalCost<C>,
    },
    Madness {
        cost: ManaCost,
    },
    Miracle {
        cost: ManaCost,
    },
    FlashWithAdditionalCost {
        additional_cost: ManaCost,
        total_cost: TotalCost<C>,
    },
    Foretell {
        cost: ManaCost,
    },
    Composed {
        name: crate::InternedStr,
        total_cost: TotalCost<C>,
        condition: Option<Cond>,
        /// Typed copiable-value override supplied by the Prototype keyword.
        ///
        /// Keeping this alongside the alternative cost lets runtime casting
        /// apply Prototype without reparsing keyword display text.
        prototype_power_toughness: Option<PowerToughness>,
    },
    FromZone {
        name: crate::InternedStr,
        zone: Zone,
        total_cost: TotalCost<C>,
        condition: Option<Cond>,
        exiles_after_resolution: bool,
    },
    Trap {
        name: crate::InternedStr,
        cost: ManaCost,
        condition: TrapCondition,
    },
    Bestow {
        total_cost: TotalCost<C>,
    },
    /// CR 702.140a: cast this creature spell for an alternative cost as a
    /// mutating creature spell with its method-specific target requirement.
    Mutate {
        cost: ManaCost,
    },
}

impl<E, C, Cond> AlternativeCastingMethod<E, C, Cond>
where
    E: Clone,
    C: CostComponent,
    Cond: Clone,
{
    pub fn cast_from_zone(&self) -> Zone {
        match self {
            Self::Dash { .. } | Self::Blitz { .. } => Zone::Hand,
            Self::Warp { .. } => Zone::Hand,
            Self::Plot { .. } | Self::Suspend { .. } => Zone::Exile,
            Self::Flashback { .. }
            | Self::Harmonize { .. }
            | Self::Retrace { .. }
            | Self::JumpStart { .. }
            | Self::Escape { .. }
            | Self::Disturb { .. } => Zone::Graveyard,
            Self::Madness { .. } | Self::Foretell { .. } => Zone::Exile,
            Self::Miracle { .. }
            | Self::FlashWithAdditionalCost { .. }
            | Self::Overload { .. }
            | Self::Cleave { .. }
            | Self::Awaken { .. }
            | Self::Composed { .. }
            | Self::Trap { .. }
            | Self::Bestow { .. }
            | Self::Mutate { .. } => Zone::Hand,
            Self::FromZone { zone, .. } => *zone,
        }
    }

    pub fn exiles_after_resolution(&self) -> bool {
        match self {
            Self::FromZone {
                exiles_after_resolution,
                ..
            } => *exiles_after_resolution,
            _ => matches!(
                self,
                Self::Flashback { .. }
                    | Self::Harmonize { .. }
                    | Self::JumpStart { .. }
                    | Self::Escape { .. }
            ),
        }
    }

    pub fn mana_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Dash { cost } => Some(cost),
            Self::Blitz { total_cost } => total_cost.mana_cost(),
            Self::Warp { cost } => Some(cost),
            Self::Plot { cost } => Some(cost),
            Self::Suspend { cost, .. } => Some(cost),
            Self::Disturb { cost } => Some(cost),
            Self::Overload { cost, .. } => Some(cost),
            Self::Cleave { cost, .. } => Some(cost),
            Self::Awaken { cost, .. } => Some(cost),
            Self::Flashback { total_cost } => total_cost.mana_cost(),
            Self::Harmonize { total_cost } => total_cost.mana_cost(),
            Self::Retrace { total_cost } => total_cost.mana_cost(),
            Self::JumpStart { .. } => None,
            Self::Escape { cost, .. } => cost.as_ref(),
            Self::Madness { cost } => Some(cost),
            Self::Miracle { cost } => Some(cost),
            Self::FlashWithAdditionalCost { total_cost, .. } => total_cost.mana_cost(),
            Self::Foretell { cost } => Some(cost),
            Self::Trap { cost, .. } => Some(cost),
            Self::Composed { total_cost, .. } => total_cost.mana_cost(),
            Self::FromZone { total_cost, .. } => total_cost.mana_cost(),
            Self::Bestow { total_cost } => total_cost.mana_cost(),
            Self::Mutate { cost } => Some(cost),
        }
    }

    pub fn non_mana_costs(&self) -> Vec<C> {
        fn non_mana_components<C: CostComponent>(total_cost: &TotalCost<C>) -> Vec<C> {
            total_cost.non_mana_costs().cloned().collect()
        }

        match self {
            Self::Flashback { total_cost } => non_mana_components(total_cost),
            Self::Blitz { total_cost } => non_mana_components(total_cost),
            Self::Harmonize { total_cost } => non_mana_components(total_cost),
            Self::Retrace { total_cost } => non_mana_components(total_cost),
            Self::JumpStart { additional_cost }
            | Self::Escape {
                additional_cost, ..
            } => non_mana_components(additional_cost),
            Self::FlashWithAdditionalCost { total_cost, .. } => non_mana_components(total_cost),
            Self::Composed { total_cost, .. } => non_mana_components(total_cost),
            Self::FromZone { total_cost, .. } => non_mana_components(total_cost),
            Self::Bestow { total_cost } => non_mana_components(total_cost),
            _ => Vec::new(),
        }
    }

    pub fn total_cost(&self) -> Option<&TotalCost<C>> {
        match self {
            Self::Flashback { total_cost } => Some(total_cost),
            Self::Blitz { total_cost } => Some(total_cost),
            Self::Harmonize { total_cost } => Some(total_cost),
            Self::Retrace { total_cost } => Some(total_cost),
            Self::FlashWithAdditionalCost { total_cost, .. } => Some(total_cost),
            Self::Composed { total_cost, .. } => Some(total_cost),
            Self::FromZone { total_cost, .. } => Some(total_cost),
            Self::Bestow { total_cost } => Some(total_cost),
            _ => None,
        }
    }

    /// Additional cost components imposed by an alternative casting method
    /// while retaining the card's printed mana cost or a separately stored
    /// alternative mana cost.
    pub fn additional_cost(&self) -> Option<&TotalCost<C>> {
        match self {
            Self::JumpStart { additional_cost }
            | Self::Escape {
                additional_cost, ..
            } => Some(additional_cost),
            _ => None,
        }
    }

    pub fn cast_condition(&self) -> Option<&Cond> {
        match self {
            Self::Composed { condition, .. } | Self::FromZone { condition, .. } => {
                condition.as_ref()
            }
            _ => None,
        }
    }

    pub fn with_cast_condition(mut self, condition: Cond) -> Self {
        match &mut self {
            Self::Composed {
                condition: existing_condition,
                ..
            }
            | Self::FromZone {
                condition: existing_condition,
                ..
            } => {
                *existing_condition = Some(condition);
            }
            _ => {}
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
            Self::Blitz { .. } => "Blitz",
            Self::Warp { .. } => "Warp",
            Self::Plot { .. } => "Plot",
            Self::Suspend { .. } => "Suspend",
            Self::Disturb { .. } => "Disturb",
            Self::Overload { .. } => "Overload",
            Self::Cleave { .. } => "Cleave",
            Self::Awaken { .. } => "Awaken",
            Self::Flashback { .. } => "Flashback",
            Self::Harmonize { .. } => "Harmonize",
            Self::Retrace { .. } => "Retrace",
            Self::JumpStart { .. } => "Jump-start",
            Self::Escape { .. } => "Escape",
            Self::Madness { .. } => "Madness",
            Self::Miracle { .. } => "Miracle",
            Self::FlashWithAdditionalCost { .. } => "Flash",
            Self::Foretell { .. } => "Foretell",
            Self::Composed { name, .. } => name.as_str(),
            Self::Trap { name, .. } => name.as_str(),
            Self::FromZone { name, .. } => name.as_str(),
            Self::Bestow { .. } => "Bestow",
            Self::Mutate { .. } => "Mutate",
        }
    }

    pub fn trap(name: &'static str, cost: ManaCost, condition: TrapCondition) -> Self {
        Self::Trap {
            name: name.into(),
            cost,
            condition,
        }
    }

    pub fn trap_condition(&self) -> Option<&TrapCondition> {
        match self {
            Self::Trap { condition, .. } => Some(condition),
            _ => None,
        }
    }

    pub fn alternative_cost(
        name: &'static str,
        mana_cost: Option<ManaCost>,
        additional_costs: Vec<C>,
    ) -> Self {
        Self::Composed {
            name: name.into(),
            total_cost: compose_total_cost(mana_cost, additional_costs),
            condition: None,
            prototype_power_toughness: None,
        }
    }

    pub fn alternative_cost_with_condition(
        name: &'static str,
        mana_cost: Option<ManaCost>,
        additional_costs: Vec<C>,
        condition: Cond,
    ) -> Self {
        Self::Composed {
            name: name.into(),
            total_cost: compose_total_cost(mana_cost, additional_costs),
            condition: Some(condition),
            prototype_power_toughness: None,
        }
    }

    pub fn prototype(cost: ManaCost, power_toughness: PowerToughness) -> Self {
        Self::Composed {
            name: "Prototype".into(),
            total_cost: compose_total_cost(Some(cost), Vec::new()),
            condition: None,
            prototype_power_toughness: Some(power_toughness),
        }
    }

    pub fn prototype_power_toughness(&self) -> Option<PowerToughness> {
        match self {
            Self::Composed {
                prototype_power_toughness,
                ..
            } => *prototype_power_toughness,
            _ => None,
        }
    }

    pub fn cast_from_zone_with_total_cost(
        name: &'static str,
        zone: Zone,
        total_cost: TotalCost<C>,
        condition: Option<Cond>,
        exiles_after_resolution: bool,
    ) -> Self {
        Self::FromZone {
            name: name.into(),
            zone,
            total_cost,
            condition,
            exiles_after_resolution,
        }
    }

    pub fn flash_with_additional_cost(additional_cost: ManaCost, total_cost: TotalCost<C>) -> Self {
        Self::FlashWithAdditionalCost {
            additional_cost,
            total_cost,
        }
    }

    pub fn requirements(&self) -> AlternativeCastRequirements {
        match self {
            Self::JumpStart { .. } => AlternativeCastRequirements {
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

    pub fn cleave_effects(&self) -> Option<&[E]> {
        match self {
            Self::Cleave { effects, .. } => Some(effects.as_slice()),
            _ => None,
        }
    }

    pub fn awaken_effects(&self) -> Option<&[E]> {
        match self {
            Self::Awaken { effects, .. } => Some(effects.as_slice()),
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

    pub fn is_mutate(&self) -> bool {
        matches!(self, Self::Mutate { .. })
    }

    pub fn mutate_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Mutate { cost } => Some(cost),
            _ => None,
        }
    }
}

impl<E, C, Cond> AlternativeCastingMethod<E, C, Cond> {
    pub fn try_map<E2, C2, Err>(
        self,
        map_effect: impl FnMut(E) -> Result<E2, Err>,
        mut map_cost: impl FnMut(C) -> Result<C2, Err>,
    ) -> Result<AlternativeCastingMethod<E2, C2, Cond>, Err>
    where
        C: Clone,
        C2: CostComponent,
    {
        self.try_map_total_costs(map_effect, |total_cost| total_cost.try_map(&mut map_cost))
    }

    /// Map effects and whole cost algebras.
    ///
    /// A single authored cost component can expand into several runtime
    /// components (choosing an object and then returning it). Mapping the
    /// total cost rather than each component keeps those siblings side by
    /// side instead of collapsing them into one composite component.
    pub fn try_map_total_costs<E2, C2, Err>(
        self,
        mut map_effect: impl FnMut(E) -> Result<E2, Err>,
        mut map_total_cost: impl FnMut(TotalCost<C>) -> Result<TotalCost<C2>, Err>,
    ) -> Result<AlternativeCastingMethod<E2, C2, Cond>, Err>
    where
        C: Clone,
        C2: CostComponent,
    {
        Ok(match self {
            Self::Dash { cost } => AlternativeCastingMethod::Dash { cost },
            Self::Blitz { total_cost } => AlternativeCastingMethod::Blitz {
                total_cost: map_total_cost(total_cost)?,
            },
            Self::Warp { cost } => AlternativeCastingMethod::Warp { cost },
            Self::Plot { cost } => AlternativeCastingMethod::Plot { cost },
            Self::Suspend { cost, time } => AlternativeCastingMethod::Suspend { cost, time },
            Self::Disturb { cost } => AlternativeCastingMethod::Disturb { cost },
            Self::Overload { cost, effects } => AlternativeCastingMethod::Overload {
                cost,
                effects: {
                    let mut mapped = Vec::with_capacity(effects.len());
                    for effect in effects {
                        mapped.push(map_effect(effect)?);
                    }
                    mapped
                },
            },
            Self::Cleave { cost, effects } => AlternativeCastingMethod::Cleave {
                cost,
                effects: {
                    let mut mapped = Vec::with_capacity(effects.len());
                    for effect in effects {
                        mapped.push(map_effect(effect)?);
                    }
                    mapped
                },
            },
            Self::Awaken {
                amount,
                cost,
                effects,
            } => AlternativeCastingMethod::Awaken {
                amount,
                cost,
                effects: {
                    let mut mapped = Vec::with_capacity(effects.len());
                    for effect in effects {
                        mapped.push(map_effect(effect)?);
                    }
                    mapped
                },
            },
            Self::Flashback { total_cost } => AlternativeCastingMethod::Flashback {
                total_cost: map_total_cost(total_cost)?,
            },
            Self::Harmonize { total_cost } => AlternativeCastingMethod::Harmonize {
                total_cost: map_total_cost(total_cost)?,
            },
            Self::Retrace { total_cost } => AlternativeCastingMethod::Retrace {
                total_cost: map_total_cost(total_cost)?,
            },
            Self::JumpStart { additional_cost } => AlternativeCastingMethod::JumpStart {
                additional_cost: map_total_cost(additional_cost)?,
            },
            Self::Escape {
                cost,
                exile_count,
                additional_cost,
            } => AlternativeCastingMethod::Escape {
                cost,
                exile_count,
                additional_cost: map_total_cost(additional_cost)?,
            },
            Self::Madness { cost } => AlternativeCastingMethod::Madness { cost },
            Self::Miracle { cost } => AlternativeCastingMethod::Miracle { cost },
            Self::FlashWithAdditionalCost {
                additional_cost,
                total_cost,
            } => AlternativeCastingMethod::FlashWithAdditionalCost {
                additional_cost,
                total_cost: map_total_cost(total_cost)?,
            },
            Self::Foretell { cost } => AlternativeCastingMethod::Foretell { cost },
            Self::Composed {
                name,
                total_cost,
                condition,
                prototype_power_toughness,
            } => AlternativeCastingMethod::Composed {
                name,
                total_cost: map_total_cost(total_cost)?,
                condition,
                prototype_power_toughness,
            },
            Self::FromZone {
                name,
                zone,
                total_cost,
                condition,
                exiles_after_resolution,
            } => AlternativeCastingMethod::FromZone {
                name,
                zone,
                total_cost: map_total_cost(total_cost)?,
                condition,
                exiles_after_resolution,
            },
            Self::Trap {
                name,
                cost,
                condition,
            } => AlternativeCastingMethod::Trap {
                name,
                cost,
                condition,
            },
            Self::Bestow { total_cost } => AlternativeCastingMethod::Bestow {
                total_cost: map_total_cost(total_cost)?,
            },
            Self::Mutate { cost } => AlternativeCastingMethod::Mutate { cost },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cost, ManaSymbol};

    fn generic_two() -> ManaCost {
        ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])
    }

    #[test]
    fn mutate_is_a_typed_hand_alternative_with_its_own_cost() {
        let cost = generic_two();
        let method: AlternativeCastingMethod<(), Cost<&'static str>, ()> =
            AlternativeCastingMethod::Mutate { cost: cost.clone() };

        assert_eq!(method.cast_from_zone(), Zone::Hand);
        assert_eq!(method.name(), "Mutate");
        assert_eq!(method.mana_cost(), Some(&cost));
        assert_eq!(method.mutate_cost(), Some(&cost));
        assert!(method.is_mutate());
        assert!(!method.exiles_after_resolution());
    }

    #[test]
    fn try_map_preserves_composed_cost_alternatives() {
        let method: AlternativeCastingMethod<(), Cost<&'static str>, ()> =
            AlternativeCastingMethod::Composed {
                name: "Choice cost".into(),
                total_cost: TotalCost::one_of(vec![
                    TotalCost::from_cost(Cost::effect("discard")),
                    TotalCost::mana(generic_two()),
                ]),
                condition: None,
                prototype_power_toughness: None,
            };

        let mapped: AlternativeCastingMethod<(), Cost<usize>, ()> = method
            .try_map(Ok::<_, ()>, |cost| {
                cost.try_map_effect(|effect| Ok::<_, ()>(effect.len()))
            })
            .expect("composed alternatives should map recursively");

        let AlternativeCastingMethod::Composed { total_cost, .. } = mapped else {
            panic!("expected mapped composed cost");
        };
        assert_eq!(
            total_cost,
            TotalCost::one_of(vec![
                TotalCost::from_cost(Cost::effect(7usize)),
                TotalCost::mana(generic_two()),
            ])
        );
    }
}
