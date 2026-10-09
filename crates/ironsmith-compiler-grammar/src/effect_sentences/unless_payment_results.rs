//! "..., then sacrifice this enchantment unless you pay {G} for each wind
//! counter on it. If you pay, this enchantment deals damage ..." (Cyclone).
//!
//! CR 118.12: "unless [a player pays]" offers that player the payment; if it
//! is paid the punished action doesn't happen, and otherwise it does. The
//! engine's `UnlessPaysEffect` reports that exactly: a paid cost is a
//! `Declined` outcome (the punished effects were prevented), and an unpaid
//! one is the punished effects' own outcome. So "If you pay" after "unless
//! you pay" is the `WasDeclined` result of the unless-wrapper, not `Did`
//! (which would read the punished action having happened, the opposite).
//! "If you pay" after an optional "you may pay" is an ordinary accepted
//! payment and stays with the result readers.

use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, IfResultPredicate, PlayerAst,
};
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};
use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

/// "If you pay," / "If you paid,".
fn if_you_pay_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you"]).parse_next(input)?;
    alt((primitives::kw("pay"), primitives::kw("paid"))).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    Ok(())
}

/// Bind "If you pay, ..." to an immediately preceding "unless you pay".
pub(super) fn try_bind_unless_payment_result(
    effects: &mut Vec<EffectAst>,
    sentence_tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    let Some(((), rest)) = primitives::parse_prefix(sentence_tokens, if_you_pay_head) else {
        return Ok(false);
    };
    if rest.is_empty() {
        return Ok(false);
    }
    if !matches!(
        effects.last(),
        Some(EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
            player: PlayerAst::You | PlayerAst::Implicit,
            ..
        }))
    ) {
        return Ok(false);
    }
    let paid_effects = super::parse_effect_sentences_lexed(rest)?;
    if paid_effects.is_empty() {
        return Ok(false);
    }
    effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::WasDeclined,
        effects: paid_effects,
    }));
    Ok(true)
}
