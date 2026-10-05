use crate::tag::TagKeyWalk;

use crate::{
    CounterType, KeywordActionKind, ManaSymbol, ObjectFilter, ObjectRef, PlayerFilter,
    SourceReferenceSurface, Value,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum AnthemCountExpression {
    MatchingFilter(ObjectFilter),
    /// Number of players whose graveyards contain at least `minimum_cards`
    /// cards. This is a count of qualifying graveyards, not a count of the
    /// cards across those graveyards.
    GraveyardsWithAtLeastCards {
        minimum_cards: u32,
    },
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
        min_name_letters: Option<u32>,
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
    TotalUnspentMana(PlayerFilter),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
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
        let cards_in_hand = ObjectFilter {
            zone: Some(Zone::Hand),
            owner: Some(PlayerFilter::ControllerOf(ObjectRef::Target)),
            ..ObjectFilter::default()
        };

        assert!(
            AnthemValue::scaled(1, AnthemCountExpression::MatchingFilter(cards_in_hand))
                .uses_affected_object()
        );
    }
}

/// Numeric bindings whose meaning is independent of whether the evaluator's
/// object anchor is the ability source or the affected permanent. Shared by
/// compiler admission and runtime Dynamic-to-layer conversion. This is a
/// bounded capability check, not a general continuous-value validator.
pub fn supports_controller_state_anthem_value(value: &Value) -> bool {
    if !crate::tag::tag_keys_of(value).is_empty() {
        return false;
    }
    match value.unhinted() {
        Value::Fixed(_) => true,
        Value::MaximumLifeTotal(PlayerFilter::Any | PlayerFilter::Opponent | PlayerFilter::You)
        | Value::CountPlayersBelowHalfStartingLifeTotal(
            PlayerFilter::Any | PlayerFilter::Opponent | PlayerFilter::You,
        ) => true,
        Value::LifeTotal(PlayerFilter::You)
        | Value::CardsInHand(PlayerFilter::You)
        | Value::CardsInLibrary(PlayerFilter::You)
        | Value::CardsInGraveyard(PlayerFilter::You)
        | Value::LifeLostThisTurn(PlayerFilter::You)
        | Value::LifeGainedThisTurn(PlayerFilter::You) => true,
        Value::TurnHistoryCount(crate::TurnHistoryCount::CardsDrawn(PlayerFilter::You))
        | Value::MaxCardsDrawnThisTurn(
            PlayerFilter::Any | PlayerFilter::Opponent | PlayerFilter::You,
        ) => true,
        Value::TurnHistoryCount(crate::TurnHistoryCount::EnteredBattlefield(filter))
        | Value::Count(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::LeastPower(filter)
        | Value::LeastToughness(filter)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter) => controller_state_filter(filter),
        Value::Scaled(inner, _) | Value::HalfRoundedDown(inner) => {
            supports_controller_state_anthem_value(inner)
        }
        Value::DividedRoundedDown(inner, divisor) => {
            *divisor != 0 && supports_controller_state_anthem_value(inner)
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            supports_controller_state_anthem_value(left)
                && supports_controller_state_anthem_value(right)
        }
        _ => false,
    }
}

fn controller_state_filter(filter: &ObjectFilter) -> bool {
    let player = |value: &Option<PlayerFilter>| {
        value
            .as_ref()
            .filter(|player| {
                matches!(
                    player,
                    PlayerFilter::You | PlayerFilter::Opponent | PlayerFilter::Any
                )
            })
            .cloned()
    };
    // A positive projection fails closed for every unlisted semantic field,
    // including future fields. In particular, `other`, chosen characteristics,
    // source/target relations and nested filters are not accidentally admitted.
    // Presentation-only surfaces have semantic PartialEq and need no rewriting.
    let independent = ObjectFilter {
        zone: filter.zone,
        controller: player(&filter.controller),
        owner: player(&filter.owner),
        card_types: filter.card_types.clone(),
        all_card_types: filter.all_card_types.clone(),
        excluded_card_types: filter.excluded_card_types.clone(),
        subtypes: filter.subtypes.clone(),
        all_subtypes: filter.all_subtypes.clone(),
        excluded_subtypes: filter.excluded_subtypes.clone(),
        supertypes: filter.supertypes.clone(),
        excluded_supertypes: filter.excluded_supertypes.clone(),
        colors: filter.colors,
        required_colors: filter.required_colors,
        excluded_colors: filter.excluded_colors,
        colorless: filter.colorless,
        multicolored: filter.multicolored,
        monocolored: filter.monocolored,
        token: filter.token,
        nontoken: filter.nontoken,
        entered_battlefield_this_turn: filter.entered_battlefield_this_turn,
        entered_battlefield_controller: player(&filter.entered_battlefield_controller),
        ..ObjectFilter::default()
    };
    filter == &independent
}

