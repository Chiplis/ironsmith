use std::{cell::Cell, fmt};

use winnow::combinator::{alt, eof, opt, peek, preceded, repeat};
use winnow::error::{ContextError, ErrMode, ParseError, ParserError, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::{any, literal, take_till};

use crate::cards::builders::{CardTextError, TextSpan};
use crate::mana::ManaSymbol;

pub(crate) use super::super::lexer::TokenWordView;
use super::super::lexer::{LexStream, LexToken, TokenKind};

pub(crate) struct MaybeTrace<P, D> {
    parser: P,
    name: D,
}

impl<P, D> MaybeTrace<P, D> {
    fn new(name: D, parser: P) -> Self {
        Self { parser, name }
    }
}

impl<I, O, E, P, D> Parser<I, O, E> for MaybeTrace<P, D>
where
    I: Stream,
    E: ParserError<I>,
    P: Parser<I, O, E>,
    D: fmt::Display,
{
    fn parse_next(&mut self, input: &mut I) -> core::result::Result<O, E> {
        if super::super::util::parser_trace_enabled() {
            let depth = TraceDepth::enter();
            let start = input.checkpoint();
            eprintln!(
                "{:depth$}> {} | {}",
                "",
                self.name,
                StreamTrace(input),
                depth = depth.get()
            );
            let result = self.parser.parse_next(input);
            let consumed = input.offset_from(&start);
            let status = if result.is_ok() {
                format!("ok +{consumed}")
            } else if result.as_ref().err().is_some_and(ParserError::is_backtrack) {
                "backtrack".to_string()
            } else if result
                .as_ref()
                .err()
                .is_some_and(ParserError::is_incomplete)
            {
                "incomplete".to_string()
            } else {
                "cut".to_string()
            };
            eprintln!(
                "{:depth$}< {} | {}",
                "",
                self.name,
                status,
                depth = depth.get()
            );
            result
        } else {
            self.parser.parse_next(input)
        }
    }
}

pub(crate) fn maybe_trace<P, D>(name: D, parser: P) -> MaybeTrace<P, D> {
    MaybeTrace::new(name, parser)
}

struct StaticTraceLabel {
    kind: &'static str,
    detail: &'static str,
}

impl fmt::Display for StaticTraceLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.kind, self.detail)
    }
}

struct PhraseTraceLabel(&'static [&'static str]);

impl fmt::Display for PhraseTraceLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("phrase(")?;
        for (idx, word) in self.0.iter().enumerate() {
            if idx > 0 {
                f.write_str(" ")?;
            }
            f.write_str(word)?;
        }
        f.write_str(")")
    }
}

thread_local! {
    static TRACE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

struct TraceDepth {
    depth: usize,
}

impl TraceDepth {
    fn enter() -> Self {
        let depth = TRACE_DEPTH.with(|value| {
            let depth = value.get();
            value.set(depth + 1);
            depth
        });
        Self { depth }
    }

    fn get(&self) -> usize {
        self.depth
    }
}

impl Drop for TraceDepth {
    fn drop(&mut self) {
        TRACE_DEPTH.with(|value| value.set(self.depth));
    }
}

struct StreamTrace<'a, I>(&'a I);

impl<I: Stream> fmt::Display for StreamTrace<'_, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.trace(f)
    }
}

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
    parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<O, CardTextError> {
    let mut parser = maybe_trace(label, parser);
    parser
        .parse(LexStream::new(tokens))
        .map_err(|err| format_parse_error(label, err, None))
}

