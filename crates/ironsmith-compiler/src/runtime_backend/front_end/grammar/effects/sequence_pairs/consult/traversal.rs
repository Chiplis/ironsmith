use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::{LibraryConsultModeAst, LibraryConsultStopRuleAst};
use crate::effect::{EventValueSpec, Value};
use crate::runtime_backend::front_end::grammar::{
    leaf, permission_shapes, primitives, sentence_markers, values,
};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView,
};

use super::super::{
    ends_content_sequence, seek_sequence_phrase, sequence_any_phrase, sequence_phrase,
    starts_content_sequence,
};

#[path = "traversal/shapes.rs"]
mod shapes;
pub(crate) use shapes::*;
#[path = "traversal/counted_stop.rs"]
mod counted_stop;
use counted_stop::parse_equal_to_counted_active_stop;

const REVEAL_VERBS: &[&[&str]] = &[&["reveal"], &["reveals"]];
const EXILE_VERBS: &[&[&str]] = &[&["exile"], &["exiles"]];
const CONSULT_VERBS: &[&[&str]] = &[&["reveal"], &["reveals"], &["exile"], &["exiles"]];

fn trim_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0;
    let mut end = tokens.len();
    while start < end && tokens[start].kind == TokenKind::Comma {
        start += 1;
    }
    while end > start && tokens[end - 1].kind == TokenKind::Comma {
        end -= 1;
    }
    &tokens[start..end]
}

fn find_phrase_span(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> Option<std::ops::Range<usize>> {
    let mut input = LexStream::new(tokens);
    let start = seek_sequence_phrase(&mut input, alternatives).ok()?;
    sequence_any_phrase(alternatives)
        .parse_next(&mut input)
        .ok()?;
    let end = tokens.len().saturating_sub(input.len());
    Some(start..end)
}

fn first_comma(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    while !input.is_empty() {
        let offset = initial_len.saturating_sub(input.len());
        let parsed: winnow::error::ModalResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        if token.kind == TokenKind::Comma {
            return Some(offset);
        }
    }
    None
}

fn parse_where_x_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let ((), value_tokens) = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        primitives::phrase(&["where", "x", "is"])
            .void()
            .parse_next(input)
    })?;
    let value_tokens = trim_commas(value_tokens);
    let value_tokens = primitives::parse_prefix(value_tokens, |input: &mut LexStream<'_>| {
        primitives::phrase(&["the", "number", "of"])
            .void()
            .parse_next(input)
    })
    .map(|(_, rest)| trim_commas(rest))
    .unwrap_or(value_tokens);
    let words = TokenWordView::new(value_tokens).word_refs();
    if let Some(value) = crate::runtime_backend::front_end::grammar::shared_util::value_helper_shapes::parse_aggregate_scope_value_words(&words) {
        return Some(value);
    }
    let (value, used) = values::parse_value_prefix_lexed(value_tokens)?;
    TokenWordView::new(&value_tokens[used..])
        .is_empty()
        .then_some(value)
}

fn parse_consult_trailing(tokens: &[OwnedLexToken]) -> (Option<Value>, Vec<OwnedLexToken>) {
    let tokens = trim_commas(tokens);
    if tokens.is_empty() {
        return (None, Vec::new());
    }
    let where_clause_end = first_comma(tokens).unwrap_or(tokens.len());
    let where_clause = trim_commas(&tokens[..where_clause_end]);
    let Some(where_x) = parse_where_x_value(where_clause) else {
        return (None, tokens.to_vec());
    };
    let trailing_effect = if where_clause_end < tokens.len() {
        trim_commas(&tokens[where_clause_end + 1..]).to_vec()
    } else {
        Vec::new()
    };
    (Some(where_x), trailing_effect)
}

