use super::*;
use ironsmith_core::ValueSurfaceHint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtremumDirection {
    Greatest,
    Least,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtremumCharacteristic {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, Copy)]
struct ExtremumSplit<'a> {
    subject_words: &'a [&'a str],
    scope_words: Option<&'a [&'a str]>,
    direction: ExtremumDirection,
    characteristic: ExtremumCharacteristic,
    implicit_scope: bool,
    tied_short: Option<bool>,
}

fn extremum_direction(word: &str) -> Option<ExtremumDirection> {
    match word {
        "greatest" => Some(ExtremumDirection::Greatest),
        "least" | "lowest" => Some(ExtremumDirection::Least),
        _ => None,
    }
}

fn extremum_characteristic(words: &[&str]) -> Option<(ExtremumCharacteristic, usize)> {
    match words {
        ["power", ..] => Some((ExtremumCharacteristic::Power, 1)),
        ["toughness", ..] => Some((ExtremumCharacteristic::Toughness, 1)),
        ["mana", "value", ..] => Some((ExtremumCharacteristic::ManaValue, 2)),
        _ => None,
    }
}

fn tie_suffix_start(words: &[&str]) -> Option<usize> {
    words
        .windows(3)
        .position(|window| window == ["or", "tied", "for"])
}

fn parse_tie_suffix(
    words: &[&str],
    direction: ExtremumDirection,
    characteristic: ExtremumCharacteristic,
) -> Option<bool> {
    let Some(rest) = words.strip_prefix(&["or", "tied", "for"]) else {
        return None;
    };
    let rest = rest.strip_prefix(&["the"]).unwrap_or(rest);
    let Some((&direction_word, rest)) = rest.split_first() else {
        return None;
    };
    if extremum_direction(direction_word) != Some(direction) {
        return None;
    }
    if rest.is_empty() {
        return Some(true);
    }
    extremum_characteristic(rest)
        .filter(|(suffix_characteristic, consumed)| {
            *suffix_characteristic == characteristic && *consumed == rest.len()
        })
        .map(|_| false)
}

