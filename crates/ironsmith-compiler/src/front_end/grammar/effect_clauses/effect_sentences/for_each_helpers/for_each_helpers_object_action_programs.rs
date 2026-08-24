use super::*;

pub(super) fn parse_relative_control_conditional(
    relative: RelativeControlClauseShape<'_>,
    participant_is_actor: bool,
    clause_text: &str,
) -> Result<EffectAst, CardTextError> {
    let mut filter = parse_object_filter(relative.filter_tokens, false)?;
    let mut branch_effects;
    let participant_where_x = parse_participant_body_where_x_value(relative.effect_tokens);
    let participant_choice_effects =
        parse_participant_choice_complement_effects(relative.effect_tokens)?;
    let predicate = if let Some(most_filter_tokens) = relative.fewer_than_most_filter_tokens {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        let mut most_filter = parse_object_filter(most_filter_tokens, false)?;
        most_filter.controller = Some(PlayerFilter::Any);
        let difference = Value::Add(
            Box::new(Value::GreatestCount(most_filter.clone())),
            Box::new(Value::Scaled(Box::new(Value::Count(filter.clone())), -1)),
        )
        .with_surface_hint(ValueSurfaceHint::Difference);

        let rewritten = rewrite_difference_bounded_search(relative.effect_tokens);
        branch_effects = if let Some(effects) = participant_choice_effects.clone() {
            effects
        } else {
            parse_maybe_effects(
                rewritten.as_deref().unwrap_or(relative.effect_tokens),
                true,
                false,
            )?
        };
        if rewritten.is_some() {
            replace_unbound_x_in_effects_anywhere(&mut branch_effects, &difference, clause_text)?;
        }
        PredicateAst::ValueComparison {
            left: Value::Count(filter),
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: Value::GreatestCount(most_filter),
        }
    } else if relative.fewer_than_you {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        let mut your_filter = filter.clone();
        your_filter.controller = Some(PlayerFilter::You);
        branch_effects = if let Some(effects) = participant_choice_effects.clone() {
            effects
        } else {
            parse_maybe_effects(relative.effect_tokens, true, false)?
        };
        PredicateAst::ValueComparison {
            left: Value::Count(filter),
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: Value::Count(your_filter),
        }
    } else if let Some(comparison) = relative.count_comparison {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        branch_effects = if let Some(effects) = participant_choice_effects.clone() {
            effects
        } else {
            parse_maybe_effects(relative.effect_tokens, true, false)?
        };
        let (operator, count) =
            comparison_to_value_comparison_operator(comparison).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported for-each control count comparison (clause: '{clause_text}')"
                ))
            })?;
        PredicateAst::ValueComparison {
            left: Value::Count(filter),
            operator,
            right: Value::Fixed(count),
        }
    } else if relative.controls_most {
        branch_effects = if let Some(effects) = participant_choice_effects.clone() {
            effects
        } else {
            parse_maybe_effects(relative.effect_tokens, true, false)?
        };
        PredicateAst::PlayerControlsMost {
            player: PlayerAst::That,
            filter,
        }
    } else {
        branch_effects = if let Some(effects) = participant_choice_effects {
            effects
        } else {
            parse_maybe_effects(relative.effect_tokens, true, false)?
        };
        PredicateAst::PlayerControls {
            player: PlayerAst::That,
            filter,
        }
    };
    if let Some(where_x) = participant_where_x {
        replace_unbound_x_in_effects_anywhere(&mut branch_effects, &where_x, clause_text)?;
    }
    if participant_is_actor {
        for effect in &mut branch_effects {
            bind_implicit_player_context(effect, PlayerAst::That);
        }
    }
    Ok(EffectAst::Conditional {
        predicate,
        if_true: branch_effects,
        if_false: Vec::new(),
    })
}
