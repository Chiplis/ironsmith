//! The readings of the condition of one "enters tapped unless ..." line: the
//! revealed-this-way and control conditions, a control quantity, the fixed
//! life and opponent-count conditions, the first three turns, and the generic
//! "unless you control ..." filter. Formerly a first-match ladder in
//! `etb_static_lines`; every reading runs, resolved by rank while the overlaps
//! are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct UnlessCondition<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) clause_words: &'a [&'a str],
}

impl UnlessCondition<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<StaticAbility>, CardTextError>,
    ) -> ParseOutcome<StaticAbility> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("enters-tapped-unless-registry-reading"),
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
    admits: fn(&UnlessCondition<'_>) -> bool,
    read: fn(&UnlessCondition<'_>) -> ParseOutcome<StaticAbility>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("enters-tapped-unless-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("revealed-this-way-or-control"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_revealed_this_way_or_control(input)),
    },
    Reading {
        id: RuleId::new("control-quantity"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_control_quantity(input)),
    },
    Reading {
        id: RuleId::new("a-player-has-13-or-less-life"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_a_player_has_13_or_less_life(input)),
    },
    Reading {
        id: RuleId::new("two-or-more-opponents"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_two_or_more_opponents(input)),
    },
    Reading {
        id: RuleId::new("first-three-turns"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_first_three_turns(input)),
    },
    Reading {
        id: RuleId::new("generic-control-filter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_control_filter(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &UnlessCondition<'_>) -> ParseOutcome<RuleMatch<StaticAbility>> {
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
    let mut distinct: Vec<RegistryCandidate<StaticAbility>> = Vec::new();
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

fn read_revealed_this_way_or_control(
    input: &UnlessCondition<'_>,
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = input.clause_words;
    let condition_tokens = input.tokens;
    if let Some(condition) = parse_revealed_this_way_or_control_condition(&condition_tokens) {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            condition,
            clause_words.join(" "),
        )));
    }
    Ok(None)
}
fn read_control_quantity(
    input: &UnlessCondition<'_>,
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = input.clause_words;
    let condition_tokens = input.tokens;
    if let Some(ability) = parse_enters_tapped_unless_control_quantity_static_ability(
        &condition_tokens,
        clause_words.join(" "),
    ) {
        return Ok(Some(ability));
    }
    Ok(None)
}
fn read_a_player_has_13_or_less_life(
    input: &UnlessCondition<'_>,
) -> Result<Option<StaticAbility>, CardTextError> {
    let condition_tokens = input.tokens;
    if parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(&condition_tokens)
        .is_some()
    {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_a_player_has_13_or_less_life(),
        ));
    }
    Ok(None)
}
fn read_two_or_more_opponents(
    input: &UnlessCondition<'_>,
) -> Result<Option<StaticAbility>, CardTextError> {
    let condition_tokens = input.tokens;
    if parse_enters_tapped_unless_two_or_more_opponents_condition(&condition_tokens).is_some() {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_two_or_more_opponents(),
        ));
    }
    Ok(None)
}
fn read_first_three_turns(
    input: &UnlessCondition<'_>,
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = input.clause_words;
    let condition_tokens = input.tokens;
    if etb_grammar::parse_first_three_turns_prefix_tokens(&condition_tokens).is_some() {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            PredicateAst::YourFirstTurnsOfTheGameOrFewer(3),
            clause_words.join(" "),
        )));
    }
    Ok(None)
}
fn read_generic_control_filter(
    input: &UnlessCondition<'_>,
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = input.clause_words;
    let condition_tokens = input.tokens;
    // Generic: "unless you control <object filter>" (covers Mount/Vehicle, etc.).
    if let Some(control_condition) = crate::grammar::conditions::parse_control_condition(
        &condition_tokens,
        crate::grammar::conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: false,
            bind_filter_controller_to_subject: false,
            allow_different_powers_tail: false,
            default_filter_zone: None,
        },
    ) && !control_condition.has_explicit_quantity()
    {
        let condition = PredicateAst::YouControl(control_condition.filter);
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            condition,
            clause_words.join(" "),
        )));
    }
    Ok(None)
}
