use super::*;

/// Distribute modifiers that precede a coordinated noun phrase whose player
/// scope is authored once at the end. In `another nontoken artifact creature
/// or Vehicle you control`, both selectors are `another` and `nontoken`;
/// independently scoped arms such as `another creature you control or a land
/// you control` remain branch-local.
pub(super) fn propagate_leading_shared_set_modifiers(
    tokens: &[OwnedLexToken],
    caller_consumed_other: bool,
    shared_player_scope: bool,
    branches: &mut [ObjectFilter],
) {
    if !shared_player_scope || branches.len() < 2 {
        return;
    }

    let leading_tokens = tokens.iter().take_while(|token| {
        !token.is_comma()
            && !token.is_word("and")
            && !token.is_word("or")
            && !token.is_word("and/or")
    });
    let leading_words = leading_tokens
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    let shared_other = caller_consumed_other
        || leading_words
            .iter()
            .any(|word| matches!(*word, "another" | "other"));
    let shared_nontoken = crate::word_primitives::sequence_occurs(&leading_words, &["nontoken"]);

    if shared_other && branches.first().is_some_and(|branch| branch.other) {
        for branch in branches.iter_mut() {
            branch.other = true;
        }
    }
    if shared_nontoken && branches.first().is_some_and(|branch| branch.nontoken) {
        for branch in branches.iter_mut() {
            branch.nontoken = true;
        }
    }
}
