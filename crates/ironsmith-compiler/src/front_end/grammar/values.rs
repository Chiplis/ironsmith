use winnow::combinator::{alt, cut_err, eof, opt, peek, preceded, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{Value, ValueComparisonOperator};
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, PlayerFilter, SacrificedObjectKind};
use crate::types::{CardType, Subtype, Supertype};
use ironsmith_core::ValueSurfaceHint;

use super::super::lexer::{
    LexStream, LexedClause, OwnedLexToken, TokenKind, lex_line, parser_token_word_refs,
};
use super::super::object_filters::parse_object_filter_lexed;
#[cfg(test)]
use super::super::util::parse_subtype_word;
#[cfg(test)]
use super::super::util::{
    parse_card_type as parse_shared_card_type, parse_supertype_word as parse_shared_supertype_word,
};
use super::super::util::{
    parse_number_word_i32, parse_number_word_u32, parse_value_expr_words,
    source_reference_surface_for_possessive_words, trim_edge_punctuation_tokens,
};
use super::{leaf, primitives};

type LexedInput<'a> = LexStream<'a>;

const SCRYFALL_EMPTY_MANA_COST_MARKERS: &[&str] = &["—"];
const EQUAL_TO_PHRASE: &[&str] = &["equal", "to"];
const POWER_WORD: &str = "power";
const TOUGHNESS_WORD: &str = "toughness";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueStatSubjectShape {
    Source,
    Tagged,
    Exploited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueStatAxisShape {
    Power,
    Toughness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValueStatSegmentShape {
    subject: ValueStatSubjectShape,
    axis: ValueStatAxisShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueManaValueSubjectShape {
    Source,
    Tagged,
    TaggedPossessivePronoun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValueManaValueSegmentShape {
    subject: ValueManaValueSubjectShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayersWhoControlDomainShape {
    Players,
    Opponents,
}

impl PlayersWhoControlDomainShape {
    fn player_filter(self) -> PlayerFilter {
        match self {
            Self::Players => PlayerFilter::Any,
            Self::Opponents => PlayerFilter::Opponent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlayersWhoControlMoreValueShape<'a> {
    players: PlayersWhoControlDomainShape,
    filter_tokens: &'a [OwnedLexToken],
    minimum_difference_token: Option<&'a [OwnedLexToken]>,
}
const THAT_WORD: &str = "that";
const MANA_VALUE_SUFFIX: &[&str] = &["mana", "value"];
const PLUS_WORD: &str = "plus";

fn parse_players_who_control_more_value_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PlayersWhoControlMoreValueShape<'a>> {
    opt(primitives::phrase(&["where", "x", "is"])).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    opt(primitives::phrase(&["number", "of"])).parse_next(input)?;
    let players = alt((
        primitives::kw("players").value(PlayersWhoControlDomainShape::Players),
        primitives::kw("opponents").value(PlayersWhoControlDomainShape::Opponents),
    ))
    .parse_next(input)?;
    primitives::phrase(&["who", "control"]).parse_next(input)?;
    let minimum_difference_token = if opt(primitives::phrase(&["at", "least"]))
        .parse_next(input)?
        .is_some()
    {
        Some(any.void().take().parse_next(input)?)
    } else {
        None
    };
    primitives::kw("more").parse_next(input)?;
    let filter_tokens = repeat_till(1.., any.void(), peek(primitives::phrase(&["than", "you"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::phrase(&["than", "you"]).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(PlayersWhoControlMoreValueShape {
        players,
        filter_tokens,
        minimum_difference_token,
    })
}

fn parse_players_who_control_more_value_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayersWhoControlMoreValueShape<'_>> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_players_who_control_more_value_shape_lexed,
        "players-who-control-more-value",
    )
    .ok()
}

fn parse_value_stat_segment_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ValueStatSegmentShape> {
    let subject = alt((
        primitives::any_phrase(&[
            &["this"],
            &["thiss"],
            &["this", "creature"],
            &["this", "creatures"],
            &["thiss", "creature"],
            &["thiss", "creatures"],
            &["its"],
        ])
        .value(ValueStatSubjectShape::Source),
        primitives::any_phrase(&[
            &["the", "exploited", "creature"],
            &["the", "exploited", "creatures"],
            &["exploited", "creature"],
            &["exploited", "creatures"],
        ])
        .value(ValueStatSubjectShape::Exploited),
        primitives::any_phrase(&[
            &["that", "creature"],
            &["that", "creatures"],
            &["that", "objects"],
            &["the", "sacrificed", "creature"],
            &["the", "sacrificed", "creatures"],
            &["sacrificed", "creature"],
            &["sacrificed", "creatures"],
            &["the", "exiled", "card"],
            &["the", "exiled", "card's"],
            &["the", "exiled", "cards"],
            &["exiled", "card"],
            &["exiled", "card's"],
            &["exiled", "cards"],
        ])
        .value(ValueStatSubjectShape::Tagged),
    ))
    .parse_next(input)?;
    let axis = alt((
        primitives::kw("power").value(ValueStatAxisShape::Power),
        primitives::kw("toughness").value(ValueStatAxisShape::Toughness),
    ))
    .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(ValueStatSegmentShape { subject, axis })
}

fn parse_value_stat_segment_shape(clause: LexedClause<'_>) -> Option<ValueStatSegmentShape> {
    primitives::parse_all(
        clause.tokens(),
        parse_value_stat_segment_shape_lexed,
        "value-stat-segment",
    )
    .ok()
}

fn value_from_stat_segment_shape(shape: ValueStatSegmentShape) -> Value {
    let choose_spec = match shape.subject {
        ValueStatSubjectShape::Source => ChooseSpec::Source,
        ValueStatSubjectShape::Tagged => ChooseSpec::Tagged(TagKey::from(IT_TAG)),
        ValueStatSubjectShape::Exploited => {
            ChooseSpec::Tagged(TagKey::from(crate::tag::EXPLOITED_TAG))
        }
    };
    match shape.axis {
        ValueStatAxisShape::Power => Value::PowerOf(Box::new(choose_spec)),
        ValueStatAxisShape::Toughness => Value::ToughnessOf(Box::new(choose_spec)),
    }
}

fn parse_value_stat_segment(clause: LexedClause<'_>) -> Option<Value> {
    parse_value_stat_segment_shape(clause).map(value_from_stat_segment_shape)
}

fn parse_value_mana_value_segment_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ValueManaValueSegmentShape> {
    let checkpoint = input.checkpoint();
    let source_matches = primitives::any_phrase(&[
        &["this", "spell", "mana", "value"],
        &["this", "creature", "mana", "value"],
        &["this", "permanent", "mana", "value"],
        &["this", "card", "mana", "value"],
    ])
    .parse_next(input)
    .is_ok();
    let source_is_complete = source_matches && {
        let result: WResult<&[OwnedLexToken]> = eof.parse_next(input);
        result.is_ok()
    };
    if source_is_complete {
        return Ok(ValueManaValueSegmentShape {
            subject: ValueManaValueSubjectShape::Source,
        });
    }
    input.reset(&checkpoint);

    let pronoun_matches = primitives::phrase(&["its", "mana", "value"])
        .parse_next(input)
        .is_ok();
    let pronoun_is_complete = pronoun_matches && {
        let result: WResult<&[OwnedLexToken]> = eof.parse_next(input);
        result.is_ok()
    };
    if pronoun_is_complete {
        return Ok(ValueManaValueSegmentShape {
            subject: ValueManaValueSubjectShape::TaggedPossessivePronoun,
        });
    }
    input.reset(&checkpoint);

    primitives::any_phrase(&[
        &["that", "spell", "mana", "value"],
        &["that", "spell's", "mana", "value"],
        &["that", "spells", "mana", "value"],
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
            "enchantment",
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
        &["mana", "value", "of", "the", "sacrificed", "enchantment"],
        &["mana", "value", "of", "the", "sacrificed", "permanent"],
        &["the", "sacrificed", "creature", "mana", "value"],
        &["the", "sacrificed", "artifact", "mana", "value"],
        &["the", "sacrificed", "enchantment", "mana", "value"],
        &["the", "sacrificed", "permanent", "mana", "value"],
        &["the", "sacrificed", "creatures", "mana", "value"],
        &["the", "sacrificed", "artifacts", "mana", "value"],
        &["the", "sacrificed", "enchantments", "mana", "value"],
        &["the", "sacrificed", "permanents", "mana", "value"],
        &["sacrificed", "creature", "mana", "value"],
        &["sacrificed", "artifact", "mana", "value"],
        &["sacrificed", "enchantment", "mana", "value"],
        &["sacrificed", "permanent", "mana", "value"],
        &["sacrificed", "creatures", "mana", "value"],
        &["sacrificed", "artifacts", "mana", "value"],
        &["sacrificed", "enchantments", "mana", "value"],
        &["sacrificed", "permanents", "mana", "value"],
    ])
    .parse_next(input)?;
    let _: Vec<&OwnedLexToken> = repeat(0.., any).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(ValueManaValueSegmentShape {
        subject: ValueManaValueSubjectShape::Tagged,
    })
}

fn parse_value_mana_value_segment_shape(
    clause: LexedClause<'_>,
) -> Option<ValueManaValueSegmentShape> {
    primitives::parse_all(
        clause.tokens(),
        parse_value_mana_value_segment_shape_lexed,
        "value-mana-value-segment",
    )
    .ok()
}

fn value_from_mana_value_segment_shape(shape: ValueManaValueSegmentShape) -> Value {
    let choose_spec = match shape.subject {
        ValueManaValueSubjectShape::Source => ChooseSpec::Source,
        ValueManaValueSubjectShape::Tagged => ChooseSpec::Tagged(TagKey::from(IT_TAG)),
        ValueManaValueSubjectShape::TaggedPossessivePronoun => ChooseSpec::Tagged(TagKey::from(
            IT_TAG,
        ))
        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
        )),
    };
    Value::ManaValueOf(Box::new(choose_spec))
}

fn parse_value_mana_value_segment(clause: LexedClause<'_>) -> Option<Value> {
    parse_value_mana_value_segment_shape(clause).map(value_from_mana_value_segment_shape)
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeLineCst {
    pub(crate) supertypes: Vec<Supertype>,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
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
    let tokens = trim_edge_punctuation_tokens(tokens);
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
    let shape = parse_players_who_control_more_value_shape(tokens)?;
    let filter = parse_object_filter_lexed(shape.filter_tokens, false).ok()?;
    let players = shape.players.player_filter();
    let Some(minimum_difference_token) = shape.minimum_difference_token else {
        return Some(Value::PlayersWhoControlMoreThanYou { players, filter });
    };
    let [minimum_difference_token] = minimum_difference_token else {
        return None;
    };
    let minimum_difference = parse_number_word_u32(minimum_difference_token.parser_text())
        .or_else(|| minimum_difference_token.parser_text().parse::<u32>().ok())?;
    Some(Value::PlayersWhoControlAtLeastMoreThanYou {
        players,
        filter,
        minimum_difference,
    })
}

pub(crate) fn parse_mana_symbol(raw: &str) -> Result<ManaSymbol, CardTextError> {
    super::leaf::parse_leaf_mana_symbol_complete(raw)
}

pub(crate) fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    super::leaf::parse_leaf_mana_symbol_group_complete(raw)
}

#[cfg(test)]
pub(crate) fn parse_mana_symbol_group_rewrite(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let tokens = lex_line(raw.trim(), 0)?;
    parse_mana_symbol_group_tokens(&tokens)
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

#[cfg(test)]
pub(crate) fn parse_mana_symbol_group_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Vec<ManaSymbol>, CardTextError> {
    super::leaf::parse_leaf_mana_symbol_group_tokens(tokens)
}

pub(crate) fn parse_mana_cost_tokens(tokens: &[OwnedLexToken]) -> Result<ManaCost, CardTextError> {
    super::leaf::parse_leaf_mana_cost_tokens(tokens)
}

pub(crate) fn parse_value_comparison_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(ValueComparisonOperator, &[OwnedLexToken])> {
    for (phrase, operator) in [
        (&["is", "exactly"][..], ValueComparisonOperator::Equal),
        (&["exactly"][..], ValueComparisonOperator::Equal),
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

pub(crate) fn parse_value_comparison_words<'a>(
    words: &'a [&'a str],
) -> Option<(ValueComparisonOperator, &'a [&'a str], usize)> {
    for (phrase, operator) in [
        (&["is", "exactly"][..], ValueComparisonOperator::Equal),
        (&["exactly"][..], ValueComparisonOperator::Equal),
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
        if let Some(rest) = primitives::parse_word_sequence_prefix(words, phrase) {
            return Some((operator, rest, phrase.len()));
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
        if let Some(after_is) = primitives::parse_word_sequence_prefix(words, &["is"])
            && let Some(operand) = primitives::parse_word_sequence_suffix(after_is, phrase)
            && !operand.is_empty()
        {
            return Some((operator, operand, words.len().saturating_sub(operand.len())));
        }

        if let Some(operand) = primitives::parse_word_sequence_suffix(words, phrase)
            && !operand.is_empty()
        {
            return Some((operator, operand, words.len().saturating_sub(operand.len())));
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

    let mut subtypes = Vec::new();
    let mut index = 0;
    while index < right_words.len() {
        if right_words.get(index..index + 2).is_some_and(|words| {
            words[0].eq_ignore_ascii_case("time") && words[1].eq_ignore_ascii_case("lord")
        }) {
            subtypes.push(Subtype::TimeLord);
            index += 2;
            continue;
        }
        if let Some(subtype) = parse_subtype(right_words[index]) {
            subtypes.push(subtype);
        }
        index += 1;
    }

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
    Ok(
        super::leaf::parse_leaf_modal_choose_range_tokens(tokens)?
            .map(|range| range.into_min_max()),
    )
}

pub(crate) fn parse_number_prefix_lexed(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    let trimmed = trim_edge_punctuation_tokens(tokens);
    let (value, rest) = primitives::parse_prefix(trimmed, leaf::parse_leaf_number_prefix_lexed)?;
    let used_tokens = trimmed.len().saturating_sub(rest.len());
    Some((value, used_tokens))
}

pub(crate) fn parse_value_prefix_lexed(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let trimmed = trim_edge_punctuation_tokens(tokens);
    let clause = LexedClause::new(trimmed);
    let word_refs = parser_token_word_refs(trimmed);
    let (value, used_words) = parse_value_expr_words(&word_refs)?;
    let used_tokens = clause
        .token_index_after_words(used_words)
        .unwrap_or(trimmed.len());
    Some((value, used_tokens))
}

pub(crate) fn parse_add_mana_equal_amount_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
    fn canonical_add_mana_equal_amount_value(value: Value) -> Value {
        match value {
            Value::SourcePower => Value::PowerOf(Box::new(ChooseSpec::Source)),
            Value::SourceToughness => Value::ToughnessOf(Box::new(ChooseSpec::Source)),
            Value::Add(left, right) => Value::Add(
                Box::new(canonical_add_mana_equal_amount_value(*left)),
                Box::new(canonical_add_mana_equal_amount_value(*right)),
            ),
            other => other,
        }
    }

    let (_, _, tail_tokens) =
        primitives::find_prefix(tokens, || primitives::phrase(EQUAL_TO_PHRASE))?;
    let tail_clause = LexedClause::new(tail_tokens).trimmed();
    let tail = tail_clause.word_refs();
    if tail.is_empty() {
        return None;
    }

    let segment_clause = |start: usize, end: usize| -> Option<LexedClause<'_>> {
        tail_clause.between_word_range(start, end)
    };

    let sacrificed_object_kind = |words: &[&str]| -> Option<SacrificedObjectKind> {
        words.windows(2).find_map(|pair| {
            if pair[0] != "sacrificed" {
                return None;
            }
            match pair[1] {
                "creature" | "creatures" | "creature's" => Some(SacrificedObjectKind::Creature),
                "artifact" | "artifacts" | "artifact's" => Some(SacrificedObjectKind::Artifact),
                "enchantment" | "enchantments" | "enchantment's" => {
                    Some(SacrificedObjectKind::Enchantment)
                }
                "permanent" | "permanents" | "permanent's" => Some(SacrificedObjectKind::Permanent),
                _ => None,
            }
        })
    };

    let parse_power_or_toughness_segment =
        |segment: &[&str], segment_clause: LexedClause<'_>| -> Option<Value> {
            if segment.last().copied() == Some(POWER_WORD)
                && let Some(surface) =
                    source_reference_surface_for_possessive_words(&segment[..segment.len() - 1])
            {
                return Some(Value::PowerOf(Box::new(
                    ChooseSpec::Source
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
                )));
            }
            if segment.last().copied() == Some(TOUGHNESS_WORD)
                && let Some(surface) =
                    source_reference_surface_for_possessive_words(&segment[..segment.len() - 1])
            {
                return Some(Value::ToughnessOf(Box::new(
                    ChooseSpec::Source
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
                )));
            }

            parse_value_stat_segment(segment_clause)
        };

    let parse_mana_value_segment =
        |segment: &[&str], segment_clause: LexedClause<'_>| -> Option<Value> {
            let is_tagged_that_object_mana_value = || {
                if segment.len() < 4 || segment[0] != THAT_WORD {
                    return false;
                }
                let suffix_start = segment.len().saturating_sub(MANA_VALUE_SUFFIX.len());
                for (idx, expected) in MANA_VALUE_SUFFIX.iter().copied().enumerate() {
                    if segment.get(suffix_start + idx).copied() != Some(expected) {
                        return false;
                    }
                }

                !segment[1..segment.len() - 2].is_empty()
            };

            if let Some(value) = parse_value_mana_value_segment(segment_clause) {
                return Some(value);
            }
            if is_tagged_that_object_mana_value() {
                return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                    TagKey::from(IT_TAG),
                ))));
            }
            None
        };

    let parse_amount_segment = |start: usize, end: usize| -> Option<Value> {
        let segment = &tail[start..end];
        let segment_clause = segment_clause(start, end)?;
        let value = parse_mana_value_segment(segment, segment_clause)
            .or_else(|| {
                parse_value_expr_words(segment)
                    .and_then(|(value, used)| (used == segment.len()).then_some(value))
            })
            .or_else(|| parse_power_or_toughness_segment(segment, segment_clause))
            .or_else(|| {
                if segment.len() == 1 {
                    parse_number_word_i32(segment[0]).map(Value::Fixed)
                } else {
                    None
                }
            })?;
        Some(match sacrificed_object_kind(segment) {
            Some(kind) => value.with_surface_hint(ValueSurfaceHint::SacrificedObject(kind)),
            None => value,
        })
    };

    let difference_prefix_len = if tail.starts_with(&["the", "difference", "between"]) {
        Some(3)
    } else if tail.starts_with(&["difference", "between"]) {
        Some(2)
    } else {
        None
    };
    if let Some(prefix_len) = difference_prefix_len
        && let Some(and_offset) = tail[prefix_len..].iter().position(|word| *word == "and")
    {
        let and_idx = prefix_len + and_offset;
        let parse_difference_segment = |start: usize, end: usize| -> Option<Value> {
            let segment = &tail[start..end];
            if segment.first().copied() == Some("that")
                && segment
                    .iter()
                    .any(|word| matches!(*word, "spell" | "spells" | "spell's"))
                && segment.ends_with(&["mana", "value"])
            {
                return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                    TagKey::from("triggering"),
                ))));
            }
            if segment.first().copied() == Some("that")
                && segment.len() > 3
                && segment.ends_with(&["mana", "value"])
            {
                let mut reference_words = segment[..segment.len() - 2].to_vec();
                if let Some(noun) = reference_words.last_mut()
                    && noun.ends_with('s')
                {
                    *noun = noun.trim_end_matches('s');
                }
                let surface = reference_words.join(" ");
                return Some(Value::ManaValueOf(Box::new(
                    ChooseSpec::Tagged(TagKey::from(IT_TAG)).with_surface_hint(
                        ChooseSpecSurfaceHint::SourceReference(
                            crate::target::SourceReferenceSurface::ThisPermanentType(surface),
                        ),
                    ),
                )));
            }
            parse_amount_segment(start, end)
        };
        if and_idx > prefix_len
            && and_idx + 1 < tail.len()
            && let Some(left) = parse_difference_segment(prefix_len, and_idx)
            && let Some(right) = parse_difference_segment(and_idx + 1, tail.len())
        {
            let absolute = Value::absolute_difference(left, right);
            return Some(absolute.with_surface_hint(ValueSurfaceHint::Difference));
        }
    }

    let mut plus_idx = None;
    for (idx, word) in tail.iter().copied().enumerate() {
        if word == PLUS_WORD {
            plus_idx = Some(idx);
            break;
        }
    }

    if let Some(plus_idx) = plus_idx
        && plus_idx > 0
        && plus_idx + 1 < tail.len()
        && let Some(left) = parse_amount_segment(0, plus_idx)
        && let Some(right) = parse_amount_segment(plus_idx + 1, tail.len())
    {
        return Some(canonical_add_mana_equal_amount_value(Value::Add(
            Box::new(left),
            Box::new(right),
        )));
    }

    if let Some(value) = parse_amount_segment(0, tail.len()) {
        return Some(canonical_add_mana_equal_amount_value(value));
    }

    None
}

