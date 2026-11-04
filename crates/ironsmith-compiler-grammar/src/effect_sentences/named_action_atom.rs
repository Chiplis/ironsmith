//! One typed named-action instruction, shared by full sentences and nested
//! conditional/may/entry clauses. The recipient and label keep separate readers.
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst, SubjectVerbActionAst};
use crate::lexer::{OwnedLexToken, TokenWordView};

pub(super) fn parse_have_performer(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if words.first() != Some(&"have") { return Ok(None); }
    let Some(perform) = words.windows(2).position(|part| part == ["perform", "the"]) else { return Ok(None); };
    let Some(action)=words.iter().enumerate().skip(perform+2).find_map(|(index,word)|(*word=="action").then_some(index))else{return Ok(None);};
    if perform <= 1 || perform + 2 >= action { return Ok(None); }
    let prior_objects=if action+1==words.len(){None}else{
        let ["with","objects",participle,"this","way"]=&words[action+1..]else{return Ok(None);};
        let Some((kind,start))=crate::grammar::shared_util::value_helper_shapes::parse_prior_effect_action(&[*participle])else{return Ok(None);};
        if start!=0{return Ok(None);}
        Some((crate::effect::EffectId::PENDING_PRIOR_ACTION,kind))
    };
    let end = view.token_start_indices()[perform];
    let performer = crate::util::parse_target_phrase(&tokens[1..end])?;
    let label = words[perform + 2..action].join(" ");
    crate::word_claims::claim_tokens(&tokens[..1]);
    crate::word_claims::claim_tokens(&tokens[end..]);
    Ok(Some(EffectAst::subject_verb(
        crate::model::ast::SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::KeywordActions(crate::cards::builders::KeywordActionAst::PerformNamedAction {
            program_completion: ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,
            label, performer: Some(performer), objects: None, prior_objects, program: None,
        }),
    )))
}

#[cfg(test)]
mod tests {
    #[test]
    fn performer_atom_is_shared_by_nested_clause_owners() {
        let probe = crate::lexer::lex_line("If you control a creature, draw a card.", 0).unwrap();
        eprintln!("P2PROBE ifclause {:?}", crate::effect_sentences::clause_dispatch::parse_effect_clause(&probe).map(|e| format!("{e:?}").chars().take(400).collect::<String>()));
        for text in [
            "Have this permanent perform the cadence action.",
            "If you control a creature, have this permanent perform the cadence action.",
            "You may have this permanent perform the cadence action.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let effect = crate::effect_sentences::clause_dispatch::parse_effect_clause(&tokens).unwrap();
            let structure = format!("{effect:?}");
            assert!(structure.contains("PerformNamedAction"), "{text}: {structure}");
            assert!(structure.contains("cadence"), "{text}: {structure}");
        }
    }

    #[test]
    fn performer_atom_requires_its_complete_action_suffix() {
        for text in [
            "Have this permanent perform the action.",
            "Have this permanent perform the cadence action then draw a card.",
            "Have this permanent perform cadence.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(super::parse_have_performer(&tokens).unwrap().is_none(), "{text}");
        }
    }
}
