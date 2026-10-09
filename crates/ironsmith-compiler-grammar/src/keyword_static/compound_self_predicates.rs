//! "Alexios attacks each combat if able, can't be sacrificed, and can't attack
//! its owner.": one source subject shared by a serial list of complete static
//! predicates. Each predicate is its own static ability (CR 113.6); the line
//! is read only when every "<subject> <predicate>" member is itself a complete
//! static line, so no member is dropped or approximated.

use super::*;
use std::cell::Cell;

thread_local! {
    static SPLITTING_COMPOUND_PREDICATES: Cell<bool> = const { Cell::new(false) };
}

struct SplitGuard {
    previous: bool,
}

impl SplitGuard {
    fn set() -> Self {
        Self {
            previous: SPLITTING_COMPOUND_PREDICATES.with(|flag| flag.replace(true)),
        }
    }
}

impl Drop for SplitGuard {
    fn drop(&mut self) {
        let previous = self.previous;
        SPLITTING_COMPOUND_PREDICATES.with(|flag| flag.set(previous));
    }
}

const PREDICATE_HEADS: &[&str] = &[
    "attacks", "blocks", "can't", "cant", "can", "has", "gets", "is", "doesn't", "doesnt",
];

fn is_predicate_head(token: &OwnedLexToken) -> bool {
    token.is_any_word(PREDICATE_HEADS)
}

pub fn parse_compound_self_predicate_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if SPLITTING_COMPOUND_PREDICATES.with(Cell::get) {
        return Ok(None);
    }
    let tokens = trim_edge_punctuation(tokens);
    if tokens.iter().any(|token| token.is_period() || token.is_quote()) {
        return Ok(None);
    }
    // The subject is a reference to this object and ends at the first
    // predicate verb.
    let Some(subject_end) = tokens.iter().position(is_predicate_head) else {
        return Ok(None);
    };
    let subject = &tokens[..subject_end];
    let subject_words = crate::lexer::token_word_refs(subject);
    if subject.is_empty() || !crate::util::is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    // Members are separated by a comma or by "and" before another predicate
    // verb; an "or" inside a member ("its owner or planeswalkers ...") stays.
    let mut members: Vec<&[OwnedLexToken]> = Vec::new();
    let mut start = subject_end;
    let mut idx = subject_end;
    while idx < tokens.len() {
        let token = &tokens[idx];
        let comma = token.kind == TokenKind::Comma;
        let and_before_predicate = token.is_word("and")
            && tokens.get(idx + 1).is_some_and(is_predicate_head);
        if comma || and_before_predicate {
            if start < idx {
                members.push(&tokens[start..idx]);
            }
            start = idx + 1;
            // ", and can't ...": the conjunction after a comma belongs to it.
            if comma
                && tokens.get(idx + 1).is_some_and(|next| next.is_word("and"))
                && tokens.get(idx + 2).is_some_and(is_predicate_head)
            {
                start = idx + 2;
                idx += 1;
            }
        }
        idx += 1;
    }
    if start < tokens.len() {
        members.push(&tokens[start..]);
    }
    if members.len() < 2 || members.iter().any(|member| !is_predicate_head(&member[0])) {
        return Ok(None);
    }
    let _guard = SplitGuard::set();
    // A last resort: a rule that already reads the whole line ("This
    // creature has flying and can't block" grant forms) keeps it, so the
    // registry never sees two different complete readings.
    if matches!(parse_static_ability_ast_line_lexed(&tokens), Ok(Some(_))) {
        return Ok(None);
    }
    let mut abilities = Vec::new();
    for member in members {
        let mut line = subject.to_vec();
        line.extend_from_slice(member);
        match parse_static_ability_ast_line_lexed(&line) {
            Ok(Some(parsed)) if !parsed.is_empty() => abilities.extend(parsed),
            _ => return Ok(None),
        }
    }
    Ok(Some(abilities))
}
