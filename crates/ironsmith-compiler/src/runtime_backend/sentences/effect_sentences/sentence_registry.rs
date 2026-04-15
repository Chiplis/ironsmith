use super::super::lexer::OwnedLexToken;
use super::super::rule_engine::LexClauseView;
use super::sentence_unsupported::diagnose_sentence_unsupported_lexed;
use super::{
    chain_carry::FALLBACK_POST_DIAGNOSTIC_INDEX_LEXED,
    sentence_primitives::{
        PRIMITIVE_POST_DIAGNOSTIC_INDEX_LEXED, PRIMITIVE_PRE_DIAGNOSTIC_INDEX_LEXED,
    },
    special_sentence_family::SPECIAL_PRE_DIAGNOSTIC_INDEX_LEXED,
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
    let view = LexClauseView::from_tokens(tokens);
    for family in [
        &SPECIAL_PRE_DIAGNOSTIC_INDEX_LEXED,
        &PRIMITIVE_PRE_DIAGNOSTIC_INDEX_LEXED,
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
        &PRIMITIVE_POST_DIAGNOSTIC_INDEX_LEXED,
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
