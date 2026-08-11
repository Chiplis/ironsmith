use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::{IT_TAG, PlayerAst};
use crate::effect::{ChoiceCount, Value};
use crate::object::CounterType;
use crate::runtime_backend::front_end::grammar::{filters, leaf, primitives, values};
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, TokenKind};
use crate::runtime_backend::front_end::shared::util::{
    source_reference_surface_for_possessive_words, source_reference_surface_for_words,
    this_source_surface_for_words,
};
use crate::target::{PlayerFilter, SourceReferenceSurface};

use super::counter_marker_shapes;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DynamicCounterCountShape {
    LifeLostThisWay {
        group_size: i32,
    },
    CreaturesDiedThisTurn,
    SpellsCastThisTurn {
        player: PlayerFilter,
        other_than_first: bool,
    },
    ColorsOfManaSpentToCastThisSpell,
    BasicLandTypesAmongLandsYouControl,
    ObjectsAffectedThisWay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterReferenceSource {
    TaggedIt,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReferentialCounterCountShape {
    pub(crate) source: CounterReferenceSource,
    pub(crate) counter_type: Option<CounterType>,
    pub(crate) consumed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterCountPrefixShape<'a> {
    UpTo {
        inner_tokens: &'a [OwnedLexToken],
    },
    EventAmount {
        consumed: usize,
    },
    Another,
    Referential(ReferentialCounterCountShape),
    NumberOf {
        value_tokens: Option<&'a [OwnedLexToken]>,
        equal_to_difference: bool,
        equal_to_after_target: bool,
    },
    ExistingCounterEqual {
        value_tokens: &'a [OwnedLexToken],
    },
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CounterDescriptorShape {
    pub(crate) count: u32,
    pub(crate) counter_type: CounterType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PutCounterTargetShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) equal_to_difference: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PutOrRemoveCounterShape<'a> {
    pub(crate) base_target_tokens: &'a [OwnedLexToken],
    pub(crate) remove_count: Value,
    pub(crate) remove_counter_type: Option<CounterType>,
    pub(crate) remove_mode_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HalfStartingLifeRounding {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HalfStartingLifeShape {
    pub(crate) player: PlayerFilter,
    pub(crate) rounding: HalfStartingLifeRounding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransformTargetShape<'a> {
    ImplicitSource,
    EachObject {
        filter_tokens: &'a [OwnedLexToken],
    },
    Source {
        surface: Option<SourceReferenceSurface>,
    },
    Target {
        target_tokens: &'a [OwnedLexToken],
        fallback_to_source: bool,
    },
}

fn trim_shape_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
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

fn has_word(tokens: &[OwnedLexToken], word: &'static str) -> bool {
    primitives::find_prefix(tokens, || possessive_surface_kw(word)).is_some()
}

fn has_phrase(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(words)).is_some()
}

fn parse_life_lost_this_way<'a>(input: &mut LexStream<'a>) -> WResult<DynamicCounterCountShape> {
    let group_size = opt(leaf::parse_leaf_number_prefix_lexed)
        .map(|count| count.unwrap_or(1) as i32)
        .parse_next(input)?;
    alt((primitives::kw("life"), primitives::kw("lives"))).parse_next(input)?;
    primitives::phrase(&["lost", "this", "way"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(DynamicCounterCountShape::LifeLostThisWay { group_size })
}

fn parse_creatures_died_this_turn<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        alt((primitives::kw("creature"), primitives::kw("creatures"))),
        primitives::phrase(&["that", "died", "this", "turn"]),
    )
        .void()
        .parse_next(input)
}

fn parse_colors_spent_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        alt((primitives::kw("color"), primitives::kw("colors"))),
        primitives::kw("of"),
        primitives::kw("mana"),
        alt((primitives::kw("spent"), primitives::kw("used"))),
        primitives::phrase(&["to", "cast", "this", "spell"]),
    )
        .void()
        .parse_next(input)
}

