use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::TagKey;
use crate::effect::{EventValueSpec, Value};
use crate::filter::TaggedOpbjectRelation;
use crate::grammar::{filters, leaf, primitives, values};
use crate::lexer::{LexStream, OwnedLexToken};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::util::{
    source_choose_spec_for_surface, source_reference_surface_for_words,
    trim_edge_punctuation_tokens,
};
use crate::zone::Zone;

use super::super::ReturnTimingShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawCardCountOffset {
    MinusOne,
    PlusOne,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawHeadCountShape<'a> {
    Resolved(Value),
    CardPrefixed { count_tokens: &'a [OwnedLexToken] },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawHeadShape<'a> {
    pub count: DrawHeadCountShape<'a>,
    pub additional: bool,
    pub tail_tokens: &'a [OwnedLexToken],
    pub parsed_offset: Option<DrawCardCountOffset>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawHeadShapeError {
    MissingCount,
    MissingCardKeyword,
    UnsupportedTrailingClause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawPlayerLoopShape<'a> {
    pub opponents_only: bool,
    pub who_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawTrailingShape<'a> {
    Instead,
    Delayed(ReturnTimingShape),
    ThenPut { put_tokens: &'a [OwnedLexToken] },
    If,
    Unless,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawKnownCountShape<'a> {
    KickCount,
    ColorsAmong { filter_tokens: &'a [OwnedLexToken] },
    CreaturesDiedThisTurn,
    CreaturesDiedThisTurnControlledByYou,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawEqualStat {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawEqualShape<'a> {
    GreatestCardsDiscardedThisWay,
    StatOfTarget {
        stat: DrawEqualStat,
        target_tokens: &'a [OwnedLexToken],
    },
    Fallback {
        references_this_way: bool,
    },
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    trim_edge_punctuation_tokens(tokens)
}

fn punctuation<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::comma(),
        primitives::period(),
        primitives::semicolon(),
    ))
    .void()
    .parse_next(input)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., punctuation),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn exact_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_all(
        trimmed(tokens),
        (
            semantic_phrase(phrase),
            repeat::<_, _, (), _, _>(0.., punctuation),
            eof,
        )
            .void(),
        "draw shape",
    )
    .is_ok()
}

fn contains_word(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(expected)).is_some()
}

fn event_amount_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["that", "amount", "of"]),
        primitives::phrase(&["that", "much"]),
        primitives::phrase(&["that", "many"]),
    ))
    .void()
    .parse_next(input)
}

fn card_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)
}

pub fn parse_draw_card_count_offset_shape(tokens: &[OwnedLexToken]) -> Option<DrawCardCountOffset> {
    let tokens = trimmed(tokens);
    if exact_phrase(tokens, &["minus", "one"]) {
        Some(DrawCardCountOffset::MinusOne)
    } else if exact_phrase(tokens, &["plus", "one"]) {
        Some(DrawCardCountOffset::PlusOne)
    } else {
        None
    }
}

pub fn parse_half_rounded_down_draw_shape(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let ((), after_half) = primitives::parse_prefix(tokens, primitives::kw("half").void())?;
    let (card_idx, (), after_card) = primitives::find_prefix(after_half, || card_noun)?;
    let inner_tokens = trimmed(&after_half[..card_idx]);
    if inner_tokens.is_empty() {
        return None;
    }
    let (inner, used) = values::parse_value_prefix_lexed(inner_tokens)?;
    if used != inner_tokens.len() {
        return None;
    }
    let ((), rest) =
        primitives::parse_prefix(after_card, semantic_phrase(&["rounded", "down"]).void())?;
    Some((
        Value::HalfRoundedDown(Box::new(inner)),
        tokens.len().saturating_sub(rest.len()),
    ))
}

fn parse_as_many_this_way(tokens: &[OwnedLexToken]) -> Option<Value> {
    let (_, rest) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["as", "many"]),
            card_noun,
            primitives::kw("as"),
        )
            .void(),
    )?;
    primitives::find_prefix(rest, || semantic_phrase(&["this", "way"]))
        .is_some()
        .then_some(
            Value::EventValue(EventValueSpec::Amount)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::AsManyCardsThisWay),
        )
}

