use super::*;

pub fn parse_sacrifice_object_shape(tokens: &[OwnedLexToken]) -> SacrificeObjectShape<'_> {
    let filter_tokens = primitives::strip_lexed_suffix_phrases(tokens, CHOICE_SUFFIXES)
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
    let words = parser_token_word_refs(filter_tokens);
    let tagged_reference = if common::exact(&words, &["that", "token"]) {
        Some(SacrificeTaggedReferenceKind::Token)
    } else if common::exact_any(&words, ONE_OF_TAGGED_SET_REFERENCES) {
        Some(SacrificeTaggedReferenceKind::OneOfTaggedSet)
    } else if common::exact_any(&words, ALL_OF_TAGGED_SET_REFERENCES) {
        Some(SacrificeTaggedReferenceKind::AllOfTaggedSet)
    } else if common::exact_any(&words, TAGGED_REFERENCES) {
        Some(SacrificeTaggedReferenceKind::ItOrCard)
    } else {
        None
    };
    if tagged_reference.is_some() {
        // The whole object phrase is the back-reference ("it", "that token",
        // "one of them", "those creatures").
        crate::word_claims::claim_words(&words);
    }
    SacrificeObjectShape {
        filter_tokens,
        tagged_reference,
    }
}
