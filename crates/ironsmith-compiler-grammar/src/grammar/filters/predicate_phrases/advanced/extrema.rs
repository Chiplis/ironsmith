use super::*;

/// A referenced object's membership in an explicitly stated extremum,
/// including ties, is a current characteristic predicate. It does not choose
/// a single winner and does not restrict an already-authored target slot.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let body = words.strip_prefix(&["it", "has"])?;
    let body = body.strip_prefix(&["the"]).unwrap_or(body);
    let direction = *body.first()?;
    if !matches!(direction, "greatest" | "least" | "lowest") {
        return None;
    }
    let (axis, used) = match body.get(1..)? {
        ["power", ..] => ("power", 1),
        ["toughness", ..] => ("toughness", 1),
        ["mana", "value", ..] => ("mana value", 2),
        _ => return None,
    };
    let tail = body.get(1 + used..)?;
    let tail = tail.strip_prefix(&["or", "is", "tied", "for"])?;
    let tail = tail.strip_prefix(&[direction])?;
    let tail = if axis == "mana value" {
        tail.strip_prefix(&["mana", "value"])?
    } else {
        tail.strip_prefix(&[axis])?
    };
    let scope_words = tail.strip_prefix(&["among"])?;
    if scope_words.is_empty() {
        return None;
    }
    let scope = crate::object_filters::parse_object_filter_words(scope_words, false).ok()?;
    let value = match (direction, axis) {
        ("greatest", "power") => Value::GreatestPower(scope),
        ("greatest", "toughness") => Value::GreatestToughness(scope),
        ("greatest", "mana value") => Value::GreatestManaValue(scope),
        (_, "power") => Value::LeastPower(scope),
        (_, "toughness") => Value::LeastToughness(scope),
        (_, "mana value") => Value::LeastManaValue(scope),
        _ => return None,
    };
    let comparison = Some(crate::filter::Comparison::EqualExpr(Box::new(value)));
    let mut subject = ObjectFilter::default();
    match axis {
        "power" => subject.power = comparison,
        "toughness" => subject.toughness = comparison,
        _ => subject.mana_value = comparison,
    }
    Some(PredicateAst::ItMatches(subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tied_extremum_predicate_keeps_the_current_implicit_object_and_entire_scope() {
        for (direction, greatest) in [("greatest", true), ("least", false)] {
            let text = format!(
                "it has the {direction} power or is tied for {direction} power among creatures on the battlefield"
            );
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            let (parsed, loss) =
                ironsmith_compiler::parse_loss::capture(|| super::super::parse_predicate(&tokens));
            let predicate = parsed.unwrap();
            assert!(!loss.is_lossy(), "{}", loss.reasons_text());
            let PredicateAst::ItMatches(subject) = predicate else {
                panic!("{predicate:?}")
            };
            let Some(crate::filter::Comparison::EqualExpr(value)) = subject.power else {
                panic!("missing equality")
            };
            match value.as_ref() {
                Value::GreatestPower(scope) if greatest => {
                    assert_eq!(scope.zone, Some(Zone::Battlefield))
                }
                Value::LeastPower(scope) if !greatest => {
                    assert_eq!(scope.zone, Some(Zone::Battlefield))
                }
                _ => panic!("{value:?}"),
            }
        }
        for text in [
            "it had the greatest power or is tied for greatest power among creatures on the battlefield",
            "it has the least power or is tied for greatest power among creatures on the battlefield",
            "it has the least power or is tied for least toughness among creatures on the battlefield",
        ] {
            assert!(
                parse(&crate::lexer::lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }
}

#[cfg(test)]
mod numeric_predicate_dispatch_tests {
    use super::*;
    #[test]
    fn complete_numeric_spell_predicate_does_not_probe_an_inapplicable_object_descriptor() {
        let tokens=crate::lexer::lex_line("its mana value is less than or equal to the greatest mana value among permanents you control",0).unwrap();
        let (parsed, loss) =
            ironsmith_compiler::parse_loss::capture(|| super::super::parse_predicate(&tokens));
        let PredicateAst::ValueComparison {
            left,
            operator,
            right,
        } = parsed.unwrap()
        else {
            panic!("numeric predicate required")
        };
        assert!(matches!(left.unhinted(), Value::ManaValueOf(_)));
        assert_eq!(operator, ValueComparisonOperator::LessThanOrEqual);
        assert!(
            matches!(right.unhinted(),Value::GreatestManaValue(filter) if filter.controller==Some(PlayerFilter::You))
        );
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    }
}
