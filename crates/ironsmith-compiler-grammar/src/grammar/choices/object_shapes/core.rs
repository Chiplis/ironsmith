use super::*;

pub(super) fn word_occurs<'a, P>(words: &'a [&'a str], parser: P) -> bool
where
    P: Parser<primitives::WordSliceInput<'a>, (), ErrMode<ContextError>>,
{
    let mut input: primitives::WordSliceInput<'a> = words;
    repeat_till(0.., any.void(), peek(parser).void())
        .map(|((), ())| ())
        .parse_next(&mut input)
        .is_ok()
}

pub(super) fn phrase_occurs(words: &[&str], expected: &'static [&'static str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till(0.., any.void(), peek(word_phrase(expected)).void())
        .map(|((), ())| ())
        .parse_next(&mut input)
        .is_ok()
}

pub(super) fn trim_punctuation_edges(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        tokens = &tokens[1..];
    }
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        tokens = &tokens[..tokens.len().saturating_sub(1)];
    }
    tokens
}
