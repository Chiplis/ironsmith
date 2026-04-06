use winnow::combinator::{alt, eof, peek};
use winnow::error::{ContextError, ErrMode, ParseError, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::{any, literal, take_till};

use crate::cards::builders::{CardTextError, TextSpan};

pub(crate) use super::super::lexer::TokenWordView;
use super::super::lexer::{LexStream, LexToken, TokenKind};

fn failure_location<'a>(
    tokens: &'a LexStream<'a>,
    offset: usize,
) -> (TextSpan, Option<&'a LexToken>) {
    if let Some(token) = tokens.get(offset) {
        return (token.span(), Some(token));
    }

    if let Some(last) = tokens.last() {
        return (
            TextSpan {
                line: last.span.line,
                start: last.span.end,
                end: last.span.end,
            },
            None,
        );
    }

    (TextSpan::synthetic(), None)
}

fn format_parse_error(
    label: &str,
    err: ParseError<LexStream<'_>, ContextError>,
    display_line_index: Option<usize>,
) -> CardTextError {
    let (span, token) = failure_location(err.input(), err.offset());
    let display_line = display_line_index.unwrap_or(span.line) + 1;
    let location = if span.start == span.end {
        format!("line {display_line} at {}", span.start)
    } else {
        format!("line {display_line} at {}..{}", span.start, span.end)
    };
    let found = token
        .map(|token| format!(" near {:?}", token.slice))
        .unwrap_or_else(|| " at end of input".to_string());

    CardTextError::ParseError(format!(
        "rewrite {label} parse failed on {location}{found}: {}",
        err.inner()
    ))
}

pub(crate) fn parse_all<'a, O>(
    tokens: &'a [LexToken],
    mut parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<O, CardTextError> {
    parser
        .parse(LexStream::new(tokens))
        .map_err(|err| format_parse_error(label, err, None))
}

pub(crate) fn parse_all_with_display_line<'a, O>(
    tokens: &'a [LexToken],
    mut parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
    display_line_index: usize,
) -> Result<O, CardTextError> {
    parser
        .parse(LexStream::new(tokens))
        .map_err(|err| format_parse_error(label, err, Some(display_line_index)))
}

pub(crate) fn parse_prefix<'a, O>(
    tokens: &'a [LexToken],
    mut parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
) -> Option<(O, &'a [LexToken])> {
    let (rest, parsed) = parser.parse_peek(LexStream::new(tokens)).ok()?;
    let remaining = tokens.get(tokens.len().checked_sub(rest.len())?..)?;
    Some((parsed, remaining))
}

/// Adapts a winnow parser into the `SentencePrimitiveParser` convention:
///
/// - Winnow backtrack (pattern mismatch) → `Ok(None)`
/// - Winnow cut (hard parse error) → `Err(CardTextError)`
/// - Winnow success with trailing tokens → `Err(CardTextError)`
/// - Winnow success consuming all input → `Ok(Some(value))`
pub(crate) fn try_parse_all<'a, O>(
    tokens: &'a [LexToken],
    mut parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<Option<O>, CardTextError> {
    let mut input = LexStream::new(tokens);
    match parser.parse_next(&mut input) {
        Ok(value) => {
            if input.is_empty() {
                Ok(Some(value))
            } else {
                let (span, token) = failure_location(&input, 0);
                let found = token
                    .map(|t| format!(" near {:?}", t.slice))
                    .unwrap_or_default();
                Err(CardTextError::ParseError(format!(
                    "rewrite {label} parse matched but has trailing tokens at {}..{}{found}",
                    span.start, span.end,
                )))
            }
        }
        Err(ErrMode::Backtrack(_)) => Ok(None),
        Err(ErrMode::Cut(inner)) => {
            let (span, token) = failure_location(&input, 0);
            let found = token
                .map(|t| format!(" near {:?}", t.slice))
                .unwrap_or_else(|| " at end of input".to_string());
            Err(CardTextError::ParseError(format!(
                "rewrite {label} parse failed at {}..{}{found}: {inner}",
                span.start, span.end,
            )))
        }
        Err(ErrMode::Incomplete(_)) => Ok(None),
    }
}

pub(crate) fn find_prefix<'a, O, P, F>(
    tokens: &'a [LexToken],
    make_parser: F,
) -> Option<(usize, O, &'a [LexToken])>
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut idx = 0usize;
    loop {
        if let Some((parsed, rest)) = parse_prefix(&tokens[idx..], make_parser()) {
            return Some((idx, parsed, rest));
        }
        if idx == tokens.len() {
            return None;
        }
        idx += 1;
    }
}

