//! Alternative casting methods for spells (Flashback, Escape, etc.)
//!
//! This module provides infrastructure for casting spells using alternative costs
//! from zones other than hand. Each alternative casting method specifies:
//! - The zone the spell can be cast from
//! - The cost to pay (usually different from the normal mana cost)
//! - What happens after the spell resolves (usually exile)

use crate::effect::Effect;
use crate::mana::ManaCost;
use crate::zone::Zone;

/// Methods for casting a spell other than from hand for normal cost.
#[derive(Debug, Clone, PartialEq)]
pub enum AlternativeCastingMethod {
    /// Flashback - cast from graveyard for alternative cost, exile after
    Flashback {
        cost: ManaCost,
        /// Non-mana additional costs paid as part of casting with flashback.
        cost_effects: Vec<Effect>,
    },

    /// Jump-start - cast from graveyard, discard a card as additional cost, exile after
    JumpStart,

    /// Escape - cast from graveyard for alternative cost, exile N other cards.
    /// If cost is None, uses the card's normal mana cost (for granted escape).
    Escape {
        cost: Option<ManaCost>,
        exile_count: u32,
    },

    /// Madness - when discarded, may cast for madness cost from exile
    Madness { cost: ManaCost },

    /// Miracle - if first card drawn this turn, may cast for miracle cost
    Miracle { cost: ManaCost },

    /// Composed alternative cost - cast from hand, with optional mana plus
    /// additional non-mana cost effects composed through the effect system.
    /// Used for cards like Force of Will ("pay 1 life, exile a blue card").
    ///
    /// The mana_cost field holds the mana portion (if any), while cost_effects
    /// holds non-mana costs that execute through the effect system.
    Composed {
        /// The name shown to the player (e.g., "Force of Will's alternative cost")
        name: &'static str,
        /// The mana portion of the alternative cost (None = no mana cost)
        mana_cost: Option<ManaCost>,
        /// Non-mana cost effects (pay life, exile cards, etc.)
        cost_effects: Vec<Effect>,
        /// Optional cast-time condition for this alternative cost.
        ///
        /// Used for lines like:
        /// "If an opponent controls a Mountain and you control a Plains,
        /// you may cast this spell without paying its mana cost."
        condition: Option<crate::static_abilities::ThisSpellCostCondition>,
    },

    /// Trap - cast for alternative (usually free) cost when a condition is met.
    /// Used for cards like Mindbreak Trap, Archive Trap, etc.
    MindbreakTrap {
        /// The name shown to the player (e.g., "Mindbreak Trap's trap cost")
        name: &'static str,
        /// The cost to pay (usually {0})
        cost: ManaCost,
        /// The condition that must be met
        condition: TrapCondition,
    },
}

/// Conditions for when a trap's alternative cost can be used.
#[derive(Debug, Clone, PartialEq)]
pub enum TrapCondition {
    /// An opponent cast N or more spells this turn
    OpponentCastSpells { count: u32 },
    /// An opponent searched their library this turn
    OpponentSearchedLibrary,
    /// An opponent had a creature enter the battlefield this turn
    OpponentCreatureEntered,
    /// A creature dealt damage to you this turn
    CreatureDealtDamageToYou,
}

/// Static requirement bits needed to use an alternative casting method.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AlternativeCastRequirements {
    /// Number of graveyard cards that must be exiled as part of the cast.
    pub exile_from_graveyard: u32,
    /// Number of cards that must be discarded from hand as part of the cast.
    pub discard_from_hand: u32,
}

impl AlternativeCastingMethod {
    /// Returns the zone this method allows casting from.
    pub fn cast_from_zone(&self) -> Zone {
        match self {
            Self::Flashback { .. } | Self::JumpStart | Self::Escape { .. } => Zone::Graveyard,
            Self::Madness { .. } => Zone::Exile,
            Self::Miracle { .. } | Self::Composed { .. } | Self::MindbreakTrap { .. } => Zone::Hand,
        }
    }

    /// Returns true if the spell should be exiled after resolution.
    pub fn exiles_after_resolution(&self) -> bool {
        matches!(
            self,
            Self::Flashback { .. } | Self::JumpStart | Self::Escape { .. }
        )
    }