#[cfg(test)]
mod controller_state_anthem_value_tests {
    use super::*;

    #[test]
    fn layer_capability_keeps_object_and_resolution_relative_values_on_the_legacy_path() {
        assert!(supports_controller_state_anthem_value(&Value::LifeTotal(
            PlayerFilter::You
        )));
        let mut grave = ObjectFilter::creature();
        grave.zone = Some(crate::Zone::Graveyard);
        grave.owner = Some(PlayerFilter::You);
        grave.set_explicit_card_noun(true);
        assert!(supports_controller_state_anthem_value(
            &Value::GreatestPower(grave.clone())
        ));
        grave.other = true;
        assert!(!supports_controller_state_anthem_value(
            &Value::GreatestPower(grave)
        ));
        for value in [
            Value::SourcePower,
            Value::PartySize(PlayerFilter::You),
            Value::ManaValueOf(Box::new(crate::ChooseSpec::Source)),
            Value::ManaValueOf(Box::new(crate::ChooseSpec::Tagged("it".into()))),
            Value::CountersOnSource(CounterType::PlusOnePlusOne),
            Value::EventValue(crate::EventValueSpec::Amount),
        ] {
            assert!(!supports_controller_state_anthem_value(&value), "{value:?}");
        }
    }
}

/// Statically bound object quantities with an explicit source/recipient anchor.
/// Keep this separate from controller-state values and from arbitrary legacy
/// Dynamic values, whose historical discovery context is unchanged.
pub fn supports_scoped_reference_anthem_value(value: &Value) -> bool {
    if !crate::tag::tag_keys_of(value).is_empty() {
        return false;
    }
    match value.unhinted() {
        Value::Fixed(_) => true,
        Value::ManaValueOf(spec) => matches!(spec.unhinted(), crate::ChooseSpec::Iterated),
        Value::CountersOn(spec, None) => matches!(spec.unhinted(), crate::ChooseSpec::Source),
        Value::Scaled(inner, _) | Value::HalfRoundedDown(inner) => {
            supports_scoped_reference_anthem_value(inner)
        }
        Value::DividedRoundedDown(inner, divisor) => {
            *divisor != 0 && supports_scoped_reference_anthem_value(inner)
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            supports_scoped_reference_anthem_value(left)
                && supports_scoped_reference_anthem_value(right)
        }
        _ => false,
    }
}

#[cfg(test)]
mod scoped_reference_tests {
    use super::*;
    #[test]
    fn scoped_anthem_capability_admits_only_proven_source_and_recipient_quantities() {
        assert!(supports_scoped_reference_anthem_value(&Value::ManaValueOf(
            Box::new(crate::ChooseSpec::Iterated)
        )));
        assert!(supports_scoped_reference_anthem_value(&Value::CountersOn(
            Box::new(crate::ChooseSpec::Source),
            None
        )));
        for value in [
            Value::SourcePower,
            Value::ManaValueOf(Box::new(crate::ChooseSpec::target(
                crate::ChooseSpec::Iterated,
            ))),
            Value::ManaValueOf(Box::new(crate::ChooseSpec::Source)),
            Value::ManaValueOf(Box::new(crate::ChooseSpec::Tagged("it".into()))),
            Value::CountersOn(Box::new(crate::ChooseSpec::Iterated), None),
            Value::EventValue(crate::EventValueSpec::Amount),
        ] {
            assert!(!supports_scoped_reference_anthem_value(&value), "{value:?}");
        }
    }
}
