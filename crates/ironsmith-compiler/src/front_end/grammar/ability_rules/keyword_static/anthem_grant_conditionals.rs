pub fn parse_conditional_anthem_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_conditional_anthem_replacement(tokens) else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let condition = crate::ConditionExpr::AttachedToSourceMatches(shape.condition_filter);
    let base = fixed_anthem_clause(
        subject.clone(),
        shape.base_power,
        shape.base_toughness,
        None,
    );
    let delta = fixed_anthem_clause(
        subject,
        shape.replacement_power - shape.base_power,
        shape.replacement_toughness - shape.base_toughness,
        Some(condition),
    );
    Ok(Some(vec![
        build_anthem_static_ability(&base).into(),
        StaticAbility::new(
            build_anthem(&delta)
                .with_replacement_surface(shape.replacement_power, shape.replacement_toughness),
        )
        .into(),
    ]))
}

pub fn parse_conditional_anthem_otherwise_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_conditional_anthem_otherwise(tokens) else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let condition = crate::ConditionExpr::AttachedToSourceMatches(shape.condition_filter);
    let true_clause = fixed_anthem_clause(
        subject.clone(),
        shape.true_power,
        shape.true_toughness,
        Some(condition.clone()),
    );
    let false_clause = fixed_anthem_clause(
        subject,
        shape.false_power,
        shape.false_toughness,
        Some(crate::ConditionExpr::Not(Box::new(condition))),
    );
    Ok(Some(vec![
        build_anthem_static_ability(&true_clause).into(),
        build_anthem_static_ability(&false_clause).into(),
    ]))
}

pub fn parse_carried_conditional_anthem_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_carried_conditional_anthem_grant(tokens) else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let condition = crate::ConditionExpr::AttachmentCount {
        attachment: shape.condition.attachment_filter,
        host: ironsmith_core::AttachmentConditionHost::Matching(shape.condition.attached_to_filter),
        comparison: shape.condition.comparison,
        display: shape.condition.display,
    };
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(shape.ability_tokens, &clause_words, true)?
    else {
        return Ok(None);
    };
    let base = fixed_anthem_clause(
        subject.clone(),
        shape.base_power,
        shape.base_toughness,
        None,
    );
    let additional = fixed_anthem_clause(
        subject.clone(),
        shape.additional_power,
        shape.additional_toughness,
        Some(condition.clone()),
    );
    let mut result = vec![
        build_anthem_static_ability(&base).into(),
        build_anthem_static_ability(&additional).into(),
    ];
    result.extend(lower_granted_tail_for_anthem_subject(
        &subject,
        &Some(condition),
        granted_tail,
    ));
    Ok(Some(result))
}

pub fn parse_anthem_and_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(line_shape) = anthem_grant_grammar::parse_anthem_keyword_head(tokens) else {
        return Ok(None);
    };
    let get_idx = line_shape.get_token;
    let have_token_idx = line_shape.have_token;

    if line_shape.order == anthem_grant_grammar::AnthemKeywordOrder::KeywordBeforeAnthem {
        let Some(shape) =
            anthem_grant_grammar::parse_keyword_before_anthem_shape(tokens, line_shape)
        else {
            return Ok(None);
        };
        let subject = parse_anthem_subject(shape.subject_tokens)?;
        let mut anthem_tokens = shape.subject_tokens.to_vec();
        anthem_tokens.extend_from_slice(shape.anthem_tail_tokens);
        let Some(anthem) = parse_anthem_line(&anthem_tokens)? else {
            return Ok(None);
        };
        let mut result = Vec::new();
        let grant_clause = ParsedAnthemClause {
            subject,
            power: AnthemValue::Fixed(0),
            toughness: AnthemValue::Fixed(0),
            condition: None,
            count_uses_where_x: false,
            additional_surface: false,
            set_quantifier_surface: None,
        };
        for raw_segment in anthem_grant_grammar::split_trailing_grant_segments(shape.keyword_tokens)
        {
            let Some(segment) = anthem_grant_grammar::parse_trailing_grant_segment(&raw_segment)
            else {
                return Ok(None);
            };
            let segment = trim_edge_punctuation(segment.body_tokens);
            if segment.is_empty() {
                return Ok(None);
            }
            if anthem_grant_grammar::parse_continuing_segment_shape(&segment)
                == anthem_grant_grammar::ContinuingSegmentShape::MustAttack
            {
                result.push(
                    grant_for_anthem_subject(&grant_clause, StaticAbility::must_attack()),
                );
                continue;
            }
            let Some(actions) = parse_ability_line(&segment) else {
                return Ok(None);
            };
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            let lowered = actions
                .into_iter()
                .filter(|action| action.lowers_to_static_ability())
                .collect::<Vec<_>>();
            if lowered.is_empty() {
                return Ok(None);
            }
            for action in lowered {
                result.push(grant_keyword_action_for_anthem_subject(
                    &grant_clause,
                    action,
                ));
            }
        }
        if result.is_empty() {
            return Ok(None);
        }
        // Preserve the semantic source order for keyword-before-anthem lines.
        // Adjacent static-ability rendering can then reconstruct "has ... and
        // gets ..." instead of reversing the two predicates.
        result.push(StaticAbilityAst::from(anthem));
        return Ok(Some(result));
    }

    // "until end of turn" in the pump clause indicates a one-shot effect.
    // Ignore timing text that appears only inside a quoted granted ability.
    if line_shape.pre_grant_is_temporary {
        return Ok(None);
    }

    // The shared anthem/keyword head recognizes both gain and loss verbs.
    // A loss tail changes the affected object's own abilities; it must not be
    // lowered as though the object were granted a nested source-removal rule.
    if tokens
        .get(have_token_idx)
        .is_some_and(|token| token.is_word("lose") || token.is_word("loses"))
    {
        let clause = parse_anthem_clause(tokens, get_idx, have_token_idx)?;
        let loss_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
        let Some(actions) = parse_ability_line(&loss_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        if actions.is_empty()
            || actions
                .iter()
                .any(|action| !action.lowers_to_static_ability())
        {
            return Ok(None);
        }
        let mut result = vec![build_anthem_static_ability(&clause).into()];
        result.extend(
            actions
                .into_iter()
                .map(|action| remove_keyword_action_for_anthem_subject(&clause, action)),
        );
        return Ok(Some(result));
    }

    if let Some(color_segment) =
        anthem_grant_grammar::parse_anthem_keyword_color_segment(tokens, line_shape)
    {
        let clause = parse_anthem_clause(tokens, get_idx, color_segment.is_token)?;
        let filter = anthem_subject_filter(&clause.subject);
        let mut result = vec![build_anthem_static_ability(&clause).into()];
        let color_static = StaticAbility::set_colors(filter, color_segment.color);
        let color_ast: StaticAbilityAst = color_static.into();
        result.push(match &clause.condition {
            Some(condition) => add_static_ability_ast_condition(color_ast, condition.clone())?,
            None => color_ast,
        });

        let ability_tokens_storage = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
        let ability_tokens = trim_outer_quotes(&ability_tokens_storage);
        if anthem_grant_grammar::parse_colon_tail_split(ability_tokens).is_some() {
            let Some(parsed) = parse_activated_line(ability_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            let display = display_text_for_tokens(ability_tokens, false);
            result.push(grant_object_ability_for_anthem_subject(
                &clause, parsed, display,
            ));
            return Ok(Some(result));
        }
    }

    if let Some(compound) =
        anthem_grant_grammar::parse_anthem_keyword_compound_split(tokens, line_shape)
    {
        let first_clause = parse_anthem_clause(tokens, get_idx, compound.split_token)?;
        let mut result = vec![build_anthem_static_ability(&first_clause).into()];

        let grant_clause = if let Some(second_get_idx) = compound.second_get_token {
            let second_tokens = &tokens[compound.tail_start..];
            let second_clause = parse_anthem_clause(
                second_tokens,
                second_get_idx - compound.tail_start,
                compound.second_tail_end - compound.tail_start,
            )?;
            result.push(build_anthem_static_ability(&second_clause).into());
            second_clause
        } else {
            let subject_tokens =
                trim_edge_punctuation(&tokens[compound.tail_start..have_token_idx]);
            if subject_tokens.is_empty() {
                return Ok(None);
            }
            ParsedAnthemClause {
                subject: parse_anthem_subject(&subject_tokens)?,
                power: AnthemValue::Fixed(0),
                toughness: AnthemValue::Fixed(0),
                condition: None,
                count_uses_where_x: false,
                additional_surface: false,
                set_quantifier_surface: None,
            }
        };

        let ability_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
        let Some(actions) = parse_ability_line(&ability_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        for action in actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
        {
            result.push(grant_keyword_action_for_anthem_subject(
                &grant_clause,
                action,
            ));
        }
        return Ok(Some(result));
    }

    let mut ability_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
    let mut trailing_condition: Option<crate::ConditionExpr> = None;
    let mut trailing_if_surface = false;
    match anthem_grant_grammar::split_anthem_keyword_trailing_condition(&ability_tokens) {
        Ok(Some(split)) => {
            trailing_condition = Some(parse_static_condition_clause(split.condition_tokens)?);
            trailing_if_surface = split.trailing_if_surface;
            ability_tokens = split.ability_tokens.to_vec();
        }
        Ok(None) => {}
        Err(anthem_grant_grammar::AnthemKeywordTrailingConditionError::MissingAbility) => {
            return Err(CardTextError::ParseError(format!(
                "missing granted keyword list before trailing condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Err(anthem_grant_grammar::AnthemKeywordTrailingConditionError::MissingCondition) => {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing keyword condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }
    let mut trailing_type_color_addition: Option<TypeColorAdditionClause> = None;
    if let Some(split) = anthem_grant_grammar::split_anthem_keyword_and_is(&ability_tokens)
        && let Some(additions) = parse_type_color_addition_clause(split.tail_tokens)? {
            trailing_type_color_addition = Some(additions);
            ability_tokens = split.head_tokens.to_vec();
        }

    let mut keyword_actions: Vec<KeywordAction> = Vec::new();
    let mut granted_activated_ability: Option<ParsedAbility> = None;
    let mut granted_activated_display: Option<String> = None;

    if let Some(and_has) = anthem_grant_grammar::split_anthem_keyword_and_have(&ability_tokens) {
        let keyword_tokens = and_has.head_tokens.to_vec();
        if !keyword_tokens.is_empty() {
            if let Some(actions) = parse_ability_line(&keyword_tokens) {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                keyword_actions.extend(
                    actions
                        .into_iter()
                        .filter(|action| action.lowers_to_static_ability()),
                );
            } else {
                return Ok(None);
            }
        }

        let ability_tail_tokens = and_has.tail_tokens.to_vec();
        if !ability_tail_tokens.is_empty() {
            let mut handled_split_keyword_activation = false;
            if let Some(colon) = anthem_grant_grammar::parse_colon_tail_split(&ability_tail_tokens)
                && let Some(split_and_idx) = colon.last_and_before_colon {
                    let trailing_keyword_tokens =
                        trim_edge_punctuation(&ability_tail_tokens[..split_and_idx]);
                    let activated_tail =
                        trim_edge_punctuation(&ability_tail_tokens[split_and_idx + 1..]);
                    if !trailing_keyword_tokens.is_empty() {
                        let Some(actions) = parse_ability_line(&trailing_keyword_tokens) else {
                            return Ok(None);
                        };
                        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                        keyword_actions.extend(
                            actions
                                .into_iter()
                                .filter(|action| action.lowers_to_static_ability()),
                        );
                    }
                    let has_colon =
                        anthem_grant_grammar::parse_colon_tail_split(&activated_tail).is_some();
                    let Some(parsed) = parse_activated_line(&activated_tail)? else {
                        if has_colon {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported granted activated ability in anthem clause (clause: '{}')",
                                clause_words.join(" ")
                            )));
                        }
                        return Ok(None);
                    };
                    let display = display_text_for_tokens(&activated_tail, false);
                    granted_activated_display = Some(display);
                    granted_activated_ability = Some(parsed);
                    handled_split_keyword_activation = true;
                }
            if !handled_split_keyword_activation {
                let has_colon =
                    anthem_grant_grammar::parse_colon_tail_split(&ability_tail_tokens).is_some();
                let Some(parsed) = parse_activated_line(&ability_tail_tokens)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = display_text_for_tokens(&ability_tail_tokens, false);
                granted_activated_display = Some(display);
                granted_activated_ability = Some(parsed);
            }
        }
    } else if let Some(colon) = anthem_grant_grammar::parse_colon_tail_split(&ability_tokens) {
        let Some(and_idx) = colon.last_and_before_colon else {
            let activated_tail_storage = trim_edge_punctuation(&ability_tokens);
            let activated_tail = trim_outer_quotes(&activated_tail_storage);
            let Some(parsed) = parse_activated_line(activated_tail)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            let display = display_text_for_tokens(activated_tail, false);
            granted_activated_display = Some(display);
            granted_activated_ability = Some(parsed);
            let mut clause = parse_anthem_clause(tokens, get_idx, line_shape.clause_tail_end)?;
            if let Some(condition) = trailing_condition {
                apply_anthem_trailing_condition(
                    &mut clause,
                    condition,
                    trailing_if_surface,
                    &clause_words,
                )?;
            }
            let mut result = vec![build_anthem_static_ability(&clause).into()];
            if let Some(ability) = granted_activated_ability {
                result.push(grant_object_ability_for_anthem_subject(
                    &clause,
                    ability,
                    granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
                ));
            }
            return Ok(Some(result));
        };
        let keyword_head = trim_edge_punctuation(&ability_tokens[..and_idx]);
        let activated_tail = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
        if keyword_head.is_empty() || activated_tail.is_empty() {
            return Ok(None);
        }
        let Some(actions) = parse_ability_line(&keyword_head) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect();
        let has_colon = anthem_grant_grammar::parse_colon_tail_split(&activated_tail).is_some();
        let Some(parsed) = parse_activated_line(&activated_tail)? else {
            if has_colon {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(None);
        };
        let display = display_text_for_tokens(&activated_tail, false);
        granted_activated_display = Some(display);
        granted_activated_ability = Some(parsed);
    } else if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, &clause_words)?
    {
        granted_activated_display = Some(display);
        granted_activated_ability = Some(ability);
    } else if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect();
    } else {
        return Ok(None);
    }

    if keyword_actions.is_empty() && granted_activated_ability.is_none() {
        return Ok(None);
    }

    let mut clause = parse_anthem_clause(tokens, get_idx, line_shape.clause_tail_end)?;
    if let Some(condition) = trailing_condition {
        apply_anthem_trailing_condition(
            &mut clause,
            condition,
            trailing_if_surface,
            &clause_words,
        )?;
    }
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    for action in keyword_actions {
        result.push(grant_keyword_action_for_anthem_subject(&clause, action));
    }
    if let Some(additions) = trailing_type_color_addition {
        push_type_color_additions_for_anthem_subject(&mut result, &clause, additions);
    }

    if let Some(ability) = granted_activated_ability {
        result.push(grant_object_ability_for_anthem_subject(
            &clause,
            ability,
            granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
        ));
    }

    Ok(Some(result))
}

pub fn parse_anthem_and_goaded_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_anthem_goaded_shape(tokens) else {
        return Ok(None);
    };

    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let display_subject = attached_goaded_display_subject(&clause.subject).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported goaded anthem subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(vec![
        build_anthem_static_ability(&clause).into(),
        crate::static_abilities::StaticAbility::attached_goaded_by_source_controller(format!(
            "{} is goaded",
            capitalize_display_subject(&display_subject)
        ))
        .into(),
    ]))
}

pub fn parse_anthem_and_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_anthem_no_defender_grant_tokens(tokens) else {
        return Ok(None);
    };
    let clause = parse_anthem_clause(tokens, shape.get_token, shape.anthem_end)?;
    let no_defender = StaticAbilityAst::Static(StaticAbility::can_attack_as_though_no_defender());
    let granted = match &clause.subject {
        AnthemSubjectAst::Source => match clause.condition.clone() {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(no_defender),
                condition,
            },
            None => no_defender,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter: filter.clone(),
            ability: Box::new(no_defender),
            condition: clause.condition.clone(),
        },
    };
    Ok(Some(vec![
        build_anthem_static_ability(&clause).into(),
        granted,
    ]))
}

