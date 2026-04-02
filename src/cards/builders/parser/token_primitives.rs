use winnow::ascii::{digit1, multispace0};
use winnow::combinator::{alt, cut_err, delimited, opt, preceded, repeat, separated, terminated};
use winnow::error::{
    ContextError, ErrMode, ModalResult as WResult, ParserError, StrContext, StrContextValue,
};
use winnow::prelude::*;
use winnow::stream::TokenSlice;
use winnow::token::{any, literal, one_of};

use crate::cards::builders::CardTextError;
use crate::effect::{Until, Value, ValueComparisonOperator};
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

use super::lexer::{LexStream, OwnedLexToken, TokenKind, lex_line};
use super::native_tokens::LowercaseWordView;

pub(crate) type LexedInput<'a> = LexStream<'a>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnDurationPhrase {
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
}

fn until_from_turn_duration_phrase(duration: TurnDurationPhrase) -> Until {
    match duration {
        TurnDurationPhrase::ThisTurn | TurnDurationPhrase::UntilEndOfTurn => Until::EndOfTurn,
        TurnDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
    }
}

fn spaced<'a, O, E, P>(parser: P) -> impl Parser<&'a str, O, E>
where
    P: Parser<&'a str, O, E>,
    E: ParserError<&'a str>,
{
    delimited(multispace0, parser, multispace0)
}

fn finish_text_parse<'a, O, E>(
    raw: &'a str,
    mut parser: impl Parser<&'a str, O, E>,
    label: &str,
) -> Result<O, CardTextError>
where
    E: std::fmt::Display,
{
    let mut input = raw.trim();
    let parsed = parser
        .parse_next(&mut input)
        .map_err(|err| CardTextError::ParseError(format!("rewrite {label} parse failed: {err}")))?;
    if !input.trim().is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite {label} parser left trailing input: '{}'",
            input.trim()
        )));
    }
    Ok(parsed)
}

fn finish_lexed_parse<'a, O, E>(
    tokens: &'a [OwnedLexToken],
    mut parser: impl Parser<LexedInput<'a>, O, E>,
    label: &str,
) -> Result<O, CardTextError>
where
    E: std::fmt::Display,
{
    let mut input = TokenSlice::new(tokens);
    let parsed = parser
        .parse_next(&mut input)
        .map_err(|err| CardTextError::ParseError(format!("rewrite {label} parse failed: {err}")))?;
    if !input.is_empty() {
        let trailing = input
            .iter()
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        return Err(CardTextError::ParseError(format!(
            "rewrite {label} parser left trailing tokens: '{trailing}'"
        )));
    }
    Ok(parsed)
}

pub(crate) fn parse_lexed_prefix<'a, O>(
    tokens: &'a [OwnedLexToken],
    mut parser: impl Parser<LexedInput<'a>, O, ErrMode<ContextError>>,
) -> Option<(O, &'a [OwnedLexToken])> {
    let mut input = TokenSlice::new(tokens);
    let parsed = parser.parse_next(&mut input).ok()?;
    let consumed = tokens.len().checked_sub(input.len())?;
    let remainder = &tokens[consumed..];
    Some((parsed, remainder))
}

pub(crate) fn strip_lexed_prefix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<&'a [OwnedLexToken]> {
    parse_lexed_prefix(tokens, parse_word_phrase(phrase)).map(|(_, rest)| rest)
}

pub(crate) fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    let lowered = LowercaseWordView::new(tokens);
    let words = lowered.to_word_refs();
    if words.len() < phrase.len() {
        return None;
    }

    let suffix_start = words.len() - phrase.len();
    if words[suffix_start..] != *phrase {
        return None;
    }

    let keep_word_count = words.len().checked_sub(phrase.len())?;
    let keep_until = if keep_word_count == 0 {
        0
    } else {
        lowered.token_index_for_word_index(keep_word_count)?
    };
    Some(&tokens[..keep_until])
}