pub(crate) fn parse_all_with_display_line<'a, O>(
    tokens: &'a [LexToken],
    parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
    display_line_index: usize,
) -> Result<O, CardTextError> {
    let mut parser = maybe_trace(label, parser);
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

pub(crate) fn parse_all_or_none<'a, O>(
    tokens: &'a [LexToken],
    parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<Option<O>, CardTextError> {
    let mut input = LexStream::new(tokens);
    let mut parser = maybe_trace(label, parser);
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

#[cfg(test)]
/// Adapts a winnow parser into the `SentencePrimitiveParser` convention:
///
/// - Winnow backtrack (pattern mismatch) → `Ok(None)`
/// - Winnow cut (hard parse error) → `Err(CardTextError)`
/// - Winnow success with trailing tokens → `Err(CardTextError)`
/// - Winnow success consuming all input → `Ok(Some(value))`
pub(crate) fn try_parse_all<'a, O>(
    tokens: &'a [LexToken],
    parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<Option<O>, CardTextError> {
    parse_all_or_none(tokens, parser, label)
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

/// Constructs a `Backtrack` error with a label and expected description.
///
/// Use this instead of manually constructing `ContextError` + `ErrMode::Backtrack`.
pub(crate) fn backtrack_err(label: &'static str, expected: &'static str) -> ErrMode<ContextError> {
    let mut err = ContextError::new();
    err.push(StrContext::Label(label));
    err.push(StrContext::Expected(StrContextValue::Description(expected)));
    ErrMode::Backtrack(err)
}

/// Constructs a `Cut` error with a label and expected description.
pub(crate) fn cut_err_ctx(label: &'static str, expected: &'static str) -> ErrMode<ContextError> {
    let mut err = ContextError::new();
    err.push(StrContext::Label(label));
    err.push(StrContext::Expected(StrContextValue::Description(expected)));
    ErrMode::Cut(err)
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
    maybe_trace(
        StaticTraceLabel {
            kind: "punct",
            detail: label,
        },
        token_kind(expected)
            .context(StrContext::Label(label))
            .context(StrContext::Expected(StrContextValue::Description(label))),
    )
}

pub(crate) fn word_text<'a>(input: &mut LexStream<'a>) -> Result<&'a str, ErrMode<ContextError>> {
    let token: &'a LexToken = any.parse_next(input)?;
    token.as_word().ok_or_else(|| backtrack_err("word", "word"))
}

/// Like `word_text` but returns the normalized `parser_text` (lowercased,
/// apostrophe-normalized) instead of the original slice.  Use this as the
/// discriminant inside `dispatch!` so that branch labels can be written in
/// lowercase regardless of how the source text was capitalized.
pub(crate) fn word_parser_text<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a str, ErrMode<ContextError>> {
    let token: &'a LexToken = any.parse_next(input)?;
    if matches!(
        token.kind,
        super::super::lexer::TokenKind::Word
            | super::super::lexer::TokenKind::Number
            | super::super::lexer::TokenKind::Tilde
    ) {
        Ok(token.parser_text())
    } else {
        Err(backtrack_err("word", "word"))
    }
}

pub(crate) fn kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    maybe_trace(
        StaticTraceLabel {
            kind: "kw",
            detail: expected,
        },
        any.verify(move |token: &&LexToken| token.is_word(expected))
            .context(StrContext::Label("keyword"))
            .context(StrContext::Expected(StrContextValue::Description(expected))),
    )
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

#[cfg(test)]
pub(crate) fn lparen<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::LParen, "left parenthesis")
}

#[cfg(test)]
pub(crate) fn rparen<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::RParen, "right parenthesis")
}

pub(crate) fn quote<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Quote, "quote")
}

/// Matches an optional period followed by end-of-input.
///
/// This is the standard trailing pattern for sentence/block parsers.
pub(crate) fn sentence_end<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (opt(period()), eof).void()
}

pub(crate) fn end_of_sentence<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    period()
        .void()
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

// ---------------------------------------------------------------------------
// Stream-based token parsers
//
// These adapt common token-slice helpers into winnow `Parser` implementations
// so call-sites can compose them with `separated`, `repeat`, `alt`, etc.
// ---------------------------------------------------------------------------

/// Parse a numeric word token (digit or english word like "three") and return
/// its `u32` value.  Consumes exactly one token on success.
pub(crate) fn number_token<'a>(input: &mut LexStream<'a>) -> Result<u32, ErrMode<ContextError>> {
    let token: &'a LexToken = any.parse_next(input)?;
    let word = token
        .as_word()
        .ok_or_else(|| backtrack_err("number", "numeric word"))?
        .to_ascii_lowercase();

    if let Ok(value) = word.parse::<u32>() {
        return Ok(value);
    }

    let value = match word.as_str() {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return Err(backtrack_err("number", "numeric word")),
    };
    Ok(value)
}

