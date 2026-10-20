use super::*;
use crate::cards::builders::{ChoiceActionAst, SubjectVerbRoleAst};

/// "It deals an amount of damage of your choice to you and target creature."
/// (Volcano Hellion): the controller chooses a number as the instruction
/// resolves, then the source deals that much damage to each named recipient.
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some((_, deal_index)) = find_verb(&tokens) else {
        return Ok(None);
    };
    if !tokens[deal_index].is_any_word(&["deal", "deals"]) {
        return Ok(None);
    }
    let source_tokens = &tokens[..deal_index];
    let source_words = non_article_token_word_refs(source_tokens);
    if !(source_words.is_empty()
        || source_words.as_slice() == ["it"]
        || is_source_reference_words(&source_words)
        || is_authored_named_source(source_tokens))
    {
        return Ok(None);
    }
    let after = &tokens[deal_index + 1..];
    const HEAD: &[&str] = &["an", "amount", "of", "damage", "of", "your", "choice", "to"];
    if after.len() <= HEAD.len()
        || !after.iter().zip(HEAD).all(|(token, word)| token.is_word(word))
    {
        return Ok(None);
    }
    let recipient_tokens = &after[HEAD.len()..];
    let mut pieces = Vec::new();
    let mut start = 0usize;
    for (index, token) in recipient_tokens.iter().enumerate() {
        if token.is_word("and") {
            pieces.push(&recipient_tokens[start..index]);
            start = index + 1;
        }
    }
    pieces.push(&recipient_tokens[start..]);
    if pieces.iter().any(|piece| trim_commas(piece).is_empty()) {
        return Ok(None);
    }
    let amount = Value::PendingPriorEffectMetric(
        ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::Outcome,
            ironsmith_core::EffectMetric::Count,
        )
        .with_action(ironsmith_core::PriorEffectAction::ChosenNumber),
    );
    let mut effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::Chooser,
        PlayerAst::You,
        SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber {
            min: 0,
            max: None,
            source_owned: false,
            draw_count_choice: false,
        }),
    )];
    for piece in pieces {
        let Ok(target) = parse_target_phrase(&trim_commas(piece)) else {
            return Ok(None);
        };
        effects.push(EffectAst::subject_verb_damage(amount.clone(), target));
    }
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chosen_damage_amount_is_one_number_choice_then_one_damage_per_recipient() {
        let tokens = crate::lexer::lex_line(
            "It deals an amount of damage of your choice to you and target creature.",
            0,
        )
        .unwrap();
        let effects = parse(&tokens).unwrap().unwrap();
        assert_eq!(effects.len(), 3);
        assert!(matches!(&effects[0], EffectAst::SubjectVerb(subject) if matches!(subject.action,
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNumber { min: 0, max: None, .. }))));
        for effect in &effects[1..] {
            assert!(matches!(effect, EffectAst::SubjectVerb(subject) if matches!(subject.action,
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { .. }))));
        }
        for text in [
            "It deals 3 damage to you and target creature.",
            "Target creature deals an amount of damage of your choice to you.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse(&tokens).unwrap().is_none(), "{text}");
        }
    }
}
