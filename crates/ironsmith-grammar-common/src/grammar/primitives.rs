use std::{cell::Cell, fmt};

use winnow::combinator::{alt, eof, opt, peek, preceded, repeat};
use winnow::error::{ContextError, ErrMode, ParseError, ParserError, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::{any, literal, take_till};

use crate::cards::builders::{CardTextError, TextSpan};
use crate::mana::ManaSymbol;

pub use super::super::lexer::TokenWordView;
use super::super::lexer::{LexStream, LexToken, TokenKind};

const POWER_AXIS_SUFFIXES: &[&[&str]] = &[&["power"], &["total", "power"], &["base", "power"]];
const TOUGHNESS_WORD: &str = "toughness";
const OR_WORD: &str = "or";
const COMPARISON_OR_TAIL_WORDS: &[&str] = &["less", "greater", "more", "fewer"];
const THAN_WORD: &str = "than";
const EQUAL_WORD: &str = "equal";

pub struct MaybeTrace<P, D> {
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

pub fn maybe_trace<P, D>(name: D, parser: P) -> MaybeTrace<P, D> {
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

pub fn parse_all<'a, O>(
    tokens: &'a [LexToken],
    parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<O, CardTextError> {
    let mut parser = maybe_trace(label, parser);
    parser
        .parse(LexStream::new(tokens))
        .map_err(|err| format_parse_error(label, err, None))
}

pub fn parse_all_with_display_line<'a, O>(
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

pub fn parse_prefix<'a, O>(
    tokens: &'a [LexToken],
    mut parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
) -> Option<(O, &'a [LexToken])> {
    let (rest, parsed) = parser.parse_peek(LexStream::new(tokens)).ok()?;
    let remaining = tokens.get(tokens.len().checked_sub(rest.len())?..)?;
    Some((parsed, remaining))
}

pub fn parse_all_or_none<'a, O>(
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
/// Adapts a winnow parser into the `SubjectVerbPrimitiveParser` convention:
///
/// - Winnow backtrack (pattern mismatch) → `Ok(None)`
/// - Winnow cut (hard parse error) → `Err(CardTextError)`
/// - Winnow success with trailing tokens → `Err(CardTextError)`
/// - Winnow success consuming all input → `Ok(Some(value))`
pub fn try_parse_all<'a, O>(
    tokens: &'a [LexToken],
    parser: impl Parser<LexStream<'a>, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<Option<O>, CardTextError> {
    parse_all_or_none(tokens, parser, label)
}

pub fn find_prefix<'a, O, P, F>(
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

pub fn locate_token_index(
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

pub fn contains_word(tokens: &[LexToken], expected: &'static str) -> bool {
    find_prefix(tokens, || kw(expected)).is_some()
}

pub fn has_phrase(tokens: &[LexToken], expected: &'static [&'static str]) -> bool {
    find_phrase_start(tokens, expected).is_some()
}

pub fn has_any_phrase(tokens: &[LexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    phrases
        .iter()
        .any(|phrase_words| has_phrase(tokens, phrase_words))
}

pub fn find_phrase_start(tokens: &[LexToken], expected: &'static [&'static str]) -> Option<usize> {
    find_prefix(tokens, || phrase(expected)).map(|(idx, _, _)| idx)
}

/// Constructs a `Backtrack` error with a label and expected description.
///
/// Use this instead of manually constructing `ContextError` + `ErrMode::Backtrack`.
pub fn backtrack_err(label: &'static str, expected: &'static str) -> ErrMode<ContextError> {
    let mut err = ContextError::new();
    err.push(StrContext::Label(label));
    err.push(StrContext::Expected(StrContextValue::Description(expected)));
    ErrMode::Backtrack(err)
}

/// Constructs a `Cut` error with a label and expected description.
pub fn cut_err_ctx(label: &'static str, expected: &'static str) -> ErrMode<ContextError> {
    let mut err = ContextError::new();
    err.push(StrContext::Label(label));
    err.push(StrContext::Expected(StrContextValue::Description(expected)));
    ErrMode::Cut(err)
}

pub fn token_slice_span(tokens: &[LexToken]) -> Option<TextSpan> {
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

pub fn token_kind<'a>(
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

pub fn word_text<'a>(input: &mut LexStream<'a>) -> Result<&'a str, ErrMode<ContextError>> {
    let token: &'a LexToken = any.parse_next(input)?;
    token.as_word().ok_or_else(|| backtrack_err("word", "word"))
}

