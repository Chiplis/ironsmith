//! "<payer> may pay any amount of mana." followed by the sentences that use
//! the amount paid. The payment ({X}, CR 107.3) publishes its accepted amount
//! as its result, and "the amount of mana that player/they paid this way"
//! reads that result through the ordinary this-way binding.
//!
//! - "That player may pay any amount of mana. This Aura deals 2 damage to
//!   that player. Prevent X of that damage, where X is the amount of mana that
//!   player paid this way." (Errant Minion, Power Leak): the prevention is the
//!   shared next-time shield with an exact-amount portion, created before the
//!   damage it applies to (CR 615.7).
//! - "Each player may pay any amount of mana. Then each player creates a
//!   number of ... tokens equal to the amount of mana they paid this way."
//!   (Liege of the Hollows): one player loop owns both instructions, so every
//!   player pays in APNAP order (CR 101.4) before the tokens are created
//!   together, each player's count reading their own payment.
use super::*;
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, TokenKind};
use crate::cards::builders::{
    DamageActionAst, ForEachEffectAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, SubjectVerbActionAst, TargetAst,
};
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

/// "Prevent X of that damage, where X is <the amount paid this way>." The
/// binding must read the preceding payment's result.
fn prevent_portion_amount(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = without_terminal_period(tokens);
    let (_, rest) = primitives::parse_prefix(tokens, primitives::kw("prevent"))?;
    let (of_idx, (), after) =
        primitives::find_prefix(rest, || primitives::phrase(&["of", "that", "damage"]))?;
    let amount_tokens = &rest[..of_idx];
    let (amount, used) = crate::util::parse_value(amount_tokens)?;
    if used != amount_tokens.len() || !matches!(amount.unhinted(), Value::X) {
        return None;
    }
    let ((_, ()), binding_tokens) = primitives::parse_prefix(
        after,
        (primitives::comma(), primitives::phrase(&["where", "x", "is"])),
    )?;
    let (binding, used) = crate::util::parse_value(binding_tokens)?;
    (used == binding_tokens.len()
        && matches!(
            binding.unhinted(),
            Value::EventValue(crate::effect::EventValueSpec::Amount)
        ))
    .then_some(binding)
}

/// The one damage instruction of a sentence, with its recipient.
fn single_damage_recipient(effects: &[EffectAst]) -> Option<&TargetAst> {
    match effects {
        [EffectAst::SourceSentence { effects, .. }] | [EffectAst::Sequence { effects }] => {
            single_damage_recipient(effects)
        }
        [EffectAst::SubjectVerb(subject_verb)] => match &subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                target,
                unpreventable: false,
                ..
            }) => Some(target),
            _ => None,
        },
        _ => None,
    }
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
    let Some(amount) = prevent_portion_amount(prevention.lowered()) else {
        return Ok(None);
    };
    // "This Aura deals N damage to <recipient>": the shield covers the
    // source's next damage to that recipient.
    if primitives::parse_prefix(damage.lowered(), primitives::kw("this")).is_none() {
        return Ok(None);
    }
    let payment = crate::effect_sentences::parse_effect_sentence_lexed(head.lowered())?;
    let damage_effects = crate::effect_sentences::parse_effect_sentence_lexed(damage.lowered())?;
    let Some(recipient) = single_damage_recipient(&damage_effects).cloned() else {
        return Ok(None);
    };
    if payment.is_empty() {
        return Ok(None);
    }
    let shield = EffectAst::subject_verb_prevent_next_time_damage_portion(
        PreventNextTimeDamageSourceAst::Target(TargetAst::Source(None)),
        PreventNextTimeDamageTargetAst::Target(recipient),
        ironsmith_core::NextTimeDamagePreventionPortion::Exactly(amount),
        false,
    );
    let mut effects = payment;
    effects.push(shield);
    effects.extend(damage_effects);
    Ok(Some(effects))
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
    let payment = crate::effect_sentences::parse_effect_sentence_lexed(head.lowered())?;
    let consequence = crate::effect_sentences::parse_effect_sentence_lexed(body)?;
    let Some(mut payment_loop) = each_player_loop(payment) else {
        return Ok(None);
    };
    let Some(consequence) = each_player_loop(consequence) else {
        return Ok(None);
    };
    let (EffectAst::ForEach(
        ForEachEffectAst::ForEachPlayer { effects }
        | ForEachEffectAst::ForEachPlayersFiltered { effects, .. },
    ),
    EffectAst::ForEach(
        ForEachEffectAst::ForEachPlayer {
            effects: consequence,
        }
        | ForEachEffectAst::ForEachPlayersFiltered {
            effects: consequence,
            ..
        },
    )) = (&mut payment_loop, consequence)
    else {
        return Ok(None);
    };
    if effects.is_empty() || consequence.is_empty() {
        return Ok(None);
    }
    effects.extend(consequence);
    Ok(Some(vec![payment_loop]))
}

/// A sentence that is exactly one loop over every player.
fn each_player_loop(mut effects: Vec<EffectAst>) -> Option<EffectAst> {
    if effects.len() != 1 {
        return None;
    }
    let effect = effects.pop()?;
    match effect {
        EffectAst::SourceSentence { effects, .. } => each_player_loop(effects),
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { .. }) => Some(effect),
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::Any,
            ..
        }) => Some(effect),
        _ => None,
    }
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
            Some(Value::EventValue(crate::effect::EventValueSpec::Amount))
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
