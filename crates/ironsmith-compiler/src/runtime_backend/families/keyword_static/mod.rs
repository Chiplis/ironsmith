use super::activation_and_restrictions::parse_cycling_line;
use super::activation_and_restrictions::{
    normalize_cant_words, parse_ability_phrase, parse_activated_line, parse_activation_cost,
    parse_choose_land_type_phrase_words, parse_payment_clause_as_total_cost,
};
use super::effect_sentences::parse_granted_abilities_for_gain_clause;
use super::grammar::abilities::{
    CombatDamageUsingToughnessSubject, DoesntUntapDuringUntapStepSpec, FlyingBlockRestrictionKind,
    is_all_permanents_colorless_line_lexed,
    is_as_long_as_power_odd_or_even_flash_marker_line_lexed,
    is_attack_as_haste_unless_entered_this_turn_marker_line_lexed,
    is_can_be_your_commander_line_lexed, is_can_block_only_flying_line_lexed,
    is_cast_this_spell_as_though_it_had_flash_line_lexed, is_companion_marker_line_lexed,
    is_creatures_cant_block_line_lexed,
    is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed,
    is_creatures_without_flying_cant_attack_line_lexed,
    is_discard_or_redirect_replacement_line_lexed, is_doctors_companion_marker_line_lexed,
    is_double_damage_from_sources_you_control_of_chosen_type_line_lexed,
    is_draw_replace_exile_top_face_down_line_lexed, is_draw_replacement_double_line_lexed,
    is_draw_replacement_skip_empty_library_line_lexed,
    is_during_your_turn_prevent_all_damage_to_source_line_lexed,
    is_effect_discard_to_library_replacement_line_lexed,
    is_enchanted_land_is_chosen_type_line_lexed,
    is_if_source_you_control_with_mana_value_double_instead_marker_line_lexed,
    is_krrik_black_mana_life_payment_line_lexed,
    is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed,
    is_mana_group_slash_marker_line_lexed, is_may_assign_damage_as_unblocked_line_lexed,
    is_minimum_spell_total_mana_three_line_lexed, is_more_than_meets_the_eye_marker_line_lexed,
    is_no_maximum_hand_size_line_lexed, is_once_each_turn_play_from_exile_marker_guard_lexed,
    is_permanents_enter_tapped_line_lexed, is_play_lands_from_graveyard_line_lexed,
    is_play_top_card_your_library_revealed_line_lexed, is_players_cant_cycle_line_lexed,
    is_players_cant_pay_life_or_sacrifice_line_lexed,
    is_players_play_top_card_libraries_revealed_line_lexed, is_players_skip_upkeep_line_lexed,
    is_prevent_all_combat_damage_to_source_line_lexed,
    is_prevent_all_damage_dealt_to_creatures_line_lexed,
    is_prevent_all_damage_to_source_by_creatures_line_lexed,
    is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed,
    is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed,
    is_protection_mana_value_marker_line_lexed, is_remove_snow_line_lexed,
    is_sab_sunen_cant_attack_or_block_unless_line_lexed,
    is_shuffle_into_library_from_graveyard_line_lexed, is_skulk_rules_text_line_lexed,
    is_this_creature_cant_attack_alone_line_lexed,
    is_this_creature_cant_attack_its_owner_line_lexed, is_this_subject_reference_lexed,
    is_you_have_shroud_line_lexed,
    is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed,
    is_you_may_look_top_card_any_time_line_lexed,
    is_your_opponents_play_with_hands_revealed_line_lexed,
    parse_activated_abilities_cant_be_activated_spec_lexed,
    parse_can_block_subtype_as_though_reach_line_lexed,
    parse_creatures_assign_combat_damage_using_toughness_line_lexed,
    parse_doesnt_untap_during_untap_step_spec_lexed,
    parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed,
    parse_flying_block_restriction_line_lexed,
    parse_reveal_first_card_you_draw_each_turn_spec_lexed,
    parse_source_is_chosen_type_in_addition_line_lexed, parse_source_tap_status_condition_lexed,
    parse_trigger_suppression_spec_lexed, parse_ward_pay_life_amount_lexed,
    split_as_long_as_condition_prefix_lexed, split_if_this_spell_costs_line_lexed,
    split_untap_each_other_players_untap_step_line_lexed,
};
use super::grammar::filters::{
    parse_object_filter_with_grammar_entrypoint, parse_spell_filter_with_grammar_entrypoint,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
use super::grammar::primitives::{
    split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_commas_or_semicolons, split_lexed_slices_on_period,
};
pub(crate) use super::grammar::values::parse_add_mana_equal_amount_value_lexed as parse_add_mana_equal_amount_value;
use super::grammar::values::parse_max_cards_in_hand_value_lexed;
use super::keyword_static_helpers::*;
use super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, contains_token_kind, find_token_kind,
    find_token_word_sequence_span, parser_token_word_refs, render_token_slice,
    split_lexed_sentences, token_slice_at_is, token_slice_at_is_any, token_slice_first_is,
    token_slice_first_is_any, token_slice_starts_with, trim_lexed_commas, word_slice_at_is,
    word_slice_at_is_any, word_slice_contains_all_words, word_slice_contains_any_phrase,
    word_slice_contains_any_word, word_slice_contains_no_words, word_slice_contains_phrase,
    word_slice_contains_word, word_slice_ends_with, word_slice_ends_with_any, word_slice_eq,
    word_slice_eq_any, word_slice_eq_any_at, word_slice_eq_at, word_slice_find_any_phrase_span,
    word_slice_find_any_phrase_start, word_slice_find_phrase_start,
    word_slice_find_phrase_start_or_zero, word_slice_find_word, word_slice_find_word_where,
    word_slice_first_is, word_slice_first_is_any, word_slice_last_is, word_slice_last_is_any,
    word_slice_starts_with, word_slice_starts_with_any, word_slice_starts_with_at,
    word_slice_strip_any_prefix,
};
use super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::rule_engine::{LexRuleHeadHint, LexRuleHintIndex, build_lex_rule_hint_index};
use super::static_ability_helpers::lower_granted_abilities_ast_to_object_abilities;
use super::token_primitives::{
    find_index, find_window_by, lexed_head_words, rfind_index, slice_contains, slice_strip_prefix,
    slice_strip_suffix, split_em_dash_label_prefix, str_strip_prefix, str_strip_suffix,
};
use super::util::{
    is_source_reference_words, leading_mana_cost_from_tokens, mana_pips_from_token,
    parse_alternative_cast_words, parse_card_type, parse_color, parse_counter_type_from_tokens,
    parse_counter_type_word, parse_filter_counter_constraint_words, parse_flashback_keyword_line,
    parse_for_each_count_value_words, parse_number_word_i32, parse_subtype_flexible, parse_value,
    parse_value_expr_words, parse_zone_word, preserve_keyword_prefix_for_parse,
    source_reference_surface_for_possessive_words, strip_leading_article_word_refs,
    strip_leading_token_words_any, strip_leading_word_refs_any, trim_commas,
    word_refs_at_is_article, words,
};
use super::util::{source_choose_spec_for_surface, source_reference_surface_for_words};
use super::value_helpers::parse_commander_cast_count_player;
#[allow(unused_imports)]
use crate::ability::{Ability, AbilityKind, TriggeredAbility};
#[allow(unused_imports)]
use crate::alternative_cast::AlternativeCastingMethod;
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, GrantedAbilityAst, IT_TAG, KeywordAction, LineAst, ParsedAbility,
    ReferenceImports, StaticAbilityAst, TagKey, TextSpan,
};
#[allow(unused_imports)]
use crate::color::{Color, ColorSet};
#[allow(unused_imports)]
use crate::cost::TotalCost;
#[allow(unused_imports)]
use crate::effect::{Condition, Effect, EventValueSpec, Value};
#[allow(unused_imports)]
use crate::mana::{ManaCost, ManaSymbol};
#[allow(unused_imports)]
use crate::object::CounterType;
#[allow(unused_imports)]
use crate::static_abilities::{
    Anthem, AnthemCountExpression, AnthemValue, GrantAbility, StaticAbility,
};
#[allow(unused_imports)]
use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, ObjectFilter, PlayerFilter};
#[allow(unused_imports)]
use crate::triggers::Trigger;
#[allow(unused_imports)]
use crate::types::{CardType, Subtype, Supertype};
#[allow(unused_imports)]
use crate::zone::Zone;
use ironsmith_core::{EffectMetric, EffectMetricSource};
use std::sync::LazyLock;

const AS_ENTERS_AURA_SUBJECTS: &[(&str, &str)] = &[("aura", "this Aura")];

pub(crate) fn parse_can_be_attached_only_to_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some((phrase_idx, phrase_end)) =
        find_token_word_sequence_span(tokens, &["can", "be", "attached", "only", "to"])
    else {
        return Ok(None);
    };
    if phrase_idx == 0 {
        return Ok(None);
    }
    let target_tokens = trim_commas(&tokens[phrase_end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "attachment restriction missing target filter (clause: '{}')",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(StaticAbilityAst::AttachmentRestriction {
        filter: filter.into(),
        display: render_token_slice(tokens),
    }))
}

const AS_ENTERS_STANDARD_SUBJECTS: &[(&str, &str)] = &[
    ("land", "this land"),
    ("creature", "this creature"),
    ("artifact", "this artifact"),
    ("enchantment", "this enchantment"),
    ("permanent", "this permanent"),
];

const AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA: &[(&str, &str)] = &[
    ("land", "this land"),
    ("creature", "this creature"),
    ("artifact", "this artifact"),
    ("enchantment", "this enchantment"),
    ("aura", "this aura"),
    ("permanent", "this permanent"),
];

fn keyword_static_clause_text(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

fn keyword_static_marker(tokens: &[OwnedLexToken]) -> StaticAbility {
    let text = keyword_static_clause_text(tokens);
    if supported_keyword_marker_text(&text) {
        return StaticAbility::keyword_marker(text);
    }
    StaticAbility::keyword_fallback_text(text)
}

fn supported_keyword_marker_text(text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    let is_power_greater_marker =
        |prefix: &str| text.starts_with(prefix) && text.ends_with(" greater.");
    text.starts_with("prototype ")
        || text.starts_with("more than meets the eye ")
        || text.starts_with("splice onto ")
        || is_ticket_sticker_marker_line(&text)
        || text == "this creature crews vehicles using its toughness rather than its power."
        || text
            == "this creature saddles mounts and crews vehicles using its toughness rather than its power."
        || is_power_greater_marker("this creature crews vehicles as though its power were ")
        || is_power_greater_marker(
            "this creature saddles mounts and crews vehicles as though its power were ",
        )
        || is_power_greater_marker(
            "this token saddles mounts and crews vehicles as though its power were ",
        )
        || (text.starts_with(
            "you may remove a loyalty counter from a planeswalker you control rather than pay ",
        ) && text.ends_with("'s crew cost."))
}

fn is_ticket_sticker_marker_line(text: &str) -> bool {
    let Some((cost, body_text)) = text.split_once('—') else {
        return false;
    };

    let mut saw_ticket_symbol = false;
    let mut remainder = cost.trim();
    while let Some(next) = remainder.strip_prefix("{tk}") {
        saw_ticket_symbol = true;
        remainder = next.trim_start();
    }
    if !saw_ticket_symbol || !remainder.is_empty() {
        return false;
    }

    !body_text.trim().is_empty()
}

fn trim_outer_quotes(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && tokens[start].is_quote() {
        start += 1;
    }
    while end > start && tokens[end - 1].is_quote() {
        end -= 1;
    }
    &tokens[start..end]
}

fn looks_like_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        tokens.first().map(|token| token.parser_text()),
        Some("when" | "whenever" | "at")
    )
}

fn looks_like_trigger_intro_after_label(tokens: &[OwnedLexToken]) -> bool {
    split_em_dash_label_prefix(tokens)
        .is_some_and(|(_, body_tokens)| looks_like_trigger_intro_tokens(body_tokens))
}

#[derive(Clone, Copy)]
enum StaticAbilityLineRuleAst {
    Single(fn(&[OwnedLexToken]) -> Result<Option<StaticAbilityAst>, CardTextError>),
    SingleInfallible(fn(&[OwnedLexToken]) -> Option<StaticAbilityAst>),
    Multi(fn(&[OwnedLexToken]) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError>),
}

#[derive(Clone, Copy)]
struct StaticAbilityLineRuleDef {
    id: &'static str,
    rule: StaticAbilityLineRuleAst,
}

type StaticAbilityLineHeadHint = LexRuleHeadHint;

fn run_static_ability_ast_line_rule(
    rule: StaticAbilityLineRuleAst,
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    match rule {
        StaticAbilityLineRuleAst::Single(parse) => Ok(parse(tokens)?.map(|ability| vec![ability])),
        StaticAbilityLineRuleAst::SingleInfallible(parse) => {
            Ok(parse(tokens).map(|ability| vec![ability]))
        }
        StaticAbilityLineRuleAst::Multi(parse) => parse(tokens),
    }
}

fn try_static_ability_ast_line_rule_indices(
    rules: &'static [StaticAbilityLineRuleDef],
    tokens: &[OwnedLexToken],
    tried: &mut [bool],
    deferred_error: &mut Option<CardTextError>,
    candidate_indices: &[usize],
) -> Option<Vec<StaticAbilityAst>> {
    for &idx in candidate_indices {
        tried[idx] = true;
        match run_static_ability_ast_line_rule(rules[idx].rule, tokens) {
            Ok(Some(abilities)) => return Some(abilities),
            Ok(None) => {}
            Err(err) => {
                deferred_error.get_or_insert(err);
            }
        }
    }

    None
}

fn static_ability_rule_head_hints(rule_id: &'static str) -> Vec<StaticAbilityLineHeadHint> {
    match rule_id {
        "parse_characteristic_defining_pt_line" => Vec::new(),
        "parse_reduced_maximum_hand_size_line" => vec![
            StaticAbilityLineHeadHint::Single("your"),
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Single("each"),
            StaticAbilityLineHeadHint::Single("opponent"),
            StaticAbilityLineHeadHint::Single("opponents"),
            StaticAbilityLineHeadHint::Single("player"),
            StaticAbilityLineHeadHint::Single("players"),
            StaticAbilityLineHeadHint::Single("as"),
        ],
        "parse_can_be_attached_only_to_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "equipment"),
        ],
        "parse_conditional_source_spell_keyword_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Pair("if", "this"),
        ],
        "parse_conditional_all_creatures_able_to_block_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "long"),
        ],
        "parse_subject_has_keywords_and_cant_be_blocked_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "long"),
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "creature"),
        ],
        "parse_landwalk_as_though_block_override_line" => vec![
            StaticAbilityLineHeadHint::Single("creatures"),
            StaticAbilityLineHeadHint::Pair("creatures", "with"),
        ],
        "parse_multi_subject_anthem_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "creature"),
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
        ],
        "parse_spell_cost_increase_per_target_beyond_first_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "spell"),
        ],
        "parse_double_damage_from_sources_you_control_of_chosen_type_line" => vec![
            StaticAbilityLineHeadHint::Single("double"),
            StaticAbilityLineHeadHint::Pair("double", "damage"),
        ],
        "parse_source_can_attack_as_though_no_defender_as_long_as_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "can"),
        ],
        "parse_no_maximum_hand_size_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "have"),
        ],
        "parse_untap_during_each_other_players_untap_step_line" => vec![
            StaticAbilityLineHeadHint::Single("untap"),
            StaticAbilityLineHeadHint::Pair("untap", "all"),
        ],
        "parse_you_may_static_grant_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
            StaticAbilityLineHeadHint::Single("during"),
            StaticAbilityLineHeadHint::Pair("during", "each"),
        ],
        "parse_you_may_cast_exile_counter_cards_with_mana_permission_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_surveilled_graveyard_play_life_cost_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_fixed_mana_cost_instead_of_mana_cost_grant_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_enter_as_copy_as_enters_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_you_may_look_top_card_any_time_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_you_may_look_face_down_creatures_you_dont_control_any_time_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_players_play_top_card_libraries_revealed_line" => vec![
            StaticAbilityLineHeadHint::Single("players"),
            StaticAbilityLineHeadHint::Pair("players", "play"),
        ],
        "parse_play_top_card_your_library_revealed_line" => vec![
            StaticAbilityLineHeadHint::Single("play"),
            StaticAbilityLineHeadHint::Pair("play", "with"),
        ],
        "parse_your_opponents_play_with_hands_revealed_line" => vec![
            StaticAbilityLineHeadHint::Single("your"),
            StaticAbilityLineHeadHint::Pair("your", "opponents"),
        ],
        "parse_control_opponents_while_searching_libraries_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "control"),
        ],
        "parse_opponent_search_exile_found_cards_line" => vec![
            StaticAbilityLineHeadHint::Single("while"),
            StaticAbilityLineHeadHint::Pair("while", "an"),
        ],
        "parse_cast_this_card_from_library_while_searching_line" => vec![
            StaticAbilityLineHeadHint::Single("while"),
            StaticAbilityLineHeadHint::Pair("while", "youre"),
        ],
        "parse_additional_land_play_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_play_lands_from_graveyard_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_graveyard_cards_have_retrace_line" => vec![
            StaticAbilityLineHeadHint::Single("instant"),
            StaticAbilityLineHeadHint::Single("instants"),
            StaticAbilityLineHeadHint::Single("sorcery"),
            StaticAbilityLineHeadHint::Single("sorceries"),
            StaticAbilityLineHeadHint::Single("each"),
        ],
        "parse_pregame_choose_color_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Single("choose"),
            StaticAbilityLineHeadHint::Pair("choose", "a"),
        ],
        "parse_pregame_mulligan_redraw_line" => vec![
            StaticAbilityLineHeadHint::Single("any"),
            StaticAbilityLineHeadHint::Pair("any", "time"),
        ],
        "parse_legend_rule_doesnt_apply_line" => vec![
            StaticAbilityLineHeadHint::Single("the"),
            StaticAbilityLineHeadHint::Pair("the", "legend"),
        ],
        _ => match str_strip_prefix(rule_id, "parse_").and_then(|id| id.split('_').next()) {
            Some("ward") => vec![StaticAbilityLineHeadHint::Single("ward")],
            Some("skulk") => vec![StaticAbilityLineHeadHint::Single("skulk")],
            Some("if") => vec![StaticAbilityLineHeadHint::Single("if")],
            Some("choose") => vec![StaticAbilityLineHeadHint::Single("choose")],
            Some("enchanted") => vec![StaticAbilityLineHeadHint::Single("enchanted")],
            Some("enters") => vec![StaticAbilityLineHeadHint::Single("enters")],
            Some("damage") => vec![StaticAbilityLineHeadHint::Single("damage")],
            Some("pay") => vec![StaticAbilityLineHeadHint::Single("pay")],
            Some("copy") => vec![StaticAbilityLineHeadHint::Single("copy")],
            Some("players") => vec![StaticAbilityLineHeadHint::Single("players")],
            Some("shuffle") => vec![StaticAbilityLineHeadHint::Single("shuffle")],
            Some("permanents") => vec![StaticAbilityLineHeadHint::Single("permanents")],
            Some("creatures") => vec![StaticAbilityLineHeadHint::Single("creatures")],
            Some("buyback") => vec![StaticAbilityLineHeadHint::Single("buyback")],
            Some("flashback") => vec![StaticAbilityLineHeadHint::Single("flashback")],
            Some("spells") => vec![StaticAbilityLineHeadHint::Single("spells")],
            Some("foretelling") => vec![StaticAbilityLineHeadHint::Single("foretelling")],
            Some("all") => vec![StaticAbilityLineHeadHint::Single("all")],
            Some("blood") => vec![StaticAbilityLineHeadHint::Single("blood")],
            Some("land") => vec![StaticAbilityLineHeadHint::Single("land")],
            Some("lands") => vec![StaticAbilityLineHeadHint::Single("lands")],
            Some("remove") => vec![StaticAbilityLineHeadHint::Single("remove")],
            Some("attached") => vec![StaticAbilityLineHeadHint::Single("attached")],
            Some("soulbond") => vec![StaticAbilityLineHeadHint::Single("soulbond")],
            Some("may") => vec![StaticAbilityLineHeadHint::Single("may")],
            Some("warp") => vec![StaticAbilityLineHeadHint::Single("warp")],
            Some("melee") => vec![StaticAbilityLineHeadHint::Single("melee")],
            Some("equipped") => vec![StaticAbilityLineHeadHint::Single("equipped")],
            Some("as") => vec![StaticAbilityLineHeadHint::Single("as")],
            Some("prevent") => vec![StaticAbilityLineHeadHint::Single("prevent")],
            Some("reveal") => vec![StaticAbilityLineHeadHint::Single("reveal")],
            Some("activated") => vec![StaticAbilityLineHeadHint::Single("activated")],
            _ => Vec::new(),
        },
    }
}

macro_rules! single_static_ability_ast_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::Single(|tokens| {
                Ok($parse(tokens)?.map(StaticAbilityAst::from))
            }),
        }
    };
}

macro_rules! single_static_ability_ast_infallible_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::SingleInfallible(|tokens| {
                $parse(tokens).map(StaticAbilityAst::from)
            }),
        }
    };
}

macro_rules! multi_static_ability_ast_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::Multi(|tokens| {
                Ok($parse(tokens)?.map(|abilities| {
                    abilities
                        .into_iter()
                        .map(StaticAbilityAst::from)
                        .collect::<Vec<_>>()
                }))
            }),
        }
    };
}

macro_rules! single_static_ability_ast_passthrough_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::Single($parse),
        }
    };
}

macro_rules! multi_static_ability_ast_passthrough_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: stringify!($parse),
            rule: StaticAbilityLineRuleAst::Multi($parse),
        }
    };
}

fn static_ability_ast_line_rules() -> &'static [StaticAbilityLineRuleDef] {
    &[
        StaticAbilityLineRuleDef {
            id: stringify!(parse_soulbond_shared_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_soulbond_shared_line),
        },
        single_static_ability_ast_rule!(parse_ward_static_ability_line),
        single_static_ability_ast_rule!(parse_skulk_rules_text_line),
        single_static_ability_ast_rule!(
            parse_filter_dont_untap_during_controllers_untap_steps_line
        ),
        single_static_ability_ast_rule!(parse_damage_doubling_mana_value_marker_line),
        single_static_ability_ast_rule!(parse_conditional_source_spell_keyword_line),
        single_static_ability_ast_rule!(parse_pregame_begin_on_battlefield_line),
        single_static_ability_ast_rule!(parse_pregame_mulligan_redraw_line),
        multi_static_ability_ast_rule!(parse_combined_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_activated_abilities_cost_increase_line),
        single_static_ability_ast_rule!(parse_choose_basic_land_type_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_card_name_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_creature_type_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_named_options_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_player_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_color_as_becomes_attached_line),
        single_static_ability_ast_rule!(parse_enchanted_land_is_chosen_type_line),
        single_static_ability_ast_rule!(parse_source_is_chosen_type_in_addition_line),
        single_static_ability_ast_rule!(parse_source_is_chosen_color_line),
        single_static_ability_ast_rule!(parse_double_token_creation_replacement_line),
        single_static_ability_ast_rule!(parse_double_counters_replacement_line),
        single_static_ability_ast_infallible_rule!(parse_static_text_marker_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_choose_color_line),
        single_static_ability_ast_rule!(parse_damage_not_removed_cleanup_line),
        single_static_ability_ast_rule!(parse_prevent_damage_to_source_remove_counter_line),
        single_static_ability_ast_passthrough_rule!(
            parse_prevent_damage_to_source_put_counters_line
        ),
        single_static_ability_ast_rule!(parse_prevent_damage_to_you_from_source_filter_line),
        single_static_ability_ast_rule!(parse_replace_damage_with_counters_instead_line),
        single_static_ability_ast_rule!(parse_choose_color_as_enters_line),
        single_static_ability_ast_rule!(parse_damage_redirect_to_source_controller_line),
        single_static_ability_ast_rule!(parse_damage_redirect_to_source_line),
        single_static_ability_ast_rule!(
            parse_no_more_than_creatures_can_attack_or_block_each_combat_line
        ),
        single_static_ability_ast_rule!(parse_characteristic_defining_pt_line),
        single_static_ability_ast_rule!(parse_no_maximum_hand_size_line),
        single_static_ability_ast_rule!(parse_can_be_your_commander_line),
        single_static_ability_ast_rule!(parse_reduced_maximum_hand_size_line),
        single_static_ability_ast_rule!(parse_effect_discard_to_library_replacement_line),
        single_static_ability_ast_rule!(parse_draw_replace_exile_top_face_down_line),
        single_static_ability_ast_rule!(parse_draw_replacement_exile_top_and_play_line),
        single_static_ability_ast_rule!(parse_conditional_draw_replacement_line),
        single_static_ability_ast_rule!(parse_draw_replacement_double_line),
        single_static_ability_ast_rule!(parse_draw_replacement_skip_empty_library_line),
        single_static_ability_ast_rule!(parse_keyword_action_replacement_line),
        single_static_ability_ast_rule!(parse_exile_to_exile_instead_of_graveyard_line),
        single_static_ability_ast_rule!(parse_exile_to_countered_exile_instead_of_graveyard_line),
        single_static_ability_ast_rule!(parse_exile_would_die_instead_line),
        single_static_ability_ast_rule!(parse_discard_or_redirect_replacement_line),
        single_static_ability_ast_rule!(parse_pay_life_or_enter_tapped_line),
        single_static_ability_ast_passthrough_rule!(parse_copy_activated_abilities_line),
        single_static_ability_ast_passthrough_rule!(parse_spend_mana_as_any_color_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_enchanted_has_activated_ability_line),
            rule: StaticAbilityLineRuleAst::Single(parse_enchanted_has_activated_ability_line),
        },
        multi_static_ability_ast_passthrough_rule!(
            parse_has_base_power_toughness_and_granted_keywords_static_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_subject_is_subtype_with_base_pt_and_granted_abilities_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_filter_is_pt_creature_in_addition_and_has_line
        ),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_filter_has_granted_ability_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_filter_has_granted_ability_line),
        },
        StaticAbilityLineRuleDef {
            id: stringify!(parse_equipped_gets_and_has_activated_ability_line),
            rule: StaticAbilityLineRuleAst::Multi(
                parse_equipped_gets_and_has_activated_ability_line,
            ),
        },
        single_static_ability_ast_rule!(parse_shuffle_into_library_from_graveyard_line),
        single_static_ability_ast_rule!(parse_permanents_enter_tapped_line),
        single_static_ability_ast_rule!(
            parse_creatures_entering_dont_cause_abilities_to_trigger_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_trigger_suppression_line_ast),
        single_static_ability_ast_rule!(parse_creatures_assign_combat_damage_using_toughness_line),
        single_static_ability_ast_rule!(parse_players_cant_cycle_line),
        single_static_ability_ast_rule!(parse_starting_life_bonus_line),
        single_static_ability_ast_rule!(parse_buyback_cost_reduction_line),
        single_static_ability_ast_passthrough_rule!(parse_can_be_attached_only_to_line),
        single_static_ability_ast_rule!(parse_spell_cost_increase_per_target_beyond_first_line),
        single_static_ability_ast_rule!(parse_equip_cost_modifier_line),
        single_static_ability_ast_rule!(parse_flashback_cost_modifier_line),
        multi_static_ability_ast_rule!(parse_spell_and_player_activated_ability_cost_modifier_line),
        single_static_ability_ast_rule!(parse_spells_cost_modifier_line),
        single_static_ability_ast_passthrough_rule!(parse_trigger_duplication_line_ast),
        single_static_ability_ast_rule!(
            parse_double_damage_from_sources_you_control_of_chosen_type_line
        ),
        single_static_ability_ast_rule!(parse_double_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_minimum_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_foretelling_cards_cost_modifier_line),
        single_static_ability_ast_rule!(parse_players_skip_upkeep_line),
        single_static_ability_ast_rule!(parse_legend_rule_doesnt_apply_line),
        multi_static_ability_ast_rule!(
            parse_subject_are_card_types_in_addition_to_their_other_types_line
        ),
        single_static_ability_ast_rule!(parse_all_permanents_colorless_line),
        single_static_ability_ast_rule!(parse_all_cards_spells_permanents_colorless_line),
        multi_static_ability_ast_rule!(parse_all_are_color_and_type_addition_line),
        single_static_ability_ast_rule!(parse_all_cards_spells_permanents_add_chosen_color_line),
        single_static_ability_ast_rule!(parse_all_creatures_are_color_line),
        single_static_ability_ast_rule!(parse_subjects_are_basic_line),
        single_static_ability_ast_rule!(parse_protection_from_colored_spells_line),
        single_static_ability_ast_rule!(parse_nonbasic_lands_are_basic_land_type_line),
        single_static_ability_ast_rule!(parse_land_type_addition_line),
        multi_static_ability_ast_rule!(parse_lands_are_pt_creatures_still_lands_line),
        single_static_ability_ast_rule!(parse_remove_snow_line),
        multi_static_ability_ast_rule!(parse_attached_is_legendary_gets_and_has_keywords_line),
        single_static_ability_ast_rule!(parse_landwalk_as_though_block_override_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_granted_keyword_static_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_granted_keyword_static_line),
        },
        multi_static_ability_ast_rule!(parse_equipment_you_control_have_equip_line),
        multi_static_ability_ast_rule!(parse_lose_all_abilities_and_transform_base_pt_line),
        multi_static_ability_ast_rule!(parse_lose_all_abilities_and_base_pt_line),
        multi_static_ability_ast_passthrough_rule!(parse_subject_loses_keywords_line),
        single_static_ability_ast_passthrough_rule!(parse_all_creatures_lose_flying_line),
        single_static_ability_ast_passthrough_rule!(
            parse_each_creature_cant_be_blocked_by_more_than_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_each_creature_can_block_additional_creature_each_combat_line
        ),
        multi_static_ability_ast_rule!(parse_anthem_and_type_color_addition_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_anthem_and_keyword_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_anthem_and_keyword_line),
        },
        multi_static_ability_ast_rule!(parse_anthem_and_goaded_line),
        multi_static_ability_ast_passthrough_rule!(parse_anthem_and_granted_ability_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_subject_has_keywords_and_cant_be_blocked_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_subject_is_every_subtype_family_line),
        single_static_ability_ast_passthrough_rule!(parse_all_have_indestructible_line),
        single_static_ability_ast_passthrough_rule!(
            parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_subject_cant_be_blocked_as_long_as_condition_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_subject_cant_be_blocked_line),
        single_static_ability_ast_rule!(parse_may_choose_not_to_untap_during_untap_step_line),
        single_static_ability_ast_rule!(parse_untap_during_each_other_players_untap_step_line),
        single_static_ability_ast_passthrough_rule!(parse_doesnt_untap_during_untap_step_line),
        multi_static_ability_ast_rule!(parse_equipped_creature_has_line),
        multi_static_ability_ast_rule!(parse_enchanted_creature_has_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_tap_abilities_cant_be_activated_line
        ),
        multi_static_ability_ast_rule!(parse_attached_type_transform_line),
        multi_static_ability_ast_rule!(parse_attached_has_and_loses_keywords_line),
        single_static_ability_ast_rule!(parse_you_control_attached_creature_line),
        single_static_ability_ast_passthrough_rule!(parse_attached_cant_attack_or_block_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_by_attached_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_combat_damage_dealt_by_attached_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_to_attached_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_attached_gets_and_cant_block_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_attached_has_keywords_and_triggered_ability_line),
            rule: StaticAbilityLineRuleAst::Multi(
                parse_attached_has_keywords_and_triggered_ability_line,
            ),
        },
        StaticAbilityLineRuleDef {
            id: stringify!(parse_attached_gets_and_has_ability_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_attached_gets_and_has_ability_line),
        },
        StaticAbilityLineRuleDef {
            id: stringify!(parse_anthem_with_trailing_segments_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_anthem_with_trailing_segments_line),
        },
        multi_static_ability_ast_passthrough_rule!(parse_gets_and_attacks_each_combat_if_able_line),
        single_static_ability_ast_passthrough_rule!(
            parse_conditional_all_creatures_able_to_block_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_as_long_as_condition_can_attack_as_though_no_defender_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_source_can_attack_as_though_no_defender_as_long_as_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_attacks_each_combat_if_able_line),
        single_static_ability_ast_rule!(parse_source_must_be_blocked_if_able_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_composed_anthem_effects_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_composed_anthem_effects_line),
        },
        single_static_ability_ast_rule!(parse_has_base_power_toughness_static_line),
        single_static_ability_ast_rule!(parse_isnt_creature_line),
        multi_static_ability_ast_rule!(parse_multi_subject_anthem_line),
        single_static_ability_ast_rule!(parse_anthem_line),
        single_static_ability_ast_rule!(parse_flying_restriction_line),
        single_static_ability_ast_rule!(parse_can_block_only_flying_line),
        single_static_ability_ast_rule!(parse_can_block_subtype_as_though_reach_line),
        single_static_ability_ast_rule!(parse_assign_damage_as_unblocked_line),
        single_static_ability_ast_rule!(parse_fixed_mana_cost_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_mana_value_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_life_mana_value_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_as_enters_or_turns_face_up_pt_choice_line),
        single_static_ability_ast_rule!(parse_as_enters_becomes_characteristics_for_filter_line),
        single_static_ability_ast_rule!(parse_enter_as_copy_as_enters_line),
        multi_static_ability_ast_rule!(
            parse_you_may_cast_exile_counter_cards_with_mana_permission_line
        ),
        multi_static_ability_ast_rule!(parse_surveilled_graveyard_play_life_cost_line),
        single_static_ability_ast_rule!(parse_play_from_permission_with_haste_this_way_line),
        single_static_ability_ast_rule!(parse_you_may_static_grant_line),
        single_static_ability_ast_rule!(parse_grant_flash_to_noncreature_spells_line),
        single_static_ability_ast_rule!(parse_cast_this_spell_as_though_it_had_flash_line),
        single_static_ability_ast_rule!(parse_during_your_turn_prevent_all_damage_to_source_line),
        single_static_ability_ast_rule!(parse_prevent_all_combat_damage_to_source_line),
        single_static_ability_ast_rule!(
            parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line
        ),
        single_static_ability_ast_rule!(parse_prevent_all_damage_to_source_by_creatures_line),
        single_static_ability_ast_rule!(
            parse_prevent_damage_to_other_creature_you_control_put_counters_line
        ),
        single_static_ability_ast_rule!(parse_prevent_all_damage_dealt_to_creatures_line),
        single_static_ability_ast_passthrough_rule!(parse_creatures_cant_block_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_counters_line),
        single_static_ability_ast_rule!(parse_enters_with_counters_line),
        single_static_ability_ast_rule!(parse_enters_with_additional_counter_for_filter_line),
        single_static_ability_ast_rule!(parse_reveal_from_hand_or_enters_tapped_line),
        single_static_ability_ast_rule!(parse_conditional_enters_tapped_unless_line),
        single_static_ability_ast_rule!(parse_enters_untapped_for_filter_line),
        single_static_ability_ast_rule!(parse_enters_tapped_for_filter_line),
        single_static_ability_ast_rule!(parse_enters_tapped_line),
        multi_static_ability_ast_rule!(parse_additional_land_play_line),
        single_static_ability_ast_rule!(parse_you_may_look_top_card_any_time_line),
        single_static_ability_ast_rule!(
            parse_you_may_look_face_down_creatures_you_dont_control_any_time_line
        ),
        single_static_ability_ast_rule!(parse_players_play_top_card_libraries_revealed_line),
        single_static_ability_ast_rule!(parse_play_top_card_your_library_revealed_line),
        single_static_ability_ast_rule!(parse_your_opponents_play_with_hands_revealed_line),
        single_static_ability_ast_rule!(parse_control_opponents_while_searching_libraries_line),
        single_static_ability_ast_rule!(parse_opponent_search_exile_found_cards_line),
        single_static_ability_ast_rule!(parse_cast_this_card_from_library_while_searching_line),
        single_static_ability_ast_rule!(parse_play_lands_from_graveyard_line),
        single_static_ability_ast_rule!(parse_graveyard_cards_have_retrace_line),
        single_static_ability_ast_rule!(parse_cast_spells_from_hand_without_paying_mana_costs_line),
        single_static_ability_ast_rule!(parse_cost_reduction_line),
        single_static_ability_ast_rule!(parse_can_block_additional_creature_each_combat_line),
        single_static_ability_ast_passthrough_rule!(parse_all_creatures_able_to_block_source_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_all_creatures_able_to_block_line
        ),
        single_static_ability_ast_rule!(parse_activated_abilities_cant_be_activated_line),
        multi_static_ability_ast_rule!(parse_cant_clauses),
    ]
}

