//! "<payer> may pay any amount of mana." followed by the sentences that use
//! the amount paid. The payment owns its X (CR 107.3): the following
//! sentences read "the amount of mana that player/they paid this way" as X.
//!
//! - "That player may pay any amount of mana. This Aura deals 2 damage to
//!   that player. Prevent X of that damage, where X is the amount of mana that
//!   player paid this way." (Errant Minion, Power Leak): one payer, and the
//!   prevention covers only the damage the previous sentence deals.
//! - "Each player may pay any amount of mana. Then each player creates a
//!   number of ... tokens equal to the amount of mana they paid this way."
//!   (Liege of the Hollows): every player pays in APNAP order (CR 101.4),
//!   then each player's program reads that player's own payment.
use super::*;
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, TokenKind};
use crate::cards::builders::ForEachEffectAst;
use winnow::prelude::*;

#[derive(Clone, Copy)]
enum PaymentHead {
    /// "that player may pay any amount of mana"
    ThatPlayer,
    /// "each player may pay any amount of mana"
    EachPlayer,
}

fn payment_head(tokens: &[OwnedLexToken]) -> Option<PaymentHead> {
    primitives::probe_all(
        tokens,
        (
            winnow::combinator::alt((
                primitives::phrase(&["that", "player"]).value(PaymentHead::ThatPlayer),
                primitives::phrase(&["each", "player"]).value(PaymentHead::EachPlayer),
            )),
            primitives::phrase(&["may", "pay", "any", "amount", "of", "mana"]),
            primitives::sentence_end(),
        )
            .map(|(head, (), ())| head),
        "variable mana payment head",
    )
}

fn without_terminal_period(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    match tokens.split_last() {
        Some((last, rest)) if last.kind == TokenKind::Period => rest,
        _ => tokens,
    }
}

fn mentions_paid_this_way(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(&["paid", "this", "way"])).is_some()
}

/// "Prevent <amount> of that damage[, where X is <the amount paid>]."
fn prevent_portion_amount(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = without_terminal_period(tokens);
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("prevent"))?;
    let (of_idx, (), after) =
        primitives::find_prefix(rest, || primitives::phrase(&["of", "that", "damage"]))?;
    let amount_tokens = &rest[..of_idx];
    let (amount, used) = crate::util::parse_value(amount_tokens)?;
    if used != amount_tokens.len() {
        return None;
    }
    if after.is_empty() {
        return Some(amount);
    }
    let ((_, ()), binding_tokens) = primitives::parse_prefix(
        after,
        (primitives::comma(), primitives::phrase(&["where", "x", "is"])),
    )?;
    let (binding, used) = crate::util::parse_value(binding_tokens)?;
    // The binding names the payment's own X; anything else is a different
    // quantity this procedure does not own.
    (used == binding_tokens.len()
        && matches!(amount.unhinted(), Value::X)
        && matches!(binding.unhinted(), Value::X))
    .then_some(Value::X)
}

/// That player pays, then the damage and the prevention of part of it.
pub(super) fn read_single_payer_damage_portion(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(head), Some(damage), Some(prevention)) = (
        sentences.get(index),
        sentences.get(index + 1),
        sentences.get(index + 2),
    ) else {
        return Ok(None);
    };
    let Some(PaymentHead::ThatPlayer) = payment_head(head.lowered()) else {
        return Ok(None);
    };
    if !mentions_paid_this_way(prevention.lowered()) {
        return Ok(None);
    }
    let Some(amount) = prevent_portion_amount(prevention.lowered()) else {
        return Ok(None);
    };
    let damage_effects = crate::effect_sentences::parse_effect_sentence_lexed(damage.lowered())?;
    if damage_effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::CollectManaPayments {
        effects: vec![EffectAst::PreventDamagePortion {
            amount,
            effects: damage_effects,
        }],
        payers: Some(PlayerAst::That),
        x_colors: None,
        apnap_order: false,
        per_payer: false,
    }]))
}

/// Every player pays, then each player's own program reads their payment.
pub(super) fn read_each_payer_program(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(head), Some(body)) = (sentences.get(index), sentences.get(index + 1)) else {
        return Ok(None);
    };
    let Some(PaymentHead::EachPlayer) = payment_head(head.lowered()) else {
        return Ok(None);
    };
    let body = body.lowered();
    if !mentions_paid_this_way(body) {
        return Ok(None);
    }
    // "Then each player ...": the sequencing adverb belongs to this
    // procedure's ordering, which the payment scope already imposes.
    let body = primitives::parse_prefix(body, primitives::kw("then"))
        .map_or(body, |(_, rest)| rest);
    let body = crate::lexer::trim_lexed_commas(body);
    let parsed = crate::effect_sentences::parse_effect_sentence_lexed(body)?;
    let effects = match parsed.as_slice() {
        [EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })] => effects.clone(),
        [
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::Any,
                effects,
                ..
            }),
        ] => effects.clone(),
        _ => return Ok(None),
    };
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::CollectManaPayments {
        effects,
        payers: None,
        x_colors: None,
        apnap_order: true,
        per_payer: true,
    }]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexed(text: &str) -> Vec<OwnedLexToken> {
        crate::lexer::lex_line(text, 0).unwrap()
    }

    #[test]
    fn prevent_portion_binds_only_the_paid_amount() {
        assert_eq!(
            prevent_portion_amount(&lexed(
                "Prevent X of that damage, where X is the amount of mana that player paid this way."
            )),
            Some(Value::X)
        );
        assert_eq!(
            prevent_portion_amount(&lexed(
                "Prevent X of that damage, where X is the number of creatures you control."
            )),
            None
        );
    }

    #[test]
    fn payment_heads_name_their_payers() {
        assert!(matches!(
            payment_head(&lexed("That player may pay any amount of mana.")),
            Some(PaymentHead::ThatPlayer)
        ));
        assert!(matches!(
            payment_head(&lexed("Each player may pay any amount of mana.")),
            Some(PaymentHead::EachPlayer)
        ));
        assert!(payment_head(&lexed("Each player may pay any amount of life.")).is_none());
    }
}
