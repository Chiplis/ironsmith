use winnow::combinator::repeat;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::InsteadSemantics;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

const THE_NEXT_TIME: &[&str] = &["the", "next", "time"];
const COUNTERED_THIS_WAY: &[&str] = &["countered", "this", "way"];
const INSTEAD_OF: &[&str] = &["instead", "of"];
const GRAVEYARD: &[&str] = &["graveyard"];
const INSTEAD_OF_PUTTING_IT_INTO: &[&str] = &["instead", "of", "putting", "it", "into"];
const INSTEAD_OF_PUTTING_THEM_INTO: &[&str] = &["instead", "of", "putting", "them", "into"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InsteadFollowupShape {
    pub(crate) semantics: InsteadSemantics,
    pub(crate) conditional_intro: bool,
    pub(crate) leading_instead_surface: bool,
}

pub(crate) fn parse_instead_followup_semantics_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<InsteadSemantics> {
    let tokens: Vec<&OwnedLexToken> = repeat(0.., any).parse_next(input)?;
    let words = tokens
        .into_iter()
        .filter_map(|token| token.as_word().map(|_| token.parser_text()))
        .collect::<Vec<_>>();
    Ok(classify_instead_words(&words))
}

pub(crate) fn classify_instead_followup_semantics_tokens(
    tokens: &[OwnedLexToken],
) -> InsteadSemantics {
    primitives::parse_prefix(tokens, parse_instead_followup_semantics_lexed)
        .map(|(semantics, _)| semantics)
        .unwrap_or(InsteadSemantics::NonReplacement)
}

pub(crate) fn parse_instead_followup_shape_tokens(
    tokens: &[OwnedLexToken],
) -> InsteadFollowupShape {
    let leading_instead_surface = tokens.windows(2).any(|pair| {
        (pair[0].is_comma() && pair[1].is_word("instead"))
            || (pair[0].is_word("may") && pair[1].is_word("instead"))
    });
    InsteadFollowupShape {
        semantics: classify_instead_followup_semantics_tokens(tokens),
        conditional_intro: primitives::parse_prefix(tokens, primitives::kw("if")).is_some(),
        leading_instead_surface,
    }
}

fn classify_instead_words(words: &[&str]) -> InsteadSemantics {
    let Some(first_instead) = first_word_offset(words, "instead") else {
        return InsteadSemantics::NonReplacement;
    };

    if first_word_offset(words, "would").is_some_and(|offset| offset < first_instead)
        || grammar_phrase_present(words, THE_NEXT_TIME)
    {
        return InsteadSemantics::FutureReplacement;
    }

    if grammar_phrase_present(words, COUNTERED_THIS_WAY)
        && grammar_phrase_present(words, INSTEAD_OF)
        && grammar_phrase_present(words, GRAVEYARD)
    {
        return InsteadSemantics::FutureReplacement;
    }

    if grammar_phrase_present(words, INSTEAD_OF_PUTTING_IT_INTO)
        || grammar_phrase_present(words, INSTEAD_OF_PUTTING_THEM_INTO)
    {
        return InsteadSemantics::FutureReplacement;
    }

    InsteadSemantics::SelfReplacement
}

fn first_word_offset(words: &[&str], expected: &str) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let parsed: WResult<&str> = any.parse_next(&mut input);
        let word = parsed.ok()?;
        if word == expected {
            return Some(offset);
        }
    }
}

fn grammar_phrase_present(words: &[&str], phrase: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let mut candidate = input;
        if parse_word_phrase(&mut candidate, phrase).is_ok() {
            return true;
        }
        let consumed: WResult<&str> = any.parse_next(&mut input);
        if consumed.is_err() {
            return false;
        }
    }
}

fn parse_word_phrase(input: &mut primitives::WordSliceInput<'_>, phrase: &[&str]) -> WResult<()> {
    if phrase.is_empty() {
        return Err(primitives::backtrack_err(
            "instead phrase",
            "non-empty phrase",
        ));
    }
    for expected in phrase {
        let word: &str = any.parse_next(input)?;
        if word != *expected {
            return Err(primitives::backtrack_err(
                "instead phrase",
                "expected phrase word",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    fn classify(raw: &str) -> InsteadSemantics {
        let tokens = lex_line(raw, 0).unwrap();
        classify_instead_followup_semantics_tokens(&tokens)
    }

    #[test]
    fn distinguishes_non_self_and_future_replacements() {
        assert_eq!(classify("draw a card"), InsteadSemantics::NonReplacement);
        assert_eq!(
            classify("exile it instead"),
            InsteadSemantics::SelfReplacement
        );
        assert_eq!(
            classify("the next time it would die, exile it instead"),
            InsteadSemantics::FutureReplacement
        );
        assert_eq!(
            classify("countered this way, exile it instead of putting it into its graveyard"),
            InsteadSemantics::FutureReplacement
        );
    }

    #[test]
    fn followup_shape_preserves_conditional_intro() {
        let tokens = lex_line("If you do, exile it instead", 0).unwrap();
        let shape = parse_instead_followup_shape_tokens(&tokens);
        assert!(shape.conditional_intro);
        assert!(!shape.leading_instead_surface);
        assert_eq!(shape.semantics, InsteadSemantics::SelfReplacement);
    }

    #[test]
    fn followup_shape_preserves_leading_instead_surface() {
        let tokens = lex_line(
            "Draw a card. If you control an artifact, instead draw two cards.",
            0,
        )
        .unwrap();
        let shape = parse_instead_followup_shape_tokens(&tokens);
        assert!(shape.leading_instead_surface);
        assert_eq!(shape.semantics, InsteadSemantics::SelfReplacement);
    }

    #[test]
    fn followup_shape_preserves_may_instead_before_the_replacement_action() {
        let tokens = lex_line(
            "Look at the top four cards of your library. If you gained life this turn, you may instead reveal two cards.",
            0,
        )
        .unwrap();
        let shape = parse_instead_followup_shape_tokens(&tokens);
        assert!(shape.leading_instead_surface);
        assert_eq!(shape.semantics, InsteadSemantics::SelfReplacement);
    }

    #[test]
    fn unrelated_self_replacement_is_not_a_conditional_intro() {
        let tokens = lex_line(
            "Return target creature to its owner's hand. If it was attacking, exile it instead.",
            0,
        )
        .unwrap();
        let shape = parse_instead_followup_shape_tokens(&tokens);
        assert!(!shape.conditional_intro);
        assert_eq!(shape.semantics, InsteadSemantics::SelfReplacement);
    }
}
