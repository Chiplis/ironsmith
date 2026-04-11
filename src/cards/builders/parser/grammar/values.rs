use winnow::ascii::{digit1, multispace0};
use winnow::combinator::{
    alt, cut_err, delimited, dispatch, eof, fail, opt, peek, preceded, repeat, separated,
};
use winnow::error::{
    ContextError, ErrMode, ModalResult as WResult, ParserError, StrContext, StrContextValue,
};
use winnow::prelude::*;
use winnow::token::one_of;

use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{Value, ValueComparisonOperator};
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};

use super::super::activation_and_restrictions::find_word_sequence_start;
#[cfg(test)]
use super::super::effect_sentences::parse_subtype_word;
use super::super::lexer::{LexStream, OwnedLexToken, TokenKind, lex_line, parser_token_word_refs};
use super::super::token_primitives::{find_index, slice_contains, slice_starts_with};
use super::super::util::{
    parse_number_word_i32, parse_value_expr_words, token_index_for_word_index,
};
use super::primitives;

type LexedInput<'a> = LexStream<'a>;

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeLineCst {
    pub(crate) supertypes: Vec<Supertype>,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

pub(crate) fn count_word_value(word: &str) -> Option<u32> {
    match word.to_ascii_lowercase().as_str() {
        "a" | "an" | "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        _ => None,
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
    parser: impl Parser<&'a str, O, E>,
    label: &str,
) -> Result<O, CardTextError>
where
    E: std::fmt::Display + ParserError<&'a str>,
{
    let mut input = raw.trim();
    let mut parser = primitives::maybe_trace(label, parser);
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

fn finish_lexed_parse<'a, O>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<LexedInput<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<O, CardTextError> {
    primitives::parse_all(tokens, parser, label)
}

fn matches_exact_value_phrase_lexed(
    tokens: &[OwnedLexToken],
    phrase: &'static [&'static str],
) -> bool {
    primitives::parse_prefix(tokens, (primitives::phrase(phrase), eof)).is_some()
}

pub(crate) fn parse_max_cards_in_hand_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
    [
        (
            &[
                "cards", "in", "the", "hand", "of", "the", "opponent", "with", "the", "most",
                "cards", "in", "hand",
            ][..],
            Value::MaxCardsInHand(PlayerFilter::Opponent),
        ),
        (
            &[
                "cards", "in", "the", "hand", "of", "an", "opponent", "with", "the", "most",
                "cards", "in", "hand",
            ][..],
            Value::MaxCardsInHand(PlayerFilter::Opponent),
        ),
        (
            &[
                "cards", "in", "the", "hand", "of", "the", "player", "with", "the", "most",
                "cards", "in", "hand",
            ][..],
            Value::MaxCardsInHand(PlayerFilter::Any),
        ),
    ]
    .into_iter()
    .find_map(|(phrase, value)| matches_exact_value_phrase_lexed(tokens, phrase).then_some(value))
}

pub(crate) fn parse_mana_symbol_inner(input: &mut &str) -> WResult<ManaSymbol> {
    alt((
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

#[cfg(test)]
pub(crate) fn parse_mana_symbol_group_rewrite(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let tokens = lex_line(raw.trim(), 0)?;
    parse_mana_symbol_group_tokens(&tokens)
}

#[cfg(test)]
pub(crate) fn parse_count_word_rewrite(raw: &str) -> Result<u32, CardTextError> {
    let tokens = lex_line(raw.trim(), 0)?;
    parse_count_word_tokens(&tokens)
}

fn parse_count_token<'a>(input: &mut LexedInput<'a>) -> WResult<u32> {
    let word = primitives::word_text.parse_next(input)?;
    if let Ok(value) = word.parse::<u32>() {
        return Ok(value);
    }

    count_word_value(word)
        .ok_or_else(|| primitives::backtrack_err("count", "numeric or counted quantity"))
}

pub(crate) fn parse_count_word_tokens(tokens: &[OwnedLexToken]) -> Result<u32, CardTextError> {
    finish_lexed_parse(tokens, parse_count_token, "count-word")
}

fn parse_mana_cost_tokens_text(raw: &str, allow_empty: bool) -> Result<ManaCost, CardTextError> {
    let trimmed = raw.trim();
    if allow_empty && (trimmed.is_empty() || trimmed == "—") {
        return Ok(ManaCost::new());
    }

    let tokens = lex_line(trimmed, 0)?;
    parse_mana_cost_tokens(&tokens)
}

pub(crate) fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    parse_mana_cost_tokens_text(raw, true)
}

#[cfg(test)]
pub(crate) fn parse_mana_cost_rewrite(raw: &str) -> Result<ManaCost, CardTextError> {
    parse_mana_cost_tokens_text(raw, false)
}

fn parse_mana_group_token<'a>(input: &mut LexedInput<'a>) -> WResult<Vec<ManaSymbol>> {
    let token = primitives::token_kind(TokenKind::ManaGroup).parse_next(input)?;
    parse_mana_symbol_group(token.slice.as_str())
        .map_err(|_| primitives::backtrack_err("mana group", "braced mana symbols"))
}

