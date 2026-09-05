//! The readings of one permission subject ("Aura spells with enchant
//! creature", "noncreature spells", "permanent spells", a spell type list, a
//! binary subject) read before the subject is an ordinary object filter.
//! Formerly a first-match ladder in `subject_filters`; every reading runs;
//! two different readings of one input are an ambiguity error.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct PermissionSubject<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl PermissionSubject<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<ObjectFilter>, CardTextError>,
    ) -> ParseOutcome<ObjectFilter> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("permission-subject-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&PermissionSubject<'_>) -> bool,
    read: fn(&PermissionSubject<'_>) -> ParseOutcome<ObjectFilter>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("permission-subject-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("aura-enchant-creature"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_aura_enchant_creature(input)),
    },
    Reading {
        id: RuleId::new("noncreature-spells"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_noncreature_spells(input)),
    },
    Reading {
        id: RuleId::new("permanent-spells"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_permanent_spells(input)),
    },
    Reading {
        id: RuleId::new("simple-spell-type-list"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_simple_spell_type_list(input)),
    },
    Reading {
        id: RuleId::new("binary-permission-subject"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("simple-spell-type-list")
        },
        read: |input| input.outcome(read_binary_permission_subject(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &PermissionSubject<'_>) -> ParseOutcome<RuleMatch<ObjectFilter>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<ObjectFilter>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_aura_enchant_creature(
    input: &PermissionSubject<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let filter_tokens = input.tokens;
    if parse_aura_enchant_creature_subject(filter_tokens).is_some() {
        return Ok(Some(
            ObjectFilter::default()
                .with_subtype(Subtype::Aura)
                .with_ability_marker("enchant creature"),
        ));
    }
    Ok(None)
}
fn read_noncreature_spells(
    input: &PermissionSubject<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let filter_tokens = input.tokens;
    if matches!(
        parse_exact_permission_subject(filter_tokens),
        Some(ExactPermissionSubject::NoncreatureSpells)
    ) {
        return Ok(Some(ObjectFilter::noncreature_spell()));
    }
    Ok(None)
}
fn read_permanent_spells(
    input: &PermissionSubject<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let filter_tokens = input.tokens;
    if matches!(
        parse_exact_permission_subject(filter_tokens),
        Some(ExactPermissionSubject::PermanentSpell | ExactPermissionSubject::PermanentSpells)
    ) {
        return Ok(Some(permanent_spell_filter()));
    }
    Ok(None)
}
fn read_simple_spell_type_list(
    input: &PermissionSubject<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let filter_tokens = input.tokens;
    if let Some(filter) = parse_simple_spell_type_list_filter_tokens(filter_tokens) {
        return Ok(Some(filter));
    }
    Ok(None)
}
fn read_binary_permission_subject(
    input: &PermissionSubject<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let filter_tokens = input.tokens;
    if let Some(filter) = parse_binary_permission_subject_filter_tokens(filter_tokens)? {
        return Ok(Some(filter));
    }
    Ok(None)
}
