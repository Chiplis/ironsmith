use crate::TagKey;
use crate::effect::{ChoiceAggregateConstraint, Value};
use crate::lexer::OwnedLexToken;
use crate::target::{ChooseSpec, ObjectFilter, SourceReferenceSurface};

/// Lift an authored `total mana value ... or less` restriction out of the
/// per-object filter and into a constraint on the chosen set.
///
/// Keeping this separate is behaviorally important: two mana-value-4 cards
/// each satisfy `mana value 6 or less`, but together do not satisfy `total
/// mana value 6 or less`.
pub fn lift_total_mana_value_choice_constraint(
    tokens: &[OwnedLexToken],
    filter: &mut ObjectFilter,
) -> Option<ChoiceAggregateConstraint> {
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    for (phrase, metric) in [
        (
            &["total", "power"][..],
            crate::effect::ChoiceAggregateMetric::Power,
        ),
        (
            &["total", "toughness"][..],
            crate::effect::ChoiceAggregateMetric::Toughness,
        ),
    ] {
        if crate::word_primitives::sequence_occurs(&words, phrase)
            && !crate::word_primitives::sequence_occurs(
                &words,
                &["total", "power", "and", "toughness"],
            )
        {
            let comparison = match metric {
                crate::effect::ChoiceAggregateMetric::Power => &mut filter.power,
                _ => &mut filter.toughness,
            };
            // "any number of creatures with total power 12 or greater"
            // (Phyrexian Dreadnought) is a lower bound on the chosen set.
            return match comparison.take()? {
                crate::filter::Comparison::LessThanOrEqual(n) => {
                    Some(ChoiceAggregateConstraint::at_most(metric, Value::Fixed(n)))
                }
                crate::filter::Comparison::LessThanOrEqualExpr(n) => {
                    Some(ChoiceAggregateConstraint::at_most(metric, *n))
                }
                crate::filter::Comparison::GreaterThanOrEqual(n) => {
                    Some(ChoiceAggregateConstraint::at_least(metric, Value::Fixed(n)))
                }
                crate::filter::Comparison::GreaterThanOrEqualExpr(n) => {
                    Some(ChoiceAggregateConstraint::at_least(metric, *n))
                }
                other => {
                    *comparison = Some(other);
                    None
                }
            };
        }
    }
    if !crate::word_primitives::sequence_occurs(&words, &["total", "mana", "value"]) {
        return None;
    }

    // "with total mana value N or greater" (The Capitoline Triad) is a lower
    // bound and "with total mana value X" (Fabrication Foundry) is an exact
    // total; both bound the chosen set, not each object (CR 118.3).
    let mut maximum = match filter.mana_value.take()? {
        crate::filter::Comparison::LessThanOrEqual(maximum) => Value::Fixed(maximum),
        crate::filter::Comparison::LessThanOrEqualExpr(maximum) => *maximum,
        crate::filter::Comparison::GreaterThanOrEqual(minimum) => {
            return Some(ChoiceAggregateConstraint::total_mana_value_at_least(
                Value::Fixed(minimum),
            ));
        }
        crate::filter::Comparison::GreaterThanOrEqualExpr(minimum) => {
            return Some(ChoiceAggregateConstraint::total_mana_value_at_least(
                *minimum,
            ));
        }
        crate::filter::Comparison::Equal(total) => {
            return Some(ChoiceAggregateConstraint {
                metric: crate::effect::ChoiceAggregateMetric::ManaValue,
                minimum: Some(Value::Fixed(total)),
                maximum: Value::Fixed(total),
            });
        }
        crate::filter::Comparison::EqualExpr(total) => {
            return Some(ChoiceAggregateConstraint {
                metric: crate::effect::ChoiceAggregateMetric::ManaValue,
                minimum: Some((*total).clone()),
                maximum: *total,
            });
        }
        other => {
            filter.mana_value = Some(other);
            return None;
        }
    };

    if let Some(sacrificed_idx) =
        crate::word_primitives::select_word_position(&words, |word| word == "sacrificed")
    {
        let object_kind = words
            .get(sacrificed_idx + 1)
            .map(|word| word.trim_end_matches("'s"))
            .filter(|word| !word.is_empty())
            .unwrap_or("permanent");
        maximum = Value::ManaValueOf(Box::new(
            ChooseSpec::Tagged((crate::tag::CompilerReferenceTag::SacrificeCost0.bind()).into())
                .with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
                    SourceReferenceSurface::ThisPermanentType(format!(
                        "the sacrificed {object_kind}"
                    )),
                )),
        ));
    }

    Some(ChoiceAggregateConstraint::total_mana_value_at_most(maximum))
}