fn attached_goaded_display_subject(subject: &AnthemSubjectAst) -> Option<String> {
    let AnthemSubjectAst::Filter(filter) = subject else {
        return None;
    };
    let attachment = filter.tagged_constraints.iter().find_map(|constraint| {
        if !matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) {
            return None;
        }
        match constraint.tag.as_str() {
            "enchanted" => Some("enchanted"),
            "equipped" => Some("equipped"),
            _ => None,
        }
    })?;

    let noun = if filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else {
        "permanent"
    };
    Some(format!("{attachment} {noun}"))
}

fn capitalize_display_subject(subject: &str) -> String {
    let mut chars = subject.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn push_type_color_additions_for_anthem_subject(
    result: &mut Vec<StaticAbilityAst>,
    clause: &ParsedAnthemClause,
    additions: TypeColorAdditionClause,
) {
    let filter = anthem_subject_filter(&clause.subject);
    let condition = clause.condition.clone();
    let mut push_static = |ability: StaticAbility| {
        let ast: StaticAbilityAst = ability.into();
        result.push(match &condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ast),
                condition: condition.clone(),
            },
            None => ast,
        });
    };

    if !additions.set_colors.is_empty() {
        push_static(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ));
    }
    if !additions.added_colors.is_empty() {
        push_static(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ));
    }
    if !additions.card_types.is_empty() {
        push_static(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ));
    }
    if !additions.subtypes.is_empty() {
        push_static(StaticAbility::add_subtypes(filter, additions.subtypes));
    }
}

fn merge_static_ability_ast_conditions(
    existing: Option<crate::ConditionExpr>,
    additional: crate::ConditionExpr,
) -> crate::ConditionExpr {
    match existing {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(additional)),
        None => additional,
    }
}

fn add_static_ability_ast_condition(
    ability: StaticAbilityAst,
    condition: crate::ConditionExpr,
) -> Result<StaticAbilityAst, CardTextError> {
    Ok(match ability {
        StaticAbilityAst::Static(_)
        | StaticAbilityAst::KeywordAction(_)
        | StaticAbilityAst::PregameRevealFromOpeningHand { .. }
        | StaticAbilityAst::LoseGameReplacement { .. } => {
            StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            }
        }
        StaticAbilityAst::ConditionalStaticAbility {
            ability,
            condition: existing,
        } => StaticAbilityAst::ConditionalStaticAbility {
            ability,
            condition: crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        },
        StaticAbilityAst::LabeledConditionalStaticAbility {
            ability,
            condition: existing,
            label,
        } => StaticAbilityAst::LabeledConditionalStaticAbility {
            ability,
            condition: crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
            label,
        },
        StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition: existing,
        } => StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition: crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        },
        StaticAbilityAst::WithSetQuantifierSurface { ability, surface } => {
            StaticAbilityAst::WithSetQuantifierSurface {
                ability: Box::new(add_static_ability_ast_condition(*ability, condition)?),
                surface,
            }
        }
        StaticAbilityAst::GrantStaticAbility {
            filter,
            ability,
            condition: existing,
        } => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition: existing,
        } => StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display,
            condition: existing,
            protection_does_not_remove_controlled_attachments,
        } => StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
            protection_does_not_remove_controlled_attachments,
        },
        StaticAbilityAst::AttachedChosenLandwalkGrant {
            snow,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedChosenLandwalkGrant {
            snow,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::RemoveStaticAbility { .. }
        | StaticAbilityAst::RemoveKeywordAction { .. }
        | StaticAbilityAst::EquipmentKeywordActionsGrant { .. }
        | StaticAbilityAst::SoulbondSharedObjectAbility { .. }
        | StaticAbilityAst::AttachmentRestriction { .. } => {
            return Err(CardTextError::ParseError(
                "cannot apply leading static condition to unsupported static ability shape"
                    .to_string(),
            ));
        }
    })
}

pub fn parse_protection_from_colored_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if anthem_grant_grammar::parse_colored_spell_protection_tokens(tokens).is_none() {
        return Ok(None);
    }

    let all_colors = crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN);
    let mut filter = ObjectFilter::spell();
    filter.colors = Some(all_colors);
    Ok(Some(StaticAbility::protection(
        crate::ability::ProtectionFrom::Permanents(filter),
    )))
}

fn with_anthem_set_quantifier_surface(
    ability: StaticAbilityAst,
    clause: &ParsedAnthemClause,
) -> StaticAbilityAst {
    if !matches!(clause.subject, AnthemSubjectAst::Filter(_)) {
        return ability;
    }
    let Some(surface) = clause.set_quantifier_surface else {
        return ability;
    };
    StaticAbilityAst::WithSetQuantifierSurface {
        ability: Box::new(ability),
        surface,
    }
}

fn grant_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: StaticAbility,
) -> StaticAbilityAst {
    let granted = match &clause.subject {
        AnthemSubjectAst::Source => match &clause.condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition: condition.clone(),
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter: filter.clone(),
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition: clause.condition.clone(),
        },
    };
    with_anthem_set_quantifier_surface(granted, clause)
}

fn every_subtype_family_for_subject(
    subject: &AnthemSubjectAst,
    family: crate::types::SubtypeFamily,
    condition: Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    let base = match subject {
        AnthemSubjectAst::Source => {
            StaticAbility::add_all_subtypes_of_family(ObjectFilter::source(), family)
        }
        AnthemSubjectAst::Filter(filter) => {
            StaticAbility::add_all_subtypes_of_family(filter.clone(), family)
        }
    };

    let ability = condition
        .as_ref()
        .map(|cond| base.clone().with_condition(cond.clone()))
        .unwrap_or({
            #[cfg(not(feature = "serialization"))]
            {
                base
            }
            #[cfg(feature = "serialization")]
            {
                Some(base)
            }
        });
    #[cfg(not(feature = "serialization"))]
    {
        StaticAbilityAst::Static(ability)
    }
    #[cfg(feature = "serialization")]
    {
        StaticAbilityAst::Static(ability.expect("runtime static ability should exist"))
    }
}

fn grant_keyword_action_for_anthem_subject(
    clause: &ParsedAnthemClause,
    action: KeywordAction,
) -> StaticAbilityAst {
    let granted = match &clause.subject {
        AnthemSubjectAst::Source => match &clause.condition {
            Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                action,
                condition: condition.clone(),
            },
            None => StaticAbilityAst::KeywordAction(action),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter: filter.clone(),
            action,
            condition: clause.condition.clone(),
        },
    };
    with_anthem_set_quantifier_surface(granted, clause)
}

/// A trailing mana-source `if` qualifies each affected spell, not the
/// permanent granting the ability. Move that exact predicate into the spell
/// filter so continuous matching evaluates the cast spell's retained mana
/// source snapshots. Other conditions remain ordinary static conditions.
fn apply_anthem_trailing_condition(
    clause: &mut ParsedAnthemClause,
    condition: crate::ConditionExpr,
    trailing_if_surface: bool,
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    if clause.condition.is_some() {
        return Err(CardTextError::ParseError(format!(
            "multiple anthem conditions are not supported (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if trailing_if_surface
        && let crate::ConditionExpr::ValueComparison {
            left:
                crate::effect::Value::ManaFromSourceSpentToCastThisSpell {
                    source_filter,
                    include_source_noun: false,
                    ..
                },
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(1),
        } = &condition
        && let AnthemSubjectAst::Filter(filter) = &mut clause.subject
        && filter.mana_from_source_spent_to_cast.is_none()
    {
        filter.mana_from_source_spent_to_cast = Some(Box::new(source_filter.clone()));
        filter.set_mana_source_spent_trailing_if_surface(true);
        return Ok(());
    }

    clause.condition = Some(bind_attachment_condition_to_subject(
        condition,
        &clause.subject,
    ));
    Ok(())
}

fn remove_keyword_action_for_anthem_subject(
    clause: &ParsedAnthemClause,
    action: KeywordAction,
) -> StaticAbilityAst {
    let removal = StaticAbilityAst::RemoveKeywordAction {
        filter: anthem_subject_filter(&clause.subject),
        action,
        mode: ironsmith_core::AbilityLossMode::Lose,
    };
    match &clause.condition {
        Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(removal),
            condition: condition.clone(),
        },
        None => removal,
    }
}

fn granted_object_ability_for_keyword_action(
    action: &KeywordAction,
) -> Option<(ParsedAbility, String)> {
    match action {
        KeywordAction::Afflict(amount) => Some((
            parsed_ability_from_ability(afflict_triggered_ability(*amount)),
            action.display_text(),
        )),
        _ => None,
    }
}

fn parse_if_its_color_tail(tokens: &[OwnedLexToken]) -> Option<ColorSet> {
    crate::grammar::anthem_grants::parse_if_source_is_color(tokens)
}

fn parse_keyword_if_color_segment(
    segment: &[OwnedLexToken],
    clause_text: &str,
) -> Result<Option<(Vec<KeywordAction>, ColorSet)>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_keyword_if_color_shape(segment) else {
        return Ok(None);
    };
    let Some(color) = parse_if_its_color_tail(shape.color_tail_tokens) else {
        return Ok(None);
    };
    let Some(actions) = parse_ability_line(shape.keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, clause_text)?;
    let actions = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if actions.is_empty() {
        return Ok(None);
    }
    Ok(Some((actions, color)))
}

fn color_filtered_grant_filter(mut filter: ObjectFilter, color: ColorSet) -> ObjectFilter {
    let existing = filter.colors.unwrap_or_default();
    filter.colors = Some(existing.union(color));
    filter
}

fn source_color_condition(color: ColorSet) -> crate::ConditionExpr {
    let mut filter = ObjectFilter::source();
    filter.colors = Some(color);
    crate::ConditionExpr::SourceMatches(filter)
}

fn append_condition(
    condition: Option<crate::ConditionExpr>,
    next: crate::ConditionExpr,
) -> crate::ConditionExpr {
    match condition {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(next)),
        None => next,
    }
}

fn parse_color_filtered_keyword_grants(
    subject_tokens: &[OwnedLexToken],
    keyword_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
    clause_text: &str,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let mut parsed_segments = Vec::new();
    for segment in anthem_grant_grammar::split_keyword_if_color_segments(keyword_tokens) {
        let Some(parsed) = parse_keyword_if_color_segment(segment, clause_text)? else {
            return Ok(None);
        };
        parsed_segments.push(parsed);
    }
    if parsed_segments.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(subject_tokens)?;
    let mut compiled = Vec::new();
    for (actions, color) in parsed_segments {
        for action in actions {
            match &subject {
                AnthemSubjectAst::Source => {
                    compiled.push(StaticAbilityAst::ConditionalKeywordAction {
                        action,
                        condition: append_condition(
                            condition.clone(),
                            source_color_condition(color),
                        ),
                    })
                }
                AnthemSubjectAst::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantKeywordAction {
                        filter: color_filtered_grant_filter(filter.clone(), color),
                        action,
                        condition: condition.clone(),
                    });
                }
            }
        }
    }

    Ok(Some(compiled))
}

fn anthem_subject_filter(subject: &AnthemSubjectAst) -> ObjectFilter {
    match &subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter.clone(),
    }
}

fn grant_object_ability_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: ParsedAbility,
    display: String,
) -> StaticAbilityAst {
    if let Some(filter) = attached_object_anthem_subject_filter(&clause.subject) {
        let subject = filter.description();
        return StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display: format!("{subject} has {display}"),
            condition: clause.condition.clone(),
        };
    }

    with_anthem_set_quantifier_surface(
        StaticAbilityAst::GrantObjectAbility {
            filter: anthem_subject_filter(&clause.subject),
            ability,
            display,
            condition: clause.condition.clone(),
        },
        clause,
    )
}

fn attached_object_anthem_subject_filter(subject: &AnthemSubjectAst) -> Option<&ObjectFilter> {
    let AnthemSubjectAst::Filter(filter) = subject else {
        return None;
    };
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            ) && matches!(constraint.tag.as_str(), "enchanted" | "equipped")
        })
        .then_some(filter)
}

fn parsed_ability_from_ability(ability: Ability) -> ParsedAbility {
    ParsedAbility {
        ability: ability.into(),
        text: None,
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }
}

pub fn parse_equipment_you_control_have_equip_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_equipment_equip_shape(tokens) else {
        return Ok(None);
    };
    let condition = parse_static_condition_clause(shape.condition_tokens)?;
    let total_cost = parse_activation_cost(shape.cost_tokens)?;
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let ability = ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::attach_to(target.clone()),
                ]),
                choices: vec![target],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some("Equip {0}".to_string()),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    };
    Ok(Some(vec![StaticAbilityAst::GrantObjectAbility {
        filter: ObjectFilter::default()
            .with_subtype(Subtype::Equipment)
            .you_control(),
        ability,
        display: "Equipment you control have equip {0}".to_string(),
        condition: Some(condition),
    }]))
}

fn parsed_exploit_ability() -> ParsedAbility {
    let effect_id = 0;
    let ability = Ability::triggered(
        Trigger::this_enters_battlefield(),
        vec![
            Effect::with_id(
                effect_id,
                Effect::may(vec![Effect::sacrifice(ObjectFilter::creature(), 1)]),
            ),
            Effect::if_then(
                effect_id,
                crate::effect::EffectPredicate::Happened,
                vec![Effect::emit_keyword_action_with_affected_object_memory_tag(
                    crate::events::KeywordActionKind::Exploit,
                    1,
                    crate::effect::EffectId(effect_id),
                    crate::tag::EXPLOITED_TAG,
                )],
            ),
        ],
    );
    ParsedAbility {
        ability: ability.into(),
        text: Some("Exploit".to_string()),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: Some(TriggerSpec::ThisEntersBattlefield {
            origin_condition: None,
        }),
    }
}

fn grant_exploit_for_anthem_subject(
    subject: &AnthemSubjectAst,
    condition: Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    StaticAbilityAst::GrantObjectAbility {
        filter: anthem_subject_filter(subject),
        ability: parsed_exploit_ability(),
        display: "exploit".to_string(),
        condition,
    }
}

fn parse_triggered_granted_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trigger_tokens = trim_edge_punctuation(tokens);
    if trigger_tokens.is_empty() {
        return Ok(None);
    }
    let intro = crate::grammar::clause_support::parse_trigger_intro_tokens(
        &trigger_tokens,
    );
    if intro.body_first == 0 {
        return Ok(None);
    }

    let ability = match crate::clause_support::parse_triggered_line_lexed(
        &trigger_tokens,
    )? {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => {
            let (effects, trigger_condition) =
                triggered_grant_effects_and_condition(&trigger, &effects)?;
            let max_condition = trigger_surface::parse_trigger_frequency_condition_tokens(
                &trigger_tokens,
                max_triggers_per_turn,
            );
            let intervening_if = match (trigger_condition, max_condition) {
                (Some(left), Some(right)) => {
                    Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)))
                }
                (Some(condition), None) | (None, Some(condition)) => Some(condition),
                (None, None) => None,
            };
            parsed_triggered_ability(
                trigger,
                effects,
                vec![Zone::Battlefield],
                Some(crate::lexer::token_word_refs(&trigger_tokens).join(" ")),
                intervening_if,
                None,
                ReferenceImports::default(),
            )
        }
        _ => return Ok(None),
    };
    if parsed_triggered_ability_is_empty(&ability) {
        return Err(CardTextError::ParseError(format!(
            "unsupported empty triggered granted ability clause (clause: '{}')",
            crate::lexer::token_word_refs(&trigger_tokens).join(" ")
        )));
    }
    Ok(Some(ability))
}

