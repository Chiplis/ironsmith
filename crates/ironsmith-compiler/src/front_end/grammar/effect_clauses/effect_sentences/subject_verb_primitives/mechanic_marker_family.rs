use super::*;

macro_rules! primitive {
    ($id:literal, $_former_order:expr, $stage:ident, $hints:expr, $parser:expr) => {
        SubjectVerbPrimitive::new($id, SubjectVerbPrimitiveStage::$stage, $hints, $parser)
    };
}

pub(crate) const PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES: &[SubjectVerbPrimitive] = &[
    primitive!(
        "implicit-become-clause",
        10,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("its"),
            LexRuleHeadHint::Single("it's"),
            LexRuleHeadHint::Single("it’s"),
            LexRuleHeadHint::Single("they"),
            LexRuleHeadHint::Single("they're"),
            LexRuleHeadHint::Single("they’re"),
            LexRuleHeadHint::Single("theyre"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("each"),
            LexRuleHeadHint::Pair("it", "is"),
            LexRuleHeadHint::Pair("they", "are"),
            LexRuleHeadHint::Pair("this", "creature"),
            LexRuleHeadHint::Pair("this", "permanent"),
            LexRuleHeadHint::Pair("this", "land"),
            LexRuleHeadHint::Pair("each", "of"),
        ],
        parse_sentence_implicit_become_clause
    ),
    primitive!(
        "fallback-mechanic-marker",
        20,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("stand"),
            LexRuleHeadHint::Single("it"),
        ],
        parse_sentence_fallback_mechanic_marker
    ),
    primitive!(
        "relative-opponent-damage-difference",
        24,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("that"),
            LexRuleHeadHint::Single("it"),
        ],
        parse_sentence_relative_opponent_damage_difference
    ),
    primitive!(
        "target-gains-or-loses-all-creature-types",
        25,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("target"),
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("that")
        ],
        parse_sentence_gains_or_loses_all_creature_types
    ),
    primitive!(
        "pump-creature-type-of-choice-pre",
        26,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("creatures"),
            LexRuleHeadHint::Single("target"),
        ],
        parse_sentence_pump_creature_type_of_choice
    ),
    primitive!(
        "lose-draw-clash-repeat-process",
        27,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("you")],
        parse_sentence_lose_draw_clash_repeat_process
    ),
    primitive!(
        "if-sacrifice-then-put-onto-battlefield-with-additional-counters",
        30,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("if")],
        parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence
    ),
    primitive!(
        "if-tagged-cards-remain-exiled",
        40,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("if")],
        parse_sentence_if_tagged_cards_remain_exiled
    ),
    primitive!(
        "if-enters-with-additional-counter",
        50,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("if")],
        parse_if_enters_with_additional_counter_sentence
    ),
    primitive!(
        "tagged-conditional-entry-counters",
        51,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("each"),
            LexRuleHeadHint::Single("all")
        ],
        parse_tagged_conditional_entry_counters_sentence
    ),
    primitive!(
        "tagged-enters-with-additional-counter",
        52,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("all"),
            LexRuleHeadHint::Single("each"),
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("that"),
        ],
        parse_tagged_enters_with_additional_counter_sentence
    ),
    primitive!(
        "if-any-tagged-cards-share-card-type-with-triggering-spell",
        55,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("if")],
        parse_if_any_tagged_cards_share_card_type_with_triggering_spell
    ),
    primitive!(
        "put-onto-battlefield-with-additional-counters",
        60,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("put")],
        parse_put_onto_battlefield_with_additional_counters_sentence
    ),
    primitive!(
        "put-fixed-and-counter-choice",
        65,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("put")],
        parse_sentence_put_fixed_and_counter_choice
    ),
    primitive!(
        "return-with-dynamic-entry-counters",
        67,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        parse_return_with_dynamic_entry_counters_sentence
    ),
    primitive!(
        "put-multiple-counters-on-target",
        70,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("put")],
        parse_sentence_put_multiple_counters_on_target
    ),
    primitive!(
        "put-sticker-on",
        80,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("put"),
            LexRuleHeadHint::Single("puts"),
        ],
        parse_sentence_put_sticker_on
    ),
    primitive!(
        "you-and-attacking-player-each-draw-and-lose",
        85,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("you")],
        parse_sentence_you_and_attacking_player_each_draw_and_lose
    ),
    primitive!(
        "you-and-target-player-each-draw",
        90,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_you_and_target_player_each_draw
    ),
    primitive!(
        "you-and-player-each-sacrifice",
        92,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_you_and_player_each_sacrifice
    ),
    primitive!(
        "you-and-player-each-gain-or-lose-life",
        95,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_you_and_player_each_gain_or_lose_life
    ),
    primitive!(
        "you-and-player-each-create",
        96,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_you_and_player_each_create
    ),
    primitive!(
        "choose-player-to-effect",
        100,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("choose"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_choose_player_to_effect
    ),
    primitive!(
        "sacrifice-then-put-onto-battlefield-with-additional-counters",
        120,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence
    ),
    primitive!(
        "sacrifice-it-next-end-step",
        130,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        parse_sentence_sacrifice_it_next_end_step
    ),
    primitive!(
        "exile-it-next-end-step",
        135,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        parse_sentence_exile_it_next_end_step
    ),
    primitive!(
        "sacrifice-at-end-of-combat",
        140,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        parse_sentence_sacrifice_at_end_of_combat
    ),
    primitive!(
        "target-player-choose-then-put-on-top-library",
        160,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        parse_sentence_target_player_chooses_then_puts_on_top_of_library
    ),
    primitive!(
        "target-player-choose-then-you-put-it-onto-battlefield",
        170,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield
    ),
    primitive!(
        "target-player-reveals-random-card-from-hand",
        180,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("target"),
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("opponent"),
            LexRuleHeadHint::Single("that"),
        ],
        parse_sentence_target_player_reveals_random_card_from_hand
    ),
    primitive!(
        "exile-hand-and-graveyard-bundle",
        190,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        parse_sentence_exile_hand_and_graveyard_bundle
    ),
];