static STATIC_ABILITY_AST_LINE_RULE_INDEX: LazyLock<LexRuleHintIndex> = LazyLock::new(|| {
    let rules = static_ability_ast_line_rules();
    build_lex_rule_hint_index(rules.len(), |idx| {
        static_ability_rule_head_hints(rules[idx].id)
    })
});

fn parse_static_ability_ast_line_lowered(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let rules = static_ability_ast_line_rules();
    let (head, second) = lexed_head_words(tokens).unwrap_or(("", None));
    let mut tried = vec![false; rules.len()];
    let mut deferred_error: Option<CardTextError> = None;

    let candidate_indices = STATIC_ABILITY_AST_LINE_RULE_INDEX.candidate_indices(head, second);
    if !candidate_indices.is_empty() {
        if let Some(abilities) = try_static_ability_ast_line_rule_indices(
            rules,
            tokens,
            &mut tried,
            &mut deferred_error,
            &candidate_indices,
        ) {
            return Ok(Some(abilities));
        }
    }

    for (idx, rule) in rules.iter().enumerate() {
        if tried[idx] {
            continue;
        }
        match run_static_ability_ast_line_rule(rule.rule, tokens) {
            Ok(Some(abilities)) => return Ok(Some(abilities)),
            Ok(None) => {}
            Err(err) => {
                deferred_error.get_or_insert(err);
            }
        }
    }

    if let Some(err) = deferred_error {
        return Err(err);
    }

    Ok(None)
}

fn parse_static_ability_ast_line_early_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let rendered = crate::runtime_backend::token_word_refs(tokens)
        .join(" ")
        .to_ascii_lowercase()
        .replace("can t", "cant")
        .replace("you ve", "youve")
        .replace("'", "");
    let rendered = rendered.trim().trim_end_matches('.').to_string();
    let marker_text = render_token_slice(tokens);
    if is_ticket_sticker_marker_line(&marker_text) {
        return Ok(Some(vec![keyword_static_marker(tokens).into()]));
    }

    if rendered == "x cant be greater than the number of players in the game" {
        return Ok(Some(vec![
            StaticAbility::this_spell_x_maximum(
                Value::CountPlayers(PlayerFilter::Any),
                "X can't be greater than the number of players in the game.",
            )
            .into(),
        ]));
    }
    if rendered
        == "during your turn as long as you havent activated an exhaust ability this turn you may activate exhaust abilities as though they havent been activated"
    {
        return Ok(Some(vec![
            StaticAbility::exhaust_abilities_as_though_unactivated_this_turn().into(),
        ]));
    }
    if rendered == "this creature cant attack unless youve cast a creature spell this turn"
        || rendered == "this cant attack unless youve cast a creature spell this turn"
    {
        return Ok(Some(vec![
            StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn().into(),
        ]));
    }
    if rendered == "this creature cant attack unless youve cast a noncreature spell this turn"
        || rendered == "this cant attack unless youve cast a noncreature spell this turn"
    {
        return Ok(Some(vec![
            StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn().into(),
        ]));
    }

    if let Some(spec) = parse_reveal_first_card_you_draw_each_turn_spec_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::reveal_first_card_you_draw_each_turn(
                spec.optional,
                spec.your_turns_only,
            )
            .into(),
        ]));
    }

    let words = parser_token_word_refs(tokens);
    if word_slice_starts_with(&words, &["this", "creature", "can", "block"]) {
        let mut idx = 4usize;
        if word_slice_at_is(&words, idx, "an") {
            idx += 1;
        }

        if word_slice_at_is(&words, idx, "additional") {
            idx += 1;
            let mut additional = 1usize;
            if let Some((count, used)) = parse_number(&tokens[idx..]) {
                additional = count as usize;
                idx += used;
            }

            if matches!(
                words.get(idx).copied(),
                Some("creature") | Some("creatures")
            ) && (words.get(idx + 1..).unwrap_or_default().is_empty()
                || word_slice_eq(&words[idx + 1..], &["each", "combat"])
                || word_slice_eq(&words[idx + 1..], &["this", "turn"]))
            {
                return Ok(Some(vec![
                    StaticAbility::can_block_additional_creature_each_combat(additional).into(),
                ]));
            }
        }
    }

    let normalized_line = render_token_slice(tokens)
        .to_ascii_lowercase()
        .replace('\u{2019}', "'")
        .replace("it's", "its")
        .replace(',', "")
        .trim()
        .trim_end_matches('.')
        .to_string();
    if normalized_line.contains("neither day nor night")
        && normalized_line.contains("it becomes day")
        && (normalized_line.contains("as this creature enters")
            || normalized_line.contains("as this permanent enters")
            || normalized_line.contains("as this object enters"))
    {
        return Ok(Some(vec![
            StaticAbility::day_night_starts_day_as_enters().into(),
        ]));
    }
    if matches!(
        words.as_slice(),
        [
            "if",
            "its",
            "neither",
            "day",
            "nor",
            "night",
            "it",
            "becomes",
            "day",
            "as",
            "this",
            "creature" | "permanent" | "object",
            "enters"
        ]
    ) {
        return Ok(Some(vec![
            StaticAbility::day_night_starts_day_as_enters().into(),
        ]));
    }
    if matches!(
        words.as_slice(),
        [
            "this",
            "creature",
            "crews",
            "vehicles",
            "using",
            "its",
            "toughness",
            "rather",
            "than",
            "its",
            "power"
        ]
    ) || matches!(
        words.as_slice(),
        [
            "this",
            "creature",
            "saddles",
            "mounts",
            "and",
            "crews",
            "vehicles",
            "using",
            "its",
            "toughness",
            "rather",
            "than",
            "its",
            "power"
        ]
    ) || matches!(
        words.as_slice(),
        [
            "this",
            "creature",
            "saddles",
            "mounts",
            "and",
            "crews",
            "vehicles",
            "as",
            "though",
            "its",
            "power",
            "were",
            _,
            "greater"
        ]
    ) || matches!(
        words.as_slice(),
        [
            "this",
            "creature",
            "crews",
            "vehicles",
            "as",
            "though",
            "its",
            "power",
            "were",
            _,
            "greater"
        ]
    ) || matches!(
        words.as_slice(),
        [
            "you",
            "may",
            "remove",
            "a",
            "loyalty",
            "counter",
            "from",
            "a",
            "planeswalker",
            "you",
            "control",
            "rather",
            "than",
            "pay",
            ..,
            "crew",
            "cost"
        ]
    ) {
        return Ok(Some(vec![keyword_static_marker(tokens).into()]));
    }
    if is_minimum_spell_total_mana_three_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::minimum_spell_total_mana(3).into(),
        ]));
    }
    if is_players_cant_pay_life_or_sacrifice_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
        ]));
    }
    if is_krrik_black_mana_life_payment_line_lexed(tokens) {
        return Ok(Some(vec![
            StaticAbility::krrik_black_mana_may_be_paid_with_life().into(),
        ]));
    }
    if let Some(ability) = parse_cycling_cost_alternative_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(abilities) = parse_spell_and_player_activated_ability_cost_modifier_line(tokens)? {
        return Ok(Some(
            abilities.into_iter().map(StaticAbilityAst::from).collect(),
        ));
    }

    if let Some(spec) = split_untap_each_other_players_untap_step_line_lexed(tokens) {
        let subject_tokens = trim_commas(spec.subject_tokens);
        let filter = parse_object_filter(&subject_tokens, false)?;
        let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(" ");
        let display = if spec.untap_all {
            format!("Untap all {subject_text} during each other player's untap step")
        } else {
            format!("Untap {subject_text} during each other player's untap step")
        };
        return Ok(Some(vec![
            StaticAbility::untap_during_each_other_players_untap_step(filter, display).into(),
        ]));
    }

    if let Some(ability) = parse_activated_abilities_cant_be_activated_line_lexed(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(ability) = parse_legend_rule_doesnt_apply_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }

    Ok(None)
}

pub(crate) fn parse_damage_doubling_mana_value_marker_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(
        &clause_words,
        &["if", "a", "source", "you", "control", "with"],
    ) || !word_slice_contains_word(&clause_words, "mana")
        || !word_slice_contains_word(&clause_words, "value")
        || find_window_by(&clause_words, 5, |window| {
            word_slice_eq_any(
                window,
                &[
                    &["would", "deal", "damage", "to", "a"],
                    &["would", "deal", "damage", "to", "target"],
                ],
            )
        })
        .is_none()
        || !word_slice_contains_word(&clause_words, "double")
        || !word_slice_last_is(&clause_words, "instead")
    {
        return Ok(None);
    }

    Ok(Some(keyword_static_marker(tokens)))
}

pub(crate) fn parse_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() > 1 {
        let mut combined = Vec::new();
        for sentence in sentences {
            match parse_static_ability_ast_line_lexed_single(sentence) {
                Ok(Some(mut parsed)) => combined.append(&mut parsed),
                Ok(None) | Err(_) => return parse_static_ability_ast_line_lexed_single(tokens),
            }
        }
        return Ok((!combined.is_empty()).then_some(combined));
    }

    parse_static_ability_ast_line_lexed_single(tokens)
}

fn parse_static_ability_ast_line_lexed_single(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if looks_like_trigger_intro_tokens(tokens) || looks_like_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if looks_like_player_counter_gain_effect_tokens(tokens) {
        return Ok(None);
    }

    if let Some(abilities) = parse_static_ability_ast_line_early_lexed(tokens)? {
        return Ok(Some(abilities));
    }

    parse_static_ability_ast_line_lowered(tokens)
}

fn looks_like_player_counter_gain_effect_tokens(tokens: &[OwnedLexToken]) -> bool {
    let Some(get_idx) = find_index(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    }) else {
        return false;
    };

    let has_counter_resource = tokens.iter().any(|token| {
        token.is_word("energy")
            || token.is_word("poison")
            || token.is_word("ticket")
            || token.is_word("e")
            || token.is_word("tk")
            || (token.kind == TokenKind::ManaGroup
                && matches!(
                    token
                        .slice
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .to_ascii_lowercase()
                        .as_str(),
                    "e" | "tk"
                ))
    });
    if !has_counter_resource {
        return false;
    }

    let tail = tokens.get(get_idx + 1..).unwrap_or_default();
    if parse_value(tail).is_some() {
        return true;
    }

    tail.iter()
        .find_map(OwnedLexToken::as_word)
        .is_some_and(|word| {
            word == "another" || ironsmith_core::parse_cardinal_word(word).is_some()
        })
}

pub(crate) fn parse_activated_abilities_cant_be_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::effect::Restriction;

    let normalized = crate::runtime_backend::token_word_refs(tokens);
    if normalized.len() < 6
        || !word_slice_starts_with(&normalized, &["activated", "abilities", "of"])
    {
        return Ok(None);
    }

    let Some(cant_idx) = find_index(&normalized, |word| *word == "cant") else {
        return Ok(None);
    };
    if cant_idx <= 3 {
        return Ok(None);
    }

    let tail = &normalized[cant_idx..];
    if !word_slice_starts_with(tail, &["cant", "be", "activated"]) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[3..cant_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    // "Activated abilities of artifacts and creatures ..." should be a union of types.
    // Our general object filter parser treats type lists joined by "and" as intersection,
    // which is correct for many adjective chains, but incorrect for this rules pattern.
    let subject_words = crate::runtime_backend::util::non_article_token_word_refs(&subject_tokens);

    let filter = if matches!(
        subject_words.as_slice(),
        ["sources", "with", "chosen", "name"] | ["sources", "with", "the", "chosen", "name"]
    ) {
        let mut filter = ObjectFilter::default();
        filter.name = Some("{chosen name}".to_string());
        filter
    } else if subject_words.len() == 3 && subject_words[1] == "and" {
        let t1 = str_strip_suffix(subject_words[0], "s").unwrap_or(subject_words[0]);
        let t2 = str_strip_suffix(subject_words[2], "s").unwrap_or(subject_words[2]);
        if let (Some(ct1), Some(ct2)) = (parse_card_type(t1), parse_card_type(t2)) {
            let mut a = ObjectFilter::default();
            a.zone = Some(Zone::Battlefield);
            a.card_types = vec![ct1];

            let mut b = ObjectFilter::default();
            b.zone = Some(Zone::Battlefield);
            b.card_types = vec![ct2];

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![a, b];
            disjunction
        } else {
            parse_object_filter(&subject_tokens, false)?
        }
    } else {
        parse_object_filter(&subject_tokens, false)?
    };

    let non_mana_only =
        word_slice_contains_phrase(&normalized, &["unless", "theyre", "mana", "abilities"]);

    let restriction = if non_mana_only {
        Restriction::activate_non_mana_abilities_of(filter)
    } else {
        Restriction::activate_abilities_of(filter)
    };

    let display_subject = subject_words.join(" ");
    let display = if non_mana_only {
        format!(
            "Activated abilities of {display_subject} can't be activated unless they're mana abilities."
        )
    } else {
        format!("Activated abilities of {display_subject} can't be activated.")
    };

    Ok(Some(StaticAbility::restriction(restriction, display)))
}

pub(crate) fn parse_activated_abilities_cost_increase_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 8
        || !word_slice_starts_with(&clause_words, &["activated", "abilities", "of"])
    {
        return Ok(None);
    }

    let Some(cost_idx) = find_index(&clause_words, |word| *word == "cost" || *word == "costs")
    else {
        return Ok(None);
    };
    if cost_idx <= 3 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[3..cost_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability cost increase subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }

    let amount_tokens = trim_commas(&tokens[cost_idx + 1..]);
    let amount_words = crate::runtime_backend::token_word_refs(&amount_tokens);
    if !word_slice_starts_with(&amount_words, &["an", "additional"])
        && !word_slice_starts_with(&amount_words, &["a", "additional"])
    {
        return Ok(None);
    }

    let cost_tokens = trim_commas(&amount_tokens[2..]);
    let Some(to_idx) = find_index(
        &crate::runtime_backend::token_word_refs(&cost_tokens),
        |word| *word == "to",
    ) else {
        return Ok(None);
    };
    let to_token_idx = crate::runtime_backend::token_index_for_word_index(&cost_tokens, to_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing activated-ability additional cost terminator (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let additional_cost_tokens = trim_commas(&cost_tokens[..to_token_idx]);
    let additional_cost_tokens = trim_outer_quotes(&additional_cost_tokens);
    if additional_cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing activated-ability additional cost (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let total_cost = parse_activation_cost(&additional_cost_tokens)?;
    if total_cost.is_free() {
        return Err(CardTextError::ParseError(format!(
            "unsupported activated-ability additional cost (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let tail_words = crate::runtime_backend::token_word_refs(&cost_tokens[to_token_idx..]);
    if !word_slice_starts_with(&tail_words, &["to", "activate"]) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::increase_activated_ability_costs(
        filter, total_cost,
    )))
}

pub(crate) fn parse_activated_abilities_cant_be_activated_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::effect::Restriction;

    let Some(spec) = parse_activated_abilities_cant_be_activated_spec_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = spec.subject_tokens;
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = crate::runtime_backend::util::non_article_token_word_refs(subject_tokens);

    let filter = if matches!(
        subject_words.as_slice(),
        ["sources", "with", "chosen", "name"] | ["sources", "with", "the", "chosen", "name"]
    ) {
        let mut filter = ObjectFilter::default();
        filter.name = Some("{chosen name}".to_string());
        filter
    } else if subject_words.len() == 3 && subject_words[1] == "and" {
        let t1 = str_strip_suffix(subject_words[0], "s").unwrap_or(subject_words[0]);
        let t2 = str_strip_suffix(subject_words[2], "s").unwrap_or(subject_words[2]);
        if let (Some(ct1), Some(ct2)) = (parse_card_type(t1), parse_card_type(t2)) {
            let mut a = ObjectFilter::default();
            a.zone = Some(Zone::Battlefield);
            a.card_types = vec![ct1];

            let mut b = ObjectFilter::default();
            b.zone = Some(Zone::Battlefield);
            b.card_types = vec![ct2];

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![a, b];
            disjunction
        } else {
            parse_object_filter_lexed(subject_tokens, false)?
        }
    } else {
        parse_object_filter_lexed(subject_tokens, false)?
    };

    let non_mana_only = spec.non_mana_only;

    let restriction = if non_mana_only {
        Restriction::activate_non_mana_abilities_of(filter)
    } else {
        Restriction::activate_abilities_of(filter)
    };

    let display_subject = subject_words.join(" ");
    let display = if non_mana_only {
        format!(
            "Activated abilities of {display_subject} can't be activated unless they're mana abilities."
        )
    } else {
        format!("Activated abilities of {display_subject} can't be activated.")
    };

    Ok(Some(StaticAbility::restriction(restriction, display)))
}

pub(crate) fn parse_pregame_begin_on_battlefield_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let mentions_opening_hand =
        word_slice_contains_phrase(&clause_words, &["your", "opening", "hand"]);
    if !word_slice_first_is(&clause_words, "if")
        || !mentions_opening_hand
        || !word_slice_contains_phrase(
            &clause_words,
            &["you", "may", "begin", "the", "game", "with"],
        )
        || !word_slice_contains_phrase(&clause_words, &["on", "the", "battlefield"])
    {
        return Ok(None);
    }

    let source_ref_start = find_source_reference_start(&tokens[1..]).map(|idx| idx + 1);
    if source_ref_start.is_none()
        && !word_slice_starts_with_at(&clause_words, 1, &["this", "card", "is"])
    {
        return Ok(None);
    }

    let require_not_starting_player = word_slice_contains_any_phrase(
        &clause_words,
        &[
            &["youre", "not", "playing", "first"],
            &["you're", "not", "playing", "first"],
            &["you", "re", "not", "playing", "first"],
            &["you", "are", "not", "playing", "first"],
            &["youre", "not", "the", "starting", "player"],
            &["you're", "not", "the", "starting", "player"],
            &["you", "re", "not", "the", "starting", "player"],
            &["you", "are", "not", "the", "starting", "player"],
            &["youre", "not", "starting", "the", "game"],
            &["you're", "not", "starting", "the", "game"],
            &["you", "re", "not", "starting", "the", "game"],
            &["you", "are", "not", "starting", "the", "game"],
        ],
    );

    let battlefield_end_word_idx = crate::runtime_backend::lexer::word_slice_find_phrase_start(
        &clause_words,
        &["on", "the", "battlefield"],
    )
    .map(|idx| idx + 3)
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing battlefield destination in pregame line (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let if_you_do_word_idx = crate::runtime_backend::lexer::word_slice_find_phrase_start(
        &clause_words,
        &["if", "you", "do"],
    );

    let mut counters = Vec::new();
    let counter_tail_start =
        token_index_for_word_index(tokens, battlefield_end_word_idx).unwrap_or(tokens.len());
    let counter_tail_end = if_you_do_word_idx
        .and_then(|idx| token_index_for_word_index(tokens, idx))
        .unwrap_or(tokens.len());
    let counter_tail = trim_commas(&tokens[counter_tail_start..counter_tail_end]);
    if !counter_tail.is_empty() {
        if !counter_tail
            .first()
            .is_some_and(|token| token.is_word("with"))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported pregame battlefield modifier (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let after_with = &counter_tail[1..];
        let (count, used) = if after_with
            .first()
            .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        {
            (1u32, 1usize)
        } else {
            parse_number(after_with).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter count in pregame line (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };
        let counter_type =
            parse_counter_type_from_tokens(&after_with[used..]).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counter type in pregame line (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let counter_word_idx = find_index(after_with, |token| {
            token.is_word("counter") || token.is_word("counters")
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter keyword in pregame line (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let trailing = crate::runtime_backend::token_word_refs(&after_with[counter_word_idx + 1..]);
        if !word_slice_eq(&trailing, &["on", "it"]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported counter placement tail in pregame line (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        counters.push((counter_type, count));
    }

    let exile_cards_from_hand = if let Some(if_you_do_word_idx) = if_you_do_word_idx {
        let exile_start =
            token_index_for_word_index(tokens, if_you_do_word_idx + 3).unwrap_or(tokens.len());
        let exile_tail = trim_commas(&tokens[exile_start..]);
        if !exile_tail
            .first()
            .is_some_and(|token| token.is_word("exile"))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported pregame follow-up clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let after_exile = &exile_tail[1..];
        let (count, used) = if after_exile
            .first()
            .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        {
            (1u32, 1usize)
        } else {
            parse_number(after_exile).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing exile count in pregame follow-up (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };
        let trailing = crate::runtime_backend::token_word_refs(&after_exile[used..]);
        if !word_slice_eq_any(
            &trailing,
            &[
                &["card", "from", "your", "hand"],
                &["cards", "from", "your", "hand"],
            ],
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported pregame exile tail (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        count as usize
    } else {
        0
    };

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::BeginOnBattlefield(
            crate::static_abilities::PregameBeginOnBattlefieldSpec {
                require_not_starting_player,
                counters,
                exile_cards_from_hand,
            },
        ),
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_pregame_mulligan_redraw_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_contains_phrase(&clause_words, &["any", "time", "you", "could", "mulligan"])
        || !word_slice_contains_phrase(&clause_words, &["is", "in", "your", "hand"])
        || !word_slice_contains_phrase(
            &clause_words,
            &[
                "you", "may", "exile", "all", "the", "cards", "from", "your", "hand",
            ],
        )
        || !word_slice_contains_phrase(&clause_words, &["then", "draw", "that", "many", "cards"])
    {
        return Ok(None);
    }

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount,
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_pregame_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let mut choose_idx = None;
    for (idx, word) in clause_words.iter().enumerate() {
        if *word == "choose" {
            choose_idx = Some(idx);
            break;
        }
    }
    let Some(choose_idx) = choose_idx else {
        return Ok(None);
    };
    let Some((consumed, excluded)) = parse_choose_color_phrase_words(&clause_words[choose_idx..])?
    else {
        return Ok(None);
    };
    if excluded.is_some() {
        return Ok(None);
    }
    let tail = &clause_words[choose_idx + consumed..];
    if !word_slice_eq(tail, &["before", "the", "game", "begins"]) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::ChooseColor,
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_combined_pregame_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let sentences = split_lexed_slices_on_period(tokens);
    if sentences.len() < 2 {
        return Ok(None);
    }

    let Some(first) = parse_pregame_choose_color_line(sentences[0])? else {
        return Ok(None);
    };
    let Some(second) = parse_source_is_chosen_color_line(sentences[1])? else {
        return Ok(None);
    };
    Ok(Some(vec![first, second]))
}

pub(crate) fn parse_can_block_additional_creature_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(&normalized, &["this", "creature", "can", "block"])
        || !matches!(
            normalized.last().copied(),
            Some("combat") | Some("turn") | Some("creature") | Some("creatures")
        )
    {
        return Ok(None);
    }

    let mut idx = 4usize;
    if word_slice_at_is(&normalized, idx, "an") {
        idx += 1;
    }

    if !word_slice_at_is(&normalized, idx, "additional") {
        return Ok(None);
    }
    idx += 1;

    let mut additional = 1usize;
    if let Some((count, used)) = parse_number(&tokens[idx..]) {
        additional = count as usize;
        idx += used;
    }

    if !matches!(
        normalized.get(idx).copied(),
        Some("creature") | Some("creatures")
    ) {
        return Ok(None);
    }
    idx += 1;

    let tail = &normalized[idx..];
    if !tail.is_empty() && !word_slice_eq_any(tail, &[&["each", "combat"], &["this", "turn"]]) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::can_block_additional_creature_each_combat(additional),
    ))
}

pub(crate) fn parse_skulk_rules_text_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_skulk_rules_text_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::cant_be_blocked_by_lower_power_than_source(),
    ))
}

pub(crate) fn parse_ward_static_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_first_is(&clause_words, "ward") {
        return Ok(None);
    }

    let cost_tokens = trim_commas(&tokens[1..]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "ward keyword missing cost".to_string(),
        ));
    }

    if let Some(cost) = parse_ward_discard_card_type_cost(&cost_tokens) {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    if let Some(cost) = parse_payment_clause_as_total_cost(&cost_tokens)? {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported ward cost clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_ward_discard_card_type_cost(tokens: &[OwnedLexToken]) -> Option<TotalCost> {
    let cost_words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_first_is(&cost_words, "discard") {
        return None;
    }

    let mut idx = 1usize;
    let mut count = 1u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }

    let words_tail = &cost_words[idx..];
    if word_slice_eq(words_tail, &["your", "hand"]) {
        return Some(TotalCost::from_cost(crate::costs::Cost::discard_hand()));
    }

    while cost_words
        .get(idx)
        .is_some_and(|word| *word == "a" || *word == "an")
    {
        idx += 1;
    }

    let mut card_types = Vec::<CardType>::new();
    while let Some(word) = cost_words.get(idx) {
        if *word == "card" || *word == "cards" {
            idx += 1;
            break;
        }
        if *word == "and" || *word == "or" || *word == "a" || *word == "an" {
            idx += 1;
            continue;
        }
        let parsed = parse_card_type(word)?;
        if !card_types.iter().any(|existing| *existing == parsed) {
            card_types.push(parsed);
        }
        idx += 1;
    }

    if idx != cost_words.len() {
        return None;
    }

    let cost = if card_types.len() > 1 {
        crate::costs::Cost::discard_types(count, card_types)
    } else if let Some(card_type) = card_types.first().copied() {
        crate::costs::Cost::discard(count, Some(card_type))
    } else {
        crate::costs::Cost::discard(count, None)
    };
    Some(TotalCost::from_cost(cost))
}

pub(crate) fn parse_composed_anthem_effects_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let comma_segments = split_anthem_trailing_segments_preserving_granted_abilities(tokens);
    if comma_segments.len() < 2 {
        return Ok(None);
    }

    if comma_segments.len() == 2 {
        let where_tail = trim_commas(&comma_segments[1]);
        let where_words = crate::runtime_backend::token_word_refs(&where_tail);
        if word_slice_starts_with(&where_words, &["where", "x", "is"])
            && let Some(ability) = parse_anthem_line(tokens)?
        {
            return Ok(Some(vec![ability.into()]));
        }
    }

    let Some(first_action_idx) = find_index(tokens, |token| {
        token.is_word("get")
            || token.is_word("gets")
            || token.is_word("have")
            || token.is_word("has")
    }) else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(&tokens[..first_action_idx]);
    if subject_tokens.is_empty() || parse_anthem_subject(&subject_tokens).is_err() {
        return Ok(None);
    }

    let mut saw_omitted_subject_clause = false;
    let mut compiled = Vec::new();

    for (idx, raw_segment) in comma_segments.into_iter().enumerate() {
        let mut segment = trim_commas(&raw_segment).to_vec();
        if segment.is_empty() {
            continue;
        }

        if token_slice_first_is(&segment, "and") {
            let trimmed = trim_commas(&segment[1..]);
            if token_slice_first_is_any(&trimmed, &["get", "gets", "have", "has"]) {
                segment = trimmed.to_vec();
            }
        }

        let starts_with_action =
            token_slice_first_is_any(&segment, &["get", "gets", "have", "has"]);
        if starts_with_action {
            if idx > 0 {
                saw_omitted_subject_clause = true;
            }
            let mut expanded = subject_tokens.clone();
            expanded.extend(segment);
            segment = expanded;
        }

        let parsed_segment =
            if let Some(abilities) = parse_anthem_and_type_color_addition_line(&segment)? {
                abilities.into_iter().map(StaticAbilityAst::from).collect()
            } else if let Some(abilities) = parse_anthem_and_keyword_line(&segment)? {
                abilities
            } else if let Some(abilities) = parse_granted_keyword_static_line(&segment)? {
                abilities
            } else if let Some(ability) = parse_anthem_line(&segment)? {
                vec![ability.into()]
            } else {
                return Ok(None);
            };
        compiled.extend(parsed_segment);
    }

    if !saw_omitted_subject_clause || compiled.len() < 2 {
        return Ok(None);
    }

    Ok(Some(compiled))
}

pub(crate) fn parse_static_text_marker_line(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    if tokens.is_empty() {
        return None;
    }

    if is_once_each_turn_play_from_exile_marker_guard_lexed(tokens) {
        return None;
    }

    if is_doctors_companion_marker_line_lexed(tokens) {
        return Some(StaticAbility::doctors_companion());
    }

    if word_slice_eq(
        &crate::runtime_backend::token_word_refs(tokens),
        &["banding"],
    ) {
        return Some(StaticAbility::banding());
    }

    if is_companion_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_more_than_meets_the_eye_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_protection_mana_value_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_mana_group_slash_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_if_source_you_control_with_mana_value_double_instead_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_as_long_as_power_odd_or_even_flash_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_attack_as_haste_unless_entered_this_turn_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_sab_sunen_cant_attack_or_block_unless_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    if is_you_have_shroud_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::be_targeted_player(PlayerFilter::You),
            "You have shroud".to_string(),
        ));
    }

    if word_slice_eq(
        &crate::runtime_backend::token_word_refs(tokens),
        &["you", "have", "hexproof"],
    ) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::be_targeted_player_from(
                PlayerFilter::You,
                ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
            ),
            "You have hexproof".to_string(),
        ));
    }

    if crate::runtime_backend::token_word_refs(tokens)
        == [
            "you",
            "have",
            "protection",
            "from",
            "each",
            "of",
            "your",
            "opponents",
        ]
    {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::be_targeted_player_from(
                PlayerFilter::You,
                ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
            ),
            "You have protection from each of your opponents".to_string(),
        ));
    }

    let words = parser_token_word_refs(tokens);
    if words
        == [
            "each", "opponent", "can", "cast", "spells", "only", "any", "time", "they", "could",
            "cast", "a", "sorcery",
        ]
    {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::cast_spells_only_as_sorcery(PlayerFilter::Opponent),
            "Each opponent can cast spells only any time they could cast a sorcery.".to_string(),
        ));
    }

    if words
        == [
            "if",
            "a",
            "source",
            "would",
            "deal",
            "damage",
            "to",
            "enchanted",
            "player",
            "it",
            "deals",
            "double",
            "that",
            "damage",
            "to",
            "that",
            "player",
            "instead",
        ]
    {
        return Some(StaticAbility::double_damage_amount_replacement(
            ObjectFilter::default(),
            Some(PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted"))),
            None,
            "If a source would deal damage to enchanted player, it deals double that damage to that player instead.".to_string(),
        ));
    }

    if is_creatures_without_flying_cant_attack_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::attack(
                ObjectFilter::creature()
                    .without_static_ability(crate::static_abilities::StaticAbilityId::Flying),
            ),
            "Creatures without flying can't attack".to_string(),
        ));
    }

    if is_this_creature_cant_attack_alone_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "This creature can't attack alone".to_string(),
        ));
    }

    if is_this_creature_cant_attack_its_owner_line_lexed(tokens) {
        return Some(StaticAbility::cant_attack_its_owner());
    }

    if let Some(amount) = parse_ward_pay_life_amount_lexed(tokens) {
        return Some(StaticAbility::ward(TotalCost::from_cost(
            crate::costs::Cost::life(amount),
        )));
    }

    if is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::untap(ObjectFilter::land()),
            "Lands don't untap during their controllers' untap steps".to_string(),
        ));
    }

    None
}

