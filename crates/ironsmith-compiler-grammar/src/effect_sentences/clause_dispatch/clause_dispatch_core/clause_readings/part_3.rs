//! Readings shard 3 of 4, in rank order.

use super::super::*;
use super::Clause;

pub(super) fn read_participant_choice_then_return_chosen_set(
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
    if let Some(effect) = parse_participant_choice_then_return_chosen_set(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_choose_color(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let choice_head = parse_choice_clause_head_tokens(tokens);
    let choice_actor = choice_head
        .as_ref()
        .map_or(ChoiceClauseActor::Implicit, |head| head.actor);
    let choice_tokens = choice_head.as_ref().map_or_else(
        || clause_grammar::strip_optional_you_choice_tokens(tokens),
        |head| head.choice_tokens,
    );
    let choice_player = match choice_actor {
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();
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
    if let Some((consumed, excluded_color)) = parse_choose_color_phrase_words(&choice_words)?
        && consumed == choice_words.len()
        && excluded_color.is_none()
    {
        return Ok(Some(EffectAst::subject_verb_choose_color(choice_player)));
    }
    Ok(None)
}
pub(super) fn read_choose_creature_type(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let choice_head = parse_choice_clause_head_tokens(tokens);
    let choice_actor = choice_head
        .as_ref()
        .map_or(ChoiceClauseActor::Implicit, |head| head.actor);
    let choice_tokens = choice_head.as_ref().map_or_else(
        || clause_grammar::strip_optional_you_choice_tokens(tokens),
        |head| head.choice_tokens,
    );
    let choice_player = match choice_actor {
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();
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
    if let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(Some(EffectAst::subject_verb_choose_creature_type(
            choice_player,
            excluded_subtypes,
        )));
    }
    Ok(None)
}
pub(super) fn read_choose_land_type(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let choice_head = parse_choice_clause_head_tokens(tokens);
    let choice_actor = choice_head
        .as_ref()
        .map_or(ChoiceClauseActor::Implicit, |head| head.actor);
    let choice_tokens = choice_head.as_ref().map_or_else(
        || clause_grammar::strip_optional_you_choice_tokens(tokens),
        |head| head.choice_tokens,
    );
    let choice_player = match choice_actor {
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();
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
    if let Some(parsed) = parse_choice_land_type_phrase_words(&choice_words)
        && parsed.consumed == choice_words.len()
    {
        return Ok(Some(EffectAst::subject_verb_choose_land_type(
            choice_player,
            parsed.exclude_basic,
        )));
    }
    Ok(None)
}
pub(super) fn read_choose_subtype_family(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let choice_head = parse_choice_clause_head_tokens(tokens);
    let choice_actor = choice_head
        .as_ref()
        .map_or(ChoiceClauseActor::Implicit, |head| head.actor);
    let choice_tokens = choice_head.as_ref().map_or_else(
        || clause_grammar::strip_optional_you_choice_tokens(tokens),
        |head| head.choice_tokens,
    );
    let choice_player = match choice_actor {
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();
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
    if let Some(parsed) = parse_choice_subtype_family_phrase_words(&choice_words)
        && parsed.consumed == choice_words.len()
    {
        return Ok(Some(EffectAst::subject_verb_choose_subtype_type(
            choice_player,
            parsed.family,
        )));
    }
    Ok(None)
}
pub(super) fn read_choose_card_type(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let choice_head = parse_choice_clause_head_tokens(tokens);
    let choice_actor = choice_head
        .as_ref()
        .map_or(ChoiceClauseActor::Implicit, |head| head.actor);
    let choice_tokens = choice_head.as_ref().map_or_else(
        || clause_grammar::strip_optional_you_choice_tokens(tokens),
        |head| head.choice_tokens,
    );
    let choice_player = match choice_actor {
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();
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
    if let Some((consumed, options)) = parse_choose_card_type_phrase_words(&choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(Some(EffectAst::subject_verb_choose_card_type(
            choice_player,
            options,
        )));
    }
    Ok(None)
}
pub(super) fn read_choose_player(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let choice_head = parse_choice_clause_head_tokens(tokens);
    let choice_actor = choice_head
        .as_ref()
        .map_or(ChoiceClauseActor::Implicit, |head| head.actor);
    let choice_tokens = choice_head.as_ref().map_or_else(
        || clause_grammar::strip_optional_you_choice_tokens(tokens),
        |head| head.choice_tokens,
    );
    let choice_player = match choice_actor {
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();
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
    if let Some(consumed) = parse_choose_player_phrase_words(&choice_words)
        && consumed == choice_words.len()
    {
        return Ok(Some(EffectAst::subject_verb_choose_player(
            choice_player,
            PlayerFilter::Any,
            crate::tag::CompilerReferenceTag::It.bind(),
            false,
            0,
        )));
    }
    Ok(None)
}
pub(super) fn read_ordered_choose_all(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
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
    if let Some(shape) = clause_grammar::parse_ordered_choose_all_shape(tokens) {
        let filter = parse_object_filter(shape.filter_tokens, false)?;
        let repeated_filter = parse_object_filter(shape.repeated_filter_tokens, false)?;
        if filter != repeated_filter {
            return Err(CardTextError::ParseError(format!(
                "ordered choice stopping filter differs from chosen filter (clause: '{}')",
                clause_words.join(" ")
            )))
            .map(Some);
        }
        return Ok(Some(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: filter.clone(),
            count: ChoiceCount::dynamic_x(),
            count_value: Some(
                Value::Count(filter).with_surface_hint(ValueSurfaceHint::ChooseAllInOrder),
            ),
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        })));
    }
    Ok(None)
}
pub(super) fn read_choose_target(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
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
    if let Some(shape) = clause_grammar::parse_choose_target_shape(tokens)
        && let Ok(mut target) = parse_target_phrase(shape.target_tokens)
    {
        if shape.excludes_chooser_controller {
            preserve_target_choice_controller_exclusion(&mut target, shape.chooser);
        }
        let player_target = match &target {
            TargetAst::Player(_, _) => true,
            TargetAst::WithCount(inner, _) => matches!(inner.as_ref(), TargetAst::Player(_, _)),
            _ => false,
        };
        if player_target
            || clause_grammar::parse_clause_subject_verb_shape(shape.target_tokens).is_none()
        {
            return Ok(Some(explicit_target_choice(shape, target)));
        }
    }
    Ok(None)
}
pub(super) fn read_you_choose_player(
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
    if let Some((chooser, choose_filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(tokens)?
    {
        return Ok(Some(EffectAst::subject_verb_choose_player(
            chooser,
            choose_filter,
            crate::tag::CompilerReferenceTag::It.bind(),
            random,
            exclude_previous_choices,
        )));
    }
    Ok(None)
}
pub(super) fn read_target_player_choose_objects_with_count(
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
    if let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_target_player_choose_objects_clause_with_count_value(tokens)?
    {
        return Ok(Some(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        })));
    }
    Ok(None)
}
pub(super) fn read_you_choose_objects_with_count(
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
    if let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_you_choose_objects_clause_with_count_value(tokens)?
    {
        return Ok(Some(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        })));
    }
    Ok(None)
}
pub(super) fn read_assigns_no_combat_damage(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
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
    if let Some(shape) = clause_grammar::parse_assigns_no_combat_damage_shape(tokens) {
        match shape {
            clause_grammar::AssignsNoCombatDamageShape::Unsupported => {
                return Err(CardTextError::ParseError(format!(
                        "unsupported assigns-no-combat-damage clause tail (clause: '{}') [rule=assigns-no-combat-damage-tail]",
                        clause_words.join(" ")
                    ))).map(Some);
            }
            clause_grammar::AssignsNoCombatDamageShape::Supported { source, duration } => {
                let source = match source {
                    clause_grammar::AssignDamageSourceShape::Source => TargetAst::Source(None),
                    clause_grammar::AssignDamageSourceShape::Tagged => TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        span_from_tokens(tokens),
                    ),
                    clause_grammar::AssignDamageSourceShape::Target(target_tokens) => {
                        parse_target_phrase(target_tokens)?
                    }
                };
                return Ok(Some(EffectAst::subject_verb_assign_no_combat_damage(
                    source, duration,
                )));
            }
        }
    }
    Ok(None)
}
pub(super) fn read_targeted_negated_restriction(
    input: &Clause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        restriction_duration,
        restriction_clause_tokens,
        restriction_duration_surface,
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
    if starts_with_target_indicator(&restriction_clause_tokens)
        && find_negation_span(&restriction_clause_tokens).is_some_and(|(neg_start, _)| {
            find_verb(&restriction_clause_tokens[..neg_start]).is_none()
        })
        && let Some(restrictions) = parse_cant_restrictions(&restriction_clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && let Some(target) = parsed.target.clone()
    {
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_cant_starting_with_duration_surface(
                    parsed.restriction.clone(),
                    restriction_duration,
                    crate::effect::RestrictionStart::Immediate,
                    restriction_duration_surface,
                    None,
                ),
            ],
        }));
    }
    Ok(None)
}
pub(super) fn read_target_only(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
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
    if let Some(shape) = clause_grammar::parse_target_only_shape(tokens) {
        if find_negation_span(tokens).is_some() || shape.restriction_like {
            return Err(CardTextError::ParseError(format!(
                    "unsupported target-only restriction clause (clause: '{}') [rule=target-only-restriction]",
                    clause_words.join(" ")
                ))).map(Some);
        }
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(Some(EffectAst::subject_verb_target_only(target)));
    }
    Ok(None)
}
pub(super) fn read_embedded_choose_target(
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
    if let Some(shape) = clause_grammar::parse_embedded_choose_target_shape(tokens) {
        let mut target = parse_target_phrase(shape.target_tokens)?;
        if shape.excludes_chooser_controller {
            preserve_target_choice_controller_exclusion(&mut target, shape.chooser);
        }
        return Ok(Some(explicit_target_choice(shape, target)));
    }
    Ok(None)
}
pub(super) fn read_next_turn_cant(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
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
    if let Some(effect) = parse_next_turn_cant_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
