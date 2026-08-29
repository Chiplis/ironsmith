use super::*;

pub fn is_dont_lose_mana_between_steps_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_phrase(
        tokens,
        &[
            "you", "dont", "lose", "this", "mana", "as", "steps", "and", "phases", "end",
        ],
    )
}
