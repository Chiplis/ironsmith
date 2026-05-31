use winnow::combinator::{alt, dispatch, fail, opt, peek, seq};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;

use crate::cards::builders::TextSpan;
use crate::effect::Until;

use super::grammar::primitives as grammar;
pub(crate) use super::grammar::values::{
    parse_count_range_prefix, parse_mana_symbol, parse_mana_symbol_group, parse_modal_choose_range,
    parse_scryfall_mana_cost, parse_type_line_with, parse_value_comparison_tokens,
};
use super::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, contains_token_kind, render_token_slice,
};
pub(crate) type LexedInput<'a> = LexStream<'a>;

const TICKET_SYMBOL_TEXT: &str = "{tk}";
const COMPLEATED_MARKER_TEXT: &str = "compleated";
const CORE_KEYWORD_MARKER_PREFIXES: &[&str] =
    &["prototype ", "more than meets the eye ", "splice onto "];
const STATIC_KEYWORD_MARKER_EXTRA_PREFIXES: &[&str] = &["dredge "];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnDurationPhrase {
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
    UntilYourNextTurnEnd,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommonSentenceHead {
    ForEach,
    If,
    Until,
    WhereXIs,
    Target,
    CountPrefix,
}

fn until_from_turn_duration_phrase(duration: TurnDurationPhrase) -> Until {
    match duration {
        TurnDurationPhrase::ThisTurn | TurnDurationPhrase::UntilEndOfTurn => Until::EndOfTurn,
        TurnDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        TurnDurationPhrase::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
    }
}

pub(crate) fn slice_starts_with<T: PartialEq>(items: &[T], prefix: &[T]) -> bool {
    crate::slice_primitives::starts_with(items, prefix)
}

pub(crate) fn slice_ends_with<T: PartialEq>(items: &[T], suffix: &[T]) -> bool {
    crate::slice_primitives::ends_with(items, suffix)
}

#[allow(dead_code)]
pub(crate) fn slice_ends_with_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    crate::slice_primitives::ends_with_any(items, patterns)
}

pub(crate) fn slice_contains<T: PartialEq>(items: &[T], expected: &T) -> bool {
    crate::slice_primitives::contains(items, expected)
}

#[allow(dead_code)]
pub(crate) fn slice_contains_any<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    crate::slice_primitives::contains_any(items, expected)
}

#[allow(dead_code)]
pub(crate) fn slice_contains_all<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    crate::slice_primitives::contains_all(items, expected)
}

#[allow(dead_code)]
pub(crate) fn slice_all_match<T>(items: &[T], predicate: impl FnMut(&T) -> bool) -> bool {
    crate::slice_primitives::all_match(items, predicate)
}

#[allow(dead_code)]
pub(crate) fn slice_eq_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    crate::slice_primitives::equals_any(items, patterns)
}

#[allow(dead_code)]
pub(crate) fn slice_starts_with_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    crate::slice_primitives::starts_with_any(items, patterns)
}

pub(crate) fn iter_contains<I, T>(items: I, expected: &T) -> bool
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<T>,
    T: PartialEq + ?Sized,
{
    crate::slice_primitives::iter_contains(items, expected)
}

pub(crate) fn iter_eq<I, J>(left: I, right: J) -> bool
where
    I: IntoIterator,
    J: IntoIterator,
    I::Item: PartialEq<J::Item>,
{
    crate::slice_primitives::iter_eq(left, right)
}

pub(crate) fn slice_strip_prefix<'a, T: PartialEq>(
    items: &'a [T],
    prefix: &[T],
) -> Option<&'a [T]> {
    crate::slice_primitives::strip_prefix(items, prefix)
}

pub(crate) fn slice_strip_suffix<'a, T: PartialEq>(
    items: &'a [T],
    suffix: &[T],
) -> Option<&'a [T]> {
    crate::slice_primitives::strip_suffix(items, suffix)
}

#[allow(dead_code)]
pub(crate) fn slice_strip_any_prefix<'a, 'p, T: PartialEq>(
    items: &'a [T],
    patterns: &'p [&'p [T]],
) -> Option<(&'p [T], &'a [T])> {
    crate::slice_primitives::strip_any_prefix(items, patterns)
}

