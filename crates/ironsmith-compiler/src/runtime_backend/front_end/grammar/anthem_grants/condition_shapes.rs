use winnow::combinator::{alt, eof, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::color::Color;
use crate::effect::{Comparison, ValueComparisonOperator};
use crate::object::CounterType;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::{filters, leaf, primitives};
use super::condition_quantities::parse_condition_quantity_prefix;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FixedStaticConditionKind {
    SourceIsEquipped,
    SourceSpellWasKicked,
    OpponentLostLifeThisTurn,
    YouDidNotCastSpellThisTurn,
    YouCastSpellThisTurn,
    NoCardsInYourLibrary,
    SourceIsOnBattlefield,
    SourceDevouredCreature,
    SourceIsSoulbondPaired,
    SourceAttackedThisTurn,
    YouAttackedThisTurn,
    SourceEnteredThisTurn,
    YourTurn,
    SourcePowerEven,
    SourcePowerOdd,
    NotYourTurn,
    YourLifeAtMostHalfStarting,
    YouCommittedCrimeThisTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DevotionPlayerKind {
    You,
    IteratedPlayer,
    Opponent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DevotionConditionShape {
    pub(crate) player: DevotionPlayerKind,
    pub(crate) colors: Vec<Color>,
    pub(crate) operator: ValueComparisonOperator,
    pub(crate) amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DevotionConditionError {
    UnsupportedPlayer,
    UnsupportedColor(String),
    MissingColor,
    UnsupportedComparison,
    MissingValue,
    UnsupportedValue(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockingSourceConditionShape {
    pub(crate) comparison: Comparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConjoinedConditionSplit<'a> {
    pub(crate) left_tokens: &'a [OwnedLexToken],
    pub(crate) right_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExistentialConditionTail<'a> {
    CardTypesInYourGraveyard {
        threshold: u32,
    },
    CardsInYourGraveyard,
    DistinctCounterTypesAmong {
        filter_tokens: &'a [OwnedLexToken],
    },
    CountersAmong {
        filter_tokens: &'a [OwnedLexToken],
        counter_type: CounterType,
    },
    SourceInGraveyard,
    Generic {
        filter_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExistentialConditionShape<'a> {
    pub(crate) comparison: Comparison,
    pub(crate) tail: ExistentialConditionTail<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EnteredCountConditionShape<'a> {
    pub(crate) comparison: Comparison,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) begins_with_other: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceCounterConditionShape {
    pub(crate) comparison: Comparison,
    pub(crate) counter_type: Option<CounterType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceCounterConditionError {
    MissingQuantity,
    MissingCounterPhrase,
    UnsupportedTail,
}

pub(crate) fn parse_fixed_static_condition_kind(
    tokens: &[OwnedLexToken],
) -> Option<FixedStaticConditionKind> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_fixed_static_condition_lexed,
        "fixed anthem condition",
    )
    .ok()
}

pub(crate) fn parse_life_total_or_less_condition(tokens: &[OwnedLexToken]) -> Option<u32> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, tail) = primitives::parse_prefix(tokens, primitives::phrase(&["you", "have"]))?;
    let quantity = parse_condition_quantity_prefix(tail, false, false)?;
    if !parse_complete_phrase(quantity.rest, &["life"]) {
        return None;
    }
    comparison_to_at_most_threshold(quantity.comparison)
}

pub(crate) fn parse_devotion_condition_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<DevotionConditionShape>, DevotionConditionError> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let Some((devotion_token, _, after_devotion)) =
        primitives::find_prefix(tokens, || primitives::kw("devotion").void())
    else {
        return Ok(None);
    };
    let Some((_, _, after_to)) =
        primitives::find_prefix(after_devotion, || primitives::kw("to").void())
    else {
        return Ok(None);
    };
    let Some((is_relative, _, after_is)) =
        primitives::find_prefix(after_to, || primitives::kw("is").void())
    else {
        return Ok(None);
    };

    let player = primitives::parse_all(
        &tokens[..devotion_token],
        parse_devotion_player_prefix,
        "devotion player",
    )
    .map_err(|_| DevotionConditionError::UnsupportedPlayer)?;

    let colors = parse_devotion_colors(&after_to[..is_relative])?;

    let Some((operator, amount_tokens)) =
        primitives::parse_prefix(after_is, parse_devotion_comparison)
    else {
        return Err(DevotionConditionError::UnsupportedComparison);
    };
    if amount_tokens.is_empty() {
        return Err(DevotionConditionError::MissingValue);
    }
    let Some((amount, _)) =
        primitives::parse_prefix(amount_tokens, leaf::parse_leaf_number_prefix_lexed)
    else {
        let word = first_parser_word(amount_tokens).unwrap_or_default();
        return Err(DevotionConditionError::UnsupportedValue(word.to_string()));
    };

    Ok(Some(DevotionConditionShape {
        player,
        colors,
        operator,
        amount,
    }))
}

pub(crate) fn parse_x_value_at_least_condition(tokens: &[OwnedLexToken]) -> Option<u32> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, tail) = primitives::parse_prefix(tokens, primitives::phrase(&["x", "is"]))?;
    let quantity = parse_condition_quantity_prefix(tail, false, true)?;
    if !quantity.rest.is_empty() {
        return None;
    }
    comparison_to_at_least_threshold(quantity.comparison, false)
}

pub(crate) fn parse_blocking_source_condition(
    tokens: &[OwnedLexToken],
) -> Option<BlockingSourceConditionShape> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let quantity = parse_condition_quantity_prefix(tokens, true, true)?;
    parse_complete_any_phrase(
        quantity.rest,
        &[
            &["creature", "is", "blocking", "it"],
            &["creature", "is", "blocking", "this", "creature"],
            &["creatures", "are", "blocking", "it"],
            &["creatures", "are", "blocking", "this", "creature"],
        ],
    )
    .then_some(BlockingSourceConditionShape {
        comparison: quantity.comparison,
    })
}

