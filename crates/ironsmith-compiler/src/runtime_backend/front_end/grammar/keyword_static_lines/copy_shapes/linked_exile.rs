use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::Zone;
use crate::filter::ObjectFilter;
use crate::object::CounterType;

use super::super::super::super::lexer::{LexStream, trim_lexed_commas};
use super::super::super::{filters, leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkedExileCopyCounterValue {
    OtherCardPower,
    OtherCardToughness,
    OtherCardManaValue,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LinkedExilePairCopyShape {
    pub(crate) may: bool,
    pub(crate) exile_count: u32,
    pub(crate) copy_count: u32,
    pub(crate) filter: ObjectFilter,
    pub(crate) counter_type: CounterType,
    pub(crate) counter_value: LinkedExileCopyCounterValue,
}

fn graveyard_zone<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    alt((
        primitives::kw("graveyard").void(),
        primitives::kw("graveyards").void(),
        (primitives::kw("all"), primitives::kw("graveyards")).void(),
    ))
    .value(Zone::Graveyard)
    .parse_next(input)
}

fn counter_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("counter"), primitives::kw("counters")))
        .void()
        .parse_next(input)
}

fn other_card_counter_value<'a>(input: &mut LexStream<'a>) -> WResult<LinkedExileCopyCounterValue> {
    let value = alt((
        primitives::kw("power").value(LinkedExileCopyCounterValue::OtherCardPower),
        primitives::kw("toughness").value(LinkedExileCopyCounterValue::OtherCardToughness),
        primitives::phrase(&["mana", "value"])
            .value(LinkedExileCopyCounterValue::OtherCardManaValue),
    ))
    .parse_next(input)?;
    primitives::phrase(&["of", "the", "other", "card"]).parse_next(input)?;
    Ok(value)
}

pub(super) fn parse_linked_exile_pair_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LinkedExilePairCopyShape> {
    primitives::kw("as").parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("enter"), primitives::kw("enters")))),
    )
    .void()
    .parse_next(input)?;
    alt((primitives::kw("enter"), primitives::kw("enters"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;

    let may = opt(primitives::phrase(&["you", "may"]))
        .map(|parsed| parsed.is_some())
        .parse_next(input)?;
    primitives::kw("exile").parse_next(input)?;
    let exile_count = leaf::parse_leaf_count_token.parse_next(input)?;
    let filter_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("from")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::kw("from").parse_next(input)?;
    let zone = graveyard_zone.parse_next(input)?;
    primitives::end_of_sentence().parse_next(input)?;

    primitives::phrase(&["if", "you", "do"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "enters", "as", "a", "copy", "of"]).parse_next(input)?;
    let copy_count = leaf::parse_leaf_count_token.parse_next(input)?;
    primitives::phrase(&[
        "of",
        "those",
        "cards",
        "with",
        "a",
        "number",
        "of",
        "additional",
    ])
    .parse_next(input)?;
    let counter_tokens = (
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(counter_noun)).void(),
        counter_noun,
    )
        .take()
        .parse_next(input)?;
    primitives::phrase(&["on", "it", "equal", "to", "the"]).parse_next(input)?;
    let counter_value = other_card_counter_value.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let mut filter =
        filters::parse_simple_object_filter_lexed(trim_lexed_commas(filter_tokens), false)
            .ok_or_else(|| {
                primitives::backtrack_err("linked exile filter", "object-card filter")
            })?;
    filter.zone = Some(zone);
    filter.nontoken = true;
    let counter_type = filters::parse_counter_type_from_tokens(counter_tokens)
        .ok_or_else(|| primitives::backtrack_err("linked exile counter", "counter type"))?;

    Ok(LinkedExilePairCopyShape {
        may,
        exile_count,
        copy_count,
        filter,
        counter_type,
        counter_value,
    })
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::lexer::lex_line;
    use crate::{CardType, Zone};

    use super::*;

    const MIMEOPLASM_TEXT: &str = "As The Mimeoplasm enters, you may exile two creature cards from graveyards. If you do, it enters as a copy of one of those cards with a number of additional +1/+1 counters on it equal to the power of the other card.";

    #[test]
    fn parses_linked_exile_copy_replacement_into_typed_fields() {
        let tokens = lex_line(MIMEOPLASM_TEXT, 0).expect("linked replacement should lex");
        let parsed = primitives::parse_all(
            &tokens,
            parse_linked_exile_pair_lexed,
            "linked exile copy test",
        )
        .expect("linked replacement should parse");

        assert!(parsed.may);
        assert_eq!(parsed.exile_count, 2);
        assert_eq!(parsed.copy_count, 1);
        assert_eq!(parsed.filter.zone, Some(Zone::Graveyard));
        assert!(parsed.filter.nontoken);
        assert_eq!(parsed.filter.card_types, vec![CardType::Creature]);
        assert_eq!(parsed.counter_type, CounterType::PlusOnePlusOne);
        assert_eq!(
            parsed.counter_value,
            LinkedExileCopyCounterValue::OtherCardPower
        );
    }

    #[test]
    fn recognizes_compound_shape_before_static_sentence_splitting() {
        let tokens = lex_line(MIMEOPLASM_TEXT, 0).expect("linked replacement should lex");
        let parsed =
            crate::runtime_backend::families::keyword_static::parse_static_ability_ast_line_lexed(
                &tokens,
            )
            .expect("static parser should accept linked replacement")
            .expect("static parser should return a replacement ability");
        let debug = format!("{parsed:#?}");
        assert!(
            debug.contains("linked_exile_pair"),
            "unexpected AST: {debug}"
        );
        assert!(debug.contains("PlusOnePlusOne"), "unexpected AST: {debug}");
    }
}
