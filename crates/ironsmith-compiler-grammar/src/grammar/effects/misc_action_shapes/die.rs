use winnow::ascii::{Caseless, digit1};
use winnow::combinator::{eof, preceded};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{literal, take_till};

use crate::grammar::{leaf, primitives};
use crate::lexer::OwnedLexToken;
use crate::model::ast::{DieNoun, DieSurface};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollDieShape {
    pub sides: u32,
    pub surface: Option<DieSurface>,
    pub consumed: usize,
}

fn compact_die_size(input: &mut &str) -> WResult<u32> {
    preceded(
        Caseless("d"),
        digit1.try_map(|digits: &str| digits.parse::<u32>()),
    )
    .parse_next(input)
}

fn suffixed_die_size(input: &mut &str) -> WResult<u32> {
    let amount: &str = take_till(1.., '-').parse_next(input)?;
    literal("-sided").parse_next(input)?;
    eof.parse_next(input)?;
    leaf::parse_number_complete(amount)
        .map_err(|_| primitives::backtrack_err("die size", "number followed by -sided"))
}

fn parse_die_word(word: &str) -> Option<u32> {
    crate::grammar::primitives::probe_shape(compact_die_size.parse(word))
}

fn die_noun(token: &OwnedLexToken) -> Option<DieNoun> {
    if token.is_word("die") {
        Some(DieNoun::Die)
    } else if token.is_word("dice") {
        Some(DieNoun::Dice)
    } else {
        None
    }
}

pub fn parse_roll_die_tokens(tokens: &[OwnedLexToken]) -> Option<RollDieShape> {
    let shape = parse_roll_die_prefix_tokens(tokens)?;
    (shape.consumed == tokens.len()).then_some(shape)
}

pub fn parse_roll_die_prefix_tokens(tokens: &[OwnedLexToken]) -> Option<RollDieShape> {
    let article = usize::from(tokens.first().is_some_and(|token| token.is_word("a") || token.is_word("an")));
    let tokens = if tokens
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        &tokens[1..]
    } else {
        tokens
    };
    let first = tokens.first()?.parser_text().to_ascii_lowercase();
    if let Some(sides) = parse_die_word(&first) {
        return Some(RollDieShape {
            sides,
            surface: None,
            consumed: article + 1,
        });
    }
    if let Some(noun) = tokens.get(1).and_then(die_noun)
        && let Ok(sides) = suffixed_die_size.parse(first.as_str())
    {
        return Some(RollDieShape {
            sides,
            surface: Some(DieSurface::Sided(noun)),
            consumed: article + 2,
        });
    }
    if tokens.get(1).is_some_and(|token| token.is_word("sided"))
        && let Some(noun) = tokens.get(2).and_then(die_noun)
    {
        let sides = crate::grammar::primitives::probe_shape(leaf::parse_number_complete(&first))?;
        return Some(RollDieShape {
            sides,
            surface: Some(DieSurface::Sided(noun)),
            consumed: article + 3,
        });
    }
    None
}

#[cfg(test)]
mod complete_token_tests {
    use super::*;
    #[test]
    fn die_leaf_requires_complete_surface_and_prefix_reports_exact_consumption() {
        for text in ["a d20", "a six-sided die", "a six sided die"] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let parsed = parse_roll_die_tokens(&tokens).unwrap();
            assert_eq!(parsed.consumed, tokens.len());
        }
        for text in ["a d20 banana", "a six-sided die banana", "a six sided die banana", "a d20 and add three"] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse_roll_die_tokens(&tokens).is_none(), "{text}");
            let prefix = parse_roll_die_prefix_tokens(&tokens).unwrap();
            assert!(prefix.consumed < tokens.len());
        }
    }
}
