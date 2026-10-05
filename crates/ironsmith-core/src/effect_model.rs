/// Comparison operations for numeric values.
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub enum Comparison {
    GreaterThan(i32),
    GreaterThanOrEqual(i32),
    Equal(i32),
    OneOf(crate::InternedI32Slice),
    LessThan(i32),
    LessThanOrEqual(i32),
    NotEqual(i32),
    BetweenInclusive(i32, i32),
}

impl Comparison {
    /// Compare a resolved count without narrowing it to the printed bound type.
    pub fn evaluate_wide(&self, value: i64) -> bool {
        match self {
            Self::GreaterThan(n) => value > i64::from(*n),
            Self::GreaterThanOrEqual(n) => value >= i64::from(*n),
            Self::Equal(n) => value == i64::from(*n),
            Self::OneOf(values) => values.iter().any(|n| value == i64::from(*n)),
            Self::LessThan(n) => value < i64::from(*n),
            Self::LessThanOrEqual(n) => value <= i64::from(*n),
            Self::NotEqual(n) => value != i64::from(*n),
            Self::BetweenInclusive(min,max) => value >= i64::from(*min) && value <= i64::from(*max),
        }
    }
    pub fn evaluate(&self, value: i32) -> bool {
        match self {
            Self::GreaterThan(n) => value > *n,
            Self::GreaterThanOrEqual(n) => value >= *n,
            Self::Equal(n) => value == *n,
            Self::OneOf(values) => values.contains(&value),
            Self::LessThan(n) => value < *n,
            Self::LessThanOrEqual(n) => value <= *n,
            Self::NotEqual(n) => value != *n,
            Self::BetweenInclusive(min, max) => value >= *min && value <= *max,
        }
    }
}

/// Comparison operations between two runtime-resolved values.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum ValueComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    Equal,
    LessThan,
    LessThanOrEqual,
    NotEqual,
}

impl ValueComparisonOperator {
    pub fn evaluate(self, left: i32, right: i32) -> bool {
        match self {
            Self::GreaterThan => left > right,
            Self::GreaterThanOrEqual => left >= right,
            Self::Equal => left == right,
            Self::LessThan => left < right,
            Self::LessThanOrEqual => left <= right,
            Self::NotEqual => left != right,
        }
    }
}

/// A quantity owned by one completed cast, never by a later stack object or
/// the amount rewritten by a replacement instruction.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum CastEventQuantity {
    ManaValue,
    ManaSymbols(crate::color::Color),
    DistinctTargets,
}

/// Event payload fields that can be referenced by effect values.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub enum EventValueSpec {
    Amount,
    LifeAmount,
    BlockersBeyondFirst { multiplier: i32 },
    DieResult,
    /// Captured life gained/lost by the event's affected player. The optional
    /// controller restriction preserves an authored "you" participant.
    LifeChange { gained: bool, for_controller: bool },
    /// Sum only the retained numeric results of this player's roll batch.
    DieBatchTotal,
    /// Number of this batch's numeric results satisfying the authored predicate.
    DieResultsAtLeast(i32),
    CastSpell(CastEventQuantity),
}

#[cfg(test)]
mod tests {
    use super::{Comparison, ValueComparisonOperator};

    #[test]
    fn comparison_evaluates_values() {
        assert!(Comparison::GreaterThan(2).evaluate(3));
        assert!(Comparison::BetweenInclusive(2, 4).evaluate(4));
        assert!(Comparison::OneOf((&[1, 3, 5][..]).into()).evaluate(3));
        assert!(!Comparison::LessThanOrEqual(1).evaluate(2));
    }

    #[test]
    fn value_operator_evaluates_values() {
        assert!(ValueComparisonOperator::GreaterThan.evaluate(5, 4));
        assert!(ValueComparisonOperator::Equal.evaluate(7, 7));
        assert!(!ValueComparisonOperator::NotEqual.evaluate(3, 3));
    }
}