/// A trailing `where <variable> is ...` clause is bound by the sentence
/// dispatcher after effect parsing; the draw clause itself is complete.
pub fn tail_is_where_variable_binding(tokens: &[OwnedLexToken]) -> bool {
    let words = super::super::super::super::lexer::parser_token_word_refs(tokens);
    words.len() >= 3 && words[0] == "where" && words[1].len() == 1 && words[2] == "is"
}

pub fn parse_draw_head_shape(
    tokens: &[OwnedLexToken],
) -> Result<DrawHeadShape<'_>, DrawHeadShapeError> {
    let tokens = trimmed(tokens);
    let mut parsed_offset = None;
    let (count, used, embedded_card) = if let Some(((), rest)) =
        primitives::parse_prefix(tokens, event_amount_prefix)
    {
        let consumed = tokens.len().saturating_sub(rest.len());
        let mut value = Value::EventValue(EventValueSpec::Amount);
        if tokens.first().is_some_and(|token| token.is_word("that"))
            && tokens.get(1).is_some_and(|token| token.is_word("many"))
        {
            value = value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ThatManyCards);
        }
        if let Some(((), after_card)) = primitives::parse_prefix(rest, card_noun) {
            let trailing = trimmed(after_card);
            parsed_offset = parse_draw_card_count_offset_shape(trailing);
            value = match parsed_offset {
                Some(DrawCardCountOffset::MinusOne) => {
                    Value::EventValueOffset(EventValueSpec::Amount, -1)
                }
                Some(DrawCardCountOffset::PlusOne) => {
                    Value::EventValueOffset(EventValueSpec::Amount, 1)
                }
                None => value,
            };
            if parsed_offset.is_none()
                && !trailing.is_empty()
                && primitives::find_prefix(trailing, || semantic_phrase(&["for", "each"])).is_none()
                && !tail_is_where_variable_binding(trailing)
            {
                return Err(DrawHeadShapeError::UnsupportedTrailingClause);
            }
        }
        (DrawHeadCountShape::Resolved(value), consumed, false)
    } else if let Some((value, used)) = parse_half_rounded_down_draw_shape(tokens) {
        (DrawHeadCountShape::Resolved(value), used, true)
    } else if let Some(value) = parse_as_many_this_way(tokens) {
        (DrawHeadCountShape::Resolved(value), tokens.len(), true)
    } else if let Some(((), rest)) =
        primitives::parse_prefix(tokens, primitives::kw("another").void())
    {
        (
            DrawHeadCountShape::Resolved(Value::Fixed(1)),
            tokens.len().saturating_sub(rest.len()),
            false,
        )
    } else if let Some(((), count_tokens)) = primitives::parse_prefix(tokens, card_noun) {
        (
            DrawHeadCountShape::CardPrefixed {
                count_tokens: trimmed(count_tokens),
            },
            tokens.len(),
            true,
        )
    } else if let Some((amount, rest)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["up", "to"]),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, amount)| amount),
    ) {
        (
            DrawHeadCountShape::Resolved(Value::Fixed(amount as i32)),
            tokens.len().saturating_sub(rest.len()),
            false,
        )
    } else if let Some((value, used)) = values::parse_value_prefix_lexed(tokens) {
        (DrawHeadCountShape::Resolved(value), used, false)
    } else {
        return Err(DrawHeadShapeError::MissingCount);
    };

    let rest = trimmed(tokens.get(used..).unwrap_or_default());
    let (additional, tail_tokens) = if embedded_card {
        (false, rest)
    } else {
        let additional =
            primitives::parse_prefix(rest, primitives::kw("additional").void()).is_some();
        let (_, tail) =
            primitives::parse_prefix(rest, (opt(primitives::kw("additional")), card_noun).void())
                .ok_or(DrawHeadShapeError::MissingCardKeyword)?;
        (additional, trimmed(tail))
    };
    Ok(DrawHeadShape {
        count,
        additional,
        tail_tokens,
        parsed_offset,
    })
}

