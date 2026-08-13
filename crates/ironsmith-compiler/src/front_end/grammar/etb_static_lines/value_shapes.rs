use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbReferenceValueKind {
    SacrificedCreaturePower,
    SacrificedCreatureToughness,
    TaggedCreatureManaValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhereXFixedPlusReferenceSpec<'a> {
    pub(crate) fixed_tokens: &'a [OwnedLexToken],
    pub(crate) reference_kind: EtbReferenceValueKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WhereXPlayerMetric {
    LifeGainedByYouThisTurn,
    LifeLostByYouThisTurn,
    LifeLostByOpponentsThisTurn,
    OpponentsDealtCombatDamageThisTurn,
    NoncombatDamageDealtToOpponentsThisTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbAggregateKind {
    Total,
    Greatest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbAggregateValueKind {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhereXAggregateFilterSpec<'a> {
    pub(crate) aggregate: EtbAggregateKind,
    pub(crate) value_kind: EtbAggregateValueKind,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhereXNumberOfFilterSpec<'a> {
    pub(crate) multiplier: i32,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhereXFixedPlusNumberOfFilterSpec<'a> {
    pub(crate) fixed_tokens: &'a [OwnedLexToken],
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbNumberOffsetOperator {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhereXNumberOfFilterOffsetSpec<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) operator: EtbNumberOffsetOperator,
    pub(crate) offset_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbSourceStatKind {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtbSourceStatFallback {
    Source,
    TaggedObject,
    TriggeringSpell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WhereXSourceStatSpec<'a> {
    pub(crate) kind: EtbSourceStatKind,
    pub(crate) reference_tokens: &'a [OwnedLexToken],
    pub(crate) fallback: Option<EtbSourceStatFallback>,
    pub(crate) as_this_ability_resolves: bool,
}

pub(crate) fn parse_equal_to_value_body_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(
        tokens,
        parse_equal_to_value_body_lexed,
        "equal-to-value-body",
    )
    .ok()
}

pub(crate) fn parse_equal_to_mana_spent_to_cast_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_equal_to_mana_spent_to_cast_lexed,
        "equal-to-mana-spent-to-cast",
    )
    .is_ok()
}

pub(crate) fn parse_equal_to_greatest_cards_drawn_this_turn_tokens(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::parse_all(
        tokens,
        parse_equal_to_greatest_cards_drawn_this_turn_lexed,
        "equal-to-greatest-cards-drawn-this-turn",
    )
    .is_ok()
}

pub(crate) fn parse_where_x_prefix_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(&["where", "x", "is"])).is_some()
}

pub(crate) fn parse_where_x_fixed_plus_reference_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXFixedPlusReferenceSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_where_x_fixed_plus_reference_lexed,
        "where-x-fixed-plus-reference",
    )
    .ok()
}

pub(crate) fn parse_where_x_player_metric_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXPlayerMetric> {
    primitives::parse_all(
        tokens,
        parse_where_x_player_metric_lexed,
        "where-x-player-metric",
    )
    .ok()
}

pub(crate) fn parse_where_x_aggregate_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXAggregateFilterSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_where_x_aggregate_filter_lexed,
        "where-x-aggregate-filter",
    )
    .ok()
}

pub(crate) fn parse_commander_battlefield_or_command_zone_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_commander_battlefield_or_command_zone_lexed,
        "commander-battlefield-or-command-zone",
    )
    .is_ok()
}

pub(crate) fn parse_where_x_differently_named_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(
        tokens,
        parse_where_x_differently_named_filter_lexed,
        "where-x-differently-named-filter",
    )
    .ok()
}

pub(crate) fn parse_where_x_different_powers_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(
        tokens,
        parse_where_x_different_powers_filter_lexed,
        "where-x-different-powers-filter",
    )
    .ok()
}

pub(crate) fn parse_where_x_greatest_number_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(
        tokens,
        parse_where_x_greatest_number_filter_lexed,
        "where-x-greatest-number-filter",
    )
    .ok()
}

pub(crate) fn parse_where_x_number_of_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXNumberOfFilterSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_where_x_number_of_filter_lexed,
        "where-x-number-of-filter",
    )
    .ok()
}