fn parse_basic_land_types_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        primitives::kw("basic"),
        primitives::kw("land"),
        alt((primitives::kw("type"), primitives::kw("types"))),
        primitives::kw("among"),
        opt(primitives::kw("the")),
        primitives::phrase(&["lands", "you", "control"]),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn parse_dynamic_counter_count_shape(
    tokens: &[OwnedLexToken],
) -> Option<DynamicCounterCountShape> {
    let tokens = trim_shape_edges(tokens);
    if let Ok(shape) = primitives::parse_all(tokens, parse_life_lost_this_way, "life lost count") {
        return Some(shape);
    }
    if primitives::parse_prefix(tokens, parse_creatures_died_this_turn).is_some() {
        return Some(DynamicCounterCountShape::CreaturesDiedThisTurn);
    }
    if has_word(tokens, "spell") || has_word(tokens, "spells") {
        let has_cast = has_word(tokens, "cast") || has_word(tokens, "casts");
        if has_cast && has_word(tokens, "turn") {
            let player =
                if has_word(tokens, "you") || has_word(tokens, "your") || has_word(tokens, "youve")
                {
                    PlayerFilter::You
                } else if has_word(tokens, "opponent") || has_word(tokens, "opponents") {
                    PlayerFilter::Opponent
                } else {
                    PlayerFilter::Any
                };
            let other_than_first = has_phrase(tokens, &["other", "than", "the", "first"]);
            if other_than_first || has_phrase(tokens, &["this", "turn"]) {
                return Some(DynamicCounterCountShape::SpellsCastThisTurn {
                    player,
                    other_than_first,
                });
            }
        }
    }
    if primitives::parse_prefix(tokens, parse_colors_spent_prefix).is_some() {
        return Some(DynamicCounterCountShape::ColorsOfManaSpentToCastThisSpell);
    }
    if primitives::parse_prefix(tokens, parse_basic_land_types_prefix).is_some() {
        return Some(DynamicCounterCountShape::BasicLandTypesAmongLandsYouControl);
    }
    if has_phrase(tokens, &["this", "way"])
        && (has_word(tokens, "destroyed")
            || has_word(tokens, "died")
            || has_word(tokens, "exiled")
            || has_word(tokens, "sacrificed")
            || has_word(tokens, "discarded")
            || has_word(tokens, "milled")
            || has_word(tokens, "revealed"))
    {
        return Some(DynamicCounterCountShape::ObjectsAffectedThisWay);
    }
    None
}

fn parse_counter_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .void()
        .parse_next(input)
}

fn parse_referential_counter_count_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReferentialCounterCountShape> {
    if let Some((source, rest)) = primitives::parse_prefix(
        tokens,
        alt((
            alt((primitives::kw("its"), primitives::kw("those")))
                .value(CounterReferenceSource::TaggedIt),
            alt((
                primitives::kw("this"),
                primitives::kw("this's"),
                primitives::kw("thiss"),
            ))
            .value(CounterReferenceSource::Source),
        )),
    ) {
        let source_consumed = tokens.len().checked_sub(rest.len())?;
        if let Some(((), after_noun)) = primitives::parse_prefix(rest, parse_counter_noun) {
            let consumed = tokens.len().checked_sub(after_noun.len())?;
            return Some(ReferentialCounterCountShape {
                source,
                counter_type: None,
                consumed,
            });
        }
        let (noun_idx, (), after_noun) = primitives::find_prefix(rest, || parse_counter_noun)?;
        if noun_idx == 0 {
            return None;
        }
        let descriptor_end = rest.len().checked_sub(after_noun.len())?;
        let counter_type = filters::parse_counter_type_from_tokens(&rest[..descriptor_end])?;
        return Some(ReferentialCounterCountShape {
            source,
            counter_type: Some(counter_type),
            consumed: source_consumed + descriptor_end,
        });
    }

    let (noun_idx, (), after_noun) = primitives::find_prefix(tokens, || parse_counter_noun)?;
    let noun_end = tokens.len().checked_sub(after_noun.len())?;
    for source_end in (1..=noun_idx).rev() {
        let source_words = primitives::TokenWordView::new(&tokens[..source_end]).to_word_refs();
        if source_reference_surface_for_possessive_words(&source_words).is_none() {
            continue;
        }
        let counter_type = if source_end == noun_idx {
            None
        } else {
            Some(filters::parse_counter_type_from_tokens(
                &tokens[source_end..noun_end],
            )?)
        };
        return Some(ReferentialCounterCountShape {
            source: CounterReferenceSource::Source,
            counter_type,
            consumed: noun_end,
        });
    }
    None
}

