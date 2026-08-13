use crate::cards::builders::CardTextError;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch, UnsupportedReason};
use crate::registry::{
    RegistryCandidate, RegistryRuleMetadata, furthest_committed_diagnostic,
    resolve_registry_candidates,
};
use std::collections::HashMap;

use super::lexer::{OwnedLexToken, TokenKind, TokenWordView, contains_token_kind};

pub(crate) const RULE_SHAPE_HAS_COLON: u32 = 1 << 0;
pub(crate) const RULE_SHAPE_HAS_COMMA: u32 = 1 << 1;
pub(crate) const RULE_SHAPE_HAS_SEMICOLON: u32 = 1 << 2;
pub(crate) const RULE_SHAPE_STARTS_IF: u32 = 1 << 3;
pub(crate) const RULE_SHAPE_STARTS_WHEN: u32 = 1 << 4;
pub(crate) const RULE_SHAPE_STARTS_WHENEVER: u32 = 1 << 5;
pub(crate) const RULE_SHAPE_STARTS_AT: u32 = 1 << 6;
pub(crate) const RULE_SHAPE_STARTS_MAY: u32 = 1 << 7;

#[derive(Debug, Clone)]
pub(crate) struct LexClauseWords<'a>(TokenWordView<'a>);

impl<'a> LexClauseWords<'a> {
    pub(crate) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self(TokenWordView::new(tokens))
    }

    pub(crate) fn first(&self) -> Option<&str> {
        self.0.first()
    }

    pub(crate) fn to_word_refs(&self) -> Vec<&str> {
        self.0.to_word_refs()
    }

    pub(crate) fn join(&self, separator: &str) -> String {
        self.0.join(separator)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LexClauseView<'a> {
    pub(crate) raw: Option<&'a str>,
    pub(crate) tokens: &'a [OwnedLexToken],
    pub(crate) words: LexClauseWords<'a>,
    pub(crate) shape: u32,
}

impl<'a> LexClauseView<'a> {
    pub(crate) fn from_tokens(tokens: &'a [OwnedLexToken]) -> Self {
        let words = LexClauseWords::new(tokens);
        let shape = compute_lex_clause_rule_shape(tokens, &words);
        Self {
            raw: None,
            tokens,
            words,
            shape,
        }
    }

    pub(crate) fn head(&self) -> &str {
        self.words.first().unwrap_or("")
    }

    pub(crate) fn display_text(&self) -> String {
        if let Some(raw) = self.raw {
            raw.trim().to_string()
        } else {
            self.words.join(" ")
        }
    }
}

pub(crate) fn unsupported_rule_error(
    rule_id: &str,
    message: &str,
    subject_label: &str,
    text: &str,
) -> CardTextError {
    CardTextError::ParseError(format!(
        "{message} ({subject_label}: '{text}') [rule={rule_id}]"
    ))
}

fn compute_lex_clause_rule_shape(tokens: &[OwnedLexToken], words: &LexClauseWords) -> u32 {
    let mut shape = 0u32;
    if contains_token_kind(tokens, TokenKind::Colon) {
        shape |= RULE_SHAPE_HAS_COLON;
    }
    if contains_token_kind(tokens, TokenKind::Comma) {
        shape |= RULE_SHAPE_HAS_COMMA;
    }
    if contains_token_kind(tokens, TokenKind::Semicolon) {
        shape |= RULE_SHAPE_HAS_SEMICOLON;
    }
    match words.first().unwrap_or("") {
        "if" => shape |= RULE_SHAPE_STARTS_IF,
        "when" => shape |= RULE_SHAPE_STARTS_WHEN,
        "whenever" => shape |= RULE_SHAPE_STARTS_WHENEVER,
        "at" => shape |= RULE_SHAPE_STARTS_AT,
        "may" => shape |= RULE_SHAPE_STARTS_MAY,
        _ => {}
    }
    shape
}

pub(crate) type LexClauseRuleFn<T> = for<'a> fn(&LexClauseView<'a>) -> ParseOutcome<T>;
pub(crate) type LegacyLexClauseRuleFn<T> =
    for<'a> fn(&LexClauseView<'a>) -> Result<Option<T>, CardTextError>;

