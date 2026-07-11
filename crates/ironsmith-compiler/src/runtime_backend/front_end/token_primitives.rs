use winnow::combinator::{alt, dispatch, fail, opt, peek, seq};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;

use crate::cards::builders::TextSpan;
use crate::effect::Until;

use super::grammar::leaf::{
    LeafDurationPhrase, LeafTurnDurationPhrase, parse_leaf_restriction_duration_prefix_tokens,
    parse_leaf_restriction_duration_suffix_tokens, parse_leaf_turn_duration_prefix_tokens,
    parse_leaf_turn_duration_suffix_tokens,
};
use super::grammar::primitives as grammar;
use super::grammar::sentence_markers;
use super::grammar::static_keyword_line_shapes;
#[cfg(test)]
pub(crate) use super::grammar::values::parse_count_range_prefix;
pub(crate) use super::grammar::values::{
    parse_mana_symbol, parse_mana_symbol_group, parse_modal_choose_range, parse_scryfall_mana_cost,
    parse_type_line_with, parse_value_comparison_tokens,
};
use super::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, contains_token_kind, render_token_slice,
};
pub(crate) type LexedInput<'a> = LexStream<'a>;

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

fn until_from_leaf_duration(duration: LeafDurationPhrase) -> Until {
    match duration {
        LeafDurationPhrase::ThisTurn | LeafDurationPhrase::UntilEndOfTurn => Until::EndOfTurn,
        LeafDurationPhrase::UntilEndOfCombat => Until::EndOfCombat,
        LeafDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        LeafDurationPhrase::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
        LeafDurationPhrase::UntilYourNextUpkeep => Until::YourNextUpkeep,
        LeafDurationPhrase::ControllersNextUntapStep => Until::ControllersNextUntapStep,
        LeafDurationPhrase::Forever => Until::Forever,
    }
}

pub(crate) fn items_start_with<T: PartialEq>(items: &[T], prefix: &[T]) -> bool {
    crate::slice_primitives::starts_with(items, prefix)
}

pub(crate) fn items_end_with<T: PartialEq>(items: &[T], suffix: &[T]) -> bool {
    crate::slice_primitives::ends_with(items, suffix)
}

#[allow(dead_code)]
pub(crate) fn items_end_with_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    crate::slice_primitives::ends_with_any(items, patterns)
}

pub(crate) fn items_have<T: PartialEq>(items: &[T], expected: &T) -> bool {
    crate::slice_primitives::contains(items, expected)
}

#[allow(dead_code)]
pub(crate) fn items_have_any<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    crate::slice_primitives::contains_any(items, expected)
}

#[allow(dead_code)]
pub(crate) fn items_have_all<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
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
pub(crate) fn items_start_with_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
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

pub(crate) use crate::slice_primitives::iter_eq;

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