fn consult_verb(
    tokens: &[OwnedLexToken],
) -> Option<(std::ops::Range<usize>, LibraryConsultModeAst)> {
    let reveal = find_phrase_span(tokens, REVEAL_VERBS);
    let exile = find_phrase_span(tokens, EXILE_VERBS);
    match (reveal, exile) {
        (Some(reveal), Some(exile)) if reveal.start <= exile.start => {
            Some((reveal, LibraryConsultModeAst::Reveal))
        }
        (Some(_), Some(exile)) => Some((exile, LibraryConsultModeAst::Exile)),
        (Some(reveal), None) => Some((reveal, LibraryConsultModeAst::Reveal)),
        (None, Some(exile)) => Some((exile, LibraryConsultModeAst::Exile)),
        (None, None) => None,
    }
}

fn split_prefix_and_consult(
    tokens: &[OwnedLexToken],
) -> Option<(Option<Vec<OwnedLexToken>>, &[OwnedLexToken])> {
    let tokens = trim_commas(tokens);
    let tokens = sentence_markers::parse_conditional_followup_tokens(tokens)
        .map(|matched| trim_commas(matched.tail_tokens))
        .unwrap_or(tokens);
    if tokens.is_empty() {
        return None;
    }

    let Some(then) = find_phrase_span(tokens, &[&["then"]]) else {
        return Some((None, tokens));
    };
    let consult = trim_commas(&tokens[then.end..]);
    if consult.is_empty() {
        return None;
    }
    let is_consult_traversal = consult_verb(consult)
        .and_then(|(verb, _)| {
            find_phrase_span(consult, &[&["until"]]).map(|until| until.start > verb.end)
        })
        .unwrap_or(false);
    if !is_consult_traversal {
        return Some((None, tokens));
    }
    if then.start == 0 {
        return Some((None, consult));
    }
    let prefix = trim_commas(&tokens[..then.start]);
    if prefix.is_empty() {
        return None;
    }
    Some((Some(prefix.to_vec()), consult))
}

fn counted_stop_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(LibraryConsultStopRuleAst, &[OwnedLexToken])> {
    if let Some(((), rest)) = primitives::parse_prefix(tokens, sequence_phrase(&["that", "many"])) {
        return Some((
            LibraryConsultStopRuleAst::MatchCount(Value::EventValue(EventValueSpec::Amount)),
            trim_commas(rest),
        ));
    }
    if let Some((count, rest)) =
        primitives::parse_prefix(tokens, leaf::parse_leaf_number_prefix_lexed)
    {
        return Some((
            LibraryConsultStopRuleAst::MatchCount(Value::Fixed(count as i32)),
            trim_commas(rest),
        ));
    }
    let (value, used) = values::parse_value_prefix_lexed(tokens)?;
    Some((
        LibraryConsultStopRuleAst::MatchCount(value),
        trim_commas(&tokens[used..]),
    ))
}

