use super::*;

pub(crate) fn cant_sentence_has_source_remains_tapped_duration(tokens: &[OwnedLexToken]) -> bool {
    let mut has_for_as_long_as = false;
    let mut has_remains = false;
    let mut has_tapped = false;
    let mut has_source_word = false;
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        if !has_for_as_long_as
            && primitives::parse_prefix(&tokens[cursor..], cant_sentence_for_as_long_as_marker)
                .is_some()
        {
            has_for_as_long_as = true;
        }

        let token = &tokens[cursor];
        has_remains |= token_is_any_word(token, &["remains"]);
        has_tapped |= token_is_any_word(token, &["tapped"]);
        has_source_word |= token_is_any_word(
            token,
            &["this", "source", "artifact", "creature", "permanent"],
        );
        cursor += 1;
    }

    has_for_as_long_as && has_remains && has_tapped && has_source_word
}
