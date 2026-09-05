//! The readings of one "can't ..." restriction line: attack-unless, a
//! blocking cost, an except-for exception, the direct shape, a casting
//! restriction, unspent-mana retention. The declines the ladder made between
//! them (temporary casts, iterated players, leading conditions, mana
//! retention riders, stat-modifier conjunctions, no negation) are the
//! admission tests of the readings ranked after them. Formerly a first-match
//! ladder in `activation_costs`; every reading runs, resolved by rank while
//! the overlaps are measured. The or/and-split clause readers are the
//! fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct CantClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl CantClause<'_> {
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
        read: Result<Option<Vec<StaticAbility>>, CardTextError>,
    ) -> ParseOutcome<Vec<StaticAbility>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("cant-clause-registry-reading"),
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
    admits: fn(&CantClause<'_>) -> bool,
    read: fn(&CantClause<'_>) -> ParseOutcome<Vec<StaticAbility>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("cant-clause-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("attack-unless"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_attack_unless(input)),
    },
    Reading {
        id: RuleId::new("block-cost"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_block_cost(input)),
    },
    Reading {
        id: RuleId::new("except-for-cant-attack"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_except_for_cant_attack(input)),
    },
    Reading {
        id: RuleId::new("direct-cant"),
        head: HeadDiscriminator::Any,
        admits: |input| !declines_1(input),
        read: |input| input.outcome(read_direct_cant(input)),
    },
    Reading {
        id: RuleId::new("cant-cast-restriction"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            !declines_1(input) && !declines_2(input)
                // Readings ranked above this one that read the input read it.
                && !input.read_by("direct-cant")
        },
        read: |input| input.outcome(read_cant_cast_restriction(input)),
    },
    Reading {
        id: RuleId::new("unspent-mana-retention"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            !declines_1(input)
                && !declines_2(input)
                && !declines_3(input)
                && !declines_4(input)
                && !declines_5(input)
                && !declines_6(input)
        },
        read: |input| input.outcome(read_unspent_mana_retention(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &CantClause<'_>) -> ParseOutcome<RuleMatch<Vec<StaticAbility>>> {
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
    let mut distinct: Vec<RegistryCandidate<Vec<StaticAbility>>> = Vec::new();
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

fn read_attack_unless(input: &CantClause<'_>) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = attack_unless_static_ability(tokens) {
        return Ok(Some(vec![ability]));
    }
    Ok(None)
}
fn read_block_cost(input: &CantClause<'_>) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = block_cost_static_ability(tokens)? {
        return Ok(Some(vec![ability]));
    }
    Ok(None)
}
fn read_except_for_cant_attack(
    input: &CantClause<'_>,
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(ability) = except_for_cant_attack_static_ability(tokens)? {
        return Ok(Some(vec![ability]));
    }
    Ok(None)
}
fn read_direct_cant(input: &CantClause<'_>) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(StaticAbilityShapeResolution::Ability(ability)) = direct_cant_static_ability(tokens)
    {
        return Ok(Some(vec![ability]));
    }
    Ok(None)
}
fn read_cant_cast_restriction(
    input: &CantClause<'_>,
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let tokens = input.tokens;
    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some(restriction) = parse_cant_cast_restriction_words(&normalized_words) {
        return Ok(Some(vec![StaticAbility::restriction(
            restriction,
            format_negated_restriction_display(tokens),
        )]));
    }
    Ok(None)
}
fn read_unspent_mana_retention(
    input: &CantClause<'_>,
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let tokens = input.tokens;
    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    // "Players/You don't lose unspent [color] mana as steps and phases end."
    // Parsed before the and-splitting below tears apart "steps and phases end".
    if let Some(ability) = parse_unspent_mana_retention_static(tokens, &normalized_words) {
        return Ok(Some(vec![ability]));
    }
    Ok(None)
}
/// A decline the ladder made before the readings ranked after it.
fn declines_1(input: &CantClause<'_>) -> bool {
    let tokens = input.tokens;
    matches!(
        direct_cant_static_ability(tokens),
        Some(StaticAbilityShapeResolution::Decline)
    )
}
/// A decline the ladder made before the readings ranked after it.
fn declines_2(input: &CantClause<'_>) -> bool {
    let tokens = input.tokens;
    cant_shapes::parse_direct_temporary_cast_decline_tokens(tokens).is_some()
}
/// A decline the ladder made before the readings ranked after it.
fn declines_3(input: &CantClause<'_>) -> bool {
    let tokens = input.tokens;
    cant_shapes::parse_iterated_player_who_decline_tokens(tokens).is_some()
}
/// A decline the ladder made before the readings ranked after it.
fn declines_4(input: &CantClause<'_>) -> bool {
    let tokens = input.tokens;
    cant_shapes::parse_leading_if_cant_decline_tokens(tokens).is_some()
}
/// A decline the ladder made before the readings ranked after it.
fn declines_5(input: &CantClause<'_>) -> bool {
    let tokens = input.tokens;
    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    matches!(
        crate::grammar::activation_restrictions::parse_mana_retention_negated_clause_words(
            &normalized_words,
        ),
        Some(
            crate::grammar::activation_restrictions::ManaRetentionNegatedClause {
                tail: crate::grammar::activation_restrictions::ManaRetentionTailKind::ThisMana,
            }
        )
    )
}
/// A decline the ladder made before the readings ranked after it.
fn declines_6(input: &CantClause<'_>) -> bool {
    let tokens = input.tokens;
    restriction_duration_remainder_retains_mana(tokens)
}

/// Whether any decline the ladder made before its fallback holds: the
/// fallback ran only when none did.
pub(super) fn declines(input: &CantClause<'_>) -> bool {
    declines_1(input)
        || declines_2(input)
        || declines_3(input)
        || declines_4(input)
        || declines_5(input)
        || declines_6(input)
}