fn equal_value_shape(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], bool)> {
    let (_, (), after_equal) =
        primitives::find_prefix(tokens, || primitives::phrase(&["equal", "to"]))?;
    let after_equal = trim_shape_edges(after_equal);
    let equal_to_difference = primitives::parse_all(
        after_equal,
        (
            opt(primitives::kw("the")),
            primitives::kw("difference"),
            primitives::sentence_end(),
        )
            .void(),
        "counter difference value",
    )
    .is_ok();
    let value_tokens =
        primitives::split_lexed_once_on_separator(after_equal, || primitives::kw("on").void())
            .map(|(head, _)| head)
            .unwrap_or(after_equal);
    Some((trim_shape_edges(value_tokens), equal_to_difference))
}

pub(crate) fn parse_counter_count_prefix_shape(
    tokens: &[OwnedLexToken],
) -> CounterCountPrefixShape<'_> {
    if let Some(((), inner_tokens)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["up", "to"]).void())
    {
        return CounterCountPrefixShape::UpTo { inner_tokens };
    }
    if let Some(((), rest)) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["that", "many"]),
            primitives::phrase(&["that", "much"]),
        )),
    ) {
        return CounterCountPrefixShape::EventAmount {
            consumed: tokens.len().saturating_sub(rest.len()),
        };
    }
    if primitives::parse_prefix(tokens, primitives::kw("another")).is_some() {
        return CounterCountPrefixShape::Another;
    }
    if let Some(shape) = parse_referential_counter_count_shape(tokens) {
        return CounterCountPrefixShape::Referential(shape);
    }
    if let Some(((), _)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["a", "number", "of"]).void())
    {
        let equal = equal_value_shape(tokens);
        let equal_to_after_target = primitives::find_prefix(tokens, || primitives::kw("on").void())
            .zip(primitives::find_prefix(tokens, || {
                primitives::phrase(&["equal", "to"]).void()
            }))
            .is_some_and(|((on_idx, _, _), (equal_idx, _, _))| on_idx < equal_idx);
        return CounterCountPrefixShape::NumberOf {
            value_tokens: equal.map(|(value_tokens, _)| value_tokens),
            equal_to_difference: equal.is_some_and(|(_, difference)| difference),
            equal_to_after_target,
        };
    }
    if filters::parse_counter_type_from_tokens(tokens).is_some()
        && let Some((_, (), after_on)) =
            primitives::find_prefix(tokens, || primitives::kw("on").void())
        && let Some((value_tokens, _)) = equal_value_shape(after_on)
        && !value_tokens.is_empty()
    {
        return CounterCountPrefixShape::ExistingCounterEqual { value_tokens };
    }
    CounterCountPrefixShape::Plain
}

fn parse_descriptor_amount<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    alt((
        leaf::parse_leaf_number_prefix_lexed,
        alt((primitives::kw("a"), primitives::kw("an"))).value(1),
    ))
    .parse_next(input)
}

pub(crate) fn parse_counter_descriptor_shape(
    tokens: &[OwnedLexToken],
) -> Option<CounterDescriptorShape> {
    let tokens = trim_shape_edges(tokens);
    let (count, after_count) = primitives::parse_prefix(tokens, parse_descriptor_amount)?;
    let (noun_idx, (), after_noun) = primitives::find_prefix(after_count, || parse_counter_noun)?;
    let descriptor_end = after_count.len().checked_sub(after_noun.len())?;
    if noun_idx == 0 && descriptor_end == 0 {
        return None;
    }
    let counter_type = filters::parse_counter_type_from_tokens(&after_count[..descriptor_end])?;
    if !trim_shape_edges(after_noun).is_empty() {
        return None;
    }
    Some(CounterDescriptorShape {
        count,
        counter_type,
    })
}

fn possessive_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &OwnedLexToken| {
        token.as_word().is_some()
            && token
                .parser_text()
                .chars()
                .next_back()
                .is_some_and(|ch| ch == 's')
    })
    .void()
    .parse_next(input)
}

pub(crate) fn is_named_source_power_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        trim_shape_edges(tokens),
        (
            possessive_word,
            primitives::kw("power"),
            primitives::sentence_end(),
        )
            .void(),
        "named source power",
    )
    .is_ok()
}

