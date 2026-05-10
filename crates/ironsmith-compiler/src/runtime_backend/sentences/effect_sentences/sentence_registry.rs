use super::super::lexer::OwnedLexToken;
use super::super::rule_engine::LexClauseView;
use super::sentence_unsupported::diagnose_sentence_unsupported_lexed;
use super::{
    chain_carry::FALLBACK_POST_DIAGNOSTIC_INDEX_LEXED,
    subject_verb_primitives::{
        SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_INDEX_LEXED,
        SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_INDEX_LEXED,
    },
    subject_verb_special_recognizers::SUBJECT_VERB_PRE_DIAGNOSTIC_INDEX_LEXED,
};
use crate::cards::builders::{CardTextError, EffectAst};

fn run_sentence_rule_family(
    index: &'static super::super::rule_engine::LexRuleIndex<Vec<EffectAst>>,
    view: &LexClauseView<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    index.run_first(view)
}

pub(super) fn run_sentence_parse_rules_lexed(
    tokens: &[OwnedLexToken],
) -> Result<(&'static str, Vec<EffectAst>), CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words == ["x", "cant", "be", "0"] || words == ["x", "can't", "be", "0"] {
        return Ok(("x_cant_be_zero_activation_restriction", Vec::new()));
    }

    let view = LexClauseView::from_tokens(tokens);
    for family in [
        &SUBJECT_VERB_PRE_DIAGNOSTIC_INDEX_LEXED,
        &SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_INDEX_LEXED,
    ] {
        match run_sentence_rule_family(family, &view) {
            Ok(Some((rule_id, effects))) => return Ok((rule_id, effects)),
            Ok(None) => {}
            Err(parse_err) => {
                if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
                    return Err(diag);
                }
                return Err(parse_err);
            }
        }
    }

    if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }

    for family in [
        &SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_INDEX_LEXED,
        &FALLBACK_POST_DIAGNOSTIC_INDEX_LEXED,
    ] {
        if let Some((rule_id, effects)) = run_sentence_rule_family(family, &view)? {
            return Ok((rule_id, effects));
        }
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing sentence parse rule for clause: '{}'",
        view.display_text()
    )))
}
