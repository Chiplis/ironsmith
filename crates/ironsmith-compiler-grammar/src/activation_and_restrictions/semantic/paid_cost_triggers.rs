//! Completed keyword-cost payments, separate from instructions that pay costs.
use super::*;
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<TriggerSpec> {
    let tokens = crate::activation_and_restrictions::strip_leading_trigger_intro(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    if let Some(prefix) = words
        .strip_suffix(&["pay", "life"])
        .or_else(|| words.strip_suffix(&["pays", "life"]))
    {
        let player = parse_trigger_subject_player_filter(prefix)?;
        crate::word_claims::claim_tokens(tokens);
        return Some(TriggerSpec::PlayerPaysLife(player));
    }
    if let Some(source) = words.strip_suffix(&["cumulative", "upkeep", "is", "paid"]) {
        let source = crate::util::possessive_normalized_word_refs(source);
        crate::util::source_reference_surface_for_possessive_words(&source)?;
        crate::word_claims::claim_tokens(tokens);
        return Some(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::CumulativeUpkeepPaid,
            player: PlayerFilter::Any,
        });
    }
    if let Some(source) = words.strip_suffix(&["echo", "cost", "is", "paid"]) {
        let source = crate::util::possessive_normalized_word_refs(source);
        crate::util::source_reference_surface_for_possessive_words(&source)?;
        crate::word_claims::claim_tokens(tokens);
        return Some(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::EchoCostPaid,
            player: PlayerFilter::Any,
        });
    }
    let (prefix, action) = if let Some(prefix) = words.strip_suffix(&["cumulative", "upkeep"]) {
        (
            prefix,
            crate::events::KeywordActionKind::CumulativeUpkeepPaid,
        )
    } else if let Some(prefix) = words.strip_suffix(&["echo", "cost"]) {
        (prefix, crate::events::KeywordActionKind::EchoCostPaid)
    } else {
        return None;
    };
    let pay = prefix
        .iter()
        .position(|word| matches!(*word, "pay" | "pays"))?;
    let player = parse_trigger_subject_player_filter(&prefix[..pay])?;
    let source = crate::util::possessive_normalized_word_refs(&prefix[pay + 1..]);
    crate::util::source_reference_surface_for_possessive_words(&source)?;
    // Payer, verb, this-source possessive and keyword cost are all read here.
    crate::word_claims::claim_tokens(tokens);
    Some(TriggerSpec::KeywordActionFromSource { action, player })
}