pub(crate) fn is_him_or_her_counter_target(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        trim_shape_edges(tokens),
        (primitives::phrase(&["him", "or", "her"]), eof).void(),
        "him or her counter target",
    )
    .is_ok()
}

pub(crate) fn strip_optional_put_prefix(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(tokens, primitives::kw("put"))
        .map(|(_, rest)| trim_shape_edges(rest))
        .unwrap_or(tokens)
}

pub(crate) fn parse_put_counter_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<PutCounterTargetShape<'_>> {
    let (_, target_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("on").void())?;
    let mut target_tokens = trim_shape_edges(target_tokens);
    // A trailing `where X is ...` binds the counter amount and is consumed by
    // the surrounding effect dispatcher. Keep it out of the target phrase so
    // nouns in the value definition (notably "that card") cannot widen the
    // target's surface metadata to "target creature card".
    if let Some((where_idx, (), _)) = primitives::find_prefix(target_tokens, || {
        primitives::phrase(&["where", "x", "is"]).void()
    }) {
        target_tokens = trim_shape_edges(&target_tokens[..where_idx]);
    }
    let mut equal_to_difference = false;
    if let Some((equal_idx, (), after_equal)) = primitives::find_prefix(target_tokens, || {
        primitives::phrase(&["equal", "to"]).void()
    }) && equal_idx > 0
    {
        equal_to_difference = primitives::parse_all(
            trim_shape_edges(after_equal),
            (
                opt(primitives::kw("the")),
                primitives::kw("difference"),
                primitives::sentence_end(),
            )
                .void(),
            "put counter difference tail",
        )
        .is_ok();
        target_tokens = trim_shape_edges(&target_tokens[..equal_idx]);
    }
    Some(PutCounterTargetShape {
        target_tokens,
        equal_to_difference,
    })
}

pub(crate) fn strip_trailing_instead(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let tokens = trim_shape_edges(tokens);
    primitives::split_lexed_once_before_suffix(tokens, 0, || {
        (primitives::kw("instead"), eof).void()
    })
    .map(|(head, ())| trim_shape_edges(head))
    .unwrap_or(tokens)
}

pub(crate) fn strip_each_counter_prefix(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("each"))?;
    Some(trim_shape_edges(rest))
}

pub(crate) fn split_for_each_counter_target(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (base, count) = primitives::split_lexed_once_on_separator(tokens, || {
        primitives::phrase(&["for", "each"]).void()
    })?;
    let base = trim_shape_edges(base);
    let count = trim_shape_edges(count);
    (!base.is_empty() && !count.is_empty()).then_some((base, count))
}

pub(crate) fn parse_shared_counter_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<counter_marker_shapes::SharedCounterTargetShape<'_>> {
    let shape = counter_marker_shapes::parse_shared_counter_target_tokens(tokens)?;
    if shape.descriptors.len() != 2
        || !has_word(shape.target_tokens, "target") && !has_word(shape.target_tokens, "targets")
    {
        return None;
    }
    let (descriptor_tokens, _) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("on").void())?;
    if descriptor_tokens
        .iter()
        .any(|token| token.kind == TokenKind::Comma)
    {
        return None;
    }
    Some(shape)
}

fn referential_remove_target<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("it").void(),
        (
            alt((primitives::kw("that"), primitives::kw("this"))),
            alt((
                primitives::kw("permanent"),
                primitives::kw("artifact"),
                primitives::kw("creature"),
                primitives::kw("saga"),
            )),
        )
            .void(),
    ))
    .parse_next(input)
}

