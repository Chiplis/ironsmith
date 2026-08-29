use super::*;

pub fn parse_return_it_then_loses_all_abilities_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(return_tokens) = chain_grammar::split_return_then_loses_tokens(tokens) else {
        return Ok(None);
    };
    let mut effects = parse_effect_chain_inner_lexed(return_tokens)?;
    effects.push(EffectAst::subject_verb_remove_abilities_from_target(
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        Vec::new(),
        Until::Forever,
    ));
    Ok(Some(effects))
}

pub fn remove_first_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    remove_first_may_word_tokens(tokens)
}

pub fn remove_through_first_word(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    remove_through_first_may_word_tokens(tokens)
}
