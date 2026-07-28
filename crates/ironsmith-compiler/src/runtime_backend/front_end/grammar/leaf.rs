mod activation_heads;
mod articles;
mod casting;
mod common;
mod condition_prefixes;
mod counts;
mod durations;
mod filter_atoms;
mod mana;
mod numbers;
mod player_subjects;
mod power_toughness;
mod references;
mod source_references;
mod targets;

pub(crate) use activation_heads::{
    LeafActivationCostHead, parse_leaf_activation_cost_head_lexed,
    parse_leaf_activation_cost_head_tokens,
};
pub(crate) use articles::{
    parse_leaf_article_complete, parse_leaf_leading_articles_tokens,
    parse_leaf_leading_articles_words, parse_leaf_leading_indefinite_article_tokens,
    parse_leaf_leading_selected_tokens,
};
pub(crate) use casting::parse_leaf_alternative_cast_prefix_words;
#[cfg(test)]
pub(crate) use condition_prefixes::parse_condition_intro_complete;
pub(crate) use condition_prefixes::{
    ConditionIntro, parse_leaf_condition_intro_prefix_tokens,
    parse_leaf_static_condition_intro_prefix_tokens,
};
#[cfg(test)]
pub(crate) use counts::parse_leaf_count_range_prefix_lexed;
pub(crate) use counts::{
    parse_leaf_another_event_count_comparison_tokens, parse_leaf_choice_count_prefix_lexed,
    parse_leaf_choice_count_prefix_tokens, parse_leaf_choice_count_prefix_words,
    parse_leaf_modal_choose_range_tokens, parse_leaf_modal_value_token,
    parse_leaf_target_count_range_prefix_lexed,
};
#[cfg(test)]
pub(crate) use durations::parse_duration_phrase_complete;
pub(crate) use durations::{
    LeafConditionalDurationKind, LeafDurationPhrase, LeafTurnDurationPhrase,
    parse_leaf_conditional_duration_kind_tokens, parse_leaf_conditional_duration_prefix_tokens,
    parse_leaf_duration_phrase_lexed, parse_leaf_restriction_duration_prefix_tokens,
    parse_leaf_restriction_duration_suffix_tokens, parse_leaf_turn_duration_phrase_lexed,
    parse_leaf_turn_duration_prefix_tokens, parse_leaf_turn_duration_suffix_tokens,
    strip_leaf_this_turn_tokens,
};
pub(crate) use filter_atoms::{
    LeafDemonstrativeObjectHead, classify_token_definition_subtype,
    parse_leaf_card_type_complete, parse_leaf_color_complete,
    parse_leaf_demonstrative_object_head_complete, parse_leaf_non_card_type_complete,
    parse_leaf_non_color_complete, parse_leaf_non_subtype_complete,
    parse_leaf_non_supertype_complete, parse_leaf_object_reference_head_complete,
    parse_leaf_subtype_complete, parse_leaf_subtype_flexible_complete,
    parse_leaf_supertype_complete, parse_leaf_zone_complete,
};
#[cfg(test)]
pub(crate) use mana::parse_leaf_mana_symbol_group_tokens;
pub(crate) use mana::{
    LeafManaCostPrefix, LeafManaPipToken, parse_leaf_bare_mana_symbol_complete,
    parse_leaf_fixed_mana_cost_prefix_lexed, parse_leaf_fixed_mana_cost_prefix_tokens,
    parse_leaf_fixed_mana_output_lexed, parse_leaf_fixed_mana_output_tokens,
    parse_leaf_mana_cost_prefix_lexed, parse_leaf_mana_cost_prefix_tokens,
    parse_leaf_mana_cost_tokens, parse_leaf_mana_group_token, parse_leaf_mana_symbol_complete,
    parse_leaf_mana_symbol_group_complete, parse_leaf_pawprint_label_count_token,
    parse_leaf_spelled_mana_word_complete, parse_leaf_surface_mana_pip_lexed,
    parse_leaf_surface_mana_pip_token,
};
#[cfg(test)]
pub(crate) use numbers::parse_number_or_x_complete;
pub(crate) use numbers::{
    LeafNumber, LeafNumberPrefix, parse_leaf_count_token, parse_leaf_die_sides_complete,
    parse_leaf_number_or_x_prefix_lexed, parse_leaf_number_or_x_prefix_tokens,
    parse_leaf_number_prefix_lexed, parse_leaf_number_prefix_tokens,
    parse_leaf_number_prefix_words, parse_leaf_number_token_lexed, parse_number_complete,
    parse_number_i32_complete,
};
pub(crate) use player_subjects::{
    LeafPlayerReferenceMode, parse_leaf_player_reference_tokens, parse_leaf_player_reference_words,
};
pub(crate) use power_toughness::{
    parse_leaf_power_toughness_complete, parse_leaf_pt_modifier_values_complete,
    parse_leaf_unsigned_pt_complete,
};
pub(crate) use references::LeafPlayerReference;
#[cfg(test)]
pub(crate) use references::parse_player_reference_complete;
pub(crate) use source_references::{
    LeafSourceAnaphor, LeafSourceReferenceAlias, parse_leaf_source_anaphor_words,
    parse_leaf_source_reference_alias_words, parse_leaf_source_reference_aliases_for_name,
    parse_leaf_source_reference_possessive_alias_words, parse_leaf_this_source_reference_surface,
    parse_leaf_this_source_reference_words, push_leaf_source_reference_alias,
    push_leaf_source_reference_alias_words, strip_leaf_source_possessive_suffix,
};
pub(crate) use targets::parse_leaf_target_head_tokens;

