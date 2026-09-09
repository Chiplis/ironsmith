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
        (&["total", "power"][..], crate::effect::ChoiceAggregateMetric::Power),
        (&["total", "toughness"][..], crate::effect::ChoiceAggregateMetric::Toughness),
    ] {
        if crate::word_primitives::sequence_occurs(&words, phrase)
            && !crate::word_primitives::sequence_occurs(&words, &["total", "power", "and", "toughness"])
        {
            let comparison = match metric {
                crate::effect::ChoiceAggregateMetric::Power => &mut filter.power,
                _ => &mut filter.toughness,
            };
            let maximum = match comparison.take()? {
                crate::filter::Comparison::LessThanOrEqual(n) => Value::Fixed(n),
                crate::filter::Comparison::LessThanOrEqualExpr(n) => *n,
                other => { *comparison = Some(other); return None; }
            };
            return Some(ChoiceAggregateConstraint::at_most(metric, maximum));
        }
    }
    if !crate::word_primitives::sequence_occurs(&words, &["total", "mana", "value"]) {
        return None;
    }

    let mut maximum = match filter.mana_value.take()? {
        crate::filter::Comparison::LessThanOrEqual(maximum) => Value::Fixed(maximum),
        crate::filter::Comparison::LessThanOrEqualExpr(maximum) => *maximum,
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