pub(crate) fn parse_put_or_remove_counter_shape(
    tokens: &[OwnedLexToken],
) -> Option<PutOrRemoveCounterShape<'_>> {
    let (or_idx, (), after_remove) =
        primitives::find_prefix(tokens, || primitives::phrase(&["or", "remove"]).void())?;
    let base_target_tokens = trim_shape_edges(&tokens[..or_idx]);
    if base_target_tokens.is_empty() {
        return None;
    }
    let remove_mode_tokens = trim_shape_edges(&tokens[or_idx + 1..]);
    let (remove_count, used) = values::parse_value_prefix_lexed(after_remove)?;
    let after_count = after_remove.get(used..)?;
    let (from_idx, (), remove_target_tokens) =
        primitives::find_prefix(after_count, || primitives::kw("from").void())?;
    let descriptor_tokens = trim_shape_edges(&after_count[..from_idx]);
    let remove_counter_type = if descriptor_tokens.is_empty() {
        None
    } else {
        if !has_word(descriptor_tokens, "counter") && !has_word(descriptor_tokens, "counters") {
            return None;
        }
        filters::parse_counter_type_from_tokens(descriptor_tokens)
    };
    primitives::parse_all(
        trim_shape_edges(remove_target_tokens),
        (referential_remove_target, primitives::sentence_end()).void(),
        "put or remove referential target",
    )
    .ok()?;
    Some(PutOrRemoveCounterShape {
        base_target_tokens,
        remove_count,
        remove_counter_type,
        remove_mode_tokens,
    })
}

fn consumed_prefix(tokens: &[OwnedLexToken], rest: &[OwnedLexToken]) -> usize {
    tokens.len().saturating_sub(rest.len())
}

pub(crate) fn parse_counter_target_count_shape(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    let tokens = trim_shape_edges(tokens);
    let mut rest = tokens;
    let mut consumed = 0usize;
    let each = if let Some(((), after_each)) = primitives::parse_prefix(
        rest,
        (primitives::kw("each"), opt(primitives::kw("of"))).void(),
    ) {
        consumed += consumed_prefix(rest, after_each);
        rest = after_each;
        true
    } else {
        false
    };
    if each {
        if let Some(((), after_x)) = primitives::parse_prefix(rest, primitives::kw("x").void())
            && primitives::parse_prefix(after_x, primitives::kw("target")).is_some()
        {
            return Some((
                ChoiceCount::dynamic_x(),
                consumed + consumed_prefix(rest, after_x),
            ));
        }
        if let Some(((), after_x)) =
            primitives::parse_prefix(rest, primitives::phrase(&["up", "to", "x"]).void())
            && primitives::parse_prefix(after_x, primitives::kw("target")).is_some()
        {
            return Some((
                ChoiceCount::up_to_dynamic_x(),
                consumed + consumed_prefix(rest, after_x),
            ));
        }
        if primitives::parse_prefix(rest, primitives::kw("target")).is_some() {
            return Some((ChoiceCount::any_number(), consumed));
        }
    }
    if let Some(((), after_any)) = primitives::parse_prefix(
        rest,
        (
            primitives::phrase(&["any", "number"]),
            opt(primitives::kw("of")),
        )
            .void(),
    ) {
        return Some((
            ChoiceCount::any_number(),
            consumed + consumed_prefix(rest, after_any),
        ));
    }
    if let Some((count, after_range)) =
        primitives::parse_prefix(rest, leaf::parse_leaf_target_count_range_prefix_lexed)
    {
        let mut after = after_range;
        if let Some(((), after_of)) = primitives::parse_prefix(after, primitives::kw("of").void()) {
            after = after_of;
        }
        return Some((count, consumed + consumed_prefix(rest, after)));
    }
    if let Some((count, after_count)) =
        primitives::parse_prefix(rest, leaf::parse_leaf_choice_count_prefix_lexed)
    {
        let mut after = after_count;
        if let Some(((), after_of)) = primitives::parse_prefix(after, primitives::kw("of").void()) {
            after = after_of;
        }
        return Some((count, consumed + consumed_prefix(rest, after)));
    }
    None
}

fn source_leaves_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        primitives::kw("until"),
        repeat_till::<_, _, (), _, _, _, _>(
            0..,
            any.void(),
            peek(primitives::phrase(&["leaves", "the", "battlefield"])),
        )
        .void(),
        primitives::phrase(&["leaves", "the", "battlefield"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn opponent_becomes_monarch_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        primitives::phrase(&["until", "an", "opponent", "becomes", "the", "monarch"]),
        primitives::sentence_end(),
    )
        .void()
        .parse_next(input)
}

