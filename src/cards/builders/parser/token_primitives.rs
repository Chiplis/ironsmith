use winnow::combinator::{alt, dispatch, fail, opt, peek, seq};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;

use crate::effect::Until;

use super::grammar::primitives as grammar;
pub(crate) use super::grammar::values::{
    parse_count_range_prefix, parse_mana_cost_inner, parse_mana_symbol, parse_mana_symbol_group,
    parse_modal_choose_range, parse_scryfall_mana_cost, parse_type_line_with,
    parse_value_comparison_tokens,
};
use super::lexer::{LexStream, OwnedLexToken, TokenKind};
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
    tokens.iter().enumerate().find_map(|(idx, token)| {
        (token.kind == delimiter).then_some((&tokens[..idx], &tokens[idx + 1..]))
    })
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
    tokens.iter().enumerate().find_map(|(idx, _)| {
        grammar::parse_prefix(
            &tokens[idx..],
            seq!(_: grammar::comma(), _: parse_word_eq("then")),
        )
        .map(|(_, rest)| (&tokens[..idx], rest))
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

pub(crate) fn parse_turn_duration_prefix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(TurnDurationPhrase, &'a [OwnedLexToken])> {
    for (phrase, duration) in [
        (
            &["until", "your", "next", "turn"][..],
            TurnDurationPhrase::UntilYourNextTurn,
        ),
        (
            &["until", "the", "end", "of", "your", "next", "turn"][..],
            TurnDurationPhrase::UntilYourNextTurn,
        ),
        (
            &["until", "end", "of", "your", "next", "turn"][..],
            TurnDurationPhrase::UntilYourNextTurn,
        ),
        (
            &["until", "the", "end", "of", "turn"][..],
            TurnDurationPhrase::UntilEndOfTurn,
        ),
        (
            &["until", "end", "of", "turn"][..],
            TurnDurationPhrase::UntilEndOfTurn,
        ),
        (&["this", "turn"][..], TurnDurationPhrase::ThisTurn),
    ] {
        if let Some(rest) = grammar::strip_lexed_prefix_phrase(tokens, phrase) {
            return Some((duration, rest));
        }
    }

    None
}

pub(crate) fn parse_turn_duration_suffix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], TurnDurationPhrase)> {
    for (phrase, duration) in [
        (
            &["until", "your", "next", "turn"][..],
            TurnDurationPhrase::UntilYourNextTurn,
        ),
        (
            &["until", "the", "end", "of", "your", "next", "turn"][..],
            TurnDurationPhrase::UntilYourNextTurn,
        ),
        (
            &["until", "end", "of", "your", "next", "turn"][..],
            TurnDurationPhrase::UntilYourNextTurn,
        ),
        (
            &["until", "the", "end", "of", "turn"][..],
            TurnDurationPhrase::UntilEndOfTurn,
        ),
        (
            &["until", "end", "of", "turn"][..],
            TurnDurationPhrase::UntilEndOfTurn,
        ),
        (&["this", "turn"][..], TurnDurationPhrase::ThisTurn),
    ] {
        if let Some(rest) = grammar::strip_lexed_suffix_phrase(tokens, phrase) {
            return Some((rest, duration));
        }
    }

    None
}

pub(crate) fn parse_simple_restriction_duration_prefix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(Until, &'a [OwnedLexToken])> {
    if let Some((duration, rest)) = parse_turn_duration_prefix(tokens) {
        return Some((until_from_turn_duration_phrase(duration), rest));
    }

    for (phrase, duration) in [
        (
            &["until", "the", "end", "of", "combat"][..],
            Until::EndOfCombat,
        ),
        (&["until", "end", "of", "combat"][..], Until::EndOfCombat),
    ] {
        if let Some(rest) = grammar::strip_lexed_prefix_phrase(tokens, phrase) {
            return Some((duration, rest));
        }
    }

    None
}

pub(crate) fn parse_simple_restriction_duration_suffix<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], Until)> {
    if let Some((rest, duration)) = parse_turn_duration_suffix(tokens) {
        return Some((rest, until_from_turn_duration_phrase(duration)));
    }

    for (phrase, duration) in [
        (
            &["until", "the", "end", "of", "combat"][..],
            Until::EndOfCombat,
        ),
        (&["until", "end", "of", "combat"][..], Until::EndOfCombat),
        (
            &["during", "your", "next", "untap", "step"][..],
            Until::ControllersNextUntapStep,
        ),
        (
            &["during", "its", "controller", "next", "untap", "step"][..],
            Until::ControllersNextUntapStep,
        ),
        (
            &["during", "its", "controllers", "next", "untap", "step"][..],
            Until::ControllersNextUntapStep,
        ),
        (
            &["during", "their", "controller", "next", "untap", "step"][..],
            Until::ControllersNextUntapStep,
        ),
        (
            &["during", "their", "controllers", "next", "untap", "step"][..],
            Until::ControllersNextUntapStep,
        ),
        (
            &["for", "the", "rest", "of", "the", "game"][..],
            Until::Forever,
        ),
    ] {
        if let Some(rest) = grammar::strip_lexed_suffix_phrase(tokens, phrase) {
            return Some((rest, duration));
        }
    }

    None
}