#[cfg(test)]
pub(crate) fn parse_mana_symbol_group_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Vec<ManaSymbol>, CardTextError> {
    finish_lexed_parse(tokens, parse_mana_group_token, "mana-group")
}

fn parse_mana_cost_tokens_inner<'a>(input: &mut LexedInput<'a>) -> WResult<ManaCost> {
    repeat(1.., parse_mana_group_token)
        .map(ManaCost::from_pips)
        .context(StrContext::Label("mana cost"))
        .context(StrContext::Expected(StrContextValue::Description(
            "mana group",
        )))
        .parse_next(input)
}

pub(crate) fn parse_mana_cost_tokens(tokens: &[OwnedLexToken]) -> Result<ManaCost, CardTextError> {
    finish_lexed_parse(tokens, parse_mana_cost_tokens_inner, "mana-cost")
}

fn parse_modal_value_token<'a>(input: &mut LexedInput<'a>) -> WResult<Value> {
    let word = primitives::word_text.parse_next(input)?;
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
        _ => {
            return Err(primitives::backtrack_err(
                "digit word",
                "number word (one–ten)",
            ));
        }
    };

    Ok(Value::Fixed(value))
}

pub(crate) fn parse_count_range_prefix(
    tokens: &[OwnedLexToken],
) -> Option<((Option<Value>, Option<Value>), &[OwnedLexToken])> {
    let parser = dispatch! {peek(primitives::word_parser_text);
        "one" => alt((
            primitives::phrase(&["one", "or", "more"]).value((Some(Value::Fixed(1)), None)),
            primitives::phrase(&["one", "or", "both"])
                .value((Some(Value::Fixed(1)), Some(Value::Fixed(2)))),
            primitives::kw("one").value((Some(Value::Fixed(1)), Some(Value::Fixed(1)))),
        )),
        "up" => (
            primitives::kw("up"),
            primitives::kw("to"),
            parse_modal_value_token,
        )
            .map(|(_, _, value)| (Some(Value::Fixed(0)), Some(value))),
        _ => parse_modal_value_token.map(|value| (Some(value.clone()), Some(value))),
    };

    primitives::parse_prefix(tokens, parser)
}

fn strip_lexed_prefix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<&'a [OwnedLexToken]> {
    primitives::parse_prefix(tokens, primitives::phrase(phrase)).map(|(_, rest)| rest)
}

fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    let word_refs = parser_token_word_refs(tokens);
    if word_refs.len() < phrase.len() {
        return None;
    }

    let suffix_start = word_refs.len() - phrase.len();
    if word_refs[suffix_start..]
        .iter()
        .copied()
        .ne(phrase.iter().copied())
    {
        return None;
    }

    let keep_word_count = word_refs.len().checked_sub(phrase.len())?;
    let keep_until = if keep_word_count == 0 {
        0
    } else if keep_word_count == word_refs.len() {
        tokens.len()
    } else {
        token_index_for_word_index(tokens, keep_word_count)?
    };
    Some(&tokens[..keep_until])
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
    let left = repeat(1.., primitives::word_text)
        .context(StrContext::Expected(StrContextValue::Description(
            "type-line words",
        )))
        .parse_next(input)?;
    let right = opt(preceded(
        primitives::token_kind(TokenKind::EmDash).context(StrContext::Expected(
            StrContextValue::Description("em dash"),
        )),
        cut_err(
            repeat(1.., primitives::word_text)
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

    let (supertypes, card_types) =
        left_words
            .iter()
            .fold((Vec::new(), Vec::new()), |(mut supers, mut types), word| {
                if let Some(supertype) = parse_supertype(word) {
                    supers.push(supertype);
                } else if let Some(card_type) = parse_card_type(word) {
                    types.push(card_type);
                }
                (supers, types)
            });

    let subtypes: Vec<_> = right_words
        .iter()
        .filter_map(|word| parse_subtype(word))
        .collect();

    Ok((supertypes, card_types, subtypes))
}

#[cfg(test)]
fn parse_card_type_word_for_rewrite(word: &str) -> Option<CardType> {
    match word.to_ascii_lowercase().as_str() {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "instant" | "instants" => Some(CardType::Instant),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        "battle" | "battles" => Some(CardType::Battle),
        "kindred" => Some(CardType::Kindred),
        _ => None,
    }
}

#[cfg(test)]
fn parse_supertype_word_for_rewrite(word: &str) -> Option<Supertype> {
    match word.to_ascii_lowercase().as_str() {
        "basic" => Some(Supertype::Basic),
        "legendary" => Some(Supertype::Legendary),
        "snow" => Some(Supertype::Snow),
        "world" => Some(Supertype::World),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn parse_type_line_rewrite(raw: &str) -> Result<TypeLineCst, CardTextError> {
    let (supertypes, card_types, subtypes) = parse_type_line_with(
        raw,
        parse_supertype_word_for_rewrite,
        parse_card_type_word_for_rewrite,
        parse_subtype_word,
    )?;

    Ok(TypeLineCst {
        supertypes,
        card_types,
        subtypes,
    })
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

fn trim_lexed_edge_punctuation(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        end -= 1;
    }
    &tokens[start..end]
}

pub(crate) fn parse_number_from_lexed(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    let trimmed = trim_lexed_edge_punctuation(tokens);
    let first_word = trimmed.first()?.as_word()?.to_ascii_lowercase();
    let value: u32 = parse_number_word_i32(&first_word).and_then(|value| value.try_into().ok())?;
    Some((value, 1))
}

pub(crate) fn parse_value_from_lexed(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let trimmed = trim_lexed_edge_punctuation(tokens);
    let word_refs = parser_token_word_refs(trimmed);
    let (value, used_words) = parse_value_expr_words(&word_refs)?;
    let used_tokens = token_index_for_word_index(trimmed, used_words).unwrap_or(trimmed.len());
    Some((value, used_tokens))
}

pub(crate) fn parse_add_mana_equal_amount_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words_all = parser_token_word_refs(tokens);
    let equal_idx = find_word_sequence_start(&words_all, &["equal", "to"])?;
    let tail = &words_all[equal_idx + 2..];
    if tail.is_empty() {
        return None;
    }

    let is_source_power_segment = |segment: &[&str]| {
        matches!(
            segment,
            ["this", "power"]
                | ["thiss", "power"]
                | ["this", "creature", "power"]
                | ["this", "creatures", "power"]
                | ["thiss", "creature", "power"]
                | ["thiss", "creatures", "power"]
                | ["its", "power"]
        )
    };
    let is_source_toughness_segment = |segment: &[&str]| {
        matches!(
            segment,
            ["this", "toughness"]
                | ["thiss", "toughness"]
                | ["this", "creature", "toughness"]
                | ["this", "creatures", "toughness"]
                | ["thiss", "creature", "toughness"]
                | ["thiss", "creatures", "toughness"]
                | ["its", "toughness"]
        )
    };

    let parse_power_or_toughness_segment = |segment: &[&str]| -> Option<Value> {
        let tagged_it_power = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
        let tagged_it_toughness =
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));

        if is_source_power_segment(segment) {
            return Some(Value::PowerOf(Box::new(ChooseSpec::Source)));
        }
        if is_source_toughness_segment(segment) {
            return Some(Value::ToughnessOf(Box::new(ChooseSpec::Source)));
        }
        if segment == ["that", "creature", "power"]
            || segment == ["that", "creatures", "power"]
            || segment == ["that", "objects", "power"]
        {
            return Some(tagged_it_power.clone());
        }
        if segment == ["that", "creature", "toughness"]
            || segment == ["that", "creatures", "toughness"]
            || segment == ["that", "objects", "toughness"]
        {
            return Some(tagged_it_toughness.clone());
        }
        if segment == ["the", "sacrificed", "creature", "power"]
            || segment == ["the", "sacrificed", "creatures", "power"]
            || segment == ["sacrificed", "creature", "power"]
            || segment == ["sacrificed", "creatures", "power"]
        {
            return Some(tagged_it_power);
        }
        if segment == ["the", "sacrificed", "creature", "toughness"]
            || segment == ["the", "sacrificed", "creatures", "toughness"]
            || segment == ["sacrificed", "creature", "toughness"]
            || segment == ["sacrificed", "creatures", "toughness"]
        {
            return Some(tagged_it_toughness);
        }
        None
    };

    let parse_mana_value_segment = |segment: &[&str]| -> Option<Value> {
        if slice_starts_with(&segment, &["that", "spell", "mana", "value"])
            || slice_starts_with(&segment, &["that", "spells", "mana", "value"])
            || slice_starts_with(&segment, &["that", "card", "mana", "value"])
            || slice_starts_with(&segment, &["that", "cards", "mana", "value"])
            || slice_starts_with(
                &segment,
                &[
                    "the",
                    "mana",
                    "value",
                    "of",
                    "the",
                    "sacrificed",
                    "creature",
                ],
            )
            || slice_starts_with(
                &segment,
                &[
                    "the",
                    "mana",
                    "value",
                    "of",
                    "the",
                    "sacrificed",
                    "artifact",
                ],
            )
            || slice_starts_with(
                &segment,
                &[
                    "the",
                    "mana",
                    "value",
                    "of",
                    "the",
                    "sacrificed",
                    "permanent",
                ],
            )
            || slice_starts_with(
                &segment,
                &["mana", "value", "of", "the", "sacrificed", "creature"],
            )
            || slice_starts_with(
                &segment,
                &["mana", "value", "of", "the", "sacrificed", "artifact"],
            )
            || slice_starts_with(
                &segment,
                &["mana", "value", "of", "the", "sacrificed", "permanent"],
            )
            || slice_starts_with(
                &segment,
                &["the", "sacrificed", "creature", "mana", "value"],
            )
            || slice_starts_with(
                &segment,
                &["the", "sacrificed", "artifact", "mana", "value"],
            )
            || slice_starts_with(
                &segment,
                &["the", "sacrificed", "permanent", "mana", "value"],
            )
            || slice_starts_with(
                &segment,
                &["the", "sacrificed", "creatures", "mana", "value"],
            )
            || slice_starts_with(
                &segment,
                &["the", "sacrificed", "artifacts", "mana", "value"],
            )
            || slice_starts_with(
                &segment,
                &["the", "sacrificed", "permanents", "mana", "value"],
            )
            || slice_starts_with(&segment, &["sacrificed", "creature", "mana", "value"])
            || slice_starts_with(&segment, &["sacrificed", "artifact", "mana", "value"])
            || slice_starts_with(&segment, &["sacrificed", "permanent", "mana", "value"])
            || slice_starts_with(&segment, &["sacrificed", "creatures", "mana", "value"])
            || slice_starts_with(&segment, &["sacrificed", "artifacts", "mana", "value"])
            || slice_starts_with(&segment, &["sacrificed", "permanents", "mana", "value"])
            || slice_starts_with(&segment, &["its", "mana", "value"])
        {
            return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                TagKey::from(IT_TAG),
            ))));
        }
        if matches!(
            segment,
            ["this", "spell", "mana", "value"]
                | ["this", "creature", "mana", "value"]
                | ["this", "permanent", "mana", "value"]
                | ["this", "card", "mana", "value"]
        ) {
            return Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)));
        }
        None
    };

    let parse_amount_segment = |segment: &[&str]| -> Option<Value> {
        parse_power_or_toughness_segment(segment)
            .or_else(|| {
                if segment.len() == 1 {
                    parse_number_word_i32(segment[0]).map(Value::Fixed)
                } else {
                    None
                }
            })
            .or_else(|| parse_mana_value_segment(segment))
    };

    if let Some(plus_idx) = find_index(tail, |word| *word == "plus")
        && plus_idx > 0
        && plus_idx + 1 < tail.len()
        && let Some(left) = parse_amount_segment(&tail[..plus_idx])
        && let Some(right) = parse_amount_segment(&tail[plus_idx + 1..])
    {
        return Some(Value::Add(Box::new(left), Box::new(right)));
    }

    if let Some(value) = parse_amount_segment(tail) {
        return Some(value);
    }

    if is_source_power_segment(tail)
        || slice_starts_with(&tail, &["that", "creature", "power"])
        || slice_starts_with(&tail, &["that", "creatures", "power"])
        || slice_starts_with(&tail, &["that", "objects", "power"])
        || slice_starts_with(&tail, &["the", "sacrificed", "creature", "power"])
        || slice_starts_with(&tail, &["the", "sacrificed", "creatures", "power"])
        || slice_starts_with(&tail, &["sacrificed", "creature", "power"])
        || slice_starts_with(&tail, &["sacrificed", "creatures", "power"])
    {
        let source = if tail[0] == "that" || slice_contains(&tail, &"sacrificed") {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::PowerOf(Box::new(source)));
    }

    if is_source_toughness_segment(tail)
        || slice_starts_with(&tail, &["that", "creature", "toughness"])
        || slice_starts_with(&tail, &["that", "creatures", "toughness"])
        || slice_starts_with(&tail, &["that", "objects", "toughness"])
        || slice_starts_with(&tail, &["the", "sacrificed", "creature", "toughness"])
        || slice_starts_with(&tail, &["the", "sacrificed", "creatures", "toughness"])
        || slice_starts_with(&tail, &["sacrificed", "creature", "toughness"])
        || slice_starts_with(&tail, &["sacrificed", "creatures", "toughness"])
    {
        let source = if tail[0] == "that" || slice_contains(&tail, &"sacrificed") {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::ToughnessOf(Box::new(source)));
    }

    if slice_starts_with(&tail, &["that", "spell", "mana", "value"])
        || slice_starts_with(&tail, &["that", "spells", "mana", "value"])
        || slice_starts_with(&tail, &["that", "card", "mana", "value"])
        || slice_starts_with(&tail, &["that", "cards", "mana", "value"])
        || slice_starts_with(
            &tail,
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "creature",
            ],
        )
        || slice_starts_with(
            &tail,
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "artifact",
            ],
        )
        || slice_starts_with(
            &tail,
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "permanent",
            ],
        )
        || slice_starts_with(
            &tail,
            &["mana", "value", "of", "the", "sacrificed", "creature"],
        )
        || slice_starts_with(
            &tail,
            &["mana", "value", "of", "the", "sacrificed", "artifact"],
        )
        || slice_starts_with(
            &tail,
            &["mana", "value", "of", "the", "sacrificed", "permanent"],
        )
        || slice_starts_with(&tail, &["the", "sacrificed", "creature", "mana", "value"])
        || slice_starts_with(&tail, &["the", "sacrificed", "artifact", "mana", "value"])
        || slice_starts_with(&tail, &["the", "sacrificed", "permanent", "mana", "value"])
        || slice_starts_with(&tail, &["the", "sacrificed", "creatures", "mana", "value"])
        || slice_starts_with(&tail, &["the", "sacrificed", "artifacts", "mana", "value"])
        || slice_starts_with(&tail, &["the", "sacrificed", "permanents", "mana", "value"])
        || slice_starts_with(&tail, &["sacrificed", "creature", "mana", "value"])
        || slice_starts_with(&tail, &["sacrificed", "artifact", "mana", "value"])
        || slice_starts_with(&tail, &["sacrificed", "permanent", "mana", "value"])
        || slice_starts_with(&tail, &["sacrificed", "creatures", "mana", "value"])
        || slice_starts_with(&tail, &["sacrificed", "artifacts", "mana", "value"])
        || slice_starts_with(&tail, &["sacrificed", "permanents", "mana", "value"])
        || slice_starts_with(&tail, &["its", "mana", "value"])
    {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }
    if matches!(
        tail,
        ["this", "spell", "mana", "value"]
            | ["this", "creature", "mana", "value"]
            | ["this", "permanent", "mana", "value"]
            | ["this", "card", "mana", "value"]
    ) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)));
    }

    None
}
