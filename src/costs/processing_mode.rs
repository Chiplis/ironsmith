//! Cost processing modes for the game loop.
//!
//! This module defines how different costs are processed during cost payment.
//! Each cost knows its own processing mode, eliminating if-else chains in the game loop.

use crate::color::ColorSet;
use crate::cost::PermanentFilter;
use crate::mana::ManaCost;
use crate::types::CardType;

/// How a cost should be processed during cost payment.
///
/// Different costs require different handling:
/// - Some can be paid immediately (tap, life)
/// - Some need the mana payment UI (mana costs)
/// - Some need target/card selection (sacrifice, discard)
/// - Some need inline handling for triggers (sacrifice self)
#[derive(Debug, Clone)]
pub enum CostProcessingMode {
    /// Cost can be paid immediately via `pay()` - no UI interaction needed.
    /// Examples: tap, untap, life payment, remove counters, discard hand.
    Immediate,

    /// Cost requires mana payment through the mana UI.
    ManaPayment {
        /// The mana cost to pay.
        cost: ManaCost,
    },

    /// Cost requires selecting a permanent to sacrifice.
    SacrificeTarget {
        /// Filter for which permanents can be sacrificed.
        filter: PermanentFilter,
    },

    /// Cost requires selecting cards to discard.
    DiscardCards {
        /// Number of cards to discard.
        count: u32,
        /// Optional card type restriction.
        card_type: Option<CardType>,
    },

    /// Cost requires selecting cards to exile from hand.
    ExileFromHand {
        /// Number of cards to exile.
        count: u32,
        /// Optional color filter for cards.
        color_filter: Option<ColorSet>,
    },

    /// Cost must be handled inline with trigger detection.
    /// Used for sacrifice self - the game loop needs to detect dies/LTB triggers.
    InlineWithTriggers,
}

impl CostProcessingMode {
    /// Get a human-readable description for UI display.
    pub fn display(&self) -> String {
        match self {
            CostProcessingMode::Immediate => "Pay cost".to_string(),

            CostProcessingMode::ManaPayment { cost } => {
                format!("Pay {}", format_mana_cost(cost))
            }

            CostProcessingMode::SacrificeTarget { filter } => describe_sacrifice_filter(filter),

            CostProcessingMode::DiscardCards { count, card_type } => {
                let type_str = card_type.map_or("card".to_string(), |ct| {
                    match ct {
                        CardType::Creature => "creature card",
                        CardType::Artifact => "artifact card",
                        CardType::Enchantment => "enchantment card",
                        CardType::Land => "land card",
                        CardType::Planeswalker => "planeswalker card",
                        CardType::Instant => "instant card",
                        CardType::Sorcery => "sorcery card",
                        CardType::Battle => "battle card",
                        CardType::Kindred => "kindred card",
                    }
                    .to_string()
                });

                if *count == 1 {
                    format!("Discard a {}", type_str)
                } else {
                    format!("Discard {} {}s", count, type_str)
                }
            }

            CostProcessingMode::ExileFromHand {
                count,
                color_filter,
            } => {
                let color_desc = if let Some(colors) = color_filter {
                    format!(" {}", format_color_filter(colors))
                } else {
                    String::new()
                };

                if *count == 1 {
                    format!("Exile a{} card from hand", color_desc)
                } else {
                    format!("Exile {}{} cards from hand", count, color_desc)
                }
            }

            CostProcessingMode::InlineWithTriggers => "Sacrifice self".to_string(),
        }
    }

    /// Returns true if this mode requires player interaction/choice.
    pub fn needs_player_choice(&self) -> bool {
        match self {
            CostProcessingMode::Immediate => false,
            CostProcessingMode::InlineWithTriggers => false,
            CostProcessingMode::ManaPayment { .. } => true,
            CostProcessingMode::SacrificeTarget { .. } => true,
            CostProcessingMode::DiscardCards { .. } => true,
            CostProcessingMode::ExileFromHand { .. } => true,
        }
    }

    /// Returns true if this is a mana payment mode.
    pub fn is_mana_payment(&self) -> bool {
        matches!(self, CostProcessingMode::ManaPayment { .. })
    }

    /// Returns the mana cost if this is a mana payment mode.
    pub fn mana_cost(&self) -> Option<&ManaCost> {
        match self {
            CostProcessingMode::ManaPayment { cost } => Some(cost),
            _ => None,
        }
    }

    /// Returns the sacrifice filter if this is a sacrifice target mode.
    pub fn sacrifice_filter(&self) -> Option<&PermanentFilter> {
        match self {
            CostProcessingMode::SacrificeTarget { filter } => Some(filter),
            _ => None,
        }
    }

    /// Returns true if this mode requires inline trigger handling.
    pub fn requires_inline_triggers(&self) -> bool {
        matches!(self, CostProcessingMode::InlineWithTriggers)
    }
}

