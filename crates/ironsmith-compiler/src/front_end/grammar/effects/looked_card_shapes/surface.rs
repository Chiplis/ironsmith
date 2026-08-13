use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::front_end::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::util::parse_number;

use super::super::super::{permission_shapes, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CountedLookedCardsIntoHandShape {
    pub(crate) count: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LookedCardBattlefieldShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) tapped: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LookedCardBattlefieldAndHandShape<'a> {
    pub(crate) battlefield_filter_tokens: &'a [OwnedLexToken],
    pub(crate) tapped: bool,
    pub(crate) hand_filter_tokens: &'a [OwnedLexToken],
}

fn from_among_looked_cards(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["from", "among", "those", "cards"]),
        primitives::phrase(&["from", "among", "the", "cards", "milled", "this", "way"]),
        primitives::phrase(&["from", "among", "the", "milled", "cards"]),
        primitives::phrase(&["from", "among", "them"]),
    ))
    .void()
    .parse_next(input)
}

fn battlefield_destination(input: &mut LexStream<'_>) -> WResult<bool> {
    primitives::kw("onto").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("battlefield").parse_next(input)?;
    Ok(opt(primitives::kw("tapped")).parse_next(input)?.is_some())
}

fn end_of_tokens(input: &mut LexStream<'_>) -> WResult<()> {
    eof.void().parse_next(input)
}

pub(crate) fn parse_looked_card_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardBattlefieldShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (filter_end, _, destination_tokens) =
        primitives::find_prefix(tokens, || from_among_looked_cards)?;
    let filter_tokens = trim_lexed_commas(tokens.get(..filter_end)?);
    if filter_tokens.is_empty() {
        return None;
    }
    let mut destination = LexStream::new(trim_lexed_commas(destination_tokens));
    let tapped = battlefield_destination.parse_next(&mut destination).ok()?;
    end_of_tokens.parse_next(&mut destination).ok()?;
    Some(LookedCardBattlefieldShape {
        filter_tokens,
        tapped,
    })
}

pub(crate) fn parse_looked_card_battlefield_and_hand_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardBattlefieldAndHandShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (first_filter_end, _, after_first_reference) =
        primitives::find_prefix(tokens, || from_among_looked_cards)?;
    let battlefield_filter_tokens = trim_lexed_commas(tokens.get(..first_filter_end)?);
    if battlefield_filter_tokens.is_empty() {
        return None;
    }

    let mut destination = LexStream::new(trim_lexed_commas(after_first_reference));
    let tapped = battlefield_destination.parse_next(&mut destination).ok()?;
    primitives::kw("and").parse_next(&mut destination).ok()?;
    let second_start = after_first_reference
        .len()
        .saturating_sub(destination.len());
    let second_tokens = after_first_reference.get(second_start..)?;
    let (second_filter_end, _, hand_destination) =
        primitives::find_prefix(second_tokens, || from_among_looked_cards)?;
    let hand_filter_tokens = trim_lexed_commas(second_tokens.get(..second_filter_end)?);
    if hand_filter_tokens.is_empty() {
        return None;
    }
    primitives::parse_prefix(
        trim_lexed_commas(hand_destination),
        primitives::phrase(&["into", "your", "hand"]).void(),
    )?;
    Some(LookedCardBattlefieldAndHandShape {
        battlefield_filter_tokens,
        tapped,
        hand_filter_tokens,
    })
}

fn looked_cards_reference(input: &mut LexStream<'_>) -> WResult<()> {
    opt(primitives::kw("of")).parse_next(input)?;
    alt((
        primitives::kw("them").void(),
        (
            primitives::kw("those"),
            opt(alt((primitives::kw("card"), primitives::kw("cards")))),
        )
            .void(),
    ))
    .parse_next(input)
}

fn into_your_hand_tail(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["into", "your", "hand"])
        .void()
        .parse_next(input)?;
    opt(primitives::kw("instead")).parse_next(input)?;
    eof.void().parse_next(input)
}

pub(crate) fn parse_counted_looked_cards_into_hand_shape(
    tokens: &[OwnedLexToken],
) -> Option<CountedLookedCardsIntoHandShape> {
    let tokens = trim_lexed_commas(tokens);
    let (_, count_tokens) = primitives::parse_prefix(tokens, primitives::kw("put").void())?;
    let (count, used) = parse_number(count_tokens)?;
    let reference_tokens = trim_lexed_commas(count_tokens.get(used..)?);
    let (_, tail) = primitives::parse_prefix(reference_tokens, looked_cards_reference)?;
    let mut input = LexStream::new(trim_lexed_commas(tail));
    into_your_hand_tail.parse_next(&mut input).ok()?;
    Some(CountedLookedCardsIntoHandShape { count })
}

pub(crate) fn parse_kicked_counted_looked_cards_into_hand_shape(
    tokens: &[OwnedLexToken],
) -> Option<CountedLookedCardsIntoHandShape> {
    let (_, tail) = primitives::parse_prefix(
        trim_lexed_commas(tokens),
        primitives::phrase(&["if", "this", "spell", "was", "kicked"]).void(),
    )?;
    parse_counted_looked_cards_into_hand_shape(trim_lexed_commas(tail))
}

fn negative_put_from_among_head(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("if").parse_next(input)?;
    primitives::kw("you").parse_next(input)?;
    alt((
        primitives::kw("dont").void(),
        primitives::kw("don't").void(),
        primitives::phrase(&["do", "not"]).void(),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("put").parse_next(input)?;
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    )))
    .parse_next(input)?;
    primitives::phrase(&["card", "from", "among"])
        .void()
        .parse_next(input)?;
    alt((
        primitives::kw("them").void(),
        primitives::phrase(&["those", "cards"]).void(),
    ))
    .parse_next(input)?;
    primitives::phrase(&["into", "your", "hand"])
        .void()
        .parse_next(input)?;
    eof.void().parse_next(input)
}

pub(crate) fn is_if_you_dont_put_looked_card_into_hand(tokens: &[OwnedLexToken]) -> bool {
    negative_put_from_among_head
        .parse(LexStream::new(trim_lexed_commas(tokens)))
        .is_ok()
}

pub(crate) fn is_put_rest_on_library_bottom(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_lexed_commas(tokens);
    primitives::parse_prefix(
        tokens,
        alt((primitives::kw("put"), primitives::kw("puts"))).void(),
    )
    .is_some()
        && permission_shapes::contains_tokens(tokens, &["rest"])
        && permission_shapes::contains_tokens(tokens, &["bottom"])
        && permission_shapes::contains_tokens(tokens, &["library"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_counted_hand_and_bottom_surfaces() {
        assert_eq!(
            parse_counted_looked_cards_into_hand_shape(&lex(
                "Put two of those cards into your hand instead"
            ))
            .unwrap()
            .count,
            2
        );
        assert!(is_if_you_dont_put_looked_card_into_hand(&lex(
            "If you don't put a card from among them into your hand"
        )));
        assert!(is_put_rest_on_library_bottom(&lex(
            "Put the rest on the bottom of your library"
        )));

        let combined = lex(
            "a creature card from among those cards onto the battlefield tapped and a land card from among them into your hand",
        );
        let shape = parse_looked_card_battlefield_and_hand_shape(&combined).unwrap();
        assert!(shape.tapped);
        assert!(!shape.battlefield_filter_tokens.is_empty());
        assert!(!shape.hand_filter_tokens.is_empty());
    }
}
