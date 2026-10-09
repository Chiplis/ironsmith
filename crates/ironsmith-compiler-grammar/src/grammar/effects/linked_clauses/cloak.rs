use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::PlayerAst;
use crate::effect::Value;
use crate::grammar::{leaf, primitives};
use crate::lexer::{LexStream, OwnedLexToken, TokenWordView};

use super::super::parse_exile_library_owner_shape;

#[derive(Debug, Clone, PartialEq)]
pub struct CloakPileSequenceShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub library_count: Value,
    pub library_owner: PlayerAst,
    pub enters_tapped: bool,
    /// "then manifest those cards" (CR 701.40a) rather than cloak
    /// (CR 701.58a): the cards enter face down without ward.
    pub manifest: bool,
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

/// "exile <target> and the top N cards of <library> in a face-down pile"
fn parse_face_down_pile_exile_prefix<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CloakPileExileShape<'a>> {
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
    // "the top card of your library" names exactly one card.
    let count = alt((
        (leaf::parse_leaf_number_prefix_lexed, card_noun).map(|(count, ())| count),
        primitives::kw("card").value(1u32),
    ))
    .parse_next(input)?;
    primitives::kw("of").parse_next(input)?;
    let owner_tokens = repeat_till(1.., any.void(), peek(pile_intro))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let owner = parse_exile_library_owner_shape(owner_tokens, PlayerAst::Implicit)
        .filter(|owner| owner.consumed_words == TokenWordView::new(owner_tokens).len())
        .ok_or_else(|| primitives::backtrack_err("cloak pile owner", "library owner"))?;
    pile_intro.parse_next(input)?;
    let library_count = i32::try_from(count)
        .map(Value::Fixed)
        .map_err(|_| primitives::backtrack_err("cloak pile count", "signed card count"))?;
    Ok(CloakPileExileShape {
        target_tokens,
        library_count,
        library_owner: owner.player,
    })
}

/// "..., shuffle that pile, then cloak/manifest those cards": true when the
/// pile is manifested.
fn parse_cloak_pile_exile<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(CloakPileExileShape<'a>, bool)> {
    let shape = parse_face_down_pile_exile_prefix.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["shuffle", "that", "pile"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::kw("then").parse_next(input)?;
    let manifest = alt((
        primitives::kw("cloak").value(false),
        primitives::kw("manifest").value(true),
    ))
    .parse_next(input)?;
    primitives::phrase(&["those", "cards"]).parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok((shape, manifest))
}

fn parse_standalone_pile_exile<'a>(input: &mut LexStream<'a>) -> WResult<CloakPileExileShape<'a>> {
    let shape = parse_face_down_pile_exile_prefix.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(shape)
}

/// "If you do, shuffle that pile and put it back on top of your library."
fn parse_pile_restack(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["if", "you", "do"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "shuffle", "that", "pile", "and", "put", "it", "back", "on", "top", "of", "your", "library",
    ])
    .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaceDownPileRestackShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub library_count: Value,
    pub library_owner: PlayerAst,
}

pub fn parse_face_down_pile_restack_shape<'a>(
    exile: &'a [OwnedLexToken],
    followup: &[OwnedLexToken],
) -> Option<FaceDownPileRestackShape<'a>> {
    let exile = crate::grammar::primitives::probe_all(
        exile,
        parse_standalone_pile_exile,
        "face-down-pile-exile",
    )?;
    crate::grammar::primitives::probe_all(followup, parse_pile_restack, "face-down-pile-restack")?;
    Some(FaceDownPileRestackShape {
        target_tokens: exile.target_tokens,
        library_count: exile.library_count,
        library_owner: exile.library_owner,
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

pub fn parse_cloak_pile_sequence_shape<'a>(
    exile: &'a [OwnedLexToken],
    entry: &[OwnedLexToken],
) -> Option<CloakPileSequenceShape<'a>> {
    let (exile, manifest) =
        crate::grammar::primitives::probe_all(exile, parse_cloak_pile_exile, "cloak-pile-exile")?;
    let enters_tapped =
        crate::grammar::primitives::probe_all(entry, parse_cloak_entry, "cloak-pile-entry")?;
    Some(CloakPileSequenceShape {
        target_tokens: exile.target_tokens,
        library_count: exile.library_count,
        library_owner: exile.library_owner,
        enters_tapped,
        manifest,
    })
}

