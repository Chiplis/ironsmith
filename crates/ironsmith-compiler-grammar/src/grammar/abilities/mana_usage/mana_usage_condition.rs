use super::*;

pub(super) fn classify_spec(tokens: &[OwnedLexToken]) -> ManaUsageSpecShape {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    while let Ok(word) = take_word(&mut input) {
        if crate::slice_primitives::contains(UNSUPPORTED_SPEC_WORDS, &word) {
            return ManaUsageSpecShape::Unsupported;
        }
    }
    let mut input: primitives::WordSliceInput<'_> = &words;
    while let Ok(word) = take_word(&mut input) {
        if !crate::slice_primitives::contains(PLAIN_SPELL_WORDS, &word) {
            return ManaUsageSpecShape::Other;
        }
    }
    ManaUsageSpecShape::PlainSpell
}
