//! Token-complete admission for linked-card and counter-result quantities.
use super::*;

fn removed_counter_descriptor_is_complete(words: &[&str]) -> bool {
    let Some(counter) = words.iter().position(|word| matches!(*word, "counter" | "counters")) else { return true; };
    if words.get(counter + 1..counter + 4) != Some(&["removed", "this", "way"][..]) { return true; }
    let start = words[..counter].windows(2).rposition(|pair| pair == ["number", "of"])
        .map(|index| index + 2)
        .or_else(|| words[..counter].windows(2).rposition(|pair| pair == ["for", "each"]).map(|index| index + 2))
        .unwrap_or(0);
    let descriptor = &words[start..counter];
    match descriptor {
        [] => true,
        [_] | ["first" | "double", "strike"] => {
            let mut full = descriptor.to_vec(); full.push("counter");
            parse_counter_type_words(&full).is_some()
        }
        _ => false,
    }
}

fn protected_reference(words: &[&str]) -> bool {
    (words.windows(2).any(|pair| matches!(pair, ["exiled", "card" | "cards"]))
        && words.iter().any(|word| matches!(*word, "power" | "toughness" | "mana")))
        || words.windows(4).any(|part| matches!(part, ["counter" | "counters", "removed", "this", "way"]))
}

pub(super) fn tokens_are_complete(tokens: &[OwnedLexToken]) -> bool {
    let mut end = tokens.len();
    while end > 0 && matches!(tokens[end - 1].kind, crate::lexer::TokenKind::Period | crate::lexer::TokenKind::Comma | crate::lexer::TokenKind::Semicolon) { end -= 1; }
    let tokens = &tokens[..end];
    let words = crate::lexer::token_word_refs(tokens);
    if !protected_reference(&words) { return true; }
    tokens.iter().all(|token| matches!(token.kind, crate::lexer::TokenKind::Word | crate::lexer::TokenKind::Number))
        && removed_counter_descriptor_is_complete(&words)
}

/// A recognized but malformed binding must fail before broad sentence and
/// word-only readers can reclaim it after a narrower parser rejects it.
pub(crate) fn validate_bindings(tokens: &[OwnedLexToken]) -> Result<(), crate::cards::builders::CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    for (index, phrase) in words.windows(3).enumerate() {
        if phrase != ["where", "x", "is"] { continue; }
        let start = view.token_start_indices()[index];
        let end = tokens[start..].iter().position(|token| matches!(token.kind,
            crate::lexer::TokenKind::Period | crate::lexer::TokenKind::Comma | crate::lexer::TokenKind::Semicolon))
            .map_or(tokens.len(), |offset| start + offset);
        if !tokens_are_complete(&tokens[start..end]) {
            return Err(crate::cards::builders::CardTextError::ParseError(
                "linked-card or removed-counter quantity contains unsupported tokens or an incomplete descriptor".into(),
            ));
        }
    }
    Ok(())
}
