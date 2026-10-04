//! The generic part of a price denoted by X remains generic (CR 107.4b).
//! Spending restrictions constrain an allocation of actual mana to that part,
//! not the colors with which the whole price can be paid.
use super::{ManaCost, ManaSpendingRestriction, ManaSymbol};
use crate::{
    color::{Color, ColorSet},
    tag::TagKeyWalk,
};

/// Actual colored mana allocated to X, in W/U/B/R/G order. Fixed colored
/// symbols and ordinary generic costs are never included in this receipt.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, TagKeyWalk)]
pub struct XManaAllocation(pub [u32; 5]);
impl XManaAllocation {
    pub fn total(self) -> u32 {
        self.0.iter().copied().fold(0, u32::saturating_add)
    }
    pub fn of_color(self, color: Color) -> u32 {
        self.0[color_index(color)]
    }
}

pub const fn color_index(color: Color) -> usize {
    match color {
        Color::White => 0,
        Color::Blue => 1,
        Color::Black => 2,
        Color::Red => 3,
        Color::Green => 4,
    }
}
pub fn actual_color(symbol: ManaSymbol) -> Option<Color> {
    match symbol {
        ManaSymbol::White => Some(Color::White),
        ManaSymbol::Blue => Some(Color::Blue),
        ManaSymbol::Black => Some(Color::Black),
        ManaSymbol::Red => Some(Color::Red),
        ManaSymbol::Green => Some(Color::Green),
        _ => None,
    }
}

/// Exact actual mana used by a server-proved assignment (W/U/B/R/G/C).
/// Used when one player's payment must leave another player's continuation
/// payable. It never describes mana as though it were another color.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, TagKeyWalk)]
pub struct ActualManaAllocation(pub [u32; 6]);
impl ActualManaAllocation {
    pub fn from_symbols(symbols: impl IntoIterator<Item = ManaSymbol>) -> Option<Self> {
        let mut result = Self::default();
        for symbol in symbols {
            let index = if symbol == ManaSymbol::Colorless {
                5
            } else {
                color_index(actual_color(symbol)?)
            };
            result.0[index] = result.0[index].checked_add(1)?;
        }
        Some(result)
    }
    pub fn symbols(self) -> Vec<ManaSymbol> {
        let mut result = Vec::new();
        for (index, symbol) in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ]
        .into_iter()
        .enumerate()
        {
            result.extend(std::iter::repeat_n(symbol, self.0[index] as usize));
        }
        result
    }
}

/// Price provenance, retained while generic pips are coalesced, reduced, or
/// satisfied without spending mana. `ordinary_generic` is the capacity of
/// unrestricted generic obligations BEFORE reductions. This represents all
/// legal allocations of a generic reduction, instead of arbitrarily assigning
/// the reduction to X or to the base cost.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, TagKeyWalk)]
pub struct XPaymentScope {
    pub(crate) symbols: u32,
    pub(crate) announced_x: Option<u32>,
    pub(crate) ordinary_generic: u32,
    /// Assist is actual mana spent, not a discount. Its real colors join the
    /// final payer's generic payment before the allocation is checked.
    pub(crate) prepaid_generic: Vec<ManaSymbol>,
    /// An optional player-selected allocation. Validation, not the client,
    /// proves that these counts can be assigned to actual generic payments.
    pub(crate) required: Option<XManaAllocation>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "scope_is_valid")
    )]
    pub(crate) incompatible: bool,
}
fn scope_is_valid(incompatible: &bool) -> bool {
    !*incompatible
}

