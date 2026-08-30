use super::*;

pub(super) fn last_semantic_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    let mut last = None;
    loop {
        let token: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = token else {
            break;
        };
        if let Some(piece) = token.parser_word_pieces().last() {
            last = Some(piece.text.as_str());
        }
    }
    last
}

pub(super) fn normalize_action_option(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    loop {
        let Some(((), rest)) =
            primitives::parse_prefix(tokens, alt((semantic_kw("and"), semantic_kw("or"))).void())
        else {
            break;
        };
        tokens = rest;
    }
    trim_lexed_commas(tokens)
}

pub(super) fn contains_semantic_word(
    tokens: &[OwnedLexToken],
    singular: &'static str,
    plural: &'static str,
) -> bool {
    primitives::find_prefix(tokens, || alt((semantic_kw(singular), semantic_kw(plural)))).is_some()
}

pub(super) fn find_semantic_phrase(
    tokens: &[OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<usize> {
    primitives::find_prefix(tokens, || semantic_phrase(phrase)).map(|(idx, (), _)| idx)
}

pub(super) fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (
        repeat::<_, _, (), _, _>(
            0..,
            any.verify(move |token: &&OwnedLexToken| {
                token.parser_word_pieces().is_empty()
                    || ((token.is_word("a") || token.is_word("an") || token.is_word("the"))
                        && !token.is_word(expected))
            })
            .void(),
        ),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

pub(super) fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

pub(super) fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| {
        token.parser_word_pieces().is_empty()
            || token.is_word("a")
            || token.is_word("an")
            || token.is_word("the")
    })
    .void()
    .parse_next(input)
}

pub(super) fn semantic_finish<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}
