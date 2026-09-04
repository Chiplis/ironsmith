use super::*;

/// "If that spell would be put into a graveyard, exile it instead." — the
/// replacement rider on a spell cast from a graveyard.
pub fn is_graveyard_cast_replacement_sentence(replacement: &[OwnedLexToken]) -> bool {
    matches_complete_sequence(
        replacement,
        &[
            THAT_SPELL_YOUR_GRAVEYARD_REPLACEMENT,
            THAT_SPELL_A_GRAVEYARD_REPLACEMENT,
            CAST_THIS_WAY_YOUR_GRAVEYARD_REPLACEMENT,
            CAST_THIS_WAY_A_GRAVEYARD_REPLACEMENT,
        ],
    )
}

pub fn parse_graveyard_cast_replacement_shape(
    cast: &[OwnedLexToken],
    replacement: &[OwnedLexToken],
) -> Option<GraveyardCastReplacementShape> {
    if !is_graveyard_cast_replacement_sentence(replacement) {
        return None;
    }
    parse_graveyard_cast_permission_shape(cast)
}

/// "You may cast target instant or sorcery card from your graveyard [without
/// paying its mana cost] [until end of turn]": the permission alone.
pub fn parse_graveyard_cast_permission_shape(
    cast: &[OwnedLexToken],
) -> Option<GraveyardCastReplacementShape> {
    let (cast, until_end_of_turn) = if let Some(rest) =
        primitives::strip_lexed_prefix_phrase(cast, &["until", "end", "of", "turn"])
    {
        (rest, true)
    } else if let Some(rest) =
        primitives::strip_lexed_prefix_phrase(cast, &["until", "the", "end", "of", "turn"])
    {
        (rest, true)
    } else {
        (cast, false)
    };
    if !starts_sequence(cast, CAST_PREFIX)
        || !contains_sequence_phrase(cast, CAST_FROM_GRAVEYARD)
        || !(contains_sequence_word(cast, "instant") || contains_sequence_word(cast, "sorcery"))
        || !contains_sequence_word(cast, "card")
    {
        return None;
    }
    let (cast, mana_spend_mode) = if let Some(rest) = primitives::strip_lexed_suffix_phrase(
        cast,
        &[
            "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "that", "spell",
        ],
    ) {
        (rest, ironsmith_core::value_model::ManaSpendMode::AnyType)
    } else if primitives::strip_lexed_suffix_phrase(
        cast,
        &["can", "be", "spent", "to", "cast", "that", "spell"],
    )
    .is_some()
    {
        // A mana-spending rider is semantic, not ignorable descriptive text.
        // Only claim the exact supported any-type grammar above.
        return None;
    } else {
        (cast, ironsmith_core::value_model::ManaSpendMode::Normal)
    };
    let additional_mana_cost =
        crate::grammar::primitives::probe_shape(additional_cast_mana_cost(cast))?;
    Some(GraveyardCastReplacementShape {
        until_end_of_turn,
        without_paying_mana_cost: contains_sequence_phrase(cast, WITHOUT_MANA),
        includes_artifact: contains_sequence_word(cast, "artifact"),
        artifact_first: crate::slice_primitives::select_position(cast, |token| {
            token.is_word("artifact")
        })
        .zip(crate::slice_primitives::select_position(cast, |token| {
            token.is_word("instant")
        }))
        .is_some_and(|(artifact, instant)| artifact < instant),
        mana_value_limit: mana_value_limit(cast),
        additional_mana_cost,
        mana_spend_mode,
    })
}

pub(super) fn parse_conditional_self_animate<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalSelfAnimateTail> {
    let initial_len = input.len();
    sequence_phrase(&["if", "this"]).parse_next(input)?;
    let mut comma_at = None;
    let mut saw_isnt = false;
    let mut saw_creature = false;
    while !input.is_empty() {
        let offset = initial_len.saturating_sub(input.len());
        let token: &'a OwnedLexToken = any.parse_next(input)?;
        if token.kind == TokenKind::Comma {
            comma_at = Some(offset);
            break;
        }
        saw_isnt |= token.is_word("isnt");
        saw_creature |= token.is_word("creature");
    }
    let _comma_at = comma_at.ok_or_else(|| {
        primitives::backtrack_err("conditional self animation", "condition comma")
    })?;
    if !saw_isnt || !saw_creature {
        return Err(primitives::backtrack_err(
            "conditional self animation",
            "isn't a creature condition",
        ));
    }
    let effect_start = initial_len.saturating_sub(input.len());
    let mut tail_probe = input.clone();
    sequence_phrase(&["it"]).parse_next(&mut tail_probe)?;
    Ok(ConditionalSelfAnimateTail {
        effect: effect_start..initial_len,
    })
}

pub fn parse_conditional_self_animate_tail(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalSelfAnimateTail> {
    primitives::parse_prefix(tokens, parse_conditional_self_animate).map(|(shape, _)| shape)
}
