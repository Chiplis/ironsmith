use super::*;
/// A finite compound restriction sentence. No source/keyword identity is read.
pub(super) fn read(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::TokenWordView::new(tokens).word_refs();
    if tokens.iter().any(|token| token.as_word().is_none() && !token.is_comma() && token.kind != crate::lexer::TokenKind::Period) { return None; }
    if words != ["this","turn","players","cant","search","libraries","bring","cards","from","outside","the","game","into","game","zones","or","move","cards","that","began","the","game","face","down","in","the","command","zone","out","of","the","command","zone"] { return None; }
    crate::word_claims::claim_tokens(tokens);
    Some(vec![
        EffectAst::subject_verb(crate::cards::builders::SubjectVerbRoleAst::Actor, PlayerAst::You,
            SubjectVerbActionAst::Cant { restriction: crate::effect::Restriction::SearchLibraries(PlayerFilter::Any), duration: Until::EndOfTurn, start: crate::effect::RestrictionStart::Immediate, duration_surface: Default::default(), condition: None }),
        EffectAst::subject_verb(crate::cards::builders::SubjectVerbRoleAst::Actor, PlayerAst::You,
            SubjectVerbActionAst::Cant { restriction: crate::effect::Restriction::MoveBetweenZones { from: Zone::OutsideGame, to: None, initial_state: None, cards_only: true }, duration: Until::EndOfTurn, start: crate::effect::RestrictionStart::Immediate, duration_surface: Default::default(), condition: None }),
        EffectAst::subject_verb(crate::cards::builders::SubjectVerbRoleAst::Actor, PlayerAst::You,
            SubjectVerbActionAst::Cant { restriction: crate::effect::Restriction::MoveBetweenZones { from: Zone::Command, to: None, initial_state: Some((Zone::Command, true)), cards_only: true }, duration: Until::EndOfTurn, start: crate::effect::RestrictionStart::Immediate, duration_surface: Default::default(), condition: None }),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_apostrophe_source_uses_lexer_word_contract_and_retains_card_scope() {
        let text = "This turn, players can't search libraries, bring cards from outside the game into game zones, or move cards that began the game face down in the command zone out of the command zone.";
        let tokens=crate::lexer::lex_line(text,0).unwrap();
        let effects=read(&tokens).expect("actual source restriction sentence");
        assert_eq!(effects.len(),3);
        let moves=effects.iter().filter_map(|effect|match effect {
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {action:SubjectVerbActionAst::Cant {restriction:crate::effect::Restriction::MoveBetweenZones {from,initial_state,cards_only,..},..},..})=>Some((*from,*initial_state,*cards_only)),
            _=>None,
        }).collect::<Vec<_>>();
        assert_eq!(moves,vec![(Zone::OutsideGame,None,true),(Zone::Command,Some((Zone::Command,true)),true)]);
        assert!(read(&crate::lexer::lex_line(&format!("{} nonsense",text.trim_end_matches('.')),0).unwrap()).is_none());
    }
}
