use crate::color::Color;

/// Atomic mana payment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManaSymbol {
    /// White mana {W}
    White,
    /// Blue mana {U}
    Blue,
    /// Black mana {B}
    Black,
    /// Red mana {R}
    Red,
    /// Green mana {G}
    Green,
    /// Colorless mana {C}
    Colorless,
    /// Generic mana {1}, {2}, etc.
    Generic(u8),
    /// Snow mana {S}
    Snow,
    /// Life payment for Phyrexian costs
    Life(u8),
    /// Variable mana {X}
    X,
}

impl ManaSymbol {
    /// Returns the mana value contribution of this symbol.
    pub fn mana_value(&self) -> u32 {
        match self {
            ManaSymbol::White => 1,
            ManaSymbol::Blue => 1,
            ManaSymbol::Black => 1,
            ManaSymbol::Red => 1,
            ManaSymbol::Green => 1,
            ManaSymbol::Colorless => 1,
            ManaSymbol::Generic(n) => *n as u32,
            ManaSymbol::Snow => 1,
            ManaSymbol::Life(_) => 0, // Life payment doesn't contribute to mana value
            ManaSymbol::X => 0,       // X is 0 except on the stack
        }
    }

    /// Creates a colored mana symbol from a Color.
    pub fn from_color(color: Color) -> Self {
        match color {
            Color::White => ManaSymbol::White,
            Color::Blue => ManaSymbol::Blue,
            Color::Black => ManaSymbol::Black,
            Color::Red => ManaSymbol::Red,
            Color::Green => ManaSymbol::Green,
        }
    }
}

/// Represents a mana cost as a sequence of pips, where each pip is a list of
/// alternative payment options (disjunction).
///
/// The outer vector represents pips that must ALL be paid (conjunction).
/// Each inner vector represents alternative ways to pay that pip (disjunction).
///
/// Examples:
/// - `{2}{W}{W}` = `[[Generic(2)], [White], [White]]`
/// - `{W/U}` (hybrid) = `[[White, Blue]]`
/// - `{2/W}` (twobrid) = `[[Generic(2), White]]`
/// - `{W/P}` (phyrexian) = `[[White, Life(2)]]`
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManaCost {
    pips: Vec<Vec<ManaSymbol>>,
}

impl ManaCost {
    /// Creates an empty mana cost.
    pub fn new() -> Self {
        Self { pips: Vec::new() }
    }

    /// Creates a mana cost from a list of pips, where each pip is a list of
    /// alternative payment options.
    pub fn from_pips(pips: Vec<Vec<ManaSymbol>>) -> Self {
        Self { pips }
    }

    /// Creates a mana cost from a simple list of symbols (each becomes one pip).
    pub fn from_symbols(symbols: Vec<ManaSymbol>) -> Self {
        Self {
            pips: symbols.into_iter().map(|s| vec![s]).collect(),
        }
    }

    /// Returns the mana value (formerly converted mana cost) of this cost.
    ///
    /// For each pip, uses the maximum mana value among its alternatives.
    pub fn mana_value(&self) -> u32 {
        self.pips
            .iter()
            .map(|pip| pip.iter().map(|s| s.mana_value()).max().unwrap_or(0))
            .sum()
    }

    /// Returns the pips in this mana cost.
    pub fn pips(&self) -> &[Vec<ManaSymbol>] {
        &self.pips
    }

    /// Format the mana cost in oracle-style syntax (e.g., "{2}{W}{W}").
    pub fn to_oracle(&self) -> String {
        fn symbol_text(symbol: ManaSymbol) -> String {
            match symbol {
                ManaSymbol::White => "W".to_string(),
                ManaSymbol::Blue => "U".to_string(),
                ManaSymbol::Black => "B".to_string(),
                ManaSymbol::Red => "R".to_string(),
                ManaSymbol::Green => "G".to_string(),
                ManaSymbol::Colorless => "C".to_string(),
                ManaSymbol::Generic(n) => n.to_string(),
                ManaSymbol::Snow => "S".to_string(),
                ManaSymbol::Life(_) => "P".to_string(),
                ManaSymbol::X => "X".to_string(),
            }
        }

        let mut out = String::new();
        for pip in &self.pips {
            let mut parts = Vec::new();
            for symbol in pip {
                parts.push(symbol_text(*symbol));
            }
            out.push('{');
            out.push_str(&parts.join("/"));
            out.push('}');
        }
        out
    }

