use super::*;

pub(super) fn draw_except_first_surface(words: &[&str]) -> bool {
    any_sequence_present(
        words,
        &[
            &[
                "a", "card", "except", "the", "first", "one", "they", "draw", "in", "each", "of",
                "their", "draw", "steps",
            ],
            &[
                "a", "card", "except", "the", "first", "card", "they", "draw", "in", "each", "of",
                "their", "draw", "steps",
            ],
            &[
                "a", "card", "except", "the", "first", "one", "you", "draw", "in", "each", "of",
                "your", "draw", "steps",
            ],
            &[
                "a", "card", "except", "the", "first", "card", "you", "draw", "in", "each", "of",
                "your", "draw", "steps",
            ],
            &[
                "a", "card", "except", "the", "first", "one", "they", "draw", "in", "their",
                "draw", "step",
            ],
            &[
                "a", "card", "except", "the", "first", "card", "they", "draw", "in", "their",
                "draw", "step",
            ],
            &[
                "a", "card", "except", "the", "first", "one", "you", "draw", "in", "your", "draw",
                "step",
            ],
            &[
                "a", "card", "except", "the", "first", "card", "you", "draw", "in", "your", "draw",
                "step",
            ],
        ],
    )
}