#[allow(dead_code)]
pub(crate) fn slice_strip_any_suffix<'a, 'p, T: PartialEq>(
    items: &'a [T],
    patterns: &'p [&'p [T]],
) -> Option<(&'p [T], &'a [T])> {
    crate::slice_primitives::strip_any_suffix(items, patterns)
}

pub(crate) fn find_index<T>(items: &[T], predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    crate::slice_primitives::find_index(items, predicate)
}

pub(crate) fn find_index_with<T>(
    items: &[T],
    predicate: impl FnMut(usize, &T) -> bool,
) -> Option<usize> {
    crate::slice_primitives::find_index_with(items, predicate)
}

pub(crate) fn rfind_index<T>(items: &[T], predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    crate::slice_primitives::rfind_index(items, predicate)
}

pub(crate) fn rfind_index_with<T>(
    items: &[T],
    predicate: impl FnMut(usize, &T) -> bool,
) -> Option<usize> {
    crate::slice_primitives::rfind_index_with(items, predicate)
}

pub(crate) fn find_window_index<T: PartialEq>(items: &[T], window: &[T]) -> Option<usize> {
    crate::slice_primitives::find_window_index(items, window)
}

pub(crate) fn find_window_by<T>(
    items: &[T],
    window_len: usize,
    predicate: impl FnMut(&[T]) -> bool,
) -> Option<usize> {
    crate::slice_primitives::find_window_by(items, window_len, predicate)
}

pub(crate) fn contains_sequence<T: PartialEq>(items: &[T], window: &[T]) -> bool {
    crate::slice_primitives::contains_sequence(items, window)
}

pub(crate) use contains_sequence as contains_window;

pub(crate) fn str_contains(text: &str, needle: &str) -> bool {
    crate::string_primitives::contains(text, needle)
}

pub(crate) fn str_contains_char(text: &str, needle: char) -> bool {
    crate::string_primitives::contains_char(text, needle)
}

pub(crate) fn str_starts_with(text: &str, prefix: &str) -> bool {
    crate::string_primitives::starts_with(text, prefix)
}

pub(crate) fn str_starts_with_char(text: &str, expected: char) -> bool {
    crate::string_primitives::starts_with_char(text, expected)
}

pub(crate) fn str_ends_with_char(text: &str, expected: char) -> bool {
    crate::string_primitives::ends_with_char(text, expected)
}

pub(crate) fn str_ends_with_any_char(text: &str, expected: &[char]) -> bool {
    crate::string_primitives::ends_with_any_char(text, expected)
}

pub(crate) fn str_find(text: &str, needle: &str) -> Option<usize> {
    crate::string_primitives::find(text, needle)
}

pub(crate) fn str_find_char(text: &str, needle: char) -> Option<usize> {
    crate::string_primitives::find_char(text, needle)
}

pub(crate) fn str_rfind(text: &str, needle: &str) -> Option<usize> {
    crate::string_primitives::rfind(text, needle)
}

#[allow(dead_code)]
pub(crate) fn str_rfind_char(text: &str, needle: char) -> Option<usize> {
    crate::string_primitives::rfind_char(text, needle)
}

pub(crate) fn str_strip_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    crate::string_primitives::strip_prefix(text, prefix)
}

pub(crate) fn str_strip_suffix<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    crate::string_primitives::strip_suffix(text, suffix)
}

pub(crate) fn str_strip_suffix_char(text: &str, suffix: char) -> Option<&str> {
    crate::string_primitives::strip_suffix_char(text, suffix)
}

pub(crate) fn is_ticket_symbol_cost_text(cost: &str) -> bool {
    let mut saw_ticket_symbol = false;
    let mut remainder = cost.trim();
    while let Some(next) = str_strip_prefix(remainder, TICKET_SYMBOL_TEXT) {
        saw_ticket_symbol = true;
        remainder = next.trim_start();
    }

    saw_ticket_symbol && remainder.is_empty()
}

pub(crate) fn is_ticket_sticker_marker_text(text: &str) -> bool {
    let Some((cost, body_text)) = str_split_once_char(text, '—') else {
        return false;
    };

    is_ticket_symbol_cost_text(cost) && !body_text.trim().is_empty()
}

pub(crate) fn is_core_keyword_marker_text(text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    CORE_KEYWORD_MARKER_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
        || is_ticket_sticker_marker_text(&text)
}