impl ManaCost {
    /// Combine independently priced components without reconstructing X
    /// provenance from their already-expanded generic pips.
    pub fn combined_with(&self, other: &Self) -> Self {
        let mut pips = self.pips.clone();
        pips.extend(other.pips.iter().cloned());
        let mut result = Self::from_pips(pips);
        for rule in self
            .spending_restrictions()
            .iter()
            .chain(other.spending_restrictions())
        {
            result = result.with_spending_restriction(rule.clone());
        }
        if result.has_x_spending_restriction() {
            let scope = |cost: &Self| {
                cost.x_payment_scope
                    .clone()
                    .unwrap_or_else(|| XPaymentScope {
                        symbols: cost
                            .pips
                            .iter()
                            .filter(|pip| pip.contains(&ManaSymbol::X))
                            .count() as u32,
                        announced_x: None,
                        ordinary_generic: cost.generic_mana_total(),
                        prepaid_generic: Vec::new(),
                        required: None,
                        incompatible: cost.has_x_spending_restriction(),
                    })
            };
            let mut left = scope(self);
            let right = scope(other);
            let left_has_x = left.symbols > 0 && left.announced_x != Some(0);
            let right_has_x = right.symbols > 0 && right.announced_x != Some(0);
            left.incompatible |= right.incompatible
                || (left.symbols > 0
                    && right.symbols > 0
                    && matches!((left.announced_x, right.announced_x), (Some(a), Some(b)) if a != b));
            left.announced_x = match (left.symbols > 0, right.symbols > 0) {
                (true, true) => left.announced_x.or(right.announced_x),
                (true, false) => left.announced_x,
                (false, true) => right.announced_x,
                (false, false) => left.announced_x.or(right.announced_x),
            };
            left.incompatible |= left.symbols.checked_add(right.symbols).is_none()
                || left
                    .ordinary_generic
                    .checked_add(right.ordinary_generic)
                    .is_none();
            left.symbols = left.symbols.saturating_add(right.symbols);
            left.ordinary_generic = left.ordinary_generic.saturating_add(right.ordinary_generic);
            left.prepaid_generic.extend(right.prepaid_generic);
            left.required = match (left.required, right.required) {
                (Some(a), Some(b)) => {
                    let mut total = [0; 5];
                    for index in 0..5 {
                        left.incompatible |= a.0[index].checked_add(b.0[index]).is_none();
                        total[index] = a.0[index].saturating_add(b.0[index]);
                    }
                    Some(XManaAllocation(total))
                }
                (Some(value), None) => {
                    left.incompatible |= right_has_x;
                    Some(value)
                }
                (None, Some(value)) => {
                    left.incompatible |= left_has_x;
                    Some(value)
                }
                (None, None) => None,
            };
            result.x_payment_scope = Some(left);
        }
        result.required_actual_payment =
            match (self.required_actual_payment, other.required_actual_payment) {
                (Some(a), Some(b)) => Some(ActualManaAllocation(std::array::from_fn(|index| {
                    a.0[index].saturating_add(b.0[index])
                }))),
                (a, b) => a.or(b),
            };
        result
    }

    /// Rendering an ability's body separately from its typed cost rider.
    pub fn without_x_spending_restrictions(mut self) -> Self {
        self.spending_restrictions
            .retain(|rule| !matches!(rule, ManaSpendingRestriction::OnX { .. }));
        self.x_payment_scope = None;
        self
    }
    pub fn required_actual_payment(&self) -> Option<ActualManaAllocation> {
        self.required_actual_payment
    }
    pub fn with_required_actual_payment(
        mut self,
        allocation: Option<ActualManaAllocation>,
    ) -> Self {
        self.required_actual_payment = allocation;
        self
    }
    /// Assist's own price is generic. Whole-transaction producer conditions
    /// still apply to its mana, while its X allocation is proved jointly with
    /// the caster's continuation.
    pub fn inherit_transaction_spending_restrictions(mut self, other: &Self) -> Self {
        for rule in other.spending_restrictions() {
            if matches!(rule, ManaSpendingRestriction::ProducedBy(_)) {
                self = self.with_spending_restriction(rule.clone());
            }
        }
        self
    }
    pub fn x_payment_scope(&self) -> Option<&XPaymentScope> {
        self.x_payment_scope.as_ref()
    }
    pub fn has_x_spending_restriction(&self) -> bool {
        self.spending_restrictions
            .iter()
            .any(|rule| matches!(rule, ManaSpendingRestriction::OnX { .. }))
    }
    /// Bind before expanding X or reducing its generic price. Rebinding an
    /// already priced cost is deliberately not performed by a request's X=0.
    pub fn bind_x_payment(mut self, x: u32) -> Self {
        if let Some(scope) = self.x_payment_scope.as_mut() {
            scope.announced_x = Some(x);
        }
        self
    }
    pub fn bind_x_payment_if_unbound(self, x: u32) -> Self {
        if self.x_payment_is_bound() {
            self
        } else {
            self.bind_x_payment(x)
        }
    }
    pub fn required_x_allocation(&self) -> Option<XManaAllocation> {
        self.x_payment_scope
            .as_ref()
            .and_then(|scope| scope.required)
    }
    pub fn with_required_x_allocation(mut self, required: Option<XManaAllocation>) -> Self {
        if let Some(scope) = self.x_payment_scope.as_mut() {
            scope.required = required;
        }
        self
    }
    /// Remove the exact generic contribution paid by Assist without treating
    /// that payment as a generic reduction. All units supplied here must be
    /// receipts of the helper's actual successful payment.
    pub fn with_prepaid_generic(mut self, actual: Vec<ManaSymbol>) -> Self {
        if let Some(scope) = self.x_payment_scope.as_mut() {
            scope.prepaid_generic = actual;
        }
        self
    }
    pub fn x_payment_is_bound(&self) -> bool {
        self.x_payment_scope
            .as_ref()
            .is_none_or(|scope| scope.announced_x.is_some())
    }

