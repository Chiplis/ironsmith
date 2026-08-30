use super::super::token_primitives::lexed_head_words;
use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst};
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId};
use crate::registry::furthest_committed_diagnostic;

pub(super) mod generic_subject_verb_sequences;

type DocumentProgramPredicate = fn(&[SentenceInput], usize) -> bool;
type DocumentProgramParser = fn(&[SentenceInput], usize) -> ParseOutcome<Vec<EffectAst>>;

fn document_program_outcome(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    result: Result<Option<Vec<EffectAst>>, CardTextError>,
) -> ParseOutcome<Vec<EffectAst>> {
    let span = sentences
        .get(sentence_idx)
        .and_then(|sentence| crate::util::span_from_tokens(sentence.lowered()));
    match result {
        Ok(Some(effects)) => ParseOutcome::matched(effects, span),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
            RuleId::new("document-program-parser"),
            span,
            error,
        )),
    }
}

macro_rules! structured_document_parser {
    ($parser:path) => {
        |sentences, sentence_idx| {
            document_program_outcome(sentences, sentence_idx, $parser(sentences, sentence_idx))
        }
    };
}

struct DocumentProgramRuleDef {
    name: &'static str,
    feature_tag: Option<&'static str>,
    consumed_sentences: usize,
    predicate: DocumentProgramPredicate,
    parser: DocumentProgramParser,
}

#[derive(Debug, Clone)]
pub struct DocumentProgramMatch {
    pub name: &'static str,
    pub feature_tag: Option<&'static str>,
    pub consumed_sentences: usize,
    pub effects: Vec<EffectAst>,
}

fn sentence_head(sentences: &[SentenceInput], sentence_idx: usize) -> Option<(&str, Option<&str>)> {
    lexed_head_words(sentences[sentence_idx].lowered())
}

fn sentence_head_word(sentences: &[SentenceInput], sentence_idx: usize) -> Option<&str> {
    sentence_head(sentences, sentence_idx).map(|(head, _)| head)
}

fn sentence_head_is(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    expected: (&str, Option<&str>),
) -> bool {
    sentence_head(sentences, sentence_idx) == Some(expected)
}

fn sentence_head_word_is(sentences: &[SentenceInput], sentence_idx: usize, expected: &str) -> bool {
    sentence_head_word(sentences, sentence_idx) == Some(expected)
}

fn sentence_head_word_in(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    expected: &[&str],
) -> bool {
    sentence_head_word(sentences, sentence_idx).is_some_and(|head| expected.contains(&head))
}

fn first_word_when_or_whenever(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["when", "whenever"])
}

fn first_word_you_or_untap(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["you", "untap"])
}

fn first_word_if(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "if")
}

fn first_word_draw(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "draw")
}

fn first_word_you(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "you")
}

fn first_word_you_or_until(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["you", "until"])
}

fn first_word_look(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "look")
}

fn first_word_mill(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "mill")
}

fn first_word_mill_sequence_candidate(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["mill", "you", "if", "target"])
}

fn first_word_search(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "search")
}

fn first_word_destroy(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "destroy")
}

fn first_word_put(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "put")
}

fn first_word_copy(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "copy")
}

fn first_word_up(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "up")
}

fn first_word_sacrifice(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "sacrifice")
}

fn first_word_exile(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "exile")
}

fn first_word_counter(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "counter")
}

fn first_word_exile_or_target(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["exile", "target"])
}

fn first_word_exile_target_or_shuffle(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["exile", "target", "shuffle"])
}

fn first_word_look_or_reveal(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["look", "reveal"])
}

fn first_word_consult_reveal_candidate(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(
        sentences,
        sentence_idx,
        &[
            "reveal",
            "defending",
            "target",
            "each",
            "that",
            "they",
            "you",
        ],
    )
}

fn first_word_target_exile_look_or_reveal(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> bool {
    sentence_head_word_in(
        sentences,
        sentence_idx,
        &[
            "if", "target", "you", "that", "they", "exile", "look", "reveal",
        ],
    )
}

fn first_word_then_target_exile_look_or_reveal(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> bool {
    sentence_head_word_in(
        sentences,
        sentence_idx,
        &[
            "then", "target", "you", "that", "they", "exile", "look", "reveal",
        ],
    )
}

fn first_word_if_target_exile_or_reveal(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(
        sentences,
        sentence_idx,
        &["if", "target", "exile", "reveal"],
    )
}

fn first_word_then_if_target_exile_or_reveal(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> bool {
    sentence_head_word_in(
        sentences,
        sentence_idx,
        &["then", "if", "target", "exile", "reveal"],
    )
}

fn first_word_prevent(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "prevent")
}

fn first_word_the(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "the")
}

fn first_word_at(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "at")
}

fn first_word_tap(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "tap")
}

fn first_word_starting(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "starting")
}

fn first_word_choose(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "choose")
}

fn sentence_words_contain(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    phrase: &[&str],
) -> bool {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return false;
    };
    let words = crate::lexer::token_word_refs(sentence.lowered());
    crate::word_primitives::sequence_occurs(&words, phrase)
}

fn choose_two_targets_counter_fight_candidate(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> bool {
    sentence_words_contain(sentences, sentence_idx, &["choose", "two", "target"])
}

fn choose_land_or_nonland_candidate(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_word_choose(sentences, sentence_idx)
        && sentence_words_contain(sentences, sentence_idx, &["land", "or", "nonland"])
}

fn choose_card_name_candidate(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_word_choose(sentences, sentence_idx)
        && sentence_words_contain(sentences, sentence_idx, &["card", "name"])
}

fn choose_explicit_stack_target_candidate(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> bool {
    if !first_word_choose(sentences, sentence_idx)
        || !sentence_words_contain(sentences, sentence_idx, &["target"])
    {
        return false;
    }
    let Some(sentence) = sentences.get(sentence_idx) else {
        return false;
    };
    sentence
        .lowered()
        .iter()
        .any(|token| token.is_any_word(&["ability", "spell"]))
}

fn choose_do_same_candidate(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_word_choose(sentences, sentence_idx)
        && (sentence_words_contain(sentences, sentence_idx, &["do", "same"])
            || sentence_words_contain(sentences, sentence_idx, &["do", "the", "same"]))
}

fn choose_same_controller_targets_candidate(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> bool {
    first_word_choose(sentences, sentence_idx)
        && (sentence_words_contain(
            sentences,
            sentence_idx,
            &["controlled", "by", "the", "same", "player"],
        ) || sentence_words_contain(
            sentences,
            sentence_idx,
            &["controlled", "by", "same", "player"],
        ))
}

fn choose_card_type_candidate(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_word_choose(sentences, sentence_idx)
        && sentence_words_contain(sentences, sentence_idx, &["card", "type"])
}

fn choose_creature_type_candidate(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_word_choose(sentences, sentence_idx)
        && sentence_words_contain(sentences, sentence_idx, &["creature", "type"])
}

fn first_word_choose_or_tempting(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["choose", "tempting"])
}

fn first_word_choose_or_each(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["choose", "each"])
}

fn first_word_each(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "each")
}

fn first_word_target(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "target")
}

fn first_word_that_or_the(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["that", "the"])
}

fn first_word_reveal(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "reveal")
}

