use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use crate::filter::{CounterConstraint, ObjectFilter};
use crate::object::CounterType;
use crate::runtime_backend::lexer::{OwnedLexToken, parser_token_word_refs};

use super::super::{leaf, primitives};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CounterTypeWordsSpec {
    pub(crate) counter_type: CounterType,
    pub(crate) consumed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FilterCounterConstraintSpec {
    pub(crate) constraint: CounterConstraint,
    pub(crate) consumed: usize,
    pub(crate) one_or_more: bool,
    pub(crate) plural_counter_noun: bool,
    pub(crate) plural_subject: bool,
}

fn counter_noun<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> Result<&'a str, ErrMode<ContextError>> {
    alt((
        primitives::word_slice_exact("counter"),
        primitives::word_slice_exact("counters"),
    ))
    .parse_next(input)
}

fn take_descriptor_before_counter<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> Result<&'a [&'a str], ErrMode<ContextError>> {
    repeat_till(0.., any.void(), peek(counter_noun).void())
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn parse_known_counter_type_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> Result<CounterType, ErrMode<ContextError>> {
    let checkpoint = *input;
    let Some((word, rest)) = checkpoint.split_first() else {
        return Err(primitives::backtrack_err(
            "counter type",
            "known counter type word",
        ));
    };
    let counter_type = match *word {
        "+1/+1" => CounterType::PlusOnePlusOne,
        "-1/-1" | "-0/-1" => CounterType::MinusOneMinusOne,
        "+1/+0" => CounterType::PlusOnePlusZero,
        "+0/+1" => CounterType::PlusZeroPlusOne,
        "+1/+2" => CounterType::PlusOnePlusTwo,
        "+2/+2" => CounterType::PlusTwoPlusTwo,
        "-0/-2" => CounterType::MinusZeroMinusTwo,
        "-2/-1" => CounterType::MinusTwoMinusOne,
        "-2/-2" => CounterType::MinusTwoMinusTwo,
        "deathtouch" => CounterType::Deathtouch,
        "decayed" => CounterType::Decayed,
        "flying" => CounterType::Flying,
        "haste" => CounterType::Haste,
        "hexproof" => CounterType::Hexproof,
        "indestructible" => CounterType::Indestructible,
        "lifelink" => CounterType::Lifelink,
        "menace" => CounterType::Menace,
        "reach" => CounterType::Reach,
        "trample" => CounterType::Trample,
        "vigilance" => CounterType::Vigilance,
        "loyalty" => CounterType::Loyalty,
        "charge" => CounterType::Charge,
        "stun" => CounterType::Stun,
        "void" => CounterType::Void,
        "depletion" => CounterType::Depletion,
        "dream" => CounterType::Dream,
        "storage" => CounterType::Storage,
        "ki" => CounterType::Ki,
        "energy" => CounterType::Energy,
        "experience" => CounterType::Experience,
        "age" => CounterType::Age,
        "blood" => CounterType::Blood,
        "ice" => CounterType::Ice,
        "finality" => CounterType::Finality,
        "fade" => CounterType::Fade,
        "flood" => CounterType::Flood,
        "time" => CounterType::Time,
        "brain" => CounterType::Brain,
        "burden" => CounterType::Named(intern_counter_name("burden")),
        "level" => CounterType::Level,
        "lore" => CounterType::Lore,
        "luck" => CounterType::Luck,
        "oil" => CounterType::Oil,
        "pressure" => CounterType::Named(intern_counter_name("pressure")),
        "quest" => CounterType::Quest,
        "rad" => CounterType::Rad,
        "shield" => CounterType::Shield,
        _ => {
            return Err(primitives::backtrack_err(
                "counter type",
                "known counter type word",
            ));
        }
    };
    *input = rest;
    Ok(counter_type)
}

fn parse_counter_type_words_spec_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> Result<CounterTypeWordsSpec, ErrMode<ContextError>> {
    let initial_len = input.len();
    let descriptor = take_descriptor_before_counter(input)?;
    counter_noun.parse_next(input)?;
    let Some(previous) = descriptor.last().copied() else {
        return Err(primitives::backtrack_err(
            "counter type",
            "descriptor before counter",
        ));
    };

    let counter_type = if let Some(known) = parse_counter_type_word(previous) {
        known
    } else if previous == "strike" && descriptor.len() >= 2 {
        match descriptor[descriptor.len() - 2] {
            "double" => CounterType::DoubleStrike,
            "first" => CounterType::FirstStrike,
            _ => {
                return Err(primitives::backtrack_err(
                    "counter type",
                    "first- or double-strike counter",
                ));
            }
        }
    } else if previous == "another" || leaf::parse_number_complete(previous).is_ok() {
        return Err(primitives::backtrack_err(
            "counter type",
            "nonnumeric counter descriptor",
        ));
    } else if previous
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        CounterType::Named(intern_counter_name(previous))
    } else {
        return Err(primitives::backtrack_err(
            "counter type",
            "recognized counter descriptor",
        ));
    };

    Ok(CounterTypeWordsSpec {
        counter_type,
        consumed: initial_len.saturating_sub(input.len()),
    })
}

