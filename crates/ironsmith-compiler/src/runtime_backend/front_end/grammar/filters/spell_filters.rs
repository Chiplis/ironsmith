use super::*;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);

pub(crate) fn parse_object_filter_with_grammar_entrypoint(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    parse_object_filter(tokens, other)
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> ObjectFilter {
    let words_view = GrammarFilterNormalizedWords::new(tokens);
    let words = non_article_word_refs(&words_view.to_word_refs());

    parse_spell_filter_from_words(&words)
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let words = non_article_token_word_refs(tokens);

    parse_spell_filter_from_words(&words)
}

pub(super) fn parse_meld_subject_filter(words: &[&str]) -> Result<ObjectFilter, CardTextError> {
    if words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing meld predicate subject".to_string(),
        ));
    }
    if is_source_reference_words(words) {
        return Ok(ObjectFilter::source());
    }

    let tokens = synth_words_as_tokens(words);
    parse_object_filter(&tokens, false)
        .or_else(|_| Ok(ObjectFilter::default().named(words.join(" "))))
}

pub(super) fn is_plausible_meld_subject_start(word: &str) -> bool {
    matches!(
        word,
        "a" | "an"
            | "another"
            | "this"
            | "that"
            | "source"
            | "artifact"
            | "battle"
            | "card"
            | "creature"
            | "enchantment"
            | "land"
            | "nonland"
            | "permanent"
            | "planeswalker"
    )
}

pub(super) fn find_meld_subject_split(words: &[&str]) -> Option<usize> {
    words
        .windows(2)
        .position(|window| {
            AND_WORD_PATTERN.matches_word(window[0]) && is_plausible_meld_subject_start(window[1])
        })
        .or_else(|| find_index(words, |word| AND_WORD_PATTERN.matches_word(word)))
}