pub(crate) fn parse_filter_dont_untap_during_controllers_untap_steps_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(dont_word_idx) = find_index(&line_words, |word| *word == "dont" || *word == "doesnt")
    else {
        return Ok(None);
    };
    if !word_slice_at_is(&line_words, dont_word_idx + 1, "untap") {
        return Ok(None);
    }

    let tail = line_words.get(dont_word_idx + 2..).unwrap_or_default();
    let has_supported_tail =
        (word_slice_starts_with(tail, &["during", "their", "controllers", "untap"])
            || word_slice_starts_with(tail, &["during", "its", "controllers", "untap"]))
            && word_slice_last_is_any(tail, &["step", "steps"]);
    if !has_supported_tail {
        return Ok(None);
    }

    let dont_token_idx = token_index_for_word_index(tokens, dont_word_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map negated untap subject (clause: '{}')",
            line_words.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..dont_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(" ");
    let mut display = format!("{subject_text} don't untap during their controllers' untap steps");
    if let Some(first) = display
        .chars()
        .next()
        .map(|ch| ch.to_ascii_uppercase().to_string())
    {
        display.replace_range(0..1, &first);
    }

    Ok(Some(StaticAbility::restriction(
        crate::effect::Restriction::untap(filter),
        display,
    )))
}

fn comparison_to_at_least_threshold(comparison: &crate::effect::Comparison) -> Option<u32> {
    match comparison {
        crate::effect::Comparison::GreaterThanOrEqual(value) if *value >= 0 => Some(*value as u32),
        crate::effect::Comparison::GreaterThan(value) if *value >= -1 => Some((*value + 1) as u32),
        crate::effect::Comparison::Equal(value) if *value >= 0 => Some(*value as u32),
        _ => None,
    }
}

fn parse_graveyard_metric_threshold_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::static_abilities::GraveyardCountMetric, u32)>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(&words_all, &["there", "are"])
        && !word_slice_starts_with(&words_all, &["there", "is"])
    {
        return Ok(None);
    }

    let quantified = &tokens[2..];
    let Ok((comparison, used)) = parse_static_quantity_prefix(quantified, false) else {
        return Ok(None);
    };
    let Some(threshold) = comparison_to_at_least_threshold(&comparison) else {
        return Ok(None);
    };

    let mut rest = &quantified[used..];
    if rest
        .first()
        .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        && !rest
            .get(1)
            .is_some_and(|token| token.is_word("type") || token.is_word("types"))
    {
        rest = &rest[1..];
    }
    let rest_words = crate::runtime_backend::token_word_refs(rest);
    let is_card_types = matches!(
        rest_words.as_slice(),
        ["card", "type", "among", "cards", "in", "your", "graveyard"]
            | ["card", "types", "among", "cards", "in", "your", "graveyard"]
    );
    if is_card_types {
        return Ok(Some((
            crate::static_abilities::GraveyardCountMetric::CardTypes,
            threshold,
        )));
    }

    let is_mana_values = matches!(
        rest_words.as_slice(),
        ["mana", "value", "among", "cards", "in", "your", "graveyard"]
            | [
                "mana",
                "values",
                "among",
                "cards",
                "in",
                "your",
                "graveyard"
            ]
    );
    if is_mana_values {
        return Ok(Some((
            crate::static_abilities::GraveyardCountMetric::ManaValues,
            threshold,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_conditional_source_spell_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 10 {
        return Ok(None);
    }

    let Some(this_idx) = word_slice_find_phrase_start(&clause_words, &["this", "spell", "has"])
    else {
        return Ok(None);
    };
    let Some(keyword_word) = clause_words.get(this_idx + 3).copied() else {
        return Ok(None);
    };
    let keyword = match keyword_word {
        "flash" => crate::static_abilities::ConditionalSpellKeywordKind::Flash,
        "cascade" => crate::static_abilities::ConditionalSpellKeywordKind::Cascade,
        _ => return Ok(None),
    };

    if !word_slice_starts_with_at(&clause_words, this_idx + 4, &["as", "long", "as"]) {
        return Ok(None);
    }

    let condition_start = token_index_for_word_index(tokens, this_idx + 7).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map conditional spell keyword condition (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let condition_tokens = trim_commas(&tokens[condition_start..]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let Some((metric, threshold)) = parse_graveyard_metric_threshold_condition(&condition_tokens)?
    else {
        return Ok(None);
    };

    let spec = crate::static_abilities::ConditionalSpellKeywordSpec {
        keyword,
        metric,
        threshold,
    };
    Ok(Some(StaticAbility::conditional_spell_keyword(spec)))
}

pub(crate) fn parse_enters_tapped_with_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_first_is(&clause_words, "this")
        || !word_slice_contains_word(&clause_words, "enters")
        || !word_slice_contains_word(&clause_words, "tapped")
    {
        return Ok(None);
    }
    let tapped_word_idx = find_index(&clause_words, |word| *word == "tapped").ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing tapped keyword in enters-tapped clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let tapped_token_idx =
        token_index_for_word_index(tokens, tapped_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let trailing = &tokens[tapped_token_idx + 1..];
    if trailing.is_empty() {
        return Ok(None);
    }
    let Some(color_choice) = parse_choose_color_as_enters_line(trailing)? else {
        return Ok(None);
    };
    Ok(Some(vec![
        StaticAbility::enters_tapped_ability(),
        color_choice,
    ]))
}

pub(crate) fn parse_damage_not_removed_cleanup_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words.len() != 9 {
        return Ok(None);
    }
    if !word_slice_first_is(&words, "damage") || !word_slice_at_is(&words, 2, "removed") {
        return Ok(None);
    }
    let is_not = word_slice_at_is_any(&words, 1, &["isnt", "isn't"]);
    let matches = is_not
        && word_slice_starts_with_at(
            &words,
            3,
            &["from", "this", "creature", "during", "cleanup", "steps"],
        );
    if matches {
        return Ok(Some(StaticAbility::damage_not_removed_during_cleanup()));
    }
    Ok(None)
}

fn parse_as_enters_choice_subject_words<'a>(
    words: &'a [&'a str],
    this_kind_display_pairs: &[(&str, &'static str)],
) -> Option<(usize, &'static str)> {
    if !word_slice_first_is(words, "as") {
        return None;
    }

    let mut idx = 1usize;
    let display_subject = if word_slice_at_is(words, idx, "this") {
        idx += 1;
        if let Some(pair_idx) = find_index(this_kind_display_pairs, |(kind, _)| {
            word_slice_at_is(words, idx, kind)
        }) {
            let (_, display) = this_kind_display_pairs[pair_idx];
            idx += 1;
            display
        } else {
            "this"
        }
    } else if word_slice_at_is(words, idx, "it") {
        idx += 1;
        "it"
    } else {
        let mut source_end = None;
        let mut scan = idx + 1;
        while scan < words.len() {
            if word_slice_at_is(words, scan, "enters")
                && source_reference_surface_for_words(&words[idx..scan]).is_some()
            {
                source_end = Some(scan);
                break;
            }
            scan += 1;
        }
        idx = source_end?;
        "this"
    };

    if !word_slice_at_is(words, idx, "enters") {
        return None;
    }
    idx += 1;

    if word_slice_starts_with(&words[idx..], &["the", "battlefield"]) {
        idx += 2;
    }

    Some((idx, display_subject))
}

pub(crate) fn parse_choose_basic_land_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_AURA_SUBJECTS)
    else {
        return Ok(None);
    };
    if let Some(consumed) = parse_choose_basic_land_type_phrase_words(&words[idx..]) {
        if idx + consumed == words.len() {
            return Ok(Some(StaticAbility::choose_basic_land_type_as_enters(
                format!("As {display_subject} enters, choose a basic land type."),
            )));
        }
    }
    let Some(consumed) = parse_choose_land_type_phrase_words(&words[idx..]) else {
        return Ok(None);
    };
    if idx + consumed != words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_land_type_as_enters(format!(
        "As {display_subject} enters, choose a land type."
    ))))
}

pub(crate) fn parse_enchanted_land_is_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_enchanted_land_is_chosen_type_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::enchanted_land_is_chosen_type(
        "Enchanted land is the chosen type.".to_string(),
    )))
}

pub(crate) fn parse_choose_card_name_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    if !word_slice_eq(&words[idx..], &["choose", "a", "card", "name"]) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_card_name_as_enters(format!(
        "As {display_subject} enters, choose a card name."
    ))))
}

pub(crate) fn parse_source_is_chosen_type_in_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(display) = parse_source_is_chosen_type_in_addition_line_lexed(tokens) else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::add_chosen_creature_type(
        ObjectFilter::source(),
        display.to_string(),
    )))
}

pub(crate) fn parse_source_is_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let mut is_idx = None;
    for (idx, word) in words.iter().enumerate() {
        if *word == "is" {
            is_idx = Some(idx);
            break;
        }
    }
    let Some(is_idx) = is_idx else {
        return Ok(None);
    };
    let subject_words = &words[..is_idx];
    let is_source =
        is_source_reference_words(subject_words) || word_slice_eq(subject_words, &["it"]);
    if !is_source {
        return Ok(None);
    }

    let display_subject = match subject_words {
        ["this", "creature"] => "This creature",
        ["this", "permanent"] => "This permanent",
        ["this", "card"] => "This card",
        ["this"] => "This",
        ["it"] => "It",
        _ => "This",
    };

    let display = match &words[is_idx + 1..] {
        ["the", "chosen", "color"] => format!("{display_subject} is the chosen color."),
        ["chosen", "color"] => format!("{display_subject} is chosen color."),
        _ => return Ok(None),
    };

    Ok(Some(StaticAbility::set_chosen_color(
        ObjectFilter::source(),
        display,
    )))
}

pub(crate) fn parse_choose_creature_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&words[idx..])?
    else {
        return Ok(None);
    };
    if !excluded_subtypes.is_empty() {
        return Ok(None);
    }
    if idx + consumed != words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_creature_type_as_enters(
        format!("As {display_subject} enters, choose a creature type."),
    )))
}

pub(crate) fn parse_choose_named_options_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let Some(choice_offset) = find_index(&words[idx..], |word| *word == "choose") else {
        return Ok(None);
    };
    let choice_idx = idx + choice_offset;
    let choice_words = &words[choice_idx..];
    if choice_words.len() < 4 || !word_slice_contains_word(choice_words, "or") {
        return Ok(None);
    }
    if parse_choose_color_phrase_words(choice_words)?.is_some()
        || parse_choose_player_phrase_words(choice_words).is_some()
        || parse_choose_basic_land_type_phrase_words(choice_words).is_some()
        || parse_choose_land_type_phrase_words(choice_words).is_some()
        || parse_choose_creature_type_phrase_words(choice_words)?.is_some()
    {
        return Ok(None);
    }

    let mut options = Vec::new();
    let mut current = Vec::new();
    for word in choice_words.iter().skip(1) {
        if *word == "or" {
            if current.is_empty() {
                return Ok(None);
            }
            options.push(current.join(" "));
            current.clear();
        } else {
            current.push((*word).to_string());
        }
    }
    if current.is_empty() {
        return Ok(None);
    }
    options.push(current.join(" "));
    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_named_option_as_enters(
        options,
        format!("As {display_subject} enters, {}.", choice_words.join(" ")),
    )))
}

fn trigger_duplication_tail_matches(words: &[&str]) -> bool {
    matches!(
        words,
        ["it", "triggers", "an", "additional", "time"]
            | ["that", "ability", "triggers", "an", "additional", "time"]
    )
}

fn parse_trigger_duplication_source_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let filter_words = crate::runtime_backend::token_word_refs(&tokens);
    if matches!(
        filter_words.as_slice(),
        ["this", "creature", "or", "an", "emblem", "you", "own"]
            | ["this", "creature", "or", "emblem", "you", "own"]
    ) {
        let mut emblem = ObjectFilter::default();
        emblem.zone = Some(Zone::Command);
        emblem.owner = Some(PlayerFilter::You);

        let mut filter = ObjectFilter::default();
        filter.any_of = vec![ObjectFilter::source(), emblem];
        return Ok(filter);
    }

    parse_object_filter_with_grammar_entrypoint(&tokens, false)
}

fn parse_trigger_duplication_event_matcher(
    tokens: &[OwnedLexToken],
) -> Result<Trigger, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let phrase_words = crate::runtime_backend::token_word_refs(&tokens);

    let build_filter = |subject_tokens: &[OwnedLexToken]| -> Result<ObjectFilter, CardTextError> {
        parse_object_filter_with_grammar_entrypoint(&trim_edge_punctuation(subject_tokens), false)
    };

    if word_slice_starts_with(&phrase_words, &["turning"])
        && word_slice_ends_with(&phrase_words, &["face", "up"])
    {
        if tokens.len() <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing turned-face-up subject in trigger-duplication clause (clause: '{}')",
                phrase_words.join(" ")
            )));
        }
        let filter = build_filter(&tokens[1..tokens.len() - 2])?;
        return Ok(Trigger::turned_face_up(filter));
    }

    if word_slice_starts_with(&phrase_words, &["you", "casting", "or", "copying"]) {
        if tokens.len() <= 4 {
            return Err(CardTextError::ParseError(format!(
                "missing spell subject in trigger-duplication clause (clause: '{}')",
                phrase_words.join(" ")
            )));
        }
        let filter = build_filter(&tokens[4..])?;
        return Ok(Trigger::either(
            Trigger::spell_cast_qualified(
                Some(filter.clone()),
                PlayerFilter::You,
                None,
                None,
                None,
                false,
            ),
            Trigger::spell_copied(Some(filter), PlayerFilter::You),
        ));
    }

    let suffixes: &[(&[&str], fn(ObjectFilter) -> Trigger)] = &[
        (
            &["dealing", "combat", "damage", "to", "a", "player"],
            |filter| Trigger::deals_combat_damage_to_player(filter, PlayerFilter::Any),
        ),
        (
            &[
                "becoming", "the", "target", "of", "a", "spell", "or", "ability",
            ],
            |filter| Trigger::becomes_targeted_object(filter),
        ),
        (&["being", "dealt", "damage"], |filter| {
            Trigger::is_dealt_damage(ChooseSpec::Object(filter))
        }),
        (
            &["entering", "or", "leaving", "the", "battlefield"],
            |filter| {
                Trigger::either(
                    Trigger::enters_battlefield(filter.clone(), None),
                    Trigger::leaves_battlefield(filter),
                )
            },
        ),
        (&["entering", "the", "battlefield"], |filter| {
            Trigger::enters_battlefield(filter, None)
        }),
        (&["leaving", "the", "battlefield"], |filter| {
            Trigger::leaves_battlefield(filter)
        }),
        (&["drawing", "a", "card"], |_filter| {
            Trigger::player_draws_card(PlayerFilter::Any)
        }),
        (&["attacking"], |filter| Trigger::attacks(filter)),
        (&["dying"], |filter| Trigger::dies(filter)),
        (&["entering"], |filter| {
            Trigger::enters_battlefield(filter, None)
        }),
    ];

    for (suffix, build) in suffixes {
        if !word_slice_ends_with(&phrase_words, suffix) || phrase_words.len() <= suffix.len() {
            continue;
        }
        let subject_len = phrase_words.len() - suffix.len();
        if word_slice_eq(suffix, &["drawing", "a", "card"]) {
            let subject_words = &phrase_words[..subject_len];
            if word_slice_eq_any(subject_words, &[&["a", "player"], &["player"]]) {
                return Ok(Trigger::player_draws_card(PlayerFilter::Any));
            }
            if word_slice_eq(subject_words, &["you"]) {
                return Ok(Trigger::player_draws_card(PlayerFilter::You));
            }
            if word_slice_eq_any(subject_words, &[&["an", "opponent"], &["opponent"]]) {
                return Ok(Trigger::player_draws_card(PlayerFilter::Opponent));
            }
        }
        let Some(subject_end_token_idx) = token_index_for_word_index(&tokens, subject_len) else {
            return Err(CardTextError::ParseError(format!(
                "failed to split trigger-duplication subject (clause: '{}')",
                phrase_words.join(" ")
            )));
        };
        let filter = build_filter(&tokens[..subject_end_token_idx])?;
        return Ok(build(filter));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported trigger-duplication cause clause (clause: '{}')",
        phrase_words.join(" ")
    )))
}

fn parse_trigger_duplication_core(
    tokens: &[OwnedLexToken],
) -> Result<Option<(StaticAbility, Option<crate::ConditionExpr>)>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let segments = split_lexed_slices_on_comma(&tokens);
    if segments.len() != 2 {
        return Ok(None);
    }

    let head_tokens = trim_commas(segments[0]);
    let tail_tokens = trim_commas(segments[1]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
    if !trigger_duplication_tail_matches(&tail_words) {
        return Ok(None);
    }

    let head_words = crate::runtime_backend::token_word_refs(&head_tokens);
    if !word_slice_starts_with(&head_words, &["if"]) || head_tokens.len() < 2 {
        return Ok(None);
    }

    let body_tokens = &head_tokens[1..];
    let body_words = crate::runtime_backend::token_word_refs(body_tokens);

    let ability_prefixes: &[&[&str]] = &[
        &["a", "triggered", "ability", "of"],
        &["an", "ability", "of"],
    ];

    let mut parsed: Option<(
        Option<ObjectFilter>,
        Option<Trigger>,
        Option<crate::ConditionExpr>,
    )> = None;

    for prefix in ability_prefixes {
        if !word_slice_starts_with(&body_words, prefix) || body_tokens.len() <= prefix.len() + 1 {
            continue;
        }
        let Some(triggers_idx) = find_index(&body_words, |word| *word == "triggers") else {
            continue;
        };
        if triggers_idx <= prefix.len() {
            continue;
        }

        let condition = if body_words
            .get(triggers_idx + 1..)
            .is_some_and(|tail| word_slice_starts_with(tail, &["while"]))
        {
            Some(parse_static_condition_clause(
                &body_tokens[triggers_idx + 2..],
            )?)
        } else if triggers_idx + 1 == body_tokens.len() {
            None
        } else {
            continue;
        };

        let source_filter =
            parse_trigger_duplication_source_filter(&body_tokens[prefix.len()..triggers_idx])?;
        parsed = Some((Some(source_filter), None, condition));
        break;
    }

    if parsed.is_none()
        && let Some(causes_idx) = find_index(&body_words, |word| *word == "causes")
    {
        let cause_tokens = &body_tokens[..causes_idx];
        let source_body_tokens = &body_tokens[causes_idx + 1..];
        let source_words = crate::runtime_backend::token_word_refs(source_body_tokens);
        for prefix in ability_prefixes {
            if !word_slice_starts_with(&source_words, prefix)
                || source_body_tokens.len() <= prefix.len() + 2
            {
                continue;
            }
            if !word_slice_ends_with(&source_words, &["to", "trigger"]) {
                continue;
            }
            let source_filter = parse_trigger_duplication_source_filter(
                &source_body_tokens[prefix.len()..source_body_tokens.len() - 2],
            )?;
            let event_matcher = parse_trigger_duplication_event_matcher(cause_tokens)?;
            parsed = Some((Some(source_filter), Some(event_matcher), None));
            break;
        }
    }

    Ok(parsed.map(|(source_filter, event_matcher, condition)| {
        (
            StaticAbility::duplicate_matching_triggered_abilities(
                source_filter,
                event_matcher,
                1,
                crate::runtime_backend::token_word_refs(&tokens).join(" "),
            ),
            condition,
        )
    }))
}

pub(crate) fn parse_trigger_duplication_line_ast(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if let Some(spec) = split_as_long_as_condition_prefix_lexed(&tokens) {
        let condition = parse_static_condition_clause(spec.condition_tokens)?;
        let Some(inner) = parse_trigger_duplication_line_ast(spec.remainder_tokens)? else {
            return Ok(None);
        };
        return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(inner),
            condition,
        }));
    }

    let Some((ability, condition)) = parse_trigger_duplication_core(&tokens)? else {
        return Ok(None);
    };
    let ast = StaticAbilityAst::Static(ability);
    Ok(Some(if let Some(condition) = condition {
        StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(ast),
            condition,
        }
    } else {
        ast
    }))
}

pub(crate) fn parse_trigger_suppression_line_ast(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(spec) = parse_trigger_suppression_spec_lexed(&tokens) else {
        return Ok(None);
    };
    let source_filter = match spec.source_filter_tokens {
        Some(source_filter_tokens) => Some(parse_trigger_duplication_source_filter(
            source_filter_tokens,
        )?),
        None => None,
    };
    let event_matcher = parse_trigger_duplication_event_matcher(spec.cause_tokens)?;
    let display = crate::runtime_backend::token_word_refs(&tokens).join(" ");

    Ok(Some(StaticAbilityAst::from(
        StaticAbility::suppress_matching_triggered_abilities(
            source_filter,
            Some(event_matcher),
            display,
        ),
    )))
}

pub(crate) fn parse_double_damage_from_sources_you_control_of_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_double_damage_from_sources_you_control_of_chosen_type_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::double_damage_from_sources_you_control_of_chosen_type(
            "Double all damage that sources you control of the chosen type would deal.".to_string(),
        ),
    ))
}

pub(crate) fn parse_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    let prefix = [
        "if",
        "a",
        "source",
        "you",
        "control",
        "would",
        "deal",
        "damage",
        "to",
        "an",
        "opponent",
        "or",
        "a",
        "permanent",
        "an",
        "opponent",
        "controls",
        "it",
        "deals",
        "that",
        "much",
        "damage",
        "plus",
    ];
    if !word_slice_starts_with(&words, &prefix) {
        return Ok(None);
    }
    let Some(delta_word) = words.get(prefix.len()) else {
        return Ok(None);
    };
    let Some(delta) = parse_number_word_i32(delta_word) else {
        return Ok(None);
    };
    if words.get(prefix.len() + 1) != Some(&"instead") || words.len() != prefix.len() + 2 {
        return Ok(None);
    }

    let display = format!(
        "If a source you control would deal damage to an opponent or a permanent an opponent controls, it deals that much damage plus {delta} instead."
    );
    Ok(Some(StaticAbility::modify_damage_amount_replacement(
        ObjectFilter::default().you_control(),
        Some(PlayerFilter::Opponent),
        Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
        delta,
        display,
    )))
}

pub(crate) fn parse_double_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    if !word_slice_starts_with(&words, &["if"]) {
        return Ok(None);
    }

    let Some(would_idx) = find_window_by(&words, 4, |window| {
        word_slice_eq(window, &["would", "deal", "damage", "to"])
    }) else {
        return Ok(None);
    };
    let Some(source_idx) = word_slice_find_word(&words[..would_idx], "source") else {
        return Ok(None);
    };
    if source_idx <= 1 {
        return Ok(None);
    }

    let Some(tail_idx) = find_window_by(&words[would_idx + 4..], 6, |window| {
        word_slice_eq(window, &["it", "deals", "double", "that", "damage", "to"])
    }) else {
        return Ok(None);
    };
    let replacement_start = would_idx + 4 + tail_idx;
    let damaged_words = &words[would_idx + 4..replacement_start];
    let replacement_target_words = &words[replacement_start + 6..];
    if damaged_words.is_empty()
        || replacement_target_words.len() < 2
        || !word_slice_first_is(replacement_target_words, "that")
        || !word_slice_last_is(replacement_target_words, "instead")
    {
        return Ok(None);
    }

    let (target_player_filter, target_object_filter) =
        parse_damage_amount_replacement_target_filters(damaged_words)?;
    if target_player_filter.is_none() && target_object_filter.is_none() {
        return Ok(None);
    }

    let source_tokens = trim_lexed_commas(
        LexedClause::new(&tokens)
            .between_word_range(1, source_idx)
            .unwrap_or_else(|| LexedClause::new(&tokens).between(tokens.len(), tokens.len()))
            .tokens(),
    );
    let mut source_filter = if source_tokens.is_empty()
        || word_slice_eq_any(&parser_token_word_refs(source_tokens), &[&["a"], &["an"]])
    {
        ObjectFilter::default()
    } else {
        parse_object_filter_lexed(source_tokens, false)?
    };

    match &words[source_idx + 1..would_idx] {
        ["you", "control"] => source_filter = source_filter.you_control(),
        ["an", "opponent", "controls"] | ["opponent", "controls"] => {
            source_filter = source_filter.controlled_by(PlayerFilter::Opponent);
        }
        [] => {}
        _ => return Ok(None),
    }

    let mut display = render_token_slice(&tokens).trim().to_string();
    if !display.ends_with('.') {
        display.push('.');
    }

    Ok(Some(StaticAbility::double_damage_amount_replacement(
        source_filter,
        target_player_filter,
        target_object_filter,
        display,
    )))
}

fn parse_damage_amount_replacement_target_filters(
    words: &[&str],
) -> Result<(Option<PlayerFilter>, Option<ObjectFilter>), CardTextError> {
    let words = strip_leading_word_refs_any(words, &["a", "an"]);
    match words {
        ["you"] => Ok((Some(PlayerFilter::You), None)),
        ["opponent"] => Ok((Some(PlayerFilter::Opponent), None)),
        ["player"] => Ok((Some(PlayerFilter::Any), None)),
        ["enchanted", "player"] => Ok((
            Some(PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted"))),
            None,
        )),
        ["permanent"] => Ok((None, Some(ObjectFilter::permanent()))),
        ["permanent", "or", "player"] | ["permanent", "or", "a", "player"] => {
            Ok((Some(PlayerFilter::Any), Some(ObjectFilter::permanent())))
        }
        ["player", "or", "permanent"] | ["player", "or", "a", "permanent"] => {
            Ok((Some(PlayerFilter::Any), Some(ObjectFilter::permanent())))
        }
        [
            "opponent",
            "or",
            "a",
            "permanent",
            "an",
            "opponent",
            "controls",
        ]
        | ["opponent", "or", "permanent", "an", "opponent", "controls"] => Ok((
            Some(PlayerFilter::Opponent),
            Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
        )),
        _ => Ok((None, None)),
    }
}

pub(crate) fn parse_minimum_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    let prefix = [
        "if",
        "a",
        "red",
        "source",
        "you",
        "control",
        "would",
        "deal",
        "an",
        "amount",
        "of",
        "noncombat",
        "damage",
        "less",
        "than",
    ];
    if !word_slice_starts_with(&words, &prefix) {
        return Ok(None);
    }

    let Some(to_idx) =
        word_slice_find_word(&words[prefix.len()..], "to").map(|offset| prefix.len() + offset)
    else {
        return Ok(None);
    };
    if !word_slice_starts_with_at(&words, to_idx + 1, &["an", "opponent"]) {
        return Ok(None);
    }

    let Some(source_deals_idx) = word_slice_find_phrase_start(
        &words,
        &["that", "source", "deals", "damage", "equal", "to"],
    ) else {
        return Ok(None);
    };
    if source_deals_idx <= to_idx {
        return Ok(None);
    }
    if !word_slice_last_is(&words, "instead") {
        return Ok(None);
    }

    let floor_words = &words[prefix.len()..to_idx];
    let replacement_floor_words = &words[source_deals_idx + 6..words.len() - 1];
    if !damage_floor_value_words_match(floor_words)
        || !damage_floor_value_words_match(replacement_floor_words)
    {
        return Ok(None);
    }

    let display = render_token_slice(&tokens);
    Ok(Some(StaticAbility::minimum_damage_amount_replacement(
        ObjectFilter::default()
            .you_control()
            .with_colors(ColorSet::from_color(Color::Red)),
        Some(PlayerFilter::Opponent),
        None,
        Value::SourcePower,
        true,
        display,
    )))
}

fn damage_floor_value_words_match(words: &[&str]) -> bool {
    word_slice_eq_any(
        words,
        &[
            &["source", "power"],
            &["sources", "power"],
            &["this", "power"],
            &["thiss", "power"],
            &["this", "creature", "power"],
            &["thiss", "creature", "power"],
        ],
    ) || words.len() >= 2 && word_slice_last_is(words, "power")
}