pub(crate) fn parse_where_x_fixed_plus_number_of_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXFixedPlusNumberOfFilterSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_where_x_fixed_plus_number_of_filter_lexed,
        "where-x-fixed-plus-number-of-filter",
    )
    .ok()
}

pub(crate) fn parse_where_x_number_of_filter_offset_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXNumberOfFilterOffsetSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_where_x_number_of_filter_offset_lexed,
        "where-x-number-of-filter-offset",
    )
    .ok()
}

pub(crate) fn parse_where_x_source_stat_tokens(
    tokens: &[OwnedLexToken],
) -> Option<WhereXSourceStatSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_where_x_source_stat_lexed,
        "where-x source statistic",
    )
    .ok()
}

fn parse_equal_to_value_body_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    primitives::phrase(&["equal", "to"]).parse_next(input)?;
    take_nonempty_sentence_body(input)
}

fn parse_equal_to_mana_spent_to_cast_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "equal", "to", "the", "amount", "of", "mana", "spent", "to", "cast",
    ])
    .parse_next(input)?;
    alt((
        primitives::phrase(&["this", "spell"]),
        primitives::kw("spell").void(),
        primitives::kw("it").void(),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_equal_to_greatest_cards_drawn_this_turn_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<()> {
    primitives::phrase(&["equal", "to"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&[
        "greatest", "number", "of", "cards", "an", "opponent", "has", "drawn", "this", "turn",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_where_x_fixed_plus_reference_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXFixedPlusReferenceSpec<'a>> {
    parse_where_x_prefix(input)?;
    let fixed_tokens = take_until_phrase(input, &["plus"])?;
    primitives::kw("plus").parse_next(input)?;
    let reference_kind = parse_reference_value_kind(input)?;
    repeat_till(0.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(WhereXFixedPlusReferenceSpec {
        fixed_tokens: trim_lexed_commas(fixed_tokens),
        reference_kind,
    })
}

fn parse_reference_value_kind<'a>(input: &mut LexStream<'a>) -> WResult<EtbReferenceValueKind> {
    alt((
        alt((
            primitives::phrase(&["the", "sacrificed", "creature", "power"]),
            primitives::phrase(&["the", "sacrificed", "creatures", "power"]),
            primitives::phrase(&["sacrificed", "creature", "power"]),
            primitives::phrase(&["sacrificed", "creatures", "power"]),
        ))
        .value(EtbReferenceValueKind::SacrificedCreaturePower),
        alt((
            primitives::phrase(&["the", "sacrificed", "creature", "toughness"]),
            primitives::phrase(&["the", "sacrificed", "creatures", "toughness"]),
            primitives::phrase(&["sacrificed", "creature", "toughness"]),
            primitives::phrase(&["sacrificed", "creatures", "toughness"]),
        ))
        .value(EtbReferenceValueKind::SacrificedCreatureToughness),
        parse_tagged_creature_mana_value_reference
            .value(EtbReferenceValueKind::TaggedCreatureManaValue),
    ))
    .parse_next(input)
}

fn parse_tagged_creature_mana_value_reference<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (
            opt(primitives::kw("the")),
            primitives::phrase(&["mana", "value", "of", "the"]),
            alt((primitives::kw("sacrificed"), primitives::kw("exiled"))),
            alt((
                primitives::kw("creature"),
                primitives::kw("creature's"),
                primitives::kw("creatures"),
            )),
        )
            .void(),
        (
            opt(primitives::kw("the")),
            alt((primitives::kw("sacrificed"), primitives::kw("exiled"))),
            alt((
                primitives::kw("creature"),
                primitives::kw("creature's"),
                primitives::kw("creatures"),
            )),
            primitives::phrase(&["mana", "value"]),
        )
            .void(),
    ))
    .parse_next(input)
}

fn parse_where_x_player_metric_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WhereXPlayerMetric> {
    parse_where_x_prefix(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    let metric = alt((
        (
            primitives::phrase(&["amount", "of", "life"]),
            alt((
                primitives::kw("you've"),
                primitives::kw("youve"),
                primitives::kw("you"),
            )),
            primitives::phrase(&["gained", "this", "turn"]),
        )
            .value(WhereXPlayerMetric::LifeGainedByYouThisTurn),
        (
            primitives::phrase(&["amount", "of", "life"]),
            alt((
                primitives::kw("you've"),
                primitives::kw("youve"),
                primitives::kw("you"),
            )),
            primitives::phrase(&["lost", "this", "turn"]),
        )
            .value(WhereXPlayerMetric::LifeLostByYouThisTurn),
        primitives::phrase(&[
            "total",
            "life",
            "lost",
            "by",
            "your",
            "opponents",
            "this",
            "turn",
        ])
        .value(WhereXPlayerMetric::LifeLostByOpponentsThisTurn),
        primitives::phrase(&[
            "number",
            "of",
            "opponents",
            "that",
            "were",
            "dealt",
            "combat",
            "damage",
            "this",
            "turn",
        ])
        .value(WhereXPlayerMetric::OpponentsDealtCombatDamageThisTurn),
        primitives::phrase(&[
            "total",
            "amount",
            "of",
            "noncombat",
            "damage",
            "dealt",
            "to",
            "your",
            "opponents",
            "this",
            "turn",
        ])
        .value(WhereXPlayerMetric::NoncombatDamageDealtToOpponentsThisTurn),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(metric)
}

fn parse_where_x_aggregate_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXAggregateFilterSpec<'a>> {
    parse_where_x_prefix(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    let aggregate = alt((
        primitives::kw("total").value(EtbAggregateKind::Total),
        primitives::kw("greatest").value(EtbAggregateKind::Greatest),
    ))
    .parse_next(input)?;
    let value_kind = alt((
        primitives::kw("power").value(EtbAggregateValueKind::Power),
        primitives::kw("toughness").value(EtbAggregateValueKind::Toughness),
        primitives::phrase(&["mana", "value"]).value(EtbAggregateValueKind::ManaValue),
    ))
    .parse_next(input)?;
    alt((primitives::kw("of"), primitives::kw("among"))).parse_next(input)?;
    let filter_tokens = take_nonempty_sentence_body(input)?;
    Ok(WhereXAggregateFilterSpec {
        aggregate,
        value_kind,
        filter_tokens,
    })
}

fn parse_commander_battlefield_or_command_zone_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    optional_articles(input)?;
    primitives::kw("commander").parse_next(input)?;
    optional_articles(input)?;
    primitives::phrase(&["you", "own", "on"]).parse_next(input)?;
    optional_articles(input)?;
    primitives::phrase(&["battlefield", "or", "in"]).parse_next(input)?;
    optional_articles(input)?;
    primitives::phrase(&["command", "zone"]).parse_next(input)?;
    optional_articles(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_where_x_differently_named_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    parse_where_x_prefix(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["number", "of", "differently", "named"]).parse_next(input)?;
    take_nonempty_sentence_body(input)
}

fn parse_where_x_different_powers_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    parse_where_x_prefix(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["number", "of", "different"]).parse_next(input)?;
    alt((primitives::kw("power"), primitives::kw("powers"))).parse_next(input)?;
    primitives::kw("among").parse_next(input)?;
    take_nonempty_sentence_body(input)
}

fn parse_where_x_greatest_number_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    parse_where_x_prefix(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["greatest", "number", "of"]).parse_next(input)?;
    take_nonempty_sentence_body(input)
}

fn parse_where_x_number_of_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXNumberOfFilterSpec<'a>> {
    parse_where_x_prefix(input)?;
    let multiplier = alt((
        primitives::phrase(&["the", "total", "number", "of"]).value(1),
        primitives::phrase(&["the", "number", "of"]).value(1),
        primitives::phrase(&["number", "of"]).value(1),
        primitives::phrase(&["twice", "the", "number", "of"]).value(2),
        primitives::phrase(&["twice", "number", "of"]).value(2),
        primitives::phrase(&["two", "times", "the", "number", "of"]).value(2),
        primitives::phrase(&["two", "times", "number", "of"]).value(2),
    ))
    .parse_next(input)?;
    let filter_tokens = take_nonempty_sentence_body(input)?;
    Ok(WhereXNumberOfFilterSpec {
        multiplier,
        filter_tokens,
    })
}

fn parse_where_x_fixed_plus_number_of_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXFixedPlusNumberOfFilterSpec<'a>> {
    parse_where_x_prefix(input)?;
    let fixed_tokens = take_until_phrase(input, &["plus"])?;
    primitives::kw("plus").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["number", "of"]).parse_next(input)?;
    let filter_tokens = take_nonempty_sentence_body(input)?;
    Ok(WhereXFixedPlusNumberOfFilterSpec {
        fixed_tokens: trim_lexed_commas(fixed_tokens),
        filter_tokens,
    })
}

