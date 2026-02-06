use crate::color::{Color, ColorSet};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Represents power or toughness values that may be variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtValue {
    /// Fixed numeric value (e.g., 4)
    Fixed(i32),
    /// Star value, determined by some characteristic (e.g., *)
    Star,
    /// Star plus a number (e.g., *+1)
    StarPlus(i32),
}

impl PtValue {
    /// Returns the base numeric value, treating Star as 0.
    pub fn base_value(self) -> i32 {
        match self {
            PtValue::Fixed(n) => n,
            PtValue::Star => 0,
            PtValue::StarPlus(n) => n,
        }
    }
}

/// Power and toughness pair for creatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerToughness {
    pub power: PtValue,
    pub toughness: PtValue,
}

impl PowerToughness {
    pub fn new(power: PtValue, toughness: PtValue) -> Self {
        Self { power, toughness }
    }

    /// Creates a P/T from fixed numeric values.
    pub fn fixed(power: i32, toughness: i32) -> Self {
        Self {
            power: PtValue::Fixed(power),
            toughness: PtValue::Fixed(toughness),
        }
    }
}

/// Static, immutable card definition.
/// This represents the printed characteristics of a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub id: CardId,
    pub name: String,
    pub mana_cost: Option<ManaCost>,
    /// Color indicator (for cards like Ancestral Vision or DFC backs)
    pub color_indicator: Option<ColorSet>,
    pub supertypes: Vec<Supertype>,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub oracle_text: String,
    pub power_toughness: Option<PowerToughness>,
    /// Starting loyalty for planeswalkers
    pub loyalty: Option<u32>,
    /// Defense value for battles
    pub defense: Option<u32>,
    /// Reference to other face for DFCs/MDFCs
    pub other_face: Option<CardId>,
    /// True if this is a token (not a real card)
    pub is_token: bool,
}

impl Card {
    /// Returns the colors of this card based on mana cost and color indicator.
    pub fn colors(&self) -> ColorSet {
        if let Some(indicator) = self.color_indicator {
            return indicator;
        }

        let Some(mana_cost) = &self.mana_cost else {
            return ColorSet::COLORLESS;
        };

        let mut colors = ColorSet::COLORLESS;
        for pip in mana_cost.pips() {
            for symbol in pip {
                match symbol {
                    ManaSymbol::White => colors = colors.with(Color::White),
                    ManaSymbol::Blue => colors = colors.with(Color::Blue),
                    ManaSymbol::Black => colors = colors.with(Color::Black),
                    ManaSymbol::Red => colors = colors.with(Color::Red),
                    ManaSymbol::Green => colors = colors.with(Color::Green),
                    _ => {}
                }
            }
        }
        colors
    }

    /// Returns the color identity of this card (for Commander format).
    /// Color identity includes colors from:
    /// - Mana cost
    /// - Color indicator
    /// - Mana symbols in rules text (e.g., "{T}: Add {G}")
    /// - Characteristic-defining abilities that set color
    pub fn color_identity(&self) -> ColorSet {
        let mut identity = ColorSet::COLORLESS;

        // Add colors from mana cost
        if let Some(mana_cost) = &self.mana_cost {
            for pip in mana_cost.pips() {
                for symbol in pip {
                    match symbol {
                        ManaSymbol::White => identity = identity.with(Color::White),
                        ManaSymbol::Blue => identity = identity.with(Color::Blue),
                        ManaSymbol::Black => identity = identity.with(Color::Black),
                        ManaSymbol::Red => identity = identity.with(Color::Red),
                        ManaSymbol::Green => identity = identity.with(Color::Green),
                        _ => {}
                    }
                }
            }
        }

        // Add colors from color indicator
        if let Some(indicator) = self.color_indicator {
            identity = identity.union(indicator);
        }

        // Parse oracle text for mana symbols
        identity = identity.union(Self::parse_colors_from_text(&self.oracle_text));

        identity
    }