pub(crate) fn parse_source_in_graveyard_condition(tokens: &[OwnedLexToken]) -> bool {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let Some((relation_token, _, tail)) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("is"), primitives::kw("are"))).void()
    }) else {
        return false;
    };
    relation_token > 0
        && is_source_condition_subject(&tokens[..relation_token])
        && parse_complete_any_phrase(tail, &[&["in", "your", "graveyard"], &["in", "graveyard"]])
}

pub(crate) fn parse_conjoined_condition_splits(
    tokens: &[OwnedLexToken],
) -> Vec<ConjoinedConditionSplit<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let mut splits = Vec::new();
    let mut search_start = 0usize;
    while search_start < tokens.len() {
        let Some((relative, _, _)) =
            primitives::find_prefix(&tokens[search_start..], || primitives::kw("and").void())
        else {
            break;
        };
        let and_token = search_start + relative;
        let left_tokens = trim_lexed_commas(&tokens[..and_token]);
        let right_tokens = trim_lexed_commas(&tokens[and_token + 1..]);
        if !left_tokens.is_empty() && !right_tokens.is_empty() {
            splits.push(ConjoinedConditionSplit {
                left_tokens,
                right_tokens,
            });
        }
        search_start = and_token + 1;
    }
    splits
}

pub(crate) fn parse_existential_condition_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<ExistentialConditionShape<'_>>, ()> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let Some((singular, quantified)) = primitives::parse_prefix(tokens, parse_existential_head)
    else {
        return Ok(None);
    };
    let quantity = parse_condition_quantity_prefix(quantified, singular, true).ok_or(())?;

    if quantity.consumed > 0
        && let Some(threshold) = comparison_to_at_least_threshold(quantity.comparison, true)
        && is_card_types_in_graveyard_metric(quantity.rest)
    {
        return Ok(Some(ExistentialConditionShape {
            comparison: quantity.comparison,
            tail: ExistentialConditionTail::CardTypesInYourGraveyard { threshold },
        }));
    }

    let filter_tokens = strip_leading_card_noun(quantity.rest);
    let tail = if parse_complete_phrase(filter_tokens, &["in", "your", "graveyard"]) {
        ExistentialConditionTail::CardsInYourGraveyard
    } else if let Some((_, among_filter)) = primitives::parse_prefix(
        filter_tokens,
        primitives::phrase(&["different", "kinds", "of", "counters", "among"]),
    ) {
        ExistentialConditionTail::DistinctCounterTypesAmong {
            filter_tokens: among_filter,
        }
    } else if let Some((counter_type, among_filter)) = parse_counter_among_tail(filter_tokens) {
        ExistentialConditionTail::CountersAmong {
            filter_tokens: among_filter,
            counter_type,
        }
    } else if parse_source_in_graveyard_condition(filter_tokens) {
        ExistentialConditionTail::SourceInGraveyard
    } else {
        ExistentialConditionTail::Generic { filter_tokens }
    };

    Ok(Some(ExistentialConditionShape {
        comparison: quantity.comparison,
        tail,
    }))
}