pub(crate) fn is_static_keyword_marker_text(text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    text == COMPLEATED_MARKER_TEXT
        || is_core_keyword_marker_text(&text)
        || STATIC_KEYWORD_MARKER_EXTRA_PREFIXES
            .iter()
            .any(|prefix| text.starts_with(prefix))
}

pub(crate) fn str_split_once<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    crate::string_primitives::split_once(text, needle)
}

pub(crate) fn str_split_once_char<'a>(text: &'a str, needle: char) -> Option<(&'a str, &'a str)> {
    crate::string_primitives::split_once_char(text, needle)
}

pub(crate) fn parse_lexed_prefix<'a, O>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<LexedInput<'a>, O, ErrMode<ContextError>>,
) -> Option<(O, &'a [OwnedLexToken])> {
    grammar::parse_prefix(tokens, parser)
}

pub(crate) fn parse_word_token<'a>(input: &mut LexedInput<'a>) -> WResult<&'a str> {
    grammar::word_text(input)
}

pub(crate) fn parse_word_eq<'a>(
    expected: &'static str,
) -> impl Parser<LexedInput<'a>, (), ErrMode<ContextError>> {
    grammar::kw(expected).void()
}

pub(crate) fn parse_word_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexedInput<'a>, (), ErrMode<ContextError>> {
    grammar::phrase(expected)
}

pub(crate) fn word_view_has_prefix(words: &TokenWordView<'_>, prefix: &[&str]) -> bool {
    words.starts_with(prefix)
}

pub(crate) fn word_view_has_any_prefix(words: &TokenWordView<'_>, prefixes: &[&[&str]]) -> bool {
    words.starts_with_any(prefixes)
}

pub(crate) fn rewrite_followup_intro_to_if_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut rewritten = tokens.to_vec();
    let words = TokenWordView::new(&rewritten);
    if !word_view_has_any_prefix(
        &words,
        &[
            &["when", "you", "do"],
            &["whenever", "you", "do"],
            &["when", "it", "connives", "this", "way"],
            &["when", "it", "connive", "this", "way"],
            &["whenever", "it", "connives", "this", "way"],
            &["whenever", "it", "connive", "this", "way"],
        ],
    ) {
        return rewritten;
    }

    let Some(first_word_idx) = words.token_index_for_word_index(0) else {
        return rewritten;
    };
    rewritten[first_word_idx].replace_word("if");
    rewritten
}

fn token_range_for_word_span(
    tokens: &[OwnedLexToken],
    words: &TokenWordView<'_>,
    start_word_idx: usize,
    word_len: usize,
) -> Option<(usize, usize)> {
    let start = if start_word_idx == 0 {
        0
    } else {
        words.token_index_after_words(start_word_idx)?
    };
    let end = words.token_index_after_words(start_word_idx + word_len)?;
    (start <= end && end <= tokens.len()).then_some((start, end))
}

pub(crate) fn remove_copy_exception_type_removal_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    const PATTERNS: &[(&[&str], usize)] = &[
        (
            &[
                "except", "its", "an", "artifact", "and", "it", "loses", "all", "other", "card",
                "types",
            ],
            4,
        ),
        (
            &[
                "except",
                "its",
                "an",
                "enchantment",
                "and",
                "it",
                "loses",
                "all",
                "other",
                "card",
                "types",
            ],
            4,
        ),
        (
            &[
                "except",
                "its",
                "an",
                "enchantment",
                "and",
                "loses",
                "all",
                "other",
                "card",
                "types",
            ],
            4,
        ),
    ];

    let mut rewritten = tokens.to_vec();
    loop {
        let words = TokenWordView::new(&rewritten);
        let mut removed_any = false;
        for (pattern, keep_words) in PATTERNS {
            let Some(start_word_idx) = words.find_phrase_start(pattern) else {
                continue;
            };
            let Some((remove_start, remove_end)) = token_range_for_word_span(
                &rewritten,
                &words,
                start_word_idx + keep_words,
                pattern.len() - keep_words,
            ) else {
                continue;
            };
            rewritten.drain(remove_start..remove_end);
            removed_any = true;
            break;
        }
        if !removed_any {
            break;
        }
    }
    rewritten
}

pub(crate) fn lexed_tokens_contain_non_prefix_instead(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.find_word("instead").is_some() && !word_view_has_prefix(&words, &["if"])
}

