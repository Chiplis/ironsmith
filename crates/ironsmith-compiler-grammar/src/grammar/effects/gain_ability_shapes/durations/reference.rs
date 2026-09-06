use crate::cards::builders::SourcePredicateAst;
use super::*;

pub(super) fn source_remains_on_battlefield(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    for_as_long_as.parse_next(input)?;
    alt((
        (
            alt((
                primitives::word_slice_exact("this"),
                primitives::word_slice_exact("thiss"),
            )),
            opt(alt((
                primitives::word_slice_exact("artifact"),
                primitives::word_slice_exact("creature"),
                primitives::word_slice_exact("enchantment"),
                primitives::word_slice_exact("permanent"),
                primitives::word_slice_exact("source"),
            ))),
        )
            .void(),
        primitives::word_slice_exact("source").void(),
    ))
    .parse_next(input)?;
    alt((
        primitives::word_slice_exact("remain"),
        primitives::word_slice_exact("remains"),
    ))
    .parse_next(input)?;
    primitives::word_slice_exact("on").parse_next(input)?;
    opt(primitives::word_slice_exact("the")).parse_next(input)?;
    primitives::word_slice_exact("battlefield").parse_next(input)?;
    Ok(Until::ThisLeavesTheBattlefield)
}

pub(super) fn source_tapped_duration<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "as", "long", "as"]).parse_next(input)?;
    primitives::kw("this").parse_next(input)?;
    opt(alt((
        primitives::kw("creature"),
        primitives::kw("permanent"),
        primitives::kw("source"),
    )))
    .parse_next(input)?;
    alt((primitives::kw("remains"), primitives::kw("is"))).parse_next(input)?;
    primitives::kw("tapped").parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

/// Parse the duration as a typed suffix over lexer tokens. The returned word
/// span is only a boundary for the surrounding gain-ability parser; semantic
/// recognition is wholly owned by the Winnow grammar above.
pub fn parse_source_tapped_gain_duration_shape(
    tokens: &[OwnedLexToken],
) -> Option<GainAbilityDurationShape> {
    let (start_token, (), rest) = primitives::find_prefix(tokens, || source_tapped_duration)?;
    if !rest.is_empty() {
        return None;
    }
    let start = TokenWordView::new(&tokens[..start_token]).len();
    let len = TokenWordView::new(&tokens[start_token..]).len();
    Some(GainAbilityDurationShape {
        start,
        len,
        duration: Until::SourceUntaps,
        condition: Some(PredicateAst::Source(SourcePredicateAst::SourceIsTapped)),
    })
}