pub(crate) fn find_token_index(
    tokens: &[LexToken],
    mut predicate: impl FnMut(&LexToken) -> bool,
) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if predicate(&tokens[idx]) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub(crate) fn rfind_token_index(
    tokens: &[LexToken],
    mut predicate: impl FnMut(&LexToken) -> bool,
) -> Option<usize> {
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if predicate(&tokens[idx]) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn contains_word(tokens: &[LexToken], expected: &'static str) -> bool {
    find_prefix(tokens, || kw(expected)).is_some()
}

pub(crate) fn contains_phrase(tokens: &[LexToken], expected: &'static [&'static str]) -> bool {
    find_phrase_start(tokens, expected).is_some()
}

pub(crate) fn contains_any_phrase(
    tokens: &[LexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    phrases
        .iter()
        .any(|phrase_words| contains_phrase(tokens, phrase_words))
}

pub(crate) fn find_phrase_start(
    tokens: &[LexToken],
    expected: &'static [&'static str],
) -> Option<usize> {
    find_prefix(tokens, || phrase(expected)).map(|(idx, _, _)| idx)
}

pub(crate) fn token_slice_span(tokens: &[LexToken]) -> Option<TextSpan> {
    let line = tokens.first()?.span().line;
    let (_, span) =
        take_till::<_, LexStream<'_>, ErrMode<ContextError>>(0.., |_token: &LexToken| false)
            .span()
            .parse_peek(LexStream::new(tokens))
            .ok()?;
    Some(TextSpan {
        line,
        start: span.start,
        end: span.end,
    })
}

pub(crate) fn token_kind<'a>(
    expected: TokenKind,
) -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    literal(expected)
        .map(|tokens: &'a [LexToken]| &tokens[0])
        .context(StrContext::Expected(StrContextValue::Description("token")))
}

fn punctuation<'a>(
    expected: TokenKind,
    label: &'static str,
) -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    token_kind(expected)
        .context(StrContext::Label(label))
        .context(StrContext::Expected(StrContextValue::Description(label)))
}

pub(crate) fn word_text<'a>(input: &mut LexStream<'a>) -> Result<&'a str, ErrMode<ContextError>> {
    let token: &'a LexToken = any.parse_next(input)?;
    token.as_word().ok_or_else(|| {
        let mut err = ContextError::new();
        err.push(StrContext::Label("word"));
        err.push(StrContext::Expected(StrContextValue::Description("word")));
        ErrMode::Backtrack(err)
    })
}

pub(crate) fn kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        let Some(token) = input.peek_token() else {
            let mut err = ContextError::new();
            err.push(StrContext::Label("keyword"));
            err.push(StrContext::Expected(expected.into()));
            return Err(ErrMode::Backtrack(err));
        };

        if !token.is_word(expected) {
            let mut err = ContextError::new();
            err.push(StrContext::Label("keyword"));
            err.push(StrContext::Expected(expected.into()));
            return Err(ErrMode::Backtrack(err));
        }

        Ok(&input.next_slice(1)[0])
    }
}

pub(crate) fn comma<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Comma, "comma")
}

pub(crate) fn period<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Period, "period")
}

pub(crate) fn colon<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Colon, "colon")
}

pub(crate) fn semicolon<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Semicolon, "semicolon")
}

pub(crate) fn lparen<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::LParen, "left parenthesis")
}

pub(crate) fn rparen<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::RParen, "right parenthesis")
}

pub(crate) fn quote<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Quote, "quote")
}

pub(crate) fn end_of_sentence<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    period()
        .map(|_| ())
        .context(StrContext::Label("end of sentence"))
        .context(StrContext::Expected(StrContextValue::Description("period")))
}

pub(crate) fn end_of_block<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    eof.value(())
        .context(StrContext::Label("end of block"))
        .context(StrContext::Expected(StrContextValue::Description(
            "end of token block",
        )))
}

pub(crate) fn end_of_sentence_or_block<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>>
{
    alt((end_of_sentence(), end_of_block()))
        .context(StrContext::Label("end of sentence or block"))
        .context(StrContext::Expected(StrContextValue::Description(
            "end of sentence or block",
        )))
}

pub(crate) fn phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            if let Err(err) = kw(*word).parse_next(input) {
                return Err(err.map(|mut inner| {
                    inner.push(StrContext::Label("phrase"));
                    inner.push(StrContext::Expected(StrContextValue::Description(
                        "word phrase",
                    )));
                    inner
                }));
            }
        }
        Ok(())
    }
}

