use winnow::combinator::{opt, peek, seq};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;

use crate::cards::builders::TextSpan;
use crate::effect::Until;

#[cfg(test)]
use super::grammar::leaf::parse_leaf_turn_duration_prefix_tokens;
use super::grammar::leaf::{
    LeafDurationPhrase, LeafTurnDurationPhrase, parse_leaf_restriction_duration_prefix_tokens,
    parse_leaf_restriction_duration_suffix_tokens, parse_leaf_turn_duration_suffix_tokens,
};
use super::grammar::primitives as grammar;
use super::grammar::sentence_markers;
use super::grammar::static_keyword_line_shapes;
#[cfg(test)]
pub use super::grammar::values::parse_value_comparison_tokens;
pub use super::grammar::values::{parse_mana_symbol, parse_scryfall_mana_cost};
use super::lexer::{LexStream, OwnedLexToken, TokenKind, contains_token_kind, render_token_slice};
pub type LexedInput<'a> = LexStream<'a>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDurationPhrase {
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
    UntilYourNextTurnEnd,
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

pub fn iter_contains<I, T>(items: I, expected: &T) -> bool
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<T>,
    T: PartialEq + ?Sized,
{
    crate::slice_primitives::iter_contains(items, expected)
}

pub fn locate_index<T>(items: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    for (idx, item) in items.iter().enumerate() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub fn find_window_by<T>(
    items: &[T],
    window_len: usize,
    predicate: impl FnMut(&[T]) -> bool,
) -> Option<usize> {
    crate::slice_primitives::find_window_by(items, window_len, predicate)
}

pub fn contains_sequence<T: PartialEq>(items: &[T], window: &[T]) -> bool {
    crate::slice_primitives::contains_sequence(items, window)
}

pub use contains_sequence as contains_window;

#[cfg(test)]
pub use crate::string_primitives::contains as str_contains;
pub use sentence_markers::recognizes_core_keyword_marker as is_core_keyword_marker_text;
pub use sentence_markers::recognizes_ticket_sticker_marker as is_ticket_sticker_marker_text;

pub fn parse_lexed_prefix<'a, O>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<LexedInput<'a>, O, ErrMode<ContextError>>,
) -> Option<(O, &'a [OwnedLexToken])> {
    grammar::parse_prefix(tokens, parser)
}

pub fn parse_word_token<'a>(input: &mut LexedInput<'a>) -> WResult<&'a str> {
    grammar::word_text(input)
}

pub fn rewrite_followup_intro_to_if_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    // Preserve reflexive "when" prefixes so lowering emits a stack-queued
    // ReflexiveTriggerEffect instead of an inline IfEffect.
    tokens.to_vec()
}

pub fn remove_copy_exception_type_removal_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut rewritten = tokens.to_vec();
    while let Some(span) =
        static_keyword_line_shapes::parse_copy_exception_type_removal_span(&rewritten)
    {
        rewritten.drain(span.start..span.end);
    }
    rewritten
}

pub fn lexed_tokens_contain_non_prefix_instead(tokens: &[OwnedLexToken]) -> bool {
    sentence_markers::has_nonconditional_instead(tokens)
}

pub fn strip_leading_if_you_do_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
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

pub fn clone_sentence_chunk_tokens(
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

pub fn split_em_dash_label_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
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

pub fn split_em_dash_label_prefix(tokens: &[OwnedLexToken]) -> Option<(String, &[OwnedLexToken])> {
    let (label_tokens, body_tokens) = split_em_dash_label_prefix_tokens(tokens)?;
    let label = render_token_slice(label_tokens).trim().to_string();
    (!label.is_empty()).then_some((label, body_tokens))
}

pub use sentence_markers::{LeadingMayActionMatch, LeadingMayActor};

pub fn parse_leading_may_action_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    verbs: &'static [&'static str],
    allow_bare: bool,
) -> Option<LeadingMayActionMatch<'a>> {
    sentence_markers::parse_leading_may_action_tokens(tokens, verbs, allow_bare)
}

fn parse_head_words<'a>(input: &mut LexedInput<'a>) -> WResult<(&'a str, Option<&'a str>)> {
    peek(seq!(
        grammar::word_parser_text,
        opt(grammar::word_parser_text)
    ))
    .parse_next(input)
}

pub fn lexed_head_words(tokens: &[OwnedLexToken]) -> Option<(&str, Option<&str>)> {
    parse_lexed_prefix(tokens, parse_head_words).map(|(head, _)| head)
}

fn turn_duration_from_leaf(duration: LeafTurnDurationPhrase) -> TurnDurationPhrase {
    match duration {
        LeafTurnDurationPhrase::ThisTurn => TurnDurationPhrase::ThisTurn,
        LeafTurnDurationPhrase::UntilEndOfTurn => TurnDurationPhrase::UntilEndOfTurn,
        LeafTurnDurationPhrase::UntilYourNextTurn => TurnDurationPhrase::UntilYourNextTurn,
        LeafTurnDurationPhrase::UntilYourNextTurnEnd => TurnDurationPhrase::UntilYourNextTurnEnd,
    }
}

#[cfg(test)]
pub fn parse_turn_duration_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(TurnDurationPhrase, &[OwnedLexToken])> {
    let parsed = parse_leaf_turn_duration_prefix_tokens(tokens)?;
    Some((turn_duration_from_leaf(parsed.duration), parsed.rest))
}

pub fn parse_turn_duration_suffix(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], TurnDurationPhrase)> {
    let parsed = parse_leaf_turn_duration_suffix_tokens(tokens)?;
    Some((parsed.rest, turn_duration_from_leaf(parsed.duration)))
}

pub fn parse_simple_restriction_duration_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(Until, &[OwnedLexToken])> {
    let parsed = parse_leaf_restriction_duration_prefix_tokens(tokens)?;
    Some((until_from_leaf_duration(parsed.duration), parsed.rest))
}

pub fn parse_simple_restriction_duration_suffix(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], Until)> {
    let parsed = parse_leaf_restriction_duration_suffix_tokens(tokens)?;
    Some((parsed.rest, until_from_leaf_duration(parsed.duration)))
}