fn parse_where_x_number_of_filter_offset_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXNumberOfFilterOffsetSpec<'a>> {
    parse_where_x_prefix(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["number", "of"]).parse_next(input)?;
    let filter_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((primitives::kw("plus"), primitives::kw("minus")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let operator = alt((
        primitives::kw("plus").value(EtbNumberOffsetOperator::Plus),
        primitives::kw("minus").value(EtbNumberOffsetOperator::Minus),
    ))
    .parse_next(input)?;
    let offset_tokens = take_nonempty_sentence_body(input)?;
    Ok(WhereXNumberOfFilterOffsetSpec {
        filter_tokens: trim_lexed_commas(filter_tokens),
        operator,
        offset_tokens,
    })
}

fn parse_where_x_source_stat_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXSourceStatSpec<'a>> {
    parse_where_x_prefix(input)?;
    let parsed =
        alt((parse_tagged_mana_value_of_stat, parse_source_stat_suffix)).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(parsed)
}

fn parse_source_stat_suffix<'a>(input: &mut LexStream<'a>) -> WResult<WhereXSourceStatSpec<'a>> {
    let reference_tokens = repeat_till(
        1..,
        any.void(),
        peek((parse_source_stat_tail, primitives::sentence_end())),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let (kind, as_this_ability_resolves) = parse_source_stat_tail(input)?;
    let fallback = parse_source_stat_fallback(reference_tokens, kind);
    Ok(WhereXSourceStatSpec {
        kind,
        reference_tokens,
        fallback,
        as_this_ability_resolves,
    })
}

