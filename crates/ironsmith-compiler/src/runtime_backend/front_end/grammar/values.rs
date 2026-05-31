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
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};
use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use ironsmith_core::ValueSurfaceHint;

use super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, contains_token_word, lex_line, parser_token_word_refs,
};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::token_primitives::find_index;
#[cfg(test)]
use super::super::util::parse_subtype_word;
#[cfg(test)]
use super::super::util::{
    parse_card_type as parse_shared_card_type, parse_supertype_word as parse_shared_supertype_word,
};
use super::super::util::{
    parse_number_word_i32, parse_value_expr_words, source_reference_surface_for_possessive_words,
    token_index_for_word_index, trim_edge_punctuation_tokens,
};
use super::primitives;

type LexedInput<'a> = LexStream<'a>;

const X_VALUE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["x"]);

const WHERE_X_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["where", "x", "is"]);
const SCRYFALL_EMPTY_MANA_COST_MARKERS: &[&str] = &["—"];
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const NUMBER_OF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["number", "of"]);
const PLAYERS_WHO_CONTROL_MORE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["players", "who", "control", "more"]);
const THAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["than"]);
const THAN_YOU_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["than", "you"]);
const EQUAL_TO_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equal", "to"]);
const POWER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["power"]);
const TOUGHNESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["toughness"]);
const SOURCE_POWER_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "power"],
            &["thiss", "power"],
            &["this", "creature", "power"],
            &["this", "creatures", "power"],
            &["thiss", "creature", "power"],
            &["thiss", "creatures", "power"],
            &["its", "power"],
        ]
);
const SOURCE_TOUGHNESS_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "toughness"],
            &["thiss", "toughness"],
            &["this", "creature", "toughness"],
            &["this", "creatures", "toughness"],
            &["thiss", "creature", "toughness"],
            &["thiss", "creatures", "toughness"],
            &["its", "toughness"],
        ]
);
const TAGGED_POWER_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "creature", "power"],
            &["that", "creatures", "power"],
            &["that", "objects", "power"],
        ]
);
const TAGGED_TOUGHNESS_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "creature", "toughness"],
            &["that", "creatures", "toughness"],
            &["that", "objects", "toughness"],
        ]
);
const SACRIFICED_POWER_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "sacrificed", "creature", "power"],
            &["the", "sacrificed", "creatures", "power"],
            &["sacrificed", "creature", "power"],
            &["sacrificed", "creatures", "power"],
        ]
);
const SACRIFICED_TOUGHNESS_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "sacrificed", "creature", "toughness"],
            &["the", "sacrificed", "creatures", "toughness"],
            &["sacrificed", "creature", "toughness"],
            &["sacrificed", "creatures", "toughness"],
        ]
);
const EXPLOITED_POWER_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "exploited", "creature", "power"],
            &["the", "exploited", "creatures", "power"],
            &["exploited", "creature", "power"],
            &["exploited", "creatures", "power"],
        ]
);
const EXPLOITED_TOUGHNESS_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "exploited", "creature", "toughness"],
            &["the", "exploited", "creatures", "toughness"],
            &["exploited", "creature", "toughness"],
            &["exploited", "creatures", "toughness"],
        ]
);
const TAGGED_SPELL_MANA_VALUE_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "spell", "mana", "value"],
            &["that", "spell's", "mana", "value"],
            &["that", "spells", "mana", "value"],
        ]
);
const TAGGED_CARD_OR_SACRIFICED_MANA_VALUE_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "card", "mana", "value"],
            &["that", "card's", "mana", "value"],
            &["that", "cards", "mana", "value"],
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "creature"
            ],
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "artifact"
            ],
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "permanent"
            ],
            &["mana", "value", "of", "the", "sacrificed", "creature"],
            &["mana", "value", "of", "the", "sacrificed", "artifact"],
            &["mana", "value", "of", "the", "sacrificed", "permanent"],
            &["the", "sacrificed", "creature", "mana", "value"],
            &["the", "sacrificed", "artifact", "mana", "value"],
            &["the", "sacrificed", "permanent", "mana", "value"],
            &["the", "sacrificed", "creatures", "mana", "value"],
            &["the", "sacrificed", "artifacts", "mana", "value"],
            &["the", "sacrificed", "permanents", "mana", "value"],
            &["sacrificed", "creature", "mana", "value"],
            &["sacrificed", "artifact", "mana", "value"],
            &["sacrificed", "permanent", "mana", "value"],
            &["sacrificed", "creatures", "mana", "value"],
            &["sacrificed", "artifacts", "mana", "value"],
            &["sacrificed", "permanents", "mana", "value"],
            &["its", "mana", "value"],
        ]
);
const SOURCE_MANA_VALUE_SEGMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "spell", "mana", "value"],
            &["this", "creature", "mana", "value"],
            &["this", "permanent", "mana", "value"],
            &["this", "card", "mana", "value"],
        ]
);
const THAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that"]);
const MANA_VALUE_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["mana", "value"]);
const PLUS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["plus"]);
const SACRIFICED_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["sacrificed"]);

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeLineCst {
    pub(crate) supertypes: Vec<Supertype>,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

pub(crate) fn count_word_value(word: &str) -> Option<u32> {
    ironsmith_core::parse_cardinal_word(word)
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

pub(crate) fn parse_players_who_control_more_than_you_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let mut idx = if WHERE_X_IS_PREFIX_PATTERN.matches_words(&words) {
        3usize
    } else {
        0usize
    };

    if words
        .get(idx)
        .is_some_and(|word| THE_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }
    if NUMBER_OF_PREFIX_PATTERN.matches_words(&words[idx..]) {
        idx += 2;
    }

    if !PLAYERS_WHO_CONTROL_MORE_PREFIX_PATTERN.matches_words(&words[idx..]) {
        return None;
    }
    idx += 4;

    let Some(than_offset) = THAN_WORD_PATTERN.find_word(&words[idx..]) else {
        return None;
    };
    let than_idx = idx + than_offset;
    let tail = &words[than_idx..];
    if !THAN_YOU_TAIL_PATTERN.matches_words(tail) {
        return None;
    }

    let filter_start_token_idx = token_index_for_word_index(tokens, idx)?;
    let filter_end_token_idx = token_index_for_word_index(tokens, than_idx)?;
    let filter_tokens = &tokens[filter_start_token_idx..filter_end_token_idx];
    if filter_tokens.is_empty() {
        return None;
    }

    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(Value::PlayersWhoControlMoreThanYou(filter))
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
    if allow_empty && is_empty_scryfall_mana_cost_text(trimmed) {
        return Ok(ManaCost::new());
    }

    let tokens = lex_line(trimmed, 0)?;
    parse_mana_cost_tokens(&tokens)
}

fn is_empty_scryfall_mana_cost_text(trimmed: &str) -> bool {
    trimmed.is_empty() || SCRYFALL_EMPTY_MANA_COST_MARKERS.contains(&trimmed)
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
    if X_VALUE_WORD_PATTERN.matches_word(&word) {
        return Ok(Value::X);
    }
    if let Ok(value) = word.parse::<i32>() {
        return Ok(Value::Fixed(value));
    }

    let value = ironsmith_core::parse_cardinal_word(&word)
        .ok_or_else(|| primitives::backtrack_err("digit word", "number word (zero-one hundred)"))?;

    Ok(Value::Fixed(value as i32))
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
        if let Some(rest) = primitives::strip_lexed_prefix_phrase(tokens, phrase) {
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
        if let Some(after_is) = primitives::strip_lexed_prefix_phrase(tokens, &["is"])
            && let Some(rest) = primitives::strip_lexed_suffix_phrase(after_is, phrase)
            && !rest.is_empty()
        {
            return Some((operator, rest));
        }

        if let Some(rest) = primitives::strip_lexed_suffix_phrase(tokens, phrase)
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
    parse_shared_card_type(&word.to_ascii_lowercase())
}

#[cfg(test)]
fn parse_supertype_word_for_rewrite(word: &str) -> Option<Supertype> {
    parse_shared_supertype_word(word)
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
    if primitives::parse_prefix(tokens, primitives::phrase(&["any", "number"])).is_some() {
        return Ok(Some((Some(Value::Fixed(0)), None)));
    }

    if let Some((range, _)) = parse_count_range_prefix(tokens) {
        return Ok(Some(range));
    }

    if contains_token_word(tokens, "or") {
        return Ok(Some((Some(Value::Fixed(1)), Some(Value::Fixed(1)))));
    }

    Ok(None)
}

pub(crate) fn parse_number_from_lexed(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    let trimmed = trim_edge_punctuation_tokens(tokens);
    let word_refs = parser_token_word_refs(trimmed);
    let (value, used_words) = ironsmith_core::parse_cardinal_words(&word_refs)?;
    let used_tokens = token_index_for_word_index(trimmed, used_words).unwrap_or(trimmed.len());
    Some((value, used_tokens))
}

pub(crate) fn parse_value_from_lexed(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let trimmed = trim_edge_punctuation_tokens(tokens);
    let word_refs = parser_token_word_refs(trimmed);
    let (value, used_words) = parse_value_expr_words(&word_refs)?;
    let used_tokens = token_index_for_word_index(trimmed, used_words).unwrap_or(trimmed.len());
    Some((value, used_tokens))
}

pub(crate) fn parse_add_mana_equal_amount_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words_all = parser_token_word_refs(tokens);
    let equal_idx = EQUAL_TO_PATTERN.find_exact_window(&words_all, 2)?;
    let tail = &words_all[equal_idx + 2..];
    if tail.is_empty() {
        return None;
    }

    let is_source_power_segment =
        |segment: &[&str]| SOURCE_POWER_SEGMENT_PATTERN.matches_words(segment);
    let is_source_toughness_segment =
        |segment: &[&str]| SOURCE_TOUGHNESS_SEGMENT_PATTERN.matches_words(segment);

    let parse_power_or_toughness_segment = |segment: &[&str]| -> Option<Value> {
        let tagged_it_power = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
        let tagged_it_toughness =
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));

        if segment
            .last()
            .is_some_and(|word| POWER_WORD_PATTERN.matches_word(word))
        {
            if let Some(surface) =
                source_reference_surface_for_possessive_words(&segment[..segment.len() - 1])
            {
                return Some(Value::PowerOf(Box::new(
                    ChooseSpec::Source
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
                )));
            }
        }
        if segment
            .last()
            .is_some_and(|word| TOUGHNESS_WORD_PATTERN.matches_word(word))
        {
            if let Some(surface) =
                source_reference_surface_for_possessive_words(&segment[..segment.len() - 1])
            {
                return Some(Value::ToughnessOf(Box::new(
                    ChooseSpec::Source
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
                )));
            }
        }

        if is_source_power_segment(segment) {
            return Some(Value::PowerOf(Box::new(ChooseSpec::Source)));
        }
        if is_source_toughness_segment(segment) {
            return Some(Value::ToughnessOf(Box::new(ChooseSpec::Source)));
        }
        if TAGGED_POWER_SEGMENT_PATTERN.matches_words(segment) {
            return Some(tagged_it_power.clone());
        }
        if TAGGED_TOUGHNESS_SEGMENT_PATTERN.matches_words(segment) {
            return Some(tagged_it_toughness.clone());
        }
        if SACRIFICED_POWER_SEGMENT_PATTERN.matches_words(segment) {
            return Some(tagged_it_power);
        }
        if EXPLOITED_POWER_SEGMENT_PATTERN.matches_words(segment) {
            return Some(Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                crate::tag::EXPLOITED_TAG,
            )))));
        }
        if SACRIFICED_TOUGHNESS_SEGMENT_PATTERN.matches_words(segment) {
            return Some(tagged_it_toughness);
        }
        if EXPLOITED_TOUGHNESS_SEGMENT_PATTERN.matches_words(segment) {
            return Some(Value::ToughnessOf(Box::new(ChooseSpec::Tagged(
                TagKey::from(crate::tag::EXPLOITED_TAG),
            ))));
        }
        None
    };

    let parse_mana_value_segment = |segment: &[&str]| -> Option<Value> {
        let is_tagged_that_object_mana_value = || {
            if segment.len() < 4
                || !THAT_WORD_PATTERN.matches_word(segment[0])
                || !MANA_VALUE_SUFFIX_PATTERN.matches_words(segment)
            {
                return false;
            }

            !segment[1..segment.len() - 2].is_empty()
        };

        if TAGGED_SPELL_MANA_VALUE_SEGMENT_PATTERN.matches_words(segment) {
            return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                TagKey::from(IT_TAG),
            ))));
        }
        if TAGGED_CARD_OR_SACRIFICED_MANA_VALUE_SEGMENT_PATTERN.matches_words(segment)
            || is_tagged_that_object_mana_value()
        {
            return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                TagKey::from(IT_TAG),
            ))));
        }
        if SOURCE_MANA_VALUE_SEGMENT_PATTERN.matches_words(segment) {
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

    if let Some(plus_idx) = find_index(tail, |word| PLUS_WORD_PATTERN.matches_word(word))
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
        || TAGGED_POWER_SEGMENT_PATTERN.matches_words(tail)
        || SACRIFICED_POWER_SEGMENT_PATTERN.matches_words(tail)
    {
        let source = if THAT_WORD_PATTERN.matches_word(tail[0])
            || SACRIFICED_MARKER_PATTERN.matches_words(tail)
        {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::PowerOf(Box::new(source)));
    }

    if is_source_toughness_segment(tail)
        || TAGGED_TOUGHNESS_SEGMENT_PATTERN.matches_words(tail)
        || SACRIFICED_TOUGHNESS_SEGMENT_PATTERN.matches_words(tail)
    {
        let source = if THAT_WORD_PATTERN.matches_word(tail[0])
            || SACRIFICED_MARKER_PATTERN.matches_words(tail)
        {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::ToughnessOf(Box::new(source)));
    }

    if TAGGED_SPELL_MANA_VALUE_SEGMENT_PATTERN.matches_words(tail) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }
    if TAGGED_CARD_OR_SACRIFICED_MANA_VALUE_SEGMENT_PATTERN.matches_words(tail) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }
    if SOURCE_MANA_VALUE_SEGMENT_PATTERN.matches_words(tail) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)));
    }

    None
}
