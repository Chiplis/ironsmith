use winnow::combinator::alt;
use winnow::prelude::*;

use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, trim_lexed_commas};

#[derive(Clone, Copy, Debug)]
pub struct ForEachCommaShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
}

fn split_for_each_comma(
    tokens: &[OwnedLexToken],
    require_for: bool,
) -> Option<ForEachCommaShape<'_>> {
    if require_for {
        primitives::parse_prefix(tokens, primitives::phrase(&["for", "each"]).void())?;
    } else {
        primitives::parse_prefix(
            tokens,
            alt((
                primitives::phrase(&["for", "each"]).void(),
                primitives::kw("each").void(),
            )),
        )?;
    }
    let (subject_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    Some(ForEachCommaShape {
        subject_tokens: trim_lexed_commas(subject_tokens),
        effect_tokens: trim_lexed_commas(effect_tokens),
    })
}

pub fn parse_for_each_target_objects_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachCommaShape<'_>> {
    split_for_each_comma(tokens, false)
}

pub fn parse_for_each_this_way_shape(tokens: &[OwnedLexToken]) -> Option<ForEachCommaShape<'_>> {
    split_for_each_comma(tokens, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

    #[test]
    fn captures_subject_and_effect_around_comma() {
        let tokens = lex_line("For each creature exiled this way, draw a card", 0).unwrap();
        let shape = parse_for_each_this_way_shape(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(shape.effect_tokens).to_word_refs(),
            ["draw", "a", "card"]
        );
    }
}
