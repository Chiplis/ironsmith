//! Complete mandatory toughness-based combat assignment, shared by persistent
//! grants and resolving effects. No optional assignment choice is inferred.
use crate::lexer::{OwnedLexToken, TokenWordView};

pub fn assignment_verb(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut quoted = false;
    for (index, token) in tokens.iter().enumerate() {
        if token.is_quote() {
            quoted = !quoted;
        }
        if !quoted && (token.is_word("assign") || token.is_word("assigns")) {
            return Some(index);
        }
    }
    None
}

/// Returns whether a complete coordinated no-defender permission follows.
pub fn assignment_body(tokens: &[OwnedLexToken]) -> Option<bool> {
    if tokens.iter().any(OwnedLexToken::is_quote) {
        return None;
    }
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let count = match words.get(..10) {
        Some(
            [
                "combat",
                "damage",
                "equal",
                "to",
                "its",
                "toughness",
                "rather",
                "than",
                "its",
                "power",
            ],
        )
        | Some(
            [
                "combat",
                "damage",
                "equal",
                "to",
                "their",
                "toughness",
                "rather",
                "than",
                "their",
                "power",
            ],
        ) => 10,
        _ => return None,
    };
    if words.len() == count {
        return Some(false);
    }
    if words.get(count) != Some(&"and") {
        return None;
    }
    let token_start = view.map_word_to_token_start(count + 1)?;
    crate::grammar::anthem_grants::parse_no_defender_granted_fragment_tokens(&tokens[token_start..])
        .then_some(true)
}
