pub(crate) const PRE_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
    SentencePrimitive {
        name: "implicit-become-clause",
        parser: parse_sentence_implicit_become_clause,
    },
    SentencePrimitive {
        name: "fallback-mechanic-marker",
        parser: parse_sentence_fallback_mechanic_marker,
    },
    SentencePrimitive {
        name: "if-tagged-cards-remain-exiled",
        parser: parse_sentence_if_tagged_cards_remain_exiled,
    },
    SentencePrimitive {
        name: "if-enters-with-additional-counter",
        parser: parse_if_enters_with_additional_counter_sentence,
    },
    SentencePrimitive {
        name: "put-multiple-counters-on-target",
        parser: parse_sentence_put_multiple_counters_on_target,
    },
    SentencePrimitive {
        name: "put-sticker-on",
        parser: parse_sentence_put_sticker_on,
    },
    SentencePrimitive {
        name: "you-and-target-player-each-draw",
        parser: parse_sentence_you_and_target_player_each_draw,
    },
    SentencePrimitive {
        name: "choose-player-to-effect",
        parser: parse_sentence_choose_player_to_effect,
    },
    SentencePrimitive {
        name: "you-and-attacking-player-each-draw-and-lose",
        parser: parse_sentence_you_and_attacking_player_each_draw_and_lose,
    },
    SentencePrimitive {
        name: "sacrifice-it-next-end-step",
        parser: parse_sentence_sacrifice_it_next_end_step,
    },
    SentencePrimitive {
        name: "sacrifice-at-end-of-combat",
        parser: parse_sentence_sacrifice_at_end_of_combat,
    },
    SentencePrimitive {
        name: "each-player-choose-keep-rest-sacrifice",
        parser: parse_sentence_each_player_choose_and_sacrifice_rest,
    },
    SentencePrimitive {
        name: "target-player-choose-then-put-on-top-library",
        parser: parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    },
    SentencePrimitive {
        name: "target-player-choose-then-you-put-it-onto-battlefield",
        parser: parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    },
    SentencePrimitive {
        name: "target-player-reveals-random-card-from-hand",
        parser: parse_sentence_target_player_reveals_random_card_from_hand,
    },
    SentencePrimitive {
        name: "exile-instead-of-graveyard",
        parser: parse_sentence_exile_instead_of_graveyard,
    },
];

pub(crate) static PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX: LazyLock<LexRuleHintIndex> =
    LazyLock::new(|| {
        build_lex_rule_hint_index(PRE_CONDITIONAL_SENTENCE_PRIMITIVES.len(), |idx| {
            sentence_primitive_head_hints(PRE_CONDITIONAL_SENTENCE_PRIMITIVES[idx].name)
        })
    });