    /// Adds a pip with a single payment option.
    pub fn push(&mut self, symbol: ManaSymbol) {
        self.pips.push(vec![symbol]);
    }

    /// Adds a pip with multiple alternative payment options.
    pub fn push_alternatives(&mut self, alternatives: Vec<ManaSymbol>) {
        self.pips.push(alternatives);
    }

    /// Returns true if this mana cost is empty (costs nothing).
    pub fn is_empty(&self) -> bool {
        self.pips.is_empty()
    }

    /// Returns the number of pips in this mana cost.
    pub fn pip_count(&self) -> usize {
        self.pips.len()
    }

    /// Returns true if this mana cost contains X.
    pub fn has_x(&self) -> bool {
        self.pips
            .iter()
            .any(|pip| pip.iter().any(|s| matches!(s, ManaSymbol::X)))
    }

    /// Returns the total generic mana cost (sum of all Generic(n) pips).
    pub fn generic_mana_total(&self) -> u32 {
        self.pips
            .iter()
            .filter_map(|pip| {
                // Only count pips where the only option is Generic
                if pip.len() == 1
                    && let ManaSymbol::Generic(n) = pip[0]
                {
                    return Some(n as u32);
                }
                None
            })
            .sum()
    }

    /// Returns a new ManaCost with generic mana reduced by the given amount.
    /// Reduction cannot make generic costs negative.
    ///
    /// This is used for abilities like Affinity that reduce generic mana costs.
    pub fn reduce_generic(&self, reduction: u32) -> ManaCost {
        let mut remaining_reduction = reduction;
        let mut new_pips = Vec::new();

        for pip in &self.pips {
            // Check if this is a pure Generic pip (single option that is Generic)
            if pip.len() == 1
                && let ManaSymbol::Generic(n) = pip[0]
            {
                let current = n as u32;
                if remaining_reduction >= current {
                    // This pip is fully reduced away
                    remaining_reduction -= current;
                    continue; // Skip this pip entirely
                } else {
                    // Partially reduce this pip
                    let new_generic = current - remaining_reduction;
                    remaining_reduction = 0;
                    if new_generic > 0 {
                        new_pips.push(vec![ManaSymbol::Generic(new_generic as u8)]);
                    }
                    continue;
                }
            }
            // Not a pure Generic pip, keep it as-is
            new_pips.push(pip.clone());
        }

        ManaCost::from_pips(new_pips)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mana_symbol_value() {
        assert_eq!(ManaSymbol::White.mana_value(), 1);
        assert_eq!(ManaSymbol::Generic(3).mana_value(), 3);
        assert_eq!(ManaSymbol::X.mana_value(), 0);
        assert_eq!(ManaSymbol::Life(2).mana_value(), 0);
        assert_eq!(ManaSymbol::Colorless.mana_value(), 1);
        assert_eq!(ManaSymbol::Snow.mana_value(), 1);
    }

    #[test]
    fn test_mana_cost_empty() {
        let cost = ManaCost::new();
        assert!(cost.is_empty());
        assert_eq!(cost.mana_value(), 0);
    }

    #[test]
    fn test_mana_cost_simple() {
        // {2}{W}{W} like Serra Angel
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]);
        assert_eq!(cost.mana_value(), 4);
        assert_eq!(cost.pip_count(), 3);
    }

    #[test]
    fn test_mana_cost_with_x() {
        // {X}{R}{R} like Fireball
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]);
        assert_eq!(cost.mana_value(), 2); // X counts as 0
    }

    #[test]
    fn test_mana_cost_hybrid() {
        // {2}{W/U}{W/U} like some Ravnica cards
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White, ManaSymbol::Blue],
            vec![ManaSymbol::White, ManaSymbol::Blue],
        ]);
        assert_eq!(cost.mana_value(), 4); // max(1,1) = 1 for each hybrid pip
    }

    #[test]
    fn test_mana_cost_twobrid() {
        // {2/W}{2/W}{2/W} like Spectral Procession
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2), ManaSymbol::White],
            vec![ManaSymbol::Generic(2), ManaSymbol::White],
            vec![ManaSymbol::Generic(2), ManaSymbol::White],
        ]);
        assert_eq!(cost.mana_value(), 6); // max(2,1) = 2 for each twobrid pip
    }

    #[test]
    fn test_mana_cost_phyrexian() {
        // {1}{W/P}{W/P} like Porcelain Legionnaire
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White, ManaSymbol::Life(2)],
            vec![ManaSymbol::White, ManaSymbol::Life(2)],
        ]);
        assert_eq!(cost.mana_value(), 3); // max(1,0) = 1 for each phyrexian pip
    }

    #[test]
    fn test_mana_cost_phyrexian_hybrid() {
        // {G/U/P} like some cards from All Will Be One
        let cost = ManaCost::from_pips(vec![vec![
            ManaSymbol::Green,
            ManaSymbol::Blue,
            ManaSymbol::Life(2),
        ]]);
        assert_eq!(cost.mana_value(), 1); // max(1,1,0) = 1
    }

    #[test]
    fn test_mana_cost_push() {
        let mut cost = ManaCost::new();
        cost.push(ManaSymbol::Generic(2));
        cost.push(ManaSymbol::Green);
        cost.push(ManaSymbol::Green);
        assert_eq!(cost.mana_value(), 4);
        assert_eq!(cost.pip_count(), 3);
    }

    #[test]
    fn test_mana_cost_push_alternatives() {
        let mut cost = ManaCost::new();
        cost.push(ManaSymbol::Generic(1));
        cost.push_alternatives(vec![ManaSymbol::White, ManaSymbol::Blue]);
        assert_eq!(cost.mana_value(), 2);
        assert_eq!(cost.pip_count(), 2);
    }

    #[test]
    fn test_mana_cost_colorless_eldrazi() {
        // {3}{C} like Thought-Knot Seer
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Colorless],
        ]);
        assert_eq!(cost.mana_value(), 4);
    }

    #[test]
    fn test_generic_mana_total() {
        // {4} - pure generic
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]);
        assert_eq!(cost.generic_mana_total(), 4);

        // {2}{W}{W} - 2 generic plus 2 colored
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]);
        assert_eq!(cost.generic_mana_total(), 2);

        // {R}{R} - no generic
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Red], vec![ManaSymbol::Red]]);
        assert_eq!(cost.generic_mana_total(), 0);

        // {2/W} hybrid - not pure generic
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2), ManaSymbol::White]]);
        assert_eq!(cost.generic_mana_total(), 0);
    }

    #[test]
    fn test_reduce_generic() {
        // {4} with 3 reduction = {1}
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]);
        let reduced = cost.reduce_generic(3);
        assert_eq!(reduced.mana_value(), 1);
        assert_eq!(reduced.generic_mana_total(), 1);

        // {4} with 4 reduction = free (empty)
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]);
        let reduced = cost.reduce_generic(4);
        assert!(reduced.is_empty());
        assert_eq!(reduced.mana_value(), 0);

        // {4} with 5 reduction = free (capped at 0)
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]);
        let reduced = cost.reduce_generic(5);
        assert!(reduced.is_empty());
        assert_eq!(reduced.mana_value(), 0);

        // {2}{W}{W} with 1 reduction = {1}{W}{W}
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]);
        let reduced = cost.reduce_generic(1);
        assert_eq!(reduced.mana_value(), 3);
        assert_eq!(reduced.pip_count(), 3);

        // {2}{W}{W} with 2 reduction = {W}{W}
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]);
        let reduced = cost.reduce_generic(2);
        assert_eq!(reduced.mana_value(), 2);
        assert_eq!(reduced.pip_count(), 2);

        // {R}{R} with any reduction = {R}{R} (no generic to reduce)
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Red], vec![ManaSymbol::Red]]);
        let reduced = cost.reduce_generic(5);
        assert_eq!(reduced.mana_value(), 2);
        assert_eq!(reduced.pip_count(), 2);

        // {2/W} hybrid with reduction - should not be affected
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2), ManaSymbol::White]]);
        let reduced = cost.reduce_generic(2);
        // Hybrid pip is not pure Generic, so it's kept
        assert_eq!(reduced.pip_count(), 1);
        assert_eq!(reduced.mana_value(), 2);
    }
}
