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

/// "If <condition>, have <performer> perform the <label> action": the same
/// performer atom owned by a leading conditional clause.
pub(crate) fn parse_conditional_have_performer(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    // "You may have <performer> perform the <label> action": the same atom
    // owned by an optional clause.
    if tokens.len() > 2 && tokens[0].is_word("you") && tokens[1].is_word("may") && tokens[2].is_word("have") {
        let probe = crate::word_claims::probe_only(|| matches!(parse_have_performer(&tokens[2..]), Ok(Some(_))));
        if !probe { return Ok(None); }
        let Some(body) = parse_have_performer(&tokens[2..])? else { return Ok(None); };
        crate::word_claims::claim_tokens(&tokens[..2]);
        return Ok(Some(EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May { effects: vec![body] })));
    }
    if !tokens.first().is_some_and(|token| token.is_word("if")) { return Ok(None); }
    let Some(comma) = tokens.iter().position(|token| token.is_comma()) else { return Ok(None); };
    if comma < 2 || !tokens.get(comma + 1).is_some_and(|token| token.is_word("have")) { return Ok(None); }
    // Both halves are probed before either keeps its claims.
    let probe = crate::word_claims::probe_only(|| {
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens[1..comma]).is_ok()
            && matches!(parse_have_performer(&tokens[comma + 1..]), Ok(Some(_)))
    });
    if !probe { return Ok(None); }
    let predicate = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&tokens[1..comma])?;
    let Some(body) = parse_have_performer(&tokens[comma + 1..])? else { return Ok(None); };
    crate::word_claims::claim_tokens(&tokens[..1]);
    crate::word_claims::claim_tokens(&tokens[comma..comma + 1]);
    Ok(Some(EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::Conditional {
        predicate,
        if_true: vec![body],
        if_false: Vec::new(),
    })))
}

/// "Perform the <label> action [with <objects>]": the bare named-action
/// instruction, read the same way as a whole sentence and as a nested clause
/// ("If you do, perform the motivate action with creatures given counters
/// this way.").
pub(crate) fn parse_perform(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !tokens.first().is_some_and(|token| token.is_word("perform")) || !words.starts_with(&["perform", "the"]) {
        return None;
    }
    let action = words.iter().position(|word| *word == "action")?;
    if action <= 2 { return None; }
    let label = words[2..action].join(" ");
    let prior_action = match words.get(action + 1..) {
        Some(["with", "cards" | "creatures" | "permanents" | "objects" | "lands", "affected", "this", "way"]) => Some(ironsmith_core::PriorEffectAction::Affected),
        Some(["with", "creatures" | "permanents" | "objects", "tapped", "this", "way"]) => Some(ironsmith_core::PriorEffectAction::Tapped),
        Some(["with", "cards" | "objects", "revealed", "this", "way"]) => Some(ironsmith_core::PriorEffectAction::Revealed),
        Some(["with", "cards" | "creatures" | "permanents" | "objects", "exiled", "this", "way"]) => Some(ironsmith_core::PriorEffectAction::Exiled),
        Some(["with", "cards" | "creatures" | "permanents" | "objects", "sacrificed", "this", "way"]) => Some(ironsmith_core::PriorEffectAction::Sacrificed),
        Some(["with", "creatures" | "permanents" | "objects", "given", "counters", "this", "way"]) => Some(ironsmith_core::PriorEffectAction::CountersPut),
        // The objects a creation instruction affected are the tokens it
        // actually created.
        Some(["with", "tokens" | "permanents" | "objects", "created", "this", "way"]
            | ["with", "the", "token" | "tokens", "created", "this", "way"]) => Some(ironsmith_core::PriorEffectAction::Affected),
        _ => None,
    };
    let objects = if prior_action.is_some() || action + 1 == words.len() { None }
        else if words.get(action + 1) == Some(&"with") {
            let start = *view.token_start_indices().get(action + 2)?;
            Some(crate::util::parse_target_phrase(&tokens[start..]).ok()?)
        } else { return None; };
    if prior_action.is_some() { crate::word_claims::claim_tokens(tokens); }
    crate::word_claims::claim_tokens(&tokens[..view.token_start_indices()[action] + 1]);
    if let Some(with) = view.token_start_indices().get(action + 1) {
        crate::word_claims::claim_tokens(&tokens[*with..*with + 1]);
    }
    Some(EffectAst::subject_verb(
        crate::model::ast::SubjectVerbRoleAst::Actor, PlayerAst::Implicit,
        SubjectVerbActionAst::KeywordActions(crate::cards::builders::KeywordActionAst::PerformNamedAction {
            program_completion: ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,
            label, performer: None, objects,
            prior_objects: prior_action.map(|action| (crate::effect::EffectId::PENDING_PRIOR_ACTION, action)),
            program: None,
        }),
    ))
}

pub(super) fn parse_perform_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    Ok(parse_perform(tokens))
}

#[cfg(test)]
mod tests {
    #[test]
    fn performer_atom_is_shared_by_nested_clause_owners() {
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