fn parse_filter_counter_constraint_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> Result<FilterCounterConstraintSpec, ErrMode<ContextError>> {
    let initial_len = input.len();
    let descriptor = take_descriptor_before_counter(input)?;
    let counter_noun = counter_noun.parse_next(input)?;
    primitives::word_slice_exact("on")
        .void()
        .parse_next(input)?;
    let subject = alt((
        primitives::word_slice_exact("it"),
        primitives::word_slice_exact("them"),
    ))
    .parse_next(input)?;

    let descriptor_words = descriptor
        .iter()
        .copied()
        .filter(|word| {
            !matches!(*word, "or" | "more") && leaf::parse_number_complete(word).is_err()
        })
        .collect::<Vec<_>>();
    let constraint = if descriptor_words.is_empty() {
        CounterConstraint::Any
    } else if descriptor_words.len() == 1 && descriptor_words[0] == "no" {
        return Err(primitives::backtrack_err(
            "filter counter constraint",
            "counter descriptor other than no",
        ));
    } else if descriptor_words.len() == 1 {
        let word = descriptor_words[0];
        CounterConstraint::Typed(
            parse_counter_type_word(word)
                .unwrap_or_else(|| CounterType::Named(intern_counter_name(word))),
        )
    } else {
        let mut descriptor_input: primitives::WordSliceInput<'_> = &descriptor_words;
        CounterConstraint::Typed(
            parse_counter_type_words_spec_word_slice(&mut descriptor_input)?.counter_type,
        )
    };

    Ok(FilterCounterConstraintSpec {
        constraint,
        consumed: initial_len.saturating_sub(input.len()),
        one_or_more: descriptor
            .windows(3)
            .any(|words| words == ["one", "or", "more"]),
        plural_counter_noun: counter_noun == "counters",
        plural_subject: subject == "them",
    })
}

pub(crate) fn intern_counter_name(word: &str) -> &'static str {
    static INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    let map = INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().expect("counter name interner lock poisoned");
    if let Some(existing) = map.get(word) {
        return *existing;
    }

    let leaked: &'static str = Box::leak(word.to_string().into_boxed_str());
    map.insert(word.to_string(), leaked);
    leaked
}

pub(crate) fn parse_counter_type_word(word: &str) -> Option<CounterType> {
    let words = [word];
    primitives::parse_full_word_slice(&words, parse_known_counter_type_word_slice)
}

pub(crate) fn parse_counter_type_words(words: &[&str]) -> Option<CounterType> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_counter_type_words_spec_word_slice
        .parse_next(&mut input)
        .ok()
        .map(|spec| spec.counter_type)
}

pub(crate) fn parse_counter_type_from_tokens(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    let mut words = parser_token_word_refs(tokens);
    parse_counter_type_words(&words).or_else(|| {
        // Many typed grammar callers capture only the descriptor because the
        // surrounding parser has already consumed `counter`/`counters`.
        // Reuse the same counter-type parser by supplying that consumed noun
        // instead of rediscovering the descriptor through a separate probe.
        words.push("counter");
        parse_counter_type_words(&words)
    })
}

pub(crate) fn parse_filter_counter_constraint_words(
    words: &[&str],
) -> Option<(CounterConstraint, usize)> {
    let spec = parse_filter_counter_constraint_spec_words(words)?;
    Some((spec.constraint, spec.consumed))
}

pub(crate) fn parse_filter_counter_constraint_spec_words(
    words: &[&str],
) -> Option<FilterCounterConstraintSpec> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_filter_counter_constraint_word_slice
        .parse_next(&mut input)
        .ok()
}

fn apply_filter_counter_constraint_surface(
    filter: &mut ObjectFilter,
    spec: FilterCounterConstraintSpec,
) {
    if filter.with_counter == Some(spec.constraint) {
        filter.set_counter_requirement_surface(
            spec.one_or_more,
            spec.plural_counter_noun,
            spec.plural_subject,
        );
    }
    if filter.without_counter == Some(spec.constraint) {
        filter.set_counter_exclusion_surface(spec.plural_counter_noun, spec.plural_subject);
    }
    for branch in &mut filter.any_of {
        apply_filter_counter_constraint_surface(branch, spec);
    }
}

