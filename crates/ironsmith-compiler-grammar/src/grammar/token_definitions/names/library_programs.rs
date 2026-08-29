use super::*;

pub(in super::super) fn graveyard_anthem_card_name(words: &[&str]) -> Option<String> {
    let named_card_idx = common::phrase_offset(words, &["card", "named"])?;
    let start = named_card_idx + 2;
    let mut end = start;
    while end < words.len()
        && !matches!(
            words[end],
            "in" | "from" | "and" | "or" | "with" | "that" | "where" | "when" | "whenever"
        )
    {
        end += 1;
    }
    (end > start).then(|| title_case_words(&words[start..end]))
}
