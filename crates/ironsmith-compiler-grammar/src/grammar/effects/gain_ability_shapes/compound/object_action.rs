use super::*;

pub fn parse_attached_and_related_get_ability_shape(
    tokens: &[OwnedLexToken],
) -> Option<AttachedAndRelatedGetAbilityShape<'_>> {
    let shape = parse_get_then_ability_shape(tokens)?;
    if !matches!(
        shape.ability_verb,
        SharedAbilityVerb::Gain | SharedAbilityVerb::Has
    ) {
        return None;
    }
    let subject = crate::grammar::primitives::probe_all(
        shape.subject_tokens,
        parse_attached_and_related_subject,
        "attached object and related creatures subject",
    )?;
    let (ability_tokens, ()) =
        primitives::split_lexed_once_before_suffix(shape.ability_tokens, 1, || {
            (
                primitives::phrase(&["until", "end", "of", "turn"]),
                primitives::sentence_end(),
            )
                .void()
        })?;
    let ability_tokens = nonempty_trimmed(ability_tokens)?;
    Some(AttachedAndRelatedGetAbilityShape {
        subject,
        pump_tokens: shape.pump_tokens,
        ability_tokens,
        duration: Until::EndOfTurn,
    })
}

pub fn parse_attached_and_related_get_shape(
    tokens: &[OwnedLexToken],
) -> Option<AttachedAndRelatedGetShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (get_token, (), after_get) = primitives::find_prefix(tokens, || get_verb)?;
    let subject_tokens = nonempty_trimmed(tokens.get(..get_token)?)?;
    let subject = crate::grammar::primitives::probe_all(
        subject_tokens,
        parse_attached_and_related_subject,
        "attached object and related creatures subject",
    )?;
    let (pump_tokens, ()) = primitives::split_lexed_once_before_suffix(after_get, 1, || {
        (
            primitives::phrase(&["until", "end", "of", "turn"]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let pump_tokens = nonempty_trimmed(pump_tokens)?;
    Some(AttachedAndRelatedGetShape {
        subject,
        pump_tokens,
        duration: Until::EndOfTurn,
    })
}
