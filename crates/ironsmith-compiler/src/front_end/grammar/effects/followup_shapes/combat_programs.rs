use super::*;

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
