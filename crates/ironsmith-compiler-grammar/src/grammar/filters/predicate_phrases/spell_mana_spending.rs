//! A turn-history predicate about actual spell payments, independent of any
//! keyword's identity, threshold event, or mana value.
use super::*;
pub(super) fn read(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clean = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(clean);
    let words = words.strip_prefix(&["if"]).unwrap_or(&words);
    let tail = words.strip_prefix(&["you", "spent", "at", "least"])
        .or_else(|| words.strip_prefix(&["you've", "spent", "at", "least"]))
        .or_else(|| words.strip_prefix(&["youve", "spent", "at", "least"]))
        .or_else(|| words.strip_prefix(&["you", "have", "spent", "at", "least"]))?;
    let amount = tail.strip_suffix(&["mana", "to", "cast", "spells", "this", "turn"])?;
    let amount = if amount == ["the", "mana", "value", "of", "this", "permanent", "as", "it", "was", "created"] {
        ironsmith_core::FrozenNumericParameter::Creation(ironsmith_core::CreationNumericProperty::ManaValue)
    } else {
        let amount_tokens = crate::lexer::lex_line(&amount.join(" "), 0).ok()?;
        let (amount, used) = parse_number(&amount_tokens)?;
        if used != amount_tokens.len() { return None; }
        ironsmith_core::FrozenNumericParameter::Fixed(amount)
    };
    crate::word_claims::claim_tokens(tokens);
    Some(PredicateAst::Player(PlayerPredicateAst::PlayerSpentManaToCastSpellsThisTurnAtLeast {
        player: PlayerAst::You, amount,
    }))
}