pub(crate) fn parse_mana_symbol_inner(input: &mut &str) -> WResult<ManaSymbol> {
    winnow::combinator::alt((
        digit1.try_map(|digits: &str| digits.parse::<u8>().map(ManaSymbol::Generic)),
        one_of([
            'W', 'w', 'U', 'u', 'B', 'b', 'R', 'r', 'G', 'g', 'C', 'c', 'S', 's', 'X', 'x', 'P',
            'p',
        ])
        .map(|ch: char| match ch.to_ascii_uppercase() {
            'W' => ManaSymbol::White,
            'U' => ManaSymbol::Blue,
            'B' => ManaSymbol::Black,
            'R' => ManaSymbol::Red,
            'G' => ManaSymbol::Green,
            'C' => ManaSymbol::Colorless,
            'S' => ManaSymbol::Snow,
            'X' => ManaSymbol::X,
            'P' => ManaSymbol::Life(2),
            _ => unreachable!("one_of constrains supported mana-symbol letters"),
        }),
    ))
    .context(StrContext::Label("mana symbol"))
    .context(StrContext::Expected(StrContextValue::Description(
        "mana symbol",
    )))
    .parse_next(input)
}

pub(crate) fn parse_mana_symbol(raw: &str) -> Result<ManaSymbol, CardTextError> {
    finish_text_parse(raw, spaced(parse_mana_symbol_inner), "mana-symbol")
}

pub(crate) fn parse_mana_symbol_group_inner(input: &mut &str) -> WResult<Vec<ManaSymbol>> {
    separated(1.., parse_mana_symbol_inner, spaced('/'))
        .context(StrContext::Label("mana symbol group"))
        .context(StrContext::Expected(StrContextValue::Description(
            "slash-delimited mana symbols",
        )))
        .parse_next(input)
}

pub(crate) fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let trimmed = raw.trim().trim_matches('{').trim_matches('}');
    finish_text_parse(trimmed, spaced(parse_mana_symbol_group_inner), "mana-group")
}

fn parse_mana_group_inner(input: &mut &str) -> WResult<Vec<ManaSymbol>> {
    preceded(
        spaced("{"),
        cut_err(terminated(
            separated(1.., parse_mana_symbol_inner, spaced('/')).context(StrContext::Expected(
                StrContextValue::Description("mana symbols"),
            )),
            spaced("}").context(StrContext::Expected('}'.into())),
        )),
    )
    .context(StrContext::Label("mana group"))
    .context(StrContext::Expected(StrContextValue::Description(
        "braced mana symbols",
    )))
    .parse_next(input)
}

pub(crate) fn parse_mana_cost_inner(input: &mut &str) -> WResult<ManaCost> {
    repeat(1.., parse_mana_group_inner)
        .map(ManaCost::from_pips)
        .context(StrContext::Label("mana cost"))
        .context(StrContext::Expected(StrContextValue::Description(
            "mana group",
        )))
        .parse_next(input)
}

fn parse_mana_group_token<'a>(input: &mut LexedInput<'a>) -> WResult<Vec<ManaSymbol>> {
    let token: &'a OwnedLexToken = any.parse_next(input)?;
    match token.kind {
        TokenKind::ManaGroup => {
            let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
            parse_mana_symbol_group(inner).map_err(|_| {
                let mut err = ContextError::new();
                err.push(StrContext::Label("mana group token"));
                err.push(StrContext::Expected(StrContextValue::Description(
                    "mana symbol group",
                )));
                ErrMode::Backtrack(err)
            })
        }
        _ => {
            let mut err = ContextError::new();
            err.push(StrContext::Label("mana group token"));
            err.push(StrContext::Expected(StrContextValue::Description(
                "mana group token",
            )));
            Err(ErrMode::Backtrack(err))
        }
    }
}

fn parse_mana_cost_tokens<'a>(input: &mut LexedInput<'a>) -> WResult<ManaCost> {
    repeat(1.., parse_mana_group_token)
        .map(ManaCost::from_pips)
        .context(StrContext::Label("mana cost"))
        .context(StrContext::Expected(StrContextValue::Description(
            "mana group token",
        )))
        .parse_next(input)
}

pub(crate) fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "—" {
        return Ok(ManaCost::new());
    }

    let tokens = lex_line(trimmed, 0)?;
    finish_lexed_parse(&tokens, parse_mana_cost_tokens, "mana-cost")
}

