use super::*;


pub fn parse_attached_is_legendary_gets_and_has_keywords_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = attached_grammar::parse_attached_legendary_gets_has_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter(parsed.subject_tokens, false)?;
    let Some(modifier_token) = parsed.modifier_token.as_word() else {
        return Ok(None);
    };
    let (power, toughness) = match parse_pt_modifier(modifier_token) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(actions) = parse_ability_line(parsed.keyword_tokens) else {
        return Ok(None);
    };

    let clause_text = crate::lexer::render_token_slice(tokens);
    let mut out = Vec::new();
    out.push(StaticAbility::add_supertypes(filter.clone(), vec![Supertype::Legendary]).into());

    let anthem_clause = ParsedAnthemClause {
        subject: AnthemSubjectAst::Filter(filter.clone()),
        power: AnthemValue::Fixed(power),
        toughness: AnthemValue::Fixed(toughness),
        condition: None,
        count_uses_where_x: false,
        additional_surface: false,
        set_quantifier_surface: None,
    };
    out.push(build_anthem_static_ability(&anthem_clause).into());

    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if action.lowers_to_static_ability() {
            out.push(StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: None,
            });
        }
    }

    Ok(Some(out))
}


pub fn parse_attached_gets_and_has_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = attached_grammar::parse_attached_gets_and_has_tokens(tokens) else {
        return Ok(None);
    };
    let line_text = crate::lexer::render_token_slice(tokens);
    let clause = parse_anthem_clause(tokens, shape.get_token, shape.and_token)?;
    let anthem = build_anthem_static_ability(&clause);
    let ability_tokens = trim_edge_punctuation(shape.ability_tokens);

    if let anthem_grant_grammar::ContinuingSegmentShape::Lose {
        ability_tokens: loss_tokens,
    } = anthem_grant_grammar::parse_continuing_segment_shape(&ability_tokens)
    {
        let loss_tokens = trim_edge_punctuation(loss_tokens);
        let Some(actions) = parse_ability_line(&loss_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &line_text)?;
        if actions.is_empty()
            || actions
                .iter()
                .any(|action| !action.lowers_to_static_ability())
        {
            return Ok(None);
        }
        let mut out = vec![anthem.into()];
        out.extend(
            actions
                .into_iter()
                .map(|action| remove_keyword_action_for_anthem_subject(&clause, action)),
        );
        return Ok(Some(out));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &line_text)?;
        let mut out = vec![anthem.clone().into()];
        let mut granted_any = false;
        for action in actions {
            if action.lowers_to_static_ability() {
                out.push(grant_keyword_action_for_anthem_subject(&clause, action));
                granted_any = true;
            }
        }
        if granted_any {
            return Ok(Some(out));
        }
    }

    for split in attached_grammar::parse_attached_ability_splits_tokens(&ability_tokens)
        .into_iter()
        .rev()
    {
        let Some(actions) = parse_ability_line(split.keyword_tokens) else {
            continue;
        };
        reject_unimplemented_keyword_actions(&actions, &line_text)?;
        let keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect::<Vec<_>>();
        if keyword_actions.is_empty() {
            continue;
        }

        if let Some(parsed) = parse_attached_granted_activated_line(split.granted_tokens)? {
            let mut out = vec![anthem.clone().into()];
            for action in keyword_actions {
                out.push(grant_keyword_action_for_anthem_subject(&clause, action));
            }
            let display = display_text_for_tokens(split.granted_tokens, false);
            let grant = grant_object_ability_for_anthem_subject(&clause, parsed, display);
            out.push(grant);
            return Ok(Some(out));
        }
    }

    let has_colon = contains_token_kind(&ability_tokens, TokenKind::Colon);
    if let Some(parsed) = parse_attached_granted_activated_line(&ability_tokens)? {
        let display = display_text_for_tokens(&ability_tokens, false);
        let grant = grant_object_ability_for_anthem_subject(&clause, parsed, display);
        return Ok(Some(vec![anthem.into(), grant]));
    }
    if has_colon {
        return Err(CardTextError::ParseError(format!(
            "unsupported attached activated-ability grant (clause: '{}')",
            line_text
        )));
    }

    if attached_grammar::parse_trigger_intro_tokens(&ability_tokens)
        && let LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } = crate::clause_support::parse_triggered_line_lexed(&ability_tokens)?
    {
        let parsed = parsed_triggered_ability(
            trigger,
            effects,
            vec![Zone::Battlefield],
            Some(crate::lexer::token_word_refs(&ability_tokens).join(" ")),
            trigger_surface::parse_trigger_frequency_condition_tokens(
                &ability_tokens,
                max_triggers_per_turn,
            ),
            None,
            ReferenceImports::default(),
        );
        if parsed_triggered_ability_is_empty(&parsed) {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty attached triggered grant clause (clause: '{}')",
                line_text
            )));
        }
        let text = crate::lexer::token_word_refs(&ability_tokens).join(" ");
        let grant = grant_object_ability_for_anthem_subject(&clause, parsed, text);
        return Ok(Some(vec![anthem.into(), grant]));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported attached granted ability clause (clause: '{}')",
        line_text
    )))
}


pub fn parse_equipped_gets_and_has_activated_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = attached_grammar::parse_equipped_activated_grant_tokens(tokens) else {
        return Ok(None);
    };
    let line_text = crate::lexer::render_token_slice(tokens);
    let ability_tokens_raw = shape.ability_tokens;
    let ability_tokens = trim_edge_punctuation(ability_tokens_raw);
    let has_colon = contains_token_kind(&ability_tokens, TokenKind::Colon);
    let Some(parsed) = parse_attached_granted_activated_line(ability_tokens_raw)? else {
        if has_colon {
            return Err(CardTextError::ParseError(format!(
                "unsupported equipped activated-ability grant (clause: '{}')",
                line_text
            )));
        }
        return Ok(None);
    };

    let mut static_abilities = Vec::new();
    if let Some((get_token, anthem_end)) = shape.anthem_bounds {
        let clause = parse_anthem_clause(tokens, get_token, anthem_end)?;
        static_abilities.push(build_anthem_static_ability(&clause).into());
    }
    static_abilities.push(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: parsed,
        display: format!(
            "{} has {}",
            crate::lexer::token_word_refs(&tokens[..shape.has_token]).join(" "),
            display_text_for_tokens(&ability_tokens, true)
        ),
        condition: None,
    });

    Ok(Some(static_abilities))
}
