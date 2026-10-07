//! Waterbend is an obligation on part of a price, not a property of every
//! generic pip later added to that price. Keep it through pricing and replay.
use super::{ManaCost, ManaSymbol};
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, TagKeyWalk)]
pub struct WaterbendObligation {
    pub fixed: u32,
    pub x_symbols: u32,
    pub announced_x: Option<u32>,
}

impl WaterbendObligation {
    pub fn amount(&self, x: u32) -> Option<u32> {
        self.x_symbols.checked_mul(self.announced_x.unwrap_or(x))?.checked_add(self.fixed)
    }
}

/// An algebraic capacity survives combining independently reduced or X-bound
/// prices. A later ordinary generic addition cannot revive an extinguished
/// share, and an unbound right-hand X is not mistaken for zero during joining.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, TagKeyWalk)]
pub enum WaterbendCapacity {
    Amount(WaterbendObligation),
    Sum(Box<WaterbendCapacity>, Box<WaterbendCapacity>),
    Minimum(Box<WaterbendCapacity>, WaterbendObligation),
}
impl WaterbendCapacity {
    fn amount(&self, x: u32) -> Option<u32> {
        match self {
            Self::Amount(amount) => amount.amount(x),
            Self::Sum(a, b) => a.amount(x)?.checked_add(b.amount(x)?),
            Self::Minimum(capacity, amount) => Some(capacity.amount(x)?.min(amount.amount(x)?)),
        }
    }
    pub(super) fn bind(&mut self, x: u32) {
        match self {
            Self::Amount(amount) => amount.announced_x = Some(x),
            Self::Sum(a, b) => { a.bind(x); b.bind(x); },
            Self::Minimum(capacity, amount) => { capacity.bind(x); amount.announced_x = Some(x); },
        }
    }
    pub(super) fn bind_if_unbound(&mut self, x: u32) {
        match self {
            Self::Amount(amount) => { amount.announced_x.get_or_insert(x); },
            Self::Sum(a, b) => { a.bind_if_unbound(x); b.bind_if_unbound(x); },
            Self::Minimum(capacity, amount) => { capacity.bind_if_unbound(x); amount.announced_x.get_or_insert(x); },
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, TagKeyWalk)]
pub struct WaterbendPaymentScope {
    pub obligations: Vec<WaterbendObligation>,
    pub capacity: WaterbendCapacity,
    /// The declaration currently attached to still-unexpanded X pips. Original
    /// obligations may already have different values after component joining.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub x_pip_binding: Option<u32>,
}

impl ManaCost {
    /// Mark this generic/X component before combining it with other costs.
    pub fn with_waterbend(mut self) -> Self {
        let amount = WaterbendObligation {
            fixed: self.generic_mana_total(),
            x_symbols: self.pips.iter().filter(|pip| pip.as_slice() == [ManaSymbol::X]).count() as u32,
            announced_x: None,
        };
        self.waterbend_payment_scope = Some(WaterbendPaymentScope {
            obligations: vec![amount.clone()], capacity: WaterbendCapacity::Amount(amount), x_pip_binding: None,
        });
        self
    }

    /// A typed cost surface. The ordinary mana remainder is kept separate from
    /// each Waterbend component, including mixed `{T}` activation costs.
    pub fn payment_surface(&self) -> String {
        let Some(scope) = &self.waterbend_payment_scope else { return self.to_oracle(); };
        let mut remainder = self.pips_with_bound_x_expanded();
        let mut parts = Vec::new();
        for obligation in &scope.obligations {
            let mut fixed = obligation.announced_x.and_then(|x| obligation.amount(x)).unwrap_or(obligation.fixed);
            let mut x = if obligation.announced_x.is_some() { 0 } else { obligation.x_symbols };
            for pip in &mut remainder {
                match pip.as_mut_slice() {
                    [ManaSymbol::Generic(amount)] => {
                        let removed = fixed.min(u32::from(*amount));
                        *amount -= removed as u8; fixed -= removed;
                    },
                    [ManaSymbol::X] if x > 0 => { pip.clear(); x -= 1; },
                    _ => {},
                }
            }
            let mut amount = ManaCost::new().add_generic(
                obligation.announced_x.and_then(|x| obligation.amount(x)).unwrap_or(obligation.fixed));
            if obligation.announced_x.is_none() {
                for _ in 0..obligation.x_symbols { amount.push(ManaSymbol::X); }
            }
            parts.push(format!("Waterbend {}", if amount.pips().is_empty() { "{0}".into() } else { amount.to_oracle() }));
        }
        remainder.retain(|pip| !pip.is_empty() && pip.as_slice() != [ManaSymbol::Generic(0)]);
        if !remainder.is_empty() { parts.push(ManaCost::from_pips(remainder).to_oracle()); }
        parts.join(", ")
    }

