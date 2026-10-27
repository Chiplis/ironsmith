//! Complete non-resolving instructions owned by one parsed ability line.
use super::{primitives, structure};
use crate::lexer::OwnedLexToken;
use winnow::Parser;
use winnow::combinator::alt;

/// Remove only complete, unquoted instruction sentences. The original line's
/// semantic facts retain the rule; no executable no-op replaces the sentence.
pub fn remove_cant_be_countered_instruction(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut removed = vec![false; tokens.len()];
    let mut found = false;
    for sentence in structure::split_lexed_sentences(tokens) {
        let Some(first) = sentence.first() else {
            continue;
        };
        let Some(start) = tokens.iter().position(|token| std::ptr::eq(token, first)) else {
            continue;
        };
        let quoted = tokens[..start]
            .iter()
            .filter(|token| token.is_quote())
            .count()
            % 2
            != 0;
        if quoted || sentence.iter().any(|token| token.is_quote()) {
            continue;
        }
        let parser = (
            primitives::kw("this"),
            primitives::kw("ability"),
            alt((
                primitives::kw("can't").void(),
                primitives::kw("cant").void(),
                primitives::kw("cannot").void(),
                (primitives::kw("can"), primitives::kw("not")).void(),
            )),
            primitives::kw("be"),
            primitives::kw("countered"),
        )
            .void();
        if primitives::probe_all(
            sentence,
            parser,
            "complete ability counter protection instruction",
        )
        .is_none()
        {
            continue;
        }
        let mut end = start + sentence.len();
        if tokens.get(end).is_some_and(|token| token.is_period()) {
            end += 1;
        }
        removed[start..end].fill(true);
        found = true;
    }
    found.then(|| {
        tokens
            .iter()
            .zip(removed)
            .filter_map(|(token, removed)| (!removed).then(|| token.clone()))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, render_token_slice};

    #[test]
    fn instructions_are_whole_sentences_and_preserve_other_instructions() {
        for suffix in [
            "This ability can't be countered.",
            "This ability cannot be countered.",
            "This ability can not be countered.",
        ] {
            let tokens = lex_line(
                &format!("Whenever this becomes tapped, you gain 3 life. {suffix}"),
                0,
            )
            .unwrap();
            let body = remove_cant_be_countered_instruction(&tokens).unwrap();
            assert_eq!(
                render_token_slice(&body),
                "Whenever this becomes tapped, you gain 3 life."
            );
        }
    }

    #[test]
    fn a_quoted_child_instruction_does_not_protect_its_parent() {
        let tokens = lex_line("Whenever this becomes tapped, it gains \"Whenever this attacks, draw a card. This ability can't be countered.\"", 0).unwrap();
        assert!(remove_cant_be_countered_instruction(&tokens).is_none());
        let tokens = lex_line(
            "Whenever this becomes tapped, this ability can't be countered until end of turn.",
            0,
        )
        .unwrap();
        assert!(remove_cant_be_countered_instruction(&tokens).is_none());
    }
}