fn parsed_triggered_ability_is_empty(ability: &ParsedAbility) -> bool {
    matches!(
        ability.kind(),
        AbilityKind::Triggered(triggered)
            if triggered.effects.is_empty()
                && ability
                    .effects_ast
                    .as_ref()
                    .is_none_or(|effects| effects.is_empty())
    )
}

fn parse_granted_keyword_fragment(segment: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    parse_ability_line(segment).or_else(|| {
        anthem_grant_grammar::parse_unblockable_keyword_fragment_tokens(segment)
            .map(|action| vec![action])
    })
}

fn parse_granted_object_ability_segment(
    raw_segment: &[OwnedLexToken],
    clause_words: &[&str],
    attached_subject: bool,
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let sanitized_tokens = raw_segment
        .iter()
        .filter(|token| token.kind != TokenKind::Quote)
        .cloned()
        .collect::<Vec<_>>();
    let ability_tokens = trim_edge_punctuation(&sanitized_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    if let Some(actions) = parse_ability_line(&ability_tokens)
        && actions.len() == 1
        && let Some(granted) = nonstatic_keyword_action_as_granted_object_ability(
            actions.into_iter().next().expect("single action exists"),
        )
    {
        return Ok(Some(granted));
    }

    if attached_subject && contains_token_kind(&ability_tokens, TokenKind::Colon) {
        let Some(parsed) = parse_attached_granted_activated_line(raw_segment)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some((ability, display)));
    }

    if let Some(parsed) = parse_attached_nonstatic_keyword_ability(&ability_tokens)? {
        return Ok(Some(parsed));
    }

    if let Some(parsed) = parse_cycling_line(&ability_tokens)? {
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    if let Some(parsed) = parse_equip_line_lexed(&ability_tokens)? {
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    if contains_token_kind(&ability_tokens, TokenKind::Colon) {
        let Some(parsed) = parse_activated_line(&ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    Ok(None)
}

fn nonstatic_keyword_action_as_granted_object_ability(
    action: KeywordAction,
) -> Option<(ParsedAbility, String)> {
    match action {
        KeywordAction::Soulshift(amount) => {
            let ability =
                crate::CardDefinitionBuilder::new(crate::CardId::from_raw(0), "Soulshift")
                    .soulshift(amount)
                    .build()
                    .abilities
                    .into_iter()
                    .next()?;
            Some((
                parsed_ability_from_ability(ability),
                format!("Soulshift {amount}"),
            ))
        }
        KeywordAction::SoulshiftValue(value) => Some((
            parsed_ability_from_ability(
                crate::CardDefinitionBuilder::soulshift_triggered_ability_from_value(value.clone()),
            ),
            format!(
                "Soulshift X, where X is {}",
                crate::payload::describe_soulshift_value(&value)
            ),
        )),
        KeywordAction::Casualty(power) => {
            let mut creature_filter = ObjectFilter::creature().you_control();
            creature_filter.power =
                Some(crate::filter::Comparison::GreaterThanOrEqual(power as i32));
            let ability = Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::you_cast_this_spell(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::may(
                        vec![
                            Effect::sacrifice(creature_filter, 1),
                            Effect::with_id(
                                0,
                                Effect::new(crate::effects::CopySpellEffect::single(
                                    ChooseSpec::Source,
                                )),
                            ),
                            Effect::may_choose_new_targets_player(
                                crate::effect::EffectId(0),
                                PlayerFilter::You,
                            ),
                        ],
                    )]),
                    choices: Vec::new(),
                    intervening_if: None,
                    presentation_label: Some(PresentationLabel::Keyword(
                        PresentationKeyword::Casualty(power),
                    )),
                }),
                functional_zones: vec![Zone::Stack],
            };
            Some((
                ParsedAbility {
                    ability: ability.into(),
                    text: Some(format!("Casualty {power}")),
                    effects_ast: None,
                    reference_imports: ReferenceImports::default(),
                    trigger_spec: None,
                },
                format!("Casualty {power}"),
            ))
        }
        _ => None,
    }
}

pub fn parse_heterogeneous_granted_tail(
    tail_tokens: &[OwnedLexToken],
    clause_words: &[&str],
    attached_subject: bool,
) -> Result<Option<ParsedGrantedTailAst>, CardTextError> {
    let mut parsed = ParsedGrantedTailAst::default();

    for raw_segment in anthem_grant_grammar::split_trailing_grant_segments(tail_tokens) {
        let Some(segment) = anthem_grant_grammar::parse_trailing_grant_segment(&raw_segment) else {
            continue;
        };
        let segment = segment.body_tokens.to_vec();
        if matches!(
            crate::lexer::token_word_refs(&segment).as_slice(),
            ["lose" | "loses", "all", "other", "abilities"]
        ) {
            parsed.removes_all_other_abilities = true;
            continue;
        }

        if let Some(additions) = parse_type_color_addition_clause(&segment)? {
            parsed.type_color_additions.push(additions);
            continue;
        }

        if is_can_block_shadow_as_though_no_shadow_clause(&segment) {
            parsed
                .granted_static
                .push(StaticAbility::can_block_as_though_no_shadow().into());
            continue;
        }

        if let Some((ability, display)) =
            parse_granted_object_ability_segment(&segment, clause_words, attached_subject)?
        {
            parsed.granted_object_abilities.push((ability, display));
            continue;
        }

        let split_fragments = split_lexed_slices_on_and(&segment)
            .into_iter()
            .map(trim_edge_punctuation)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if split_fragments.len() > 1 {
            let mut split_keywords = Vec::new();
            let mut split_static = Vec::new();
            let mut valid = true;
            for fragment in split_fragments {
                if anthem_grant_grammar::parse_no_defender_granted_fragment_tokens(&fragment) {
                    split_static.push(StaticAbility::can_attack_as_though_no_defender().into());
                    continue;
                }
                let Some(actions) = parse_granted_keyword_fragment(&fragment) else {
                    valid = false;
                    break;
                };
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                if !actions.iter().all(KeywordAction::lowers_to_static_ability) {
                    valid = false;
                    break;
                }
                split_keywords.extend(actions);
            }
            if valid {
                parsed.granted_keyword_actions.extend(split_keywords);
                parsed.granted_static.extend(split_static);
                continue;
            }
        }

        if let Some(actions) = parse_granted_keyword_fragment(&segment) {
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            if let [KeywordAction::CumulativeUpkeep { total_cost, .. }] = actions.as_slice() {
                parsed.granted_object_abilities.push((
                    ParsedAbility {
                        ability: cumulative_upkeep_granted_ability(total_cost.clone()).into(),
                        text: Some(display_text_for_tokens(&segment, false)),
                        effects_ast: None,
                        reference_imports: ReferenceImports::default(),
                        trigger_spec: None,
                    },
                    display_text_for_tokens(&segment, false),
                ));
                continue;
            }

            let lowered = actions
                .into_iter()
                .filter(|action| action.lowers_to_static_ability())
                .collect::<Vec<_>>();
            if lowered.is_empty() {
                return Ok(None);
            }
            parsed.granted_keyword_actions.extend(lowered);
            continue;
        }

        if let Some(marker) = parse_static_text_marker_line(&segment) {
            parsed.granted_static.push(marker.into());
            continue;
        }

        let mut segment_with_period = segment.to_vec();
        segment_with_period.push(OwnedLexToken::period(
            crate::cards::builders::TextSpan::synthetic(),
        ));
        if let Some(marker) = parse_static_text_marker_line(&segment_with_period) {
            parsed.granted_static.push(marker.into());
            continue;
        }

        if let Some(abilities) = parse_static_ability_ast_line_lexed(&segment)? {
            parsed.granted_static.extend(abilities);
            continue;
        }

        return Ok(None);
    }

    if parsed.granted_static.is_empty()
        && parsed.granted_keyword_actions.is_empty()
        && parsed.granted_object_abilities.is_empty()
        && !parsed.removes_all_other_abilities
        && parsed.type_color_additions.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(parsed))
}

fn is_can_block_shadow_as_though_no_shadow_clause(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        trim_edge_punctuation(tokens)
            .iter()
            .filter_map(|token| token.as_word().map(|_| token.parser_text()))
            .collect::<Vec<_>>()
            .as_slice(),
        [
            "can",
            "block",
            "creatures",
            "with",
            "shadow",
            "as",
            "though",
            "they",
            "didnt" | "didn't",
            "have",
            "shadow"
        ]
    )
}

/// Parse an attached-creature characteristic line whose final coordinated
/// member is the rules permission to block shadow creatures. Keeping this
/// complete line together prevents the ordinary granted-ability grammar from
/// reducing the permission's final `shadow` noun to a quoted ability marker.
pub fn parse_attached_anthem_reach_shadow_permission_line(
    tokens: &[OwnedLexToken],
) -> Option<Vec<StaticAbilityAst>> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if !matches!(
        words.as_slice(),
        [
            "enchanted",
            "creature",
            "gets",
            "+1/+1",
            "has",
            "reach",
            "and",
            "can",
            "block",
            "creatures",
            "with",
            "shadow",
            "as",
            "though",
            "they",
            "didnt" | "didn't",
            "have",
            "shadow"
        ]
    ) {
        return None;
    }

    let filter = ObjectFilter::creature().in_zone(Zone::Battlefield).match_tagged(
        "enchanted",
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    Some(vec![
        StaticAbilityAst::Static(StaticAbility::new(Anthem::new(filter.clone(), 1, 1))),
        StaticAbilityAst::GrantStaticAbility {
            filter: filter.clone(),
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::reach())),
            condition: None,
        },
        StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_block_as_though_no_shadow(),
            )),
            condition: None,
        },
    ])
}

pub fn parse_source_can_block_shadow_as_though_no_shadow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    let words = trimmed
        .iter()
        .filter_map(|token| token.as_word().map(|_| token.parser_text()))
        .collect::<Vec<_>>();
    if !matches!(words.as_slice(), ["this", "creature", ..])
        || !is_can_block_shadow_as_though_no_shadow_clause(&tokens[2..])
    {
        return Ok(None);
    }
    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::can_block_as_though_no_shadow(),
    )))
}

pub fn parse_targeting_as_though_no_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(spec) = crate::effect_sentences::
        parse_targeting_as_though_no_ability_spec(tokens)?
    else {
        return Ok(None);
    };
    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::targeting_as_though_no_ability(spec),
    )))
}

#[test]
fn shadow_block_permission_is_typed_and_rejects_plain_shadow() {
    let tokens = crate::lexer::lex_line(
        "This creature can block creatures with shadow as though they didn't have shadow.",
        0,
    )
    .expect("permission should lex");
    let parsed = parse_source_can_block_shadow_as_though_no_shadow_line(&tokens)
        .expect("permission should parse")
        .expect("permission should be claimed");
    assert!(
        format!("{parsed:#?}").contains("CanBlockAsThoughNoShadow"),
        "{parsed:#?}"
    );

    for near_miss in [
        "This creature has shadow.",
        "This creature can block creatures with shadow.",
    ] {
        let tokens = crate::lexer::lex_line(near_miss, 0)
            .expect("near miss should lex");
        assert!(
            parse_source_can_block_shadow_as_though_no_shadow_line(&tokens)
                .expect("near miss should parse safely")
                .is_none(),
            "claimed near miss: {near_miss}"
        );
    }
}

pub fn lower_granted_tail_for_anthem_subject(
    subject: &AnthemSubjectAst,
    condition: &Option<crate::ConditionExpr>,
    granted_tail: ParsedGrantedTailAst,
) -> Vec<StaticAbilityAst> {
    let wrapper_clause = ParsedAnthemClause {
        subject: subject.clone(),
        power: AnthemValue::Fixed(0),
        toughness: AnthemValue::Fixed(0),
        condition: condition.clone(),
        count_uses_where_x: false,
        additional_surface: false,
        set_quantifier_surface: None,
    };
    let mut granted = Vec::new();
    if granted_tail.removes_all_other_abilities {
        let remove: StaticAbilityAst =
            StaticAbility::remove_all_abilities(anthem_subject_filter(subject)).into();
        granted.push(match condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(remove),
                condition: condition.clone(),
            },
            None => remove,
        });
    }
    if !granted_tail.granted_static.is_empty() {
        granted.extend(grant_static_anthem_abilities_for_subject(
            &wrapper_clause,
            granted_tail.granted_static,
        ));
    }
    for action in granted_tail.granted_keyword_actions {
        granted.push(grant_keyword_action_for_anthem_subject(
            &wrapper_clause,
            action,
        ));
    }
    for (ability, display) in granted_tail.granted_object_abilities {
        granted.push(grant_object_ability_for_anthem_subject(
            &wrapper_clause,
            ability,
            display,
        ));
    }
    for additions in granted_tail.type_color_additions {
        push_type_color_additions_for_anthem_subject(&mut granted, &wrapper_clause, additions);
    }
    granted
}

pub fn parse_attached_restriction_and_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = attached_grammar::parse_attached_combat_restriction_grant_tokens(tokens)
    else {
        return Ok(None);
    };
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(shape.ability_tokens, &clause_words, true)?
    else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let subject_display = shape.subject.display();
    let (restriction, display) = match shape.kind {
        attached_grammar::AttachedCombatRestrictionKind::CantAttack => (
            crate::effect::Restriction::attack(ObjectFilter::source()),
            format!("{subject_display} can't attack"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantBlock => (
            crate::effect::Restriction::block(ObjectFilter::source()),
            format!("{subject_display} can't block"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantAttackOrBlock => (
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            format!("{subject_display} can't attack or block"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantBeBlocked => return Ok(None),
    };
    let mut result = vec![StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
            restriction,
            display.clone(),
        ))),
        display,
        condition: None,
    }];
    result.extend(lower_granted_tail_for_anthem_subject(
        &subject,
        &None,
        granted_tail,
    ));
    Ok(Some(result))
}

pub fn parse_subject_color_and_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_subject_color_and_grant_tokens(tokens) else {
        return Ok(None);
    };
    let condition = shape
        .condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let clause_words = crate::lexer::token_word_refs(tokens);
    let attached_subject =
        anthem_grant_grammar::parse_granted_subject_facts(shape.subject_tokens).attached_subject;
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(shape.ability_tokens, &clause_words, attached_subject)?
    else {
        return Ok(None);
    };
    let set_color = StaticAbilityAst::Static(StaticAbility::set_colors(
        anthem_subject_filter(&subject),
        shape.color,
    ));
    let mut result = vec![match condition.clone() {
        Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(set_color),
            condition,
        },
        None => set_color,
    }];
    result.extend(lower_granted_tail_for_anthem_subject(
        &subject,
        &condition,
        granted_tail,
    ));
    Ok(Some(result))
}

fn wrap_conditioned_animation_static_ability(
    ability: StaticAbility,
    condition: &Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    if let Some(condition) = condition {
        #[cfg(not(feature = "serialization"))]
        {
            return ability.with_condition(condition.clone()).into();
        }
        #[cfg(feature = "serialization")]
        {
            return ability
                .with_condition(condition.clone())
                .expect("runtime conditioned static ability should exist")
                .into();
        }
    }
    ability.into()
}