pub(crate) fn parse_enter_as_copy_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    fn parse_added_copy_abilities(
        tokens: &[OwnedLexToken],
        clause_words: &[&str],
        has_word_idx: usize,
    ) -> Result<Vec<crate::ability::Ability>, CardTextError> {
        let ability_start_token_idx = token_index_for_word_index(tokens, has_word_idx)
            .map(|idx| idx + 1)
            .unwrap_or(tokens.len());
        let ability_tokens = trim_commas(&tokens[ability_start_token_idx..]);
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty enters-as-copy ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let (abilities, _choice) =
            parse_granted_abilities_for_gain_clause(&ability_tokens, clause_words, false)?;
        if abilities.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        lower_granted_abilities_ast_to_object_abilities(&abilities)
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_starts_with(&clause_words, &["as"])
        && let Some(enter_idx) =
            find_index(&clause_words, |word| *word == "enter" || *word == "enters")
        && clause_words.get(enter_idx + 1..enter_idx + 8)
            == Some(&["you", "may", "exile", "two", "creature", "cards", "from"])
        && word_slice_at_is(&clause_words, enter_idx + 8, "graveyards")
        && word_slice_find_phrase_start(
            &clause_words,
            &[
                "if", "you", "do", "it", "enters", "as", "a", "copy", "of", "one", "of", "those",
                "cards",
            ],
        )
        .is_some()
        && word_slice_contains_word(&clause_words, "additional")
        && word_slice_contains_word(&clause_words, "+1/+1")
        && word_slice_contains_word(&clause_words, "counters")
        && word_slice_contains_word(&clause_words, "power")
        && word_slice_contains_word(&clause_words, "other")
    {
        return Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
            crate::static_abilities::EnterAsCopyAsEntersSpec {
                filter: ObjectFilter::creature().in_zone(Zone::Graveyard).nontoken(),
                affected_filter: None,
                may: true,
                enters_tapped_if_chosen: false,
                linked_exile_pair: Some(crate::static_abilities::EnterAsCopyLinkedExilePairSpec {
                    counter_type: CounterType::PlusOnePlusOne,
                }),
                copy_source_self: false,
                copy_source_enchanted: false,
                name_override: None,
                added_card_types: Vec::new(),
                removed_supertypes: Vec::new(),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
                set_base_power_toughness: None,
                set_base_power_toughness_from_self: false,
            },
            render_token_slice(tokens).trim().to_string(),
        )));
    }
    if !word_slice_starts_with(&clause_words, &["you", "may", "have"])
        && let Some(enter_idx) =
            find_index(&clause_words, |word| *word == "enter" || *word == "enters")
        && enter_idx > 0
    {
        let mut after_enter = enter_idx + 1;
        if word_slice_at_is(&clause_words, after_enter, "the")
            && word_slice_at_is(&clause_words, after_enter + 1, "battlefield")
        {
            after_enter += 2;
        }
        if word_slice_starts_with(&clause_words[after_enter..], &["as", "a", "copy", "of"]) {
            let filter_end_token_idx =
                token_index_for_word_index(tokens, enter_idx).unwrap_or(tokens.len());
            let affected_tokens = trim_commas(&tokens[..filter_end_token_idx]);
            let affected_filter = parse_object_filter(&affected_tokens, false)?;
            let copy_source_words = &clause_words[after_enter + 4..];
            let (filter, copy_source_self, copy_source_enchanted) =
                if word_slice_starts_with(copy_source_words, &["this"]) {
                    (ObjectFilter::source(), true, false)
                } else if word_slice_starts_with(copy_source_words, &["enchanted"]) {
                    (ObjectFilter::source(), false, true)
                } else {
                    let copy_start_word_idx = after_enter + 4;
                    let copy_start_token_idx =
                        token_index_for_word_index(tokens, copy_start_word_idx)
                            .unwrap_or(tokens.len());
                    let copy_tokens = trim_commas(&tokens[copy_start_token_idx..]);
                    (parse_object_filter(&copy_tokens, false)?, false, false)
                };

            return Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
                crate::static_abilities::EnterAsCopyAsEntersSpec {
                    filter,
                    affected_filter: Some(affected_filter),
                    may: false,
                    enters_tapped_if_chosen: false,
                    linked_exile_pair: None,
                    copy_source_self,
                    copy_source_enchanted,
                    name_override: None,
                    added_card_types: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                    set_base_power_toughness: None,
                    set_base_power_toughness_from_self: false,
                },
                clause_words.join(" "),
            )));
        }
    }

    if clause_words.len() < 11 || !word_slice_starts_with(&clause_words, &["you", "may", "have"]) {
        return Ok(None);
    }

    let mut idx = 3usize;
    let mut named_copy_subject: Option<String> = None;
    if word_slice_at_is(&clause_words, idx, "this") {
        idx += 1;
    } else if let Some(enter_idx) = find_index(&clause_words[idx..], |word| {
        *word == "enter" || *word == "enters"
    }) {
        named_copy_subject = Some(
            clause_words[idx..idx + enter_idx]
                .iter()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => {
                            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
                        }
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        );
        idx += enter_idx;
    } else {
        return Ok(None);
    }
    if clause_words.get(idx).is_some_and(|word| {
        matches!(
            *word,
            "land" | "creature" | "artifact" | "enchantment" | "permanent"
        )
    }) {
        idx += 1;
    }

    if !word_slice_at_is_any(&clause_words, idx, &["enter", "enters"]) {
        return Ok(None);
    }
    idx += 1;

    if word_slice_at_is(&clause_words, idx, "the")
        && word_slice_at_is(&clause_words, idx + 1, "battlefield")
    {
        idx += 2;
    }

    let mut enters_tapped_if_chosen = false;
    if word_slice_at_is(&clause_words, idx, "tapped") {
        enters_tapped_if_chosen = true;
        idx += 1;
    }

    if !word_slice_starts_with_at(&clause_words, idx, &["as", "a", "copy", "of"]) {
        return Ok(None);
    }
    idx += 4;

    let except_idx = find_index(&clause_words, |word| *word == "except");
    let filter_end_word_idx = except_idx.unwrap_or(clause_words.len());
    let filter_tokens = trim_commas(
        LexedClause::new(tokens)
            .between_word_range(idx, filter_end_word_idx)
            .unwrap_or_else(|| LexedClause::new(tokens).between(tokens.len(), tokens.len()))
            .tokens(),
    );
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false)?;

    let mut name_override = None;
    let mut added_card_types = Vec::new();
    let mut removed_supertypes = Vec::new();
    let mut added_subtypes = Vec::new();
    let mut added_abilities = Vec::new();
    let mut set_base_power_toughness = None;
    let mut set_base_power_toughness_from_self = false;
    if let Some(except_idx) = except_idx {
        let tail = &clause_words[except_idx + 1..];
        if tail.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy exception clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        if word_slice_starts_with(tail, &["its", "name", "is"]) {
            let mut name_words = Vec::new();
            for word in &tail[3..] {
                if matches!(*word, "it's" | "it’s" | "and") {
                    break;
                }
                name_words.push(*word);
            }
            if word_slice_eq(&name_words, &["this"]) {
                name_override = named_copy_subject.clone();
            } else if !name_words.is_empty() {
                name_override = Some(name_words.join(" "));
            }
            // Name/legendary exception riders such as Sakashima's do not change
            // the copied object's functional abilities for current engine use.
        } else if word_slice_starts_with(tail, &["it", "has"]) {
            added_abilities = parse_added_copy_abilities(tokens, &clause_words, except_idx + 2)?;
        } else {
            let mut tail_idx = 0usize;
            if word_slice_starts_with(tail, &["it", "isn't", "legendary"])
                || word_slice_starts_with(tail, &["it", "is", "not", "legendary"])
            {
                removed_supertypes.push(crate::types::Supertype::Legendary);
                tail_idx = if word_slice_at_is(tail, 1, "isn't") {
                    3
                } else {
                    4
                };
            }

            let tail_word_offset = tail_idx;
            let tail = &tail[tail_idx..];
            let type_idx = if word_slice_first_is(tail, "its")
                && word_slice_at_is_any(tail, 1, &["a", "an"])
            {
                2usize
            } else if word_slice_first_is(tail, "is") && word_slice_at_is_any(tail, 1, &["a", "an"])
            {
                2usize
            } else if (word_slice_starts_with(tail, &["it", "is"])
                || word_slice_starts_with(tail, &["it", "s"]))
                && word_slice_at_is_any(tail, 2, &["a", "an"])
            {
                3usize
            } else if word_slice_first_is_any(tail, &["it's", "it’s"])
                && word_slice_at_is_any(tail, 1, &["a", "an"])
            {
                2usize
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported enters-as-copy exception clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };

            let mut cursor = type_idx;
            if let Ok((power, toughness)) = parse_pt_modifier(tail[cursor]) {
                set_base_power_toughness = Some((power, toughness));
                cursor += 1;
            }

            let mut parsed_type_or_subtype = false;
            while cursor < tail.len() {
                if let Some(card_type) = parse_card_type(tail[cursor]) {
                    crate::slice_primitives::push_unique(&mut added_card_types, card_type);
                    parsed_type_or_subtype = true;
                    cursor += 1;
                    continue;
                }
                if let Some(subtype) = parse_subtype_word(tail[cursor])
                    .or_else(|| parse_subtype_flexible(tail[cursor]))
                {
                    crate::slice_primitives::push_unique(&mut added_subtypes, subtype);
                    parsed_type_or_subtype = true;
                    cursor += 1;
                    continue;
                }
                break;
            }

            if !parsed_type_or_subtype && set_base_power_toughness.is_none() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported enters-as-copy type '{}' (clause: '{}')",
                    tail[type_idx],
                    clause_words.join(" ")
                )));
            }

            let mut remainder_start = cursor;
            if word_slice_starts_with(
                &tail[remainder_start..],
                &["in", "addition", "to", "its", "other", "types"],
            ) {
                remainder_start += 6;
            }

            if !tail[remainder_start..].is_empty() {
                if word_slice_starts_with_any(
                    &tail[remainder_start..],
                    &[
                        &[
                            "and",
                            "its",
                            "power",
                            "and",
                            "toughness",
                            "are",
                            "equal",
                            "to",
                            "this",
                        ],
                        &[
                            "its",
                            "power",
                            "and",
                            "toughness",
                            "are",
                            "equal",
                            "to",
                            "this",
                        ],
                    ],
                ) {
                    set_base_power_toughness_from_self = true;
                } else if !word_slice_starts_with(&tail[remainder_start..], &["and", "it", "has"]) {
                    if !word_slice_starts_with(&tail[remainder_start..], &["and", "has"]) {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported enters-as-copy exception clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    added_abilities = parse_added_copy_abilities(
                        tokens,
                        &clause_words,
                        except_idx + 1 + tail_word_offset + remainder_start + 1,
                    )?;
                } else {
                    added_abilities = parse_added_copy_abilities(
                        tokens,
                        &clause_words,
                        except_idx + 1 + tail_word_offset + remainder_start + 2,
                    )?;
                }
            }
        }
    }

    Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
        crate::static_abilities::EnterAsCopyAsEntersSpec {
            filter,
            affected_filter: None,
            may: true,
            enters_tapped_if_chosen,
            linked_exile_pair: None,
            copy_source_self: false,
            copy_source_enchanted: false,
            name_override,
            added_card_types,
            added_subtypes,
            added_abilities,
            set_base_power_toughness,
            set_base_power_toughness_from_self,
            removed_supertypes,
        },
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_choose_color_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let Some((consumed, excluded_color_set)) = parse_choose_color_phrase_words(&words[idx..])?
    else {
        return Ok(None);
    };
    if idx + consumed != words.len() {
        return Ok(None);
    }

    let excluded = if let Some(color_set) = excluded_color_set {
        Some(color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice in choose-color clause (clause: '{}')",
                words.join(" ")
            ))
        })?)
    } else {
        None
    };
    let display = match excluded {
        Some(color) => format!(
            "As {display_subject} enters, choose a color other than {}.",
            color.name().to_string()
        ),
        None => format!("As {display_subject} enters, choose a color."),
    };

    Ok(Some(StaticAbility::choose_color_as_enters(
        excluded, display,
    )))
}

pub(crate) fn parse_choose_color_as_becomes_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(&words, &["as", "this"]) {
        return Ok(None);
    }
    let display_subject = match words.get(2).copied() {
        Some("equipment") => "this Equipment",
        Some("aura") => "this Aura",
        Some("permanent") => "this permanent",
        Some("artifact") => "this artifact",
        Some("enchantment") => "this enchantment",
        _ => return Ok(None),
    };
    if words.get(3).copied() != Some("becomes")
        || words.get(4).copied() != Some("attached")
        || words.get(5).copied() != Some("to")
    {
        return Ok(None);
    }
    let Some(choose_idx) = word_slice_find_word(&words, "choose") else {
        return Ok(None);
    };
    if choose_idx <= 6 {
        return Ok(None);
    }
    let Some((consumed, excluded_color_set)) =
        parse_choose_color_phrase_words(&words[choose_idx..])?
    else {
        return Ok(None);
    };
    if choose_idx + consumed != words.len() {
        return Ok(None);
    }
    let excluded = if let Some(color_set) = excluded_color_set {
        Some(color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice in choose-color attached clause (clause: '{}')",
                words.join(" ")
            ))
        })?)
    } else {
        None
    };
    if excluded.is_some() {
        return Err(CardTextError::ParseError(format!(
            "excluded color choices for attachment choices are not supported yet (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(Some(StaticAbility::choose_color_as_becomes_attached(
        format!("As {display_subject} becomes attached, choose a color."),
    )))
}

pub(crate) fn parse_choose_player_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let Some(consumed) = parse_choose_player_phrase_words(&words[idx..]) else {
        return Ok(None);
    };
    if idx + consumed != words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_player_as_enters(format!(
        "As {display_subject} enters, choose a player."
    ))))
}

pub(crate) fn parse_damage_redirect_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() != 19 {
        return Ok(None);
    }
    let matches = word_slice_starts_with(
        &words,
        &[
            "all", "damage", "that", "would", "be", "dealt", "to", "you", "and", "other",
        ],
    ) && word_slice_at_is_any(&words, 10, &["permanents", "permanent"])
        && word_slice_starts_with_at(
            &words,
            11,
            &[
                "you", "control", "is", "dealt", "to", "this", "creature", "instead",
            ],
        );
    if matches {
        return Ok(Some(
            StaticAbility::redirect_damage_from_you_and_other_permanents_to_source(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_damage_redirect_to_source_controller_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.first() != Some(&"if") {
        return Ok(None);
    }

    let Some(would_idx) = find_window_by(&words, 5, |window| {
        word_slice_eq(window, &["would", "deal", "damage", "to", "you"])
    }) else {
        return Ok(None);
    };
    let tail = &words[would_idx + 5..];
    if tail
        != [
            "it",
            "deals",
            "that",
            "damage",
            "to",
            "its",
            "controller",
            "instead",
        ]
    {
        return Ok(None);
    }
    if would_idx <= 1 {
        return Ok(None);
    }

    let source_tokens = trim_lexed_commas(
        LexedClause::new(tokens)
            .between_word_range(1, would_idx)
            .unwrap_or_else(|| LexedClause::new(tokens).between(tokens.len(), tokens.len()))
            .tokens(),
    );
    let source_filter = parse_object_filter_lexed(source_tokens, false)?;
    let mut display = render_token_slice(tokens).trim().to_string();
    if !display.ends_with('.') {
        display.push('.');
    }

    Ok(Some(StaticAbility::redirect_damage_to_source_controller(
        source_filter,
        PlayerFilter::You,
        display,
    )))
}

pub(crate) fn parse_no_more_than_creatures_can_attack_or_block_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    if line_words.len() < 8 || !word_slice_starts_with(&line_words, &["no", "more", "than"]) {
        return Ok(None);
    }

    let Some((maximum, used)) = parse_number(&tokens[3..]) else {
        return Ok(None);
    };

    let tail = crate::runtime_backend::token_word_refs(&tokens[3 + used..]);
    if tail.len() != 5 && tail.len() != 6 {
        return Ok(None);
    }

    let attack_you = tail.len() == 6;
    if !matches!(tail[0], "creature" | "creatures")
        || tail[1] != "can"
        || tail[3 + usize::from(attack_you)] != "each"
        || tail[4 + usize::from(attack_you)] != "combat"
    {
        return Ok(None);
    }

    if attack_you && tail[3] != "you" {
        return Ok(None);
    }

    let ability = match tail[2] {
        "attack" if attack_you => {
            StaticAbility::max_attackers_can_attack_you_each_combat(maximum as usize)
        }
        "attack" => StaticAbility::max_attackers_each_combat(maximum as usize),
        "block" => StaticAbility::max_blockers_each_combat(maximum as usize),
        _ => return Ok(None),
    };
    Ok(Some(ability))
}

pub(crate) fn parse_characteristic_defining_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if split_lexed_sentences(tokens).len() > 1 {
        return Ok(None);
    }

    let line_words = crate::runtime_backend::token_word_refs(tokens);
    let has_pt_axes = word_slice_contains_phrase(&line_words, &["power", "and", "toughness"]);
    if has_pt_axes
        && word_slice_contains_phrase(&line_words, &["equal", "to"])
        && let Some(equal_word_idx) = word_slice_find_phrase_start(&line_words, &["equal", "to"])
    {
        let start_word_idx = equal_word_idx + 2;
        if let Some(start_token_idx) = token_index_for_word_index(tokens, start_word_idx) {
            let mut tail_tokens = &tokens[start_token_idx..];
            while tail_tokens
                .last()
                .is_some_and(|token| token.is_word("respectively") || token.is_period())
            {
                tail_tokens = &tail_tokens[..tail_tokens.len().saturating_sub(1)];
            }
            if !tail_tokens.is_empty() {
                let value =
                    parse_characteristic_defining_stat_value(tail_tokens).ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported characteristic defining P/T value (value: '{}')",
                            crate::runtime_backend::token_word_refs(tail_tokens).join(" ")
                        ))
                    })?;
                return Ok(Some(StaticAbility::characteristic_defining_pt(
                    value.clone(),
                    value,
                )));
            }
        }
    }

    let mut parsed_power: Option<Value> = None;
    let mut parsed_toughness: Option<Value> = None;
    let mut previous_value: Option<Value> = None;
    let mut idx = 0usize;
    while idx < line_words.len() {
        let Some((axis, value_start_word_idx)) =
            parse_characteristic_axis_clause_start(&line_words, idx)
        else {
            idx += 1;
            continue;
        };

        let mut value_end_word_idx = line_words.len();
        let mut next_clause_word_idx = None;
        for and_idx in value_start_word_idx..line_words.len() {
            if line_words[and_idx] != "and" {
                continue;
            }
            if let Some((_next_axis, _)) =
                parse_characteristic_axis_clause_start(&line_words, and_idx + 1)
            {
                value_end_word_idx = and_idx;
                next_clause_word_idx = Some(and_idx + 1);
                break;
            }
        }

        let Some(value_start_token_idx) = token_index_for_word_index(tokens, value_start_word_idx)
        else {
            break;
        };
        let value_end_token_idx = if value_end_word_idx < line_words.len() {
            token_index_for_word_index(tokens, value_end_word_idx).unwrap_or(tokens.len())
        } else {
            tokens.len()
        };
        let value_tokens =
            trim_edge_punctuation(&tokens[value_start_token_idx..value_end_token_idx]);
        if value_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing characteristic defining {} value (line: '{}')",
                axis,
                line_words.join(" ")
            )));
        }

        let value = parse_characteristic_defining_stat_value(&value_tokens)
            .or_else(|| {
                previous_value.as_ref().and_then(|base| {
                    parse_characteristic_defining_relative_value(&value_tokens, base)
                })
            })
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported characteristic defining {} value (value: '{}')",
                    axis,
                    crate::runtime_backend::token_word_refs(&value_tokens).join(" ")
                ))
            })?;

        match axis {
            "power" => parsed_power = Some(value.clone()),
            "toughness" => parsed_toughness = Some(value.clone()),
            _ => {}
        }
        previous_value = Some(value);

        if let Some(next_idx) = next_clause_word_idx {
            idx = next_idx;
        } else {
            break;
        }
    }

    if parsed_power.is_none() && parsed_toughness.is_none() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::characteristic_defining_pt(
        parsed_power.unwrap_or(Value::SourcePower),
        parsed_toughness.unwrap_or(Value::SourceToughness),
    )))
}

fn parse_characteristic_defining_relative_value(
    tokens: &[OwnedLexToken],
    base: &Value,
) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let words = crate::runtime_backend::token_word_refs(&trimmed);
    if !word_slice_starts_with(&words, &["that", "number"]) {
        return None;
    }
    if words.len() == 2 {
        return Some(base.clone());
    }
    if words.len() == 4 && words[2] == "plus" {
        let (amount, used) = parse_number(&trimmed[3..])?;
        if used == trimmed[3..].len() {
            return Some(Value::Add(
                Box::new(base.clone()),
                Box::new(Value::Fixed(amount as i32)),
            ));
        }
    }
    None
}

fn parse_characteristic_axis_clause_start<'a>(
    words: &'a [&'a str],
    idx: usize,
) -> Option<(&'a str, usize)> {
    let is_self_ref = |word: &str| matches!(word, "this" | "thiss" | "its");

    let first = words.get(idx).copied()?;
    if matches!(first, "power" | "toughness")
        && word_slice_at_is(words, idx + 1, "is")
        && word_slice_at_is(words, idx + 2, "equal")
        && word_slice_at_is(words, idx + 3, "to")
    {
        return Some((first, idx + 4));
    }

    if first == "creature"
        && word_slice_at_is_any(words, idx + 1, &["power", "toughness"])
        && word_slice_at_is(words, idx + 2, "is")
        && word_slice_at_is(words, idx + 3, "equal")
        && word_slice_at_is(words, idx + 4, "to")
    {
        return Some((words[idx + 1], idx + 5));
    }

    if !is_self_ref(first) {
        return None;
    }

    if word_slice_at_is_any(words, idx + 1, &["power", "toughness"])
        && word_slice_at_is(words, idx + 2, "is")
        && word_slice_at_is(words, idx + 3, "equal")
        && word_slice_at_is(words, idx + 4, "to")
    {
        return Some((words[idx + 1], idx + 5));
    }

    if word_slice_at_is(words, idx + 1, "creature")
        && word_slice_at_is_any(words, idx + 2, &["power", "toughness"])
        && word_slice_at_is(words, idx + 3, "is")
        && word_slice_at_is(words, idx + 4, "equal")
        && word_slice_at_is(words, idx + 5, "to")
    {
        return Some((words[idx + 2], idx + 6));
    }

    None
}

fn parse_characteristic_defining_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let trimmed_words = crate::runtime_backend::token_word_refs(&trimmed);
    if trimmed_words.is_empty() {
        return None;
    }

    if matches!(
        trimmed_words.as_slice(),
        ["its", "power"]
            | ["this", "power"]
            | ["thiss", "power"]
            | ["its", "creature", "power"]
            | ["this", "creature", "power"]
            | ["thiss", "creature", "power"]
    ) {
        return Some(Value::SourcePower);
    }
    if matches!(
        trimmed_words.as_slice(),
        ["its", "toughness"]
            | ["this", "toughness"]
            | ["thiss", "toughness"]
            | ["its", "creature", "toughness"]
            | ["this", "creature", "toughness"]
            | ["thiss", "creature", "toughness"]
    ) {
        return Some(Value::SourceToughness);
    }

    let mut equal_prefixed = Vec::with_capacity(trimmed.len() + 2);
    equal_prefixed.push(OwnedLexToken::word(
        "equal".to_string(),
        TextSpan::synthetic(),
    ));
    equal_prefixed.push(OwnedLexToken::word("to".to_string(), TextSpan::synthetic()));
    equal_prefixed.extend(trimmed.iter().cloned());

    parse_equal_to_aggregate_filter_value(&equal_prefixed)
        .or_else(|| parse_add_mana_equal_amount_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(&equal_prefixed))
        .or_else(|| parse_characteristic_defining_pt_value(&trimmed))
}

pub(crate) fn parse_characteristic_defining_pt_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty() {
        return None;
    }

    let plus_positions: Vec<usize> = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| (*word == "plus").then_some(idx))
        .collect();
    if plus_positions.is_empty() {
        return parse_characteristic_defining_pt_term(tokens);
    }

    let mut values = Vec::new();
    let mut start_word_idx = 0usize;
    let clause = LexedClause::new(tokens);
    for plus_word_idx in plus_positions {
        values.push(parse_characteristic_defining_pt_term(
            clause
                .between_word_range(start_word_idx, plus_word_idx)?
                .tokens(),
        )?);
        start_word_idx = plus_word_idx + 1;
    }
    values.push(parse_characteristic_defining_pt_term(
        clause.from_word(start_word_idx)?.tokens(),
    )?);

    let mut iter = values.into_iter();
    let mut acc = iter.next()?;
    for value in iter {
        acc = Value::Add(Box::new(acc), Box::new(value));
    }
    Some(acc)
}

pub(crate) fn parse_characteristic_defining_pt_term(tokens: &[OwnedLexToken]) -> Option<Value> {
    if tokens.is_empty() {
        return None;
    }

    if let Some((number, used)) = parse_number(tokens) {
        if tokens.len() == used {
            return Some(Value::Fixed(number as i32));
        }
    }

    let mut start = tokens;
    while start
        .first()
        .is_some_and(|token| token.as_word().is_some_and(is_article))
    {
        start = &start[1..];
    }
    if start.is_empty() {
        return None;
    }

    if token_slice_starts_with(start, &["number", "of"]) {
        start = &start[2..];
    }
    if start.is_empty() {
        return None;
    }

    // "the number of cards in the hand of the opponent with the most cards in hand"
    // (Adamaro, First to Desire)
    if let Some(value) = parse_max_cards_in_hand_value_lexed(start) {
        return Some(value);
    }

    let start_words = crate::runtime_backend::token_word_refs(start);
    if word_slice_starts_with_any(
        &start_words,
        &[
            &["basic", "land", "type", "among"],
            &["basic", "land", "types", "among"],
        ],
    ) {
        let lands_token_idx = token_index_for_word_index(start, 4)?;
        let lands_tokens = trim_commas(&start[lands_token_idx..]);
        if let Ok(filter) = parse_object_filter(&lands_tokens, false) {
            return Some(Value::BasicLandTypesAmong(filter));
        }
    }

    if word_slice_starts_with_any(&start_words, &[&["color", "among"], &["colors", "among"]]) {
        let mut scope_word_idx = 2usize;
        if word_slice_at_is(&start_words, scope_word_idx, "the") {
            scope_word_idx += 1;
        }
        let scope_token_idx = token_index_for_word_index(start, scope_word_idx)?;
        let scope_tokens = trim_commas(&start[scope_token_idx..]);
        if let Ok(filter) = parse_object_filter(&scope_tokens, false) {
            return Some(Value::ColorsAmong(filter));
        }
    }

    if word_slice_starts_with_any(
        &start_words,
        &[&["card", "type", "among"], &["card", "types", "among"]],
    ) {
        let mut scope_word_idx = 3usize;
        if word_slice_at_is(&start_words, scope_word_idx, "the") {
            scope_word_idx += 1;
        }
        let scope_token_idx = token_index_for_word_index(start, scope_word_idx)?;
        let scope_tokens = trim_commas(&start[scope_token_idx..]);
        if let Ok(filter) = parse_object_filter(&scope_tokens, false)
            && filter.zone == Some(Zone::Graveyard)
        {
            let player = match filter.owner.clone() {
                Some(player) => player,
                None if !filter.single_graveyard => PlayerFilter::Any,
                None => PlayerFilter::You,
            };
            return Some(Value::CardTypesInGraveyard(player));
        }
    }

    let filter = parse_object_filter(start, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_shuffle_into_library_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_shuffle_into_library_from_graveyard_line_lexed(tokens) {
        return Ok(Some(StaticAbility::shuffle_into_library_from_graveyard()));
    }

    Ok(None)
}

pub(crate) fn parse_permanents_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_permanents_enter_tapped_line_lexed(tokens) {
        return Ok(Some(StaticAbility::permanents_enter_tapped()));
    }
    Ok(None)
}

pub(crate) fn parse_creatures_entering_dont_cause_abilities_to_trigger_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::creatures_entering_dont_cause_abilities_to_trigger(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_creatures_assign_combat_damage_using_toughness_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    match parse_creatures_assign_combat_damage_using_toughness_line_lexed(tokens) {
        Some(CombatDamageUsingToughnessSubject::ThisCreature) => {
            return Ok(Some(
                StaticAbility::this_creature_assigns_combat_damage_using_toughness(),
            ));
        }
        Some(CombatDamageUsingToughnessSubject::EachCreature) => {
            return Ok(Some(
                StaticAbility::creatures_assign_combat_damage_using_toughness(),
            ));
        }
        Some(CombatDamageUsingToughnessSubject::EachCreatureYouControl) => {
            return Ok(Some(
                StaticAbility::creatures_you_control_assign_combat_damage_using_toughness(),
            ));
        }
        None => {}
    }
    Ok(None)
}

pub(crate) fn parse_players_cant_cycle_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_cant_cycle_line_lexed(tokens) {
        return Ok(Some(StaticAbility::players_cant_cycle()));
    }
    Ok(None)
}

pub(crate) fn parse_starting_life_bonus_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(&words, &["you", "start", "the", "game"]) {
        return Ok(None);
    }
    if !word_slice_contains_all_words(&words, &["additional", "life"]) {
        return Ok(None);
    }
    let mut amount = None;
    for (idx, _token) in tokens.iter().enumerate() {
        if let Some((value, _)) = parse_number(&tokens[idx..]) {
            amount = Some(value);
            break;
        }
    }
    let amount = amount
        .ok_or_else(|| CardTextError::ParseError("missing starting life amount".to_string()))?;
    Ok(Some(StaticAbility::starting_life_bonus(amount as i32)))
}

pub(crate) fn parse_buyback_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(&words, &["buyback", "costs", "cost"]) {
        return Ok(None);
    }
    let (amount, _) = parse_number(&tokens[3..])
        .ok_or_else(|| CardTextError::ParseError("missing buyback reduction amount".to_string()))?;
    if !word_slice_contains_word(&words, "less") {
        return Ok(None);
    }
    Ok(Some(StaticAbility::buyback_cost_reduction(amount)))
}

pub(crate) fn parse_spell_cost_increase_per_target_beyond_first_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let line_start = if word_slice_starts_with(&words, &["this", "spell", "costs"]) {
        0
    } else if let Some(idx) = word_slice_find_phrase_start(&words, &["this", "spell", "costs"]) {
        idx
    } else {
        return Ok(None);
    };
    if !word_slice_contains_word(&words, "more")
        || !word_slice_contains_word(&words, "target")
        || !word_slice_contains_word(&words, "beyond")
    {
        return Ok(None);
    }

    let search_tokens = &tokens[line_start..];
    let costs_idx = find_index(search_tokens, |token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &search_tokens[costs_idx + 1..];
    if let Some((cost, used)) = parse_cost_modifier_mana_cost(amount_tokens)
        && amount_tokens
            .get(used)
            .is_some_and(|token| token.is_word("more"))
    {
        return Ok(Some(
            StaticAbility::cost_increase_mana_cost_per_target_beyond_first(cost),
        ));
    }
    let (amount_value, _) =
        parse_cost_modifier_amount(amount_tokens).unwrap_or((Value::Fixed(1), 0));
    let amount = if let Value::Fixed(v) = amount_value {
        v.max(0) as u32
    } else {
        1
    };

    Ok(Some(StaticAbility::cost_increase_per_target_beyond_first(
        amount,
    )))
}

#[allow(dead_code)]
pub(crate) fn parse_if_this_spell_costs_less_to_cast_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(&words_all, &["if"]) {
        return Ok(None);
    }

    let Some(comma_idx) = find_index(tokens, |t| t.is_comma()) else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(&tokens[1..comma_idx]);
    let tail_tokens = trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
    if !word_slice_starts_with(&tail_words, &["this", "spell", "costs"]) {
        return Ok(None);
    }

    let condition = parse_this_spell_cost_condition(&condition_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let costs_idx = find_index(&tail_tokens, |token: &OwnedLexToken| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = tail_tokens.get(costs_idx + 1..).unwrap_or_default();
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .unwrap_or_else(|| (Value::Fixed(0), 0));
    let used = if used > 0 {
        used
    } else if let Some((_, used)) = parsed_mana_cost {
        used
    } else {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    };
    let remaining_words =
        crate::runtime_backend::token_word_refs(amount_tokens.get(used..).unwrap_or_default());
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::Less)
        || !word_slice_contains_word(&remaining_words, "cast")
    {
        return Ok(None);
    }

    if let Some((reduction, _)) = parsed_mana_cost {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(reduction, condition),
        )));
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(amount_value, condition),
    )))
}

pub(crate) fn parse_if_this_spell_costs_less_to_cast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = split_if_this_spell_costs_line_lexed(tokens) else {
        return Ok(None);
    };

    let condition = parse_this_spell_cost_condition(spec.condition_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;

    let costs_idx = find_index(spec.tail_tokens, |token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = spec.tail_tokens.get(costs_idx + 1..).unwrap_or_default();
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .unwrap_or_else(|| (Value::Fixed(0), 0));
    let used = if used > 0 {
        used
    } else if let Some((_, used)) = parsed_mana_cost {
        used
    } else {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    };
    let remaining_words = parser_token_word_refs(amount_tokens.get(used..).unwrap_or_default());
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::Less)
        || !word_slice_contains_word(&remaining_words, "cast")
    {
        return Ok(None);
    }

    if let Some((reduction, _)) = parsed_mana_cost {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(reduction, condition),
        )));
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(amount_value, condition),
    )))
}

pub(crate) fn parse_this_spell_target_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let w = crate::runtime_backend::token_word_refs(tokens);
    let target_start = if word_slice_starts_with(&w, &["it", "targets"]) {
        2
    } else if word_slice_starts_with(&w, &["this", "spell", "targets"]) {
        3
    } else {
        return None;
    };
    let target_tokens = trim_commas(tokens.get(target_start..).unwrap_or_default());
    if target_tokens.is_empty() {
        return None;
    }
    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if word_slice_starts_with(&target_words, &["you"]) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::You));
    }
    if word_slice_starts_with_any(&target_words, &[&["an", "opponent"], &["opponent"]]) {
        return Some(ThisSpellCostCondition::TargetsPlayer(
            PlayerFilter::Opponent,
        ));
    }
    if word_slice_starts_with_any(&target_words, &[&["a", "player"], &["player"]]) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::Any));
    }
    parse_object_filter(&target_tokens, false)
        .ok()
        .map(ThisSpellCostCondition::TargetsObject)
}

