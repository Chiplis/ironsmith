use super::super::rule_engine::{LexClauseView, recognize_lex_rule_indices};
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
use crate::recognition::{ParseOutcome, RuleMatch};
use crate::grammar::effects::{
    SentencePreludeShape, parse_sentence_prelude_shape_tokens,
};
use crate::lexer::OwnedLexToken;

fn run_sentence_rule_family(
    registry: crate::recognition::RuleId,
    indices: &[&super::super::rule_engine::LexRuleIndex<Vec<EffectAst>>],
    view: &LexClauseView<'_>,
) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    recognize_lex_rule_indices(registry, indices, view)
}

pub(super) fn run_sentence_parse_rules_lexed(
    tokens: &[OwnedLexToken],
) -> Result<(&'static str, Vec<EffectAst>), CardTextError> {
    if let Some(diag) = super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens) {
        return Err(diag);
    }
    if let Some(prelude) = parse_sentence_prelude_shape_tokens(tokens) {
        match prelude {
            SentencePreludeShape::XCantBeZero => {
                return Ok(("x_cant_be_zero_activation_restriction", Vec::new()));
            }
            SentencePreludeShape::RollDiceChooseOneResult {
                count,
                sides,
                die_text,
            } => {
                return Ok((
                    "roll_dice_choose_one_result",
                    vec![
                        EffectAst::subject_verb_roll_dice_choose_result_with_die_text(
                            crate::cards::builders::PlayerAst::Implicit,
                            count,
                            sides,
                            Some(die_text),
                        ),
                    ],
                ));
            }
        }
    }

    let view = LexClauseView::from_tokens(tokens);
    match run_sentence_rule_family(
        crate::recognition::RuleId::new("effect-sentence-pre-diagnostic-registry"),
        &[
            &SUBJECT_VERB_PRE_DIAGNOSTIC_INDEX_LEXED,
            &SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_INDEX_LEXED,
        ],
        &view,
    ) {
        ParseOutcome::Match(matched) => {
            let matched = matched.value;
            return Ok((matched.rule.as_str(), matched.value));
        }
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => {
            if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
                return Err(diag);
            }
            return Err(diagnostic.into_legacy_error());
        }
    }

    if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }

    match run_sentence_rule_family(
        crate::recognition::RuleId::new("effect-sentence-post-diagnostic-registry"),
        &[
            &SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_INDEX_LEXED,
            &FALLBACK_POST_DIAGNOSTIC_INDEX_LEXED,
        ],
        &view,
    ) {
        ParseOutcome::Match(matched) => {
            let matched = matched.value;
            return Ok((matched.rule.as_str(), matched.value));
        }
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_legacy_error()),
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing sentence parse rule for clause: '{}'",
        view.display_text()
    )))
}