fn parse_source_stat_tail(input: &mut LexStream<'_>) -> WResult<(EtbSourceStatKind, bool)> {
    let kind = parse_source_stat_kind(input)?;
    let as_this_ability_resolves = opt((
        opt(primitives::comma()),
        primitives::phrase(&["as", "this", "ability", "resolves"]),
    )
        .void())
    .parse_next(input)?
    .is_some();
    Ok((kind, as_this_ability_resolves))
}

fn parse_tagged_mana_value_of_stat<'a>(
    input: &mut LexStream<'a>,
) -> WResult<WhereXSourceStatSpec<'a>> {
    const SUBJECTS: &[&[&str]] = &[
        &["the", "amassed", "army"],
        &["the", "amassed", "army's"],
        &["the", "amassed", "armys"],
        &["the", "army", "you", "amassed"],
    ];
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["mana", "value", "of"]).parse_next(input)?;
    let (_, reference_tokens) = primitives::any_phrase(SUBJECTS)
        .with_taken()
        .parse_next(input)?;
    Ok(WhereXSourceStatSpec {
        kind: EtbSourceStatKind::ManaValue,
        reference_tokens,
        fallback: Some(EtbSourceStatFallback::TaggedObject),
        as_this_ability_resolves: false,
    })
}

fn parse_source_stat_fallback(
    tokens: &[OwnedLexToken],
    kind: EtbSourceStatKind,
) -> Option<EtbSourceStatFallback> {
    if primitives::parse_all(
        tokens,
        parse_direct_source_subject,
        "direct ETB source subject",
    )
    .is_ok()
    {
        return Some(EtbSourceStatFallback::Source);
    }
    if kind == EtbSourceStatKind::ManaValue
        && primitives::parse_all(
            tokens,
            parse_triggering_spell_subject,
            "triggering spell ETB subject",
        )
        .is_ok()
    {
        return Some(EtbSourceStatFallback::TriggeringSpell);
    }
    let tagged = match kind {
        EtbSourceStatKind::Power | EtbSourceStatKind::Toughness => primitives::parse_all(
            tokens,
            parse_tagged_power_or_toughness_subject,
            "tagged ETB stat subject",
        )
        .is_ok(),
        EtbSourceStatKind::ManaValue => primitives::parse_all(
            tokens,
            parse_tagged_mana_value_subject,
            "tagged ETB mana-value subject",
        )
        .is_ok(),
    };
    tagged.then_some(EtbSourceStatFallback::TaggedObject)
}