pub(crate) static PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX: LazyLock<LexRuleHintIndex> =
    LazyLock::new(|| {
        build_lex_rule_hint_index(PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES.len(), |idx| {
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES[idx]
                .head_hints
                .to_vec()
        })
    });

pub(crate) const POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES: &[SubjectVerbPrimitive] = &[
    primitive!(
        "exile-target-creature-with-greatest-power",
        10,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        parse_sentence_exile_target_creature_with_greatest_power
    ),
    primitive!(
        "counter-target-spell-thats-second-cast-this-turn",
        20,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("counter")],
        parse_sentence_counter_target_spell_thats_second_cast_this_turn
    ),
    primitive!(
        "counter-target-spell-if-it-was-kicked",
        30,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("counter")],
        parse_sentence_counter_target_spell_if_it_was_kicked
    ),
    primitive!(
        "return-half-the-creatures-they-control-to-their-owners-hand",
        40,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("return"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_return_half_the_creatures_they_control_to_their_owners_hand
    ),
    primitive!(
        "destroy-creature-type-of-choice",
        50,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        parse_sentence_destroy_creature_type_of_choice
    ),
    primitive!(
        "pump-creature-type-of-choice",
        60,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("creatures"),
            LexRuleHeadHint::Single("target"),
        ],
        parse_sentence_pump_creature_type_of_choice
    ),
    primitive!(
        "must-attack-creature-type-of-choice",
        65,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("creatures")],
        parse_sentence_must_attack_creature_type_of_choice
    ),
    primitive!(
        "return-multiple-targets",
        70,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        parse_sentence_return_multiple_targets
    ),
    primitive!(
        "choose-all-battlefield-graveyard-to-hand",
        80,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("choose")],
        parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand
    ),
    primitive!(
        "for-each-of-target-objects",
        90,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        parse_sentence_for_each_of_target_objects
    ),
    primitive!(
        "return-creature-type-of-choice",
        100,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        parse_sentence_return_targets_of_creature_type_of_choice
    ),
    primitive!(
        "distribute-counters",
        110,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("distribute")],
        parse_sentence_distribute_counters
    ),
    primitive!(
        "keyword-then-chain",
        120,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        parse_sentence_keyword_then_chain
    ),
    primitive!(
        "chain-then-keyword",
        130,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        parse_sentence_chain_then_keyword
    ),
    primitive!(
        "exile-then-may-put-from-exile",
        140,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        parse_sentence_exile_then_may_put_from_exile
    ),
    primitive!(
        "exile-then-shuffle-graveyard-into-library",
        150,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        parse_exile_then_shuffle_graveyard_into_library_sentence
    ),
    primitive!(
        "exile-source-with-counters",
        160,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        parse_sentence_exile_source_with_counters
    ),
    primitive!(
        "destroy-all-attached-to-target",
        170,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        parse_sentence_destroy_all_attached_to_target
    ),
    primitive!(
        "comma-then-chain-special",
        180,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        parse_sentence_comma_then_chain_special
    ),
    primitive!(
        "destroy-then-land-controller-graveyard-count-damage",
        190,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        parse_sentence_destroy_then_land_controller_graveyard_count_damage
    ),
    primitive!(
        "draw-then-connive",
        200,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("draw")],
        parse_sentence_draw_then_connive
    ),
    primitive!(
        "choose-then-do-same-for-filter",
        210,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("choose")],
        parse_sentence_choose_then_do_same_for_filter
    ),
    primitive!(
        "choose-then-choose-objects",
        215,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("choose"),
            LexRuleHeadHint::Pair("you", "choose"),
        ],
        parse_sentence_choose_then_choose_objects
    ),
    primitive!(
        "return-then-do-same-for-subtypes",
        220,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        parse_sentence_return_then_do_same_for_subtypes
    ),
    primitive!(
        "return-then-create",
        230,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        parse_sentence_return_then_create
    ),
    primitive!(
        "put-counter-sequence",
        240,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("put")],
        parse_sentence_put_counter_sequence
    ),
    primitive!(
        "gets-then-fights",
        250,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("gets")],
        parse_sentence_gets_then_fights
    ),
    primitive!(
        "return-with-counters-on-it",
        260,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("return"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_return_with_counters_on_it
    ),
    primitive!(
        "each-player-return-with-additional-counter",
        270,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("each")],
        parse_sentence_each_player_return_with_additional_counter
    ),
    primitive!(
        "sacrifice-any-number",
        280,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        parse_sentence_sacrifice_any_number
    ),
    primitive!(
        "sacrifice-one-or-more",
        290,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        parse_sentence_sacrifice_one_or_more
    ),
    primitive!(
        "for-each-counter-kind-put-or-remove",
        320,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        parse_sentence_for_each_counter_kind_put_or_remove
    ),
    primitive!(
        "transform-with-followup",
        350,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("transform"),
            LexRuleHeadHint::Single("convert"),
        ],
        parse_sentence_transform_with_followup
    ),
    primitive!(
        "cant-effect",
        370,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("cant")],
        parse_sentence_cant_effect
    ),
    primitive!(
        "serial-target-pt-modifiers",
        375,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("until"),
            LexRuleHeadHint::Single("target"),
        ],
        parse_sentence_serial_target_pt_modifiers
    ),
    primitive!(
        "compound-damage-fanout",
        380,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("deal"),
            LexRuleHeadHint::Single("deals"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("target"),
        ],
        parse_sentence_compound_damage_fanout
    ),
    primitive!(
        "shared-color-target-fanout",
        390,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("until"),
            LexRuleHeadHint::Single("target"),
            LexRuleHeadHint::Pair("target", "radiance"),
        ],
        parse_sentence_shared_color_target_fanout
    ),
    primitive!(
        "gain-x-plus-life",
        440,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("gain")],
        parse_sentence_gain_x_plus_life
    ),
    primitive!(
        "for-each-exiled-this-way",
        450,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        parse_sentence_for_each_exiled_this_way
    ),
    primitive!(
        "for-each-put-into-graveyard-this-way",
        460,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        parse_sentence_for_each_put_into_graveyard_this_way
    ),
    primitive!(
        "draw-for-each-card-exiled-from-hand-this-way",
        470,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("draw"),
            LexRuleHeadHint::Single("draws"),
            LexRuleHeadHint::Single("that"),
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_draw_for_each_card_exiled_from_hand_this_way
    ),
    primitive!(
        "each-player-reveals-top-count-put-permanents-rest-graveyard",
        480,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("each")],
        parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard
    ),
    primitive!(
        "each-player-put-permanent-cards-exiled-with-source",
        490,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("each")],
        parse_sentence_each_player_put_permanent_cards_exiled_with_source
    ),
    primitive!(
        "for-each-destroyed-this-way",
        500,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        parse_sentence_for_each_destroyed_this_way
    ),
    primitive!(
        "delayed-next-step-unless-pays",
        510,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("at")],
        parse_sentence_delayed_next_step_unless_pays
    ),
    primitive!(
        "delayed-next-upkeep-unless-pays-lose-game",
        520,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("search"),
            LexRuleHeadHint::Single("the"),
        ],
        parse_sentence_delayed_next_upkeep_unless_pays_lose_game
    ),
    primitive!(
        "search-library",
        540,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("search")],
        parse_sentence_search_library
    ),
    primitive!(
        "shuffle-graveyard-into-library",
        550,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("shuffle")],
        parse_sentence_shuffle_graveyard_into_library
    ),
    primitive!(
        "shuffle-object-into-library",
        560,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("shuffle"),
            LexRuleHeadHint::Single("the"),
            LexRuleHeadHint::Single("target"),
            LexRuleHeadHint::Single("its"),
        ],
        parse_sentence_shuffle_object_into_library
    ),
    primitive!(
        "target-player-exiles-creature-and-graveyard",
        580,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        parse_sentence_target_player_exiles_creature_and_graveyard
    ),
    primitive!(
        "look-at-top-then-exile-one",
        600,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("look")],
        parse_sentence_look_at_top_then_exile_one
    ),
    primitive!(
        "look-at-hand",
        610,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("look")],
        parse_sentence_look_at_hand
    ),
    primitive!(
        "gain-life-equal-to-age",
        620,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("gain")],
        parse_sentence_gain_life_equal_to_age
    ),
    primitive!(
        "for-each-player-doesnt",
        630,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("for"),
            LexRuleHeadHint::Single("then"),
            LexRuleHeadHint::Single("each"),
        ],
        parse_sentence_for_each_player_doesnt
    ),
    primitive!(
        "each-opponent-loses-x-and-you-gain-x",
        650,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("each")],
        parse_sentence_each_opponent_loses_x_and_you_gain_x
    ),
    primitive!(
        "same-name-target-fanout",
        700,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        parse_sentence_same_name_target_fanout
    ),
    primitive!(
        "same-name-gets-fanout",
        710,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        parse_sentence_same_name_gets_fanout
    ),
    primitive!(
        "delayed-next-end-step",
        720,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("at")],
        parse_sentence_delayed_until_next_end_step
    ),
    primitive!(
        "delayed-when-that-dies-this-turn",
        730,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("when")],
        parse_delayed_when_that_dies_this_turn_sentence
    ),
    primitive!(
        "delayed-when-that-leaves-battlefield",
        735,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("when"),
            LexRuleHeadHint::Single("whenever"),
        ],
        parse_delayed_when_that_leaves_battlefield_sentence
    ),
    primitive!(
        "delayed-trigger-this-turn",
        740,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("if"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("when"),
            LexRuleHeadHint::Single("whenever"),
        ],
        parse_sentence_delayed_trigger_this_turn
    ),
    primitive!(
        "destroy-or-exile-all-split",
        750,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        parse_sentence_destroy_or_exile_all_split
    ),
    primitive!(
        "exile-up-to-one-each-target-type",
        760,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        parse_sentence_exile_up_to_one_each_target_type
    ),
    primitive!(
        "exile-multi-target",
        770,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        parse_sentence_exile_multi_target
    ),
    primitive!(
        "destroy-multi-target",
        780,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        parse_sentence_destroy_multi_target
    ),
    primitive!(
        "reveal-selected-cards-in-your-hand",
        790,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("reveal")],
        parse_sentence_reveal_selected_cards_in_your_hand
    ),
    primitive!(
        "damage-unless-controller-has-source-deal-damage",
        800,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("damage"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("destroy"),
        ],
        parse_sentence_damage_unless_controller_has_source_deal_damage
    ),
    primitive!(
        "damage-to-that-player-unless-enchanted-attacked",
        810,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("damage"),
            LexRuleHeadHint::Single("this"),
        ],
        parse_sentence_damage_to_that_player_unless_enchanted_attacked
    ),
    primitive!(
        "damage-to-that-player-half-damage-of-those-spells",
        820,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("damage"),
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        parse_sentence_damage_to_that_player_half_damage_of_those_spells
    ),
    primitive!(
        "unless-pays",
        830,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("unless"),
            LexRuleHeadHint::Single("for"),
            LexRuleHeadHint::Single("each")
        ],
        parse_sentence_unless_pays
    ),
];

