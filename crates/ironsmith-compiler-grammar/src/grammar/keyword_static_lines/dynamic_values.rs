use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::object::CounterType;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{filters, primitives, static_keyword_cost_shapes};
use super::{
    CharacteristicAggregateKind, DynamicPlayerKind, parse_cards_drawn_this_turn_player_tokens,
    parse_characteristic_aggregate_prefix_tokens, parse_spell_cast_this_turn_player_tokens,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellCastDynamicKind {
    CardTypes,
    OtherThanFirst,
    MatchingTypes {
        instant: bool,
        sorcery: bool,
        exclude_source: bool,
    },
    Simple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicThisWayMetric {
    Destroyed,
    Sacrificed,
    Discarded,
    Exiled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterReferenceKind {
    Source,
    Tagged,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterReferenceSpec<'a> {
    pub counter_type: Option<CounterType>,
    pub reference_tokens: &'a [OwnedLexToken],
    pub reference_kind: CounterReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicCostValueShape<'a> {
    CardsDrawn(DynamicPlayerKind),
    LifeGained(DynamicPlayerKind),
    KickCount,
    CreaturesDiedThisTurn,
    OpponentsLifeLostThisTurn,
    ControlledCreaturesDiedThisTurn,
    PlayersBeingAttacked,
    SpellCast {
        player: DynamicPlayerKind,
        kind: SpellCastDynamicKind,
    },
    CardTypesInGraveyard(DynamicPlayerKind),
    ColorsSpentToCastThisSpell,
    PartySize,
    AggregateScope,
    CardTypesAmong {
        scope_tokens: &'a [OwnedLexToken],
    },
    UnsupportedCardTypesAmong,
    CountersRemovedThisWay,
    PlayerCounters(CounterType),
    ThisWayMetric(DynamicThisWayMetric),
    RevealedPublic,
    RevealedOther,
    CounterReference(CounterReferenceSpec<'a>),
    UnsupportedThisWay,
    Other {
        filter_tokens: &'a [OwnedLexToken],
    },
}

pub fn parse_dynamic_cost_value_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<DynamicCostValueShape<'_>> {
    if let Some(player) = parse_cards_drawn_this_turn_player_tokens(tokens) {
        return Some(DynamicCostValueShape::CardsDrawn(player));
    }
    let each_idx = static_keyword_cost_shapes::parse_dynamic_cost_each_word(tokens)?.token;
    let filter_tokens = tokens.get(each_idx + 1..)?;
    if filter_tokens.is_empty() {
        return None;
    }
    Some(classify_dynamic_filter(filter_tokens))
}

fn classify_dynamic_filter(tokens: &[OwnedLexToken]) -> DynamicCostValueShape<'_> {
    if starts_with_any(
        tokens,
        &[
            &["1", "life", "you", "gained", "this", "turn"],
            &["1", "life", "youve", "gained", "this", "turn"],
            &["1", "life", "you've", "gained", "this", "turn"],
            &["one", "life", "you", "gained", "this", "turn"],
            &["life", "you", "gained", "this", "turn"],
        ],
    ) {
        return DynamicCostValueShape::LifeGained(DynamicPlayerKind::You);
    }
    if starts_with_any(
        tokens,
        &[
            &["1", "life", "your", "opponents", "gained", "this", "turn"],
            &["one", "life", "your", "opponents", "gained", "this", "turn"],
            &["life", "your", "opponents", "gained", "this", "turn"],
        ],
    ) {
        return DynamicCostValueShape::LifeGained(DynamicPlayerKind::Opponent);
    }
    if starts_with_any(
        tokens,
        &[
            &["time", "this", "was", "kicked"],
            &["time", "this", "spell", "was", "kicked"],
        ],
    ) {
        return DynamicCostValueShape::KickCount;
    }
    if starts_with_any(
        tokens,
        &[
            &["creature", "that", "died", "this", "turn"],
            &["creatures", "that", "died", "this", "turn"],
        ],
    ) {
        return DynamicCostValueShape::CreaturesDiedThisTurn;
    }
    if has_all_semantic_words(tokens, &["life", "opponents", "lost", "this", "turn"]) {
        return DynamicCostValueShape::OpponentsLifeLostThisTurn;
    }
    if starts_with_any(
        tokens,
        &[
            &["creature", "that", "died", "under", "your", "control"],
            &["creatures", "that", "died", "under", "your", "control"],
        ],
    ) && has_phrase(tokens, &["this", "turn"])
    {
        return DynamicCostValueShape::ControlledCreaturesDiedThisTurn;
    }
    if starts_with_any(
        tokens,
        &[
            &["opponent", "youre", "attacking"],
            &["opponents", "youre", "attacking"],
            &["opponent", "you're", "attacking"],
            &["opponents", "you're", "attacking"],
            &["opponent", "you", "are", "attacking"],
            &["opponents", "you", "are", "attacking"],
        ],
    ) {
        return DynamicCostValueShape::PlayersBeingAttacked;
    }
    if let Some(player) = parse_spell_cast_this_turn_player_tokens(tokens) {
        let kind = if has_card_type_marker(tokens) {
            Some(SpellCastDynamicKind::CardTypes)
        } else if has_phrase(tokens, &["other", "than", "the", "first"])
            || has_phrase(tokens, &["other", "than", "first"])
        {
            Some(SpellCastDynamicKind::OtherThanFirst)
        } else {
            let instant = has_word(tokens, "instant");
            let sorcery = has_word(tokens, "sorcery");
            if instant || sorcery {
                Some(SpellCastDynamicKind::MatchingTypes {
                    instant,
                    sorcery,
                    exclude_source: has_word(tokens, "other"),
                })
            } else if is_simple_spell_cast_value(tokens) {
                Some(SpellCastDynamicKind::Simple)
            } else {
                None
            }
        };
        if let Some(kind) = kind {
            return DynamicCostValueShape::SpellCast { player, kind };
        }
    }
    if starts_with_any(
        tokens,
        &[
            &["card", "youve", "drawn", "this", "turn"],
            &["cards", "youve", "drawn", "this", "turn"],
            &["card", "you", "have", "drawn", "this", "turn"],
            &["cards", "you", "have", "drawn", "this", "turn"],
            &["card", "you", "ve", "drawn", "this", "turn"],
            &["cards", "you", "ve", "drawn", "this", "turn"],
        ],
    ) {
        return DynamicCostValueShape::CardsDrawn(DynamicPlayerKind::You);
    }
    if let Some(player) = parse_cards_drawn_this_turn_player_tokens(tokens) {
        return DynamicCostValueShape::CardsDrawn(player);
    }
    if starts_with_any(
        tokens,
        &[
            &[
                "color", "of", "mana", "spent", "to", "cast", "this", "spell",
            ],
            &[
                "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
            ],
            &["color", "of", "mana", "used", "to", "cast", "this", "spell"],
            &[
                "colors", "of", "mana", "used", "to", "cast", "this", "spell",
            ],
        ],
    ) {
        return DynamicCostValueShape::ColorsSpentToCastThisSpell;
    }
    if starts_with_any(
        tokens,
        &[
            &["creature", "in", "your", "party"],
            &["creatures", "in", "your", "party"],
        ],
    ) {
        return DynamicCostValueShape::PartySize;
    }
    if let Some(aggregate) = parse_characteristic_aggregate_prefix_tokens(tokens) {
        if aggregate.kind == CharacteristicAggregateKind::CardTypes {
            let scope_tokens = trim_sentence_end(aggregate.scope_tokens);
            return DynamicCostValueShape::CardTypesAmong { scope_tokens };
        }
        return DynamicCostValueShape::AggregateScope;
    }
    if has_card_types_among_marker(tokens) {
        return DynamicCostValueShape::UnsupportedCardTypesAmong;
    }
    if has_card_type_marker(tokens) && has_word(tokens, "graveyard") {
        let player = if has_phrase(tokens, &["opponents", "graveyard"])
            || has_phrase(tokens, &["opponent", "graveyard"])
        {
            DynamicPlayerKind::Opponent
        } else {
            DynamicPlayerKind::You
        };
        return DynamicCostValueShape::CardTypesInGraveyard(player);
    }
    if has_phrase(tokens, &["this", "way"])
        && has_word(tokens, "removed")
        && (has_word(tokens, "counter") || has_word(tokens, "counters"))
    {
        return DynamicCostValueShape::CountersRemovedThisWay;
    }
    if let Some(counter_type) = parse_player_counters(tokens) {
        return DynamicCostValueShape::PlayerCounters(counter_type);
    }
    if has_phrase(tokens, &["this", "way"]) {
        for (word, metric) in [
            ("destroyed", DynamicThisWayMetric::Destroyed),
            ("sacrificed", DynamicThisWayMetric::Sacrificed),
            ("discarded", DynamicThisWayMetric::Discarded),
            ("exiled", DynamicThisWayMetric::Exiled),
        ] {
            if has_word(tokens, word) {
                return DynamicCostValueShape::ThisWayMetric(metric);
            }
        }
        if has_word(tokens, "revealed") {
            if primitives::parse_all(
                tokens,
                (
                    alt((primitives::kw("card"), primitives::kw("cards"))),
                    primitives::phrase(&["revealed", "this", "way"]),
                    primitives::sentence_end(),
                )
                    .void(),
                "public revealed dynamic value",
            )
            .is_ok()
            {
                return DynamicCostValueShape::RevealedPublic;
            }
            return DynamicCostValueShape::RevealedOther;
        }
    }
    if let Some(reference) = parse_counter_reference(tokens) {
        return DynamicCostValueShape::CounterReference(reference);
    }
    if has_phrase(tokens, &["this", "way"]) {
        DynamicCostValueShape::UnsupportedThisWay
    } else {
        DynamicCostValueShape::Other {
            filter_tokens: tokens,
        }
    }
}

fn parse_player_counters(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    let mut input = LexStream::new(tokens);
    let counter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .parse_next(&mut input)
        .ok()?;
    primitives::phrase(&["you", "have"])
        .parse_next(&mut input)
        .ok()?;
    primitives::sentence_end().parse_next(&mut input).ok()?;
    filters::parse_counter_type_from_tokens(counter_tokens)
}

fn parse_counter_reference(tokens: &[OwnedLexToken]) -> Option<CounterReferenceSpec<'_>> {
    let mut input = LexStream::new(tokens);
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("one"),
        primitives::kw("another"),
    )))
    .parse_next(&mut input)
    .ok()?;
    let counter_type_tokens = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    let counter_type = parse_counter_reference_type(counter_type_tokens)?;
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .parse_next(&mut input)
        .ok()?;
    primitives::kw("on").parse_next(&mut input).ok()?;
    let reference_tokens = take_sentence_body(&mut input).ok()?;
    let reference_kind = if starts_with_any(
        reference_tokens,
        &[
            &["it"],
            &["this"],
            &["that", "object"],
            &["that", "permanent"],
        ],
    ) {
        CounterReferenceKind::Source
    } else if is_tagged_counter_reference(reference_tokens) {
        CounterReferenceKind::Tagged
    } else {
        CounterReferenceKind::Other
    };
    Some(CounterReferenceSpec {
        counter_type,
        reference_tokens,
        reference_kind,
    })
}

