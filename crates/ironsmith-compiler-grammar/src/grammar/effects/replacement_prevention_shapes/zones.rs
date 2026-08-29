use super::super::*;

use crate::grammar::leaf;
use winnow::combinator::{alt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAllVerbShape {
    Destroy,
    Exile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAllConnectiveShape {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitAllShape<'a> {
    pub verb: SplitAllVerbShape,
    pub connective: SplitAllConnectiveShape,
    pub body_tokens: &'a [OwnedLexToken],
    pub filter_tokens: Vec<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExileReturnSameShape<'a> {
    pub exile_tokens: &'a [OwnedLexToken],
    pub return_tokens: &'a [OwnedLexToken],
    pub counter_tokens: Option<&'a [OwnedLexToken]>,
    pub delayed_until_end_of_combat: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileReturnReferenceShape {
    It,
    ThatCard,
    Them,
    ThoseCards,
}

pub fn parse_exile_return_reference_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExileReturnReferenceShape> {
    let (_, tail) = primitives::parse_prefix(tokens, primitives::kw("return"))?;
    primitives::parse_prefix(
        tail,
        alt((
            primitives::kw("it").value(ExileReturnReferenceShape::It),
            primitives::phrase(&["that", "card"]).value(ExileReturnReferenceShape::ThatCard),
            primitives::kw("them").value(ExileReturnReferenceShape::Them),
            primitives::phrase(&["those", "cards"]).value(ExileReturnReferenceShape::ThoseCards),
        )),
    )
    .map(|(shape, _)| shape)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExileEachTargetTypeShape<'a> {
    pub filter_tokens: Vec<&'a [OwnedLexToken]>,
}

fn marker_anywhere<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

fn list_segments(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    for and_segment in primitives::split_lexed_slices_on_list_conjunction(tokens) {
        for comma_segment in primitives::split_lexed_slices_on_comma(and_segment) {
            let segment = trim_lexed_commas(comma_segment);
            if !segment.is_empty() {
                segments.push(segment);
            }
        }
    }
    segments
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZoneTailShape {
    Hand,
    Graveyard,
}

fn zone_tail<'a>(input: &mut LexStream<'a>) -> WResult<ZoneTailShape> {
    let (_, zone) = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        alt((
            alt((primitives::kw("hand"), primitives::kw("hands"))).value(ZoneTailShape::Hand),
            alt((primitives::kw("graveyard"), primitives::kw("graveyards")))
                .value(ZoneTailShape::Graveyard),
        )),
    )
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(zone)
}

fn is_multi_zone_card_exile(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, zones)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["exile", "all", "cards", "from"]),
    ) else {
        return false;
    };
    let Some((first, second)) =
        primitives::split_lexed_once_on_separator(zones, || primitives::kw("and").void())
    else {
        return false;
    };
    let Ok(first) = primitives::parse_all(first, zone_tail, "first exile zone") else {
        return false;
    };
    let Ok(second) = primitives::parse_all(second, zone_tail, "second exile zone") else {
        return false;
    };
    matches!(
        (first, second),
        (ZoneTailShape::Hand, ZoneTailShape::Graveyard)
            | (ZoneTailShape::Graveyard, ZoneTailShape::Hand)
    )
}

fn is_temporary_exile(tokens: &[OwnedLexToken]) -> bool {
    let Some((before, ())) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        (
            primitives::phrase(&["leaves", "the", "battlefield"]),
            primitives::sentence_end(),
        )
            .void()
    }) else {
        return false;
    };
    marker_anywhere(before, primitives::kw("until"))
}