fn parse_triggering_spell_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    const SUBJECTS: &[&[&str]] = &[
        &["that", "spell"],
        &["that", "spell's"],
        &["that", "spells"],
    ];
    primitives::any_phrase(SUBJECTS).void().parse_next(input)
}

fn parse_tagged_power_or_toughness_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    const SUBJECTS: &[&[&str]] = &[
        &["that", "creature"],
        &["that", "creature's"],
        &["that", "creatures"],
        &["that", "object"],
        &["that", "object's"],
        &["that", "objects"],
        &["the", "sacrificed", "creature"],
        &["the", "sacrificed", "creature's"],
        &["the", "sacrificed", "creatures"],
        &["sacrificed", "creature"],
        &["sacrificed", "creature's"],
        &["sacrificed", "creatures"],
        &["the", "amassed", "army"],
        &["the", "amassed", "army's"],
        &["the", "amassed", "armys"],
        &["amassed", "army"],
        &["amassed", "army's"],
        &["amassed", "armys"],
        &["the", "army", "you", "amassed"],
        &["army", "you", "amassed"],
    ];
    primitives::any_phrase(SUBJECTS).void().parse_next(input)
}

fn parse_tagged_mana_value_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    const SUBJECTS: &[&[&str]] = &[
        &["that", "card"],
        &["that", "card's"],
        &["that", "cards"],
        &["that", "creature"],
        &["that", "creature's"],
        &["that", "creatures"],
        &["the", "sacrificed", "creature"],
        &["the", "sacrificed", "creature's"],
        &["the", "sacrificed", "creatures"],
        &["sacrificed", "creature"],
        &["sacrificed", "creature's"],
        &["sacrificed", "creatures"],
        &["the", "amassed", "army"],
        &["the", "amassed", "army's"],
        &["the", "amassed", "armys"],
        &["amassed", "army"],
        &["amassed", "army's"],
        &["amassed", "armys"],
    ];
    primitives::any_phrase(SUBJECTS).void().parse_next(input)
}

fn parse_direct_source_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    const SUBJECTS: &[&[&str]] = &[
        &["this", "creature's"],
        &["this", "creatures"],
        &["this", "creature"],
        &["thiss", "creature's"],
        &["thiss", "creatures"],
        &["thiss", "creature"],
        &["this"],
        &["thiss"],
        &["its"],
    ];
    primitives::any_phrase(SUBJECTS).void().parse_next(input)
}

fn parse_source_stat_kind<'a>(input: &mut LexStream<'a>) -> WResult<EtbSourceStatKind> {
    alt((
        primitives::phrase(&["mana", "value"]).value(EtbSourceStatKind::ManaValue),
        primitives::kw("power").value(EtbSourceStatKind::Power),
        primitives::kw("toughness").value(EtbSourceStatKind::Toughness),
    ))
    .parse_next(input)
}

fn parse_where_x_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["where", "x", "is"]).parse_next(input)
}

fn optional_articles<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            primitives::kw("a"),
            primitives::kw("an"),
            primitives::kw("the"),
        )),
    )
    .parse_next(input)
}

