//! Immediate opponent payment offers and independently named failure receipts.
use super::*;
pub(crate) fn read(tokens:&[OwnedLexToken])->Result<Option<Vec<StaticAbilityAst>>,CardTextError> {
    let view=TokenWordView::new(tokens);let words=view.word_refs();
    let prefix=["as","you","cast","this","spell","an","opponent","of","your","choice","may","pay"];
    if !words.starts_with(&prefix) {return Ok(None)}
    let Some(start)=view.token_start_indices().get(prefix.len()).copied() else{return Ok(None)};
    let Some((amount,used))=crate::util::parse_number(&tokens[start..]) else{return Ok(None)};
    let tail=&tokens[start+used..];let rest=TokenWordView::new(tail);let rest_words=rest.word_refs();
    if !rest_words.starts_with(&["life"]) {return Ok(None)}
    // This independent clause declares the receipt's authored label; no
    // mechanic name is known to the runtime or this construction owner.
    // TokenWordView owns parser words: apostrophes are omitted by the lexer,
    // while authored source slices remain unchanged.
    let suffix=&rest_words[1..];
    let label=if let Some(label)=suffix.strip_prefix(&["if","that","player","doesnt","they","fail","the"]) {label}
        else if let Some(label)=suffix.strip_prefix(&["if","they","dont","they","fail","the"]) {label}
        else{return Ok(None)};
    if label.is_empty() || label.iter().any(|word|!word.chars().all(|ch|ch.is_alphabetic())) {return Ok(None)}
    let label=label.join(" ");
    let choose=EffectAst::subject_verb_choose_player(
        crate::cards::builders::PlayerAst::You,PlayerFilter::Opponent,
        crate::tag::TagRef::of("announcement_payment_payer"),false,0);
    let pay=EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
        crate::cards::builders::PlayerAst::That,
        crate::cards::builders::SubjectVerbActionAst::LifeResources(
            crate::cards::builders::LifeResourceActionAst::PayLife {
                amount:crate::effect::Value::Fixed(amount as i32),
            }));
    let payment=EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::MayPay {
        player:crate::cards::builders::PlayerAst::That,effects:vec![pay],payment_receipt:Some(label),
    });
    // One lexical sequence keeps the payer choice visible while the generic
    // static-model adapter lowers each independently owned program child.
    let program=ironsmith_core::ResolutionProgram::from_effects(vec![EffectAst::Sequence {effects:vec![choose,payment]}]);
    crate::word_claims::claim_tokens(tokens);
    Ok(Some(vec![StaticAbilityAst::Static(StaticAbility::cast_announcement_program(program))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declaration_retains_ast_payer_and_complete_payment_label_until_lowering(){
        let tokens=crate::lexer::lex_line("As you cast this spell, an opponent of your choice may pay 4 life. If that player doesn't, they fail the moon trial.",0).unwrap();
        let parsed=read(&tokens).unwrap().unwrap();
        let StaticAbilityAst::Static(rule)=&parsed[0] else{panic!("static announcement program")};
        let ironsmith_core::StaticAbilityPayload::CastAnnouncementProgram{program}=&rule.payload else{panic!("announcement payload")};
        let roots=program.all_effects();
        let [EffectAst::Sequence{effects}]=roots.as_slice() else{panic!("payer choice and payment share one lexical program")};
        assert_eq!(effects.len(),2);
        assert!(matches!(&effects[0],EffectAst::SubjectVerb(_)));
        assert!(matches!(&effects[1],EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::MayPay{player:crate::cards::builders::PlayerAst::That,payment_receipt:Some(label),..})if label=="moon trial"));
    }
}

#[cfg(test)]
mod normalized_failure_clause_words {
    use super::*;
    #[test]
    fn authored_contractions_share_parser_words_but_keep_arbitrary_failure_identity() {
        for clause in ["If that player doesn't, they fail the silver test.","If that player doesn’t, they fail the silver test.","If they don't, they fail the silver test."] {
            let text=format!("As you cast this spell, an opponent of your choice may pay 2 life. {clause}");
            let tokens=crate::lexer::lex_line(&text,0).unwrap();
            assert!(read(&tokens).unwrap().is_some(),"{text}");
        }
    }
}
