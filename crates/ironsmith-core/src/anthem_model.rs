use crate::{CounterType, ManaSymbol, ObjectFilter, PlayerFilter, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum AnthemCountExpression {
    MatchingFilter(ObjectFilter),
    GreatestManaValueAmong(ObjectFilter),
    AttachedToSource(ObjectFilter),
    AttachedToAffected(ObjectFilter),
    ColorsOfAffected,
    AffectedAttackedThisTurn,
    CountersOnSource(CounterType),
    CountersAmong(ObjectFilter, CounterType),
    BasicLandTypesAmong(ObjectFilter),
    CreatureTypesAmong(ObjectFilter),
    CommanderCastCount(PlayerFilter),
    PlayerSpeed(PlayerFilter),
    UnspentMana {
        player: PlayerFilter,
        symbol: ManaSymbol,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnthemValue {
    Fixed(i32),
    Dynamic(Value),
    PerCount {
        multiplier: i32,
        count: AnthemCountExpression,
    },
}

impl AnthemValue {
    pub fn scaled(multiplier: i32, count: AnthemCountExpression) -> Self {
        if multiplier == 0 {
            Self::Fixed(0)
        } else {
            Self::PerCount { multiplier, count }
        }
    }

    pub fn uses_affected_object(&self) -> bool {
        matches!(
            self,
            Self::PerCount {
                count: AnthemCountExpression::AttachedToAffected(_)
                    | AnthemCountExpression::ColorsOfAffected
                    | AnthemCountExpression::AffectedAttackedThisTurn,
                ..
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{AnthemCountExpression, AnthemValue};
    use crate::ObjectFilter;

    #[test]
    fn anthem_value_scaled_collapses_zero() {
        assert_eq!(
            AnthemValue::scaled(
                0,
                AnthemCountExpression::MatchingFilter(ObjectFilter::creature())
            ),
            AnthemValue::Fixed(0)
        );
    }

    #[test]
    fn anthem_value_reports_affected_object_dependency() {
        assert!(
            AnthemValue::scaled(
                1,
                AnthemCountExpression::AttachedToAffected(ObjectFilter::artifact())
            )
            .uses_affected_object()
        );
    }
}
