use super::*;
use crate::cards::builders::PredicateAst;

pub(super) fn parse_activate_only_count_per_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_phrase_words(input, &["activate", "only"])
    })?;
    let start = words.len().checked_sub(input.len())?;
    let shape = parse_count_each_turn_shape(&words, start)?;
    let count_tokens = token_slice_for_words(tokens, &view, shape.count_start, shape.count_end)?;
    let count_words = words.get(shape.count_start..shape.count_end)?;
    let (count, used) = parse_number(count_tokens)?;
    (used == count_words.len()).then_some(PredicateAst::MaxActivationsPerTurn(count))
}

pub(super) fn parse_activate_count_each_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_phrase_words(input, &["activate"])
    })?;
    let start = words.len().checked_sub(input.len())?;
    let shape = parse_count_each_turn_shape(&words, start)?;
    let count_tokens = token_slice_for_words(tokens, &view, shape.count_start, shape.count_end)?;
    let count_words = words.get(shape.count_start..shape.count_end)?;
    let count = crate::grammar::primitives::probe_shape(parse_less_than_or_equal_quantity_prefix(
        count_tokens,
        false,
        false,
        "activation frequency condition",
    ))
    .flatten()
    .and_then(|(count, used)| (used == count_words.len()).then_some(count))?;
    Some(PredicateAst::MaxActivationsPerTurn(count))
}