#[cfg(test)]
mod migrated_shape_tests {
    use super::*;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn value_comparison_accepts_authored_is_exactly_surface() {
        let tokens = lex("is exactly 20");
        let (operator, remaining) = parse_value_comparison_tokens(&tokens)
            .expect("is exactly should be an equality comparison");
        assert_eq!(operator, ValueComparisonOperator::Equal);
        assert_eq!(parser_token_word_refs(remaining), vec!["20"]);
    }

    #[test]
    fn parses_players_who_control_more_filter_shape() {
        let parsed = parse_players_who_control_more_than_you_value_lexed(&lex(
            "the number of players who control more lands than you",
        ))
        .unwrap();
        let Value::PlayersWhoControlMoreThanYou { players, filter } = parsed else {
            panic!("expected players-who-control-more value");
        };
        assert_eq!(players, PlayerFilter::Any);
        assert_eq!(filter.card_types, vec![CardType::Land]);
    }

    #[test]
    fn parses_opponents_who_control_more_and_at_least_shapes() {
        let parsed = parse_players_who_control_more_than_you_value_lexed(&lex(
            "the number of opponents who control more creatures than you",
        ))
        .unwrap();
        let Value::PlayersWhoControlMoreThanYou { players, filter } = parsed else {
            panic!("expected opponents-who-control-more value");
        };
        assert_eq!(players, PlayerFilter::Opponent);
        assert_eq!(filter.card_types, vec![CardType::Creature]);

        let parsed = parse_players_who_control_more_than_you_value_lexed(&lex(
            "the number of opponents who control at least two more lands than you",
        ))
        .unwrap();
        let Value::PlayersWhoControlAtLeastMoreThanYou {
            players,
            filter,
            minimum_difference,
        } = parsed
        else {
            panic!("expected opponents-who-control-at-least-more value");
        };
        assert_eq!(players, PlayerFilter::Opponent);
        assert_eq!(filter.card_types, vec![CardType::Land]);
        assert_eq!(minimum_difference, 2);
    }