    /// Sound pruning bound for a suffix containing only generic pips. Fixed
    /// colored payments have already consumed their units; unused units are
    /// an optimistic supply, so a negative answer is an actual impossibility.
    pub fn x_payment_can_complete_generic_suffix(
        &self,
        paid: &[ManaSymbol],
        unused: &[ManaSymbol],
        remaining: usize,
        x: u32,
    ) -> bool {
        let Some(scope) = self.x_payment_scope.as_ref() else {
            return !self.has_x_spending_restriction();
        };
        let total = paid
            .len()
            .saturating_add(remaining)
            .saturating_add(scope.prepaid_generic.len());
        let Ok(total) = u32::try_from(total) else {
            return false;
        };
        let Some(original_x) = scope.symbols.checked_mul(scope.announced_x.unwrap_or(x)) else {
            return false;
        };
        let minimum = total.saturating_sub(scope.ordinary_generic);
        let maximum = total.min(original_x);
        if minimum > maximum {
            return false;
        }
        let mut available = [0u32; 5];
        for symbol in paid.iter().chain(unused).chain(&scope.prepaid_generic) {
            if let Some(color) = actual_color(*symbol) {
                available[color_index(color)] += 1;
            }
        }
        for rule in self.spending_restrictions() {
            if let ManaSpendingRestriction::OnX {
                colors,
                maximum_per_color,
            } = rule
            {
                for color in Color::ALL {
                    let count = &mut available[color_index(color)];
                    *count = if colors.contains(color) {
                        (*count).min(maximum_per_color.unwrap_or(u32::MAX))
                    } else {
                        0
                    };
                }
            }
        }
        if let Some(required) = scope.required {
            return required.total() >= minimum
                && required.total() <= maximum
                && required
                    .0
                    .iter()
                    .zip(available)
                    .all(|(need, supply)| *need <= supply);
        }
        available.into_iter().sum::<u32>() >= minimum
    }

