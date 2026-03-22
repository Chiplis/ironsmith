use super::activation_and_restrictions::parse_cycling_line;
use super::activation_and_restrictions::{
    normalize_cant_words, parse_ability_phrase, parse_activated_line, parse_activation_cost,
    parse_triggered_line,
};
use super::keyword_static_helpers::*;
use super::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};
use super::lowering_support::rewrite_parsed_triggered_ability as parsed_triggered_ability;
use super::native_tokens::LowercaseWordView;
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::util::{
    is_source_reference_words, parse_card_type, parse_color, parse_counter_type_from_tokens,
    parse_counter_type_word, parse_flashback_keyword_line, parse_subtype_flexible, parse_value,
    parse_zone_word, mana_pips_from_token, split_on_and, split_on_comma_or_semicolon,
    split_on_period, trim_commas, words,
};
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
use crate::color::ColorSet;
#[allow(unused_imports)]
use crate::cost::TotalCost;
#[allow(unused_imports)]
use crate::effect::{Effect, EventValueSpec, Value};
#[allow(unused_imports)]
use crate::mana::{ManaCost, ManaSymbol};
#[allow(unused_imports)]
use crate::object::CounterType;
#[allow(unused_imports)]
use crate::static_abilities::{
    Anthem, AnthemCountExpression, AnthemValue, GrantAbility, StaticAbility,
};
#[allow(unused_imports)]
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
#[allow(unused_imports)]
use crate::triggers::Trigger;
#[allow(unused_imports)]
use crate::types::{CardType, Subtype, Supertype};
#[allow(unused_imports)]
use crate::zone::Zone;
use std::sync::LazyLock;

fn lowercase_word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut lowered = tokens.to_vec();
    for token in &mut lowered {
        if let Some(word) = token.word_mut() {
            *word = word.to_ascii_lowercase();
        }
    }
    lowered
}

include!("keyword_lines.rs");
include!("anthem_grant_lines.rs");
include!("etb_static_lines.rs");
include!("attached_object_static_lines.rs");

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

struct StaticAbilityLineRuleIndex {
    by_head: std::collections::HashMap<&'static str, Vec<usize>>,
}

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

fn build_static_ability_line_rule_index(
    rules: &'static [StaticAbilityLineRuleDef],
) -> StaticAbilityLineRuleIndex {
    let mut by_head = std::collections::HashMap::<&'static str, Vec<usize>>::new();
    for (idx, rule) in rules.iter().enumerate() {
        for head in static_ability_rule_head_hints(rule.id) {
            by_head.entry(head).or_default().push(idx);
        }
    }
    StaticAbilityLineRuleIndex { by_head }
}

