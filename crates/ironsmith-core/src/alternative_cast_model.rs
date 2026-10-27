use crate::tag::TagKeyWalk;

use crate::{CostComponent, ManaCost, PowerToughness, TotalCost, Zone};

/// An intrinsic alternative cost which a separate ability may authorize
/// from another zone without authorizing the card's other casting costs.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum AlternativeCastKeyword {
    Bestow,
    Blitz,
    Warp,
    /// CR 702.190a sneak, a named alternative cost; appended for wire
    /// compatibility.
    Sneak,
    /// CR 702.140a mutate, cast for its mutate cost; appended for wire
    /// compatibility ("You may cast this card from your graveyard using its
    /// mutate ability", Brokkos).
    Mutate,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum TrapCondition {
    OpponentCastSpells { count: u32 },
    OpponentSearchedLibrary,
    OpponentCreatureEntered,
    CreatureDealtDamageToYou,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub struct AlternativeCastRequirements {
    pub exile_from_graveyard: u32,
    pub discard_from_hand: u32,
}

/// A paid special action from hand which prepares a later exile cast.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub enum ExilePreparation {
    FaceDown { price: ManaCost },
    FaceUpSorcery { price: ManaCost },
}

/// Instructions belonging to a casting permission, independent of its name.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct ZoneCastInstructions {
    /// Resolved against the source definition during lowering.
    pub add_printed_mana_cost: bool,
    /// Use the source's linked back face while casting.
    pub transformed: bool,
    /// Replace every move off the stack with exile, including a return to hand.
    pub exile_on_stack_exit: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub haste_on_battlefield: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub draw_on_death: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub return_at_next_end_step: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub sacrifice_at_next_end_step: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub sacrifice_on_entry: bool,
    /// A discard replacement exiles this card and grants a one-shot cast.
    #[cfg_attr(feature = "serde", serde(default))]
    pub cast_when_discarded_into_exile: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub tap_creature_reduces_generic: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub returns_attacker_enters_attacking: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub sacrifice_reduces_generic_by_mana_value: bool,
    /// Reveal as the first card drawn, then cast only through that linked trigger.
    #[cfg_attr(feature = "serde", serde(default))]
    pub first_draw_reveal_cast: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub exile_preparation: Option<ExilePreparation>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub aura_cast_with_unattached_fallback: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub merge_with_same_owner_nonhuman: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub timed_exile: Option<(crate::SuspendTime, ManaCost)>,
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

// Serialize the payload as a struct inside the externally tagged variant.
// erased-serde 0.4.9 dispatches StructVariant::skip_field to its Struct arm;
// this preserves the legacy omitted empty rider without that invalid dispatch.
#[cfg(feature = "serde")]
fn serialize_from_zone_payload<S, C, Cond>(
    name: &crate::InternedStr,
    zone: &Zone,
    total_cost: &TotalCost<C>,
    condition: &Option<Cond>,
    exiles_after_resolution: &bool,
    entry_counters: &Vec<(crate::CounterType, u32)>,
    instructions: &ZoneCastInstructions,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    C: serde::Serialize,
    Cond: serde::Serialize,
{
    #[derive(serde::Serialize)]
    struct Payload<'a, C, Cond> {
        name: &'a crate::InternedStr,
        zone: &'a Zone,
        total_cost: &'a TotalCost<C>,
        condition: &'a Option<Cond>,
        exiles_after_resolution: &'a bool,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        entry_counters: &'a Vec<(crate::CounterType, u32)>,
        instructions: &'a ZoneCastInstructions,
    }
    serde::Serialize::serialize(
        &Payload {
            name,
            zone,
            total_cost,
            condition,
            exiles_after_resolution,
            entry_counters,
            instructions,
        },
        serializer,
    )
}

