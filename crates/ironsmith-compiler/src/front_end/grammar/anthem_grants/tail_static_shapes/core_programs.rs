use super::*;

pub(super) fn parse_isnt_creature_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    let subject_tokens = repeat_till(1.., any.void(), peek(parse_negated_creature_tail))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    parse_negated_creature_tail.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(trim_lexed_commas(subject_tokens))
}

pub(super) fn parse_negated_creature_tail(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        alt((primitives::kw("isnt"), primitives::kw("isn't"))).void(),
        (
            alt((primitives::kw("is"), primitives::kw("are"))),
            primitives::kw("not"),
        )
            .void(),
        (
            alt((primitives::kw("is"), primitives::kw("are"))),
            primitives::phrase(&["no", "longer"]),
        )
            .void(),
    ))
    .parse_next(input)?;
    winnow::combinator::opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures")))
        .void()
        .parse_next(input)
}

pub(super) fn take_until_have<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(parse_have))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

pub(super) fn parse_have(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("has"), primitives::kw("have")))
        .void()
        .parse_next(input)
}

pub(super) fn parse_fixed_power_toughness(input: &mut LexStream<'_>) -> WResult<(i32, i32)> {
    let raw = primitives::word_parser_text.parse_next(input)?;
    let parsed = leaf::parse_leaf_power_toughness_complete(raw).map_err(|_| {
        primitives::backtrack_err("base power/toughness", "fixed power/toughness value")
    })?;
    match (parsed.power, parsed.toughness) {
        (PtValue::Fixed(power), PtValue::Fixed(toughness)) => Ok((power, toughness)),
        _ => Err(primitives::backtrack_err(
            "base power/toughness",
            "fixed numeric power/toughness value",
        )),
    }
}

pub(super) fn contains_parser<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    let mut input = LexStream::new(tokens);
    loop {
        let mut candidate = input.clone();
        if make_parser().parse_next(&mut candidate).is_ok() {
            return true;
        }
        if take_token(&mut input).is_err() {
            return false;
        }
    }
}

pub(super) fn has_prefix(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(words)).is_some()
}
