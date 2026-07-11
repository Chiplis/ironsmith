use winnow::prelude::*;

use crate::runtime_backend::grammar::primitives;
use crate::runtime_backend::lexer::{OwnedLexToken, parser_token_word_refs};
use crate::runtime_backend::util::{parse_card_type, parse_color, parse_subtype_flexible};

const LIST_CONJUNCTIONS: &[&[&str]] = &[&["or"], &["and"], &["and/or"]];
const TYPEISH_MARKERS: &[&[&str]] = &[&["artifact"], &["artifacts"], &["creature"], &["creatures"]];
const DISCARD_MARKERS: &[&[&str]] = &[&["discard"], &["discards"]];
const CARD_MARKERS: &[&[&str]] = &[&["card"], &["cards"]];
const SPELL_MARKERS: &[&[&str]] = &[&["spell"], &["spells"]];
const FIRST_TIME_EACH_TURN_SUFFIXES: &[&[&str]] = &[
    &["for", "the", "first", "time", "each", "turn"],
    &["for", "the", "first", "time", "this", "turn"],
];
const THAT_ATTACHED_REFERENCE_PREFIXES: &[&[&str]] =
    &[&["that", "creature"], &["that", "permanent"]];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TriggerListTailKind {
    Type,
    Color,
    Object,
    Numeric,
    DiscardQualifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerEffectListTailSplit {
    pub(crate) split_token_idx: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FirstTimeEachTurnTriggerSplit<'a> {
    pub(crate) trigger_tokens: &'a [OwnedLexToken],
    pub(crate) limit: u32,
}

fn phrase_occurs(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn one_of_phrases_occurs(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    primitives::find_prefix(tokens, || primitives::any_phrase(phrases)).is_some()
}

fn starts_with_one_of(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    primitives::parse_prefix(tokens, primitives::any_phrase(phrases)).is_some()
}

fn without_trailing_s(word: &str) -> Option<&str> {
    let bytes = word.as_bytes();
    if bytes.last().copied() == Some(b's') && word.len() > 1 {
        word.get(..word.len() - 1)
    } else {
        None
    }
}

fn objectish_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || parse_subtype_flexible(word).is_some()
        || without_trailing_s(word).is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_flexible(stem).is_some()
        })
}

fn first_comma(tokens: &[OwnedLexToken]) -> Option<(usize, &[OwnedLexToken])> {
    let (idx, (), after) = primitives::find_prefix(tokens, || primitives::comma().void())?;
    Some((idx, after))
}

fn first_comma_after_marker(
    tokens: &[OwnedLexToken],
    markers: &'static [&'static [&'static str]],
) -> Option<usize> {
    let (idx, after) = first_comma(tokens)?;
    if one_of_phrases_occurs(&tokens[..idx], markers) {
        return Some(idx);
    }
    first_comma_after_marker(after, markers).map(|next| idx + 1 + next)
}

fn last_comma(tokens: &[OwnedLexToken]) -> Option<usize> {
    let (idx, after) = first_comma(tokens)?;
    last_comma(after).map(|next| idx + 1 + next).or(Some(idx))
}

fn first_non_list_comma(tokens: &[OwnedLexToken], kind: TriggerListTailKind) -> Option<usize> {
    let (idx, after) = first_comma(tokens)?;
    let next_word = after.first().and_then(OwnedLexToken::as_word)?;
    let continues_list = starts_with_one_of(after, LIST_CONJUNCTIONS)
        || match kind {
            TriggerListTailKind::Color => parse_color(next_word).is_some(),
            TriggerListTailKind::Object => objectish_word(next_word),
            _ => false,
        };
    if !continues_list {
        return Some(idx);
    }
    first_non_list_comma(after, kind).map(|next| idx + 1 + next)
}

fn object_list_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    let after_conjunction = LIST_CONJUNCTIONS
        .iter()
        .find_map(|prefix| primitives::parse_word_sequence_prefix(&words, prefix));
    let first = after_conjunction.unwrap_or(&words).first().copied();
    first.is_some_and(objectish_word) && first_comma(tokens).is_some()
}

fn discard_qualifier_tail(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> bool {
    if !one_of_phrases_occurs(trigger_prefix_tokens, DISCARD_MARKERS) {
        return false;
    }
    let words = parser_token_word_refs(tail_tokens);
    let Some(first) = words.first().copied() else {
        return false;
    };
    let typeish = parse_card_type(first).is_some()
        || TYPEISH_MARKERS
            .iter()
            .any(|phrase| phrase.first().copied() == Some(first))
        || LIST_CONJUNCTIONS
            .iter()
            .any(|phrase| phrase.first().copied() == Some(first));
    typeish && first_comma_after_marker(tail_tokens, CARD_MARKERS).is_some()
}

fn type_list_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    let Some(first) = words.first().copied() else {
        return false;
    };
    objectish_word(first)
        && one_of_phrases_occurs(tokens, SPELL_MARKERS)
        && phrase_occurs(tokens, &["or"])
        && first_comma(tokens).is_some()
}

fn color_list_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    words
        .first()
        .is_some_and(|word| parse_color(word).is_some())
        && phrase_occurs(tokens, &["or"])
        && first_comma(tokens).is_some()
}

fn numeric_list_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    words.len() >= 3
        && words[0].parse::<i32>().is_ok()
        && words
            .get(1..)
            .is_some_and(|rest| rest.iter().any(|word| word.parse::<i32>().is_ok()))
        && phrase_occurs(tokens, &["or"])
}