pub fn parse_draw_player_loop_shape(tokens: &[OwnedLexToken]) -> Option<DrawPlayerLoopShape<'_>> {
    let tokens = trimmed(tokens);
    let (opponents_only, who_tokens) = primitives::parse_prefix(
        tokens,
        (
            alt((
                primitives::phrase(&["for", "each"]).void(),
                primitives::kw("each").void(),
            )),
            alt((
                alt((primitives::kw("opponent"), primitives::kw("opponents"))).value(true),
                alt((primitives::kw("player"), primitives::kw("players"))).value(false),
            )),
        )
            .map(|(_, opponents_only)| opponents_only),
    )?;
    primitives::parse_prefix(who_tokens, primitives::kw("who"))?;
    Some(DrawPlayerLoopShape {
        opponents_only,
        who_tokens: trimmed(who_tokens),
    })
}

pub fn parse_draw_trailing_shape(tokens: &[OwnedLexToken]) -> Option<DrawTrailingShape<'_>> {
    let tokens = trimmed(tokens);
    let words = primitives::TokenWordView::new(tokens).to_word_refs();
    // One declared alternation: the alternatives are exclusive shapes, and the
    // first that reads the input names it.
    let alternation = None::<DrawTrailingShape<'_>>
        .or_else(|| {
            if exact_phrase(tokens, &["instead"]) {
                return Some(DrawTrailingShape::Instead);
            }
            None
        })
        .or_else(|| {
            if let Some(timing) = super::super::parse_return_timing_words_shape(&words) {
                return Some(DrawTrailingShape::Delayed(timing));
            }
            None
        })
        .or_else(|| {
            if primitives::parse_all(
                tokens,
                (
                    primitives::kw("at"),
                    opt(primitives::kw("the")),
                    primitives::kw("beginning"),
                    primitives::kw("of"),
                    opt(primitives::kw("the")),
                    primitives::kw("next"),
                    semantic_kw("turns"),
                    primitives::kw("upkeep"),
                    primitives::sentence_end(),
                )
                    .void(),
                "next turn upkeep draw timing",
            )
            .is_ok()
            {
                return Some(DrawTrailingShape::Delayed(ReturnTimingShape::NextUpkeep(
                    crate::cards::builders::PlayerAst::Any,
                )));
            }
            None
        })
        .or_else(|| {
            if let Some(((), put_tokens)) =
                primitives::parse_prefix(tokens, primitives::phrase(&["then", "put"]).void())
            {
                return Some(DrawTrailingShape::ThenPut {
                    put_tokens: trimmed(put_tokens),
                });
            }
            None
        })
        .or_else(|| {
            if primitives::parse_prefix(tokens, primitives::kw("if")).is_some() {
                return Some(DrawTrailingShape::If);
            }
            None
        })
        .or_else(|| {
            if primitives::parse_prefix(tokens, primitives::kw("unless")).is_some() {
                return Some(DrawTrailingShape::Unless);
            }
            None
        });
    if let Some(shape) = alternation {
        return Some(shape);
    }
    None
}

pub fn strip_draw_for_each_prefix(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, filter_tokens) =
        primitives::parse_prefix(trimmed(tokens), primitives::phrase(&["for", "each"]).void())?;
    let filter_tokens = trimmed(filter_tokens);
    (!filter_tokens.is_empty()).then_some(filter_tokens)
}

pub fn contains_draw_for_each_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(trimmed(tokens), || semantic_phrase(&["for", "each"])).is_some()
}

fn exact_source_reference(tokens: &[OwnedLexToken]) -> bool {
    if exact_phrase(tokens, &["this"])
        || exact_phrase(tokens, &["this", "spell"])
        || exact_phrase(tokens, &["it"])
    {
        return true;
    }
    let words = primitives::TokenWordView::new(tokens).to_word_refs();
    source_reference_surface_for_words(&words).is_some()
}

fn is_kick_count(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, after_time)) = primitives::parse_prefix(
        trimmed(tokens),
        alt((primitives::kw("time"), primitives::kw("times"))),
    ) else {
        return false;
    };
    let Some((source, ())) = primitives::split_lexed_once_before_suffix(after_time, 0, || {
        (
            semantic_phrase(&["was", "kicked"]),
            repeat::<_, _, (), _, _>(0.., punctuation),
            eof,
        )
            .void()
    }) else {
        return false;
    };
    exact_source_reference(trimmed(source))
}