/// The face-down pile sequence as one complete sentence, with no entry
/// sentence after it: "exile it and the top card of your library in a
/// face-down pile, shuffle that pile, then manifest those cards."
pub fn parse_face_down_pile_sentence_shape(
    tokens: &[OwnedLexToken],
) -> Option<CloakPileSequenceShape<'_>> {
    let (exile, manifest) =
        crate::grammar::primitives::probe_all(tokens, parse_cloak_pile_exile, "face-down-pile")?;
    Some(CloakPileSequenceShape {
        target_tokens: exile.target_tokens,
        library_count: exile.library_count,
        library_owner: exile.library_owner,
        enters_tapped: false,
        manifest,
    })
}

/// What happens to the looked-at cards a face-down selection leaves behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookedFaceDownRemainder {
    /// "then put the other on the top or bottom of your library"
    TopOrBottom,
    /// "and put the rest on the bottom of your library in <order>"
    Bottom(crate::cards::builders::LibraryBottomOrderAst),
}

/// "Manifest one of those cards, then put the other on the top or bottom of
/// your library." / "Cloak two of them and put the rest on the bottom of
/// your library in a random order.": a selection out of a looked-at group
/// that enters face down (CR 701.40a manifest, CR 701.58a cloak), and the
/// disposition of the rest.
#[derive(Debug, Clone, PartialEq)]
pub struct LookedFaceDownSelectionShape {
    pub manifest: bool,
    pub count: crate::cards::builders::ChoiceCount,
    pub remainder: LookedFaceDownRemainder,
}

fn looked_face_down_remainder(input: &mut LexStream<'_>) -> WResult<LookedFaceDownRemainder> {
    use crate::cards::builders::LibraryBottomOrderAst;
    primitives::kw("on").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    alt((
        (
            primitives::phrase(&["top", "or", "bottom"]),
            opt(primitives::phrase(&["of", "your", "library"])),
        )
            .value(LookedFaceDownRemainder::TopOrBottom),
        (
            primitives::kw("bottom"),
            opt(primitives::phrase(&["of", "your", "library"])),
            alt((
                primitives::phrase(&["in", "any", "order"])
                    .value(LibraryBottomOrderAst::ChooserChooses),
                primitives::phrase(&["in", "a", "random", "order"])
                    .value(LibraryBottomOrderAst::Random),
            )),
        )
            .map(|(_, _, order)| LookedFaceDownRemainder::Bottom(order)),
    ))
    .parse_next(input)
}

fn looked_face_down_selection(input: &mut LexStream<'_>) -> WResult<LookedFaceDownSelectionShape> {
    let manifest = alt((
        primitives::kw("cloak").value(false),
        primitives::kw("manifest").value(true),
    ))
    .parse_next(input)?;
    let count = leaf::parse_leaf_choice_count_prefix_lexed.parse_next(input)?;
    alt((
        primitives::phrase(&["of", "them"]),
        primitives::phrase(&["of", "those", "cards"]),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    alt((primitives::kw("then"), primitives::kw("and"))).parse_next(input)?;
    primitives::kw("put").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    alt((primitives::kw("other"), primitives::kw("rest"))).parse_next(input)?;
    let remainder = looked_face_down_remainder.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(LookedFaceDownSelectionShape {
        manifest,
        count,
        remainder,
    })
}

pub fn parse_looked_face_down_selection_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedFaceDownSelectionShape> {
    crate::grammar::primitives::probe_all(
        tokens,
        looked_face_down_selection,
        "looked-face-down-selection",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

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
        assert!(!shape.manifest);
        assert_eq!(
            TokenWordView::new(shape.target_tokens).word_refs(),
            vec!["target", "nontoken", "creature", "you", "own"]
        );
    }
}