/// Parses only a counter descriptor, rather than accepting arbitrary object-filter
/// text that happens to end immediately before the word "counter". This keeps
/// `for each creature ... with a +1/+1 counter on it` in the object-count path.
fn parse_counter_reference_type(tokens: &[OwnedLexToken]) -> Option<Option<CounterType>> {
    let words = crate::lexer::token_word_refs(tokens);
    if words.is_empty() {
        Some(None)
    } else if words.len() == 1 {
        let word = words[0];
        filters::parse_counter_type_word(word)
            .or_else(|| {
                word.chars()
                    .all(|character| character.is_ascii_alphabetic())
                    .then(|| CounterType::Named(filters::intern_counter_name(word).into()))
            })
            .map(Some)
    } else if primitives::parse_word_sequence_complete(&words, &["first", "strike"]).is_some() {
        Some(Some(CounterType::FirstStrike))
    } else if primitives::parse_word_sequence_complete(&words, &["double", "strike"]).is_some() {
        Some(Some(CounterType::DoubleStrike))
    } else {
        None
    }
}

fn is_tagged_counter_reference(tokens: &[OwnedLexToken]) -> bool {
    const REFERENCES: &[&[&str]] = &[
        &["that"],
        &["that", "card"],
        &["that", "creature"],
        &["that", "object"],
        &["that", "permanent"],
        &["those"],
        &["those", "cards"],
        &["those", "creatures"],
        &["those", "objects"],
        &["those", "permanents"],
    ];
    REFERENCES.iter().any(|words| {
        primitives::parse_all(
            tokens,
            (primitives::phrase(words), primitives::sentence_end()).void(),
            "tagged counter reference",
        )
        .is_ok()
    })
}

