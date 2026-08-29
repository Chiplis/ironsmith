use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::{LibraryConsultModeAst, LibraryConsultStopRuleAst};
use crate::effect::{EventValueSpec, Value};
use crate::grammar::{leaf, permission_shapes, primitives, sentence_markers, values};
use crate::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};

use super::super::{
    ends_content_sequence, seek_sequence_phrase, sequence_any_phrase, sequence_phrase,
    starts_content_sequence,
};

#[path = "traversal/shapes.rs"]
mod shapes;
pub use shapes::*;
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

fn comma_starts_explicit_card_filter_union_arm(tokens: &[OwnedLexToken]) -> bool {
    let arm_end = first_comma(tokens).unwrap_or(tokens.len());
    let words = TokenWordView::new(trim_commas(&tokens[..arm_end])).word_refs();
    let noun_start = if crate::word_primitives::first_is_any(&words, &["a", "an"]) {
        0
    } else if crate::word_primitives::parse_choice_sequence_prefix(
        &words,
        &[&["or", "and/or"], &["a", "an"]],
    ) {
        1
    } else {
        return false;
    };
    words
        .get(noun_start..)
        .is_some_and(|arm| arm.iter().any(|word| matches!(*word, "card" | "cards")))
}

/// Find the comma separating a consult stop condition from its inline
/// follow-up without treating commas inside an explicit repeated-card union as
/// clause boundaries.
fn first_consult_trailing_comma(tokens: &[OwnedLexToken]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        (token.kind == TokenKind::Comma
            && !comma_starts_explicit_card_filter_union_arm(&tokens[idx + 1..]))
        .then_some(idx)
    })
}

fn parse_where_x_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let ((), value_tokens) = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        primitives::phrase(&["where", "x", "is"])
            .void()
            .parse_next(input)
    })?;
    let value_tokens = trim_commas(value_tokens);
    // Keep ordinary counted object scopes typed.  The aggregate helper below
    // owns surfaces such as "the number of colors among ...", while consult
    // traversal values also admit the much more common "the number of
    // creatures you control ..." form.
    if let Some(value) = super::parse_consult_condition_value_shape(value_tokens) {
        return Some(value);
    }
    let value_tokens = primitives::parse_prefix(value_tokens, |input: &mut LexStream<'_>| {
        primitives::phrase(&["the", "number", "of"])
            .void()
            .parse_next(input)
    })
    .map(|(_, rest)| trim_commas(rest))
    .unwrap_or(value_tokens);
    let words = TokenWordView::new(value_tokens).word_refs();
    if let Some(value) =
        crate::grammar::shared_util::value_helper_shapes::parse_aggregate_scope_value_words(&words)
    {
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
    // In an active stop condition, a leading indefinite article belongs to
    // the card filter: "until you reveal a creature card" is a first-match
    // stop, not an explicitly counted stop. Keeping the article is also
    // semantically important for repeated complete-noun unions, whose branch
    // scope and canonical list surface depend on every arm retaining `a`/`an`.
    if tokens
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        return None;
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

#[cfg(test)]
#[path = "traversal/tests.rs"]
mod tests;

#[path = "traversal/core_programs.rs"]
mod core_programs;
pub use core_programs::parse_consult_traversal_shape;
#[path = "traversal/library_programs.rs"]
mod library_programs;
use library_programs::{parse_active_stop, parse_matching_filter_or_exposed_count_stop};