pub(crate) fn parse_this_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let w = crate::runtime_backend::token_word_refs(tokens);
    if w.is_empty() {
        return None;
    }

    // you have 3 or less life
    if w.len() >= 6 && w[0] == "you" && w[1] == "have" && word_slice_contains_word(&w, "life") {
        if let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default()) {
            if w[3] == "or" && w[4] == "less" && w[5] == "life" {
                return Some(ThisSpellCostCondition::YouLifeTotalOrLess(n as i32));
            }
        }
    }
    // your life total is 5 or less
    if w.len() >= 7
        && w[0] == "your"
        && w[1] == "life"
        && w[2] == "total"
        && w[3] == "is"
        && word_slice_ends_with(&w, &["or", "less"])
        && let Some((n, _)) = parse_number(tokens.get(4..).unwrap_or_default())
    {
        return Some(ThisSpellCostCondition::YouLifeTotalOrLess(n as i32));
    }
    if w.as_slice()
        == [
            "your", "life", "total", "is", "less", "than", "your", "starting", "life", "total",
        ]
    {
        return Some(ThisSpellCostCondition::LifeTotalLessThanStarting);
    }

    if word_slice_eq_any(
        &w,
        &[
            &["you", "attacked", "this", "turn"],
            &["youve", "attacked", "this", "turn"],
        ],
    ) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::AttackedThisTurn,
            display: w.join(" "),
        });
    }
    if word_slice_eq_any(
        &w,
        &[
            &["a", "creature", "died", "this", "turn"],
            &["creature", "died", "this", "turn"],
        ],
    ) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::CreatureDiedThisTurn,
            display: w.join(" "),
        });
    }
    if word_slice_eq_any(
        &w,
        &[
            &["you", "gained", "life", "this", "turn"],
            &["youve", "gained", "life", "this", "turn"],
        ],
    ) {
        return Some(ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(1));
    }
    if word_slice_starts_with_any(&w, &[&["youve", "gained"], &["you", "gained"]])
        && w.len() >= 7
        && word_slice_ends_with(&w, &["life", "this", "turn"])
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
        && word_slice_starts_with(&w[3..], &["or", "more"])
    {
        return Some(ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(n));
    }
    if word_slice_eq_any(&w, &[&["its", "night"], &["it", "is", "night"]]) {
        return Some(ThisSpellCostCondition::IsNight);
    }
    if word_slice_eq_any(
        &w,
        &[
            &["its", "bargained"],
            &["it's", "bargained"],
            &["it", "is", "bargained"],
            &["this", "spell", "is", "bargained"],
            &["this", "spell", "was", "bargained"],
        ],
    ) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::ThisSpellPaidLabel("Bargain".to_string()),
            display: w.join(" "),
        });
    }
    if word_slice_eq_any(
        &w,
        &[
            &["youve", "sacrificed", "an", "artifact", "this", "turn"],
            &["you", "sacrificed", "an", "artifact", "this", "turn"],
        ],
    ) {
        return Some(ThisSpellCostCondition::YouSacrificedArtifactThisTurn);
    }
    if word_slice_eq_any(
        &w,
        &[
            &["youve", "committed", "a", "crime", "this", "turn"],
            &["you", "committed", "a", "crime", "this", "turn"],
        ],
    ) {
        return Some(ThisSpellCostCondition::YouCommittedCrimeThisTurn);
    }
    if w.as_slice()
        == [
            "a",
            "creature",
            "left",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ]
    {
        return Some(ThisSpellCostCondition::CreatureLeftBattlefieldUnderYourControlThisTurn);
    }
    if word_slice_starts_with_any(
        &w,
        &[
            &["youve", "cast", "another"],
            &["you've", "cast", "another"],
            &["you", "cast", "another"],
            &["you", "ve", "cast", "another"],
        ],
    ) && word_slice_ends_with(&w, &["this", "turn"])
    {
        if word_slice_contains_word(&w, "instant") || word_slice_contains_word(&w, "sorcery") {
            let mut types = Vec::new();
            if word_slice_contains_word(&w, "instant") {
                types.push(CardType::Instant);
            }
            if word_slice_contains_word(&w, "sorcery") {
                types.push(CardType::Sorcery);
            }
            return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
                count: 1,
                card_types: types,
            });
        }
        return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
            count: 1,
            card_types: Vec::new(),
        });
    }
    if word_slice_starts_with_any(
        &w,
        &[
            &["youve", "cast"],
            &["you've", "cast"],
            &["you", "cast"],
            &["you", "ve", "cast"],
        ],
    ) && word_slice_ends_with(&w, &["this", "turn"])
        && (word_slice_contains_word(&w, "instant") || word_slice_contains_word(&w, "sorcery"))
    {
        let mut types = Vec::new();
        if word_slice_contains_word(&w, "instant") {
            types.push(CardType::Instant);
        }
        if word_slice_contains_word(&w, "sorcery") {
            types.push(CardType::Sorcery);
        }
        return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
            count: 1,
            card_types: types,
        });
    }

    if word_slice_eq_any(&w, &[&["you", "werent", "the", "starting", "player"]]) {
        return Some(ThisSpellCostCondition::NotStartingPlayer);
    }
    if word_slice_eq_any(&w, &[&["a", "creature", "is", "attacking", "you"]]) {
        return Some(ThisSpellCostCondition::CreatureIsAttackingYou);
    }
    if w.as_slice()
        == [
            "a",
            "creature",
            "card",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn",
        ]
    {
        return Some(ThisSpellCostCondition::CreatureCardPutIntoYourGraveyardThisTurn);
    }
    if w.len() >= 11
        && w[0] == "there"
        && w[1] == "are"
        && word_slice_contains_word(&w, "card")
        && word_slice_contains_word(&w, "types")
        && word_slice_contains_word(&w, "graveyard")
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
    {
        return Some(ThisSpellCostCondition::DistinctCardTypesInYourGraveyardOrMore(n));
    }
    if word_slice_starts_with(&w, &["you", "have"])
        && word_slice_ends_with(&w, &["in", "your", "graveyard"])
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
    {
        if word_slice_contains_word(&w, "instant") || word_slice_contains_word(&w, "sorcery") {
            let mut types = Vec::new();
            if word_slice_contains_word(&w, "instant") {
                types.push(CardType::Instant);
            }
            if word_slice_contains_word(&w, "sorcery") {
                types.push(CardType::Sorcery);
            }
            return Some(
                ThisSpellCostCondition::YouHaveCardsOfTypesInYourGraveyardOrMore {
                    count: n,
                    card_types: types,
                },
            );
        }
        return Some(ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(n));
    }
    if w.len() >= 7
        && ((w[0] == "an" && w[1] == "opponent" && w[2] == "has")
            || (w[0] == "opponent" && w[1] == "has"))
    {
        let count_start = if w[0] == "an" { 3 } else { 2 };
        if let Some((n, _)) = parse_number(tokens.get(count_start..).unwrap_or_default()) {
            let tail = &w[count_start + 1..];
            if word_slice_eq_any(
                tail,
                &[
                    &["or", "more", "poison", "counters"],
                    &["or", "more", "poison", "counter"],
                ],
            ) {
                return Some(ThisSpellCostCondition::OpponentHasPoisonCountersOrMore(n));
            }
            if word_slice_eq_any(
                tail,
                &[
                    &["or", "more", "cards", "in", "their", "graveyard"],
                    &["or", "more", "cards", "in", "his", "graveyard"],
                    &["or", "more", "cards", "in", "her", "graveyard"],
                    &["or", "more", "card", "in", "their", "graveyard"],
                ],
            ) {
                return Some(ThisSpellCostCondition::OpponentHasCardsInGraveyardOrMore(n));
            }
        }
    }

    if word_slice_starts_with(&w, &["there", "are", "no"])
        && word_slice_ends_with(&w, &["in", "your", "hand"])
    {
        let filter_tokens = trim_commas(tokens.get(3..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::NoCardsInHandMatching {
                filter,
                display: w.join(" "),
            });
        }
    }
    if ((word_slice_starts_with(&w, &["you", "have", "no", "other", "creature", "cards"])
        && word_slice_contains_phrase(&w, &["or", "if"]))
        || word_slice_starts_with(
            &w,
            &[
                "the", "only", "other", "creature", "cards", "in", "your", "hand", "are", "named",
            ],
        ))
        && let Some(named_idx) = find_index(&w, |word| *word == "named")
        && named_idx + 1 < w.len()
    {
        let name = w[named_idx + 1..].join(" ");
        if !name.is_empty() {
            return Some(ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name));
        }
    }

    if word_slice_starts_with(&w, &["there", "is"])
        && word_slice_ends_with(&w, &["in", "your", "graveyard"])
    {
        let filter_tokens = trim_commas(tokens.get(2..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::CardInYourGraveyardMatching {
                filter,
                display: w.join(" "),
            });
        }
    }

    if w.as_slice()
        == [
            "it", "targets", "a", "spell", "or", "ability", "that", "targets", "a", "creature",
            "you", "control", "with", "power", "7", "or", "greater",
        ]
    {
        let mut protected = ObjectFilter::creature().you_control();
        protected.power = Some(crate::filter::Comparison::GreaterThanOrEqual(7));
        let mut stack_target = ObjectFilter::default();
        stack_target.zone = Some(Zone::Stack);
        stack_target.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        stack_target.targets_object = Some(Box::new(protected));
        return Some(ThisSpellCostCondition::TargetsObject(stack_target));
    }

    if let Some(target_condition) = parse_this_spell_target_condition(tokens) {
        return Some(target_condition);
    }

    // an opponent has no cards in hand
    if word_slice_eq_any(
        &w,
        &[
            &["an", "opponent", "has", "no", "cards", "in", "hand"],
            &["opponent", "has", "no", "cards", "in", "hand"],
        ],
    ) {
        return Some(ThisSpellCostCondition::OpponentHasNoCardsInHand);
    }

    // an opponent controls seven or more lands
    if w.len() >= 7 && w[0] == "an" && w[1] == "opponent" && w[2] == "controls" {
        if let Some((n, _)) = parse_number(tokens.get(3..).unwrap_or_default()) {
            let tail = &w[4..];
            if word_slice_eq_any(tail, &[&["or", "more", "lands"], &["or", "more", "land"]]) {
                return Some(ThisSpellCostCondition::OpponentControlsLandsOrMore(n));
            }
        }
    }

    // an opponent controls at least four more creatures than you
    if w.len() >= 10
        && w[0] == "an"
        && w[1] == "opponent"
        && w[2] == "controls"
        && w[3] == "at"
        && w[4] == "least"
    {
        if let Some((n, _)) = parse_number(tokens.get(5..).unwrap_or_default()) {
            let tail = &w[6..];
            if word_slice_eq_any(
                tail,
                &[
                    &["more", "creatures", "than", "you"],
                    &["more", "creature", "than", "you"],
                ],
            ) {
                return Some(
                    ThisSpellCostCondition::OpponentControlsAtLeastNMoreCreaturesThanYou(n),
                );
            }
        }
    }

    // there are ten or more creature cards total in all graveyards
    if w.len() >= 12 && w[0] == "there" && w[1] == "are" {
        if let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default()) {
            let tail = &w[3..];
            if tail
                == [
                    "or",
                    "more",
                    "creature",
                    "cards",
                    "total",
                    "in",
                    "all",
                    "graveyards",
                ]
            {
                return Some(ThisSpellCostCondition::TotalCreatureCardsInAllGraveyardsOrMore(n));
            }
        }
    }

    // an opponent cast two or more spells this turn
    if w.len() >= 9
        && ((w[0] == "an" && w[1] == "opponent" && w[2] == "cast")
            || (w[0] == "opponent" && w[1] == "cast"))
    {
        let count_start = if w[0] == "an" { 3 } else { 2 };
        if let Some((n, _)) = parse_number(tokens.get(count_start..).unwrap_or_default()) {
            let tail = &w[count_start + 1..];
            if word_slice_eq_any(
                tail,
                &[
                    &["or", "more", "spells", "this", "turn"],
                    &["or", "more", "spell", "this", "turn"],
                ],
            ) {
                return Some(ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(n));
            }
        }
    }

    // an opponent has drawn four or more cards this turn
    if w.len() >= 10
        && ((w[0] == "an" && w[1] == "opponent" && w[2] == "has" && w[3] == "drawn")
            || (w[0] == "opponent" && w[1] == "has" && w[2] == "drawn"))
    {
        let count_start = if w[0] == "an" { 4 } else { 3 };
        if let Some((n, _)) = parse_number(tokens.get(count_start..).unwrap_or_default()) {
            let tail = &w[count_start + 1..];
            if word_slice_eq_any(
                tail,
                &[
                    &["or", "more", "cards", "this", "turn"],
                    &["or", "more", "card", "this", "turn"],
                ],
            ) {
                return Some(ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(n));
            }
        }
    }

    // you've been dealt damage by two or more creatures this turn
    if word_slice_starts_with_any(
        &w,
        &[
            &["youve", "been", "dealt", "damage", "by"],
            &["you", "have", "been", "dealt", "damage", "by"],
        ],
    ) && w.len() >= 11
    {
        let count_start = if w[0] == "youve" { 5 } else { 6 };
        if let Some((n, _)) = parse_number(tokens.get(count_start..).unwrap_or_default()) {
            let tail = &w[count_start + 1..];
            if word_slice_eq_any(
                tail,
                &[
                    &["or", "more", "creatures", "this", "turn"],
                    &["or", "more", "creature", "this", "turn"],
                ],
            ) {
                return Some(
                    ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(n),
                );
            }
        }
    }

    if word_slice_starts_with(
        &w,
        &[
            "you",
            "dealt",
            "combat",
            "damage",
            "to",
            "a",
            "player",
            "this",
            "turn",
            "with",
            "an",
            "assassin",
            "or",
            "commander",
        ],
    ) {
        return Some(
            ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                Subtype::Assassin,
            ),
        );
    }

    if let Some(condition_expr) = parse_conjoined_this_spell_cost_condition(tokens) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: condition_expr,
            display: w.join(" "),
        });
    }

    if let Ok(condition_expr) = parse_static_condition_clause(tokens) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: condition_expr,
            display: w.join(" "),
        });
    }

    None
}

fn parse_conjoined_this_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let and_positions = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| (*word == "and").then_some(idx))
        .collect::<Vec<_>>();
    for and_word_idx in and_positions {
        let and_token_idx = token_index_for_word_index(tokens, and_word_idx)?;
        let left_tokens = trim_commas(&tokens[..and_token_idx]);
        let right_tokens = trim_commas(&tokens[and_token_idx + 1..]);
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        let Ok(left) = parse_static_condition_clause(&left_tokens) else {
            continue;
        };
        let right = parse_conjoined_this_spell_cost_condition(&right_tokens)
            .or_else(|| parse_static_condition_clause(&right_tokens).ok());
        if let Some(right) = right {
            return Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)));
        }
    }
    None
}

pub(crate) fn parse_trailing_this_spell_cost_condition(
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<crate::static_abilities::ThisSpellCostCondition>, CardTextError> {
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(if_idx) = find_index(&remaining_words, |word| *word == "if") else {
        return Ok(None);
    };
    let condition_token_idx =
        token_index_for_word_index(remaining_tokens, if_idx + 1).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map this-spell cost condition (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let condition_tokens = trim_commas(&remaining_tokens[condition_token_idx..]);
    if condition_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let Some(condition) = parse_this_spell_cost_condition(&condition_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    Ok(Some(condition))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CostModifierDirection {
    Less,
    More,
}

fn parse_cost_modifier_direction(words: &[&str]) -> Option<CostModifierDirection> {
    match (
        word_slice_contains_word(words, "less"),
        word_slice_contains_word(words, "more"),
    ) {
        (true, false) => Some(CostModifierDirection::Less),
        (false, true) => Some(CostModifierDirection::More),
        _ => None,
    }
}

fn parse_cost_modifier_target_spec(
    target_tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<Box<ObjectFilter>>), CardTextError> {
    let target_words = crate::runtime_backend::token_word_refs(target_tokens);
    if word_slice_starts_with(&target_words, &["you"]) {
        return Ok((Some(PlayerFilter::You), None));
    }
    if word_slice_starts_with_any(&target_words, &[&["opponent"], &["opponents"]]) {
        return Ok((Some(PlayerFilter::Opponent), None));
    }
    if word_slice_starts_with_any(&target_words, &[&["player"], &["players"]]) {
        return Ok((Some(PlayerFilter::Any), None));
    }

    Ok((
        None,
        Some(Box::new(parse_object_filter(target_tokens, false)?)),
    ))
}

pub(crate) fn parse_cost_modifier_prefix_condition(
    tokens: &[OwnedLexToken],
    spells_token_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    let subject_end = spells_token_idx.min(tokens.len());
    let head_tokens = &tokens[..subject_end];

    if token_slice_starts_with(tokens, &["during", "turns", "other", "than", "yours"]) {
        let subject_start = find_index(head_tokens, |token| token.is_comma())
            .map(|idx| idx + 1)
            .unwrap_or(5);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }

    if token_slice_starts_with(tokens, &["during", "your", "turn"]) {
        let subject_start = find_index(head_tokens, |token| token.is_comma())
            .map(|idx| idx + 1)
            .unwrap_or(3);
        return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
    }

    if token_slice_starts_with(tokens, &["as", "long", "as"]) {
        let subject_start = find_index(head_tokens, |token| token.is_comma())
            .map(|idx| idx + 1)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        if subject_start <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        let condition = match parse_static_condition_clause(&condition_tokens) {
            Ok(condition) => condition,
            Err(_) => {
                parse_source_tap_status_condition_lexed(&condition_tokens).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported static condition clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(&condition_tokens).join(" ")
                    ))
                })?
            }
        };
        return Ok((Some(condition), subject_start));
    }

    Ok((None, 0))
}

pub(crate) fn parse_spells_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let Some(spells_token_idx) = find_index(tokens, |token| {
        token.is_word("spell") || token.is_word("spells")
    }) else {
        return Ok(None);
    };

    if word_slice_contains_word(&clause_words, "first")
        && word_slice_contains_word(&clause_words, "each")
        && word_slice_contains_word(&clause_words, "turn")
        && clause_words
            .iter()
            .any(|word| *word == "cost" || *word == "costs")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported first-spell-each-turn cost modifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (prefix_condition, subject_start) =
        parse_cost_modifier_prefix_condition(tokens, spells_token_idx)?;
    if subject_start > spells_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[subject_start..spells_token_idx]);
    let is_this_spell = is_this_subject_reference_lexed(&subject_tokens);

    let mut cost_token_idx = None;
    for idx in spells_token_idx + 1..tokens.len() {
        if !tokens[idx].is_word("cost") && !tokens[idx].is_word("costs") {
            continue;
        }
        let amount_tokens = &tokens[idx + 1..];
        let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
        if parsed_amount.is_some() || parsed_mana_cost.is_some() {
            cost_token_idx = Some(idx);
            break;
        }
    }
    let Some(cost_token_idx) = cost_token_idx else {
        return Ok(None);
    };
    if cost_token_idx <= spells_token_idx {
        return Ok(None);
    }

    let mut filter = if is_this_spell {
        ObjectFilter::default()
    } else {
        parse_spell_filter_with_grammar_entrypoint(&subject_tokens)
    };

    let between_tokens = &tokens[spells_token_idx + 1..cost_token_idx];
    let between_words = crate::runtime_backend::token_word_refs(between_tokens);
    if !is_this_spell {
        for (idx, token) in between_tokens.iter().enumerate() {
            if !token.is_word("spell") && !token.is_word("spells") {
                continue;
            }
            let mut start = idx;
            while start > 0 {
                if between_tokens[start - 1].is_word("and")
                    || between_tokens[start - 1].is_word("or")
                    || between_tokens[start - 1].is_comma()
                {
                    break;
                }
                start -= 1;
            }
            let descriptor_tokens = trim_commas(&between_tokens[start..idx]);
            if descriptor_tokens.is_empty() {
                continue;
            }
            let extra_filter = parse_spell_filter_with_grammar_entrypoint(
                strip_relative_target_clause(&descriptor_tokens),
            );
            if spell_filter_has_identity(&extra_filter) {
                merge_spell_filters(&mut filter, extra_filter);
            }
        }
        let between_filter = parse_spell_filter_with_grammar_entrypoint(
            strip_relative_target_clause(between_tokens),
        );
        if spell_filter_has_identity(&between_filter) {
            merge_spell_filters(&mut filter, between_filter);
        }
        if word_slice_contains_phrase(&between_words, &["you", "cast"]) {
            filter.cast_by = Some(PlayerFilter::You);
        }
        if word_slice_contains_phrase(&between_words, &["from", "your", "graveyard"]) {
            filter.zone = Some(Zone::Graveyard);
            filter.owner = Some(PlayerFilter::You);
        }
        if between_words
            .iter()
            .any(|word| *word == "opponent" || *word == "opponents")
            && between_words
                .iter()
                .any(|word| *word == "cast" || *word == "casts")
        {
            filter.cast_by = Some(PlayerFilter::Opponent);
        }
        let mut targets_idx = None;
        for (idx, token) in between_tokens.iter().enumerate() {
            if token.is_word("target") || token.is_word("targets") {
                if idx > 0 && between_tokens[idx - 1].is_word("that") {
                    targets_idx = Some(idx);
                    break;
                }
            }
        }
        if let Some(targets_idx) = targets_idx {
            let target_tokens = &between_tokens[targets_idx + 1..];
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing target in spells-cost modifier clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let (target_player, target_object) = parse_cost_modifier_target_spec(target_tokens)?;
            filter.targets_player = target_player;
            filter.targets_object = target_object;
        }
    }

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, mut parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (mut amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or_else(|| {
            if let Some((_, used)) = &parsed_mana_cost {
                (Value::Fixed(1), *used)
            } else {
                (Value::Fixed(1), 0)
            }
        });
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(direction) = parse_cost_modifier_direction(&remaining_words) else {
        return Ok(None);
    };

    if let Some(dynamic_value) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        // Wording like "{G} less for each green creature you control" is still a dynamic
        // reduction even though the printed amount is a colored symbol. Model as a generic
        // dynamic reduction so the clause remains playable.
        let multiplier = parsed_amount
            .as_ref()
            .and_then(|(value, _)| match value {
                Value::Fixed(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(1);
        if parsed_mana_cost.is_some() {
            parsed_mana_cost = None;
        }
        amount_value = scale_dynamic_cost_modifier_value(dynamic_value, multiplier);
    } else if parsed_amount.is_none() && parsed_mana_cost.is_none() {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    }

    // Handle trailing "where X is ..." clauses, e.g.
    // "This spell costs {X} less to cast, where X is the number of differently named lands you control."
    if word_slice_contains_phrase(&remaining_words, &["where", "x", "is"]) {
        let clause = clause_words.join(" ");
        let where_word_idx =
            word_slice_find_phrase_start(&remaining_words, &["where", "x", "is"]).unwrap_or(0);
        let where_token_idx = token_index_for_word_index(remaining_tokens, where_word_idx)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map where-x clause in spells-cost modifier (clause: '{clause}')"
                ))
            })?;
        let where_tokens = trim_commas(&remaining_tokens[where_token_idx..]);
        let x_value = parse_value_binding_clause(&where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in spells-cost modifier (clause: '{clause}')"
            ))
        })?;
        if !value_contains_unbound_x(&amount_value) {
            return Err(CardTextError::ParseError(format!(
                "missing where-x clause in spells-cost modifier (clause: '{clause}')"
            )));
        }
        amount_value = replace_unbound_x_with_value(amount_value, &x_value, &clause)?;
    }
    if direction == CostModifierDirection::Less
        && let Some(cap) = parse_cost_reduction_cap(remaining_tokens)
    {
        amount_value = Value::Min(Box::new(amount_value), Box::new(Value::Fixed(cap)));
    }

    if !is_this_spell {
        parse_trailing_targets_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
    }

    let this_spell_condition = if is_this_spell {
        if let Some(condition) =
            parse_trailing_this_spell_cost_condition(remaining_tokens, &clause_words)?
        {
            condition
        } else if let Some(prefix) = &prefix_condition {
            match prefix {
                crate::ConditionExpr::YourTurn => {
                    crate::static_abilities::ThisSpellCostCondition::YourTurn
                }
                crate::ConditionExpr::Not(inner)
                    if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn) =>
                {
                    crate::static_abilities::ThisSpellCostCondition::NotYourTurn
                }
                other => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported leading this-spell cost condition (clause: '{}'; condition: {other:?})",
                        clause_words.join(" ")
                    )));
                }
            }
        } else {
            crate::static_abilities::ThisSpellCostCondition::Always
        }
    } else {
        crate::static_abilities::ThisSpellCostCondition::Always
    };

    let non_this_condition = if is_this_spell {
        None
    } else {
        prefix_condition.clone()
    };

    if direction == CostModifierDirection::Less {
        // "This spell costs {N} less to cast" is a self-only modifier that should not
        // apply from the permanent on the battlefield after it resolves.
        if is_this_spell && parsed_mana_cost.is_none() {
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount_value,
                    this_spell_condition,
                ),
            )));
        }
        if is_this_spell && let Some((cost, _)) = parsed_mana_cost.clone() {
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReductionManaCost::new(
                    cost,
                    this_spell_condition,
                ),
            )));
        }
        if let Some((cost, _)) = parsed_mana_cost {
            let mut ability = crate::static_abilities::CostReductionManaCost::new(filter, cost);
            if let Some(condition) = non_this_condition.clone() {
                ability = ability.with_condition(condition);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        let mut ability = crate::static_abilities::CostReduction::new(filter, amount_value);
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    if let Some((cost, _)) = parsed_mana_cost {
        let mut ability = crate::static_abilities::CostIncreaseManaCost::new(filter, cost);
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    let mut ability = crate::static_abilities::CostIncrease::new(filter, amount_value);
    if let Some(condition) = non_this_condition.clone() {
        ability = ability.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(ability)))
}

pub(crate) fn parse_spell_and_player_activated_ability_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(and_idx) = find_window_by(tokens, 2, |window| {
        token_slice_starts_with(window, &["and", "abilities"])
    }) else {
        return Ok(None);
    };

    let left_tokens = trim_commas(&tokens[..and_idx]);
    let right_tokens = trim_commas(&tokens[and_idx + 1..]);
    let Some(spell_cost_ability) = parse_spells_cost_modifier_line(&left_tokens)? else {
        return Ok(None);
    };
    let Some(mut activated_cost_ability) =
        parse_player_activated_ability_cost_modifier_clause(&right_tokens)?
    else {
        return Ok(None);
    };

    if let Some(spells_idx) = find_index(tokens, |token| {
        token.is_word("spell") || token.is_word("spells")
    }) {
        let (prefix_condition, _) = parse_cost_modifier_prefix_condition(tokens, spells_idx)?;
        if let Some(condition) = prefix_condition {
            activated_cost_ability = activated_cost_ability.with_condition(condition);
        }
    }

    Ok(Some(vec![spell_cost_ability, activated_cost_ability]))
}

fn parse_cycling_cost_alternative_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 8 {
        return Ok(None);
    }

    let (condition, body_start_word_idx) =
        if word_slice_starts_with(&clause_words, &["as", "long", "as"]) {
            let Some(body_rel_idx) = crate::runtime_backend::lexer::word_slice_find_phrase_start(
                &clause_words[3..],
                &["you", "may", "pay"],
            ) else {
                return Ok(None);
            };
            let body_word_idx = body_rel_idx + 3;
            let condition_start = token_index_for_word_index(tokens, 3).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map cycling-cost alternative condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            let condition_end =
                token_index_for_word_index(tokens, body_word_idx).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unable to map cycling-cost alternative body (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?;
            let condition_tokens = trim_commas(&tokens[condition_start..condition_end]);
            let condition = parse_static_condition_clause(&condition_tokens)?;
            (Some(condition), body_word_idx)
        } else {
            (None, 0)
        };

    let body_words = &clause_words[body_start_word_idx..];
    if !word_slice_starts_with(body_words, &["you", "may", "pay"]) {
        return Ok(None);
    }
    let Some(rather_rel_idx) = crate::runtime_backend::lexer::word_slice_find_phrase_start(
        body_words,
        &["rather", "than", "pay", "cycling", "costs"],
    ) else {
        return Ok(None);
    };
    let rather_word_idx = body_start_word_idx + rather_rel_idx;
    let pay_token_idx =
        token_index_for_word_index(tokens, body_start_word_idx + 2).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map cycling-cost alternative cost (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let cost_start = pay_token_idx + 1;
    let cost_end = token_index_for_word_index(tokens, rather_word_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cycling-cost alternative tail (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let replacement_mana_cost = if cost_end <= cost_start {
        ManaCost::new()
    } else {
        let replacement_cost_tokens = trim_commas(&tokens[cost_start..cost_end]);
        let replacement_total_cost = parse_activation_cost(&replacement_cost_tokens)?;
        if replacement_total_cost.has_non_mana_costs() {
            return Err(CardTextError::ParseError(format!(
                "unsupported non-mana cycling alternative cost (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        replacement_total_cost.mana_cost().cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing cycling alternative mana cost (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    let mut filter = ObjectFilter::default().with_ability_marker("cycling");
    filter.zone = Some(Zone::Hand);
    let display = format!(
        "You may pay {} rather than pay cycling costs",
        replacement_mana_cost.to_oracle()
    );
    let mut ability =
        StaticAbility::replace_activated_ability_mana_cost(filter, replacement_mana_cost, display);
    if let Some(condition) = condition {
        ability = ability.with_condition(condition);
    }
    Ok(Some(ability))
}

fn parse_player_activated_ability_cost_modifier_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 7 || !word_slice_first_is(&clause_words, "abilities") {
        return Ok(None);
    }

    let Some(activate_idx) = find_index(&clause_words, |word| {
        *word == "activate" || *word == "activates"
    }) else {
        return Ok(None);
    };
    let activator_words = &clause_words[1..activate_idx];
    let activator = match activator_words {
        ["you"] => PlayerFilter::You,
        ["your", "opponents"] | ["opponents"] => PlayerFilter::Opponent,
        _ => return Ok(None),
    };

    let Some(cost_idx) = find_index(&clause_words[activate_idx + 1..], |word| {
        *word == "cost" || *word == "costs"
    })
    .map(|idx| idx + activate_idx + 1) else {
        return Ok(None);
    };
    let cost_token_idx = crate::runtime_backend::token_index_for_word_index(tokens, cost_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map activated-ability cost modifier amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (increase, used) = if let Some((mana_cost, used)) = parsed_mana_cost {
        (TotalCost::mana(mana_cost), used)
    } else if let Some((Value::Fixed(amount), used)) = parsed_amount {
        if amount < 0 {
            return Ok(None);
        }
        let generic = amount.min(u8::MAX as i32) as u8;
        (
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(generic)])),
            used,
        )
    } else {
        return Ok(None);
    };
    let remaining_words = crate::runtime_backend::token_word_refs(&amount_tokens[used..]);
    if parse_cost_modifier_direction(&remaining_words) != Some(CostModifierDirection::More)
        || !word_slice_contains_phrase(&remaining_words, &["to", "activate"])
    {
        return Ok(None);
    }

    let non_mana_only =
        word_slice_contains_phrase(&remaining_words, &["unless", "theyre", "mana", "abilities"])
            || word_slice_contains_phrase(
                &remaining_words,
                &["unless", "they're", "mana", "abilities"],
            );
    Ok(Some(
        StaticAbility::increase_activated_ability_costs_for_activator(
            activator,
            increase,
            non_mana_only,
        ),
    ))
}

fn strip_relative_target_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(target_clause_idx) = find_window_by(tokens, 2, |window| {
        token_slice_first_is(window, "that")
            && token_slice_at_is_any(window, 1, &["target", "targets"])
    }) else {
        return tokens;
    };

    &tokens[..target_clause_idx]
}

pub(crate) fn parse_trailing_targets_condition_in_cost_modifier(
    filter: &mut ObjectFilter,
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(if_word_idx) = find_index(&remaining_words, |word| *word == "if") else {
        return Ok(());
    };
    let condition_words = &remaining_words[if_word_idx..];
    if condition_words.len() < 4
        || condition_words[0] != "if"
        || condition_words[1] != "it"
        || (condition_words[2] != "targets" && condition_words[2] != "target")
    {
        return Ok(());
    }

    let target_word_idx = if_word_idx + 3;
    let target_token_idx = token_index_for_word_index(remaining_tokens, target_word_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map trailing target condition in spells-cost modifier (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_tokens = &remaining_tokens[target_token_idx..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in trailing spells-cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (targets_player, targets_object) = parse_cost_modifier_target_spec(target_tokens)?;
    filter.targets_player = targets_player;
    filter.targets_object = targets_object;
    Ok(())
}

pub(crate) fn parse_flashback_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some((kind, consumed)) = parse_alternative_cast_words(&clause_words) else {
        return Ok(None);
    };
    if clause_words.len() < consumed + 5 {
        return Ok(None);
    }
    if clause_words.get(consumed).copied() != Some("costs") {
        return Ok(None);
    }
    let cost_idx = rfind_index(tokens, |token| {
        token.is_word("cost") || token.is_word("costs")
    });
    let Some(cost_idx) = cost_idx else {
        return Ok(None);
    };
    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::runtime_backend::token_word_refs(remaining_tokens);
    let Some(direction) = parse_cost_modifier_direction(&remaining_words) else {
        return Ok(None);
    };
    if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing flashback cost modifier amount".to_string(),
        ));
    }

    let mut filter = ObjectFilter::default();
    filter.alternative_cast = Some(kind);
    if word_slice_contains_phrase(&clause_words, &["you", "pay"]) {
        filter.cast_by = Some(PlayerFilter::You);
    } else if word_slice_contains_any_phrase(
        &clause_words,
        &[
            &["your", "opponents", "pay"],
            &["opponents", "pay"],
            &["opponent", "pays"],
        ],
    ) {
        filter.cast_by = Some(PlayerFilter::Opponent);
    }

    if direction == CostModifierDirection::Less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

pub(crate) fn parse_equip_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 6 || !word_slice_first_is(&clause_words, "equip") {
        return Ok(None);
    }
    if clause_words.get(1).copied() != Some("costs") {
        return Ok(None);
    }
    let Some(cost_idx) = rfind_index(tokens, |token| {
        token.is_word("cost") || token.is_word("costs")
    }) else {
        return Ok(None);
    };

    let amount_tokens = &tokens[cost_idx + 1..];
    let Some((amount_value, used)) = parse_cost_modifier_amount(amount_tokens) else {
        return Ok(None);
    };
    let Value::Fixed(amount) = amount_value else {
        return Ok(None);
    };
    if amount < 0 {
        return Ok(None);
    }

    let remaining_words = crate::runtime_backend::token_word_refs(&amount_tokens[used..]);
    let Some(direction) = parse_cost_modifier_direction(&remaining_words) else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default().with_ability_marker("equip");
    if word_slice_contains_phrase(&clause_words, &["you", "pay"]) {
        filter.controller = Some(PlayerFilter::You);
    } else if word_slice_contains_any_phrase(
        &clause_words,
        &[&["your", "opponents", "pay"], &["opponents", "pay"]],
    ) {
        filter.controller = Some(PlayerFilter::Opponent);
    }

    if direction == CostModifierDirection::Less {
        let amount_text = format!("{{{amount}}}");
        let display = if filter.controller == Some(PlayerFilter::Opponent) {
            format!("Equip costs your opponents pay cost {amount_text} less")
        } else {
            format!("Equip costs you pay cost {amount_text} less")
        };
        return Ok(Some(
            StaticAbility::reduce_activated_ability_costs_with_display(
                filter,
                amount as u32,
                None,
                display,
            ),
        ));
    }

    let increase = TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(
        amount.min(u8::MAX as i32) as u8,
    )]));
    Ok(Some(StaticAbility::increase_activated_ability_costs(
        filter, increase,
    )))
}