pub fn parse_split_all_shape(tokens: &[OwnedLexToken]) -> Option<SplitAllShape<'_>> {
    let (verb, body) = if let Some((_, rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["destroy", "all"]))
    {
        (SplitAllVerbShape::Destroy, rest)
    } else {
        let (_, rest) = primitives::parse_prefix(tokens, primitives::phrase(&["exile", "all"]))?;
        (SplitAllVerbShape::Exile, rest)
    };
    let connective =
        if primitives::split_lexed_once_on_separator(body, || primitives::kw("and").void())
            .is_some()
        {
            SplitAllConnectiveShape::And
        } else {
            let disjuncts = primitives::split_lexed_slices_on_or(body);
            if disjuncts.len() < 2
                || disjuncts.iter().skip(1).any(|segment| {
                    primitives::parse_prefix(trim_lexed_commas(segment), primitives::kw("all"))
                        .is_none()
                })
            {
                return None;
            }
            SplitAllConnectiveShape::Or
        };
    if marker_anywhere(tokens, primitives::kw("except"))
        || (verb == SplitAllVerbShape::Exile && is_temporary_exile(tokens))
        || is_multi_zone_card_exile(tokens)
    {
        return None;
    }
    let segments = match connective {
        SplitAllConnectiveShape::And => list_segments(body),
        SplitAllConnectiveShape::Or => primitives::split_lexed_slices_on_or(body),
    };
    let filter_tokens = segments
        .into_iter()
        .filter_map(|segment| {
            let segment = primitives::parse_prefix(segment, primitives::kw("all"))
                .map(|(_, rest)| rest)
                .unwrap_or(segment);
            let segment = trim_lexed_commas(segment);
            (!segment.is_empty()).then_some(segment)
        })
        .collect::<Vec<_>>();
    (filter_tokens.len() >= 2).then_some(SplitAllShape {
        verb,
        connective,
        body_tokens: body,
        filter_tokens,
    })
}

fn strip_end_combat(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    let tokens = trim_lexed_commas(tokens);
    for suffix in [
        &["at", "the", "end", "of", "combat"][..],
        &["at", "end", "of", "combat"][..],
    ] {
        if let Some((before, ())) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
            (dynamic_phrase(suffix), primitives::sentence_end()).void()
        }) {
            return (trim_lexed_commas(before), true);
        }
    }
    (trim_lexed_commas(tokens), false)
}

fn dynamic_phrase<'a, 'p>(
    words: &'p [&'p str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut LexStream<'a>| {
        for word in words {
            let expected = *word;
            any.verify(move |token: &&OwnedLexToken| token.is_word(expected))
                .void()
                .parse_next(input)?;
        }
        Ok(())
    }
}

fn counter_followup<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    primitives::kw("with").parse_next(input)?;
    let counter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            primitives::kw("on"),
            alt((primitives::kw("it"), primitives::kw("them"))),
        )),
    )
    .take()
    .parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    alt((primitives::kw("it"), primitives::kw("them"))).parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any.void()).parse_next(input)?;
    Ok(trim_lexed_commas(counter_tokens))
}

pub fn parse_exile_return_same_shape(tokens: &[OwnedLexToken]) -> Option<ExileReturnSameShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let tokens = primitives::parse_prefix(tokens, primitives::phrase(&["you", "may"]))
        .map(|(_, rest)| rest)
        .or_else(|| primitives::parse_prefix(tokens, primitives::kw("you")).map(|(_, rest)| rest))
        .unwrap_or(tokens);
    let (exile_tokens, return_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("then").void())?;
    primitives::parse_prefix(exile_tokens, primitives::kw("exile"))?;
    primitives::parse_prefix(return_tokens, primitives::kw("return"))?;
    let (exile_tokens, delayed_until_end_of_combat) = strip_end_combat(exile_tokens);
    let return_tokens = trim_lexed_commas(return_tokens);
    let counter_tokens =
        primitives::split_lexed_once_before_suffix(return_tokens, 1, || counter_followup)
            .map(|(_, counter_tokens)| counter_tokens)
            .filter(|counter_tokens| !counter_tokens.is_empty());
    Some(ExileReturnSameShape {
        exile_tokens,
        return_tokens,
        counter_tokens,
        delayed_until_end_of_combat,
    })
}

fn target_filter_segment(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let choice = leaf::parse_leaf_choice_count_prefix_tokens(tokens)?;
    if choice.count != ChoiceCount::up_to(1) {
        return None;
    }
    let (_, filter_tokens) =
        primitives::parse_prefix(tokens.get(choice.consumed..)?, primitives::kw("target"))?;
    let filter_tokens = trim_lexed_commas(filter_tokens);
    (!filter_tokens.is_empty()).then_some(filter_tokens)
}

pub fn parse_exile_each_target_type_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExileEachTargetTypeShape<'_>> {
    let (_, body) = primitives::parse_prefix(tokens, primitives::kw("exile"))?;
    let mut filter_tokens = Vec::new();
    for segment in list_segments(body) {
        for disjunct in primitives::split_lexed_slices_on_or(segment) {
            filter_tokens.push(target_filter_segment(trim_lexed_commas(disjunct))?);
        }
    }
    (filter_tokens.len() >= 2).then_some(ExileEachTargetTypeShape { filter_tokens })
}

#[cfg(test)]
#[path = "zones_inline_tests.rs"]
mod tests;