fn classify_tail(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> Option<TriggerListTailKind> {
    if discard_qualifier_tail(trigger_prefix_tokens, tail_tokens) {
        Some(TriggerListTailKind::DiscardQualifier)
    } else if numeric_list_tail(tail_tokens) {
        Some(TriggerListTailKind::Numeric)
    } else if type_list_tail(tail_tokens) {
        Some(TriggerListTailKind::Type)
    } else if color_list_tail(tail_tokens) {
        Some(TriggerListTailKind::Color)
    } else if object_list_tail(tail_tokens) {
        Some(TriggerListTailKind::Object)
    } else {
        None
    }
}

pub(crate) fn parse_trigger_effect_list_tail_split_lexed(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> Option<TriggerEffectListTailSplit> {
    let kind = classify_tail(trigger_prefix_tokens, tail_tokens)?;
    let split_token_idx = match kind {
        TriggerListTailKind::DiscardQualifier => {
            first_comma_after_marker(tail_tokens, CARD_MARKERS)?
        }
        TriggerListTailKind::Numeric => last_comma(tail_tokens)?,
        TriggerListTailKind::Type => first_comma_after_marker(tail_tokens, SPELL_MARKERS)?,
        TriggerListTailKind::Color | TriggerListTailKind::Object => {
            first_comma_after_marker(tail_tokens, SPELL_MARKERS)
                .or_else(|| first_non_list_comma(tail_tokens, kind))?
        }
    };
    Some(TriggerEffectListTailSplit { split_token_idx })
}

pub(crate) fn parse_first_time_each_turn_trigger_suffix_lexed(
    trigger_tokens: &[OwnedLexToken],
) -> Option<FirstTimeEachTurnTriggerSplit<'_>> {
    FIRST_TIME_EACH_TURN_SUFFIXES.iter().find_map(|suffix| {
        primitives::strip_lexed_suffix_phrase(trigger_tokens, suffix).map(|trigger_tokens| {
            FirstTimeEachTurnTriggerSplit {
                trigger_tokens,
                limit: 1,
            }
        })
    })
}

pub(crate) fn rewrite_attached_controller_effect_tokens_lexed(
    trigger_tokens: &[OwnedLexToken],
    effects_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    fn enchanted_controller_prefix(
        input: &mut primitives::WordSliceInput<'_>,
    ) -> winnow::error::ModalResult<()> {
        primitives::word_slice_exact("enchanted")
            .void()
            .parse_next(input)?;
        winnow::combinator::alt((
            winnow::combinator::alt((
                primitives::word_slice_exact("creature"),
                primitives::word_slice_exact("creatures"),
                primitives::word_slice_exact("permanent"),
                primitives::word_slice_exact("permanents"),
                primitives::word_slice_exact("artifact"),
                primitives::word_slice_exact("artifacts"),
                primitives::word_slice_exact("enchantment"),
                primitives::word_slice_exact("enchantments"),
            )),
            winnow::combinator::alt((
                primitives::word_slice_exact("land"),
                primitives::word_slice_exact("lands"),
            )),
        ))
        .void()
        .parse_next(input)?;
        primitives::word_slice_exact("controller")
            .void()
            .parse_next(input)
    }

    let trigger_words = parser_token_word_refs(trigger_tokens);
    let references_enchanted_controller = (0..trigger_words.len()).any(|start| {
        let mut input: primitives::WordSliceInput<'_> = &trigger_words[start..];
        enchanted_controller_prefix.parse_next(&mut input).is_ok()
    });
    if !references_enchanted_controller {
        return effects_tokens.to_vec();
    }

    let mut rewritten = Vec::with_capacity(effects_tokens.len());
    let mut remaining = effects_tokens;
    while let Some((first, rest)) = remaining.split_first() {
        let attached_reference = primitives::parse_prefix(
            remaining,
            primitives::any_phrase(THAT_ATTACHED_REFERENCE_PREFIXES),
        );
        if let Some((_phrase, after_reference)) = attached_reference {
            let consumed = remaining.len() - after_reference.len();
            let mut enchanted = first.clone();
            let _ = enchanted.replace_word("enchanted");
            rewritten.push(enchanted);
            rewritten.extend_from_slice(&remaining[1..consumed]);
            remaining = after_reference;
        } else {
            rewritten.push(first.clone());
            remaining = rest;
        }
    }
    rewritten
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::lexer::{lex_line, parser_token_word_refs};

    use super::*;

    #[test]
    fn parses_trigger_list_tail_boundaries() {
        let prefix = lex_line("Whenever you discard", 0).unwrap();
        let tail = lex_line("artifact or creature cards, draw a card", 0).unwrap();
        let split = parse_trigger_effect_list_tail_split_lexed(&prefix, &tail).unwrap();
        assert_eq!(tail[split.split_token_idx].parser_text(), ",");

        let tail = lex_line("1, 2, or 3, draw a card", 0).unwrap();
        let split = parse_trigger_effect_list_tail_split_lexed(&[], &tail).unwrap();
        assert_eq!(tail[split.split_token_idx].parser_text(), ",");
        assert_eq!(
            parser_token_word_refs(&tail[..split.split_token_idx]),
            ["1", "2", "or", "3"]
        );
    }

    #[test]
    fn parses_frequency_suffix_and_attached_controller_rewrite() {
        let trigger = lex_line(
            "enchanted creature's controller attacks for the first time each turn",
            0,
        )
        .unwrap();
        let split = parse_first_time_each_turn_trigger_suffix_lexed(&trigger).unwrap();
        assert_eq!(split.limit, 1);

        let effects = lex_line("that creature draws a card", 0).unwrap();
        let rewritten = rewrite_attached_controller_effect_tokens_lexed(&trigger, &effects);
        assert_eq!(parser_token_word_refs(&rewritten)[0], "enchanted");
    }
}