fn starts_with_any(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::parse_prefix(tokens, primitives::phrase(phrase)).is_some())
}

fn has_all_semantic_words(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    words.iter().all(|word| has_word(tokens, word))
}

fn has_word(tokens: &[OwnedLexToken], word: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(word).void()).is_some()
}

fn has_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase).void()).is_some()
}

fn has_card_type_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        (
            primitives::kw("card"),
            alt((primitives::kw("type"), primitives::kw("types"))),
        )
            .void()
    })
    .is_some()
}

fn has_card_types_among_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        (
            primitives::kw("card"),
            alt((primitives::kw("type"), primitives::kw("types"))),
            primitives::kw("among"),
        )
            .void()
    })
    .is_some()
}

fn is_simple_spell_cast_value(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            alt((
                primitives::phrase(&["spell", "youve", "cast", "this", "turn"]),
                primitives::phrase(&["spells", "youve", "cast", "this", "turn"]),
                primitives::phrase(&["spell", "you've", "cast", "this", "turn"]),
                primitives::phrase(&["spells", "you've", "cast", "this", "turn"]),
                primitives::phrase(&["spell", "you", "cast", "this", "turn"]),
                primitives::phrase(&["spells", "you", "cast", "this", "turn"]),
                primitives::phrase(&["spell", "your", "cast", "this", "turn"]),
                primitives::phrase(&["spells", "your", "cast", "this", "turn"]),
            )),
            primitives::sentence_end(),
        )
            .void(),
        "simple spells-cast dynamic value",
    )
    .is_ok()
}

