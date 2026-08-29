use super::*;

pub fn contains_graveyard_and_hand(tokens: &[OwnedLexToken]) -> bool {
    let graveyard = primitives::contains_word(tokens, "graveyard")
        || primitives::contains_word(tokens, "graveyards");
    let hand =
        primitives::contains_word(tokens, "hand") || primitives::contains_word(tokens, "hands");
    graveyard && hand
}
