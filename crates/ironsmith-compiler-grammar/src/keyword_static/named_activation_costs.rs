use super::*;

pub(super) fn read(tokens:&[OwnedLexToken])->Result<Option<StaticAbility>,CardTextError> {
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
    if tokens.len()<10 || !tokens[0].is_word("activated") || !tokens[1].is_word("abilities")
        || !tokens[2].is_word("named") || !tokens[3].is_quote() {return Ok(None)}
    let Some(close)=tokens[4..].iter().position(|token|token.is_quote()).map(|index|index+4) else {return Ok(None)};
    if close==4 || !tokens.get(close+1).is_some_and(|token|token.is_word("of")){return Ok(None)}
    let label=render_token_slice(&tokens[4..close]).to_ascii_lowercase();
    let mut quoted=false;let mut boundary=None;
    for index in close+2..tokens.len() {
        if tokens[index].is_quote(){quoted=!quoted;continue;}
        if !quoted && tokens[index].is_any_word(&["cost","costs"]) {boundary=Some(index);break;}
    }
    let Some(cost)=boundary else {return Ok(None)};
    let Some((amount,used))=parse_cost_modifier_amount(&tokens[cost+1..]) else{return Ok(None)};
    let Value::Fixed(amount)=amount else{return Ok(None)};
    let Ok(amount)=u32::try_from(amount) else{return Ok(None)};
    let tail=&tokens[cost+1+used..];
    if crate::lexer::token_word_refs(tail)!=["less","to","activate"]{return Ok(None)}
    let filter=parse_object_filter_lexed(&tokens[close+2..cost],false)?;
    crate::word_claims::claim_tokens(&tokens[..close+2]);
    crate::word_claims::claim_tokens(&tokens[cost..]);
    Ok(Some(StaticAbility::reduce_activated_ability_costs(filter,amount,None)
        .with_activated_ability_cost_condition(ironsmith_core::ActivatedAbilityCostCondition::Named(label))))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_named_activation_has_an_independent_graveyard_object_filter() {
        let tokens=crate::lexer::lex_line("Activated abilities named \"revive\" of cards in your graveyard cost {1} less to activate.",0).unwrap();
        let ability=read(&tokens).unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostReduction{filter,condition,..}=ability.payload else {panic!("reduction payload")};
        assert_eq!(filter.zone,Some(Zone::Graveyard));
        assert_eq!(condition,Some(ironsmith_core::ActivatedAbilityCostCondition::Named("revive".into())));
    }
}