fn static_ability_rule_head_hints(rule_id: &'static str) -> &'static [&'static str] {
    match rule_id {
        "parse_characteristic_defining_pt_line" => &[],
        "parse_reduced_maximum_hand_size_line" => &[
            "your",
            "you",
            "each",
            "opponent",
            "opponents",
            "player",
            "players",
            "as",
        ],
        "parse_conditional_source_spell_keyword_line" => &["if"],
        "parse_conditional_all_creatures_able_to_block_line" => &["as"],
        "parse_toph_first_metalbender_line" => &["the"],
        "parse_spell_cost_increase_per_target_beyond_first_line" => &["this"],
        "parse_double_damage_from_sources_you_control_of_chosen_type_line" => &["double"],
        "parse_source_can_attack_as_though_no_defender_as_long_as_line" => &["this"],
        "parse_no_maximum_hand_size_line" => &["you"],
        "parse_you_may_static_grant_line" => &["you"],
        "parse_you_may_look_top_card_any_time_line" => &["you"],
        "parse_additional_land_play_line" => &["you"],
        "parse_play_lands_from_graveyard_line" => &["you"],
        "parse_pregame_choose_color_line" => &["if", "choose"],
        "parse_legend_rule_doesnt_apply_line" => &["the"],
        _ => match rule_id
            .strip_prefix("parse_")
            .and_then(|id| id.split('_').next())
        {
            Some("ward") => &["ward"],
            Some("skulk") => &["skulk"],
            Some("if") => &["if"],
            Some("choose") => &["choose"],
            Some("enchanted") => &["enchanted"],
            Some("enters") => &["enters"],
            Some("damage") => &["damage"],
            Some("pay") => &["pay"],
            Some("copy") => &["copy"],
            Some("players") => &["players"],
            Some("shuffle") => &["shuffle"],
            Some("permanents") => &["permanents"],
            Some("creatures") => &["creatures"],
            Some("buyback") => &["buyback"],
            Some("flashback") => &["flashback"],
            Some("spells") => &["spells"],
            Some("foretelling") => &["foretelling"],
            Some("all") => &["all"],
            Some("blood") => &["blood"],
            Some("land") => &["land"],
            Some("lands") => &["lands"],
            Some("remove") => &["remove"],
            Some("attached") => &["attached"],
            Some("soulbond") => &["soulbond"],
            Some("may") => &["may"],
            Some("warp") => &["warp"],
            Some("melee") => &["melee"],
            Some("equipped") => &["equipped"],
            Some("as") => &["as"],
            Some("prevent") => &["prevent"],
            Some("reveal") => &["reveal"],
            Some("activated") => &["activated"],
            _ => &[],
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
        single_static_ability_ast_rule!(parse_ward_static_ability_line),
        single_static_ability_ast_rule!(parse_skulk_rules_text_line),
        single_static_ability_ast_rule!(
            parse_filter_dont_untap_during_controllers_untap_steps_line
        ),
        single_static_ability_ast_rule!(parse_conditional_source_spell_keyword_line),
        single_static_ability_ast_rule!(parse_pregame_begin_on_battlefield_line),
        multi_static_ability_ast_rule!(parse_combined_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_choose_basic_land_type_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_creature_type_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_named_options_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_player_as_enters_line),
        single_static_ability_ast_rule!(parse_enchanted_land_is_chosen_type_line),
        single_static_ability_ast_rule!(parse_source_is_chosen_type_in_addition_line),
        single_static_ability_ast_rule!(parse_source_is_chosen_color_line),
        single_static_ability_ast_infallible_rule!(parse_static_text_marker_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_choose_color_line),
        single_static_ability_ast_rule!(parse_damage_not_removed_cleanup_line),
        single_static_ability_ast_rule!(parse_prevent_damage_to_source_remove_counter_line),
        single_static_ability_ast_rule!(parse_choose_color_as_enters_line),
        single_static_ability_ast_rule!(parse_damage_redirect_to_source_line),
        single_static_ability_ast_rule!(
            parse_no_more_than_creatures_can_attack_or_block_each_combat_line
        ),
        single_static_ability_ast_rule!(parse_characteristic_defining_pt_line),
        single_static_ability_ast_rule!(parse_no_maximum_hand_size_line),
        single_static_ability_ast_rule!(parse_can_be_your_commander_line),
        single_static_ability_ast_rule!(parse_reduced_maximum_hand_size_line),
        single_static_ability_ast_rule!(parse_library_of_leng_discard_replacement_line),
        single_static_ability_ast_rule!(parse_draw_replace_exile_top_face_down_line),
        single_static_ability_ast_rule!(parse_exile_to_countered_exile_instead_of_graveyard_line),
        single_static_ability_ast_rule!(parse_toph_first_metalbender_line),
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
        single_static_ability_ast_rule!(parse_spell_cost_increase_per_target_beyond_first_line),
        single_static_ability_ast_rule!(parse_flashback_cost_modifier_line),
        single_static_ability_ast_rule!(parse_spells_cost_modifier_line),
        single_static_ability_ast_passthrough_rule!(parse_trigger_duplication_line_ast),
        single_static_ability_ast_rule!(
            parse_double_damage_from_sources_you_control_of_chosen_type_line
        ),
        single_static_ability_ast_rule!(parse_foretelling_cards_cost_modifier_line),
        single_static_ability_ast_rule!(parse_players_skip_upkeep_line),
        single_static_ability_ast_rule!(parse_legend_rule_doesnt_apply_line),
        single_static_ability_ast_rule!(parse_all_permanents_are_artifacts_line),
        single_static_ability_ast_rule!(parse_all_permanents_colorless_line),
        single_static_ability_ast_rule!(parse_all_cards_spells_permanents_colorless_line),
        multi_static_ability_ast_rule!(parse_all_are_color_and_type_addition_line),
        single_static_ability_ast_rule!(parse_all_creatures_are_color_line),
        single_static_ability_ast_rule!(parse_protection_from_colored_spells_line),
        single_static_ability_ast_rule!(parse_blood_moon_line),
        single_static_ability_ast_rule!(parse_land_type_addition_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_filter_is_pt_creature_in_addition_and_has_line
        ),
        multi_static_ability_ast_rule!(parse_lands_are_pt_creatures_still_lands_line),
        single_static_ability_ast_rule!(parse_remove_snow_line),
        multi_static_ability_ast_rule!(parse_attached_is_legendary_gets_and_has_keywords_line),
        StaticAbilityLineRuleDef {
            id: stringify!(parse_soulbond_shared_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_soulbond_shared_line),
        },
        StaticAbilityLineRuleDef {
            id: stringify!(parse_granted_keyword_static_line),
            rule: StaticAbilityLineRuleAst::Multi(parse_granted_keyword_static_line),
        },
        multi_static_ability_ast_rule!(parse_lose_all_abilities_and_transform_base_pt_line),
        multi_static_ability_ast_rule!(parse_lose_all_abilities_and_base_pt_line),
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
        multi_static_ability_ast_passthrough_rule!(parse_anthem_and_granted_ability_line),
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
        multi_static_ability_ast_rule!(parse_attached_has_and_loses_keywords_line),
        single_static_ability_ast_rule!(parse_you_control_attached_creature_line),
        single_static_ability_ast_passthrough_rule!(parse_attached_cant_attack_or_block_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_by_attached_line
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
        single_static_ability_ast_rule!(parse_anthem_line),
        single_static_ability_ast_rule!(parse_flying_restriction_line),
        single_static_ability_ast_rule!(parse_can_block_only_flying_line),
        single_static_ability_ast_rule!(parse_assign_damage_as_unblocked_line),
        single_static_ability_ast_rule!(parse_you_may_static_grant_line),
        single_static_ability_ast_rule!(parse_grant_flash_to_noncreature_spells_line),
        single_static_ability_ast_rule!(parse_cast_this_spell_as_though_it_had_flash_line),
        single_static_ability_ast_rule!(parse_prevent_all_combat_damage_to_source_line),
        single_static_ability_ast_rule!(parse_prevent_all_damage_to_source_by_creatures_line),
        single_static_ability_ast_rule!(parse_prevent_all_damage_dealt_to_creatures_line),
        single_static_ability_ast_passthrough_rule!(parse_creatures_cant_block_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_counters_line),
        single_static_ability_ast_rule!(parse_enters_with_counters_line),
        single_static_ability_ast_rule!(parse_enters_with_additional_counter_for_filter_line),
        single_static_ability_ast_rule!(parse_reveal_from_hand_or_enters_tapped_line),
        single_static_ability_ast_rule!(parse_conditional_enters_tapped_unless_line),
        single_static_ability_ast_rule!(parse_enters_untapped_for_filter_line),
        single_static_ability_ast_rule!(parse_enter_as_copy_as_enters_line),
        single_static_ability_ast_rule!(parse_enters_tapped_for_filter_line),
        single_static_ability_ast_rule!(parse_enters_tapped_line),
        multi_static_ability_ast_rule!(parse_additional_land_play_line),
        single_static_ability_ast_rule!(parse_you_may_look_top_card_any_time_line),
        single_static_ability_ast_rule!(parse_play_lands_from_graveyard_line),
        single_static_ability_ast_rule!(parse_cast_spells_from_hand_without_paying_mana_costs_line),
        single_static_ability_ast_rule!(parse_cost_reduction_line),
        single_static_ability_ast_rule!(parse_can_block_additional_creature_each_combat_line),
        single_static_ability_ast_passthrough_rule!(parse_all_creatures_able_to_block_source_line),
        single_static_ability_ast_rule!(parse_activated_abilities_cant_be_activated_line),
        multi_static_ability_ast_rule!(parse_cant_clauses),
    ]
}

static STATIC_ABILITY_AST_LINE_RULE_INDEX: LazyLock<StaticAbilityLineRuleIndex> =
    LazyLock::new(|| build_static_ability_line_rule_index(static_ability_ast_line_rules()));

pub(crate) fn parse_static_ability_ast_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let rules = static_ability_ast_line_rules();
    let head = words(tokens).first().copied().unwrap_or("");
    let mut tried = vec![false; rules.len()];

    if let Some(candidate_indices) = STATIC_ABILITY_AST_LINE_RULE_INDEX.by_head.get(head) {
        for &idx in candidate_indices {
            tried[idx] = true;
            if let Some(abilities) = run_static_ability_ast_line_rule(rules[idx].rule, tokens)? {
                return Ok(Some(abilities));
            }
        }
    }

    for (idx, rule) in rules.iter().enumerate() {
        if tried[idx] {
            continue;
        }
        if let Some(abilities) = run_static_ability_ast_line_rule(rule.rule, tokens)? {
            return Ok(Some(abilities));
        }
    }
    Ok(None)
}

pub(crate) fn parse_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if let Some(ability) = parse_activated_abilities_cant_be_activated_line_lexed(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }

    let lowered = lowercase_word_tokens(tokens);
    parse_static_ability_ast_line(&lowered)
}

pub(crate) fn parse_activated_abilities_cant_be_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::effect::Restriction;

    let normalized = words(tokens);
    if normalized.len() < 6 || !normalized.starts_with(&["activated", "abilities", "of"]) {
        return Ok(None);
    }

    let Some(cant_idx) = normalized.iter().position(|word| *word == "cant") else {
        return Ok(None);
    };
    if cant_idx <= 3 {
        return Ok(None);
    }

    let tail = &normalized[cant_idx..];
    if !tail.starts_with(&["cant", "be", "activated"]) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[3..cant_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    // "Activated abilities of artifacts and creatures ..." should be a union of types.
    // Our general object filter parser treats type lists joined by "and" as intersection,
    // which is correct for many adjective chains, but incorrect for this rules pattern.
    let subject_words: Vec<&str> = words(&subject_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let filter = if subject_words.len() == 3 && subject_words[1] == "and" {
        let t1 = subject_words[0]
            .strip_suffix('s')
            .unwrap_or(subject_words[0]);
        let t2 = subject_words[2]
            .strip_suffix('s')
            .unwrap_or(subject_words[2]);
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

    let non_mana_only = normalized
        .windows(4)
        .any(|window| window == ["unless", "theyre", "mana", "abilities"]);

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

pub(crate) fn parse_activated_abilities_cant_be_activated_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::effect::Restriction;

    let normalized = LowercaseWordView::new(tokens);
    if normalized.len() < 6 || !normalized.starts_with(&["activated", "abilities", "of"]) {
        return Ok(None);
    }

    let Some(cant_word_idx) = normalized.find("cant") else {
        return Ok(None);
    };
    if cant_word_idx <= 3 {
        return Ok(None);
    }

    let tail = normalized.to_word_refs();
    if !tail[cant_word_idx..].starts_with(&["cant", "be", "activated"]) {
        return Ok(None);
    }

    let Some(subject_start) = normalized.token_index_for_word_index(3) else {
        return Ok(None);
    };
    let subject_end = normalized
        .token_index_for_word_index(cant_word_idx)
        .unwrap_or(tokens.len());
    let subject_tokens = trim_lexed_commas(&tokens[subject_start..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words_view = LowercaseWordView::new(subject_tokens);
    let subject_words: Vec<&str> = subject_words_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let filter = if subject_words.len() == 3 && subject_words[1] == "and" {
        let t1 = subject_words[0]
            .strip_suffix('s')
            .unwrap_or(subject_words[0]);
        let t2 = subject_words[2]
            .strip_suffix('s')
            .unwrap_or(subject_words[2]);
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

    let non_mana_only = tail
        .windows(4)
        .any(|window| window == ["unless", "theyre", "mana", "abilities"]);

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
    let clause_words = words(tokens);
    let mentions_opening_hand = clause_words
        .windows(4)
        .any(|window| window == ["your", "opening", "hand", "and"])
        || clause_words
            .windows(3)
            .any(|window| window == ["your", "opening", "hand"]);
    if clause_words.first().copied() != Some("if")
        || !mentions_opening_hand
        || !clause_words
            .windows(6)
            .any(|window| window == ["you", "may", "begin", "the", "game", "with"])
        || !clause_words
            .windows(3)
            .any(|window| window == ["on", "the", "battlefield"])
    {
        return Ok(None);
    }

    let source_ref_start = find_source_reference_start(&tokens[1..]).map(|idx| idx + 1);
    if source_ref_start.is_none() && clause_words.get(1..4) != Some(&["this", "card", "is"][..]) {
        return Ok(None);
    }

    let require_not_starting_player = clause_words
        .windows(4)
        .any(|window| window == ["youre", "not", "playing", "first"])
        || clause_words
            .windows(4)
            .any(|window| window == ["you're", "not", "playing", "first"])
        || clause_words
            .windows(5)
            .any(|window| window == ["you", "re", "not", "playing", "first"])
        || clause_words
            .windows(5)
            .any(|window| window == ["you", "are", "not", "playing", "first"])
        || clause_words
            .windows(5)
            .any(|window| window == ["youre", "not", "the", "starting", "player"])
        || clause_words
            .windows(5)
            .any(|window| window == ["you're", "not", "the", "starting", "player"])
        || clause_words
            .windows(6)
            .any(|window| window == ["you", "re", "not", "the", "starting", "player"])
        || clause_words
            .windows(6)
            .any(|window| window == ["you", "are", "not", "the", "starting", "player"])
        || clause_words
            .windows(5)
            .any(|window| window == ["youre", "not", "starting", "the", "game"])
        || clause_words
            .windows(5)
            .any(|window| window == ["you're", "not", "starting", "the", "game"])
        || clause_words
            .windows(6)
            .any(|window| window == ["you", "re", "not", "starting", "the", "game"])
        || clause_words
            .windows(6)
            .any(|window| window == ["you", "are", "not", "starting", "the", "game"]);

    let battlefield_end_word_idx = clause_words
        .windows(3)
        .position(|window| window == ["on", "the", "battlefield"])
        .map(|idx| idx + 3)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing battlefield destination in pregame line (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let if_you_do_word_idx = clause_words
        .windows(3)
        .position(|window| window == ["if", "you", "do"]);

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
        let counter_word_idx = after_with
            .iter()
            .position(|token| token.is_word("counter") || token.is_word("counters"))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter keyword in pregame line (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let trailing = words(&after_with[counter_word_idx + 1..]);
        if trailing.as_slice() != ["on", "it"] {
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
        let trailing = words(&after_exile[used..]);
        if trailing.as_slice() != ["card", "from", "your", "hand"]
            && trailing.as_slice() != ["cards", "from", "your", "hand"]
        {
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

pub(crate) fn parse_pregame_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    let Some(choose_idx) = clause_words.iter().position(|word| *word == "choose") else {
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
    if tail != ["before", "the", "game", "begins"] {
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
    let sentences = split_on_period(tokens);
    if sentences.len() < 2 {
        return Ok(None);
    }

    let Some(first) = parse_pregame_choose_color_line(&sentences[0])? else {
        return Ok(None);
    };
    let Some(second) = parse_source_is_chosen_color_line(&sentences[1])? else {
        return Ok(None);
    };
    Ok(Some(vec![first, second]))
}

pub(crate) fn parse_can_block_additional_creature_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens);
    if normalized.as_slice()
        == [
            "this",
            "creature",
            "can",
            "block",
            "an",
            "additional",
            "creature",
            "each",
            "combat",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "can",
                "block",
                "an",
                "additional",
                "creature",
            ]
    {
        return Ok(Some(
            StaticAbility::can_block_additional_creature_each_combat(1),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_skulk_rules_text_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let is_skulk_rules_text = clause_words.as_slice()
        == [
            "creatures",
            "with",
            "power",
            "less",
            "than",
            "this",
            "creatures",
            "power",
            "cant",
            "block",
            "it",
        ]
        || clause_words.as_slice()
            == [
                "creatures",
                "with",
                "power",
                "less",
                "than",
                "this",
                "creatures",
                "power",
                "cant",
                "block",
                "this",
                "creature",
            ];
    if !is_skulk_rules_text {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::cant_be_blocked_by_lower_power_than_source(),
    ))
}

pub(crate) fn parse_ward_static_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("ward") {
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

    if let Ok(cost) = parse_activation_cost(&cost_tokens)
        && !cost.is_free()
    {
        return Ok(Some(StaticAbility::ward(cost)));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported ward cost clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_ward_discard_card_type_cost(tokens: &[OwnedLexToken]) -> Option<TotalCost> {
    let cost_words = words(tokens);
    if cost_words.first().copied() != Some("discard") {
        return None;
    }

    let mut idx = 1usize;
    let mut count = 1u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }

    let words_tail = &cost_words[idx..];
    if words_tail.starts_with(&["your", "hand"]) && words_tail.len() == 2 {
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
        if !card_types.contains(&parsed) {
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
    let clause_words = words(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let comma_segments = split_anthem_trailing_segments_preserving_granted_abilities(tokens);
    if comma_segments.len() < 2 {
        return Ok(None);
    }

    if comma_segments.len() == 2 {
        let where_tail = trim_commas(&comma_segments[1]);
        if words(&where_tail).starts_with(&["where", "x", "is"])
            && let Some(ability) = parse_anthem_line(tokens)?
        {
            return Ok(Some(vec![ability.into()]));
        }
    }

    let Some(first_action_idx) = tokens.iter().position(|token| {
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

        if segment.first().is_some_and(|token| token.is_word("and")) {
            let trimmed = trim_commas(&segment[1..]);
            if trimmed.first().is_some_and(|token| {
                token.is_word("get")
                    || token.is_word("gets")
                    || token.is_word("have")
                    || token.is_word("has")
            }) {
                segment = trimmed.to_vec();
            }
        }

        let starts_with_action = segment.first().is_some_and(|token| {
            token.is_word("get")
                || token.is_word("gets")
                || token.is_word("have")
                || token.is_word("has")
        });
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
    let words = words(tokens);
    if words.is_empty() {
        return None;
    }

    let is_once_each_turn_play_from_exile = words
        .starts_with(&["once", "each", "turn", "you", "may", "play"])
        && words.contains(&"from")
        && words.contains(&"exile")
        && words.contains(&"cast")
        && words.windows(2).any(|window| window == ["spend", "mana"])
        && words
            .windows(4)
            .any(|window| window == ["as", "though", "it", "were"])
        && words
            .windows(3)
            .any(|window| window == ["any", "color", "to"]);
    if is_once_each_turn_play_from_exile {
        return None;
    }

    if words.starts_with(&["doctors", "companion"]) || words.starts_with(&["doctor's", "companion"])
    {
        return Some(StaticAbility::doctors_companion());
    }

    if words == ["you", "have", "shroud"] {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::be_targeted_player(PlayerFilter::You),
            "You have shroud".to_string(),
        ));
    }

    if words == ["creatures", "without", "flying", "cant", "attack"] {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::attack(
                ObjectFilter::creature()
                    .without_static_ability(crate::static_abilities::StaticAbilityId::Flying),
            ),
            "Creatures without flying can't attack".to_string(),
        ));
    }

    if words == ["this", "creature", "cant", "attack", "alone"] {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "This creature can't attack alone".to_string(),
        ));
    }

    if words.len() == 4
        && words[0] == "ward"
        && words[1] == "pay"
        && words[3] == "life"
        && words[2].parse::<u32>().is_ok()
    {
        return Some(StaticAbility::keyword_marker(format!(
            "Ward—Pay {} life",
            words[2]
        )));
    }

    if words.starts_with(&[
        "lands",
        "dont",
        "untap",
        "during",
        "their",
        "controllers",
        "untap",
    ]) && matches!(words.last(), Some(&"step") | Some(&"steps"))
    {
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
    let line_words = words(tokens);
    let Some(dont_word_idx) = line_words
        .iter()
        .position(|word| *word == "dont" || *word == "doesnt")
    else {
        return Ok(None);
    };
    if line_words.get(dont_word_idx + 1) != Some(&"untap") {
        return Ok(None);
    }

    let tail = line_words.get(dont_word_idx + 2..).unwrap_or_default();
    let has_supported_tail = (tail.starts_with(&["during", "their", "controllers", "untap"])
        || tail.starts_with(&["during", "its", "controllers", "untap"]))
        && matches!(tail.last(), Some(&"step") | Some(&"steps"));
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
    let subject_text = words(&subject_tokens).join(" ");
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
    let words_all = words(tokens);
    if !words_all.starts_with(&["there", "are"]) && !words_all.starts_with(&["there", "is"]) {
        return Ok(None);
    }

    let quantified = &tokens[2..];
    let (comparison, used) = parse_static_quantity_prefix(quantified, false)?;
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
    let rest_words = words(rest);
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
    let clause_words = words(tokens);
    if clause_words.len() < 10 {
        return Ok(None);
    }

    let Some(this_idx) = clause_words
        .windows(3)
        .position(|window| window == ["this", "spell", "has"])
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

    if clause_words.get(this_idx + 4..this_idx + 7) != Some(["as", "long", "as"].as_slice()) {
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
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("this")
        || !clause_words.contains(&"enters")
        || !clause_words.contains(&"tapped")
    {
        return Ok(None);
    }
    let tapped_word_idx = clause_words
        .iter()
        .position(|word| *word == "tapped")
        .ok_or_else(|| {
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
    let words = words(tokens);
    if words.len() != 9 {
        return Ok(None);
    }
    if words[0] != "damage" || words[2] != "removed" {
        return Ok(None);
    }
    let is_not = words[1] == "isnt" || words[1] == "isn't";
    let matches = is_not
        && words[3] == "from"
        && words[4] == "this"
        && words[5] == "creature"
        && words[6] == "during"
        && words[7] == "cleanup"
        && words[8] == "steps";
    if matches {
        return Ok(Some(StaticAbility::damage_not_removed_during_cleanup()));
    }
    Ok(None)
}

fn parse_as_enters_choice_subject_words<'a>(
    words: &'a [&'a str],
    this_kind_display_pairs: &[(&str, &'static str)],
) -> Option<(usize, &'static str)> {
    if words.first().copied() != Some("as") {
        return None;
    }

    let mut idx = 1usize;
    let display_subject = if words.get(idx) == Some(&"this") {
        idx += 1;
        if let Some((_, display)) = this_kind_display_pairs
            .iter()
            .find(|(kind, _)| words.get(idx) == Some(kind))
        {
            idx += 1;
            *display
        } else {
            "this"
        }
    } else if words.get(idx) == Some(&"it") {
        idx += 1;
        "it"
    } else {
        return None;
    };

    if words.get(idx) != Some(&"enters") {
        return None;
    }
    idx += 1;

    if words.get(idx) == Some(&"the") && words.get(idx + 1) == Some(&"battlefield") {
        idx += 2;
    }

    Some((idx, display_subject))
}

pub(crate) fn parse_choose_basic_land_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_words(&words, &[("aura", "this Aura")])
    else {
        return Ok(None);
    };
    let Some(consumed) = parse_choose_basic_land_type_phrase_words(&words[idx..]) else {
        return Ok(None);
    };
    if idx + consumed != words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_basic_land_type_as_enters(
        format!("As {display_subject} enters, choose a basic land type."),
    )))
}

pub(crate) fn parse_enchanted_land_is_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let matches = words.as_slice() == ["enchanted", "land", "is", "the", "chosen", "type"]
        || words.as_slice() == ["enchanted", "land", "is", "chosen", "type"];
    if !matches {
        return Ok(None);
    }

    Ok(Some(StaticAbility::enchanted_land_is_chosen_type(
        "Enchanted land is the chosen type.".to_string(),
    )))
}

pub(crate) fn parse_source_is_chosen_type_in_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let display = match words.as_slice() {
        [
            "this",
            "creature",
            "is",
            "the",
            "chosen",
            "type",
            "in",
            "addition",
            "to",
            "its",
            "other",
            "types",
        ] => "This creature is the chosen type in addition to its other types.",
        [
            "this",
            "permanent",
            "is",
            "the",
            "chosen",
            "type",
            "in",
            "addition",
            "to",
            "its",
            "other",
            "types",
        ] => "This permanent is the chosen type in addition to its other types.",
        [
            "it",
            "is",
            "the",
            "chosen",
            "type",
            "in",
            "addition",
            "to",
            "its",
            "other",
            "types",
        ] => "It is the chosen type in addition to its other types.",
        _ => return Ok(None),
    };

    Ok(Some(StaticAbility::add_chosen_creature_type(
        ObjectFilter::source(),
        display.to_string(),
    )))
}

pub(crate) fn parse_source_is_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let Some(is_idx) = words.iter().position(|word| *word == "is") else {
        return Ok(None);
    };
    let subject_words = &words[..is_idx];
    let is_source = is_source_reference_words(subject_words) || subject_words == ["it"];
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
    let words = words(tokens);
    let Some((idx, display_subject)) = parse_as_enters_choice_subject_words(
        &words,
        &[
            ("land", "this land"),
            ("creature", "this creature"),
            ("artifact", "this artifact"),
            ("enchantment", "this enchantment"),
            ("permanent", "this permanent"),
        ],
    ) else {
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
    let words = words(tokens);
    let Some((idx, display_subject)) = parse_as_enters_choice_subject_words(
        &words,
        &[
            ("land", "this land"),
            ("creature", "this creature"),
            ("artifact", "this artifact"),
            ("enchantment", "this enchantment"),
            ("permanent", "this permanent"),
        ],
    ) else {
        return Ok(None);
    };
    let Some(choice_idx) = words[idx..].iter().position(|word| *word == "choose") else {
        return Ok(None);
    };
    let choice_words = &words[idx + choice_idx..];
    if choice_words.len() < 4 || !choice_words.contains(&"or") {
        return Ok(None);
    }
    if parse_choose_color_phrase_words(choice_words)?.is_some()
        || parse_choose_player_phrase_words(choice_words).is_some()
        || parse_choose_basic_land_type_phrase_words(choice_words).is_some()
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
    let filter_words = words(&tokens);
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

    parse_object_filter(&tokens, false)
}

fn parse_trigger_duplication_event_matcher(
    tokens: &[OwnedLexToken],
) -> Result<Trigger, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let phrase_words = words(&tokens);

    let build_filter = |subject_tokens: &[OwnedLexToken]| -> Result<ObjectFilter, CardTextError> {
        parse_object_filter(&trim_edge_punctuation(subject_tokens), false)
    };

    if phrase_words.starts_with(&["turning"]) && phrase_words.ends_with(&["face", "up"]) {
        if tokens.len() <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing turned-face-up subject in trigger-duplication clause (clause: '{}')",
                phrase_words.join(" ")
            )));
        }
        let filter = build_filter(&tokens[1..tokens.len() - 2])?;
        return Ok(Trigger::turned_face_up(filter));
    }

    if phrase_words.starts_with(&["you", "casting", "or", "copying"]) {
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
                    Trigger::enters_battlefield(filter.clone()),
                    Trigger::leaves_battlefield(filter),
                )
            },
        ),
        (&["entering", "the", "battlefield"], |filter| {
            Trigger::enters_battlefield(filter)
        }),
        (&["leaving", "the", "battlefield"], |filter| {
            Trigger::leaves_battlefield(filter)
        }),
        (&["drawing", "a", "card"], |_filter| {
            Trigger::player_draws_card(PlayerFilter::Any)
        }),
        (&["attacking"], |filter| Trigger::attacks(filter)),
        (&["dying"], |filter| Trigger::dies(filter)),
        (&["entering"], |filter| Trigger::enters_battlefield(filter)),
    ];

    for (suffix, build) in suffixes {
        if !phrase_words.ends_with(suffix) || phrase_words.len() <= suffix.len() {
            continue;
        }
        let subject_len = phrase_words.len() - suffix.len();
        if *suffix == ["drawing", "a", "card"] {
            let subject_words = &phrase_words[..subject_len];
            if matches!(subject_words, ["a", "player"] | ["player"]) {
                return Ok(Trigger::player_draws_card(PlayerFilter::Any));
            }
            if matches!(subject_words, ["you"]) {
                return Ok(Trigger::player_draws_card(PlayerFilter::You));
            }
            if matches!(subject_words, ["an", "opponent"] | ["opponent"]) {
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
    let segments = split_on_comma(&tokens);
    if segments.len() != 2 {
        return Ok(None);
    }

    let head_tokens = trim_commas(&segments[0]);
    let tail_tokens = trim_commas(&segments[1]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = words(&tail_tokens);
    if !trigger_duplication_tail_matches(&tail_words) {
        return Ok(None);
    }

    let head_words = words(&head_tokens);
    if !head_words.starts_with(&["if"]) || head_tokens.len() < 2 {
        return Ok(None);
    }

    let body_tokens = &head_tokens[1..];
    let body_words = words(body_tokens);

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
        if !body_words.starts_with(prefix) || body_tokens.len() <= prefix.len() + 1 {
            continue;
        }
        let Some(triggers_idx) = body_words.iter().position(|word| *word == "triggers") else {
            continue;
        };
        if triggers_idx <= prefix.len() {
            continue;
        }

        let condition = if body_words
            .get(triggers_idx + 1..)
            .is_some_and(|tail| tail.starts_with(&["while"]))
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
        && let Some(causes_idx) = body_words.iter().position(|word| *word == "causes")
    {
        let cause_tokens = &body_tokens[..causes_idx];
        let source_body_tokens = &body_tokens[causes_idx + 1..];
        for prefix in ability_prefixes {
            if !words(source_body_tokens).starts_with(prefix)
                || source_body_tokens.len() <= prefix.len() + 2
            {
                continue;
            }
            let source_words = words(source_body_tokens);
            if !source_words.ends_with(&["to", "trigger"]) {
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
                words(&tokens).join(" "),
            ),
            condition,
        )
    }))
}

pub(crate) fn parse_trigger_duplication_line_ast(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let token_words = words(&tokens);
    if token_words.starts_with(&["as", "long", "as"]) {
        let Some(comma_idx) = tokens.iter().position(|token| token.is_comma()) else {
            return Ok(None);
        };
        let condition = parse_static_condition_clause(&tokens[3..comma_idx])?;
        let remainder = trim_commas(&tokens[comma_idx + 1..]);
        let Some(inner) = parse_trigger_duplication_line_ast(&remainder)? else {
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
    let token_words = words(&tokens);
    let Some(neg_idx) = token_words
        .iter()
        .position(|word| matches!(*word, "dont" | "don't" | "doesnt" | "doesn't"))
    else {
        return Ok(None);
    };
    if token_words.get(neg_idx + 1).copied() != Some("cause") {
        return Ok(None);
    }
    if neg_idx == 0 {
        return Ok(None);
    }

    let Some(cause_end_token_idx) = token_index_for_word_index(&tokens, neg_idx) else {
        return Err(CardTextError::ParseError(format!(
            "failed to split trigger-suppression cause (clause: '{}')",
            token_words.join(" ")
        )));
    };
    let cause_tokens = &tokens[..cause_end_token_idx];
    let body_words = &token_words[neg_idx + 2..];
    let source_filter = match body_words {
        ["abilities", "to", "trigger"] => None,
        ["abilities", "of", middle @ .., "to", "trigger"] => {
            let start = token_index_for_word_index(&tokens, neg_idx + 4).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "failed to split trigger-suppression source filter (clause: '{}')",
                    token_words.join(" ")
                ))
            })?;
            let end =
                token_index_for_word_index(&tokens, token_words.len() - 2).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "failed to trim trigger-suppression source tail (clause: '{}')",
                        token_words.join(" ")
                    ))
                })?;
            let _ = middle;
            Some(parse_trigger_duplication_source_filter(
                &tokens[start..end],
            )?)
        }
        _ => return Ok(None),
    };
    let event_matcher = parse_trigger_duplication_event_matcher(cause_tokens)?;

    Ok(Some(StaticAbilityAst::from(
        StaticAbility::suppress_matching_triggered_abilities(
            source_filter,
            Some(event_matcher),
            token_words.join(" "),
        ),
    )))
}