fn split_lexed_slices_on_separator<'a, P, F>(
    tokens: &'a [LexToken],
    make_separator: F,
) -> Vec<&'a [LexToken]>
where
    F: Fn() -> P + Copy,
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    split_lexed_slices_with_parser(tokens, || {
        move |input: &mut LexStream<'a>| parse_segment_until_separator(input, make_separator)
    })
}

fn split_lexed_slices_with_parser<'a, P, F>(
    tokens: &'a [LexToken],
    make_segment_parser: F,
) -> Vec<&'a [LexToken]>
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, &'a [LexToken], ErrMode<ContextError>>,
{
    let mut segments = Vec::new();
    let mut remaining = tokens;

    while !remaining.is_empty() {
        let Some((segment, rest)) = parse_prefix(remaining, make_segment_parser()) else {
            break;
        };

        if !segment.is_empty() {
            segments.push(segment);
        }

        if rest.len() == remaining.len() {
            break;
        }
        remaining = rest;
    }

    segments
}

fn parse_segment_until_separator<'a, P, F>(
    input: &mut LexStream<'a>,
    make_separator: F,
) -> Result<&'a [LexToken], ErrMode<ContextError>>
where
    F: Fn() -> P + Copy,
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    let segment = (move |input: &mut LexStream<'a>| {
        while input.peek_token().is_some() {
            if peek(make_separator()).parse_next(input).is_ok() {
                return Ok(());
            }

            any.parse_next(input)?;
        }
        Ok(())
    })
    .take()
    .parse_next(input)?;

    if input.peek_token().is_some() {
        make_separator().parse_next(input)?;
    }

    Ok(segment)
}

pub(crate) fn split_lexed_once_on_separator<'a, P, F>(
    tokens: &'a [LexToken],
    make_separator: F,
) -> Option<(&'a [LexToken], &'a [LexToken])>
where
    F: Fn() -> P + Copy,
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    let (head, rest) = parse_prefix(tokens, move |input: &mut LexStream<'a>| {
        parse_segment_until_separator(input, make_separator)
    })?;
    (head.len() + rest.len() < tokens.len()).then_some((head, rest))
}

pub(crate) fn split_lexed_once_on_delimiter<'a>(
    tokens: &'a [LexToken],
    delimiter: TokenKind,
) -> Option<(&'a [LexToken], &'a [LexToken])> {
    let parser = take_till(0.., move |token: &LexToken| token.kind == delimiter).with_taken();
    let (rest, ((_, head), _)) = (parser, token_kind(delimiter))
        .parse_peek(LexStream::new(tokens))
        .ok()?;
    let remaining = tokens.get(tokens.len().checked_sub(rest.len())?..)?;
    Some((head, remaining))
}

pub(crate) fn split_lexed_once_on_comma<'a>(
    tokens: &'a [LexToken],
) -> Option<(&'a [LexToken], &'a [LexToken])> {
    split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
}

pub(crate) fn split_lexed_slices_on_and<'a>(tokens: &'a [LexToken]) -> Vec<&'a [LexToken]> {
    split_lexed_slices_on_separator(tokens, || phrase(&["and"]))
}

pub(crate) fn split_lexed_slices_on_comma<'a>(tokens: &'a [LexToken]) -> Vec<&'a [LexToken]> {
    split_lexed_slices_on_separator(tokens, || comma().map(|_| ()))
}

fn is_comparison_or_delimiter(previous_word: Option<&str>, next_word: Option<&str>) -> bool {
    if matches!(next_word, Some("less" | "greater" | "more" | "fewer")) {
        return true;
    }

    previous_word == Some("than") && next_word == Some("equal")
}

pub(crate) fn split_lexed_slices_on_or<'a>(tokens: &'a [LexToken]) -> Vec<&'a [LexToken]> {
    split_lexed_slices_with_parser(tokens, || parse_segment_until_or_separator)
}

pub(crate) fn split_lexed_slices_on_commas_or_semicolons<'a>(
    tokens: &'a [LexToken],
) -> Vec<&'a [LexToken]> {
    split_lexed_slices_on_separator(tokens, || {
        alt((comma().map(|_| ()), semicolon().map(|_| ())))
    })
}

pub(crate) fn split_lexed_slices_on_period<'a>(tokens: &'a [LexToken]) -> Vec<&'a [LexToken]> {
    split_lexed_slices_with_parser(tokens, || parse_segment_until_period)
}