    pub fn waterbend_payment_scope(&self) -> Option<&WaterbendPaymentScope> {
        self.waterbend_payment_scope.as_ref()
    }

    pub fn has_waterbend_obligation(&self) -> bool {
        self.waterbend_payment_scope.is_some()
    }

    /// Convenience for already validated prices. Checked payment boundaries
    /// use waterbend_capacity_checked so overflow is incomplete evidence.
    pub fn waterbend_capacity(&self, x: u32) -> u32 {
        self.waterbend_capacity_checked(x).unwrap_or(0)
    }

    pub fn waterbend_capacity_checked(&self, x: u32) -> Option<u32> {
        let Some(scope) = &self.waterbend_payment_scope else { return Some(0); };
        if scope.obligations.is_empty() { return None; }
        let original = scope.obligations.iter().try_fold(0u32, |total, part| total.checked_add(part.amount(x)?))?;
        Some(scope.capacity.amount(x)?.min(original).min(self.generic_payment_amount(x)))
    }

    /// A still-unexpanded bound component keeps its own declaration when a
    /// public payer has no new X choice. Ordinary costs retain caller X.
    pub fn payment_x_value(&self, fallback: u32) -> u32 {
        self.waterbend_payment_scope.as_ref().and_then(|scope| scope.x_pip_binding).unwrap_or(fallback)
    }

    pub(super) fn generic_payment_amount(&self, x: u32) -> u32 {
        let x = self.payment_x_value(x);
        self.pips.iter().filter_map(|pip| match pip.as_slice() {
            [ManaSymbol::Generic(amount)] => Some(u32::from(*amount)),
            [ManaSymbol::X] => Some(x),
            _ => None,
        }).fold(0, u32::saturating_add)
    }

    pub(super) fn reprice_waterbend_from(&mut self, previous: &Self) {
        let Some(scope) = self.waterbend_payment_scope.as_mut() else { return; };
        let old_x = previous.pips.iter().filter(|pip| pip.contains(&ManaSymbol::X)).count();
        let new_x = self.pips.iter().filter(|pip| pip.contains(&ManaSymbol::X)).count();
        if new_x == 0 || new_x > old_x { scope.x_pip_binding = None; }
        let amount = WaterbendObligation {
            fixed: self.pips.iter().filter_map(|pip| match pip.as_slice() {
                [ManaSymbol::Generic(amount)] => Some(u32::from(*amount)), _ => None,
            }).fold(0, u32::saturating_add),
            x_symbols: self.pips.iter().filter(|pip| pip.as_slice() == [ManaSymbol::X]).count() as u32,
            announced_x: scope.x_pip_binding,
        };
        scope.capacity = WaterbendCapacity::Minimum(Box::new(scope.capacity.clone()), amount);
    }

    /// Expand each independently declared component before joining it to
    /// another price. One later request X cannot stand for two declarations.
    pub(super) fn pips_with_bound_x_expanded(&self) -> Vec<Vec<ManaSymbol>> {
        let binding = match &self.waterbend_payment_scope {
            Some(scope) => scope.x_pip_binding,
            None => self.x_payment_scope.as_ref().and_then(|scope| scope.announced_x),
        };
        let Some(x) = binding else { return self.pips.clone(); };
        let mut pips = Vec::new();
        for pip in &self.pips {
            if pip.as_slice() == [ManaSymbol::X] {
                let mut remaining = x;
                while remaining > 0 {
                    let amount = remaining.min(u8::MAX as u32) as u8;
                    pips.push(vec![ManaSymbol::Generic(amount)]);
                    remaining -= u32::from(amount);
                }
            } else { pips.push(pip.clone()); }
        }
        pips
    }