    /// Parses mana symbols from rules text and returns the colors found.
    fn parse_colors_from_text(text: &str) -> ColorSet {
        let mut colors = ColorSet::COLORLESS;
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // Find the closing brace
                if let Some(end) = chars[i..].iter().position(|&c| c == '}') {
                    let symbol: String = chars[i + 1..i + end].iter().collect();
                    // Check for color symbols (including in hybrid like "W/U")
                    for c in symbol.chars() {
                        match c {
                            'W' => colors = colors.with(Color::White),
                            'U' => colors = colors.with(Color::Blue),
                            'B' => colors = colors.with(Color::Black),
                            'R' => colors = colors.with(Color::Red),
                            'G' => colors = colors.with(Color::Green),
                            _ => {}
                        }
                    }
                    i += end + 1;
                    continue;
                }
            }
            i += 1;
        }
        colors
    }

    /// Returns the mana value of this card.
    pub fn mana_value(&self) -> u32 {
        self.mana_cost.as_ref().map_or(0, |c| c.mana_value())
    }

    /// Returns true if this card has the given card type.
    pub fn has_card_type(&self, card_type: CardType) -> bool {
        self.card_types.contains(&card_type)
    }

    /// Returns true if this card has the given supertype.
    pub fn has_supertype(&self, supertype: Supertype) -> bool {
        self.supertypes.contains(&supertype)
    }

    /// Returns true if this card has the given subtype.
    pub fn has_subtype(&self, subtype: Subtype) -> bool {
        self.subtypes.contains(&subtype)
    }

    /// Returns true if this is a creature card.
    pub fn is_creature(&self) -> bool {
        self.has_card_type(CardType::Creature)
    }

    /// Returns true if this is a land card.
    pub fn is_land(&self) -> bool {
        self.has_card_type(CardType::Land)
    }

    /// Returns true if this is an instant card.
    pub fn is_instant(&self) -> bool {
        self.has_card_type(CardType::Instant)
    }

    /// Returns true if this is a sorcery card.
    pub fn is_sorcery(&self) -> bool {
        self.has_card_type(CardType::Sorcery)
    }

    /// Returns true if this is an artifact card.
    pub fn is_artifact(&self) -> bool {
        self.has_card_type(CardType::Artifact)
    }

    /// Returns true if this is an enchantment card.
    pub fn is_enchantment(&self) -> bool {
        self.has_card_type(CardType::Enchantment)
    }

    /// Returns true if this is a planeswalker card.
    pub fn is_planeswalker(&self) -> bool {
        self.has_card_type(CardType::Planeswalker)
    }

    /// Returns true if this is a legendary card.
    pub fn is_legendary(&self) -> bool {
        self.has_supertype(Supertype::Legendary)
    }
}

/// Builder for constructing Card instances.
#[derive(Debug, Default, Clone)]
pub struct CardBuilder {
    id: CardId,
    name: String,
    mana_cost: Option<ManaCost>,
    color_indicator: Option<ColorSet>,
    supertypes: Vec<Supertype>,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    oracle_text: String,
    power_toughness: Option<PowerToughness>,
    loyalty: Option<u32>,
    defense: Option<u32>,
    other_face: Option<CardId>,
    is_token: bool,
}

impl CardBuilder {
    pub fn new(id: CardId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn mana_cost(mut self, cost: ManaCost) -> Self {
        self.mana_cost = Some(cost);
        self
    }

    pub fn color_indicator(mut self, colors: ColorSet) -> Self {
        self.color_indicator = Some(colors);
        self
    }

    pub fn supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        self.supertypes = supertypes;
        self
    }

    pub fn card_types(mut self, types: Vec<CardType>) -> Self {
        self.card_types = types;
        self
    }

