use super::super::token_primitives::lexed_head_words;
use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst};

pub(super) mod generic_subject_verb_sequences;

type SequenceRulePredicate = fn(&[SentenceInput], usize) -> bool;
type SequenceRuleParser =
    fn(&[SentenceInput], usize) -> Result<Option<Vec<EffectAst>>, CardTextError>;

struct SequenceRuleDef {
    name: &'static str,
    feature_tag: Option<&'static str>,
    priority: u16,
    consumed_sentences: usize,
    predicate: SequenceRulePredicate,
    parser: SequenceRuleParser,
}

pub(crate) struct SequenceRuleMatch {
    pub(crate) name: &'static str,
    pub(crate) feature_tag: Option<&'static str>,
    pub(crate) consumed_sentences: usize,
    pub(crate) effects: Vec<EffectAst>,
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
    sentence_head_word(sentences, sentence_idx)
        .is_some_and(|head| expected.iter().any(|candidate| head == *candidate))
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
    sentence_head_word_in(sentences, sentence_idx, &["mill", "you", "if"])
}

fn first_word_search(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "search")
}

fn first_word_destroy(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "destroy")
}

fn first_word_sacrifice(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "sacrifice")
}

fn first_word_exile(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "exile")
}

fn first_word_exile_or_shuffle(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["exile", "shuffle"])
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

