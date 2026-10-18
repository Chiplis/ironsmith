//! A retained physical mana multiset, independent of spending conversions.
use super::{ManaCost, ManaSymbol};
use crate::tag::TagKeyWalk;
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, TagKeyWalk)]
pub struct ManaPaymentBudget {
    /// W, U, B, R, G, C units actually spent.
    pub units: [u32; 6],
    /// Snow-produced subset of each corresponding actual mana type.
    pub snow: [u32; 6],
    /// An authored subcost (such as X) may use only this many of the paid units.
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_usable_units: Option<u32>,
}
impl ManaPaymentBudget {
    /// Is some subset sufficient? X is zero in a card outside the stack;
    /// life and spending-as-though alternatives are not physical mana.
    pub fn can_pay(&self, cost: &ManaCost) -> bool {
        fn visit(pips: &[Vec<ManaSymbol>], budget: ManaPaymentBudget, generic: u32, remaining_limit: u32) -> bool {
            let Some((pip, tail)) = pips.split_first() else { return budget.units.iter().map(|units| u64::from(*units)).sum::<u64>() >= u64::from(generic) && generic <= remaining_limit; };
            for symbol in pip {
                match *symbol {
                    ManaSymbol::Life(_) => {},
                    ManaSymbol::X => { if visit(tail, budget.clone(), generic, remaining_limit) { return true; } },
                    ManaSymbol::Generic(amount) => { if visit(tail, budget.clone(), generic.saturating_add(u32::from(amount)), remaining_limit) { return true; } },
                    _ => {
                        let types: &[usize] = match symbol {
                            ManaSymbol::White => &[0], ManaSymbol::Blue => &[1], ManaSymbol::Black => &[2],
                            ManaSymbol::Red => &[3], ManaSymbol::Green => &[4], ManaSymbol::Colorless => &[5],
                            ManaSymbol::Snow => &[0, 1, 2, 3, 4, 5], _ => &[],
                        };
                        for &index in types {
                            // Enumerate snow and non-snow units independently:
                            // consuming a colored pip must preserve the option
                            // of saving a snow-produced unit for a later {S}.
                            for snowy in [false, true] {
                                if matches!(symbol, ManaSymbol::Snow) && !snowy { continue; }
                                let available = if snowy { budget.snow[index] } else { budget.units[index].saturating_sub(budget.snow[index]) };
                                if available == 0 || remaining_limit == 0 { continue; }
                                let mut remaining = budget.clone(); remaining.units[index] -= 1;
                                if snowy { remaining.snow[index] -= 1; }
                                if visit(tail, remaining, generic, remaining_limit - 1) { return true; }
                            }
                        }
                    }
                }
            }
            false
        }
        if self.snow.iter().zip(self.units).any(|(snow, units)| *snow > units) { return false; }
        visit(cost.pips(), self.clone(), 0, self.max_usable_units.unwrap_or(u32::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mana_subsets_preserve_colors_hybrid_snow_and_mana_only_phyrexian() {
        let budget = ManaPaymentBudget { units: [1, 0, 0, 0, 1, 0], snow: [1, 0, 0, 0, 0, 0], max_usable_units: None };
        let cost = |pips| ManaCost::from_pips(pips);
        assert!(budget.can_pay(&cost(vec![vec![ManaSymbol::Green], vec![ManaSymbol::Snow]])));
        assert!(!budget.can_pay(&cost(vec![vec![ManaSymbol::White], vec![ManaSymbol::Snow]])));
        assert!(budget.can_pay(&cost(vec![vec![ManaSymbol::White, ManaSymbol::Green], vec![ManaSymbol::White]])));
        assert!(budget.can_pay(&cost(vec![vec![ManaSymbol::Generic(1)]])));
        assert!(!budget.can_pay(&cost(vec![vec![ManaSymbol::Blue, ManaSymbol::Life(2)]])));
        assert!(!budget.can_pay(&cost(vec![vec![ManaSymbol::Generic(3)]])));
        let mut scoped = budget.clone(); scoped.max_usable_units = Some(1);
        assert!(!scoped.can_pay(&cost(vec![vec![ManaSymbol::White], vec![ManaSymbol::Green]])));
        assert!(scoped.can_pay(&cost(vec![vec![ManaSymbol::White]])));
        scoped.max_usable_units = Some(0);
        assert!(!scoped.can_pay(&cost(vec![vec![ManaSymbol::White]])));
    }
}