fn parse_token_kind<'a>(
    expected: TokenKind,
) -> impl Parser<LexedInput<'a>, &'a OwnedLexToken, ErrMode<ContextError>> {
    literal(expected).map(|tokens: &'a [OwnedLexToken]| &tokens[0])
}

pub(crate) fn parse_word_token<'a>(input: &mut LexedInput<'a>) -> WResult<&'a str> {
    let token: &'a OwnedLexToken = any.parse_next(input)?;
    token.as_word().ok_or_else(|| {
        let mut err = ContextError::new();
        err.push(StrContext::Label("word"));
        err.push(StrContext::Expected(StrContextValue::Description("word")));
        ErrMode::Backtrack(err)
    })
}

pub(crate) fn parse_word_eq<'a>(
    expected: &'static str,
) -> impl Parser<LexedInput<'a>, (), ErrMode<ContextError>> {
    parse_word_token
        .verify(move |word: &&str| word.eq_ignore_ascii_case(expected))
        .context(StrContext::Expected(expected.into()))
        .map(|_| ())
}

pub(crate) fn parse_word_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexedInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexedInput<'a>| {
        let checkpoint = input.checkpoint();
        for word in expected {
            if parse_word_eq(*word).parse_next(input).is_err() {
                input.reset(&checkpoint);
                return Err(ErrMode::Backtrack(ContextError::new()));
            }
        }
        Ok(())
    }
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
        if let Some(rest) = strip_lexed_prefix_phrase(tokens, phrase) {
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
        if let Some(rest) = strip_lexed_suffix_phrase(tokens, phrase) {
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
        if let Some(rest) = strip_lexed_prefix_phrase(tokens, phrase) {
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
        if let Some(rest) = strip_lexed_suffix_phrase(tokens, phrase) {
            return Some((rest, duration));
        }
    }

    None
}

fn parse_modal_value_token<'a>(input: &mut LexedInput<'a>) -> WResult<Value> {
    let word = parse_word_token.parse_next(input)?;
    if word.eq_ignore_ascii_case("x") {
        return Ok(Value::X);
    }
    if let Ok(value) = word.parse::<i32>() {
        return Ok(Value::Fixed(value));
    }

    let value = match word.to_ascii_lowercase().as_str() {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return Err(ErrMode::Backtrack(ContextError::new())),
    };

    Ok(Value::Fixed(value))
}

pub(crate) fn parse_count_range_prefix(
    tokens: &[OwnedLexToken],
) -> Option<((Option<Value>, Option<Value>), &[OwnedLexToken])> {
    let parser = alt((
        (
            parse_word_eq("one"),
            parse_word_eq("or"),
            parse_word_eq("more"),
        )
            .value((Some(Value::Fixed(1)), None)),
        (
            parse_word_eq("one"),
            parse_word_eq("or"),
            parse_word_eq("both"),
        )
            .value((Some(Value::Fixed(1)), Some(Value::Fixed(2)))),
        (
            parse_word_eq("up"),
            parse_word_eq("to"),
            parse_modal_value_token,
        )
            .map(|(_, _, value)| (Some(Value::Fixed(0)), Some(value))),
        parse_modal_value_token.map(|value| (Some(value.clone()), Some(value))),
    ));

    parse_lexed_prefix(tokens, parser)
}

