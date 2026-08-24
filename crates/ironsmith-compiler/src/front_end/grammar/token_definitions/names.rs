use winnow::combinator::repeat;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::lexer::{LexStream, OwnedLexToken, parser_token_word_refs, render_token_slice};

use super::super::{leaf, primitives};
use super::common;

const TITLE_LOWERCASE_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "nor", "for", "so", "yet", "of", "in", "on", "at", "to",
    "from", "with", "without", "by", "as", "into", "onto", "over", "under",
];

const NAMED_CARD_STOP_WORDS: &[&str] = &[
    "from",
    "to",
    "and",
    "with",
    "that",
    "thats",
    "it",
    "at",
    "until",
    "if",
    "where",
    "when",
    "whenever",
    "this",
    "token",
    "tokens",
    "tapped",
    "attacking",
    "add",
    "sacrifice",
    "draw",
    "deals",
    "deal",
    "damage",
    "gets",
    "gains",
    "gain",
    "cant",
    "can",
    "attack",
    "block",
    "flying",
    "trample",
    "haste",
    "vigilance",
    "menace",
    "deathtouch",
    "lifelink",
    "reach",
    "hexproof",
    "indestructible",
    "infect",
    "flash",
    "islandwalk",
    "mountainwalk",
    "forestwalk",
    "swampwalk",
    "plainswalk",
    "first",
    "double",
    "strike",
    "t",
    "w",
    "u",
    "b",
    "r",
    "g",
    "c",
];

const LEADING_NAME_STOP_WORDS: &[&str] = &[
    "a",
    "an",
    "the",
    "legendary",
    "snow",
    "basic",
    "named",
    "with",
    "that",
    "which",
    "when",
    "whenever",
    "if",
    "at",
    "until",
    "this",
    "it",
    "those",
    "token",
    "tokens",
    "and",
    "or",
    "to",
    "from",
    "add",
    "sacrifice",
    "draw",
    "deals",
    "deal",
    "damage",
    "dies",
    "gets",
    "gains",
    "gain",
    "cant",
    "can",
    "attack",
    "block",
    "flying",
    "haste",
    "deathtouch",
    "trample",
    "vigilance",
    "lifelink",
    "menace",
    "reach",
    "hexproof",
    "indestructible",
    "infect",
    "flash",
    "islandwalk",
    "mountainwalk",
    "forestwalk",
    "swampwalk",
    "plainswalk",
    "prowess",
    "first",
    "double",
    "strike",
    "white",
    "blue",
    "black",
    "red",
    "green",
    "colorless",
    "w",
    "u",
    "b",
    "r",
    "g",
    "c",
    "t",
];

