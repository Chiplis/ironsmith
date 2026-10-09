//! "Tap and goad the chosen creatures." (Fell Beast's Shriek): two imperative
//! verbs sharing one already-bound object set are two instructions on that
//! set, performed in order. Only untargeted references qualify, so the split
//! never duplicates a target announcement.

use crate::lexer::OwnedLexToken;

fn is_pair_head(tokens: &[OwnedLexToken]) -> bool {
    tokens.len() > 3
        && tokens[0].is_any_word(&["tap", "untap"])
        && tokens[1].is_word("and")
        && tokens[2].is_word("goad")
}

/// The token stream with every qualifying verb-pair sentence split in two,
/// or `None` when no sentence qualifies.
pub(super) fn split_shared_object_verb_pairs(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut out = Vec::with_capacity(tokens.len() + 4);
    let mut changed = false;
    let mut start = 0;
    while start < tokens.len() {
        let end = tokens[start..]
            .iter()
            .position(OwnedLexToken::is_period)
            .map_or(tokens.len(), |idx| start + idx);
        let sentence = &tokens[start..end];
        if is_pair_head(sentence) && !sentence.iter().any(|token| token.is_word("target")) {
            let object = &sentence[3..];
            let separator = tokens
                .get(end)
                .cloned()
                .unwrap_or_else(|| OwnedLexToken::period(sentence[2].span()));
            out.push(sentence[0].clone());
            out.extend_from_slice(object);
            out.push(separator);
            out.push(sentence[2].clone());
            out.extend_from_slice(object);
            changed = true;
        } else {
            out.extend_from_slice(sentence);
        }
        if let Some(period) = tokens.get(end) {
            out.push(period.clone());
        }
        start = end + 1;
    }
    changed.then_some(out)
}
