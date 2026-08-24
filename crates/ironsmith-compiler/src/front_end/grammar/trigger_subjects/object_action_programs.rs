use super::*;

pub(super) fn attached_controller_occurs(words: &[&str], subject: &str) -> bool {
    const OBJECTS: &[&str] = &[
        "creature",
        "creatures",
        "permanent",
        "permanents",
        "artifact",
        "artifacts",
        "enchantment",
        "enchantments",
        "land",
        "lands",
    ];

    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let mut candidate = input;
        if parse_normalized_word(&mut candidate, subject).is_ok()
            && parse_normalized_word_choice(&mut candidate, OBJECTS).is_ok()
            && parse_normalized_word(&mut candidate, "controller").is_ok()
        {
            return true;
        }
        if take_word_slice_any(&mut input).is_err() {
            return false;
        }
    }
}