fn take_sentence_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(trim_lexed_commas(tokens))
}

fn trim_sentence_end(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_all(
        tokens,
        parse_sentence_body_lexed,
        "dynamic value sentence body",
    )
    .unwrap_or(tokens)
}

fn parse_sentence_body_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let body =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn classifies_dynamic_values() {
        let tokens = lex_line("for each spell you've cast this turn", 0).unwrap();
        assert!(matches!(
            parse_dynamic_cost_value_shape_tokens(&tokens),
            Some(DynamicCostValueShape::SpellCast { .. })
        ));
        let tokens = lex_line("for each card type among cards in your graveyard", 0).unwrap();
        assert!(matches!(
            parse_dynamic_cost_value_shape_tokens(&tokens),
            Some(DynamicCostValueShape::CardTypesAmong { .. })
        ));
    }

    #[test]
    fn object_filter_with_counter_is_not_a_counter_reference_value() {
        let tokens = lex_line(
            "for each creature you control with a +1/+1 counter on it",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_dynamic_cost_value_shape_tokens(&tokens),
            Some(DynamicCostValueShape::Other { filter_tokens })
                if crate::lexer::token_word_refs(filter_tokens)
                    == ["creature", "you", "control", "with", "a", "+1/+1", "counter", "on", "it"]
        ));
    }

    #[test]
    fn exact_counter_reference_value_keeps_typed_counter_kind() {
        let tokens = lex_line("for each +1/+1 counter on this creature", 0).unwrap();
        assert!(matches!(
            parse_dynamic_cost_value_shape_tokens(&tokens),
            Some(DynamicCostValueShape::CounterReference(
                CounterReferenceSpec {
                    counter_type: Some(CounterType::PlusOnePlusOne),
                    reference_kind: CounterReferenceKind::Source,
                    ..
                }
            ))
        ));
    }
}
