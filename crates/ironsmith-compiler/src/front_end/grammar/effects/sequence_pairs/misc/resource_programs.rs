use super::*;

pub fn has_life_gain_surface(tokens: &[OwnedLexToken]) -> bool {
    contains_sequence_word(tokens, "life")
        && (contains_sequence_word(tokens, "gain") || contains_sequence_word(tokens, "gains"))
}
