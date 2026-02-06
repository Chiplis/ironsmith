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
    Flashback { cost: ManaCost },

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

    /// Alternative cost - cast from hand, but pay a different cost instead of mana cost.
    /// Used for cards like Force of Will ("pay 1 life, exile a blue card").
    ///
    /// The mana_cost field holds the mana portion (if any), while cost_effects
    /// holds non-mana costs that execute through the effect system.
    AlternativeCost {
        /// The name shown to the player (e.g., "Force of Will's alternative cost")
        name: &'static str,
        /// The mana portion of the alternative cost (None = no mana cost)
        mana_cost: Option<ManaCost>,
        /// Non-mana cost effects (pay life, exile cards, etc.)
        cost_effects: Vec<Effect>,
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

impl AlternativeCastingMethod {
    /// Returns the zone this method allows casting from.
    pub fn cast_from_zone(&self) -> Zone {
        match self {
            Self::Flashback { .. } | Self::JumpStart | Self::Escape { .. } => Zone::Graveyard,
            Self::Madness { .. } => Zone::Exile,
            Self::Miracle { .. } | Self::AlternativeCost { .. } | Self::MindbreakTrap { .. } => {
                Zone::Hand
            }
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
            Self::Flashback { cost } => Some(cost),
            Self::JumpStart => None, // Uses normal mana cost
            Self::Escape { cost, .. } => cost.as_ref(), // None means use normal mana cost
            Self::Madness { cost } => Some(cost),
            Self::Miracle { cost } => Some(cost),
            Self::MindbreakTrap { cost, .. } => Some(cost),
            Self::AlternativeCost { mana_cost, .. } => mana_cost.as_ref(),
        }
    }

    /// Returns the cost effects for this alternative casting method.
    /// These are non-mana costs that execute through the effect system.
    pub fn cost_effects(&self) -> &[Effect] {
        match self {
            Self::AlternativeCost { cost_effects, .. } => cost_effects,
            _ => &[],
        }
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
            Self::AlternativeCost { name, .. } => name,
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

    /// Create an alternative cost method (for cards like Force of Will).
    ///
    /// # Arguments
    /// * `name` - Display name for the alternative cost
    /// * `mana_cost` - Mana portion of the cost (None for no mana)
    /// * `cost_effects` - Non-mana cost effects (pay life, exile cards, etc.)
    pub fn alternative_cost(
        name: &'static str,
        mana_cost: Option<ManaCost>,
        cost_effects: Vec<Effect>,
    ) -> Self {
        Self::AlternativeCost {
            name,
            mana_cost,
            cost_effects,
        }
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
    fn test_alternative_cost_properties() {
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