pub(crate) fn strip_leading_if_you_do_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let words = TokenWordView::new(tokens);
    let Some(prefix_len) = (word_view_has_prefix(&words, &["if", "you", "do"]).then_some(3usize))
        .or_else(|| word_view_has_prefix(&words, &["if", "they", "do"]).then_some(3usize))
        .or_else(|| {
            word_view_has_prefix(&words, &["if", "that", "player", "does"]).then_some(4usize)
        })
        .or_else(|| {
            word_view_has_prefix(&words, &["if", "the", "player", "does"]).then_some(4usize)
        })
    else {
        return tokens;
    };
    let start = words
        .token_index_after_words(prefix_len)
        .unwrap_or(tokens.len());
    &tokens[start..]
}

fn find_token_index_with_span(tokens: &[OwnedLexToken], span: TextSpan) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx].span == span {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub(crate) fn clone_sentence_chunk_tokens(
    tokens: &[OwnedLexToken],
    sentences: &[&[OwnedLexToken]],
) -> Option<Vec<OwnedLexToken>> {
    let first = sentences.first()?.first()?;
    let last_sentence = sentences.last()?;
    let last_first = last_sentence.first()?;
    let start = find_token_index_with_span(tokens, first.span)?;
    let end_start = find_token_index_with_span(tokens, last_first.span)?;
    Some(tokens[start..end_start + last_sentence.len()].to_vec())
}

pub(crate) fn split_em_dash_label_prefix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let mut inside_quotes = false;
    let split_idx = tokens.iter().enumerate().find_map(|(idx, token)| {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            return None;
        }
        (token.kind == TokenKind::EmDash && !inside_quotes).then_some(idx)
    })?;
    let label_tokens = &tokens[..split_idx];
    let body_tokens = &tokens[split_idx + 1..];
    if label_tokens.is_empty()
        || body_tokens.is_empty()
        || contains_token_kind(label_tokens, TokenKind::Period)
    {
        return None;
    }

    let body = render_token_slice(body_tokens);
    if body.trim().is_empty() {
        return None;
    }

    Some((label_tokens, body_tokens))
}

pub(crate) fn split_em_dash_label_prefix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(String, &'a [OwnedLexToken])> {
    let (label_tokens, body_tokens) = split_em_dash_label_prefix_tokens(tokens)?;
    let label = render_token_slice(label_tokens).trim().to_string();
    (!label.is_empty()).then_some((label, body_tokens))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeadingMayActor {
    You,
    ThatPlayer,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingMayActionMatch<'a> {
    pub(crate) actor: LeadingMayActor,
    pub(crate) verb: &'static str,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_leading_may_action_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    verbs: &'static [&'static str],
    allow_bare: bool,
) -> Option<LeadingMayActionMatch<'a>> {
    let words = TokenWordView::new(tokens);
    if words.is_empty() {
        return None;
    }

    for (actor, verb_word_idx, prefix) in [
        (LeadingMayActor::You, 2usize, &["you", "may"][..]),
        (
            LeadingMayActor::ThatPlayer,
            3usize,
            &["that", "player", "may"][..],
        ),
        (LeadingMayActor::ThatPlayer, 2usize, &["they", "may"][..]),
        (LeadingMayActor::Default, 1usize, &["may"][..]),
    ] {
        if !word_view_has_prefix(&words, prefix) {
            continue;
        }
        for verb in verbs {
            if !words.slice_eq(verb_word_idx, &[*verb]) {
                continue;
            }
            let tail_start = words
                .token_index_after_words(verb_word_idx + 1)
                .unwrap_or(tokens.len());
            return Some(LeadingMayActionMatch {
                actor,
                verb,
                tail_tokens: &tokens[tail_start..],
            });
        }
        return None;
    }

    if allow_bare {
        for verb in verbs {
            if !words.slice_eq(0, &[*verb]) {
                continue;
            }
            let tail_start = words.token_index_after_words(1).unwrap_or(tokens.len());
            return Some(LeadingMayActionMatch {
                actor: LeadingMayActor::Default,
                verb,
                tail_tokens: &tokens[tail_start..],
            });
        }
    }

    None
}

fn parse_head_words<'a>(input: &mut LexedInput<'a>) -> WResult<(&'a str, Option<&'a str>)> {
    peek(seq!(parse_word_token, opt(parse_word_token))).parse_next(input)
}

