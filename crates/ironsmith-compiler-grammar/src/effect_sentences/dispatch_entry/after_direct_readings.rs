//! The readings of an effect document after the direct routes decline: the
//! complete simple shapes (face-down exile, votes, secret choices, controlled
//! object choice, draw, mill, typed coordination, a simple gain) and the
//! composable typed statements. Formerly a first-match ladder in
//! `dispatch_entry`; every reading runs, resolved by rank while the overlaps
//! are measured. The legacy document readings are the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct AfterDirect<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) sentences: &'a [&'a [OwnedLexToken]],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl AfterDirect<'_> {
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
        read: Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<EffectAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("after-direct-registry-reading"),
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
    admits: fn(&AfterDirect<'_>) -> bool,
    read: fn(&AfterDirect<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("after-direct-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("otherwise-face-down-exile-top"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_otherwise_face_down_exile_top(input)),
    },
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
        id: RuleId::new("secret-choices-match-conditional"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_secret_choices_match_conditional(input)),
    },
    Reading {
        id: RuleId::new("complete-simple-controlled-object-choice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_complete_simple_controlled_object_choice(input)),
    },
    Reading {
        id: RuleId::new("simple-face-down-exile-top"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_simple_face_down_exile_top(input)),
    },
    Reading {
        id: RuleId::new("simple-draw"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_simple_draw(input)),
    },
    Reading {
        id: RuleId::new("simple-mill"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_simple_mill(input)),
    },
    Reading {
        id: RuleId::new("direct-typed-coordination"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_direct_typed_coordination(input)),
    },
    Reading {
        id: RuleId::new("simple-gain-ability"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_simple_gain_ability(input)),
    },
    Reading {
        id: RuleId::new("composable-typed-statements"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("simple-face-down-exile-top")
        },
        read: |input| input.outcome(read_composable_typed_statements(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &AfterDirect<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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
    let mut distinct: Vec<RegistryCandidate<Vec<EffectAst>>> = Vec::new();
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

fn read_otherwise_face_down_exile_top(
    input: &AfterDirect<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some((exile_tokens, count)) = complete_simple_otherwise_face_down_exile_top_shape(tokens)
    {
        return Ok(Some(build_complete_simple_otherwise_face_down_exile_top(
            exile_tokens,
            count,
        )));
    }
    Ok(None)
}
fn read_secret_number_choice_vote_start(
    input: &AfterDirect<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) =
        super::super::dispatch_inner::parse_secret_number_choice_vote_start(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_vote_reveal(input: &AfterDirect<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = super::super::dispatch_inner::parse_vote_reveal_sentence(tokens) {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_secret_choices_match_conditional(
    input: &AfterDirect<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(source_type) = secret_choices_match_conditional_source_type(tokens)
    {
        return Ok(Some(build_secret_choices_match_conditional_effects(
            source_type,
        )));
    }
    Ok(None)
}
fn read_complete_simple_controlled_object_choice(
    input: &AfterDirect<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_complete_simple_controlled_object_choice(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_simple_face_down_exile_top(
    input: &AfterDirect<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = build_complete_simple_face_down_exile_top(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_simple_draw(input: &AfterDirect<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    if sentences.len() == 1
        && let Some(effect) = parse_complete_simple_draw_sentence(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_simple_mill(input: &AfterDirect<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    if sentences.len() == 1
        && let Some(effect) = parse_complete_simple_mill_sentence(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_direct_typed_coordination(
    input: &AfterDirect<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    if sentences.len() == 1
        && let Some(effects) = parse_direct_typed_coordination(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_simple_gain_ability(
    input: &AfterDirect<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    if sentences.len() == 1
        && let Some(shape) =
            effect_grammar::gain_ability_shapes::parse_simple_gain_ability_shape(tokens)
        && shape.complete
        && !shape.subject_tokens.first().is_some_and(|token| token.is_any_word(
            &["if", "unless", "when", "whenever", "at", "as", "then", "instead"]
        ))
        && !shape.subject_tokens.iter().any(|token| token.is_any_word(&["has", "have", "get", "gets"]))
        && !crate::word_primitives::sequence_occurs(
            &crate::lexer::parser_token_word_refs(tokens),
            &["as", "long", "as"],
        )
        && super::super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens).len() == 1
        && super::super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![tokens]).len()
            == 1
        && let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_composable_typed_statements(
    input: &AfterDirect<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let sentences = input.sentences;
    if !sentences.iter().any(|sentence| {
        effect_grammar::clause_primitive_shapes::parse_fight_shape(sentence).is_some()
    }) && let Some(effects) = parse_composable_typed_statements(sentences, tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