#[derive(Clone, Copy)]
pub(crate) enum LexRuleHandler<T> {
    Structured(LexClauseRuleFn<T>),
    Legacy(LegacyLexClauseRuleFn<T>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LexRuleHeadHint {
    Single(&'static str),
    Pair(&'static str, &'static str),
}

#[derive(Debug, Clone)]
pub(crate) struct LexRuleHintIndex {
    by_head: HashMap<&'static str, Vec<usize>>,
    by_head_pair: HashMap<(&'static str, &'static str), Vec<usize>>,
}

pub(crate) fn build_lex_rule_hint_index(
    rule_count: usize,
    mut head_hints_for_rule: impl FnMut(usize) -> Vec<LexRuleHeadHint>,
) -> LexRuleHintIndex {
    let mut by_head = HashMap::<&'static str, Vec<usize>>::new();
    let mut by_head_pair = HashMap::<(&'static str, &'static str), Vec<usize>>::new();
    for idx in 0..rule_count {
        for hint in head_hints_for_rule(idx) {
            match hint {
                LexRuleHeadHint::Single(word) => by_head.entry(word).or_default().push(idx),
                LexRuleHeadHint::Pair(first, second) => {
                    by_head_pair.entry((first, second)).or_default().push(idx);
                }
            }
        }
    }
    LexRuleHintIndex {
        by_head,
        by_head_pair,
    }
}

impl LexRuleHintIndex {
    pub(crate) fn candidate_indices(&self, head: &str, second: Option<&str>) -> Vec<usize> {
        let mut candidate_indices = Vec::new();
        if let Some(second) = second
            && let Some(indices) = self.by_head_pair.get(&(head, second))
        {
            candidate_indices.extend(indices.iter().copied());
        }
        if let Some(indices) = self.by_head.get(head) {
            candidate_indices.extend(indices.iter().copied());
        }
        candidate_indices.sort_unstable();
        candidate_indices.dedup();
        candidate_indices
    }
}

#[derive(Clone, Copy)]
pub(crate) struct LexRuleDef<T> {
    pub(crate) metadata: RegistryRuleMetadata,
    pub(crate) shape_mask: u32,
    pub(crate) run: LexRuleHandler<T>,
}

#[derive(Clone, Copy)]
pub(crate) struct LexRuleIndex<T: 'static> {
    rules: &'static [LexRuleDef<T>],
}

impl<T: 'static> LexRuleIndex<T> {
    pub(crate) const fn new(rules: &'static [LexRuleDef<T>]) -> Self {
        Self { rules }
    }

    pub(crate) fn recognize<'a>(&self, view: &LexClauseView<'a>) -> ParseOutcome<RuleMatch<T>> {
        let candidate_indices = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| lex_rule_matches_view(rule, view))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        let mut candidates = Vec::new();
        let mut diagnostics = Vec::new();
        for idx in candidate_indices {
            let rule = &self.rules[idx];
            let outcome = match rule.run {
                LexRuleHandler::Structured(run) => run(view).within(rule.metadata.id),
                LexRuleHandler::Legacy(run) => ParseOutcome::from_legacy_result_option(
                    rule.metadata.id,
                    lex_clause_span(view),
                    run(view),
                ),
            };
            match outcome {
                ParseOutcome::NoMatch => {}
                ParseOutcome::Match(matched) => {
                    candidates.push(RegistryCandidate::new(
                        rule.metadata,
                        matched.value,
                        matched.span,
                    ));
                }
                ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
            }
        }

        resolve_registry_candidates(RuleId::new("lex-rule-registry"), candidates, diagnostics)
    }
}

fn lex_clause_span(view: &LexClauseView<'_>) -> Option<crate::cards::TextSpan> {
    let first = view.tokens.first()?;
    let last = view.tokens.last()?;
    (first.span.line == last.span.line).then_some(crate::cards::TextSpan {
        line: first.span.line,
        start: first.span.start,
        end: last.span.end,
    })
}

fn lex_rule_matches_view<T>(rule: &LexRuleDef<T>, view: &LexClauseView<'_>) -> bool {
    if !rule.metadata.head.accepts(view.head()) {
        return false;
    }
    if rule.shape_mask == 0 {
        return true;
    }
    (view.shape & rule.shape_mask) == rule.shape_mask
}

pub(crate) type LexUnsupportedPredicate = for<'a> fn(&LexClauseView<'a>) -> bool;

#[derive(Clone, Copy)]
pub(crate) struct LexUnsupportedRuleDef {
    pub(crate) metadata: RegistryRuleMetadata,
    pub(crate) shape_mask: u32,
    pub(crate) message: &'static str,
    pub(crate) predicate: LexUnsupportedPredicate,
}

#[derive(Clone, Copy)]
pub(crate) struct LexUnsupportedDiagnoser {
    rules: &'static [LexUnsupportedRuleDef],
}

impl LexUnsupportedDiagnoser {
    pub(crate) const fn new(rules: &'static [LexUnsupportedRuleDef]) -> Self {
        Self { rules }
    }

    pub(crate) fn diagnose(
        &self,
        view: &LexClauseView<'_>,
        subject_label: &'static str,
    ) -> ParseOutcome<()> {
        let candidate_indices = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| lex_unsupported_rule_matches_view(rule, view))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        let mut diagnostics = Vec::new();
        for idx in candidate_indices {
            let rule = &self.rules[idx];
            if (rule.predicate)(view) {
                let detail = format!(
                    "{} ({}: '{}')",
                    rule.message,
                    subject_label,
                    view.display_text()
                );
                diagnostics.push(ParseDiagnostic::unsupported(
                    rule.metadata.id,
                    lex_clause_span(view),
                    UnsupportedReason::new(rule.metadata.id.as_str(), detail.clone()),
                    detail,
                ));
            }
        }
        furthest_committed_diagnostic(diagnostics)
            .map(ParseOutcome::Error)
            .unwrap_or(ParseOutcome::NoMatch)
    }
}

fn lex_unsupported_rule_matches_view(
    rule: &LexUnsupportedRuleDef,
    view: &LexClauseView<'_>,
) -> bool {
    if !rule.metadata.head.accepts(view.head()) {
        return false;
    }
    if rule.shape_mask == 0 {
        return true;
    }
    (view.shape & rule.shape_mask) == rule.shape_mask
}
