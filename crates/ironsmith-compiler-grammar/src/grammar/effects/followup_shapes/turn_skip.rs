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
