use super::*;

pub fn condition_for_chosen_option(context: &ChosenOptionContext) -> crate::ConditionExpr {
    match context {
        ChosenOptionContext::SourceOption(label) => {
            crate::ConditionExpr::SourceChosenOption(label.clone())
        }
        ChosenOptionContext::MaxSpeed => crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::Speed(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(4),
        },
        ChosenOptionContext::StationThreshold(threshold)
        | ChosenOptionContext::StationThresholdSupport(threshold) => {
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::CountersOnSource(crate::CounterType::Charge),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(*threshold),
            }
        }
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

pub fn wrap_chosen_option_static_chunk(
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
        ChosenOptionContext::StationThreshold(threshold) => {
            crate::cards::builders::StaticAbilityAst::LabeledConditionalStaticAbility {
                ability: Box::new(ability),
                condition: condition.clone(),
                label: format!(
                    "{}{threshold}",
                    ironsmith_core::static_ability_model::STATION_THRESHOLD_STATIC_LABEL_PREFIX
                ),
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
                    ChosenOptionContext::MaxSpeed | ChosenOptionContext::StationThreshold(_) => {
                        wrap_static_ast(crate::cards::builders::StaticAbilityAst::KeywordAction(
                            action,
                        ))
                    }
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
                    ChosenOptionContext::StationThreshold(threshold) => static_ability
                        .clone()
                        .with_labeled_condition(
                            condition.clone(),
                            format!(
                                "{}{threshold}",
                                ironsmith_core::static_ability_model::STATION_THRESHOLD_STATIC_LABEL_PREFIX
                            ),
                        ),
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

    #[test]
    fn station_static_chunk_keeps_typed_threshold_presentation_provenance() {
        let wrapped = wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(crate::cards::builders::StaticAbilityAst::KeywordAction(
                crate::cards::builders::KeywordAction::Flying,
            )),
            Some(&ChosenOptionContext::StationThreshold(3)),
        )
        .expect("station static chunk should wrap");

        let LineAst::StaticAbility(
            crate::cards::builders::StaticAbilityAst::LabeledConditionalStaticAbility {
                ability,
                condition,
                label,
            },
        ) = wrapped
        else {
            panic!("station row must retain a typed labeled condition: {wrapped:#?}");
        };
        assert!(matches!(
            ability.as_ref(),
            crate::cards::builders::StaticAbilityAst::KeywordAction(
                crate::cards::builders::KeywordAction::Flying
            )
        ));
        assert_eq!(
            label,
            format!(
                "{}3",
                ironsmith_core::static_ability_model::STATION_THRESHOLD_STATIC_LABEL_PREFIX
            )
        );
        assert!(matches!(
            condition,
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::CountersOnSource(crate::CounterType::Charge),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(3),
            }
        ));
    }

    #[test]
    fn station_implicit_creature_support_keeps_condition_without_row_marker() {
        let wrapped = wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(crate::cards::builders::StaticAbilityAst::KeywordAction(
                crate::cards::builders::KeywordAction::Flying,
            )),
            Some(&ChosenOptionContext::StationThresholdSupport(8)),
        )
        .expect("station support should wrap");

        let LineAst::StaticAbility(
            crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
                ability,
                condition,
            },
        ) = wrapped
        else {
            panic!("implicit support must not claim the authored row marker: {wrapped:#?}");
        };
        assert!(matches!(
            ability.as_ref(),
            crate::cards::builders::StaticAbilityAst::KeywordAction(
                crate::cards::builders::KeywordAction::Flying
            )
        ));
        assert!(matches!(
            condition,
            crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::CountersOnSource(crate::CounterType::Charge),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(8),
            }
        ));
    }
}
