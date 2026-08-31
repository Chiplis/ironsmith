use super::*;

pub fn parse_become_attack_color(words: &[&str]) -> Option<ColorSet> {
    let [
        color_word,
        "until",
        "end",
        "of",
        "turn",
        "and",
        "attacks",
        tail @ ..,
    ] = words
    else {
        return None;
    };
    if !matches!(tail, ["if", "able"] | ["this", "turn", "if", "able"]) {
        return None;
    }
    crate::grammar::primitives::probe_shape(leaf::parse_leaf_color_complete(color_word))
}
