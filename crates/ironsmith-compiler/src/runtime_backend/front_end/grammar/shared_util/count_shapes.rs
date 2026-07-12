use crate::TagKey;
use crate::cards::builders::IT_TAG;
use crate::effect::Value;
use crate::runtime_backend::grammar::filters::{
    parse_counter_type_from_tokens, parse_counter_type_words,
};
use crate::runtime_backend::lexer::synthetic_word_tokens;
use crate::runtime_backend::object_filters::parse_object_filter_words;
use crate::runtime_backend::util::{
    is_article, source_choose_spec_for_surface, source_reference_surface_for_words,
    this_source_surface_for_words,
};
use crate::target::{ChooseSpec, PlayerFilter, TaggedOpbjectRelation};

use super::super::permission_shapes;
use super::value_helper_shapes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ForEachHead {
    item_start: usize,
    other: bool,
}

pub(crate) fn parse_for_each_count_value_words(words: &[&str]) -> Option<(Value, usize)> {
    let head = parse_for_each_head(words)?;
    let idx = head.item_start;

    let mut counter_descriptor_start = idx;
    if words
        .get(counter_descriptor_start)
        .is_some_and(|word| is_article(word))
        || permission_shapes::starts_at_words(words, counter_descriptor_start, &["one"])
    {
        counter_descriptor_start += 1;
    }
    if let Some(counter_idx) = first_counter_word(&words[counter_descriptor_start..])
        .map(|relative_idx| counter_descriptor_start + relative_idx)
    {
        let parsed_counter_type = if counter_idx > counter_descriptor_start {
            parse_counter_type_words(&words[counter_descriptor_start..=counter_idx])
        } else {
            None
        };
        if let Some(counter_type) = parsed_counter_type
            && words
                .get(counter_idx + 1..counter_idx + 3)
                .is_some_and(|tail| exact_one_of(tail, &[&["you", "have"], &["you", "ve"]]))
        {
            return Some((
                Value::PlayerCounters(PlayerFilter::You, counter_type),
                counter_idx + 3,
            ));
        }
        if permission_shapes::starts_at_words(words, counter_idx + 1, &["on"]) {
            let reference_start = counter_idx + 2;
            let reference_end = value_boundary(&words[reference_start..]) + reference_start;
            let reference = &words[reference_start..reference_end];
            if is_source_counter_reference(reference) {
                let value = match parsed_counter_type {
                    Some(counter_type) => match this_source_surface_for_words(reference) {
                        Some(surface) => Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            Some(counter_type),
                        ),
                        None => Value::CountersOnSource(counter_type),
                    },
                    None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
                };
                return Some((value, reference_end));
            }
            if let Some(surface) = source_reference_surface_for_words(reference) {
                let value = Value::CountersOn(
                    Box::new(source_choose_spec_for_surface(surface)),
                    parsed_counter_type,
                );
                return Some((value, reference_end));
            }
            if is_tagged_counter_reference(reference) {
                let value = Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                    parsed_counter_type,
                );
                return Some((value, reference_end));
            }
        }
    }

    let filter_end = value_boundary(&words[idx..]) + idx;
    if let Some(value) =
        value_helper_shapes::parse_aggregate_scope_value_words(&words[idx..filter_end])
    {
        return Some((value, filter_end));
    }

    if let Some(relative_this_way) =
        permission_shapes::find_words(&words[idx..filter_end], &["this", "way"])
    {
        let this_way_start = idx + relative_this_way;
        for candidate_end in (idx + 1..this_way_start).rev() {
            if let Ok(filter) = parse_object_filter_words(&words[idx..candidate_end], head.other) {
                return Some((
                    Value::Count(
                        filter.match_tagged(
                            TagKey::from(IT_TAG),
                            TaggedOpbjectRelation::IsTaggedObject,
                        ),
                    ),
                    filter_end,
                ));
            }
        }
    }

    let count_words = &words[idx..filter_end];
    if exact_one_of(
        count_words,
        &[
            &["time", "it", "regenerated", "this", "turn"],
            &["times", "it", "regenerated", "this", "turn"],
        ],
    ) {
        return Some((Value::SourceRegeneratedThisTurnCount, filter_end));
    }
    if exact_one_of(
        count_words,
        &[
            &["card", "youve", "drawn", "this", "turn"],
            &["cards", "youve", "drawn", "this", "turn"],
            &["card", "you've", "drawn", "this", "turn"],
            &["cards", "you've", "drawn", "this", "turn"],
            &["card", "you", "have", "drawn", "this", "turn"],
            &["cards", "you", "have", "drawn", "this", "turn"],
        ],
    ) {
        return Some((Value::MaxCardsDrawnThisTurn(PlayerFilter::You), filter_end));
    }
    if exact_one_of(
        count_words,
        &[
            &["card", "an", "opponent", "has", "drawn", "this", "turn"],
            &["cards", "an", "opponent", "has", "drawn", "this", "turn"],
            &["card", "opponents", "have", "drawn", "this", "turn"],
            &["cards", "opponents", "have", "drawn", "this", "turn"],
        ],
    ) {
        return Some((
            Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
            filter_end,
        ));
    }
    if is_kick_count(count_words) {
        return Some((Value::KickCount, filter_end));
    }
    if let Some(counter_idx) = first_counter_word(count_words) {
        let counter_tokens = synthetic_word_tokens(count_words);
        if let Some(counter_type) = parse_counter_type_from_tokens(&counter_tokens) {
            if count_words
                .get(counter_idx + 1..counter_idx + 3)
                .is_some_and(|tail| exact_one_of(tail, &[&["you", "have"], &["you", "ve"]]))
            {
                return Some((
                    Value::PlayerCounters(PlayerFilter::You, counter_type),
                    filter_end,
                ));
            }
            if permission_shapes::starts_at_words(count_words, counter_idx + 1, &["on"]) {
                let reference = &count_words[counter_idx + 2..];
                if is_source_counter_reference(reference) {
                    if let Some(surface) = this_source_surface_for_words(reference) {
                        return Some((
                            Value::CountersOn(
                                Box::new(source_choose_spec_for_surface(surface)),
                                Some(counter_type),
                            ),
                            filter_end,
                        ));
                    }
                    return Some((Value::CountersOnSource(counter_type), filter_end));
                }
                if let Some(surface) = source_reference_surface_for_words(reference) {
                    return Some((
                        Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            Some(counter_type),
                        ),
                        filter_end,
                    ));
                }
                if is_tagged_counter_reference(reference) {
                    return Some((
                        Value::CountersOn(
                            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                            Some(counter_type),
                        ),
                        filter_end,
                    ));
                }
            }
        }
    }

    let filter = parse_object_filter_words(&words[idx..filter_end], head.other).ok()?;
    Some((Value::Count(filter), filter_end))
}

