use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::{leaf, primitives};
use super::{SimpleCopyReferenceKind, TokenLifecycleSentenceKind, TriggerSourceSubject};

pub(super) fn parse_trigger_source_subject_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<TriggerSourceSubject> {
    alt((
        (
            primitives::word_slice_exact("a"),
            primitives::word_slice_exact("source"),
        )
            .value(TriggerSourceSubject::AnySource),
        primitives::word_slice_exact("source").value(TriggerSourceSubject::AnySource),
    ))
    .parse_next(input)
}

pub(super) fn parse_simple_copy_reference_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SimpleCopyReferenceKind> {
    (
        primitives::word_slice_exact("copy"),
        alt((
            primitives::word_slice_exact("it").value(SimpleCopyReferenceKind::It),
            primitives::word_slice_exact("this").value(SimpleCopyReferenceKind::This),
            (
                primitives::word_slice_exact("that"),
                primitives::word_slice_exact("card"),
            )
                .value(SimpleCopyReferenceKind::ThatCard),
            (
                primitives::word_slice_exact("the"),
                primitives::word_slice_exact("exiled"),
                primitives::word_slice_exact("card"),
            )
                .value(SimpleCopyReferenceKind::ExiledCard),
            primitives::word_slice_exact("that").value(SimpleCopyReferenceKind::That),
        )),
    )
        .map(|(_, reference)| reference)
        .parse_next(input)
}

pub(super) fn parse_token_lifecycle_sentence_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<TokenLifecycleSentenceKind> {
    alt((
        parse_exile_created_token_when_source_leaves_words,
        parse_sacrifice_source_when_created_token_leaves_words,
    ))
    .parse_next(input)
}

fn parse_exile_created_token_when_source_leaves_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<TokenLifecycleSentenceKind> {
    alt((
        primitives::word_slice_exact("exile"),
        primitives::word_slice_exact("exiles"),
    ))
    .parse_next(input)?;
    parse_created_token_reference_words.parse_next(input)?;
    primitives::word_slice_exact("when").parse_next(input)?;
    parse_source_before_leaves_battlefield_words.parse_next(input)?;
    Ok(TokenLifecycleSentenceKind::ExileCreatedTokenWhenSourceLeaves)
}

fn parse_sacrifice_source_when_created_token_leaves_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<TokenLifecycleSentenceKind> {
    alt((
        primitives::word_slice_exact("sacrifice"),
        primitives::word_slice_exact("sacrifices"),
    ))
    .parse_next(input)?;
    let mut when_word = None;
    for (index, word) in input.iter().enumerate() {
        if *word == "when" {
            when_word = Some(index);
            break;
        }
    }
    let when_word =
        when_word.ok_or_else(|| primitives::backtrack_err("token lifecycle", "when"))?;
    let source_words = &input[..when_word];
    if !is_trigger_source_reference_words(source_words) {
        return Err(primitives::backtrack_err(
            "token lifecycle",
            "source reference",
        ));
    }
    *input = &input[when_word + 1..];
    (
        primitives::word_slice_exact("that"),
        primitives::word_slice_exact("token"),
        primitives::word_slice_exact("leaves"),
        primitives::word_slice_exact("the"),
        primitives::word_slice_exact("battlefield"),
    )
        .parse_next(input)?;
    Ok(TokenLifecycleSentenceKind::SacrificeSourceWhenCreatedTokenLeaves)
}

fn parse_created_token_reference_words(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    alt((
        (
            primitives::word_slice_exact("that"),
            primitives::word_slice_exact("token"),
        )
            .void(),
        (
            primitives::word_slice_exact("those"),
            primitives::word_slice_exact("tokens"),
        )
            .void(),
        primitives::word_slice_exact("them").void(),
        primitives::word_slice_exact("it").void(),
    ))
    .parse_next(input)
}

fn parse_source_before_leaves_battlefield_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<()> {
    let suffix_len = 3;
    if input.len() <= suffix_len {
        return Err(primitives::backtrack_err(
            "token lifecycle",
            "source before leaves the battlefield",
        ));
    }
    let source_end = input.len() - suffix_len;
    if !is_trigger_source_reference_words(&input[..source_end]) {
        return Err(primitives::backtrack_err(
            "token lifecycle",
            "source reference",
        ));
    }
    *input = &input[source_end..];
    (
        primitives::word_slice_exact("leaves"),
        primitives::word_slice_exact("the"),
        primitives::word_slice_exact("battlefield"),
    )
        .void()
        .parse_next(input)
}

fn is_trigger_source_reference_words(words: &[&str]) -> bool {
    leaf::parse_leaf_this_source_reference_words(words).is_some()
        || crate::runtime_backend::util::source_reference_surface_for_words(words).is_some()
}
