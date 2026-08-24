use super::*;


pub(super) fn captured_non_article_label(clause: LexedClause<'_>) -> Option<String> {
    let tokens = captured_non_article_tokens(clause);
    (!tokens.is_empty()).then(|| render_token_slice(&tokens).trim().to_string())
}


pub(super) fn captured_numeric_label(clause: LexedClause<'_>) -> Option<String> {
    let tokens = captured_non_article_tokens(clause);
    if tokens.len() == 1
        && let Some(word) = tokens[0].as_word()
        && word.chars().all(|ch| ch.is_ascii_digit())
    {
        return Some(word.to_string());
    }
    None
}
