//! The readings of the "for each ..." phrase of one scaling anthem: the
//! special shapes, aggregates, commander casts, sticker counts, compound count
//! filters and source counter counts, read before the phrase is an object
//! filter. Formerly a first-match ladder in `anthem_grant_lines`; every
//! reading runs; two different readings of one input are an ambiguity error.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct ForEachPhrase<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) rest: &'a [OwnedLexToken],
}

impl ForEachPhrase<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<AnthemCountExpression>, CardTextError>,
    ) -> ParseOutcome<AnthemCountExpression> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("anthem-for-each-registry-reading"),
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
    admits: fn(&ForEachPhrase<'_>) -> bool,
    read: fn(&ForEachPhrase<'_>) -> ParseOutcome<AnthemCountExpression>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("anthem-for-each-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("special-shape"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_special_shape(input)),
    },
    Reading {
        id: RuleId::new("aggregate-scope"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_aggregate_scope(input)),
    },
    Reading {
        id: RuleId::new("commander-cast-count"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_commander_cast_count(input)),
    },
    Reading {
        id: RuleId::new("sticker-count"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sticker_count(input)),
    },
    Reading {
        id: RuleId::new("compound-count-filter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_compound_count_filter(input)),
    },
    Reading {
        id: RuleId::new("source-counter-count"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_counter_count(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &ForEachPhrase<'_>) -> ParseOutcome<RuleMatch<AnthemCountExpression>> {
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
    let mut distinct: Vec<RegistryCandidate<AnthemCountExpression>> = Vec::new();
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

fn read_special_shape(
    input: &ForEachPhrase<'_>,
) -> Result<Option<AnthemCountExpression>, CardTextError> {
    let tokens = input.tokens;
    let rest = input.rest;
    if let Some(shape) = anthem_grant_grammar::parse_for_each_special_shape(rest) {
        match shape {
            anthem_grant_grammar::ForEachSpecialShape::AffectedAttackedThisTurn => {
                return Ok(Some(AnthemCountExpression::AffectedAttackedThisTurn));
            }
            anthem_grant_grammar::ForEachSpecialShape::ColorsOfAffected => {
                return Ok(Some(AnthemCountExpression::ColorsOfAffected));
            }
            anthem_grant_grammar::ForEachSpecialShape::CreatureTypesOfAffected => {
                return Ok(Some(AnthemCountExpression::CreatureTypesAmong(
                    ObjectFilter::source(),
                )));
            }
            anthem_grant_grammar::ForEachSpecialShape::GraveyardsWithAtLeastCards {
                minimum_cards,
            } => {
                return Ok(Some(AnthemCountExpression::GraveyardsWithAtLeastCards {
                    minimum_cards,
                }));
            }
            anthem_grant_grammar::ForEachSpecialShape::BlockingSource => {
                return Ok(Some(AnthemCountExpression::BlockingSource));
            }
            anthem_grant_grammar::ForEachSpecialShape::AttachedToSource { filter_tokens } => {
                let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported attached-object filter in anthem scaling clause (clause: '{}')",
                            crate::lexer::token_word_refs(&tokens).join(" ")
                        ))
                    })?;
                return Ok(Some(AnthemCountExpression::AttachedToSource(filter)));
            }
            anthem_grant_grammar::ForEachSpecialShape::UnspentGreenManaYouHave => {
                return Ok(Some(AnthemCountExpression::UnspentMana {
                    player: PlayerFilter::You,
                    symbol: crate::mana::ManaSymbol::Green,
                }));
            }
        }
    }
    Ok(None)
}
fn read_aggregate_scope(
    input: &ForEachPhrase<'_>,
) -> Result<Option<AnthemCountExpression>, CardTextError> {
    let rest = input.rest;
    if let Some(aggregate_value) = parse_aggregate_scope_value_lexed(rest) {
        match aggregate_value {
            Value::BasicLandTypesAmong(filter) => {
                return Ok(Some(AnthemCountExpression::BasicLandTypesAmong(filter)));
            }
            Value::CreatureTypesAmong(filter) => {
                return Ok(Some(AnthemCountExpression::CreatureTypesAmong(filter)));
            }
            _ => {}
        }
    }
    Ok(None)
}
fn read_commander_cast_count(
    input: &ForEachPhrase<'_>,
) -> Result<Option<AnthemCountExpression>, CardTextError> {
    let rest = input.rest;
    if let Some(player) = parse_commander_cast_count_player(rest) {
        return Ok(Some(AnthemCountExpression::CommanderCastCount(player)));
    }
    Ok(None)
}
fn read_sticker_count(
    input: &ForEachPhrase<'_>,
) -> Result<Option<AnthemCountExpression>, CardTextError> {
    let rest = input.rest;
    if let Some(sticker_count) = parse_sticker_count_expression(rest) {
        return Ok(Some(sticker_count));
    }
    Ok(None)
}
fn read_compound_count_filter(
    input: &ForEachPhrase<'_>,
) -> Result<Option<AnthemCountExpression>, CardTextError> {
    let rest = input.rest;
    if let Some(filter) = parse_compound_anthem_count_filter(rest) {
        return Ok(Some(AnthemCountExpression::MatchingFilter(filter)));
    }
    Ok(None)
}
fn read_source_counter_count(
    input: &ForEachPhrase<'_>,
) -> Result<Option<AnthemCountExpression>, CardTextError> {
    let rest = input.rest;
    if let Some(counter_clause) =
        crate::grammar::anthem_grants::parse_source_counter_count_clause(rest)
        && let Some(counter_type) = parse_counter_type_word(counter_clause.counter_type_word)
    {
        let source_words = crate::lexer::token_word_refs(counter_clause.source_tokens);
        if counter_clause.starts_with_source_pronoun
            || explicit_source_counter_surface(&source_words).is_some()
        {
            return Ok(Some(source_counter_count_expression(
                counter_type,
                &source_words,
            )));
        }
    }
    Ok(None)
}