fn target_leaves_suffix<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    primitives::kw("until").parse_next(input)?;
    let target = (
        primitives::kw("target"),
        repeat_till::<_, _, (), _, _, _, _>(
            0..,
            any.void(),
            peek(primitives::phrase(&["leaves", "the", "battlefield"])),
        )
        .void(),
    )
        .take()
        .parse_next(input)?;
    primitives::phrase(&["leaves", "the", "battlefield"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(target)
}

pub(crate) fn split_until_target_leaves_shape(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    primitives::split_lexed_once_before_suffix(tokens, 1, || target_leaves_suffix)
}

pub(crate) fn split_until_source_leaves_shape(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], bool) {
    primitives::split_lexed_once_before_suffix(tokens, 1, || source_leaves_suffix)
        .map(|(head, ())| (head, true))
        .unwrap_or((tokens, false))
}

pub(crate) fn split_until_opponent_becomes_monarch_shape(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    primitives::split_lexed_once_before_suffix(tokens, 1, || opponent_becomes_monarch_suffix)
        .map(|(head, ())| head)
}

fn half_starting_life_player<'a>(input: &mut LexStream<'a>) -> WResult<PlayerFilter> {
    alt((
        primitives::kw("your").value(PlayerFilter::You),
        (
            primitives::kw("target"),
            alt((
                possessive_surface_kw("player"),
                possessive_surface_kw("players"),
            )),
        )
            .value(PlayerFilter::target_player()),
        (
            primitives::kw("an"),
            alt((
                possessive_surface_kw("opponent"),
                possessive_surface_kw("opponents"),
            )),
        )
            .value(PlayerFilter::Opponent),
    ))
    .parse_next(input)
}

fn possessive_surface_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    any.verify(move |token: &&OwnedLexToken| {
        token.is_word(expected)
            || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
    })
    .void()
}

fn parse_half_starting_life<'a>(input: &mut LexStream<'a>) -> WResult<HalfStartingLifeShape> {
    primitives::kw("half").parse_next(input)?;
    let player = half_starting_life_player.parse_next(input)?;
    primitives::phrase(&["starting", "life", "total"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let rounding = opt((
        primitives::kw("rounded"),
        alt((
            primitives::kw("up").value(HalfStartingLifeRounding::Up),
            primitives::kw("down").value(HalfStartingLifeRounding::Down),
        )),
    ))
    .map(|rounding| {
        rounding
            .map(|(_, value)| value)
            .unwrap_or(HalfStartingLifeRounding::Up)
    })
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(HalfStartingLifeShape { player, rounding })
}

pub(crate) fn parse_half_starting_life_shape(
    tokens: &[OwnedLexToken],
) -> Option<HalfStartingLifeShape> {
    primitives::parse_all(
        trim_shape_edges(tokens),
        parse_half_starting_life,
        "half starting life total",
    )
    .ok()
}

fn exact_self_reference(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        trim_shape_edges(tokens),
        (
            alt((
                primitives::kw("it").void(),
                primitives::kw("this").void(),
                (
                    primitives::kw("this"),
                    alt((
                        primitives::kw("creature"),
                        primitives::kw("land"),
                        primitives::kw("permanent"),
                    )),
                )
                    .void(),
            )),
            primitives::sentence_end(),
        )
            .void(),
        "transform self reference",
    )
    .is_ok()
}

pub(crate) fn parse_transform_target_shape(tokens: &[OwnedLexToken]) -> TransformTargetShape<'_> {
    let tokens = trim_shape_edges(tokens);
    if tokens.is_empty() {
        return TransformTargetShape::ImplicitSource;
    }
    if let Some((_, filter_tokens)) =
        primitives::parse_prefix(tokens, alt((primitives::kw("all"), primitives::kw("each"))))
    {
        return TransformTargetShape::EachObject {
            filter_tokens: trim_shape_edges(filter_tokens),
        };
    }
    let words = primitives::TokenWordView::new(tokens).to_word_refs();
    if exact_self_reference(tokens) {
        return TransformTargetShape::Source {
            surface: this_source_surface_for_words(&words),
        };
    }
    if let Some(surface) =
        source_reference_surface_for_words(&words).or_else(|| this_source_surface_for_words(&words))
    {
        return TransformTargetShape::Source {
            surface: Some(surface),
        };
    }
    let fallback_to_source = words.len() <= 3
        && !words.iter().any(|word| {
            matches!(
                *word,
                "target" | "another" | "other" | "each" | "all" | "that" | "those"
            )
        });
    TransformTargetShape::Target {
        target_tokens: tokens,
        fallback_to_source,
    }
}