fn split_extremum_words<'a>(words: &'a [&'a str]) -> Option<ExtremumSplit<'a>> {
    for with_index in 0..words.len() {
        if words[with_index] != "with" || with_index == 0 {
            continue;
        }

        let mut cursor = with_index + 1;
        if words.get(cursor) == Some(&"the") {
            cursor += 1;
        }
        let Some(direction) = words.get(cursor).and_then(|word| extremum_direction(word)) else {
            continue;
        };
        cursor += 1;
        let Some((characteristic, consumed)) =
            words.get(cursor..).and_then(extremum_characteristic)
        else {
            continue;
        };
        cursor += consumed;

        let Some(tail) = words.get(cursor..) else {
            continue;
        };
        let (scope_words, tied_short) = if let Some(scope_tail) = tail.strip_prefix(&["among"]) {
            let tie_start = tie_suffix_start(scope_tail).unwrap_or(scope_tail.len());
            let (scope, tie_suffix) = scope_tail.split_at(tie_start);
            if scope.is_empty() {
                continue;
            }
            let tied_short = if tie_suffix.is_empty() {
                None
            } else {
                let Some(short) = parse_tie_suffix(tie_suffix, direction, characteristic) else {
                    continue;
                };
                Some(short)
            };
            (Some(scope), tied_short)
        } else if tail.is_empty() {
            (None, None)
        } else if let Some(short) = parse_tie_suffix(tail, direction, characteristic) {
            (None, Some(short))
        } else {
            continue;
        };

        return Some(ExtremumSplit {
            subject_words: &words[..with_index],
            scope_words,
            direction,
            characteristic,
            implicit_scope: scope_words.is_none(),
            tied_short,
        });
    }
    None
}

fn inherit_scope_boundaries(selected: &mut ObjectFilter, scope: &ObjectFilter) {
    if selected.card_types.is_empty() {
        selected.card_types = scope.card_types.clone();
    }
    if selected.controller.is_none() {
        selected.controller = scope.controller.clone();
    }
    if selected.owner.is_none() {
        selected.owner = scope.owner.clone();
    }
    if selected.zone.is_none() {
        selected.zone = scope.zone;
    }
}

pub(crate) fn parse_extremum_object_filter_words(
    words: &[&str],
    other: bool,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(split) = split_extremum_words(words) else {
        return Ok(None);
    };

    let mut selected = crate::runtime_backend::object_filters::parse_object_filter_words(
        split.subject_words,
        other,
    )?;
    let scope = if let Some(scope_words) = split.scope_words {
        let scope =
            crate::runtime_backend::object_filters::parse_object_filter_words(scope_words, false)?;
        inherit_scope_boundaries(&mut selected, &scope);
        scope
    } else {
        selected.clone()
    };

    let value = match (split.direction, split.characteristic) {
        (ExtremumDirection::Greatest, ExtremumCharacteristic::Power) => Value::GreatestPower(scope),
        (ExtremumDirection::Greatest, ExtremumCharacteristic::Toughness) => {
            Value::GreatestToughness(scope)
        }
        (ExtremumDirection::Greatest, ExtremumCharacteristic::ManaValue) => {
            Value::GreatestManaValue(scope)
        }
        (ExtremumDirection::Least, ExtremumCharacteristic::Power) => Value::LeastPower(scope),
        (ExtremumDirection::Least, ExtremumCharacteristic::Toughness) => {
            Value::LeastToughness(scope)
        }
        (ExtremumDirection::Least, ExtremumCharacteristic::ManaValue) => {
            Value::LeastManaValue(scope)
        }
    }
    .with_surface_hints(
        split
            .implicit_scope
            .then_some(ValueSurfaceHint::ExtremumImplicitScope)
            .into_iter()
            .chain(split.tied_short.map(|short| {
                if short {
                    ValueSurfaceHint::ExtremumTiedShort
                } else {
                    ValueSurfaceHint::ExtremumTiedForCharacteristic
                }
            })),
    );
    let comparison = Some(crate::filter::Comparison::EqualExpr(Box::new(value)));
    match split.characteristic {
        ExtremumCharacteristic::Power => selected.power = comparison,
        ExtremumCharacteristic::Toughness => selected.toughness = comparison,
        ExtremumCharacteristic::ManaValue => selected.mana_value = comparison,
    }

    Ok(Some(selected))
}

pub(crate) fn parse_extremum_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = TokenWordView::new(tokens).to_word_refs();
    parse_extremum_object_filter_words(&words, other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::Comparison;

    fn parse(words: &[&str]) -> ObjectFilter {
        parse_extremum_object_filter_words(words, false)
            .expect("extremum filter should parse")
            .expect("extremum filter should be recognized")
    }

    #[test]
    fn parses_explicit_greatest_power_scope() {
        let filter = parse(&[
            "a",
            "creature",
            "with",
            "the",
            "greatest",
            "power",
            "among",
            "creatures",
            "target",
            "opponent",
            "controls",
        ]);
        let Some(Comparison::EqualExpr(value)) = filter.power else {
            panic!("expected a power comparison");
        };
        let Value::GreatestPower(scope) = value.unhinted() else {
            panic!("expected a greatest-power value");
        };
        assert!(scope.controller.is_some());
        assert_eq!(filter.controller, scope.controller);
    }

    #[test]
    fn parses_implicit_lowest_mana_value_and_tie_reminder() {
        let filter = parse(&[
            "target",
            "nonland",
            "permanent",
            "with",
            "the",
            "lowest",
            "mana",
            "value",
            "or",
            "tied",
            "for",
            "the",
            "lowest",
            "mana",
            "value",
        ]);
        assert!(matches!(
            filter.mana_value,
            Some(Comparison::EqualExpr(value))
                if matches!(value.unhinted(), Value::LeastManaValue(_))
        ));
    }

    #[test]
    fn parses_explicit_scope_with_short_tie_suffix() {
        let filter = parse(&[
            "the", "card", "with", "the", "greatest", "mana", "value", "among", "those", "cards",
            "or", "tied", "for", "greatest",
        ]);
        assert!(matches!(
            filter.mana_value,
            Some(Comparison::EqualExpr(value))
                if matches!(value.unhinted(), Value::GreatestManaValue(_))
        ));
    }
}