/// Name of a printed free alternative cost that also casts the spell as
/// though it had flash: "you may cast this creature without paying its mana
/// cost and as though it had flash" (Qasali Ambusher). The timing permission
/// belongs to this casting method only (CR 601.3e).
pub const FREE_CAST_AS_THOUGH_FLASH_METHOD: &str = "Free cast as though it had flash";

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum AlternativeCastingMethod<E, C, Cond> {
    Dash {
        cost: ManaCost,
    },
    Blitz {
        total_cost: TotalCost<C>,
    },
    Warp {
        cost: ManaCost,
        #[cfg_attr(feature = "serde", serde(default))]
        descriptive: bool,
        #[cfg_attr(feature = "serde", serde(default = "TotalCost::free"))]
        additional_cost: TotalCost<C>,
    },
    Plot {
        cost: ManaCost,
    },
    Suspend {
        cost: ManaCost,
        time: crate::SuspendTime,
    },
    Disturb {
        cost: ManaCost,
    },
    Overload {
        cost: ManaCost,
        effects: Vec<E>,
        #[cfg_attr(feature = "serde", serde(default))]
        descriptive: bool,
        #[cfg_attr(feature = "serde", serde(default = "TotalCost::free"))]
        additional_cost: TotalCost<C>,
    },
    /// CR 702.148: an alternative cost whose stack text omits every
    /// square-bracketed segment before modes and targets are announced.
    Cleave {
        cost: ManaCost,
        effects: Vec<E>,
        #[cfg_attr(feature = "serde", serde(default))]
        descriptive: bool,
        #[cfg_attr(feature = "serde", serde(default = "TotalCost::free"))]
        additional_cost: TotalCost<C>,
    },
    Awaken {
        amount: u32,
        cost: ManaCost,
        effects: Vec<E>,
        #[cfg_attr(feature = "serde", serde(default))]
        descriptive: bool,
        #[cfg_attr(feature = "serde", serde(default = "TotalCost::free"))]
        additional_cost: TotalCost<C>,
    },
    Flashback {
        total_cost: TotalCost<C>,
        /// "If you cast this spell this way, X can't be 0" (Light Up the
        /// Night): the smallest X this method allows (CR 107.3, CR 601.2b).
        #[cfg_attr(feature = "serde", serde(default))]
        x_minimum: u32,
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
        total_cost: TotalCost<C>,
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
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "serialize_from_zone_payload")
    )]
    FromZone {
        name: crate::InternedStr,
        zone: Zone,
        total_cost: TotalCost<C>,
        condition: Option<Cond>,
        exiles_after_resolution: bool,
        /// Entry replacement instructions belonging only to this chosen cost.
        /// Spell copies retain the alternative-cost choice (CR 707.10).
        #[cfg_attr(feature = "serde", serde(default))]
        entry_counters: Vec<(crate::CounterType, u32)>,
        #[cfg_attr(feature = "serde", serde(default))]
        instructions: ZoneCastInstructions,
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
    /// Cast from the graveyard by paying mana and returning a creature as a cost.
    BeamMeUp {
        total_cost: TotalCost<C>,
    },
}