pub fn lower_static_animation_bundle(
    bundle: StaticAnimationBundleAst,
) -> Vec<StaticAbilityAst> {
    let filter = anthem_subject_filter(&bundle.subject);
    let mut lowered = Vec::new();

    if bundle.ensure_creature_type {
        lowered.push(wrap_conditioned_animation_static_ability(
            StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
            &bundle.condition,
        ));
    }
    if let Some((power, toughness)) = bundle.base_power_toughness {
        let ability = match (&power, &toughness) {
            (Value::Fixed(power), Value::Fixed(toughness)) => {
                StaticAbility::set_base_power_toughness(filter.clone(), *power, *toughness)
            }
            _ => StaticAbility::set_base_power_toughness_value(filter.clone(), power, toughness),
        };
        lowered.push(wrap_conditioned_animation_static_ability(
            ability,
            &bundle.condition,
        ));
    }
    if !bundle.subtypes.is_empty() {
        let ability = match bundle.subtype_mode {
            AnimationSubtypeMode::Add => StaticAbility::add_subtypes(filter, bundle.subtypes),
            AnimationSubtypeMode::ReplaceCreatureTypes => {
                StaticAbility::set_creature_subtypes(filter, bundle.subtypes)
            }
        };
        lowered.push(wrap_conditioned_animation_static_ability(
            ability,
            &bundle.condition,
        ));
    }

    lowered.extend(lower_granted_tail_for_anthem_subject(
        &bundle.subject,
        &bundle.condition,
        bundle.granted_tail,
    ));

    lowered
}

fn grant_static_anthem_abilities_for_subject(
    clause: &ParsedAnthemClause,
    abilities: Vec<StaticAbilityAst>,
) -> Vec<StaticAbilityAst> {
    let mut granted = Vec::new();
    for ability in abilities {
        granted.push(match &clause.subject {
            AnthemSubjectAst::Source => match &clause.condition {
                Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(ability),
                    condition: condition.clone(),
                },
                None => ability,
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                filter: filter.clone(),
                ability: Box::new(ability),
                condition: clause.condition.clone(),
            },
        });
    }
    granted
}

fn parse_continuing_anthem_granted_segment(
    clause: &ParsedAnthemClause,
    clause_words: &[&str],
    segment: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sanitized_tokens = segment
        .iter()
        .filter(|token| token.kind != TokenKind::Quote)
        .cloned()
        .collect::<Vec<_>>();
    let ability_tokens = trim_edge_punctuation(&sanitized_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![grant_object_ability_for_anthem_subject(
            clause, ability, display,
        )]));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        let granted = actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect::<Vec<_>>();
        if granted.is_empty() {
            return Ok(None);
        }
        return Ok(Some(
            granted
                .into_iter()
                .map(|ability| grant_for_anthem_subject(clause, ability))
                .collect(),
        ));
    }

    if let Some(marker) = parse_static_text_marker_line(&ability_tokens) {
        return Ok(Some(vec![grant_for_anthem_subject(clause, marker)]));
    }

    let mut ability_tokens_with_period = ability_tokens.to_vec();
    ability_tokens_with_period.push(OwnedLexToken::period(
        crate::cards::builders::TextSpan::synthetic(),
    ));
    if let Some(amount) =
        super::grammar::abilities::parse_ward_pay_life_amount_lexed(&ability_tokens_with_period)
    {
        return Ok(Some(vec![grant_for_anthem_subject(
            clause,
            StaticAbility::ward(crate::cost::TotalCost::from_cost(crate::costs::Cost::life(
                amount,
            ))),
        )]));
    }
    if let Some(marker) = parse_static_text_marker_line(&ability_tokens_with_period) {
        return Ok(Some(vec![grant_for_anthem_subject(clause, marker)]));
    }

    if let Some(abilities) = parse_static_ability_ast_line_lexed(&ability_tokens)? {
        return Ok(Some(grant_static_anthem_abilities_for_subject(
            clause, abilities,
        )));
    }

    Ok(None)
}

pub fn parse_anthem_with_trailing_segments_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(head) = anthem_grant_grammar::parse_persistent_anthem_tail_head(tokens) else {
        return Ok(None);
    };
    let get_idx = head.get_token;
    let work_tokens = head.tokens;
    if parse_pt_modifier(&head.modifier_word).is_err() {
        return Ok(None);
    }

    let clause = parse_anthem_clause(&work_tokens, get_idx, head.tail_start)?;
    let tail_tokens = trim_commas(&work_tokens[head.tail_start..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let direct_have_tail =
        anthem_grant_grammar::parse_direct_have_tail(&tail_tokens).map(|tokens| tokens.to_vec());

    if let Some(grant_tail) = direct_have_tail {
        let mut extras: Vec<StaticAbilityAst> = Vec::new();
        for raw_segment in anthem_grant_grammar::split_trailing_grant_segments(&grant_tail) {
            let Some(segment) = anthem_grant_grammar::parse_trailing_grant_segment(&raw_segment)
            else {
                continue;
            };
            let segment = segment.body_tokens.to_vec();

            if let Some(mut granted) =
                parse_continuing_anthem_granted_segment(&clause, &clause_words, &segment)?
            {
                extras.append(&mut granted);
                continue;
            }

            let segment_shape = anthem_grant_grammar::parse_continuing_segment_shape(&segment);
            if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::MustAttack {
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()));
                continue;
            }
            if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::CantAttackAlone {
                extras.push(
                    grant_for_anthem_subject(
                        &clause,
                        StaticAbility::restriction(
                            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
                            "This creature can't attack alone".to_string(),
                        ),
                    ),
                );
                continue;
            }
            if let anthem_grant_grammar::ContinuingSegmentShape::BeEverySubtype(family) =
                segment_shape
            {
                extras.push(every_subtype_family_for_subject(
                    &clause.subject,
                    family,
                    clause.condition.clone(),
                ));
                continue;
            }

            return Ok(None);
        }

        if extras.is_empty() {
            return Ok(None);
        }

        let mut result = vec![build_anthem_static_ability(&clause).into()];
        result.extend(extras);
        return Ok(Some(result));
    }

    let mut extras: Vec<StaticAbilityAst> = Vec::new();
    let mut continuing_have_clause = false;
    for raw_segment in anthem_grant_grammar::split_trailing_grant_segments(&tail_tokens) {
        let Some(segment) = anthem_grant_grammar::parse_trailing_grant_segment(&raw_segment) else {
            continue;
        };
        let segment = segment.body_tokens.to_vec();

        let segment_shape = anthem_grant_grammar::parse_continuing_segment_shape(&segment);
        if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::CantBlock {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::cant_block()));
            continue;
        }
        if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::CantAttackAlone {
            extras.push(
                grant_for_anthem_subject(
                    &clause,
                    StaticAbility::restriction(
                        crate::effect::Restriction::attack_alone(ObjectFilter::source()),
                        "This creature can't attack alone".to_string(),
                    ),
                ),
            );
            continue;
        }
        if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::MustAttack {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()));
            continue;
        }
        if let anthem_grant_grammar::ContinuingSegmentShape::CantBeBlockedByMoreThan(count) =
            segment_shape
        {
            extras.push(
                grant_for_anthem_subject(
                    &clause,
                    StaticAbility::cant_be_blocked_by_more_than(count),
                ),
            );
            continue;
        }
        if let anthem_grant_grammar::ContinuingSegmentShape::SetColor { color_word } = segment_shape
        {
            let Some(color) = parse_color(color_word) else {
                return Ok(None);
            };
            let filter = match &clause.subject {
                AnthemSubjectAst::Source => ObjectFilter::source(),
                AnthemSubjectAst::Filter(filter) => filter.clone(),
            };
            let mut set_colors = crate::static_abilities::SetColorsForFilter::new(filter, color);
            if let Some(condition) = &clause.condition {
                set_colors = set_colors.with_condition(condition.clone());
            }
            extras.push(StaticAbility::new(set_colors).into());
            continue;
        }
        if let anthem_grant_grammar::ContinuingSegmentShape::BeEverySubtype(family) = segment_shape
        {
            extras.push(every_subtype_family_for_subject(
                &clause.subject,
                family,
                clause.condition.clone(),
            ));
            continue;
        }

        if let anthem_grant_grammar::ContinuingSegmentShape::Lose { ability_tokens } = segment_shape
        {
            let ability_tokens = trim_edge_punctuation(ability_tokens);
            if ability_tokens.is_empty() {
                return Ok(None);
            }
            let Some(actions) = parse_ability_line(&ability_tokens) else {
                return Ok(None);
            };
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            if actions.is_empty()
                || actions
                    .iter()
                    .any(|action| !action.lowers_to_static_ability())
            {
                return Ok(None);
            }
            for action in actions {
                extras.push(remove_keyword_action_for_anthem_subject(&clause, action));
            }
            continue;
        }

        if let anthem_grant_grammar::ContinuingSegmentShape::Have { ability_tokens } = segment_shape
        {
            let mut ability_tokens = trim_edge_punctuation(ability_tokens);
            if ability_tokens.is_empty() {
                return Ok(None);
            }

            let mut grant_must_attack = false;
            if let Some(head) = anthem_grant_grammar::strip_must_attack_suffix(&ability_tokens) {
                ability_tokens = head.to_vec();
                grant_must_attack = true;
            }

            let mut granted_activated: Option<ParsedAbility> = None;
            let mut granted_activated_display: Option<String> = None;
            let split_keyword_and_activated = if let Some(split) =
                anthem_grant_grammar::split_keyword_and_activated(&ability_tokens)
            {
                let keyword_head = trim_edge_punctuation(split.keyword_tokens);
                let activated_tail = trim_edge_punctuation(split.activated_tokens);
                if keyword_head.is_empty() || activated_tail.is_empty() {
                    return Ok(None);
                }
                let Some(actions) = parse_ability_line(&keyword_head) else {
                    return Ok(None);
                };
                let has_colon = contains_token_kind(&activated_tail, TokenKind::Colon);
                let Some(parsed) = parse_activated_line(&activated_tail)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = display_text_for_tokens(&activated_tail, false);
                granted_activated_display = Some(display);
                granted_activated = Some(parsed);
                Some(actions)
            } else {
                None
            };
            let actions = if let Some(actions) = split_keyword_and_activated {
                Some(actions)
            } else if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
                parse_granted_activated_or_triggered_ability_for_gain(
                    &ability_tokens,
                    &clause_words,
                )?
            {
                granted_activated_display = Some(display);
                granted_activated = Some(ability);
                None
            } else if let Some(actions) = parse_ability_line(&ability_tokens) {
                Some(actions)
            } else if contains_token_kind(&ability_tokens, TokenKind::Colon) {
                let Some(split) =
                    anthem_grant_grammar::split_keyword_and_activated(&ability_tokens)
                else {
                    return Ok(None);
                };
                let keyword_head = trim_edge_punctuation(split.keyword_tokens);
                let activated_tail = trim_edge_punctuation(split.activated_tokens);
                if keyword_head.is_empty() || activated_tail.is_empty() {
                    return Ok(None);
                }
                let Some(actions) = parse_ability_line(&keyword_head) else {
                    return Ok(None);
                };
                let has_colon = contains_token_kind(&activated_tail, TokenKind::Colon);
                let Some(parsed) = parse_activated_line(&activated_tail)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = display_text_for_tokens(&activated_tail, false);
                granted_activated_display = Some(display);
                granted_activated = Some(parsed);
                Some(actions)
            } else {
                None
            };

            if let Some(triggered) = parse_triggered_granted_ability(&ability_tokens)? {
                let display = format!(
                    "{} has {}",
                    clause_words.join(" "),
                    crate::lexer::token_word_refs(&ability_tokens).join(" ")
                );
                extras.push(grant_object_ability_for_anthem_subject(
                    &clause, triggered, display,
                ));
            } else if let Some(actions) = actions {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                let granted = actions
                    .into_iter()
                    .filter_map(keyword_action_to_static_ability)
                    .collect::<Vec<_>>();
                if granted.is_empty() {
                    return Ok(None);
                }
                for ability in granted {
                    extras.push(grant_for_anthem_subject(&clause, ability));
                }

                if let Some(activated) = granted_activated {
                    extras.push(grant_object_ability_for_anthem_subject(
                        &clause,
                        activated,
                        granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
                    ));
                }
            } else {
                return Ok(None);
            }

            if grant_must_attack {
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()));
            }
            continuing_have_clause = true;
            continue;
        }

        if continuing_have_clause
            && let Some(mut granted) =
                parse_continuing_anthem_granted_segment(&clause, &clause_words, &segment)?
        {
            extras.append(&mut granted);
            continue;
        }

        if let Some(triggered) = parse_triggered_granted_ability(&segment)? {
            let display = format!(
                "{} has {}",
                clause_words.join(" "),
                crate::lexer::token_word_refs(&segment).join(" ")
            );
            extras.push(grant_object_ability_for_anthem_subject(
                &clause, triggered, display,
            ));
            continue;
        }

        return Ok(None);
    }

    if extras.is_empty() {
        return Ok(None);
    }

    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.extend(extras);
    Ok(Some(result))
}

pub fn parse_conditional_all_creatures_able_to_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_conditional_must_block_shape(tokens) else {
        return Ok(None);
    };
    let condition = parse_static_condition_clause(shape.condition_tokens)?;
    match shape.target {
        anthem_grant_grammar::ConditionalMustBlockTarget::Source => {
            Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                    crate::effect::Restriction::must_block_specific_attacker(
                        ObjectFilter::creature(),
                        ObjectFilter::source(),
                    ),
                    "All creatures able to block this creature do so".to_string(),
                ))),
                condition,
            }))
        }
        anthem_grant_grammar::ConditionalMustBlockTarget::EnchantedCreature => {
            let display = "All creatures able to block enchanted creature do so".to_string();
            Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                    crate::effect::Restriction::must_block_specific_attacker(
                        ObjectFilter::creature(),
                        ObjectFilter::source(),
                    ),
                    display.clone(),
                ))),
                display,
                condition: Some(condition),
            }))
        }
    }
}

#[test]
fn persistent_lure_cards_lower_to_specific_attacker_rule_restrictions() {
    fn assert_lure_rule_restriction(ability: &StaticAbilityAst, expected_display: &str) {
        let StaticAbilityAst::Static(ability) = ability else {
            panic!("expected a static rule restriction: {ability:#?}");
        };
        let crate::static_abilities::StaticAbilityPayload::RuleRestriction {
            restriction,
            additional_restrictions,
            display,
        } = &ability.payload
        else {
            panic!("expected a RuleRestriction payload: {ability:#?}");
        };
        let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
            restriction
        else {
            panic!("expected MustBlockSpecificAttacker: {restriction:#?}");
        };
        assert_eq!(blockers, &ObjectFilter::creature());
        assert_eq!(attacker, &ObjectFilter::source());
        assert!(additional_restrictions.is_empty());
        assert_eq!(display, expected_display);
    }

    // Nessian Boar and Shinen of Life's Roar share this unconditional line.
    let tokens = crate::lexer::lex_line(
        "All creatures able to block this creature do so.",
        0,
    )
    .expect("lex unconditional lure line");
    let unconditional = parse_all_creatures_able_to_block_source_line(&tokens)
        .expect("parse unconditional lure line")
        .expect("unconditional lure line should be recognized");
    assert_lure_rule_restriction(
        &unconditional,
        "All creatures able to block this creature do so",
    );

    // Stone-Tongue Basilisk uses the source-relative conditional form.
    let tokens = crate::lexer::lex_line(
        "As long as there are seven or more cards in your graveyard, all creatures able to block this creature do so.",
        0,
    )
    .expect("lex conditional source lure line");
    let conditional_source = parse_conditional_all_creatures_able_to_block_line(&tokens)
        .expect("parse conditional source lure line")
        .expect("conditional source lure line should be recognized");
    let ability = match &conditional_source {
        StaticAbilityAst::ConditionalStaticAbility { ability, .. } => ability.as_ref(),
        other => panic!("expected a conditional source restriction: {other:#?}"),
    };
    assert_lure_rule_restriction(ability, "All creatures able to block this creature do so");

    // Seton's Desire uses the attached-object conditional form.
    let tokens = crate::lexer::lex_line(
        "As long as there are seven or more cards in your graveyard, all creatures able to block enchanted creature do so.",
        0,
    )
    .expect("lex conditional enchanted lure line");
    let conditional_enchanted = parse_conditional_all_creatures_able_to_block_line(&tokens)
        .expect("parse conditional enchanted lure line")
        .expect("conditional enchanted lure line should be recognized");
    let (ability, display) = match &conditional_enchanted {
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            display,
            condition: Some(_),
        } => (ability.as_ref(), display),
        other => panic!("expected a conditional attached restriction: {other:#?}"),
    };
    assert_eq!(
        display,
        "All creatures able to block enchanted creature do so"
    );
    assert_lure_rule_restriction(
        ability,
        "All creatures able to block enchanted creature do so",
    );
}