    /// Validate one completed per-pip assignment. The input contains ONLY mana
    /// units assigned to generic pips, using actual colors, not as-though ones.
    /// `Some(None)` is an unconstrained cost; `None` is an invalid assignment.
    pub fn allocate_mana_to_x(
        &self,
        actual_generic: &[ManaSymbol],
        x: u32,
    ) -> Option<Option<XManaAllocation>> {
        let Some(scope) = &self.x_payment_scope else {
            return (!self.has_x_spending_restriction()).then_some(None);
        };
        if scope.incompatible {
            return None;
        }
        let mut allowed = Color::ALL.into_iter().collect::<ColorSet>();
        let mut per_color = u32::MAX;
        for rule in &self.spending_restrictions {
            if let ManaSpendingRestriction::OnX {
                colors,
                maximum_per_color,
            } = rule
            {
                allowed = allowed.intersection(*colors);
                if let Some(limit) = maximum_per_color {
                    per_color = per_color.min(*limit);
                }
            }
        }
        let mut available = [0u32; 5];
        let mut total = 0u32;
        for symbol in actual_generic.iter().chain(scope.prepaid_generic.iter()) {
            // Only real mana units belong to this receipt.
            if !matches!(
                symbol,
                ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green
                    | ManaSymbol::Colorless
            ) {
                return None;
            }
            total = total.checked_add(1)?;
            if let Some(color) = actual_color(*symbol) {
                available[color_index(color)] += 1;
            }
        }
        let original_x = scope.symbols.checked_mul(scope.announced_x.unwrap_or(x))?;
        let minimum = total.saturating_sub(scope.ordinary_generic);
        let maximum = total.min(original_x);
        if minimum > maximum {
            return None;
        }
        for color in Color::ALL {
            let index = color_index(color);
            available[index] = if allowed.contains(color) {
                available[index].min(per_color)
            } else {
                0
            };
        }
        if let Some(required) = scope.required {
            let count = required.total();
            return (count >= minimum
                && count <= maximum
                && required
                    .0
                    .iter()
                    .zip(available)
                    .all(|(chosen, available)| *chosen <= available))
            .then_some(Some(required));
        }
        let mut count = available.iter().copied().sum::<u32>().min(maximum);
        if count < minimum {
            return None;
        }
        let mut selected = [0; 5];
        for (selected, available) in selected.iter_mut().zip(available) {
            *selected = available.min(count);
            count -= *selected;
        }
        Some(Some(XManaAllocation(selected)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn black_x() -> ManaCost {
        ManaCost::from_symbols(vec![
            ManaSymbol::X,
            ManaSymbol::Generic(1),
            ManaSymbol::Black,
        ])
        .with_spending_restriction(ManaSpendingRestriction::OnX {
            colors: ColorSet::BLACK,
            maximum_per_color: None,
        })
    }
    fn priced(cost: ManaCost, x: u32) -> ManaCost {
        let cost = cost.bind_x_payment(x);
        let mut pips = Vec::new();
        for pip in cost.pips() {
            if pip.as_slice() == [ManaSymbol::X] {
                for _ in 0..x {
                    pips.push(vec![ManaSymbol::Generic(1)]);
                }
            } else {
                pips.push(pip.clone());
            }
        }
        cost.with_pips(pips)
    }
    #[test]
    fn x_restrictions_leave_base_tax_and_fixed_colors_outside_the_allocation() {
        let cost = priced(black_x(), 2).add_generic(2);
        assert!(
            cost.allocate_mana_to_x(
                &[
                    ManaSymbol::Black,
                    ManaSymbol::Black,
                    ManaSymbol::Red,
                    ManaSymbol::Green,
                    ManaSymbol::Colorless
                ],
                0
            )
            .is_some()
        );
        assert!(
            cost.allocate_mana_to_x(
                &[
                    ManaSymbol::Black,
                    ManaSymbol::Red,
                    ManaSymbol::Red,
                    ManaSymbol::Green,
                    ManaSymbol::Colorless
                ],
                0
            )
            .is_none()
        );
    }
    #[test]
    fn generic_reduction_allocation_is_a_validated_choice() {
        let cost = priced(black_x(), 3).reduce_generic(2);
        let one = XManaAllocation([0, 0, 1, 0, 0]);
        let two = XManaAllocation([0, 0, 2, 0, 0]);
        for allocation in [one, two] {
            assert_eq!(
                cost.clone()
                    .with_required_x_allocation(Some(allocation))
                    .allocate_mana_to_x(&[ManaSymbol::Black, ManaSymbol::Black], 0),
                Some(Some(allocation))
            );
        }
        assert!(
            cost.clone()
                .with_required_x_allocation(Some(XManaAllocation::default()))
                .allocate_mana_to_x(&[ManaSymbol::Black, ManaSymbol::Black], 0)
                .is_none()
        );
        // One of the remaining generic obligations can be paid by an actual
        // helper unit. It does not discount the remaining X requirement.
        assert!(
            cost.clone()
                .with_prepaid_generic(vec![ManaSymbol::Red])
                .allocate_mana_to_x(&[ManaSymbol::Red], 0)
                .is_none()
        );
        assert!(
            cost.with_prepaid_generic(vec![ManaSymbol::Black])
                .allocate_mana_to_x(&[ManaSymbol::Red], 0)
                .is_some()
        );
    }
    #[test]
    fn per_color_cap_limits_actual_payment_not_announced_x() {
        let raw = ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]).with_spending_restriction(
            ManaSpendingRestriction::OnX {
                colors: Color::ALL.into_iter().collect(),
                maximum_per_color: Some(1),
            },
        );
        let mut with_kicker = raw.clone();
        with_kicker.push(ManaSymbol::X);
        let cost = priced(with_kicker, 7).reduce_generic(4);
        let five = Color::ALL.map(ManaSymbol::from_color);
        assert!(cost.allocate_mana_to_x(&five, 0).is_some());
        // A repeated color can pay the unrestricted base, but not a second X.
        let x_only = priced(
            ManaCost::from_symbols(vec![ManaSymbol::X]).with_spending_restriction(
                ManaSpendingRestriction::OnX {
                    colors: Color::ALL.into_iter().collect(),
                    maximum_per_color: Some(1),
                },
            ),
            2,
        );
        assert!(
            x_only
                .allocate_mana_to_x(&[ManaSymbol::Black, ManaSymbol::Black], 0)
                .is_none()
        );
        assert!(
            x_only
                .allocate_mana_to_x(&[ManaSymbol::Black, ManaSymbol::Red], 0)
                .is_some()
        );
        assert_eq!(
            priced(raw, 0).allocate_mana_to_x(&[ManaSymbol::Colorless; 2], 0),
            Some(Some(XManaAllocation::default()))
        );
    }
    #[test]
    fn generic_substitutions_and_zero_payment_do_not_invent_spent_mana() {
        let cost = priced(black_x(), 3);
        let residual = cost.with_pips(vec![vec![ManaSymbol::Black]]);
        assert_eq!(
            residual.allocate_mana_to_x(&[], 0),
            Some(Some(XManaAllocation::default()))
        );
        assert!(
            cost.allocate_mana_to_x(&[ManaSymbol::White; 4], 0)
                .is_none()
        );
    }
    #[test]
    fn combining_priced_components_preserves_right_side_x_and_both_generic_capacities() {
        let bound = priced(black_x(), 2);
        let joined = ManaCost::new().combined_with(&bound);
        assert_eq!(joined.x_payment_scope(), bound.x_payment_scope());
        assert!(
            joined
                .allocate_mana_to_x(&[ManaSymbol::Red; 3], 0)
                .is_none()
        );
        let taxed = ManaCost::new().add_generic(2).combined_with(&bound);
        assert!(
            taxed
                .allocate_mana_to_x(
                    &[
                        ManaSymbol::Black,
                        ManaSymbol::Black,
                        ManaSymbol::Red,
                        ManaSymbol::Red,
                        ManaSymbol::Red
                    ],
                    0
                )
                .is_some()
        );
        let repeated = bound.combined_with(&bound);
        assert!(
            repeated
                .allocate_mana_to_x(
                    &[
                        ManaSymbol::Black,
                        ManaSymbol::Black,
                        ManaSymbol::Black,
                        ManaSymbol::Black,
                        ManaSymbol::Red,
                        ManaSymbol::Red
                    ],
                    0
                )
                .is_some()
        );
        assert!(
            repeated
                .allocate_mana_to_x(
                    &[
                        ManaSymbol::Black,
                        ManaSymbol::Black,
                        ManaSymbol::Black,
                        ManaSymbol::Red,
                        ManaSymbol::Red,
                        ManaSymbol::Red
                    ],
                    0
                )
                .is_none()
        );
        assert!(
            bound
                .combined_with(&priced(black_x(), 3))
                .allocate_mana_to_x(&[ManaSymbol::Black; 7], 0)
                .is_none()
        );
    }
}