pub(crate) fn lexed_head_words(tokens: &[OwnedLexToken]) -> Option<(&str, Option<&str>)> {
    parse_lexed_prefix(tokens, parse_head_words).map(|(head, _)| head)
}

#[allow(dead_code)]
fn parse_common_sentence_head_inner<'a>(input: &mut LexedInput<'a>) -> WResult<CommonSentenceHead> {
    use CommonSentenceHead::{CountPrefix, ForEach, If, Target, Until, WhereXIs};

    dispatch! {peek(grammar::word_parser_text);
        "for" => parse_word_phrase(&["for", "each"]).value(ForEach),
        "each" => parse_word_eq("each").value(ForEach),
        "if" => parse_word_eq("if").value(If),
        "until" => parse_word_eq("until").value(Until),
        "where" => parse_word_phrase(&["where", "x", "is"]).value(WhereXIs),
        "target" => parse_word_eq("target").value(Target),
        "up" => parse_word_phrase(&["up", "to"]).value(CountPrefix),
        "one" => alt((
            parse_word_phrase(&["one", "or", "more"]),
            parse_word_phrase(&["one", "or", "both"]),
        ))
        .value(CountPrefix),
        "a" => parse_word_eq("a").value(CountPrefix),
        "an" => parse_word_eq("an").value(CountPrefix),
        _ => fail::<_, CommonSentenceHead, _>,
    }
    .parse_next(input)
}

#[allow(dead_code)]
pub(crate) fn parse_common_sentence_head(
    tokens: &[OwnedLexToken],
) -> Option<(CommonSentenceHead, &[OwnedLexToken])> {
    parse_lexed_prefix(tokens, parse_common_sentence_head_inner)
}

#[allow(dead_code)]
pub(crate) fn split_lexed_once_on_delimiter(
    tokens: &[OwnedLexToken],
    delimiter: TokenKind,
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    grammar::split_lexed_once_on_delimiter(tokens, delimiter)
}

#[allow(dead_code)]
pub(crate) fn split_lexed_once_on_comma(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
}

#[allow(dead_code)]
pub(crate) fn split_lexed_once_on_period(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_lexed_once_on_delimiter(tokens, TokenKind::Period)
}

pub(crate) fn parse_i32_word_token<'a>(input: &mut LexedInput<'a>) -> WResult<i32> {
    let word = parse_word_token.parse_next(input)?;
    word.parse::<i32>()
        .map_err(|_| grammar::backtrack_err("integer word", "integer"))
}

fn parse_turn_duration_phrase_inner<'a>(input: &mut LexedInput<'a>) -> WResult<TurnDurationPhrase> {
    dispatch! {peek(grammar::word_parser_text);
        "until" => alt((
            grammar::phrase(&["until", "your", "next", "turn"])
                .value(TurnDurationPhrase::UntilYourNextTurn),
            grammar::phrase(&["until", "your", "next", "end", "step"])
                .value(TurnDurationPhrase::UntilYourNextTurnEnd),
            grammar::phrase(&["until", "the", "end", "of", "your", "next", "turn"])
                .value(TurnDurationPhrase::UntilYourNextTurnEnd),
            grammar::phrase(&["until", "end", "of", "your", "next", "turn"])
                .value(TurnDurationPhrase::UntilYourNextTurnEnd),
            grammar::phrase(&["until", "the", "end", "of", "turn"])
                .value(TurnDurationPhrase::UntilEndOfTurn),
            grammar::phrase(&["until", "end", "of", "turn"])
                .value(TurnDurationPhrase::UntilEndOfTurn),
        )),
        "this" => grammar::phrase(&["this", "turn"]).value(TurnDurationPhrase::ThisTurn),
        _ => fail::<_, TurnDurationPhrase, _>,
    }
    .parse_next(input)
}

fn turn_duration_from_suffix_phrase(phrase: &[&str]) -> Option<TurnDurationPhrase> {
    match phrase {
        ["until", "your", "next", "turn"] => Some(TurnDurationPhrase::UntilYourNextTurn),
        ["until", "your", "next", "end", "step"] => Some(TurnDurationPhrase::UntilYourNextTurnEnd),
        ["until", "the", "end", "of", "your", "next", "turn"]
        | ["until", "end", "of", "your", "next", "turn"] => {
            Some(TurnDurationPhrase::UntilYourNextTurnEnd)
        }
        ["until", "the", "end", "of", "turn"] | ["until", "end", "of", "turn"] => {
            Some(TurnDurationPhrase::UntilEndOfTurn)
        }
        ["this", "turn"] => Some(TurnDurationPhrase::ThisTurn),
        _ => None,
    }
}

