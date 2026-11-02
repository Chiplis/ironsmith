use super::*;


// A fixed additive modification of the original loss event. Keep its
// affected-player scope; this is not a replacement effect program that
// recursively creates another loss event or changes its cause.
fn parse_additive_life_loss(tokens:&[OwnedLexToken])->Option<StaticAbility>{
    let tokens=trim_edge_punctuation_tokens(tokens);
    let comma=tokens.iter().position(OwnedLexToken::is_comma)?;
    let head=parser_token_word_refs(&tokens[..comma]);
    let player=match head.as_slice(){
        ["if","you","would","lose","life"]=>PlayerFilter::You,
        ["if","a","player","would","lose","life"]=>PlayerFilter::Any,
        ["if","an","opponent","would","lose","life"]=>PlayerFilter::Opponent,
        _=>return None,
    };
    let body=parser_token_word_refs(&tokens[comma+1..]);
    let rest=match (player.clone(),body.as_slice()){
        (PlayerFilter::You,["you","lose",rest @ ..])=>rest,
        (PlayerFilter::Any|PlayerFilter::Opponent,["they","lose",rest @ ..])=>rest,
        (PlayerFilter::Any|PlayerFilter::Opponent,["that","player","loses",rest @ ..])=>rest,
        _=>return None,
    };
    let ["that","much","life","plus",amount,"instead"]=rest else{return None};
    let additional=i32::try_from(crate::util::parse_number_word_u32(amount)?).ok()?;
    crate::word_claims::claim_tokens(tokens);
    Some(StaticAbility::add_life_change_replacement(player,additional,true,render_token_slice(tokens)))
}

pub fn parse_if_you_would_gain_life_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(ability)=parse_additive_life_loss(tokens){return Ok(Some(ability));}
    let Some(shape) = keyword_static_lines::parse_gain_life_replacement_tokens(tokens)
    else { return Ok(None); };
    let display = render_token_slice(tokens);
    let ability = match shape.amount {
        keyword_static_lines::GainLifeReplacementAmount::Add(additional) =>
            StaticAbility::add_life_gain_replacement(PlayerFilter::You, additional, display),
        keyword_static_lines::GainLifeReplacementAmount::Double =>
            StaticAbility::double_life_change_replacement(PlayerFilter::You, false, display),
    };
    Ok(Some(match shape.condition_tokens {
        Some(condition) => ability.with_condition(parse_static_condition_clause(condition)?),
        None => ability,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    #[test]
    fn complete_life_gain_replacements_keep_addition_and_live_condition() {
        let parsed = parse_if_you_would_gain_life_replacement_line(&lex_line("If you would gain life, you gain that much life plus 1 instead.", 0).unwrap()).unwrap().unwrap();
        assert!(matches!(parsed.payload, ironsmith_core::StaticAbilityPayload::AddLifeGainReplacement { additional: 1, player: PlayerFilter::You, .. }));
        let parsed = parse_if_you_would_gain_life_replacement_line(&lex_line("If you would gain life while you have 5 or less life, you gain twice that much life instead.", 0).unwrap()).unwrap().unwrap();
        assert!(matches!(parsed.payload, ironsmith_core::StaticAbilityPayload::Conditional { .. }));
        for text in [
            "If you would gain life, an opponent gains that much life plus 1 instead.",
            "If you would gain life, you gain that much life plus 1 instead. Draw a card.",
            "If you would gain life, you gain that much life plus 1 instead unless an opponent pays 1 life.",
            "If you would gain life, you gain that much life plus 4294967295 instead.",
        ] {
            assert!(!matches!(parse_if_you_would_gain_life_replacement_line(&lex_line(text, 0).unwrap()), Ok(Some(_))), "{text}");
        }
    }
    #[test]
    fn conditional_draw_requires_one_complete_instead_marker() {
        for text in [
            "If you would draw a card while you have no cards in hand, draw two cards instead.",
            "If you would draw a card while you have no cards in hand, instead draw two cards.",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let shape = late_static_facts::parse_conditional_draw_replacement_tokens(&tokens).unwrap();
            assert_eq!(shape.draw_count, 2);
            assert!(crate::grammar::conditions::parse_player_cards_in_hand_condition(shape.condition_tokens).unwrap().is_no_cards_in_hand());
        }
        for text in [
            "If you would draw a card while you have no cards in hand, draw two cards.",
            "If you would draw a card while you have no cards in hand, instead draw two cards instead.",
        ] { assert!(late_static_facts::parse_conditional_draw_replacement_tokens(&lex_line(text, 0).unwrap()).is_none()); }
    }
}


#[cfg(test)]
mod additive_loss_tests {
 use super::*;
 #[test]
 fn additive_loss_keeps_affected_player_and_rejects_a_changed_recipient_or_incomplete_amount(){
  let tokens=crate::lexer::lex_line("If an opponent would lose life, they lose that much life plus 1 instead.",0).unwrap();
  let rule=parse_if_you_would_gain_life_replacement_line(&tokens).unwrap().unwrap();
  assert!(matches!(rule.payload,ironsmith_core::StaticAbilityPayload::AddLifeGainReplacement{player:PlayerFilter::Opponent,additional:1,loss:true,..}));
  for text in["If an opponent would lose life, you lose that much life plus 1 instead.","If an opponent would lose life, they lose that much life plus 1.","If an opponent would lose life, they lose that much life plus 4294967295 instead.","If an opponent would lose life, they lose that much life plus 1 instead. Draw a card."]{
   let tokens=crate::lexer::lex_line(text,0).unwrap();assert!(parse_if_you_would_gain_life_replacement_line(&tokens).unwrap().is_none(),"{text}");
  }
 }
}
