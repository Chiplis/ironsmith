use super::*;

pub(crate) fn condition_for_chosen_option(context: &ChosenOptionContext) -> crate::ConditionExpr {
    match context {
        ChosenOptionContext::SourceOption(label) => {
            crate::ConditionExpr::SourceChosenOption(label.clone())
        }
        ChosenOptionContext::MaxSpeed => crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::Speed(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(4),
        },
        ChosenOptionContext::StationThreshold(threshold) => crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::CountersOnSource(crate::CounterType::Charge),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(*threshold),
        },
        ChosenOptionContext::ControlsSubtypePermanent(subtype) => {
            let filter = ObjectFilter::permanent()
                .you_control()
                .with_subtype(*subtype);
            crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(filter),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some(format!("you control a {subtype}")),
            }
        }
        ChosenOptionContext::ControlsEitherColorPermanent { left, right } => {
            let left_name = left.name();
            let right_name = right.name();
            let left_filter = ObjectFilter::permanent()
                .you_control()
                .with_colors(ColorSet::from_color(*left));
            let right_filter = ObjectFilter::permanent()
                .you_control()
                .with_colors(ColorSet::from_color(*right));
            let left_condition = crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(left_filter),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some(format!("you control a {left_name} permanent")),
            };
            let right_condition = crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(right_filter),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some(format!("you control a {right_name} permanent")),
            };
            crate::ConditionExpr::Or(Box::new(left_condition), Box::new(right_condition))
        }
    }
}

pub(crate) fn wrap_chosen_option_static_chunk(
    chunk: LineAst,
    chosen_option: Option<&ChosenOptionContext>,
) -> Result<LineAst, CardTextError> {
    let Some(context) = chosen_option else {
        return Ok(chunk);
    };
    let condition = condition_for_chosen_option(context);
    let wrap_static_ast = |ability| match context {
        ChosenOptionContext::MaxSpeed => {
            crate::cards::builders::StaticAbilityAst::LabeledConditionalStaticAbility {
                ability: Box::new(ability),
                condition: condition.clone(),
                label: "Max speed".to_string(),
            }
        }
        _ => crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(ability),
            condition: condition.clone(),
        },
    };
    Ok(match chunk {
        LineAst::Multiple(chunks) => LineAst::Multiple(
            chunks
                .into_iter()
                .map(|chunk| wrap_chosen_option_static_chunk(chunk, chosen_option))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        LineAst::StaticAbility(ability) => LineAst::StaticAbility(wrap_static_ast(ability)),
        LineAst::StaticAbilities(abilities) => {
            LineAst::StaticAbilities(abilities.into_iter().map(wrap_static_ast).collect())
        }
        LineAst::Abilities(actions) => LineAst::StaticAbilities(
            actions
                .into_iter()
                .map(|action| match context {
                    ChosenOptionContext::MaxSpeed => wrap_static_ast(
                        crate::cards::builders::StaticAbilityAst::KeywordAction(action),
                    ),
                    _ => crate::cards::builders::StaticAbilityAst::ConditionalKeywordAction {
                        action,
                        condition: condition.clone(),
                    },
                })
                .collect(),
        ),
        LineAst::Ability(mut parsed) => {
            if let AbilityKind::Static(static_ability) = parsed.kind_mut() {
                *static_ability = match context {
                    ChosenOptionContext::MaxSpeed => static_ability
                        .clone()
                        .with_labeled_condition(condition.clone(), "Max speed"),
                    _ => static_ability
                        .clone()
                        .with_condition(condition.clone())
                        .unwrap_or_else(|| {
                            crate::static_abilities::StaticAbility::new(
                                crate::static_abilities::GrantAbility::source(
                                    static_ability.clone(),
                                )
                                .with_condition(condition.clone()),
                            )
                        }),
                };
            }
            LineAst::Ability(parsed)
        }
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chosen_option_conditions_consume_typed_contexts() {
        assert!(matches!(
            condition_for_chosen_option(&ChosenOptionContext::source_option("khans")),
            crate::ConditionExpr::SourceChosenOption(option) if option == "khans"
        ));
        assert!(matches!(
            condition_for_chosen_option(&ChosenOptionContext::StationThreshold(5)),
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::CountersOnSource(crate::CounterType::Charge),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(5),
            }
        ));
        assert!(matches!(
            condition_for_chosen_option(&ChosenOptionContext::MaxSpeed),
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::Speed(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(4),
            }
        ));
    }
}