pub(crate) fn parse_foretelling_cards_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 7 {
        return Ok(None);
    }
    if !word_slice_starts_with(
        &clause_words,
        &["foretelling", "cards", "from", "your", "hand", "costs"],
    ) {
        return Ok(None);
    }

    let has_any_players_turn = word_slice_contains_any_phrase(
        &clause_words,
        &[
            &["on", "any", "players", "turn"],
            &["on", "any", "player", "turn"],
            &["on", "any", "player", "s", "turn"],
        ],
    );
    if parse_cost_modifier_direction(&clause_words) != Some(CostModifierDirection::Less)
        || !has_any_players_turn
    {
        return Ok(None);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported foretelling cost modifier clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_cost_modifier_amount(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    if let Some((amount, used)) = parse_number(tokens) {
        return Some((Value::Fixed(amount as i32), used));
    }

    let first_token = tokens.first()?;
    let group = mana_pips_from_token(first_token)?;
    if group.len() != 1 {
        return None;
    }
    let symbol = group[0];
    if let ManaSymbol::Generic(amount) = symbol {
        return Some((Value::Fixed(amount as i32), 1));
    }
    if symbol == ManaSymbol::X {
        return Some((Value::X, 1));
    }
    None
}

pub(crate) fn parse_cost_modifier_mana_cost(
    tokens: &[OwnedLexToken],
) -> Option<(crate::mana::ManaCost, usize)> {
    use crate::mana::{ManaCost, ManaSymbol};

    let mut pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut used = 0usize;
    while let Some(token) = tokens.get(used) {
        let Some(group) = mana_pips_from_token(token) else {
            break;
        };
        if group.iter().any(|symbol| {
            matches!(
                symbol,
                ManaSymbol::X | ManaSymbol::Snow | ManaSymbol::Life(_)
            )
        }) {
            break;
        }
        pips.push(group);
        used += 1;
    }
    if used == 0 {
        return None;
    }
    Some((ManaCost::from_pips(pips), used))
}

pub(crate) fn parse_cost_modifier_components(
    amount_tokens: &[OwnedLexToken],
) -> (
    Option<(Value, usize)>,
    Option<(crate::mana::ManaCost, usize)>,
) {
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let parsed_mana_cost = parse_cost_modifier_mana_cost(amount_tokens);

    let amount_used = parsed_amount.as_ref().map(|(_, used)| *used).unwrap_or(0);
    let mana_used = parsed_mana_cost
        .as_ref()
        .map(|(_, used)| *used)
        .unwrap_or(0);

    // Prefer mana-symbol parsing when it consumes a longer contiguous mana sequence
    // (e.g. "{2}{U}{U}" should stay a single mana-cost reduction component).
    if mana_used > amount_used {
        return (None, parsed_mana_cost);
    }

    (parsed_amount, None)
}

fn parse_cost_reduction_cap(tokens: &[OwnedLexToken]) -> Option<i32> {
    for idx in 2..tokens.len().saturating_sub(1) {
        if !tokens[idx - 2].is_word("by")
            || !tokens[idx - 1].is_word("more")
            || !tokens[idx].is_word("than")
        {
            continue;
        }
        let group = mana_pips_from_token(tokens.get(idx + 1)?)?;
        if group.len() != 1 {
            return None;
        }
        return match group[0] {
            ManaSymbol::Generic(amount) => Some(amount as i32),
            _ => None,
        };
    }
    None
}

pub(crate) fn parse_dynamic_cost_modifier_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    let has_cards_drawn_this_turn_anywhere = (word_slice_contains_word(&words_all, "card")
        || word_slice_contains_word(&words_all, "cards"))
        && word_slice_contains_word(&words_all, "drawn")
        && word_slice_contains_word(&words_all, "this")
        && word_slice_contains_word(&words_all, "turn");
    if has_cards_drawn_this_turn_anywhere
        && (word_slice_contains_word(&words_all, "you")
            || word_slice_contains_word(&words_all, "your")
            || word_slice_contains_word(&words_all, "youve")
            || word_slice_contains_word(&words_all, "you've"))
    {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::You)));
    }
    if has_cards_drawn_this_turn_anywhere
        && (word_slice_contains_word(&words_all, "opponent")
            || word_slice_contains_word(&words_all, "opponents"))
    {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent)));
    }

    let Some(each_idx) = find_index(&words_all, |word| *word == "each") else {
        return Ok(None);
    };

    let filter_tokens = &tokens[each_idx + 1..];
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    if filter_words.is_empty() {
        return Ok(None);
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["creature", "that", "died", "this", "turn"],
            &["creatures", "that", "died", "this", "turn"],
        ],
    ) {
        return Ok(Some(Value::CreaturesDiedThisTurn));
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[
            &[
                "1",
                "life",
                "your",
                "opponents",
                "have",
                "lost",
                "this",
                "turn",
            ],
            &["life", "your", "opponents", "have", "lost", "this", "turn"],
            &["1", "life", "opponents", "have", "lost", "this", "turn"],
            &["life", "opponents", "have", "lost", "this", "turn"],
        ],
    ) {
        return Ok(Some(Value::LifeLostThisTurn(PlayerFilter::Opponent)));
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["creature", "that", "died", "under", "your", "control"],
            &["creatures", "that", "died", "under", "your", "control"],
        ],
    ) {
        if word_slice_contains_word(&filter_words, "this")
            && word_slice_contains_word(&filter_words, "turn")
        {
            return Ok(Some(Value::CreaturesDiedThisTurnControlledBy(
                PlayerFilter::You,
            )));
        }
    }
    // "for each spell you've cast this turn" (and limited variants like "instant and sorcery spell")
    let has_spell_cast_turn = (word_slice_contains_word(&filter_words, "spell")
        || word_slice_contains_word(&filter_words, "spells"))
        && (word_slice_contains_word(&filter_words, "cast")
            || word_slice_contains_word(&filter_words, "casts"))
        && word_slice_contains_word(&filter_words, "this")
        && word_slice_contains_word(&filter_words, "turn");
    if has_spell_cast_turn {
        let player = if filter_words
            .iter()
            .any(|word| matches!(*word, "you" | "your" | "youve" | "you've"))
        {
            PlayerFilter::You
        } else if filter_words
            .iter()
            .any(|word| matches!(*word, "opponent" | "opponents"))
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };

        if word_slice_contains_phrase(&filter_words, &["card", "type"])
            || word_slice_contains_phrase(&filter_words, &["card", "types"])
        {
            let mut filter = ObjectFilter::spell();
            filter.cast_by = Some(player);
            return Ok(Some(Value::CardTypesAmong(filter)));
        }

        let other_than_first =
            word_slice_contains_phrase(&filter_words, &["other", "than", "the", "first"]);
        if other_than_first {
            return Ok(Some(Value::Add(
                Box::new(Value::SpellsCastThisTurn(player)),
                Box::new(Value::Fixed(-1)),
            )));
        }

        let exclude_source = word_slice_contains_word(&filter_words, "other");
        let has_instant = word_slice_contains_word(&filter_words, "instant");
        let has_sorcery = word_slice_contains_word(&filter_words, "sorcery");
        if has_instant || has_sorcery {
            let mut filter = ObjectFilter::spell();
            filter.card_types = if has_instant && has_sorcery {
                vec![CardType::Instant, CardType::Sorcery]
            } else if has_instant {
                vec![CardType::Instant]
            } else {
                vec![CardType::Sorcery]
            };
            return Ok(Some(Value::SpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source,
            }));
        }

        let simple = matches!(
            filter_words.as_slice(),
            ["spell", "youve", "cast", "this", "turn"]
                | ["spells", "youve", "cast", "this", "turn"]
                | ["spell", "you", "cast", "this", "turn"]
                | ["spells", "you", "cast", "this", "turn"]
                | ["spell", "your", "cast", "this", "turn"]
                | ["spells", "your", "cast", "this", "turn"]
        );
        if simple {
            return Ok(Some(Value::SpellsCastThisTurn(player)));
        }
    }

    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["card", "youve", "drawn", "this", "turn"],
            &["cards", "youve", "drawn", "this", "turn"],
            &["card", "you", "have", "drawn", "this", "turn"],
            &["cards", "you", "have", "drawn", "this", "turn"],
            &["card", "you", "ve", "drawn", "this", "turn"],
            &["cards", "you", "ve", "drawn", "this", "turn"],
        ],
    ) {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::You)));
    }

    let has_cards_drawn_this_turn = (word_slice_contains_word(&filter_words, "card")
        || word_slice_contains_word(&filter_words, "cards"))
        && word_slice_contains_word(&filter_words, "drawn")
        && word_slice_contains_word(&filter_words, "this")
        && word_slice_contains_word(&filter_words, "turn");
    if has_cards_drawn_this_turn
        && (word_slice_contains_word(&filter_words, "you")
            || word_slice_contains_word(&filter_words, "your")
            || word_slice_contains_word(&filter_words, "youve")
            || word_slice_contains_word(&filter_words, "you've"))
    {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::You)));
    }

    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["card", "an", "opponent", "has", "drawn", "this", "turn"],
            &["cards", "an", "opponent", "has", "drawn", "this", "turn"],
            &["card", "opponents", "have", "drawn", "this", "turn"],
            &["cards", "opponents", "have", "drawn", "this", "turn"],
        ],
    ) {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent)));
    }
    if has_cards_drawn_this_turn
        && (word_slice_contains_word(&filter_words, "opponent")
            || word_slice_contains_word(&filter_words, "opponents"))
    {
        return Ok(Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent)));
    }

    if word_slice_contains_any_phrase(&filter_words, &[&["card", "type"], &["card", "types"]])
        && word_slice_contains_word(&filter_words, "graveyard")
    {
        let player = if word_slice_contains_phrase(&filter_words, &["your", "graveyard"]) {
            PlayerFilter::You
        } else if word_slice_contains_any_phrase(
            &filter_words,
            &[&["opponents", "graveyard"], &["opponent", "graveyard"]],
        ) {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Ok(Some(Value::CardTypesInGraveyard(player)));
    }

    if word_slice_starts_with_any(
        &filter_words,
        &[
            &[
                "color", "of", "mana", "spent", "to", "cast", "this", "spell",
            ],
            &[
                "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
            ],
            &["color", "of", "mana", "used", "to", "cast", "this", "spell"],
            &[
                "colors", "of", "mana", "used", "to", "cast", "this", "spell",
            ],
        ],
    ) {
        return Ok(Some(Value::ColorsOfManaSpentToCastThisSpell));
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["creature", "in", "your", "party"],
            &["creatures", "in", "your", "party"],
        ],
    ) {
        return Ok(Some(Value::PartySize(PlayerFilter::You)));
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["basic", "land", "type", "among"],
            &["basic", "land", "types", "among"],
        ],
    ) {
        let lands_tokens = &filter_tokens[4..];
        if let Ok(filter) = parse_object_filter(lands_tokens, false) {
            return Ok(Some(Value::BasicLandTypesAmong(filter)));
        }
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["creature", "type", "among"],
            &["creature", "types", "among"],
        ],
    ) {
        let Some(after_among_token_idx) = token_index_for_word_index(filter_tokens, 3) else {
            return Ok(None);
        };
        let mut end_token_idx = filter_tokens.len();
        if let Some(period_idx) =
            find_token_kind(&filter_tokens[after_among_token_idx..], TokenKind::Period)
        {
            end_token_idx = after_among_token_idx + period_idx;
        }
        let creature_scope_tokens =
            trim_commas(&filter_tokens[after_among_token_idx..end_token_idx]);
        if let Ok(filter) = parse_object_filter(&creature_scope_tokens, false) {
            return Ok(Some(Value::CreatureTypesAmong(filter)));
        }
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[&["card", "type", "among"], &["card", "types", "among"]],
    ) {
        let Some(after_among_token_idx) = token_index_for_word_index(filter_tokens, 3) else {
            return Ok(None);
        };
        let mut end_token_idx = filter_tokens.len();
        if let Some(period_idx) =
            find_token_kind(&filter_tokens[after_among_token_idx..], TokenKind::Period)
        {
            end_token_idx = after_among_token_idx + period_idx;
        }
        let card_scope_tokens = trim_commas(&filter_tokens[after_among_token_idx..end_token_idx]);
        if let Ok(filter) = parse_object_filter(&card_scope_tokens, false) {
            return Ok(Some(Value::CardTypesAmong(filter)));
        }
    }
    if word_slice_starts_with_any(
        &filter_words,
        &[
            &["different", "powers", "among"],
            &["different", "power", "values", "among"],
            &["different", "power", "among"],
        ],
    ) {
        let scope_start_word_idx = if word_slice_at_is(&filter_words, 2, "among") {
            3
        } else {
            4
        };
        let Some(scope_start_token_idx) =
            token_index_for_word_index(filter_tokens, scope_start_word_idx)
        else {
            return Ok(None);
        };
        let mut end_token_idx = filter_tokens.len();
        if let Some(period_idx) =
            find_token_kind(&filter_tokens[scope_start_token_idx..], TokenKind::Period)
        {
            end_token_idx = scope_start_token_idx + period_idx;
        }
        let scope_tokens = trim_commas(&filter_tokens[scope_start_token_idx..end_token_idx]);
        if let Ok(filter) = parse_object_filter(&scope_tokens, false) {
            return Ok(Some(Value::DistinctPowers(filter)));
        }
    }

    let has_card_type_among = word_slice_contains_any_phrase(
        &filter_words,
        &[&["card", "type", "among"], &["card", "types", "among"]],
    );
    if has_card_type_among {
        return Err(CardTextError::ParseError(format!(
            "unsupported card-types-among dynamic value (clause: '{}')",
            words_all.join(" ")
        )));
    }

    // "for each <counter> counter removed this way" (storage lands, mana batteries, etc.)
    // The remove-counters cost plumbs the removed total through `CostContext.x_value`,
    // so model the dynamic amount as `X`.
    if (word_slice_contains_word(&filter_words, "counter")
        || word_slice_contains_word(&filter_words, "counters"))
        && word_slice_contains_word(&filter_words, "removed")
        && word_slice_contains_phrase(&filter_words, &["this", "way"])
    {
        return Ok(Some(Value::X));
    }
    if word_slice_contains_word(&filter_words, "destroyed")
        && word_slice_contains_phrase(&filter_words, &["this", "way"])
    {
        return Ok(Some(Value::PendingEffectMetric {
            source: EffectMetricSource::AffectedObjects,
            metric: EffectMetric::Count,
        }));
    }
    if word_slice_contains_word(&filter_words, "discarded")
        && word_slice_contains_phrase(&filter_words, &["this", "way"])
    {
        return Ok(Some(Value::PendingEffectMetric {
            source: EffectMetricSource::Outcome,
            metric: EffectMetric::Count,
        }));
    }
    if word_slice_contains_word(&filter_words, "revealed")
        && word_slice_contains_phrase(&filter_words, &["this", "way"])
        && let Some((value, used_words)) = parse_for_each_count_value_words(&words_all)
        && used_words == words_all.len()
    {
        return Ok(Some(value));
    }

    let mut source_counter_words = filter_words.as_slice();
    if source_counter_words
        .first()
        .is_some_and(|word| is_article(word) || *word == "one" || *word == "another")
    {
        source_counter_words = &source_counter_words[1..];
    }
    let source_counter_match = if source_counter_words.len() >= 3
        && (source_counter_words[0] == "counter" || source_counter_words[0] == "counters")
        && source_counter_words[1] == "on"
    {
        Some((None, 1usize))
    } else if source_counter_words.len() >= 4
        && parse_counter_type_word(source_counter_words[0]).is_some()
        && (source_counter_words[1] == "counter" || source_counter_words[1] == "counters")
        && source_counter_words[2] == "on"
    {
        Some((parse_counter_type_word(source_counter_words[0]), 2usize))
    } else {
        None
    };
    if let Some((counter_type, on_idx)) = source_counter_match {
        let tail = &source_counter_words[on_idx + 1..];
        let on_source = word_slice_starts_with_any(
            &tail,
            &[
                &["it"],
                &["this"],
                &["that", "object"],
                &["that", "permanent"],
            ],
        );
        if on_source {
            return Ok(Some(match counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            }));
        }
        if let Some(surface) = source_reference_surface_for_words(tail) {
            return Ok(Some(Value::CountersOn(
                Box::new(source_choose_spec_for_surface(surface)),
                counter_type,
            )));
        }
    }

    if let Some(player) = parse_commander_cast_count_player(filter_tokens) {
        return Ok(Some(Value::CommanderCastCount(player)));
    }

    if word_slice_contains_phrase(&filter_words, &["this", "way"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-way dynamic value (clause: '{}')",
            words_all.join(" ")
        )));
    }

    if let Ok(filter) = parse_object_filter(filter_tokens, false) {
        return Ok(Some(Value::Count(filter)));
    }

    Ok(None)
}

pub(crate) fn parse_add_mana_that_much_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_starts_with(&words_all, &["that", "much"]) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    None
}

pub(crate) fn parse_players_skip_upkeep_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_skip_upkeep_line_lexed(tokens) {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

pub(crate) fn parse_legend_rule_doesnt_apply_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let has_negative = word_slice_contains_word(&words, "doesnt")
        || word_slice_contains_word(&words, "doesn")
        || (word_slice_contains_word(&words, "does") && word_slice_contains_word(&words, "not"));
    if word_slice_contains_word(&words, "legend")
        && word_slice_contains_word(&words, "rule")
        && word_slice_contains_word(&words, "apply")
        && has_negative
    {
        return Ok(Some(StaticAbility::legend_rule_doesnt_apply()));
    }
    Ok(None)
}

pub(crate) fn parse_all_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_all_permanents_colorless_line_lexed(tokens) {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_subject_are_card_types_in_addition_to_their_other_types_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(be_idx) = find_index(&words, |word| matches!(*word, "is" | "are")) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 2 >= words.len() {
        return Ok(None);
    }

    let tail = &words[be_idx + 1..];
    let Some(addition_idx) = find_window_by(tail, 5, |window| {
        matches!(
            window,
            ["in", "addition", "to", "its", "other"] | ["in", "addition", "to", "their", "other"]
        )
    }) else {
        return Ok(None);
    };
    if addition_idx == 0 {
        return Ok(None);
    }

    if !word_slice_eq_any_at(&tail, addition_idx + 5, &[&["type"], &["types"]]) {
        return Ok(None);
    }

    let subject_tokens = &tokens[..be_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let added_words = &tail[..addition_idx];
    if matches!(added_words, ["the", "chosen", "type"] | ["chosen", "type"]) {
        let filter = parse_object_filter(subject_tokens, false)?;
        return Ok(Some(vec![StaticAbility::add_chosen_creature_type(
            filter,
            render_token_slice(tokens),
        )]));
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in &tail[..addition_idx] {
        if is_article(descriptor) || matches!(*descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !slice_contains(&card_types, &card_type) {
                card_types.push(card_type);
            }
            continue;
        }

        let Some(subtype) = parse_subtype_flexible(descriptor) else {
            return Ok(None);
        };
        if !slice_contains(&subtypes, &subtype) {
            subtypes.push(subtype);
        }
    }
    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter(subject_tokens, false)?;

    let mut abilities = Vec::new();
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub(crate) fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_contains_word(&words, "colorless")
        && word_slice_contains_word(&words, "cards")
        && word_slice_contains_word(&words, "spells")
        && word_slice_contains_word(&words, "permanents")
    {
        return Ok(Some(StaticAbility::make_colorless(ObjectFilter::default())));
    }
    Ok(None)
}

pub(crate) fn parse_all_cards_spells_permanents_add_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        [
            "all",
            "cards",
            "that",
            "arent",
            "on",
            "the",
            "battlefield",
            "spells",
            "and",
            "permanents",
            "are",
            "the",
            "chosen",
            "color",
            "in",
            "addition",
            "to",
            "their",
            "other",
            "colors"
        ]
    ) {
        return Ok(Some(StaticAbility::add_chosen_color(
            ObjectFilter::default(),
            render_token_slice(tokens),
        )));
    }

    Ok(None)
}

pub(crate) fn parse_all_are_color_and_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 10 {
        return Ok(None);
    }
    let Some(are_idx) = find_index(&words, |word| *word == "are") else {
        return Ok(None);
    };
    if are_idx == 0 || are_idx + 4 >= words.len() {
        return Ok(None);
    }

    let Some(base_color) = words.get(are_idx + 1).and_then(|word| parse_color(word)) else {
        return Ok(None);
    };

    // Pattern: "<subject> are <color> and are <subtype>... in addition to their other creature types"
    if !word_slice_starts_with_at(&words, are_idx + 2, &["and", "are"]) {
        return Ok(None);
    }

    let tail = &words[are_idx + 4..];
    let Some(addition_idx) =
        word_slice_find_phrase_start(tail, &["in", "addition", "to", "their", "other"])
    else {
        return Ok(None);
    };
    if addition_idx == 0 {
        return Ok(None);
    }

    let scope = &tail[addition_idx + 5..];
    if !word_slice_eq_any(scope, &[&["creature", "type"], &["creature", "types"]]) {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in &tail[..addition_idx] {
        if is_article(descriptor) || matches!(*descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !slice_contains(&card_types, &card_type) {
                card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            if !slice_contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in are-color-and-type-addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..are_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(subject_tokens, false)?;

    let mut abilities = vec![StaticAbility::set_colors(filter.clone(), base_color)];
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub(crate) fn parse_all_creatures_are_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 4 {
        return Ok(None);
    }
    let Some(are_idx) = find_index(&words, |word| *word == "is" || *word == "are") else {
        return Ok(None);
    };
    if are_idx == 0 {
        return Ok(None);
    }
    let color = match &words[are_idx + 1..] {
        ["all", "colors"] => crate::color::Color::ALL.into_iter().collect::<ColorSet>(),
        [color_word] => {
            let Some(color) = parse_color(color_word) else {
                return Ok(None);
            };
            color
        }
        _ => return Ok(None),
    };

    let subject_tokens = &tokens[..are_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(subject_tokens, false)?;

    Ok(Some(StaticAbility::set_colors(filter, color)))
}

pub(crate) fn parse_subjects_are_basic_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(be_idx) = find_index(&words, |word| matches!(*word, "is" | "are")) else {
        return Ok(None);
    };
    if be_idx == 0 || !word_slice_eq(&words[be_idx + 1..], &["basic"]) {
        return Ok(None);
    }

    let subject_tokens = trim_lexed_commas(&tokens[..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_segments = split_lexed_slices_on_and(subject_tokens);
    let filter = if subject_segments.len() > 1 {
        let mut branches = Vec::with_capacity(subject_segments.len());
        for segment in subject_segments {
            let segment = trim_lexed_commas(segment);
            if segment.is_empty() {
                return Ok(None);
            }
            branches.push(parse_object_filter_lexed(segment, false)?);
        }
        let mut filter = ObjectFilter::default();
        filter.any_of = branches;
        filter
    } else {
        parse_object_filter_lexed(subject_tokens, false)?
    };

    Ok(Some(StaticAbility::add_supertypes(
        filter,
        vec![Supertype::Basic],
    )))
}

pub(crate) fn parse_nonbasic_lands_are_basic_land_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(be_idx) = find_index(&words, |word| matches!(*word, "is" | "are")) else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
    }

    let mut subtype_idx = be_idx + 1;
    if word_refs_at_is_article(&words, subtype_idx) {
        subtype_idx += 1;
    }
    let subtype_words = &words[subtype_idx..];
    if subtype_words.len() != 1 {
        return Ok(None);
    }

    let Some(subtype) = parse_subtype_flexible(subtype_words[0]) else {
        return Ok(None);
    };
    if !matches!(
        subtype,
        Subtype::Plains | Subtype::Island | Subtype::Swamp | Subtype::Mountain | Subtype::Forest
    ) {
        return Ok(None);
    }

    let subject_tokens = &tokens[..be_idx];
    let filter = parse_object_filter(subject_tokens, false)?;
    let filter_words = &words[..be_idx];
    if !filter_words
        .iter()
        .any(|word| matches!(*word, "land" | "lands"))
    {
        return Ok(None);
    }

    Ok(Some(StaticAbility::set_land_subtypes(
        filter,
        vec![subtype],
    )))
}

pub(crate) fn parse_remove_snow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_remove_snow_line_lexed(tokens) {
        return Ok(Some(StaticAbility::remove_supertypes(
            ObjectFilter::land(),
            vec![Supertype::Snow],
        )));
    }
    Ok(None)
}

pub(crate) fn parse_land_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 10 {
        return Ok(None);
    }

    let Some(be_idx) = find_index(&words, |word| *word == "is" || *word == "are") else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
    }

    if matches!(
        &words[be_idx + 1..],
        [
            "every",
            "basic",
            "land",
            "type" | "types",
            "in",
            "addition",
            "to",
            "its" | "their",
            "other",
            "type" | "types"
        ]
    ) {
        let filter_tokens = &tokens[..be_idx];
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(Some(StaticAbility::add_subtypes(
            filter,
            vec![
                Subtype::Plains,
                Subtype::Island,
                Subtype::Swamp,
                Subtype::Mountain,
                Subtype::Forest,
            ],
        )));
    }

    let mut subtype_word_idx = be_idx + 1;
    if words
        .get(subtype_word_idx)
        .is_some_and(|word| is_article(word))
    {
        subtype_word_idx += 1;
    }
    let Some(subtype_word) = words.get(subtype_word_idx).copied() else {
        return Ok(None);
    };
    let Some(subtype) = parse_subtype_flexible(subtype_word) else {
        return Ok(None);
    };
    if !is_land_subtype(subtype) {
        return Ok(None);
    }

    let tail = &words[subtype_word_idx + 1..];
    let valid_tail = matches!(
        tail,
        ["in", "addition", "to", "its", "other", "land", "type"]
            | ["in", "addition", "to", "its", "other", "land", "types"]
            | ["in", "addition", "to", "their", "other", "land", "type"]
            | ["in", "addition", "to", "their", "other", "land", "types"]
    );
    if !valid_tail {
        return Ok(None);
    }

    let filter_tokens = &tokens[..be_idx];
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(filter_tokens, false)?;

    Ok(Some(StaticAbility::add_subtypes(filter, vec![subtype])))
}

pub(crate) fn parse_lands_are_pt_creatures_still_lands_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(be_idx) = find_index(&words, |word| *word == "is" || *word == "are") else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 2 >= words.len() {
        return Ok(None);
    }
    let (power, toughness) = match parse_pt_modifier(words[be_idx + 1]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    if !matches!(words[be_idx + 2], "creature" | "creatures") {
        return Ok(None);
    }

    let tail = &words[be_idx + 3..];
    let valid_tail = matches!(
        tail,
        ["that", "are", "still", "land"]
            | ["that", "are", "still", "lands"]
            | ["that", "is", "still", "land"]
            | ["that", "is", "still", "a", "land"]
    );
    if !valid_tail {
        return Ok(None);
    }

    let filter_tokens = &tokens[..be_idx];
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(filter_tokens, false)?;

    Ok(Some(vec![
        StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
        StaticAbility::set_base_power_toughness(filter, power, toughness),
    ]))
}

pub(crate) fn parse_filter_is_pt_creature_in_addition_and_has_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(be_idx) = find_index(tokens, |token| token.is_word("is") || token.is_word("are"))
    else {
        return Ok(None);
    };
    let Some(has_idx) = find_index(&tokens[be_idx + 1..], |token| {
        token.is_word("has") || token.is_word("have")
    })
    .map(|offset| be_idx + 1 + offset) else {
        return Ok(None);
    };

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let attached_subject = crate::runtime_backend::token_word_refs(&subject_tokens)
        .first()
        .is_some_and(|word| matches!(*word, "enchanted" | "equipped"));

    let before_has = trim_commas(&tokens[be_idx + 1..has_idx]);
    if before_has.is_empty() {
        return Ok(None);
    }
    let before_has_words = crate::runtime_backend::token_word_refs(&before_has);
    let before_has_words = strip_leading_article_word_refs(&before_has_words);
    if before_has_words.len() < 8 {
        return Ok(None);
    }

    let (power, toughness) = match parse_pt_modifier(before_has_words[0]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let Some(creature_idx) = find_index(&before_has_words, |word| {
        matches!(*word, "creature" | "creatures")
    }) else {
        return Ok(None);
    };
    if creature_idx == 0 {
        return Ok(None);
    }
    let subtype_words = &before_has_words[1..creature_idx];
    let mut subtypes = Vec::new();
    for word in subtype_words {
        if is_article(word) {
            continue;
        }
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }
    let mut tail = &before_has_words[creature_idx + 1..];
    if word_slice_last_is(tail, "and") {
        tail = &tail[..tail.len().saturating_sub(1)];
    }
    let valid_tail = matches!(
        tail,
        ["in", "addition", "to", "its", "other", "type"]
            | ["in", "addition", "to", "its", "other", "types"]
            | ["in", "addition", "to", "their", "other", "type"]
            | ["in", "addition", "to", "their", "other", "types"]
    );
    if !valid_tail {
        return Ok(None);
    }

    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(&tokens[has_idx + 1..], &clause_words, attached_subject)?
    else {
        return Ok(None);
    };

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::Add,
            base_power_toughness: Some((power, toughness)),
            granted_tail,
        },
    )))
}

pub(crate) fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = if let Some((label, body_tokens)) = split_em_dash_label_prefix(tokens) {
        if preserve_keyword_prefix_for_parse(label.as_str()) {
            tokens
        } else {
            body_tokens
        }
    } else {
        tokens
    };
    let Some(be_idx) = find_index(tokens, |token| token.is_word("is") || token.is_word("are"))
    else {
        return Ok(None);
    };
    let Some(with_idx) = find_index(&tokens[be_idx + 1..], |token| token.is_word("with"))
        .map(|offset| be_idx + 1 + offset)
    else {
        return Ok(None);
    };

    let (_condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let attached_subject = crate::runtime_backend::token_word_refs(&subject_tokens)
        .first()
        .is_some_and(|word| matches!(*word, "enchanted" | "equipped"));

    let type_tokens = trim_commas(&tokens[be_idx + 1..with_idx]);
    if type_tokens.is_empty() {
        return Ok(None);
    }
    let type_words = crate::runtime_backend::token_word_refs(&type_tokens);
    let type_words = strip_leading_article_word_refs(&type_words);
    if type_words.is_empty() {
        return Ok(None);
    }
    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }

    let mut after_with = trim_commas(&tokens[with_idx + 1..]).to_vec();
    if after_with.is_empty() {
        return Ok(None);
    }

    let _loses_other_creature_types = {
        let words = crate::runtime_backend::token_word_refs(&after_with)
            .into_iter()
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        let mut note_start = None;
        let mut idx = 0usize;
        while idx + 6 <= words.len() {
            let window = &words[idx..idx + 6];
            if matches!(
                window,
                [it_or_this, loses, all, other, creature, types]
                    if matches!(it_or_this.as_str(), "it" | "this")
                        && loses == "loses"
                        && all == "all"
                        && other == "other"
                        && creature == "creature"
                        && types == "types"
            ) {
                note_start = Some(idx);
                break;
            }
            idx += 1;
        }
        if let Some(note_start) = note_start {
            let Some(token_idx) = token_index_for_word_index(&after_with, note_start) else {
                return Ok(None);
            };
            after_with.truncate(token_idx);
            true
        } else {
            false
        }
    };

    let after_with = trim_edge_punctuation(&after_with);
    let after_with_words = crate::runtime_backend::token_word_refs(&after_with);
    if after_with_words.len() < 5
        || !word_slice_starts_with(&after_with_words, &["base", "power", "and", "toughness"])
    {
        return Ok(None);
    }
    let (power, toughness) = match parse_pt_modifier(after_with_words[4]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };

    let ability_start_word_idx = if word_slice_at_is(&after_with_words, 5, ",") {
        6
    } else {
        5
    };
    if ability_start_word_idx >= after_with_words.len() {
        return Ok(None);
    }
    let Some(ability_start_idx) = token_index_for_word_index(&after_with, ability_start_word_idx)
    else {
        return Ok(None);
    };
    let ability_tokens = trim_commas(&after_with[ability_start_idx..]);
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(&ability_tokens, &after_with_words, attached_subject)?
    else {
        return Ok(None);
    };

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition: _condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::ReplaceCreatureTypes,
            base_power_toughness: Some((power, toughness)),
            granted_tail,
        },
    )))
}

