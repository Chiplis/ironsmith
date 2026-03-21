use crate::cards::builders::{
    CardTextError, EffectAst, KeywordAction, LineAst, StaticAbilityAst, Token, TriggerSpec,
    parse_ability_phrase, parse_card_type, parse_color, parse_flashback_keyword_line,
    parse_subtype_flexible, split_on_and, split_on_comma_or_semicolon, words,
};

fn parse_protection_chain(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let mut words = words(tokens);
    if words.first().copied() == Some("and") {
        words.remove(0);
    }
    if words.len() < 3 {
        return None;
    }
    if words[0] != "protection" || words[1] != "from" {
        return None;
    }

    let mut actions = Vec::new();
    let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
        let value = *words.get(idx + 1)?;
        match value {
            "the" if words.get(idx + 2).copied() == Some("chosen")
                && words.get(idx + 3).copied() == Some("player") =>
            {
                Some(KeywordAction::ProtectionFromChosenPlayer)
            }
            "colorless" => Some(KeywordAction::ProtectionFromColorless),
            "everything" => Some(KeywordAction::ProtectionFromEverything),
            "all" if matches!(words.get(idx + 2).copied(), Some("color") | Some("colors")) => {
                Some(KeywordAction::ProtectionFromAllColors)
            }
            _ => parse_color(value)
                .map(KeywordAction::ProtectionFrom)
                .or_else(|| parse_card_type(value).map(KeywordAction::ProtectionFromCardType))
                .or_else(|| {
                    parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype)
                }),
        }
    };

    let mut from_count = 0usize;
    let mut parsed_count = 0usize;
    for idx in 0..words.len().saturating_sub(1) {
        if words[idx] != "from" {
            continue;
        }
        from_count += 1;
        if let Some(action) = parse_from_target(&words, idx) {
            parsed_count += 1;
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
    }

    if actions.is_empty() || parsed_count < from_count {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn rewrite_parse_ability_line(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }

    let segments = split_on_comma_or_semicolon(tokens);
    let mut actions = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(protection_actions) = parse_protection_chain(&segment) {
            actions.extend(protection_actions);
            continue;
        }

        if let Some(action) = parse_ability_phrase(&segment) {
            actions.push(action);
        } else {
            let and_parts = split_on_and(&segment);
            if and_parts.len() > 1 {
                let mut all_ok = true;
                for part in &and_parts {
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

pub(crate) fn rewrite_parse_effect_sentences(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    super::ported_effects_sentences::parse_effect_sentences(tokens)
}

pub(crate) fn rewrite_parse_triggered_line(tokens: &[Token]) -> Result<LineAst, CardTextError> {
    super::ported_activation_and_restrictions::parse_triggered_line(tokens)
}

pub(crate) fn rewrite_parse_trigger_clause(
    tokens: &[Token],
) -> Result<TriggerSpec, CardTextError> {
    super::ported_activation_and_restrictions::parse_trigger_clause(tokens)
}

pub(crate) fn rewrite_parse_static_ability_ast_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    super::ported_keyword_static::parse_static_ability_ast_line(tokens)
}
