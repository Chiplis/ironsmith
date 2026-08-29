use super::*;

pub(super) fn life_equal_followup(tokens: &[OwnedLexToken]) -> bool {
    starts_any(
        tokens,
        &[
            &["you", "gain", "life", "equal", "to", "that"],
            &["you", "gain", "life", "equal", "to", "its"],
            &["you", "gain", "life", "equal", "to", "their"],
            &["you", "lose", "life", "equal", "to", "that"],
            &["you", "lose", "life", "equal", "to", "its"],
            &["you", "lose", "life", "equal", "to", "their"],
            &["gain", "life", "equal", "to", "that"],
            &["gain", "life", "equal", "to", "its"],
            &["gain", "life", "equal", "to", "their"],
            &["gains", "life", "equal", "to", "that"],
            &["gains", "life", "equal", "to", "its"],
            &["gains", "life", "equal", "to", "their"],
            &["lose", "life", "equal", "to", "that"],
            &["lose", "life", "equal", "to", "its"],
            &["lose", "life", "equal", "to", "their"],
            &["loses", "life", "equal", "to", "that"],
            &["loses", "life", "equal", "to", "its"],
            &["loses", "life", "equal", "to", "their"],
        ],
    )
}
