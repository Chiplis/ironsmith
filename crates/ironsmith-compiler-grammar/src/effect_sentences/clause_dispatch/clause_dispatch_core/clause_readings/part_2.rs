//! Readings shard 2 of 4, in rank order.

use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::LifeResourceActionAst;
use super::super::*;
use super::Clause;

pub(super) fn read_leading_may_additional_land_plays(
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
    // In permission text such as "You may play an additional land this
    // turn", "may" describes the granted game-rule permission. It is not
    // an optional resolution action and therefore must not become a
    // MayEffect decision at resolution time.
    if let Some(shape) = clause_grammar::parse_leading_may_shape(tokens) {
        if let Some(mut permission) = parse_additional_land_plays_clause(shape.effect_tokens)? {
            if let clause_grammar::LeadingMayActorShape::Player(player) = shape.actor {
                bind_implicit_player_context(&mut permission, player);
            }
            return Ok(Some(permission));
        }
        let mut effects = parse_effect_chain_with_subject_verb_primitives(shape.effect_tokens)?;
        return Ok(Some(match shape.actor {
            clause_grammar::LeadingMayActorShape::Player(player) => {
                for effect in &mut effects {
                    bind_implicit_player_context(effect, player);
                }
                EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player, effects })
            }
            clause_grammar::LeadingMayActorShape::Implicit => EffectAst::Permissions(PermissionEffectAst::May { effects }),
        }));
    }
    Ok(None)
}
pub(super) fn read_tagged_plural_pump(
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
    if let Some(shape) = clause_grammar::parse_tagged_plural_pump_shape(tokens)
        && let Some(effect) =
            parse_get_pump_clause(shape.subject_tokens, shape.modifier_tokens, tokens)?
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_for_each_prevent_damage(
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
    if let Some(effect) = parse_for_each_prevent_damage_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_for_each_counter_group_removed_this_way(
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
    if let Some(effect) = parse_for_each_counter_group_removed_this_way_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_turn_target_face_up(
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
    if let Some(shape) = clause_grammar::parse_turn_target_face_up_shape(tokens) {
        return Ok(Some(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp {
                target: parse_target_phrase(shape.target_tokens)?,
            }),
        )));
    }
    Ok(None)
}
pub(super) fn read_direct_clause_shape(
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
    if let Some(shape) = clause_grammar::parse_direct_clause_shape(tokens) {
        return Ok(Some(lower_direct_clause_shape(shape, tokens)));
    }
    Ok(None)
}
pub(super) fn read_shared_ability_gain(
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
    if let Some(shape) = clause_grammar::parse_shared_ability_gain_shape(tokens) {
        return Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                Some(crate::cards::builders::TextSpan::synthetic()),
            ),
            shape
                .abilities
                .into_iter()
                .map(GrantedAbilityAst::from)
                .collect(),
            Until::Forever,
        )));
    }
    Ok(None)
}
pub(super) fn read_take_extra_turn(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
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
    if let Some(effect) = parse_take_extra_turn_sentence(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_additional_phase(
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
    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_mana_replacement_clause(
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
    if let Some(spec) = parse_mana_replacement_clause_spec_lexed(tokens) {
        return Ok(Some(EffectAst::subject_verb_register_mana_replacement(
            ObjectFilter::land().you_control(),
            vec![spec.replacement_mana],
            crate::effects::ReplacementApplyMode::UntilEndOfTurn,
        )));
    }
    Ok(None)
}
pub(super) fn read_for_each_card_payment(
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
    if let Some(shape) = clause_grammar::parse_for_each_card_payment_shape(tokens) {
        let mut filter = ObjectFilter::default();
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter,
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        span_from_tokens(tokens),
                    ),
                    crate::zone::Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                alternative: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                        amount: Value::Fixed(shape.life_amount as i32),
                    }),
                )],
                player: PlayerAst::You,
            })],
        })));
    }
    Ok(None)
}
pub(super) fn read_opponent_return_choice(
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
    if let Some(shape) = clause_grammar::parse_opponent_return_choice_shape(tokens) {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
                    effects: vec![EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                        false,
                    )],
                    alternative: vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                            count: Value::Fixed(1),
                        }),
                    )],
                    player: PlayerAst::ItsController,
                }),
            ],
        })));
    }
    Ok(None)
}
pub(super) fn read_delayed_next_step_unless_pays(
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
    if let Some(effects) =
        parse_sentence_delayed_next_step_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(Some(match effects.as_slice() {
            [effect] => effect.clone(),
            _ => EffectAst::Sequence { effects },
        }));
    }
    Ok(None)
}
pub(super) fn read_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(
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
    if let Some(effect) =
        parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(tokens)
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_clause_primitives(
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
    if let Some(effect) = run_clause_primitives(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_unless_clause(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let clause = SubjectVerbPrimitiveClause::new(tokens);
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
    if let Some(unless_idx) = find_unquoted_token_word(clause, "unless") {
        let main_tokens = trim_commas(&tokens[..unless_idx]);
        if !main_tokens.is_empty()
            && let Ok(main_effect) = parse_effect_clause(&main_tokens)
            && let Some(unless_effect) = try_build_unless(vec![main_effect], clause, unless_idx)?
        {
            return Ok(Some(unless_effect));
        }
    }
    Ok(None)
}
pub(super) fn read_has_base_power(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
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
    if let Some(effect) = parse_has_base_power_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_has_base_power_toughness(
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
    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_passive_sacrifice_by_controller(
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
    if let Some(effect) = parse_passive_sacrifice_by_controller_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_copular_base_pt_animation(
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
    if let Some(effect) = parse_copular_base_pt_animation_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
