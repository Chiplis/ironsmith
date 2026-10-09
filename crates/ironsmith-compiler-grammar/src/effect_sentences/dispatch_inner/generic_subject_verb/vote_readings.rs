//! The readings of one voting sentence: a secret number choice, the reveal,
//! the generic vote start, an option's effects, an extra vote. Formerly a
//! first-match ladder in `generic_subject_verb`; every reading runs, resolved
//! by rank while the overlaps are measured.

use crate::cards::builders::VoteEffectAst;
use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct VoteSentence<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl VoteSentence<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("vote-sentence-registry-reading"),
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
    admits: fn(&VoteSentence<'_>) -> bool,
    read: fn(&VoteSentence<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("vote-sentence-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("secret-number-choice-vote-start"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_secret_number_choice_vote_start(input)),
    },
    Reading {
        id: RuleId::new("vote-reveal"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_vote_reveal(input)),
    },
    Reading {
        id: RuleId::new("generic-vote-start"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_vote_start(input)),
    },
    Reading {
        id: RuleId::new("generic-vote-option-effects"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_vote_option_effects(input)),
    },
    Reading {
        id: RuleId::new("each-player-per-vote-received"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_each_player_per_vote_received(input)),
    },
    Reading {
        id: RuleId::new("each-player-who-received-no-votes"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_each_player_who_received_no_votes(input)),
    },
    Reading {
        id: RuleId::new("generic-extra-vote"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_extra_vote(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &VoteSentence<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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

fn read_secret_number_choice_vote_start(
    input: &VoteSentence<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_secret_number_choice_vote_start(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_vote_reveal(input: &VoteSentence<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_vote_reveal_sentence(tokens) {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_generic_vote_start(input: &VoteSentence<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_generic_vote_start(tokens)? {
        if let EffectAst::Votes(VoteEffectAst::VoteStart {
            options,
            secret,
            starting_with_controller,
        }) = effect
        {
            return Ok(Some(
                GenericVoteProgram::Start {
                    options,
                    secret,
                    starting_with_controller,
                }
                .lower(),
            ));
        }
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_generic_vote_option_effects(
    input: &VoteSentence<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_generic_vote_option_effects(tokens)? {
        if let EffectAst::Votes(VoteEffectAst::VoteOption { option, effects }) = effect {
            return Ok(Some(
                GenericVoteProgram::OptionEffects { option, effects }.lower(),
            ));
        }
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_generic_extra_vote(input: &VoteSentence<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_generic_extra_vote(tokens) {
        if let EffectAst::Votes(VoteEffectAst::VoteExtra { count, optional }) = effect {
            return Ok(Some(GenericVoteProgram::Extra { count, optional }.lower()));
        }
        return Ok(Some(effect));
    }
    Ok(None)
}

/// The participant body as "that player <body>", parsed as an ordinary chain
/// with the iterated player as actor.
fn that_player_body(body: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    let mut tokens = vec![
        OwnedLexToken::word("that", crate::cards::builders::TextSpan::synthetic()),
        OwnedLexToken::word("player", crate::cards::builders::TextSpan::synthetic()),
    ];
    tokens.extend_from_slice(crate::lexer::trim_lexed_commas(body));
    crate::effect_sentences::parse_effect_chain_lexed(&tokens)
}

/// "Each player draws a card for each vote they received." (Círdan the
/// Shipwright): each player repeats the body once per vote they received
/// in the preceding player vote (CR 701.38).
fn read_each_player_per_vote_received(
    input: &VoteSentence<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::grammar::primitives;
    let tokens = crate::util::trim_edge_punctuation_tokens(input.tokens);
    let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::phrase(&["each", "player"]))
    else {
        return Ok(None);
    };
    const SUFFIX: &[&str] = &["for", "each", "vote", "they", "received"];
    let Some(body_len) = rest.len().checked_sub(SUFFIX.len()) else {
        return Ok(None);
    };
    let (body, suffix) = rest.split_at(body_len);
    if body.is_empty()
        || primitives::parse_prefix(suffix, primitives::phrase(SUFFIX))
            .is_none_or(|(_, after)| !after.is_empty())
    {
        return Ok(None);
    }
    let effects = that_player_body(body)?;
    Ok(Some(EffectAst::ForEach(
        crate::cards::builders::ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::ForEach(
                crate::cards::builders::ForEachEffectAst::RepeatEffects {
                    count: Value::PlayerVoteCount(PlayerFilter::IteratedPlayer),
                    effects,
                },
            )],
        },
    )))
}

/// "Each player who received no votes may put a permanent card from their
/// hand onto the battlefield." (Círdan the Shipwright): the body applies to
/// each player whose vote total in the preceding player vote is zero.
fn read_each_player_who_received_no_votes(
    input: &VoteSentence<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    use crate::grammar::primitives;
    let tokens = crate::util::trim_edge_punctuation_tokens(input.tokens);
    let Some((_, body)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["each", "player", "who", "received", "no", "votes"]),
    ) else {
        return Ok(None);
    };
    if body.is_empty() {
        return Ok(None);
    }
    let effects = that_player_body(body)?;
    Ok(Some(EffectAst::ForEach(
        crate::cards::builders::ForEachEffectAst::ForEachPlayer {
            effects: vec![EffectAst::Conditionals(
                crate::cards::builders::ConditionalEffectAst::Conditional {
                    predicate: crate::cards::builders::PredicateAst::ValueComparison {
                        left: Value::PlayerVoteCount(PlayerFilter::IteratedPlayer),
                        operator: crate::effect::ValueComparisonOperator::Equal,
                        right: Value::Fixed(0),
                    },
                    if_true: effects,
                    if_false: Vec::new(),
                },
            )],
        },
    )))
}
