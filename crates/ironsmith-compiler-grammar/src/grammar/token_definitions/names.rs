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
    named_card_name_parts(&common::token_description_projection(tokens).outer).map(|(name, _)| name)
}

pub(super) fn named_card_name_parts(tokens: &[OwnedLexToken]) -> Option<(String, std::ops::Range<usize>)> {
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
        && (end == named_idx + 1 || !NAMED_CARD_STOP_WORDS
            .iter()
            .any(|candidate| *candidate == pieces[end].text.as_str()))
    {
        end += 1;
    }
    if end <= named_idx + 1 {
        return None;
    }

    let clause_start = pieces[named_idx].span.start;
    let mut name_end = pieces[end - 1].span.end;
    // Rewritten tokens can share source spans with terminal punctuation.
    // Ownership follows lexical token boundaries, not span overlap: the
    // sentence's final period is not part of an unquoted proper name.
    let view = crate::lexer::TokenWordView::new(tokens);
    let token_start = view.token_start_indices()[named_idx + 1];
    // `token_index_after_words(end)` points at the next WORD (or the
    // entire slice end), so it also includes a period before that word.
    // Own through the final name word's lexical token, not its successor.
    let last_name_token = *view.token_start_indices().get(end.checked_sub(1)?)?;
    let mut token_end = last_name_token + 1;
    // An explicitly quoted name may itself end in punctuation. Preserve the
    // complete authored interior rather than trimming the rendered string.
    if token_start > 0 && tokens[token_start - 1].is_quote() {
        if let Some(close) = (token_start..tokens.len()).find(|index| tokens[*index].is_quote()) {
            token_end = close;
            name_end = tokens[close].span.end;
        }
    }
    let name_tokens = tokens[token_start..token_end].to_vec();
    if !name_tokens.is_empty() {
        let raw_name = render_token_slice(&name_tokens);
        let titled = title_case_phrase_preserving_punctuation(raw_name.as_str());
        if !titled.is_empty() {
            return Some((titled, clause_start..name_end));
        }
    }

    Some((title_case_words(&piece_words[named_idx + 1..end]), clause_start..name_end))
}

/// Parses the named-token template whose proper name precedes a comma, as in
/// `Tamiyo's Notebook, a legendary ... token`. Keeping the original token
/// slice here is important: parser words intentionally normalize apostrophes,
/// but the token's runtime/display name must retain them.
/// Preserve and title-case a proper token name that precedes its appositive
/// definition (`Name, Epithet, a legendary ... token`).
pub fn leading_appositive_token_name(tokens: &[OwnedLexToken]) -> Option<String> {
    leading_appositive_token_parts(&common::token_description_projection(tokens).outer).map(|(name, _)| name)
}

pub(super) fn leading_appositive_token_parts(tokens: &[OwnedLexToken]) -> Option<(String, &[OwnedLexToken])> {
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
    if matches!(first, "a" | "an") || is_token_pt(first) {
        return None;
    }

    let raw = render_token_slice(prefix);
    let titled = title_case_phrase_preserving_punctuation(raw.trim());
    (!titled.is_empty()).then_some((titled, &tokens[comma + 1..]))
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

#[path = "names/library.rs"]
mod library_programs;
pub(super) use library_programs::graveyard_anthem_card_name;
#[path = "names/core.rs"]
mod core_programs;
pub(super) use core_programs::{
    creature_surface_name, leading_explicit_name, leading_name_phrase, vehicle_surface_name,
};
#[path = "names/condition.rs"]
mod condition_programs;
pub(super) use condition_programs::artifact_surface_name;

#[cfg(test)]
mod explicit_name_token_boundary_tests {
    use super::*;
    #[test]
    fn terminal_sentence_period_is_not_owned_by_named_token_even_with_overlapping_spans(){
        let mut tokens=crate::util::tokenize_line("a colorless artifact token named Keepsake.",0);
        let last=tokens.len()-1;assert!(tokens[last].is_period());
        tokens[last].span=tokens[last-1].span;
        assert_eq!(named_card_name_parts(&tokens).unwrap().0,"Keepsake");
    }
    #[test]
    fn internal_and_explicit_quoted_punctuation_are_preserved(){
        for (text,expected) in [("a token named Canvas, Dusk.","Canvas, Dusk"),("a token named \"Keepsake.\".","Keepsake.")]{
            let tokens=crate::util::tokenize_line(text,0);
            assert_eq!(named_card_name_parts(&tokens).unwrap().0,expected);
        }
    }
}

#[cfg(test)]
mod quoted_name_complete_shape_tests {
    #[test]
    fn complete_token_shape_owns_quoted_stop_words_as_name_not_rule(){
        let tokens=crate::util::tokenize_line("a colorless artifact token named \"Sage with Wings\".",0);
        let shape=crate::grammar::token_definitions::parse_token_definition_shape_tokens(&tokens).unwrap();
        let crate::model::token_definition::TokenDefinitionSpec::Artifact(shape)=shape else{panic!("named artifact descriptor changed shape")};
        assert_eq!(shape.name,"Sage with Wings");
        assert!(shape.token_rules.parsed_abilities.is_empty() && shape.token_rules.unread_rules.is_empty());
    }
}

#[cfg(test)]
mod named_token_sentence_boundary_regressions {
    use super::*;
    #[test]
    fn next_word_index_does_not_own_the_terminal_period_or_next_instruction(){
        for text in ["a colorless artifact token named Keepsake.","a colorless artifact token named Record. Draw a card."] {
            let tokens=crate::util::tokenize_line(text,0);
            let (name,_) = named_card_name_parts(&tokens).unwrap();
            assert_eq!(name,if text.contains("Keepsake"){ "Keepsake" }else{"Record"});
        }
    }
    #[test]
    fn quoted_terminal_punctuation_and_internal_punctuation_remain_name_data(){
        for (text,expected) in [(r#"a token named "Record."."#,"Record."),("a token named Mr. Record.","Mr. Record"),("a token named Canvas, Dusk.","Canvas, Dusk")] {
            let tokens=crate::util::tokenize_line(text,0);assert_eq!(named_card_name_parts(&tokens).unwrap().0,expected);
        }
    }
}
