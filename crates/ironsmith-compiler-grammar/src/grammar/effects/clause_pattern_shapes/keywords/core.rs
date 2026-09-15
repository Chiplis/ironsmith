use super::*;

pub(super) fn parse_explore<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    let subject_tokens = tokens_before(
        input,
        0,
        alt((primitives::kw("explore"), primitives::kw("explores"))).void(),
    )?;
    // The subject is one clause: "... until end of turn. It explores." must
    // not swallow the preceding sentence (Seasoned Dungeoneer).
    if subject_tokens.iter().any(|token| token.is_period()) {
        return Err(primitives::backtrack_err(
            "explore subject",
            "a single-sentence subject",
        ));
    }
    alt((primitives::kw("explore"), primitives::kw("explores"))).parse_next(input)?;
    let repeat = if peek(primitives::sentence_end()).parse_next(input).is_ok() {
        primitives::sentence_end().parse_next(input)?;
        KeywordRepeatShape::Once
    } else {
        let mut again_probe = input.clone();
        if primitives::kw("again").parse_next(&mut again_probe).is_ok()
            && primitives::sentence_end()
                .parse_next(&mut again_probe)
                .is_ok()
        {
            *input = again_probe;
            KeywordRepeatShape::Once
        } else {
            repeat_tail.parse_next(input)?
        }
    };
    Ok(KeywordMechanicShape::Explore {
        subject: classify_subject(subject_tokens),
        repeat,
    })
}

pub(super) fn parse_endure<'a>(input: &mut LexStream<'a>) -> WResult<KeywordMechanicShape<'a>> {
    let subject_tokens = tokens_before(
        input,
        0,
        alt((primitives::kw("endure"), primitives::kw("endures"))).void(),
    )?;
    alt((primitives::kw("endure"), primitives::kw("endures"))).parse_next(input)?;
    let amount_tokens = tokens_before(input, 1, primitives::sentence_end())?;
    primitives::sentence_end().parse_next(input)?;
    Ok(KeywordMechanicShape::Endure {
        subject: classify_subject(subject_tokens),
        amount_tokens,
    })
}