pub(crate) fn source_spec_for_reference(source: CounterReferenceSource) -> crate::ChooseSpec {
    match source {
        CounterReferenceSource::TaggedIt => crate::ChooseSpec::Tagged(crate::TagKey::from(IT_TAG)),
        CounterReferenceSource::Source => crate::ChooseSpec::Source,
    }
}

pub(crate) fn player_filter_for_half_reference(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Active => Some(PlayerFilter::Active),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::PlayerToYourLeft => Some(PlayerFilter::PlayerToYourLeft),
        PlayerAst::PlayerToYourRight => Some(PlayerFilter::PlayerToYourRight),
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
        PlayerAst::ThatPlayerOrTargetController
        | PlayerAst::ItsController
        | PlayerAst::ItsOwner
        | PlayerAst::Enchanted => None,
        PlayerAst::TriggeringSourceController => Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged("triggering_source"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn tokens(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn parses_dynamic_counts_as_typed_facts() {
        assert_eq!(
            parse_dynamic_counter_count_shape(&tokens("two life lost this way")),
            Some(DynamicCounterCountShape::LifeLostThisWay { group_size: 2 })
        );
        assert_eq!(
            parse_dynamic_counter_count_shape(&tokens("spells you cast this turn")),
            Some(DynamicCounterCountShape::SpellsCastThisTurn {
                player: PlayerFilter::You,
                other_than_first: false,
            })
        );
    }

    #[test]
    fn parses_target_counts_and_half_life() {
        let target_tokens = tokens("each of up to X target creatures");
        assert_eq!(
            parse_counter_target_count_shape(&target_tokens),
            Some((ChoiceCount::up_to_dynamic_x(), 5))
        );
        assert_eq!(
            parse_half_starting_life_shape(&tokens(
                "half target player's starting life total, rounded down"
            )),
            Some(HalfStartingLifeShape {
                player: PlayerFilter::target_player(),
                rounding: HalfStartingLifeRounding::Down,
            })
        );
    }

    #[test]
    fn parses_counter_surfaces() {
        assert!(is_named_source_power_shape(&tokens("Krenko's power")));
        assert!(is_him_or_her_counter_target(&tokens("him or her")));
        assert_eq!(
            parse_counter_count_prefix_shape(&tokens("this's counters on that creature")),
            CounterCountPrefixShape::Referential(ReferentialCounterCountShape {
                source: CounterReferenceSource::Source,
                counter_type: None,
                consumed: 2,
            })
        );
        crate::runtime_backend::front_end::shared::util::with_source_reference_context(
            "Counter Bear",
            || {
                assert_eq!(
                    parse_counter_count_prefix_shape(&tokens(
                        "Counter Bear's counters on that creature"
                    )),
                    CounterCountPrefixShape::Referential(ReferentialCounterCountShape {
                        source: CounterReferenceSource::Source,
                        counter_type: None,
                        consumed: 3,
                    })
                );
            },
        );
        let counter_tokens = tokens("+1/+1 counters on target creature equal to the difference");
        let shape = parse_put_counter_target_shape(&counter_tokens).unwrap();
        assert!(shape.equal_to_difference);

        let counter_tokens = tokens(
            "X +1/+1 counters on target creature you control, where X is the mana value of that card",
        );
        let shape = parse_put_counter_target_shape(&counter_tokens).unwrap();
        assert_eq!(
            primitives::TokenWordView::new(shape.target_tokens).to_word_refs(),
            ["target", "creature", "you", "control"]
        );
    }

    #[test]
    fn preserves_distinct_target_in_until_leaves_suffix() {
        let tokens = tokens(
            "target creature or enchantment you don't control until target enchantment you control leaves the battlefield",
        );
        let (exiled, watcher) =
            split_until_target_leaves_shape(&tokens).expect("target watcher suffix");

        assert_eq!(
            primitives::TokenWordView::new(exiled).to_word_refs(),
            [
                "target",
                "creature",
                "or",
                "enchantment",
                "you",
                "dont",
                "control"
            ]
        );
        assert_eq!(
            primitives::TokenWordView::new(watcher).to_word_refs(),
            ["target", "enchantment", "you", "control"]
        );
    }
}
