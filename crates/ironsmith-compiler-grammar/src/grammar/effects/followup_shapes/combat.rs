use super::*;

/// A conditional amount replacement has no recipient of its own. It is a
/// dependent sentence, not a standalone damage instruction with a default
/// player target. The sentence dispatcher must bind it to one prior damage
/// instruction before constructing an executable effect.
pub struct DamageAmountReplacementShape<'a> {
    pub predicate_tokens: &'a [OwnedLexToken],
    pub amount_tokens: &'a [OwnedLexToken],
    pub repeats_source: bool,
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
    if !repeats_source && crate::util::source_reference_surface_for_words(&source).is_none() {
        return None;
    }
    // The entire action ends at `damage`: explicit destinations, additional
    // instructions, riders and repeated `instead` markers belong elsewhere.
    if !action.last()?.is_word("damage") || deal + 2 >= action.len() {
        return None;
    }
    let amount_tokens = &action[deal + 1..action.len() - 1];
    Some(DamageAmountReplacementShape {
        predicate_tokens,
        amount_tokens,
        repeats_source,
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