fn first_head_look_at(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_is(sentences, sentence_idx, ("look", Some("at")))
}

fn first_head_look_at_or_if(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_head_look_at(sentences, sentence_idx)
        || sentence_head_word_is(sentences, sentence_idx, "if")
}

fn first_head_when_that(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_is(sentences, sentence_idx, ("when", Some("that")))
}

fn next_upkeep_unless_pays_window(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx + 1, "at")
        && sentence_head_word_is(sentences, sentence_idx + 2, "if")
}

fn prefixed_consult_window(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx + 1, &["exile", "reveal", "look"])
}

fn iterative_library_procedure_window(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "exile")
        && sentence_head_word_is(sentences, sentence_idx + 2, "repeat")
}

fn copy_for_each_target_window(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(
        sentences,
        sentence_idx,
        &["copy", "that", "you", "for", "if"],
    ) && sentence_head_word_is(sentences, sentence_idx + 1, "each")
}

fn for_each_tagged_copy_window(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["if", "for"])
        && sentence_head_word_is(sentences, sentence_idx + 1, "the")
}

fn damage_excess_exile_permission_window(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx + 1, "exile")
        && sentence_head_word_is(sentences, sentence_idx + 2, "you")
}

fn general_looked_destination_fallback(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    (first_word_then_target_exile_look_or_reveal(sentences, sentence_idx)
        || sentence_head_word_is(sentences, sentence_idx, "if"))
        && !matches!(
            generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_may_put_with_counter_then_rest_bottom(
                sentences,
                sentence_idx,
            ),
            Ok(Some(_))
        )
        && !matches!(
            generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_put_match_into_hand_rest_graveyard(
                sentences,
                sentence_idx,
            ),
            Ok(Some(_))
        )
        && !matches!(
            generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_put_match_onto_battlefield_and_match_into_hand_rest_bottom(
                sentences,
                sentence_idx,
            ),
            Ok(Some(_))
        )
}

fn reveal_hand_remainder_fallback(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_head_look_at(sentences, sentence_idx)
        && !matches!(
            generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_reveal_any_matching_to_hand_rest_bottom(
                sentences,
                sentence_idx,
            ),
            Ok(Some(_))
        )
}

fn singleton_hand_bottom_fallback(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_head_look_at(sentences, sentence_idx)
        && !matches!(
            generic_subject_verb_sequences::reference_linked_programs::parse_look_at_top_then_put_counted_hand_rest_bottom(
                sentences,
                sentence_idx,
            ),
            Ok(Some(_))
        )
}

fn singleton_hand_graveyard_fallback(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_head_look_at(sentences, sentence_idx)
        && !matches!(
            generic_subject_verb_sequences::reference_linked_programs::parse_look_at_top_then_partition_selected_and_remainder(
                sentences,
                sentence_idx,
            ),
            Ok(Some(_))
        )
}

fn consult_graveyard_remainder_fallback(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    first_word_target_exile_look_or_reveal(sentences, sentence_idx)
        && !matches!(
            generic_subject_verb_sequences::reference_linked_programs::parse_consult_match_into_battlefield_others_graveyard(
                sentences,
                sentence_idx,
            ),
            Ok(Some(_))
        )
}

