//! Readings shard 4 of 4, in rank order.

use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::StackActionAst;
use super::super::*;
use super::Clause;

pub(super) fn read_restriction_duration_cant(
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
        has_restriction_duration,
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
    if has_restriction_duration
        && find_negation_span(&restriction_clause_tokens).is_some()
        && let Some(restrictions) = parse_cant_restrictions(&restriction_clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && parsed.target.is_none()
    {
        return Ok(Some(
            EffectAst::subject_verb_cant_starting_with_duration_surface(
                parsed.restriction.clone(),
                restriction_duration,
                crate::effect::RestrictionStart::Immediate,
                restriction_duration_surface,
                None,
            ),
        ));
    }
    Ok(None)
}
pub(super) fn read_hexproof_targeting_override(
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
    if let Some(effect) = parse_hexproof_targeting_override_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_cast_target_without_paying(
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
    if let Some(shape) = clause_grammar::parse_cast_target_without_paying_shape(tokens) {
        let _ = parse_target_phrase(shape.target_tokens)?;
        return Ok(Some(EffectAst::SubjectVerb(
            crate::model::ast::SubjectVerbEffectAst {
                subject: crate::model::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::Implicit,
                },
                action: SubjectVerbActionAst::Stack(StackActionAst::CastTagged {
                    tag: crate::tag::CompilerReferenceTag::It.bind(),
                    player: PlayerAst::Implicit,
                    allow_land: false,
                    as_copy: false,
                    copy_cast_reminder_surface: false,
                    copy_instruction_surface: None,
                    without_paying_mana_cost: true,
                    additional_mana_cost: None,
                    cost_reduction: None,
                    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                }),
            },
        )));
    }
    Ok(None)
}
pub(super) fn read_passive_goad(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
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
    if let Some(effect) = parse_passive_goad_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_control_player(input: &Clause<'_>) -> Result<Option<EffectAst>, CardTextError> {
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
    if let Some(effect) = parse_control_player_clause(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
pub(super) fn read_trailing_if_fallback(
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
    // Generic "X if <predicate>" fallback: clauses like "play the exiled card
    // without paying its mana cost if you attacked with three or more
    // creatures this turn" have no known leading verb, but the head parses on
    // its own and the tail is a recognizable predicate. Only attempted where
    // the clause would otherwise be a hard no-verb error.
    if clause_grammar::parse_clause_subject_verb_shape(tokens).is_none()
        && let Some(shape) = clause_grammar::parse_trailing_if_fallback_shape(tokens)
        && let Ok(head_effects) =
            super::super::super::super::parse_effect_sentence_lexed(shape.head_tokens)
        && !head_effects.is_empty()
    {
        parser_trace("parse_effect_clause:trailing-if-fallback", tokens);
        return Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: shape.predicate,
            if_true: head_effects,
            if_false: Vec::new(),
        })));
    }
    Ok(None)
}