pub(crate) fn locate_index<T>(items: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    for (idx, item) in items.iter().enumerate() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn locate_index_with<T>(
    items: &[T],
    mut predicate: impl FnMut(usize, &T) -> bool,
) -> Option<usize> {
    for (idx, item) in items.iter().enumerate() {
        if predicate(idx, item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn locate_last_index<T>(
    items: &[T],
    mut predicate: impl FnMut(&T) -> bool,
) -> Option<usize> {
    let mut idx = items.len();
    while idx > 0 {
        idx -= 1;
        if predicate(&items[idx]) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn locate_window_index<T: PartialEq>(items: &[T], window: &[T]) -> Option<usize> {
    if window.is_empty() {
        return Some(0);
    }
    if window.len() > items.len() {
        return None;
    }
    let mut idx = 0usize;
    while idx + window.len() <= items.len() {
        let end = idx + window.len();
        if items
            .get(idx..end)
            .is_some_and(|candidate| candidate == window)
        {
            return Some(idx);
        }
        idx += 1;
    }
    None
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

#[cfg(test)]
pub(crate) use crate::string_primitives::contains as str_contains;
pub(crate) use sentence_markers::recognizes_core_keyword_marker as is_core_keyword_marker_text;
pub(crate) use sentence_markers::recognizes_ticket_sticker_marker as is_ticket_sticker_marker_text;

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

pub(crate) use sentence_markers::parse_any_word_prefix_presence as word_view_has_any_prefix;
pub(crate) use sentence_markers::parse_word_prefix_presence as word_view_has_prefix;

pub(crate) fn rewrite_followup_intro_to_if_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    // Preserve reflexive "when" prefixes so lowering emits a stack-queued
    // ReflexiveTriggerEffect instead of an inline IfEffect.
    tokens.to_vec()
}

pub(crate) fn remove_copy_exception_type_removal_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let mut rewritten = tokens.to_vec();
    while let Some(span) =
        static_keyword_line_shapes::parse_copy_exception_type_removal_span(&rewritten)
    {
        rewritten.drain(span.start..span.end);
    }
    rewritten
}

pub(crate) fn lexed_tokens_contain_non_prefix_instead(tokens: &[OwnedLexToken]) -> bool {
    sentence_markers::has_nonconditional_instead(tokens)
}

pub(crate) fn strip_leading_if_you_do_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    sentence_markers::parse_conditional_followup_tokens(tokens)
        .map(|matched| matched.tail_tokens)
        .unwrap_or(tokens)
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
        (matches!(token.kind, TokenKind::Dash | TokenKind::EmDash) && !inside_quotes).then_some(idx)
    })?;
    let label_tokens = &tokens[..split_idx];
    let body_tokens = &tokens[split_idx + 1..];
    if label_tokens.is_empty()
        || body_tokens.is_empty()
        || label_has_disallowed_period(label_tokens)
    {
        return None;
    }

    let body = render_token_slice(body_tokens);
    if body.trim().is_empty() {
        return None;
    }

    Some((label_tokens, body_tokens))
}

fn label_has_disallowed_period(tokens: &[OwnedLexToken]) -> bool {
    if !contains_token_kind(tokens, TokenKind::Period) {
        return false;
    }

    let mut first_non_period = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Period {
            first_non_period = Some(idx);
            break;
        }
    }
    let Some(first_non_period) = first_non_period else {
        return true;
    };

    let mut last_non_period = None;
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if tokens[idx].kind != TokenKind::Period {
            last_non_period = Some(idx);
            break;
        }
    }
    let Some(last_non_period) = last_non_period else {
        return true;
    };

    let mut idx = first_non_period;
    while idx <= last_non_period {
        if tokens[idx].kind == TokenKind::Period {
            return true;
        }
        idx += 1;
    }
    false
}

pub(crate) fn split_em_dash_label_prefix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(String, &'a [OwnedLexToken])> {
    let (label_tokens, body_tokens) = split_em_dash_label_prefix_tokens(tokens)?;
    let label = render_token_slice(label_tokens).trim().to_string();
    (!label.is_empty()).then_some((label, body_tokens))
}

pub(crate) use sentence_markers::{LeadingMayActionMatch, LeadingMayActor};

pub(crate) fn parse_leading_may_action_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    verbs: &'static [&'static str],
    allow_bare: bool,
) -> Option<LeadingMayActionMatch<'a>> {
    sentence_markers::parse_leading_may_action_tokens(tokens, verbs, allow_bare)
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

fn turn_duration_from_leaf(duration: LeafTurnDurationPhrase) -> TurnDurationPhrase {
    match duration {
        LeafTurnDurationPhrase::ThisTurn => TurnDurationPhrase::ThisTurn,
        LeafTurnDurationPhrase::UntilEndOfTurn => TurnDurationPhrase::UntilEndOfTurn,
        LeafTurnDurationPhrase::UntilYourNextTurn => TurnDurationPhrase::UntilYourNextTurn,
        LeafTurnDurationPhrase::UntilYourNextTurnEnd => TurnDurationPhrase::UntilYourNextTurnEnd,
    }
}

pub(crate) fn parse_turn_duration_prefix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(TurnDurationPhrase, &'a [OwnedLexToken])> {
    let parsed = parse_leaf_turn_duration_prefix_tokens(tokens)?;
    Some((turn_duration_from_leaf(parsed.duration), parsed.rest))
}

pub(crate) fn parse_turn_duration_suffix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], TurnDurationPhrase)> {
    let parsed = parse_leaf_turn_duration_suffix_tokens(tokens)?;
    Some((parsed.rest, turn_duration_from_leaf(parsed.duration)))
}

pub(crate) fn parse_simple_restriction_duration_prefix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(Until, &'a [OwnedLexToken])> {
    let parsed = parse_leaf_restriction_duration_prefix_tokens(tokens)?;
    Some((until_from_leaf_duration(parsed.duration), parsed.rest))
}

pub(crate) fn parse_simple_restriction_duration_suffix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], Until)> {
    let parsed = parse_leaf_restriction_duration_suffix_tokens(tokens)?;
    Some((parsed.rest, until_from_leaf_duration(parsed.duration)))
}
