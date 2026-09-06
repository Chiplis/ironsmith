use crate::cards::builders::ConditionalEffectAst;
use super::*;

/// Parse the coordinated conditional animation used by effects such as
/// "that permanent becomes saddled if it's a Mount and becomes an artifact
/// creature if it's a Vehicle".  The ordinary trailing-if splitter cannot
/// consume this shape because the first predicate is followed by another
/// effect rather than the end of the clause.
pub fn parse_conditional_become_pair(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some((verb, _)) = find_verb(tokens) else {
        return Ok(None);
    };
    if verb != Verb::Become {
        return Ok(None);
    }

    parse_conditional_become_pair_impl(tokens)
}

pub(super) fn parse_conditional_become_pair_impl(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_clause_subject_verb_shape(tokens) else {
        return Ok(None);
    };

    let words = parser_token_word_positions(shape.action_tokens);
    let Some(first_if_idx) = words
        .iter()
        .find_map(|(idx, word)| (*word == "if").then_some(*idx))
    else {
        return Ok(None);
    };
    let Some(and_idx) = words
        .iter()
        .find_map(|(idx, word)| (*idx > first_if_idx && *word == "and").then_some(*idx))
    else {
        return Ok(None);
    };
    let Some(second_become_idx) = words
        .iter()
        .find_map(|(idx, word)| (*idx > and_idx && *word == "becomes").then_some(*idx))
    else {
        return Ok(None);
    };
    let Some(second_if_idx) = words
        .iter()
        .find_map(|(idx, word)| (*idx > second_become_idx && *word == "if").then_some(*idx))
    else {
        return Ok(None);
    };

    let first_body = trim_lexed_commas(&shape.action_tokens[..first_if_idx]);
    let first_predicate_tokens = trim_lexed_commas(&shape.action_tokens[first_if_idx + 1..and_idx]);
    let second_body = trim_lexed_commas(&shape.action_tokens[second_become_idx + 1..second_if_idx]);
    let second_predicate_tokens = trim_lexed_commas(&shape.action_tokens[second_if_idx + 1..]);
    if first_body.is_empty()
        || first_predicate_tokens.is_empty()
        || second_body.is_empty()
        || second_predicate_tokens.is_empty()
    {
        return Ok(None);
    }

    let first_predicate = parse_predicate_with_grammar_entrypoint_lexed(first_predicate_tokens)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported conditional become predicate (clause: '{}')",
                render_lower_words(first_predicate_tokens)
            ))
        })?;
    let second_predicate = parse_predicate_with_grammar_entrypoint_lexed(second_predicate_tokens)
        .map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported conditional become predicate (clause: '{}')",
            render_lower_words(second_predicate_tokens)
        ))
    })?;

    let first_effect = parse_become_clause(shape.subject_tokens, first_body)?;
    let second_effect = parse_become_clause(shape.subject_tokens, second_body)?;
    Ok(Some(EffectAst::Sequence {
        effects: vec![
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: first_predicate,
                if_true: vec![first_effect],
                if_false: Vec::new(),
            }),
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: second_predicate,
                if_true: vec![second_effect],
                if_false: Vec::new(),
            }),
        ],
    }))
}
