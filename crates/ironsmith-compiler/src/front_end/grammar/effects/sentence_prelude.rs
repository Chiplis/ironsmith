use winnow::combinator::{alt, eof};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{OwnedLexToken, TokenWordView};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SentencePreludeShape {
    XCantBeZero,
    RollDiceChooseOneResult {
        count: u32,
        sides: u32,
        die_text: String,
    },
}

pub fn parse_sentence_prelude_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SentencePreludeShape> {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    alt((parse_x_cant_be_zero, parse_roll_dice_choose_one_result))
        .parse_next(&mut input)
        .ok()
}

fn parse_x_cant_be_zero(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SentencePreludeShape> {
    primitives::word_slice_exact("x").parse_next(input)?;
    alt((
        primitives::word_slice_exact("cant"),
        primitives::word_slice_exact("can't"),
    ))
    .parse_next(input)?;
    primitives::word_slice_exact("be").parse_next(input)?;
    primitives::word_slice_exact("0").parse_next(input)?;
    eof.parse_next(input)?;
    Ok(SentencePreludeShape::XCantBeZero)
}

fn parse_roll_dice_choose_one_result(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SentencePreludeShape> {
    primitives::word_slice_exact("roll").parse_next(input)?;
    let count_word: &str = any.parse_next(input)?;
    let (count, consumed) = leaf::parse_leaf_number_prefix_words(&[count_word])
        .and_then(leaf::LeafNumberPrefix::into_fixed)
        .ok_or_else(|| primitives::backtrack_err("roll count", "fixed number"))?;
    if consumed != 1 {
        return Err(primitives::backtrack_err("roll count", "one word"));
    }
    let die_text: &str = any.parse_next(input)?;
    let sides = leaf::parse_leaf_die_sides_complete(die_text)
        .map_err(|_| primitives::backtrack_err("die notation", "d followed by sides"))?;
    for word in ["and", "choose", "one", "result"] {
        primitives::word_slice_exact(word).parse_next(input)?;
    }
    eof.parse_next(input)?;
    Ok(SentencePreludeShape::RollDiceChooseOneResult {
        count,
        sides,
        die_text: die_text.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn sentence_prelude_shapes_are_typed() {
        let x = lex_line("X can't be 0", 0).unwrap();
        assert_eq!(
            parse_sentence_prelude_shape_tokens(&x),
            Some(SentencePreludeShape::XCantBeZero)
        );

        let roll = lex_line("Roll two d20 and choose one result", 0).unwrap();
        assert_eq!(
            parse_sentence_prelude_shape_tokens(&roll),
            Some(SentencePreludeShape::RollDiceChooseOneResult {
                count: 2,
                sides: 20,
                die_text: "d20".to_string(),
            })
        );
    }
}
