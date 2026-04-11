use super::super::lexer::OwnedLexToken;
use super::super::rule_engine::{LexClauseView, LexRuleDef, LexRuleIndex, RULE_SHAPE_STARTS_IF};
use super::dispatch_inner as inner;
use super::sentence_unsupported::diagnose_sentence_unsupported_lexed;
use crate::cards::builders::{CardTextError, EffectAst};

const SENTENCE_PRE_DIAGNOSTIC_PARSE_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 5] = [
    LexRuleDef {
        id: "redirect-next-damage",
        priority: 100,
        heads: &["the"],
        shape_mask: 0,
        run: inner::parse_redirect_next_damage_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "prevent-next-time-damage",
        priority: 110,
        heads: &["the"],
        shape_mask: 0,
        run: inner::parse_prevent_next_time_damage_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "scale-target-power",
        priority: 120,
        heads: &["double", "triple"],
        shape_mask: 0,
        run: inner::parse_scaled_target_power_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "spell-this-way-pay-life",
        priority: 130,
        heads: &["if"],
        shape_mask: RULE_SHAPE_STARTS_IF,
        run: inner::parse_spell_this_way_pay_life_rule_lexed,
    },
    LexRuleDef {
        id: "preconditional-primitives",
        priority: 135,
        heads: inner::SENTENCE_PRIMITIVE_RULE_HEADS,
        shape_mask: 0,
        run: inner::parse_preconditional_sentence_primitives_rule_lexed,
    },
];

const SENTENCE_POST_DIAGNOSTIC_PARSE_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 2] = [
    LexRuleDef {
        id: "postconditional-primitives",
        priority: 160,
        heads: inner::SENTENCE_PRIMITIVE_RULE_HEADS,
        shape_mask: 0,
        run: inner::parse_postconditional_sentence_primitives_rule_lexed,
    },
    LexRuleDef {
        id: "effect-chain",
        priority: 170,
        heads: &[],
        shape_mask: 0,
        run: inner::parse_effect_chain_rule_lexed,
    },
];

const SENTENCE_PRE_DIAGNOSTIC_PARSE_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SENTENCE_PRE_DIAGNOSTIC_PARSE_RULES_LEXED);
const SENTENCE_POST_DIAGNOSTIC_PARSE_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SENTENCE_POST_DIAGNOSTIC_PARSE_RULES_LEXED);

pub(super) fn run_sentence_parse_rules_lexed(
    tokens: &[OwnedLexToken],
) -> Result<(&'static str, Vec<EffectAst>), CardTextError> {
    let view = LexClauseView::from_tokens(tokens);
    match SENTENCE_PRE_DIAGNOSTIC_PARSE_INDEX_LEXED.run_first(&view) {
        Ok(Some((rule_id, effects))) => return Ok((rule_id, effects)),
        Ok(None) => {}
        Err(parse_err) => {
            if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
                return Err(diag);
            }
            return Err(parse_err);
        }
    }

    if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }

    if let Some((rule_id, effects)) = SENTENCE_POST_DIAGNOSTIC_PARSE_INDEX_LEXED.run_first(&view)?
    {
        return Ok((rule_id, effects));
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing sentence parse rule for clause: '{}'",
        view.display_text()
    )))
}
