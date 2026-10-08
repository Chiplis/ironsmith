//! Complete persistent P/T assignments over controller-relative state.
//! Resolution-local targets, event values and unbound X never become statics.
use super::*;

fn state_value(tokens: &[OwnedLexToken]) -> Result<Value, CardTextError> {
    // Use the complete shared expression, not the legacy CDA filter fallback:
    // a recognized count prefix cannot discard the rest of the definition.
    let words = parser_token_word_refs(tokens);
    let complete_tokens = tokens.iter().enumerate().all(|(index, token)| {
        matches!(token.kind, TokenKind::Word | TokenKind::Number)
            || (token.is_comma() && matches!(parser_token_word_refs(&tokens[index + 1..]).as_slice(),
                ["rounded", "up" | "down"]))
    });
    let value = complete_tokens.then(|| parse_value_expr_words(&words)).flatten()
        .filter(|(_, used)| *used == words.len())
        .map(|(value, _)| value)
        .filter(ironsmith_core::anthem_model::supports_controller_state_anthem_value)
        .ok_or_else(|| CardTextError::ParseError(format!(
            "unsupported persistent power/toughness value (value: '{}')",
            render_token_slice(tokens),
        )))?;
    Ok(value)
}

fn setting_subject(tokens: &[OwnedLexToken]) -> Result<AnthemSubjectAst, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if tokens.iter().all(|token| token.as_word().is_some()) {
        if is_source_reference_words(&words) || anthem_grant_grammar::is_source_it_subject(tokens) {
            return Ok(AnthemSubjectAst::Source);
        }
        if matches!(words.as_slice(), ["equipped" | "enchanted", "creature" | "permanent" | "artifact" | "land"]) {
            return parse_anthem_subject(tokens);
        }
        if let Some(anthem_grant_grammar::AnthemSubjectGrammarMatch::Filter(filter)) =
            anthem_grant_grammar::parse_exact_anthem_subject_grammar(tokens)
        {
            return Ok(AnthemSubjectAst::Filter(filter));
        }
        if let Some(filter) = crate::grammar::filters::parse_simple_object_filter_lexed(tokens, false) {
            return Ok(AnthemSubjectAst::Filter(filter));
        }
        if crate::lexer::is_authored_proper_name_phrase(tokens) {
            return Ok(AnthemSubjectAst::Source);
        }
    }
    Err(CardTextError::ParseError(format!(
        "unsupported complete base power/toughness subject: '{}'", render_token_slice(tokens),
    )))
}

pub(super) fn parse_bound_base_pt(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((subject, definition)) =
        anthem_grant_grammar::parse_base_power_toughness_where_x_shape(tokens)
    else {
        return Ok(None);
    };
    let subject = setting_subject(subject)?;
    // "X/X, where X is ...": keep the authored binding surface.
    let value = state_value(definition)?
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs);
    Ok(Some(StaticAbility::set_base_power_toughness_value(
        anthem_subject_filter(&subject), value.clone(), value,
    )))
}

