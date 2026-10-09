//! "Rather than pay {2} for each previous time you've cast this spell from
//! the command zone this game, pay 2 life that many times." (Liesa, Shroud of
//! Dusk): the commander tax (CR 903.8) is paid in life instead of mana.
use super::*;

use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;
use crate::lexer::{LexStream, TokenKind};

fn commander_tax_life_shape(input: &mut LexStream<'_>) -> winnow::error::ModalResult<u32> {
    primitives::phrase(&["rather", "than", "pay"]).parse_next(input)?;
    // The commander tax is always {2} per previous cast (CR 903.8).
    any.verify(|token: &&OwnedLexToken| token.kind == TokenKind::ManaGroup && token.slice == "{2}")
        .parse_next(input)?;
    primitives::phrase(&[
        "for", "each", "previous", "time", "you've", "cast", "this", "spell", "from", "the",
        "command", "zone", "this", "game",
    ])
    .parse_next(input)?;
    winnow::combinator::opt(primitives::comma()).parse_next(input)?;
    primitives::kw("pay").parse_next(input)?;
    let amount = any
        .verify_map(|token: &OwnedLexToken| {
            (token.kind == TokenKind::Number)
                .then(|| token.parser_text().parse::<u32>().ok())
                .flatten()
        })
        .parse_next(input)?;
    primitives::phrase(&["life", "that", "many", "times"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(amount)
}

pub(super) fn parse_commander_tax_life_line(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    let amount = commander_tax_life_shape.parse(LexStream::new(tokens)).ok()?;
    (amount > 0).then(|| StaticAbility::commander_tax_life_substitution(amount))
}
