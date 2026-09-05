//! The typed top-level subject/verb recognitions of one sentence, each with
//! its effect route, formerly a first-match ladder in `generic_subject_verb`.
//! What no reading claims goes to the generic top-level programs.
use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// One sentence read at the top level.
pub(super) struct TopLevelSentence<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl TopLevelSentence<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError>,
    ) -> ParseOutcome<(&'static str, Vec<EffectAst>)> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("top-level-subject-verb-registry-reading"),
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
    admits: fn(&TopLevelSentence<'_>) -> bool,
    read: fn(&TopLevelSentence<'_>) -> ParseOutcome<(&'static str, Vec<EffectAst>)>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("top-level-subject-verb-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("triggering-object-had-counters-create"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_triggering_object_had_counters_create(input)),
    },
    Reading {
        id: RuleId::new("source-exiled-counted-return-remainder-to-owners-libraries"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_source_exiled_counted_return_remainder_to_owners_libraries(input))
        },
    },
    Reading {
        id: RuleId::new("copular-animation"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_copular_animation(input)),
    },
    Reading {
        id: RuleId::new("branch-scoped-collection"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_branch_scoped_collection(input)),
    },
    Reading {
        id: RuleId::new("as-you-cast-from-zone-this-turn-grant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_as_you_cast_from_zone_this_turn_grant(input)),
    },
    Reading {
        id: RuleId::new("any-player-may-have-source-deal-damage"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_any_player_may_have_source_deal_damage(input)),
    },
    Reading {
        id: RuleId::new("destroy-attached-object-then-source-damage-to-controller"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_destroy_attached_object_then_source_damage_to_controller(input))
        },
    },
    Reading {
        id: RuleId::new("generic-play-exiled-cards-for-as-long-as-exiled"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_play_exiled_cards_for_as_long_as_exiled(input)),
    },
    Reading {
        id: RuleId::new("generic-mana-any-type-cast-tagged-this-way"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_mana_any_type_cast_tagged_this_way(input)),
    },
    Reading {
        id: RuleId::new("source-gets-unblockable"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_gets_unblockable(input)),
    },
    Reading {
        id: RuleId::new("target-gets-unblockable"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_gets_unblockable(input)),
    },
    Reading {
        id: RuleId::new("cant-blocked-then-base-pt"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cant_blocked_then_base_pt(input)),
    },
    Reading {
        id: RuleId::new("source-gets-filter-gains"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_gets_filter_gains(input)),
    },
    Reading {
        id: RuleId::new("target-player-controls-get"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_player_controls_get(input)),
    },
    Reading {
        id: RuleId::new("target-gains-then-gets"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_gains_then_gets(input)),
    },
    Reading {
        id: RuleId::new("attached-and-related-get"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_attached_and_related_get(input)),
    },
    Reading {
        id: RuleId::new("target-gets-then-gains"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let tokens = input.tokens;
            // "Creatures target player controls get ..." is the embedded-controller reading's.
            !crate::word_primitives::parse_sequence_prefix(
                &crate::lexer::parser_token_word_refs(tokens),
                &["creatures", "target", "player", "controls"],
            )
        },
        read: |input| input.outcome(read_target_gets_then_gains(input)),
    },
    Reading {
        id: RuleId::new("target-has-base-pt-then-loses"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_has_base_pt_then_loses(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs; two
/// readings that disagree are an ambiguity.
pub(super) fn read(
    input: &TopLevelSentence<'_>,
) -> ParseOutcome<RuleMatch<(&'static str, Vec<EffectAst>)>> {
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
    let mut distinct: Vec<RegistryCandidate<(&'static str, Vec<EffectAst>)>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    match &outcome {
        ParseOutcome::Match(matched) => {
            crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
        }
        ParseOutcome::Error(diagnostic) => {
            crate::parse_trace::event(format!("{REGISTRY}: error: {}", diagnostic.message));
        }
        ParseOutcome::NoMatch => {}
    }
    outcome
}

fn read_triggering_object_had_counters_create(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_triggering_object_had_counters_create_tokens(tokens)? {
        return Ok(Some((
            "subject-verb verb=Create subject=implicit recognizer=triggering-object-counter-lki",
            vec![effect],
        )));
    }
    Ok(None)
}
fn read_source_exiled_counted_return_remainder_to_owners_libraries(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_source_exiled_counted_return_remainder_to_owners_libraries(tokens)
    {
        return Ok(Some((
            "subject-verb verb=Return subject=source-exiled recognizer=counted-return-remainder",
            effects,
        )));
    }
    Ok(None)
}
fn read_copular_animation(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    // Copular animation clauses such as "those permanents are 4/4 creatures
    // in addition to their other types" are effect-backed state changes. They
    // must reach the generic animation parser before the broad `are`/`get`
    // subject-verb recognizers reinterpret the type and power text as a
    // granted static ability.
    if let Some(shape) =
        effect_grammar::clause_dispatch_shapes::parse_copular_animation_shape(tokens)
    {
        let effect = super::super::clause_dispatch::parse_become_clause(
            shape.subject_tokens,
            shape.animation_tokens,
        )?;
        return Ok(Some((
            "subject-verb verb=Become subject=explicit recognizer=copular-animation",
            vec![effect],
        )));
    }
    Ok(None)
}
fn read_branch_scoped_collection(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(parsed) = parse_branch_scoped_collection_subject_verb(tokens) {
        return Ok(Some(parsed));
    }
    Ok(None)
}
fn read_as_you_cast_from_zone_this_turn_grant(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_as_you_cast_from_zone_this_turn_grant(tokens)? {
        return Ok(Some((
            "subject-verb verb=Gain subject=cast-from-zone recognizer=as-you-cast-this-turn",
            vec![effect],
        )));
    }
    Ok(None)
}
fn read_any_player_may_have_source_deal_damage(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_any_player_may_have_source_deal_damage(tokens)? {
        return Ok(Some((
            "subject-verb verb=Deal subject=source recognizer=any-player-may-have-source-damage",
            effects,
        )));
    }
    Ok(None)
}
fn read_destroy_attached_object_then_source_damage_to_controller(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_destroy_attached_object_then_source_damage_to_controller(tokens)? {
        return Ok(Some((
            "subject-verb verb=Destroy subject=attached recognizer=destroy-attached-source-damage",
            effects,
        )));
    }
    Ok(None)
}
fn read_generic_play_exiled_cards_for_as_long_as_exiled(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_generic_play_exiled_cards_for_as_long_as_exiled(tokens) {
        return Ok(Some((
            "subject-verb verb=Play subject=implicit recognizer=exiled-cards-play-permission",
            vec![effect],
        )));
    }
    Ok(None)
}
fn read_generic_mana_any_type_cast_tagged_this_way(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_generic_mana_any_type_cast_tagged_this_way(tokens) {
        return Ok(Some((
            "subject-verb verb=Cast subject=implicit recognizer=tagged-any-mana-permission",
            vec![effect],
        )));
    }
    Ok(None)
}
fn read_source_gets_unblockable(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_source_gets_unblockable_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=source recognizer=source-pump-unblockable",
            effects,
        )));
    }
    Ok(None)
}
fn read_target_gets_unblockable(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_target_gets_unblockable_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=target recognizer=target-pump-unblockable",
            effects,
        )));
    }
    Ok(None)
}
fn read_cant_blocked_then_base_pt(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_cant_blocked_then_base_pt_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Cant subject=target recognizer=cant-blocked-base-pt",
            effects,
        )));
    }
    Ok(None)
}
fn read_source_gets_filter_gains(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_source_gets_filter_gains_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=source recognizer=source-pump-filter-gain",
            effects,
        )));
    }
    Ok(None)
}
fn read_target_player_controls_get(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_target_player_controls_get_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=target-player-controls recognizer=embedded-controller-pump",
            effects,
        )));
    }
    Ok(None)
}
fn read_target_gains_then_gets(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_target_gains_then_gets_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Gain subject=target recognizer=shared-subject-gain-get",
            effects,
        )));
    }
    Ok(None)
}
fn read_attached_and_related_get(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_attached_and_related_get_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=attached-and-related recognizer=shared-characteristic-pump",
            effects,
        )));
    }
    Ok(None)
}
fn read_target_gets_then_gains(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_target_gets_then_gains_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=target recognizer=shared-subject-get-gain",
            effects,
        )));
    }
    Ok(None)
}
fn read_target_has_base_pt_then_loses(
    input: &TopLevelSentence<'_>,
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_target_has_base_pt_then_loses_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Have subject=target recognizer=shared-subject-base-pt-lose",
            effects,
        )));
    }
    Ok(None)
}