pub(crate) fn parse_turn_duration_prefix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(TurnDurationPhrase, &'a [OwnedLexToken])> {
    parse_lexed_prefix(tokens, parse_turn_duration_phrase_inner)
}

pub(crate) fn parse_turn_duration_suffix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], TurnDurationPhrase)> {
    let phrases = [
        &["until", "your", "next", "turn"][..],
        &["until", "your", "next", "end", "step"][..],
        &["until", "the", "end", "of", "your", "next", "turn"][..],
        &["until", "end", "of", "your", "next", "turn"][..],
        &["until", "the", "end", "of", "turn"][..],
        &["until", "end", "of", "turn"][..],
        &["this", "turn"][..],
    ];
    let (phrase, rest) = grammar::strip_lexed_suffix_phrases(tokens, &phrases)?;
    Some((rest, turn_duration_from_suffix_phrase(phrase)?))
}

fn parse_simple_restriction_duration_prefix_inner<'a>(
    input: &mut LexedInput<'a>,
) -> WResult<Until> {
    dispatch! {peek(grammar::word_parser_text);
        "until" => alt((
            grammar::phrase(&["until", "your", "next", "upkeep"]).value(Until::YourNextUpkeep),
            grammar::phrase(&["until", "the", "end", "of", "combat"]).value(Until::EndOfCombat),
            grammar::phrase(&["until", "end", "of", "combat"]).value(Until::EndOfCombat),
            parse_turn_duration_phrase_inner.map(until_from_turn_duration_phrase),
        )),
        "this" => grammar::phrase(&["this", "turn"])
            .value(TurnDurationPhrase::ThisTurn)
            .map(until_from_turn_duration_phrase),
        _ => fail::<_, Until, _>,
    }
    .parse_next(input)
}

fn simple_restriction_duration_from_suffix_phrase(phrase: &[&str]) -> Option<Until> {
    match phrase {
        ["until", "the", "end", "of", "combat"] | ["until", "end", "of", "combat"] => {
            Some(Until::EndOfCombat)
        }
        ["during", "your", "next", "untap", "step"]
        | ["during", "its", "controller", "next", "untap", "step"]
        | ["during", "its", "controllers", "next", "untap", "step"]
        | ["during", "their", "controller", "next", "untap", "step"]
        | ["during", "their", "controllers", "next", "untap", "step"] => {
            Some(Until::ControllersNextUntapStep)
        }
        ["for", "the", "rest", "of", "the", "game"] => Some(Until::Forever),
        ["until", "your", "next", "upkeep"] => Some(Until::YourNextUpkeep),
        _ => turn_duration_from_suffix_phrase(phrase).map(until_from_turn_duration_phrase),
    }
}

pub(crate) fn parse_simple_restriction_duration_prefix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(Until, &'a [OwnedLexToken])> {
    parse_lexed_prefix(tokens, parse_simple_restriction_duration_prefix_inner)
}

pub(crate) fn parse_simple_restriction_duration_suffix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], Until)> {
    let phrases = [
        &["until", "your", "next", "turn"][..],
        &["until", "your", "next", "end", "step"][..],
        &["until", "your", "next", "upkeep"][..],
        &["until", "the", "end", "of", "your", "next", "turn"][..],
        &["until", "end", "of", "your", "next", "turn"][..],
        &["until", "the", "end", "of", "turn"][..],
        &["until", "end", "of", "turn"][..],
        &["this", "turn"][..],
        &["until", "the", "end", "of", "combat"][..],
        &["until", "end", "of", "combat"][..],
        &["during", "your", "next", "untap", "step"][..],
        &["during", "its", "controller", "next", "untap", "step"][..],
        &["during", "its", "controllers", "next", "untap", "step"][..],
        &["during", "their", "controller", "next", "untap", "step"][..],
        &["during", "their", "controllers", "next", "untap", "step"][..],
        &["for", "the", "rest", "of", "the", "game"][..],
    ];
    let (phrase, rest) = grammar::strip_lexed_suffix_phrases(tokens, &phrases)?;
    Some((
        rest,
        simple_restriction_duration_from_suffix_phrase(phrase)?,
    ))
}