pub(crate) fn parse_creatures_cant_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if is_creatures_cant_block_line_lexed(tokens) {
        return Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter: ObjectFilter::creature(),
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_block())),
            condition: None,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_all_damage_dealt_to_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_dealt_to_creatures_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_damage_dealt_to_creatures()));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_damage_to_other_creature_you_control_put_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::prevent_damage_to_other_creature_you_control_put_counters_instead(
            crate::object::CounterType::PlusOnePlusOne,
            display_text_for_tokens(tokens, true),
        ),
    ))
}

fn parse_damage_source_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut words = strip_leading_article_word_refs(words).to_vec();
    if word_slice_last_is_any(&words, &["source", "sources"]) {
        words.pop();
    }
    if words.is_empty() {
        return Some(ObjectFilter::default());
    }

    let mut filter = ObjectFilter::default();
    let mut colors: Option<ColorSet> = None;
    for word in words {
        if matches!(word, "and" | "or") {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = Some(colors.unwrap_or_else(ColorSet::new).union(color));
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            filter.card_types.push(card_type);
            continue;
        }
        return None;
    }
    if let Some(colors) = colors {
        filter.colors = Some(colors);
    }
    Some(filter)
}

pub(crate) fn parse_prevent_damage_to_you_from_source_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_first_is(&words, "if") {
        return Ok(None);
    }
    let Some(would_idx) = word_slice_find_phrase_start(
        &words,
        &["would", "deal", "damage", "to", "you"],
    ) else {
        return Ok(None);
    };
    if would_idx <= 1 {
        return Ok(None);
    }
    let tail = &words[would_idx + 5..];
    if tail.len() != 5
        || !word_slice_first_is(tail, "prevent")
        || !word_slice_eq(&tail[2..], &["of", "that", "damage"])
    {
        return Ok(None);
    }
    let Some(amount) = parse_number_word_i32(tail[1]).filter(|amount| *amount > 0) else {
        return Ok(None);
    };
    let Some(source_filter) = parse_damage_source_filter_words(&words[1..would_idx]) else {
        return Ok(None);
    };
    let display = format!(
        "If {}, prevent {} of that damage.",
        words[1..would_idx + 5].join(" "),
        tail[1]
    );

    Ok(Some(StaticAbility::prevent_damage_to_you_from_source_filter(
        amount as u32,
        source_filter,
        display,
    )))
}

pub(crate) fn parse_replace_damage_with_counters_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let soul_scar_words = [
        "if",
        "a",
        "source",
        "you",
        "control",
        "would",
        "deal",
        "noncombat",
        "damage",
        "to",
        "a",
        "creature",
        "an",
        "opponent",
        "controls",
        "put",
        "that",
        "many",
        "-1/-1",
        "counters",
        "on",
        "that",
        "creature",
        "instead",
    ];
    let soul_scar_words_no_articles = [
        "if",
        "source",
        "you",
        "control",
        "would",
        "deal",
        "noncombat",
        "damage",
        "to",
        "creature",
        "opponent",
        "controls",
        "put",
        "that",
        "many",
        "-1/-1",
        "counters",
        "on",
        "that",
        "creature",
        "instead",
    ];
    if words.as_slice() != soul_scar_words && words.as_slice() != soul_scar_words_no_articles {
        return Ok(None);
    }

    Ok(Some(StaticAbility::replace_damage_with_counters_instead(
        CounterType::MinusOneMinusOne,
        ObjectFilter::default().controlled_by(PlayerFilter::You),
        ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        Some(false),
        display_text_for_tokens(tokens, true),
    )))
}

pub(crate) fn parse_double_counters_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    let generic_under_your_control = [
        "if",
        "an",
        "effect",
        "would",
        "put",
        "one",
        "or",
        "more",
        "counters",
        "on",
        "a",
        "permanent",
        "you",
        "control",
        "it",
        "puts",
        "twice",
        "that",
        "many",
        "of",
        "those",
        "counters",
        "on",
        "that",
        "permanent",
        "instead",
    ];
    if line_words == generic_under_your_control {
        return Ok(Some(StaticAbility::double_counters_replacement(
            ObjectFilter::permanent().controlled_by(PlayerFilter::You),
            None,
            display_text_for_tokens(tokens, true),
        )));
    }

    let prefix = [
        "if", "one", "or", "more", "+1/+1", "counters", "would", "be", "put", "on",
    ];
    if !word_slice_starts_with(&line_words, &prefix)
        || !word_slice_ends_with_any(
            &line_words,
            &[
                &["twice", "that", "many", "are", "put", "on", "it", "instead"],
                &[
                    "twice", "that", "many", "are", "put", "on", "that", "creature", "instead",
                ],
                &[
                    "twice", "that", "many", "+1/+1", "counters", "are", "put", "on", "it",
                    "instead",
                ],
                &[
                    "twice", "that", "many", "+1/+1", "counters", "are", "put", "on", "that",
                    "creature", "instead",
                ],
            ],
        )
    {
        return Ok(None);
    }

    let Some(twice_idx) = find_index(&line_words, |word| *word == "twice") else {
        return Ok(None);
    };
    if twice_idx <= prefix.len() {
        return Ok(None);
    }

    let filter_tokens = line_words[prefix.len()..twice_idx]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let filter = parse_object_filter_lexed(&filter_tokens, false)?;

    Ok(Some(StaticAbility::double_counters_replacement(
        filter,
        Some(crate::object::CounterType::PlusOnePlusOne),
        display_text_for_tokens(tokens, true),
    )))
}

pub(crate) fn parse_double_token_creation_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    let under_your_control_active = [
        "if", "an", "effect", "would", "create", "one", "or", "more", "tokens", "under", "your",
        "control", "it", "creates", "twice", "that", "many", "of", "those", "tokens", "instead",
    ];
    let under_your_control_passive = [
        "if", "one", "or", "more", "tokens", "would", "be", "created", "under", "your", "control",
        "twice", "that", "many", "of", "those", "tokens", "are", "created", "instead",
    ];

    if line_words == under_your_control_active || line_words == under_your_control_passive {
        return Ok(Some(StaticAbility::double_token_creation_replacement(
            PlayerFilter::You,
            display_text_for_tokens(tokens, true),
        )));
    }

    let add_one_prefix = ["if", "you", "would", "create", "one", "or", "more"];
    if word_slice_starts_with(&line_words, &add_one_prefix)
        && let Some(token_idx) = line_words[add_one_prefix.len()..]
            .iter()
            .position(|word| matches!(*word, "token" | "tokens"))
            .map(|idx| idx + add_one_prefix.len())
    {
        let descriptor = &line_words[add_one_prefix.len()..token_idx];
        let additional_prefix = [
            "instead",
            "create",
            "those",
            "tokens",
            "plus",
            "an",
            "additional",
        ];
        let after_token = &line_words[token_idx + 1..];
        if !descriptor.is_empty()
            && word_slice_starts_with(after_token, &additional_prefix)
            && word_slice_last_is(after_token, "token")
            && &after_token[additional_prefix.len()..after_token.len() - 1] == descriptor
        {
            if !word_slice_eq(descriptor, &["treasure"]) {
                return Ok(None);
            }
            let mut token_filter = ObjectFilter::default().token();
            for word in descriptor {
                if let Some(card_type) = parse_card_type(word) {
                    token_filter = token_filter.with_type(card_type);
                } else if let Some(subtype) = parse_subtype_flexible(word) {
                    token_filter = token_filter.with_subtype(subtype);
                } else {
                    return Ok(None);
                }
            }
            return Ok(Some(StaticAbility::add_token_creation_replacement(
                PlayerFilter::You,
                token_filter,
                ironsmith_core::AdditionalTokenKind::Treasure,
                1,
                display_text_for_tokens(tokens, true),
            )));
        }
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_combat_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_combat_damage_to_source_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_combat_damage_to_self()));
    }

    Ok(None)
}

pub(crate) fn parse_during_your_turn_prevent_all_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_during_your_turn_prevent_all_damage_to_source_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self()
                .with_condition(crate::ConditionExpr::YourTurn),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_noncombat_damage_to_other_creatures_you_control(),
        ));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_damage_to_source_by_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_to_source_by_creatures_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self_by_creatures(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_may_choose_not_to_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !matches!(
        words.as_slice(),
        ["you", "may", "choose", "not", "to", "untap", ..]
    ) {
        return Ok(None);
    }
    if !word_slice_ends_with(&words, &["during", "your", "untap", "step"]) {
        return Ok(None);
    }
    if words.len() <= 10 {
        return Ok(None);
    }

    let subject_words = &words[6..words.len() - 4];
    let subject_allowed = matches!(
        subject_words,
        ["this"]
            | ["it"]
            | ["this", "artifact"]
            | ["this", "creature"]
            | ["this", "land"]
            | ["this", "permanent"]
            | ["this", "card"]
    );
    if !subject_allowed {
        return Ok(None);
    }

    let subject = subject_words.join(" ");
    Ok(Some(
        StaticAbility::may_choose_not_to_untap_during_untap_step(subject),
    ))
}

pub(crate) fn parse_untap_during_each_other_players_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = split_untap_each_other_players_untap_step_line_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.subject_tokens);
    if subject_tokens.is_empty() {
        let line_words = crate::runtime_backend::token_word_refs(tokens);
        return Err(CardTextError::ParseError(format!(
            "missing subject in other-players untap ability (clause: '{}')",
            line_words.join(" ")
        )));
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(" ");
    Ok(Some(
        StaticAbility::untap_during_each_other_players_untap_step(
            filter,
            format!("Untap all {subject_text} during each other player's untap step"),
        ),
    ))
}

pub(crate) fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    match parse_doesnt_untap_during_untap_step_spec_lexed(tokens) {
        Some(DoesntUntapDuringUntapStepSpec::Source { tail_tokens }) => {
            let clause_words = crate::runtime_backend::token_word_refs(tokens);
            let tail_tokens = trim_commas(tail_tokens);
            if tail_tokens.is_empty() {
                return Ok(Some(
                    StaticAbilityAst::Static(StaticAbility::doesnt_untap()),
                ));
            }
            let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
            if word_slice_first_is(&tail_words, "if") {
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after untap-step if-clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                let condition = parse_static_condition_clause(&condition_tokens)?;
                return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                    condition,
                }));
            }

            Err(CardTextError::ParseError(format!(
                "unsupported trailing untap-step clause (clause: '{}')",
                clause_words.join(" ")
            )))
        }
        Some(DoesntUntapDuringUntapStepSpec::Attached { subject_tokens }) => {
            let subject = crate::runtime_backend::token_word_refs(subject_tokens).join(" ");
            let text = format!("{subject} doesnt untap during its controllers untap step");
            Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                display: text,
                condition: None,
            }))
        }
        None => Ok(None),
    }
}

pub(crate) fn parse_flying_restriction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(match parse_flying_block_restriction_line_lexed(tokens) {
        Some(FlyingBlockRestrictionKind::FlyingOnly) => {
            Some(StaticAbility::flying_only_restriction())
        }
        Some(FlyingBlockRestrictionKind::FlyingOrReach) => {
            Some(StaticAbility::flying_restriction())
        }
        None => None,
    })
}

pub(crate) fn parse_can_block_only_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_block_only_flying_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_block_only_flying()));
    }

    Ok(None)
}

pub(crate) fn parse_can_block_subtype_as_though_reach_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(parse_can_block_subtype_as_though_reach_line_lexed(tokens)
        .map(StaticAbility::can_block_subtype_as_though_reach))
}

pub(crate) fn parse_assign_damage_as_unblocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_may_assign_damage_as_unblocked_line_lexed(tokens) {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }

    Ok(None)
}

pub(crate) fn parse_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };
    let Some((_, head_tokens)) = super::grammar::primitives::strip_lexed_suffix_phrases(
        tokens,
        &[
            &["where", "x", "is", "that", "spell's", "mana", "value"],
            &["where", "x", "is", "that", "spells", "mana", "value"],
        ],
    ) else {
        return Ok(None);
    };
    let head_tokens = trim_lexed_commas(head_tokens);
    let words = super::lexer::parser_token_word_refs(head_tokens);
    if !word_slice_starts_with(
        words.as_slice(),
        &[
            "you", "may", "pay", "x", "rather", "than", "pay", "the", "mana", "cost", "for",
        ],
    ) {
        return Ok(None);
    }

    let Some(for_idx) = find_index(head_tokens, |token| token.is_word("for")) else {
        return Ok(None);
    };
    let subject_tokens = trim_lexed_commas(head_tokens.get(for_idx + 1..).unwrap_or_default());
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    if subject_tokens.is_empty()
        || !word_slice_contains_word(&subject_words, "spell")
            && !word_slice_contains_word(&subject_words, "spells")
    {
        return Ok(None);
    }

    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(subject_tokens);
    Ok(Some(StaticAbility::grants(crate::grant::GrantSpec::new(
        crate::grant::Grantable::mana_value_as_generic_from_hand(),
        filter,
        Zone::Hand,
    ))))
}

pub(crate) fn parse_life_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };

    let words = super::lexer::parser_token_word_refs(tokens);
    if !word_slice_starts_with(
        words.as_slice(),
        &[
            "once", "during", "each", "of", "your", "turns", "you", "may", "cast",
        ],
    ) {
        return Ok(None);
    }

    let Some(cast_idx) = find_index(tokens, |token| token.is_word("cast")) else {
        return Ok(None);
    };
    let Some(by_idx) = find_index(tokens, |token| token.is_word("by")) else {
        return Ok(None);
    };
    if by_idx <= cast_idx + 1 {
        return Ok(None);
    }
    let subject_tokens = trim_lexed_commas(tokens.get(cast_idx + 1..by_idx).unwrap_or_default());
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    if subject_tokens.is_empty()
        || !slice_contains(&subject_words, &"spell") && !slice_contains(&subject_words, &"spells")
    {
        return Ok(None);
    }

    let tail_words =
        crate::runtime_backend::token_word_refs(tokens.get(by_idx + 1..).unwrap_or_default());
    if tail_words
        != [
            "paying", "life", "equal", "to", "its", "mana", "value", "rather", "than", "paying",
            "its", "mana", "cost",
        ]
        && tail_words
            != [
                "pay", "life", "equal", "to", "its", "mana", "value", "rather", "than", "pay",
                "its", "mana", "cost",
            ]
    {
        return Ok(None);
    }

    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(subject_tokens);
    Ok(Some(StaticAbility::grants(crate::grant::GrantSpec::new(
        crate::grant::Grantable::life_equal_mana_value_from_hand(Some(
            crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns,
        )),
        filter,
        Zone::Hand,
    ))))
}

pub(crate) fn parse_fixed_mana_cost_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };
    let words = super::lexer::parser_token_word_refs(tokens);
    if !word_slice_starts_with(words.as_slice(), &["you", "may", "pay"])
        || word_slice_contains_word(&words, "where")
    {
        return Ok(None);
    }

    let Some((mana_cost, consumed)) =
        leading_mana_cost_from_tokens(tokens.get(3..).unwrap_or_default())
    else {
        return Ok(None);
    };

    let tail_tokens = tokens.get(3 + consumed..).unwrap_or_default();
    let tail_words = super::lexer::parser_token_word_refs(tail_tokens);
    if !word_slice_starts_with(
        tail_words.as_slice(),
        &["rather", "than", "pay", "the", "mana", "cost", "for"],
    ) {
        return Ok(None);
    }

    let subject_tokens = trim_lexed_commas(tail_tokens.get(7..).unwrap_or_default());
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    if subject_tokens.is_empty()
        || !word_slice_contains_word(&subject_words, "spell")
            && !word_slice_contains_word(&subject_words, "spells")
    {
        return Ok(None);
    }

    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(subject_tokens);
    Ok(Some(StaticAbility::grants(
        crate::grant::GrantSpec::cast_from_hand_for_alternative_mana_cost_matching(
            filter, mana_cost,
        ),
    )))
}

pub(crate) fn parse_grant_flash_to_noncreature_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec == crate::grant::GrantSpec::flash_to_noncreature_spells() => {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

fn static_grant_beneficiary(player: crate::cards::builders::PlayerAst) -> Option<PlayerFilter> {
    match player {
        crate::cards::builders::PlayerAst::You | crate::cards::builders::PlayerAst::Implicit => {
            Some(PlayerFilter::You)
        }
        crate::cards::builders::PlayerAst::Any => Some(PlayerFilter::Any),
        _ => None,
    }
}

pub(crate) fn parse_you_may_cast_exile_counter_cards_with_mana_permission_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let cast_prefix = [
        "you", "may", "cast", "spells", "from", "among", "cards", "in", "exile",
    ];
    let play_lands_and_cast_prefix = [
        "you",
        "may",
        "play",
        "lands",
        "and",
        "cast",
        "noncreature",
        "spells",
        "from",
        "among",
        "cards",
        "you",
        "exiled",
    ];
    let is_cast_from_exile_family = word_slice_starts_with(&words, &cast_prefix);
    let is_play_lands_and_cast_noncreature_family =
        word_slice_starts_with(&words, &play_lands_and_cast_prefix);
    if !is_cast_from_exile_family && !is_play_lands_and_cast_noncreature_family {
        return Ok(None);
    }

    let Some(and_idx) =
        word_slice_find_phrase_start(&words, &["and", "you", "may", "spend", "mana"])
    else {
        return Ok(None);
    };
    let (counter_start_idx, counters_idx) =
        if let Some(with_idx) = word_slice_find_word(&words[..and_idx], "with") {
            let Some(counters_idx) = words[with_idx + 1..and_idx]
                .iter()
                .position(|word| matches!(*word, "counter" | "counters"))
                .map(|offset| with_idx + 1 + offset)
            else {
                return Ok(None);
            };
            (with_idx + 1, counters_idx)
        } else if is_play_lands_and_cast_noncreature_family {
            let that_have_prefix = ["that", "have"];
            let Some(that_have_idx) =
                word_slice_find_phrase_start(&words[..and_idx], &that_have_prefix)
            else {
                return Ok(None);
            };
            let Some(counters_idx) = words[that_have_idx + that_have_prefix.len()..and_idx]
                .iter()
                .position(|word| matches!(*word, "counter" | "counters"))
                .map(|offset| that_have_idx + that_have_prefix.len() + offset)
            else {
                return Ok(None);
            };
            (that_have_idx + that_have_prefix.len(), counters_idx)
        } else {
            return Ok(None);
        };
    if counters_idx + 3 > and_idx
        || !word_slice_starts_with_at(&words, counters_idx + 1, &["on", "them"])
    {
        return Ok(None);
    }

    let owner = if is_play_lands_and_cast_noncreature_family {
        None
    } else {
        let owner_words = &words[cast_prefix.len()..counter_start_idx.saturating_sub(1)];
        match owner_words {
            [] => None,
            ["your", "opponents", "own"]
            | ["your", "opponent", "owns"]
            | ["opponents", "own"]
            | ["opponent", "owns"] => Some(PlayerFilter::Opponent),
            _ => return Ok(None),
        }
    };

    let counter_tokens = &tokens[counter_start_idx..counters_idx + 1];
    let Some(counter_type) = parse_counter_type_from_tokens(counter_tokens) else {
        return Ok(None);
    };

    let spend_words = &words[and_idx..];
    let uses_snow_sources = matches!(
        spend_words,
        [
            "and", "you", "may", "spend", "mana", "from", "snow", "sources", "as", "though", "it",
            "were", "mana", "of", "any", "color", "to", "cast", "those", "spells"
        ]
    );
    let valid_spend_suffix = uses_snow_sources
        || matches!(
            spend_words,
            [
                "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of",
                "any", "color", "to", "cast", "those", "spells"
            ]
        );
    if !valid_spend_suffix {
        return Ok(None);
    }

    let mut base_filter = ObjectFilter {
        zone: Some(Zone::Exile),
        owner,
        with_counter: Some(crate::filter::CounterConstraint::Typed(counter_type)),
        ..ObjectFilter::default()
    };
    if is_play_lands_and_cast_noncreature_family {
        base_filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
    }

    let mut filter = if is_play_lands_and_cast_noncreature_family {
        ObjectFilter {
            any_of: vec![
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
                ObjectFilter {
                    excluded_card_types: vec![CardType::Creature, CardType::Land],
                    ..base_filter.clone()
                },
            ],
            ..ObjectFilter::default()
        }
    } else {
        ObjectFilter {
            excluded_card_types: vec![CardType::Land],
            ..base_filter
        }
    };
    filter.has_mana_cost = false;

    let grant = StaticAbility::grants(
        crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            filter.clone(),
            Zone::Exile,
        )
        .with_beneficiary(PlayerFilter::You),
    );
    let permission = if uses_snow_sources {
        crate::effect::ManaSpendPermission::any_color_from_sources_for_casting_matching(
            PlayerFilter::You,
            filter,
            ObjectFilter::default().with_supertype(Supertype::Snow),
        )
    } else {
        crate::effect::ManaSpendPermission::any_color_for_casting_matching(
            PlayerFilter::You,
            filter,
        )
    };
    let mana_permission =
        StaticAbility::mana_spend_permission(permission, render_token_slice(tokens));

    Ok(Some(vec![grant, mana_permission]))
}

pub(crate) fn parse_surveilled_graveyard_play_life_cost_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let normalized = words
        .iter()
        .map(|word| word.replace(['\'', '’'], ""))
        .collect::<Vec<_>>();
    let refs = normalized.iter().map(String::as_str).collect::<Vec<_>>();
    if !matches!(
        refs.as_slice(),
        [
            "you",
            "may",
            "play",
            "lands",
            "and",
            "cast",
            "spells",
            "from",
            "among",
            "cards",
            "in",
            "your",
            "graveyard",
            "youve",
            "surveilled",
            "this",
            "turn",
            "if",
            "you",
            "cast",
            "a",
            "spell",
            "this",
            "way",
            "you",
            "pay",
            "life",
            "equal",
            "to",
            "its",
            "mana",
            "value",
            "rather",
            "than",
            "paying",
            "its",
            "mana",
            "cost"
        ]
    ) {
        return Ok(None);
    }

    let base_filter = ObjectFilter {
        zone: Some(Zone::Graveyard),
        owner: Some(PlayerFilter::You),
        surveilled_this_turn: true,
        ..ObjectFilter::default()
    };
    let mut spell_filter = base_filter.clone();
    spell_filter.excluded_card_types.push(CardType::Land);

    Ok(Some(vec![
        StaticAbility::grants(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                base_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
        StaticAbility::grants(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::life_equal_mana_value_from_zone(Zone::Graveyard, None),
                spell_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
    ]))
}

pub(crate) fn parse_you_may_static_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let source_linked_exile_cast_prefix = [
        "during", "each", "players", "turn", "that", "player", "may", "cast", "a", "spell", "from",
        "among", "the", "cards", "they", "dont", "own", "exiled", "with",
    ];
    let any_mana_cast_suffix = [
        "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "it",
    ];
    if word_slice_starts_with(&words, &source_linked_exile_cast_prefix)
        && word_slice_ends_with(&words, &any_mana_cast_suffix)
        && words.len() > source_linked_exile_cast_prefix.len() + any_mana_cast_suffix.len()
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.owner = Some(PlayerFilter::NotYou);
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(Some(StaticAbility::grants(
            crate::grant::GrantSpec::new(crate::grant::Grantable::play_from(), filter, Zone::Exile)
                .with_beneficiary(PlayerFilter::Any),
        )));
    }

    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) => {
            let clause_words = crate::runtime_backend::token_word_refs(tokens);
            let singular_spell = word_slice_find_any_phrase_start(
                &clause_words,
                &[&["cast", "a", "spell"], &["cast", "one", "spell"]],
            )
            .is_some();
            if singular_spell
                && spec.zone == Zone::Hand
                && matches!(
                    &spec.grantable,
                    crate::grant::Grantable::AlternativeCast(method)
                        if method.cast_from_zone() == Zone::Hand
                            && method.mana_cost().is_none()
                            && method.non_mana_costs().is_empty()
                )
            {
                return Ok(None);
            }
            Ok(static_grant_beneficiary(player)
                .map(|beneficiary| StaticAbility::grants(spec.with_beneficiary(beneficiary))))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_play_from_permission_with_haste_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [permission_sentence, haste_sentence] = sentences.as_slice() else {
        return Ok(None);
    };

    let haste_words = parser_token_word_refs(haste_sentence);
    if haste_words
        != [
            "if", "you", "cast", "a", "creature", "spell", "this", "way", "it", "gains",
            "haste", "until", "end", "of", "turn",
        ]
    {
        return Ok(None);
    }

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
            && spec.filter.card_types.as_slice() == [CardType::Creature] =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::haste()),
                )
            }))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_you_may_look_top_card_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_top_card_any_time_line_lexed(tokens) {
        return Ok(Some(StaticAbility::look_at_top_card_of_library()));
    }
    Ok(None)
}

pub(crate) fn parse_you_may_look_face_down_creatures_you_dont_control_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::look_at_face_down_creatures_you_dont_control(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_players_play_top_card_libraries_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_play_top_card_libraries_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_top_cards_of_libraries(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_play_top_card_your_library_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_top_card_your_library_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_your_top_library_card(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_your_opponents_play_with_hands_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_your_opponents_play_with_hands_revealed_line_lexed(tokens) {
        return Ok(Some(StaticAbility::opponents_play_with_hands_revealed()));
    }
    Ok(None)
}

pub(crate) fn parse_control_opponents_while_searching_libraries_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        [
            "you",
            "control",
            "your",
            "opponents",
            "while",
            "theyre" | "they're",
            "searching",
            "their",
            "libraries",
        ]
    ) {
        return Ok(Some(
            StaticAbility::control_opponents_while_searching_libraries(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_opponent_search_exile_found_cards_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words
        == [
            "while",
            "an",
            "opponent",
            "is",
            "searching",
            "their",
            "library",
            "they",
            "exile",
            "each",
            "card",
            "they",
            "find",
            "you",
            "may",
            "play",
            "those",
            "cards",
            "for",
            "as",
            "long",
            "as",
            "they",
            "remain",
            "exiled",
            "and",
            "you",
            "may",
            "spend",
            "mana",
            "as",
            "though",
            "it",
            "were",
            "mana",
            "of",
            "any",
            "color",
            "to",
            "cast",
            "them",
        ]
    {
        return Ok(Some(StaticAbility::opponent_search_exile_found_cards()));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_card_from_library_while_searching_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        [
            "while",
            "youre" | "you're",
            "searching",
            "your",
            "library",
            "you",
            "may",
            "cast",
            "this",
            "card",
            "from",
            "your",
            "library",
        ]
    ) {
        return Ok(Some(
            StaticAbility::cast_this_card_from_library_while_searching(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_spell_as_though_it_had_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_cast_this_spell_as_though_it_had_flash_line_lexed(tokens) {
        return Ok(Some(StaticAbility::flash()));
    }
    Ok(None)
}

pub(crate) fn parse_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        [
            "all",
            "creatures",
            "attack",
            "enchanted",
            "creatures" | "creature" | "creature's",
            "controller",
            "each",
            "combat",
            "if",
            "able",
        ]
    ) {
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::all_creatures_attack_attached_controller_each_combat_if_able(),
        )));
    }

    let Some(attack_idx) = find_index(&words, |word| *word == "attack" || *word == "attacks")
    else {
        return Ok(None);
    };
    if !word_slice_eq_any(
        &words[attack_idx..],
        &[
            &["attacks", "each", "combat", "if", "able"],
            &["attack", "each", "combat", "if", "able"],
        ],
    ) {
        return Ok(None);
    }

    if attack_idx == 0 {
        return Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())));
    }

    let subject_tokens = trim_commas(&tokens[..attack_idx]);
    if subject_tokens.is_empty() {
        return Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;
    match subject {
        AnthemSubjectAst::Source => {
            Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())))
        }
        AnthemSubjectAst::Filter(filter) => Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
            condition: None,
        })),
    }
}

pub(crate) fn parse_additional_land_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(&words, &["you", "may", "play"]) {
        return Ok(None);
    }

    let mut count_word_idx = 3;
    if word_slice_starts_with(&words[count_word_idx..], &["up", "to"]) {
        count_word_idx += 2;
    }

    let mut count_token_idx = None;
    let mut seen_word_idx = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_word_idx == count_word_idx {
            count_token_idx = Some(idx);
            break;
        }
        seen_word_idx += 1;
    }
    let Some(count_token_idx) = count_token_idx else {
        return Ok(None);
    };
    let Some((count, used)) = parse_number(&tokens[count_token_idx..]) else {
        return Ok(None);
    };
    let rest_word_idx = count_word_idx + used;
    if rest_word_idx >= words.len() {
        return Ok(None);
    }
    let rest_words = &words[rest_word_idx..];
    let is_match = matches!(
        rest_words,
        ["additional", "land", "on", "each", "of", "your", "turns"]
            | ["additional", "lands", "on", "each", "of", "your", "turns"]
    );
    if !is_match {
        return Ok(None);
    }
    if count == 0 {
        return Ok(None);
    }

    Ok(Some(vec![StaticAbility::additional_land_plays(count)]))
}

pub(crate) fn parse_play_lands_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_lands_from_graveyard_line_lexed(tokens) {
        let spec = crate::grant::GrantSpec::play_lands_from_graveyard();
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

pub(crate) fn parse_graveyard_cards_have_retrace_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(have_idx) = find_index(&words, |word| matches!(*word, "has" | "have")) else {
        return Ok(None);
    };
    if !word_slice_eq(&words[have_idx + 1..], &["retrace"]) {
        return Ok(None);
    }
    let mut prefix = &words[..have_idx];
    if word_slice_first_is(prefix, "each") {
        prefix = &prefix[1..];
    }
    if !word_slice_ends_with(prefix, &["in", "your", "graveyard"]) || prefix.len() <= 3 {
        return Ok(None);
    }

    let subject = &prefix[..prefix.len() - 3];
    let Some(card_types) = parse_retrace_grant_card_types(subject) else {
        return Ok(None);
    };
    let mut filter = ObjectFilter {
        card_types,
        owner: Some(PlayerFilter::You),
        ..ObjectFilter::default()
    };
    filter.zone = Some(Zone::Graveyard);
    let spec = crate::grant::GrantSpec::new(
        crate::grant::Grantable::retrace_from_cards_mana_cost(),
        filter,
        Zone::Graveyard,
    );
    Ok(Some(StaticAbility::grants(spec)))
}

fn parse_retrace_grant_card_types(words: &[&str]) -> Option<Vec<CardType>> {
    let mut card_types = Vec::new();
    for word in words {
        match *word {
            "instant" | "instants" => {
                if !card_types.contains(&CardType::Instant) {
                    card_types.push(CardType::Instant);
                }
            }
            "sorcery" | "sorceries" => {
                if !card_types.contains(&CardType::Sorcery) {
                    card_types.push(CardType::Sorcery);
                }
            }
            "and" | "or" | "card" | "cards" => {}
            _ => return None,
        }
    }
    (!card_types.is_empty()).then_some(card_types)
}

pub(crate) fn parse_cast_spells_from_hand_without_paying_mana_costs_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_find_any_phrase_start(
        &clause_words,
        &[&["cast", "a", "spell"], &["cast", "one", "spell"]],
    )
    .is_some()
    {
        return Ok(None);
    }
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec.zone == Zone::Hand
            && matches!(
                &spec.grantable,
                crate::grant::Grantable::AlternativeCast(method)
                    if method.cast_from_zone() == Zone::Hand
                        && method.mana_cost().is_none()
                        && method.non_mana_costs().is_empty()
            ) =>
        {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

pub(crate) fn parse_pt_modifier(raw: &str) -> Result<(i32, i32), CardTextError> {
    let parts: Vec<&str> = raw.split('/').collect();
    if parts.len() != 2 {
        return Err(CardTextError::ParseError(
            "missing power/toughness modifier".to_string(),
        ));
    }
    let power_str = parts[0].trim_start_matches('+');
    let toughness_str = parts[1].trim_start_matches('+');
    let power = power_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power modifier".to_string()))?;
    let toughness = toughness_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid toughness modifier".to_string()))?;
    Ok((power, toughness))
}

pub(crate) fn parse_signed_pt_component(raw: &str) -> Result<Value, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CardTextError::ParseError(
            "missing power/toughness component".to_string(),
        ));
    }

    let (sign, value_text) = if let Some(rest) = str_strip_prefix(trimmed, "+") {
        (1, rest)
    } else if let Some(rest) = str_strip_prefix(trimmed, "-") {
        (-1, rest)
    } else {
        (1, trimmed)
    };

    if value_text.eq_ignore_ascii_case("x") {
        return Ok(match sign {
            1 => Value::X,
            -1 => Value::XTimes(-1),
            _ => Value::XTimes(sign),
        });
    }

    let parsed = value_text
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power/toughness component".to_string()))?;
    Ok(Value::Fixed(parsed * sign))
}