/// Parse a single mana symbol from the next token (word, number, or
/// `{…}` mana-group).  Returns the individual `ManaSymbol` values found
/// in that token.
pub(crate) fn mana_pips_token<'a>(
    input: &mut LexStream<'a>,
) -> Result<Vec<ManaSymbol>, ErrMode<ContextError>> {
    let checkpoint = input.checkpoint();
    let token: &'a LexToken = any.parse_next(input)?;

    let result = match token.kind {
        TokenKind::Word | TokenKind::Number => {
            super::values::parse_mana_symbol(token.slice.as_str())
                .ok()
                .map(|s| vec![s])
        }
        TokenKind::ManaGroup => {
            let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
            if inner.is_empty() {
                None
            } else {
                super::values::parse_mana_symbol_group(inner)
                    .ok()
                    .filter(|g| !g.is_empty())
            }
        }
        _ => None,
    };

    result.ok_or_else(|| {
        input.reset(&checkpoint);
        backtrack_err("mana", "mana symbol")
    })
}

/// Parse a single mana symbol (flattened) from the next token.
pub(crate) fn mana_symbol_token<'a>(
    input: &mut LexStream<'a>,
) -> Result<ManaSymbol, ErrMode<ContextError>> {
    let checkpoint = input.checkpoint();
    let token: &'a LexToken = any.parse_next(input)?;
    let word = token.as_word().ok_or_else(|| {
        input.reset(&checkpoint);
        backtrack_err("mana", "mana symbol word")
    })?;

    super::values::parse_mana_symbol(word).map_err(|_| {
        input.reset(&checkpoint);
        backtrack_err("mana", "mana symbol word")
    })
}

/// Skip one or more tokens that are commas and/or the keyword "or".
/// Suitable as the separator argument to `separated()`.
pub(crate) fn comma_or_separator<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    // At least one comma (optionally followed by "or" and more commas),
    // or just "or" (optionally followed by commas).
    let skip_commas = || repeat::<_, _, (), _, _>(0.., comma().void());
    alt((
        (
            repeat::<_, _, (), _, _>(1.., comma().void()),
            opt(kw("or").void()),
            skip_commas(),
        )
            .void(),
        (kw("or").void(), skip_commas()).void(),
    ))
    .context(StrContext::Label("separator"))
    .context(StrContext::Expected(StrContextValue::Description(
        "comma or 'or'",
    )))
    .parse_next(input)
}

/// Skip one token that is a comma, "and", or "or".
pub(crate) fn list_separator<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((comma().void(), kw("and").void(), kw("or").void())).parse_next(input)
}

/// Skip tokens that are noise words in mana clauses
/// ("mana", "to", "your", "their", "its", "pool", articles).
pub(crate) fn skip_mana_noise<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    any.verify(|token: &&LexToken| {
        token.as_word().is_some_and(|word| {
            matches!(
                word.to_ascii_lowercase().as_str(),
                "mana" | "to" | "your" | "their" | "its" | "pool"
            ) || super::super::util::is_article(word)
        })
    })
    .void()
    .context(StrContext::Label("noise"))
    .context(StrContext::Expected(StrContextValue::Description(
        "mana noise word",
    )))
    .parse_next(input)
}

/// Collect mana pips from a token stream, skipping noise words and commas.
/// Returns a flat `Vec<ManaSymbol>`.
pub(crate) fn collect_mana_symbols<'a>(
    input: &mut LexStream<'a>,
) -> Result<Vec<ManaSymbol>, ErrMode<ContextError>> {
    let skip_noise =
        repeat::<_, _, (), _, _>(0.., alt((skip_mana_noise, comma().void(), period().void())));
    let groups: Vec<Vec<ManaSymbol>> = repeat(1.., preceded(skip_noise, mana_pips_token))
        .context(StrContext::Label("mana"))
        .context(StrContext::Expected(StrContextValue::Description(
            "mana symbols",
        )))
        .parse_next(input)?;
    Ok(groups.into_iter().flatten().collect())
}

/// Collect mana pip groups (each group is a Vec<ManaSymbol>) from a token
/// stream, skipping noise words.  Returns `Vec<Vec<ManaSymbol>>`.
pub(crate) fn collect_mana_pip_groups<'a>(
    input: &mut LexStream<'a>,
) -> Result<Vec<Vec<ManaSymbol>>, ErrMode<ContextError>> {
    let skip_noise = repeat::<_, _, (), _, _>(0.., alt((skip_mana_noise, comma().void())));
    repeat(1.., preceded(skip_noise, mana_pips_token))
        .context(StrContext::Label("mana"))
        .context(StrContext::Expected(StrContextValue::Description(
            "mana pip groups",
        )))
        .parse_next(input)
}

pub(crate) fn phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    maybe_trace(
        PhraseTraceLabel(expected),
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
        },
    )
}

