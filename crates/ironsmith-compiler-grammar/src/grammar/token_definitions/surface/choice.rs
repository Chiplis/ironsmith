use super::*;

pub fn source_chosen_token_characteristics(words: &[&str]) -> (bool, bool) {
    let use_source_chosen_color = [
        &["token", "of", "the", "chosen", "color"][..],
        &["token", "of", "that", "color"][..],
    ]
    .into_iter()
    .any(|phrase| common::phrase_present(words, phrase));
    let use_source_chosen_creature_type = [
        &["chosen", "color", "and", "type"][..],
        &["that", "color", "and", "type"][..],
        &["chosen", "color", "and", "creature", "type"][..],
        &["that", "color", "and", "creature", "type"][..],
        &["token", "of", "the", "chosen", "type"][..],
        &["token", "of", "that", "type"][..],
    ]
    .into_iter()
    .any(|phrase| common::phrase_present(words, phrase));
    (use_source_chosen_color, use_source_chosen_creature_type)
}
