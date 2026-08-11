use crate::{
    CounterType, KeywordActionKind, ManaSymbol, ObjectFilter, ObjectRef, PlayerFilter,
    SourceReferenceSurface, Value,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCounterPronounSurface {
    Him,
    Her,
}

impl SourceCounterPronounSurface {
    pub const fn object_pronoun(self) -> &'static str {
        match self {
            Self::Him => "him",
            Self::Her => "her",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnthemCountExpression {
    MatchingFilter(ObjectFilter),
    GreatestManaValueAmong(ObjectFilter),
    AttachedToSource(ObjectFilter),
    AttachedToAffected(ObjectFilter),
    ColorsOfAffected,
    AffectedAttackedThisTurn,
    CountersOnSource(CounterType),
    CountersOnSourceWithSurface {
        counter_type: CounterType,
        surface: SourceReferenceSurface,
    },
    CountersOnSourceWithPronoun {
        counter_type: CounterType,
        pronoun: SourceCounterPronounSurface,
    },
    StickersOnSource {
        action: KeywordActionKind,
        surface: Option<SourceReferenceSurface>,
        max_name_letters: Option<u32>,
    },
    CountersOnAffected(CounterType),
    CountersAmong(ObjectFilter, CounterType),
    DistinctCounterTypesAmong(ObjectFilter),
    BasicLandTypesAmong(ObjectFilter),
    CreatureTypesAmong(ObjectFilter),
    BlockingSource,
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
    /// A count-scaled modifier with an authored upper bound, as in
    /// "gets +1/+1 for each ... , to a maximum of 10."
    CappedPerCount {
        multiplier: i32,
        count: AnthemCountExpression,
        maximum: i32,
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

    pub fn scaled_capped(multiplier: i32, count: AnthemCountExpression, maximum: i32) -> Self {
        if multiplier == 0 {
            Self::Fixed(0)
        } else {
            Self::CappedPerCount {
                multiplier,
                count,
                maximum,
            }
        }
    }

    pub fn uses_affected_object(&self) -> bool {
        match self {
            Self::PerCount {
                count:
                    AnthemCountExpression::AttachedToAffected(_)
                    | AnthemCountExpression::ColorsOfAffected
                    | AnthemCountExpression::AffectedAttackedThisTurn
                    | AnthemCountExpression::CountersOnAffected(_),
                ..
            }
            | Self::CappedPerCount {
                count:
                    AnthemCountExpression::AttachedToAffected(_)
                    | AnthemCountExpression::ColorsOfAffected
                    | AnthemCountExpression::AffectedAttackedThisTurn
                    | AnthemCountExpression::CountersOnAffected(_),
                ..
            } => true,
            Self::PerCount {
                count: AnthemCountExpression::MatchingFilter(filter),
                ..
            } => matches!(
                filter.owner.as_ref().or(filter.controller.as_ref()),
                Some(
                    PlayerFilter::ControllerOf(ObjectRef::Target)
                        | PlayerFilter::OwnerOf(ObjectRef::Target)
                )
            ),
            Self::PerCount {
                count: AnthemCountExpression::CreatureTypesAmong(filter),
                ..
            }
            | Self::CappedPerCount {
                count: AnthemCountExpression::CreatureTypesAmong(filter),
                ..
            } => filter.source,
            Self::CappedPerCount {
                count: AnthemCountExpression::MatchingFilter(filter),
                ..
            } => matches!(
                filter.owner.as_ref().or(filter.controller.as_ref()),
                Some(
                    PlayerFilter::ControllerOf(ObjectRef::Target)
                        | PlayerFilter::OwnerOf(ObjectRef::Target)
                )
            ),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AnthemCountExpression, AnthemValue};
    use crate::{ObjectFilter, ObjectRef, PlayerFilter, Zone};

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

    #[test]
    fn controller_of_affected_object_count_reports_dependency() {
        let mut cards_in_hand = ObjectFilter::default();
        cards_in_hand.zone = Some(Zone::Hand);
        cards_in_hand.owner = Some(PlayerFilter::ControllerOf(ObjectRef::Target));

        assert!(
            AnthemValue::scaled(1, AnthemCountExpression::MatchingFilter(cards_in_hand))
                .uses_affected_object()
        );
    }
}