pub(crate) fn any_phrase<'a, 'b>(
    phrases: &'b [&'static [&'static str]],
) -> impl Parser<LexStream<'a>, &'static [&'static str], ErrMode<ContextError>> + 'b {
    move |input: &mut LexStream<'a>| {
        for phrase_words in phrases {
            let mut probe = input.clone();
            if phrase(phrase_words).parse_next(&mut probe).is_ok() {
                *input = probe;
                return Ok(*phrase_words);
            }
        }

        Err(backtrack_err(
            "phrase choice",
            "one of the expected phrases",
        ))
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

pub(crate) fn split_lexed_once_before_suffix<'a, O, P, F>(
    tokens: &'a [LexToken],
    min_prefix_len: usize,
    make_suffix_parser: F,
) -> Option<(&'a [LexToken], O)>
where
    F: Fn() -> P + Copy,
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let search = tokens.get(min_prefix_len..)?;
    let (relative_idx, parsed, _) = find_prefix(search, || {
        (make_suffix_parser(), eof).map(|(parsed, _)| parsed)
    })?;
    let split_idx = min_prefix_len + relative_idx;
    Some((&tokens[..split_idx], parsed))
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
    split_lexed_slices_on_separator(tokens, || comma().void())
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
    split_lexed_slices_on_separator(tokens, || alt((comma().void(), semicolon().void())))
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

    opt(period()).parse_next(input)?;

    Ok(segment)
}

pub(crate) fn strip_lexed_prefix_phrase<'a>(
    tokens: &'a [LexToken],
    phrase_words: &'static [&'static str],
) -> Option<&'a [LexToken]> {
    parse_prefix(tokens, phrase(phrase_words)).map(|(_, rest)| rest)
}

pub(crate) fn strip_lexed_prefix_phrases<'a, 'b>(
    tokens: &'a [LexToken],
    phrases: &'b [&'static [&'static str]],
) -> Option<(&'static [&'static str], &'a [LexToken])> {
    parse_prefix(tokens, any_phrase(phrases))
}

pub(crate) fn starts_with_any_phrase<'b>(
    tokens: &[LexToken],
    phrases: &'b [&'static [&'static str]],
) -> bool {
    parse_prefix(tokens, any_phrase(phrases)).is_some()
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
pub(crate) fn words_match_prefix<'a>(
    tokens: &'a [LexToken],
    expected: &[&str],
) -> Option<&'a [LexToken]> {
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

pub(crate) fn words_match_any_prefix<'a, 'b>(
    tokens: &'a [LexToken],
    phrases: &'b [&'static [&'static str]],
) -> Option<(&'static [&'static str], &'a [LexToken])> {
    phrases
        .iter()
        .find_map(|phrase| words_match_prefix(tokens, phrase).map(|rest| (*phrase, rest)))
}

/// Checks whether the word pieces at the end of `tokens` match `expected`,
/// using `TokenWordView` for proper multi-word token splitting.
/// Returns the token slice before the matched suffix.
pub(crate) fn words_match_suffix<'a>(
    tokens: &'a [LexToken],
    expected: &[&str],
) -> Option<&'a [LexToken]> {
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
#[cfg(test)]
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

// ---------------------------------------------------------------------------
// Word-slice parsers
//
// These combinators operate on `&[&str]` slices (already-split word lists)
// rather than on `LexStream`.  They are shared by `object_filters`,
// `grammar::filters`, and `effect_sentences::chain_carry`.
// ---------------------------------------------------------------------------

/// Input type for word-slice parsers.
pub(crate) type WordSliceInput<'a> = &'a [&'a str];

/// Matches a single word (case-insensitive) and consumes it, returning `()`.
pub(crate) fn word_slice_eq<'a>(
    expected: &'static str,
) -> impl Parser<WordSliceInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut WordSliceInput<'a>| {
        let Some((word, rest)) = input.split_first() else {
            return Err(backtrack_err("word", expected));
        };
        if word.eq_ignore_ascii_case(expected) {
            *input = rest;
            Ok(())
        } else {
            Err(backtrack_err("word", expected))
        }
    }
}

/// Matches a single word (exact, case-sensitive) and consumes it, returning
/// the matched `&str`.
pub(crate) fn word_slice_exact<'a>(
    expected: &'static str,
) -> impl Parser<WordSliceInput<'a>, &'a str, ErrMode<ContextError>> {
    move |input: &mut WordSliceInput<'a>| {
        let Some((word, rest)) = input.split_first() else {
            return Err(backtrack_err("word", expected));
        };
        if *word == expected {
            *input = rest;
            Ok(*word)
        } else {
            Err(backtrack_err("word", expected))
        }
    }
}

