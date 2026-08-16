use std::ops::Range;

use winnow::combinator::{alt, opt, peek, repeat_till, separated};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::{any, take};

use super::super::primitives;
use crate::lexer::{LexStream, OwnedLexToken, lex_line, render_token_slice};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SameIsTrueSurface {
    pub(crate) targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BorrowPhraseOccurrencesSurface {
    pub(crate) ranges: Vec<Range<usize>>,
}

fn same_is_true_separator(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        (
            alt((primitives::comma(), primitives::semicolon())),
            opt(primitives::kw("and")),
        )
            .void(),
        primitives::kw("and").void(),
    ))
    .parse_next(input)
}

fn same_is_true_target_boundary(input: &mut LexStream<'_>) -> WResult<()> {
    alt((same_is_true_separator, primitives::sentence_end())).parse_next(input)
}

fn parse_same_is_true_target_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(same_is_true_target_boundary))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn parse_same_is_true_surface_lexed(input: &mut LexStream<'_>) -> WResult<SameIsTrueSurface> {
    primitives::phrase(&["the", "same", "is", "true", "for"])
        .void()
        .parse_next(input)?;
    let target_tokens: Vec<&[OwnedLexToken]> =
        separated(1.., parse_same_is_true_target_lexed, same_is_true_separator)
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let targets = target_tokens
        .into_iter()
        .map(render_token_slice)
        .map(|target| target.trim().to_string())
        .filter(|target| !target.is_empty())
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(primitives::backtrack_err(
            "same-is-true target list",
            "one or more targets",
        ));
    }
    Ok(SameIsTrueSurface { targets })
}

pub(crate) fn parse_same_is_true_surface(sentence: &str) -> Option<SameIsTrueSurface> {
    let tokens = lex_line(sentence.trim(), 0).ok()?;
    primitives::parse_all(
        &tokens,
        parse_same_is_true_surface_lexed,
        "same-is-true target list",
    )
    .ok()
}

fn same_token_phrase(actual: &[OwnedLexToken], expected: &[OwnedLexToken]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(actual, expected)| {
            actual.kind == expected.kind && actual.parser_text() == expected.parser_text()
        })
}

fn exact_token_phrase_parser<'a, 'p>(
    expected: &'p [OwnedLexToken],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut LexStream<'a>| {
        let actual: &[OwnedLexToken] = take(expected.len()).parse_next(input)?;
        if !same_token_phrase(actual, expected) {
            return Err(primitives::backtrack_err(
                "borrow phrase occurrence",
                "exact token phrase",
            ));
        }
        Ok(())
    }
}

pub(crate) fn parse_borrow_phrase_occurrences(
    sentence: &str,
    phrase: &str,
) -> Option<BorrowPhraseOccurrencesSurface> {
    let sentence = sentence.trim();
    let tokens = lex_line(sentence, 0).ok()?;
    let expected = lex_line(phrase.trim(), 0).ok()?;
    if expected.is_empty() {
        return None;
    }

    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    while cursor < tokens.len() {
        let Some((relative_start, (), _)) = primitives::find_prefix(&tokens[cursor..], || {
            exact_token_phrase_parser(expected.as_slice())
        }) else {
            break;
        };
        let start_token = cursor + relative_start;
        let end_token = start_token + expected.len();
        ranges.push(
            tokens.get(start_token)?.span.start..tokens.get(end_token.checked_sub(1)?)?.span.end,
        );
        cursor = end_token;
    }

    (!ranges.is_empty()).then_some(BorrowPhraseOccurrencesSurface { ranges })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_same_is_true_targets_and_borrow_occurrences() {
        assert_eq!(
            parse_same_is_true_surface("The same is true for artifacts")
                .expect("target")
                .targets,
            ["artifacts"]
        );
        assert_eq!(
            parse_same_is_true_surface(
                "The same is true for first strike, trample, and protection from any color."
            )
            .expect("targets")
            .targets,
            ["first strike", "trample", "protection from any color"]
        );
        assert_eq!(
            parse_same_is_true_surface(
                "The same is true for creature spells you control and creature cards you own that aren't on the battlefield."
            )
            .expect("targets")
            .targets,
            [
                "creature spells you control",
                "creature cards you own that aren't on the battlefield"
            ]
        );
        assert_eq!(
            parse_borrow_phrase_occurrences(
                "As long as a creature with flying is in a graveyard, creatures have flying",
                "flying",
            )
            .expect("occurrences")
            .ranges
            .len(),
            2
        );
    }
}
