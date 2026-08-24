use super::*;

pub(super) fn rfind_unquoted_dynamic_word(
    tokens: &[OwnedLexToken],
    word: &'static str,
) -> Option<usize> {
    let mut inside_quotes = false;
    let mut result = None;

    for (idx, token) in tokens.iter().enumerate() {
        if is_sentence_quote(token) {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes && token_matches_dynamic_word(token, word) {
            result = Some(idx);
        }
    }

    result
}