pub(crate) fn parse_entered_count_condition(
    tokens: &[OwnedLexToken],
) -> Option<EnteredCountConditionShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    primitives::find_prefix(tokens, || primitives::kw("entered").void())?;
    let quantity = parse_condition_quantity_prefix(tokens, true, true)?;
    let begins_with_other = primitives::parse_prefix(
        quantity.rest,
        alt((primitives::kw("other"), primitives::kw("another"))).void(),
    )
    .is_some();
    Some(EnteredCountConditionShape {
        comparison: quantity.comparison,
        filter_tokens: quantity.rest,
        begins_with_other,
    })
}

pub(crate) fn parse_source_counter_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<SourceCounterConditionShape>, SourceCounterConditionError> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let Some((relation_token, _, quantified)) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("has"), primitives::kw("have"))).void()
    }) else {
        return Ok(None);
    };
    if relation_token == 0 || !is_source_condition_subject(&tokens[..relation_token]) {
        return Ok(None);
    }

    let quantity = parse_condition_quantity_prefix(quantified, true, true)
        .ok_or(SourceCounterConditionError::MissingQuantity)?;
    let Some((counter_token, _, tail)) = primitives::find_prefix(quantity.rest, || {
        alt((primitives::kw("counter"), primitives::kw("counters"))).void()
    }) else {
        return Err(SourceCounterConditionError::MissingCounterPhrase);
    };

    let counter_type = if counter_token > 0 {
        let descriptor = TokenWordView::new(&quantity.rest[..counter_token]);
        descriptor
            .get(descriptor.len().saturating_sub(1))
            .and_then(filters::parse_counter_type_word)
    } else {
        None
    };
    if primitives::parse_prefix(
        tail,
        primitives::any_phrase(&[
            &["on", "it"],
            &["on", "this"],
            &["on", "him"],
            &["on", "her"],
        ]),
    )
    .is_none()
    {
        return Err(SourceCounterConditionError::UnsupportedTail);
    }

    Ok(Some(SourceCounterConditionShape {
        comparison: quantity.comparison,
        counter_type,
    }))
}

fn parse_fixed_static_condition_lexed(
    input: &mut LexStream<'_>,
) -> WResult<FixedStaticConditionKind> {
    alt((
        parse_fixed_group_one,
        parse_fixed_group_two,
        parse_fixed_group_three,
    ))
    .parse_next(input)
}

