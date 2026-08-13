use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::{EntersWithCounterConditionShape, LexStream, primitives};

pub(super) fn parse_enters_with_counter_condition_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    alt((
        parse_fixed_counter_condition_shape,
        parse_x_value_condition_shape,
        parse_cast_spells_condition_shape,
        parse_colors_mana_spent_condition_shape,
    ))
    .parse_next(input)
}

fn parse_fixed_counter_condition_shape<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    alt((
        (
            alt((
                primitives::phrase(&["you", "attacked", "this", "turn"]),
                primitives::phrase(&["youve", "attacked", "this", "turn"]),
                primitives::phrase(&["you've", "attacked", "this", "turn"]),
                primitives::phrase(&["you", "ve", "attacked", "this", "turn"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::AttackedThisTurn),
        (
            alt((
                primitives::phrase(&["you", "cast", "it"]),
                primitives::phrase(&["you", "cast", "this"]),
                primitives::phrase(&["you", "cast", "this", "spell"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::SourceWasCast),
        (
            alt((
                primitives::phrase(&["this", "spell", "was", "kicked"]),
                primitives::phrase(&["this", "creature", "was", "kicked"]),
                primitives::phrase(&["this", "permanent", "was", "kicked"]),
                primitives::phrase(&["it", "was", "kicked"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::ThisSpellWasKicked),
        (
            alt((
                primitives::phrase(&["this", "spell", "escaped"]),
                primitives::phrase(&["it", "escaped"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::ThisSpellEscaped),
        (
            alt((
                primitives::phrase(&["a", "creature", "died", "this", "turn"]),
                primitives::phrase(&["one", "or", "more", "creatures", "died", "this", "turn"]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::CreatureDiedThisTurn),
        (
            alt((
                primitives::phrase(&["an", "opponent", "lost", "life", "this", "turn"]),
                primitives::phrase(&[
                    "one",
                    "or",
                    "more",
                    "opponents",
                    "lost",
                    "life",
                    "this",
                    "turn",
                ]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::OpponentLostLifeThisTurn),
        (
            alt((
                primitives::phrase(&[
                    "a",
                    "permanent",
                    "left",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ]),
                primitives::phrase(&[
                    "one",
                    "or",
                    "more",
                    "permanents",
                    "left",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::PermanentLeftUnderYourControl),
        (
            alt((
                primitives::phrase(&[
                    "it", "wasnt", "cast", "or", "no", "mana", "was", "spent", "to", "cast", "it",
                ]),
                primitives::phrase(&[
                    "it", "wasn't", "cast", "or", "no", "mana", "was", "spent", "to", "cast", "it",
                ]),
            )),
            primitives::sentence_end(),
        )
            .value(EntersWithCounterConditionShape::NotCastOrNoManaSpent),
    ))
    .parse_next(input)
}

fn parse_x_value_condition_shape<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    primitives::phrase(&["x", "is"]).parse_next(input)?;
    let amount = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EntersWithCounterConditionShape::XValueAtLeast(amount))
}

fn parse_cast_spells_condition_shape<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    alt((
        primitives::phrase(&["youve", "cast"]),
        primitives::phrase(&["you've", "cast"]),
        primitives::phrase(&["you", "ve", "cast"]),
        primitives::phrase(&["you", "have", "cast"]),
        primitives::phrase(&["you", "cast"]),
    ))
    .parse_next(input)?;
    let amount = repeat_till(
        1..,
        any.void(),
        peek((
            alt((primitives::kw("spell"), primitives::kw("spells"))),
            primitives::phrase(&["this", "turn"]),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("spell"), primitives::kw("spells"))).parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EntersWithCounterConditionShape::YouCastSpellsThisTurn(
        amount,
    ))
}

fn parse_colors_mana_spent_condition_shape<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EntersWithCounterConditionShape<'a>> {
    let amount = repeat_till(
        1..,
        any.void(),
        peek((
            alt((
                primitives::kw("color"),
                primitives::kw("colors"),
                primitives::kw("colour"),
                primitives::kw("colours"),
            )),
            primitives::phrase(&["of", "mana"]),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((
        primitives::kw("color"),
        primitives::kw("colors"),
        primitives::kw("colour"),
        primitives::kw("colours"),
    ))
    .parse_next(input)?;
    primitives::phrase(&["of", "mana"]).parse_next(input)?;
    repeat_till(
        0..,
        any.void(),
        peek(primitives::phrase(&["spent", "to", "cast"])),
    )
    .map(|((), ())| ())
    .parse_next(input)?;
    primitives::phrase(&["spent", "to", "cast"]).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::kw("this").void(),
        primitives::phrase(&["this", "spell"]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(EntersWithCounterConditionShape::ColorsOfManaSpent(amount))
}
