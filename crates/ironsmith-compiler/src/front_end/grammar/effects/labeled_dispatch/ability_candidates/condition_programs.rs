use super::*;

/// Separate gain/loss arms with explicit subjects and a local condition must
/// be parsed as a coordinated chain. Treating the whole sentence as one
/// ability-modifier candidate makes the first arm's trailing condition share
/// a tail with the later arm, so it can no longer be consumed as a predicate.
pub(super) fn independent_gain_or_lose_arms_with_local_condition(tokens: &[OwnedLexToken]) -> bool {
    let segments = split_effect_chain_on_and_tokens(tokens, true);
    if segments.len() < 2 {
        return false;
    }

    let all_independent_ability_arms = segments.iter().all(|segment| {
        find_chain_verb_tokens(segment).is_some_and(|verb| {
            verb.word_index > 0 && matches!(verb.kind, ChainVerbKind::Gain | ChainVerbKind::Lose)
        })
    });
    all_independent_ability_arms
        && segments[..segments.len() - 1]
            .iter()
            .any(|segment| split_trailing_if_clause_lexed(segment).is_some())
}

/// A later ability modifier must not claim an earlier independent action.
/// In clauses such as `you draw X cards and the chosen creatures get +X/+X
/// and gain trample`, the gain arm shares the creature subject from the pump
/// arm, while the draw remains a separate player action. Let the coordinated
/// chain parser preserve all three actions instead of routing the whole
/// sentence through the broad gain-ability parser.
pub(super) fn independent_action_precedes_ability_modifier(tokens: &[OwnedLexToken]) -> bool {
    let segments = split_effect_chain_on_and_tokens(tokens, true);
    let Some(first_ability_index) =
        crate::slice_primitives::select_position(&segments, |segment| {
            common::first_word_offset_any(&parser_token_word_refs(segment), GAIN_HAS_LOSE_WORDS)
                .is_some()
        })
    else {
        return false;
    };
    if first_ability_index == 0 {
        return false;
    }

    segments[..first_ability_index].iter().any(|segment| {
        find_chain_verb_tokens(segment).is_some_and(|verb| {
            verb.word_index > 0
                && !matches!(
                    verb.kind,
                    ChainVerbKind::Get | ChainVerbKind::Gain | ChainVerbKind::Lose
                )
        })
    })
}
