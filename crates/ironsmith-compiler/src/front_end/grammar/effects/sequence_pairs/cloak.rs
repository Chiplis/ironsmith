use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::PlayerAst;
use crate::effect::Value;
use crate::grammar::{leaf, primitives};
use crate::front_end::lexer::{LexStream, OwnedLexToken, TokenWordView};

use super::super::parse_exile_library_owner_shape;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CloakPileSequenceShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) library_count: Value,
    pub(crate) library_owner: PlayerAst,
    pub(crate) enters_tapped: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct CloakPileExileShape<'a> {
    target_tokens: &'a [OwnedLexToken],
    library_count: Value,
    library_owner: PlayerAst,
}

fn card_noun(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)
}

fn face_down(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("face-down").void(),
        primitives::kw("facedown").void(),
        primitives::phrase(&["face", "down"]),
    ))
    .parse_next(input)
}

fn pile_intro(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("in").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("the")))).parse_next(input)?;
    face_down.parse_next(input)?;
    primitives::kw("pile").parse_next(input)?;
    Ok(())
}

fn parse_cloak_pile_exile<'a>(input: &mut LexStream<'a>) -> WResult<CloakPileExileShape<'a>> {
    primitives::kw("exile").parse_next(input)?;
    let target_tokens = repeat_till(
        1..,
        any.void(),
        peek((
            primitives::kw("and"),
            opt(primitives::kw("the")),
            primitives::kw("top"),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("top").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    card_noun.parse_next(input)?;
    primitives::kw("of").parse_next(input)?;
    let owner_tokens = repeat_till(1.., any.void(), peek(pile_intro))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let owner = parse_exile_library_owner_shape(owner_tokens, PlayerAst::Implicit)
        .filter(|owner| owner.consumed_words == TokenWordView::new(owner_tokens).len())
        .ok_or_else(|| primitives::backtrack_err("cloak pile owner", "library owner"))?;
    pile_intro.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["shuffle", "that", "pile"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["then", "cloak", "those", "cards"]).parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;

    let library_count = i32::try_from(count)
        .map(Value::Fixed)
        .map_err(|_| primitives::backtrack_err("cloak pile count", "signed card count"))?;
    Ok(CloakPileExileShape {
        target_tokens,
        library_count,
        library_owner: owner.player,
    })
}

fn parse_cloak_entry(input: &mut LexStream<'_>) -> WResult<bool> {
    alt((
        primitives::phrase(&["they", "enter"]),
        primitives::phrase(&["those", "cards", "enter"]),
    ))
    .parse_next(input)?;
    opt(primitives::phrase(&["the", "battlefield"])).parse_next(input)?;
    primitives::kw("tapped").parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(true)
}

pub(crate) fn parse_cloak_pile_sequence_shape<'a>(
    exile: &'a [OwnedLexToken],
    entry: &[OwnedLexToken],
) -> Option<CloakPileSequenceShape<'a>> {
    let exile = primitives::parse_all(exile, parse_cloak_pile_exile, "cloak-pile-exile").ok()?;
    let enters_tapped = primitives::parse_all(entry, parse_cloak_entry, "cloak-pile-entry").ok()?;
    Some(CloakPileSequenceShape {
        target_tokens: exile.target_tokens,
        library_count: exile.library_count,
        library_owner: exile.library_owner,
        enters_tapped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_typed_cloak_pile_sequence() {
        let exile = lex_line(
            "Exile target nontoken creature you own and the top two cards of your library in a face-down pile, shuffle that pile, then cloak those cards.",
            0,
        )
        .unwrap();
        let entry = lex_line("They enter tapped.", 0).unwrap();
        let shape = parse_cloak_pile_sequence_shape(&exile, &entry).unwrap();

        assert_eq!(shape.library_count, Value::Fixed(2));
        assert_eq!(shape.library_owner, PlayerAst::You);
        assert!(shape.enters_tapped);
        assert_eq!(
            TokenWordView::new(shape.target_tokens).word_refs(),
            vec!["target", "nontoken", "creature", "you", "own"]
        );
    }
}
