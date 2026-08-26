use super::super::*;

use winnow::combinator::{alt, opt, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommaThenTailShape {
    ThatPlayer,
    ReturnSourceToHand,
    PutSourceOnLibrary,
    ChooseCardName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommaThenSpecialShape<'a> {
    pub head_tokens: &'a [OwnedLexToken],
    pub tail_tokens: &'a [OwnedLexToken],
    pub tail: CommaThenTailShape,
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

pub fn parse_comma_then_special_shape(
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