fn parse_fixed_group_one(input: &mut LexStream<'_>) -> WResult<FixedStaticConditionKind> {
    alt((
        primitives::any_phrase(&[
            &["this", "equipment", "is", "attached", "to", "a", "creature"],
            &["this", "equipment", "attached", "to", "a", "creature"],
        ])
        .value(FixedStaticConditionKind::SourceIsEquipped),
        primitives::any_phrase(&[
            &["this", "spell", "was", "kicked"],
            &["it", "was", "kicked"],
        ])
        .value(FixedStaticConditionKind::SourceSpellWasKicked),
        primitives::any_phrase(&[
            &["an", "opponent", "lost", "life", "this", "turn"],
            &[
                "one",
                "or",
                "more",
                "opponents",
                "lost",
                "life",
                "this",
                "turn",
            ],
        ])
        .value(FixedStaticConditionKind::OpponentLostLifeThisTurn),
        parse_you_did_not_cast_spell.value(FixedStaticConditionKind::YouDidNotCastSpellThisTurn),
        parse_you_cast_spell.value(FixedStaticConditionKind::YouCastSpellThisTurn),
        primitives::any_phrase(&[
            &["there", "are", "no", "cards", "in", "your", "library"],
            &["your", "library", "has", "no", "cards", "in", "it"],
        ])
        .value(FixedStaticConditionKind::NoCardsInYourLibrary),
        primitives::any_phrase(&[
            &["this", "creature", "is", "on", "the", "battlefield"],
            &["this", "permanent", "is", "on", "the", "battlefield"],
            &["this", "is", "on", "the", "battlefield"],
            &["it", "is", "on", "the", "battlefield"],
        ])
        .value(FixedStaticConditionKind::SourceIsOnBattlefield),
    ))
    .parse_next(input)
}

fn parse_fixed_group_two(input: &mut LexStream<'_>) -> WResult<FixedStaticConditionKind> {
    alt((
        primitives::any_phrase(&[
            &["it", "devoured", "a", "creature"],
            &["it", "devoured", "one", "or", "more", "creatures"],
            &["this", "creature", "devoured", "a", "creature"],
            &[
                "this",
                "creature",
                "devoured",
                "one",
                "or",
                "more",
                "creatures",
            ],
        ])
        .value(FixedStaticConditionKind::SourceDevouredCreature),
        primitives::any_phrase(&[
            &["this", "is", "paired", "with", "another", "creature"],
            &[
                "this", "creature", "is", "paired", "with", "another", "creature",
            ],
            &["it", "is", "paired", "with", "another", "creature"],
        ])
        .value(FixedStaticConditionKind::SourceIsSoulbondPaired),
        primitives::any_phrase(&[
            &["it", "attacked", "this", "turn"],
            &["this", "creature", "attacked", "this", "turn"],
            &["this", "permanent", "attacked", "this", "turn"],
            &["that", "creature", "attacked", "this", "turn"],
        ])
        .value(FixedStaticConditionKind::SourceAttackedThisTurn),
        primitives::phrase(&["you", "attacked", "this", "turn"])
            .value(FixedStaticConditionKind::YouAttackedThisTurn),
        primitives::any_phrase(&[
            &["it", "entered", "this", "turn"],
            &["this", "creature", "entered", "this", "turn"],
            &["this", "permanent", "entered", "this", "turn"],
        ])
        .value(FixedStaticConditionKind::SourceEnteredThisTurn),
        parse_your_turn.value(FixedStaticConditionKind::YourTurn),
    ))
    .parse_next(input)
}

fn parse_fixed_group_three(input: &mut LexStream<'_>) -> WResult<FixedStaticConditionKind> {
    alt((
        parse_source_power_parity
            .verify(|even| *even)
            .value(FixedStaticConditionKind::SourcePowerEven),
        parse_source_power_parity
            .verify(|even| !*even)
            .value(FixedStaticConditionKind::SourcePowerOdd),
        parse_not_your_turn.value(FixedStaticConditionKind::NotYourTurn),
        primitives::phrase(&[
            "your", "life", "total", "is", "less", "than", "or", "equal", "to", "half", "your",
            "starting", "life", "total",
        ])
        .value(FixedStaticConditionKind::YourLifeAtMostHalfStarting),
        parse_you_committed_crime.value(FixedStaticConditionKind::YouCommittedCrimeThisTurn),
    ))
    .parse_next(input)
}

fn parse_you_did_not_cast_spell(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        (
            primitives::kw("you"),
            alt((primitives::kw("haven't"), primitives::kw("havent"))),
            primitives::phrase(&["cast", "a", "spell", "this", "turn"]),
        )
            .void(),
        primitives::phrase(&["you", "have", "not", "cast", "a", "spell", "this", "turn"]),
        (
            primitives::kw("you"),
            alt((primitives::kw("didn't"), primitives::kw("didnt"))),
            primitives::phrase(&["cast", "a", "spell", "this", "turn"]),
        )
            .void(),
        primitives::phrase(&["you", "did", "not", "cast", "a", "spell", "this", "turn"]),
    ))
    .void()
    .parse_next(input)
}

