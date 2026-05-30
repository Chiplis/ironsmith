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
use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use ironsmith_core::ValueSurfaceHint;

use super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, contains_token_word, lex_line, parser_token_word_refs,
    word_slice_at_is, word_slice_contains_word, word_slice_eq, word_slice_eq_any,
    word_slice_find_phrase_start, word_slice_find_phrase_start_or_zero, word_slice_last_is,
    word_slice_starts_with, word_slice_starts_with_any,
};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::token_primitives::find_index;
#[cfg(test)]
use super::super::util::parse_subtype_word;
use super::super::util::{
    parse_number_word_i32, parse_value_expr_words, source_reference_surface_for_possessive_words,
    token_index_for_word_index, trim_edge_punctuation_tokens,
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
    let mut idx = if word_slice_starts_with(&words, &["where", "x", "is"]) {
        3usize
    } else {
        0usize
    };

    if word_slice_at_is(&words, idx, "the") {
        idx += 1;
    }
    if word_slice_at_is(&words, idx, "number") {
        if !word_slice_at_is(&words, idx + 1, "of") {
            return None;
        }
        idx += 2;
    }

    if words.get(idx).copied() != Some("players") {
        return None;
    }
    idx += 1;

    if words.get(idx).copied() != Some("who") {
        return None;
    }
    idx += 1;

    if words.get(idx).copied() != Some("control") {
        return None;
    }
    idx += 1;

    if words.get(idx).copied() != Some("more") {
        return None;
    }
    idx += 1;

    let Some(than_offset) = word_slice_find_phrase_start(&words[idx..], &["than"]) else {
        return None;
    };
    let than_idx = idx + than_offset;
    let tail = &words[than_idx..];
    if !word_slice_eq(tail, &["than", "you"]) {
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
    let equal_idx = word_slice_find_phrase_start_or_zero(&words_all, &["equal", "to"])?;
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

        if segment.len() >= 2 && word_slice_last_is(segment, "power") {
            if let Some(surface) =
                source_reference_surface_for_possessive_words(&segment[..segment.len() - 1])
            {
                return Some(Value::PowerOf(Box::new(
                    ChooseSpec::Source
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
                )));
            }
        }
        if segment.len() >= 2 && word_slice_last_is(segment, "toughness") {
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
        if word_slice_eq_any(
            segment,
            &[
                &["that", "creature", "power"],
                &["that", "creatures", "power"],
                &["that", "objects", "power"],
            ],
        ) {
            return Some(tagged_it_power.clone());
        }
        if word_slice_eq_any(
            segment,
            &[
                &["that", "creature", "toughness"],
                &["that", "creatures", "toughness"],
                &["that", "objects", "toughness"],
            ],
        ) {
            return Some(tagged_it_toughness.clone());
        }
        if word_slice_eq_any(
            segment,
            &[
                &["the", "sacrificed", "creature", "power"],
                &["the", "sacrificed", "creatures", "power"],
                &["sacrificed", "creature", "power"],
                &["sacrificed", "creatures", "power"],
            ],
        ) {
            return Some(tagged_it_power);
        }
        if word_slice_eq_any(
            segment,
            &[
                &["the", "exploited", "creature", "power"],
                &["the", "exploited", "creatures", "power"],
                &["exploited", "creature", "power"],
                &["exploited", "creatures", "power"],
            ],
        ) {
            return Some(Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                crate::tag::EXPLOITED_TAG,
            )))));
        }
        if word_slice_eq_any(
            segment,
            &[
                &["the", "sacrificed", "creature", "toughness"],
                &["the", "sacrificed", "creatures", "toughness"],
                &["sacrificed", "creature", "toughness"],
                &["sacrificed", "creatures", "toughness"],
            ],
        ) {
            return Some(tagged_it_toughness);
        }
        if word_slice_eq_any(
            segment,
            &[
                &["the", "exploited", "creature", "toughness"],
                &["the", "exploited", "creatures", "toughness"],
                &["exploited", "creature", "toughness"],
                &["exploited", "creatures", "toughness"],
            ],
        ) {
            return Some(Value::ToughnessOf(Box::new(ChooseSpec::Tagged(
                TagKey::from(crate::tag::EXPLOITED_TAG),
            ))));
        }
        None
    };

    const TAGGED_SPELL_MANA_VALUE_PREFIXES: &[&[&str]] = &[
        &["that", "spell", "mana", "value"],
        &["that", "spell's", "mana", "value"],
        &["that", "spells", "mana", "value"],
    ];
    const TAGGED_CARD_OR_SACRIFICED_MANA_VALUE_PREFIXES: &[&[&str]] = &[
        &["that", "card", "mana", "value"],
        &["that", "card's", "mana", "value"],
        &["that", "cards", "mana", "value"],
        &["the", "revealed", "card", "mana", "value"],
        &["the", "revealed", "card's", "mana", "value"],
        &["the", "revealed", "cards", "mana", "value"],
        &["revealed", "card", "mana", "value"],
        &["revealed", "card's", "mana", "value"],
        &["revealed", "cards", "mana", "value"],
        &[
            "the",
            "mana",
            "value",
            "of",
            "the",
            "sacrificed",
            "creature",
        ],
        &[
            "the",
            "mana",
            "value",
            "of",
            "the",
            "sacrificed",
            "artifact",
        ],
        &[
            "the",
            "mana",
            "value",
            "of",
            "the",
            "sacrificed",
            "permanent",
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
    ];
    const TAGGED_POWER_PREFIXES: &[&[&str]] = &[
        &["that", "creature", "power"],
        &["that", "creatures", "power"],
        &["that", "objects", "power"],
        &["the", "sacrificed", "creature", "power"],
        &["the", "sacrificed", "creatures", "power"],
        &["sacrificed", "creature", "power"],
        &["sacrificed", "creatures", "power"],
    ];
    const TAGGED_TOUGHNESS_PREFIXES: &[&[&str]] = &[
        &["that", "creature", "toughness"],
        &["that", "creatures", "toughness"],
        &["that", "objects", "toughness"],
        &["the", "sacrificed", "creature", "toughness"],
        &["the", "sacrificed", "creatures", "toughness"],
        &["sacrificed", "creature", "toughness"],
        &["sacrificed", "creatures", "toughness"],
    ];

    let parse_mana_value_segment = |segment: &[&str]| -> Option<Value> {
        let is_tagged_that_object_mana_value = || {
            if segment.len() < 4
                || segment[0] != "that"
                || segment[segment.len() - 2] != "mana"
                || segment[segment.len() - 1] != "value"
            {
                return false;
            }

            !segment[1..segment.len() - 2].is_empty()
        };

        if word_slice_starts_with_any(segment, TAGGED_SPELL_MANA_VALUE_PREFIXES) {
            return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                TagKey::from(IT_TAG),
            ))));
        }
        if word_slice_starts_with_any(segment, TAGGED_CARD_OR_SACRIFICED_MANA_VALUE_PREFIXES)
            || is_tagged_that_object_mana_value()
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

    if is_source_power_segment(tail) || word_slice_starts_with_any(tail, TAGGED_POWER_PREFIXES) {
        let source = if tail[0] == "that" || word_slice_contains_word(&tail, "sacrificed") {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::PowerOf(Box::new(source)));
    }

    if is_source_toughness_segment(tail)
        || word_slice_starts_with_any(tail, TAGGED_TOUGHNESS_PREFIXES)
    {
        let source = if tail[0] == "that" || word_slice_contains_word(&tail, "sacrificed") {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::ToughnessOf(Box::new(source)));
    }

    if word_slice_starts_with_any(tail, TAGGED_SPELL_MANA_VALUE_PREFIXES) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }
    if word_slice_starts_with_any(tail, TAGGED_CARD_OR_SACRIFICED_MANA_VALUE_PREFIXES) {
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
