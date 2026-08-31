use super::*;

pub(super) fn parse_sticker_filter_words(
    words: &[&str],
) -> Option<(crate::events::KeywordActionKind, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    let sticker = crate::grammar::primitives::take_leaf(&mut input, parse_sticker_filter_prefix)?;
    Some((sticker, initial_len.saturating_sub(input.len())))
}

pub(super) fn try_apply_sticker_filter_clause(
    filter: &mut ObjectFilter,
    words: &mut Vec<&str>,
) -> bool {
    for start in 0..words.len() {
        let mut input: primitives::WordSliceInput<'_> = &words[start..];
        let initial_len = input.len();
        let Ok(sticker) = (
            primitives::word_slice_exact("with"),
            parse_sticker_filter_prefix,
        )
            .map(|(_, sticker)| sticker)
            .parse_next(&mut input)
        else {
            continue;
        };
        let consumed = initial_len.saturating_sub(input.len());
        filter.sticker = Some(sticker);
        words.drain(start..start + consumed);
        return true;
    }
    false
}

fn parse_sticker_filter_prefix<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> winnow::error::ModalResult<crate::events::KeywordActionKind> {
    use crate::events::KeywordActionKind;
    let _ = winnow::combinator::opt(alt((
        primitives::word_slice_exact("a"),
        primitives::word_slice_exact("an"),
        primitives::word_slice_exact("the"),
    )))
    .parse_next(input)?;
    let action = alt((
        primitives::word_slice_exact("art").value(KeywordActionKind::ArtSticker),
        primitives::word_slice_exact("ability").value(KeywordActionKind::AbilitySticker),
        (
            primitives::word_slice_exact("power"),
            primitives::word_slice_exact("and"),
            primitives::word_slice_exact("toughness"),
        )
            .value(KeywordActionKind::PowerToughnessSticker),
    ))
    .parse_next(input)?;
    (
        primitives::word_slice_exact("sticker"),
        primitives::word_slice_exact("on"),
        primitives::word_slice_exact("it"),
    )
        .void()
        .parse_next(input)?;
    Ok(action)
}

pub(super) fn try_apply_required_both_colors_clause(
    filter: &mut ObjectFilter,
    words: &mut Vec<&str>,
) -> bool {
    for start in 0..words.len() {
        let mut input: primitives::WordSliceInput<'_> = &words[start..];
        let initial_len = input.len();
        let Ok(colors) = parse_required_both_colors_prefix(&mut input) else {
            continue;
        };
        let consumed = initial_len.saturating_sub(input.len());
        filter.required_colors = Some(colors);
        words.drain(start..start + consumed);
        return true;
    }
    false
}

fn parse_required_both_colors_prefix<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> winnow::error::ModalResult<ColorSet> {
    primitives::word_slice_exact("both").parse_next(input)?;
    let left = parse_required_color_word(input)?;
    primitives::word_slice_exact("and").parse_next(input)?;
    let right = parse_required_color_word(input)?;
    Ok(left.union(right))
}

fn parse_required_color_word<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> winnow::error::ModalResult<ColorSet> {
    let word: &str = winnow::token::any.parse_next(input)?;
    crate::grammar::leaf::parse_leaf_color_complete(word)
        .map_err(|_| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))
}