/// Succeeds only when the word-slice input is fully consumed.
pub(crate) fn word_slice_eof<'a>(
    input: &mut WordSliceInput<'a>,
) -> Result<(), ErrMode<ContextError>> {
    if input.is_empty() {
        Ok(())
    } else {
        Err(backtrack_err("word input", "end of words"))
    }
}

/// Runs `parser` on `words`, succeeding only if the entire slice is consumed.
pub(crate) fn parse_full_word_slice<'a, O>(
    words: &'a [&'a str],
    parser: impl Parser<WordSliceInput<'a>, O, ErrMode<ContextError>>,
) -> Option<O> {
    let mut input: WordSliceInput<'a> = words;
    (parser, word_slice_eof)
        .map(|(parsed, ())| parsed)
        .parse_next(&mut input)
        .ok()
}

/// Runs `parser` on `words`, returning the parsed value on success (may leave
/// trailing words unconsumed).
pub(crate) fn parse_prefix_word_slice<'a, O>(
    words: &'a [&'a str],
    mut parser: impl Parser<WordSliceInput<'a>, O, ErrMode<ContextError>>,
) -> Option<O> {
    let mut input: WordSliceInput<'a> = words;
    parser.parse_next(&mut input).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::compiler::lexer::lex_line;
    use winnow::combinator::cut_err;

    #[test]
    fn words_match_prefix_basic() {
        let tokens = lex_line("target creature gets", 0).unwrap();
        assert!(matches!(
            words_match_prefix(&tokens, &["target", "creature"]),
            Some(_)
        ));
        assert!(words_match_prefix(&tokens, &["target", "gets"]).is_none());
        assert!(matches!(words_match_prefix(&tokens, &[]), Some(_)));
    }

    #[test]
    fn words_match_suffix_basic() {
        let tokens = lex_line("target creature gets", 0).unwrap();
        assert!(words_match_suffix(&tokens, &["creature", "gets"]).is_some());
        assert!(words_match_suffix(&tokens, &["target", "gets"]).is_none());
    }

    #[test]
    fn words_match_any_prefix_skips_leading_non_word_tokens() {
        let tokens = lex_line("\"At the beginning of the end step\"", 0).unwrap();
        let (matched, rest) =
            words_match_any_prefix(&tokens, &[&["at", "the", "beginning"]]).unwrap();

        assert_eq!(matched, &["at", "the", "beginning"]);
        assert_eq!(
            TokenWordView::new(rest).word_refs(),
            ["of", "the", "end", "step"]
        );
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
    fn strip_lexed_prefix_phrases_returns_matched_phrase_and_rest() {
        let tokens = lex_line("choose a new target for target spell", 0).unwrap();
        let (matched, rest) = strip_lexed_prefix_phrases(
            &tokens,
            &[
                &["choose", "new", "targets", "for"],
                &["choose", "a", "new", "target", "for"],
            ],
        )
        .unwrap();

        assert_eq!(matched, &["choose", "a", "new", "target", "for"]);
        assert_eq!(TokenWordView::new(rest).word_refs(), ["target", "spell"]);
    }

    #[test]
    fn starts_with_any_phrase_matches_any_prefix_choice() {
        let tokens = lex_line("for each opponent draw a card", 0).unwrap();
        assert!(starts_with_any_phrase(
            &tokens,
            &[
                &["each", "player"],
                &["for", "each", "opponent"],
                &["target", "opponent"],
            ],
        ));
    }

    #[test]
    fn split_lexed_once_before_suffix_finds_prefix_before_full_tail_match() {
        let tokens = lex_line(
            "untap all creatures during each other player's untap step",
            0,
        )
        .unwrap();
        let remainder = words_match_prefix(&tokens, &["untap", "all"]).unwrap();
        let (subject_tokens, ()) = split_lexed_once_before_suffix(remainder, 1, || {
            phrase(&["during", "each", "other", "player's", "untap", "step"])
        })
        .unwrap();
        assert_eq!(
            TokenWordView::new(subject_tokens).word_refs(),
            ["creatures"]
        );
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
        let parser = (kw("target").void(), cut_err(kw("opponent")).void()).void();
        let result = try_parse_all(&tokens, parser, "test");
        assert!(result.is_err());
    }
}