pub(crate) fn parse_pt_modifier_values(raw: &str) -> Result<(Value, Value), CardTextError> {
    let parts: Vec<&str> = raw.split('/').collect();
    if parts.len() != 2 {
        return Err(CardTextError::ParseError(
            "missing power/toughness modifier".to_string(),
        ));
    }

    let power = parse_signed_pt_component(parts[0])?;
    let toughness = parse_signed_pt_component(parts[1])?;
    Ok((power, toughness))
}

pub(crate) fn parse_no_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_no_maximum_hand_size_line_lexed(tokens) {
        return Ok(Some(StaticAbility::no_maximum_hand_size()));
    }
    Ok(None)
}

pub(crate) fn parse_can_be_your_commander_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_be_your_commander_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_be_commander()));
    }
    Ok(None)
}

pub(crate) fn parse_reduced_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let max_hand_size_subject_prefix_len = |tail: &[&str]| -> Option<usize> {
        if word_slice_starts_with_any(&tail, &[&["your"], &["you"]]) {
            Some(1)
        } else if word_slice_starts_with(&tail, &["each", "opponent's"]) {
            Some(2)
        } else if word_slice_starts_with(&tail, &["each", "opponent", "s"]) {
            Some(3)
        } else if word_slice_starts_with_any(
            &tail,
            &[&["each", "opponent"], &["each", "opponents"]],
        ) {
            Some(2)
        } else if word_slice_starts_with(&tail, &["opponent's"]) {
            Some(1)
        } else if word_slice_starts_with(&tail, &["opponent", "s"]) {
            Some(2)
        } else if word_slice_starts_with_any(&tail, &[&["opponent"], &["opponents"]]) {
            Some(1)
        } else if word_slice_starts_with(&tail, &["each", "player's"]) {
            Some(2)
        } else if word_slice_starts_with(&tail, &["each", "player", "s"]) {
            Some(3)
        } else if word_slice_starts_with_any(&tail, &[&["each", "player"], &["each", "players"]]) {
            Some(2)
        } else if word_slice_starts_with(&tail, &["player's"]) {
            Some(1)
        } else if word_slice_starts_with(&tail, &["player", "s"]) {
            Some(2)
        } else if word_slice_starts_with_any(&tail, &[&["player"], &["players"]]) {
            Some(1)
        } else {
            None
        }
    };

    let mut min_card_types_condition: Option<u32> = None;
    let mut line_words = crate::runtime_backend::token_word_refs(tokens);
    if line_words.is_empty() {
        return Ok(None);
    }

    let working_tokens_storage = if word_slice_starts_with(&line_words, &["as", "long", "as"]) {
        let (condition_end_idx, remainder_start_idx) =
            if let Some(comma_idx) = find_index(tokens, |token| token.is_comma()) {
                if comma_idx <= 3 {
                    return Ok(None);
                }
                (comma_idx, comma_idx + 1)
            } else {
                let mut split_word_idx = None;
                for word_idx in 4..line_words.len() {
                    let tail = &line_words[word_idx..];
                    let Some(prefix_len) = max_hand_size_subject_prefix_len(tail) else {
                        continue;
                    };
                    if tail.get(prefix_len..prefix_len + 4)
                        == Some(["maximum", "hand", "size", "is"].as_slice())
                    {
                        split_word_idx = Some(word_idx);
                        break;
                    }
                }
                let Some(split_word_idx) = split_word_idx else {
                    return Ok(None);
                };
                let split_token_idx = token_index_for_word_index(tokens, split_word_idx)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unable to map delirium hand-size subject split (clause: '{}')",
                            line_words.join(" ")
                        ))
                    })?;
                (split_token_idx, split_token_idx)
            };

        let condition_tokens = trim_commas(&tokens[3..condition_end_idx]);
        let Some((metric, threshold)) =
            parse_graveyard_metric_threshold_condition(&condition_tokens)?
        else {
            return Ok(None);
        };
        if metric != crate::static_abilities::GraveyardCountMetric::CardTypes {
            return Ok(None);
        }
        min_card_types_condition = Some(threshold);
        Some(trim_commas(&tokens[remainder_start_idx..]))
    } else {
        None
    };
    let working_tokens = working_tokens_storage.as_deref().unwrap_or(tokens);
    line_words = crate::runtime_backend::token_word_refs(working_tokens);
    if line_words.is_empty() {
        return Ok(None);
    }

    let (player, mut idx) = if word_slice_starts_with_any(&line_words, &[&["your"], &["you"]]) {
        (crate::target::PlayerFilter::You, 1usize)
    } else if word_slice_starts_with(&line_words, &["each", "opponent's"]) {
        (crate::target::PlayerFilter::Opponent, 2usize)
    } else if word_slice_starts_with_any(
        &line_words,
        &[
            &["each", "opponent"],
            &["each", "opponents"],
            &["each", "opponent", "s"],
        ],
    ) {
        (
            crate::target::PlayerFilter::Opponent,
            if word_slice_starts_with(&line_words, &["each", "opponent", "s"]) {
                3usize
            } else {
                2usize
            },
        )
    } else if word_slice_starts_with(&line_words, &["opponent's"]) {
        (crate::target::PlayerFilter::Opponent, 1usize)
    } else if word_slice_starts_with_any(
        &line_words,
        &[&["opponent"], &["opponents"], &["opponent", "s"]],
    ) {
        (
            crate::target::PlayerFilter::Opponent,
            if word_slice_starts_with(&line_words, &["opponent", "s"]) {
                2usize
            } else {
                1usize
            },
        )
    } else if word_slice_starts_with(&line_words, &["each", "player's"]) {
        (crate::target::PlayerFilter::Any, 2usize)
    } else if word_slice_starts_with_any(
        &line_words,
        &[
            &["each", "player"],
            &["each", "players"],
            &["each", "player", "s"],
        ],
    ) {
        (
            crate::target::PlayerFilter::Any,
            if word_slice_starts_with(&line_words, &["each", "player", "s"]) {
                3usize
            } else {
                2usize
            },
        )
    } else if word_slice_starts_with(&line_words, &["player's"]) {
        (crate::target::PlayerFilter::Any, 1usize)
    } else if word_slice_starts_with_any(
        &line_words,
        &[&["player"], &["players"], &["player", "s"]],
    ) {
        (
            crate::target::PlayerFilter::Any,
            if word_slice_starts_with(&line_words, &["player", "s"]) {
                2usize
            } else {
                1usize
            },
        )
    } else {
        return Ok(None);
    };

    if word_slice_starts_with_at(
        &line_words,
        idx,
        &["maximum", "hand", "size", "is", "reduced"],
    ) {
        idx += 5;
        if !word_slice_at_is(&line_words, idx, "by") {
            return Ok(None);
        }
        idx += 1;

        let Some(amount_word) = line_words.get(idx) else {
            return Err(CardTextError::ParseError(format!(
                "missing maximum-hand-size reduction amount (clause: '{}')",
                line_words.join(" ")
            )));
        };
        let Some(amount) = parse_named_number(amount_word) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported maximum-hand-size reduction amount '{}' (clause: '{}')",
                amount_word,
                line_words.join(" ")
            )));
        };
        idx += 1;

        if idx != line_words.len() {
            return Ok(None);
        }

        return Ok(Some(StaticAbility::reduce_maximum_hand_size(
            player, amount,
        )));
    }

    if word_slice_starts_with_at(&line_words, idx, &["maximum", "hand", "size", "is"]) {
        idx += 4;

        if line_words.get(idx..idx + 10)
            == Some(
                [
                    "equal", "to", "seven", "minus", "the", "number", "of", "those", "card",
                    "types",
                ]
                .as_slice(),
            )
            || line_words.get(idx..idx + 10)
                == Some(
                    [
                        "equal", "to", "seven", "minus", "the", "number", "of", "those", "card",
                        "type",
                    ]
                    .as_slice(),
                )
        {
            idx += 10;
            if idx != line_words.len() {
                return Ok(None);
            }
            return Ok(Some(
                StaticAbility::max_hand_size_seven_minus_your_graveyard_card_types(
                    player,
                    min_card_types_condition.unwrap_or(0),
                ),
            ));
        }

        let Some(amount_word) = line_words.get(idx) else {
            return Err(CardTextError::ParseError(format!(
                "missing maximum-hand-size value (clause: '{}')",
                line_words.join(" ")
            )));
        };
        let Some(amount) = parse_named_number(amount_word) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported maximum-hand-size value '{}' (clause: '{}')",
                amount_word,
                line_words.join(" ")
            )));
        };
        idx += 1;
        if idx != line_words.len() {
            return Ok(None);
        }

        return Ok(Some(StaticAbility::set_maximum_hand_size(player, amount)));
    }
    Ok(None)
}

pub(crate) fn parse_effect_discard_to_library_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_effect_discard_to_library_replacement_line_lexed(tokens) {
        return Ok(Some(StaticAbility::effect_discard_to_library_replacement()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replace_exile_top_face_down_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replace_exile_top_face_down_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_exile_top_face_down()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replacement_exile_top_and_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words.len() < 20 {
        return Ok(None);
    }

    if !word_slice_starts_with(
        &words,
        &[
            "if", "you", "would", "draw", "a", "card", "exile", "the", "top",
        ],
    ) {
        return Ok(None);
    }

    let Some(count_word) = words.get(9).copied() else {
        return Ok(None);
    };
    let Some(count) = parse_named_number(count_word) else {
        return Ok(None);
    };

    if !word_slice_at_is_any(&words, 10, &["card", "cards"]) {
        return Ok(None);
    }

    let tail = [
        "of", "your", "library", "instead", "you", "may", "play", "those", "cards", "this", "turn",
    ];
    if !word_slice_eq_at(&words, 11, &tail) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::draw_replacement_exile_top_and_play(
        count,
    )))
}

pub(crate) fn parse_draw_replacement_double_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_double_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_double()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replacement_skip_empty_library_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_skip_empty_library_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_skip_empty_library()));
    }

    Ok(None)
}

fn parse_conditional_draw_replacement_amount(word: &str) -> Option<u32> {
    word.parse::<u32>()
        .ok()
        .or_else(|| parse_named_number(word))
}

pub(crate) fn parse_conditional_draw_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let draw_subject_len = if word_slice_starts_with(
        &words,
        &["if", "you", "would", "draw", "a", "card", "while"],
    ) {
        7
    } else if word_slice_starts_with(&words, &["if", "you", "would", "draw", "card", "while"]) {
        6
    } else {
        return Ok(None);
    };

    let Some(instead_idx) = find_index(&words[draw_subject_len..], |word| *word == "instead")
    else {
        return Ok(None);
    };
    let instead_idx = draw_subject_len + instead_idx;
    if !word_slice_eq(
        &words[draw_subject_len..instead_idx],
        &["you", "have", "no", "cards", "in", "hand"],
    ) {
        return Ok(None);
    }

    let effect_words = &words[instead_idx + 1..];
    let draw_idx = if word_slice_first_is(effect_words, "you") {
        1
    } else {
        0
    };
    if effect_words.get(draw_idx).copied() != Some("draw") {
        return Ok(None);
    }
    let Some(draw_count_word) = effect_words.get(draw_idx + 1).copied() else {
        return Ok(None);
    };
    let Some(draw_count) = parse_conditional_draw_replacement_amount(draw_count_word) else {
        return Ok(None);
    };
    if !matches!(
        effect_words.get(draw_idx + 2).copied(),
        Some("card" | "cards")
    ) {
        return Ok(None);
    }

    let mut next_idx = draw_idx + 3;
    if word_slice_at_is(effect_words, next_idx, "instead") {
        next_idx += 1;
    }

    let mut replacement_effects = vec![Effect::draw(draw_count as i32)];
    let mut life_loss = None;
    if next_idx < effect_words.len() {
        let tail = &effect_words[next_idx..];
        if tail.len() != 5
            || tail[0] != "and"
            || tail[1] != "you"
            || tail[2] != "lose"
            || tail[4] != "life"
        {
            return Ok(None);
        }
        let Some(amount) = parse_conditional_draw_replacement_amount(tail[3]) else {
            return Ok(None);
        };
        life_loss = Some(amount);
        replacement_effects.push(Effect::lose_life(amount as i32));
    }

    let draw_amount_text = match draw_count {
        1 => "a".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        _ => draw_count.to_string(),
    };
    let draw_card_text = if draw_count == 1 { "card" } else { "cards" };
    let mut display = format!(
        "If you would draw a card while you have no cards in hand, instead you draw {draw_amount_text} {draw_card_text}"
    );
    if let Some(amount) = life_loss {
        display.push_str(&format!(" and you lose {amount} life"));
    }
    display.push('.');

    Ok(Some(StaticAbility::conditional_draw_replacement(
        Condition::Not(Box::new(Condition::CardsInHandOrMore(1))),
        replacement_effects,
        display,
    )))
}

fn keyword_action_replacement_subject_explores(words: &[&str]) -> bool {
    word_slice_eq_any(
        words,
        &[&["it", "explores"], &["that", "creature", "explores"]],
    )
}

pub(crate) fn parse_keyword_action_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = parser_token_word_refs(tokens);
    let proliferate_display = render_token_slice(tokens);
    if line_words
        == [
            "if",
            "you",
            "would",
            "proliferate",
            "proliferate",
            "twice",
            "instead",
        ]
    {
        return Ok(Some(StaticAbility::keyword_action_replacement(
            crate::events::KeywordActionKind::Proliferate,
            ObjectFilter::default().controlled_by(PlayerFilter::You),
            vec![Effect::proliferate(2)],
            proliferate_display,
        )));
    }
    if line_words
        == [
            "if",
            "an",
            "opponent",
            "would",
            "proliferate",
            "that",
            "player",
            "proliferates",
            "twice",
            "instead",
        ]
    {
        return Ok(Some(StaticAbility::keyword_action_replacement(
            crate::events::KeywordActionKind::Proliferate,
            ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
            vec![Effect::proliferate(2)],
            proliferate_display,
        )));
    }

    let prefix = [
        "if", "a", "creature", "you", "control", "would", "explore", "instead",
    ];
    if !word_slice_starts_with(&line_words, &prefix) {
        return Ok(None);
    }
    let tail = &line_words[prefix.len()..];
    let explored_creature = ChooseSpec::tagged(IT_TAG);
    let explore_effect = || Effect::explore(explored_creature.clone());
    let source_filter = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    let display = render_token_slice(tokens);

    if let Some(then_idx) = word_slice_find_word(tail, "then")
        && word_slice_starts_with(tail, &["you", "scry"])
        && keyword_action_replacement_subject_explores(&tail[then_idx + 1..])
    {
        let value_tokens = &tail[2..then_idx];
        let (count, used) = parse_value_expr_words(value_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported scry amount in keyword-action replacement (clause: '{}')",
                render_token_slice(tokens)
            ))
        })?;
        if used != value_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported scry amount in keyword-action replacement (clause: '{}')",
                render_token_slice(tokens)
            )));
        }
        return Ok(Some(StaticAbility::keyword_action_replacement(
            crate::events::KeywordActionKind::Explore,
            source_filter,
            vec![Effect::scry(count), explore_effect()],
            display,
        )));
    }

    if word_slice_eq(tail, &["it", "explores", "then", "it", "explores", "again"]) {
        return Ok(Some(StaticAbility::keyword_action_replacement(
            crate::events::KeywordActionKind::Explore,
            source_filter,
            vec![explore_effect(), explore_effect()],
            display,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_exile_to_countered_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(tokens) else {
        return Ok(None);
    };

    Ok(Some(
        StaticAbility::exile_to_countered_exile_instead_of_graveyard(
            spec.player,
            spec.counter_type,
        ),
    ))
}

pub(crate) fn parse_exile_to_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if !word_slice_starts_with(&words, &["if"])
        || !word_slice_contains_word(&words, "would")
        || !word_slice_contains_word(&words, "graveyard")
        || !word_slice_contains_word(&words, "anywhere")
        || !word_slice_ends_with(&words, &["exile", "it", "instead"])
    {
        return Ok(None);
    }

    let Some(would_idx) = word_slice_find_word(&words, "would") else {
        return Ok(None);
    };
    let Some(graveyard_idx) = word_slice_find_word(&words, "graveyard") else {
        return Ok(None);
    };
    let Some(graveyard_owner) = words
        .get(would_idx..graveyard_idx)
        .and_then(parse_would_be_put_into_graveyard_owner_words)
    else {
        return Ok(None);
    };
    let exclude_cycled = words.get(graveyard_idx + 1..).is_some_and(|tail| {
        word_slice_contains_any_phrase(
            tail,
            &[
                &["and", "it", "wasnt", "cycled"],
                &["and", "it", "wasn't", "cycled"],
            ],
        )
    });

    let filter_tokens = trim_lexed_commas(&tokens[1..would_idx]);
    let filter_words = &words[1..would_idx];
    let filter = if is_source_reference_words(filter_words) {
        ObjectFilter::source()
    } else if matches!(
        filter_words,
        ["a", "card", "or", "token"] | ["card", "or", "token"]
    ) {
        ObjectFilter::default()
    } else {
        parse_object_filter(filter_tokens, false).or_else(|_| match filter_words {
            ["a", "card"] | ["card"] => Ok(ObjectFilter::default()),
            ["a", "creature", "card"] | ["creature", "card"] => Ok(ObjectFilter::creature()),
            ["a", "card", "that", "has", "a", "cycling", "ability"]
            | ["card", "that", "has", "a", "cycling", "ability"] => {
                Ok(ObjectFilter::default().with_ability_marker("cycling"))
            }
            _ => Err(CardTextError::ParseError(format!(
                "unsupported exile-to-graveyard replacement subject (subject: '{}')",
                filter_words.join(" ")
            ))),
        })?
    };
    let ability = if exclude_cycled {
        StaticAbility::exile_to_exile_instead_of_graveyard_unless_cycled(filter, graveyard_owner)
    } else {
        StaticAbility::exile_to_exile_instead_of_graveyard(filter, graveyard_owner)
    };
    Ok(Some(ability))
}

fn parse_would_be_put_into_graveyard_owner_words(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["would", "be", "put", "into", "a"]
        | ["would", "be", "put", "into", "a", "players"]
        | ["would", "be", "put", "into", "a", "player's"] => Some(PlayerFilter::Any),
        ["would", "be", "put", "into", "your"] => Some(PlayerFilter::You),
        ["would", "be", "put", "into", "an", "opponents"]
        | ["would", "be", "put", "into", "an", "opponent's"]
        | ["would", "be", "put", "into", "opponents"]
        | ["would", "be", "put", "into", "opponent's"] => Some(PlayerFilter::Opponent),
        _ => None,
    }
}

pub(crate) fn parse_exile_would_die_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let nontoken_opponent_prefix = [
        "if", "a", "nontoken", "creature", "an", "opponent", "controls", "would", "die",
    ];
    let nontoken_opponent_prefix_no_article = [
        "if", "nontoken", "creature", "opponent", "controls", "would", "die",
    ];
    let nontoken_any_prefix = ["if", "a", "nontoken", "creature", "would", "die"];
    let nontoken_any_prefix_no_article = ["if", "nontoken", "creature", "would", "die"];
    let matched_nontoken_filter = if word_slice_starts_with(&words, &nontoken_opponent_prefix) {
        Some((
            ObjectFilter::creature()
                .nontoken()
                .controlled_by(PlayerFilter::Opponent),
            nontoken_opponent_prefix.len(),
        ))
    } else if word_slice_starts_with(&words, &nontoken_opponent_prefix_no_article) {
        Some((
            ObjectFilter::creature()
                .nontoken()
                .controlled_by(PlayerFilter::Opponent),
            nontoken_opponent_prefix_no_article.len(),
        ))
    } else if word_slice_starts_with(&words, &nontoken_any_prefix) {
        Some((
            ObjectFilter::creature().nontoken(),
            nontoken_any_prefix.len(),
        ))
    } else if word_slice_starts_with(&words, &nontoken_any_prefix_no_article) {
        Some((
            ObjectFilter::creature().nontoken(),
            nontoken_any_prefix_no_article.len(),
        ))
    } else {
        None
    };
    if let Some((matched_filter, prefix_len)) = matched_nontoken_filter {
        let tail = &words[prefix_len..];
        let (exile_with_counters, follow_up) = match tail {
            [
                "exile",
                "that",
                "card",
                "instead",
                "and",
                "create",
                "a",
                "2/2",
                "black",
                "zombie",
                "creature",
                "token",
            ]
            | [
                "instead",
                "exile",
                "that",
                "card",
                "and",
                "create",
                "a",
                "2/2",
                "black",
                "zombie",
                "creature",
                "token",
            ] => (
                Vec::new(),
                vec![Effect::create_tokens(kalitas_zombie_token(), 1)],
            ),
            [
                "exile",
                "that",
                "card",
                "with",
                "an",
                "ice",
                "counter",
                "on",
                "it",
                "instead",
            ] => (vec![(CounterType::Ice, 1)], Vec::new()),
            ["exile", "that", "card", "instead"] | ["exile", "it", "instead"] => {
                (Vec::new(), Vec::new())
            }
            [
                "instead",
                "exile",
                "that",
                "card",
                "with",
                "a",
                counter_word,
                "counter",
                "on",
                "it",
            ]
            | [
                "instead",
                "exile",
                "it",
                "with",
                "a",
                counter_word,
                "counter",
                "on",
                "it",
            ]
            | [
                "exile",
                "that",
                "card",
                "with",
                "a",
                counter_word,
                "counter",
                "on",
                "it",
                "instead",
            ]
            | [
                "exile",
                "it",
                "with",
                "a",
                counter_word,
                "counter",
                "on",
                "it",
                "instead",
            ] => {
                let Some(counter_type) = parse_counter_type_word(counter_word) else {
                    return Ok(None);
                };
                (vec![(counter_type, 1)], Vec::new())
            }
            _ => return Ok(None),
        };
        return Ok(Some(
            StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                matched_filter,
                None,
                exile_with_counters,
                follow_up,
            ),
        ));
    }

    if let Some(dealt_idx) = word_slice_find_word(&words, "dealt")
        && word_slice_starts_with(&words[dealt_idx + 1..], &["damage", "by"])
        && word_slice_ends_with(
            &words,
            &["this", "turn", "would", "die", "exile", "it", "instead"],
        )
    {
        let victim_words = &words[1..dealt_idx];
        let victim = match victim_words {
            ["a", "creature"] | ["creature"] => ObjectFilter::creature(),
            ["a", "permanent"] | ["permanent"] => ObjectFilter::permanent(),
            _ => return Ok(None),
        };

        let damager_words = &words[dealt_idx + 3..words.len() - 7];
        let has_named_source_words = !damager_words.is_empty()
            && !matches!(
                damager_words.first().copied(),
                Some("a" | "an" | "the" | "target" | "that" | "this" | "equipped" | "enchanted")
            )
            && !damager_words.iter().any(|word| {
                matches!(
                    *word,
                    "creature" | "creatures" | "permanent" | "permanents" | "source" | "sources"
                )
            });
        let damaged_by = if word_slice_eq_any(
            &damager_words,
            &[
                &["this", "creature"],
                &["this", "permanent"],
                &["this", "source"],
                &["this"],
            ],
        ) || has_named_source_words
        {
            Some(ironsmith_core::DamagedBySource::ThisCreature)
        } else if word_slice_eq(&damager_words, &["equipped", "creature"]) {
            Some(ironsmith_core::DamagedBySource::EquippedCreature)
        } else if word_slice_eq(&damager_words, &["enchanted", "creature"]) {
            Some(ironsmith_core::DamagedBySource::EnchantedCreature)
        } else {
            None
        };

        if let Some(damaged_by) = damaged_by {
            return Ok(Some(
                StaticAbility::exile_would_die_instead_with_damage_source(victim, Some(damaged_by)),
            ));
        }
    }

    let player = match words.as_slice() {
        [
            "if",
            "a",
            "creature",
            "an",
            "opponent",
            "controls",
            "would",
            "die",
            "exile",
            "it",
            "instead",
        ]
        | [
            "if",
            "creature",
            "an",
            "opponent",
            "controls",
            "would",
            "die",
            "exile",
            "it",
            "instead",
        ] => PlayerFilter::Opponent,
        [
            "if",
            "a",
            "creature",
            "you",
            "control",
            "would",
            "die",
            "exile",
            "it",
            "instead",
        ]
        | [
            "if",
            "creature",
            "you",
            "control",
            "would",
            "die",
            "exile",
            "it",
            "instead",
        ] => PlayerFilter::You,
        [
            "if",
            "a",
            "creature",
            "would",
            "die",
            "exile",
            "it",
            "instead",
        ]
        | ["if", "creature", "would", "die", "exile", "it", "instead"] => PlayerFilter::Any,
        _ => return Ok(None),
    };

    Ok(Some(StaticAbility::exile_would_die_instead(
        ObjectFilter::creature().controlled_by(player),
    )))
}

fn kalitas_zombie_token() -> crate::cards::CardDefinition {
    crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zombie")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .color_indicator(ColorSet::BLACK)
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build()
}

pub(crate) fn parse_discard_or_redirect_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_discard_or_redirect_replacement_line_lexed(tokens) {
        return Ok(Some(StaticAbility::discard_or_redirect_replacement(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_pay_life_or_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized_storage = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .map(|word| match word.replace('’', "'").as_str() {
            "don't" => "dont".to_string(),
            _ => word.to_string(),
        })
        .collect::<Vec<_>>();
    let words = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if words.len() < 8 {
        return Ok(None);
    }

    let starts_with_as_this = word_slice_starts_with(&words, &["as", "this"]);
    let has_pay = word_slice_contains_word(&words, "pay");
    let has_life = word_slice_contains_word(&words, "life");
    if !starts_with_as_this || !has_pay || !has_life {
        return Ok(None);
    }

    let Some(pay_idx) = find_index(tokens, |token| token.is_word("pay")) else {
        return Err(CardTextError::ParseError(format!(
            "missing 'pay' keyword in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };
    if !words[..pay_idx]
        .iter()
        .any(|word| *word == "enter" || *word == "enters")
    {
        return Ok(None);
    }
    if !word_slice_contains_word(&words[..pay_idx], "may") {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-life ETB prefix (clause: '{}')",
            words.join(" ")
        )));
    }

    let Some((value, _)) = parse_number(&tokens[pay_idx + 1..]) else {
        return Err(CardTextError::ParseError(format!(
            "missing life payment amount in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let if_dont_idx = word_slice_find_phrase_start(&words, &["if", "you", "dont"]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported pay-life ETB trailing clause (expected 'if you don't ...') (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let trailing = &words[if_dont_idx + 3..];
    let valid_trailing = word_slice_starts_with_any(
        &trailing,
        &[
            &["it", "enters", "tapped"],
            &["it", "enter", "tapped"],
            &["it", "enters", "the", "battlefield", "tapped"],
            &["it", "enter", "the", "battlefield", "tapped"],
        ],
    );
    if !valid_trailing {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-life ETB trailing clause (clause: '{}')",
            words.join(" ")
        )));
    };

    parser_trace("parse_static:pay-life-etb:matched", tokens);
    Ok(Some(StaticAbility::pay_life_or_enter_tapped(value)))
}

pub(crate) fn parse_copy_activated_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }

    let mut has_idx = None;
    for idx in 0..clause_words.len().saturating_sub(4) {
        if (clause_words[idx] == "has" || clause_words[idx] == "have")
            && clause_words[idx + 1] == "all"
            && clause_words[idx + 2] == "activated"
            && clause_words[idx + 3] == "abilities"
            && clause_words[idx + 4] == "of"
        {
            has_idx = Some(idx);
            break;
        }
    }
    let Some(has_idx) = has_idx else {
        return Ok(None);
    };

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let filter_tokens = trim_edge_punctuation(&tokens[(has_idx + 5)..]);
    let mut filter_tokens =
        strip_leading_token_words_any(&filter_tokens, &["all", "each"]).to_vec();
    let after_of_words = crate::runtime_backend::lexer::parser_token_word_refs(&filter_tokens);
    let once_each_turn_start = word_slice_find_phrase_start(
        &after_of_words,
        &[
            "you",
            "may",
            "activate",
            "each",
            "of",
            "those",
            "abilities",
            "only",
            "once",
            "each",
            "turn",
        ],
    );
    let force_once_each_turn = once_each_turn_start.is_some();
    if let Some(start) = once_each_turn_start
        && let Some(token_idx) = token_index_for_word_index(&filter_tokens, start)
    {
        filter_tokens.truncate(token_idx);
        filter_tokens = trim_edge_punctuation(&filter_tokens);
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let counter = parse_counter_type_from_tokens(&filter_tokens);

    let exclude_source_name = find_window_by(&clause_words, 5, |window| {
        word_slice_eq_any(
            window,
            &[
                &["same", "name", "as", "this", "creature"],
                &["same", "name", "as", "thiss", "creature"],
            ],
        )
    })
    .is_some();
    let display = if force_once_each_turn {
        if let Some(start) = find_window_by(&clause_words, 11, |window| {
            window
                .iter()
                .zip([
                    "you",
                    "may",
                    "activate",
                    "each",
                    "of",
                    "those",
                    "abilities",
                    "only",
                    "once",
                    "each",
                    "turn",
                ])
                .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
        }) {
            format!(
                "{}. You may activate each of those abilities only once each turn",
                clause_words[..start].join(" ").trim()
            )
        } else {
            clause_words.join(" ")
        }
    } else {
        clause_words.join(" ")
    };

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(display);
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if force_once_each_turn {
        ability = ability.with_once_each_turn();
    }

    let ability = StaticAbility::copy_activated_abilities(ability);
    let ast = match subject {
        AnthemSubjectAst::Source => match condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition,
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(subject_filter) => StaticAbilityAst::GrantStaticAbility {
            filter: subject_filter,
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition,
        },
    };

    Ok(Some(ast))
}

pub(crate) fn parse_spend_mana_as_any_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_starts_with_any(
        &clause_words,
        &[
            &[
                "you", "can", "spend", "mana", "of", "any", "type", "to", "cast",
            ],
            &[
                "you", "may", "spend", "mana", "of", "any", "type", "to", "cast",
            ],
        ],
    ) {
        let filter_start = 9usize;
        let filter_tokens = trim_edge_punctuation(&tokens[filter_start..]);
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let filter = parse_object_filter(&filter_tokens, false)
            .map(|mut filter| {
                filter.zone = None;
                filter.stack_kind = None;
                filter.has_mana_cost = false;
                filter
            })
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported mana spend cast filter (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::mana_spend_permission(
                crate::effect::ManaSpendPermission::any_color_for_casting_matching(
                    PlayerFilter::You,
                    filter,
                ),
                clause_words.join(" "),
            ),
        )));
    }

    let (player, tail_start, display) = if word_slice_starts_with(
        &clause_words,
        &[
            "players", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
            "color",
        ],
    ) {
        (
            PlayerFilter::Any,
            12usize,
            "Players may spend mana as though it were mana of any color".to_string(),
        )
    } else if word_slice_starts_with(
        &clause_words,
        &[
            "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
            "color",
        ],
    ) {
        (PlayerFilter::You, 12usize, clause_words.join(" "))
    } else {
        return Ok(None);
    };

    let tail_tokens = trim_edge_punctuation(&tokens[tail_start..]);
    let permission = if tail_tokens.is_empty() {
        crate::effect::ManaSpendPermission::any_color(player)
    } else {
        let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
        if word_slice_starts_with(
            &tail_words,
            &["to", "pay", "the", "activation", "costs", "of"],
        ) {
            let ability_words = crate::runtime_backend::token_word_refs(&tail_tokens[6..]);
            if !ability_words
                .iter()
                .any(|word| *word == "abilities" || *word == "ability")
            {
                return Ok(None);
            }
            crate::effect::ManaSpendPermission::any_color_for_activation(
                player,
                ObjectFilter::source(),
            )
        } else if word_slice_starts_with(&tail_words, &["to", "activate", "abilities", "of"]) {
            let filter_tokens = trim_edge_punctuation(&tail_tokens[4..]);
            if filter_tokens.is_empty() {
                return Ok(None);
            }
            let filter = match parse_object_filter(&filter_tokens, false) {
                Ok(filter) => filter,
                Err(_) => return Ok(None),
            };
            crate::effect::ManaSpendPermission::any_color_for_activation(player, filter)
        } else {
            return Ok(None);
        }
    };

    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::mana_spend_permission(permission, display),
    )))
}
include!("keyword_lines.rs");
include!("anthem_grant_lines.rs");
include!("etb_static_lines.rs");
include!("attached_object_static_lines.rs");