/// Describe a sacrifice filter for display to the player.
fn describe_sacrifice_filter(filter: &PermanentFilter) -> String {
    let mut parts = Vec::new();

    if filter.other {
        parts.push("another");
    }

    if filter.nontoken {
        parts.push("nontoken");
    }

    if filter.token {
        parts.push("token");
    }

    if !filter.card_types.is_empty() {
        let types: Vec<&str> = filter
            .card_types
            .iter()
            .map(|t| match t {
                CardType::Creature => "creature",
                CardType::Artifact => "artifact",
                CardType::Enchantment => "enchantment",
                CardType::Land => "land",
                CardType::Planeswalker => "planeswalker",
                CardType::Instant => "instant",
                CardType::Sorcery => "sorcery",
                CardType::Battle => "battle",
                CardType::Kindred => "kindred",
            })
            .collect();
        parts.push(Box::leak(types.join(" or ").into_boxed_str()));
    } else {
        parts.push("permanent");
    }

    format!("Sacrifice a {}", parts.join(" "))
}

/// Format a mana cost for display.
fn format_mana_cost(cost: &ManaCost) -> String {
    use crate::mana::ManaSymbol;

    let mut parts = Vec::new();

    for pip in cost.pips() {
        if pip.len() == 1 {
            // Single option pip
            match pip[0] {
                ManaSymbol::White => parts.push("{W}".to_string()),
                ManaSymbol::Blue => parts.push("{U}".to_string()),
                ManaSymbol::Black => parts.push("{B}".to_string()),
                ManaSymbol::Red => parts.push("{R}".to_string()),
                ManaSymbol::Green => parts.push("{G}".to_string()),
                ManaSymbol::Colorless => parts.push("{C}".to_string()),
                ManaSymbol::Generic(n) => parts.push(format!("{{{}}}", n)),
                ManaSymbol::Snow => parts.push("{S}".to_string()),
                ManaSymbol::Life(n) => parts.push(format!("{{{}/P}}", n)),
                ManaSymbol::X => parts.push("{X}".to_string()),
            }
        } else {
            // Hybrid/alternative pip
            let alts: Vec<String> = pip
                .iter()
                .map(|s| match s {
                    ManaSymbol::White => "W".to_string(),
                    ManaSymbol::Blue => "U".to_string(),
                    ManaSymbol::Black => "B".to_string(),
                    ManaSymbol::Red => "R".to_string(),
                    ManaSymbol::Green => "G".to_string(),
                    ManaSymbol::Colorless => "C".to_string(),
                    ManaSymbol::Generic(n) => format!("{}", n),
                    ManaSymbol::Snow => "S".to_string(),
                    ManaSymbol::Life(n) => format!("{}/P", n),
                    ManaSymbol::X => "X".to_string(),
                })
                .collect();
            parts.push(format!("{{{}}}", alts.join("/")));
        }
    }

    if parts.is_empty() {
        "{0}".to_string()
    } else {
        parts.join("")
    }
}

/// Format a color filter for display.
fn format_color_filter(colors: &ColorSet) -> String {
    use crate::color::Color;

    let mut color_names = Vec::new();
    if colors.contains(Color::White) {
        color_names.push("white");
    }
    if colors.contains(Color::Blue) {
        color_names.push("blue");
    }
    if colors.contains(Color::Black) {
        color_names.push("black");
    }
    if colors.contains(Color::Red) {
        color_names.push("red");
    }
    if colors.contains(Color::Green) {
        color_names.push("green");
    }

    if color_names.is_empty() {
        String::new()
    } else {
        color_names.join(" or ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immediate_mode() {
        let mode = CostProcessingMode::Immediate;
        assert!(!mode.needs_player_choice());
        assert!(!mode.is_mana_payment());
        assert!(!mode.requires_inline_triggers());
    }

    #[test]
    fn test_mana_payment_mode() {
        let cost = ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Generic(2)]]);
        let mode = CostProcessingMode::ManaPayment { cost: cost.clone() };

        assert!(mode.needs_player_choice());
        assert!(mode.is_mana_payment());
        assert_eq!(mode.mana_cost(), Some(&cost));
        assert!(!mode.requires_inline_triggers());
    }

    #[test]
    fn test_sacrifice_target_mode() {
        let filter = PermanentFilter::creature();
        let mode = CostProcessingMode::SacrificeTarget {
            filter: filter.clone(),
        };

        assert!(mode.needs_player_choice());
        assert!(!mode.is_mana_payment());
        assert_eq!(mode.sacrifice_filter(), Some(&filter));
        assert_eq!(mode.display(), "Sacrifice a creature");
    }

    #[test]
    fn test_discard_cards_mode() {
        let mode = CostProcessingMode::DiscardCards {
            count: 2,
            card_type: None,
        };

        assert!(mode.needs_player_choice());
        assert_eq!(mode.display(), "Discard 2 cards");

        let mode_typed = CostProcessingMode::DiscardCards {
            count: 1,
            card_type: Some(CardType::Creature),
        };
        assert_eq!(mode_typed.display(), "Discard a creature card");
    }

    #[test]
    fn test_exile_from_hand_mode() {
        let mode = CostProcessingMode::ExileFromHand {
            count: 1,
            color_filter: None,
        };

        assert!(mode.needs_player_choice());
        assert_eq!(mode.display(), "Exile a card from hand");
    }

    #[test]
    fn test_inline_with_triggers_mode() {
        let mode = CostProcessingMode::InlineWithTriggers;

        assert!(!mode.needs_player_choice());
        assert!(mode.requires_inline_triggers());
        assert_eq!(mode.display(), "Sacrifice self");
    }
}
