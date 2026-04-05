use winnow::combinator::{alt, dispatch, fail, opt, peek, seq};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::any;

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

fn parse_segment_len_until_comma_then<'a>(input: &mut LexedInput<'a>) -> WResult<usize> {
    let initial_len = input.len();

    while input.peek_token().is_some() {
        if input.peek_token().is_some_and(|token| token.is_comma())
            && input.get(1).is_some_and(|token| token.is_word("then"))
        {
            let head_len = initial_len - input.len();
            grammar::comma().parse_next(input)?;
            parse_word_eq("then").parse_next(input)?;
            return Ok(head_len);
        }

        any.parse_next(input)?;
    }

    Err(ErrMode::Backtrack(ContextError::new()))
}

pub(crate) fn split_lexed_once_on_comma_then(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (head_len, rest) = parse_lexed_prefix(tokens, parse_segment_len_until_comma_then)?;
    Some((&tokens[..head_len], rest))
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
