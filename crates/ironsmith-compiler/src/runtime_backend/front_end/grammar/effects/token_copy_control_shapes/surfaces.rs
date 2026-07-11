use super::super::*;

use winnow::combinator::{alt, opt, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EachPlayerRevealPermanentsShape {
    pub(crate) count: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommaThenTailShape {
    ThatPlayer,
    ReturnSourceToHand,
    PutSourceOnLibrary,
    ChooseCardName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommaThenSpecialShape<'a> {
    pub(crate) head_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: &'a [OwnedLexToken],
    pub(crate) tail: CommaThenTailShape,
}

fn puts_revealed_permanents<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["puts", "all", "permanent", "cards"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["revealed", "this", "way"]),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["onto", "the", "battlefield"]),
    )
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(())
}

fn strip_leading_connectors(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(
        tokens,
        repeat::<_, _, (), _, _>(
            0..,
            alt((primitives::kw("then"), primitives::kw("and"))).void(),
        ),
    )
    .map(|(_, rest)| trim_lexed_commas(rest))
    .unwrap_or(tokens)
}

pub(crate) fn parse_each_player_reveal_permanents_shape(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerRevealPermanentsShape> {
    let segments = primitives::split_lexed_slices_on_comma(tokens);
    let [reveal_tokens, put_tokens, rest_tokens] = segments.as_slice() else {
        return None;
    };
    let (_, count_tokens) = primitives::parse_prefix(
        trim_lexed_commas(reveal_tokens),
        primitives::phrase(&[
            "each", "player", "reveals", "a", "number", "of", "cards", "from", "the", "top", "of",
            "their", "library", "equal", "to",
        ]),
    )?;
    let count_tokens = trim_lexed_commas(count_tokens);
    if count_tokens.is_empty() {
        return None;
    }
    let (_, count_filter_tokens) = primitives::parse_prefix(
        count_tokens,
        alt((
            primitives::phrase(&["the", "number", "of"]),
            primitives::phrase(&["number", "of"]),
        )),
    )?;
    let count_filter = parse_object_filter_lexed(count_filter_tokens, false).ok()?;
    primitives::parse_all(
        trim_lexed_commas(put_tokens),
        puts_revealed_permanents,
        "put revealed permanents",
    )
    .ok()?;
    primitives::parse_all(
        strip_leading_connectors(rest_tokens),
        (
            primitives::phrase(&["puts", "the", "rest", "into", "their", "graveyard"]),
            primitives::sentence_end(),
        )
            .void(),
        "put reveal rest into graveyard",
    )
    .ok()?;
    Some(EachPlayerRevealPermanentsShape {
        count: Value::Count(count_filter),
    })
}

fn return_source_to_hand<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("return"), primitives::kw("returns"))).parse_next(input)?;
    primitives::kw("this").parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), primitives::kw("to")).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        alt((primitives::kw("hand"), primitives::kw("hands"))),
    )
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(())
}

fn put_source_on_library<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("put"), primitives::kw("puts"))).parse_next(input)?;
    primitives::kw("this").parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        primitives::any_phrase(&[
            &["on", "top"],
            &["on", "the", "top"],
            &["third", "from", "top"],
            &["third", "from", "the", "top"],
        ]),
    )
    .parse_next(input)?;
    opt(primitives::kw("of")).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        alt((primitives::kw("library"), primitives::kw("libraries"))),
    )
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(())
}

pub(crate) fn parse_comma_then_special_shape(
    tokens: &[OwnedLexToken],
) -> Option<CommaThenSpecialShape<'_>> {
    let (head_tokens, tail_tokens) = primitives::split_lexed_once_on_separator(tokens, || {
        (primitives::comma(), primitives::kw("then")).void()
    })?;
    let head_tokens = trim_lexed_commas(head_tokens);
    let tail_tokens = trim_lexed_commas(tail_tokens);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return None;
    }

    let tail = if primitives::parse_prefix(tail_tokens, primitives::phrase(&["that", "player"]))
        .is_some()
    {
        CommaThenTailShape::ThatPlayer
    } else if primitives::parse_all(
        tail_tokens,
        return_source_to_hand,
        "return source to owner hand",
    )
    .is_ok()
        && primitives::parse_prefix(
            head_tokens,
            alt((primitives::kw("tap"), primitives::kw("untap"))).void(),
        )
        .is_some()
    {
        CommaThenTailShape::ReturnSourceToHand
    } else if primitives::parse_all(
        tail_tokens,
        put_source_on_library,
        "put source on owner library",
    )
    .is_ok()
        && primitives::parse_prefix(head_tokens, primitives::kw("draw").void()).is_some()
    {
        CommaThenTailShape::PutSourceOnLibrary
    } else if primitives::parse_prefix(
        tail_tokens,
        alt((
            primitives::phrase(&["choose", "any", "card", "name"]),
            primitives::phrase(&["choose", "a", "card", "name"]),
        )),
    )
    .is_some()
        && primitives::parse_prefix(head_tokens, primitives::kw("look").void()).is_some()
    {
        CommaThenTailShape::ChooseCardName
    } else {
        return None;
    };

    Some(CommaThenSpecialShape {
        head_tokens,
        tail_tokens,
        tail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_each_player_reveal_surface() {
        let tokens = lex_line(
            "Each player reveals a number of cards from the top of their library equal to the number of nonland permanents they control, puts all permanent cards they revealed this way onto the battlefield, and puts the rest into their graveyard.",
            0,
        )
        .unwrap();
        assert!(parse_each_player_reveal_permanents_shape(&tokens).is_some());
    }

    #[test]
    fn parses_comma_then_special_surfaces() {
        let tokens = lex_line(
            "tap target creature, then return this artifact to its owners hand",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_comma_then_special_shape(&tokens).map(|shape| shape.tail),
            Some(CommaThenTailShape::ReturnSourceToHand)
        ));
    }
}