pub(crate) fn parse_double_damage_from_sources_you_control_of_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice()
        != [
            "double", "all", "damage", "that", "sources", "you", "control", "of", "the", "chosen",
            "type", "would", "deal",
        ]
    {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::double_damage_from_sources_you_control_of_chosen_type(
            "Double all damage that sources you control of the chosen type would deal.".to_string(),
        ),
    ))
}

pub(crate) fn parse_enter_as_copy_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 11 || !clause_words.starts_with(&["you", "may", "have"]) {
        return Ok(None);
    }

    let mut idx = 3usize;
    if clause_words.get(idx).copied() != Some("this") {
        return Ok(None);
    }
    idx += 1;
    if clause_words.get(idx).is_some_and(|word| {
        matches!(
            *word,
            "land" | "creature" | "artifact" | "enchantment" | "permanent"
        )
    }) {
        idx += 1;
    }

    if clause_words.get(idx).copied() != Some("enter")
        && clause_words.get(idx).copied() != Some("enters")
    {
        return Ok(None);
    }
    idx += 1;

    if clause_words.get(idx).copied() == Some("the")
        && clause_words.get(idx + 1).copied() == Some("battlefield")
    {
        idx += 2;
    }

    let mut enters_tapped_if_chosen = false;
    if clause_words.get(idx).copied() == Some("tapped") {
        enters_tapped_if_chosen = true;
        idx += 1;
    }

    if clause_words.get(idx..idx + 4) != Some(&["as", "a", "copy", "of"]) {
        return Ok(None);
    }
    idx += 4;

    let except_idx = clause_words.iter().position(|word| *word == "except");
    let filter_end_word_idx = except_idx.unwrap_or(clause_words.len());
    let filter_start_token_idx = token_index_for_word_index(tokens, idx).unwrap_or(tokens.len());
    let filter_end_token_idx =
        token_index_for_word_index(tokens, filter_end_word_idx).unwrap_or(tokens.len());
    let filter_tokens = trim_commas(&tokens[filter_start_token_idx..filter_end_token_idx]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false)?;

    let mut added_subtypes = Vec::new();
    if let Some(except_idx) = except_idx {
        let tail = &clause_words[except_idx..];
        if tail.first().copied() != Some("except") {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy exception clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let (article_idx, subtype_idx) = if tail.get(1).copied() == Some("its")
            && matches!(tail.get(2).copied(), Some("a" | "an"))
        {
            (2usize, 3usize)
        } else if (tail.get(1..3) == Some(&["it", "is"])
            || tail.get(1..3) == Some(&["it", "s"]))
            && matches!(tail.get(3).copied(), Some("a" | "an"))
        {
            (3usize, 4usize)
        } else if matches!(tail.get(1).copied(), Some("it's" | "it’s"))
            && matches!(tail.get(2).copied(), Some("a" | "an"))
        {
            (2usize, 3usize)
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy exception clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };

        if !matches!(tail.get(article_idx).copied(), Some("a" | "an"))
            || tail.get(subtype_idx + 1..)
                != Some(&["in", "addition", "to", "its", "other", "types"])
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy exception clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let Some(subtype) = parse_subtype_word(tail[subtype_idx])
            .or_else(|| parse_subtype_flexible(tail[subtype_idx]))
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy subtype '{}' (clause: '{}')",
                tail[subtype_idx],
                clause_words.join(" ")
            )));
        };
        added_subtypes.push(subtype);
    }

    Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
        crate::static_abilities::EnterAsCopyAsEntersSpec {
            filter,
            may: true,
            enters_tapped_if_chosen,
            added_subtypes,
        },
        clause_words.join(" "),
    )))
}

