use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use crate::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, Copy)]
pub(crate) struct RegistryWordSplit<'a> {
    pub(crate) before: &'a [OwnedLexToken],
    pub(crate) after: &'a [OwnedLexToken],
}

pub(crate) fn split_registry_clause_at_word(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<RegistryWordSplit<'_>> {
    let mut input = LexStream::new(tokens);
    let mut consumed_words = 0usize;
    while consumed_words < word_index {
        let token: &OwnedLexToken = any::<_, ErrMode<ContextError>>
            .parse_next(&mut input)
            .ok()?;
        if token.as_word().is_some() {
            consumed_words += 1;
        }
    }
    let token_index = tokens.len().checked_sub(input.len())?;
    Some(RegistryWordSplit {
        before: tokens.get(..token_index)?,
        after: tokens.get(token_index..)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

    #[test]
    fn parses_word_boundaries() {
        let tokens = lex_line("two cards, then draw", 0).unwrap();
        let split = split_registry_clause_at_word(&tokens, 2).unwrap();
        assert_eq!(
            TokenWordView::new(split.before).to_word_refs(),
            vec!["two", "cards"]
        );
    }
}
