pub(crate) fn parse_conditional_anthem_replacement_line(
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
        StaticAbility::new(build_anthem(&delta).with_replacement_surface(
            shape.replacement_power,
            shape.replacement_toughness,
        ))
        .into(),
    ]))
}

pub(crate) fn parse_conditional_anthem_otherwise_line(
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

pub(crate) fn parse_carried_conditional_anthem_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_carried_conditional_anthem_grant(tokens) else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let condition = crate::ConditionExpr::MatchingObjectAttachedToMatchingObject {
        attachment: shape.condition.attachment_filter,
        attached_to: shape.condition.attached_to_filter,
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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

pub(crate) fn parse_anthem_and_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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
        let Some(actions) = parse_ability_line(shape.keyword_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;

        let mut anthem_tokens = shape.subject_tokens.to_vec();
        anthem_tokens.extend_from_slice(shape.anthem_tail_tokens);
        let Some(anthem) = parse_anthem_line(&anthem_tokens)? else {
            return Ok(None);
        };
        let mut result = vec![StaticAbilityAst::from(anthem)];
        let grant_clause = ParsedAnthemClause {
            subject,
            power: AnthemValue::Fixed(0),
            toughness: AnthemValue::Fixed(0),
            condition: None,
            count_uses_where_x: false,
        };
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

    // "until end of turn" in the pump clause indicates a one-shot effect.
    // Ignore timing text that appears only inside a quoted granted ability.
    if line_shape.pre_grant_is_temporary {
        return Ok(None);
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
        if anthem_grant_grammar::parse_colon_tail_split(&ability_tokens).is_some() {
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
    match anthem_grant_grammar::split_anthem_keyword_trailing_condition(&ability_tokens) {
        Ok(Some(split)) => {
            trailing_condition = Some(parse_static_condition_clause(split.condition_tokens)?);
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
                "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }
    let mut trailing_type_color_addition: Option<TypeColorAdditionClause> = None;
    if let Some(split) = anthem_grant_grammar::split_anthem_keyword_and_is(&ability_tokens) {
        if let Some(additions) = parse_type_color_addition_clause(split.tail_tokens)? {
            trailing_type_color_addition = Some(additions);
            ability_tokens = split.head_tokens.to_vec();
        }
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
            {
                if let Some(split_and_idx) = colon.last_and_before_colon {
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
                if clause.condition.is_some() {
                    return Err(CardTextError::ParseError(format!(
                        "multiple anthem conditions are not supported (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                clause.condition = Some(condition);
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
        if clause.condition.is_some() {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        clause.condition = Some(condition);
    }
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    for action in keyword_actions {
        result.push(grant_keyword_action_for_anthem_subject(&clause, action));
    }
    if let Some(additions) = trailing_type_color_addition {
        push_type_color_additions_for_anthem_subject(&mut result, &clause, additions)?;
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

pub(crate) fn parse_anthem_and_goaded_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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

pub(crate) fn parse_anthem_and_no_defender_line(
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
) -> Result<(), CardTextError> {
    let filter = anthem_subject_filter(&clause.subject);
    let condition = clause.condition.clone();
    let mut push_static = |ability: StaticAbility| -> Result<(), CardTextError> {
        let ast: StaticAbilityAst = ability.into();
        result.push(match &condition {
            Some(condition) => add_static_ability_ast_condition(ast, condition.clone())?,
            None => ast,
        });
        Ok(())
    };

    if !additions.set_colors.is_empty() {
        push_static(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ))?;
    }
    if !additions.added_colors.is_empty() {
        push_static(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ))?;
    }
    if !additions.card_types.is_empty() {
        push_static(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ))?;
    }
    if !additions.subtypes.is_empty() {
        push_static(StaticAbility::add_subtypes(filter, additions.subtypes))?;
    }

    Ok(())
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
        StaticAbilityAst::Static(_) | StaticAbilityAst::KeywordAction(_) => {
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
        StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition: existing,
        } => StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition: crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        },
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
        } => StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
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

pub(crate) fn parse_protection_from_colored_spells_line(
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

fn grant_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: StaticAbility,
) -> StaticAbilityAst {
    match &clause.subject {
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
    }
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
    match &clause.subject {
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
    crate::runtime_backend::grammar::anthem_grants::parse_if_source_is_color(tokens)
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
    let existing = filter.colors.unwrap_or(ColorSet::new());
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

    StaticAbilityAst::GrantObjectAbility {
        filter: anthem_subject_filter(&clause.subject),
        ability,
        display,
        condition: clause.condition.clone(),
    }
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

pub(crate) fn parse_equipment_you_control_have_equip_line(
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
        trigger_spec: Some(TriggerSpec::ThisEntersBattlefield),
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
    let intro = crate::runtime_backend::grammar::clause_support::parse_trigger_intro_tokens(
        &trigger_tokens,
    );
    if intro.body_first == 0 {
        return Ok(None);
    }

    let ability = match crate::runtime_backend::clause_support::parse_triggered_line_lexed(
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
                Some(crate::runtime_backend::token_word_refs(&trigger_tokens).join(" ")),
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
            crate::runtime_backend::token_word_refs(&trigger_tokens).join(" ")
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

pub(crate) fn parse_heterogeneous_granted_tail(
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

        if let Some((ability, display)) =
            parse_granted_object_ability_segment(&segment, clause_words, attached_subject)?
        {
            parsed.granted_object_abilities.push((ability, display));
            continue;
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
                    split_static.push(
                        StaticAbility::can_attack_as_though_no_defender().into(),
                    );
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
    {
        return Ok(None);
    }

    Ok(Some(parsed))
}

pub(crate) fn lower_granted_tail_for_anthem_subject(
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
    };
    let mut granted = Vec::new();
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
    granted
}

pub(crate) fn parse_attached_restriction_and_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) =
        attached_grammar::parse_attached_combat_restriction_grant_tokens(tokens)
    else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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

pub(crate) fn parse_subject_color_and_granted_ability_line(
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
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let attached_subject =
        anthem_grant_grammar::parse_granted_subject_facts(shape.subject_tokens).attached_subject;
    let Some(granted_tail) = parse_heterogeneous_granted_tail(
        shape.ability_tokens,
        &clause_words,
        attached_subject,
    )?
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

pub(crate) fn lower_static_animation_bundle(
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

pub(crate) fn parse_anthem_with_trailing_segments_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
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
                    )
                    .into(),
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
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::cant_block()).into());
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
                )
                .into(),
            );
            continue;
        }
        if segment_shape == anthem_grant_grammar::ContinuingSegmentShape::MustAttack {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
            continue;
        }
        if let anthem_grant_grammar::ContinuingSegmentShape::CantBeBlockedByMoreThan(count) =
            segment_shape
        {
            extras.push(
                grant_for_anthem_subject(
                    &clause,
                    StaticAbility::cant_be_blocked_by_more_than(count),
                )
                .into(),
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
            let removed = actions
                .into_iter()
                .filter_map(|action| keyword_action_to_static_ability(action))
                .collect::<Vec<_>>();
            if removed.is_empty() {
                return Ok(None);
            }
            for ability in removed {
                extras.push(match &clause.subject {
                    AnthemSubjectAst::Source => StaticAbilityAst::RemoveStaticAbility {
                        filter: ObjectFilter::source(),
                        ability: Box::new(StaticAbilityAst::Static(ability)),
                    },
                    AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                        filter: filter.clone(),
                        ability: Box::new(StaticAbilityAst::RemoveStaticAbility {
                            filter: ObjectFilter::source(),
                            ability: Box::new(StaticAbilityAst::Static(ability)),
                        }),
                        condition: clause.condition.clone(),
                    },
                });
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
                    crate::runtime_backend::token_word_refs(&ability_tokens).join(" ")
                );
                extras.push(grant_object_ability_for_anthem_subject(
                    &clause, triggered, display,
                ));
            } else if let Some(actions) = actions {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                let granted = actions
                    .into_iter()
                    .filter_map(|action| keyword_action_to_static_ability(action))
                    .collect::<Vec<_>>();
                if granted.is_empty() {
                    return Ok(None);
                }
                for ability in granted {
                    extras.push(grant_for_anthem_subject(&clause, ability).into());
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
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
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
                crate::runtime_backend::token_word_refs(&segment).join(" ")
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

pub(crate) fn parse_conditional_all_creatures_able_to_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_conditional_must_block_shape(tokens) else {
        return Ok(None);
    };
    let condition = parse_static_condition_clause(shape.condition_tokens)?;
    match shape.target {
        anthem_grant_grammar::ConditionalMustBlockTarget::Source => {
            Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
                condition,
            }))
        }
        anthem_grant_grammar::ConditionalMustBlockTarget::EnchantedCreature => {
            Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
                display: "enchanted creature has this creature must be blocked if able".to_string(),
                condition: Some(condition),
            }))
        }
    }
}

pub(crate) fn parse_source_can_attack_as_though_no_defender_as_long_as_line(
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

pub(crate) fn parse_attached_can_attack_as_though_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_attached_no_defender_shape(tokens) else {
        return Ok(None);
    };

    let subject = crate::runtime_backend::token_word_refs(shape.subject_tokens).join(" ");
    Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
        ability: Box::new(StaticAbilityAst::Static(
            StaticAbility::can_attack_as_though_no_defender(),
        )),
        display: format!("{subject} can attack as though it didn't have defender"),
        condition: None,
    }))
}

pub(crate) fn parse_as_long_as_condition_can_attack_as_though_no_defender_line(
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

pub(crate) fn parse_gets_and_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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

pub(crate) fn parse_anthem_and_granted_ability_line(
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

pub(crate) fn parse_subject_is_every_subtype_family_line(
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

pub(crate) fn parse_anthem_line(
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

pub(crate) fn parse_multi_subject_anthem_line(
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

pub(crate) fn parse_has_base_power_toughness_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = anthem_grant_grammar::parse_base_power_toughness_shape(tokens) else {
        return Ok(None);
    };
    let subject = parse_anthem_subject(shape.subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    let base = StaticAbility::set_base_power_toughness(
        filter,
        shape.power,
        shape.toughness,
    );
    let Some(condition_tokens) = shape.condition_tokens else {
        return Ok(Some(base));
    };
    let condition = parse_static_condition_clause(condition_tokens)?;
    #[cfg(not(feature = "serialization"))]
    {
        Ok(Some(base.with_condition(condition)))
    }
    #[cfg(feature = "serialization")]
    {
        Ok(base.with_condition(condition))
    }
}

pub(crate) fn parse_isnt_creature_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let display = crate::runtime_backend::token_word_refs(tokens).join(" ");
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

pub(crate) fn parse_has_base_power_toughness_and_granted_keywords_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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
    let Some(granted_tail) = parse_heterogeneous_granted_tail(
        shape.ability_tokens,
        &clause_words,
        attached_subject,
    )?
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

pub(crate) fn parse_has_base_power_and_granted_ability_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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
    let Some(granted_tail) = parse_heterogeneous_granted_tail(
        shape.ability_tokens,
        &clause_words,
        attached_subject,
    )?
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

pub(crate) fn parse_filter_has_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
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
        if let Some(split) = anthem_grant_grammar::split_type_addition_subject(&subject_tokens) {
            if let Some(additions) = parse_type_color_addition_clause(split.addition_tokens)? {
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
                if !additions.set_colors.is_empty() {
                    result.push(
                        StaticAbility::set_colors(filter.clone(), additions.set_colors).into(),
                    );
                }
                if !additions.added_colors.is_empty() {
                    result.push(
                        StaticAbility::add_colors(filter.clone(), additions.added_colors).into(),
                    );
                }
                if !additions.card_types.is_empty() {
                    result.push(
                        StaticAbility::add_card_types(filter.clone(), additions.card_types).into(),
                    );
                }
                if !additions.subtypes.is_empty() {
                    result.push(
                        StaticAbility::add_subtypes(filter.clone(), additions.subtypes).into(),
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
        }
        let subject_facts = anthem_grant_grammar::parse_granted_subject_facts(&subject_tokens);
        if subject_facts.rejected_action || subject_facts.has_may {
            continue;
        }

        let mut ability_tokens = trim_commas(&tokens[has_idx + 1..]);
        let mut condition_failed = false;
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
            ability_tokens = split.ability_tokens.to_vec();
        }
        if condition_failed {
            continue;
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
        let attached_subject_filter =
            infer_attached_subject_filter_from_condition_expr(condition.as_ref());
        let subject = match parse_anthem_subject_with_attached_fallback(
            &subject_tokens,
            attached_subject_filter.as_ref(),
        ) {
            Ok(subject) => subject,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        let granted = lower_granted_tail_for_anthem_subject(&subject, &condition, granted_tail);
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
fn quoted_static_marker_grants_parse_for_filtered_subjects() {
    let tokens = crate::runtime_backend::lexer::lex_line(
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
        &crate::runtime_backend::token_word_refs(&tokens),
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
fn conditional_attached_quoted_equipment_grant_uses_nested_subject() {
    let tokens = crate::runtime_backend::lexer::lex_line(
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
        let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
            .expect("lex type-addition grant");
        let abilities = parse_filter_has_granted_ability_line(&tokens)
            .expect("parse type-addition grant")
            .expect("type-addition grant should be recognized");
        assert!(abilities.len() >= 3, "expected type and both grants: {text}");
    }
}

#[test]
fn player_counter_conditions_lower_for_conditional_anthems() {
    let tokens = crate::runtime_backend::lexer::lex_line(
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

    let routed = super::parse_static_ability_ast_line_lexed(&tokens)
        .expect("route conditional anthem")
        .expect("typed conditional anthem should reach static-line lowering");
    assert_eq!(routed.len(), 2, "{routed:#?}");
}

#[test]
fn conditional_anthems_preserve_no_defender_attack_permission() {
    for text in [
        "As long as this creature is monstrous, it gets +2/+2 and can attack as though it didn't have defender.",
        "As long as you control three or more artifacts, this creature gets +2/+2 and can attack as though it didn't have defender.",
    ] {
        let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
            .expect("lex no-defender anthem");
        let abilities = parse_anthem_and_no_defender_line(&tokens)
            .expect("parse no-defender anthem")
            .expect("no-defender anthem should be recognized");
        assert_eq!(abilities.len(), 2, "{abilities:#?}");
        assert!(
            format!("{abilities:#?}").contains("CanAttackAsThoughNoDefender"),
            "{abilities:#?}"
        );
    }

    let tokens = crate::runtime_backend::lexer::lex_line(
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
fn carried_subject_type_addition_lowers_with_preceding_anthem() {
    let tokens = crate::runtime_backend::lexer::lex_line(
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
    let tokens = crate::runtime_backend::lexer::lex_line(
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
        let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
            .expect("lex conditional continuation");
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


    let tokens = crate::runtime_backend::lexer::lex_line(
        "Equipped creature gets +2/+0. It gets an additional +0/+2 and has first strike as long as an Equipment named Groom's Finery is attached to a creature you control.",
        0,
    )
    .expect("lex carried conditional grant");
    let abilities = parse_carried_conditional_anthem_grant_line(&tokens)
        .expect("parse carried conditional grant")
        .expect("carried conditional grant should be recognized");
    assert_eq!(abilities.len(), 3, "{abilities:#?}");
    let debug = format!("{abilities:#?}");
    assert!(
        debug.contains("MatchingObjectAttachedToMatchingObject"),
        "{debug}"
    );
    let routed = parse_static_ability_ast_line_lexed(&tokens)
        .expect("route carried conditional grant")
        .expect("static line router should recognize carried conditional grant");
    assert_eq!(routed.len(), 3, "{routed:#?}");
}

#[test]
fn base_power_toughness_grants_accept_quoted_triggered_abilities() {
    for text in ["Enchanted creature has base power and toughness 8/8 and has \"Whenever this creature attacks, you may tap target creature with power 8 or less.\""] {
        let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
            .expect("lex base characteristic grant");
        let abilities = parse_has_base_power_toughness_and_granted_keywords_static_line(&tokens)
            .expect("parse base characteristic grant")
            .unwrap_or_else(|| panic!("base characteristic grant should be recognized: {text}"));
        assert!(abilities.len() >= 2, "expected base value and grant: {text}");
    }
}

#[test]
fn attached_combat_restrictions_preserve_quoted_ability_grants() {
    let text = "Enchanted creature can't attack or block and has \"{7}: Its controller sacrifices it and draws a card. Activate only as a sorcery.\"";
    let tokens = crate::runtime_backend::lexer::lex_line(text, 0)
        .expect("lex restriction and grant");
    let abilities = parse_attached_restriction_and_granted_ability_line(&tokens)
        .expect("parse restriction and grant")
        .expect("restriction and grant should be recognized");
    assert_eq!(abilities.len(), 2);
}

#[test]
fn conditional_color_assignments_preserve_quoted_ability_grants() {
    let text = "As long as there are seven or more cards in your graveyard, this creature is white and has \"{T}: Destroy target black creature.\"";
    let tokens =
        crate::runtime_backend::lexer::lex_line(text, 0).expect("lex color assignment and grant");
    let abilities = parse_subject_color_and_granted_ability_line(&tokens)
        .expect("parse color assignment and grant")
        .expect("color assignment and grant should be recognized");
    assert_eq!(abilities.len(), 2);
}

#[test]
fn static_turn_threshold_conditions_use_typed_anthem_grammar() {
    let drawn = crate::runtime_backend::lexer::lex_line(
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
        crate::runtime_backend::lexer::lex_line("You have rolled two or more dice this turn", 0)
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
    let devotion = crate::runtime_backend::lexer::lex_line(
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
        crate::runtime_backend::lexer::lex_line("It is your turn and you attacked this turn.", 0)
            .expect("conjoined condition should lex");
    let crate::ConditionExpr::And(left, right) =
        parse_static_condition_clause(&conjoined).expect("conjoined condition should parse")
    else {
        panic!("expected a typed conjunction");
    };
    assert_eq!(*left, crate::ConditionExpr::YourTurn);
    assert_eq!(*right, crate::ConditionExpr::AttackedThisTurn);

    let graveyard = crate::runtime_backend::lexer::lex_line(
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
    let tokens = crate::runtime_backend::lexer::lex_line(
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
fn granted_escape_tail_captures_dynamic_exile_count() {
    let tokens = crate::runtime_backend::lexer::lex_line(
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
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Its miracle cost is equal to its mana cost reduced by {4}.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_granted_miracle_cost_reduction_tail_clause(&tokens)
        .expect("miracle tail should parse");
    let (cost, used) =
        crate::runtime_backend::front_end::shared::util::leading_mana_cost_from_tokens(
            parsed.reduction_cost_tokens,
        )
        .expect("captured cost should parse");

    assert_eq!(cost.generic_mana_total(), 4);
    assert_eq!(used, parsed.reduction_cost_tokens.len());
}

#[test]
fn cant_be_blocked_by_more_than_clause_captures_subject_and_threshold() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Each creature you control with a +1/+1 counter on it can't be blocked by more than one creature.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_cant_be_blocked_by_more_than_clause(&tokens)
        .expect("max-blockers clause should parse");
    let subject_words =
        crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
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
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Each creature you control can block two additional creatures each combat.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_can_block_additional_creature_clause(&tokens)
        .expect("additional-blocker clause should parse");
    let subject_words =
        crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
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