    #[test]
    fn max_cards_in_hand_accepts_sentence_punctuation() {
        assert_eq!(
            parse_max_cards_in_hand_value_lexed(&lex(
                "cards in the hand of the opponent with the most cards in hand."
            )),
            Some(Value::MaxCardsInHand(PlayerFilter::Opponent))
        );
    }

    #[test]
    fn type_line_keeps_time_lord_distinct_from_doctor() {
        let parsed = parse_type_line_rewrite("Legendary Creature — Time Lord Doctor")
            .expect("Time Lord type line should parse");
        assert_eq!(parsed.subtypes, vec![Subtype::TimeLord, Subtype::Doctor]);
    }

    #[test]
    fn parses_stat_and_mana_value_segment_shapes() {
        let stat_tokens = lex("that creature power");
        assert_eq!(
            parse_value_stat_segment_shape(LexedClause::new(&stat_tokens)),
            Some(ValueStatSegmentShape {
                subject: ValueStatSubjectShape::Tagged,
                axis: ValueStatAxisShape::Power,
            })
        );

        let mana_tokens = lex("that spell mana value");
        assert_eq!(
            parse_value_mana_value_segment_shape(LexedClause::new(&mana_tokens)),
            Some(ValueManaValueSegmentShape {
                subject: ValueManaValueSubjectShape::Tagged,
            })
        );

        let possessive_tokens = lex("its mana value");
        let possessive_shape =
            parse_value_mana_value_segment_shape(LexedClause::new(&possessive_tokens))
                .expect("possessive mana value should parse");
        assert_eq!(
            possessive_shape,
            ValueManaValueSegmentShape {
                subject: ValueManaValueSubjectShape::TaggedPossessivePronoun,
            }
        );
        let Value::ManaValueOf(spec) = value_from_mana_value_segment_shape(possessive_shape) else {
            panic!("possessive shape should lower to a mana-value reference");
        };
        assert_eq!(
            spec.source_reference_surface(),
            Some(&crate::target::SourceReferenceSurface::ThisPermanentType(
                "it".to_string()
            ))
        );
    }

    #[test]
    fn lady_loki_parses_absolute_mana_value_difference() {
        let value = parse_add_mana_equal_amount_value_lexed(&lex(
            "equal to the difference between that spell's mana value and that nonland card's mana value",
        ))
        .expect("Lady Loki mana-value difference should parse");
        let debug = format!("{value:#?}");

        assert!(
            value.has_surface_hint(ValueSurfaceHint::Difference),
            "{debug}"
        );
        assert!(debug.contains("Min"), "{debug}");
        assert!(debug.contains("triggering"), "{debug}");
        assert!(debug.contains("__it__"), "{debug}");
    }

    #[test]
    fn parses_add_mana_equal_amount_tail_with_typed_shape() {
        let parsed =
            parse_add_mana_equal_amount_value_lexed(&lex("add mana equal to that creature power"))
                .unwrap();
        assert_eq!(
            parsed,
            Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        );
    }
}