fn parse_segment_until_or_separator<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [LexToken], ErrMode<ContextError>> {
    let segment = (|input: &mut LexStream<'a>| {
        let mut previous_word = None;

        while let Some(token) = input.peek_token() {
            if token.is_comma() {
                return Ok(());
            }

            if token.is_word("or") {
                let next_word = input.get(1).and_then(LexToken::as_word);
                if !is_comparison_or_delimiter(previous_word, next_word) {
                    return Ok(());
                }
            }

            let consumed_token: &'a LexToken = any.parse_next(input)?;
            if let Some(word) = consumed_token.as_word() {
                previous_word = Some(word);
            }
        }

        Ok(())
    })
    .take()
    .parse_next(input)?;

    if let Some(token) = input.peek_token() {
        if token.is_comma() {
            comma().parse_next(input)?;
        } else if token.is_word("or") {
            let previous_word = segment.iter().rev().find_map(|token| token.as_word());
            let next_word = input.get(1).and_then(LexToken::as_word);
            if !is_comparison_or_delimiter(previous_word, next_word) {
                kw("or").parse_next(input)?;
            }
        }
    }

    Ok(segment)
}

fn parse_segment_until_period<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [LexToken], ErrMode<ContextError>> {
    let segment = (|input: &mut LexStream<'a>| {
        let mut inside_quotes = false;

        while let Some(token) = input.peek_token() {
            if token.is_quote() {
                quote().parse_next(input)?;
                inside_quotes = !inside_quotes;
                continue;
            }

            if token.is_period() && !inside_quotes {
                return Ok(());
            }

            any.parse_next(input)?;
        }

        Ok(())
    })
    .take()
    .parse_next(input)?;

    if input.peek_token().is_some_and(|token| token.is_period()) {
        period().parse_next(input)?;
    }

    Ok(segment)
}

pub(crate) fn strip_lexed_prefix_phrase<'a>(
    tokens: &'a [LexToken],
    phrase_words: &'static [&'static str],
) -> Option<&'a [LexToken]> {
    parse_prefix(tokens, phrase(phrase_words)).map(|(_, rest)| rest)
}

pub(crate) fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [LexToken],
    phrase: &[&str],
) -> Option<&'a [LexToken]> {
    let words = TokenWordView::new(tokens);
    let word_refs = words.word_refs();
    if word_refs.len() < phrase.len() {
        return None;
    }

    let suffix_start = word_refs.len() - phrase.len();
    if !words.slice_eq(suffix_start, phrase) {
        return None;
    }

    let keep_word_count = word_refs.len().checked_sub(phrase.len())?;
    let keep_until = if keep_word_count == 0 {
        0
    } else {
        words.token_index_for_word_index(keep_word_count)?
    };
    Some(&tokens[..keep_until])
}

pub(crate) fn strip_lexed_suffix_phrases<'a, 'b>(
    tokens: &'a [LexToken],
    phrases: &'b [&'b [&'b str]],
) -> Option<(&'b [&'b str], &'a [LexToken])> {
    phrases
        .iter()
        .find_map(|phrase| strip_lexed_suffix_phrase(tokens, phrase).map(|rest| (*phrase, rest)))
}

// ---------------------------------------------------------------------------
// Word-level bridge functions
//
// These operate on `&[LexToken]` but match words while skipping non-word
// tokens (commas, etc.), mirroring the behavior of `token_word_refs` +
// `slice_starts_with`.  They bridge the gap between old word-slice-based
// code and the token-stream-based grammar primitives.
// ---------------------------------------------------------------------------

/// Checks whether the word pieces at the start of `tokens` match `expected`,
/// using `TokenWordView` for proper multi-word token splitting (e.g.,
/// hyphenated words like "life-gaining" → ["life", "gaining"]).
/// Returns the token slice after the matched prefix.
pub(crate) fn words_match_prefix<'a>(tokens: &'a [LexToken], expected: &[&str]) -> Option<&'a [LexToken]> {
    if expected.is_empty() {
        return Some(tokens);
    }
    let view = TokenWordView::new(tokens);
    if !view.starts_with(expected) {
        return None;
    }
    let token_end = view.token_index_after_words(expected.len())?;
    Some(&tokens[token_end..])
}