fn parse_for_each_head(words: &[&str]) -> Option<ForEachHead> {
    let mut item_start = if permission_shapes::prefix_words(words, &["for", "each"]) {
        2
    } else if permission_shapes::prefix_words(words, &["each"]) {
        1
    } else {
        return None;
    };
    if permission_shapes::starts_at_words(words, item_start, &["of"]) {
        item_start += 1;
    }
    if item_start >= words.len() {
        return None;
    }

    let other = permission_shapes::starts_at_words(words, item_start, &["other"])
        || permission_shapes::starts_at_words(words, item_start, &["another"]);
    if other {
        item_start += 1;
    }
    (item_start < words.len()).then_some(ForEachHead { item_start, other })
}

fn value_boundary(words: &[&str]) -> usize {
    ["plus", "minus"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
        .unwrap_or(words.len())
}

fn first_counter_word(words: &[&str]) -> Option<usize> {
    ["counter", "counters"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
}

fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

fn is_source_counter_reference(words: &[&str]) -> bool {
    exact_one_of(
        words,
        &[
            &["it"],
            &["this"],
            &["this", "card"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "source"],
            &["this", "artifact"],
            &["this", "land"],
            &["this", "enchantment"],
        ],
    )
}

fn is_tagged_counter_reference(words: &[&str]) -> bool {
    exact_one_of(
        words,
        &[
            &["that"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
            &["those"],
            &["those", "cards"],
            &["those", "creatures"],
            &["those", "permanents"],
        ],
    )
}

fn is_kick_count(words: &[&str]) -> bool {
    let Some((first, rest)) = words.split_first() else {
        return false;
    };
    if !permission_shapes::exact_words(&[*first], &["time"])
        && !permission_shapes::exact_words(&[*first], &["times"])
    {
        return false;
    }
    if rest.len() < 2 || !permission_shapes::suffix_words(rest, &["was", "kicked"]) {
        return false;
    }
    let source_words = &rest[..rest.len() - 2];
    exact_one_of(source_words, &[&["this"], &["this", "spell"], &["it"]])
        || source_reference_surface_for_words(source_words).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_for_each_draw_and_kick_counts() {
        assert_eq!(
            parse_for_each_count_value_words(&[
                "for", "each", "card", "youve", "drawn", "this", "turn"
            ]),
            Some((Value::MaxCardsDrawnThisTurn(PlayerFilter::You), 7))
        );
        assert_eq!(
            parse_for_each_count_value_words(&[
                "for", "each", "time", "this", "spell", "was", "kicked"
            ]),
            Some((Value::KickCount, 7))
        );
    }
}