pub fn parse_source_can_attack_as_though_no_defender_as_long_as_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_subject_no_defender_as_long_shape(tokens) else {
        return Ok(None);
    };
    let condition = parse_static_condition_clause(shape.condition_tokens)?;

    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub fn parse_attached_can_attack_as_though_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_attached_no_defender_shape(tokens) else {
        return Ok(None);
    };

    let subject = crate::lexer::token_word_refs(shape.subject_tokens).join(" ");
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(
            StaticAbility::can_attack_as_though_no_defender(),
        )),
        display: format!("{subject} can attack as though it didn't have defender"),
        condition: None,
    }))
}

pub fn parse_plain_can_attack_as_though_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_plain_no_defender_shape(tokens) else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let permission = StaticAbilityAst::Static(
        StaticAbility::can_attack_as_though_no_defender(),
    );
    Ok(Some(match subject {
        AnthemSubjectAst::Source => permission,
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(permission),
            condition: None,
        },
    }))
}

pub fn parse_attacked_player_can_attack_as_though_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    let words = crate::lexer::token_word_refs(&trimmed);
    if words.as_slice()
        != [
            "this", "creature", "can", "attack", "players", "who", "attacked", "you",
            "during", "their", "last", "turn", "as", "though", "it", "didn't", "have",
            "defender",
        ]
        && words.as_slice()
            != [
                "this", "creature", "can", "attack", "players", "who", "attacked", "you",
                "during", "their", "last", "turn", "as", "though", "it", "didnt", "have",
                "defender",
            ]
    {
        return Ok(None);
    }
    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::can_attack_players_who_attacked_controller_last_turn_as_though_no_defender(),
    )))
}

#[test]
fn plain_no_defender_permissions_lower_to_the_typed_combat_ability() {
    for text in [
        "Wall creatures can attack as though they didn't have defender.",
        "Creatures you control can attack as though they didn't have defender.",
        "Modified creatures you control can attack as though they didn't have defender.",
        "This creature can attack as though it didn't have defender.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).expect("lex permission");
        let parsed = parse_plain_can_attack_as_though_no_defender_line(&tokens)
            .expect("parse permission")
            .expect("plain permission shape");
        assert!(
            format!("{parsed:#?}").contains("CanAttackAsThoughNoDefender"),
            "{text}: {parsed:#?}"
        );
    }

    let near_miss = crate::lexer::lex_line(
        "Wall creatures have defender.",
        0,
    )
    .expect("lex near miss");
    assert!(
        parse_plain_can_attack_as_though_no_defender_line(&near_miss)
            .expect("parse near miss")
            .is_none()
    );
}

pub fn parse_as_long_as_condition_can_attack_as_though_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_leading_condition_no_defender_shape(tokens)
    else {
        return Ok(None);
    };

    let condition = parse_static_condition_clause(shape.condition_tokens)?;
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub fn parse_gets_and_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_gets_attacks_shape(tokens) else {
        return Ok(None);
    };
    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.push(grant_for_anthem_subject(
        &clause,
        StaticAbility::must_attack(),
    ));

    if result.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "failed to parse gets-and-attacks clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(result))
}

pub fn parse_anthem_and_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if anthem_grant_grammar::parse_static_grant_duration_fact(tokens).is_some() {
        return Ok(None);
    }

    let Some(shape) = anthem_grant_grammar::parse_anthem_and_granted_tail(tokens) else {
        return Ok(None);
    };
    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    match shape.tail_kind {
        anthem_grant_grammar::AnthemGrantedTailKind::CantBeBlocked => result.push(
            grant_for_anthem_subject(&clause, StaticAbility::unblockable()),
        ),
        anthem_grant_grammar::AnthemGrantedTailKind::BeEverySubtype(family) => {
            result.push(every_subtype_family_for_subject(
                &clause.subject,
                family,
                clause.condition.clone(),
            ));
        }
    }

    Ok(Some(result))
}

pub fn parse_subject_is_every_subtype_family_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if anthem_grant_grammar::parse_static_grant_duration_fact(tokens).is_some() {
        return Ok(None);
    }
    let Some(shape) = anthem_grant_grammar::parse_subject_every_subtype_shape(tokens) else {
        return Ok(None);
    };
    let condition = shape
        .condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    Ok(Some(every_subtype_family_for_subject(
        &subject,
        shape.family,
        condition,
    )))
}

pub fn parse_anthem_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(head) = anthem_grant_grammar::parse_anthem_modifier_head(tokens) else {
        return Ok(None);
    };
    if head.has_target || head.temporary {
        return Ok(None);
    }
    let modifier_word = tokens[head.modifier_token].as_word().unwrap_or_default();
    if parse_pt_modifier_values(modifier_word).is_err()
        && parse_dynamic_xy_anthem_values(
            modifier_word,
            &trim_edge_punctuation(tokens.get(head.modifier_token + 1..).unwrap_or_default()),
        )
        .is_none()
    {
        return Ok(None);
    }
    let clause = parse_anthem_clause(tokens, head.get_token, tokens.len())?;
    Ok(Some(build_anthem_static_ability(&clause)))
}

pub fn parse_multi_subject_anthem_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(head) = anthem_grant_grammar::parse_anthem_modifier_head(tokens) else {
        return Ok(None);
    };
    if head.has_target || head.temporary {
        return Ok(None);
    }
    let get_idx = head.get_token;
    let modifier_word = tokens[head.modifier_token].as_word().unwrap_or_default();
    if parse_pt_modifier_values(modifier_word).is_err()
        && parse_dynamic_xy_anthem_values(
            modifier_word,
            &trim_edge_punctuation(tokens.get(head.modifier_token + 1..).unwrap_or_default()),
        )
        .is_none()
    {
        return Ok(None);
    }

    let Ok((_prefix_condition, subject_start)) = parse_anthem_prefix_condition(tokens, get_idx)
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    // A comma-separated subtype/type list is one anthem subject, even when its
    // final member is introduced by "and" (for example, "Rabbits, Bats,
    // Birds, and Mice you control").  Prefer the exact whole-subject grammar
    // before considering the genuinely distributive multi-subject form.
    if matches!(
        anthem_grant_grammar::parse_exact_anthem_subject_grammar(&subject_tokens),
        Some(anthem_grant_grammar::AnthemSubjectGrammarMatch::Filter(_))
    ) {
        return Ok(None);
    }
    let Some(segments) = anthem_grant_grammar::parse_multi_subject_segments(&subject_tokens) else {
        return Ok(None);
    };

    let mut abilities = Vec::with_capacity(segments.len());
    for segment in segments {
        let mut clause_tokens = Vec::with_capacity(tokens.len());
        clause_tokens.extend_from_slice(&tokens[..subject_start]);
        clause_tokens.extend_from_slice(segment);
        clause_tokens.extend_from_slice(&tokens[get_idx..]);
        let adjusted_get_idx = subject_start + segment.len();
        let clause =
            match parse_anthem_clause(&clause_tokens, adjusted_get_idx, clause_tokens.len()) {
                Ok(clause) => clause,
                Err(_) => return Ok(None),
            };
        abilities.push(build_anthem_static_ability(&clause));
    }

    Ok(Some(abilities))
}

#[test]
fn subtype_list_anthem_remains_one_subject() {
    let tokens = crate::lexer::lex_line(
        "Other Rabbits, Bats, Birds, and Mice you control get +1/+1.",
        0,
    )
    .expect("lex subtype-list anthem");

    assert!(
        parse_multi_subject_anthem_line(&tokens)
            .expect("probe multi-subject anthem")
            .is_none(),
        "a subtype enumeration must not be split into independent anthem subjects"
    );
    assert!(
        parse_anthem_line(&tokens)
            .expect("parse subtype-list anthem")
            .is_some(),
        "the same line should remain accepted by the single-subject anthem parser"
    );
}

#[test]
fn shared_head_supertype_subtype_anthem_remains_one_typed_subject() {
    let tokens = crate::lexer::lex_line(
        "Other snow and Zombie creatures you control get +1/+1.",
        0,
    )
    .expect("lex shared-head anthem");

    assert!(
        parse_multi_subject_anthem_line(&tokens)
            .expect("probe multi-subject anthem")
            .is_none(),
        "a coordinated characteristic phrase must not become sibling anthems"
    );
    let ability = parse_anthem_line(&tokens)
        .expect("parse shared-head anthem")
        .expect("single-subject anthem should match");
    let crate::static_abilities::StaticAbilityPayload::Anthem(anthem) = &ability.payload else {
        panic!("expected a typed anthem: {ability:#?}");
    };
    let filter = anthem
        .filter
        .as_ref()
        .expect("coordinated anthem should use an object filter");

    assert_eq!(filter.card_types, [crate::CardType::Creature]);
    assert_eq!(filter.controller, Some(crate::PlayerFilter::You));
    assert!(filter.other);
    assert_eq!(filter.any_of.len(), 2);
    assert!(filter.any_of.iter().any(|branch| {
        branch.supertypes == [crate::Supertype::Snow] && branch.card_types.is_empty()
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.subtypes == [crate::Subtype::Zombie] && branch.card_types.is_empty()
    }));
}

pub fn parse_has_base_power_toughness_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_base_power_toughness_shape(tokens) else {
        return Ok(None);
    };
    let attached_subject_filter = match shape.condition {
        anthem_grant_grammar::BasePowerToughnessConditionShape::Tokens(condition_tokens) => {
            infer_attached_subject_filter_from_condition_tokens(condition_tokens)
        }
        anthem_grant_grammar::BasePowerToughnessConditionShape::None
        | anthem_grant_grammar::BasePowerToughnessConditionShape::YourTurn => None,
    };
    let subject = parse_anthem_subject_with_attached_fallback(
        shape.subject_tokens,
        attached_subject_filter.as_ref(),
    )?;
    let filter = anthem_subject_filter(&subject);

    let base = StaticAbility::set_base_power_toughness(filter, shape.power, shape.toughness);
    let condition = match shape.condition {
        anthem_grant_grammar::BasePowerToughnessConditionShape::None => return Ok(Some(base)),
        anthem_grant_grammar::BasePowerToughnessConditionShape::Tokens(condition_tokens) => {
            parse_static_condition_clause(condition_tokens)?
        }
        anthem_grant_grammar::BasePowerToughnessConditionShape::YourTurn => {
            crate::ConditionExpr::YourTurn
        }
    };
    let condition = bind_attachment_condition_to_subject(condition, &subject);
    #[cfg(not(feature = "serialization"))]
    {
        Ok(Some(base.with_condition(condition)))
    }
    #[cfg(feature = "serialization")]
    {
        Ok(base.with_condition(condition))
    }
}

pub fn parse_has_base_power_toughness_and_type_color_addition_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_base_power_toughness_type_addition_shape(tokens)
    else {
        return Ok(None);
    };
    let Some(additions) = parse_type_color_addition_clause(shape.addition_tokens)? else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let set_base = StaticAbility::set_base_power_toughness(
        anthem_subject_filter(&subject),
        shape.power,
        shape.toughness,
    );
    let clause = fixed_anthem_clause(subject, 0, 0, None);
    let mut compiled = vec![set_base.into()];
    push_type_color_additions_for_anthem_subject(&mut compiled, &clause, additions);
    Ok(Some(compiled))
}

pub fn parse_isnt_creature_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let display = crate::lexer::token_word_refs(tokens).join(" ");
    let shape = match anthem_grant_grammar::parse_isnt_creature_shape(tokens) {
        Ok(Some(shape)) => shape,
        Ok(None) => return Ok(None),
        Err(anthem_grant_grammar::IsntCreatureShapeError::MissingLeadingCondition) => {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{display}')"
            )));
        }
        Err(anthem_grant_grammar::IsntCreatureShapeError::MissingUnlessCondition) => {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'unless' clause (clause: '{display}')"
            )));
        }
    };
    let mut condition = shape
        .leading_condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    if let Some(unless_tokens) = shape.unless_condition_tokens {
        let unless_condition =
            crate::ConditionExpr::Not(Box::new(parse_static_condition_clause(unless_tokens)?));
        condition = Some(match condition {
            Some(existing) => {
                crate::ConditionExpr::And(Box::new(existing), Box::new(unless_condition))
            }
            None => unless_condition,
        });
    }

    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    let mut remove =
        crate::static_abilities::RemoveCardTypesForFilter::new(filter, vec![CardType::Creature]);
    if let Some(condition) = condition {
        remove = remove.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(remove)))
}

pub fn parse_has_base_power_toughness_and_granted_keywords_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_base_power_toughness_grant_shape(tokens) else {
        return Ok(None);
    };
    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, shape.has_token) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..shape.has_token]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_facts = anthem_grant_grammar::persistent_anthem_subject_facts(&subject_tokens);
    if !subject_facts.accepted {
        return Ok(None);
    }

    let attached_subject =
        anthem_grant_grammar::parse_granted_subject_facts(&subject_tokens).attached_subject;
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(shape.ability_tokens, &clause_words, attached_subject)?
    else {
        return Ok(None);
    };

    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut compiled = Vec::new();
    match &subject {
        AnthemSubjectAst::Source => {
            let source_filter = if subject_facts.is_this_creature {
                ObjectFilter::source().with_type(CardType::Creature)
            } else {
                ObjectFilter::source()
            };
            let set_base = StaticAbility::set_base_power_toughness(
                source_filter,
                shape.power,
                shape.toughness,
            )
            .into();
            compiled.push(if let Some(condition) = condition.clone() {
                StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(set_base),
                    condition,
                }
            } else {
                set_base
            });
        }
        AnthemSubjectAst::Filter(filter) => {
            let set_base = StaticAbility::set_base_power_toughness(
                filter.clone(),
                shape.power,
                shape.toughness,
            )
            .into();
            compiled.push(if let Some(condition) = condition.clone() {
                StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(set_base),
                    condition,
                }
            } else {
                set_base
            });
        }
    }

    compiled.extend(lower_granted_tail_for_anthem_subject(
        &subject,
        &condition,
        granted_tail,
    ));

    Ok(Some(compiled))
}

