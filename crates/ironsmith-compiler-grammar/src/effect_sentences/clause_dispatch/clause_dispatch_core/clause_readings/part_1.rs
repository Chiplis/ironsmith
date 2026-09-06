//! Readings shard 1 of 4, in rank order.

use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ZoneMoveActionAst;
use super::super::*;
use super::Clause;

pub(super) fn read_any_player_or_opponent_may(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(player) =
        super::super::super::super::chain_carry::parse_leading_player_may_lexed(tokens)
        && matches!(player, PlayerAst::Any | PlayerAst::Opponent)
    {
        let stripped = super::super::super::super::chain_carry::remove_through_first_word(tokens);
        let stripped = crate::util::trim_edge_punctuation_tokens(&stripped);
        if stripped.first().is_some_and(|token| token.is_word("pay")) {
            let payment = super::super::super::super::zone_handlers::parse_pay(
                crate::util::trim_edge_punctuation_tokens(&stripped[1..]),
                Some(SubjectAst::Player(PlayerAst::That)),
            )?;
            return Ok(Some(EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay {
                players: if player == PlayerAst::Opponent {
                    PlayerFilter::Opponent
                } else {
                    PlayerFilter::Any
                },
                effects: vec![payment],
            })));
        }
    }
    Ok(None)
}
pub(super) fn read_any_player_may_sacrifice(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    // A standalone effect sentence reaches clause dispatch directly, without
    // passing through the coordinated-chain parser. Preserve the dedicated
    // sequential-offer model for "any player/opponent may sacrifice ..." here
    // too: a broad player filter is not itself an actor and must not become
    // the chooser or sacrificing player for a single MayEffect.
    if let Some(shape) = effect_grammar::parse_any_player_may_sacrifice_shape(tokens) {
        let sacrifice = parse_sacrifice(
            shape.action_tokens,
            Some(SubjectAst::Player(PlayerAst::That)),
            None,
        )?;
        return Ok(Some(EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay {
            players: shape.players,
            effects: vec![sacrifice],
        })));
    }
    Ok(None)
}
pub(super) fn read_assigns_no_combat_damage_then_coordinated(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    // `assigns no combat damage` is a complete effect even when Oracle
    // coordinates another effect after it. The direct shape intentionally
    // requires a sentence boundary, so split this prefix before dispatching
    // the rest of the coordinated clause.
    for (and_idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let prefix = trim_edge_punctuation(&tokens[..and_idx]);
        let suffix = trim_edge_punctuation(&tokens[and_idx + 1..]);
        if suffix.is_empty()
            || !matches!(
                clause_grammar::parse_assigns_no_combat_damage_shape(&prefix),
                Some(clause_grammar::AssignsNoCombatDamageShape::Supported { .. })
            )
        {
            continue;
        }
        let first = parse_effect_clause(&prefix)?;
        let mut effects = vec![first];
        effects.extend(crate::effect_sentences::parse_effect_chain_lexed(&suffix)?);
        if effects.len() > 1 {
            return Ok(Some(EffectAst::Sequence { effects }));
        }
    }
    Ok(None)
}
pub(super) fn read_conditional_become_pair(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(effect) = parse_conditional_become_pair(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_counter_linked_land_subtype_followup(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(shape) = followup_grammar::parse_counter_linked_land_subtype_followup(tokens) {
        return Ok(Some(EffectAst::subject_verb_add_subtypes(
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_tokens(tokens),
            ),
            vec![shape.subtype],
            Until::ForAsLongAs(
                ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                    shape.counter_type,
                ),
            ),
        )));
    }
    Ok(None)
}
pub(super) fn read_prevent_damage_sentence(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(effect) = effect_grammar::parse_prevent_damage_sentence_lexed(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_heal_damage(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(effect) = parse_heal_damage_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_conditional_return_then_turn_face_up(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    // A return may itself be conditional and then immediately turn the
    // returned object face up:
    //
    // "return it ... face down if it's a permanent card, then turn it face
    // up."
    //
    // The ordinary trailing-if splitter intentionally accepts a broad
    // predicate tail, so without this structural split the final turn action
    // is swallowed into the predicate (and even makes "face up" look like a
    // characteristic of "permanent card").  Recognize the typed return/turn
    // pair before the general trailing-if route.
    for split in 0..tokens.len().saturating_sub(1) {
        if !tokens[split].is_comma() || !tokens[split + 1].is_word("then") {
            continue;
        }
        let prefix = trim_edge_punctuation(&tokens[..split]);
        let suffix = trim_edge_punctuation(&tokens[split + 2..]);
        let Some(trailing_if) = split_trailing_if_clause_lexed(&prefix) else {
            continue;
        };
        let Some(return_tokens) = trailing_if
            .leading_tokens
            .first()
            .is_some_and(|token| token.is_word("return"))
            .then_some(&trailing_if.leading_tokens[1..])
        else {
            continue;
        };
        let Ok(return_effect) = parse_return(return_tokens) else {
            continue;
        };
        let Some(turn_shape) = clause_grammar::parse_direct_clause_shape(&suffix) else {
            continue;
        };
        let turn_effect = lower_direct_clause_shape(turn_shape, &suffix);
        let returns_face_down = matches!(
            &return_effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    zone: Zone::Battlefield,
                    battlefield_face_down: true,
                    ..
                }),
                ..
            })
        );
        let turns_face_up = matches!(
            &turn_effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp { .. }),
                ..
            })
        );
        if returns_face_down && turns_face_up {
            return Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                predicate: trailing_if.predicate,
                effects: vec![return_effect, turn_effect],
            })));
        }
    }
    Ok(None)
}
pub(super) fn read_anaphoric_destroy_battlefield_guard(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if effect_grammar::control_flow::is_anaphoric_destroy_battlefield_guard(tokens)
        && tokens.first().is_some_and(|token| token.is_word("destroy"))
    {
        return crate::effect_sentences::parse_destroy(&tokens[1..]).map(Some);
    }
    Ok(None)
}
pub(super) fn read_trailing_if_clause(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens)
        && let Ok(base_effect) = parse_effect_clause(trailing_if.leading_tokens)
    {
        return Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: trailing_if.predicate,
            effects: vec![base_effect],
        })));
    }
    Ok(None)
}
pub(super) fn read_may_cast_it(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(spec) = parse_may_cast_it_sentence(tokens) {
        return Ok(Some(build_may_cast_tagged_effect(&spec)));
    }
    Ok(None)
}
pub(super) fn read_play_exiled_cards_for_as_long_as_exiled(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(effect) = parse_play_exiled_cards_for_as_long_as_exiled_clause(tokens) {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_cast_target_from_your_graveyard_this_turn(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(shape) =
        clause_grammar::parse_cast_target_from_your_graveyard_this_turn_shape(tokens)
    {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    PlayerAst::You,
                    false,
                    false,
                    false,
                ),
            ],
        }));
    }
    Ok(None)
}
pub(super) fn read_cast_or_play_tagged(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(effect) = parse_cast_or_play_tagged_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_cast_any_number_from_among_tagged(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(effect) = parse_cast_any_number_from_among_tagged_clause(tokens) {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_cast_single_spell_from_among_hand_cards(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(effect) = parse_cast_single_spell_from_among_hand_cards_clause(tokens) {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_mana_any_type_cast_tagged_this_way(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        _restriction_duration,
        _restriction_clause_tokens,
        _restriction_duration_surface,
        _has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };
    if let Some(effect) = parse_mana_any_type_cast_tagged_this_way_clause(tokens) {
        return Ok(Some(effect));
    }
    Ok(None)
}
