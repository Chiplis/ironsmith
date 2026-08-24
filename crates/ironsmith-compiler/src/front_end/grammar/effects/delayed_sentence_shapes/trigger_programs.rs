use super::*;

/// Parse the object-kind portion of a delayed
/// "target ... is put into your graveyard" trigger.
pub fn parse_delayed_target_put_into_your_graveyard_subject(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let tokens = trimmed(tokens);
    let (_, after_target) = primitives::parse_prefix(tokens, primitives::kw("target"))?;
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(after_target, 1, || {
        (
            primitives::phrase(&["is", "put", "into", "your", "graveyard"]),
            eof,
        )
            .void()
    })?;
    let subject_tokens = trimmed(subject_tokens);
    (!subject_tokens.is_empty()).then_some(subject_tokens)
}

/// Parse a delayed trigger that watches the object selected immediately
/// before this sentence: "When it's put into a graveyard this turn, ...".
///
/// Oracle's contraction is normalized by the lexer to `its`, so keep that
/// exact event-shaped spelling alongside the uncontracted and demonstrative
/// forms. The narrow `put into a graveyard` tail prevents a possessive `its`
/// from being mistaken for this pronoun elsewhere.
pub fn is_delayed_prior_object_put_into_a_graveyard(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        trimmed(tokens),
        (
            alt((
                semantic_phrase(&["its", "put", "into", "a", "graveyard"]),
                semantic_phrase(&["it", "is", "put", "into", "a", "graveyard"]),
                semantic_phrase(&["that", "card", "is", "put", "into", "a", "graveyard"]),
                semantic_phrase(&["that", "creature", "is", "put", "into", "a", "graveyard"]),
                semantic_phrase(&["that", "permanent", "is", "put", "into", "a", "graveyard"]),
            )),
            eof,
        )
            .void(),
        "delayed prior-object graveyard trigger",
    )
    .is_ok()
}

pub fn parse_delayed_dies_shape(tokens: &[OwnedLexToken]) -> Option<DelayedDiesShape<'_>> {
    let tokens = trimmed(tokens);
    let (header_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let effect_tokens = trimmed(effect_tokens);
    if effect_tokens.is_empty() {
        return None;
    }
    let (_, trigger_tokens) = primitives::parse_prefix(trimmed(header_tokens), dies_intro)?;

    if let Some(((), after_that)) =
        primitives::parse_prefix(trigger_tokens, primitives::kw("that").void())
        && primitives::split_lexed_once_before_suffix(after_that, 0, || {
            (primitives::phrase(&["dies", "this", "turn"]), eof).void()
        })
        .is_some()
    {
        return Some(DelayedDiesShape::ThatReference { effect_tokens });
    }

    // A definite filtered noun after a prior targeted action names that
    // exact target rather than every object matching the filter: "When the
    // permanent you don't control dies this turn, ...". Keep the subject
    // tokens so semantic lowering can retain both its filter and antecedent.
    if let Some((subject_tokens, ())) =
        primitives::split_lexed_once_before_suffix(trigger_tokens, 2, || {
            (primitives::phrase(&["dies", "this", "turn"]), eof).void()
        })
    {
        let subject_tokens = trimmed(subject_tokens);
        if primitives::parse_prefix(subject_tokens, primitives::kw("the").void()).is_some() {
            return Some(DelayedDiesShape::DefinitePriorTarget {
                subject_tokens,
                effect_tokens,
            });
        }
    }

    let (subject_tokens, ()) =
        primitives::split_lexed_once_before_suffix(trigger_tokens, 1, || {
            (dies_this_way_suffix, eof).void()
        })?;
    let subject_tokens = trimmed(subject_tokens);
    (!subject_tokens.is_empty()).then_some(DelayedDiesShape::ThisWay {
        subject_tokens,
        effect_tokens,
    })
}