pub(crate) fn parse_choose_color_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let Some((idx, display_subject)) = parse_as_enters_choice_subject_words(
        &words,
        &[
            ("land", "this land"),
            ("creature", "this creature"),
            ("artifact", "this artifact"),
            ("enchantment", "this enchantment"),
            ("aura", "this aura"),
            ("permanent", "this permanent"),
        ],
    ) else {
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

pub(crate) fn parse_choose_player_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let Some((idx, display_subject)) = parse_as_enters_choice_subject_words(
        &words,
        &[
            ("land", "this land"),
            ("creature", "this creature"),
            ("artifact", "this artifact"),
            ("enchantment", "this enchantment"),
            ("aura", "this aura"),
            ("permanent", "this permanent"),
        ],
    ) else {
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
    let words = words(tokens);
    if words.len() != 19 {
        return Ok(None);
    }
    let matches = words[0] == "all"
        && words[1] == "damage"
        && words[2] == "that"
        && words[3] == "would"
        && words[4] == "be"
        && words[5] == "dealt"
        && words[6] == "to"
        && words[7] == "you"
        && words[8] == "and"
        && words[9] == "other"
        && (words[10] == "permanents" || words[10] == "permanent")
        && words[11] == "you"
        && words[12] == "control"
        && words[13] == "is"
        && words[14] == "dealt"
        && words[15] == "to"
        && words[16] == "this"
        && words[17] == "creature"
        && words[18] == "instead";
    if matches {
        return Ok(Some(
            StaticAbility::redirect_damage_from_you_and_other_permanents_to_source(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_no_more_than_creatures_can_attack_or_block_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 8 || !line_words.starts_with(&["no", "more", "than"]) {
        return Ok(None);
    }

    let Some((maximum, used)) = parse_number(&tokens[3..]) else {
        return Ok(None);
    };

    let tail = words(&tokens[3 + used..]);
    if tail.len() != 5 {
        return Ok(None);
    }

    if !matches!(tail[0], "creature" | "creatures")
        || tail[1] != "can"
        || tail[3] != "each"
        || tail[4] != "combat"
    {
        return Ok(None);
    }

    let ability = match tail[2] {
        "attack" => StaticAbility::max_attackers_each_combat(maximum as usize),
        "block" => StaticAbility::max_blockers_each_combat(maximum as usize),
        _ => return Ok(None),
    };
    Ok(Some(ability))
}

pub(crate) fn parse_characteristic_defining_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    let has_pt_axes = line_words
        .windows(3)
        .any(|window| window == ["power", "and", "toughness"]);
    if has_pt_axes
        && line_words
            .windows(2)
            .any(|window| window == ["equal", "to"])
        && let Some(equal_word_idx) = line_words
            .windows(2)
            .position(|window| window == ["equal", "to"])
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
                            words(tail_tokens).join(" ")
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
                    words(&value_tokens).join(" ")
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
    let words = words(&trimmed);
    if !words.starts_with(&["that", "number"]) {
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
        && words.get(idx + 1).copied() == Some("is")
        && words.get(idx + 2).copied() == Some("equal")
        && words.get(idx + 3).copied() == Some("to")
    {
        return Some((first, idx + 4));
    }

    if first == "creature"
        && matches!(words.get(idx + 1).copied(), Some("power" | "toughness"))
        && words.get(idx + 2).copied() == Some("is")
        && words.get(idx + 3).copied() == Some("equal")
        && words.get(idx + 4).copied() == Some("to")
    {
        return Some((words[idx + 1], idx + 5));
    }

    if !is_self_ref(first) {
        return None;
    }

    if matches!(words.get(idx + 1).copied(), Some("power" | "toughness"))
        && words.get(idx + 2).copied() == Some("is")
        && words.get(idx + 3).copied() == Some("equal")
        && words.get(idx + 4).copied() == Some("to")
    {
        return Some((words[idx + 1], idx + 5));
    }

    if words.get(idx + 1).copied() == Some("creature")
        && matches!(words.get(idx + 2).copied(), Some("power" | "toughness"))
        && words.get(idx + 3).copied() == Some("is")
        && words.get(idx + 4).copied() == Some("equal")
        && words.get(idx + 5).copied() == Some("to")
    {
        return Some((words[idx + 2], idx + 6));
    }

    None
}

fn parse_characteristic_defining_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let trimmed_words = words(&trimmed);
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

    parse_add_mana_equal_amount_value(&equal_prefixed)
        .or_else(|| parse_equal_to_aggregate_filter_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(&equal_prefixed))
        .or_else(|| parse_characteristic_defining_pt_value(&trimmed))
}

pub(crate) fn parse_characteristic_defining_pt_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = words(tokens);
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
    for plus_word_idx in plus_positions {
        let start_token_idx = token_index_for_word_index(tokens, start_word_idx)?;
        let end_token_idx = token_index_for_word_index(tokens, plus_word_idx)?;
        values.push(parse_characteristic_defining_pt_term(
            &tokens[start_token_idx..end_token_idx],
        )?);
        start_word_idx = plus_word_idx + 1;
    }
    let final_start_token_idx = token_index_for_word_index(tokens, start_word_idx)?;
    values.push(parse_characteristic_defining_pt_term(
        &tokens[final_start_token_idx..],
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

    if start.first().is_some_and(|token| token.is_word("number"))
        && start.get(1).is_some_and(|token| token.is_word("of"))
    {
        start = &start[2..];
    }
    if start.is_empty() {
        return None;
    }

    // "the number of cards in the hand of the opponent with the most cards in hand"
    // (Adamaro, First to Desire)
    let start_words = words(start);
    if start_words.as_slice()
        == [
            "cards", "in", "the", "hand", "of", "the", "opponent", "with", "the", "most", "cards",
            "in", "hand",
        ]
        || start_words.as_slice()
            == [
                "cards", "in", "the", "hand", "of", "an", "opponent", "with", "the", "most",
                "cards", "in", "hand",
            ]
    {
        return Some(Value::MaxCardsInHand(PlayerFilter::Opponent));
    }
    if start_words.as_slice()
        == [
            "cards", "in", "the", "hand", "of", "the", "player", "with", "the", "most", "cards",
            "in", "hand",
        ]
    {
        return Some(Value::MaxCardsInHand(PlayerFilter::Any));
    }

    let filter = parse_object_filter(start, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_shuffle_into_library_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_would_be_put = words
        .windows(3)
        .any(|window| window == ["would", "be", "put"]);
    let has_graveyard = words.contains(&"graveyard");
    let has_anywhere = words.contains(&"anywhere");
    let has_shuffle = words.contains(&"shuffle");
    let has_library = words.contains(&"library");
    let has_instead = words.contains(&"instead");

    if has_would_be_put
        && has_graveyard
        && has_anywhere
        && has_shuffle
        && has_library
        && has_instead
    {
        return Ok(Some(StaticAbility::shuffle_into_library_from_graveyard()));
    }

    Ok(None)
}

pub(crate) fn parse_permanents_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["permanents", "enter", "tapped"]
        || words.as_slice() == ["permanents", "enters", "tapped"]
    {
        return Ok(Some(StaticAbility::permanents_enter_tapped()));
    }
    Ok(None)
}

pub(crate) fn parse_creatures_entering_dont_cause_abilities_to_trigger_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice()
        == [
            "creatures",
            "entering",
            "dont",
            "cause",
            "abilities",
            "to",
            "trigger",
        ]
        || words.as_slice()
            == [
                "creatures",
                "entering",
                "don't",
                "cause",
                "abilities",
                "to",
                "trigger",
            ]
    {
        return Ok(Some(
            StaticAbility::creatures_entering_dont_cause_abilities_to_trigger(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_creatures_assign_combat_damage_using_toughness_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice()
        == [
            "each",
            "creature",
            "assigns",
            "combat",
            "damage",
            "equal",
            "to",
            "its",
            "toughness",
            "rather",
            "than",
            "its",
            "power",
        ]
    {
        return Ok(Some(
            StaticAbility::creatures_assign_combat_damage_using_toughness(),
        ));
    }
    if words.as_slice()
        == [
            "each",
            "creature",
            "you",
            "control",
            "assigns",
            "combat",
            "damage",
            "equal",
            "to",
            "its",
            "toughness",
            "rather",
            "than",
            "its",
            "power",
        ]
    {
        return Ok(Some(
            StaticAbility::creatures_you_control_assign_combat_damage_using_toughness(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_players_cant_cycle_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["players", "cant", "cycle", "cards"] {
        return Ok(Some(StaticAbility::players_cant_cycle()));
    }
    Ok(None)
}

pub(crate) fn parse_starting_life_bonus_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 8 || !words.starts_with(&["you", "start", "the", "game"]) {
        return Ok(None);
    }
    if !words.contains(&"additional") || !words.contains(&"life") {
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
    let words = words(tokens);
    if words.len() < 5 || !words.starts_with(&["buyback", "costs", "cost"]) {
        return Ok(None);
    }
    let (amount, _) = parse_number(&tokens[3..])
        .ok_or_else(|| CardTextError::ParseError("missing buyback reduction amount".to_string()))?;
    if !words.contains(&"less") {
        return Ok(None);
    }
    Ok(Some(StaticAbility::buyback_cost_reduction(amount)))
}

pub(crate) fn parse_spell_cost_increase_per_target_beyond_first_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }
    if !words.contains(&"more") || !words.contains(&"target") || !words.contains(&"beyond") {
        return Ok(None);
    }

    let costs_idx = tokens
        .iter()
        .position(|token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &tokens[costs_idx + 1..];
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
    let words_all = words(tokens);
    if !words_all.starts_with(&["if"]) {
        return Ok(None);
    }

    let Some(comma_idx) = tokens.iter().position(|t| t.is_comma()) else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(&tokens[1..comma_idx]);
    let tail_tokens = trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    let tail_words = words(&tail_tokens);
    if !tail_words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }

    let condition = parse_this_spell_cost_condition(&condition_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let costs_idx = tail_tokens
        .iter()
        .position(|token| token.is_word("costs"))
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
    let remaining_words = words(amount_tokens.get(used..).unwrap_or_default());
    if !remaining_words.contains(&"less") || !remaining_words.contains(&"cast") {
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
    let words_all = LowercaseWordView::new(tokens);
    if !words_all.starts_with(&["if"]) {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
    else {
        return Ok(None);
    };
    let condition_tokens = trim_lexed_commas(&tokens[1..comma_idx]);
    let tail_tokens = trim_lexed_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    let tail_words = LowercaseWordView::new(tail_tokens);
    if !tail_words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }

    let lowered_condition = lowercase_word_tokens(condition_tokens);
    let condition = parse_this_spell_cost_condition(&lowered_condition).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            words_all.to_word_refs().join(" ")
        ))
    })?;

    let costs_idx = tail_tokens
        .iter()
        .position(|token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = tail_tokens.get(costs_idx + 1..).unwrap_or_default();
    let lowered_amount = lowercase_word_tokens(amount_tokens);
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(&lowered_amount);
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
    let remaining_words = words(lowered_amount.get(used..).unwrap_or_default());
    if !remaining_words.contains(&"less") || !remaining_words.contains(&"cast") {
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

    let w = words(tokens);
    let target_start = if w.starts_with(&["it", "targets"]) {
        2
    } else if w.starts_with(&["this", "spell", "targets"]) {
        3
    } else {
        return None;
    };
    let target_tokens = trim_commas(tokens.get(target_start..).unwrap_or_default());
    if target_tokens.is_empty() {
        return None;
    }
    let target_words = words(&target_tokens);
    if target_words.starts_with(&["you"]) {
        return Some(ThisSpellCostCondition::TargetsPlayer(PlayerFilter::You));
    }
    if target_words.starts_with(&["an", "opponent"]) || target_words.starts_with(&["opponent"]) {
        return Some(ThisSpellCostCondition::TargetsPlayer(
            PlayerFilter::Opponent,
        ));
    }
    if target_words.starts_with(&["a", "player"]) || target_words.starts_with(&["player"]) {
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

    let w = words(tokens);
    if w.is_empty() {
        return None;
    }

    // you have 3 or less life
    if w.len() >= 6 && w[0] == "you" && w[1] == "have" && w.contains(&"life") {
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
        && w[w.len().saturating_sub(2)..] == ["or", "less"]
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

    if w.as_slice() == ["you", "attacked", "this", "turn"]
        || w.as_slice() == ["youve", "attacked", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::AttackedThisTurn,
            display: w.join(" "),
        });
    }
    if w.as_slice() == ["a", "creature", "died", "this", "turn"]
        || w.as_slice() == ["creature", "died", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::CreatureDiedThisTurn,
            display: w.join(" "),
        });
    }
    if w.as_slice() == ["you", "gained", "life", "this", "turn"]
        || w.as_slice() == ["youve", "gained", "life", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(1));
    }
    if (w.starts_with(&["youve", "gained"]) || w.starts_with(&["you", "gained"]))
        && w.len() >= 7
        && w[w.len() - 3..] == ["life", "this", "turn"]
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
        && w.get(3) == Some(&"or")
        && w.get(4) == Some(&"more")
    {
        return Some(ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(n));
    }
    if w.as_slice() == ["its", "night"] || w.as_slice() == ["it", "is", "night"] {
        return Some(ThisSpellCostCondition::IsNight);
    }
    if w.as_slice() == ["youve", "sacrificed", "an", "artifact", "this", "turn"]
        || w.as_slice() == ["you", "sacrificed", "an", "artifact", "this", "turn"]
    {
        return Some(ThisSpellCostCondition::YouSacrificedArtifactThisTurn);
    }
    if w.as_slice() == ["youve", "committed", "a", "crime", "this", "turn"]
        || w.as_slice() == ["you", "committed", "a", "crime", "this", "turn"]
    {
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
    if (w.starts_with(&["youve", "cast", "another"]) || w.starts_with(&["you", "cast", "another"]))
        && w.ends_with(&["this", "turn"])
    {
        if w.contains(&"instant") || w.contains(&"sorcery") {
            let mut types = Vec::new();
            if w.contains(&"instant") {
                types.push(CardType::Instant);
            }
            if w.contains(&"sorcery") {
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
    if (w.starts_with(&["youve", "cast"]) || w.starts_with(&["you", "cast"]))
        && w.ends_with(&["this", "turn"])
        && (w.contains(&"instant") || w.contains(&"sorcery"))
    {
        let mut types = Vec::new();
        if w.contains(&"instant") {
            types.push(CardType::Instant);
        }
        if w.contains(&"sorcery") {
            types.push(CardType::Sorcery);
        }
        return Some(ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
            count: 1,
            card_types: types,
        });
    }

    if w.as_slice() == ["you", "werent", "the", "starting", "player"] {
        return Some(ThisSpellCostCondition::NotStartingPlayer);
    }
    if w.as_slice() == ["a", "creature", "is", "attacking", "you"] {
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
        && w.contains(&"card")
        && w.contains(&"types")
        && w.contains(&"graveyard")
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
    {
        return Some(ThisSpellCostCondition::DistinctCardTypesInYourGraveyardOrMore(n));
    }
    if w.starts_with(&["you", "have"])
        && w.ends_with(&["in", "your", "graveyard"])
        && let Some((n, _)) = parse_number(tokens.get(2..).unwrap_or_default())
    {
        if w.contains(&"instant") || w.contains(&"sorcery") {
            let mut types = Vec::new();
            if w.contains(&"instant") {
                types.push(CardType::Instant);
            }
            if w.contains(&"sorcery") {
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
            if tail == ["or", "more", "poison", "counters"]
                || tail == ["or", "more", "poison", "counter"]
            {
                return Some(ThisSpellCostCondition::OpponentHasPoisonCountersOrMore(n));
            }
            if tail == ["or", "more", "cards", "in", "their", "graveyard"]
                || tail == ["or", "more", "cards", "in", "his", "graveyard"]
                || tail == ["or", "more", "cards", "in", "her", "graveyard"]
                || tail == ["or", "more", "card", "in", "their", "graveyard"]
            {
                return Some(ThisSpellCostCondition::OpponentHasCardsInGraveyardOrMore(n));
            }
        }
    }

    if w.starts_with(&["there", "are", "no"]) && w.ends_with(&["in", "your", "hand"]) {
        let filter_tokens = trim_commas(tokens.get(3..).unwrap_or_default());
        if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
            return Some(ThisSpellCostCondition::NoCardsInHandMatching {
                filter,
                display: w.join(" "),
            });
        }
    }
    if ((w.starts_with(&["you", "have", "no", "other", "creature", "cards"])
        && w.windows(2).any(|window| window == ["or", "if"]))
        || w.starts_with(&[
            "the", "only", "other", "creature", "cards", "in", "your", "hand", "are", "named",
        ]))
        && let Some(named_idx) = w.iter().position(|word| *word == "named")
        && named_idx + 1 < w.len()
    {
        let name = w[named_idx + 1..].join(" ");
        if !name.is_empty() {
            return Some(ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name));
        }
    }

    if w.starts_with(&["there", "is"]) && w.ends_with(&["in", "your", "graveyard"]) {
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
    if w.as_slice() == ["an", "opponent", "has", "no", "cards", "in", "hand"]
        || w.as_slice() == ["opponent", "has", "no", "cards", "in", "hand"]
    {
        return Some(ThisSpellCostCondition::OpponentHasNoCardsInHand);
    }

    // an opponent controls seven or more lands
    if w.len() >= 7 && w[0] == "an" && w[1] == "opponent" && w[2] == "controls" {
        if let Some((n, _)) = parse_number(tokens.get(3..).unwrap_or_default()) {
            let tail = &w[4..];
            if tail == ["or", "more", "lands"] || tail == ["or", "more", "land"] {
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
            if tail == ["more", "creatures", "than", "you"]
                || tail == ["more", "creature", "than", "you"]
            {
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
            if tail == ["or", "more", "spells", "this", "turn"]
                || tail == ["or", "more", "spell", "this", "turn"]
            {
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
            if tail == ["or", "more", "cards", "this", "turn"]
                || tail == ["or", "more", "card", "this", "turn"]
            {
                return Some(ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(n));
            }
        }
    }

    // you've been dealt damage by two or more creatures this turn
    if (w.starts_with(&["youve", "been", "dealt", "damage", "by"])
        || w.starts_with(&["you", "have", "been", "dealt", "damage", "by"]))
        && w.len() >= 11
    {
        let count_start = if w[0] == "youve" { 5 } else { 6 };
        if let Some((n, _)) = parse_number(tokens.get(count_start..).unwrap_or_default()) {
            let tail = &w[count_start + 1..];
            if tail == ["or", "more", "creatures", "this", "turn"]
                || tail == ["or", "more", "creature", "this", "turn"]
            {
                return Some(
                    ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(n),
                );
            }
        }
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
    let words = words(tokens);
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
    let remaining_words = words(remaining_tokens);
    let Some(if_idx) = remaining_words.iter().position(|word| *word == "if") else {
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

pub(crate) fn parse_cost_modifier_prefix_condition(
    tokens: &[OwnedLexToken],
    spells_token_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    let subject_end = spells_token_idx.min(tokens.len());
    let head_tokens = &tokens[..subject_end];

    if words_start_with(tokens, &["during", "turns", "other", "than", "yours"]) {
        let subject_start = head_tokens
            .iter()
            .position(|token| token.is_comma())
            .map(|idx| idx + 1)
            .unwrap_or(5);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }

    if words_start_with(tokens, &["during", "your", "turn"]) {
        let subject_start = head_tokens
            .iter()
            .position(|token| token.is_comma())
            .map(|idx| idx + 1)
            .unwrap_or(3);
        return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
    }

    if words_start_with(tokens, &["as", "long", "as"]) {
        let subject_start = head_tokens
            .iter()
            .position(|token| token.is_comma())
            .map(|idx| idx + 1)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
        if subject_start <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        let condition = match parse_static_condition_clause(&condition_tokens) {
            Ok(condition) => condition,
            Err(_) => {
                let condition_word_view = LowercaseWordView::new(&condition_tokens);
                let condition_words = condition_word_view.to_word_refs();
                match condition_words.as_slice() {
                    ["this", "creature", "is", "tapped"]
                    | ["this", "permanent", "is", "tapped"]
                    | ["it", "is", "tapped"] => crate::ConditionExpr::SourceIsTapped,
                    ["this", "creature", "is", "untapped"]
                    | ["this", "permanent", "is", "untapped"]
                    | ["it", "is", "untapped"] => crate::ConditionExpr::SourceIsUntapped,
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported static condition clause (clause: '{}')",
                            condition_words.join(" ")
                        )));
                    }
                }
            }
        };
        return Ok((Some(condition), subject_start));
    }

    Ok((None, 0))
}

pub(crate) fn parse_spells_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let Some(spells_token_idx) = tokens
        .iter()
        .position(|token| token.is_word("spell") || token.is_word("spells"))
    else {
        return Ok(None);
    };

    if clause_words.contains(&"first")
        && clause_words.contains(&"each")
        && clause_words.contains(&"turn")
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
    let subject_words = words(&subject_tokens);
    let is_this_spell =
        subject_words.as_slice() == ["this"] || subject_words.as_slice() == ["thiss"];

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
        parse_spell_filter(&subject_tokens)
    };

    let between_tokens = &tokens[spells_token_idx + 1..cost_token_idx];
    let between_words = words(between_tokens);
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
            let extra_filter = parse_spell_filter(strip_relative_target_clause(&descriptor_tokens));
            if spell_filter_has_identity(&extra_filter) {
                merge_spell_filters(&mut filter, extra_filter);
            }
        }
        let between_filter = parse_spell_filter(strip_relative_target_clause(between_tokens));
        if spell_filter_has_identity(&between_filter) {
            merge_spell_filters(&mut filter, between_filter);
        }
        if between_words
            .windows(2)
            .any(|window| window == ["you", "cast"])
        {
            filter.cast_by = Some(PlayerFilter::You);
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
        let mut target_player: Option<PlayerFilter> = None;
        let mut target_object: Option<Box<ObjectFilter>> = None;
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
            let target_words = words(target_tokens);
            if target_words.starts_with(&["you"]) {
                target_player = Some(PlayerFilter::You);
            } else if target_words.starts_with(&["opponent"])
                || target_words.starts_with(&["opponents"])
            {
                target_player = Some(PlayerFilter::Opponent);
            } else if target_words.starts_with(&["player"])
                || target_words.starts_with(&["players"])
            {
                target_player = Some(PlayerFilter::Any);
            } else {
                target_object = Some(Box::new(parse_object_filter(target_tokens, false)?));
            }
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
    let remaining_words = words(remaining_tokens);
    let is_less = remaining_words.contains(&"less");
    let is_more = remaining_words.contains(&"more");
    if !is_less && !is_more {
        return Ok(None);
    }

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
    if remaining_words
        .windows(3)
        .any(|window| window == ["where", "x", "is"])
    {
        let clause = clause_words.join(" ");
        let where_word_idx = remaining_words
            .windows(3)
            .position(|window| window == ["where", "x", "is"])
            .unwrap_or(0);
        let where_token_idx = token_index_for_word_index(remaining_tokens, where_word_idx)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map where-x clause in spells-cost modifier (clause: '{clause}')"
                ))
            })?;
        let where_tokens = trim_commas(&remaining_tokens[where_token_idx..]);
        let x_value = parse_where_x_value_clause(&where_tokens).ok_or_else(|| {
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

    if is_less {
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

fn strip_relative_target_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(target_clause_idx) = tokens.windows(2).position(|window| {
        window[0].is_word("that") && (window[1].is_word("target") || window[1].is_word("targets"))
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
    let remaining_words = words(remaining_tokens);
    let Some(if_word_idx) = remaining_words.iter().position(|word| *word == "if") else {
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

    let target_words = words(target_tokens);
    if target_words.starts_with(&["you"]) {
        filter.targets_player = Some(PlayerFilter::You);
        filter.targets_object = None;
        return Ok(());
    }
    if target_words.starts_with(&["opponent"]) || target_words.starts_with(&["opponents"]) {
        filter.targets_player = Some(PlayerFilter::Opponent);
        filter.targets_object = None;
        return Ok(());
    }
    if target_words.starts_with(&["player"]) || target_words.starts_with(&["players"]) {
        filter.targets_player = Some(PlayerFilter::Any);
        filter.targets_object = None;
        return Ok(());
    }

    filter.targets_object = Some(Box::new(parse_object_filter(target_tokens, false)?));
    filter.targets_player = None;
    Ok(())
}

pub(crate) fn parse_flashback_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 6 || clause_words.first().copied() != Some("flashback") {
        return Ok(None);
    }
    let cost_idx = tokens
        .iter()
        .rposition(|token| token.is_word("cost") || token.is_word("costs"));
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
    let remaining_words = words(remaining_tokens);
    let is_less = remaining_words.contains(&"less");
    let is_more = remaining_words.contains(&"more");
    if !is_less && !is_more {
        return Ok(None);
    }
    if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing flashback cost modifier amount".to_string(),
        ));
    }

    let mut filter = ObjectFilter::default();
    filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Flashback);
    if clause_words
        .windows(2)
        .any(|window| window == ["you", "pay"])
    {
        filter.cast_by = Some(PlayerFilter::You);
    } else if clause_words
        .windows(3)
        .any(|window| window == ["your", "opponents", "pay"])
        || clause_words
            .windows(2)
            .any(|window| window == ["opponents", "pay"] || window == ["opponent", "pays"])
    {
        filter.cast_by = Some(PlayerFilter::Opponent);
    }

    if is_less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

pub(crate) fn parse_foretelling_cards_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 7 {
        return Ok(None);
    }
    if !clause_words.starts_with(&["foretelling", "cards", "from", "your", "hand", "costs"]) {
        return Ok(None);
    }

    let has_less = clause_words.contains(&"less");
    let has_any_players_turn = clause_words.windows(5).any(|window| {
        window == ["on", "any", "players", "turn"] || window == ["on", "any", "player", "turn"]
    }) || clause_words
        .windows(5)
        .any(|window| window == ["on", "any", "player", "s", "turn"]);
    if !has_less || !has_any_players_turn {
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
        if group
            .iter()
            .any(|symbol| matches!(symbol, ManaSymbol::X | ManaSymbol::Snow | ManaSymbol::Life(_)))
        {
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

pub(crate) fn parse_dynamic_cost_modifier_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let words_all = words(tokens);
    let Some(each_idx) = words_all.iter().position(|word| *word == "each") else {
        return Ok(None);
    };

    let filter_tokens = &tokens[each_idx + 1..];
    let filter_words = words(filter_tokens);
    if filter_words.is_empty() {
        return Ok(None);
    }
    if filter_words.starts_with(&["creature", "that", "died", "this", "turn"])
        || filter_words.starts_with(&["creatures", "that", "died", "this", "turn"])
    {
        return Ok(Some(Value::CreaturesDiedThisTurn));
    }
    if filter_words.starts_with(&["creature", "that", "died", "under", "your", "control"])
        || filter_words.starts_with(&["creatures", "that", "died", "under", "your", "control"])
    {
        if filter_words.contains(&"this") && filter_words.contains(&"turn") {
            return Ok(Some(Value::CreaturesDiedThisTurnControlledBy(
                PlayerFilter::You,
            )));
        }
    }
    // "for each spell you've cast this turn" (and limited variants like "instant and sorcery spell")
    let has_spell_cast_turn = (filter_words.contains(&"spell") || filter_words.contains(&"spells"))
        && (filter_words.contains(&"cast") || filter_words.contains(&"casts"))
        && filter_words.contains(&"this")
        && filter_words.contains(&"turn");
    if has_spell_cast_turn {
        let player = if filter_words
            .iter()
            .any(|word| matches!(*word, "you" | "your" | "youve"))
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

        let other_than_first = filter_words
            .windows(4)
            .any(|window| window == ["other", "than", "the", "first"]);
        if other_than_first {
            return Ok(Some(Value::Add(
                Box::new(Value::SpellsCastThisTurn(player)),
                Box::new(Value::Fixed(-1)),
            )));
        }

        let exclude_source = filter_words.contains(&"other");
        let has_instant = filter_words.contains(&"instant");
        let has_sorcery = filter_words.contains(&"sorcery");
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

    if filter_words.windows(2).any(|pair| pair == ["card", "type"])
        && filter_words.contains(&"graveyard")
    {
        let player = if filter_words
            .windows(2)
            .any(|pair| pair == ["your", "graveyard"])
        {
            PlayerFilter::You
        } else if filter_words
            .windows(2)
            .any(|pair| pair == ["opponents", "graveyard"] || pair == ["opponent", "graveyard"])
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Ok(Some(Value::CardTypesInGraveyard(player)));
    }

    if filter_words.starts_with(&[
        "color", "of", "mana", "spent", "to", "cast", "this", "spell",
    ]) || filter_words.starts_with(&[
        "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
    ]) || filter_words
        .starts_with(&["color", "of", "mana", "used", "to", "cast", "this", "spell"])
        || filter_words.starts_with(&[
            "colors", "of", "mana", "used", "to", "cast", "this", "spell",
        ])
    {
        return Ok(Some(Value::ColorsOfManaSpentToCastThisSpell));
    }
    if filter_words.starts_with(&["creature", "in", "your", "party"])
        || filter_words.starts_with(&["creatures", "in", "your", "party"])
    {
        return Ok(Some(Value::PartySize(PlayerFilter::You)));
    }
    if filter_words.starts_with(&["basic", "land", "type", "among"])
        || filter_words.starts_with(&["basic", "land", "types", "among"])
    {
        let lands_tokens = &filter_tokens[4..];
        if let Ok(filter) = parse_object_filter(lands_tokens, false) {
            return Ok(Some(Value::BasicLandTypesAmong(filter)));
        }
    }

    // "for each <counter> counter removed this way" (storage lands, mana batteries, etc.)
    // The remove-counters cost plumbs the removed total through `CostContext.x_value`,
    // so model the dynamic amount as `X`.
    if (filter_words.contains(&"counter") || filter_words.contains(&"counters"))
        && filter_words.contains(&"removed")
        && filter_words.windows(2).any(|pair| pair == ["this", "way"])
    {
        return Ok(Some(Value::X));
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
        let on_source = tail.starts_with(&["it"])
            || tail.starts_with(&["this"])
            || tail.starts_with(&["that", "object"])
            || tail.starts_with(&["that", "permanent"]);
        if on_source {
            return Ok(Some(match counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            }));
        }
    }

    if let Ok(filter) = parse_object_filter(filter_tokens, false) {
        return Ok(Some(Value::Count(filter)));
    }

    Ok(None)
}

pub(crate) fn parse_add_mana_equal_amount_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = LowercaseWordView::new(tokens);
    let words_all = word_view.to_word_refs();
    let equal_idx = words_all
        .windows(2)
        .position(|window| window == ["equal", "to"])?;
    let tail = &words_all[equal_idx + 2..];
    if tail.is_empty() {
        return None;
    }

    let is_source_power_segment = |segment: &[&str]| {
        matches!(
            segment,
            ["this", "power"]
                | ["thiss", "power"]
                | ["this", "creature", "power"]
                | ["this", "creatures", "power"]
                | ["thiss", "creature", "power"]
                | ["thiss", "creatures", "power"]
                | ["its", "power"]
        )
    };
    let is_source_toughness_segment = |segment: &[&str]| {
        matches!(
            segment,
            ["this", "toughness"]
                | ["thiss", "toughness"]
                | ["this", "creature", "toughness"]
                | ["this", "creatures", "toughness"]
                | ["thiss", "creature", "toughness"]
                | ["thiss", "creatures", "toughness"]
                | ["its", "toughness"]
        )
    };

    let parse_power_or_toughness_segment = |segment: &[&str]| -> Option<Value> {
        let tagged_it_power = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
        let tagged_it_toughness =
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));

        if is_source_power_segment(segment) {
            return Some(Value::PowerOf(Box::new(ChooseSpec::Source)));
        }
        if is_source_toughness_segment(segment) {
            return Some(Value::ToughnessOf(Box::new(ChooseSpec::Source)));
        }
        if segment == ["that", "creature", "power"]
            || segment == ["that", "creatures", "power"]
            || segment == ["that", "objects", "power"]
        {
            return Some(tagged_it_power.clone());
        }
        if segment == ["that", "creature", "toughness"]
            || segment == ["that", "creatures", "toughness"]
            || segment == ["that", "objects", "toughness"]
        {
            return Some(tagged_it_toughness.clone());
        }
        if segment == ["the", "sacrificed", "creature", "power"]
            || segment == ["the", "sacrificed", "creatures", "power"]
            || segment == ["sacrificed", "creature", "power"]
            || segment == ["sacrificed", "creatures", "power"]
        {
            return Some(tagged_it_power);
        }
        if segment == ["the", "sacrificed", "creature", "toughness"]
            || segment == ["the", "sacrificed", "creatures", "toughness"]
            || segment == ["sacrificed", "creature", "toughness"]
            || segment == ["sacrificed", "creatures", "toughness"]
        {
            return Some(tagged_it_toughness);
        }
        None
    };

    let parse_mana_value_segment = |segment: &[&str]| -> Option<Value> {
        if segment.starts_with(&["that", "spell", "mana", "value"])
            || segment.starts_with(&["that", "spells", "mana", "value"])
            || segment.starts_with(&["that", "card", "mana", "value"])
            || segment.starts_with(&["that", "cards", "mana", "value"])
            || segment.starts_with(&[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "creature",
            ])
            || segment.starts_with(&[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "artifact",
            ])
            || segment.starts_with(&[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "permanent",
            ])
            || segment.starts_with(&["mana", "value", "of", "the", "sacrificed", "creature"])
            || segment.starts_with(&["mana", "value", "of", "the", "sacrificed", "artifact"])
            || segment.starts_with(&["mana", "value", "of", "the", "sacrificed", "permanent"])
            || segment.starts_with(&["the", "sacrificed", "creature", "mana", "value"])
            || segment.starts_with(&["the", "sacrificed", "artifact", "mana", "value"])
            || segment.starts_with(&["the", "sacrificed", "permanent", "mana", "value"])
            || segment.starts_with(&["the", "sacrificed", "creatures", "mana", "value"])
            || segment.starts_with(&["the", "sacrificed", "artifacts", "mana", "value"])
            || segment.starts_with(&["the", "sacrificed", "permanents", "mana", "value"])
            || segment.starts_with(&["sacrificed", "creature", "mana", "value"])
            || segment.starts_with(&["sacrificed", "artifact", "mana", "value"])
            || segment.starts_with(&["sacrificed", "permanent", "mana", "value"])
            || segment.starts_with(&["sacrificed", "creatures", "mana", "value"])
            || segment.starts_with(&["sacrificed", "artifacts", "mana", "value"])
            || segment.starts_with(&["sacrificed", "permanents", "mana", "value"])
            || segment.starts_with(&["its", "mana", "value"])
        {
            return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                TagKey::from(IT_TAG),
            ))));
        }
        if matches!(
            segment,
            ["this", "spell", "mana", "value"]
                | ["this", "creature", "mana", "value"]
                | ["this", "permanent", "mana", "value"]
                | ["this", "card", "mana", "value"]
        ) {
            return Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)));
        }
        None
    };

    let parse_amount_segment = |segment: &[&str]| -> Option<Value> {
        parse_power_or_toughness_segment(segment)
            .or_else(|| {
                if segment.len() == 1 {
                    parse_number_word_i32(segment[0]).map(Value::Fixed)
                } else {
                    None
                }
            })
            .or_else(|| parse_mana_value_segment(segment))
    };

    if let Some(plus_idx) = tail.iter().position(|word| *word == "plus")
        && plus_idx > 0
        && plus_idx + 1 < tail.len()
        && let Some(left) = parse_amount_segment(&tail[..plus_idx])
        && let Some(right) = parse_amount_segment(&tail[plus_idx + 1..])
    {
        return Some(Value::Add(Box::new(left), Box::new(right)));
    }

    if let Some(value) = parse_amount_segment(tail) {
        return Some(value);
    }

    if is_source_power_segment(tail)
        || tail.starts_with(&["that", "creature", "power"])
        || tail.starts_with(&["that", "creatures", "power"])
        || tail.starts_with(&["that", "objects", "power"])
        || tail.starts_with(&["the", "sacrificed", "creature", "power"])
        || tail.starts_with(&["the", "sacrificed", "creatures", "power"])
        || tail.starts_with(&["sacrificed", "creature", "power"])
        || tail.starts_with(&["sacrificed", "creatures", "power"])
    {
        let source = if tail[0] == "that" || tail.contains(&"sacrificed") {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::PowerOf(Box::new(source)));
    }

    if is_source_toughness_segment(tail)
        || tail.starts_with(&["that", "creature", "toughness"])
        || tail.starts_with(&["that", "creatures", "toughness"])
        || tail.starts_with(&["that", "objects", "toughness"])
        || tail.starts_with(&["the", "sacrificed", "creature", "toughness"])
        || tail.starts_with(&["the", "sacrificed", "creatures", "toughness"])
        || tail.starts_with(&["sacrificed", "creature", "toughness"])
        || tail.starts_with(&["sacrificed", "creatures", "toughness"])
    {
        let source = if tail[0] == "that" || tail.contains(&"sacrificed") {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::ToughnessOf(Box::new(source)));
    }

    if tail.starts_with(&["that", "spell", "mana", "value"])
        || tail.starts_with(&["that", "spells", "mana", "value"])
        || tail.starts_with(&["that", "card", "mana", "value"])
        || tail.starts_with(&["that", "cards", "mana", "value"])
        || tail.starts_with(&[
            "the",
            "mana",
            "value",
            "of",
            "the",
            "sacrificed",
            "creature",
        ])
        || tail.starts_with(&[
            "the",
            "mana",
            "value",
            "of",
            "the",
            "sacrificed",
            "artifact",
        ])
        || tail.starts_with(&[
            "the",
            "mana",
            "value",
            "of",
            "the",
            "sacrificed",
            "permanent",
        ])
        || tail.starts_with(&["mana", "value", "of", "the", "sacrificed", "creature"])
        || tail.starts_with(&["mana", "value", "of", "the", "sacrificed", "artifact"])
        || tail.starts_with(&["mana", "value", "of", "the", "sacrificed", "permanent"])
        || tail.starts_with(&["the", "sacrificed", "creature", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "artifact", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "permanent", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "creatures", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "artifacts", "mana", "value"])
        || tail.starts_with(&["the", "sacrificed", "permanents", "mana", "value"])
        || tail.starts_with(&["sacrificed", "creature", "mana", "value"])
        || tail.starts_with(&["sacrificed", "artifact", "mana", "value"])
        || tail.starts_with(&["sacrificed", "permanent", "mana", "value"])
        || tail.starts_with(&["sacrificed", "creatures", "mana", "value"])
        || tail.starts_with(&["sacrificed", "artifacts", "mana", "value"])
        || tail.starts_with(&["sacrificed", "permanents", "mana", "value"])
        || tail.starts_with(&["its", "mana", "value"])
    {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }
    if matches!(
        tail,
        ["this", "spell", "mana", "value"]
            | ["this", "creature", "mana", "value"]
            | ["this", "permanent", "mana", "value"]
            | ["this", "card", "mana", "value"]
    ) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)));
    }

    None
}

