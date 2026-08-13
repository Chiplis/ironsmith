use crate::diagnostics::TextSpan;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeadDiscriminator {
    Any,
    Words(&'static [&'static str]),
    /// A named typed grammar performed structural candidate filtering before
    /// the generic registry resolver was entered.
    Grammar(&'static str),
}

impl HeadDiscriminator {
    pub const fn words(words: &'static [&'static str]) -> Self {
        if words.is_empty() {
            Self::Any
        } else {
            Self::Words(words)
        }
    }

    pub const fn grammar(name: &'static str) -> Self {
        Self::Grammar(name)
    }

    pub fn accepts(self, head: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Words(words) => words.contains(&head),
            Self::Grammar(_) => true,
        }
    }

    pub const fn indexed_words(self) -> &'static [&'static str] {
        match self {
            Self::Any | Self::Grammar(_) => &[],
            Self::Words(words) => words,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceSpanPolicy {
    WholeInput,
    RecognizerProvided,
    Synthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticEquivalenceKey(&'static str);

impl SemanticEquivalenceKey {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegistryRuleMetadata {
    pub id: RuleId,
    pub head: HeadDiscriminator,
    pub span_policy: SourceSpanPolicy,
    pub equivalence: Option<SemanticEquivalenceKey>,
}

impl RegistryRuleMetadata {
    pub const fn distinct(id: RuleId, head: HeadDiscriminator) -> Self {
        Self {
            id,
            head,
            span_policy: SourceSpanPolicy::WholeInput,
            equivalence: None,
        }
    }

    pub const fn equivalent(
        id: RuleId,
        head: HeadDiscriminator,
        equivalence: SemanticEquivalenceKey,
    ) -> Self {
        Self {
            id,
            head,
            span_policy: SourceSpanPolicy::WholeInput,
            equivalence: Some(equivalence),
        }
    }
}

#[derive(Debug)]
pub struct RegistryCandidate<T> {
    pub metadata: RegistryRuleMetadata,
    pub value: T,
    pub span: Option<TextSpan>,
}

impl<T> RegistryCandidate<T> {
    pub fn new(metadata: RegistryRuleMetadata, value: T, span: Option<TextSpan>) -> Self {
        Self {
            metadata,
            value,
            span,
        }
    }
}

pub fn resolve_registry_candidates<T>(
    registry: RuleId,
    mut candidates: Vec<RegistryCandidate<T>>,
    diagnostics: Vec<ParseDiagnostic>,
) -> ParseOutcome<RuleMatch<T>> {
    if candidates.is_empty() {
        return furthest_committed_diagnostic(diagnostics)
            .map(ParseOutcome::Error)
            .unwrap_or(ParseOutcome::NoMatch);
    }

    if candidates.len() == 1 {
        let candidate = candidates.pop().expect("single registry candidate");
        return ParseOutcome::matched(
            RuleMatch {
                rule: candidate.metadata.id,
                value: candidate.value,
                span: candidate.span,
            },
            candidate.span,
        );
    }

    let equivalence = candidates[0].metadata.equivalence;
    let explicitly_equivalent = equivalence.is_some()
        && candidates
            .iter()
            .all(|candidate| candidate.metadata.equivalence == equivalence);
    if explicitly_equivalent {
        let candidate = candidates.remove(0);
        return ParseOutcome::matched(
            RuleMatch {
                rule: candidate.metadata.id,
                value: candidate.value,
                span: candidate.span,
            },
            candidate.span,
        );
    }

    let alternatives = candidates
        .iter()
        .map(|candidate| candidate.metadata.id)
        .collect::<Vec<_>>();
    let span = candidates
        .iter()
        .filter_map(|candidate| candidate.span)
        .max_by_key(span_key);
    ParseOutcome::Error(ParseDiagnostic::ambiguous(
        registry,
        span,
        alternatives,
        "non-equivalent registry rules recognized the same input",
    ))
}

pub fn furthest_committed_diagnostic(diagnostics: Vec<ParseDiagnostic>) -> Option<ParseDiagnostic> {
    diagnostics.into_iter().max_by_key(|diagnostic| {
        diagnostic
            .furthest_committed_span
            .or(diagnostic.span)
            .map(|span| span_key(&span))
            .unwrap_or((0, 0, 0))
    })
}

fn span_key(span: &TextSpan) -> (usize, usize, usize) {
    (span.line, span.end, span.start)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LegacyOrderRank(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyCompatibilityRule {
    pub registry: RuleId,
    pub rule: RuleId,
    pub rank: LegacyOrderRank,
    pub removal_pr: u8,
}
