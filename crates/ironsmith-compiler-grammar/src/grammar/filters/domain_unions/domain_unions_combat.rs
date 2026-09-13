use super::*;

pub(super) fn contains_attacking_player_or_planeswalker_relation(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let Some(attacking) = crate::word_primitives::parse_sequence_start(&words, &["attacking"])
    else {
        return false;
    };
    words[attacking..].iter().enumerate().any(|(index, word)| {
        if *word != "or" {
            return false;
        }
        let tail = &words[attacking + index + 1..];
        let tail = if tail.first().is_some_and(|word| matches!(*word, "a" | "an" | "the")) {
            &tail[1..]
        } else {
            tail
        };
        tail.first().is_some_and(|word| matches!(*word, "planeswalker" | "planeswalkers"))
    })
}

/// `creature that blocked or was blocked by a Zombie this turn` is one
/// historical relation with a nested partner filter, not an object-domain
/// union. Splitting at `or` flattens it into the nonsensical pair "blocked
/// creature or blocked Zombie" before the reference/tag grammar can retain
/// the partner characteristics.
pub(super) fn contains_historical_block_partner_relation(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    crate::word_primitives::sequence_occurs(&words, &["blocked", "or", "was", "blocked", "by"])
        && crate::word_primitives::sequence_occurs(&words, &["this", "turn"])
}

/// `creature blocking or blocked by this creature` describes one creature
/// related to the source, not a union between a blocking creature and a
/// blocked creature. Leave the connective for the reference/tag grammar so it
/// can retain the source-relative combat constraint.
pub(super) fn contains_current_block_partner_relation(tokens: &[OwnedLexToken]) -> bool {
    crate::word_primitives::sequence_occurs(
        &TokenWordView::new(tokens).word_refs(),
        &["blocking", "or", "blocked", "by"],
    )
}