pub fn parse_draw_known_count_shape(tokens: &[OwnedLexToken]) -> Option<DrawKnownCountShape<'_>> {
    let tokens = trimmed(tokens);
    if is_kick_count(tokens) {
        return Some(DrawKnownCountShape::KickCount);
    }
    if let Some(((), filter_tokens)) = primitives::parse_prefix(
        tokens,
        (
            alt((primitives::kw("color"), primitives::kw("colors"))),
            primitives::kw("among"),
        )
            .void(),
    ) {
        let filter_tokens = trimmed(filter_tokens);
        if !filter_tokens.is_empty() {
            return Some(DrawKnownCountShape::ColorsAmong { filter_tokens });
        }
    }
    if exact_phrase(tokens, &["creature", "that", "died", "this", "turn"])
        || exact_phrase(tokens, &["creatures", "that", "died", "this", "turn"])
    {
        return Some(DrawKnownCountShape::CreaturesDiedThisTurn);
    }
    if exact_phrase(
        tokens,
        &[
            "creature", "that", "died", "under", "your", "control", "this", "turn",
        ],
    ) || exact_phrase(
        tokens,
        &[
            "creatures",
            "that",
            "died",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ],
    ) {
        return Some(DrawKnownCountShape::CreaturesDiedThisTurnControlledByYou);
    }
    None
}

pub fn parse_draw_this_way_metric_shape(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trimmed(tokens);
    primitives::split_lexed_once_before_suffix(tokens, 0, || {
        (
            semantic_phrase(&["this", "way"]),
            repeat::<_, _, (), _, _>(0.., punctuation),
            eof,
        )
            .void()
    })?;

    // Keep the object restriction carried by filtered mill-result phrases such
    // as "for each creature card put into their graveyard this way". Other
    // prior-action counts (notably "destroyed this way") deliberately retain
    // the effect-metric path below, which counts the producer's actual outcome.
    let words = primitives::TokenWordView::new(tokens).to_word_refs();
    let counter_words = crate::word_primitives::strip_any_prefix(
        &words,
        &[&["the", "number", "of"], &["number", "of"]],
    )
    .map_or(words.as_slice(), |(_, tail)| tail);
    if let Some(counter_idx) = crate::word_primitives::select_word_position(counter_words, |word| {
        matches!(word, "counter" | "counters")
    })
    .filter(|counter_idx| *counter_idx <= 2)
        && counter_words.get(counter_idx + 1..).is_some_and(|tail| {
            crate::word_primitives::parse_sequence_complete(tail, &["removed", "this", "way"])
        })
    {
        let counter_type = filters::parse_counter_type_words(&counter_words[..=counter_idx]);
        return Some(
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::Removed)
                .with_counter_type(counter_type),
            )
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay),
        );
    }
    let put_into_graveyard = crate::word_primitives::sequence_occurs(&words, &["put", "into"])
        && crate::word_primitives::contains_word(&words, "graveyard");
    let mut for_each_words = vec!["for", "each"];
    for_each_words.extend(words.iter().copied());
    if let Some((value @ Value::PendingPriorEffectMetric(_), used)) =
        crate::grammar::shared_util::count_shapes::parse_for_each_count_value_words(&for_each_words)
        && used == for_each_words.len()
    {
        return Some(value);
    }
    if put_into_graveyard
        && let Some((Value::Count(mut filter), used)) =
            crate::grammar::shared_util::count_shapes::parse_for_each_count_value_words(
                &for_each_words,
            )
        && used == for_each_words.len()
    {
        filter.zone = Some(Zone::Graveyard);
        if crate::word_primitives::sequence_occurs(&words, &["their", "graveyard"]) {
            filter.owner = Some(PlayerFilter::IteratedPlayer);
        } else if crate::word_primitives::sequence_occurs(&words, &["your", "graveyard"]) {
            filter.owner = Some(PlayerFilter::You);
        }
        return Some(Value::Count(filter));
    }

    let metric = Value::PendingEffectMetric {
        source: ironsmith_core::EffectMetricSource::Outcome,
        metric: ironsmith_core::EffectMetric::Count,
    };
    if crate::word_primitives::parse_sequence_complete(
        counter_words,
        &["opponents", "dealt", "damage", "this", "way"],
    ) {
        return Some(
            metric.with_surface_hint(ironsmith_core::ValueSurfaceHint::OpponentsDealtDamageThisWay),
        );
    }
    if contains_word(tokens, "discarded") {
        return Some(
            metric.with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsDiscardedThisWay),
        );
    }
    if contains_word(tokens, "sacrificed")
        && (contains_word(tokens, "permanent") || contains_word(tokens, "permanents"))
    {
        return Some(
            metric.with_surface_hint(ironsmith_core::ValueSurfaceHint::PermanentsSacrificedThisWay),
        );
    }
    if contains_word(tokens, "exiled") {
        return Some(
            metric.with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsExiledThisWay),
        );
    }
    Some(metric)
}

