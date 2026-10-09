use super::*;

/// A target mentioned only by a life quantity is still announced before the
/// conditional resolves. Its declaration also supplies the following “that
/// player” antecedent. Repeated references to the same target are one slot.
pub(super) fn target_prelude(
    predicate: &PredicateAst,
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let PredicateAst::ValueComparison { left, right, .. } = predicate else {
        return Ok(Vec::new());
    };
    fn targets(value: &Value, found: &mut Vec<PlayerFilter>) {
        match value.unhinted() {
            Value::LifeTotal(player)
            | Value::StartingLifeTotal(player)
            | Value::MaximumLifeTotal(player)
            | Value::CountPlayersBelowHalfStartingLifeTotal(player)
            // "target player has fewer than nine poison counters" (Vraska,
            // Betrayal's Sting): the same single authored player target.
            | Value::PlayerCounters(player, _) => {
                if let PlayerFilter::Target(inner) = player
                    && !found.contains(inner.as_ref())
                {
                    found.push((**inner).clone());
                }
            }
            Value::Add(left, right) | Value::Min(left, right) => {
                targets(left, found);
                targets(right, found);
            }
            Value::Scaled(inner, _)
            | Value::HalfRoundedDown(inner)
            | Value::DividedRoundedDown(inner, _) => targets(inner, found),
            _ => {}
        }
    }
    let mut found = Vec::new();
    targets(left, &mut found);
    targets(right, &mut found);
    if found.is_empty() {
        return Ok(Vec::new());
    }
    let Some(comma) = tokens.iter().position(OwnedLexToken::is_comma) else {
        return Ok(Vec::new());
    };
    let predicate_tokens = &tokens[..comma];
    // Legacy anaphoric “that player's” values may still contain Target(Any).
    // Only an authored target in this condition establishes a new choice.
    let authored_targets = predicate_tokens
        .iter()
        .filter(|token| token.is_word("target"))
        .count();
    if authored_targets == 0 {
        return Ok(Vec::new());
    }
    // An expression such as absolute difference repeats one authored target
    // algebraically. Two authored target phrases, even with equal filters,
    // are distinct declarations and are outside this single-slot reader.
    if authored_targets != 1 {
        return Err(CardTextError::ParseError(
            "unsupported multiple authored targets in life-value condition".into(),
        ));
    }
    let mut quoted = false;
    let outer_instead = tokens.iter().any(|token| {
        if token.is_quote() {
            quoted = !quoted;
        }
        !quoted && token.is_word("instead")
    });
    if found.len() != 1 || outer_instead {
        return Err(CardTextError::ParseError(
            "unsupported multiple-target or replacement life-value condition".into(),
        ));
    }
    Ok(vec![EffectAst::subject_verb_explicit_target_only(
        TargetAst::Player(
            found.pop().expect("one target"),
            Some(span_from_tokens(predicate_tokens).unwrap_or_else(TextSpan::synthetic)),
        ),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_target_is_announced_outside_the_numeric_condition_and_duplicate_value_occurrences_share_it()
     {
        let text = "If the difference between your life total and target player's life total is 5 or less, exchange life totals with that player.";
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let effects = super::super::parse_conditional_sentence_with_grammar_entrypoint_lexed(
            &tokens,
            crate::effect_sentences::parse_effect_chain_lexed,
        )
        .unwrap();
        assert_eq!(effects.len(), 2);
        assert!(
            matches!(&effects[0],EffectAst::SubjectVerb(subject) if matches!(&subject.action,SubjectVerbActionAst::TargetOnly { target:TargetAst::Player(PlayerFilter::Any,Some(_)),explicit_declaration:true }))
        );
        assert!(matches!(
            &effects[1],
            EffectAst::Conditionals(ConditionalEffectAst::Conditional { .. })
        ));
    }
    #[test]
    fn an_anaphoric_life_comparison_does_not_declare_a_new_target() {
        let tokens = crate::lexer::lex_line(
            "If that player's life total is greater than your starting life total, draw a card.",
            0,
        )
        .unwrap();
        let predicate = PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::target_player()),
            operator: crate::effect::ValueComparisonOperator::GreaterThan,
            right: Value::StartingLifeTotal(PlayerFilter::You),
        };
        assert!(target_prelude(&predicate, &tokens).unwrap().is_empty());
    }

    #[test]
    fn two_authored_targets_with_equal_filters_cannot_collapse_into_one_choice() {
        let tokens = crate::lexer::lex_line(
            "If target player's life total is greater than target player's life total, draw a card.", 0,
        ).unwrap();
        let predicate = PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::target_player()),
            operator: crate::effect::ValueComparisonOperator::GreaterThan,
            right: Value::LifeTotal(PlayerFilter::target_player()),
        };
        assert!(target_prelude(&predicate, &tokens).is_err());
        assert!(
            super::super::parse_conditional_sentence_with_grammar_entrypoint_lexed(
                &tokens,
                crate::effect_sentences::parse_effect_chain_lexed,
            )
            .is_err()
        );
    }
}
