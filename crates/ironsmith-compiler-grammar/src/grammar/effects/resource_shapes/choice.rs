use super::*;

pub(super) fn chosen_name_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        alt((primitives::kw("name"), primitives::kw("names"))),
        primitives::phrase(&["chosen", "for", "this"]),
        object_noun,
        repeat::<_, _, (), _, _>(0.., alt((primitives::kw("this"), primitives::kw("way")))),
    )
        .void()
        .parse_next(input)
}

pub fn parse_resource_chosen_name_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<ResourceChosenNameTargetShape<'_>> {
    let tokens = trimmed(tokens);
    let mut search = tokens;
    while !search.is_empty() {
        let consumed = tokens.len().saturating_sub(search.len());
        let (with_idx, (), after_with) =
            primitives::find_prefix(search, || primitives::kw("with").void())?;
        let absolute_with = consumed + with_idx;
        let tail = strip_articles(after_with);
        let base = trimmed(&tokens[..absolute_with]);
        if !base.is_empty() && exact_unit(tail, chosen_name_tail) {
            let words = TokenWordView::new(tail).word_refs();
            let chosen_name_start =
                crate::word_primitives::parse_sequence_start(&words, &["chosen", "for", "this"])?;
            let chosen_name_source = words
                .get(chosen_name_start + 3)
                .and_then(|noun| ironsmith_core::ChosenNameSourceSurface::from_noun(noun))?;
            return Some(ResourceChosenNameTargetShape {
                base_tokens: base,
                chosen_name_source,
            });
        }
        search = after_with;
    }
    None
}
