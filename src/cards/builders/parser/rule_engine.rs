#![allow(dead_code)]

use crate::cards::builders::CardTextError;

use super::lexer::{OwnedLexToken, TokenKind, TokenWordView, token_word_refs};

pub(crate) const RULE_SHAPE_HAS_COLON: u32 = 1 << 0;
pub(crate) const RULE_SHAPE_HAS_COMMA: u32 = 1 << 1;
pub(crate) const RULE_SHAPE_HAS_SEMICOLON: u32 = 1 << 2;
pub(crate) const RULE_SHAPE_STARTS_IF: u32 = 1 << 3;
pub(crate) const RULE_SHAPE_STARTS_WHEN: u32 = 1 << 4;
pub(crate) const RULE_SHAPE_STARTS_WHENEVER: u32 = 1 << 5;
pub(crate) const RULE_SHAPE_STARTS_AT: u32 = 1 << 6;
pub(crate) const RULE_SHAPE_STARTS_MAY: u32 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuleKey<'a> {
    pub(crate) head: &'a str,
    pub(crate) shape: u32,
}

impl<'a> RuleKey<'a> {
    pub(crate) fn new(head: &'a str, shape: u32) -> Self {
        Self { head, shape }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClauseView<'a> {
    pub(crate) raw: Option<&'a str>,
    pub(crate) tokens: &'a [OwnedLexToken],
    pub(crate) words: Vec<&'a str>,
    pub(crate) key: RuleKey<'a>,
}

impl<'a> ClauseView<'a> {
    pub(crate) fn from_tokens(tokens: &'a [OwnedLexToken]) -> Self {
        let words = token_word_refs(tokens);
        let head = words.first().copied().unwrap_or("");
        let shape = clause_shape(tokens, &words);
        Self {
            raw: None,
            tokens,
            words,
            key: RuleKey::new(head, shape),
        }
    }

    pub(crate) fn display_text(&self) -> String {
        if let Some(raw) = self.raw {
            raw.trim().to_string()
        } else {
            self.words.join(" ")
        }
    }
}

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
        let shape = clause_shape_lexed(tokens, &words);
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

pub(crate) fn unsupported_rule_error_for_view(
    rule_id: &str,
    message: &str,
    subject_label: &str,
    view: &ClauseView<'_>,
) -> CardTextError {
    let text = view.display_text();
    unsupported_rule_error(rule_id, message, subject_label, &text)
}

fn clause_shape(tokens: &[OwnedLexToken], words: &[&str]) -> u32 {
    let mut shape = 0u32;
    if tokens.iter().any(|token| token.is_colon()) {
        shape |= RULE_SHAPE_HAS_COLON;
    }
    if tokens.iter().any(|token| token.is_comma()) {
        shape |= RULE_SHAPE_HAS_COMMA;
    }
    if tokens.iter().any(|token| token.is_semicolon()) {
        shape |= RULE_SHAPE_HAS_SEMICOLON;
    }
    match words.first().copied().unwrap_or("") {
        "if" => shape |= RULE_SHAPE_STARTS_IF,
        "when" => shape |= RULE_SHAPE_STARTS_WHEN,
        "whenever" => shape |= RULE_SHAPE_STARTS_WHENEVER,
        "at" => shape |= RULE_SHAPE_STARTS_AT,
        "may" => shape |= RULE_SHAPE_STARTS_MAY,
        _ => {}
    }
    shape
}

fn clause_shape_lexed(tokens: &[OwnedLexToken], words: &LexClauseWords) -> u32 {
    let mut shape = 0u32;
    if tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Colon))
    {
        shape |= RULE_SHAPE_HAS_COLON;
    }
    if tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Comma))
    {
        shape |= RULE_SHAPE_HAS_COMMA;
    }
    if tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Semicolon))
    {
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

pub(crate) type ClauseRuleFn<T> = for<'a> fn(&ClauseView<'a>) -> Result<Option<T>, CardTextError>;
pub(crate) type LexClauseRuleFn<T> =
    for<'a> fn(&LexClauseView<'a>) -> Result<Option<T>, CardTextError>;

#[derive(Clone, Copy)]
pub(crate) struct RuleDef<T> {
    pub(crate) id: &'static str,
    pub(crate) priority: u16,
    pub(crate) heads: &'static [&'static str],
    pub(crate) shape_mask: u32,
    pub(crate) run: ClauseRuleFn<T>,
}

#[derive(Clone, Copy)]
pub(crate) struct LexRuleDef<T> {
    pub(crate) id: &'static str,
    pub(crate) priority: u16,
    pub(crate) heads: &'static [&'static str],
    pub(crate) shape_mask: u32,
    pub(crate) run: LexClauseRuleFn<T>,
}

#[derive(Clone, Copy)]
pub(crate) struct RuleIndex<T: 'static> {
    rules: &'static [RuleDef<T>],
}

#[derive(Clone, Copy)]
pub(crate) struct LexRuleIndex<T: 'static> {
    rules: &'static [LexRuleDef<T>],
}

impl<T: 'static> RuleIndex<T> {
    pub(crate) const fn new(rules: &'static [RuleDef<T>]) -> Self {
        Self { rules }
    }

    pub(crate) fn run_first<'a>(
        &self,
        view: &ClauseView<'a>,
    ) -> Result<Option<(&'static str, T)>, CardTextError> {
        let mut candidate_indices = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule_matches_view(rule, view))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        candidate_indices.sort_by_key(|idx| self.rules[*idx].priority);
        for idx in candidate_indices {
            let rule = &self.rules[idx];
            if let Some(result) = (rule.run)(view)? {
                return Ok(Some((rule.id, result)));
            }
        }

        Ok(None)
    }
}

