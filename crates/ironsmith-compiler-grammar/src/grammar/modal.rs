use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalPointHeaderShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalPointLabelShape {
    pub count: u32,
    pub delimiter_token: usize,
    pub body_first: usize,
}

pub fn parse_modal_point_header_tokens(tokens: &[OwnedLexToken]) -> Option<ModalPointHeaderShape> {
    if !tokens
        .iter()
        .any(|token| leaf::parse_leaf_pawprint_label_count_token(token).is_some())
    {
        return None;
    }
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        find_phrase_lexed(input, &["worth", "of", "modes"])
    })?;
    Some(ModalPointHeaderShape)
}

pub fn parse_modal_point_label_tokens(tokens: &[OwnedLexToken]) -> Option<ModalPointLabelShape> {
    let mut index = 0usize;
    if tokens
        .get(index)
        .is_some_and(|token| matches!(token.kind, TokenKind::Bullet | TokenKind::Dash))
    {
        index += 1;
    }

    let mut count = 0u32;
    while let Some(point_count) = tokens
        .get(index)
        .and_then(leaf::parse_leaf_pawprint_label_count_token)
    {
        count = count.checked_add(point_count)?;
        index += 1;
    }
    if count == 0
        || !tokens
            .get(index)
            .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
    {
        return None;
    }
    Some(ModalPointLabelShape {
        count,
        delimiter_token: index,
        body_first: index + 1,
    })
}

fn find_phrase_lexed<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
) -> WResult<()> {
    loop {
        let mut candidate = input.clone();
        if primitives::phrase(phrase)
            .parse_next(&mut candidate)
            .is_ok()
        {
            *input = candidate;
            return Ok(());
        }
        any.void().parse_next(input)?;
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn modal_point_header_and_labels_are_typed() {
        let header = lex_line("Choose up to five {P} worth of modes", 0).unwrap();
        assert_eq!(
            parse_modal_point_header_tokens(&header),
            Some(ModalPointHeaderShape)
        );

        let label = lex_line("{P}{P}{P} — Return target permanent", 0).unwrap();
        let parsed = parse_modal_point_label_tokens(&label).unwrap();
        assert_eq!(parsed.count, 3);
        assert_eq!(label[parsed.delimiter_token].kind, TokenKind::EmDash);
        assert_eq!(label[parsed.body_first].parser_text(), "return");
    }
}