fn parse_you_cast_spell(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        (
            alt((primitives::kw("you've"), primitives::kw("youve"))),
            primitives::phrase(&["cast", "a", "spell", "this", "turn"]),
        )
            .void(),
        primitives::phrase(&["you", "ve", "cast", "a", "spell", "this", "turn"]),
        primitives::phrase(&["you", "have", "cast", "a", "spell", "this", "turn"]),
        primitives::phrase(&["you", "cast", "a", "spell", "this", "turn"]),
    ))
    .void()
    .parse_next(input)
}

fn parse_your_turn(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["it", "is", "your", "turn"]),
        (
            alt((primitives::kw("it's"), primitives::kw("its"))),
            primitives::phrase(&["your", "turn"]),
        )
            .void(),
    ))
    .parse_next(input)
}

fn parse_not_your_turn(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["it", "is", "not", "your", "turn"]),
        (
            alt((primitives::kw("it's"), primitives::kw("its"))),
            primitives::phrase(&["not", "your", "turn"]),
        )
            .void(),
    ))
    .parse_next(input)
}

fn parse_source_power_parity(input: &mut LexStream<'_>) -> WResult<bool> {
    alt((
        primitives::kw("this's"),
        primitives::kw("thiss"),
        primitives::kw("this"),
    ))
    .parse_next(input)?;
    primitives::phrase(&["power", "is"]).parse_next(input)?;
    alt((
        primitives::kw("even").value(true),
        primitives::kw("odd").value(false),
    ))
    .parse_next(input)
}

fn parse_you_committed_crime(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        (
            alt((primitives::kw("you've"), primitives::kw("youve"))),
            primitives::phrase(&["committed", "a", "crime", "this", "turn"]),
        )
            .void(),
        primitives::phrase(&["you", "ve", "committed", "a", "crime", "this", "turn"]),
        primitives::phrase(&["you", "have", "committed", "a", "crime", "this", "turn"]),
    ))
    .void()
    .parse_next(input)
}

fn parse_devotion_player_prefix(input: &mut LexStream<'_>) -> WResult<DevotionPlayerKind> {
    let (_, player): ((), DevotionPlayerKind) =
        repeat_till(0.., any.void(), parse_devotion_player).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(player)
}

fn parse_devotion_player(input: &mut LexStream<'_>) -> WResult<DevotionPlayerKind> {
    alt((
        primitives::kw("your").value(DevotionPlayerKind::You),
        primitives::kw("their").value(DevotionPlayerKind::IteratedPlayer),
        alt((
            primitives::kw("opponent"),
            primitives::kw("opponents"),
            primitives::kw("opponent's"),
            primitives::kw("opponents'"),
        ))
        .value(DevotionPlayerKind::Opponent),
    ))
    .parse_next(input)
}

fn parse_devotion_colors(tokens: &[OwnedLexToken]) -> Result<Vec<Color>, DevotionConditionError> {
    let mut colors = Vec::new();
    let mut input = LexStream::new(tokens);
    while !input.is_empty() {
        if alt((primitives::kw("and").void(), primitives::comma().void()))
            .parse_next(&mut input)
            .is_ok()
        {
            continue;
        }
        let mut probe = input.clone();
        let Ok(word) = primitives::word_parser_text.parse_next(&mut probe) else {
            return Err(DevotionConditionError::UnsupportedColor(String::new()));
        };
        let Some(color) = Color::from_name(word) else {
            return Err(DevotionConditionError::UnsupportedColor(word.to_string()));
        };
        colors.push(color);
        input = probe;
    }
    if colors.is_empty() {
        return Err(DevotionConditionError::MissingColor);
    }
    Ok(colors)
}