/// Restore Oracle-only number and pronoun choices after the semantic filter
/// parser has consumed a counter constraint. These hints do not participate in
/// filter equality or runtime matching.
pub(crate) fn preserve_filter_counter_constraint_surface_words(
    filter: &mut ObjectFilter,
    words: &[&str],
) {
    for start in 0..words.len() {
        if let Some(spec) = parse_filter_counter_constraint_spec_words(&words[start..]) {
            apply_filter_counter_constraint_surface(filter, spec);
        }
    }
}

pub(crate) fn preserve_filter_counter_constraint_surface_tokens(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    let words = parser_token_word_refs(tokens);
    preserve_filter_counter_constraint_surface_words(filter, &words);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_compound_and_named_counter_types() {
        assert_eq!(
            parse_counter_type_words(&["a", "double", "strike", "counter"]),
            Some(CounterType::DoubleStrike)
        );
        assert_eq!(
            parse_counter_type_words(&["quest", "counter", "on", "it"]),
            Some(CounterType::Quest)
        );
        assert_eq!(
            parse_counter_type_words(&["rad", "counter"]),
            Some(CounterType::Rad)
        );
        assert_eq!(parse_counter_type_words(&["two", "counters"]), None);
        assert_eq!(parse_counter_type_words(&["another", "counter"]), None);
    }

    #[test]
    fn token_adapter_accepts_full_counter_phrases_and_captured_descriptors() {
        use crate::runtime_backend::lexer::lex_line;

        let full = lex_line("a +1/+1 counter", 0).unwrap();
        assert_eq!(
            parse_counter_type_from_tokens(&full),
            Some(CounterType::PlusOnePlusOne)
        );

        let descriptor = lex_line("+1/+1", 0).unwrap();
        assert_eq!(
            parse_counter_type_from_tokens(&descriptor),
            Some(CounterType::PlusOnePlusOne)
        );

        let compound = lex_line("double strike", 0).unwrap();
        assert_eq!(
            parse_counter_type_from_tokens(&compound),
            Some(CounterType::DoubleStrike)
        );
    }

    #[test]
    fn parses_any_and_typed_filter_constraints_with_consumption() {
        assert_eq!(
            parse_filter_counter_constraint_words(&["counter", "on", "it", "this", "turn"]),
            Some((CounterConstraint::Any, 3))
        );
        assert_eq!(
            parse_filter_counter_constraint_words(&[
                "two", "+1/+1", "counters", "on", "them", "this", "turn"
            ]),
            Some((CounterConstraint::Typed(CounterType::PlusOnePlusOne), 5))
        );
        assert_eq!(
            parse_filter_counter_constraint_words(&["stun", "counter", "on", "it"]),
            Some((CounterConstraint::Typed(CounterType::Stun), 4))
        );

        let plural = parse_filter_counter_constraint_spec_words(&[
            "+1/+1", "counters", "on", "them", "this", "turn",
        ])
        .expect("plural filter counter surface");
        assert_eq!(
            plural.constraint,
            CounterConstraint::Typed(CounterType::PlusOnePlusOne)
        );
        assert_eq!(plural.consumed, 4);
        assert!(plural.plural_counter_noun);
        assert!(plural.plural_subject);
        assert!(!plural.one_or_more);

        let one_or_more = parse_filter_counter_constraint_spec_words(&[
            "one", "or", "more", "loyalty", "counters", "on", "it",
        ])
        .expect("one-or-more filter counter surface");
        assert_eq!(
            one_or_more.constraint,
            CounterConstraint::Typed(CounterType::Loyalty)
        );
        assert!(one_or_more.one_or_more);
        assert!(one_or_more.plural_counter_noun);
        assert!(!one_or_more.plural_subject);

        let singular =
            parse_filter_counter_constraint_spec_words(&["a", "+1/+1", "counter", "on", "it"])
                .expect("singular filter counter surface");
        assert!(!singular.plural_counter_noun);
        assert!(!singular.plural_subject);
        assert!(!singular.one_or_more);
    }

    #[test]
    fn rejects_no_counter_and_incomplete_reference_shapes() {
        assert!(parse_filter_counter_constraint_words(&["no", "counters", "on", "it"]).is_none());
        assert!(parse_filter_counter_constraint_words(&["stun", "counter"]).is_none());
        assert!(parse_filter_counter_constraint_words(&["stun", "counter", "on", "you"]).is_none());
    }
}