/// Checks whether the word pieces at the end of `tokens` match `expected`,
/// using `TokenWordView` for proper multi-word token splitting.
/// Returns the token slice before the matched suffix.
pub(crate) fn words_match_suffix<'a>(tokens: &'a [LexToken], expected: &[&str]) -> Option<&'a [LexToken]> {
    if expected.is_empty() {
        return Some(tokens);
    }
    let view = TokenWordView::new(tokens);
    if view.len() < expected.len() {
        return None;
    }
    let suffix_start_word = view.len() - expected.len();
    if !view.slice_eq(suffix_start_word, expected) {
        return None;
    }
    let token_start = view.token_index_for_word_index(suffix_start_word)?;
    Some(&tokens[..token_start])
}

/// Finds the first occurrence of `expected` word sequence in `tokens`,
/// using `TokenWordView` for proper multi-word token splitting.
/// Returns the token index where the match starts.
pub(crate) fn words_find_phrase(tokens: &[LexToken], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() {
        return Some(0);
    }
    let view = TokenWordView::new(tokens);
    let word_idx = view.find_phrase_start(expected)?;
    view.token_index_for_word_index(word_idx)
}

/// Splits `tokens` at the first occurrence of word sequence `separator`,
/// using `TokenWordView` for proper multi-word token splitting.
/// Returns `(before, after)` where `before` ends just before the separator
/// and `after` starts just after it.
pub(crate) fn words_split_once<'a>(
    tokens: &'a [LexToken],
    separator: &[&str],
) -> Option<(&'a [LexToken], &'a [LexToken])> {
    if separator.is_empty() {
        return Some((&[], tokens));
    }
    let view = TokenWordView::new(tokens);
    let word_idx = view.find_phrase_start(separator)?;
    let token_start = view.token_index_for_word_index(word_idx)?;
    let after_word_idx = word_idx + separator.len();
    let token_end = view.token_index_after_words(after_word_idx)?;
    Some((&tokens[..token_start], &tokens[token_end..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::parser::lexer::lex_line;
    use winnow::combinator::cut_err;

    #[test]
    fn words_match_prefix_basic() {
        let tokens = lex_line("target creature gets", 0).unwrap();
        assert!(words_match_prefix(&tokens, &["target", "creature"]).is_some());
        assert!(words_match_prefix(&tokens, &["target", "gets"]).is_none());
        assert!(words_match_prefix(&tokens, &[]).is_some());
    }

    #[test]
    fn words_match_suffix_basic() {
        let tokens = lex_line("target creature gets", 0).unwrap();
        assert!(words_match_suffix(&tokens, &["creature", "gets"]).is_some());
        assert!(words_match_suffix(&tokens, &["target", "gets"]).is_none());
    }

    #[test]
    fn words_find_phrase_basic() {
        let tokens = lex_line("target creature gets big", 0).unwrap();
        assert_eq!(words_find_phrase(&tokens, &["creature", "gets"]), Some(1));
        assert_eq!(words_find_phrase(&tokens, &["target"]), Some(0));
        assert_eq!(words_find_phrase(&tokens, &["nope"]), None);
    }

    #[test]
    fn words_split_once_basic() {
        let tokens = lex_line("exile target creature from graveyard", 0).unwrap();
        let (before, after) = words_split_once(&tokens, &["from"]).unwrap();
        assert_eq!(before.len(), 3); // "exile", "target", "creature"
        assert_eq!(after.len(), 1); // "graveyard"
    }

    #[test]
    fn try_parse_all_returns_some_on_full_match() {
        let tokens = lex_line("target creature", 0).unwrap();
        let result = try_parse_all(&tokens, phrase(&["target", "creature"]), "test");
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn try_parse_all_returns_none_on_backtrack() {
        let tokens = lex_line("target creature", 0).unwrap();
        let result = try_parse_all(&tokens, phrase(&["exile", "creature"]), "test");
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn try_parse_all_returns_err_on_trailing_tokens() {
        let tokens = lex_line("target creature gets", 0).unwrap();
        let result = try_parse_all(&tokens, phrase(&["target", "creature"]), "test");
        assert!(result.is_err());
    }

    #[test]
    fn try_parse_all_returns_err_on_cut() {
        let tokens = lex_line("target creature", 0).unwrap();
        let parser = |input: &mut LexStream<'_>| -> Result<(), ErrMode<ContextError>> {
            kw("target").void().parse_next(input)?;
            cut_err(kw("opponent")).void().parse_next(input)?;
            Ok(())
        };
        let result = try_parse_all(&tokens, parser, "test");
        assert!(result.is_err());
    }
}