/// Like `word_text` but returns the normalized `parser_text` (lowercased,
/// apostrophe-normalized) instead of the original slice.  Use this as the
/// discriminant inside `dispatch!` so that branch labels can be written in
/// lowercase regardless of how the source text was capitalized.
pub fn word_parser_text<'a>(input: &mut LexStream<'a>) -> Result<&'a str, ErrMode<ContextError>> {
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

pub fn kw<'a>(
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

pub fn comma<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Comma, "comma")
}

pub fn period<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Period, "period")
}

pub fn colon<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Colon, "colon")
}

pub fn semicolon<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Semicolon, "semicolon")
}

pub fn lparen<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::LParen, "left parenthesis")
}

pub fn rparen<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::RParen, "right parenthesis")
}

pub fn quote<'a>() -> impl Parser<LexStream<'a>, &'a LexToken, ErrMode<ContextError>> {
    punctuation(TokenKind::Quote, "quote")
}

/// Matches an optional period followed by end-of-input.
///
/// This is the standard trailing pattern for sentence/block parsers.
pub fn sentence_end<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (opt(period()), eof).void()
}

pub fn end_of_sentence<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    period()
        .void()
        .context(StrContext::Label("end of sentence"))
        .context(StrContext::Expected(StrContextValue::Description("period")))
}

pub fn end_of_block<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    eof.value(())
        .context(StrContext::Label("end of block"))
        .context(StrContext::Expected(StrContextValue::Description(
            "end of token block",
        )))
}

pub fn end_of_sentence_or_block<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
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
pub fn number_token<'a>(input: &mut LexStream<'a>) -> Result<u32, ErrMode<ContextError>> {
    super::leaf::parse_leaf_number_token_lexed.parse_next(input)
}

/// Parse a single mana symbol from the next token (word, number, or
/// `{…}` mana-group).  Returns the individual `ManaSymbol` values found
/// in that token.
pub fn mana_pips_token<'a>(
    input: &mut LexStream<'a>,
) -> Result<Vec<ManaSymbol>, ErrMode<ContextError>> {
    super::leaf::parse_leaf_surface_mana_pip_lexed
        .map(super::leaf::LeafManaPipToken::into_pip)
        .parse_next(input)
}

/// Skip one or more tokens that are commas and/or the keyword "or".
/// Suitable as the separator argument to `separated()`.
pub fn comma_or_separator<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
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

/// Skip tokens that are noise words in mana clauses
/// ("mana", "to", "your", "their", "its", "pool", articles).
pub fn skip_mana_noise<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
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
pub fn collect_mana_symbols<'a>(
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
pub fn collect_mana_pip_groups<'a>(
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

