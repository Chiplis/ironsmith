use winnow::combinator::alt;
use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::primitives;
use super::mana::parse_leaf_surface_mana_pip_lexed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafActivationCostHead {
    LoyaltyShorthand,
    Mana,
    Tap,
    Pay,
    Discard,
    Mill,
    Sacrifice,
    PutCounter,
    RemoveCounter,
    Exile,
    Return,
    Energy,
}

pub fn parse_leaf_activation_cost_head_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafActivationCostHead> {
    alt((
        parse_loyalty_shorthand_head,
        parse_leaf_surface_mana_pip_lexed.value(LeafActivationCostHead::Mana),
        parse_named_activation_cost_head,
    ))
    .context(StrContext::Label("activation-cost head"))
    .context(StrContext::Expected(StrContextValue::Description(
        "loyalty, mana, tap, payment, or object cost",
    )))
    .parse_next(input)
}

pub fn parse_leaf_activation_cost_head_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeafActivationCostHead> {
    primitives::parse_prefix(tokens, parse_leaf_activation_cost_head_lexed).map(|(head, _)| head)
}

fn parse_loyalty_shorthand_head<'a>(input: &mut LexStream<'a>) -> WResult<LeafActivationCostHead> {
    primitives::token_kind(TokenKind::LBracket).parse_next(input)?;
    let token: &OwnedLexToken = any.parse_next(input)?;
    let valid = matches!(
        token.kind,
        TokenKind::Plus | TokenKind::Dash | TokenKind::Number
    ) || token.as_word().is_some_and(|word| word == "0");
    if valid {
        Ok(LeafActivationCostHead::LoyaltyShorthand)
    } else {
        Err(primitives::backtrack_err(
            "loyalty cost",
            "signed or zero loyalty amount",
        ))
    }
}

fn parse_named_activation_cost_head<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafActivationCostHead> {
    let word = primitives::word_parser_text.parse_next(input)?;
    match word {
        "tap" | "t" => Ok(LeafActivationCostHead::Tap),
        "pay" => Ok(LeafActivationCostHead::Pay),
        "discard" => Ok(LeafActivationCostHead::Discard),
        "mill" => Ok(LeafActivationCostHead::Mill),
        "sacrifice" => Ok(LeafActivationCostHead::Sacrifice),
        "put" => Ok(LeafActivationCostHead::PutCounter),
        "remove" => Ok(LeafActivationCostHead::RemoveCounter),
        "exile" => Ok(LeafActivationCostHead::Exile),
        "return" => Ok(LeafActivationCostHead::Return),
        "e" => Ok(LeafActivationCostHead::Energy),
        _ => Err(primitives::backtrack_err(
            "activation cost",
            "known activation-cost verb or symbol",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn recognizes_typed_activation_heads() {
        for (raw, expected) in [
            ("[ + 1 ]", LeafActivationCostHead::LoyaltyShorthand),
            ("{2}", LeafActivationCostHead::Mana),
            ("Pay 2 life", LeafActivationCostHead::Pay),
            ("Sacrifice a creature", LeafActivationCostHead::Sacrifice),
            ("E", LeafActivationCostHead::Energy),
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            assert_eq!(
                parse_leaf_activation_cost_head_tokens(&tokens),
                Some(expected)
            );
        }
    }

    #[test]
    fn rejects_non_cost_sentence_heads() {
        let tokens = lex_line("Draw a card", 0).unwrap();
        assert_eq!(parse_leaf_activation_cost_head_tokens(&tokens), None);
    }
}
