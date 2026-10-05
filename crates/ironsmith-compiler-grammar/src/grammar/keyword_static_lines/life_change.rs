use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;
use crate::lexer::{LexStream, OwnedLexToken};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GainLifeReplacementAmount { Add(i32), Double }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GainLifeReplacementShape<'a> {
    pub amount: GainLifeReplacementAmount,
    pub condition_tokens: Option<&'a [OwnedLexToken]>,
}

pub fn parse_gain_life_replacement_tokens(tokens: &[OwnedLexToken]) -> Option<GainLifeReplacementShape<'_>> {
    primitives::probe_all(tokens, parse_gain_life_replacement, "complete life-gain replacement")
}

fn parse_gain_life_replacement<'a>(input: &mut LexStream<'a>) -> WResult<GainLifeReplacementShape<'a>> {
    primitives::phrase(&["if", "you", "would", "gain", "life"]).parse_next(input)?;
    let condition_tokens = opt((primitives::kw("while"),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::comma()))
            .map(|((), _)| ()).take(),
    )).parse_next(input)?.map(|(_, tokens)| tokens);
    primitives::comma().parse_next(input)?;
    opt(primitives::kw("you")).parse_next(input)?;
    primitives::kw("gain").parse_next(input)?;
    let amount = alt((
        primitives::phrase(&["twice", "that", "much", "life"]).value(GainLifeReplacementAmount::Double),
        (primitives::phrase(&["that", "much", "life", "plus"]), leaf::parse_leaf_number_prefix_lexed)
            .try_map(|(_, amount)| i32::try_from(amount).map(GainLifeReplacementAmount::Add)),
    )).parse_next(input)?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(GainLifeReplacementShape { amount, condition_tokens })
}