pub fn phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    maybe_trace(
        PhraseTraceLabel(expected),
        move |input: &mut LexStream<'a>| {
            for word in expected {
                if let Err(err) = kw(word).parse_next(input) {
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

pub fn any_phrase<'a, 'b>(
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
        let mut inside_quotes = false;
        while input.peek_token().is_some() {
            if input
                .peek_token()
                .is_some_and(|token| token.kind == TokenKind::Quote)
            {
                inside_quotes = !inside_quotes;
                any.parse_next(input)?;
                continue;
            }

            if !inside_quotes && peek(make_separator()).parse_next(input).is_ok() {
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

pub fn split_lexed_once_on_separator<'a, P, F>(
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

pub fn split_lexed_once_before_suffix<'a, O, P, F>(
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

pub fn split_lexed_once_on_delimiter(
    tokens: &[LexToken],
    delimiter: TokenKind,
) -> Option<(&[LexToken], &[LexToken])> {
    let parser = take_till(0.., move |token: &LexToken| token.kind == delimiter).with_taken();
    let (rest, ((_, head), _)) = (parser, token_kind(delimiter))
        .parse_peek(LexStream::new(tokens))
        .ok()?;
    let remaining = tokens.get(tokens.len().checked_sub(rest.len())?..)?;
    Some((head, remaining))
}

pub fn split_lexed_once_on_comma(tokens: &[LexToken]) -> Option<(&[LexToken], &[LexToken])> {
    split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
}

fn should_keep_and_for_power_toughness_axis<'a>(
    current: &'a [LexToken],
    remaining: &'a [LexToken],
) -> bool {
    let current_words = TokenWordView::new(current).word_refs();
    let remaining_words = TokenWordView::new(remaining).word_refs();
    POWER_AXIS_SUFFIXES
        .iter()
        .any(|suffix| parse_word_sequence_suffix(&current_words, suffix).is_some())
        && parse_word_sequence_prefix(&remaining_words, &[TOUGHNESS_WORD]).is_some()
}

pub fn split_lexed_slices_on_and(tokens: &[LexToken]) -> Vec<&[LexToken]> {
    let raw = split_lexed_slices_on_separator(tokens, || phrase(&["and"]));
    let mut merged = Vec::new();
    let mut idx = 0usize;
    while idx < raw.len() {
        if idx + 1 < raw.len() && should_keep_and_for_power_toughness_axis(raw[idx], raw[idx + 1]) {
            let start = raw[idx]
                .as_ptr()
                .addr()
                .saturating_sub(tokens.as_ptr().addr())
                / std::mem::size_of::<LexToken>();
            let end = raw[idx + 1]
                .as_ptr()
                .addr()
                .saturating_sub(tokens.as_ptr().addr())
                / std::mem::size_of::<LexToken>()
                + raw[idx + 1].len();
            if start < end && end <= tokens.len() {
                merged.push(&tokens[start..end]);
                idx += 2;
                continue;
            }
        }
        merged.push(raw[idx]);
        idx += 1;
    }
    merged
}

/// Splits a coordinated Oracle list on either `and` or `and/or` while
/// preserving the power-and-toughness axis as one phrase.  The latter is a
/// single lexical word, so callers that only split on `and` otherwise leave a
/// leading `and/or` attached to the final list item.
pub fn split_lexed_slices_on_list_conjunction(tokens: &[LexToken]) -> Vec<&[LexToken]> {
    let raw = split_lexed_slices_on_separator(tokens, || alt((kw("and"), kw("and/or"))).void());
    let mut merged = Vec::new();
    let mut idx = 0usize;
    while idx < raw.len() {
        if idx + 1 < raw.len() && should_keep_and_for_power_toughness_axis(raw[idx], raw[idx + 1]) {
            let start = raw[idx]
                .as_ptr()
                .addr()
                .saturating_sub(tokens.as_ptr().addr())
                / std::mem::size_of::<LexToken>();
            let end = raw[idx + 1]
                .as_ptr()
                .addr()
                .saturating_sub(tokens.as_ptr().addr())
                / std::mem::size_of::<LexToken>()
                + raw[idx + 1].len();
            if start < end && end <= tokens.len() {
                merged.push(&tokens[start..end]);
                idx += 2;
                continue;
            }
        }
        merged.push(raw[idx]);
        idx += 1;
    }
    merged
}

pub fn split_lexed_slices_on_comma(tokens: &[LexToken]) -> Vec<&[LexToken]> {
    split_lexed_slices_on_separator(tokens, || comma().void())
}

fn is_comparison_or_delimiter(previous_word: Option<&str>, next_word: Option<&str>) -> bool {
    if next_word.is_some_and(|word| COMPARISON_OR_TAIL_WORDS.contains(&word)) {
        return true;
    }

    previous_word == Some(THAN_WORD) && next_word == Some(EQUAL_WORD)
}

pub fn split_lexed_slices_on_or(tokens: &[LexToken]) -> Vec<&[LexToken]> {
    split_lexed_slices_with_parser(tokens, || parse_segment_until_or_separator)
}

pub fn split_lexed_slices_on_commas_or_semicolons(tokens: &[LexToken]) -> Vec<&[LexToken]> {
    split_lexed_slices_on_separator(tokens, || alt((comma().void(), semicolon().void())))
}

pub fn split_lexed_slices_on_period(tokens: &[LexToken]) -> Vec<&[LexToken]> {
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

            if token.is_word(OR_WORD) {
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
        } else if token.is_word(OR_WORD) {
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

pub fn strip_lexed_prefix_phrase<'a>(
    tokens: &'a [LexToken],
    phrase_words: &'static [&'static str],
) -> Option<&'a [LexToken]> {
    parse_prefix(tokens, phrase(phrase_words)).map(|(_, rest)| rest)
}

pub fn strip_lexed_prefix_phrases<'a>(
    tokens: &'a [LexToken],
    phrases: &[&'static [&'static str]],
) -> Option<(&'static [&'static str], &'a [LexToken])> {
    parse_prefix(tokens, any_phrase(phrases))
}

pub fn starts_with_any_phrase(tokens: &[LexToken], phrases: &[&'static [&'static str]]) -> bool {
    parse_prefix(tokens, any_phrase(phrases)).is_some()
}

pub fn strip_lexed_suffix_phrase<'a>(
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
    let suffix_range = words.token_span_for_words(keep_word_count, word_refs.len())?;
    Some(&tokens[..suffix_range.start])
}

