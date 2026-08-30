use super::*;

pub(super) fn is_card_type_word(word: &str) -> bool {
    CARD_TYPE_WORDS.contains(&word)
}
