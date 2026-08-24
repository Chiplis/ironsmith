use super::*;

pub fn has_from_among_hand_surface(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, _, after_among)) =
        primitives::find_prefix(tokens, || primitives::phrase(&["from", "among"]).void())
    else {
        return false;
    };
    primitives::contains_word(after_among, "hand")
        || primitives::contains_word(after_among, "hands")
}
