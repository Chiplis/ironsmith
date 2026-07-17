use super::*;

fn same_name_reference_noun<'a>(input: &mut primitives::WordSliceInput<'a>) -> ModalResult<()> {
    alt((
        alt((
            primitives::word_slice_exact("card"),
            primitives::word_slice_exact("cards"),
            primitives::word_slice_exact("creature"),
            primitives::word_slice_exact("creatures"),
            primitives::word_slice_exact("artifact"),
            primitives::word_slice_exact("artifacts"),
            primitives::word_slice_exact("enchantment"),
            primitives::word_slice_exact("enchantments"),
        ))
        .void(),
        alt((
            primitives::word_slice_exact("land"),
            primitives::word_slice_exact("lands"),
            primitives::word_slice_exact("permanent"),
            primitives::word_slice_exact("permanents"),
            primitives::word_slice_exact("spell"),
            primitives::word_slice_exact("spells"),
            primitives::word_slice_exact("object"),
            primitives::word_slice_exact("objects"),
        ))
        .void(),
    ))
    .parse_next(input)
}

fn plural_same_name_reference_noun<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> ModalResult<()> {
    alt((
        primitives::word_slice_exact("cards"),
        primitives::word_slice_exact("creatures"),
        primitives::word_slice_exact("artifacts"),
        primitives::word_slice_exact("enchantments"),
        primitives::word_slice_exact("lands"),
        primitives::word_slice_exact("permanents"),
        primitives::word_slice_exact("spells"),
        primitives::word_slice_exact("objects"),
    ))
    .void()
    .parse_next(input)
}

fn same_name_that_reference<'a>(input: &mut primitives::WordSliceInput<'a>) -> ModalResult<()> {
    alt((
        (
            primitives::word_slice_exact("that"),
            same_name_reference_noun,
        )
            .void(),
        (
            primitives::word_slice_exact("those"),
            plural_same_name_reference_noun,
        )
            .void(),
    ))
    .parse_next(input)
}

pub(crate) fn is_same_name_that_reference_words(words: &[&str]) -> bool {
    primitives::parse_full_word_slice(words, same_name_that_reference).is_some()
}

pub(crate) fn same_name_antecedent_surface_words(
    words: &[&str],
) -> Option<ironsmith_core::SameNameAntecedentSurface> {
    words
        .iter()
        .copied()
        .find_map(ironsmith_core::SameNameAntecedentSurface::from_noun)
}

#[cfg(test)]
mod same_name_reference_tests {
    use super::*;

    #[test]
    fn recognizes_typed_demonstrative_references() {
        assert!(is_same_name_that_reference_words(&["that", "card"]));
        assert!(is_same_name_that_reference_words(&["those", "permanents"]));
        assert!(!is_same_name_that_reference_words(&["those", "permanent"]));
        assert_eq!(
            same_name_antecedent_surface_words(&["target", "nontoken", "creature"]),
            Some(ironsmith_core::SameNameAntecedentSurface::Creature)
        );
    }
}