pub(crate) fn parse_add_mana_that_much_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words_all = words(tokens);
    if words_all.starts_with(&["that", "much"]) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    None
}

pub(crate) fn parse_players_skip_upkeep_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["players", "skip", "their", "upkeep", "steps"] {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

pub(crate) fn parse_legend_rule_doesnt_apply_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"legend") && words.contains(&"rule") && words.contains(&"doesnt") {
        return Ok(Some(StaticAbility::legend_rule_doesnt_apply()));
    }
    Ok(None)
}

pub(crate) fn parse_all_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "permanents", "are", "colorless"] {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_all_permanents_are_artifacts_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.starts_with(&["all", "permanents", "are", "artifacts"]) {
        return Ok(Some(StaticAbility::add_card_types(
            ObjectFilter::permanent(),
            vec![CardType::Artifact],
        )));
    }
    Ok(None)
}

pub(crate) fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"colorless")
        && words.contains(&"cards")
        && words.contains(&"spells")
        && words.contains(&"permanents")
    {
        return Ok(Some(StaticAbility::make_colorless(ObjectFilter::default())));
    }
    Ok(None)
}

pub(crate) fn parse_all_are_color_and_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 10 {
        return Ok(None);
    }
    let Some(are_idx) = words.iter().position(|word| *word == "are") else {
        return Ok(None);
    };
    if are_idx == 0 || are_idx + 4 >= words.len() {
        return Ok(None);
    }

    let Some(base_color) = words.get(are_idx + 1).and_then(|word| parse_color(word)) else {
        return Ok(None);
    };

    // Pattern: "<subject> are <color> and are <subtype>... in addition to their other creature types"
    if words.get(are_idx + 2) != Some(&"and") || words.get(are_idx + 3) != Some(&"are") {
        return Ok(None);
    }

    let tail = &words[are_idx + 4..];
    let Some(addition_idx) = tail
        .windows(5)
        .position(|window| window == ["in", "addition", "to", "their", "other"])
    else {
        return Ok(None);
    };
    if addition_idx == 0 {
        return Ok(None);
    }

    let scope = &tail[addition_idx + 5..];
    if !matches!(scope, ["creature", "type"] | ["creature", "types"]) {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in &tail[..addition_idx] {
        if is_article(descriptor) || matches!(*descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !card_types.contains(&card_type) {
                card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !subtypes.contains(&subtype) {
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
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }
    let are_idx = words.iter().position(|word| *word == "are");
    let Some(are_idx) = are_idx else {
        return Ok(None);
    };
    if are_idx == 0 {
        return Ok(None);
    }
    if words.len() != are_idx + 2 {
        return Ok(None);
    }

    let color_word = words.get(are_idx + 1).copied();
    let Some(color_word) = color_word else {
        return Ok(None);
    };
    let Some(color) = parse_color(color_word) else {
        return Ok(None);
    };

    let subject_tokens = &tokens[..are_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(subject_tokens, false)?;

    Ok(Some(StaticAbility::set_colors(filter, color)))
}

pub(crate) fn parse_blood_moon_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["nonbasic", "lands", "are", "mountains"] {
        return Ok(Some(StaticAbility::blood_moon()));
    }
    Ok(None)
}

pub(crate) fn parse_remove_snow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "lands", "are", "no", "longer", "snow"] {
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
    let words = words(tokens);
    if words.len() < 10 {
        return Ok(None);
    }

    let Some(be_idx) = words
        .iter()
        .position(|word| *word == "is" || *word == "are")
    else {
        return Ok(None);
    };
    if be_idx == 0 || be_idx + 1 >= words.len() {
        return Ok(None);
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
    let Some(subtype) = parse_subtype_word(subtype_word)
        .or_else(|| subtype_word.strip_suffix('s').and_then(parse_subtype_word))
    else {
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
    let words = words(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(be_idx) = words
        .iter()
        .position(|word| *word == "is" || *word == "are")
    else {
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
    let clause_words = words(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let Some(be_idx) = tokens
        .iter()
        .position(|token| token.is_word("is") || token.is_word("are"))
    else {
        return Ok(None);
    };
    let Some(has_idx) = tokens.iter().enumerate().find_map(|(idx, token)| {
        (idx > be_idx && (token.is_word("has") || token.is_word("have"))).then_some(idx)
    }) else {
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

    let before_has = trim_commas(&tokens[be_idx + 1..has_idx]);
    if before_has.is_empty() {
        return Ok(None);
    }
    let mut before_has_words = words(&before_has);
    if before_has_words
        .first()
        .is_some_and(|word| is_article(word))
    {
        before_has_words.remove(0);
    }
    if before_has_words.len() < 8 {
        return Ok(None);
    }

    let (power, toughness) = match parse_pt_modifier(before_has_words[0]) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let Some(creature_idx) = before_has_words
        .iter()
        .position(|word| matches!(*word, "creature" | "creatures"))
    else {
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
    if tail.last().copied() == Some("and") {
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

    let filter = match &subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter.clone(),
    };
    let wrap_static = |ability: StaticAbility, condition: &Option<crate::ConditionExpr>| {
        let ast = StaticAbilityAst::Static(ability);
        if let Some(condition) = condition {
            StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ast),
                condition: condition.clone(),
            }
        } else {
            ast
        }
    };

    let mut abilities = vec![
        wrap_static(
            StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
            &condition,
        ),
        wrap_static(
            StaticAbility::set_base_power_toughness(filter.clone(), power, toughness),
            &condition,
        ),
    ];
    if !subtypes.is_empty() {
        abilities.push(wrap_static(
            StaticAbility::add_subtypes(filter.clone(), subtypes),
            &condition,
        ));
    }

    let mut granted_any = false;
    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    for raw_segment in split_anthem_trailing_segments_preserving_granted_abilities(&ability_tokens)
    {
        let mut segment = trim_commas(&raw_segment).to_vec();
        while segment.first().is_some_and(|token| token.is_word("and")) {
            segment = trim_commas(&segment[1..]).to_vec();
        }
        if segment.is_empty() {
            continue;
        }

        if let Some(triggered) = parse_triggered_granted_ability(&segment)? {
            abilities.push(StaticAbilityAst::GrantObjectAbility {
                filter: filter.clone(),
                ability: triggered,
                display: words(&segment).join(" "),
                condition: condition.clone(),
            });
            granted_any = true;
            continue;
        }

        let Some(actions) = parse_ability_line(&segment) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        let mut granted = false;
        for action in actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
        {
            let ast = match &subject {
                AnthemSubjectAst::Source => match &condition {
                    Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                        action,
                        condition: condition.clone(),
                    },
                    None => StaticAbilityAst::KeywordAction(action),
                },
                AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                    filter: filter.clone(),
                    action,
                    condition: condition.clone(),
                },
            };
            abilities.push(ast);
            granted = true;
            granted_any = true;
        }
        if !granted {
            return Ok(None);
        }
    }

    if !granted_any {
        return Ok(None);
    }

    Ok(Some(abilities))
}

pub(crate) fn parse_creatures_cant_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["creatures", "cant", "block"] {
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
    let words = words(tokens);
    if words.as_slice()
        == [
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "creatures",
        ]
    {
        return Ok(Some(StaticAbility::prevent_all_damage_dealt_to_creatures()));
    }
    Ok(None)
}

pub(crate) fn parse_prevent_all_combat_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let is_this_creature = words.as_slice()
        == [
            "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to", "this",
            "creature",
        ];
    let is_this_permanent = words.as_slice()
        == [
            "prevent",
            "all",
            "combat",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "this",
            "permanent",
        ];
    let is_it = words.as_slice()
        == [
            "prevent", "all", "combat", "damage", "that", "would", "be", "dealt", "to", "it",
        ];

    if is_this_creature || is_this_permanent || is_it {
        return Ok(Some(StaticAbility::prevent_all_combat_damage_to_self()));
    }

    Ok(None)
}

pub(crate) fn parse_prevent_all_damage_to_source_by_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let is_this_creature = words.as_slice()
        == [
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "this",
            "creature",
            "by",
            "creatures",
        ];
    let is_this_permanent = words.as_slice()
        == [
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "this",
            "permanent",
            "by",
            "creatures",
        ];

    if is_this_creature || is_this_permanent {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self_by_creatures(),
        ));
    }
    Ok(None)
}

pub(crate) fn parse_may_choose_not_to_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["you", "may", "choose", "not", "to", "untap"]) {
        return Ok(None);
    }
    if !words.ends_with(&["during", "your", "untap", "step"]) {
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
    let line_words = words(tokens);
    if !is_untap_during_each_other_players_untap_step_words(&line_words) {
        return Ok(None);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported untap-during-each-other-players-untap-step clause (clause: '{}')",
        line_words.join(" ")
    )))
}