fn parse_devotion_comparison(input: &mut LexStream<'_>) -> WResult<ValueComparisonOperator> {
    alt((
        primitives::phrase(&["less", "than", "or", "equal", "to"])
            .value(ValueComparisonOperator::LessThanOrEqual),
        primitives::phrase(&["less", "than"]).value(ValueComparisonOperator::LessThan),
        primitives::phrase(&["greater", "than", "or", "equal", "to"])
            .value(ValueComparisonOperator::GreaterThanOrEqual),
        primitives::phrase(&["greater", "than"]).value(ValueComparisonOperator::GreaterThan),
        primitives::phrase(&["equal", "to"]).value(ValueComparisonOperator::Equal),
        primitives::phrase(&["not", "equal", "to"]).value(ValueComparisonOperator::NotEqual),
    ))
    .parse_next(input)
}

fn parse_existential_head(input: &mut LexStream<'_>) -> WResult<bool> {
    primitives::kw("there").parse_next(input)?;
    alt((
        primitives::kw("is").value(true),
        primitives::kw("are").value(false),
    ))
    .parse_next(input)
}

fn is_card_types_in_graveyard_metric(tokens: &[OwnedLexToken]) -> bool {
    let tokens = if primitives::parse_prefix(
        tokens,
        alt((primitives::kw("card"), primitives::kw("cards"))).void(),
    )
    .is_some_and(|(_, rest)| {
        primitives::parse_prefix(
            rest,
            alt((primitives::kw("type"), primitives::kw("types"))).void(),
        )
        .is_none()
    }) {
        primitives::parse_prefix(
            tokens,
            alt((primitives::kw("card"), primitives::kw("cards"))).void(),
        )
        .map(|(_, rest)| rest)
        .unwrap_or(tokens)
    } else {
        tokens
    };
    parse_complete_any_phrase(
        tokens,
        &[
            &["card", "type", "among", "cards", "in", "your", "graveyard"],
            &["card", "types", "among", "cards", "in", "your", "graveyard"],
        ],
    )
}

fn strip_leading_card_noun(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(
        tokens,
        alt((primitives::kw("card"), primitives::kw("cards"))).void(),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens)
}

fn parse_counter_among_tail(tokens: &[OwnedLexToken]) -> Option<(CounterType, &[OwnedLexToken])> {
    let (counter_token, _, filter_tokens) = primitives::find_prefix(tokens, || {
        (
            alt((primitives::kw("counter"), primitives::kw("counters"))),
            primitives::kw("among"),
        )
            .void()
    })?;
    if counter_token == 0 {
        return None;
    }
    let descriptor = TokenWordView::new(&tokens[..counter_token]);
    let word = descriptor.get(descriptor.len().checked_sub(1)?)?;
    Some((filters::parse_counter_type_word(word)?, filter_tokens))
}

fn is_source_condition_subject(tokens: &[OwnedLexToken]) -> bool {
    if parse_complete_any_phrase(tokens, &[&["it"], &["its"]]) {
        return true;
    }
    let view = TokenWordView::new(tokens);
    crate::runtime_backend::util::is_source_reference_words(&view.word_refs())
}

fn first_parser_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    primitives::find_prefix(tokens, || primitives::word_parser_text).map(|(_, word, _)| word)
}

fn comparison_to_at_least_threshold(comparison: Comparison, allow_equal: bool) -> Option<u32> {
    match comparison {
        Comparison::GreaterThanOrEqual(value) if value >= 0 => Some(value as u32),
        Comparison::GreaterThan(value) if value >= -1 => Some((value + 1) as u32),
        Comparison::Equal(value) if allow_equal && value >= 0 => Some(value as u32),
        _ => None,
    }
}

fn comparison_to_at_most_threshold(comparison: Comparison) -> Option<u32> {
    match comparison {
        Comparison::LessThanOrEqual(value) if value >= 0 => Some(value as u32),
        Comparison::LessThan(value) if value > 0 => Some((value - 1) as u32),
        _ => None,
    }
}

fn parse_complete_phrase(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    primitives::parse_all(tokens, primitives::phrase(words), "anthem condition phrase").is_ok()
}