const DOCUMENT_PROGRAM_RULES: &[DocumentProgramRuleDef] = &[
    DocumentProgramRuleDef {
        name: "draw-reveal-triggering-creature-mana-value-result",
        feature_tag: Some("drawn-card-triggering-object-dual-reference"),
        consumed_sentences: 2,
        predicate: first_word_draw,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_draw_reveal_then_triggering_creature_mana_value_result),
    },
    DocumentProgramRuleDef {
        name: "target-modifier-counter-instead-common-damage",
        feature_tag: Some("conditional-replacement-common-continuation"),
        consumed_sentences: 3,
        predicate: first_word_target,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_target_modifier_counter_instead_then_common_damage),
    },
    DocumentProgramRuleDef {
        name: "damage-excess-exile-top-play-until-next-turn",
        feature_tag: Some("damage-outcome-exile-permission"),
        consumed_sentences: 3,
        predicate: damage_excess_exile_permission_window,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_damage_then_excess_exile_top_then_play_until_next_turn),
    },
    DocumentProgramRuleDef {
        name: "destroy-set-no-regeneration-rider",
        feature_tag: Some("destroy-no-regeneration"),
        consumed_sentences: 2,
        predicate: first_word_destroy,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_destroy_then_no_regeneration_sequence),
    },
    DocumentProgramRuleDef {
        name: "reveal-then-exile-noncreature-nonland-hand-graveyard",
        feature_tag: Some("shared-owner-zone-union"),
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_reveal_then_exile_noncreature_nonland_hand_graveyard_sequence),
    },
    DocumentProgramRuleDef {
        name: "target-opponent-copy-triggering-spell-retarget",
        feature_tag: Some("target-opponent-copy-triggering-spell"),
        consumed_sentences: 2,
        predicate: first_word_up,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_target_opponent_may_copy_triggering_spell_then_retarget),
    },
    DocumentProgramRuleDef {
        name: "copy-next-spell-when-cast-retarget",
        feature_tag: Some("delayed-next-spell-copy"),
        consumed_sentences: 2,
        predicate: first_word_copy,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_copy_next_spell_when_cast_then_retarget),
    },
    DocumentProgramRuleDef {
        name: "opponent-optional-sacrifice-or-discard-correlated-damage",
        feature_tag: Some("per-opponent-optional-alternative-result"),
        consumed_sentences: 2,
        predicate: first_word_each,
        parser: structured_document_parser!(generic_subject_verb_sequences::optional_sacrifice_discard::parse_each_opponent_may_sacrifice_or_discard_then_damage_nonparticipants),
    },
    DocumentProgramRuleDef {
        name: "source-block-history-counter-otherwise",
        feature_tag: Some("source-block-history"),
        consumed_sentences: 2,
        predicate: first_word_put,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_counter_on_source_if_blocked_or_been_blocked_since_last_upkeep),
    },
    DocumentProgramRuleDef {
        name: "enchanted-combat-history-counter-otherwise",
        feature_tag: Some("enchanted-combat-history"),
        consumed_sentences: 2,
        predicate: first_word_put,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_counter_on_enchanted_if_attacked_or_blocked_since_last_upkeep),
    },
    DocumentProgramRuleDef {
        name: "participant-loot-greatest-mana-value-followup",
        feature_tag: Some("participant-result-extremum"),
        consumed_sentences: 2,
        predicate: first_word_you,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_controller_defending_loot_then_greatest_mana_value_followup),
    },
    DocumentProgramRuleDef {
        name: "counter-spell-artifact-creature-battlefield-replacement",
        feature_tag: Some("counter-zone-control-replacement"),
        consumed_sentences: 2,
        predicate: first_word_counter,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_counter_spell_then_artifact_or_creature_enters_under_your_control),
    },
    DocumentProgramRuleDef {
        name: "resolving-card-exile-return-next-end-step",
        feature_tag: Some("linked-zone-replacement-followup"),
        consumed_sentences: 2,
        predicate: first_word_exile,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_resolving_card_exile_then_return_next_end_step),
    },
    DocumentProgramRuleDef {
        name: "participant-secret-object-choice-reveal-sacrifice",
        feature_tag: Some("secret-participant-object-choice"),
        consumed_sentences: 2,
        predicate: first_word_you,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_participant_secret_object_choice_then_reveal_and_sacrifice),
    },
    DocumentProgramRuleDef {
        name: "exile-each-player-put-return-exiled-exile-source",
        feature_tag: Some("exiled-collection-return-after-player-actions"),
        consumed_sentences: 4,
        predicate: first_word_exile,
        parser: structured_document_parser!(generic_subject_verb_sequences::exiled_collections::parse_exile_each_player_put_return_exiled_then_exile_source),
    },
    DocumentProgramRuleDef {
        name: "graveyard-exile-then-copy-cast-copy",
        feature_tag: Some("graveyard-card-copy-cast"),
        consumed_sentences: 3,
        predicate: first_word_exile,
        parser: structured_document_parser!(generic_subject_verb_sequences::graveyard_copy_cast::parse_graveyard_exile_then_copy_then_may_cast_copy),
    },
    DocumentProgramRuleDef {
        name: "graveyard-exile-if-copy-cast-copy",
        feature_tag: Some("conditional-graveyard-card-copy-cast"),
        consumed_sentences: 3,
        predicate: first_word_exile,
        parser: structured_document_parser!(generic_subject_verb_sequences::graveyard_copy_cast::parse_graveyard_exile_if_copy_then_may_cast_copy),
    },
    DocumentProgramRuleDef {
        name: "graveyard-exile-copy-cast-copy",
        feature_tag: Some("graveyard-card-copy-cast"),
        consumed_sentences: 2,
        predicate: first_word_exile,
        parser: structured_document_parser!(generic_subject_verb_sequences::graveyard_copy_cast::parse_graveyard_exile_copy_then_may_cast_copy),
    },
    DocumentProgramRuleDef {
        name: "exile-top-play-event-followup",
        feature_tag: Some("exile-play-event-followup"),
        consumed_sentences: 3,
        predicate: first_word_exile,
        parser: structured_document_parser!(generic_subject_verb_sequences::exile_permission_followups::parse_exile_top_play_then_event_followup),
    },
    DocumentProgramRuleDef {
        name: "random-graveyard-exile-choose-copy-cast-copy",
        feature_tag: Some("exiled-collection-copy-cast"),
        consumed_sentences: 3,
        predicate: first_word_exile,
        parser: structured_document_parser!(generic_subject_verb_sequences::exiled_collections::parse_random_graveyard_exile_choose_copy_then_cast_copy),
    },
    DocumentProgramRuleDef {
        name: "exile-top-put-from-among-onto-battlefield",
        feature_tag: Some("exiled-collection-battlefield"),
        consumed_sentences: 2,
        predicate: first_word_exile,
        parser: structured_document_parser!(generic_subject_verb_sequences::exiled_collections::parse_exile_top_then_put_from_among_onto_battlefield),
    },
    DocumentProgramRuleDef {
        name: "exile-top-cast-collection-partition",
        feature_tag: Some("exiled-collection-cast-partition"),
        consumed_sentences: 3,
        predicate: first_word_exile_target_or_shuffle,
        parser: structured_document_parser!(generic_subject_verb_sequences::exiled_collections::parse_exile_top_cast_collection_then_partition),
    },
    DocumentProgramRuleDef {
        name: "exile-top-cast-collection-free",
        feature_tag: Some("exiled-collection-cast-choice"),
        consumed_sentences: 2,
        predicate: first_word_exile_target_or_shuffle,
        parser: structured_document_parser!(generic_subject_verb_sequences::exiled_collections::parse_exile_top_then_cast_collection_free),
    },
    DocumentProgramRuleDef {
        name: "tempting-offer-copy-spell",
        feature_tag: Some("tempting-offer-copy-spell"),
        consumed_sentences: 4,
        predicate: first_word_choose_or_tempting,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_tempting_offer_copy_spell_sequence),
    },
    DocumentProgramRuleDef {
        name: "revealed-opponent-hand-or-their-graveyard-choice",
        feature_tag: Some("revealed-hand-graveyard-choice"),
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_reveal_opponent_hand_then_choose_from_it_or_their_graveyard),
    },
    DocumentProgramRuleDef {
        name: "revealed-hand-optional-free-cast",
        feature_tag: Some("revealed-hand-cast-choice"),
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_reveal_target_opponent_hand_then_may_cast_from_those_cards),
    },
    DocumentProgramRuleDef {
        name: "looked-hand-optional-free-cast",
        feature_tag: Some("looked-hand-cast-choice"),
        consumed_sentences: 2,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_look_at_players_hand_then_may_cast_from_those_cards),
    },
    DocumentProgramRuleDef {
        name: "revealed-hand-shared-terminal-union-count",
        feature_tag: Some("revealed-hand-union-count"),
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_reveal_hand_then_draw_shared_terminal_union),
    },
    DocumentProgramRuleDef {
        name: "multi-target-restriction-destroy-typed-subset",
        feature_tag: Some("tagged-target-subset"),
        consumed_sentences: 2,
        predicate: first_word_up,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_multi_target_restriction_then_destroy_typed_subset),
    },
    DocumentProgramRuleDef {
        name: "reciprocal-creature-control",
        feature_tag: Some("reciprocal-creature-control"),
        consumed_sentences: 3,
        predicate: first_word_you_or_untap,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_reciprocal_creature_control_sequence),
    },
    DocumentProgramRuleDef {
        name: "revealed-and-or-choice-destination-override",
        feature_tag: Some("looked-cards-and-or-destination-replacement"),
        consumed_sentences: 4,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_reveal_top_choose_and_or_hand_rest_bottom_with_destination_override),
    },
    DocumentProgramRuleDef {
        name: "looked-matching-battlefield-then-shuffle",
        feature_tag: Some("looked-cards-battlefield-shuffle"),
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_put_matching_onto_battlefield_then_shuffle),
    },
    DocumentProgramRuleDef {
        name: "looked-battlefield-grant-rest-bottom",
        feature_tag: Some("looked-cards-battlefield-grant-remainder"),
        consumed_sentences: 4,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_top_cards_move_then_grant_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-reveal-one-or-instead-two-rest-bottom",
        feature_tag: Some("looked-cards-count-replacement-partition"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_reveal_one_or_instead_two_then_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "top-cards-move-rest-typed-when-result",
        feature_tag: Some("looked-cards-reflexive-move"),
        consumed_sentences: 4,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_top_cards_move_rest_then_typed_when_result),
    },
    DocumentProgramRuleDef {
        name: "consult-cleanup-typed-when-result",
        feature_tag: Some("consult-reflexive-cleanup"),
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_consult_cleanup_then_typed_when_result),
    },
    DocumentProgramRuleDef {
        name: "consult-reveal-pump-triggering-creature-move-revealed",
        feature_tag: Some("consult-revealed-collection-followup"),
        consumed_sentences: 3,
        predicate: first_word_consult_reveal_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_consult_reveal_then_pump_triggering_creature_then_move_revealed),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-reveal-counted-hand-then-shuffle",
        feature_tag: Some("looked-cards-reveal-hand-shuffle"),
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_reveal_counted_to_hand_then_shuffle),
    },
    DocumentProgramRuleDef {
        name: "sacrifice-reveal-top-choose-any-revealed-land-nonland-split-rest-bottom",
        feature_tag: Some("sacrifice-revealed-land-nonland-bottom"),
        consumed_sentences: 4,
        predicate: first_word_sacrifice,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_sacrifice_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-exile-counted-rest-bottom-play-while-exiled",
        feature_tag: Some("looked-cards-exile-play-while-exiled"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_exile_counted_rest_bottom_play_while_exiled),
    },
    DocumentProgramRuleDef {
        name: "search-reveal-named-match-battlefield-else-hand-then-shuffle",
        feature_tag: Some("search-named-card-branch"),
        consumed_sentences: 4,
        predicate: first_word_search,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_search_reveal_named_match_battlefield_else_hand_then_shuffle),
    },
    DocumentProgramRuleDef {
        name: "look-may-sacrifice-if-did-select-battlefield-rest-bottom",
        feature_tag: Some("looked-cards-intervening-action-partition"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-may-action-result-branches-move-looked-card",
        feature_tag: Some("looked-cards-result-branch-linkage"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_then_may_action_if_did_or_did_not_move_looked_card),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-conditional-hand-counts-rest-bottom",
        feature_tag: Some("looked-cards-conditional-cardinality-partition"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_conditional_hand_counts_then_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-optional-battlefield-conditional-entry-counters-rest-bottom",
        feature_tag: Some("looked-card-conditional-entry-counters"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_optional_battlefield_conditional_entry_counters_then_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-optional-battlefield-conditional-remainder",
        feature_tag: Some("looked-cards-conditional-remainder-partition"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_optional_battlefield_then_conditional_remainder),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-put-counted-into-hand-rest-bottom-kicker-override",
        feature_tag: Some("looked-cards-kicker-override"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-exile-one-rest-bottom-cast-else-hand",
        feature_tag: Some("looked-card-exile-cast-else-hand"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_exile_one_rest_bottom_cast_else_hand),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-may-exile-match-rest-bottom-cast-exiled",
        feature_tag: Some("looked-card-may-exile-cast-exiled"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_may_exile_match_rest_bottom_cast_exiled),
    },
    DocumentProgramRuleDef {
        name: "look-reveal-match-hand-selected-condition-rest-bottom",
        feature_tag: Some("looked-card-selected-condition-remainder"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_reveal_match_to_hand_if_selected_matches_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "reveal-top-optional-battlefield-then-hand-rest-graveyard",
        feature_tag: Some("looked-card-two-stage-graveyard-partition"),
        consumed_sentences: 4,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_reveal_top_optional_battlefield_then_hand_rest_graveyard),
    },
    DocumentProgramRuleDef {
        name: "look-reveal-your-turn-battlefield-else-hand-rest-bottom",
        feature_tag: Some("looked-card-your-turn-destination-partition"),
        consumed_sentences: 5,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_may_reveal_then_your_turn_battlefield_else_hand_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "destroy-historical-blocker-reanimation",
        feature_tag: Some("historical-block-controller-reanimation"),
        // This three-sentence program extends the ordinary destroy/no-
        // regeneration pair with a provenance-sensitive reanimation loop.
        // The document-program resolver compares consumed spans, so the
        // complete three-sentence candidate owns all of its authored input.
        consumed_sentences: 3,
        predicate: first_word_destroy,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_destroy_historically_blocked_then_reanimate_from_historical_controller),
    },
    DocumentProgramRuleDef {
        name: "destroy-for-each-destroyed-consult-exile-put-shuffle",
        feature_tag: Some("destroyed-consult-exile-put"),
        consumed_sentences: 3,
        predicate: first_word_destroy,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_destroy_for_each_destroyed_consult_exile_put_shuffle),
    },
    DocumentProgramRuleDef {
        name: "destroy-all-search-target-opponent-graveyard-shuffle",
        feature_tag: Some("destroy-search-library-partition"),
        consumed_sentences: 2,
        predicate: first_word_destroy,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_destroy_all_then_search_target_opponent_to_graveyard_then_shuffle),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-may-put-match-onto-battlefield-if-not-put-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-battlefield-or-hand"),
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-may-reveal-match-bargain-battlefield-else-hand-then-shuffle",
        feature_tag: Some("looked-cards-bargain-branch"),
        consumed_sentences: 5,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::branching_selection_programs::parse_look_at_top_may_reveal_match_bargain_battlefield_else_hand_then_shuffle),
    },
    DocumentProgramRuleDef {
        name: "reveal-top-opponent-chooses-then-exact-partition",
        feature_tag: Some("revealed-card-opponent-partition"),
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_reveal_top_opponent_chooses_then_partition),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-optional-one-top-remainder-bottom",
        feature_tag: Some("looked-cards-optional-top-bottom-partition"),
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_then_optional_one_top_then_remainder_bottom),
    },
    DocumentProgramRuleDef {
        name: "reveal-top-opponent-chooses-one-move-then-followup",
        feature_tag: Some("revealed-card-opponent-choice"),
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_reveal_top_opponent_chooses_one_then_move_and_followup),
    },
    DocumentProgramRuleDef {
        name: "each-player-mill-land-result-then-cast-one-milled-spell",
        feature_tag: Some("mill-result-permission"),
        consumed_sentences: 3,
        predicate: first_word_each,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_each_player_mill_then_land_result_then_cast_one_milled_spell),
    },
    DocumentProgramRuleDef {
        name: "choose-two-targets-counter-first-if-power-then-fight",
        feature_tag: Some("target-set-counter-fight"),
        consumed_sentences: 3,
        predicate: choose_two_targets_counter_fight_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_choose_two_targets_counter_first_if_power_then_fight),
    },
    DocumentProgramRuleDef {
        name: "choose-land-or-nonland-consult-hand-bottom",
        feature_tag: Some("consult-choice-kind"),
        consumed_sentences: 3,
        predicate: choose_land_or_nonland_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_choose_land_or_nonland_then_consult_to_hand_bottom),
    },
    DocumentProgramRuleDef {
        name: "choose-name-reveal-top-matching-hand-rest-graveyard",
        feature_tag: Some("looked-cards-chosen-name-rest-graveyard"),
        consumed_sentences: 3,
        predicate: choose_card_name_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_choose_name_reveal_top_matching_hand_rest_graveyard),
    },
    DocumentProgramRuleDef {
        name: "search-two-put-one-hand-other-graveyard-then-shuffle",
        feature_tag: Some("search-two-hand-graveyard"),
        consumed_sentences: 3,
        predicate: first_word_search,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_search_two_then_put_one_hand_other_graveyard_then_shuffle),
    },
    DocumentProgramRuleDef {
        name: "mill-then-payment-if-you-do-put-from-among-into-hand",
        feature_tag: Some("mill-payment-followup-choice"),
        consumed_sentences: 3,
        predicate: first_word_mill,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_mill_then_optional_payment_if_you_do_put_from_among_into_hand),
    },
    DocumentProgramRuleDef {
        name: "mill-then-put-from-among-into-hand-then-if-you-dont",
        feature_tag: Some("mill-followup-choice"),
        consumed_sentences: 3,
        predicate: first_word_mill,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_mill_then_may_put_from_among_into_hand_then_if_you_dont),
    },
    DocumentProgramRuleDef {
        name: "each-player-mill-exile-milled-creatures-create-power-token",
        feature_tag: Some("mill-exile-power-token"),
        consumed_sentences: 3,
        predicate: first_word_each,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_each_player_mill_then_exile_milled_creatures_then_create_power_token),
    },
    DocumentProgramRuleDef {
        name: "reveal-top-opponent-exiles-one-rest-hand-then-may-cast",
        feature_tag: Some("reveal-opponent-exile-rest-hand-cast"),
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_reveal_top_opponent_exiles_one_put_rest_hand_then_may_cast),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-may-put-with-counter-rest-bottom",
        feature_tag: Some("looked-card-optional-countered-entry"),
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_may_put_with_counter_then_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-partition-face-down-filtered-permission",
        feature_tag: Some("looked-card-hidden-filtered-permission"),
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_partition_face_down_then_filtered_permission),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-exile-match-and-rest-bottom-cast-exiled",
        feature_tag: Some("looked-card-exile-cast-exiled"),
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled),
    },
    DocumentProgramRuleDef {
        name: "search-player-names-card-conditional-put-then-shuffle",
        feature_tag: Some("search-name-choice-conditional-put"),
        consumed_sentences: 3,
        predicate: first_word_search,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_search_then_player_names_card_conditional_put_then_shuffle),
    },
    DocumentProgramRuleDef {
        name: "search-face-down-exile-conditional-cast-else-hand",
        feature_tag: Some("search-face-down-cast"),
        consumed_sentences: 3,
        predicate: first_word_search,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_search_face_down_exile_conditional_cast_else_hand),
    },
    DocumentProgramRuleDef {
        name: "top-cards-one-hand-then-matching-to-zone-rest-graveyard",
        feature_tag: Some("looked-cards-multi-subset-graveyard"),
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_one_hand_then_matching_to_zone_rest_graveyard),
    },
    DocumentProgramRuleDef {
        name: "top-cards-reveal-selection-rest-bottom-land-creature-split",
        feature_tag: Some("looked-cards-selected-type-split"),
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_reveal_selection_rest_bottom_then_land_creature_split),
    },
    DocumentProgramRuleDef {
        name: "optional-look-reveal-put-top-rest-bottom",
        feature_tag: Some("looked-cards-optional-top-bottom"),
        consumed_sentences: 2,
        predicate: first_word_you,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_optional_look_then_reveal_put_top_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "effect-then-next-upkeep-unless-pays-lose-game",
        feature_tag: Some("delayed-upkeep-payment"),
        consumed_sentences: 3,
        predicate: next_upkeep_unless_pays_window,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_search_delayed_upkeep_unless_pays_sequence),
    },
    DocumentProgramRuleDef {
        name: "next-upkeep-unless-pays-lose-game",
        feature_tag: Some("delayed-upkeep-payment"),
        consumed_sentences: 2,
        predicate: first_word_at,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_delayed_upkeep_unless_pays_sequence),
    },
    DocumentProgramRuleDef {
        name: "exile-until-match-cast-rest-bottom",
        feature_tag: Some("consult-cast-bottom"),
        consumed_sentences: 3,
        predicate: first_word_if_target_exile_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_exile_until_match_cast_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "exile-until-match-cast-else-hand",
        feature_tag: Some("consult-cast-or-hand"),
        consumed_sentences: 3,
        predicate: first_word_if_target_exile_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_exile_until_match_cast_else_hand),
    },
    DocumentProgramRuleDef {
        name: "reveal-top-choose-any-revealed-land-nonland-split-rest-bottom",
        feature_tag: Some("looked-cards-land-nonland-split"),
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "reveal-top-one-hand-gain-mana-value-rest-graveyard",
        feature_tag: Some("revealed-card-hand-value-rest"),
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_reveal_top_one_hand_gain_mana_value_rest_graveyard),
    },
    DocumentProgramRuleDef {
        name: "top-cards-put-match-into-hand-rest-graveyard",
        feature_tag: Some("looked-cards-hand-graveyard"),
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_put_match_into_hand_rest_graveyard),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-may-put-same-name-as-permanent-rest-bottom",
        feature_tag: Some("looked-cards-same-name-permanent"),
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_may_put_same_name_as_permanent_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "top-cards-may-cast-match-rest-bottom",
        feature_tag: Some("looked-cards-cast-bottom"),
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_may_cast_match_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "top-cards-put-any-matching-to-zone-rest-bottom",
        feature_tag: Some("looked-cards-any-matching-bottom"),
        consumed_sentences: 3,
        predicate: general_looked_destination_fallback,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_put_any_matching_to_zone_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-put-one-hand-bottom-cast-non-hand-put-all-hand",
        feature_tag: Some("looked-cards-cast-non-hand-override"),
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_put_one_hand_bottom_cast_non_hand_put_all_hand),
    },
    DocumentProgramRuleDef {
        name: "top-cards-reveal-any-matching-to-hand-rest-bottom",
        feature_tag: Some("looked-cards-revealed-hand-bottom"),
        consumed_sentences: 3,
        predicate: first_word_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_reveal_any_matching_to_hand_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-split-hand-bottom-exile-play",
        feature_tag: Some("looked-cards-split-play-exiled"),
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_split_hand_bottom_exile_then_play_exiled),
    },
    DocumentProgramRuleDef {
        name: "top-cards-choose-for-each-filter-one-battlefield-others-hand-rest-graveyard",
        feature_tag: Some("looked-cards-filter-bundle"),
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard),
    },
    DocumentProgramRuleDef {
        name: "top-cards-for-each-card-type-put-matching-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-card-type-choice"),
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "top-cards-for-each-card-type-among-spells-put-matching-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-card-type-choice"),
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "top-cards-put-match-onto-battlefield-and-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-battlefield-and-hand"),
        // This is a strict superset of the single-destination looked-card
        // rule. Run it first so the first "put" clause cannot consume the
        // sentence while silently dropping the coordinated hand choice.
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_put_match_onto_battlefield_and_match_into_hand_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-reveal-match-put-top-rest-bottom",
        feature_tag: Some("looked-cards-reveal-and-top"),
        consumed_sentences: 3,
        predicate: first_head_look_at,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_reveal_match_put_top_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-reveal-match-put-rest-bottom",
        feature_tag: Some("looked-cards-reveal-and-hand"),
        consumed_sentences: 3,
        predicate: reveal_hand_remainder_fallback,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_look_at_top_reveal_match_put_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "prefix-then-consult-match-move-bottom-remainder",
        feature_tag: Some("consult-prefixed-bottom"),
        consumed_sentences: 3,
        predicate: prefixed_consult_window,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_prefix_then_consult_match_move_and_bottom_remainder),
    },

    DocumentProgramRuleDef {
        name: "prefix-then-consult-match-into-hand-exile-others",
        feature_tag: Some("consult-prefixed-hand-exile"),
        consumed_sentences: 3,
        predicate: prefixed_consult_window,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_prefixed_library_consult_hand_exile_sequence),
    },
    DocumentProgramRuleDef {
        name: "iterative-library-procedure-sequence",
        feature_tag: Some("repeat-process"),
        consumed_sentences: 3,
        predicate: iterative_library_procedure_window,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_iterative_library_procedure_sequence),
    },
    DocumentProgramRuleDef {
        name: "each-player-repeat-pay-life-tokens",
        feature_tag: Some("repeat-process"),
        consumed_sentences: 3,
        predicate: first_word_starting,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_each_player_repeat_pay_life_tokens_sequence),
    },
    DocumentProgramRuleDef {
        name: "starting-each-player-optional-repeat",
        feature_tag: Some("repeat-process"),
        consumed_sentences: 2,
        predicate: first_word_starting,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_starting_each_player_optional_repeat_sequence),
    },
    DocumentProgramRuleDef {
        name: "target-gains-flashback-until-eot-targets-mana-cost",
        feature_tag: Some("flashback-cost-followup"),
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_parameterized_flashback_grant_sequence),
    },
    DocumentProgramRuleDef {
        name: "exile-face-down-pile-then-cloak-tapped",
        feature_tag: Some("cloak-pile"),
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_exile_face_down_pile_then_cloak),
    },
    DocumentProgramRuleDef {
        name: "each-player-shuffle-reveal-put-revealed-types-rest-bottom",
        feature_tag: Some("mass-reveal-battlefield-bottom"),
        consumed_sentences: 2,
        predicate: first_word_each,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_each_player_shuffle_reveal_then_put_revealed_types_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-counted-hand-rest-bottom",
        feature_tag: Some("looked-cards-counted-hand-bottom"),
        consumed_sentences: 2,
        predicate: first_word_look,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_look_at_top_then_put_counted_hand_rest_bottom),
    },
    DocumentProgramRuleDef {
        name: "top-cards-put-any-matching-to-zone-rest-bottom-same-sentence",
        feature_tag: Some("looked-cards-any-matching-bottom"),
        consumed_sentences: 2,
        predicate: first_word_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_top_cards_put_any_matching_to_zone_rest_same_sentence),
    },
    DocumentProgramRuleDef {
        name: "choose-phase-then-skip-chosen-this-turn",
        feature_tag: Some("choose-step-phase-skip"),
        consumed_sentences: 2,
        predicate: first_word_that_or_the,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_choose_draw_main_or_combat_phase_then_skip_chosen_this_turn),
    },
    DocumentProgramRuleDef {
        name: "copy-for-each-target-each-copy-different",
        feature_tag: Some("copy-target-assignment"),
        consumed_sentences: 2,
        predicate: copy_for_each_target_window,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_copy_for_each_target_then_each_copy_targets_different),
    },
    DocumentProgramRuleDef {
        name: "explicit-stack-target-copy-for-each-target",
        feature_tag: Some("copy-target-assignment"),
        consumed_sentences: 3,
        predicate: choose_explicit_stack_target_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::ordered_control_flow_programs::parse_explicit_stack_target_then_copy_for_each_target),
    },
    DocumentProgramRuleDef {
        name: "for-each-tagged-copy-then-copy-targets-it",
        feature_tag: Some("copy-target-assignment"),
        consumed_sentences: 2,
        predicate: for_each_tagged_copy_window,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_for_each_tagged_copy_then_copy_targets_it),
    },
    DocumentProgramRuleDef {
        name: "whenever-gain-life-then-self-animate-source",
        feature_tag: Some("self-animate-source"),
        consumed_sentences: 2,
        predicate: first_word_when_or_whenever,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_whenever_gain_life_then_self_animate_source),
    },
    DocumentProgramRuleDef {
        name: "filtered-future-exile-then-return-next-end-step",
        feature_tag: Some("filtered-future-zone-replacement"),
        consumed_sentences: 2,
        predicate: first_word_if,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_filtered_future_exile_then_return_next_end_step),
    },
    DocumentProgramRuleDef {
        name: "when-result-may-cast-target-graveyard-spell-then-exile-replacement",
        feature_tag: Some("reflexive-cast-target-graveyard-spell-replacement"),
        consumed_sentences: 2,
        predicate: first_word_when_or_whenever,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_when_result_may_cast_target_graveyard_spell_then_exile_replacement),
    },
    DocumentProgramRuleDef {
        name: "may-cast-target-graveyard-spell-then-exile-replacement",
        feature_tag: Some("cast-target-graveyard-spell-replacement"),
        consumed_sentences: 2,
        predicate: first_word_you_or_until,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_may_cast_target_graveyard_spell_then_exile_replacement),
    },
    DocumentProgramRuleDef {
        name: "gain-life-then-self-animate-source",
        feature_tag: Some("self-animate-source"),
        consumed_sentences: 2,
        predicate: first_word_you,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_gain_life_then_self_animate_source),
    },
    DocumentProgramRuleDef {
        name: "damage-prevention-then-delayed-creature-counters",
        feature_tag: Some("damage-prevention-delayed-followup"),
        consumed_sentences: 2,
        predicate: first_word_prevent,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_damage_prevention_delayed_counter_sequence),
    },
    DocumentProgramRuleDef {
        name: "damage-prevention-then-damage-any-target",
        feature_tag: Some("damage-prevention-followup"),
        consumed_sentences: 2,
        predicate: first_word_prevent,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_damage_prevention_reflect_to_any_target_sequence),
    },
    DocumentProgramRuleDef {
        name: "damage-prevention-then-put-counters",
        feature_tag: Some("damage-prevention-followup"),
        consumed_sentences: 2,
        predicate: first_word_prevent,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_damage_prevention_counter_sequence),
    },
    DocumentProgramRuleDef {
        name: "next-damage-prevention-then-gain-prevented-life",
        feature_tag: Some("damage-prevention-followup"),
        consumed_sentences: 2,
        predicate: first_word_the,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_next_damage_prevention_gain_life_sequence),
    },
    DocumentProgramRuleDef {
        name: "fixed-damage-prevention-then-gain-prevented-life",
        feature_tag: Some("damage-prevention-followup"),
        consumed_sentences: 2,
        predicate: first_word_prevent,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_next_damage_prevention_gain_life_sequence),
    },
    DocumentProgramRuleDef {
        name: "next-damage-prevention-then-exile-prevented-top-cards",
        feature_tag: Some("damage-prevention-followup"),
        consumed_sentences: 2,
        predicate: first_word_the,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_next_damage_prevention_exile_top_sequence),
    },
    DocumentProgramRuleDef {
        name: "tap-all-then-they-dont-untap-while-source-tapped",
        feature_tag: Some("tap-lock-followup"),
        consumed_sentences: 2,
        predicate: first_word_tap,
        parser: structured_document_parser!(generic_subject_verb_sequences::parse_tap_lock_sequence),
    },
    DocumentProgramRuleDef {
        name: "choose-then-do-same-for-filter-then-return-to-battlefield",
        feature_tag: Some("choose-repeat-filter"),
        consumed_sentences: 2,
        predicate: choose_do_same_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_choose_then_do_same_for_filter_then_return_to_battlefield),
    },
    DocumentProgramRuleDef {
        name: "choose-same-controller-targets-then-sacrifice-one-return-other",
        feature_tag: Some("same-controller-target-choice"),
        consumed_sentences: 3,
        predicate: choose_same_controller_targets_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_choose_same_controller_targets_then_sacrifice_one_return_other),
    },
    DocumentProgramRuleDef {
        name: "choose-same-controller-targets-then-sacrifice-one",
        feature_tag: Some("same-controller-target-choice"),
        consumed_sentences: 2,
        predicate: choose_same_controller_targets_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_choose_same_controller_targets_then_sacrifice_one),
    },
    DocumentProgramRuleDef {
        name: "choose-then-affect-rest",
        feature_tag: Some("choice-remainder-action"),
        consumed_sentences: 2,
        predicate: first_word_choose_or_each,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_choose_then_affect_rest),
    },
    DocumentProgramRuleDef {
        name: "subject-reveals-top-choose-one-and-move",
        feature_tag: Some("revealed-card-candidate-choice"),
        consumed_sentences: 2,
        predicate: first_word_that_or_the,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_reveal_top_then_choose_revealed_and_move),
    },
    DocumentProgramRuleDef {
        name: "delayed-dies-exile-top-power-choose-play",
        feature_tag: Some("delayed-dies-consult"),
        consumed_sentences: 2,
        predicate: first_head_when_that,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_delayed_dies_exile_top_power_choose_play),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-exile-face-down-play-while-exiled",
        feature_tag: Some("looked-card-play-while-exiled"),
        consumed_sentences: 2,
        predicate: first_head_look_at,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_look_at_top_then_exile_face_down_then_play_while_exiled),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-exact-one-graveyard",
        feature_tag: Some("looked-cards-exact-singleton-move"),
        consumed_sentences: 2,
        predicate: first_head_look_at,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_look_at_top_then_move_exact_one_to_graveyard),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-partition-selected-and-remainder",
        feature_tag: Some("looked-cards-selected-remainder-partition"),
        consumed_sentences: 2,
        predicate: first_head_look_at_or_if,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_look_at_top_then_partition_selected_and_remainder),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-put-one-hand-other-bottom",
        feature_tag: Some("looked-cards-hand-bottom"),
        consumed_sentences: 2,
        predicate: singleton_hand_bottom_fallback,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_look_at_top_then_put_one_hand_other_bottom),
    },
    DocumentProgramRuleDef {
        name: "look-at-top-put-one-hand-other-graveyard",
        feature_tag: Some("looked-cards-hand-graveyard"),
        consumed_sentences: 2,
        predicate: singleton_hand_graveyard_fallback,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_look_at_top_then_put_one_hand_other_graveyard),
    },
    DocumentProgramRuleDef {
        name: "mill-then-may-cast-from-among",
        feature_tag: Some("mill-followup-cast"),
        consumed_sentences: 2,
        predicate: first_word_mill_sequence_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_mill_then_may_cast_from_among),
    },
    DocumentProgramRuleDef {
        name: "mill-then-put-from-among-to-zone",
        feature_tag: Some("mill-followup-choice"),
        consumed_sentences: 2,
        predicate: first_word_mill_sequence_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_mill_then_may_put_from_among_into_hand),
    },
    DocumentProgramRuleDef {
        name: "exile-until-match-put-counters-on-match",
        feature_tag: Some("consult-match-counters"),
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_exile_until_match_put_counters_on_match),
    },
    DocumentProgramRuleDef {
        name: "exile-until-match-grant-play-this-turn",
        feature_tag: Some("consult-grant-play"),
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_exile_until_match_grant_play_this_turn),
    },
    DocumentProgramRuleDef {
        name: "target-chooses-other-cant-block",
        feature_tag: Some("target-choice-cant-block"),
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_target_player_chooses_then_other_cant_block),
    },
    DocumentProgramRuleDef {
        name: "choose-card-type-then-reveal-and-put",
        feature_tag: Some("choose-card-type"),
        consumed_sentences: 2,
        predicate: choose_card_type_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand),
    },
    DocumentProgramRuleDef {
        name: "choose-creature-type-then-become-type",
        feature_tag: Some("choose-creature-type"),
        consumed_sentences: 2,
        predicate: choose_creature_type_candidate,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_choose_creature_type_then_become_type),
    },
    DocumentProgramRuleDef {
        name: "reveal-top-matching-into-hand-rest-graveyard",
        feature_tag: Some("reveal-top-rest-graveyard"),
        consumed_sentences: 2,
        predicate: first_word_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard),
    },
    DocumentProgramRuleDef {
        name: "conditional-consult-match-move-bottom-remainder",
        feature_tag: Some("consult-conditional-bottom-remainder"),
        consumed_sentences: 2,
        predicate: first_word_then_if_target_exile_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_conditional_consult_match_move_and_bottom_remainder),
    },
    DocumentProgramRuleDef {
        name: "consult-match-move-bottom-remainder",
        feature_tag: Some("consult-bottom-remainder"),
        consumed_sentences: 2,
        predicate: first_word_then_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_consult_match_move_and_bottom_remainder),
    },
    DocumentProgramRuleDef {
        name: "directional-adjacent-player-control",
        feature_tag: Some("directional-player-choice-control"),
        consumed_sentences: 2,
        predicate: first_word_starting,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_directional_adjacent_player_control),
    },
    DocumentProgramRuleDef {
        name: "consult-match-onto-battlefield-or-into-hand",
        feature_tag: Some("consult-battlefield-or-hand"),
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_consult_match_into_battlefield_or_hand),
    },
    DocumentProgramRuleDef {
        name: "consult-match-move-graveyard-remainder",
        feature_tag: Some("consult-graveyard-remainder"),
        consumed_sentences: 2,
        predicate: consult_graveyard_remainder_fallback,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_consult_match_move_all_to_graveyard),
    },
    DocumentProgramRuleDef {
        name: "consult-match-into-hand-exile-others",
        feature_tag: Some("consult-hand-exile-others"),
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_consult_match_into_hand_exile_others),
    },
    DocumentProgramRuleDef {
        name: "consult-match-into-hand-others-graveyard",
        feature_tag: Some("consult-hand-graveyard-others"),
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_consult_match_into_hand_others_graveyard),
    },
    DocumentProgramRuleDef {
        name: "consult-match-into-battlefield-others-graveyard",
        feature_tag: Some("consult-battlefield-graveyard-others"),
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: structured_document_parser!(generic_subject_verb_sequences::reference_linked_programs::parse_consult_match_into_battlefield_others_graveyard),
    },
];