pub(crate) fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let prefix_len = [
        ["this", "doesnt", "untap", "during", "your", "untap", "step"].as_slice(),
        [
            "this", "land", "doesnt", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "artifact", "doesnt", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "creature", "doesnt", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this", "does", "not", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
    ]
    .iter()
    .find(|pattern| clause_words.starts_with(pattern))
    .map(|pattern| pattern.len());

    if let Some(prefix_len) = prefix_len {
        let tail_tokens = trim_commas(&tokens[prefix_len..]);
        if tail_tokens.is_empty() {
            return Ok(Some(
                StaticAbilityAst::Static(StaticAbility::doesnt_untap()),
            ));
        }
        let tail_words = words(&tail_tokens);
        if tail_words.first().copied() == Some("if") {
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

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing untap-step clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let attached_subject_len = if clause_words.starts_with(&["enchanted", "creature"])
        || clause_words.starts_with(&["enchanted", "permanent"])
        || clause_words.starts_with(&["enchanted", "artifact"])
        || clause_words.starts_with(&["enchanted", "land"])
        || clause_words.starts_with(&["equipped", "creature"])
        || clause_words.starts_with(&["equipped", "permanent"])
    {
        Some(2usize)
    } else {
        None
    };
    if let Some(subject_len) = attached_subject_len {
        let remainder = &clause_words[subject_len..];
        let attached_matches = matches!(
            remainder,
            [
                "doesnt",
                "untap",
                "during",
                "its",
                "controller",
                "untap",
                "step"
            ] | [
                "doesnt",
                "untap",
                "during",
                "its",
                "controllers",
                "untap",
                "step"
            ] | [
                "does",
                "not",
                "untap",
                "during",
                "its",
                "controller",
                "untap",
                "step"
            ] | [
                "does",
                "not",
                "untap",
                "during",
                "its",
                "controllers",
                "untap",
                "step"
            ]
        );

        if attached_matches {
            let subject = clause_words[..subject_len].join(" ");
            let text = format!("{subject} doesnt untap during its controllers untap step");
            return Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                display: text,
                condition: None,
            }));
        }
    }

    Ok(None)
}

