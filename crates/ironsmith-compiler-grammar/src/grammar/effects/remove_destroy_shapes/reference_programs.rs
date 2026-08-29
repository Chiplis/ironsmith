use super::*;

pub(super) fn target_count_before_target<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        leaf::parse_leaf_target_count_range_prefix_lexed.void(),
        leaf::parse_leaf_choice_count_prefix_lexed.void(),
    ))
    .parse_next(input)?;
    alt((
        primitives::kw("target").void(),
        primitives::kw("targets").void(),
    ))
    .parse_next(input)
}

pub(super) fn has_multi_target_tail(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, (), tail)) = primitives::find_prefix(tokens, || primitives::kw("and").void())
    else {
        return false;
    };
    primitives::parse_prefix(tail, primitives::kw("target")).is_some()
        || primitives::parse_prefix(tail, target_count_before_target).is_some()
}

pub(super) fn parse_destroy_target_and_attached_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyTargetAndAttachedShape<'_>> {
    let (target_tokens, attached_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("and").void())?;
    let target_tokens = trim_lexed_commas(target_tokens);
    let target_starts_with_selection =
        primitives::parse_prefix(target_tokens, primitives::kw("target").void()).is_some()
            || primitives::parse_prefix(target_tokens, target_count_before_target).is_some();
    if !target_starts_with_selection {
        return None;
    }

    let ((), attached_tokens) =
        primitives::parse_prefix(attached_tokens, primitives::kw("all").void())?;
    let (attachment_filter_tokens, attachment_reference_tokens) =
        primitives::split_lexed_once_on_separator(attached_tokens, || {
            primitives::phrase(&["attached", "to"]).void()
        })?;
    let attachment_filter_tokens = trim_lexed_commas(attachment_filter_tokens);
    let attachment_reference_tokens = trim_lexed_commas(attachment_reference_tokens);
    if attachment_filter_tokens.is_empty() {
        return None;
    }

    let demonstrative_antecedent = if exact_tokens(attachment_reference_tokens, &["it"])
        || exact_tokens(attachment_reference_tokens, &["them"])
    {
        None
    } else {
        let [that, noun] = attachment_reference_tokens else {
            return None;
        };
        if !that.is_word("that") {
            return None;
        }
        Some(ironsmith_core::DemonstrativeAntecedentSurface::from_noun(
            noun.as_word()?,
        )?)
    };

    Some(DestroyTargetAndAttachedShape {
        target_tokens,
        attachment_filter_tokens,
        demonstrative_antecedent,
    })
}

pub(super) fn parse_inline_no_regeneration_target(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (target_tokens, ()) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        (
            primitives::kw("and"),
            primitives::kw("it"),
            alt((
                primitives::kw("cant").void(),
                primitives::kw("can't").void(),
                primitives::kw("cannot").void(),
            )),
            primitives::kw("be"),
            primitives::kw("regenerated"),
        )
            .void()
    })?;
    let target_tokens = trim_lexed_commas(target_tokens);
    (!target_tokens.is_empty()).then_some(target_tokens)
}
