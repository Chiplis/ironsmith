use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EachOpponentExileChoiceShape {
    pub(crate) choice: Vec<OwnedLexToken>,
}

fn hand_owner(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("their").void(),
        primitives::phrase(&["that", "player"]).void(),
        primitives::phrase(&["that", "players"]).void(),
        primitives::phrase(&["that", "player's"]).void(),
    ))
    .parse_next(input)
}

fn permanent_controller(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("they").void(),
        primitives::phrase(&["that", "player"]).void(),
        primitives::phrase(&["that", "players"]).void(),
        primitives::phrase(&["that", "player's"]).void(),
    ))
    .parse_next(input)
}

fn finish_non_words(input: &mut LexStream<'_>) -> WResult<()> {
    while !input.is_empty() {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(input);
        let token = parsed?;
        if token.as_word().is_some() {
            return Err(primitives::backtrack_err(
                "exile hand-or-permanent choice",
                "end of choice",
            ));
        }
    }
    Ok(())
}

fn hand_or_permanent_choice(input: &mut LexStream<'_>) -> WResult<()> {
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::phrase(&["card", "from"]).parse_next(input)?;
    hand_owner.parse_next(input)?;
    primitives::phrase(&["hand", "or"]).parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::kw("permanent").parse_next(input)?;
    permanent_controller.parse_next(input)?;
    alt((primitives::kw("control"), primitives::kw("controls")))
        .void()
        .parse_next(input)?;
    finish_non_words(input)
}

fn each_opponent_exiles(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("each").parse_next(input)?;
    alt((primitives::kw("opponent"), primitives::kw("opponents")))
        .void()
        .parse_next(input)?;
    alt((primitives::kw("exile"), primitives::kw("exiles")))
        .void()
        .parse_next(input)
}

pub(crate) fn is_exile_hand_or_permanent_choice_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        hand_or_permanent_choice,
        "exile-hand-or-permanent-choice",
    )
    .is_ok()
}

pub(crate) fn parse_each_opponent_exile_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<EachOpponentExileChoiceShape> {
    let ((), choice) = primitives::parse_prefix(tokens, each_opponent_exiles)?;
    is_exile_hand_or_permanent_choice_shape(choice).then(|| EachOpponentExileChoiceShape {
        choice: choice.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_each_opponent_hand_or_permanent_choice() {
        let parsed = parse_each_opponent_exile_choice_shape(&lex(
            "Each opponent exiles a card from their hand or a permanent they control",
        ))
        .unwrap();
        assert!(is_exile_hand_or_permanent_choice_shape(&parsed.choice));
        assert!(is_exile_hand_or_permanent_choice_shape(&lex(
            "card from that player's hand or permanent that player controls"
        )));
    }
}