fn title_case_words(words: &[&str]) -> String {
    words
        .iter()
        .filter(|word| !word.is_empty())
        .enumerate()
        .map(|(idx, word)| {
            if idx > 0
                && TITLE_LOWERCASE_WORDS
                    .iter()
                    .any(|candidate| candidate == word)
            {
                return (*word).to_string();
            }
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                let mut out = first.to_uppercase().to_string();
                out.push_str(chars.as_str());
                out
            } else {
                String::new()
            }
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case_phrase_preserving_punctuation(phrase: &str) -> String {
    let title_word = |idx: usize, word: &str| {
        let letters_only: String = word
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .map(|ch| ch.to_ascii_lowercase())
            .collect();
        let keep_lowercase = idx > 0 && TITLE_LOWERCASE_WORDS.contains(&letters_only.as_str());
        if keep_lowercase {
            return word.to_string();
        }
        let mut out = String::with_capacity(word.len());
        let mut uppercased = false;
        for ch in word.chars() {
            if !uppercased && ch.is_ascii_alphabetic() {
                out.extend(ch.to_uppercase());
                uppercased = true;
            } else {
                out.push(ch);
            }
        }
        out
    };

    let mut out = String::with_capacity(phrase.len());
    let mut word = String::new();
    let mut word_idx = 0usize;
    for ch in phrase.chars() {
        if ch.is_whitespace() {
            if !word.is_empty() {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(title_word(word_idx, word.as_str()).as_str());
                word.clear();
                word_idx += 1;
            }
            continue;
        }
        word.push(ch);
    }
    if !word.is_empty() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(title_word(word_idx, word.as_str()).as_str());
    }
    out
}

fn simple_name_word(word: &str) -> bool {
    word.chars()
        .all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-')
}

fn explicit_name_descriptor(word: &str) -> bool {
    matches!(
        word,
        "legendary"
            | "snow"
            | "basic"
            | "artifact"
            | "enchantment"
            | "creature"
            | "land"
            | "instant"
            | "sorcery"
            | "battle"
            | "planeswalker"
            | "token"
            | "tokens"
            | "white"
            | "blue"
            | "black"
            | "red"
            | "green"
            | "colorless"
            | "named"
            | "with"
            | "that"
            | "which"
            | "and"
            | "or"
            | "a"
            | "an"
            | "flying"
            | "haste"
            | "deathtouch"
            | "trample"
            | "vigilance"
            | "lifelink"
            | "menace"
            | "reach"
            | "hexproof"
            | "indestructible"
            | "infect"
            | "flash"
            | "islandwalk"
            | "mountainwalk"
            | "forestwalk"
            | "swampwalk"
            | "plainswalk"
            | "prowess"
            | "first"
            | "double"
            | "strike"
            | "when"
            | "whenever"
            | "if"
            | "this"
            | "it"
            | "those"
            | "cant"
            | "can"
            | "attack"
            | "block"
            | "dies"
            | "deals"
            | "deal"
            | "damage"
            | "draw"
            | "add"
            | "sacrifice"
            | "counter"
            | "gets"
            | "gains"
            | "gain"
    )
}

fn is_token_pt(word: &str) -> bool {
    leaf::parse_leaf_unsigned_pt_complete(word).is_ok()
}

fn is_card_type(word: &str) -> bool {
    leaf::parse_leaf_card_type_complete(word).is_ok()
}

fn is_subtype(word: &str) -> bool {
    leaf::parse_leaf_subtype_complete(word).is_ok()
}

pub(super) fn named_card_name(tokens: &[OwnedLexToken]) -> Option<String> {
    let pieces = tokens
        .iter()
        .flat_map(|token| token.parser_word_pieces())
        .collect::<Vec<_>>();
    let piece_words = pieces
        .iter()
        .map(|piece| piece.text.as_str())
        .collect::<Vec<_>>();
    let named_idx = common::first_word_offset(&piece_words, "named")?;
    if named_idx > 0 && matches!(piece_words[named_idx - 1], "card" | "cards") {
        return None;
    }

    let mut end = named_idx + 1;
    while end < pieces.len()
        && !NAMED_CARD_STOP_WORDS
            .iter()
            .any(|candidate| *candidate == pieces[end].text.as_str())
    {
        end += 1;
    }
    if end <= named_idx + 1 {
        return None;
    }

    let name_start = pieces[named_idx + 1].span.start;
    let name_end = pieces[end - 1].span.end;
    let name_tokens = tokens
        .iter()
        .filter(|token| token.span.end > name_start && token.span.start < name_end)
        .cloned()
        .collect::<Vec<_>>();
    if !name_tokens.is_empty() {
        let raw_name = render_token_slice(&name_tokens);
        let titled = title_case_phrase_preserving_punctuation(raw_name.as_str());
        if !titled.is_empty() {
            return Some(titled);
        }
    }

    Some(title_case_words(&piece_words[named_idx + 1..end]))
}

/// Parses the named-token template whose proper name precedes a comma, as in
/// `Tamiyo's Notebook, a legendary ... token`. Keeping the original token
/// slice here is important: parser words intentionally normalize apostrophes,
/// but the token's runtime/display name must retain them.
pub(super) fn leading_comma_name(tokens: &[OwnedLexToken]) -> Option<String> {
    // The separator belongs to the appositive token description, not
    // necessarily the first comma: proper token names themselves can contain
    // commas (for example, `Name, Epithet, a legendary ... token`).
    let comma = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.is_comma())
        .filter_map(|(idx, _)| {
            let suffix_words = parser_token_word_refs(tokens.get(idx + 1..)?);
            let starts_appositive = suffix_words
                .first()
                .is_some_and(|word| matches!(*word, "a" | "an"));
            let describes_token = crate::word_primitives::contains_word(&suffix_words, "token");
            (starts_appositive && describes_token).then_some(idx)
        })
        .next()?;
    let prefix = tokens.get(..comma)?;
    let words = parser_token_word_refs(prefix);
    let first = *words.first()?;
    if matches!(first, "a" | "an")
        || (first != "the" && explicit_name_descriptor(first))
        || is_token_pt(first)
        || is_card_type(first)
    {
        return None;
    }
    if words.iter().any(|word| {
        is_token_pt(word)
            || is_card_type(word)
            || matches!(
                *word,
                "token"
                    | "tokens"
                    | "legendary"
                    | "white"
                    | "blue"
                    | "black"
                    | "red"
                    | "green"
                    | "colorless"
            )
    }) {
        return None;
    }

    let raw = render_token_slice(prefix);
    let titled = title_case_phrase_preserving_punctuation(raw.trim());
    (!titled.is_empty()).then_some(titled)
}

pub(super) fn referenced_card_name(tokens: &[OwnedLexToken]) -> Option<String> {
    let (_, name_tokens, _) = primitives::find_prefix(tokens, parse_referenced_card_name)?;
    let raw_name = render_token_slice(&name_tokens);
    let titled = title_case_phrase_preserving_punctuation(raw_name.as_str());
    (!titled.is_empty()).then_some(titled)
}

fn parse_referenced_card_name<'a>()
-> impl Parser<LexStream<'a>, Vec<OwnedLexToken>, winnow::error::ErrMode<winnow::error::ContextError>>
{
    |input: &mut LexStream<'a>| -> WResult<Vec<OwnedLexToken>> {
        primitives::phrase(&["card", "named"]).parse_next(input)?;
        repeat(
            1..,
            any.verify(|token: &&OwnedLexToken| !token.is_any_word(NAMED_CARD_STOP_WORDS))
                .map(|token: &OwnedLexToken| token.clone()),
        )
        .parse_next(input)
    }
}

#[path = "names/library_programs.rs"]
mod library_programs;
pub(super) use library_programs::graveyard_anthem_card_name;
#[path = "names/core_programs.rs"]
mod core_programs;
pub(super) use core_programs::{
    creature_surface_name, leading_explicit_name, leading_name_phrase, vehicle_surface_name,
};
#[path = "names/condition_programs.rs"]
mod condition_programs;
pub(super) use condition_programs::artifact_surface_name;
