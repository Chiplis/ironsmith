//! The readings of one trigger clause before the compatibility matcher: the
//! small complete shapes (beginning of combat, the source entering), the
//! "while" qualifier, the passive sacrificed-or-destroyed and attack shapes,
//! the trigger unions, combat damage and filtered attack counts. Formerly a
//! first-match ladder in `trigger_clause_core`; every reading runs, resolved
//! by rank while the overlaps are measured. The legacy aggregate matcher is
//! the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct TriggerClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl TriggerClause<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<TriggerSpec>, CardTextError>,
    ) -> ParseOutcome<TriggerSpec> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("trigger-clause-registry-reading"),
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
    admits: fn(&TriggerClause<'_>) -> bool,
    read: fn(&TriggerClause<'_>) -> ParseOutcome<TriggerSpec>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("trigger-clause-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("simple-beginning-of-combat"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_simple_beginning_of_combat(input)),
    },
    Reading {
        id: RuleId::new("simple-source-enters"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_simple_source_enters(input)),
    },
    Reading {
        id: RuleId::new("while-qualified-event"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_while_qualified_event(input)),
    },
    Reading {
        id: RuleId::new("passive-sacrificed-or-destroyed"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_passive_sacrificed_or_destroyed(input)),
    },
    Reading {
        id: RuleId::new("player-attack-with-aggregate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_attack_with_aggregate(input)),
    },
    Reading {
        id: RuleId::new("player-puts-object-onto-battlefield"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_puts_object_onto_battlefield(input)),
    },
    Reading {
        id: RuleId::new("player-attack-with-one-or-more"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_attack_with_one_or_more(input)),
    },
    Reading {
        id: RuleId::new("source-and-another-attack-different-players"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_and_another_attack_different_players(input)),
    },
    Reading {
        id: RuleId::new("shared-player-attack-draw-cast-union"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_shared_player_attack_draw_cast_union(input)),
    },
    Reading {
        id: RuleId::new("repeated-intro-attack-union"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_repeated_intro_attack_union(input)),
    },
    Reading {
        id: RuleId::new("trigger-union"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_trigger_union(input)),
    },
    Reading {
        id: RuleId::new("combat-damage-trigger"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_combat_damage_trigger(input)),
    },
    Reading {
        id: RuleId::new("source-with-filtered-attack-count-trigger"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_with_filtered_attack_count_trigger(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &TriggerClause<'_>) -> ParseOutcome<RuleMatch<TriggerSpec>> {
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
    let mut distinct: Vec<RegistryCandidate<TriggerSpec>> = Vec::new();
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

fn read_simple_beginning_of_combat(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(trigger) = try_parse_simple_beginning_of_combat_trigger_lexed(tokens) {
        return Ok(Some(trigger));
    }
    Ok(None)
}
fn read_simple_source_enters(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    // A source ETB is one of the smallest and most frequent trigger clauses.
    // Claim its complete grammar shape before entering the compatibility
    // matchers below: several of those handlers carry large typed filter
    // temporaries even when their first word cannot match this family.
    if let Some(trigger) = try_parse_simple_source_enters_trigger_lexed(tokens) {
        return Ok(Some(trigger));
    }
    Ok(None)
}
fn read_while_qualified_event(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    // `... while <condition>` qualifies the event itself. Preserve it as
    // a typed matcher wrapper before union parsing or the broad attack/
    // cast routes can accept only the event prefix and silently discard
    // the board-state requirement. Recursive parsing is safe because the
    // left slice no longer contains the `while` separator.
    if let Some(while_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("while"))
        && while_idx > 0
        && while_idx + 1 < tokens.len()
    {
        let trigger_tokens = trim_edge_punctuation(&tokens[..while_idx]);
        let condition_tokens = trim_edge_punctuation(&tokens[while_idx + 1..]);
        let trigger = parse_trigger_clause_lexed(&trigger_tokens)?;
        let condition = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
            &condition_tokens,
        )?;
        return Ok(Some(TriggerSpec::ConditionQualified {
            trigger: Box::new(trigger),
            condition,
            surface: crate::lexer::render_token_slice(&condition_tokens)
                .trim()
                .trim_end_matches('.')
                .to_string(),
        }));
    }
    Ok(None)
}
fn read_passive_sacrificed_or_destroyed(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(trigger) = try_parse_passive_sacrificed_or_destroyed_lexed(tokens)? {
        return Ok(Some(trigger));
    }
    Ok(None)
}
fn read_player_attack_with_aggregate(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(trigger) = try_parse_player_attack_with_aggregate_lexed(tokens)? {
        return Ok(Some(trigger));
    }
    Ok(None)
}
fn read_player_puts_object_onto_battlefield(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(trigger) = try_parse_player_puts_object_onto_battlefield_lexed(tokens)? {
        return Ok(Some(trigger));
    }
    Ok(None)
}
fn read_player_attack_with_one_or_more(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(trigger) = try_parse_player_attack_with_one_or_more_lexed(tokens)? {
        return Ok(Some(trigger));
    }
    Ok(None)
}
fn read_source_and_another_attack_different_players(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(trigger) = try_parse_source_and_another_attack_different_players(tokens) {
        return Ok(Some(trigger));
    }
    Ok(None)
}
fn read_shared_player_attack_draw_cast_union(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(union) = try_parse_shared_player_attack_draw_cast_union_lexed(tokens)? {
        return Ok(Some(union));
    }
    Ok(None)
}
fn read_repeated_intro_attack_union(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(union) = try_parse_repeated_intro_attack_union_lexed(tokens) {
        return Ok(Some(union));
    }
    Ok(None)
}
fn read_trigger_union(input: &TriggerClause<'_>) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(union) = try_parse_trigger_union_lexed(tokens) {
        return Ok(Some(union));
    }
    Ok(None)
}
fn read_combat_damage_trigger(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(trigger) = try_parse_combat_damage_trigger_lexed(tokens)? {
        return Ok(Some(trigger));
    }
    Ok(None)
}
fn read_source_with_filtered_attack_count_trigger(
    input: &TriggerClause<'_>,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = input.tokens;
    if let Some(trigger) = try_parse_source_with_filtered_attack_count_trigger_lexed(tokens)? {
        return Ok(Some(trigger));
    }
    Ok(None)
}
