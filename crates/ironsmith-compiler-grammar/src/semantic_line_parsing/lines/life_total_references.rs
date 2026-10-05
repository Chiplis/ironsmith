use super::*;

/// The numeric subject of an intervening life comparison may be repeated as
/// “it” in a following total-setting instruction. The typed predicate proves
/// that antecedent; the same words without that proof remain unclaimed.
pub fn conditional_life_total_set(
    predicate: Option<&PredicateAst>,
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let Some(PredicateAst::ValueComparison { left, .. }) = predicate else {
        return None;
    };
    if !matches!(left.unhinted(), Value::LifeTotal(PlayerFilter::You)) {
        return None;
    }
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let (_, rest) = crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&["it", "becomes", "equal", "to"]),
    )?;
    let (amount, used) = crate::util::parse_value(rest)?;
    if used != rest.len() {
        return None;
    }
    Some(vec![EffectAst::subject_verb_set_life_total(
        PlayerAst::You,
        amount,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_numeric_life_antecedent_cannot_be_guessed_from_an_object_or_missing_condition() {
        let tokens =
            crate::lexer::lex_line("it becomes equal to your starting life total", 0).unwrap();
        let condition = |left| PredicateAst::ValueComparison {
            left,
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: Value::StartingLifeTotal(PlayerFilter::You),
        };
        let life = condition(Value::LifeTotal(PlayerFilter::You));
        let parsed = conditional_life_total_set(Some(&life), &tokens).unwrap();
        assert_eq!(
            parsed,
            vec![EffectAst::subject_verb_set_life_total(
                PlayerAst::You,
                Value::StartingLifeTotal(PlayerFilter::You)
            )]
        );
        assert!(conditional_life_total_set(None, &tokens).is_none());
        assert!(
            conditional_life_total_set(Some(&condition(Value::SourcePower)), &tokens).is_none()
        );
        let extra = crate::lexer::lex_line(
            "it becomes equal to your starting life total. Draw a card.",
            0,
        )
        .unwrap();
        assert!(conditional_life_total_set(Some(&life), &extra).is_none());
    }
}