pub(super) fn parse_timed_source_pt(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::grammar::primitives;
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    let condition_and_prefix = [
        (&["during", "your", "turn"][..], PredicateAst::YourTurn),
        (&["during", "turns", "other", "than", "yours"][..],
            PredicateAst::Not(Box::new(PredicateAst::YourTurn))),
    ].into_iter().find(|(prefix, _)| words.starts_with(prefix));
    let Some((prefix, condition)) = condition_and_prefix else { return Ok(None); };
    if !words.windows(5).any(|words| words == ["power", "and", "toughness", "are", "each"]) {
        return Ok(None);
    }
    let comma = tokens.iter().position(|token| token.is_comma())
        .filter(|comma| tokens[..*comma].iter().all(|token| token.as_word().is_some())
            && parser_token_word_refs(&tokens[..*comma]) == prefix)
        .ok_or_else(|| CardTextError::ParseError("incomplete timed power/toughness condition prefix".into()))?;
    let body = &tokens[comma + 1..];
    let Some((axis, _, value)) = primitives::find_prefix(body, || {
        primitives::phrase(&["power", "and", "toughness", "are", "each"])
    }) else {
        return Err(CardTextError::ParseError("incomplete timed power/toughness axis".into()));
    };
    let source_tokens = &body[..axis];
    let source_words = parser_token_word_refs(source_tokens);
    // Possessive source aliases are distinct from attached recipients. A
    // capitalized printed name is the same established source-name surface
    // used by the CDA reader; ordinary noun phrases cannot borrow it.
    if !source_tokens.iter().all(|token| token.as_word().is_some())
        || (crate::util::source_reference_surface_for_possessive_words(&source_words).is_none()
            && !crate::lexer::is_authored_proper_name_phrase(source_tokens))
    {
        return Err(CardTextError::ParseError("timed source power/toughness requires a complete source subject".into()));
    }
    let value = primitives::parse_prefix(value, primitives::phrase(&["equal", "to"]))
        .map_or(value, |(_, tail)| tail);
    let value = state_value(value)?;
    // A definition conditional on the current turn is not a CDA (604.3).
    // It functions on the battlefield and uses layer 7b, even though its
    // authored wording omits the word "base".
    Ok(Some(StaticAbility::set_base_power_toughness_value(
        ObjectFilter::source(), value.clone(), value,
    ).with_condition(condition)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith_core::StaticAbilityPayload;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        crate::lexer::lex_line(text, 0).unwrap()
    }

    #[test]
    fn bound_base_values_keep_source_and_attached_recipient_roles_separate() {
        for subject in ["Equipped creature", "Enchanted creature", "This creature",
            "Creatures you control"]
        {
            let verb = if subject == "Creatures you control" { "have" } else { "has" };
            let tokens = lex(&format!(
                "{subject} {verb} base power and toughness X/X, where X is your life total."));
            let parsed = parse_static_ability_ast_line_lexed(&tokens).unwrap().unwrap();
            assert_eq!(parsed.len(), 1);
            let StaticAbilityAst::Static(ability) = &parsed[0] else { panic!("typed static"); };
            let StaticAbilityPayload::SetBasePowerToughnessValue { filter, power, toughness } =
                &ability.payload else { panic!("layer 7b payload: {ability:?}"); };
            assert_eq!(power.unhinted(), &Value::LifeTotal(PlayerFilter::You));
            assert_eq!(toughness, power);
            assert_eq!(filter.source, subject == "This creature");
            assert_eq!(filter.with_attached_object.is_some(),
                matches!(subject, "Equipped creature" | "Enchanted creature"));
        }
    }

    #[test]
    fn timed_source_values_are_two_independent_setting_conditions() {
        let tokens = lex("During your turn, this creature's power and toughness are each equal to 2 plus the number of Swamps your opponents control. During turns other than yours, this creature's power and toughness are each 2.");
        let parsed = parse_static_ability_ast_line_lexed(&tokens).unwrap().unwrap();
        assert_eq!(parsed.len(), 2);
        for (index, ability) in parsed.iter().enumerate() {
            let StaticAbilityAst::Static(ability) = ability else { panic!("typed setting"); };
            assert_eq!(ability.id, Some(crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter));
            let StaticAbilityPayload::Conditional { condition, ability } = &ability.payload
                else { panic!("each assignment retains its own condition"); };
            assert_eq!(condition, &if index == 0 { PredicateAst::YourTurn }
                else { PredicateAst::Not(Box::new(PredicateAst::YourTurn)) });
            let StaticAbilityPayload::SetBasePowerToughnessValue { filter, .. } =
                &ability.payload else { panic!("not a CDA"); };
            assert!(filter.source);
        }
    }

    #[test]
    fn dynamic_setting_keeps_branch_local_subject_exclusions() {
        let tokens = lex("Artifacts or non-Aura enchantments you control have base power and toughness X/X, where X is your life total.");
        let ability = parse_bound_base_pt(&tokens).unwrap().unwrap();
        let StaticAbilityPayload::SetBasePowerToughnessValue { filter, .. } = &ability.payload
            else { panic!("typed setting"); };
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(!filter.excluded_subtypes.contains(&crate::Subtype::Aura));
        let artifacts = filter.any_of.iter().find(|branch| branch.card_types.contains(&CardType::Artifact)).unwrap();
        let enchantments = filter.any_of.iter().find(|branch| branch.card_types.contains(&CardType::Enchantment)).unwrap();
        assert!(!artifacts.excluded_subtypes.contains(&crate::Subtype::Aura));
        assert!(enchantments.excluded_subtypes.contains(&crate::Subtype::Aura));
    }

    #[test]
    fn static_value_readers_reject_targets_unbound_values_and_ignored_tails() {
        for text in [
            "Target creature has base power and toughness X/X, where X is your life total.",
            "Equipped creature has base power and toughness X/X, where X is X.",
            "Equipped creature has base power and toughness X/X, where X is target player's life total.",
            "Equipped creature has base power and toughness X/X, where X is that creature's power.",
            "Equipped creature has base power and toughness X/X, where X is your life total and draws a card.",
            "During your turn, equipped creature's power and toughness are each 7.",
            "During your turn nonsense, this creature's power and toughness are each 7.",
            "During turns other than yours, this creature's power and toughness are each 2 and flies.",
            "During your turn, this creature's power and toughness are each equal to 2 plus the number of Swamps your opponents control dancing.",
            "Equipped creature has base power and toughness X/X, where X is the number of creatures you control and draws a card.",
            "Equipped creature has base power and toughness X/X, where X is your life {R} total.",
            "Unrecognized equipped creature has base power and toughness X/X, where X is your life total.",
        ] {
            let tokens = lex(text);
            assert!(!matches!(parse_bound_base_pt(&tokens), Ok(Some(_))), "{text}");
            assert!(!matches!(parse_timed_source_pt(&tokens), Ok(Some(_))), "{text}");
        }
    }

    #[test]
    fn malformed_timed_bodies_cannot_fall_back_to_unconditional_cdas() {
        for text in [
            "During turns other than yours nonsense, this creature's power and toughness are each equal to 7.",
            "During your turn nonsense, this creature's power and toughness are each equal to 7.",
            "During your turn this creature's power and toughness are each equal to 7.",
            "During turns other than yours, equipped creature's power and toughness are each equal to 7.",
            "During your {R} turn, this creature's power and toughness are each 7.",
            "During your turn, this {R} creature's power and toughness are each 7.",
            "During your turn, this creature's power {R} and toughness are each 7.",
        ] {
            assert!(parse_static_ability_ast_line_lexed(&lex(text)).is_err(), "{text}");
        }
    }
}
