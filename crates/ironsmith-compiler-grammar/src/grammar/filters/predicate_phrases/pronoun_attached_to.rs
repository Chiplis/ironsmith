//! "if it's attached to a creature you control" (Stolen Uniform): the
//! referenced object is attached to an object matching the filter. Without
//! this reading the generic copula path kept only "a creature you control"
//! and silently dropped the attachment relation.
use super::*;

pub(super) fn parse_pronoun_attached_to_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let starts = view.token_start_indices();
    let object_word = if words.get(..3) == Some(&["its", "attached", "to"][..]) {
        3
    } else if words.get(..4) == Some(&["it", "is", "attached", "to"][..]) {
        4
    } else {
        return Ok(None);
    };
    let Some(&object_token) = starts.get(object_word) else {
        return Ok(None);
    };
    let Some(attached_to) = crate::grammar::primitives::probe_shape(parse_object_filter(
        &tokens[object_token..],
        false,
    )) else {
        return Ok(None);
    };
    let filter = ObjectFilter {
        attached_to_object: Some(Box::new(attached_to)),
        ..Default::default()
    };
    Ok(Some(PredicateAst::ItMatches(filter)))
}
