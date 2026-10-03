//! Blocking capacity clauses, independent of individual blocking restrictions.
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};
use winnow::Parser;
use winnow::combinator::{opt, peek, repeat_till};
use winnow::error::ModalResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy)]
pub struct CanBlockAnyNumber<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub this_turn: bool,
}

pub fn parse_can_block_any_number(tokens: &[OwnedLexToken]) -> Option<CanBlockAnyNumber<'_>> {
    primitives::probe_all(tokens, parse_lexed, "can block any number")
}

fn parse_lexed<'a>(input: &mut LexStream<'a>) -> ModalResult<CanBlockAnyNumber<'a>> {
    let subject_tokens = repeat_till(0.., any.void(), peek(primitives::phrase(&["can", "block"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    // A capacity leaf must not consume an earlier instruction or a compound
    // grant. Shared-subject sequencing owns those and supplies a clean clause.
    if subject_tokens.iter().any(|token| {
        token.is_period()
            || token.is_comma()
            || token.is_any_word(&["gets", "get", "has", "have", "gains", "gain", "and"])
    }) {
        return Err(primitives::backtrack_err(
            "can block any number",
            "clean subject",
        ));
    }
    primitives::phrase(&["can", "block", "any", "number", "of", "creatures"]).parse_next(input)?;
    let this_turn = opt(primitives::phrase(&["this", "turn"]))
        .parse_next(input)?
        .is_some();
    opt(primitives::sentence_end()).parse_next(input)?;
    Ok(CanBlockAnyNumber {
        subject_tokens,
        this_turn,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn unlimited_blocking_retains_scope_and_duration() {
        for (text, temporary) in [
            ("This creature can block any number of creatures.", false),
            (
                "Enchanted creature can block any number of creatures.",
                false,
            ),
            (
                "Target creature can block any number of creatures this turn.",
                true,
            ),
            ("can block any number of creatures this turn", true),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let shape = parse_can_block_any_number(&tokens).expect(text);
            assert_eq!(shape.this_turn, temporary);
        }
        for text in [
            "Target creature gets +2/+6 until end of turn and can block any number of creatures this turn.",
            "This creature can block any number of creatures with flying.",
            "This creature can block any number of creatures as long as you're the monarch.",
            "Target creature can block any number of creatures until your next turn.",
        ] {
            assert!(
                parse_can_block_any_number(&lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }
}
