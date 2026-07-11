use crate::runtime_backend::families::activation_and_restrictions::keyword_action_costs::parse_single_word_keyword_action;
use crate::runtime_backend::front_end::grammar::{permission_shapes, primitives};
use crate::runtime_backend::lexer::{TokenWordView, lex_line};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SingleKeywordVerbSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeywordAbilityNameSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VoteChoiceSurface;

pub(crate) fn parse_single_keyword_verb(name: &str) -> Option<SingleKeywordVerbSurface> {
    let tokens = lex_line(name.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
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

pub(crate) fn parse_keyword_ability_name(name: &str) -> Option<KeywordAbilityNameSurface> {
    let tokens = lex_line(name.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
    if permission_shapes::exact_words(&words, &["first", "strike"])
        || permission_shapes::exact_words(&words, &["double", "strike"])
        || permission_shapes::exact_words(&words, &["ward"])
    {
        return Some(KeywordAbilityNameSurface);
    }
    let [word] = words.as_slice() else {
        return None;
    };
    parse_single_word_keyword_action(word).map(|_| KeywordAbilityNameSurface)
}

pub(crate) fn parse_vote_choice_surface(text: &str) -> Option<VoteChoiceSurface> {
    let tokens = lex_line(text.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
    (permission_shapes::find_words(&words, &["vote", "for"]).is_some()
        || permission_shapes::find_words(&words, &["votes", "for"]).is_some())
    .then_some(VoteChoiceSurface)
}

pub(crate) fn parse_short_self_reference_name(name: &str) -> String {
    let trimmed = name.trim();
    let Ok(tokens) = lex_line(trimmed, 0) else {
        return trimmed.to_string();
    };
    if let Some((_, comma, _)) = primitives::find_prefix(&tokens, || primitives::comma()) {
        let alias = trimmed.get(..comma.span.start).unwrap_or(trimmed).trim();
        if !alias.is_empty() {
            return alias.to_string();
        }
    }

    let words = TokenWordView::new(&tokens);
    if words.len() <= 1 {
        return trimmed.to_string();
    }
    let Some(token_index) = words.token_start_indices().first().copied() else {
        return trimmed.to_string();
    };
    let alias = tokens[token_index]
        .slice
        .trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '-');
    if alias.len() <= 2 || is_reserved_short_alias(alias) {
        trimmed.to_string()
    } else {
        alias.to_string()
    }
}

fn is_reserved_short_alias(alias: &str) -> bool {
    matches!(
        alias.to_ascii_lowercase().as_str(),
        "a" | "an"
            | "the"
            | "one"
            | "two"
            | "three"
            | "four"
            | "five"
            | "six"
            | "seven"
            | "eight"
            | "nine"
            | "ten"
            | "x"
            | "this"
            | "that"
            | "these"
            | "those"
            | "you"
            | "your"
            | "when"
            | "whenever"
            | "if"
            | "at"
            | "add"
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
            | "look"
            | "lose"
            | "gain"
            | "put"
            | "sacrifice"
            | "create"
            | "investigate"
            | "attach"
            | "remove"
            | "return"
            | "exchange"
            | "become"
            | "switch"
            | "skip"
            | "surveil"
            | "shuffle"
            | "reorder"
            | "pay"
            | "goad"
            | "power"
            | "toughness"
            | "mana"
            | "life"
            | "commander"
            | "player"
            | "opponent"
            | "creature"
            | "artifact"
            | "enchantment"
            | "land"
            | "spell"
            | "card"
            | "token"
            | "permanent"
            | "library"
            | "graveyard"
            | "hand"
            | "battlefield"
            | "controller"
            | "owner"
            | "planeswalker"
            | "battle"
            | "equipment"
            | "aura"
    ) || parse_single_word_keyword_action(alias).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keyword_and_short_name_surfaces() {
        assert!(parse_single_keyword_verb("Mill").is_some());
        assert!(parse_keyword_ability_name("double strike").is_some());
        assert_eq!(
            parse_short_self_reference_name("Brago, King Eternal"),
            "Brago"
        );
        assert_eq!(
            parse_short_self_reference_name("Draw the Line"),
            "Draw the Line"
        );
    }
}