fn parse_passive_stop(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<ConsultTraversalStopShape> {
    let tokens = trim_commas(tokens);
    let (count, rest) = if let Some((count, rest)) =
        primitives::parse_prefix(tokens, leaf::parse_leaf_number_prefix_lexed)
    {
        (Value::Fixed(count as i32), rest)
    } else {
        let (value, used) = values::parse_value_prefix_lexed(tokens)?;
        (value, &tokens[used..])
    };
    let tail = trim_commas(rest);
    let words = TokenWordView::new(tail);
    let word_refs = words.word_refs();
    let suffixes: &[&[&str]] = match mode {
        LibraryConsultModeAst::Reveal => &[
            &["cards", "are", "revealed"],
            &["card", "is", "revealed"],
            &["is", "revealed"],
        ],
        LibraryConsultModeAst::Exile => &[
            &["cards", "are", "exiled"],
            &["card", "is", "exiled"],
            &["is", "exiled"],
        ],
    };
    let suffix_len = suffixes.iter().find_map(|suffix| {
        permission_shapes::suffix_words(&word_refs, suffix).then_some(suffix.len())
    })?;
    let filter_word_count = words.len().saturating_sub(suffix_len);
    let filter_end = words
        .token_index_after_words(filter_word_count)
        .unwrap_or(tail.len());
    Some(ConsultTraversalStopShape {
        stop_rule: LibraryConsultStopRuleAst::MatchCount(count),
        max_exposed: None,
        filter: trim_commas(&tail[..filter_end]).to_vec(),
        kind: ConsultTraversalStopKind::Passive,
    })
}

fn parse_first_match_or_exposed_count_stop(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<ConsultTraversalStopShape> {
    let (match_tokens, count_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("or").void())?;
    let match_tokens = trim_commas(match_tokens);
    if match_tokens.is_empty() {
        return None;
    }
    let count_stop = parse_passive_stop(count_tokens, mode)?;
    if !count_stop.filter.is_empty() || count_stop.max_exposed.is_some() {
        return None;
    }
    let LibraryConsultStopRuleAst::MatchCount(max_exposed) = count_stop.stop_rule else {
        return None;
    };
    Some(ConsultTraversalStopShape {
        stop_rule: LibraryConsultStopRuleAst::FirstMatch,
        max_exposed: Some(max_exposed),
        filter: match_tokens.to_vec(),
        kind: ConsultTraversalStopKind::Passive,
    })
}

fn parse_active_stop(tokens: &[OwnedLexToken]) -> Option<ConsultTraversalStopShape> {
    let tokens = trim_commas(tokens);
    let verb = find_phrase_span(tokens, CONSULT_VERBS)?;
    if verb.start == 0 {
        return None;
    }
    let filter = trim_commas(&tokens[verb.end..]);
    if filter.is_empty() {
        return None;
    }
    if let Some(stop) = parse_equal_to_counted_active_stop(filter) {
        return Some(stop);
    }
    let (stop_rule, filter) = counted_stop_prefix(filter)
        .filter(|(_, filter)| !filter.is_empty())
        .unwrap_or((LibraryConsultStopRuleAst::FirstMatch, filter));
    Some(ConsultTraversalStopShape {
        stop_rule,
        max_exposed: None,
        filter: filter.to_vec(),
        kind: ConsultTraversalStopKind::Active,
    })
}

pub(crate) fn parse_consult_traversal_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConsultTraversalShape> {
    let (prefix, consult) = split_prefix_and_consult(tokens)?;
    let (verb, mode) = consult_verb(consult)?;
    if crate::runtime_backend::front_end::grammar::effects::for_each_shapes::parse_for_each_object_effect_shape(consult)
        .is_some()
    {
        // The iteration header owns this sentence.  Its payload is parsed as a
        // consult only after the outer typed for-each shape has bound `it`.
        return None;
    }
    let player = if verb.start == 0 {
        ConsultTraversalPlayerShape::ImpliedByPrefixOrYou
    } else if permission_shapes::exact_tokens(&consult[..verb.start], &["they"]) {
        ConsultTraversalPlayerShape::ThatPlayer
    } else {
        ConsultTraversalPlayerShape::Subject(trim_commas(&consult[..verb.start]).to_vec())
    };

    let until = find_phrase_span(consult, &[&["until"]])?;
    if until.start <= verb.end {
        return None;
    }
    let library_head = &consult[verb.end..until.start];
    if !starts_content_sequence(library_head, &[&["cards", "from", "top", "of"]])
        || !ends_content_sequence(library_head, &[&["library"]])
    {
        return None;
    }

    let mut stop_tokens = trim_commas(&consult[until.end..]);
    let (where_x, mut trailing_effect) = if let Some(comma) = first_comma(stop_tokens) {
        let trailing = trim_commas(&stop_tokens[comma + 1..]);
        stop_tokens = trim_commas(&stop_tokens[..comma]);
        parse_consult_trailing(trailing)
    } else {
        (None, Vec::new())
    };
    let stop = parse_first_match_or_exposed_count_stop(stop_tokens, mode)
        .or_else(|| parse_passive_stop(stop_tokens, mode))
        .or_else(|| parse_active_stop(stop_tokens))?;
    if stop.max_exposed.is_some()
        && permission_shapes::exact_tokens(&trailing_effect, &["whichever", "comes", "first"])
    {
        trailing_effect.clear();
    }
    Some(ConsultTraversalShape {
        prefix,
        player,
        mode,
        stop,
        where_x,
        trailing_effect,
    })
}

#[cfg(test)]
#[path = "traversal/tests.rs"]
mod tests;
