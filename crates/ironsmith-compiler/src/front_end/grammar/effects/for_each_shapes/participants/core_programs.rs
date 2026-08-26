use super::*;

pub(super) fn tapped_land_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, _, rest) = primitives::find_prefix(tokens, || {
        (
            primitives::kw("tapped"),
            opt(primitives::kw("a")),
            primitives::phrase(&["land", "for", "mana", "this", "turn"]),
        )
            .void()
    })?;
    Some(WhoClauseShape::TappedLandForMana {
        effect_tokens: trim(rest),
    })
}

pub(super) fn negated_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    let (_, after_who) = primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let negation_words = crate::lexer::token_word_refs(after_who);
    let is_cant_failure = negation_words
        .first()
        .is_some_and(|word| matches!(*word, "can't" | "cant" | "cannot"))
        || matches!(negation_words.as_slice(), ["can", "not", ..]);
    let (_, after_negation) = primitives::parse_prefix(after_who, negated_auxiliary)?;
    let effect_tokens = if is_cant_failure {
        // `can't` refers back to the preceding per-player action; everything
        // after it is the failure effect. Preserve commas inside that effect,
        // notably `loses half their life, rounded up`.
        trim(after_negation)
    } else if let Some((_, _, rest)) =
        primitives::find_prefix(tokens, || primitives::comma().void())
    {
        trim(rest)
    } else if let Some((_, _, rest)) =
        primitives::find_prefix(tokens, || primitives::phrase(&["this", "way"]).void())
    {
        trim(rest)
    } else {
        trim(after_negation)
    };
    Some(WhoClauseShape::Negated {
        effect_tokens,
        tagged_filter_tokens: tagged_filter_after_negation(tokens),
        implicit_player_is_iterated: is_cant_failure,
    })
}

pub(super) fn did_this_way_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, _, rest) =
        primitives::find_prefix(tokens, || primitives::phrase(&["this", "way"]).void())?;
    Some(WhoClauseShape::DidThisWay {
        effect_tokens: trim(rest),
        tagged_filter_tokens: tagged_filter_after_action(tokens),
    })
}

pub(super) fn did_action_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    let (_, after_action) = primitives::parse_prefix(
        trim(tokens),
        (
            primitives::kw("who"),
            alt((
                primitives::kw("does"),
                primitives::kw("do"),
                primitives::kw("did"),
            )),
        )
            .void(),
    )?;
    let (effect_tokens, implicit_player_is_you) =
        primitives::find_prefix(tokens, || primitives::comma().void())
            .map(|(_, _, rest)| (trim(rest), true))
            .unwrap_or_else(|| (trim(after_action), false));
    Some(WhoClauseShape::DidAction {
        effect_tokens,
        implicit_player_is_you,
    })
}

pub fn parse_who_clause_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    tapped_land_shape(tokens)
        .or_else(|| negated_shape(tokens))
        .or_else(|| did_this_way_shape(tokens))
        .or_else(|| did_action_shape(tokens))
}

pub(super) fn ignore_scry_or_surveil(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, amount)) = primitives::parse_prefix(
        trim(tokens),
        alt((
            primitives::kw("scries"),
            primitives::kw("scry"),
            primitives::kw("surveils"),
            primitives::kw("surveil"),
        ))
        .void(),
    ) else {
        return false;
    };
    let Some(parsed) = crate::grammar::leaf::parse_leaf_number_prefix_tokens(amount) else {
        return false;
    };
    trim(amount.get(parsed.consumed..).unwrap_or_default()).is_empty()
}

pub fn parse_opponent_special_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<OpponentSpecialShape<'_>>, CardTextError> {
    if ignore_scry_or_surveil(tokens) {
        return Ok(Some(OpponentSpecialShape::IgnoreScryOrSurveil));
    }
    if let Some(target_tokens) = choose_return_unless(tokens) {
        return Ok(Some(OpponentSpecialShape::ChooseReturnUnlessDraw {
            target_tokens,
        }));
    }
    if let Some(effect_tokens) = less_life(tokens) {
        return Ok(Some(OpponentSpecialShape::LessLifeThanYou {
            effect_tokens,
        }));
    }
    if let Some((count, effect_tokens)) = poison_counters(tokens)? {
        return Ok(Some(OpponentSpecialShape::PoisonCounters {
            count,
            effect_tokens,
        }));
    }
    Ok(None)
}
