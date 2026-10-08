use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::{filters, leaf, primitives};
use crate::lexer::{LexStream, OwnedLexToken};
use crate::object::CounterType;
use crate::types::Subtype;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterLinkedLandSubtypeFollowupShape {
    pub subtype: Subtype,
    pub counter_type: CounterType,
    /// "in addition to its other types" keeps the land's other subtypes;
    /// without it the land subtype is set (CR 305.7).
    pub preserve_other_types: bool,
}

fn parse_counter_linked_land_subtype_followup_lexed(
    input: &mut LexStream<'_>,
) -> WResult<CounterLinkedLandSubtypeFollowupShape> {
    // "That land is ..." (Quicksilver Fountain) or the pronoun copula
    // "It's ..." (Eluge, the Shoreless Sea) naming the land just countered.
    alt((
        primitives::phrase(&["that", "land", "is"]),
        primitives::phrase(&["it", "is"]),
        primitives::phrase(&["it", "s"]),
        primitives::kw("it's").void(),
        primitives::kw("it’s").void(),
        primitives::kw("its").void(),
    ))
    .parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let subtype_word = primitives::word_parser_text.parse_next(input)?;
    let subtype = leaf::parse_leaf_subtype_flexible_complete(subtype_word)
        .map_err(|_| primitives::backtrack_err("counter-linked land type", "known subtype"))?;
    let preserve_other_types = opt(primitives::phrase(&[
        "in", "addition", "to", "its", "other", "types",
    ]))
    .parse_next(input)?
    .is_some();
    if !preserve_other_types && !subtype.is_basic_land_type() {
        return Err(primitives::backtrack_err(
            "counter-linked land type",
            "basic land type when setting the land's subtype",
        ));
    }
    primitives::phrase(&["for", "as", "long", "as", "it", "has"]).parse_next(input)?;
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
        preserve_other_types,
    })
}

pub fn parse_counter_linked_land_subtype_followup(
    tokens: &[OwnedLexToken],
) -> Option<CounterLinkedLandSubtypeFollowupShape> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_counter_linked_land_subtype_followup_lexed,
        "counter-linked land subtype followup",
    )
}
