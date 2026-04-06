use std::borrow::Borrow;

use winnow::combinator::{alt, dispatch, fail, opt, peek, seq};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;

use crate::cards::builders::TextSpan;
use crate::effect::Until;

use super::grammar::primitives as grammar;
pub(crate) use super::grammar::values::{
    parse_count_range_prefix, parse_mana_symbol, parse_mana_symbol_group, parse_modal_choose_range,
    parse_scryfall_mana_cost, parse_type_line_with, parse_value_comparison_tokens,
};
use super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView, render_token_slice};
pub(crate) type LexedInput<'a> = LexStream<'a>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnDurationPhrase {
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
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
    }
}

pub(crate) fn slice_starts_with<T: PartialEq>(items: &[T], prefix: &[T]) -> bool {
    items.len() >= prefix.len() && items[..prefix.len()] == *prefix
}

pub(crate) fn slice_ends_with<T: PartialEq>(items: &[T], suffix: &[T]) -> bool {
    items.len() >= suffix.len() && items[items.len() - suffix.len()..] == *suffix
}

pub(crate) fn slice_contains<T: PartialEq>(items: &[T], expected: &T) -> bool {
    items.iter().any(|item| item == expected)
}

pub(crate) fn slice_contains_str(items: &[&str], expected: &str) -> bool {
    items.iter().any(|item| *item == expected)
}

pub(crate) fn slice_contains_any<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    expected
        .iter()
        .any(|candidate| slice_contains(items, candidate))
}

pub(crate) fn slice_contains_all<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    expected
        .iter()
        .all(|candidate| slice_contains(items, candidate))
}

pub(crate) fn iter_contains<I, T>(items: I, expected: &T) -> bool
where
    I: IntoIterator,
    I::Item: Borrow<T>,
    T: PartialEq + ?Sized,
{
    items.into_iter().any(|item| item.borrow() == expected)
}

pub(crate) fn slice_strip_prefix<'a, T: PartialEq>(
    items: &'a [T],
    prefix: &[T],
) -> Option<&'a [T]> {
    slice_starts_with(items, prefix).then(|| &items[prefix.len()..])
}

pub(crate) fn slice_strip_suffix<'a, T: PartialEq>(
    items: &'a [T],
    suffix: &[T],
) -> Option<&'a [T]> {
    slice_ends_with(items, suffix).then(|| &items[..items.len() - suffix.len()])
}

pub(crate) fn find_index<T>(items: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    for (idx, item) in items.iter().enumerate() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn find_str_index(items: &[&str], expected: &str) -> Option<usize> {
    find_index(items, |item| *item == expected)
}

pub(crate) fn find_str_by(
    items: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    for (idx, item) in items.iter().enumerate() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn find_any_str_index(items: &[&str], expected: &[&str]) -> Option<usize> {
    find_index(items, |item| {
        expected.iter().any(|candidate| *item == *candidate)
    })
}

pub(crate) fn rfind_index<T>(items: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    for (idx, item) in items.iter().enumerate().rev() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn rfind_str_by(
    items: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    for (idx, item) in items.iter().enumerate().rev() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn find_window_index<T: PartialEq>(items: &[T], window: &[T]) -> Option<usize> {
    if window.is_empty() {
        return Some(0);
    }
    if items.len() < window.len() {
        return None;
    }
    let mut start = 0usize;
    while start + window.len() <= items.len() {
        if items[start..start + window.len()] == *window {
            return Some(start);
        }
        start += 1;
    }
    None
}

pub(crate) fn find_window_by<T>(
    items: &[T],
    window_len: usize,
    mut predicate: impl FnMut(&[T]) -> bool,
) -> Option<usize> {
    if window_len == 0 {
        return Some(0);
    }
    if items.len() < window_len {
        return None;
    }
    let mut start = 0usize;
    while start + window_len <= items.len() {
        if predicate(&items[start..start + window_len]) {
            return Some(start);
        }
        start += 1;
    }
    None
}

pub(crate) fn contains_window<T: PartialEq>(items: &[T], window: &[T]) -> bool {
    find_window_index(items, window).is_some()
}

pub(crate) fn str_contains(text: &str, needle: &str) -> bool {
    text.contains(needle)
}

pub(crate) fn str_starts_with(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

pub(crate) fn str_starts_with_char(text: &str, expected: char) -> bool {
    text.starts_with(expected)
}

pub(crate) fn str_ends_with(text: &str, suffix: &str) -> bool {
    text.ends_with(suffix)
}

pub(crate) fn str_ends_with_char(text: &str, expected: char) -> bool {
    text.ends_with(expected)
}

pub(crate) fn str_find(text: &str, needle: &str) -> Option<usize> {
    text.find(needle)
}

pub(crate) fn str_find_char(text: &str, needle: char) -> Option<usize> {
    text.find(needle)
}

pub(crate) fn str_strip_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.strip_prefix(prefix)
}

pub(crate) fn str_strip_suffix<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    text.strip_suffix(suffix)
}

pub(crate) fn str_split_once<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    text.split_once(needle)
}

pub(crate) fn str_split_once_char<'a>(text: &'a str, needle: char) -> Option<(&'a str, &'a str)> {
    text.split_once(needle)
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
    grammar::kw(expected).map(|_| ())
}

pub(crate) fn parse_word_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexedInput<'a>, (), ErrMode<ContextError>> {
    grammar::phrase(expected)
}