    pub(super) fn combine_waterbend_from(&mut self, left: &Self, right: &Self) {
        match (&left.waterbend_payment_scope, &right.waterbend_payment_scope) {
            (None, None) => {},
            (Some(scope), None) | (None, Some(scope)) => {
                let mut scope = scope.clone(); scope.x_pip_binding = None;
                self.waterbend_payment_scope = Some(scope);
            },
            (Some(a), Some(b)) => {
                let mut obligations = a.obligations.clone();
                obligations.extend(b.obligations.clone());
                self.waterbend_payment_scope = Some(WaterbendPaymentScope { obligations,
                    capacity: WaterbendCapacity::Sum(Box::new(a.capacity.clone()), Box::new(b.capacity.clone())),
                    x_pip_binding: None,
                });
            },
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waterbend_scope_survives_binding_reduction_and_ordinary_addition() {
        let cost = ManaCost::from_symbols(vec![ManaSymbol::X]).with_waterbend();
        let bound = cost.bind_x_payment_if_unbound(5)
            .with_pips(vec![vec![ManaSymbol::Generic(5)]]);
        assert_eq!(bound.add_generic(3).waterbend_capacity(0), 5);
        assert_eq!(bound.reduce_generic(5).add_generic(3).waterbend_capacity(0), 0);
        assert!(bound.reduce_generic(5).has_waterbend_obligation());
        assert!(!bound.reduce_generic(5).is_empty(), "zero still needs acceptance");
        assert_eq!(ManaCost::new().add_generic(3).combined_with(&bound).waterbend_capacity(0), 5);
    }
}

#[cfg(test)]
mod independent_x_contracts {
    use super::*;
    fn x_cost() -> ManaCost { ManaCost::from_symbols(vec![ManaSymbol::X]).with_waterbend() }
    #[test]
    fn differently_bound_components_expand_before_joining_under_a_new_request_x() {
        let left = x_cost().bind_x_payment_if_unbound(2);
        let right = x_cost().bind_x_payment_if_unbound(3);
        let combined = left.combined_with(&right);
        assert!(!combined.has_x());
        assert_eq!(combined.generic_mana_total(), 5);
        assert_eq!(combined.waterbend_capacity_checked(0), Some(5));
        assert_eq!(combined.waterbend_payment_scope().unwrap().obligations.iter()
            .map(|obligation| obligation.amount(0).unwrap()).collect::<Vec<_>>(), [2, 3]);
    }
    #[test]
    fn bound_plus_unbound_right_x_remains_distinct_through_later_binding_and_joining() {
        let bound = x_cost().bind_x_payment_if_unbound(2);
        for mixed in [bound.combined_with(&x_cost()), x_cost().combined_with(&bound)] {
            assert!(mixed.has_x());
            assert_eq!(mixed.generic_mana_total(), 2);
            assert_eq!(mixed.waterbend_capacity_checked(3), Some(5));
            let later = mixed.bind_x_payment_if_unbound(3).combined_with(&ManaCost::new().add_generic(4));
            assert!(!later.has_x());
            assert_eq!(later.generic_mana_total(), 9);
            assert_eq!(later.waterbend_capacity_checked(0), Some(5));
            assert_eq!(later.reduce_generic(9).add_generic(2).waterbend_capacity_checked(0), Some(0));
        }
    }
}

#[cfg(test)]
mod rebinding_contracts {
    use super::*;
    #[test]
    fn explicit_binding_replaces_a_declaration_while_payment_binding_preserves_it() {
        let cost = ManaCost::from_symbols(vec![ManaSymbol::X]).with_waterbend().bind_x_payment(2);
        assert_eq!(cost.clone().bind_x_payment_if_unbound(0).payment_x_value(0), 2);
        let rebound = cost.bind_x_payment(3);
        assert_eq!(rebound.payment_x_value(0), 3);
        assert_eq!(rebound.waterbend_capacity_checked(0), Some(3));
        assert_eq!(rebound.waterbend_payment_scope().unwrap().obligations[0].amount(0), Some(3));
        assert_eq!(rebound.combined_with(&ManaCost::new()).generic_mana_total(), 3);
    }
}

#[cfg(test)]
mod constrained_x_coexistence_contracts {
    use super::*;
    #[test]
    fn differently_bound_waterbend_and_constrained_x_preserve_the_existing_conflict_policy() {
        let waterbend = ManaCost::from_symbols(vec![ManaSymbol::X]).with_waterbend().bind_x_payment(2);
        let restricted = ManaCost::from_symbols(vec![ManaSymbol::X]).with_spending_restriction(
            super::super::ManaSpendingRestriction::OnX {
                colors: crate::color::ColorSet::BLACK, maximum_per_color: None,
            }).bind_x_payment(3);
        for joined in [waterbend.combined_with(&restricted), restricted.combined_with(&waterbend)] {
            assert_eq!(joined.generic_mana_total(), 5);
            assert_eq!(joined.waterbend_capacity_checked(0), Some(2));
            assert!(joined.allocate_mana_to_x(&[ManaSymbol::Black; 5], 0).is_none(),
                "an independently declared X=2 cannot be relabeled as the other component's X=3");
        }
    }
}
