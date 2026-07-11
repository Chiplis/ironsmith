#![allow(dead_code, unused_imports)]

use crate::cards::builders::IfResultPredicate;

use super::lexer::OwnedLexToken;
pub(crate) use super::util::{
    find_activation_cost_start, is_article, non_article_word_refs, replace_unbound_x_with_value,
    starts_with_activation_cost, value_contains_unbound_x,
};

pub(crate) fn parse_if_result_predicate(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    super::grammar::modal_results::parse_if_result_predicate_tokens(tokens)
}

pub(crate) fn parse_if_result_predicate_lexed(
    tokens: &[OwnedLexToken],
) -> Option<IfResultPredicate> {
    super::grammar::modal_results::parse_if_result_predicate_lexed_tokens(tokens)
}
