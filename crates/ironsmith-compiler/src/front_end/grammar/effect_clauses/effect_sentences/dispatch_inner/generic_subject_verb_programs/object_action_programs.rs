use super::*;


pub(super) fn captured_non_article_tokens(clause: LexedClause<'_>) -> Vec<OwnedLexToken> {
    clause
        .trimmed()
        .tokens()
        .iter()
        .filter(|token| token.as_word().is_none_or(|word| !is_article(word)))
        .cloned()
        .collect()
}
