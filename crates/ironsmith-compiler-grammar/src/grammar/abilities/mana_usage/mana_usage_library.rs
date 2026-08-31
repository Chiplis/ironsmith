use super::*;

pub(super) fn parse_simple_bonus_card_type(tokens: &[OwnedLexToken]) -> Option<CardType> {
    let tokens = strip_article(tokens);
    let words = TokenWordView::new(tokens).word_refs();
    let [word] = words.as_slice() else {
        return None;
    };
    crate::grammar::primitives::probe_shape(leaf::parse_leaf_card_type_complete(word))
}