pub fn parse_has_base_power_and_granted_ability_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(shape) = anthem_grant_grammar::parse_base_power_grant_shape(tokens) else {
        return Ok(None);
    };
    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, shape.has_token) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..shape.has_token]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_facts = anthem_grant_grammar::persistent_anthem_subject_facts(&subject_tokens);
    if !subject_facts.accepted {
        return Ok(None);
    }
    let attached_subject =
        anthem_grant_grammar::parse_granted_subject_facts(&subject_tokens).attached_subject;
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(shape.ability_tokens, &clause_words, attached_subject)?
    else {
        return Ok(None);
    };
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let filter = match &subject {
        AnthemSubjectAst::Source if subject_facts.is_this_creature => {
            ObjectFilter::source().with_type(CardType::Creature)
        }
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter.clone(),
    };
    let set_base: StaticAbilityAst = StaticAbility::set_base_power(filter, shape.power).into();
    let mut compiled = vec![if let Some(condition) = condition.clone() {
        StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(set_base),
            condition,
        }
    } else {
        set_base
    }];
    compiled.extend(lower_granted_tail_for_anthem_subject(
        &subject,
        &condition,
        granted_tail,
    ));
    Ok(Some(compiled))
}

pub fn parse_filter_has_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let mut deferred_error: Option<CardTextError> = None;
    for candidate in anthem_grant_grammar::parse_granted_ability_candidates(tokens) {
        let has_idx = candidate.has_token;
        let (mut condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
            Ok(parsed) => parsed,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
        if subject_tokens.is_empty() {
            continue;
        }
        if let Some(split) = anthem_grant_grammar::split_type_addition_subject(&subject_tokens)
            && let Some(additions) = parse_type_color_addition_clause(split.addition_tokens)? {
                let base_subject = match parse_anthem_subject(split.base_subject_tokens) {
                    Ok(subject) => subject,
                    Err(err) => {
                        deferred_error.get_or_insert(err);
                        continue;
                    }
                };
                let AnthemSubjectAst::Filter(filter) = &base_subject else {
                    continue;
                };
                let ability_tokens = trim_commas(&tokens[has_idx + 1..]);
                let attached_subject =
                    anthem_grant_grammar::parse_granted_subject_facts(split.base_subject_tokens)
                        .attached_subject;
                let granted_tail = match parse_heterogeneous_granted_tail(
                    &ability_tokens,
                    &clause_words,
                    attached_subject,
                ) {
                    Ok(Some(tail)) => tail,
                    Ok(None) => continue,
                    Err(err) => {
                        deferred_error.get_or_insert(err);
                        continue;
                    }
                };
                let mut result = Vec::new();
                let with_shared_condition = |ability: StaticAbility| match condition.clone() {
                    Some(condition) => ability.with_condition(condition),
                    None => ability,
                };
                if !additions.set_colors.is_empty() {
                    result.push(
                        with_shared_condition(StaticAbility::set_colors(
                            filter.clone(),
                            additions.set_colors,
                        ))
                        .into(),
                    );
                }
                if !additions.added_colors.is_empty() {
                    result.push(
                        with_shared_condition(StaticAbility::add_colors(
                            filter.clone(),
                            additions.added_colors,
                        ))
                        .into(),
                    );
                }
                if !additions.card_types.is_empty() {
                    result.push(
                        with_shared_condition(StaticAbility::add_card_types(
                            filter.clone(),
                            additions.card_types,
                        ))
                        .into(),
                    );
                }
                if !additions.subtypes.is_empty() {
                    result.push(
                        with_shared_condition(StaticAbility::add_subtypes(
                            filter.clone(),
                            additions.subtypes,
                        ))
                        .into(),
                    );
                }
                result.extend(lower_granted_tail_for_anthem_subject(
                    &base_subject,
                    &condition,
                    granted_tail,
                ));
                if !result.is_empty() {
                    return Ok(Some(result));
                }
            }
        let subject_facts = anthem_grant_grammar::parse_granted_subject_facts(&subject_tokens);
        let prefix_attached_subject_filter =
            anthem_grant_grammar::parse_prefix_condition_shape(tokens, has_idx)
                .filter(|shape| {
                    shape.kind == anthem_grant_grammar::AnthemPrefixConditionKind::AsLongAs
                        && subject_start > shape.prefix_end
                })
                .and_then(|shape| {
                    infer_attached_subject_filter_from_condition_tokens(
                        &tokens[shape.prefix_end..subject_start],
                    )
                });
        let attached_subject_filter =
            infer_attached_subject_filter_from_condition_expr(condition.as_ref())
                .or(prefix_attached_subject_filter);
        if subject_facts.rejected_action
            || subject_facts.has_may
            || (subject_facts.unbound_pronoun && condition.is_none())
        {
            continue;
        }

        let mut ability_tokens = trim_commas(&tokens[has_idx + 1..]);
        let mut condition_failed = false;
        let mut trailing_if_surface = false;
        for kind in [
            anthem_grant_grammar::GrantedAbilityConditionKind::AsLongAs,
            anthem_grant_grammar::GrantedAbilityConditionKind::If,
        ] {
            let Some(split) =
                anthem_grant_grammar::split_granted_ability_condition(&ability_tokens, kind)
            else {
                continue;
            };
            let parsed_condition = match parse_static_condition_clause(split.condition_tokens) {
                Ok(condition) => condition,
                Err(err) => {
                    deferred_error.get_or_insert(err);
                    condition_failed = true;
                    break;
                }
            };
            condition = Some(match condition {
                Some(existing) => {
                    crate::ConditionExpr::And(Box::new(existing), Box::new(parsed_condition))
                }
                None => parsed_condition,
            });
            if kind == anthem_grant_grammar::GrantedAbilityConditionKind::If {
                trailing_if_surface = true;
            }
            ability_tokens = split.ability_tokens.to_vec();
        }
        if condition_failed {
            continue;
        }

        // This broad grant family is ordered before the narrower keyword
        // grant rule in the static-line registry. Preserve the same authored
        // trailing timing condition here so `this creature has first strike
        // during your turn` cannot be accepted as an unconditional grant.
        if let Some(keyword_prefix) =
            anthem_grant_grammar::split_trailing_during_your_turn_clause(&ability_tokens)
        {
            let timing = crate::ConditionExpr::ActivationTiming(
                crate::ability::ActivationTiming::DuringYourTurn,
            );
            condition = Some(match condition {
                Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(timing)),
                None => timing,
            });
            ability_tokens = keyword_prefix.to_vec();
        }

        if let Some(keyword) = anthem_grant_grammar::parse_special_granted_keyword(&ability_tokens)
        {
            let parsed = match keyword {
                anthem_grant_grammar::SpecialGrantedKeyword::Blitz => {
                    granted_blitz_abilities_from_subject(&subject_tokens, condition.clone())
                }
                anthem_grant_grammar::SpecialGrantedKeyword::Emerge => {
                    granted_emerge_abilities_from_subject(&subject_tokens, condition.clone())
                }
                anthem_grant_grammar::SpecialGrantedKeyword::Scavenge => {
                    granted_scavenge_abilities_from_subject(&subject_tokens, condition.clone())
                }
            };
            match parsed {
                Ok(Some(grants)) => return Ok(Some(grants)),
                Ok(None) => continue,
                Err(err) => {
                    deferred_error.get_or_insert(err);
                    continue;
                }
            }
        }
        let granted_tail = match parse_heterogeneous_granted_tail(
            &ability_tokens,
            &clause_words,
            subject_facts.attached_subject,
        ) {
            Ok(Some(tail)) => tail,
            Ok(None) => continue,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        // Union subjects ("you and this creature have hexproof", "Dion and
        // other Knights you control have flying") compile one ability per
        // half; the ordinary subject parse below would lossily recover only
        // the best-scoring suffix filter and drop the left half.
        if let Some((left_subject, right_subject)) = split_union_grant_subjects(&subject_tokens) {
            let mut granted = Vec::new();
            let mut supported = true;
            for half in [&left_subject, &right_subject] {
                match half {
                    UnionGrantSubject::PlayerYou => {
                        // The player half only models keywords with a
                        // player-level restriction form ("You have hexproof").
                        if matches!(
                            parse_ability_line(&ability_tokens).as_deref(),
                            Some([KeywordAction::Hexproof])
                        ) {
                            granted.push(conditional_static_ability(
                                player_you_hexproof_static(),
                                condition.clone(),
                            ));
                        } else {
                            supported = false;
                            break;
                        }
                    }
                    UnionGrantSubject::Source => {
                        granted.extend(lower_granted_tail_for_anthem_subject(
                            &AnthemSubjectAst::Source,
                            &condition,
                            granted_tail.clone(),
                        ));
                    }
                    UnionGrantSubject::Filter(filter) => {
                        granted.extend(lower_granted_tail_for_anthem_subject(
                            &AnthemSubjectAst::Filter(filter.clone()),
                            &condition,
                            granted_tail.clone(),
                        ));
                    }
                }
            }
            if supported && !granted.is_empty() {
                return Ok(Some(granted));
            }
        }
        let mut subject = match parse_anthem_subject_with_attached_fallback(
            &subject_tokens,
            attached_subject_filter.as_ref(),
        ) {
            Ok(subject) => subject,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        if trailing_if_surface
            && let Some(crate::ConditionExpr::ValueComparison {
                left:
                    crate::effect::Value::ManaFromSourceSpentToCastThisSpell {
                        source_filter,
                        include_source_noun: false,
                        ..
                    },
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(1),
            }) = condition.as_ref()
            && let AnthemSubjectAst::Filter(filter) = &mut subject
            && filter.mana_from_source_spent_to_cast.is_none()
        {
            filter.mana_from_source_spent_to_cast = Some(Box::new(source_filter.clone()));
            filter.set_mana_source_spent_trailing_if_surface(true);
            condition = None;
        }
        let condition =
            condition.map(|condition| bind_attachment_condition_to_subject(condition, &subject));
        let granted = lower_granted_tail_for_anthem_subject(&subject, &condition, granted_tail)
            .into_iter()
            .map(|ability| with_leading_set_quantifier_surface(ability, &subject_tokens))
            .collect::<Vec<_>>();
        if granted.is_empty() {
            continue;
        }
        return Ok(Some(granted));
    }

    if let Some(err) = deferred_error {
        return Err(err);
    }
    Ok(None)
}

#[test]
fn attached_object_anthem_subject_uses_tagged_constraints() {
    let enchanted = AnthemSubjectAst::Filter(ObjectFilter::tagged("enchanted"));
    assert!(attached_object_anthem_subject_filter(&enchanted).is_some());

    let equipped = AnthemSubjectAst::Filter(ObjectFilter::tagged("equipped"));
    assert!(attached_object_anthem_subject_filter(&equipped).is_some());

    let creature = AnthemSubjectAst::Filter(ObjectFilter::creature());
    assert!(attached_object_anthem_subject_filter(&creature).is_none());
}

#[test]
fn leading_attached_characteristic_condition_binds_pronoun_grant_to_the_host() {
    let tokens = crate::lexer::lex_line(
        "As long as equipped creature is a Human, it has lifelink.",
        0,
    )
    .expect("lex conditional attached grant");
    let abilities = parse_filter_has_granted_ability_line(&tokens)
        .expect("parse conditional attached grant")
        .expect("conditional attached grant should match");
    let [
        StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Lifelink,
            condition: Some(condition),
        },
    ] = abilities.as_slice()
    else {
        panic!("expected one conditional lifelink grant: {abilities:#?}");
    };

    assert!(
        !filter.source,
        "the Equipment itself must not gain lifelink"
    );
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "equipped"
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(
        matches!(condition, crate::ConditionExpr::AttachedToSourceMatches(host)
            if host.subtypes == [crate::Subtype::Human]),
        "the Human requirement remains an executable attachment condition: {condition:#?}"
    );

    let near_miss = crate::lexer::lex_line(
        "As long as you control a Human, this Equipment has lifelink.",
        0,
    )
    .expect("lex source grant near miss");
    let near_miss = parse_filter_has_granted_ability_line(&near_miss)
        .expect("parse source grant near miss")
        .expect("source grant near miss should match");
    assert!(matches!(
        near_miss.as_slice(),
        [StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Lifelink,
            ..
        }] if filter.source
    ));
}

#[test]
fn persistent_anthem_loss_tail_removes_keyword_from_the_affected_set_directly() {
    for text in [
        "Equipped creature gets +10/+10 and loses flying.",
        "Enchanted creature gets -6/-0 and loses flying.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0)
            .expect("attached anthem with a keyword-loss tail should lex");
        let parsed = parse_static_ability_ast_line_lexed(&tokens)
            .expect("attached anthem loss should not error")
            .expect("attached anthem loss should route through the static parser");
        assert_eq!(parsed.len(), 2, "{text}: {parsed:#?}");
        assert!(
            matches!(
                &parsed[1],
                StaticAbilityAst::RemoveKeywordAction {
                    filter,
                    action: KeywordAction::Flying,
                    mode: ironsmith_core::AbilityLossMode::Lose,
                } if filter.tagged_constraints.iter().any(|constraint| {
                    matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    ) && matches!(constraint.tag.as_str(), "equipped" | "enchanted")
                })
            ),
            "keyword loss must target the attached object directly, not grant it a nested removal ability: {text}: {parsed:#?}"
        );
    }
}

#[test]
fn quoted_static_marker_grants_parse_for_filtered_subjects() {
    let tokens = crate::lexer::lex_line(
        "Commander creatures you own have \"Room abilities of dungeons you own trigger an additional time.\"",
        0,
    )
    .expect("lex quoted static grant");
    let candidates = anthem_grant_grammar::parse_granted_ability_candidates(&tokens);
    assert_eq!(candidates.len(), 1, "expected one have-tail candidate");
    let has_token = candidates[0].has_token;
    let tail = trim_commas(&tokens[has_token + 1..]);
    let parsed_tail = parse_heterogeneous_granted_tail(
        &tail,
        &crate::lexer::token_word_refs(&tokens),
        false,
    )
    .expect("parse heterogeneous quoted tail");
    assert!(parsed_tail.is_some(), "expected quoted marker tail");
    let abilities = parse_filter_has_granted_ability_line(&tokens)
        .expect("parse quoted static grant")
        .expect("quoted marker grant should be recognized");
    assert!(abilities.iter().any(|ability| matches!(
        ability,
        StaticAbilityAst::GrantStaticAbility { ability, .. }
            if matches!(ability.as_ref(), StaticAbilityAst::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::DungeonRoomTriggerDuplication)
    )));
}

#[test]
fn quoted_filtered_subject_cost_tax_routes_as_a_granted_static_ability() {
    let tokens = crate::lexer::lex_line(
        "Creatures you control with flying have \"Spells your opponents cast that target this creature cost {2} more to cast.\"",
        0,
    )
    .expect("lex quoted cost-tax grant");
    let routed = parse_static_ability_ast_line_lexed(&tokens)
        .expect("route quoted cost-tax grant")
        .expect("quoted cost-tax grant should be recognized");
    let [
        StaticAbilityAst::GrantStaticAbility {
            filter,
            ability,
            condition,
        },
    ] = routed.as_slice()
    else {
        panic!("expected one filtered static-ability grant: {routed:#?}");
    };

    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert_eq!(
        filter.static_abilities,
        [crate::static_abilities::StaticAbilityId::Flying]
    );
    assert!(condition.is_none());

    let StaticAbilityAst::Static(ability) = ability.as_ref() else {
        panic!("expected a typed static cost increase: {ability:#?}");
    };
    let ironsmith_core::StaticAbilityPayload::CostIncrease(increase) = &ability.payload else {
        panic!("expected a generic-mana cost increase: {ability:#?}");
    };
    assert_eq!(increase.amount, Value::Fixed(2));
    assert_eq!(increase.filter.cast_by, Some(PlayerFilter::Opponent));
    let target = increase
        .filter
        .targets_object
        .as_deref()
        .expect("the quoted tax should retain its affected-object target");
    assert!(
        target.source,
        "\"this creature\" must refer to each object receiving the ability"
    );
}

