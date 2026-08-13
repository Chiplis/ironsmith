use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::object::CounterType;
use crate::grammar::{filters, leaf, primitives};
use crate::front_end::lexer::{LexStream, OwnedLexToken};
use crate::types::Subtype;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CounterLinkedLandSubtypeFollowupShape {
    pub(crate) subtype: Subtype,
    pub(crate) counter_type: CounterType,
}

fn parse_counter_linked_land_subtype_followup_lexed(
    input: &mut LexStream<'_>,
) -> WResult<CounterLinkedLandSubtypeFollowupShape> {
    primitives::phrase(&["that", "land", "is"]).parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let subtype_word = primitives::word_parser_text.parse_next(input)?;
    let subtype = leaf::parse_leaf_subtype_flexible_complete(subtype_word)
        .map_err(|_| primitives::backtrack_err("counter-linked land type", "known subtype"))?;
    primitives::phrase(&[
        "in", "addition", "to", "its", "other", "types", "for", "as", "long", "as", "it", "has",
    ])
    .parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let counter_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::phrase(&["on", "it"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let counter_type =
        filters::parse_counter_type_from_tokens(counter_tokens).ok_or_else(|| {
            primitives::backtrack_err("counter-linked land type", "known counter type")
        })?;
    Ok(CounterLinkedLandSubtypeFollowupShape {
        subtype,
        counter_type,
    })
}

pub(crate) fn parse_counter_linked_land_subtype_followup(
    tokens: &[OwnedLexToken],
) -> Option<CounterLinkedLandSubtypeFollowupShape> {
    primitives::parse_all(
        tokens,
        parse_counter_linked_land_subtype_followup_lexed,
        "counter-linked land subtype followup",
    )
    .ok()
}
