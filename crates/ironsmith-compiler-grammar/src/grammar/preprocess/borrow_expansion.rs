use std::ops::Range;

use winnow::combinator::{alt, opt, peek, repeat_till, separated};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::{any, take};

use super::super::primitives;
use crate::lexer::{LexStream, OwnedLexToken, render_token_slice};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SameIsTrueSurface {
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowPhraseOccurrencesSurface {
    pub ranges: Vec<Range<usize>>,
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

#[cfg(test)]
pub fn parse_same_is_true_surface(sentence: &str) -> Option<SameIsTrueSurface> {
    let tokens = crate::util::lex_fragment(sentence.trim(), 0)?;
    parse_same_is_true_surface_tokens(&tokens)
}

pub fn parse_same_is_true_surface_tokens(tokens: &[OwnedLexToken]) -> Option<SameIsTrueSurface> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_same_is_true_surface_lexed,
        "same-is-true target list",
    )
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

/// Where a borrowed-ability phrase occurs in a sentence, as token index
/// ranges. The phrase is one of the fixed borrowed-ability names, so its
/// tokens are synthesized from its words rather than lexed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowPhraseTokenOccurrences {
    pub ranges: Vec<Range<usize>>,
}

pub fn parse_borrow_phrase_occurrences_tokens(
    tokens: &[OwnedLexToken],
    phrase: &str,
) -> Option<BorrowPhraseTokenOccurrences> {
    let expected = crate::lexer::synthetic_phrase_tokens(phrase);
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
        ranges.push(start_token..end_token);
        cursor = end_token;
    }

    (!ranges.is_empty()).then_some(BorrowPhraseTokenOccurrences { ranges })
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
            parse_borrow_phrase_occurrences_tokens(
                &crate::util::lex_fragment(
                    "as long as a creature with flying is in a graveyard, creatures have flying",
                    0,
                )
                .expect("lexes"),
                "flying",
            )
            .expect("occurrences")
            .ranges
            .len(),
            2
        );
    }
}
