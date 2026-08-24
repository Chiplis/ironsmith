use super::*;

pub(super) fn counted_permanents_and_or_hand_cards(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("x").parse_next(input)?;
    alt((primitives::kw("permanent"), primitives::kw("permanents")))
        .void()
        .parse_next(input)?;
    permanent_controller.parse_next(input)?;
    alt((primitives::kw("control"), primitives::kw("controls")))
        .void()
        .parse_next(input)?;
    and_or.parse_next(input)?;
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)?;
    primitives::kw("from").parse_next(input)?;
    hand_owner.parse_next(input)?;
    primitives::kw("hand").parse_next(input)?;
    finish_non_words(input)
}