pub(crate) fn parse_flying_restriction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let flying_only_matches = normalized.as_slice()
        == [
            "this",
            "cant",
            "be",
            "blocked",
            "except",
            "by",
            "creatures",
            "with",
            "flying",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
            ];

    if flying_only_matches {
        return Ok(Some(StaticAbility::flying_only_restriction()));
    }

    let flying_or_reach_matches = normalized.as_slice()
        == [
            "this",
            "cant",
            "be",
            "blocked",
            "except",
            "by",
            "creatures",
            "with",
            "flying",
            "or",
            "reach",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
                "or",
                "reach",
            ];

    if flying_or_reach_matches {
        return Ok(Some(StaticAbility::flying_restriction()));
    }

    Ok(None)
}

pub(crate) fn parse_can_block_only_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let matches = normalized.as_slice()
        == [
            "this",
            "can",
            "block",
            "only",
            "creatures",
            "with",
            "flying",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "can",
                "block",
                "only",
                "creatures",
                "with",
                "flying",
            ]
        || normalized.as_slice() == ["can", "block", "only", "creatures", "with", "flying"]
        || normalized.as_slice() == ["this", "can", "block", "only", "creature", "with", "flying"]
        || normalized.as_slice()
            == [
                "this", "creature", "can", "block", "only", "creature", "with", "flying",
            ];

    if matches {
        return Ok(Some(StaticAbility::can_block_only_flying()));
    }

    Ok(None)
}

pub(crate) fn parse_assign_damage_as_unblocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if normalized.first().copied() != Some("you") {
        return Ok(None);
    }

    let mut idx = 1;
    if normalized.get(idx) == Some(&"may") {
        idx += 1;
    }
    if normalized.get(idx) != Some(&"have") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) != Some(&"this") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) == Some(&"creature") {
        idx += 1;
    }

    let tail = &normalized[idx..];
    let matches =
        tail == [
            "assign", "its", "combat", "damage", "as", "though", "it", "werent", "blocked",
        ] || tail
            == [
                "assign", "its", "combat", "damage", "as", "though", "it", "wasnt", "blocked",
            ];

    if matches {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }

    Ok(None)
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

pub(crate) fn parse_you_may_static_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player:
                crate::cards::builders::PlayerAst::You | crate::cards::builders::PlayerAst::Implicit,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) => Ok(Some(StaticAbility::grants(spec))),
        _ => Ok(None),
    }
}

pub(crate) fn parse_you_may_look_top_card_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice()
        == [
            "you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any",
            "time",
        ]
        || words.as_slice()
            == [
                "you", "may", "look", "at", "top", "card", "of", "your", "library", "any", "time",
            ]
    {
        return Ok(Some(StaticAbility::rule_text_placeholder(
            "You may look at the top card of your library any time.",
        )));
    }
    Ok(None)
}

pub(crate) fn parse_cast_this_spell_as_though_it_had_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens);
    let matches = normalized.as_slice()
        == [
            "you", "may", "cast", "this", "spell", "as", "though", "it", "had", "flash",
        ]
        || normalized.as_slice()
            == [
                "you", "may", "cast", "this", "as", "though", "it", "had", "flash",
            ];
    if matches {
        return Ok(Some(StaticAbility::flash()));
    }
    Ok(None)
}

