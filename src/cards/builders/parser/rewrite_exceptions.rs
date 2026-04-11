use crate::cards::builders::{CardTextError, LineAst};

use super::ir::{RewriteKeywordLine, RewriteStatementLine, RewriteTriggeredLine};
use super::lexer::OwnedLexToken;
use super::lower;

pub(crate) fn lower_rewrite_statement_token_groups_to_chunks(
    info: super::shared_types::LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    lower::lower_rewrite_statement_token_groups_to_chunks(info, text, parse_tokens, parse_groups)
}

pub(crate) fn lower_rewrite_triggered_to_chunk(
    info: super::shared_types::LineInfo,
    full_text: &str,
    full_parse_tokens: &[OwnedLexToken],
    trigger_text: &str,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_text: &str,
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<crate::cards::builders::PredicateAst>,
    max_triggers_per_turn: Option<u32>,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower::lower_rewrite_triggered_to_chunk(
        info,
        full_text,
        full_parse_tokens,
        trigger_text,
        trigger_parse_tokens,
        effect_text,
        effect_parse_tokens,
        intervening_if,
        max_triggers_per_turn,
        chosen_option_label,
    )
}

pub(crate) fn apply_statement_rewrite_exceptions(
    line: &RewriteStatementLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<LineAst>>, CardTextError> {
    if let Some(unsupported_chunk) = lower::lower_rewrite_statement_to_unsupported_chunk(line) {
        return Ok(Some(vec![unsupported_chunk]));
    }
    if let Some(chunk) = lower::lower_rewrite_pact_statement_to_chunk(line, parse_tokens)? {
        return Ok(Some(vec![chunk]));
    }
    if let Some(chunk) = lower::lower_rewrite_soul_partition_statement_to_chunk(line, parse_tokens)?
    {
        return Ok(Some(vec![chunk]));
    }
    if let Some(chunk) = lower::lower_rewrite_divvy_statement_to_chunk(line, parse_tokens)? {
        return Ok(Some(vec![chunk]));
    }
    if let Some(chunk) = lower::lower_rewrite_empty_laboratory_statement_to_chunk(line, parse_tokens)?
    {
        return Ok(Some(vec![chunk]));
    }
    if let Some(chunk) = lower::lower_rewrite_shape_anew_statement_to_chunk(line, parse_tokens)? {
        return Ok(Some(vec![chunk]));
    }
    if let Some(chunk) =
        lower::lower_rewrite_nissas_encouragement_statement_to_chunk(line, parse_tokens)?
    {
        return Ok(Some(vec![chunk]));
    }
    Ok(None)
}

pub(crate) fn apply_triggered_rewrite_exceptions(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    lower::lower_special_rewrite_triggered_chunk(line, trigger_parse_tokens, effect_parse_tokens)
}

pub(crate) fn apply_keyword_rewrite_exceptions(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = lower::try_lower_optional_cost_with_cast_trigger(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = lower::try_lower_optional_behold_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}