pub fn try_parse_document_program(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<DocumentProgramMatch>, CardTextError> {
    match recognize_document_program_registry(sentences, sentence_idx) {
        ParseOutcome::NoMatch => Ok(None),
        ParseOutcome::Match(matched) => Ok(Some(matched.value)),
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
    }
}

/// Recognize every typed document-program candidate at this sentence, keep
/// only candidates that consume the longest complete program, deduplicate
/// equivalent ASTs, and reject any remaining non-equivalent overlap.
/// Registration order is therefore never a semantic tie-breaker.
pub fn recognize_document_program_registry(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> ParseOutcome<DocumentProgramMatch> {
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    let span = sentences
        .get(sentence_idx)
        .and_then(|sentence| crate::util::span_from_tokens(sentence.lowered()));
    for rule in DOCUMENT_PROGRAM_RULES {
        if sentence_idx + rule.consumed_sentences > sentences.len() {
            continue;
        }
        if !(rule.predicate)(sentences, sentence_idx) {
            continue;
        }
        let effects = match (rule.parser)(sentences, sentence_idx).within(RuleId::new(rule.name)) {
            ParseOutcome::Match(matched) => matched.value,
            ParseOutcome::NoMatch => continue,
            ParseOutcome::Error(diagnostic) => {
                diagnostics.push(diagnostic);
                continue;
            }
        };
        let candidate = DocumentProgramMatch {
            name: rule.name,
            feature_tag: rule.feature_tag,
            consumed_sentences: rule.consumed_sentences,
            effects,
        };
        if candidates.iter().any(|existing: &DocumentProgramMatch| {
            existing.consumed_sentences == candidate.consumed_sentences
                && existing.effects == candidate.effects
        }) {
            continue;
        }
        candidates.push(candidate);
    }

    let longest = candidates
        .iter()
        .map(|candidate| candidate.consumed_sentences)
        .max()
        .unwrap_or(0);
    candidates.retain(|candidate| candidate.consumed_sentences == longest);

    match candidates.as_mut_slice() {
        [] => furthest_committed_diagnostic(diagnostics)
            .map(ParseOutcome::Error)
            .unwrap_or(ParseOutcome::NoMatch),
        [matched] => ParseOutcome::matched(matched.clone(), span),
        _ => {
            let alternatives = candidates
                .iter()
                .map(|candidate| RuleId::new(candidate.name))
                .collect::<Vec<_>>();
            let names = alternatives
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            ParseOutcome::Error(crate::recognition::ParseDiagnostic::ambiguous(
                RuleId::new("document-program"),
                span,
                alternatives,
                format!("non-equivalent document programs recognized the same input: {names}"),
            ))
        }
    }
}

pub fn document_program_route(name: &str) -> &'static str {
    match name {
        "prefix-then-consult-match-into-hand-exile-others" => {
            "document-program verb=Search subject=explicit recognizer=consult-library-procedure"
        }
        "iterative-library-procedure-sequence" => {
            "document-program verb=Exile subject=explicit recognizer=iterative-library-procedure"
        }
        "target-gains-flashback-until-eot-targets-mana-cost" => {
            "document-program verb=Gain subject=explicit recognizer=parameterized-flashback-grant"
        }
        _ => "document-program verb=Do subject=implicit recognizer=typed-composition",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{IfResultPredicate, SubjectVerbActionAst, SubjectVerbEffectAst};
    use crate::{lex_line, split_lexed_sentences};

    #[test]
    fn leading_then_looked_partition_uses_one_provenance_program() {
        let tokens = lex_line(
            "Then look at the top X cards of your library, where X is the number of time counters on this creature. You may put a nonland permanent card with mana value 3 or less from among them onto the battlefield. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();

        assert!(
            first_word_then_target_exile_look_or_reveal(&sentences, 0),
            "leading-then predicate must admit the sentence"
        );
        assert!(
            generic_subject_verb_sequences::ordered_control_flow_programs::parse_top_cards_put_any_matching_to_zone_rest_bottom(
                &sentences,
                0,
            )
            .expect("specialized parser")
            .is_some(),
            "specialized looked-partition parser must accept the three-sentence shape"
        );
        let matched = try_parse_document_program(&sentences, 0)
            .expect("sequence parse")
            .expect("leading-then looked partition should match a typed sequence rule");
        assert_eq!(
            matched.name,
            "top-cards-put-any-matching-to-zone-rest-bottom"
        );

        let [look, choose, move_each, remainder] = matched.effects.as_slice() else {
            panic!(
                "expected look/choose/move/remainder provenance program: {:#?}",
                matched.effects
            );
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtTopCards { tag: looked, .. },
            ..
        }) = look
        else {
            panic!("expected looked-card producer: {look:#?}");
        };
        let EffectAst::ChooseTaggedObjectsInZone {
            filter,
            tag: chosen,
            ..
        } = choose
        else {
            panic!("expected looked-card selection: {choose:#?}");
        };
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag == *looked),
            "selection must consume the looked-card pool: {filter:#?}"
        );
        assert!(matches!(
            move_each,
            EffectAst::ForEachTagged { tag, .. } if tag == chosen
        ));
        assert!(matches!(
            remainder,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: Some(keep_tagged),
                        ..
                    },
                ..
            }) if tag == looked && keep_tagged == chosen
        ));
    }

    #[test]
    fn conditional_looked_partition_keeps_the_full_looked_collection_for_the_remainder() {
        let tokens = lex_line(
            "If you do, look at the top X cards of your library, where X is that creature's mana value. You may put a creature card from among them onto the battlefield. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();

        assert!(general_looked_destination_fallback(&sentences, 0));
        let matched = try_parse_document_program(&sentences, 0)
            .expect("sequence parse")
            .expect("conditional looked partition should match a typed sequence rule");
        assert_eq!(
            matched.name,
            "top-cards-put-any-matching-to-zone-rest-bottom"
        );

        let [
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects,
            },
        ] = matched.effects.as_slice()
        else {
            panic!(
                "expected one conditional looked partition: {:#?}",
                matched.effects
            );
        };
        let [look, choose, move_each, remainder] = effects.as_slice() else {
            panic!("expected look/choose/move/remainder effects: {effects:#?}");
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtTopCards { tag: looked, .. },
            ..
        }) = look
        else {
            panic!("expected a looked-card producer: {look:#?}");
        };
        let EffectAst::ChooseTaggedObjectsInZone { tag: chosen, .. } = choose else {
            panic!("expected a typed looked-card choice: {choose:#?}");
        };
        assert!(matches!(move_each, EffectAst::ForEachTagged { tag, .. } if tag == chosen));
        assert!(matches!(
            remainder,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    order: crate::cards::builders::LibraryBottomOrderAst::Random,
                    ..
                },
                ..
            }) if tag == looked && keep_tagged == chosen
        ));
    }

    #[test]
    fn starting_each_player_optional_action_becomes_one_typed_repeat_process() {
        let tokens = lex_line(
            "Starting with you, each player may put a permanent card from their hand onto the battlefield. Repeat this process until no one puts a card onto the battlefield.",
            0,
        )
        .expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();

        let matched = try_parse_document_program(&sentences, 0)
            .expect("sequence parse")
            .expect("the optional participant process should match");
        assert_eq!(matched.name, "starting-each-player-optional-repeat");
        assert_eq!(matched.consumed_sentences, 2);

        let [
            EffectAst::RepeatProcess {
                effects,
                continue_effect_index,
                continue_predicate: IfResultPredicate::Did,
            },
        ] = matched.effects.as_slice()
        else {
            panic!(
                "expected one typed repeat process, got: {:#?}",
                matched.effects
            );
        };
        assert_eq!(*continue_effect_index, 0);
        let [
            EffectAst::SourceSentence {
                effects,
                starting_with_controller: true,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("the repeat body must retain authored participant order: {effects:#?}");
        };
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::ForEachPlayer {
                effects: per_player,
            }] if matches!(per_player.as_slice(), [EffectAst::May { .. }])
        ));
    }
}