impl<T: 'static> LexRuleIndex<T> {
    pub(crate) const fn new(rules: &'static [LexRuleDef<T>]) -> Self {
        Self { rules }
    }

    pub(crate) fn run_first<'a>(
        &self,
        view: &LexClauseView<'a>,
    ) -> Result<Option<(&'static str, T)>, CardTextError> {
        let mut candidate_indices = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| lex_rule_matches_view(rule, view))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        candidate_indices.sort_by_key(|idx| self.rules[*idx].priority);
        for idx in candidate_indices {
            let rule = &self.rules[idx];
            if let Some(result) = (rule.run)(view)? {
                return Ok(Some((rule.id, result)));
            }
        }

        Ok(None)
    }
}

fn rule_matches_view<T>(rule: &RuleDef<T>, view: &ClauseView<'_>) -> bool {
    let head_matches = rule.heads.is_empty()
        || rule
            .heads
            .iter()
            .any(|candidate| *candidate == view.key.head);
    if !head_matches {
        return false;
    }
    if rule.shape_mask == 0 {
        return true;
    }
    (view.key.shape & rule.shape_mask) == rule.shape_mask
}

fn lex_rule_matches_view<T>(rule: &LexRuleDef<T>, view: &LexClauseView<'_>) -> bool {
    let head_matches =
        rule.heads.is_empty() || rule.heads.iter().any(|candidate| *candidate == view.head());
    if !head_matches {
        return false;
    }
    if rule.shape_mask == 0 {
        return true;
    }
    (view.shape & rule.shape_mask) == rule.shape_mask
}

pub(crate) type UnsupportedPredicate = for<'a> fn(&ClauseView<'a>) -> bool;
pub(crate) type LexUnsupportedPredicate = for<'a> fn(&LexClauseView<'a>) -> bool;

#[derive(Clone, Copy)]
pub(crate) struct UnsupportedRuleDef {
    pub(crate) id: &'static str,
    pub(crate) priority: u16,
    pub(crate) heads: &'static [&'static str],
    pub(crate) shape_mask: u32,
    pub(crate) message: &'static str,
    pub(crate) predicate: UnsupportedPredicate,
}

#[derive(Clone, Copy)]
pub(crate) struct LexUnsupportedRuleDef {
    pub(crate) id: &'static str,
    pub(crate) priority: u16,
    pub(crate) heads: &'static [&'static str],
    pub(crate) shape_mask: u32,
    pub(crate) message: &'static str,
    pub(crate) predicate: LexUnsupportedPredicate,
}

#[derive(Clone, Copy)]
pub(crate) struct UnsupportedDiagnoser {
    rules: &'static [UnsupportedRuleDef],
}

#[derive(Clone, Copy)]
pub(crate) struct LexUnsupportedDiagnoser {
    rules: &'static [LexUnsupportedRuleDef],
}

impl UnsupportedDiagnoser {
    pub(crate) const fn new(rules: &'static [UnsupportedRuleDef]) -> Self {
        Self { rules }
    }

    pub(crate) fn diagnose(
        &self,
        view: &ClauseView<'_>,
        subject_label: &'static str,
    ) -> Option<CardTextError> {
        let mut candidate_indices = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| unsupported_rule_matches_view(rule, view))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        candidate_indices.sort_by_key(|idx| self.rules[*idx].priority);
        for idx in candidate_indices {
            let rule = &self.rules[idx];
            if (rule.predicate)(view) {
                return Some(unsupported_rule_error_for_view(
                    rule.id,
                    rule.message,
                    subject_label,
                    view,
                ));
            }
        }
        None
    }
}

impl LexUnsupportedDiagnoser {
    pub(crate) const fn new(rules: &'static [LexUnsupportedRuleDef]) -> Self {
        Self { rules }
    }

    pub(crate) fn diagnose(
        &self,
        view: &LexClauseView<'_>,
        subject_label: &'static str,
    ) -> Option<CardTextError> {
        let mut candidate_indices = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| lex_unsupported_rule_matches_view(rule, view))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        candidate_indices.sort_by_key(|idx| self.rules[*idx].priority);
        for idx in candidate_indices {
            let rule = &self.rules[idx];
            if (rule.predicate)(view) {
                return Some(unsupported_rule_error(
                    rule.id,
                    rule.message,
                    subject_label,
                    &view.display_text(),
                ));
            }
        }
        None
    }
}

fn unsupported_rule_matches_view(rule: &UnsupportedRuleDef, view: &ClauseView<'_>) -> bool {
    let head_matches = rule.heads.is_empty()
        || rule
            .heads
            .iter()
            .any(|candidate| *candidate == view.key.head);
    if !head_matches {
        return false;
    }
    if rule.shape_mask == 0 {
        return true;
    }
    (view.key.shape & rule.shape_mask) == rule.shape_mask
}

fn lex_unsupported_rule_matches_view(
    rule: &LexUnsupportedRuleDef,
    view: &LexClauseView<'_>,
) -> bool {
    let head_matches =
        rule.heads.is_empty() || rule.heads.iter().any(|candidate| *candidate == view.head());
    if !head_matches {
        return false;
    }
    if rule.shape_mask == 0 {
        return true;
    }
    (view.shape & rule.shape_mask) == rule.shape_mask
}