fn take_until_phrase<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(primitives::phrase(phrase)))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn take_nonempty_sentence_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let body = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let body = trim_lexed_commas(body);
    if body.is_empty() {
        return Err(primitives::backtrack_err(
            "ETB value body",
            "non-empty value body",
        ));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_equal_to_value_shapes() {
        let tokens = lex_line("equal to the amount of mana spent to cast this spell.", 0).unwrap();
        assert!(parse_equal_to_mana_spent_to_cast_tokens(&tokens));

        let tokens = lex_line(
            "equal to the greatest number of cards an opponent has drawn this turn.",
            0,
        )
        .unwrap();
        assert!(parse_equal_to_greatest_cards_drawn_this_turn_tokens(
            &tokens
        ));
    }

    #[test]
    fn parses_where_x_metric_and_aggregate_shapes() {
        let tokens = lex_line("where X is the amount of life you've gained this turn.", 0).unwrap();
        assert_eq!(
            parse_where_x_player_metric_tokens(&tokens),
            Some(WhereXPlayerMetric::LifeGainedByYouThisTurn)
        );

        let tokens = lex_line("where X is the amount of life you lost this turn.", 0).unwrap();
        assert_eq!(
            parse_where_x_player_metric_tokens(&tokens),
            Some(WhereXPlayerMetric::LifeLostByYouThisTurn)
        );

        let tokens = lex_line(
            "where X is the greatest mana value among creatures you control.",
            0,
        )
        .unwrap();
        let parsed = parse_where_x_aggregate_filter_tokens(&tokens).unwrap();
        assert_eq!(parsed.aggregate, EtbAggregateKind::Greatest);
        assert_eq!(parsed.value_kind, EtbAggregateValueKind::ManaValue);
        assert_eq!(
            render_token_slice(parsed.filter_tokens),
            "creatures you control"
        );
    }

    #[test]
    fn parses_where_x_count_variants() {
        let tokens = lex_line("where X is twice the number of creatures you control.", 0).unwrap();
        let parsed = parse_where_x_number_of_filter_tokens(&tokens).unwrap();
        assert_eq!(parsed.multiplier, 2);
        assert_eq!(
            render_token_slice(parsed.filter_tokens),
            "creatures you control"
        );

        let tokens = lex_line("where X is the number of cards in your hand minus two.", 0).unwrap();
        let parsed = parse_where_x_number_of_filter_offset_tokens(&tokens).unwrap();
        assert_eq!(parsed.operator, EtbNumberOffsetOperator::Minus);
        assert_eq!(
            render_token_slice(parsed.filter_tokens),
            "cards in your hand"
        );
        assert_eq!(render_token_slice(parsed.offset_tokens), "two");
    }

    #[test]
    fn parses_where_x_reference_and_filter_shapes() {
        let tokens = lex_line(
            "where X is two plus the sacrificed creature's mana value.",
            0,
        )
        .unwrap();
        let parsed = parse_where_x_fixed_plus_reference_tokens(&tokens).unwrap();
        assert_eq!(
            parsed.reference_kind,
            EtbReferenceValueKind::TaggedCreatureManaValue
        );
        assert_eq!(render_token_slice(parsed.fixed_tokens), "two");

        let tokens = lex_line(
            "where X is the number of differently named creatures you control.",
            0,
        )
        .unwrap();
        let filter = parse_where_x_differently_named_filter_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(filter), "creatures you control");
    }

    #[test]
    fn parses_where_x_source_stat_shapes() {
        let tokens = lex_line("where X is this creature's power.", 0).unwrap();
        assert_eq!(
            parse_where_x_source_stat_tokens(&tokens).map(|parsed| (parsed.kind, parsed.fallback)),
            Some((
                EtbSourceStatKind::Power,
                Some(EtbSourceStatFallback::Source),
            ))
        );

        let tokens = lex_line(
            "where X is this creature's power as this ability resolves.",
            0,
        )
        .unwrap();
        let parsed =
            parse_where_x_source_stat_tokens(&tokens).expect("resolution-time source stat");
        assert_eq!(parsed.kind, EtbSourceStatKind::Power);
        assert_eq!(parsed.fallback, Some(EtbSourceStatFallback::Source));
        assert!(parsed.as_this_ability_resolves);

        let tokens = lex_line("where X is that spell's mana value.", 0).unwrap();
        assert_eq!(
            parse_where_x_source_stat_tokens(&tokens).map(|parsed| (parsed.kind, parsed.fallback)),
            Some((
                EtbSourceStatKind::ManaValue,
                Some(EtbSourceStatFallback::TriggeringSpell),
            ))
        );

        let tokens = lex_line("where X is the sacrificed creature's toughness.", 0).unwrap();
        assert_eq!(
            parse_where_x_source_stat_tokens(&tokens).map(|parsed| (parsed.kind, parsed.fallback)),
            Some((
                EtbSourceStatKind::Toughness,
                Some(EtbSourceStatFallback::TaggedObject),
            ))
        );

        let tokens = lex_line("where X is that creature's mana value.", 0).unwrap();
        assert_eq!(
            parse_where_x_source_stat_tokens(&tokens).map(|parsed| (parsed.kind, parsed.fallback)),
            Some((
                EtbSourceStatKind::ManaValue,
                Some(EtbSourceStatFallback::TaggedObject),
            ))
        );
    }
}