    /// Returns the mana cost for this alternative casting method.
    /// Returns None for methods that use the card's normal mana cost (Jump-start, granted Escape).
    pub fn mana_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Flashback { cost, .. } => Some(cost),
            Self::JumpStart => None, // Uses normal mana cost
            Self::Escape { cost, .. } => cost.as_ref(), // None means use normal mana cost
            Self::Madness { cost } => Some(cost),
            Self::Miracle { cost } => Some(cost),
            Self::MindbreakTrap { cost, .. } => Some(cost),
            Self::Composed { mana_cost, .. } => mana_cost.as_ref(),
        }
    }

    /// Returns the cost effects for this alternative casting method.
    /// These are non-mana costs that execute through the effect system.
    pub fn cost_effects(&self) -> &[Effect] {
        match self {
            Self::Flashback { cost_effects, .. } => cost_effects,
            Self::Composed { cost_effects, .. } => cost_effects,
            _ => &[],
        }
    }

    /// Returns the cast-time condition for this alternative casting method, if any.
    pub fn cast_condition(&self) -> Option<&crate::static_abilities::ThisSpellCostCondition> {
        match self {
            Self::Composed { condition, .. } => condition.as_ref(),
            _ => None,
        }
    }

    /// Attach a cast-time condition to a composed alternative cast method.
    ///
    /// Non-composed methods are returned unchanged.
    pub fn with_cast_condition(
        mut self,
        condition: crate::static_abilities::ThisSpellCostCondition,
    ) -> Self {
        if let Self::Composed {
            condition: existing_condition,
            ..
        } = &mut self
        {
            *existing_condition = Some(condition);
        }
        self
    }

    /// Returns the exile from hand requirements, if any.
    ///
    /// This checks the cost_effects for ExileFromHandAsCostEffect and returns
    /// the (count, color_filter) if found.
    pub fn exile_from_hand_requirement(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        for effect in self.cost_effects() {
            if let Some(info) = effect.0.exile_from_hand_cost_info() {
                return Some(info);
            }
        }
        None
    }

    /// Returns the name of this casting method for display.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Flashback { .. } => "Flashback",
            Self::JumpStart => "Jump-start",
            Self::Escape { .. } => "Escape",
            Self::Madness { .. } => "Madness",
            Self::Miracle { .. } => "Miracle",
            Self::Composed { name, .. } => name,
            Self::MindbreakTrap { name, .. } => name,
        }
    }

    /// Create a trap alternative casting method.
    pub fn trap(name: &'static str, cost: ManaCost, condition: TrapCondition) -> Self {
        Self::MindbreakTrap {
            name,
            cost,
            condition,
        }
    }

    /// Returns the trap condition, if this is a trap.
    pub fn trap_condition(&self) -> Option<&TrapCondition> {
        match self {
            Self::MindbreakTrap { condition, .. } => Some(condition),
            _ => None,
        }
    }

    /// Create a composed alternative cast method (for cards like Force of Will).
    ///
    /// # Arguments
    /// * `name` - Display name for the method
    /// * `mana_cost` - Mana portion of the cost (None for no mana)
    /// * `cost_effects` - Non-mana cost effects (pay life, exile cards, etc.)
    pub fn alternative_cost(
        name: &'static str,
        mana_cost: Option<ManaCost>,
        cost_effects: Vec<Effect>,
    ) -> Self {
        Self::Composed {
            name,
            mana_cost,
            cost_effects,
            condition: None,
        }
    }

    /// Create a composed alternative cast method with an explicit cast-time condition.
    pub fn alternative_cost_with_condition(
        name: &'static str,
        mana_cost: Option<ManaCost>,
        cost_effects: Vec<Effect>,
        condition: crate::static_abilities::ThisSpellCostCondition,
    ) -> Self {
        Self::Composed {
            name,
            mana_cost,
            cost_effects,
            condition: Some(condition),
        }
    }

    /// Returns the zone-independent gameplay requirements for this alternative casting method.
    ///
    /// This captures non-mana requirements expressed outside the base mana cost, such as
    /// graveyard exile counts or hand discard counts. Additional costs like life payment,
    /// card choice, etc. remain composed in `cost_effects()`.
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
            Self::Flashback { .. } | Self::Miracle { .. } | Self::Madness { .. } => {
                AlternativeCastRequirements::default()
            }
            Self::Composed { .. } => AlternativeCastRequirements::default(),
            Self::MindbreakTrap { .. } => AlternativeCastRequirements::default(),
        }
    }

    /// Returns true if this method is paid through composed non-mana cost effects.
    pub fn uses_composed_cost_effects(&self) -> bool {
        matches!(self, Self::Composed { .. })
    }

    /// Returns true if this is the Miracle alternative casting method.
    pub fn is_miracle(&self) -> bool {
        matches!(self, Self::Miracle { .. })
    }

    /// Returns the miracle cost if this is a Miracle method.
    pub fn miracle_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Miracle { cost } => Some(cost),
            _ => None,
        }
    }

    /// Returns true if this is the Madness alternative casting method.
    pub fn is_madness(&self) -> bool {
        matches!(self, Self::Madness { .. })
    }

    /// Returns the madness cost if this is a Madness method.
    pub fn madness_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Madness { cost } => Some(cost),
            _ => None,
        }
    }
}

