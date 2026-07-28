use winnow::prelude::*;

use crate::cards::builders::LibraryConsultStopRuleAst;
use crate::effect::{EventValueSpec, Value};
use crate::runtime_backend::front_end::grammar::{permission_shapes, primitives, values};
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, TokenWordView};

use super::{ConsultTraversalStopKind, ConsultTraversalStopShape, trim_commas};

fn prior_action_count_value(words: &[&str]) -> Option<Value> {
    let basis = words
        .strip_prefix(&["the", "number", "of"])
        .or_else(|| words.strip_prefix(&["number", "of"]))
        .unwrap_or(words);
    let mut for_each_words = Vec::with_capacity(basis.len() + 2);
    for_each_words.extend(["for", "each"]);
    for_each_words.extend_from_slice(basis);
    let (value, used) =
        crate::runtime_backend::front_end::grammar::shared_util::count_shapes::
            parse_for_each_count_value_words(&for_each_words)?;
    (used == for_each_words.len()).then_some(value)
}

pub(super) fn parse_equal_to_counted_active_stop(
    tokens: &[OwnedLexToken],
) -> Option<ConsultTraversalStopShape> {
    let (filter_tokens, count_tokens) = primitives::split_lexed_once_on_separator(tokens, || {
        primitives::phrase(&["equal", "to"]).void()
    })?;
    let (_, filter_tokens) =
        primitives::parse_prefix(filter_tokens, |input: &mut LexStream<'_>| {
            primitives::any_phrase(&[&["a", "number", "of"], &["number", "of"]])
                .void()
                .parse_next(input)
        })?;
    let filter_tokens = trim_commas(filter_tokens);
    let count_tokens = trim_commas(count_tokens);
    if filter_tokens.is_empty() || count_tokens.is_empty() {
        return None;
    }

    let count_words = TokenWordView::new(count_tokens).word_refs();
    let count = if permission_shapes::suffix_words(&count_words, &["sacrificed", "this", "way"]) {
        prior_action_count_value(&count_words).unwrap_or(Value::EventValue(EventValueSpec::Amount))
    } else {
        let (value, used) = values::parse_value_prefix_lexed(count_tokens)?;
        (used == count_tokens.len()).then_some(value)?
    };

    Some(ConsultTraversalStopShape {
        stop_rule: LibraryConsultStopRuleAst::MatchCount(count),
        max_exposed: None,
        filter: filter_tokens.to_vec(),
        kind: ConsultTraversalStopKind::Active,
    })
}