pub(crate) fn word_view_has_prefix(words: &TokenWordView, prefix: &[&str]) -> bool {
    words.len() >= prefix.len() && words.slice_eq(0, prefix)
}

pub(crate) fn word_view_has_any_prefix(words: &TokenWordView, prefixes: &[&[&str]]) -> bool {
    prefixes
        .iter()
        .any(|prefix| word_view_has_prefix(words, prefix))
}

pub(crate) fn rewrite_followup_intro_to_if_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut rewritten = tokens.to_vec();
    let words = TokenWordView::new(&rewritten);
    if !word_view_has_any_prefix(
        &words,
        &[&["when", "you", "do"], &["whenever", "you", "do"]],
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
    words: &TokenWordView,
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
    let (label_tokens, body_tokens) =
        grammar::split_lexed_once_on_delimiter(tokens, TokenKind::EmDash)?;
    if label_tokens.is_empty()
        || body_tokens.is_empty()
        || label_tokens.iter().any(OwnedLexToken::is_period)
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

    dispatch! {peek(parse_word_token);
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

pub(crate) fn split_lexed_once_on_comma_then(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    grammar::split_lexed_once_on_separator(tokens, || {
        (grammar::comma(), grammar::kw("then")).void()
    })
}

pub(crate) fn parse_i32_word_token<'a>(input: &mut LexedInput<'a>) -> WResult<i32> {
    let word = parse_word_token.parse_next(input)?;
    word.parse::<i32>().map_err(|_| {
        let mut err = ContextError::new();
        err.push(StrContext::Label("integer word"));
        err.push(StrContext::Expected(StrContextValue::Description(
            "integer",
        )));
        ErrMode::Backtrack(err)
    })
}

fn parse_turn_duration_phrase_inner<'a>(input: &mut LexedInput<'a>) -> WResult<TurnDurationPhrase> {
    alt((
        grammar::phrase(&["until", "your", "next", "turn"])
            .value(TurnDurationPhrase::UntilYourNextTurn),
        grammar::phrase(&["until", "the", "end", "of", "your", "next", "turn"])
            .value(TurnDurationPhrase::UntilYourNextTurn),
        grammar::phrase(&["until", "end", "of", "your", "next", "turn"])
            .value(TurnDurationPhrase::UntilYourNextTurn),
        grammar::phrase(&["until", "the", "end", "of", "turn"])
            .value(TurnDurationPhrase::UntilEndOfTurn),
        grammar::phrase(&["until", "end", "of", "turn"]).value(TurnDurationPhrase::UntilEndOfTurn),
        grammar::phrase(&["this", "turn"]).value(TurnDurationPhrase::ThisTurn),
    ))
    .parse_next(input)
}

fn turn_duration_from_suffix_phrase(phrase: &[&str]) -> Option<TurnDurationPhrase> {
    match phrase {
        ["until", "your", "next", "turn"]
        | ["until", "the", "end", "of", "your", "next", "turn"]
        | ["until", "end", "of", "your", "next", "turn"] => {
            Some(TurnDurationPhrase::UntilYourNextTurn)
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
    alt((
        parse_turn_duration_phrase_inner.map(until_from_turn_duration_phrase),
        grammar::phrase(&["until", "the", "end", "of", "combat"]).value(Until::EndOfCombat),
        grammar::phrase(&["until", "end", "of", "combat"]).value(Until::EndOfCombat),
    ))
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
