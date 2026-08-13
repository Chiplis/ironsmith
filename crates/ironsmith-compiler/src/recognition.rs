use crate::diagnostics::{CardTextError, TextSpan};

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
    Malformed { expected: Vec<ParseExpectation> },
    Unsupported { reason: UnsupportedReason },
    Ambiguous { alternatives: Vec<RuleId> },
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

    pub fn from_legacy_error(rule: RuleId, span: Option<TextSpan>, error: CardTextError) -> Self {
        match error {
            CardTextError::UnsupportedLine(message) => Self::unsupported(
                rule,
                span,
                UnsupportedReason::new("legacy-unsupported", message.clone()),
                message,
            ),
            CardTextError::ParseError(message) => {
                Self::malformed(rule, span, std::iter::empty(), message)
            }
            CardTextError::InvariantViolation(message) => Self::invariant(rule, span, message),
        }
    }

    pub fn within(mut self, parent: RuleId) -> Self {
        if self.rule_path.first().copied() != Some(parent) {
            self.rule_path.insert(0, parent);
        }
        self
    }

    pub fn into_legacy_error(self) -> CardTextError {
        let path = self
            .rule_path
            .iter()
            .map(|rule| rule.as_str())
            .collect::<Vec<_>>()
            .join(" > ");
        let message = if path.is_empty() {
            self.message
        } else {
            format!("{} [rule-path={path}]", self.message)
        };
        match self.kind {
            ParseDiagnosticKind::Unsupported { .. } => CardTextError::UnsupportedLine(message),
            ParseDiagnosticKind::Malformed { .. } | ParseDiagnosticKind::Ambiguous { .. } => {
                CardTextError::ParseError(message)
            }
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

    pub fn from_legacy_option(value: Option<T>, span: Option<TextSpan>) -> Self {
        match value {
            Some(value) => Self::matched(value, span),
            None => Self::NoMatch,
        }
    }

    pub fn from_legacy_result_option(
        rule: RuleId,
        span: Option<TextSpan>,
        result: Result<Option<T>, CardTextError>,
    ) -> Self {
        match result {
            Ok(value) => Self::from_legacy_option(value, span),
            Err(error) => Self::Error(ParseDiagnostic::from_legacy_error(rule, span, error)),
        }
    }

    pub fn from_legacy_result(
        rule: RuleId,
        span: Option<TextSpan>,
        result: Result<T, CardTextError>,
    ) -> Self {
        match result {
            Ok(value) => Self::matched(value, span),
            Err(error) => Self::Error(ParseDiagnostic::from_legacy_error(rule, span, error)),
        }
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

    pub fn into_legacy_result_option(self) -> Result<Option<T>, CardTextError> {
        match self {
            Self::NoMatch => Ok(None),
            Self::Match(matched) => Ok(Some(matched.value)),
            Self::Error(diagnostic) => Err(diagnostic.into_legacy_error()),
        }
    }

    pub fn into_legacy_result(
        self,
        no_match: impl FnOnce() -> CardTextError,
    ) -> Result<T, CardTextError> {
        match self {
            Self::NoMatch => Err(no_match()),
            Self::Match(matched) => Ok(matched.value),
            Self::Error(diagnostic) => Err(diagnostic.into_legacy_error()),
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
