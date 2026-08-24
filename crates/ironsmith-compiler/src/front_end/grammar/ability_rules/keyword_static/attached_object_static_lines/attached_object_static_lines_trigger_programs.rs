use super::*;


pub fn parse_attached_has_keywords_and_triggered_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = attached_grammar::parse_attached_keywords_and_trigger_tokens(tokens) else {
        return Ok(None);
    };
    let clause_text = crate::lexer::render_token_slice(tokens);
    let keyword_tokens = trim_edge_punctuation(parsed.keyword_tokens);
    let trigger_tokens = trim_edge_punctuation(parsed.trigger_tokens);
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };

    let mut keyword_actions = Vec::new();
    let mut extra_grants: Vec<StaticAbilityAst> = Vec::new();
    for action in actions {
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let KeywordAction::Annihilator(amount) = action {
            extra_grants.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed_ability_from_ability(annihilator_granted_ability(amount)),
                display: format!("{} has annihilator {amount}", parsed.subject.display()),
                condition: None,
            });
        } else if action.lowers_to_static_ability() {
            keyword_actions.push(action);
        }
    }
    if keyword_actions.is_empty() && extra_grants.is_empty() {
        return Ok(None);
    }

    let triggered = match crate::clause_support::parse_triggered_line_lexed(
        &trigger_tokens,
    )? {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => parsed_triggered_ability(
            trigger,
            effects,
            vec![Zone::Battlefield],
            Some(crate::lexer::token_word_refs(&trigger_tokens).join(" ")),
            trigger_surface::parse_trigger_frequency_condition_tokens(
                &trigger_tokens,
                max_triggers_per_turn,
            ),
            None,
            ReferenceImports::default(),
        ),
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported attached triggered grant clause (clause: '{}')",
                clause_text
            )));
        }
    };
    if parsed_triggered_ability_is_empty(&triggered) {
        return Err(CardTextError::ParseError(format!(
            "unsupported empty attached triggered grant clause (clause: '{}')",
            clause_text
        )));
    }

    let subject = match parse_anthem_subject(parsed.subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let filter = match subject {
        AnthemSubjectAst::Filter(filter) => filter,
        AnthemSubjectAst::Source => ObjectFilter::source(),
    };

    let mut static_abilities = Vec::new();
    for action in keyword_actions {
        static_abilities.push(StaticAbilityAst::GrantKeywordAction {
            filter: filter.clone(),
            action,
            condition: None,
        });
    }
    static_abilities.extend(extra_grants);
    let display = format!(
        "{} has {}",
        parsed.subject.display(),
        crate::lexer::token_word_refs(&trigger_tokens).join(" ")
    );
    static_abilities.push(StaticAbilityAst::AttachedObjectAbilityGrant {
        ability: triggered,
        display,
        condition: None,
    });

    Ok(Some(static_abilities))
}