pub fn strip_lexed_suffix_phrases<'a, 'b>(
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
// `items_start_with`.  They bridge the gap between old word-slice-based
// code and the token-stream-based grammar primitives.
// ---------------------------------------------------------------------------

/// Checks whether the word pieces at the start of `tokens` match `expected`,
/// using `TokenWordView` for proper multi-word token splitting (e.g.,
/// hyphenated words like "life-gaining" → ["life", "gaining"]).
/// Returns the token slice after the matched prefix.
pub fn match_word_prefix<'a>(tokens: &'a [LexToken], expected: &[&str]) -> Option<&'a [LexToken]> {
    if expected.is_empty() {
        return Some(tokens);
    }
    let view = TokenWordView::new(tokens);
    if !view.parses_prefix(expected) {
        return None;
    }
    let token_end = view.token_index_after_words(expected.len())?;
    Some(&tokens[token_end..])
}

pub fn match_any_word_prefix<'a>(
    tokens: &'a [LexToken],
    phrases: &[&'static [&'static str]],
) -> Option<(&'static [&'static str], &'a [LexToken])> {
    phrases
        .iter()
        .find_map(|phrase| match_word_prefix(tokens, phrase).map(|rest| (*phrase, rest)))
}

/// Checks whether the word pieces at the end of `tokens` match `expected`,
/// using `TokenWordView` for proper multi-word token splitting.
/// Returns the token slice before the matched suffix.
pub fn match_word_suffix<'a>(tokens: &'a [LexToken], expected: &[&str]) -> Option<&'a [LexToken]> {
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
    let token_start = view
        .token_span_for_words(suffix_start_word, view.len())?
        .start;
    Some(&tokens[..token_start])
}

// ---------------------------------------------------------------------------
// Word-slice parsers
//
// These combinators operate on `&[&str]` slices (already-split word lists)
// rather than on `LexStream`.  They are shared by `object_filters`,
// `grammar::filters`, and `effect_sentences::chain_carry`.
// ---------------------------------------------------------------------------

/// Input type for word-slice parsers.
pub type WordSliceInput<'a> = &'a [&'a str];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordSequenceSpan {
    pub start: usize,
    pub len: usize,
}

fn dynamic_word_sequence<'a, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<WordSliceInput<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut WordSliceInput<'a>| {
        for expected_word in expected {
            let Some((word, rest)) = input.split_first() else {
                return Err(backtrack_err("word sequence", "expected word"));
            };
            if word != expected_word {
                return Err(backtrack_err("word sequence", "expected word"));
            }
            *input = rest;
        }
        Ok(())
    }
}

pub fn parse_word_sequence_complete(words: &[&str], expected: &[&str]) -> Option<()> {
    let mut input: WordSliceInput<'_> = words;
    (dynamic_word_sequence(expected), word_slice_eof)
        .void()
        .parse_next(&mut input)
        .ok()
}

pub fn parse_word_sequence_prefix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    let mut input: WordSliceInput<'a> = words;
    dynamic_word_sequence(expected)
        .parse_next(&mut input)
        .ok()?;
    Some(input)
}

pub fn parse_word_sequence_suffix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    let split = words.len().checked_sub(expected.len())?;
    parse_word_sequence_complete(&words[split..], expected)?;
    Some(&words[..split])
}

pub fn parse_word_sequence_span(words: &[&str], expected: &[&str]) -> Option<WordSequenceSpan> {
    if expected.is_empty() {
        return None;
    }
    for start in 0..=words.len().saturating_sub(expected.len()) {
        if parse_word_sequence_prefix(&words[start..], expected).is_some() {
            return Some(WordSequenceSpan {
                start,
                len: expected.len(),
            });
        }
    }
    None
}

/// Matches a single word (exact, case-sensitive) and consumes it, returning
/// the matched `&str`.
pub fn word_slice_exact<'a>(
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
pub fn word_slice_eof<'a>(input: &mut WordSliceInput<'a>) -> Result<(), ErrMode<ContextError>> {
    if input.is_empty() {
        Ok(())
    } else {
        Err(backtrack_err("word input", "end of words"))
    }
}

/// Runs `parser` on `words`, succeeding only if the entire slice is consumed.
pub fn parse_full_word_slice<'a, O>(
    words: &'a [&'a str],
    parser: impl Parser<WordSliceInput<'a>, O, ErrMode<ContextError>>,
) -> Option<O> {
    let mut input: WordSliceInput<'a> = words;
    (parser, word_slice_eof)
        .map(|(parsed, ())| parsed)
        .parse_next(&mut input)
        .ok()
}

#[cfg(test)]
#[path = "primitives/tests.rs"]
mod tests;