const SUBJECT_VERB_SEQUENCE_RULES: &[SequenceRuleDef] = &[
    SequenceRuleDef {
        name: "participant-secret-object-choice-reveal-sacrifice",
        feature_tag: Some("secret-participant-object-choice"),
        priority: 447,
        consumed_sentences: 2,
        predicate: first_word_you,
        parser: generic_subject_verb_sequences::pairs::parse_participant_secret_object_choice_then_reveal_and_sacrifice,
    },
    SequenceRuleDef {
        name: "exile-each-player-put-return-exiled-exile-source",
        feature_tag: Some("exiled-collection-return-after-player-actions"),
        priority: 446,
        consumed_sentences: 4,
        predicate: first_word_exile,
        parser: generic_subject_verb_sequences::exiled_collections::parse_exile_each_player_put_return_exiled_then_exile_source,
    },
    SequenceRuleDef {
        name: "exile-top-play-event-followup",
        feature_tag: Some("exile-play-event-followup"),
        priority: 445,
        consumed_sentences: 3,
        predicate: first_word_exile,
        parser: generic_subject_verb_sequences::exile_permission_followups::parse_exile_top_play_then_event_followup,
    },
    SequenceRuleDef {
        name: "random-graveyard-exile-choose-copy-cast-copy",
        feature_tag: Some("exiled-collection-copy-cast"),
        priority: 444,
        consumed_sentences: 3,
        predicate: first_word_exile,
        parser: generic_subject_verb_sequences::exiled_collections::parse_random_graveyard_exile_choose_copy_then_cast_copy,
    },
    SequenceRuleDef {
        name: "exile-top-put-from-among-onto-battlefield",
        feature_tag: Some("exiled-collection-battlefield"),
        priority: 443,
        consumed_sentences: 2,
        predicate: first_word_exile,
        parser: generic_subject_verb_sequences::exiled_collections::parse_exile_top_then_put_from_among_onto_battlefield,
    },
    SequenceRuleDef {
        name: "exile-top-cast-any-number-free",
        feature_tag: Some("exiled-collection-cast-any"),
        priority: 443,
        consumed_sentences: 2,
        predicate: first_word_exile_or_shuffle,
        parser: generic_subject_verb_sequences::exiled_collections::parse_exile_top_then_cast_any_number_free,
    },
    SequenceRuleDef {
        name: "tempting-offer-copy-spell",
        feature_tag: Some("tempting-offer-copy-spell"),
        priority: 440,
        consumed_sentences: 4,
        predicate: first_word_choose_or_tempting,
        parser: generic_subject_verb_sequences::pairs::parse_tempting_offer_copy_spell_sequence,
    },
    SequenceRuleDef {
        name: "reciprocal-creature-control",
        feature_tag: Some("reciprocal-creature-control"),
        priority: 439,
        consumed_sentences: 3,
        predicate: first_word_you_or_untap,
        parser: generic_subject_verb_sequences::pairs::parse_reciprocal_creature_control_sequence,
    },
    SequenceRuleDef {
        name: "revealed-and-or-choice-destination-override",
        feature_tag: Some("looked-cards-and-or-destination-replacement"),
        priority: 439,
        consumed_sentences: 4,
        predicate: first_word_reveal,
        parser: generic_subject_verb_sequences::quads::parse_reveal_top_choose_and_or_hand_rest_bottom_with_destination_override,
    },
    SequenceRuleDef {
        name: "looked-matching-battlefield-then-shuffle",
        feature_tag: Some("looked-cards-battlefield-shuffle"),
        priority: 439,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_look_at_top_put_matching_onto_battlefield_then_shuffle,
    },
    SequenceRuleDef {
        name: "looked-battlefield-grant-rest-bottom",
        feature_tag: Some("looked-cards-battlefield-grant-remainder"),
        priority: 439,
        consumed_sentences: 4,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::quads::parse_top_cards_move_then_grant_rest_bottom,
    },
    SequenceRuleDef {
        name: "top-cards-move-rest-typed-when-result",
        feature_tag: Some("looked-cards-reflexive-move"),
        priority: 438,
        consumed_sentences: 4,
        predicate: first_word_look_or_reveal,
        parser:
            generic_subject_verb_sequences::quads::parse_top_cards_move_rest_then_typed_when_result,
    },
    SequenceRuleDef {
        name: "consult-cleanup-typed-when-result",
        feature_tag: Some("consult-reflexive-cleanup"),
        priority: 438,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser:
            generic_subject_verb_sequences::triples::parse_consult_cleanup_then_typed_when_result,
    },
    SequenceRuleDef {
        name: "consult-reveal-pump-triggering-creature-move-revealed",
        feature_tag: Some("consult-revealed-collection-followup"),
        priority: 438,
        consumed_sentences: 3,
        predicate: first_word_consult_reveal_candidate,
        parser: generic_subject_verb_sequences::triples::parse_consult_reveal_then_pump_triggering_creature_then_move_revealed,
    },
    SequenceRuleDef {
        name: "look-at-top-reveal-counted-hand-then-shuffle",
        feature_tag: Some("looked-cards-reveal-hand-shuffle"),
        priority: 437,
        consumed_sentences: 3,
        predicate: first_word_look,
        parser:
            generic_subject_verb_sequences::triples::parse_look_at_top_reveal_counted_to_hand_then_shuffle,
    },
    SequenceRuleDef {
        name: "sacrifice-reveal-top-choose-any-revealed-land-nonland-split-rest-bottom",
        feature_tag: Some("sacrifice-revealed-land-nonland-bottom"),
        priority: 433,
        consumed_sentences: 4,
        predicate: first_word_sacrifice,
        parser:
            generic_subject_verb_sequences::quads::parse_sacrifice_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-exile-counted-rest-bottom-play-while-exiled",
        feature_tag: Some("looked-cards-exile-play-while-exiled"),
        priority: 432,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_at_top_exile_counted_rest_bottom_play_while_exiled,
    },
    SequenceRuleDef {
        name: "search-reveal-named-match-battlefield-else-hand-then-shuffle",
        feature_tag: Some("search-named-card-branch"),
        priority: 431,
        consumed_sentences: 4,
        predicate: first_word_search,
        parser: generic_subject_verb_sequences::quads::parse_search_reveal_named_match_battlefield_else_hand_then_shuffle,
    },
    SequenceRuleDef {
        name: "look-may-sacrifice-if-did-select-battlefield-rest-bottom",
        feature_tag: Some("looked-cards-intervening-action-partition"),
        priority: 432,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-conditional-hand-counts-rest-bottom",
        feature_tag: Some("looked-cards-conditional-cardinality-partition"),
        priority: 431,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_at_top_conditional_hand_counts_then_rest_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-optional-battlefield-conditional-remainder",
        feature_tag: Some("looked-cards-conditional-remainder-partition"),
        priority: 431,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_at_top_optional_battlefield_then_conditional_remainder,
    },
    SequenceRuleDef {
        name: "look-at-top-put-counted-into-hand-rest-bottom-kicker-override",
        feature_tag: Some("looked-cards-kicker-override"),
        priority: 430,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override,
    },
    SequenceRuleDef {
        name: "look-at-top-exile-one-rest-bottom-cast-else-hand",
        feature_tag: Some("looked-card-exile-cast-else-hand"),
        priority: 430,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_at_top_exile_one_rest_bottom_cast_else_hand,
    },
    SequenceRuleDef {
        name: "look-at-top-may-exile-match-rest-bottom-cast-exiled",
        feature_tag: Some("looked-card-may-exile-cast-exiled"),
        priority: 430,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_at_top_may_exile_match_rest_bottom_cast_exiled,
    },
    SequenceRuleDef {
        name: "look-reveal-match-hand-selected-condition-rest-bottom",
        feature_tag: Some("looked-card-selected-condition-remainder"),
        priority: 430,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_reveal_match_to_hand_if_selected_matches_rest_bottom,
    },
    SequenceRuleDef {
        name: "reveal-top-optional-battlefield-then-hand-rest-graveyard",
        feature_tag: Some("looked-card-two-stage-graveyard-partition"),
        priority: 430,
        consumed_sentences: 4,
        predicate: first_word_reveal,
        parser: generic_subject_verb_sequences::quads::parse_reveal_top_optional_battlefield_then_hand_rest_graveyard,
    },
    SequenceRuleDef {
        name: "look-reveal-your-turn-battlefield-else-hand-rest-bottom",
        feature_tag: Some("looked-card-your-turn-destination-partition"),
        priority: 430,
        consumed_sentences: 5,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::quads::parse_look_may_reveal_then_your_turn_battlefield_else_hand_rest_bottom,
    },
    SequenceRuleDef {
        name: "destroy-for-each-destroyed-consult-exile-put-shuffle",
        feature_tag: Some("destroyed-consult-exile-put"),
        priority: 429,
        consumed_sentences: 3,
        predicate: first_word_destroy,
        parser:
            generic_subject_verb_sequences::parse_destroy_for_each_destroyed_consult_exile_put_shuffle,
    },
    SequenceRuleDef {
        name: "look-at-top-may-put-match-onto-battlefield-if-not-put-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-battlefield-or-hand"),
        priority: 429,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser:
            generic_subject_verb_sequences::quads::parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-may-reveal-match-bargain-battlefield-else-hand-then-shuffle",
        feature_tag: Some("looked-cards-bargain-branch"),
        priority: 428,
        consumed_sentences: 5,
        predicate: first_word_look,
        parser:
            generic_subject_verb_sequences::quads::parse_look_at_top_may_reveal_match_bargain_battlefield_else_hand_then_shuffle,
    },
    SequenceRuleDef {
        name: "look-at-top-optional-one-top-remainder-bottom",
        feature_tag: Some("looked-cards-optional-top-bottom-partition"),
        priority: 344,
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::triples::parse_look_at_top_then_optional_one_top_then_remainder_bottom,
    },
    SequenceRuleDef {
        name: "reveal-top-opponent-chooses-one-move-then-followup",
        feature_tag: Some("revealed-card-opponent-choice"),
        priority: 343,
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser:
            generic_subject_verb_sequences::triples::parse_reveal_top_opponent_chooses_one_then_move_and_followup,
    },
    SequenceRuleDef {
        name: "choose-two-targets-counter-first-if-power-then-fight",
        feature_tag: Some("target-set-counter-fight"),
        priority: 342,
        consumed_sentences: 3,
        predicate: first_word_choose,
        parser:
            generic_subject_verb_sequences::triples::parse_choose_two_targets_counter_first_if_power_then_fight,
    },
    SequenceRuleDef {
        name: "choose-land-or-nonland-consult-hand-bottom",
        feature_tag: Some("consult-choice-kind"),
        priority: 341,
        consumed_sentences: 3,
        predicate: first_word_choose,
        parser: generic_subject_verb_sequences::triples::parse_choose_land_or_nonland_then_consult_to_hand_bottom,
    },
    SequenceRuleDef {
        name: "choose-name-reveal-top-matching-hand-rest-graveyard",
        feature_tag: Some("looked-cards-chosen-name-rest-graveyard"),
        priority: 341,
        consumed_sentences: 3,
        predicate: first_word_choose,
        parser: generic_subject_verb_sequences::triples::parse_choose_name_reveal_top_matching_hand_rest_graveyard,
    },
    SequenceRuleDef {
        name: "search-two-put-one-hand-other-graveyard-then-shuffle",
        feature_tag: Some("search-two-hand-graveyard"),
        priority: 341,
        consumed_sentences: 3,
        predicate: first_word_search,
        parser: generic_subject_verb_sequences::triples::parse_search_two_then_put_one_hand_other_graveyard_then_shuffle,
    },
    SequenceRuleDef {
        name: "mill-then-payment-if-you-do-put-from-among-into-hand",
        feature_tag: Some("mill-payment-followup-choice"),
        priority: 341,
        consumed_sentences: 3,
        predicate: first_word_mill,
        parser: generic_subject_verb_sequences::triples::parse_mill_then_optional_payment_if_you_do_put_from_among_into_hand,
    },
    SequenceRuleDef {
        name: "mill-then-put-from-among-into-hand-then-if-you-dont",
        feature_tag: Some("mill-followup-choice"),
        priority: 340,
        consumed_sentences: 3,
        predicate: first_word_mill,
        parser: generic_subject_verb_sequences::triples::parse_mill_then_may_put_from_among_into_hand_then_if_you_dont,
    },
    SequenceRuleDef {
        name: "each-player-mill-exile-milled-creatures-create-power-token",
        feature_tag: Some("mill-exile-power-token"),
        priority: 340,
        consumed_sentences: 3,
        predicate: first_word_each,
        parser: generic_subject_verb_sequences::triples::parse_each_player_mill_then_exile_milled_creatures_then_create_power_token,
    },
    SequenceRuleDef {
        name: "reveal-top-opponent-exiles-one-rest-hand-then-may-cast",
        feature_tag: Some("reveal-opponent-exile-rest-hand-cast"),
        priority: 340,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_reveal_top_opponent_exiles_one_put_rest_hand_then_may_cast,
    },
    SequenceRuleDef {
        name: "look-at-top-exile-match-and-rest-bottom-cast-exiled",
        feature_tag: Some("looked-card-exile-cast-exiled"),
        priority: 340,
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::triples::parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled,
    },
    SequenceRuleDef {
        name: "search-player-names-card-conditional-put-then-shuffle",
        feature_tag: Some("search-name-choice-conditional-put"),
        priority: 340,
        consumed_sentences: 3,
        predicate: first_word_search,
        parser: generic_subject_verb_sequences::triples::parse_search_then_player_names_card_conditional_put_then_shuffle,
    },
    SequenceRuleDef {
        name: "search-face-down-exile-conditional-cast-else-hand",
        feature_tag: Some("search-face-down-cast"),
        priority: 339,
        consumed_sentences: 3,
        predicate: first_word_search,
        parser: generic_subject_verb_sequences::triples::parse_search_face_down_exile_conditional_cast_else_hand,
    },
    SequenceRuleDef {
        name: "top-cards-one-hand-then-matching-to-zone-rest-graveyard",
        feature_tag: Some("looked-cards-multi-subset-graveyard"),
        priority: 339,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_one_hand_then_matching_to_zone_rest_graveyard,
    },
    SequenceRuleDef {
        name: "top-cards-reveal-selection-rest-bottom-land-creature-split",
        feature_tag: Some("looked-cards-selected-type-split"),
        priority: 339,
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_reveal_selection_rest_bottom_then_land_creature_split,
    },
    SequenceRuleDef {
        name: "optional-look-reveal-put-top-rest-bottom",
        feature_tag: Some("looked-cards-optional-top-bottom"),
        priority: 339,
        consumed_sentences: 2,
        predicate: first_word_you,
        parser: generic_subject_verb_sequences::pairs::parse_optional_look_then_reveal_put_top_rest_bottom,
    },
    SequenceRuleDef {
        name: "effect-then-next-upkeep-unless-pays-lose-game",
        feature_tag: Some("delayed-upkeep-payment"),
        priority: 338,
        consumed_sentences: 3,
        predicate: next_upkeep_unless_pays_window,
        parser: generic_subject_verb_sequences::parse_search_delayed_upkeep_unless_pays_sequence,
    },
    SequenceRuleDef {
        name: "next-upkeep-unless-pays-lose-game",
        feature_tag: Some("delayed-upkeep-payment"),
        priority: 338,
        consumed_sentences: 2,
        predicate: first_word_at,
        parser: generic_subject_verb_sequences::parse_delayed_upkeep_unless_pays_sequence,
    },
    SequenceRuleDef {
        name: "exile-until-match-cast-rest-bottom",
        feature_tag: Some("consult-cast-bottom"),
        priority: 337,
        consumed_sentences: 3,
        predicate: first_word_if_target_exile_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_exile_until_match_cast_rest_bottom,
    },
    SequenceRuleDef {
        name: "exile-until-match-cast-else-hand",
        feature_tag: Some("consult-cast-or-hand"),
        priority: 336,
        consumed_sentences: 3,
        predicate: first_word_if_target_exile_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_exile_until_match_cast_else_hand,
    },
    SequenceRuleDef {
        name: "reveal-top-choose-any-revealed-land-nonland-split-rest-bottom",
        feature_tag: Some("looked-cards-land-nonland-split"),
        priority: 336,
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser:
            generic_subject_verb_sequences::triples::parse_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom,
    },
    SequenceRuleDef {
        name: "reveal-top-one-hand-gain-mana-value-rest-graveyard",
        feature_tag: Some("revealed-card-hand-value-rest"),
        priority: 336,
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser:
            generic_subject_verb_sequences::triples::parse_reveal_top_one_hand_gain_mana_value_rest_graveyard,
    },
    SequenceRuleDef {
        name: "top-cards-put-match-into-hand-rest-graveyard",
        feature_tag: Some("looked-cards-hand-graveyard"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_put_match_into_hand_rest_graveyard,
    },
    SequenceRuleDef {
        name: "look-at-top-may-put-same-name-as-permanent-rest-bottom",
        feature_tag: Some("looked-cards-same-name-permanent"),
        priority: 336,
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::triples::parse_look_at_top_may_put_same_name_as_permanent_rest_bottom,
    },
    SequenceRuleDef {
        name: "top-cards-may-cast-match-rest-bottom",
        feature_tag: Some("looked-cards-cast-bottom"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_may_cast_match_rest_bottom,
    },
    SequenceRuleDef {
        name: "top-cards-put-any-matching-to-zone-rest-bottom",
        feature_tag: Some("looked-cards-any-matching-bottom"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_then_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_put_any_matching_to_zone_rest_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-put-one-hand-bottom-cast-non-hand-put-all-hand",
        feature_tag: Some("looked-cards-cast-non-hand-override"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::triples::parse_look_at_top_put_one_hand_bottom_cast_non_hand_put_all_hand,
    },
    SequenceRuleDef {
        name: "top-cards-reveal-any-matching-to-hand-rest-bottom",
        feature_tag: Some("looked-cards-revealed-hand-bottom"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_target_exile_look_or_reveal,
        parser:
            generic_subject_verb_sequences::triples::parse_top_cards_reveal_any_matching_to_hand_rest_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-split-hand-bottom-exile-play",
        feature_tag: Some("looked-cards-split-play-exiled"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_look_at_top_split_hand_bottom_exile_then_play_exiled,
    },
    SequenceRuleDef {
        name: "top-cards-choose-for-each-filter-one-battlefield-others-hand-rest-graveyard",
        feature_tag: Some("looked-cards-filter-bundle"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard,
    },
    SequenceRuleDef {
        name: "top-cards-for-each-card-type-put-matching-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-card-type-choice"),
        priority: 334,
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom,
    },
    SequenceRuleDef {
        name: "top-cards-for-each-card-type-among-spells-put-matching-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-card-type-choice"),
        priority: 334,
        consumed_sentences: 3,
        predicate: first_word_reveal,
        parser:
            generic_subject_verb_sequences::triples::parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom,
    },
    SequenceRuleDef {
        name: "top-cards-put-match-onto-battlefield-and-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-battlefield-and-hand"),
        // This is a strict superset of the single-destination looked-card
        // rule. Run it first so the first "put" clause cannot consume the
        // sentence while silently dropping the coordinated hand choice.
        priority: 336,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_put_match_onto_battlefield_and_match_into_hand_rest_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-reveal-match-put-top-rest-bottom",
        feature_tag: Some("looked-cards-reveal-and-top"),
        priority: 333,
        consumed_sentences: 3,
        predicate: first_head_look_at,
        parser: generic_subject_verb_sequences::triples::parse_look_at_top_reveal_match_put_top_rest_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-reveal-match-put-rest-bottom",
        feature_tag: Some("looked-cards-reveal-and-hand"),
        priority: 332,
        consumed_sentences: 3,
        predicate: first_head_look_at,
        parser: generic_subject_verb_sequences::triples::parse_look_at_top_reveal_match_put_rest_bottom,
    },
    SequenceRuleDef {
        name: "prefix-then-consult-match-move-bottom-remainder",
        feature_tag: Some("consult-prefixed-bottom"),
        priority: 331,
        consumed_sentences: 3,
        predicate: prefixed_consult_window,
        parser: generic_subject_verb_sequences::triples::parse_prefix_then_consult_match_move_and_bottom_remainder,
    },

    SequenceRuleDef {
        name: "prefix-then-consult-match-into-hand-exile-others",
        feature_tag: Some("consult-prefixed-hand-exile"),
        priority: 330,
        consumed_sentences: 3,
        predicate: prefixed_consult_window,
        parser: generic_subject_verb_sequences::parse_prefixed_library_consult_hand_exile_sequence,
    },
    SequenceRuleDef {
        name: "iterative-library-procedure-sequence",
        feature_tag: Some("repeat-process"),
        priority: 329,
        consumed_sentences: 3,
        predicate: iterative_library_procedure_window,
        parser: generic_subject_verb_sequences::parse_iterative_library_procedure_sequence,
    },
    SequenceRuleDef {
        name: "each-player-repeat-pay-life-tokens",
        feature_tag: Some("repeat-process"),
        priority: 328,
        consumed_sentences: 3,
        predicate: first_word_starting,
        parser: generic_subject_verb_sequences::parse_each_player_repeat_pay_life_tokens_sequence,
    },
    SequenceRuleDef {
        name: "starting-each-player-optional-repeat",
        feature_tag: Some("repeat-process"),
        priority: 327,
        consumed_sentences: 2,
        predicate: first_word_starting,
        parser:
            generic_subject_verb_sequences::parse_starting_each_player_optional_repeat_sequence,
    },
    SequenceRuleDef {
        name: "target-gains-flashback-until-eot-targets-mana-cost",
        feature_tag: Some("flashback-cost-followup"),
        priority: 236,
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: generic_subject_verb_sequences::parse_parameterized_flashback_grant_sequence,
    },
    SequenceRuleDef {
        name: "exile-face-down-pile-then-cloak-tapped",
        feature_tag: Some("cloak-pile"),
        priority: 245,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_exile_face_down_pile_then_cloak,
    },
    SequenceRuleDef {
        name: "each-player-shuffle-reveal-put-revealed-types-rest-bottom",
        feature_tag: Some("mass-reveal-battlefield-bottom"),
        priority: 243,
        consumed_sentences: 2,
        predicate: first_word_each,
        parser: generic_subject_verb_sequences::parse_each_player_shuffle_reveal_then_put_revealed_types_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-counted-hand-rest-bottom",
        feature_tag: Some("looked-cards-counted-hand-bottom"),
        priority: 244,
        consumed_sentences: 2,
        predicate: first_word_look,
        parser: generic_subject_verb_sequences::pairs::parse_look_at_top_then_put_counted_hand_rest_bottom,
    },
    SequenceRuleDef {
        name: "top-cards-put-any-matching-to-zone-rest-bottom-same-sentence",
        feature_tag: Some("looked-cards-any-matching-bottom"),
        priority: 243,
        consumed_sentences: 2,
        predicate: first_word_look_or_reveal,
        parser:
            generic_subject_verb_sequences::pairs::parse_top_cards_put_any_matching_to_zone_rest_same_sentence,
    },
    SequenceRuleDef {
        name: "choose-phase-then-skip-chosen-this-turn",
        feature_tag: Some("choose-step-phase-skip"),
        priority: 244,
        consumed_sentences: 2,
        predicate: first_word_that_or_the,
        parser: generic_subject_verb_sequences::pairs::parse_choose_draw_main_or_combat_phase_then_skip_chosen_this_turn,
    },
    SequenceRuleDef {
        name: "copy-for-each-target-each-copy-different",
        feature_tag: Some("copy-target-assignment"),
        priority: 242,
        consumed_sentences: 2,
        predicate: copy_for_each_target_window,
        parser: generic_subject_verb_sequences::pairs::parse_copy_for_each_target_then_each_copy_targets_different,
    },
    SequenceRuleDef {
        name: "for-each-tagged-copy-then-copy-targets-it",
        feature_tag: Some("copy-target-assignment"),
        priority: 242,
        consumed_sentences: 2,
        predicate: for_each_tagged_copy_window,
        parser: generic_subject_verb_sequences::pairs::parse_for_each_tagged_copy_then_copy_targets_it,
    },
    SequenceRuleDef {
        name: "whenever-gain-life-then-self-animate-source",
        feature_tag: Some("self-animate-source"),
        priority: 241,
        consumed_sentences: 2,
        predicate: first_word_when_or_whenever,
        parser: generic_subject_verb_sequences::pairs::parse_whenever_gain_life_then_self_animate_source,
    },
    SequenceRuleDef {
        name: "filtered-future-exile-then-return-next-end-step",
        feature_tag: Some("filtered-future-zone-replacement"),
        priority: 243,
        consumed_sentences: 2,
        predicate: first_word_if,
        parser: generic_subject_verb_sequences::pairs::parse_filtered_future_exile_then_return_next_end_step,
    },
    SequenceRuleDef {
        name: "may-cast-target-graveyard-spell-then-exile-replacement",
        feature_tag: Some("cast-target-graveyard-spell-replacement"),
        priority: 242,
        consumed_sentences: 2,
        predicate: first_word_you_or_until,
        parser: generic_subject_verb_sequences::pairs::parse_may_cast_target_graveyard_spell_then_exile_replacement,
    },
    SequenceRuleDef {
        name: "gain-life-then-self-animate-source",
        feature_tag: Some("self-animate-source"),
        priority: 241,
        consumed_sentences: 2,
        predicate: first_word_you,
        parser: generic_subject_verb_sequences::pairs::parse_gain_life_then_self_animate_source,
    },
    SequenceRuleDef {
        name: "damage-prevention-then-damage-any-target",
        feature_tag: Some("damage-prevention-followup"),
        priority: 241,
        consumed_sentences: 2,
        predicate: first_word_prevent,
        parser: generic_subject_verb_sequences::parse_damage_prevention_reflect_to_any_target_sequence,
    },
    SequenceRuleDef {
        name: "damage-prevention-then-put-counters",
        feature_tag: Some("damage-prevention-followup"),
        priority: 240,
        consumed_sentences: 2,
        predicate: first_word_prevent,
        parser: generic_subject_verb_sequences::parse_damage_prevention_counter_sequence,
    },
    SequenceRuleDef {
        name: "next-damage-prevention-then-gain-prevented-life",
        feature_tag: Some("damage-prevention-followup"),
        priority: 240,
        consumed_sentences: 2,
        predicate: first_word_the,
        parser: generic_subject_verb_sequences::parse_next_damage_prevention_gain_life_sequence,
    },
    SequenceRuleDef {
        name: "fixed-damage-prevention-then-gain-prevented-life",
        feature_tag: Some("damage-prevention-followup"),
        priority: 240,
        consumed_sentences: 2,
        predicate: first_word_prevent,
        parser: generic_subject_verb_sequences::parse_next_damage_prevention_gain_life_sequence,
    },
    SequenceRuleDef {
        name: "next-damage-prevention-then-exile-prevented-top-cards",
        feature_tag: Some("damage-prevention-followup"),
        priority: 240,
        consumed_sentences: 2,
        predicate: first_word_the,
        parser: generic_subject_verb_sequences::parse_next_damage_prevention_exile_top_sequence,
    },
    SequenceRuleDef {
        name: "tap-all-then-they-dont-untap-while-source-tapped",
        feature_tag: Some("tap-lock-followup"),
        priority: 239,
        consumed_sentences: 2,
        predicate: first_word_tap,
        parser: generic_subject_verb_sequences::parse_tap_lock_sequence,
    },
    SequenceRuleDef {
        name: "choose-then-do-same-for-filter-then-return-to-battlefield",
        feature_tag: Some("choose-repeat-filter"),
        priority: 238,
        consumed_sentences: 2,
        predicate: first_word_choose,
        parser: generic_subject_verb_sequences::pairs::parse_choose_then_do_same_for_filter_then_return_to_battlefield,
    },
    SequenceRuleDef {
        name: "choose-same-controller-targets-then-sacrifice-one-return-other",
        feature_tag: Some("same-controller-target-choice"),
        priority: 239,
        consumed_sentences: 3,
        predicate: first_word_choose,
        parser: generic_subject_verb_sequences::pairs::parse_choose_same_controller_targets_then_sacrifice_one_return_other,
    },
    SequenceRuleDef {
        name: "choose-same-controller-targets-then-sacrifice-one",
        feature_tag: Some("same-controller-target-choice"),
        priority: 238,
        consumed_sentences: 2,
        predicate: first_word_choose,
        parser: generic_subject_verb_sequences::pairs::parse_choose_same_controller_targets_then_sacrifice_one,
    },
    SequenceRuleDef {
        name: "choose-then-affect-rest",
        feature_tag: Some("choice-remainder-action"),
        priority: 238,
        consumed_sentences: 2,
        predicate: first_word_choose_or_each,
        parser: generic_subject_verb_sequences::pairs::parse_choose_then_affect_rest,
    },
    SequenceRuleDef {
        name: "subject-reveals-top-choose-one-and-move",
        feature_tag: Some("revealed-card-candidate-choice"),
        priority: 237,
        consumed_sentences: 2,
        predicate: first_word_that_or_the,
        parser:
            generic_subject_verb_sequences::pairs::parse_reveal_top_then_choose_revealed_and_move,
    },
    SequenceRuleDef {
        name: "delayed-dies-exile-top-power-choose-play",
        feature_tag: Some("delayed-dies-consult"),
        priority: 237,
        consumed_sentences: 2,
        predicate: first_head_when_that,
        parser: generic_subject_verb_sequences::pairs::parse_delayed_dies_exile_top_power_choose_play,
    },
    SequenceRuleDef {
        name: "look-at-top-exile-face-down-play-while-exiled",
        feature_tag: Some("looked-card-play-while-exiled"),
        priority: 236,
        consumed_sentences: 2,
        predicate: first_head_look_at,
        parser: generic_subject_verb_sequences::pairs::parse_look_at_top_then_exile_face_down_then_play_while_exiled,
    },
    SequenceRuleDef {
        name: "look-at-top-exact-one-graveyard",
        feature_tag: Some("looked-cards-exact-singleton-move"),
        priority: 238,
        consumed_sentences: 2,
        predicate: first_head_look_at,
        parser: generic_subject_verb_sequences::pairs::parse_look_at_top_then_move_exact_one_to_graveyard,
    },
    SequenceRuleDef {
        name: "look-at-top-partition-selected-and-remainder",
        feature_tag: Some("looked-cards-selected-remainder-partition"),
        priority: 237,
        consumed_sentences: 2,
        predicate: first_head_look_at_or_if,
        parser: generic_subject_verb_sequences::pairs::parse_look_at_top_then_partition_selected_and_remainder,
    },
    SequenceRuleDef {
        name: "look-at-top-put-one-hand-other-bottom",
        feature_tag: Some("looked-cards-hand-bottom"),
        priority: 236,
        consumed_sentences: 2,
        predicate: first_head_look_at,
        parser: generic_subject_verb_sequences::pairs::parse_look_at_top_then_put_one_hand_other_bottom,
    },
    SequenceRuleDef {
        name: "look-at-top-put-one-hand-other-graveyard",
        feature_tag: Some("looked-cards-hand-graveyard"),
        priority: 236,
        consumed_sentences: 2,
        predicate: first_head_look_at,
        parser: generic_subject_verb_sequences::pairs::parse_look_at_top_then_put_one_hand_other_graveyard,
    },
    SequenceRuleDef {
        name: "mill-then-put-from-among-to-zone",
        feature_tag: Some("mill-followup-choice"),
        priority: 235,
        consumed_sentences: 2,
        predicate: first_word_mill_sequence_candidate,
        parser: generic_subject_verb_sequences::pairs::parse_mill_then_may_put_from_among_into_hand,
    },
    SequenceRuleDef {
        name: "exile-until-match-put-counters-on-match",
        feature_tag: Some("consult-match-counters"),
        priority: 235,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_exile_until_match_put_counters_on_match,
    },
    SequenceRuleDef {
        name: "exile-until-match-grant-play-this-turn",
        feature_tag: Some("consult-grant-play"),
        priority: 234,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_exile_until_match_grant_play_this_turn,
    },
    SequenceRuleDef {
        name: "target-chooses-other-cant-block",
        feature_tag: Some("target-choice-cant-block"),
        priority: 233,
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: generic_subject_verb_sequences::pairs::parse_target_player_chooses_then_other_cant_block,
    },
    SequenceRuleDef {
        name: "choose-card-type-then-reveal-and-put",
        feature_tag: Some("choose-card-type"),
        priority: 232,
        consumed_sentences: 2,
        predicate: first_word_choose,
        parser: generic_subject_verb_sequences::pairs::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
    },
    SequenceRuleDef {
        name: "choose-creature-type-then-become-type",
        feature_tag: Some("choose-creature-type"),
        priority: 231,
        consumed_sentences: 2,
        predicate: first_word_choose,
        parser: generic_subject_verb_sequences::pairs::parse_choose_creature_type_then_become_type,
    },
    SequenceRuleDef {
        name: "reveal-top-matching-into-hand-rest-graveyard",
        feature_tag: Some("reveal-top-rest-graveyard"),
        priority: 230,
        consumed_sentences: 2,
        predicate: first_word_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
    },
    SequenceRuleDef {
        name: "conditional-consult-match-move-bottom-remainder",
        feature_tag: Some("consult-conditional-bottom-remainder"),
        priority: 230,
        consumed_sentences: 2,
        predicate: first_word_then_if_target_exile_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_conditional_consult_match_move_and_bottom_remainder,
    },
    SequenceRuleDef {
        name: "consult-match-move-bottom-remainder",
        feature_tag: Some("consult-bottom-remainder"),
        priority: 229,
        consumed_sentences: 2,
        predicate: first_word_then_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_consult_match_move_and_bottom_remainder,
    },
    SequenceRuleDef {
        name: "directional-adjacent-player-control",
        feature_tag: Some("directional-player-choice-control"),
        priority: 260,
        consumed_sentences: 2,
        predicate: first_word_starting,
        parser: generic_subject_verb_sequences::pairs::parse_directional_adjacent_player_control,
    },
    SequenceRuleDef {
        name: "consult-match-onto-battlefield-or-into-hand",
        feature_tag: Some("consult-battlefield-or-hand"),
        priority: 229,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_consult_match_into_battlefield_or_hand,
    },
    SequenceRuleDef {
        name: "consult-match-move-graveyard-remainder",
        feature_tag: Some("consult-graveyard-remainder"),
        priority: 228,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_consult_match_move_all_to_graveyard,
    },
    SequenceRuleDef {
        name: "consult-match-into-hand-exile-others",
        feature_tag: Some("consult-hand-exile-others"),
        priority: 227,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_consult_match_into_hand_exile_others,
    },
    SequenceRuleDef {
        name: "consult-match-into-hand-others-graveyard",
        feature_tag: Some("consult-hand-graveyard-others"),
        priority: 227,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_consult_match_into_hand_others_graveyard,
    },
    SequenceRuleDef {
        name: "consult-match-into-battlefield-others-graveyard",
        feature_tag: Some("consult-battlefield-graveyard-others"),
        priority: 229,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_consult_match_into_battlefield_others_graveyard,
    },
];

pub(crate) fn try_parse_subject_verb_sequence_rule(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<SequenceRuleMatch>, CardTextError> {
    let mut best_match: Option<(u16, SequenceRuleMatch)> = None;
    for rule in SUBJECT_VERB_SEQUENCE_RULES {
        if best_match
            .as_ref()
            .is_some_and(|(best_priority, _)| *best_priority >= rule.priority)
        {
            continue;
        }
        if sentence_idx + rule.consumed_sentences > sentences.len() {
            continue;
        }
        if !(rule.predicate)(sentences, sentence_idx) {
            continue;
        }
        let Some(effects) = (rule.parser)(sentences, sentence_idx)? else {
            continue;
        };
        let candidate = SequenceRuleMatch {
            name: rule.name,
            feature_tag: rule.feature_tag,
            consumed_sentences: rule.consumed_sentences,
            effects,
        };
        let replace = best_match
            .as_ref()
            .is_none_or(|(best_priority, _)| rule.priority > *best_priority);
        if replace {
            best_match = Some((rule.priority, candidate));
        }
    }

    Ok(best_match.map(|(_, matched)| matched))
}

pub(crate) fn subject_verb_sequence_route(name: &str) -> &'static str {
    match name {
        "prefix-then-consult-match-into-hand-exile-others" => {
            "subject-verb verb=Search subject=explicit recognizer=consult-library-procedure"
        }
        "iterative-library-procedure-sequence" => {
            "subject-verb verb=Exile subject=explicit recognizer=iterative-library-procedure"
        }
        "target-gains-flashback-until-eot-targets-mana-cost" => {
            "subject-verb verb=Gain subject=explicit recognizer=parameterized-flashback-grant"
        }
        _ => "subject-verb verb=Do subject=implicit recognizer=sequence-procedure",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{
        IfResultPredicate, SubjectVerbActionAst, SubjectVerbEffectAst,
    };
    use crate::runtime_backend::{lex_line, split_lexed_sentences};

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
            generic_subject_verb_sequences::triples::parse_top_cards_put_any_matching_to_zone_rest_bottom(
                &sentences,
                0,
            )
            .expect("specialized parser")
            .is_some(),
            "specialized looked-partition parser must accept the three-sentence shape"
        );
        let matched = try_parse_subject_verb_sequence_rule(&sentences, 0)
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

        let matched = try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("sequence parse")
            .expect("the optional participant process should match");
        assert_eq!(matched.name, "starting-each-player-optional-repeat");
        assert_eq!(matched.consumed_sentences, 2);

        let [EffectAst::RepeatProcess {
            effects,
            continue_effect_index,
            continue_predicate: IfResultPredicate::Did,
        }] = matched.effects.as_slice()
        else {
            panic!(
                "expected one typed repeat process, got: {:#?}",
                matched.effects
            );
        };
        assert_eq!(*continue_effect_index, 0);
        let [EffectAst::SourceSentence {
            effects,
            starting_with_controller: true,
            ..
        }] = effects.as_slice()
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