/// Which method is being used to cast a spell.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CastingMethod {
    /// Normal casting from hand with normal mana cost.
    #[default]
    Normal,
    /// Alternative casting using the method at the given index in the card's alternative_casts.
    Alternative(usize),
    /// Escape granted by another permanent (e.g., Underworld Breach).
    /// Uses the card's own mana cost plus exiling N other cards from graveyard.
    GrantedEscape {
        /// The permanent granting escape
        source: crate::ids::ObjectId,
        /// Number of cards to exile from graveyard
        exile_count: u32,
    },
    /// Flashback granted by another card (e.g., Snapcaster Mage).
    /// Uses the card's own mana cost and exiles after resolution.
    GrantedFlashback,
    /// Cast from a non-hand zone as if from hand (Yawgmoth's Will, etc.).
    /// Can use normal mana cost or any alternative cost the card has.
    /// Does NOT automatically exile - the granting effect has a separate replacement effect if needed.
    PlayFrom {
        /// The source granting this ability
        source: crate::ids::ObjectId,
        /// The zone the card is being cast from
        zone: Zone,
        /// If Some, use the alternative cost at this index instead of normal cost
        use_alternative: Option<usize>,
    },
}

impl CastingMethod {
    /// Returns true if this is an alternative casting method.
    pub fn is_alternative(&self) -> bool {
        matches!(self, Self::Alternative(_))
    }

    /// Returns true if the spell should be exiled after resolution.
    pub fn exiles_after_resolution(&self) -> bool {
        matches!(self, Self::GrantedFlashback | Self::GrantedEscape { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana::ManaSymbol;

    #[test]
    fn test_flashback_properties() {
        let flashback = AlternativeCastingMethod::Flashback {
            cost: ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Blue]]),
            cost_effects: Vec::new(),
        };

        assert_eq!(flashback.cast_from_zone(), Zone::Graveyard);
        assert!(flashback.exiles_after_resolution());
        assert!(flashback.mana_cost().is_some());
        assert_eq!(flashback.name(), "Flashback");
    }

    #[test]
    fn test_jump_start_properties() {
        let jump_start = AlternativeCastingMethod::JumpStart;

        assert_eq!(jump_start.cast_from_zone(), Zone::Graveyard);
        assert!(jump_start.exiles_after_resolution());
        assert!(jump_start.mana_cost().is_none()); // Uses normal cost
        assert_eq!(jump_start.name(), "Jump-start");
    }

    #[test]
    fn test_escape_properties() {
        let escape = AlternativeCastingMethod::Escape {
            cost: Some(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ])),
            exile_count: 4,
        };

        assert_eq!(escape.cast_from_zone(), Zone::Graveyard);
        assert!(escape.exiles_after_resolution());
        assert!(escape.mana_cost().is_some());
        assert_eq!(escape.name(), "Escape");
    }

    #[test]
    fn test_madness_properties() {
        let madness = AlternativeCastingMethod::Madness {
            cost: ManaCost::from_pips(vec![vec![ManaSymbol::Red]]),
        };

        assert_eq!(madness.cast_from_zone(), Zone::Exile);
        assert!(!madness.exiles_after_resolution());
        assert!(madness.mana_cost().is_some());
        assert_eq!(madness.name(), "Madness");
    }

    #[test]
    fn test_miracle_properties() {
        let miracle = AlternativeCastingMethod::Miracle {
            cost: ManaCost::from_pips(vec![vec![ManaSymbol::White]]),
        };

        assert_eq!(miracle.cast_from_zone(), Zone::Hand);
        assert!(!miracle.exiles_after_resolution());
        assert!(miracle.mana_cost().is_some());
        assert_eq!(miracle.name(), "Miracle");
    }

    #[test]
    fn test_casting_method() {
        let normal = CastingMethod::Normal;
        let alternative = CastingMethod::Alternative(0);

        assert!(!normal.is_alternative());
        assert!(alternative.is_alternative());

        // Default should be Normal
        assert_eq!(CastingMethod::default(), CastingMethod::Normal);
    }

    #[test]
    fn test_composed_alternative_properties() {
        use crate::color::{Color, ColorSet};
        use crate::effect::Effect;

        // Force of Will style: pay 1 life, exile a blue card
        let cost_effects = vec![
            Effect::pay_life(1),
            Effect::exile_from_hand_as_cost(1, Some(ColorSet::from(Color::Blue))),
        ];

        let alternative = AlternativeCastingMethod::alternative_cost(
            "Force of Will",
            None, // No mana cost
            cost_effects,
        );

        assert_eq!(alternative.cast_from_zone(), Zone::Hand);
        assert!(!alternative.exiles_after_resolution());
        assert!(alternative.mana_cost().is_none()); // No mana cost for FoW alternative
        assert_eq!(alternative.cost_effects().len(), 2);
        assert_eq!(alternative.name(), "Force of Will");
    }
}
