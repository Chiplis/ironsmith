use super::*;

pub(super) fn damage_equal_followup(tokens: &[OwnedLexToken]) -> bool {
    starts_any(
        tokens,
        &[
            &["it", "deal", "damage", "equal", "to"],
            &["it", "deals", "damage", "equal", "to"],
            &["that", "creature", "deal", "damage", "equal", "to"],
            &["that", "creature", "deals", "damage", "equal", "to"],
            &["that", "objects", "deal", "damage", "equal", "to"],
            &["that", "objects", "deals", "damage", "equal", "to"],
        ],
    ) || (find_chain_verb_tokens(tokens)
        .is_some_and(|found| found.kind == super::super::ChainVerbKind::Deal)
        && contains_all(tokens, &["damage", "equal", "to"]))
}
