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
    /// What this candidate's reading claimed, recorded in its own scope. Only
    /// the candidate the resolver keeps passes its claims on; a losing
    /// candidate's claims are dropped like a discarded probe's.
    pub claims: crate::word_claims::ClaimSet,
}

impl<T> RegistryCandidate<T> {
    pub fn new(metadata: RegistryRuleMetadata, value: T, span: Option<TextSpan>) -> Self {
        Self {
            metadata,
            value,
            span,
            claims: crate::word_claims::ClaimSet::default(),
        }
    }

    /// Attach the claims the reading recorded under
    /// [`crate::word_claims::scoped`].
    pub fn with_claims(mut self, claims: crate::word_claims::ClaimSet) -> Self {
        self.claims = claims;
        self
    }
}

/// `IRONSMITH_REGISTRY_TRACE=1`: one stderr line per resolved registry with
/// every matching candidate, the kept one (or `-` for none/ambiguity), and
/// the words the kept candidate claimed.
fn trace_registry_resolution<T>(
    registry: RuleId,
    candidates: &[RegistryCandidate<T>],
    kept: Option<usize>,
    note: &str,
) {
    if !crate::word_claims::registry_trace_enabled() {
        return;
    }
    let names = candidates
        .iter()
        .map(|candidate| candidate.metadata.id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let kept_name = kept
        .and_then(|index| candidates.get(index))
        .map(|candidate| candidate.metadata.id.to_string())
        .unwrap_or_else(|| "-".to_string());
    let claims = kept
        .and_then(|index| candidates.get(index))
        .map(|candidate| crate::word_claims::describe_claims(&candidate.claims))
        .unwrap_or_default();
    eprintln!("REGISTRY {registry} [{names}] kept={kept_name}{note} claims: {claims}");
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
        trace_registry_resolution(registry, &candidates, Some(0), "");
        let candidate = candidates.pop().expect("single registry candidate");
        crate::word_claims::adopt(&candidate.claims);
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
        trace_registry_resolution(registry, &candidates, Some(0), " (equivalent)");
        // Declared-equivalent readings are one reading: the words any of
        // them read count as read.
        for other in &candidates[1..] {
            crate::word_claims::adopt(&other.claims);
        }
        let candidate = candidates.remove(0);
        crate::word_claims::adopt(&candidate.claims);
        return ParseOutcome::matched(
            RuleMatch {
                rule: candidate.metadata.id,
                value: candidate.value,
                span: candidate.span,
            },
            candidate.span,
        );
    }

    trace_registry_resolution(registry, &candidates, None, " (ambiguous)");
    let alternatives = candidates
        .iter()
        .map(|candidate| candidate.metadata.id)
        .collect::<Vec<_>>();
    let alternative_names = alternatives
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let span = candidates
        .iter()
        .filter_map(|candidate| candidate.span)
        .max_by_key(span_key);
    ParseOutcome::Error(ParseDiagnostic::ambiguous(
        registry,
        span,
        alternatives,
        format!("non-equivalent registry rules recognized the same input: {alternative_names}"),
    ))
}

/// The registries that still resolve by rank: every viable rule runs, and when
/// more than one non-equivalent rule matches the first registered wins while
/// the overlap is recorded. Each name leaves this list when its overlaps on
/// the corpus reach zero and it resolves through
/// [`resolve_registry_candidates`] instead. The count is a ratchet.
pub const RANKED_REGISTRIES: &[&str] = &[
    "sentence-composition-registry",
    "chain-composition-registry",
    "alternative-cast-registry",
    "single-static-line-registry",
    "document-remaining-registry",
    "object-filter-inner-registry",
    "object-filter-lexed-inner-registry",
    "exile-clause-registry",
    "put-clause-registry",
    "granted-object-segment-registry",
    "ward-cost-registry",
    "keyword-special-case-registry",
    "tagged-permission-registry",
    "payment-cost-registry",
];

/// Resolve like [`resolve_registry_candidates`], except that several
/// non-equivalent matches keep the first registered and record the overlap in
/// the overlap ledger instead of raising an ambiguity. Only a registry named in
/// [`RANKED_REGISTRIES`] may resolve this way.
pub fn resolve_ranked_candidates<T>(
    registry: RuleId,
    mut candidates: Vec<RegistryCandidate<T>>,
    diagnostics: Vec<ParseDiagnostic>,
    text: impl FnOnce() -> String,
) -> ParseOutcome<RuleMatch<T>> {
    debug_assert!(
        RANKED_REGISTRIES.contains(&registry.as_str()),
        "{registry} resolves by rank but is not a declared ranked registry"
    );
    if candidates.len() > 1 {
        let equivalence = candidates[0].metadata.equivalence;
        let explicitly_equivalent = equivalence.is_some()
            && candidates
                .iter()
                .all(|candidate| candidate.metadata.equivalence == equivalence);
        if !explicitly_equivalent {
            trace_registry_resolution(registry, &candidates, Some(0), " (ranked)");
            let rules = candidates
                .iter()
                .map(|candidate| candidate.metadata.id)
                .collect::<Vec<_>>();
            crate::overlap_ledger::note(registry, &rules, text);
            crate::parse_trace::event(format!(
                "ranked registry {registry}: {} kept over {}",
                rules[0],
                rules[1..]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        candidates.truncate(1);
    }
    resolve_registry_candidates(registry, candidates, diagnostics)
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