pub(crate) fn parse_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = words(tokens);
    let Some(attack_idx) = words
        .iter()
        .position(|word| *word == "attack" || *word == "attacks")
    else {
        return Ok(None);
    };
    if words[attack_idx..] != ["attacks", "each", "combat", "if", "able"]
        && words[attack_idx..] != ["attack", "each", "combat", "if", "able"]
    {
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
    let words = words(tokens);
    if !words.starts_with(&["you", "may", "play"]) {
        return Ok(None);
    }

    let mut count_word_idx = 3;
    if words.get(count_word_idx) == Some(&"up") && words.get(count_word_idx + 1) == Some(&"to") {
        count_word_idx += 2;
    }

    let Some(count_token_idx) = token_index_for_word_index(tokens, count_word_idx) else {
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
    let is_match = rest_words == ["additional", "land", "on", "each", "of", "your", "turns"]
        || rest_words == ["additional", "lands", "on", "each", "of", "your", "turns"];
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
    let words = words(tokens);
    if words.as_slice() == ["you", "may", "play", "lands", "from", "your", "graveyard"] {
        let spec = crate::grant::GrantSpec::play_lands_from_graveyard();
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

pub(crate) fn parse_cast_spells_from_hand_without_paying_mana_costs_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
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

    let (sign, value_text) = if let Some(rest) = trimmed.strip_prefix('+') {
        (1, rest)
    } else if let Some(rest) = trimmed.strip_prefix('-') {
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
    let words = words(tokens);
    if words.as_slice() == ["you", "have", "no", "maximum", "hand", "size"] {
        return Ok(Some(StaticAbility::no_maximum_hand_size()));
    }
    Ok(None)
}

pub(crate) fn parse_can_be_your_commander_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["this", "can", "be", "your", "commander"] {
        return Ok(Some(StaticAbility::can_be_commander()));
    }
    Ok(None)
}

pub(crate) fn parse_reduced_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let max_hand_size_subject_prefix_len = |tail: &[&str]| -> Option<usize> {
        if tail.starts_with(&["your"]) || tail.starts_with(&["you"]) {
            Some(1)
        } else if tail.starts_with(&["each", "opponent's"]) {
            Some(2)
        } else if tail.starts_with(&["each", "opponent", "s"]) {
            Some(3)
        } else if tail.starts_with(&["each", "opponent"])
            || tail.starts_with(&["each", "opponents"])
        {
            Some(2)
        } else if tail.starts_with(&["opponent's"]) {
            Some(1)
        } else if tail.starts_with(&["opponent", "s"]) {
            Some(2)
        } else if tail.starts_with(&["opponent"]) || tail.starts_with(&["opponents"]) {
            Some(1)
        } else if tail.starts_with(&["each", "player's"]) {
            Some(2)
        } else if tail.starts_with(&["each", "player", "s"]) {
            Some(3)
        } else if tail.starts_with(&["each", "player"]) || tail.starts_with(&["each", "players"]) {
            Some(2)
        } else if tail.starts_with(&["player's"]) {
            Some(1)
        } else if tail.starts_with(&["player", "s"]) {
            Some(2)
        } else if tail.starts_with(&["player"]) || tail.starts_with(&["players"]) {
            Some(1)
        } else {
            None
        }
    };

    let mut min_card_types_condition: Option<u32> = None;
    let mut line_words = words(tokens);
    if line_words.is_empty() {
        return Ok(None);
    }

    let working_tokens_storage = if line_words.starts_with(&["as", "long", "as"]) {
        let (condition_end_idx, remainder_start_idx) =
            if let Some(comma_idx) = tokens.iter().position(|token| token.is_comma()) {
                if comma_idx <= 3 {
                    return Ok(None);
                }
                (comma_idx, comma_idx + 1)
            } else {
                let Some(split_word_idx) = (4..line_words.len()).find(|word_idx| {
                    let tail = &line_words[*word_idx..];
                    let Some(prefix_len) = max_hand_size_subject_prefix_len(tail) else {
                        return false;
                    };
                    tail.get(prefix_len..prefix_len + 4)
                        == Some(["maximum", "hand", "size", "is"].as_slice())
                }) else {
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
    line_words = words(working_tokens);
    if line_words.is_empty() {
        return Ok(None);
    }

    let (player, mut idx) = if line_words.starts_with(&["your"]) || line_words.starts_with(&["you"])
    {
        (crate::target::PlayerFilter::You, 1usize)
    } else if line_words.starts_with(&["each", "opponent's"]) {
        (crate::target::PlayerFilter::Opponent, 2usize)
    } else if line_words.starts_with(&["each", "opponent"])
        || line_words.starts_with(&["each", "opponents"])
        || line_words.starts_with(&["each", "opponent", "s"])
    {
        (
            crate::target::PlayerFilter::Opponent,
            if line_words.starts_with(&["each", "opponent", "s"]) {
                3usize
            } else {
                2usize
            },
        )
    } else if line_words.starts_with(&["opponent's"]) {
        (crate::target::PlayerFilter::Opponent, 1usize)
    } else if line_words.starts_with(&["opponent"])
        || line_words.starts_with(&["opponents"])
        || line_words.starts_with(&["opponent", "s"])
    {
        (
            crate::target::PlayerFilter::Opponent,
            if line_words.starts_with(&["opponent", "s"]) {
                2usize
            } else {
                1usize
            },
        )
    } else if line_words.starts_with(&["each", "player's"]) {
        (crate::target::PlayerFilter::Any, 2usize)
    } else if line_words.starts_with(&["each", "player"])
        || line_words.starts_with(&["each", "players"])
        || line_words.starts_with(&["each", "player", "s"])
    {
        (
            crate::target::PlayerFilter::Any,
            if line_words.starts_with(&["each", "player", "s"]) {
                3usize
            } else {
                2usize
            },
        )
    } else if line_words.starts_with(&["player's"]) {
        (crate::target::PlayerFilter::Any, 1usize)
    } else if line_words.starts_with(&["player"])
        || line_words.starts_with(&["players"])
        || line_words.starts_with(&["player", "s"])
    {
        (
            crate::target::PlayerFilter::Any,
            if line_words.starts_with(&["player", "s"]) {
                2usize
            } else {
                1usize
            },
        )
    } else {
        return Ok(None);
    };

    if line_words.get(idx..idx + 5) == Some(["maximum", "hand", "size", "is", "reduced"].as_slice())
    {
        idx += 5;
        if line_words.get(idx) != Some(&"by") {
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

    if line_words.get(idx..idx + 4) == Some(["maximum", "hand", "size", "is"].as_slice()) {
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

        if amount <= 7 {
            return Ok(Some(StaticAbility::reduce_maximum_hand_size(
                player,
                7 - amount,
            )));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported maximum-hand-size increase clause (clause: '{}')",
            line_words.join(" ")
        )));
    }
    Ok(None)
}

pub(crate) fn parse_library_of_leng_discard_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_effect_causes = words.windows(3).any(|w| w == ["effect", "causes", "you"]);
    let has_discard = words.contains(&"discard");
    let has_top = words.contains(&"top");
    let has_library = words.contains(&"library");
    let has_instead = words.contains(&"instead");
    let has_graveyard = words.contains(&"graveyard");

    if has_effect_causes && has_discard && has_top && has_library && has_instead && has_graveyard {
        return Ok(Some(StaticAbility::library_of_leng_discard_replacement()));
    }

    Ok(None)
}

pub(crate) fn parse_draw_replace_exile_top_face_down_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 {
        return Ok(None);
    }

    if !words.starts_with(&["if", "you", "would", "draw", "a", "card"]) {
        return Ok(None);
    }

    let has_exile = words.contains(&"exile");
    let has_top_card = words.windows(2).any(|window| window == ["top", "card"]);
    let has_library = words.contains(&"library");
    let has_face_down = words.windows(2).any(|window| window == ["face", "down"]);
    let has_instead = words.contains(&"instead");

    if has_exile && has_top_card && has_library && has_face_down && has_instead {
        return Ok(Some(StaticAbility::draw_replacement_exile_top_face_down()));
    }

    Ok(None)
}

pub(crate) fn parse_exile_to_countered_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let word_view = LowercaseWordView::new(tokens);
    let words = word_view.to_word_refs();
    if words.len() < 12 || !words.starts_with(&["if"]) {
        return Ok(None);
    }

    let has_would_put = words
        .windows(4)
        .any(|window| window == ["would", "be", "put", "into"]);
    let has_from_anywhere = words
        .windows(2)
        .any(|window| window == ["from", "anywhere"]);
    let has_instead_exile = words
        .windows(2)
        .any(|window| window == ["instead", "exile"])
        || words.contains(&"exile");
    if !has_would_put || !has_from_anywhere || !has_instead_exile {
        return Ok(None);
    }

    let Some(into_idx) = words.iter().position(|word| *word == "into") else {
        return Ok(None);
    };
    let Some(from_idx) = words.iter().position(|word| *word == "from") else {
        return Ok(None);
    };
    if from_idx <= into_idx + 1 {
        return Ok(None);
    }

    let player = match &words[into_idx + 1..from_idx] {
        ["your", "graveyard"] => PlayerFilter::You,
        ["an", "opponents", "graveyard"] | ["opponents", "graveyard"] => PlayerFilter::Opponent,
        ["a", "players", "graveyard"] | ["players", "graveyard"] => PlayerFilter::Any,
        _ => return Ok(None),
    };

    let Some(counter_idx) = words.iter().position(|word| *word == "counter") else {
        return Ok(None);
    };
    if counter_idx == 0 || words.get(counter_idx + 1).copied() != Some("on") {
        return Ok(None);
    }
    let Some(counter_type) = parse_counter_type_word(words[counter_idx - 1]) else {
        return Ok(None);
    };

    Ok(Some(
        StaticAbility::exile_to_countered_exile_instead_of_graveyard(player, counter_type),
    ))
}

pub(crate) fn parse_toph_first_metalbender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_nontoken = words.contains(&"nontoken");
    let has_artifact = words
        .iter()
        .any(|word| *word == "artifact" || *word == "artifacts");
    let has_you_control = words
        .windows(2)
        .any(|pair| pair == ["you", "control"] || pair == ["you", "controls"]);
    let has_land = words.iter().any(|word| *word == "land" || *word == "lands");
    let has_addition = words
        .windows(4)
        .any(|pair| pair == ["in", "addition", "to", "their"]);

    if has_nontoken && has_artifact && has_you_control && has_land && has_addition {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::TophFirstMetalbender,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_discard_or_redirect_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_enter = words
        .iter()
        .any(|word| *word == "enter" || *word == "enters");
    let has_battlefield = words.contains(&"battlefield");
    let has_discard = words.contains(&"discard");
    let has_land = words.contains(&"land");
    let has_instead = words.contains(&"instead");
    let has_graveyard = words.contains(&"graveyard");

    if has_enter && has_battlefield && has_discard && has_land && has_instead && has_graveyard {
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
    let normalized_storage = words(tokens)
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

    let starts_with_as_this = words.starts_with(&["as", "this"]);
    let has_pay = words.contains(&"pay");
    let has_life = words.contains(&"life");
    if !starts_with_as_this || !has_pay || !has_life {
        return Ok(None);
    }

    let Some(pay_idx) = tokens.iter().position(|token| token.is_word("pay")) else {
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
    if !words[..pay_idx].contains(&"may") {
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

    let if_dont_idx = words
        .windows(3)
        .position(|window| window == ["if", "you", "dont"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported pay-life ETB trailing clause (expected 'if you don't ...') (clause: '{}')",
                words.join(" ")
            ))
        })?;

    let trailing = &words[if_dont_idx + 3..];
    let valid_trailing = trailing.starts_with(&["it", "enters", "tapped"])
        || trailing.starts_with(&["it", "enter", "tapped"])
        || trailing.starts_with(&["it", "enters", "the", "battlefield", "tapped"])
        || trailing.starts_with(&["it", "enter", "the", "battlefield", "tapped"]);
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
    let clause_words = words(tokens);
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

    let mut filter_tokens = trim_edge_punctuation(&tokens[(has_idx + 5)..]);
    while filter_tokens
        .first()
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        filter_tokens.remove(0);
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let after_of_words = words(&filter_tokens);
    let counter = after_of_words
        .iter()
        .zip(after_of_words.iter().skip(1))
        .find_map(|(word, next)| {
            if *next == "counter" {
                parse_counter_type_word(word)
            } else {
                None
            }
        });

    let exclude_source_name = clause_words.windows(5).any(|window| {
        window == ["same", "name", "as", "this", "creature"]
            || window == ["same", "name", "as", "thiss", "creature"]
    });

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(clause_words.join(" "));
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
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
    let clause_words = words(tokens);
    let (player, tail_start, display) = if clause_words.starts_with(&[
        "players", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color",
    ]) {
        (
            PlayerFilter::Any,
            12usize,
            "Players may spend mana as though it were mana of any color".to_string(),
        )
    } else if clause_words.starts_with(&[
        "you", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any", "color",
    ]) {
        (PlayerFilter::You, 12usize, clause_words.join(" "))
    } else {
        return Ok(None);
    };

    let tail_tokens = trim_edge_punctuation(&tokens[tail_start..]);
    let permission = if tail_tokens.is_empty() {
        crate::effect::ManaSpendPermission::any_color(player)
    } else {
        let tail_words = words(&tail_tokens);
        if tail_words.starts_with(&["to", "pay", "the", "activation", "costs", "of"]) {
            let ability_words = words(&tail_tokens[6..]);
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
        } else if tail_words.starts_with(&["to", "activate", "abilities", "of"]) {
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
