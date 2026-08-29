use super::*;

/// A later `that creature loses flying` arm is an independent tagged action,
/// not the subject and payload of one whole-clause ability-removal sentence.
/// Keep the ability candidate route from consuming the source damage arm so
/// the ordinary coordinated-chain parser can preserve both actions.
pub(super) fn source_damage_then_tagged_loses_ability(tokens: &[OwnedLexToken]) -> bool {
    let segments = split_effect_chain_on_and_tokens(tokens, true);
    let [damage_tokens, removal_tokens] = segments.as_slice() else {
        return false;
    };

    let Some(damage_verb) = find_chain_verb_tokens(damage_tokens) else {
        return false;
    };
    let damage_words = parser_token_word_refs(damage_tokens);
    if damage_verb.kind != ChainVerbKind::Deal
        || damage_verb.word_index == 0
        || !is_source_reference(&damage_words[..damage_verb.word_index])
        || !common::present(&damage_words[damage_verb.word_index + 1..], &["damage"])
        || !common::present(&damage_words[damage_verb.word_index + 1..], &["target"])
    {
        return false;
    }

    let Some(removal_verb) = find_chain_verb_tokens(removal_tokens) else {
        return false;
    };
    let removal_words = parser_token_word_refs(removal_tokens);
    removal_verb.kind == ChainVerbKind::Lose
        && removal_verb.word_index > 0
        && is_tagged_object_reference(&removal_words[..removal_verb.word_index])
        && common::present_any(
            &removal_words[removal_verb.word_index + 1..],
            SIMPLE_ABILITY_WORDS,
        )
}
