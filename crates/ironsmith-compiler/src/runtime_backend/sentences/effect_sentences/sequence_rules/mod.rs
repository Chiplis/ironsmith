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

fn first_word_you(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "you")
}

fn first_word_look(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "look")
}

fn first_word_mill(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "mill")
}

fn first_word_search(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "search")
}

fn first_word_look_or_reveal(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["look", "reveal"])
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

fn first_word_tap(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "tap")
}

fn first_word_choose(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "choose")
}

fn first_word_choose_or_each(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_in(sentences, sentence_idx, &["choose", "each"])
}

fn first_word_target(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "target")
}

fn first_word_reveal(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "reveal")
}

fn first_head_look_at(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_is(sentences, sentence_idx, ("look", Some("at")))
}

fn first_head_when_that(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_is(sentences, sentence_idx, ("when", Some("that")))
}

fn search_upkeep_window(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    sentence_head_word_is(sentences, sentence_idx, "search")
        && sentence_head_word_is(sentences, sentence_idx + 1, "at")
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
        name: "search-reveal-named-match-battlefield-else-hand-then-shuffle",
        feature_tag: Some("search-named-card-branch"),
        priority: 431,
        consumed_sentences: 4,
        predicate: first_word_search,
        parser: generic_subject_verb_sequences::quads::parse_search_reveal_named_match_battlefield_else_hand_then_shuffle,
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
        name: "look-at-top-may-put-match-onto-battlefield-if-not-put-into-hand-rest-bottom",
        feature_tag: Some("looked-cards-battlefield-or-hand"),
        priority: 429,
        consumed_sentences: 4,
        predicate: first_word_look,
        parser:
            generic_subject_verb_sequences::quads::parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom,
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
        name: "search-two-put-one-hand-other-graveyard-then-shuffle",
        feature_tag: Some("search-two-hand-graveyard"),
        priority: 341,
        consumed_sentences: 3,
        predicate: first_word_search,
        parser: generic_subject_verb_sequences::triples::parse_search_two_then_put_one_hand_other_graveyard_then_shuffle,
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
        name: "reveal-top-opponent-exiles-one-rest-hand-then-may-cast",
        feature_tag: Some("reveal-opponent-exile-rest-hand-cast"),
        priority: 340,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_reveal_top_opponent_exiles_one_put_rest_hand_then_may_cast,
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
        name: "search-then-next-upkeep-unless-pays-lose-game",
        feature_tag: Some("search-delayed-upkeep"),
        priority: 338,
        consumed_sentences: 3,
        predicate: search_upkeep_window,
        parser: generic_subject_verb_sequences::parse_search_delayed_upkeep_unless_pays_sequence,
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
        name: "top-cards-put-match-into-hand-rest-graveyard",
        feature_tag: Some("looked-cards-hand-graveyard"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_put_match_into_hand_rest_graveyard,
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
        predicate: first_word_look_or_reveal,
        parser: generic_subject_verb_sequences::triples::parse_top_cards_put_any_matching_to_zone_rest_bottom,
    },
    SequenceRuleDef {
        name: "top-cards-reveal-any-matching-to-hand-rest-bottom",
        feature_tag: Some("looked-cards-revealed-hand-bottom"),
        priority: 335,
        consumed_sentences: 3,
        predicate: first_word_look_or_reveal,
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
        priority: 333,
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
        name: "target-gains-flashback-until-eot-targets-mana-cost",
        feature_tag: Some("flashback-cost-followup"),
        priority: 236,
        consumed_sentences: 2,
        predicate: first_word_target,
        parser: generic_subject_verb_sequences::parse_parameterized_flashback_grant_sequence,
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
        name: "may-cast-target-graveyard-spell-then-exile-replacement",
        feature_tag: Some("cast-target-graveyard-spell-replacement"),
        priority: 242,
        consumed_sentences: 2,
        predicate: first_word_you,
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
        name: "damage-prevention-then-put-counters",
        feature_tag: Some("damage-prevention-followup"),
        priority: 240,
        consumed_sentences: 2,
        predicate: first_word_prevent,
        parser: generic_subject_verb_sequences::parse_damage_prevention_counter_sequence,
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
        name: "mill-then-put-from-among-into-hand",
        feature_tag: Some("mill-hand-choice"),
        priority: 235,
        consumed_sentences: 2,
        predicate: first_word_mill,
        parser: generic_subject_verb_sequences::pairs::parse_mill_then_may_put_from_among_into_hand,
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
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_consult_match_move_and_bottom_remainder,
    },
    SequenceRuleDef {
        name: "consult-match-move-graveyard-remainder",
        feature_tag: Some("consult-graveyard-remainder"),
        priority: 229,
        consumed_sentences: 2,
        predicate: first_word_target_exile_look_or_reveal,
        parser: generic_subject_verb_sequences::pairs::parse_consult_match_move_all_to_graveyard,
    },
    SequenceRuleDef {
        name: "consult-match-into-hand-exile-others",
        feature_tag: Some("consult-hand-exile-others"),
        priority: 228,
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
        priority: 227,
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