#[cfg(test)]
use crate::effect::{Comparison, Value};
#[cfg(test)]
use crate::mana::ManaSymbol;

#[cfg(test)]
use super::primitives;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_parser_accepts_digits_words_and_articles() {
        assert_eq!(parse_number_complete("12").unwrap(), 12);
        assert_eq!(parse_number_complete("three").unwrap(), 3);
        assert_eq!(parse_number_complete("an").unwrap(), 1);
    }

    #[test]
    fn article_parser_rejects_prefix_words() {
        assert!(parse_number_complete("another").is_err());
    }

    #[test]
    fn another_event_count_parser_maps_to_second_or_later_threshold() {
        use super::super::super::lexer::lex_line;

        let tokens = lex_line("another", 0).unwrap();
        assert_eq!(
            parse_leaf_another_event_count_comparison_tokens(&tokens).unwrap(),
            Some(Comparison::GreaterThanOrEqual(2))
        );

        let tokens = lex_line("two", 0).unwrap();
        assert_eq!(
            parse_leaf_another_event_count_comparison_tokens(&tokens).unwrap(),
            None
        );
    }

    #[test]
    fn number_or_x_preserves_x() {
        assert_eq!(parse_number_or_x_complete("X").unwrap(), LeafNumber::X);
        assert_eq!(
            parse_number_or_x_complete("two").unwrap(),
            LeafNumber::Fixed(2)
        );
    }

    #[test]
    fn duration_parser_prefers_longer_next_turn_end() {
        assert_eq!(
            parse_duration_phrase_complete("until the end of your next turn").unwrap(),
            LeafDurationPhrase::UntilYourNextTurnEnd
        );
        assert_eq!(
            parse_duration_phrase_complete("until the end of combat").unwrap(),
            LeafDurationPhrase::UntilEndOfCombat
        );
        assert_eq!(
            parse_duration_phrase_complete("this combat").unwrap(),
            LeafDurationPhrase::UntilEndOfCombat
        );
    }

    #[test]
    fn duration_prefix_parsers_preserve_typed_duration_and_remainder() {
        use super::super::super::lexer::{TokenWordView, lex_line};

        let upkeep = lex_line("Until your next upkeep, activate only as a sorcery", 0).unwrap();
        let parsed = parse_leaf_restriction_duration_prefix_tokens(&upkeep).unwrap();
        assert_eq!(parsed.duration, LeafDurationPhrase::UntilYourNextUpkeep);
        assert_eq!(
            TokenWordView::new(parsed.rest).to_word_refs(),
            vec!["activate", "only", "as", "a", "sorcery"]
        );

        let rest_of_game =
            lex_line("For the rest of the game, that player can't gain life", 0).unwrap();
        let parsed = parse_leaf_restriction_duration_prefix_tokens(&rest_of_game).unwrap();
        assert_eq!(parsed.duration, LeafDurationPhrase::Forever);
        assert_eq!(
            TokenWordView::new(parsed.rest).to_word_refs(),
            vec!["that", "player", "cant", "gain", "life"]
        );
    }

    #[test]
    fn duration_suffix_parsers_cover_turn_and_controller_untap_phrases() {
        use super::super::super::lexer::{TokenWordView, lex_line};

        let turn = lex_line("You may play that card until your next turn.", 0).unwrap();
        let parsed = parse_leaf_turn_duration_suffix_tokens(&turn).unwrap();
        assert_eq!(parsed.duration, LeafTurnDurationPhrase::UntilYourNextTurn);
        assert_eq!(
            TokenWordView::new(parsed.rest).to_word_refs(),
            vec!["you", "may", "play", "that", "card"]
        );

        let untap = lex_line(
            "That creature doesn't untap during its controller's next untap step.",
            0,
        )
        .unwrap();
        let parsed = parse_leaf_restriction_duration_suffix_tokens(&untap).unwrap();
        assert_eq!(
            parsed.duration,
            LeafDurationPhrase::ControllersNextUntapStep
        );
        assert_eq!(
            TokenWordView::new(parsed.rest).to_word_refs(),
            vec!["that", "creature", "doesnt", "untap"]
        );
    }

    #[test]
    fn condition_intro_parser_maps_heads() {
        assert_eq!(
            parse_condition_intro_complete("for as long as").unwrap(),
            ConditionIntro::ForAsLongAs
        );
        assert_eq!(
            parse_condition_intro_complete("unless").unwrap(),
            ConditionIntro::Unless
        );
    }

    #[test]
    fn player_reference_parser_maps_target_opponent() {
        assert_eq!(
            parse_player_reference_complete("target opponent").unwrap(),
            LeafPlayerReference::TargetOpponent
        );
        assert_eq!(
            parse_player_reference_complete("its controller").unwrap(),
            LeafPlayerReference::ItsController
        );
    }

    #[test]
    fn mana_parser_accepts_symbols_groups_and_costs() {
        assert_eq!(
            parse_leaf_mana_symbol_complete("{p}").unwrap(),
            ManaSymbol::Life(2)
        );
        assert_eq!(
            parse_leaf_mana_symbol_group_complete("{w/u}").unwrap(),
            vec![ManaSymbol::White, ManaSymbol::Blue]
        );
    }

    #[test]
    fn count_range_parser_maps_common_prefixes() {
        use super::super::super::lexer::lex_line;

        let tokens = lex_line("up to X target creatures", 0).unwrap();
        let (range, rest) =
            primitives::parse_prefix(&tokens, parse_leaf_count_range_prefix_lexed).unwrap();
        assert_eq!(
            range.into_min_max(),
            (Some(Value::Fixed(0)), Some(Value::X))
        );
        assert_eq!(rest[0].parser_text(), "target");

        let tokens = lex_line("one or more opponents", 0).unwrap();
        let (range, rest) =
            primitives::parse_prefix(&tokens, parse_leaf_count_range_prefix_lexed).unwrap();
        assert_eq!(range.into_min_max(), (Some(Value::Fixed(1)), None));
        assert_eq!(rest[0].parser_text(), "opponents");
    }

    #[test]
    fn modal_choose_range_preserves_or_fallback() {
        use super::super::super::lexer::lex_line;

        let tokens = lex_line("gain life or draw a card", 0).unwrap();
        let range = parse_leaf_modal_choose_range_tokens(&tokens)
            .unwrap()
            .unwrap();
        assert_eq!(
            range.into_min_max(),
            (Some(Value::Fixed(1)), Some(Value::Fixed(1)))
        );
    }

    #[test]
    fn target_count_range_parser_preserves_two_and_three_part_ranges() {
        use super::super::super::lexer::lex_line;

        let tokens = lex_line("one or two targets", 0).unwrap();
        let (count, rest) =
            primitives::parse_prefix(&tokens, parse_leaf_target_count_range_prefix_lexed).unwrap();
        assert_eq!(count.min, 1);
        assert_eq!(count.max, Some(2));
        assert_eq!(rest[0].parser_text(), "targets");

        let tokens = lex_line("one, two, or three target creatures", 0).unwrap();
        let (count, rest) =
            primitives::parse_prefix(&tokens, parse_leaf_target_count_range_prefix_lexed).unwrap();
        assert_eq!(count.min, 1);
        assert_eq!(count.max, Some(3));
        assert_eq!(rest[0].parser_text(), "target");
    }
}