pub(crate) const POST_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
    SentencePrimitive {
        name: "exile-target-creature-with-greatest-power",
        parser: parse_sentence_exile_target_creature_with_greatest_power,
    },
    SentencePrimitive {
        name: "counter-target-spell-thats-second-cast-this-turn",
        parser: parse_sentence_counter_target_spell_thats_second_cast_this_turn,
    },
    SentencePrimitive {
        name: "counter-target-spell-if-it-was-kicked",
        parser: parse_sentence_counter_target_spell_if_it_was_kicked,
    },
    SentencePrimitive {
        name: "return-half-the-creatures-they-control-to-their-owners-hand",
        parser: parse_sentence_return_half_the_creatures_they_control_to_their_owners_hand,
    },
    SentencePrimitive {
        name: "destroy-creature-type-of-choice",
        parser: parse_sentence_destroy_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "pump-creature-type-of-choice",
        parser: parse_sentence_pump_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "return-multiple-targets",
        parser: parse_sentence_return_multiple_targets,
    },
    SentencePrimitive {
        name: "choose-all-battlefield-graveyard-to-hand",
        parser: parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand,
    },
    SentencePrimitive {
        name: "for-each-of-target-objects",
        parser: parse_sentence_for_each_of_target_objects,
    },
    SentencePrimitive {
        name: "return-creature-type-of-choice",
        parser: parse_sentence_return_targets_of_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "distribute-counters",
        parser: parse_sentence_distribute_counters,
    },
    SentencePrimitive {
        name: "keyword-then-chain",
        parser: parse_sentence_keyword_then_chain,
    },
    SentencePrimitive {
        name: "chain-then-keyword",
        parser: parse_sentence_chain_then_keyword,
    },
    SentencePrimitive {
        name: "exile-then-may-put-from-exile",
        parser: parse_sentence_exile_then_may_put_from_exile,
    },
    SentencePrimitive {
        name: "exile-then-shuffle-graveyard-into-library",
        parser: parse_exile_then_shuffle_graveyard_into_library_sentence,
    },
    SentencePrimitive {
        name: "exile-source-with-counters",
        parser: parse_sentence_exile_source_with_counters,
    },
    SentencePrimitive {
        name: "destroy-all-attached-to-target",
        parser: parse_sentence_destroy_all_attached_to_target,
    },
    SentencePrimitive {
        name: "comma-then-chain-special",
        parser: parse_sentence_comma_then_chain_special,
    },
    SentencePrimitive {
        name: "destroy-then-land-controller-graveyard-count-damage",
        parser: parse_sentence_destroy_then_land_controller_graveyard_count_damage,
    },
    SentencePrimitive {
        name: "draw-then-connive",
        parser: parse_sentence_draw_then_connive,
    },
    SentencePrimitive {
        name: "choose-then-do-same-for-filter",
        parser: parse_sentence_choose_then_do_same_for_filter,
    },
    SentencePrimitive {
        name: "return-then-do-same-for-subtypes",
        parser: parse_sentence_return_then_do_same_for_subtypes,
    },
    SentencePrimitive {
        name: "return-then-create",
        parser: parse_sentence_return_then_create,
    },
    SentencePrimitive {
        name: "put-counter-sequence",
        parser: parse_sentence_put_counter_sequence,
    },
    SentencePrimitive {
        name: "gets-then-fights",
        parser: parse_sentence_gets_then_fights,
    },
    SentencePrimitive {
        name: "return-with-counters-on-it",
        parser: parse_sentence_return_with_counters_on_it,
    },
    SentencePrimitive {
        name: "each-player-return-with-additional-counter",
        parser: parse_sentence_each_player_return_with_additional_counter,
    },
    SentencePrimitive {
        name: "sacrifice-any-number",
        parser: parse_sentence_sacrifice_any_number,
    },
    SentencePrimitive {
        name: "sacrifice-one-or-more",
        parser: parse_sentence_sacrifice_one_or_more,
    },
    SentencePrimitive {
        name: "monstrosity",
        parser: parse_sentence_monstrosity,
    },
    SentencePrimitive {
        name: "for-each-counter-removed",
        parser: parse_sentence_for_each_counter_removed,
    },
    SentencePrimitive {
        name: "for-each-counter-kind-put-or-remove",
        parser: parse_sentence_for_each_counter_kind_put_or_remove,
    },
    SentencePrimitive {
        name: "take-extra-turn",
        parser: parse_sentence_take_extra_turn,
    },
    SentencePrimitive {
        name: "earthbend",
        parser: parse_sentence_earthbend,
    },
    SentencePrimitive {
        name: "transform-with-followup",
        parser: parse_sentence_transform_with_followup,
    },
    SentencePrimitive {
        name: "enchant",
        parser: parse_sentence_enchant,
    },
    SentencePrimitive {
        name: "cant-effect",
        parser: parse_sentence_cant_effect,
    },
    SentencePrimitive {
        name: "prevent-damage",
        parser: parse_sentence_prevent_damage,
    },
    SentencePrimitive {
        name: "shared-color-target-fanout",
        parser: parse_sentence_shared_color_target_fanout,
    },
    SentencePrimitive {
        name: "gain-ability-to-source",
        parser: parse_sentence_gain_ability_to_source,
    },
    SentencePrimitive {
        name: "gain-ability",
        parser: parse_sentence_gain_ability,
    },
    SentencePrimitive {
        name: "vote-with-you",
        parser: parse_sentence_you_and_each_opponent_voted_with_you,
    },
    SentencePrimitive {
        name: "gain-life-equal-to-power",
        parser: parse_sentence_gain_life_equal_to_power,
    },
    SentencePrimitive {
        name: "gain-x-plus-life",
        parser: parse_sentence_gain_x_plus_life,
    },
    SentencePrimitive {
        name: "for-each-exiled-this-way",
        parser: parse_sentence_for_each_exiled_this_way,
    },
    SentencePrimitive {
        name: "for-each-put-into-graveyard-this-way",
        parser: parse_sentence_for_each_put_into_graveyard_this_way,
    },
    SentencePrimitive {
        name: "draw-for-each-card-exiled-from-hand-this-way",
        parser: parse_sentence_draw_for_each_card_exiled_from_hand_this_way,
    },
    SentencePrimitive {
        name: "each-player-reveals-top-count-put-permanents-rest-graveyard",
        parser:
            parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard,
    },
    SentencePrimitive {
        name: "each-player-put-permanent-cards-exiled-with-source",
        parser: parse_sentence_each_player_put_permanent_cards_exiled_with_source,
    },
    SentencePrimitive {
        name: "for-each-destroyed-this-way",
        parser: parse_sentence_for_each_destroyed_this_way,
    },
    SentencePrimitive {
        name: "delayed-next-step-unless-pays",
        parser: parse_sentence_delayed_next_step_unless_pays,
    },
    SentencePrimitive {
        name: "search-delayed-next-upkeep-unless-pays-lose-game",
        parser: parse_sentence_delayed_next_upkeep_unless_pays_lose_game,
    },
    SentencePrimitive {
        name: "exile-then-return-same-object",
        parser: parse_sentence_exile_then_return_same_object,
    },
    SentencePrimitive {
        name: "search-library",
        parser: parse_sentence_search_library,
    },
    SentencePrimitive {
        name: "shuffle-graveyard-into-library",
        parser: parse_sentence_shuffle_graveyard_into_library,
    },
    SentencePrimitive {
        name: "shuffle-object-into-library",
        parser: parse_sentence_shuffle_object_into_library,
    },
    SentencePrimitive {
        name: "exile-hand-and-graveyard-bundle",
        parser: parse_sentence_exile_hand_and_graveyard_bundle,
    },
    SentencePrimitive {
        name: "target-player-exiles-creature-and-graveyard",
        parser: parse_sentence_target_player_exiles_creature_and_graveyard,
    },
    SentencePrimitive {
        name: "play-from-graveyard",
        parser: parse_sentence_play_from_graveyard,
    },
    SentencePrimitive {
        name: "look-at-top-then-exile-one",
        parser: parse_sentence_look_at_top_then_exile_one,
    },
    SentencePrimitive {
        name: "look-at-hand",
        parser: parse_sentence_look_at_hand,
    },
    SentencePrimitive {
        name: "gain-life-equal-to-age",
        parser: parse_sentence_gain_life_equal_to_age,
    },
    SentencePrimitive {
        name: "for-each-player-doesnt",
        parser: parse_sentence_for_each_player_doesnt,
    },
    SentencePrimitive {
        name: "for-each-opponent-doesnt",
        parser: parse_sentence_for_each_opponent_doesnt,
    },
    SentencePrimitive {
        name: "each-opponent-loses-x-and-you-gain-x",
        parser: parse_sentence_each_opponent_loses_x_and_you_gain_x,
    },
    SentencePrimitive {
        name: "vote-start",
        parser: parse_sentence_vote_start,
    },
    SentencePrimitive {
        name: "for-each-vote-clause",
        parser: parse_sentence_for_each_vote_clause,
    },
    SentencePrimitive {
        name: "vote-extra",
        parser: parse_sentence_vote_extra,
    },
    SentencePrimitive {
        name: "after-turn",
        parser: parse_sentence_after_turn,
    },
    SentencePrimitive {
        name: "same-name-target-fanout",
        parser: parse_sentence_same_name_target_fanout,
    },
    SentencePrimitive {
        name: "same-name-gets-fanout",
        parser: parse_sentence_same_name_gets_fanout,
    },
    SentencePrimitive {
        name: "delayed-next-end-step",
        parser: parse_sentence_delayed_until_next_end_step,
    },
    SentencePrimitive {
        name: "delayed-when-that-dies-this-turn",
        parser: parse_delayed_when_that_dies_this_turn_sentence,
    },
    SentencePrimitive {
        name: "delayed-trigger-this-turn",
        parser: parse_sentence_delayed_trigger_this_turn,
    },
    SentencePrimitive {
        name: "destroy-or-exile-all-split",
        parser: parse_sentence_destroy_or_exile_all_split,
    },
    SentencePrimitive {
        name: "exile-up-to-one-each-target-type",
        parser: parse_sentence_exile_up_to_one_each_target_type,
    },
    SentencePrimitive {
        name: "exile-multi-target",
        parser: parse_sentence_exile_multi_target,
    },
    SentencePrimitive {
        name: "destroy-multi-target",
        parser: parse_sentence_destroy_multi_target,
    },
    SentencePrimitive {
        name: "reveal-selected-cards-in-your-hand",
        parser: parse_sentence_reveal_selected_cards_in_your_hand,
    },
    SentencePrimitive {
        name: "damage-unless-controller-has-source-deal-damage",
        parser: parse_sentence_damage_unless_controller_has_source_deal_damage,
    },
    SentencePrimitive {
        name: "damage-to-that-player-unless-enchanted-attacked",
        parser: parse_sentence_damage_to_that_player_unless_enchanted_attacked,
    },
    SentencePrimitive {
        name: "damage-to-that-player-half-damage-of-those-spells",
        parser: parse_sentence_damage_to_that_player_half_damage_of_those_spells,
    },
    SentencePrimitive {
        name: "unless-pays",
        parser: parse_sentence_unless_pays,
    },
];

pub(crate) static POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX: LazyLock<LexRuleHintIndex> =
    LazyLock::new(|| {
        build_lex_rule_hint_index(POST_CONDITIONAL_SENTENCE_PRIMITIVES.len(), |idx| {
            sentence_primitive_head_hints(POST_CONDITIONAL_SENTENCE_PRIMITIVES[idx].name)
        })
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::compiler::util::tokenize_line;

    #[test]
    fn parse_sentence_implicit_become_clause_handles_explicit_self_negative_type_with_duration() {
        let tokens = tokenize_line("this creature isn't a creature until end of turn.", 0);
        let effects = parse_sentence_implicit_become_clause(&tokens)
            .expect("parse should succeed")
            .expect("clause should be recognized");

        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::RemoveCardTypes {
                    target: TargetAst::Source(_),
                    card_types,
                    duration: Until::EndOfTurn,
                }] if card_types.as_slice() == [CardType::Creature]
            ),
            "expected explicit self negative-type clause to parse into source-scoped remove-card-types until end of turn, got {effects:?}"
        );
    }
}
