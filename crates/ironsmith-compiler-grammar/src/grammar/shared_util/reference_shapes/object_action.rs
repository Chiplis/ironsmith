use super::*;

pub(super) fn is_controller_or_owner(word: &str) -> bool {
    starts_with_one_of_words(&[word], 0, &["controller", "owner"])
}
