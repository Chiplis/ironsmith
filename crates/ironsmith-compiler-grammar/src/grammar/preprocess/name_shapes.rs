use crate::activation_and_restrictions::keyword_action_costs::{
    is_known_keyword_action_head, parse_single_word_keyword_action,
};
use crate::grammar::{permission_shapes, primitives};
use crate::lexer::{OwnedLexToken, TokenWordView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SingleKeywordVerbSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordAbilityNameSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoteChoiceSurface;

/// A card name that is a single keyword verb ("Mill"), read from the name's
/// tokens.
pub fn parse_single_keyword_verb_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SingleKeywordVerbSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    let [word] = words.as_slice() else {
        return None;
    };
    matches!(
        *word,
        "add"
            | "move"
            | "deal"
            | "draw"
            | "counter"
            | "destroy"
            | "exile"
            | "untap"
            | "scry"
            | "discard"
            | "transform"
            | "regenerate"
            | "mill"
            | "get"
            | "reveal"
            | "lose"
            | "gain"
            | "put"
            | "sacrifice"
            | "create"
            | "investigate"
            | "remove"
            | "return"
            | "exchange"
            | "become"
            | "switch"
            | "skip"
            | "surveil"
            | "pay"
    )
    .then_some(SingleKeywordVerbSurface)
}

#[cfg(test)]
pub fn parse_single_keyword_verb(name: &str) -> Option<SingleKeywordVerbSurface> {
    let tokens = crate::util::lex_fragment(name.trim(), 0)?;
    parse_single_keyword_verb_tokens(&tokens)
}

/// A card name that is a keyword ability ("double strike", "ward"), read from
/// the name's tokens.
pub fn parse_keyword_ability_name_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordAbilityNameSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    if permission_shapes::exact_words(&words, &["first", "strike"])
        || permission_shapes::exact_words(&words, &["double", "strike"])
        || permission_shapes::exact_words(&words, &["ward"])
    {
        return Some(KeywordAbilityNameSurface);
    }
    let [word] = words.as_slice() else {
        return None;
    };
    (parse_single_word_keyword_action(word).is_some() || is_known_keyword_action_head(word))
        .then_some(KeywordAbilityNameSurface)
}

#[cfg(test)]
pub fn parse_keyword_ability_name(name: &str) -> Option<KeywordAbilityNameSurface> {
    let tokens = crate::util::lex_fragment(name.trim(), 0)?;
    parse_keyword_ability_name_tokens(&tokens)
}

/// A clause that votes for something ("each player votes for ..."), read from
/// the clause's tokens.
pub fn parse_vote_choice_surface_tokens(tokens: &[OwnedLexToken]) -> Option<VoteChoiceSurface> {
    let words = TokenWordView::new(tokens).word_refs();
    (permission_shapes::find_words(&words, &["vote", "for"]).is_some()
        || permission_shapes::find_words(&words, &["votes", "for"]).is_some())
    .then_some(VoteChoiceSurface)
}

#[cfg(test)]
pub fn parse_vote_choice_surface(text: &str) -> Option<VoteChoiceSurface> {
    let tokens = crate::util::lex_fragment(text.trim(), 0)?;
    parse_vote_choice_surface_tokens(&tokens)
}

/// The short alias rules text uses for a card ("Brago" for "Brago, King
/// Eternal"; the full name when there is no shorter one). `tokens` are the
/// name's tokens, lexed from `name.trim()` at offset 0; a name the lexer
/// rejected (no tokens) keeps its full text.
pub fn parse_short_self_reference_name_tokens(name: &str, tokens: &[OwnedLexToken]) -> String {
    let trimmed = name.trim();
    if tokens.is_empty() {
        return trimmed.to_string();
    }
    if let Some((_, comma, _)) = primitives::find_prefix(tokens, primitives::comma) {
        let alias = trimmed.get(..comma.span.start).unwrap_or(trimmed).trim();
        if !alias.is_empty() {
            return alias.to_string();
        }
    }

    let words = TokenWordView::new(tokens);
    if words.len() <= 1 {
        return trimmed.to_string();
    }
    let Some(token_index) = words.token_start_indices().first().copied() else {
        return trimmed.to_string();
    };
    let alias_token = &tokens[token_index];
    // Trim punctuation, not letters: "Éomer" is the alias, not "omer".
    let alias = alias_token
        .slice
        .trim_matches(|character: char| !crate::lexer::is_word_char(character) && character != '-');
    if alias.len() <= 2 || is_reserved_short_alias(alias, std::slice::from_ref(alias_token)) {
        trimmed.to_string()
    } else {
        alias.to_string()
    }
}

#[cfg(test)]
pub fn parse_short_self_reference_name(name: &str) -> String {
    let trimmed = name.trim();
    let tokens = crate::util::lex_fragment(trimmed, 0).unwrap_or_default();
    parse_short_self_reference_name_tokens(trimmed, &tokens)
}

#[cfg(test)]
#[path = "name_shapes_inline_tests.rs"]
mod tests;

#[path = "name_shapes/core.rs"]
mod core_programs;
use core_programs::is_reserved_short_alias;
