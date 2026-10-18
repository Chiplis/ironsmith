use super::*;

/// A conditional amount replacement has no recipient of its own. It is a
/// dependent sentence, not a standalone damage instruction with a default
/// player target. The sentence dispatcher must bind it to one prior damage
/// instruction before constructing an executable effect.
pub struct DamageAmountReplacementShape<'a> {
    pub predicate_tokens: &'a [OwnedLexToken],
    pub amount_tokens: &'a [OwnedLexToken],
    pub repeats_source: bool,
    /// "the creature you control deals twice that much damage instead": a
    /// definite description of the preceding instruction's chosen damage
    /// source, rather than `it` or this object.
    pub definite_source: bool,
    /// "it deals twice that much damage to itself instead" (Cut
    /// Propulsion): the replacement repeats a self-damage recipient.
    pub self_recipient: bool,
}

pub fn parse_damage_amount_replacement(
    tokens: &[OwnedLexToken],
) -> Option<DamageAmountReplacementShape<'_>> {
    let tokens = crate::grammar::effects::split_labeled_effect_prefix_lexed(tokens)
        .unwrap_or(tokens);
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let (predicate_tokens, action) = if tokens.first()?.is_word("if") {
        let comma = tokens.iter().position(|token| token.is_comma())?;
        let action = &tokens[comma + 1..];
        if !action.last()?.is_word("instead") {
            return None;
        }
        (&tokens[..comma], &action[..action.len() - 1])
    } else {
        let instead = tokens.iter().position(|token| token.is_word("instead"))?;
        let predicate = &tokens[instead + 1..];
        if !predicate.first()?.is_word("if") {
            return None;
        }
        (predicate, &tokens[..instead])
    };
    let deal = action.iter().position(|token| token.is_word("deals"))?;
    let source = token_word_refs(&action[..deal]);
    let repeats_source = source.as_slice() == ["it"];
    let self_source = crate::util::source_reference_surface_for_words(&source).is_some();
    let definite_source = !repeats_source
        && !self_source
        && source.len() >= 2
        && action.first().is_some_and(|token| token.is_word("the"))
        && crate::object_filters::parse_object_filter(&action[1..deal], false).is_ok();
    if !repeats_source && !self_source && !definite_source {
        return None;
    }
    // "... deals twice that much damage to itself": the self recipient is
    // the repeated recipient of a self-damage instruction; the caller checks
    // the replaced instruction damages its own source.
    let self_recipient = repeats_source
        && action.len() >= 2
        && action[action.len() - 2].is_word("to")
        && action[action.len() - 1].is_word("itself");
    let action = if self_recipient { &action[..action.len() - 2] } else { action };
    // The entire action ends at `damage`: explicit destinations, additional
    // instructions, riders and repeated `instead` markers belong elsewhere.
    let amount_tokens = if action.get(deal + 1)?.is_word("damage")
        && action.get(deal + 2)?.is_word("equal")
        && action.get(deal + 3)?.is_word("to")
    {
        // A repeated amount may follow "damage equal to" as well as
        // precede "damage". The dependent clause still has no recipient.
        let amount = &action[deal + 4..];
        if amount.is_empty() { return None; }
        amount
    } else {
        if !action.last()?.is_word("damage") || deal + 2 >= action.len() {
            return None;
        }
        &action[deal + 1..action.len() - 1]
    };
    Some(DamageAmountReplacementShape {
        predicate_tokens,
        amount_tokens,
        repeats_source,
        definite_source,
        self_recipient,
    })
}

pub fn is_anaphoric_damage_self_replacement(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    if !crate::word_primitives::parse_sequence_prefix(&words, &["it", "deals"])
        || !crate::word_primitives::contains_word(&words, "instead")
    {
        return false;
    }
    if crate::word_primitives::sequence_occurs(&words, &["to", "that", "creature"]) {
        return true;
    }

    // "It deals N damage instead" omits both arguments because it repeats
    // the source and target of the default damage event. Do not apply this to
    // a clause that names a different destination explicitly.
    let Some(damage_idx) =
        crate::word_primitives::select_word_position(&words, |word| word == "damage")
    else {
        return false;
    };
    let Some(instead_idx) =
        crate::word_primitives::select_word_position(&words, |word| word == "instead")
    else {
        return false;
    };
    damage_idx < instead_idx
        && !crate::word_primitives::contains_word(&words[damage_idx + 1..instead_idx], "to")
}
