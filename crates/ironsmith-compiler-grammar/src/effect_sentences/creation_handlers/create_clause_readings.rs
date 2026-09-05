//! The readings of one "create ..." clause before the token-definition
//! grammar: a choice of options, a direct alternative, a direct conjunction,
//! a delayed combat token action. Formerly a first-match ladder in
//! `creation_handlers`; every reading runs, resolved by rank while the
//! overlaps are measured. The token definition is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct CreateClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) subject: Option<SubjectAst>,
}

impl CreateClause<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("create-clause-registry-reading"),
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
    admits: fn(&CreateClause<'_>) -> bool,
    read: fn(&CreateClause<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("create-clause-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("choice-of-options"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choice_of_options(input)),
    },
    Reading {
        id: RuleId::new("direct-token-creation-alternative"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_direct_token_creation_alternative(input)),
    },
    Reading {
        id: RuleId::new("direct-token-creation-conjunction"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_direct_token_creation_conjunction(input)),
    },
    Reading {
        id: RuleId::new("delayed-combat-token-action"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_delayed_combat_token_action(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &CreateClause<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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
    let mut distinct: Vec<RegistryCandidate<EffectAst>> = Vec::new();
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

fn read_choice_of_options(input: &CreateClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(choice) = parse_create_choice_of_options(tokens)? {
        return Ok(Some(choice));
    }
    Ok(None)
}
fn read_direct_token_creation_alternative(
    input: &CreateClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    if let Some(alternative) = parse_direct_token_creation_alternative(tokens, subject) {
        return Ok(Some(alternative));
    }
    Ok(None)
}
fn read_direct_token_creation_conjunction(
    input: &CreateClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    if let Some(conjunction) = parse_direct_token_creation_conjunction(tokens, subject) {
        return Ok(Some(conjunction));
    }
    Ok(None)
}
fn read_delayed_combat_token_action(
    input: &CreateClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let non_article_words = crate::util::non_article_token_word_refs(tokens);
    if let Some(action) =
        creation_grammar::parse_delayed_combat_token_action_words(&non_article_words)
    {
        let effect = match action {
            creation_grammar::DelayedCombatTokenAction::Exile => EffectAst::subject_verb_exile(
                TargetAst::Object(
                    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    span_from_tokens(tokens),
                    None,
                ),
                false,
            ),
            creation_grammar::DelayedCombatTokenAction::Sacrifice => {
                EffectAst::subject_verb_sacrifice(
                    PlayerAst::Implicit,
                    ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                    1,
                    None,
                )
            }
        };
        return Ok(Some(EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        }));
    }
    Ok(None)
}