#[test]
fn conditional_attached_quoted_equipment_grant_uses_nested_subject() {
    let tokens = crate::lexer::lex_line(
        "As long as enchanted permanent is an Equipment, it has \"Equipped creature gets +1/+1 and has trample.\"",
        0,
    )
    .expect("lex conditional attached grant");
    let direct = parse_filter_has_granted_ability_line(&tokens)
        .expect("direct grant parser")
        .expect("typed conditional grant");
    let routed = parse_static_ability_ast_line_lexed(&tokens).expect("static router");
    assert_eq!(routed, Some(direct));
}

#[test]
fn type_addition_subjects_preserve_trailing_quoted_and_keyword_grants() {
    for text in [
        "Clues you control are Equipment in addition to their other types and have \"Equipped creature gets +2/+0\" and equip {2}.",
        "Treasures you control are Equipment in addition to their other types and have \"Equipped creature gets +2/+0,\" equip Pirate {1}, and equip {3}.",
    ] {
        let tokens =
            crate::lexer::lex_line(text, 0).expect("lex type-addition grant");
        let abilities = parse_filter_has_granted_ability_line(&tokens)
            .expect("parse type-addition grant")
            .expect("type-addition grant should be recognized");
        assert!(
            abilities.len() >= 3,
            "expected type and both grants: {text}"
        );
    }
}

#[test]
fn keyword_and_attack_requirements_before_anthem_share_the_clean_subject_filter() {
    let tokens = crate::lexer::lex_line(
        "Each Skeleton you control has trample, attacks each combat if able, and gets +X/+0, where X is the number of other Skeletons you control.",
        0,
    )
    .expect("lex shared-subject keyword, attack, and anthem line");
    let abilities = parse_anthem_and_keyword_line(&tokens)
        .expect("parse shared-subject static line")
        .expect("shared-subject line should be recognized");

    let [
        StaticAbilityAst::GrantKeywordAction {
            filter: trample_filter,
            action: KeywordAction::Trample,
            ..
        },
        StaticAbilityAst::GrantStaticAbility {
            filter: attack_filter,
            ability: attack_ability,
            ..
        },
        StaticAbilityAst::Static(anthem),
    ] = abilities.as_slice()
    else {
        panic!("expected trample, attack requirement, then anthem: {abilities:#?}");
    };
    let crate::static_abilities::StaticAbilityPayload::Anthem(anthem) = &anthem.payload else {
        panic!("expected terminal anthem: {anthem:#?}");
    };
    let anthem_filter = anthem.filter.as_ref().expect("filtered anthem subject");
    assert_eq!(trample_filter, attack_filter);
    assert_eq!(trample_filter, anthem_filter);
    assert!(
        trample_filter.static_abilities.is_empty(),
        "{trample_filter:#?}"
    );
    assert!(matches!(
        attack_ability.as_ref(),
        StaticAbilityAst::Static(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::MustAttack
    ));
}

#[test]
fn player_counter_conditions_lower_for_conditional_anthems() {
    let tokens = crate::lexer::lex_line(
        "As long as an opponent has three or more poison counters, enchanted creature gets an additional +1/+0 and has first strike.",
        0,
    )
    .expect("lex conditional anthem");
    let abilities = parse_anthem_and_keyword_line(&tokens)
        .expect("parse conditional anthem")
        .expect("conditional anthem should be recognized");
    assert_eq!(abilities.len(), 2);
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("PlayerCounters"), "{debug}");
    assert!(debug.contains("Poison"), "{debug}");

    let routed = crate::clause_support::parse_static_ability_ast_line_lexed(&tokens)
        .expect("route conditional anthem")
        .expect("typed conditional anthem should reach static-line lowering");
    assert_eq!(routed.len(), 2, "{routed:#?}");
}

#[test]
fn generic_leading_as_long_as_conditions_every_thunderfoot_sibling() {
    let tokens = crate::lexer::lex_line(
        "As long as you control your commander, this creature gets +2/+2 and other creatures you control get +2/+2 and have trample.",
        0,
    )
    .expect("lex lieutenant static line");

    let baseline = parse_static_ability_ast_line_lexed_single_without_leading_condition(&tokens)
        .expect("parse existing lieutenant shape")
        .expect("existing lieutenant shape should parse");
    assert_eq!(baseline.len(), 3, "{baseline:#?}");
    assert_eq!(
        baseline
            .iter()
            .map(static_ability_ast_has_explicit_condition)
            .collect::<Vec<_>>(),
        [true, false, false],
        "the existing compound parser should expose the missing sibling conditions"
    );

    let routed = parse_static_ability_ast_line_lexed(&tokens)
        .expect("route lieutenant static line")
        .expect("lieutenant static line should parse");
    assert_eq!(routed.len(), 3, "{routed:#?}");
    assert!(
        routed.iter().all(static_ability_ast_has_explicit_condition),
        "every sibling must retain the commander condition: {routed:#?}"
    );
    let debug = format!("{routed:#?}");
    assert_eq!(
        debug.matches("you control your commander").count(),
        3,
        "{debug}"
    );
}

#[test]
fn fixed_not_your_turn_prefix_conditions_source_type_identity() {
    let tokens = crate::lexer::lex_line(
        "During turns other than yours, this Vehicle is an artifact creature.",
        0,
    )
    .expect("lex fixed turn condition");

    let (routed, loss) =
        crate::parse_loss::capture(|| parse_static_ability_ast_line_lexed(&tokens));
    assert!(!loss.is_lossy(), "unexpected parse loss: {loss:#?}");
    let routed = routed
        .expect("route fixed turn condition")
        .expect("fixed turn condition should parse");
    assert_eq!(routed.len(), 1, "{routed:#?}");
    assert!(
        routed.iter().all(static_ability_ast_has_explicit_condition),
        "type identity must retain the leading turn condition: {routed:#?}"
    );
    let debug = format!("{routed:#?}");
    assert!(debug.contains("SetCardTypes"), "{debug}");
    assert!(
        debug.contains("Not(") && debug.contains("YourTurn"),
        "{debug}"
    );
    assert!(!debug.contains("other: true"), "{debug}");
}

#[test]
fn generic_leading_if_conditions_source_spell_keyword() {
    let tokens = crate::lexer::lex_line(
        "If this spell was kicked, it has split second.",
        0,
    )
    .expect("lex kicked static keyword");

    let (routed, loss) =
        crate::parse_loss::capture(|| parse_static_ability_ast_line_lexed(&tokens));
    assert!(!loss.is_lossy(), "unexpected parse loss: {loss:#?}");
    let routed = routed
        .expect("route kicked static keyword")
        .expect("kicked static keyword should parse");
    assert_eq!(routed.len(), 1, "{routed:#?}");
    assert!(
        routed.iter().all(static_ability_ast_has_explicit_condition),
        "split second must retain its kicked condition: {routed:#?}"
    );
    let debug = format!("{routed:#?}");
    assert!(debug.contains("SplitSecond"), "{debug}");
    assert!(debug.contains("ThisSpellWasKicked"), "{debug}");
    assert!(!debug.contains("other: true"), "{debug}");
}

#[test]
fn contracted_source_animation_and_keyword_share_the_threshold_condition() {
    let tokens = crate::lexer::lex_line(
        "As long as this artifact has eight or more +1/+1 counters on it, it's a 0/0 creature in addition to its other types and it has annihilator 2.",
        0,
    )
    .expect("lex contracted source animation");

    let (routed, loss) =
        crate::parse_loss::capture(|| parse_static_ability_ast_line_lexed(&tokens));
    assert!(!loss.is_lossy(), "unexpected parse loss: {loss:#?}");
    let routed = routed
        .expect("route contracted source animation")
        .expect("contracted source animation should parse");
    assert_eq!(routed.len(), 3, "{routed:#?}");
    assert!(
        routed.iter().all(static_ability_ast_has_explicit_condition),
        "animation and keyword siblings must share the counter condition: {routed:#?}"
    );
    let debug = format!("{routed:#?}");
    assert!(debug.contains("AddCardTypes"), "{debug}");
    assert!(debug.contains("SetBasePowerToughness"), "{debug}");
    assert!(debug.contains("Annihilator"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    assert_eq!(
        debug.matches("CountersOnSource(").count(),
        3,
        "the threshold must count counters, not permanents that merely have one: {debug}"
    );
}

#[test]
fn rejected_static_rules_do_not_leak_suffix_loss_into_delirium_line() {
    let tokens = crate::lexer::lex_line(
        "Delirium — As long as there are four or more card types among cards in your graveyard, this creature gets +2/+2, has flying, and attacks each combat if able.",
        0,
    )
    .expect("lex delirium line");

    let (routed, loss) =
        crate::parse_loss::capture(|| parse_static_ability_ast_line_lexed(&tokens));
    assert!(!loss.is_lossy(), "unexpected parse loss: {loss:#?}");
    let routed = routed
        .expect("route delirium line")
        .expect("delirium line should parse");
    assert_eq!(routed.len(), 3, "{routed:#?}");
    assert!(
        routed.iter().all(static_ability_ast_has_explicit_condition),
        "all delirium siblings must retain the threshold condition: {routed:#?}"
    );
}

#[test]
fn generic_leading_as_long_as_wraps_explicit_subject_static_families() {
    for (text, condition_fragments) in [
        (
            "As long as it's your turn and you control an Army, this is an artifact creature.",
            &["YourTurn", "Army"][..],
        ),
        (
            "As long as you control eight or more permanents named Phoenix Fleet Airship, this Vehicle is an artifact creature.",
            &["phoenix fleet airship", "GreaterThanOrEqual"][..],
        ),
        (
            "As long as there are four or more quest counters on this enchantment, untap all creatures you control during each other player's untap step.",
            &["Quest", "GreaterThanOrEqual"][..],
        ),
        (
            "As long as this creature has a conqueror counter on it, nonbasic lands are Mountains.",
            &["conqueror", "GreaterThanOrEqual"][..],
        ),
    ] {
        let tokens = crate::lexer::lex_line(text, 0)
            .unwrap_or_else(|err| panic!("lex explicit-subject condition '{text}': {err}"));
        let routed = parse_static_ability_ast_line_lexed(&tokens)
            .unwrap_or_else(|err| panic!("route explicit-subject condition '{text}': {err}"))
            .unwrap_or_else(|| panic!("explicit-subject condition should parse: {text}"));
        assert!(
            !routed.is_empty() && routed.iter().all(static_ability_ast_has_explicit_condition),
            "every emitted static sibling must be conditioned for '{text}': {routed:#?}"
        );
        let debug = format!("{routed:#?}");
        for fragment in condition_fragments {
            assert!(
                debug.contains(fragment),
                "condition fragment '{fragment}' was lost for '{text}': {debug}"
            );
        }
    }
}

#[test]
fn generic_leading_as_long_as_preserves_specialized_condition_controls() {
    for text in [
        "As long as it's your turn, this creature has first strike.",
        "As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.",
        "As long as this enchantment has seven or more quest counters on it, creatures you control get +5/+5.",
        "As long as enchanted creature is black, it gets +1/+1 and has wither.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0)
            .unwrap_or_else(|err| panic!("lex specialized condition '{text}': {err}"));
        let baseline =
            parse_static_ability_ast_line_lexed_single_without_leading_condition(&tokens)
                .unwrap_or_else(|err| panic!("parse specialized condition '{text}': {err}"));
        let routed = parse_static_ability_ast_line_lexed_single(&tokens)
            .unwrap_or_else(|err| panic!("route specialized condition '{text}': {err}"));
        assert_eq!(routed, baseline, "generic prefix must not steal '{text}'");
    }

    for text in [
        "This creature has reach as long as it has a +1/+1 counter on it.",
        "When this creature enters, target Forest becomes a 4/5 green Treefolk creature for as long as this creature remains on the battlefield.",
        "Gain control of target artifact for as long as you control this creature.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0)
            .unwrap_or_else(|err| panic!("lex non-prefix duration '{text}': {err}"));
        assert!(
            split_as_long_as_condition_prefix_lexed(&tokens).is_none(),
            "generic leading-condition path must ignore trailing/duration text: {text}"
        );
    }
}

#[test]
fn exact_one_control_condition_binds_that_creature_subject() {
    fn assert_bound_filter(filter: &ObjectFilter, condition: Option<&crate::ConditionExpr>) {
        assert_eq!(filter.zone, Some(Zone::Battlefield), "{filter:#?}");
        assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
        assert!(
            filter
                .tagged_constraints
                .iter()
                .all(|constraint| constraint.tag.as_str() != "__it__"),
            "exact-one antecedent must not retain an unresolved __it__: {filter:#?}"
        );
        let Some(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(counted_filter),
            comparison: crate::effect::Comparison::Equal(1),
            ..
        }) = condition
        else {
            panic!("expected an exact-one matching-filter condition: {condition:#?}");
        };
        assert_eq!(filter, counted_filter);
    }

    for (text, expected_abilities) in [
        (
            "As long as you control exactly one creature, that creature gets +2/+0 and has deathtouch and lifelink.",
            3,
        ),
        (
            "As long as you control exactly one creature, that creature gets +3/+1 and has lifelink.",
            2,
        ),
    ] {
        let tokens = crate::lexer::lex_line(text, 0)
            .expect("lex exact-one conditional anthem");
        let abilities = parse_anthem_and_keyword_line(&tokens)
            .expect("parse exact-one conditional anthem")
            .expect("exact-one conditional anthem should be recognized");
        assert_eq!(abilities.len(), expected_abilities, "{abilities:#?}");

        for ability in &abilities {
            match ability {
                StaticAbilityAst::Static(static_ability) => {
                    let crate::static_abilities::StaticAbilityPayload::Anthem(anthem) =
                        &static_ability.payload
                    else {
                        panic!("expected anthem payload: {static_ability:#?}");
                    };
                    assert_bound_filter(
                        anthem.filter.as_ref().expect("filtered anthem subject"),
                        anthem.condition.as_ref(),
                    );
                }
                StaticAbilityAst::GrantKeywordAction {
                    filter, condition, ..
                } => assert_bound_filter(filter, condition.as_ref()),
                other => panic!("unexpected exact-one static ability: {other:#?}"),
            }
        }
    }
}

#[test]
fn conditional_anthems_preserve_no_defender_attack_permission() {
    let tail_tokens = crate::lexer::lex_line(
        "trample and can attack as though it didn't have defender.",
        0,
    )
    .expect("lex compound grant tail");
    let tail = parse_heterogeneous_granted_tail(&tail_tokens, &[], false)
        .expect("parse compound grant tail")
        .expect("compound grant tail should be recognized");
    assert_eq!(tail.granted_keyword_actions.len(), 1, "{tail:#?}");
    assert_eq!(tail.granted_static.len(), 1, "{tail:#?}");

    for text in [
        "As long as this creature is monstrous, it gets +2/+2 and can attack as though it didn't have defender.",
        "As long as you control three or more artifacts, this creature gets +2/+2 and can attack as though it didn't have defender.",
    ] {
        let tokens =
            crate::lexer::lex_line(text, 0).expect("lex no-defender anthem");
        let abilities = parse_anthem_and_no_defender_line(&tokens)
            .expect("parse no-defender anthem")
            .expect("no-defender anthem should be recognized");
        assert_eq!(abilities.len(), 2, "{abilities:#?}");
        assert!(
            format!("{abilities:#?}").contains("CanAttackAsThoughNoDefender"),
            "{abilities:#?}"
        );
    }

    let tokens = crate::lexer::lex_line(
        "As long as this creature is monstrous, it has trample and can attack as though it didn't have defender.",
        0,
    )
    .expect("lex conditional grant");
    let abilities = parse_filter_has_granted_ability_line(&tokens)
        .expect("parse conditional grant")
        .expect("conditional no-defender grant should parse");
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("CanAttackAsThoughNoDefender"), "{debug}");
}

