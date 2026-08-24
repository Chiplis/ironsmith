use super::*;

pub(super) fn cast_from_outside_hand_surface(words: &[&str]) -> bool {
    if any_sequence_present(
        words,
        &[
            &["from", "anywhere", "other", "than", "your", "hand"],
            &["from", "anywhere", "other", "than", "their", "hand"],
            &["from", "anywhere", "other", "than", "hand"],
        ],
    ) {
        return true;
    }

    if words.len() < 4 {
        return false;
    }
    let mut first = None;
    for index in 0..=words.len() - 4 {
        if word_slice_is(
            &words[index..index + 4],
            &["from", "anywhere", "other", "than"],
        ) {
            first = Some(index);
            break;
        }
    }
    first.is_some_and(|index| {
        words[index + 4..]
            .iter()
            .take(4)
            .any(|word| *word == "hand")
    })
}
