pub fn parse_ability_line(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }
    if let Some(parsed) =
        crate::grammar::keyword_action_costs::parse_dynamic_soulshift_tokens(
            tokens,
        )
    {
        return Some(vec![KeywordAction::SoulshiftValue(
            crate::effect::Value::Count(parsed.count_filter),
        )]);
    }
    if let Some(action) = parse_dynamic_firebending(tokens) {
        return Some(vec![action]);
    }
    if let Some(action @ KeywordAction::CumulativeUpkeep { .. }) = parse_ability_phrase(tokens) {
        return Some(vec![action]);
    }

    let segments = split_lexed_slices_on_commas_or_semicolons(tokens);
    let mut actions = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(protection_actions) =
            crate::clause_support::parse_protection_chain(segment)
        {
            actions.extend(protection_actions);
            continue;
        }

        // Try the segment as-is first, then split on "and" for compound keywords
        if let Some(action) = parse_ability_phrase(segment) {
            actions.push(action);
        } else {
            // Split on "and" to handle "menace and deathtouch", "trample and haste", etc.
            let and_parts = split_lexed_slices_on_and(segment);
            if and_parts.len() > 1 {
                let mut all_ok = true;
                for part in and_parts {
                    if part.is_empty() {
                        continue;
                    }
                    if let Some(action) = parse_ability_phrase(part) {
                        actions.push(action);
                    } else {
                        all_ok = false;
                        break;
                    }
                }
                if !all_ok {
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub fn parse_dynamic_firebending(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    let view = crate::grammar::primitives::TokenWordView::new(tokens);
    let words = view.to_word_refs();
    if words.first().copied() != Some("firebending") || words.get(1).copied() != Some("x") {
        return None;
    }
    let where_word = words.iter().position(|word| *word == "where")?;
    let binding_range = view.token_span_for_words(where_word, view.len())?;
    let amount = parse_value_binding_clause(&tokens[binding_range])?;
    let surface_range = view.token_span_for_words(1, view.len())?;
    let surface =
        crate::lexer::render_token_slice(&tokens[surface_range])
            .trim()
            .trim_end_matches('.')
            .to_string();
    Some(KeywordAction::FirebendingValue { amount, surface })
}

pub fn reject_unimplemented_keyword_actions(
    _actions: &[KeywordAction],
    _clause: &str,
) -> Result<(), CardTextError> {
    Ok(())
}

pub fn keyword_action_to_static_ability(action: KeywordAction) -> Option<StaticAbility> {
    static_ability_for_keyword_action(action)
}