pub(crate) fn parse_value_comparison_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(ValueComparisonOperator, &'a [OwnedLexToken])> {
    for (phrase, operator) in [
        (&["is", "equal", "to"][..], ValueComparisonOperator::Equal),
        (&["equal", "to"][..], ValueComparisonOperator::Equal),
        (
            &["is", "not", "equal", "to"][..],
            ValueComparisonOperator::NotEqual,
        ),
        (
            &["not", "equal", "to"][..],
            ValueComparisonOperator::NotEqual,
        ),
        (
            &["is", "less", "than", "or", "equal", "to"][..],
            ValueComparisonOperator::LessThanOrEqual,
        ),
        (
            &["less", "than", "or", "equal", "to"][..],
            ValueComparisonOperator::LessThanOrEqual,
        ),
        (
            &["is", "greater", "than", "or", "equal", "to"][..],
            ValueComparisonOperator::GreaterThanOrEqual,
        ),
        (
            &["greater", "than", "or", "equal", "to"][..],
            ValueComparisonOperator::GreaterThanOrEqual,
        ),
        (
            &["is", "less", "than"][..],
            ValueComparisonOperator::LessThan,
        ),
        (&["less", "than"][..], ValueComparisonOperator::LessThan),
        (
            &["is", "greater", "than"][..],
            ValueComparisonOperator::GreaterThan,
        ),
        (
            &["greater", "than"][..],
            ValueComparisonOperator::GreaterThan,
        ),
    ] {
        if let Some(rest) = strip_lexed_prefix_phrase(tokens, phrase) {
            return Some((operator, rest));
        }
    }

    for (phrase, operator) in [
        (
            &["or", "less"][..],
            ValueComparisonOperator::LessThanOrEqual,
        ),
        (
            &["or", "fewer"][..],
            ValueComparisonOperator::LessThanOrEqual,
        ),
        (
            &["or", "greater"][..],
            ValueComparisonOperator::GreaterThanOrEqual,
        ),
        (
            &["or", "more"][..],
            ValueComparisonOperator::GreaterThanOrEqual,
        ),
    ] {
        if let Some(after_is) = strip_lexed_prefix_phrase(tokens, &["is"])
            && let Some(rest) = strip_lexed_suffix_phrase(after_is, phrase)
            && !rest.is_empty()
        {
            return Some((operator, rest));
        }

        if let Some(rest) = strip_lexed_suffix_phrase(tokens, phrase)
            && !rest.is_empty()
        {
            return Some((operator, rest));
        }
    }

    None
}

fn parse_type_line_tokens<'a>(input: &mut LexedInput<'a>) -> WResult<(Vec<&'a str>, Vec<&'a str>)> {
    let left = repeat(1.., parse_word_token)
        .context(StrContext::Expected(StrContextValue::Description(
            "type-line words",
        )))
        .parse_next(input)?;
    let right = opt(preceded(
        parse_token_kind(TokenKind::EmDash).context(StrContext::Expected(
            StrContextValue::Description("em dash"),
        )),
        cut_err(
            repeat(1.., parse_word_token)
                .context(StrContext::Label("type-line subtype section"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "subtype words",
                ))),
        ),
    ))
    .context(StrContext::Label("type-line"))
    .parse_next(input)?
    .unwrap_or_default();
    Ok((left, right))
}

pub(crate) fn parse_type_line_with(
    raw: &str,
    mut parse_supertype: impl FnMut(&str) -> Option<Supertype>,
    mut parse_card_type: impl FnMut(&str) -> Option<CardType>,
    mut parse_subtype: impl FnMut(&str) -> Option<Subtype>,
) -> Result<(Vec<Supertype>, Vec<CardType>, Vec<Subtype>), CardTextError> {
    let normalized = raw.trim();
    let front_face = normalized.split("//").next().unwrap_or(normalized).trim();
    let tokens = lex_line(front_face, 0)?;
    let (left_words, right_words) =
        finish_lexed_parse(&tokens, parse_type_line_tokens, "type-line")?;

    let mut supertypes = Vec::new();
    let mut card_types = Vec::new();
    for word in left_words {
        if let Some(supertype) = parse_supertype(word) {
            supertypes.push(supertype);
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            card_types.push(card_type);
        }
    }

    let mut subtypes = Vec::new();
    for word in right_words {
        if let Some(subtype) = parse_subtype(word) {
            subtypes.push(subtype);
        }
    }

    Ok((supertypes, card_types, subtypes))
}

pub(crate) fn parse_modal_choose_range(
    tokens: &[OwnedLexToken],
) -> Result<Option<(Option<Value>, Option<Value>)>, CardTextError> {
    if let Some((range, _)) = parse_count_range_prefix(tokens) {
        return Ok(Some(range));
    }

    if tokens.iter().any(|token| token.is_word("or")) {
        return Ok(Some((Some(Value::Fixed(1)), Some(Value::Fixed(1)))));
    }

    Ok(None)
}
