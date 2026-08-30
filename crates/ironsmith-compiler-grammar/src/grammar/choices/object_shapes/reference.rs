use super::*;

pub(super) fn parse_player_target_prefix_words(words: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        primitives::word_slice_exact("target"),
        alt((
            primitives::word_slice_exact("player"),
            primitives::word_slice_exact("opponent"),
        )),
    )
        .parse_next(&mut input)
        .is_ok()
}
