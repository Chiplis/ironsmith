use super::*;

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
    let words: Vec<&str> = words_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    parse_spell_filter_from_words(&words)
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let words: Vec<&str> = crate::cards::builders::compiler::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

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
        .iter()
        .enumerate()
        .find_map(|(idx, word)| {
            (*word == "and"
                && words
                    .get(idx + 1)
                    .is_some_and(|next| is_plausible_meld_subject_start(next)))
            .then_some(idx)
        })
        .or_else(|| find_index(words, |word| *word == "and"))
}
