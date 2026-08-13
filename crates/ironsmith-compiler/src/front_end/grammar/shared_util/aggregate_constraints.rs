use crate::TagKey;
use crate::effect::{ChoiceAggregateConstraint, Value};
use crate::front_end::lexer::OwnedLexToken;
use crate::target::{ChooseSpec, ObjectFilter, SourceReferenceSurface};

/// Lift an authored `total mana value ... or less` restriction out of the
/// per-object filter and into a constraint on the chosen set.
///
/// Keeping this separate is behaviorally important: two mana-value-4 cards
/// each satisfy `mana value 6 or less`, but together do not satisfy `total
/// mana value 6 or less`.
pub(crate) fn lift_total_mana_value_choice_constraint(
    tokens: &[OwnedLexToken],
    filter: &mut ObjectFilter,
) -> Option<ChoiceAggregateConstraint> {
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    if !words
        .windows(3)
        .any(|window| window == ["total", "mana", "value"])
    {
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

    if let Some(sacrificed_idx) = words.iter().position(|word| *word == "sacrificed") {
        let object_kind = words
            .get(sacrificed_idx + 1)
            .map(|word| word.trim_end_matches("'s"))
            .filter(|word| !word.is_empty())
            .unwrap_or("permanent");
        maximum = Value::ManaValueOf(Box::new(
            ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0")).with_surface_hint(
                crate::target::ChooseSpecSurfaceHint::SourceReference(
                    SourceReferenceSurface::ThisPermanentType(format!(
                        "the sacrificed {object_kind}"
                    )),
                ),
            ),
        ));
    }

    Some(ChoiceAggregateConstraint::total_mana_value_at_most(maximum))
}