    pub fn subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.subtypes = subtypes;
        self
    }

    pub fn oracle_text(mut self, text: impl Into<String>) -> Self {
        self.oracle_text = text.into();
        self
    }

    pub fn name_ref(&self) -> &str {
        &self.name
    }

    pub fn mana_cost_ref(&self) -> Option<&ManaCost> {
        self.mana_cost.as_ref()
    }

    pub fn supertypes_ref(&self) -> &[Supertype] {
        &self.supertypes
    }

    pub fn card_types_ref(&self) -> &[CardType] {
        &self.card_types
    }

    pub fn subtypes_ref(&self) -> &[Subtype] {
        &self.subtypes
    }

    pub fn power_toughness_ref(&self) -> Option<PowerToughness> {
        self.power_toughness
    }

    pub fn loyalty_ref(&self) -> Option<u32> {
        self.loyalty
    }

    pub fn defense_ref(&self) -> Option<u32> {
        self.defense
    }

    pub fn power_toughness(mut self, pt: PowerToughness) -> Self {
        self.power_toughness = Some(pt);
        self
    }

    pub fn loyalty(mut self, loyalty: u32) -> Self {
        self.loyalty = Some(loyalty);
        self
    }

    pub fn defense(mut self, defense: u32) -> Self {
        self.defense = Some(defense);
        self
    }

    pub fn other_face(mut self, face: CardId) -> Self {
        self.other_face = Some(face);
        self
    }

    /// Mark this as a token (not a real card).
    pub fn token(mut self) -> Self {
        self.is_token = true;
        self
    }

    pub fn build(self) -> Card {
        Card {
            id: self.id,
            name: self.name,
            mana_cost: self.mana_cost,
            color_indicator: self.color_indicator,
            supertypes: self.supertypes,
            card_types: self.card_types,
            subtypes: self.subtypes,
            oracle_text: self.oracle_text,
            power_toughness: self.power_toughness,
            loyalty: self.loyalty,
            defense: self.defense,
            other_face: self.other_face,
            is_token: self.is_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lightning_bolt() -> Card {
        CardBuilder::new(CardId::from_raw(1), "Lightning Bolt")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .oracle_text("Lightning Bolt deals 3 damage to any target.")
            .build()
    }

    fn serra_angel() -> Card {
        CardBuilder::new(CardId::from_raw(2), "Serra Angel")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::White],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .oracle_text("Flying, vigilance")
            .power_toughness(PowerToughness::fixed(4, 4))
            .build()
    }

    fn nicol_bolas_planeswalker() -> Card {
        // Nicol Bolas, Planeswalker - {4}{U}{B}{B}{R}
        CardBuilder::new(CardId::from_raw(3), "Nicol Bolas, Planeswalker")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(4)],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Red],
            ]))
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Planeswalker])
            .oracle_text("+3: Destroy target noncreature permanent.\n-2: Gain control of target creature.\n-9: Nicol Bolas, Planeswalker deals 7 damage to target player or planeswalker. That player or that planeswalker's controller discards seven cards, then sacrifices seven permanents.")
            .loyalty(5)
            .build()
    }

    fn basic_forest() -> Card {
        CardBuilder::new(CardId::from_raw(4), "Forest")
            .supertypes(vec![Supertype::Basic])
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .oracle_text("{T}: Add {G}.")
            .build()
    }

    #[test]
    fn test_lightning_bolt() {
        let bolt = lightning_bolt();
        assert_eq!(bolt.name, "Lightning Bolt");
        assert_eq!(bolt.mana_value(), 1);
        assert!(bolt.colors().contains(Color::Red));
        assert_eq!(bolt.colors().count(), 1);
        assert!(bolt.is_instant());
        assert!(!bolt.is_creature());
    }

    #[test]
    fn test_serra_angel() {
        let angel = serra_angel();
        assert_eq!(angel.name, "Serra Angel");
        assert_eq!(angel.mana_value(), 5);
        assert!(angel.colors().contains(Color::White));
        assert_eq!(angel.colors().count(), 1);
        assert!(angel.is_creature());
        assert!(angel.has_subtype(Subtype::Angel));
        let pt = angel.power_toughness.unwrap();
        assert_eq!(pt.power.base_value(), 4);
        assert_eq!(pt.toughness.base_value(), 4);
    }

    #[test]
    fn test_nicol_bolas_planeswalker() {
        let bolas = nicol_bolas_planeswalker();
        assert_eq!(bolas.mana_value(), 8);
        assert!(bolas.is_legendary());
        assert!(bolas.is_planeswalker());
        assert_eq!(bolas.loyalty, Some(5));

        let colors = bolas.colors();
        assert!(colors.contains(Color::Blue));
        assert!(colors.contains(Color::Black));
        assert!(colors.contains(Color::Red));
        assert_eq!(colors.count(), 3);
    }

    #[test]
    fn test_basic_forest() {
        let forest = basic_forest();
        assert_eq!(forest.mana_value(), 0);
        assert!(forest.colors().is_empty());
        assert!(forest.is_land());
        assert!(forest.has_supertype(Supertype::Basic));
        assert!(forest.has_subtype(Subtype::Forest));
    }

    #[test]
    fn test_color_indicator() {
        // Simulate a card like Ancestral Vision (no mana cost, blue color indicator)
        let card = CardBuilder::new(CardId::from_raw(5), "Ancestral Vision")
            .color_indicator(ColorSet::BLUE)
            .card_types(vec![CardType::Sorcery])
            .oracle_text("Suspend 4—{U}\nTarget player draws three cards.")
            .build();

        assert!(card.colors().contains(Color::Blue));
        assert_eq!(card.colors().count(), 1);
        assert_eq!(card.mana_value(), 0);
    }

    #[test]
    fn test_pt_value() {
        assert_eq!(PtValue::Fixed(5).base_value(), 5);
        assert_eq!(PtValue::Star.base_value(), 0);
        assert_eq!(PtValue::StarPlus(1).base_value(), 1);
    }

    #[test]
    fn test_color_identity_basic() {
        // Lightning Bolt: {R} mana cost, no text symbols
        let bolt = lightning_bolt();
        let identity = bolt.color_identity();
        assert!(identity.contains(Color::Red));
        assert_eq!(identity.count(), 1);
    }

    #[test]
    fn test_color_identity_with_mana_in_text() {
        // A basic Forest has "{T}: Add {G}." in its text
        let forest = basic_forest();
        let identity = forest.color_identity();
        assert!(identity.contains(Color::Green));
        assert_eq!(identity.count(), 1);
    }

    #[test]
    fn test_color_identity_multicolor_text() {
        // Simulate a card like Birds of Paradise: {G} cost, "{T}: Add one mana of any color."
        // But let's make one with explicit symbols like Deathrite Shaman
        let card = CardBuilder::new(CardId::from_raw(10), "Test Mana Dork")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .oracle_text("{T}: Add {W}, {U}, {B}, or {R}.")
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let identity = card.color_identity();
        assert!(identity.contains(Color::White));
        assert!(identity.contains(Color::Blue));
        assert!(identity.contains(Color::Black));
        assert!(identity.contains(Color::Red));
        assert!(identity.contains(Color::Green));
        assert_eq!(identity.count(), 5);
    }

    #[test]
    fn test_color_identity_with_color_indicator() {
        // DFC back with color indicator and mana symbols in text
        let card = CardBuilder::new(CardId::from_raw(11), "Transformed Side")
            .color_indicator(ColorSet::RED)
            .card_types(vec![CardType::Creature])
            .oracle_text("At the beginning of your upkeep, add {B}.")
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();

        let identity = card.color_identity();
        assert!(identity.contains(Color::Red));
        assert!(identity.contains(Color::Black));
        assert_eq!(identity.count(), 2);
    }
}