#[test]
fn broad_spell_grant_moves_trailing_mana_source_predicate_into_spell_filter() {
    let tokens = crate::lexer::lex_line(
        "Each spell you cast has split second if mana from an artifact was spent to cast it.",
        0,
    )
    .expect("lex mana-qualified spell grant");
    let abilities = parse_filter_has_granted_ability_line(&tokens)
        .expect("parse mana-qualified spell grant")
        .expect("broad grant parser should match");
    let filter = abilities.iter().find_map(|ability| match ability {
        StaticAbilityAst::GrantKeywordAction {
            filter,
            condition: None,
            ..
        }
        | StaticAbilityAst::GrantObjectAbility {
            filter,
            condition: None,
            ..
        } => Some(filter),
        StaticAbilityAst::Static(static_ability) => {
            let crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
                &static_ability.payload
            else {
                return None;
            };
            grant.condition.is_none().then_some(&grant.filter)
        }
        _ => None,
    });
    let filter = filter.expect("grant should retain its affected-spell filter");
    let mana_source = filter
        .mana_from_source_spent_to_cast
        .as_deref()
        .expect("trailing mana-source predicate should move into spell filter");
    assert_eq!(
        mana_source.card_types,
        vec![crate::types::CardType::Artifact]
    );
    assert!(filter.has_mana_source_spent_trailing_if_surface());
}

#[test]
fn carried_subject_type_addition_lowers_with_preceding_anthem() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature gets +1/+1 and has flying, haste, and \"{1}: This creature gets +1/+0 until end of turn.\" It's a Dragon in addition to its other types.",
        0,
    )
    .expect("lex compound static line");
    let abilities = parse_carried_subject_type_addition_line(&tokens)
        .expect("parse compound static line")
        .expect("compound type addition should be recognized");
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("Dragon"), "{debug}");
    assert!(abilities.len() >= 4, "{abilities:#?}");
}

#[test]
fn base_power_only_grant_preserves_toughness_and_quoted_ability() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature has base power 0 and has \"At the beginning of your upkeep, you lose 1 life unless you sacrifice this creature.\"",
        0,
    )
    .expect("lex base-power grant");
    let abilities = parse_has_base_power_and_granted_ability_static_line(&tokens)
        .expect("parse base-power grant")
        .expect("base-power grant should be recognized");
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("SetBasePower"), "{debug}");
    assert!(debug.contains("Triggered"), "{debug}");
    assert!(!debug.contains("SetBasePowerToughnessValue"), "{debug}");
}

#[test]
fn attached_conditional_anthem_continuations_lower_typed_conditions() {
    for text in [
        "Equipped creature gets +1/+1. If it's a Warrior, it gets +2/+1 instead.",
        "Enchanted creature gets +3/+0 as long as it's attacking. Otherwise, it gets -2/-1.",
    ] {
        let tokens =
            crate::lexer::lex_line(text, 0).expect("lex conditional continuation");
        let abilities = parse_conditional_anthem_replacement_line(&tokens)
            .expect("parse replacement continuation")
            .or_else(|| {
                parse_conditional_anthem_otherwise_line(&tokens)
                    .expect("parse otherwise continuation")
            })
            .expect("typed conditional continuation should be recognized");
        assert_eq!(abilities.len(), 2, "{abilities:#?}");
        let debug = format!("{abilities:#?}");
        assert!(debug.contains("AttachedToSourceMatches"), "{debug}");
        if text.contains(" instead") {
            assert!(debug.contains("replacement_surface"), "{debug}");
        }
    }

    let tokens = crate::lexer::lex_line(
        "Equipped creature gets +2/+0. It gets an additional +0/+2 and has first strike as long as an Equipment named Groom's Finery is attached to a creature you control.",
        0,
    )
    .expect("lex carried conditional grant");
    let abilities = parse_carried_conditional_anthem_grant_line(&tokens)
        .expect("parse carried conditional grant")
        .expect("carried conditional grant should be recognized");
    assert_eq!(abilities.len(), 3, "{abilities:#?}");
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("AttachmentCount"), "{debug}");
    let routed = parse_static_ability_ast_line_lexed(&tokens)
        .expect("route carried conditional grant")
        .expect("static line router should recognize carried conditional grant");
    assert_eq!(routed.len(), 3, "{routed:#?}");
}

#[test]
fn attachment_count_conditions_bind_typed_hosts_for_target_cards() {
    let cases = [
        (
            "Balan has double strike as long as two or more Equipment are attached to it.",
            &["AttachmentCount", "host: Source", "GreaterThanOrEqual"][..],
        ),
        (
            "Equipped creature has double strike as long as two or more Equipment are attached to it.",
            &[
                "AttachmentCount",
                "SourceAttachedObject",
                "GreaterThanOrEqual",
                "equipped",
            ][..],
        ),
        (
            "As long as another Aura is attached to enchanted creature, it has first strike and lifelink.",
            &[
                "AttachmentCount",
                "SourceAttachedObject",
                "other: true",
                "enchanted",
                "FirstStrike",
                "Lifelink",
            ][..],
        ),
        (
            "Equipped creature gets +2/+0. It gets an additional +0/+2 and has first strike as long as an Equipment named Groom's Finery is attached to a creature you control.",
            &[
                "AttachmentCount",
                "Matching(",
                "grooms finery",
                "GreaterThanOrEqual",
            ][..],
        ),
        (
            "Equipped creature gets +2/+0. It gets an additional +0/+2 and has deathtouch as long as an Equipment named Bride's Gown is attached to a creature you control.",
            &[
                "AttachmentCount",
                "Matching(",
                "brides gown",
                "GreaterThanOrEqual",
            ][..],
        ),
    ];

    for (text, expected_fragments) in cases {
        let tokens = crate::lexer::lex_line(text, 0)
            .unwrap_or_else(|error| panic!("lex attachment condition '{text}': {error}"));
        let abilities = parse_static_ability_ast_line_lexed(&tokens)
            .unwrap_or_else(|error| panic!("parse attachment condition '{text}': {error}"))
            .unwrap_or_else(|| panic!("attachment condition should route: {text}"));
        let debug = format!("{abilities:#?}");
        for fragment in expected_fragments {
            assert!(
                debug.contains(fragment),
                "missing '{fragment}' for '{text}': {debug}"
            );
        }
    }
}

#[test]
fn base_power_toughness_grants_accept_quoted_triggered_abilities() {
    {
        let text = "Enchanted creature has base power and toughness 8/8 and has \"Whenever this creature attacks, you may tap target creature with power 8 or less.\"";
        let tokens = crate::lexer::lex_line(text, 0)
            .expect("lex base characteristic grant");
        let abilities = parse_has_base_power_toughness_and_granted_keywords_static_line(&tokens)
            .expect("parse base characteristic grant")
            .unwrap_or_else(|| panic!("base characteristic grant should be recognized: {text}"));
        assert!(
            abilities.len() >= 2,
            "expected base value and grant: {text}"
        );
    }
}

#[test]
fn attached_combat_restrictions_preserve_quoted_ability_grants() {
    crate::util::with_source_reference_context(
        "Hold for Ransom",
        || {
            let text = "Enchanted creature can't attack or block and has \"{7}: Hold for Ransom's controller sacrifices it and draws a card. Activate only as a sorcery.\"";
            let tokens = crate::lexer::lex_line(text, 0)
                .expect("lex restriction and grant");
            let abilities = parse_static_ability_ast_line_lexed(&tokens)
                .expect("route restriction and grant")
                .expect("restriction and grant should be recognized");
            assert_eq!(abilities.len(), 2);
            let debug = format!("{abilities:#?}");
            assert!(debug.contains("AttachedObjectAbilityGrant"), "{debug}");
            assert!(
                debug.contains("source_surface: Some")
                    && debug.contains("FullName")
                    && debug.contains("Hold for Ransom"),
                "{debug}"
            );
            assert!(debug.contains("SorcerySpeed"), "{debug}");
        },
    );
}

#[test]
fn conditional_color_assignments_preserve_quoted_ability_grants() {
    let text = "As long as there are seven or more cards in your graveyard, this creature is white and has \"{T}: Destroy target black creature.\"";
    let tokens =
        crate::lexer::lex_line(text, 0).expect("lex color assignment and grant");
    let abilities = parse_subject_color_and_granted_ability_line(&tokens)
        .expect("parse color assignment and grant")
        .expect("color assignment and grant should be recognized");
    assert_eq!(abilities.len(), 2);
}

#[test]
fn static_turn_threshold_conditions_use_typed_anthem_grammar() {
    let drawn = crate::lexer::lex_line(
        "An opponent has drawn three or more cards this turn",
        0,
    )
    .expect("draw threshold should lex");
    assert_eq!(
        parse_cards_drawn_this_turn_static_condition(&drawn),
        Some(crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(3),
        })
    );

    let rolled =
        crate::lexer::lex_line("You have rolled two or more dice this turn", 0)
            .expect("dice threshold should lex");
    assert_eq!(
        parse_dice_rolled_this_turn_static_condition(&rolled),
        Some(crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::MaxDiceRolledThisTurn(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(2),
        })
    );
}

#[test]
fn static_condition_family_consumes_typed_condition_shapes() {
    let devotion = crate::lexer::lex_line(
        "Your devotion to white and blue is greater than or equal to three.",
        0,
    )
    .expect("devotion condition should lex");
    assert!(matches!(
        parse_static_condition_clause(&devotion).expect("devotion condition should parse"),
        crate::ConditionExpr::ValueComparison {
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(3),
            ..
        }
    ));

    let conjoined =
        crate::lexer::lex_line("It is your turn and you attacked this turn.", 0)
            .expect("conjoined condition should lex");
    let crate::ConditionExpr::And(left, right) =
        parse_static_condition_clause(&conjoined).expect("conjoined condition should parse")
    else {
        panic!("expected a typed conjunction");
    };
    assert_eq!(*left, crate::ConditionExpr::YourTurn);
    assert_eq!(*right, crate::ConditionExpr::AttackedThisTurn);

    let graveyard = crate::lexer::lex_line(
        "There are four or more card types among cards in your graveyard.",
        0,
    )
    .expect("graveyard condition should lex");
    assert_eq!(
        parse_static_condition_clause(&graveyard).expect("graveyard condition should parse"),
        crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
            player: PlayerFilter::You,
            count: 4,
        }
    );
}

#[test]
fn keyword_and_unblockable_tail_keeps_multiple_captured_keywords() {
    let tokens = crate::lexer::lex_line(
        "This creature has flying and vigilance and can't be blocked.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_subject_has_keywords_and_cant_be_blocked_line(&tokens)
        .expect("parser should not error")
        .expect("line should parse");

    assert!(matches!(
        parsed.as_slice(),
        [
            StaticAbilityAst::KeywordAction(KeywordAction::Flying),
            StaticAbilityAst::KeywordAction(KeywordAction::Vigilance),
            StaticAbilityAst::KeywordAction(KeywordAction::Unblockable),
        ]
    ));
}

#[test]
fn keyword_and_maximum_blocker_tail_share_the_attached_subject() {
    let tokens = crate::lexer::lex_line(
        "Equipped creature has trample and can't be blocked by more than one creature.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_subject_has_keywords_and_cant_be_blocked_by_more_than_line(&tokens)
        .expect("parser should not error")
        .expect("line should parse");

    assert!(matches!(
        parsed.as_slice(),
        [
            StaticAbilityAst::GrantKeywordAction {
                action: KeywordAction::Trample,
                filter: keyword_filter,
                ..
            },
            StaticAbilityAst::GrantStaticAbility {
                filter: restriction_filter,
                ability,
                ..
            },
        ] if keyword_filter == restriction_filter
            && matches!(
                ability.as_ref(),
                StaticAbilityAst::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::CantBeBlockedByMoreThan
            )
    ));
}

#[test]
fn unblockable_and_keyword_tail_keeps_the_reverse_authored_order() {
    let tokens = crate::lexer::lex_line(
        "Enchanted creature can't be blocked and has shroud.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_subject_cant_be_blocked_and_has_keywords_line(&tokens)
        .expect("parser should not error")
        .expect("line should parse");

    assert!(matches!(
        parsed.as_slice(),
        [
            StaticAbilityAst::GrantKeywordAction {
                action: KeywordAction::Unblockable,
                ..
            },
            StaticAbilityAst::GrantKeywordAction {
                action: KeywordAction::Shroud,
                ..
            },
        ]
    ));
}

#[test]
fn granted_escape_tail_captures_dynamic_exile_count() {
    let tokens = crate::lexer::lex_line(
        "The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_granted_escape_cost_tail_clause(&tokens).expect("escape tail should parse");
    let (count, used) =
        parse_number(parsed.exile_count_tokens).expect("captured count should parse");

    assert_eq!(count, 3);
    assert_eq!(used, parsed.exile_count_tokens.len());
}

#[test]
fn granted_miracle_tail_captures_dynamic_cost_reduction() {
    let tokens = crate::lexer::lex_line(
        "Its miracle cost is equal to its mana cost reduced by {4}.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_granted_miracle_cost_reduction_tail_clause(&tokens)
        .expect("miracle tail should parse");
    let (cost, used) =
        crate::util::leading_mana_cost_from_tokens(
            parsed.reduction_cost_tokens,
        )
        .expect("captured cost should parse");

    assert_eq!(cost.generic_mana_total(), 4);
    assert_eq!(used, parsed.reduction_cost_tokens.len());
}

#[test]
fn cant_be_blocked_by_more_than_clause_captures_subject_and_threshold() {
    let tokens = crate::lexer::lex_line(
        "Each creature you control with a +1/+1 counter on it can't be blocked by more than one creature.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_cant_be_blocked_by_more_than_clause(&tokens)
        .expect("max-blockers clause should parse");
    let subject_words =
        crate::lexer::parser_token_word_refs(parsed.subject_tokens);
    let (minimum_blockers, used) = parse_greater_than_or_equal_quantity_prefix(
        parsed.blocker_threshold_tokens,
        false,
        false,
        "test blocker threshold",
    )
    .expect("threshold should parse")
    .expect("threshold should be present");

    assert_eq!(
        subject_words.as_slice(),
        &[
            "each", "creature", "you", "control", "with", "a", "+1/+1", "counter", "on", "it"
        ]
    );
    assert_eq!(minimum_blockers, 2);
    assert_eq!(used, parsed.blocker_threshold_tokens.len());
}

#[test]
fn can_block_additional_creature_clause_captures_subject_and_count() {
    let tokens = crate::lexer::lex_line(
        "Each creature you control can block two additional creatures each combat.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_can_block_additional_creature_clause(&tokens)
        .expect("additional-blocker clause should parse");
    let subject_words =
        crate::lexer::parser_token_word_refs(parsed.subject_tokens);
    let (count, used) = parse_number(parsed.additional_count_tokens)
        .expect("captured additional blocker count should parse");

    assert_eq!(
        subject_words.as_slice(),
        &["each", "creature", "you", "control"]
    );
    assert_eq!(count, 2);
    assert_eq!(used, parsed.additional_count_tokens.len());
}

#[test]
fn landwalk_override_tail_uses_keyword_action_parser() {
    assert!(is_landwalk_ability_word("islandwalk"));
    assert!(is_landwalk_ability_word("forestwalk"));
    assert!(!is_landwalk_ability_word("planeswalk"));
    assert!(!is_landwalk_ability_word("walk"));
}
