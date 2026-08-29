use super::*;

pub(super) fn parse_normalized_word_choice<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    let word = take_word_slice_any(input)?;
    if expected
        .iter()
        .any(|candidate| normalized_word_matches(word, candidate))
    {
        Ok(())
    } else {
        Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ))
    }
}
