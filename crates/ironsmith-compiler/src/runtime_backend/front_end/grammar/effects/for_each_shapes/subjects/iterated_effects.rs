use super::*;

pub(crate) fn parse_for_each_spent_mana_effect_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachSpentManaEffectShape<'_>> {
    let (subject_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let (_, after_prefix) = primitives::parse_prefix(
        trim_lexed_commas(subject_tokens),
        primitives::phrase(&["for", "each", "mana", "from"]),
    )?;
    let (source_tokens, ()) = primitives::split_lexed_once_before_suffix(after_prefix, 1, || {
        primitives::phrase(&["spent", "to", "cast", "this", "spell"]).void()
    })?;
    let source_tokens = trim_lexed_commas(source_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    (!source_tokens.is_empty() && !effect_tokens.is_empty()).then_some(
        ForEachSpentManaEffectShape {
            source_tokens,
            effect_tokens,
        },
    )
}

pub(crate) fn parse_for_each_dynamic_target_effect_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachDynamicTargetEffectShape<'_>> {
    let (subject_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let (_, filter_tokens) = primitives::parse_prefix(
        trim_lexed_commas(subject_tokens),
        (
            primitives::phrase(&["for", "each", "of"]),
            primitives::kw("x"),
            primitives::kw("target"),
        )
            .void(),
    )?;
    let filter_tokens = trim_lexed_commas(filter_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    (!filter_tokens.is_empty() && !effect_tokens.is_empty()).then_some(
        ForEachDynamicTargetEffectShape {
            filter_tokens,
            effect_tokens,
        },
    )
}
