use crate::diagnostics::{CardTextError, TextSpan};
use crate::model::symbols::{ReferenceQuery, SymbolId, SymbolResolutionError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleId(&'static str);

impl RuleId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseExpectation {
    pub label: String,
}

impl ParseExpectation {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedReason {
    pub code: String,
    pub detail: String,
}

impl UnsupportedReason {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseDiagnosticKind {
    Malformed {
        expected: Vec<ParseExpectation>,
    },
    Unsupported {
        reason: UnsupportedReason,
    },
    Ambiguous {
        alternatives: Vec<RuleId>,
    },
    UnresolvedReference {
        query: ReferenceQuery,
    },
    AmbiguousReference {
        query: ReferenceQuery,
        candidates: Vec<SymbolId>,
    },
    InvalidReference {
        query: ReferenceQuery,
        candidates: Vec<SymbolId>,
    },
    Invariant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub rule: RuleId,
    pub rule_path: Vec<RuleId>,
    pub span: Option<TextSpan>,
    pub furthest_committed_span: Option<TextSpan>,
    pub kind: ParseDiagnosticKind,
    pub message: String,
}

impl ParseDiagnostic {
    pub fn malformed(
        rule: RuleId,
        span: Option<TextSpan>,
        expected: impl IntoIterator<Item = ParseExpectation>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule,
            rule_path: vec![rule],
            span,
            furthest_committed_span: span,
            kind: ParseDiagnosticKind::Malformed {
                expected: expected.into_iter().collect(),
            },
            message: message.into(),
        }
    }

    pub fn unsupported(
        rule: RuleId,
        span: Option<TextSpan>,
        reason: UnsupportedReason,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule,
            rule_path: vec![rule],
            span,
            furthest_committed_span: span,
            kind: ParseDiagnosticKind::Unsupported { reason },
            message: message.into(),
        }
    }

    pub fn invariant(rule: RuleId, span: Option<TextSpan>, message: impl Into<String>) -> Self {
        Self {
            rule,
            rule_path: vec![rule],
            span,
            furthest_committed_span: span,
            kind: ParseDiagnosticKind::Invariant,
            message: message.into(),
        }
    }

    pub fn ambiguous(
        rule: RuleId,
        span: Option<TextSpan>,
        alternatives: Vec<RuleId>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule,
            rule_path: vec![rule],
            span,
            furthest_committed_span: span,
            kind: ParseDiagnosticKind::Ambiguous { alternatives },
            message: message.into(),
        }
    }

    pub fn from_card_text_error(
        rule: RuleId,
        span: Option<TextSpan>,
        error: CardTextError,
    ) -> Self {
        match error {
            CardTextError::UnsupportedLine(message) => Self::unsupported(
                rule,
                span,
                UnsupportedReason::new("unsupported-input", message.clone()),
                message,
            ),
            CardTextError::ParseError(message) => {
                Self::malformed(rule, span, std::iter::empty(), message)
            }
            CardTextError::InvariantViolation(message) => Self::invariant(rule, span, message),
        }
    }

    pub fn from_symbol_error(
        rule: RuleId,
        span: Option<TextSpan>,
        error: SymbolResolutionError,
    ) -> Self {
        let (kind, message) = match error {
            SymbolResolutionError::Unresolved(query) => (
                ParseDiagnosticKind::UnresolvedReference { query },
                format!(
                    "unresolved {:?} reference in {:?} domain",
                    query.role, query.domain
                ),
            ),
            SymbolResolutionError::Ambiguous { query, candidates } => (
                ParseDiagnosticKind::AmbiguousReference {
                    query,
                    candidates: candidates.clone(),
                },
                format!(
                    "ambiguous {:?} reference matched symbols {candidates:?}",
                    query.role
                ),
            ),
            SymbolResolutionError::WrongDomain { query, candidates } => (
                ParseDiagnosticKind::InvalidReference {
                    query,
                    candidates: candidates.clone(),
                },
                format!(
                    "{:?} reference resolved only in the wrong domain: {candidates:?}",
                    query.role
                ),
            ),
            SymbolResolutionError::WrongCardinality { query, candidates } => (
                ParseDiagnosticKind::InvalidReference {
                    query,
                    candidates: candidates.clone(),
                },
                format!(
                    "{:?} reference resolved only with the wrong cardinality: {candidates:?}",
                    query.role
                ),
            ),
            SymbolResolutionError::UnknownScope(scope) => (
                ParseDiagnosticKind::Invariant,
                format!("unknown lexical symbol scope {scope:?}"),
            ),
        };
        Self {
            rule,
            rule_path: vec![rule],
            span,
            furthest_committed_span: span,
            kind,
            message,
        }
    }

    pub fn within(mut self, parent: RuleId) -> Self {
        if self.rule_path.first().copied() != Some(parent) {
            self.rule_path.insert(0, parent);
        }
        self
    }

    pub fn into_card_text_error(self) -> CardTextError {
        let path = self
            .rule_path
            .iter()
            .map(|rule| rule.as_str())
            .collect::<Vec<_>>()
            .join(" > ");
        let mut message = if path.is_empty() {
            self.message
        } else {
            format!("{} [rule-path={path}]", self.message)
        };
        if let ParseDiagnosticKind::Ambiguous { alternatives } = &self.kind
            && !alternatives.is_empty()
        {
            let alternatives = alternatives
                .iter()
                .map(|rule| rule.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            message.push_str(&format!(" [alternatives={alternatives}]"));
        }
        match self.kind {
            ParseDiagnosticKind::Unsupported { .. } => CardTextError::UnsupportedLine(message),
            ParseDiagnosticKind::Malformed { .. }
            | ParseDiagnosticKind::Ambiguous { .. }
            | ParseDiagnosticKind::UnresolvedReference { .. }
            | ParseDiagnosticKind::AmbiguousReference { .. }
            | ParseDiagnosticKind::InvalidReference { .. } => CardTextError::ParseError(message),
            ParseDiagnosticKind::Invariant => CardTextError::InvariantViolation(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMatch<T> {
    pub value: T,
    pub span: Option<TextSpan>,
}

impl<T> ParseMatch<T> {
    pub fn new(value: T, span: Option<TextSpan>) -> Self {
        Self { value, span }
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> ParseMatch<U> {
        ParseMatch {
            value: map(self.value),
            span: self.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseOutcome<T> {
    NoMatch,
    Match(ParseMatch<T>),
    Error(ParseDiagnostic),
}

impl<T> ParseOutcome<T> {
    pub fn matched(value: T, span: Option<TextSpan>) -> Self {
        Self::Match(ParseMatch::new(value, span))
    }

    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> ParseOutcome<U> {
        match self {
            Self::NoMatch => ParseOutcome::NoMatch,
            Self::Match(matched) => ParseOutcome::Match(matched.map(map)),
            Self::Error(diagnostic) => ParseOutcome::Error(diagnostic),
        }
    }

    pub fn within(self, parent: RuleId) -> Self {
        match self {
            Self::Error(diagnostic) => Self::Error(diagnostic.within(parent)),
            other => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMatch<T> {
    pub rule: RuleId,
    pub value: T,
    pub span: Option<TextSpan>,
}

impl<T> RuleMatch<T> {
    pub fn new(rule: RuleId, matched: ParseMatch<T>) -> Self {
        Self {
            rule,
            value: matched.value,
            span: matched.span,
        }
    }
}
