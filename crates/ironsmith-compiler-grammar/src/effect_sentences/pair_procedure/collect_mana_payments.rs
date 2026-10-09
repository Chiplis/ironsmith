//! Bind the total of accepted contributions to the complete following sentence.
//! The payment protocol is shared; the ordinary grammar still owns every body.
use super::*;
use crate::lexer::{OwnedLexToken, TokenKind};
use crate::grammar::primitives;
use winnow::prelude::*;

fn payment_head(tokens: &[OwnedLexToken]) -> bool {
    let tokens = crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(tokens)
        .map_or(tokens, |label| label.body_tokens);
    // "Starting with you, each player may pay ..." and "each player starting
    // with you may pay ..." (Mana-Charged Dragon) are the same protocol.
    primitives::probe_all(tokens, (
        winnow::combinator::alt((
            (
                primitives::phrase(&["starting", "with", "you"]),
                primitives::comma(),
                primitives::phrase(&["each", "player"]),
            ).void(),
            primitives::phrase(&["each", "player", "starting", "with", "you"]).void(),
        )),
        primitives::phrase(&["may", "pay", "any", "amount", "of", "mana"]),
        primitives::sentence_end(),
    ).void(), "controller-first collective mana payment").is_some()
}

pub(super) fn read(sentences: &[SentenceInput], index: usize) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1))
    else { return Ok(None); };
    if !payment_head(first.lowered()) { return Ok(None); }
    let body = second.lowered();
    const TOTAL: &[&str] = &["where", "x", "is", "the", "total", "amount", "of", "mana", "paid", "this", "way"];
    let bindings = body.windows(TOTAL.len() + 1).enumerate().filter_map(|(index, tokens)| {
        (tokens[0].kind == TokenKind::Comma && tokens[1..].iter().zip(TOTAL)
            .all(|(token, word)| token.is_word(word))).then_some(index)
    }).collect::<Vec<_>>();
    let [binding] = bindings.as_slice() else { return Ok(None); };
    let end = binding + TOTAL.len() + 1;
    if !body.get(end).is_none_or(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period)) {
        return Ok(None);
    }
    let mut unbound_body = body[..*binding].to_vec();
    unbound_body.extend_from_slice(&body[end..]);
    // The binding belongs to this body only. Do not discard a trailing clause
    // (notably Collective Voyage's tapped entry and mandatory shuffle).
    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&unbound_body)?;
    if effects.is_empty() { return Ok(None); }
    Ok(Some(vec![EffectAst::CollectManaPayments { effects }]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(head: &str, body: &str) -> Result<Option<Vec<EffectAst>>, CardTextError> {
        let first = crate::lexer::lex_line(head, 0).unwrap();
        let second = crate::lexer::lex_line(body, 0).unwrap();
        read(&[SentenceInput::from_lexed(&first), SentenceInput::from_lexed(&second)], 0)
    }
    #[test]
    fn all_four_complete_bodies_share_one_typed_payment_scope() {
        for body in [
            "Each player creates X 1/1 white Soldier creature tokens, where X is the total amount of mana paid this way.",
            "Each player searches their library for up to X basic land cards, where X is the total amount of mana paid this way, puts them onto the battlefield tapped, then shuffles.",
            "Each player draws X cards, where X is the total amount of mana paid this way.",
            "Each player mills X cards, where X is the total amount of mana paid this way.",
        ] {
            let effects = parse("Starting with you, each player may pay any amount of mana.", body).unwrap().unwrap();
            assert!(matches!(effects.as_slice(), [EffectAst::CollectManaPayments { effects }] if !effects.is_empty()));
        }
    }
    #[test]
    fn incomplete_or_different_resource_bindings_are_not_collective_mana() {
        let head = "Starting with you, each player may pay any amount of mana.";
        for body in [
            "Each player draws X cards.",
            "Each player draws X cards, where X is the amount of mana paid this way.",
            "Each player draws X cards, where X is the total amount of life paid this way.",
            "Each player draws X cards, where X is the total amount of mana paid this way {G}.",
        ] { assert!(parse(head, body).unwrap().is_none()); }
        assert!(parse("Starting with you, each player may pay any amount of life.",
            "Each player draws X cards, where X is the total amount of mana paid this way.").unwrap().is_none());
    }
}
