//! Status scope, complete-cost substitution and per-ability usage are separate
//! readers. No custom mechanic name is part of their grammar.
use super::*;
use crate::grammar::primitives as p;
use winnow::combinator::{alt, eof, opt, repeat_till};
use winnow::prelude::*;
use winnow::token::any;
fn scope<'a>(
    input: &mut crate::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    p::kw("while").parse_next(input)?;
    let target = repeat_till(1.., any.void(), p::peek(p::phrase(&["has", "the"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    p::phrase(&["has", "the"]).parse_next(input)?;
    let status = repeat_till(1.., any.void(), p::peek(p::kw("status")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    p::kw("status").parse_next(input)?;
    p::comma().parse_next(input)?;
    Ok((target, status))
}
fn terms(
    input: &mut crate::lexer::LexStream<'_>,
) -> winnow::error::ModalResult<(crate::mana::ManaCost, bool, u32)> {
    p::phrase(&["you", "may", "pay"]).parse_next(input)?;
    let cost = crate::grammar::leaf::parse_leaf_fixed_mana_cost_prefix_lexed
        .parse_next(input)?
        .cost;
    p::phrase(&[
        "rather",
        "than",
        "pay",
        "the",
        "activation",
        "cost",
        "of",
        "each",
        "of",
        "its",
    ])
    .parse_next(input)?;
    let nonloyalty = opt(p::kw("nonloyalty")).parse_next(input)?.is_some();
    p::phrase(&["activated", "abilities"]).parse_next(input)?;
    let count = alt((
        p::kw("once").value(1),
        (
            crate::grammar::leaf::parse_leaf_number_prefix_lexed,
            p::kw("times"),
        )
            .map(|(n, _)| n),
    ))
    .parse_next(input)?;
    p::phrase(&["each", "turn"]).parse_next(input)?;
    opt(p::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok((cost, nonloyalty, count))
}
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((
        (target_tokens, status_tokens),
        (replacement_cost, nonloyalty_only, max_uses_per_ability_each_turn),
    )) = p::probe_all(
        tokens,
        (scope, terms),
        "status-scoped-activation-alternative",
    )
    else {
        return Ok(None);
    };
    if status_tokens
        .iter()
        .any(|token| token.kind != TokenKind::Word)
    {
        return Ok(None);
    }
    let status = crate::lexer::render_token_slice(status_tokens).to_ascii_lowercase();
    let target = parse_target_phrase(target_tokens)?;
    // The bounded status-name operand is data read by this grant. The
    // structural combinators and target reader own the surrounding words.
    crate::word_claims::claim_tokens(status_tokens);
    Ok(Some(vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::You,
        SubjectVerbActionAst::Grants(GrantActionAst::GrantActivationCostAlternative {
            target,
            status,
            replacement_cost,
            max_uses_per_ability_each_turn,
            nonloyalty_only,
        }),
    )]))
}
