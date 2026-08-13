use crate::cards::builders::{CardTextError, LineAst};
use crate::recognition::{ParseOutcome, RuleId};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

use super::cst::KeywordLineCst;
use super::grammar::document_shapes::{LabelPrefixKind, parse_label_prefix_kind_tokens};
use super::ir::RewriteKeywordLine;
use super::keyword_families::{keyword_line_rules, parse_keyword_dispatch_hint};
use super::lexer::OwnedLexToken;
use super::preprocess::PreprocessedLine;
use super::token_primitives::split_em_dash_label_prefix_tokens;

pub(super) use super::keyword_payloads::*;

pub(crate) fn parse_keyword_line_cst(
    line: &PreprocessedLine,
) -> Result<Option<KeywordLineCst>, CardTextError> {
    recognize_keyword_line_cst(line).into_legacy_result_option()
}

pub(crate) fn recognize_keyword_line_cst(
    line: &PreprocessedLine,
) -> ParseOutcome<KeywordLineCst> {
    let tokens = rewrite_keyword_dash_parse_tokens(&line.tokens);
    let full_parse_tokens = line.info.source_tokens.clone();
    let Some(hint) = parse_keyword_dispatch_hint(&tokens) else {
        return ParseOutcome::NoMatch;
    };
    let rules = keyword_line_rules();
    let span = crate::runtime_backend::span_from_tokens(&tokens);
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();

    for rule in &rules {
        if !rule.hints.iter().any(|candidate| candidate == &hint) {
            continue;
        }
        match ParseOutcome::from_legacy_result_option(
            rule.id,
            span,
            (rule.parse)(line, &tokens, &full_parse_tokens),
        ) {
            ParseOutcome::Match(matched) => {
                candidates.push(RegistryCandidate::new(
                    RegistryRuleMetadata::distinct(rule.id, HeadDiscriminator::Any),
                    (rule.cst_kind, matched.value),
                    matched.span,
                ));
            }
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    match resolve_registry_candidates(
        RuleId::new("keyword-line-registry"),
        candidates,
        diagnostics,
    ) {
        ParseOutcome::Match(matched) => {
            let rule_match = matched.value;
            let (kind, payload) = rule_match.value;
            ParseOutcome::matched(
                KeywordLineCst {
                    info: line.info.clone(),
                    parse_tokens: tokens,
                    full_parse_tokens,
                    kind,
                    payload,
                },
                rule_match.span,
            )
        }
        ParseOutcome::NoMatch => ParseOutcome::NoMatch,
        ParseOutcome::Error(diagnostic) => ParseOutcome::Error(diagnostic),
    }
}
pub(crate) fn lower_keyword_line_ast(line: &RewriteKeywordLine) -> Result<LineAst, CardTextError> {
    Ok(line.payload.to_line_ast())
}

#[cfg(test)]
pub(crate) fn parse_keyword_payload_for_kind(
    mut info: crate::cards::builders::LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    full_parse_tokens: &[OwnedLexToken],
    kind: super::cst::KeywordLineKindCst,
) -> Result<LineAst, CardTextError> {
    info.normalized.normalized = text.to_string();
    let line = PreprocessedLine {
        info,
        tokens: parse_tokens.to_vec(),
    };
    let rule = keyword_line_rules()
        .into_iter()
        .find(|rule| rule.cst_kind == kind)
        .ok_or_else(|| {
            CardTextError::InvariantViolation(format!("no keyword parser registered for {kind:?}"))
        })?;
    let payload = (rule.parse)(&line, parse_tokens, full_parse_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "keyword parser for {kind:?} did not recognize '{}'",
            line.info.raw_line
        ))
    })?;
    Ok(payload.to_line_ast())
}

pub(crate) fn rewrite_keyword_dash_parse_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let Some((label_tokens, body_tokens)) = split_em_dash_label_prefix_tokens(tokens) else {
        return tokens.to_vec();
    };

    match parse_label_prefix_kind_tokens(label_tokens) {
        Some(LabelPrefixKind::CouncilChoice) => return body_tokens.to_vec(),
        Some(LabelPrefixKind::PreservedKeyword(_)) => {
            let mut rewritten = Vec::with_capacity(label_tokens.len() + body_tokens.len());
            rewritten.extend(label_tokens.iter().cloned());
            rewritten.extend(body_tokens.iter().cloned());
            return rewritten;
        }
        None => {}
    }

    tokens.to_vec()
}
