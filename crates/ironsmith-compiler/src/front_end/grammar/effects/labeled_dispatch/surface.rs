use crate::lexer::{OwnedLexToken, parser_token_word_refs};
use winnow::prelude::*;

use super::super::super::primitives;
use super::common;

const EACH_PLAYER_CHOOSE_PREFIXES: &[&[&str]] = &[
    &["each", "player", "choose"],
    &["each", "player", "chooses"],
    &["each", "opponent", "choose"],
    &["each", "opponent", "chooses"],
];
const PRE_EXTENSION_HEADS: &[&[&str]] = &[&["prevent"], &["take"], &["monstrosity"]];
const HAND_WORDS: &[&[&str]] = &[&["hand"], &["hands"]];
const GRAVEYARD_WORDS: &[&[&str]] = &[&["graveyard"], &["graveyards"]];
const GAIN_LOSE_WORDS: &[&[&str]] = &[&["gain"], &["gains"], &["lose"], &["loses"]];
const VOTE_WORDS: &[&[&str]] = &[&["vote"], &["votes"]];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastFromAmongShape {
    pub mana_value_or_less: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct LabeledDispatchShape<'a> {
    pub round_up_each_time: bool,
    pub starts_if: bool,
    pub pre_extension_head: bool,
    pub exile_then: bool,
    pub then_tail: Option<&'a [OwnedLexToken]>,
    pub each_player_choose: bool,
    pub cast_from_among_free: Option<CastFromAmongShape>,
    pub cast_hand_free: bool,
    pub has_unquoted_search: bool,
    pub exile_all_cards_from_hand_graveyard: bool,
    pub starts_enchant: bool,
    pub starts_earthbend: bool,
    pub has_unless: bool,
    pub has_gain_or_lose: bool,
    pub has_vote: bool,
    pub return_rounded_up: bool,
    pub choose_do_same_for: bool,
    pub cast_any_number_graveyard_free: bool,
    pub starts_sacrifice: bool,
    pub sacrifice_counted: bool,
    pub tap_all_or_each_then_untap_all_or_each: bool,
}

fn mana_value_or_less_bound(tokens: &[OwnedLexToken]) -> Option<u32> {
    let mut search_tokens = tokens;
    while !search_tokens.is_empty() {
        let (_, _, tail) = primitives::find_prefix(search_tokens, || {
            primitives::phrase(&["mana", "value"]).void()
        })?;
        if let Some((count, _)) = crate::util::parse_less_than_or_equal_quantity_prefix(
            tail,
            false,
            false,
            "mana value bound",
        )
        .ok()
        .flatten()
        {
            return Some(count);
        }
        search_tokens = tail;
    }
    None
}

fn contains_unquoted_search(tokens: &[OwnedLexToken]) -> bool {
    let mut inside_quotes = false;
    for token in tokens {
        if token.is_quote() {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes && matches!(token.as_word(), Some("search" | "searches")) {
            return true;
        }
    }
    false
}

pub fn parse_labeled_dispatch_shape(tokens: &[OwnedLexToken]) -> LabeledDispatchShape<'_> {
    let words = parser_token_word_refs(tokens);
    let then_tail = primitives::parse_prefix(tokens, primitives::kw("then").void())
        .map(|(_, rest)| rest)
        .filter(|rest| !rest.is_empty());
    let cast_from_among_free = (common::prefix(&words, &["you", "may", "cast"])
        && common::present(&words, &["from", "among", "them"])
        && common::present(&words, &["without", "paying", "its", "mana", "cost"]))
    .then(|| CastFromAmongShape {
        mana_value_or_less: mana_value_or_less_bound(tokens),
    });

    LabeledDispatchShape {
        round_up_each_time: common::prefix(&words, &["round", "up", "each", "time"]),
        starts_if: common::prefix(&words, &["if"]),
        pre_extension_head: common::prefix_any(&words, PRE_EXTENSION_HEADS),
        exile_then: common::prefix(&words, &["exile"]) && common::present(&words, &["then"]),
        then_tail,
        each_player_choose: common::prefix_any(&words, EACH_PLAYER_CHOOSE_PREFIXES),
        cast_from_among_free,
        cast_hand_free: common::prefix(
            &words,
            &["you", "may", "cast", "a", "spell", "from", "your", "hand"],
        ) && common::present(&words, &["without", "paying", "its", "mana", "cost"]),
        has_unquoted_search: contains_unquoted_search(tokens),
        exile_all_cards_from_hand_graveyard: common::prefix(
            &words,
            &["exile", "all", "cards", "from"],
        ) && common::present_any(&words, HAND_WORDS)
            && common::present_any(&words, GRAVEYARD_WORDS),
        starts_enchant: common::prefix(&words, &["enchant"]),
        starts_earthbend: common::prefix(&words, &["earthbend"]),
        has_unless: common::present(&words, &["unless"]),
        has_gain_or_lose: common::present_any(&words, GAIN_LOSE_WORDS),
        has_vote: common::present_any(&words, VOTE_WORDS),
        return_rounded_up: common::prefix(&words, &["return"])
            && common::present(&words, &["rounded"])
            && common::present(&words, &["up"]),
        choose_do_same_for: common::prefix(&words, &["choose"])
            && common::present(&words, &["do", "the", "same", "for"]),
        cast_any_number_graveyard_free: common::prefix(&words, &["cast", "any", "number", "of"])
            && common::present(&words, &["from", "your", "graveyard"])
            && common::present(&words, &["without", "paying", "their", "mana", "costs"]),
        starts_sacrifice: common::prefix(&words, &["sacrifice"]),
        sacrifice_counted: common::prefix(&words, &["sacrifice", "any", "number"])
            || common::prefix(&words, &["sacrifice", "one", "or", "more"]),
        tap_all_or_each_then_untap_all_or_each: common::prefix_any(
            &words,
            &[&["tap", "all"], &["tap", "each"]],
        ) && common::present_any(
            &words,
            &[&["or", "untap", "all"], &["or", "untap", "each"]],
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex_line;

    use super::*;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn cast_from_among_shape_owns_free_cast_and_bound_recognition() {
        let tokens = lex(
            "You may cast a spell with mana value 5 or less from among them without paying its mana cost.",
        );
        let shape = parse_labeled_dispatch_shape(&tokens);
        assert_eq!(
            shape.cast_from_among_free,
            Some(CastFromAmongShape {
                mana_value_or_less: Some(5)
            })
        );
    }

    #[test]
    fn dispatch_shape_preserves_route_markers_and_unquoted_search() {
        let tokens =
            lex("Exile all cards from each player's hand and graveyard, then search your library.");
        let shape = parse_labeled_dispatch_shape(&tokens);
        assert!(shape.exile_then);
        assert!(shape.exile_all_cards_from_hand_graveyard);
        assert!(shape.has_unquoted_search);

        let quoted_tokens = lex("Create a token with \"Search your library.\"");
        let quoted = parse_labeled_dispatch_shape(&quoted_tokens);
        assert!(!quoted.has_unquoted_search);
    }
}