impl<E, C, Cond> AlternativeCastingMethod<E, C, Cond>
where
    E: Clone,
    C: CostComponent,
    Cond: Clone,
{
    /// Alternative casting instructions which use the linked back face.
    /// This casting method itself permits casting at instant timing.
    pub fn battlefield_instructions(&self) -> ZoneCastInstructions {
        match self {
            Self::Dash { .. } => ZoneCastInstructions { haste_on_battlefield: true, return_at_next_end_step: true, ..Default::default() },
            Self::Blitz { .. } => ZoneCastInstructions { haste_on_battlefield: true, draw_on_death: true, sacrifice_at_next_end_step: true, ..Default::default() },
            Self::FromZone { instructions, .. } => instructions.clone(),
            _ => ZoneCastInstructions::default(),
        }
    }

    pub fn sacrifice_reduces_generic_by_mana_value(&self) -> bool {
        matches!(self, Self::Composed { name, .. } if name.eq_ignore_ascii_case("Emerge"))
            || matches!(self, Self::FromZone { instructions, .. } if instructions.sacrifice_reduces_generic_by_mana_value)
    }

    pub fn returns_attacker_enters_attacking(&self) -> bool {
        matches!(self, Self::Composed { name, .. } if name.eq_ignore_ascii_case("Sneak"))
            || matches!(self, Self::FromZone { instructions, .. } if instructions.returns_attacker_enters_attacking)
    }

    pub fn grants_instant_timing(&self) -> bool {
        matches!(self, Self::Composed { name, .. }
            if name.eq_ignore_ascii_case(FREE_CAST_AS_THOUGH_FLASH_METHOD))
    }

    pub fn casts_transformed(&self) -> bool {
        matches!(self, Self::Disturb { .. })
            || matches!(self, Self::FromZone { instructions, .. } if instructions.transformed)
            || matches!(self, Self::Composed { name, .. } if name.eq_ignore_ascii_case("More than meets the eye"))
    }

    /// Methods whose rules text restricts them to casting the card from its
    /// owner's hand, so a command-zone commander cast can't use them even
    /// though they report `Zone::Hand` (warp, CR 702.185a).
    pub fn requires_cast_from_hand(&self) -> bool {
        matches!(self, Self::Warp { .. })
    }

    pub fn cast_from_zone(&self) -> Zone {
        match self {
            Self::Dash { .. } | Self::Blitz { .. } => Zone::Hand,
            Self::Warp { .. } => Zone::Hand,
            Self::Plot { .. } | Self::Suspend { .. } => Zone::Exile,
            Self::Flashback { .. }
            | Self::BeamMeUp { .. }
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

    pub fn keyword(&self) -> Option<AlternativeCastKeyword> {
        match self {
            Self::Bestow { .. } => Some(AlternativeCastKeyword::Bestow),
            Self::Blitz { .. } => Some(AlternativeCastKeyword::Blitz),
            Self::Warp { descriptive: false, .. } => Some(AlternativeCastKeyword::Warp),
            Self::Warp { descriptive: true, .. } => None,
            Self::Mutate { .. } => Some(AlternativeCastKeyword::Mutate),
            // Sneak is a named composed alternative cost; its timing and
            // resolution are keyed to the same method name.
            Self::Composed { .. } if self.name().eq_ignore_ascii_case("Sneak") => {
                Some(AlternativeCastKeyword::Sneak)
            }
            _ => None,
        }
    }

    pub fn exiles_after_resolution(&self) -> bool {
        match self {
            Self::FromZone {
                exiles_after_resolution, instructions,
                ..
            } => *exiles_after_resolution || instructions.exile_on_stack_exit,
            _ => matches!(
                self,
                Self::Flashback { .. } | Self::BeamMeUp { .. }
            | Self::Harmonize { .. } | Self::JumpStart { .. }
            ),
        }
    }

    pub fn mana_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Dash { cost } => Some(cost),
            Self::Blitz { total_cost } => total_cost.mana_cost(),
            Self::Warp { cost, .. } => Some(cost),
            Self::Plot { cost } => Some(cost),
            Self::Suspend { cost, .. } => Some(cost),
            Self::Disturb { cost } => Some(cost),
            Self::Overload { cost, .. } => Some(cost),
            Self::Cleave { cost, .. } => Some(cost),
            Self::Awaken { cost, .. } => Some(cost),
            Self::Flashback { total_cost, .. } => total_cost.mana_cost(),
            Self::Harmonize { total_cost } | Self::BeamMeUp { total_cost } => total_cost.mana_cost(),
            Self::Retrace { total_cost } => total_cost.mana_cost(),
            Self::JumpStart { .. } => None,
            Self::Escape { cost, .. } => cost.as_ref(),
            Self::Madness { total_cost } => total_cost.mana_cost(),
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
            Self::Flashback { total_cost, .. } => non_mana_components(total_cost),
            Self::Blitz { total_cost } | Self::Madness { total_cost } => {
                non_mana_components(total_cost)
            }
            Self::Harmonize { total_cost } | Self::BeamMeUp { total_cost } => non_mana_components(total_cost),
            Self::Retrace { total_cost } => non_mana_components(total_cost),
            Self::Awaken { additional_cost, .. } | Self::Overload { additional_cost, .. } | Self::Cleave { additional_cost, .. }
            | Self::JumpStart { additional_cost }
            | Self::Warp {
                additional_cost, ..
            }
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
            Self::Flashback { total_cost, .. } => Some(total_cost),
            Self::Blitz { total_cost } | Self::Madness { total_cost } => Some(total_cost),
            Self::Harmonize { total_cost } | Self::BeamMeUp { total_cost } => Some(total_cost),
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
            Self::Awaken { additional_cost, .. } | Self::Overload { additional_cost, .. } | Self::Cleave { additional_cost, .. }
            | Self::JumpStart { additional_cost }
            | Self::Warp {
                additional_cost, ..
            }
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
            Self::Warp { descriptive: false, .. } => "Warp",
            Self::Warp { descriptive: true, .. } => "Defined delayed exile",
            Self::Plot { .. } => "Plot",
            Self::Suspend { .. } => "Suspend",
            Self::Disturb { .. } => "Disturb",
            Self::Overload { descriptive: true, .. } => "Defined text substitution",
            Self::Overload { .. } => "Overload",
            Self::Cleave { descriptive: true, .. } => "Defined text substitution",
            Self::Cleave { .. } => "Cleave",
            Self::Awaken { descriptive: true, .. } => "Defined land animation",
            Self::Awaken { .. } => "Awaken",
            Self::Flashback { .. } => "Flashback",
            Self::Harmonize { .. } => "Harmonize",
            Self::BeamMeUp { .. } => "Beam me up",
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
            entry_counters: Vec::new(),
            instructions: ZoneCastInstructions::default(),
        }
    }

    pub fn with_zone_cast_instructions(mut self, value: ZoneCastInstructions) -> Self {
        if let Self::FromZone { instructions, .. } = &mut self { *instructions = value; }
        self
    }

    pub fn exiles_on_stack_exit(&self) -> bool {
        matches!(self, Self::Flashback { .. } | Self::JumpStart { .. } | Self::Harmonize { .. } | Self::BeamMeUp { .. })
            || matches!(self, Self::FromZone { instructions, .. } if instructions.exile_on_stack_exit)
    }

    pub fn with_entry_counters(mut self, counters: Vec<(crate::CounterType, u32)>) -> Self {
        if let Self::FromZone { entry_counters, .. } = &mut self {
            *entry_counters = counters;
        }
        self
    }

    pub fn entry_counters(&self) -> &[(crate::CounterType, u32)] {
        match self {
            Self::FromZone { entry_counters, .. } => entry_counters,
            _ => &[],
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

    pub fn uses_first_draw_reveal(&self) -> bool {
        self.is_miracle() || matches!(self, Self::FromZone { instructions, .. } if instructions.first_draw_reveal_cast)
    }

    pub fn first_draw_total_cost(&self) -> Option<TotalCost<C>> {
        match self {
            Self::Miracle { cost } => Some(TotalCost::from_costs(vec![C::mana(cost.clone())])),
            Self::FromZone { total_cost, instructions, .. } if instructions.first_draw_reveal_cast => Some(total_cost.clone()),
            _ => None,
        }
    }

    pub fn exile_preparation(&self) -> Option<&ExilePreparation> {
        match self { Self::FromZone { instructions, .. } => instructions.exile_preparation.as_ref(), _ => None }
    }
    pub fn prepares_face_down(&self) -> bool {
        matches!(self, Self::Foretell { .. }) || matches!(self.exile_preparation(), Some(ExilePreparation::FaceDown { .. }))
    }
    pub fn prepares_face_up_sorcery(&self) -> bool {
        matches!(self, Self::Plot { .. }) || matches!(self.exile_preparation(), Some(ExilePreparation::FaceUpSorcery { .. }))
    }
    pub fn plot_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Plot { cost } => Some(cost),
            _ => match self.exile_preparation() {
                Some(ExilePreparation::FaceUpSorcery { price }) => Some(price), _ => None,
            },
        }
    }

    pub fn suspend_spec(&self) -> Option<(crate::SuspendTime, &ManaCost)> {
        match self {
            Self::Suspend { cost, time } => Some((*time, cost)),
            Self::FromZone { instructions, .. } => instructions.timed_exile.as_ref().map(|(time, cost)| (*time, cost)),
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

    pub fn can_tap_creature_for_power_reduction(&self) -> bool {
        matches!(self, Self::Harmonize { .. }) || matches!(self, Self::FromZone { instructions, .. } if instructions.tap_creature_reduces_generic)
    }

    pub fn is_madness(&self) -> bool {
        matches!(self, Self::Madness { .. })
    }

    pub fn discard_exile_cast_cost(&self) -> Option<&TotalCost<C>> {
        match self {
            Self::Madness { total_cost } => Some(total_cost),
            Self::FromZone { total_cost, instructions, .. } if instructions.cast_when_discarded_into_exile => Some(total_cost),
            _ => None,
        }
    }

    pub fn madness_cost(&self) -> Option<&TotalCost<C>> {
        match self {
            Self::Madness { total_cost } => Some(total_cost),
            _ => None,
        }
    }

    pub fn is_bestow(&self) -> bool {
        matches!(self, Self::Bestow { .. }) || matches!(self, Self::FromZone { instructions, .. } if instructions.aura_cast_with_unattached_fallback)
    }

    pub fn is_mutate(&self) -> bool {
        matches!(self, Self::Mutate { .. }) || matches!(self, Self::FromZone { instructions, .. } if instructions.merge_with_same_owner_nonhuman)
    }

    pub fn mutate_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Mutate { cost } => Some(cost),
            Self::FromZone { total_cost, instructions, .. } if instructions.merge_with_same_owner_nonhuman => total_cost.mana_cost(),
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
            Self::Warp {
                cost,
                additional_cost,
                descriptive,
            } => AlternativeCastingMethod::Warp {
                cost,
                descriptive,
                additional_cost: map_total_cost(additional_cost)?,
            },
            Self::Plot { cost } => AlternativeCastingMethod::Plot { cost },
            Self::Suspend { cost, time } => AlternativeCastingMethod::Suspend { cost, time },
            Self::Disturb { cost } => AlternativeCastingMethod::Disturb { cost },
            Self::Overload { cost, effects, descriptive, additional_cost } => AlternativeCastingMethod::Overload {
                cost, descriptive, additional_cost: map_total_cost(additional_cost)?,
                effects: {
                    let mut mapped = Vec::with_capacity(effects.len());
                    for effect in effects {
                        mapped.push(map_effect(effect)?);
                    }
                    mapped
                },
            },
            Self::Cleave { cost, effects, descriptive, additional_cost } => AlternativeCastingMethod::Cleave {
                cost, descriptive, additional_cost: map_total_cost(additional_cost)?,
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
                effects, descriptive, additional_cost,
            } => AlternativeCastingMethod::Awaken {
                amount,
                cost, descriptive, additional_cost: map_total_cost(additional_cost)?,
                effects: {
                    let mut mapped = Vec::with_capacity(effects.len());
                    for effect in effects {
                        mapped.push(map_effect(effect)?);
                    }
                    mapped
                },
            },
            Self::Flashback {
                total_cost,
                x_minimum,
            } => AlternativeCastingMethod::Flashback {
                total_cost: map_total_cost(total_cost)?,
                x_minimum,
            },
            Self::BeamMeUp { total_cost } => AlternativeCastingMethod::BeamMeUp {
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
            Self::Madness { total_cost } => AlternativeCastingMethod::Madness {
                total_cost: map_total_cost(total_cost)?,
            },
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
                entry_counters,
                instructions,
            } => AlternativeCastingMethod::FromZone {
                name,
                zone,
                total_cost: map_total_cost(total_cost)?,
                condition,
                exiles_after_resolution,
                entry_counters,
                instructions,
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