fn parse_complete_any_phrase(
    tokens: &[OwnedLexToken],
    phrases: &[&'static [&'static str]],
) -> bool {
    primitives::parse_all(
        tokens,
        primitives::any_phrase(phrases),
        "anthem condition phrase",
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn classifies_fixed_condition_surfaces() {
        let not_cast = lex("You haven't cast a spell this turn.");
        assert_eq!(
            parse_fixed_static_condition_kind(&not_cast),
            Some(FixedStaticConditionKind::YouDidNotCastSpellThisTurn)
        );
        let not_your_turn = lex("It's not your turn.");
        assert_eq!(
            parse_fixed_static_condition_kind(&not_your_turn),
            Some(FixedStaticConditionKind::NotYourTurn)
        );
        let crime = lex("You've committed a crime this turn.");
        assert_eq!(
            parse_fixed_static_condition_kind(&crime),
            Some(FixedStaticConditionKind::YouCommittedCrimeThisTurn)
        );
        let kicked = lex("This spell was kicked.");
        assert_eq!(
            parse_fixed_static_condition_kind(&kicked),
            Some(FixedStaticConditionKind::SourceSpellWasKicked)
        );
    }

    #[test]
    fn parses_devotion_player_colors_and_comparison() {
        let tokens = lex("Your devotion to white and blue is greater than or equal to three.");
        let parsed = parse_devotion_condition_shape(&tokens)
            .expect("valid devotion")
            .expect("devotion shape");
        assert_eq!(parsed.player, DevotionPlayerKind::You);
        assert_eq!(parsed.colors, vec![Color::White, Color::Blue]);
        assert_eq!(parsed.operator, ValueComparisonOperator::GreaterThanOrEqual);
        assert_eq!(parsed.amount, 3);
    }

    #[test]
    fn parses_existential_counter_and_graveyard_shapes() {
        let graveyard = lex("There are four or more card types among cards in your graveyard.");
        let parsed = parse_existential_condition_shape(&graveyard)
            .expect("valid existential")
            .expect("existential shape");
        assert!(matches!(
            parsed.tail,
            ExistentialConditionTail::CardTypesInYourGraveyard { threshold: 4 }
        ));

        let counters = lex("There are three or more charge counters among artifacts you control.");
        let parsed = parse_existential_condition_shape(&counters)
            .expect("valid existential")
            .expect("existential shape");
        assert!(matches!(
            parsed.tail,
            ExistentialConditionTail::CountersAmong {
                counter_type: CounterType::Charge,
                ..
            }
        ));
    }

    #[test]
    fn captures_conjoined_condition_boundaries() {
        let tokens = lex("It is your turn and you control a creature.");
        let splits = parse_conjoined_condition_splits(&tokens);
        assert_eq!(splits.len(), 1);
        assert!(parse_complete_phrase(
            splits[0].left_tokens,
            &["it", "is", "your", "turn"]
        ));
        assert!(parse_complete_phrase(
            splits[0].right_tokens,
            &["you", "control", "a", "creature"]
        ));
    }

    #[test]
    fn parses_typed_quantity_and_source_relation_shapes() {
        let life = lex("You have five or less life.");
        assert_eq!(parse_life_total_or_less_condition(&life), Some(5));

        let x_value = lex("X is greater than three.");
        assert_eq!(parse_x_value_at_least_condition(&x_value), Some(4));

        let blocking = lex("Two or more creatures are blocking it.");
        assert_eq!(
            parse_blocking_source_condition(&blocking),
            Some(BlockingSourceConditionShape {
                comparison: Comparison::GreaterThanOrEqual(2),
            })
        );

        let source_counter = lex("This creature has three or more charge counters on it.");
        assert_eq!(
            parse_source_counter_condition(&source_counter),
            Ok(Some(SourceCounterConditionShape {
                comparison: Comparison::GreaterThanOrEqual(3),
                counter_type: Some(CounterType::Charge),
            }))
        );

        let graveyard = lex("This card is in your graveyard.");
        assert!(parse_source_in_graveyard_condition(&graveyard));
    }
}