pub fn parse_draw_equal_this_way_metric_shape(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trimmed(tokens);
    let ((), value_tokens) =
        primitives::parse_prefix(tokens, primitives::phrase(&["equal", "to"]).void())?;
    parse_draw_this_way_metric_shape(value_tokens)
}

fn exact_any(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    phrases.iter().any(|phrase| exact_phrase(tokens, phrase))
}

pub fn parse_draw_counter_reference_shape(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trimmed(tokens);
    let (counter_idx, (), after_counter) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("counter"), primitives::kw("counters"))).void()
    })?;
    // A counter-reference count starts with its counter descriptor. A later
    // counter noun belongs to an object filter such as "creature you control
    // with a +1/+1 counter on it" and must fall through to that parser.
    if counter_idx == 0 || counter_idx > 2 {
        return None;
    }
    let descriptor_tokens = trimmed(&tokens[..=counter_idx]);
    let counter_type = filters::parse_counter_type_from_tokens(descriptor_tokens);
    let tail = trimmed(after_counter);
    if exact_any(tail, &[&["you", "have"], &["you", "ve"]]) {
        return counter_type
            .map(|counter_type| Value::PlayerCounters(PlayerFilter::You, counter_type));
    }
    let ((), reference_tokens) = primitives::parse_prefix(tail, primitives::kw("on").void())?;
    let reference_tokens = trimmed(reference_tokens);
    let choose = if exact_any(
        reference_tokens,
        &[
            &["it"],
            &["this"],
            &["this", "artifact"],
            &["this", "aura"],
            &["this", "battle"],
            &["this", "card"],
            &["this", "creature"],
            &["this", "enchantment"],
            &["this", "land"],
            &["this", "permanent"],
            &["this", "planeswalker"],
            &["this", "source"],
        ],
    ) {
        if let Some(counter_type) = counter_type {
            return Some(Value::CountersOnSource(counter_type));
        }
        ChooseSpec::Source
    } else if exact_any(
        reference_tokens,
        &[
            &["that"],
            &["that", "creature"],
            &["that", "object"],
            &["that", "permanent"],
            &["those"],
            &["those", "creatures"],
            &["those", "permanents"],
        ],
    ) {
        ChooseSpec::Tagged((crate::tag::CompilerReferenceTag::It.bind()).into())
    } else {
        let words = primitives::TokenWordView::new(reference_tokens).to_word_refs();
        source_choose_spec_for_surface(source_reference_surface_for_words(&words)?)
    };
    Some(Value::CountersOn(Box::new(choose), counter_type))
}

#[cfg(test)]
#[path = "draw_inline_tests.rs"]
mod tests;

#[path = "draw/zone.rs"]
mod zone_programs;
pub use zone_programs::same_name_graveyard_count_value;
#[path = "draw/counter.rs"]
mod counter_programs;
pub use counter_programs::counter_same_name_graveyard_shape;
#[path = "draw/resource.rs"]
mod resource_programs;
pub use resource_programs::parse_draw_equal_shape;