pub(crate) static POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX: LazyLock<LexRuleHintIndex> =
    LazyLock::new(|| {
        build_lex_rule_hint_index(POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES.len(), |idx| {
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES[idx]
                .head_hints
                .to_vec()
        })
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ast::SubjectVerbSubjectAst;
    use crate::util::tokenize_line;

    #[test]
    fn parse_sentence_implicit_become_clause_handles_explicit_self_negative_type_with_duration() {
        let tokens = tokenize_line("this creature isn't a creature until end of turn.", 0);
        let effects =
            parse_sentence_implicit_become_clause(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("parse should succeed")
                .expect("clause should be recognized");

        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::RemoveCardTypes {
                            target: TargetAst::Source(_),
                            card_types,
                            duration: Until::EndOfTurn,
                        },
                    ..
                })] if card_types.as_slice() == [CardType::Creature]
            ),
            "expected explicit self negative-type clause to parse into source-scoped remove-card-types until end of turn, got {effects:?}"
        );
    }

    #[test]
    fn parse_sentence_implicit_become_clause_removes_one_explicit_subtype() {
        let tokens = tokenize_line("it isn't an Equipment.", 0);
        let effects =
            parse_sentence_implicit_become_clause(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("parse should succeed")
                .expect("clause should be recognized");

        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::RemoveSubtypes {
                            target: TargetAst::Tagged(_, _),
                            subtypes,
                            duration: Until::Forever,
                        },
                    ..
                })] if subtypes.as_slice() == [crate::types::Subtype::Equipment]
            ),
            "expected the negated subtype to remain a typed removal, got {effects:#?}"
        );
    }

    #[test]
    fn affirmative_subtype_clause_does_not_inherit_negated_subtype_removal() {
        let tokens = tokenize_line("it is an Equipment.", 0);
        let effects =
            parse_sentence_implicit_become_clause(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("parse should succeed")
                .expect("clause should be recognized");

        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::AddSubtypes { subtypes, .. },
                    ..
                })] if subtypes.as_slice() == [crate::types::Subtype::Equipment]
            ),
            "ordinary affirmative subtype changes must remain additions, got {effects:#?}"
        );
    }

    #[test]
    fn parse_sentence_implicit_become_clause_handles_plural_tagged_characteristics() {
        let tokens = tokenize_line("They're 2/2 Cyberman artifact creatures.", 0);
        let effects =
            parse_sentence_implicit_become_clause(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("parse should succeed")
                .expect("clause should be recognized");

        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::BecomeBasePtCreature {
                            target: TargetAst::Tagged(_, _),
                            power: Value::Fixed(2),
                            toughness: Value::Fixed(2),
                            card_types,
                            subtypes,
                            ..
                        },
                    ..
                })] if card_types.contains(&CardType::Artifact)
                    && card_types.contains(&CardType::Creature)
                    && subtypes.contains(&crate::types::Subtype::Cyberman)
            ),
            "expected plural tagged characteristics to parse structurally, got {effects:?}"
        );
    }

    #[test]
    fn preconditional_registry_claims_coordinated_conditional_entry_counters() {
        let tokens = crate::lexer::lex_line(
            "Each of them enters with an additional +1/+1 counter on it if it's a creature and an additional loyalty counter on it if it's a planeswalker.",
            0,
        )
        .expect("conditional entry-counter fixture should lex");
        let effects = run_subject_verb_primitives_lexed(
            &tokens,
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )
        .expect("registry parse should succeed")
        .expect("conditional entry-counter primitive should claim the sentence");

        assert_eq!(effects.len(), 2, "{effects:#?}");
        assert!(
            effects
                .iter()
                .all(|effect| matches!(effect, EffectAst::Conditional { .. })),
            "{effects:#?}"
        );
    }

    #[test]
    fn postconditional_registry_claims_owner_subject_shuffle_clauses_atomically() {
        for text in [
            "The owner of target nonland permanent shuffles it into their library, then draws two cards.",
            "Target creature's owner shuffles it into their library.",
            "Its owner shuffles it into their library.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).expect("shuffle fixture should lex");
            let effects = run_subject_verb_primitives_lexed(
                &tokens,
                POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
                &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
            )
            .unwrap_or_else(|error| panic!("registry parse should succeed for {text:?}: {error}"))
            .expect("owner-subject shuffle primitive should claim the sentence");

            assert!(
                matches!(
                    effects.first(),
                    Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::ShuffleObjectsIntoLibrary { .. },
                        ..
                    }))
                ),
                "expected one atomic shuffle-objects effect for {text:?}, got {effects:#?}"
            );
            assert!(
                effects.iter().all(|effect| !matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::ShuffleLibrary,
                        ..
                    })
                )),
                "owner-subject shuffle must not lower as a separate move plus library shuffle: {effects:#?}"
            );
        }
    }

    #[test]
    fn owner_subject_shuffle_binds_same_sentence_library_followup_to_the_owner() {
        let text = "The owner of target nonenchantment permanent shuffles it into their library, then exiles the top card of their library.";
        let tokens = crate::lexer::lex_line(text, 0).expect("owner-subject fixture should lex");
        let effects = run_subject_verb_primitives_lexed(
            &tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )
        .unwrap_or_else(|error| panic!("owner-subject parse should succeed: {error}"))
        .expect("owner-subject shuffle primitive should claim the sentence");

        assert!(
            matches!(
                effects.get(1),
                Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::ItsOwner,
                        ..
                    },
                    action: SubjectVerbActionAst::ExileTopOfLibrary { .. },
                }))
            ),
            "the same-sentence `their library` follow-up must remain owner-correlated: {effects:#?}"
        );

        let full_parse = crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
            .expect("the public sentence dispatcher should preserve the same owner binding");
        let full_parse_debug = format!("{full_parse:#?}");
        assert!(
            full_parse_debug.contains("player: ItsOwner")
                && !full_parse_debug.contains("player: ItsController"),
            "the public sentence route must preserve the owner-correlated follow-up: {full_parse:#?}"
        );

        let compiled = crate::compile_card_text(
            crate::CardDefinitionBuilder::new(crate::CardId::new(), "Owner Shuffle Probe")
                .card_types(vec![crate::CardType::Instant]),
            text,
            false,
        )
        .expect("the full card compiler should preserve the owner binding");
        let compiled_debug = format!("{:#?}", compiled.definition.spell_effect);
        assert!(
            compiled_debug.contains("player: OwnerOf(")
                && !compiled_debug.contains("player: ControllerOf("),
            "the compiled follow-up must read the shuffled object's owner library: {compiled_debug}"
        );
    }

    #[test]
    fn sentence_primitive_metadata_sets_stage_and_hints() {
        assert!(
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES
                .iter()
                .all(
                    |primitive| primitive.stage == SubjectVerbPrimitiveStage::PreDiagnostic
                        && !primitive.head_hints.is_empty()
                )
        );
        assert!(
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES
                .iter()
                .all(
                    |primitive| primitive.stage == SubjectVerbPrimitiveStage::PostDiagnostic
                        && !primitive.head_hints.is_empty()
                )
        );
    }
}
