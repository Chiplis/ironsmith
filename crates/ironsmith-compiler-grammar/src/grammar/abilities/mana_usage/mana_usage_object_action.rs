use super::*;

pub(super) fn token_slice_for_words<'a>(
    tokens: &'a [OwnedLexToken],
    view: &TokenWordView<'a>,
    start: usize,
    end: usize,
) -> Option<&'a [OwnedLexToken]> {
    Some(trim_lexed_commas(
        tokens.get(view.token_span_for_words(start, end)?)?,
    ))
}
