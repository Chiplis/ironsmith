//! The typed multi-sentence effect bundles (two- and three-sentence programs the
//! composition layer reads as one), formerly a first-match ladder in
//! `composition_core`. Every reading runs, resolved by rank while the overlaps
//! are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct Bundle<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) sentences: Vec<&'a [OwnedLexToken]>,
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl Bundle<'_> {
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
                RuleId::new("effect-bundle-registry-reading"),
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
    admits: fn(&Bundle<'_>) -> bool,
    read: fn(&Bundle<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("effect-bundle-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("two-sentence-procedure"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_two_sentence_procedure(input)),
    },
    Reading {
        id: RuleId::new("resolving-card-exile-then-return-next-end-step"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_resolving_card_exile_then_return_next_end_step(input)),
    },
    Reading {
        id: RuleId::new("choose-mixed-targets-then-for-each"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_mixed_targets_then_for_each(input)),
    },
    Reading {
        id: RuleId::new("untap-then-phase-out-until-source-leaves-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_untap_then_phase_out_until_source_leaves_bundle(input)),
    },
    Reading {
        id: RuleId::new("inline-look-exile-face-down-permission-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_inline_look_exile_face_down_permission_bundle(input)),
    },
    Reading {
        id: RuleId::new("inline-exile-top-then-put-from-among-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_inline_exile_top_then_put_from_among_bundle(input)),
    },
    Reading {
        id: RuleId::new("inline-mill-then-put-from-among-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_inline_mill_then_put_from_among_bundle(input)),
    },
    Reading {
        id: RuleId::new("hidden-exile-partition-permission-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_hidden_exile_partition_permission_bundle(input)),
    },
    Reading {
        id: RuleId::new("discard-redraw-mana-value-ladder-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_discard_redraw_mana_value_ladder_bundle(input)),
    },
    Reading {
        id: RuleId::new("energy-pay-any-destroy-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_energy_pay_any_destroy_bundle(input)),
    },
    Reading {
        id: RuleId::new("consult-disposition-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_consult_disposition_bundle(input)),
    },
    Reading {
        id: RuleId::new("reveal-repeated-disposition-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_reveal_repeated_disposition_bundle(input)),
    },
    Reading {
        id: RuleId::new("reveal-from-outside-game-to-hand"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_reveal_from_outside_game_to_hand(input)),
    },
    Reading {
        id: RuleId::new("each-player-hand-exile-play-constraints-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_each_player_hand_exile_play_constraints_bundle(input)),
    },
    Reading {
        id: RuleId::new("look-hand-optional-exile-play-tax-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_look_hand_optional_exile_play_tax_bundle(input)),
    },
    Reading {
        id: RuleId::new("persistent-exile-play-tax-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_persistent_exile_play_tax_bundle(input)),
    },
    Reading {
        id: RuleId::new("controller-sacrifice-consult-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_controller_sacrifice_consult_bundle(input)),
    },
    Reading {
        id: RuleId::new("each-player-shuffle-then-consult-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_each_player_shuffle_then_consult_bundle(input)),
    },
    Reading {
        id: RuleId::new("proliferate-choose-phase-out-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_proliferate_choose_phase_out_bundle(input)),
    },
    Reading {
        id: RuleId::new("tap-controlled-objects-then-empty-mana-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tap_controlled_objects_then_empty_mana_bundle(input)),
    },
    Reading {
        id: RuleId::new("reveal-until-land-put-all-graveyard-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_reveal_until_land_put_all_graveyard_bundle(input)),
    },
    Reading {
        id: RuleId::new("bid-life-for-control-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_bid_life_for_control_bundle(input)),
    },
    Reading {
        id: RuleId::new("exile-collection-each-upkeep-return"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_collection_each_upkeep_return(input)),
    },
    Reading {
        id: RuleId::new("choose-each-graveyard-then-owner-shuffle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_each_graveyard_then_owner_shuffle(input)),
    },
    Reading {
        id: RuleId::new("untap-then-phase-out-then-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_untap_then_phase_out_then_followup(input)),
    },
    Reading {
        id: RuleId::new("regenerate-then-gain-control"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_regenerate_then_gain_control(input)),
    },
    Reading {
        id: RuleId::new("consult-then-put-matches-battlefield-rest-bottom"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_consult_then_put_matches_battlefield_rest_bottom(input)),
    },
    Reading {
        id: RuleId::new("exile-then-source-leaves-return"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_then_source_leaves_return(input)),
    },
    Reading {
        id: RuleId::new("exile-top-library-then-play"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_top_library_then_play(input)),
    },
    Reading {
        id: RuleId::new("optional-result-exile-choice-play"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_optional_result_exile_choice_play(input)),
    },
    Reading {
        id: RuleId::new("choose-one-of-them-three-sentences"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_one_of_them_three_sentences(input)),
    },
    Reading {
        id: RuleId::new("may-cast-spell-for-alternative-cost"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_may_cast_spell_for_alternative_cost(input)),
    },
    Reading {
        id: RuleId::new("choose-type-then-phase-out"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_type_then_phase_out(input)),
    },
    Reading {
        id: RuleId::new("reveal-from-outside-game-or-choose-face-up-exile-to-hand"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_reveal_from_outside_game_or_choose_face_up_exile_to_hand(input))
        },
    },
    Reading {
        id: RuleId::new("selected-hand-double-choice-discard"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_selected_hand_double_choice_discard(input)),
    },
    Reading {
        id: RuleId::new("discard-reveal-choose-discard-chosen"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_discard_reveal_choose_discard_chosen(input)),
    },
    Reading {
        id: RuleId::new("choose-mixed-targets-then-for-each-three"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_mixed_targets_then_for_each_three(input)),
    },
    Reading {
        id: RuleId::new("choose-mixed-targets-then-for-each-two"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_mixed_targets_then_for_each_two(input)),
    },
    Reading {
        id: RuleId::new("choose-objects-then-for-each-of-those-three"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("choose-mixed-targets-then-for-each-three")
        },
        read: |input| input.outcome(read_choose_objects_then_for_each_of_those_three(input)),
    },
    Reading {
        id: RuleId::new("choose-objects-then-for-each-of-those-two"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("choose-mixed-targets-then-for-each")
        },
        read: |input| input.outcome(read_choose_objects_then_for_each_of_those_two(input)),
    },
    Reading {
        id: RuleId::new("choose-counter-on-target-then-put-or-remove"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_counter_on_target_then_put_or_remove(input)),
    },
    Reading {
        id: RuleId::new("choose-counter-on-target-then-put-additional"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_counter_on_target_then_put_additional(input)),
    },
    Reading {
        id: RuleId::new("choose-card-type-then-reveal-top-and-put-chosen-to-hand"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_choose_card_type_then_reveal_top_and_put_chosen_to_hand(input))
        },
    },
    Reading {
        id: RuleId::new("choice-then-reveal-top-count-put-matching"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choice_then_reveal_top_count_put_matching(input)),
    },
    Reading {
        id: RuleId::new("kicked-counter-mana-value-replacement-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_kicked_counter_mana_value_replacement_bundle(input)),
    },
    Reading {
        id: RuleId::new("search-library-slots-to-hand-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_search_library_slots_to_hand_bundle(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &Bundle<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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

fn read_two_sentence_procedure(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2 {
        let sentence_inputs = sentences
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();
        if let Ok(Some(effects)) = super::super::super::sequence_rules::generic_subject_verb_sequences::exile_permission_followups::parse_dynamic_exile_top_then_play_for_as_long_as_exiled(
                &sentence_inputs,
                0,
            ) {
                return Ok(Some(effects));
            }
    }
    Ok(None)
}
fn read_resolving_card_exile_then_return_next_end_step(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && bundle_grammar::is_resolving_card_exile_then_return_next_end_step_shape(
            sentences[0],
            sentences[1],
        )
    {
        return Ok(Some(vec![
            EffectAst::subject_verb_register_zone_replacement_with_linked_exile_follow_up(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None),
                Some(Zone::Stack),
                Some(Zone::Graveyard),
                Zone::Exile,
                ZoneReplacementDurationAst::OneShot,
                ironsmith_core::LinkedExileFollowUp::ReturnToHandAtNextEndStep,
            ),
        ]));
    }
    Ok(None)
}
fn read_choose_mixed_targets_then_for_each(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    // A consult procedure nested under "for each of" belongs to the declared
    // mixed target collection. Claim that typed declaration/iteration shape
    // before the broad consult-disposition recognizer can start at the inner
    // reveal clause and discard the outer target declaration.
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_mixed_targets_then_for_each_bundle(sentences[0], sentences[1], None)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_untap_then_phase_out_until_source_leaves_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_untap_then_phase_out_until_source_leaves_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_inline_look_exile_face_down_permission_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Ok(Some(effects)) = parse_inline_look_exile_face_down_permission_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_inline_exile_top_then_put_from_among_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Ok(Some(effects)) = parse_inline_exile_top_then_put_from_among_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_inline_mill_then_put_from_among_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Ok(Some(effects)) = parse_inline_mill_then_put_from_among_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_hidden_exile_partition_permission_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Ok(Some(effects)) = parse_hidden_exile_partition_permission_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_discard_redraw_mana_value_ladder_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_discard_redraw_mana_value_ladder_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_energy_pay_any_destroy_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_energy_pay_any_destroy_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_consult_disposition_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_consult_disposition_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_reveal_repeated_disposition_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_reveal_repeated_disposition_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_reveal_from_outside_game_to_hand(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Ok(Some(effects)) = parse_reveal_from_outside_game_to_hand(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_each_player_hand_exile_play_constraints_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_each_player_hand_exile_play_constraints_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_look_hand_optional_exile_play_tax_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_look_hand_optional_exile_play_tax_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_persistent_exile_play_tax_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_persistent_exile_play_tax_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_controller_sacrifice_consult_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_controller_sacrifice_consult_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_each_player_shuffle_then_consult_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_each_player_shuffle_then_consult_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_proliferate_choose_phase_out_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_proliferate_choose_phase_out_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_tap_controlled_objects_then_empty_mana_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_tap_controlled_objects_then_empty_mana_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_reveal_until_land_put_all_graveyard_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_reveal_until_land_put_all_graveyard_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_bid_life_for_control_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_bid_life_for_control_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_collection_each_upkeep_return(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_collection_each_upkeep_return_bundle(sentences[0], sentences[1])
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_each_graveyard_then_owner_shuffle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_each_graveyard_then_owner_shuffle_bundle(sentences[0], sentences[1])
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_untap_then_phase_out_then_followup(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Some(mut effects) =
            parse_untap_then_phase_out_until_source_leaves_bundle(sentences[0])
        && let Ok(mut follow_up) = effect_sentences::parse_effect_sentence_lexed(sentences[1])
    {
        effects.append(&mut follow_up);
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_regenerate_then_gain_control(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Some(effects) =
            parse_regenerate_then_gain_control_if_regenerates_bundle(sentences[0], sentences[1])
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_consult_then_put_matches_battlefield_rest_bottom(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_consult_then_put_matches_battlefield_rest_bottom_bundle(
            sentences[0],
            sentences[1],
        )
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_then_source_leaves_return(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_then_source_leaves_return_bundle(sentences[0], sentences[1])
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_exile_top_library_then_play(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_top_library_then_play_bundle(sentences[0], sentences[1], None)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_optional_result_exile_choice_play(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_optional_result_exile_choice_play_bundle(&sentences)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_one_of_them_three_sentences(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 3
        && matches!(
            words(sentences[1]).as_slice(),
            ["choose", "one", "of", "them"]
                | ["you", "choose", "one", "of", "them"]
                | ["choose", "one", "of", "those", "cards"]
                | ["you", "choose", "one", "of", "those", "cards"]
        )
        && let Ok(Some(effects)) =
            parse_exile_top_library_then_play_bundle(sentences[0], sentences[1], Some(sentences[2]))
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_may_cast_spell_for_alternative_cost(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Some(effects) =
            parse_may_cast_spell_for_alternative_cost_bundle(sentences[0], sentences[1])
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_type_then_phase_out(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_type_then_phase_out_bundle(sentences[0], sentences[1])
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
            sentences[0],
            sentences[1],
        )
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_selected_hand_double_choice_discard(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_selected_hand_double_choice_discard_bundle(&sentences)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_discard_reveal_choose_discard_chosen(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_discard_reveal_choose_discard_chosen_bundle(&sentences)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_mixed_targets_then_for_each_three(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_choose_mixed_targets_then_for_each_bundle(
            sentences[0],
            sentences[1],
            Some(sentences[2]),
        )
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_mixed_targets_then_for_each_two(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_mixed_targets_then_for_each_bundle(sentences[0], sentences[1], None)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_objects_then_for_each_of_those_three(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_choose_objects_then_for_each_of_those_bundle(
            sentences[0],
            sentences[1],
            Some(sentences[2]),
        )
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_objects_then_for_each_of_those_two(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_objects_then_for_each_of_those_bundle(sentences[0], sentences[1], None)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_counter_on_target_then_put_or_remove(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_counter_on_target_then_put_or_remove_bundle(sentences[0], sentences[1])
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_counter_on_target_then_put_additional(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_counter_on_target_then_put_additional_bundle(sentences[0], sentences[1])
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
                sentences[0],
                sentences[1],
            )
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_choice_then_reveal_top_count_put_matching(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = &input.sentences;
    if sentences.len() == 3
        && {
            let first_words = crate::lexer::token_word_refs(sentences[0]);
            let choice_words = if first_words.first().copied() == Some("you") {
                &first_words[1..]
            } else {
                &first_words[..]
            };
            matches!(
                parse_choose_card_type_phrase_words(choice_words),
                Ok(Some((consumed, _))) if consumed == choice_words.len()
            )
        }
        && let Ok(Some(mut effects)) =
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
                sentences[1],
                sentences[2],
            )
    {
        let first_words = crate::lexer::token_word_refs(sentences[0]);
        let choice_words = if first_words.first().copied() == Some("you") {
            &first_words[1..]
        } else {
            &first_words[..]
        };
        let (_, options) = crate::grammar::primitives::probe_shape(
            parse_choose_card_type_phrase_words(choice_words),
        )
        .flatten()
        .expect("validated choose-card-type bundle prefix");
        let mut combined = vec![EffectAst::subject_verb_choose_card_type(
            PlayerAst::You,
            options,
        )];
        combined.append(&mut effects);
        return Ok(Some(combined));
    }
    Ok(None)
}
fn read_kicked_counter_mana_value_replacement_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_kicked_counter_mana_value_replacement_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_search_library_slots_to_hand_bundle(
    input: &Bundle<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Ok(Some(effects)) = parse_search_library_slots_to_hand_bundle(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
