//! A following amount limit constrains the preceding variable life payment.
use super::*;
use crate::cards::builders::LifeResourceActionAst;

pub(crate) fn read(sentences: &[SentenceInput], index: usize) -> Option<Vec<EffectAst>> {
    let first = crate::lexer::parser_token_word_refs(sentences.get(index)?.lowered());
    if first != ["pay", "any", "amount", "of", "life"]
        && first != ["you", "pay", "any", "amount", "of", "life"] {
        return None;
    }
    let second = crate::lexer::parser_token_word_refs(sentences.get(index + 1)?.lowered());
    let tail = second.strip_prefix(&["the", "amount", "you", "pay", "cant", "be", "more", "than"])?;
    let (mut maximum, used) = crate::util::parse_value_expr_words(tail)?;
    if used != tail.len() { return None; }
    // The plural possessive refers to the preceding counted controller group,
    // rather than introducing an unbound iterated player.
    if tail.contains(&"their") {
        let Value::Add(left, right) = &mut maximum else { return None; };
        let (Value::Count(controlled), Value::Count(owned)) = (left.as_mut(), right.as_mut())
            else { return None; };
        let controller = controlled.controller.clone()?;
        if owned.owner != Some(PlayerFilter::IteratedPlayer) { return None; }
        owned.owner = Some(controller);
    }
    Some(vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife {
            min_amount: 0, maximum: Some(maximum), source_owned: false,
        }),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn variable_life_payment_retains_complete_limit_and_plural_owner() {
        let tokens = crate::lexer::lex_line("Pay any amount of life. The amount you pay can't be more than the total number of white nontoken permanents your opponents control plus the total number of white cards in their graveyards.", 0).unwrap();
        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
        let [EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife { maximum: Some(Value::Add(left, right)), .. }), .. })] = effects.as_slice()
            else { panic!("one capped payment required: {effects:?}"); };
        let (Value::Count(controlled), Value::Count(owned)) = (left.as_ref(), right.as_ref())
            else { panic!("two complete object counts required"); };
        assert_eq!(controlled.controller, Some(PlayerFilter::Opponent));
        assert!(controlled.nontoken);
        assert_eq!(controlled.zone, Some(Zone::Battlefield));
        assert_eq!(owned.owner, Some(PlayerFilter::Opponent));
        assert_eq!(owned.zone, Some(Zone::Graveyard));
        assert!(controlled.colors.is_some());
        assert_eq!(controlled.colors, owned.colors);
    }
    #[test]
    fn variable_life_limit_requires_the_complete_followup() {
        let sentences = ["Pay any amount of life.", "The amount you pay can't be more than 7."]
            .into_iter().map(|text| SentenceInput::from_lexed(&crate::lexer::lex_line(text, 0).unwrap())).collect::<Vec<_>>();
        assert!(read(&sentences, 0).is_some());
        let sentences = ["Pay any amount of life.", "The amount you pay can't be more than 7 unless you control a creature."]
            .into_iter().map(|text| SentenceInput::from_lexed(&crate::lexer::lex_line(text, 0).unwrap())).collect::<Vec<_>>();
        assert!(read(&sentences, 0).is_none());
    }

}
