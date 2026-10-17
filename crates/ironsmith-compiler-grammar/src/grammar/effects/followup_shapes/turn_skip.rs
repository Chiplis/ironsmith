use super::*;

fn parse_skip_tapped_source_turn_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "if", "you", "would", "begin", "your", "turn", "while", "this", "artifact", "is", "tapped",
    ])
    .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["you", "may", "skip", "that", "turn", "instead"]).parse_next(input)?;
    // Effect-sentence dispatch trims edge punctuation before consulting
    // follow-up grammar, while direct grammar callers retain it.
    opt(primitives::sentence_end()).parse_next(input)?;
    eof.void().parse_next(input)
}

pub fn is_skip_tapped_source_turn_replacement(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_skip_tapped_source_turn_lexed,
        "skip tapped source turn replacement",
    )
    .is_ok()
}

fn parse_if_did_untap_source_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you", "do"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::kw("untap").parse_next(input)?;
    primitives::kw("this").parse_next(input)?;
    alt((
        primitives::kw("artifact"),
        primitives::kw("permanent"),
        primitives::kw("source"),
    ))
    .parse_next(input)?;
    opt(primitives::sentence_end()).parse_next(input)?;
    eof.void().parse_next(input)
}

pub fn is_if_did_untap_source_followup(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_if_did_untap_source_lexed,
        "if skipped turn untap source followup",
    )
    .is_ok()
}

fn parse_may_skip_draw_step_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "you", "would", "begin", "your", "draw", "step"])
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::phrase(&["you", "may", "skip", "that", "step", "instead"]).parse_next(input)?;
    opt(primitives::sentence_end()).parse_next(input)?;
    eof.void().parse_next(input)
}

/// "If you would begin your draw step, you may skip that step instead."
/// (Fasting; CR 614.10): an optional replacement of the step's beginning.
pub fn is_may_skip_draw_step_replacement(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_may_skip_draw_step_lexed,
        "may skip draw step replacement",
    )
    .is_ok()
}

/// The words after "If you do," of the skip replacement's follow-up
/// sentence, or `None` when the sentence is not an "if you do" rider.
pub fn if_you_do_followup_body(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let [if_word, you, did, comma, body @ ..] = tokens else {
        return None;
    };
    (if_word.is_word("if")
        && you.is_word("you")
        && did.is_word("do")
        && comma.is_comma()
        && !body.is_empty())
    .then_some(body)
}
