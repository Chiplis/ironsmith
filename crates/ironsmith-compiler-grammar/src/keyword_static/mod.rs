mod characteristic_assertions;
use crate::cards::builders::PlayerPredicateAst;
use crate::cards::builders::PredicateAst;
use crate::cards::builders::SourcePredicateAst;
use crate::cards::builders::TurnEventPredicateAst;
use characteristic_assertions::parse_supertype_assertion_line;
mod blocking_permissions;
mod combat_requirements;
mod conditional_attack_requirement;
pub use conditional_attack_requirement::parse_conditional_attack_requirement_line;
pub use combat_requirements::{
    parse_self_combat_requirement_line, parse_combat_requirement_static_line,
    parse_source_owned_flying_block_limit_line,
};
mod dynamic_anthem_values;
mod dynamic_characteristic_statics;
pub use blocking_permissions::parse_blocking_capacity_static_line;
mod alternative_prices;
mod enters_tapped_untap_conjunction;
mod echo_cost_alternative;
mod commander_tax_life;
mod costs_replacements_and_permissions;
pub use alternative_prices::parse_independent_alternative_price_line;
mod damage_prevention;
mod damage_redirection;
pub use damage_redirection::parse_scoped_damage_redirection_line;
pub(crate) use damage_redirection::redirection_recipient_filters;
mod life_change_replacements;
mod event_instead_replacements;
pub use event_instead_replacements::parse_if_event_would_happen_instead_line;
mod event_amount_replacements;
pub use event_amount_replacements::{
    parse_damage_life_floor_static_line, parse_enters_or_escapes_instead_counters_line,
    parse_if_event_would_happen_amount_line,
    parse_you_may_look_at_additional_cards_each_time_line,
};
mod prevention_follow_ups;
pub use damage_prevention::{parse_filtered_damage_prevention_line, parse_permanent_self_damage_prevention_line, parse_persistent_filtered_damage_prevention_line, parse_shared_color_pair_damage_prevention_line};
pub use life_change_replacements::parse_if_you_would_gain_life_replacement_line;
pub use prevention_follow_ups::{
    parse_prevention_amount_follow_up_line, parse_prevention_proposed_amount_follow_up_line,
};
mod leading_conditional_sentence_chain;
mod devour_quality;
mod granted_encore;
pub use granted_encore::parse_granted_encore_line;
mod commander_ninjutsu;
pub use commander_ninjutsu::parse_commander_ninjutsu_line;
mod base_toughness_line;
pub use base_toughness_line::parse_base_toughness_only_line;
mod absorb_keyword;
pub use absorb_keyword::parse_absorb_keyword_line;
mod granted_casting_keywords;
pub use granted_casting_keywords::{
    parse_cast_from_zone_using_keyword_abilities_line, parse_granted_casting_keyword_line,
    parse_granted_madness_line,
};
mod granted_spell_keywords;
mod domain_landwalk;
pub use domain_landwalk::parse_domain_landwalk_line;
pub use granted_spell_keywords::{
    granted_intrinsic_spell_keyword_ability, parse_granted_spell_keyword_line,
};
mod leading_condition_wrapper;
pub use leading_condition_wrapper::parse_leading_condition_wrapped_static_line;
mod filtered_lure;
pub use filtered_lure::parse_filtered_creatures_able_to_block_source_line;
mod each_player_land_plays;
mod first_spell_permissions;
mod block_as_though_untapped;
pub use block_as_though_untapped::parse_can_block_as_though_untapped_line;
pub use first_spell_permissions::parse_first_spell_flash_permission_line;
pub use each_player_land_plays::parse_each_player_additional_land_play_line;
pub use devour_quality::parse_devour_quality_line;
pub use costs_replacements_and_permissions::*;

use super::activation_and_restrictions::activation_restriction_clauses::parse_negated_object_restriction_clause;
use super::activation_and_restrictions::{
    parse_ability_phrase, parse_activated_line, parse_activation_cost,
    parse_choose_land_type_phrase_words, parse_compiler_activation_cost,
    parse_payment_clause_as_total_cost, parse_single_word_keyword_action,
};
use super::activation_and_restrictions::{parse_cycling_line, parse_equip_line_lexed};
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
    is_dependent_cant_become_untapped_line_lexed,
    is_dependent_doesnt_untap_during_controller_untap_step_line_lexed,
    is_doctors_companion_marker_line_lexed,
    is_double_damage_from_sources_you_control_of_chosen_type_line_lexed,
    is_draw_replace_exile_top_face_down_line_lexed, is_draw_replacement_double_line_lexed,
    is_draw_replacement_skip_empty_library_line_lexed,
    is_draw_replacement_win_empty_library_line_lexed,
    is_during_your_turn_prevent_all_damage_to_source_line_lexed,
    is_effect_discard_to_library_replacement_line_lexed,
    is_enchanted_land_is_chosen_type_line_lexed,
    is_if_source_you_control_with_mana_value_double_instead_marker_line_lexed,
    is_krrik_black_mana_life_payment_line_lexed,
    is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed,
    is_lethal_damage_to_creatures_you_control_uses_power_line_lexed,
    is_mana_group_slash_marker_line_lexed, is_may_assign_damage_as_unblocked_line_lexed,
    is_minimum_spell_total_mana_three_line_lexed, is_more_than_meets_the_eye_marker_line_lexed,
    is_no_maximum_hand_size_line_lexed, is_once_each_turn_play_from_exile_marker_guard_lexed,
    is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed,
    is_permanents_enter_tapped_line_lexed, is_play_lands_from_graveyard_line_lexed,
    is_play_top_card_your_library_revealed_line_lexed, is_players_cant_cycle_line_lexed,
    is_players_cant_pay_life_or_sacrifice_line_lexed,
    is_players_play_top_card_libraries_revealed_line_lexed, is_players_skip_upkeep_line_lexed,
    is_prevent_all_combat_damage_to_matching_permanents_line_lexed,
    is_prevent_all_combat_damage_to_source_line_lexed,
    is_prevent_all_damage_dealt_to_creatures_line_lexed,
    is_prevent_all_damage_to_source_by_creatures_line_lexed,
    is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed,
    is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed,
    is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed,
    is_protection_mana_value_marker_line_lexed, is_remove_snow_line_lexed,
    is_sab_sunen_cant_attack_or_block_unless_line_lexed,
    is_shuffle_into_library_from_graveyard_line_lexed, is_skip_your_draw_step_line_lexed,
    is_skulk_rules_text_line_lexed, is_this_creature_cant_attack_alone_line_lexed,
    is_this_creature_cant_attack_its_owner_line_lexed, is_this_subject_reference_lexed,
    is_you_assign_combat_damage_of_creatures_attacking_you_line_lexed,
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
    parse_trigger_suppression_spec_lexed, split_as_long_as_condition_prefix_lexed,
    split_if_this_spell_costs_line_lexed, split_untap_each_other_players_untap_step_line_lexed,
};
use super::grammar::anthem_grants as anthem_grant_grammar;
use super::grammar::conditions::{
    PlayerLifeChangeDirectionAst, PlayerLifeChangeThisTurnConditionAst,
    parse_player_life_change_this_turn_condition,
};
use super::grammar::filters::{
    parse_object_filter_with_grammar_entrypoint, parse_spell_filter_with_grammar_entrypoint,
    parse_spell_filter_with_grammar_entrypoint_lexed,
};
use super::grammar::leaf::{
    parse_leaf_fixed_mana_cost_prefix_tokens, parse_leaf_mana_cost_prefix_tokens,
};
use super::grammar::permission_facts::graveyard_source as permission_graveyard_facts;
use super::grammar::primitives::{
    split_lexed_slices_on_and, split_lexed_slices_on_commas_or_semicolons,
    split_lexed_slices_on_period,
};
use super::grammar::static_keyword_facts::early as early_static_facts;
use super::grammar::static_keyword_facts::late as late_static_facts;
use super::grammar::static_keyword_facts::mid::{
    self as static_mid_facts, CostModifierDirectionFact as CostModifierDirection,
};
use super::grammar::static_keyword_facts::type_and_color as type_and_color_facts;
use super::grammar::static_keyword_shapes;
pub use super::grammar::values::parse_add_mana_equal_amount_value_lexed as parse_add_mana_equal_amount_value;
use super::grammar::values::parse_max_cards_in_hand_value_lexed;
use super::grammar::{
    attached_object_static_lines as attached_grammar, document_shapes as document_grammar,
    keyword_static_lines, static_keyword_cost_shapes, static_keyword_line_shapes,
    static_keyword_replacement_shapes, trigger_surface,
};
use super::keyword_static_helpers::*;
use super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, TokenWordView, contains_token_kind, locate_token_kind,
    parser_token_word_refs, render_token_slice, split_lexed_sentences, token_slice_first_is,
    trim_lexed_commas, word_slice_last_is_any,
};
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::rule_engine::{LexRuleHeadHint, LexRuleHintIndex, build_lex_rule_hint_index};
use super::static_ability_helpers::static_ability_for_keyword_action;
use super::token_primitives::{
    is_core_keyword_marker_tokens, is_ticket_sticker_marker_tokens, split_em_dash_label_prefix,
    split_em_dash_label_prefix_tokens,
};
use super::util::{
    comparison_to_at_least_threshold, is_source_reference_words, mana_pips_from_token,
    parse_alternative_cast_words, parse_card_type, parse_choice_count_token_prefix_consumed,
    parse_color, parse_counter_type_word, parse_flashback_keyword_line,
    parse_for_each_count_value_words, parse_greater_than_or_equal_quantity_prefix,
    parse_less_than_or_equal_quantity_prefix, parse_number_word_i32,
    parse_quantity_comparison_prefix, parse_subtype_flexible, parse_value, parse_value_expr_words,
    parse_zone_word, source_reference_surface_for_possessive_words,
    strip_leading_article_word_refs, strip_leading_token_words_any, strip_leading_word_refs_any,
    trim_commas, trim_edge_punctuation_tokens,
};
use super::util::{source_choose_spec_for_surface, source_reference_surface_for_words};
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, GrantedAbilityAst, KeywordAction, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, LineAst, ParsedAbility, PlayerAst,
    ReferenceImports, ReturnControllerAst, StaticAbilityAst, SubjectVerbActionAst,
    SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, TriggerSpec,
};
use crate::color::{Color, ColorSet};
use crate::cost::TotalCost;
use crate::effect::{ChoiceCount, Condition, Effect, EventValueSpec, Value};
use crate::grammar::shared_util::value_semantics::{
    parse_aggregate_scope_value_lexed, parse_commander_cast_count_player,
};
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::compiler_semantic::{
    CompilerAbilityCore as Ability, CompilerAbilityKindCore as AbilityKind,
    CompilerTriggeredAbilityCore as TriggeredAbility,
};
use crate::model::{
    CompilerGrantAbilityCore as GrantAbility,
    CompilerPowerToughnessChoiceOptionCore as PowerToughnessChoiceOption,
    CompilerStaticAbilityCore as StaticAbility,
};
use crate::object::CounterType;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};
use crate::static_abilities::{Anthem, AnthemCountExpression, AnthemValue};
use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use ironsmith_compiler_semantic::keyword_abilities::assemble_parsed_triggered_ability as parsed_triggered_ability;
use ironsmith_core::{EffectMetric, EffectMetricSource};
use std::sync::LazyLock;

fn chosen_name_source_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.name = Some("{chosen name}".to_string());
    filter
}

fn parse_once_each_turn_paid_die_reroll_line(tokens: &[OwnedLexToken]) -> Option<StaticAbilityAst> {
    let pay_index = crate::slice_primitives::select_position(tokens, |token| token.is_word("pay"))?;
    if !crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(&tokens[..pay_index]),
        &["once", "each", "turn", "you", "may"],
    ) {
        return None;
    }
    let mana = parse_leaf_mana_cost_prefix_tokens(&tokens[pay_index + 1..])?;
    let tail = tokens.get(pay_index + 1 + mana.consumed..)?;
    if !crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(tail),
        &["to", "reroll", "one", "or", "more", "dice", "you", "rolled"],
    ) {
        return None;
    }
    Some(
        StaticAbility::die_roll_reroll(
            PlayerFilter::You,
            mana.cost,
            true,
            render_token_slice(tokens),
        )
        .into(),
    )
}

fn two_card_type_union_filter(left_type: CardType, right_type: CardType) -> ObjectFilter {
    let mut left_filter = ObjectFilter::default();
    left_filter.zone = Some(Zone::Battlefield);
    left_filter.card_types = vec![left_type];

    let mut right_filter = ObjectFilter::default();
    right_filter.zone = Some(Zone::Battlefield);
    right_filter.card_types = vec![right_type];

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = vec![left_filter, right_filter];
    disjunction
}

fn activated_ability_subject_special_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    match keyword_static_lines::parse_activated_ability_special_subject_tokens(tokens)? {
        keyword_static_lines::ActivatedAbilitySpecialSubject::ChosenName => {
            Some(chosen_name_source_filter())
        }
        keyword_static_lines::ActivatedAbilitySpecialSubject::TwoCardTypes(left, right) => {
            Some(two_card_type_union_filter(left, right))
        }
    }
}

fn parse_life_total_or_less_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;

    let shape = early_static_facts::parse_life_total_cost_condition_shape_tokens(tokens)?;
    let quantity_tokens = &tokens[shape.quantity_tokens];
    let (amount, used) =
        crate::grammar::primitives::probe_shape(parse_less_than_or_equal_quantity_prefix(
            quantity_tokens,
            false,
            false,
            "life total cost condition",
        ))
        .flatten()?;
    (used == shape.quantity_words)
        .then_some(ThisSpellCostCondition::YouLifeTotalOrLess(amount as i32))
}

pub fn parse_can_be_attached_only_to_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(attachment) = static_keyword_line_shapes::parse_attachment_restriction_span(tokens)
    else {
        return Ok(None);
    };
    if attachment.start == 0 {
        return Ok(None);
    }
    let target_tokens = trim_commas(&tokens[attachment.end..]);
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

fn parse_enchant_attachment_restriction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if tokens.first().map(OwnedLexToken::parser_text) != Some("enchant") {
        return Ok(None);
    }
    let target_tokens = tokens.get(1..).unwrap_or_default();
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let words = parser_token_word_refs(target_tokens);
    let filter = if crate::word_primitives::parse_sequence_complete(&words, &["player"]) {
        crate::object::AuraAttachmentFilter::Player(crate::target::PlayerFilter::Any)
    } else if crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[&["opponent"], &["an", "opponent"]],
    ) {
        crate::object::AuraAttachmentFilter::Player(crate::target::PlayerFilter::Opponent)
    } else if crate::word_primitives::parse_sequence_complete(&words, &["you"]) {
        crate::object::AuraAttachmentFilter::Player(crate::target::PlayerFilter::You)
    } else {
        crate::object::AuraAttachmentFilter::Object(parse_object_filter_lexed(
            target_tokens,
            false,
        )?)
    };
    Ok(Some(StaticAbilityAst::AttachmentRestriction {
        filter,
        display: render_token_slice(&tokens),
    }))
}

const AS_ENTERS_AURA_SUBJECTS: &[(&str, &str)] = &[("aura", "this aura")];

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
    let mut text = keyword_static_clause_text(tokens);
    if supported_keyword_marker_tokens(tokens) {
        if matches!(
            early_static_facts::parse_early_keyword_marker_tokens(tokens),
            Some(early_static_facts::EarlyKeywordMarkerKind::GreaterPowerCrewsVehicles)
        ) && !crate::string_primitives::ends_with_char(&text, '.')
        {
            text.push('.');
        }
        return StaticAbility::keyword_marker(text);
    }
    StaticAbility::keyword_fallback_text(text)
}

fn parse_companion_ability(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    let condition = keyword_static_lines::parse_companion_deck_condition_tokens(tokens)?;
    let text = split_em_dash_label_prefix_tokens(tokens)
        .map(|(_, condition_tokens)| keyword_static_clause_text(condition_tokens))
        .unwrap_or_else(|| keyword_static_clause_text(tokens));

    Some(StaticAbility::companion(condition, text))
}

fn supported_keyword_marker_tokens(tokens: &[OwnedLexToken]) -> bool {
    is_core_keyword_marker_tokens(tokens)
        || early_static_facts::parse_early_keyword_marker_tokens(tokens).is_some()
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

type StaticAbilityLineParser = fn(&[OwnedLexToken]) -> ParseOutcome<Vec<StaticAbilityAst>>;

#[derive(Clone, Copy)]
struct StaticAbilityLineRuleDef {
    id: RuleId,
    parse: StaticAbilityLineParser,
}

type StaticAbilityLineHeadHint = LexRuleHeadHint;

fn run_static_ability_ast_line_rule(
    rule_id: RuleId,
    parse: StaticAbilityLineParser,
    tokens: &[OwnedLexToken],
) -> ParseOutcome<Vec<StaticAbilityAst>> {
    parse(tokens).within(rule_id)
}

fn static_single_rule_outcome(
    rule_id: RuleId,
    tokens: &[OwnedLexToken],
    result: Result<Option<StaticAbilityAst>, CardTextError>,
) -> ParseOutcome<Vec<StaticAbilityAst>> {
    let span = crate::util::span_from_tokens(tokens);
    match result {
        Ok(Some(ability)) => ParseOutcome::matched(vec![ability], span),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => {
            ParseOutcome::Error(ParseDiagnostic::from_card_text_error(rule_id, span, error))
        }
    }
}

fn static_multi_rule_outcome(
    rule_id: RuleId,
    tokens: &[OwnedLexToken],
    result: Result<Option<Vec<StaticAbilityAst>>, CardTextError>,
) -> ParseOutcome<Vec<StaticAbilityAst>> {
    let span = crate::util::span_from_tokens(tokens);
    match result {
        Ok(Some(abilities)) => ParseOutcome::matched(abilities, span),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => {
            ParseOutcome::Error(ParseDiagnostic::from_card_text_error(rule_id, span, error))
        }
    }
}

fn replay_static_rule_parse_loss(report: &crate::parse_loss::ParseLossReport) {
    for diagnostic in report.diagnostics() {
        crate::parse_loss::record(diagnostic.code.clone(), diagnostic.message.clone());
    }
}

fn static_ability_rule_head_hints(rule_id: RuleId) -> Vec<StaticAbilityLineHeadHint> {
    if rule_id.as_str().contains("_as_enters_") || rule_id.as_str().ends_with("_as_enters_line") {
        return vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "this"),
        ];
    }
    match rule_id.as_str() {
        "parse_enchant_attachment_restriction_line" => {
            vec![StaticAbilityLineHeadHint::Single("enchant")]
        }
        "parse_characteristic_defining_pt_line"
        | "parse_combat_requirement_static_line"
        | "parse_source_owned_flying_block_limit_line" => Vec::new(),
        // The complete assignment suffix proves its grammar; its source,
        // filtered-set and leading-condition subjects have no single head.
        "parse_filtered_toughness_assignment_line" => Vec::new(),
        // These rule names describe semantic operations, not the lexical
        // subjects accepted by their complete grammars. Deriving a head from
        // the name silently made already-supported static families unreachable.
        "parse_creatures_assign_combat_damage_using_toughness_line" => vec![
            StaticAbilityLineHeadHint::Single("each"),
            StaticAbilityLineHeadHint::Single("this"),
        ],
        "parse_attached_prevent_all_damage_dealt_to_and_by_attached_line"
        | "parse_attached_prevent_all_damage_dealt_by_attached_line"
        | "parse_attached_prevent_all_combat_damage_dealt_by_attached_line"
        | "parse_attached_prevent_all_damage_dealt_to_attached_line" => {
            vec![StaticAbilityLineHeadHint::Single("prevent")]
        }
        "parse_scoped_damage_redirection_line" => vec![
            StaticAbilityLineHeadHint::Single("all"),
            StaticAbilityLineHeadHint::Single("as"),
        ],
        "parse_damage_redirect_to_source_line" => {
            vec![StaticAbilityLineHeadHint::Single("all")]
        }
        "parse_damage_redirect_to_source_controller_line" => {
            vec![StaticAbilityLineHeadHint::Single("if")]
        }
        "parse_choose_color_as_becomes_attached_line" => {
            vec![StaticAbilityLineHeadHint::Single("as")]
        }
        // A land-animation subject is an arbitrary object filter, including
        // subtypes such as "Forests". The complete copular animation grammar
        // supplies the discriminator; no finite lexical head list is complete.
        "parse_lands_are_pt_creatures_still_lands_line" => Vec::new(),
        "parse_soulbond_shared_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "long"),
        ],
        "parse_reduced_maximum_hand_size_line" => vec![
            StaticAbilityLineHeadHint::Single("your"),
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Single("each"),
            StaticAbilityLineHeadHint::Single("opponent"),
            StaticAbilityLineHeadHint::Single("opponents"),
            StaticAbilityLineHeadHint::Single("player"),
            StaticAbilityLineHeadHint::Single("players"),
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("the", "chosen"),
        ],
        "parse_base_pt_and_blocker_restriction_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Single("creatures"),
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Single("during"),
        ],
        "parse_conditional_no_defender_and_unblockable_line" => vec![
            StaticAbilityLineHeadHint::Pair("as", "long"),
        ],
        "parse_can_be_attached_only_to_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "equipment"),
        ],
        "parse_conditional_source_spell_keyword_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Pair("if", "this"),
        ],
        "parse_damage_prevention_with_owner_shuffle_line" => {
            vec![StaticAbilityLineHeadHint::Single("if")]
        }
        "parse_damage_amount_replacement_line" => vec![StaticAbilityLineHeadHint::Single("if")],
        "parse_prevent_half_damage_replacement_line" => {
            vec![StaticAbilityLineHeadHint::Single("if")]
        }
        "parse_if_you_would_draw_instead_effects_line" => {
            vec![StaticAbilityLineHeadHint::Single("if")]
        }
        "parse_activate_abilities_as_though_haste_line" => {
            vec![StaticAbilityLineHeadHint::Single("you")]
        }
        "parse_zero_loyalty_state_based_exception_line" => {
            vec![StaticAbilityLineHeadHint::Single("planeswalkers")]
        }
        "parse_loyalty_abilities_any_time_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Single("as"),
        ],
        "parse_play_from_top_pay_life_line" => vec![StaticAbilityLineHeadHint::Single("you")],
        "parse_double_counters_replacement_line" => vec![StaticAbilityLineHeadHint::Single("if")],
        // "Skip your upkeep step [if <condition>]" may follow an ability-word
        // label ("Hellbent — Skip your upkeep step if ..."), and the rule name
        // only derives the "players" head. Both readings are fully anchored.
        "parse_players_skip_upkeep_line" => Vec::new(),
        "parse_players_skip_extra_turns_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Pair("if", "an"),
            StaticAbilityLineHeadHint::Pair("if", "a"),
            StaticAbilityLineHeadHint::Pair("if", "you"),
        ],
        "parse_prevent_damage_to_other_creature_you_control_put_counters_line"
        | "parse_prevent_damage_to_source_remove_counter_line"
        | "parse_prevent_damage_to_source_put_counters_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Pair("if", "damage"),
            StaticAbilityLineHeadHint::Single("for"),
            StaticAbilityLineHeadHint::Pair("for", "each"),
        ],
        "parse_conditional_all_creatures_able_to_block_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "long"),
        ],
        "parse_attached_all_creatures_able_to_block_line" => vec![
            StaticAbilityLineHeadHint::Single("all"),
            StaticAbilityLineHeadHint::Pair("all", "creatures"),
        ],
        "parse_attached_type_transform_line" => vec![
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Single("equipped"),
        ],
        "parse_attached_cant_attack_or_block_line" => vec![
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Pair("enchanted", "permanent"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
        ],
        "parse_subject_has_keywords_and_cant_be_blocked_line"
        | "parse_subject_has_keywords_and_cant_be_blocked_by_more_than_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "long"),
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "creature"),
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
        ],
        "parse_subject_cant_be_blocked_and_has_keywords_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "creature"),
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
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
        "parse_spell_additional_life_cost_per_target_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "spell"),
        ],
        // The parser owns both plural-filter subjects ("Creature spells")
        // and the singular self subject ("This spell").  Deriving only the
        // word `spells` from its function name made the latter unreachable
        // through the migrated registry.
        "parse_spells_cost_modifier_line" => vec![
            StaticAbilityLineHeadHint::Single("spells"),
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
        "parse_attached_can_attack_as_though_haste_line" => vec![
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
        ],
        "parse_attached_can_attack_as_though_no_defender_line" => vec![
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Pair("enchanted", "wall"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
            StaticAbilityLineHeadHint::Single("attached"),
        ],
        "parse_plain_can_attack_as_though_no_defender_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Single("wall"),
            StaticAbilityLineHeadHint::Single("walls"),
            StaticAbilityLineHeadHint::Single("creature"),
            StaticAbilityLineHeadHint::Single("creatures"),
            StaticAbilityLineHeadHint::Single("modified"),
        ],
        "parse_source_can_block_shadow_as_though_no_shadow_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "creature"),
        ],
        "parse_attacked_player_can_attack_as_though_no_defender_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Pair("this", "creature"),
        ],
        "parse_targeting_as_though_no_ability_line" => vec![
            StaticAbilityLineHeadHint::Single("creatures"),
            StaticAbilityLineHeadHint::Single("your"),
            StaticAbilityLineHeadHint::Single("autumn"),
        ],
        "parse_attached_restriction_and_granted_ability_line" => vec![
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "creature"),
            StaticAbilityLineHeadHint::Pair("enchanted", "permanent"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Pair("equipped", "creature"),
        ],
        "parse_attached_land_ability_reset_line" => vec![
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Pair("enchanted", "land"),
        ],
        "parse_no_maximum_hand_size_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Single("players"),
            StaticAbilityLineHeadHint::Single("each"),
            StaticAbilityLineHeadHint::Single("your"),
            StaticAbilityLineHeadHint::Pair("the", "chosen"),
        ],
        "parse_opponents_must_target_flagbearers_line" => {
            vec![StaticAbilityLineHeadHint::Pair("while", "an")]
        }
        "parse_prevent_all_damage_to_you_line" | "parse_permanent_self_damage_prevention_line"
        | "parse_persistent_filtered_damage_prevention_line"
        | "parse_shared_color_pair_damage_prevention_line" => {
            vec![StaticAbilityLineHeadHint::Pair("prevent", "all")]
        }
        "parse_if_you_would_gain_life_replacement_line"
        | "parse_prevention_proposed_amount_follow_up_line"
        | "parse_prevention_amount_follow_up_line"
        | "parse_filtered_damage_prevention_line" => {
            vec![StaticAbilityLineHeadHint::Single("if")]
        }
        "parse_prevent_damage_to_you_from_source_filter_line" => vec![
            StaticAbilityLineHeadHint::Pair("if", "a"),
            StaticAbilityLineHeadHint::Pair("if", "an"),
        ],
        "parse_search_limited_to_top_cards_line" => vec![
            StaticAbilityLineHeadHint::Pair("if", "an"),
            StaticAbilityLineHeadHint::Pair("if", "a"),
        ],
        "parse_untap_step_limit_line" => vec![
            StaticAbilityLineHeadHint::Single("players"),
            StaticAbilityLineHeadHint::Pair("each", "player"),
            StaticAbilityLineHeadHint::Single("you"),
        ],
        "parse_untap_during_each_other_players_untap_step_line" => vec![
            StaticAbilityLineHeadHint::Single("untap"),
            StaticAbilityLineHeadHint::Pair("untap", "all"),
        ],
        "parse_may_choose_not_to_untap_during_untap_step_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_you_may_static_grant_line" => vec![
            StaticAbilityLineHeadHint::Pair("once", "each"),
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
            StaticAbilityLineHeadHint::Single("during"),
            StaticAbilityLineHeadHint::Pair("during", "each"),
            StaticAbilityLineHeadHint::Single("once"),
            StaticAbilityLineHeadHint::Pair("once", "during"),
        ],
        "parse_as_you_cascade_land_drop_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "you"),
        ],
        "parse_play_from_permission_with_haste_this_way_line"
        | "parse_play_from_permission_with_enter_counter_this_way_line"
        | "parse_play_from_permission_with_enter_tapped_this_way_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
            StaticAbilityLineHeadHint::Single("once"),
            StaticAbilityLineHeadHint::Pair("once", "during"),
        ],
        "parse_you_have_protection_from_opponents_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "have"),
        ],
        "parse_player_may_cast_spells_free_and_flash_line" => vec![
            StaticAbilityLineHeadHint::Pair("any", "player"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_you_may_cast_exile_counter_cards_with_mana_permission_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
            StaticAbilityLineHeadHint::Single("during"),
            StaticAbilityLineHeadHint::Pair("during", "your"),
        ],
        "parse_can_block_as_though_untapped_line" => vec![
            StaticAbilityLineHeadHint::Single("tapped"),
            StaticAbilityLineHeadHint::Pair("tapped", "creatures"),
        ],
        "parse_first_spell_flash_permission_line" => vec![
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_surveilled_graveyard_play_life_cost_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_source_exiled_play_life_cost_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
            StaticAbilityLineHeadHint::Single("during"),
            StaticAbilityLineHeadHint::Pair("during", "your"),
        ],
        "parse_independent_alternative_price_line" => vec![
            StaticAbilityLineHeadHint::Single("rather"),
            StaticAbilityLineHeadHint::Pair("rather", "than"),
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
            StaticAbilityLineHeadHint::Single("once"),
            StaticAbilityLineHeadHint::Pair("once", "each"),
        ],
        "parse_fixed_mana_cost_instead_of_mana_cost_grant_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_enter_as_copy_as_enters_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_damage_life_floor_static_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Single("damage"),
        ],
        "parse_you_may_look_at_additional_cards_each_time_line" => vec![
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
        "parse_self_or_global_hands_revealed_line" => vec![
            StaticAbilityLineHeadHint::Single("play"),
            StaticAbilityLineHeadHint::Single("players"),
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
        "parse_filtered_creatures_able_to_block_source_line" => {
            vec![StaticAbilityLineHeadHint::Single("all")]
        }
        "parse_conditional_attack_requirement_line" => {
            vec![StaticAbilityLineHeadHint::Single("if")]
        }
        "parse_leading_condition_wrapped_static_line" => vec![
            StaticAbilityLineHeadHint::Pair("during", "your"),
            StaticAbilityLineHeadHint::Pair("as", "long"),
        ],
        "parse_commander_ninjutsu_line" => vec![StaticAbilityLineHeadHint::Pair("commander", "ninjutsu")],
        // The rule reads every alternative casting kind ("Dash costs you pay
        // cost {2} less", Warbringer), not only flashback.
        "parse_flashback_cost_modifier_line" => vec![
            StaticAbilityLineHeadHint::Single("flashback"),
            StaticAbilityLineHeadHint::Single("dash"),
            StaticAbilityLineHeadHint::Single("blitz"),
            StaticAbilityLineHeadHint::Single("escape"),
            StaticAbilityLineHeadHint::Single("madness"),
            StaticAbilityLineHeadHint::Single("miracle"),
            StaticAbilityLineHeadHint::Single("suspend"),
            StaticAbilityLineHeadHint::Single("foretell"),
            StaticAbilityLineHeadHint::Single("jump"),
            StaticAbilityLineHeadHint::Single("jump-start"),
            StaticAbilityLineHeadHint::Single("jumpstart"),
        ],
        "parse_devour_quality_line" => vec![StaticAbilityLineHeadHint::Single("devour")],
        "parse_each_player_additional_land_play_line" => vec![
            StaticAbilityLineHeadHint::Single("each"),
            StaticAbilityLineHeadHint::Pair("each", "player"),
        ],
        "parse_play_lands_from_graveyard_line" => vec![
            StaticAbilityLineHeadHint::Single("you"),
            StaticAbilityLineHeadHint::Pair("you", "may"),
        ],
        "parse_during_your_turn_graveyard_cards_have_retrace_line" => {
            vec![StaticAbilityLineHeadHint::Single("during")]
        }
        // The subject may be any subtype list ("Merfolk and Druid cards in
        // your graveyard have retrace"); the fully anchored retrace grammar is
        // its own discriminator.
        "parse_graveyard_cards_have_retrace_line" => Vec::new(),
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
        "parse_lose_game_replacement_line" | "parse_token_creation_templates_line" => {
            vec![StaticAbilityLineHeadHint::Single("if")]
        }
        // The rule name describes the changed characteristic, not the
        // authored subject. "Enchanted land" must reach its complete tail.
        "parse_land_type_addition_line" => Vec::new(),
        // "Each nonland permanent you control is all colors." (Leyline of the
        // Guildpact) and the older "All creatures are ..." lines share one
        // color-identity grammar whose subject can start with any object noun.
        "parse_all_creatures_are_color_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Single("all"),
            StaticAbilityLineHeadHint::Single("each"),
            StaticAbilityLineHeadHint::Single("creatures"),
            StaticAbilityLineHeadHint::Single("creature"),
            StaticAbilityLineHeadHint::Single("permanents"),
            StaticAbilityLineHeadHint::Single("permanent"),
            StaticAbilityLineHeadHint::Single("nonland"),
            StaticAbilityLineHeadHint::Single("noncreature"),
            StaticAbilityLineHeadHint::Single("nontoken"),
            StaticAbilityLineHeadHint::Single("lands"),
            StaticAbilityLineHeadHint::Single("artifacts"),
            StaticAbilityLineHeadHint::Single("enchantments"),
            StaticAbilityLineHeadHint::Single("spells"),
            StaticAbilityLineHeadHint::Single("other"),
        ],
        "parse_shuffle_into_library_from_graveyard_line" => vec![
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Pair("if", "this"),
        ],
        // These source-ETB rule names describe the predicate rather than the
        // lexical subject. Their typed grammars accept source references (and
        // the counter family also accepts a leading condition), so indexing
        // them only under the derived word `enters` makes them unreachable.
        "parse_enters_under_chosen_control_line"
        | "parse_enters_prepared_line"
        | "parse_enters_tapped_line"
        | "parse_enters_tapped_with_choose_color_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Single("it"),
        ],
        "parse_enters_tapped_with_counters_line"
        | "parse_enters_with_counters_line"
        | "parse_enters_or_escapes_instead_counters_line" => vec![
            StaticAbilityLineHeadHint::Single("this"),
            StaticAbilityLineHeadHint::Single("it"),
            StaticAbilityLineHeadHint::Single("if"),
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Single("converge"),
            StaticAbilityLineHeadHint::Single("morbid"),
            StaticAbilityLineHeadHint::Single("revolt"),
            StaticAbilityLineHeadHint::Single("undergrowth"),
        ],
        // The entering subject of these filtered ETB replacements is an
        // open-ended object filter ("Gates you control", "Non-Phyrexian
        // creatures", "Legendary creatures you control"), so no finite head
        // list is complete. Each grammar requires the complete
        // "<filter> enter(s) tapped/untapped/with ... counter" shape and
        // declines source, pronoun and trigger-intro subjects itself.
        "parse_enters_tapped_for_filter_line"
        | "parse_enters_untapped_for_filter_line"
        | "parse_enters_with_additional_counter_for_filter_line" => Vec::new(),
        // These rule names describe the choice payload, while every accepted
        // source line begins with the replacement-style `As this ... enters`
        // subject. Keep the correlated/named choice specialists reachable
        // before the broad document fallback flattens their alternatives.
        "parse_choose_basic_land_type_as_enters_line"
        | "parse_revealed_hand_choose_nonland_card_name_as_enters_line"
        | "parse_choose_card_name_as_enters_line"
        | "parse_choose_color_and_creature_type_as_enters_line"
        | "parse_choose_color_creature_type_pairs_as_enters_line"
        | "parse_choose_creature_type_as_enters_line"
        | "parse_choose_named_options_as_enters_line"
        | "parse_choose_player_as_enters_line"
        | "parse_note_life_total_as_enters_line"
        | "parse_discard_hand_as_enters_line" => vec![
            StaticAbilityLineHeadHint::Single("as"),
            StaticAbilityLineHeadHint::Pair("as", "this"),
            StaticAbilityLineHeadHint::Pair("as", "it"),
        ],
        // The grant form names its recipients with an open-ended object
        // filter ("Foods you control", "Creatures you control with +1/+1
        // counters on them"), so no lexical head enumerates this rule's
        // subjects. Its own grammar requires the "has/have all activated
        // abilities of" marker, which guards the whole-line candidacy.
        "parse_copy_activated_abilities_line" => Vec::new(),
        "parse_attached_has_and_loses_keywords_line"
        | "parse_attached_has_keywords_and_is_goaded_line"
        | "parse_attached_has_keywords_and_negated_restriction_line"
        | "parse_attached_is_goaded_line" => vec![
            StaticAbilityLineHeadHint::Single("enchanted"),
            StaticAbilityLineHeadHint::Single("equipped"),
            StaticAbilityLineHeadHint::Single("fortified"),
        ],
        _ => match static_keyword_shapes::parse_rule_id_head(rule_id.as_str()) {
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

#[derive(Debug, Clone)]
enum StaticAbilityLineHeadDiscriminator {
    Lexical(Vec<StaticAbilityLineHeadHint>),
    WholeLine,
}

fn static_ability_rule_head_discriminator(rule_id: RuleId) -> StaticAbilityLineHeadDiscriminator {
    let hints = static_ability_rule_head_hints(rule_id);
    if hints.is_empty() {
        StaticAbilityLineHeadDiscriminator::WholeLine
    } else {
        StaticAbilityLineHeadDiscriminator::Lexical(hints)
    }
}

#[cfg(test)]
mod repeated_static_head_tests;

#[cfg(test)]
mod registry_head_hint_tests {
    use super::*;

    #[test]
    fn conjoined_type_animation_reaches_the_full_static_parser() {
        let tokens = crate::lexer::lex_line(
            "All Forests and all Saprolings are 2/2 green Saproling creatures and Forest lands in addition to their other types.", 0,
        ).unwrap();
        assert!(
            parse_all_are_pt_color_type_addition_line(&tokens)
                .unwrap()
                .is_some()
        );
        assert!(registry_result(&tokens).unwrap().is_some());
        assert!(
            parse_static_ability_ast_line_lexed(&tokens)
                .unwrap()
                .is_some()
        );
    }

    fn registry_result(
        tokens: &[OwnedLexToken],
    ) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
        match recognize_static_ability_ast_line_registry(tokens) {
            ParseOutcome::NoMatch => Ok(None),
            ParseOutcome::Match(matched) => Ok(Some(matched.value)),
            ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
        }
    }

    #[test]
    fn source_enters_tapped_rule_is_reachable_from_its_subject_head() {
        let tokens = crate::lexer::lex_line("This land enters tapped.", 0)
            .expect("source ETB line should lex");
        let parsed = registry_result(&tokens)
            .expect("source ETB registry parse should not error")
            .expect("source ETB registry should claim the line");

        assert!(parsed.iter().any(|ability| {
            matches!(
                ability,
                StaticAbilityAst::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTapped
            )
        }));
    }

    #[test]
    fn extra_turn_skip_rule_is_reachable_from_its_if_head() {
        let tokens = crate::lexer::lex_line(
            "If an opponent would begin an extra turn, that player skips that turn instead.",
            0,
        )
        .expect("extra-turn skip line should lex");
        let parsed = registry_result(&tokens)
            .expect("extra-turn skip registry parse should not error")
            .expect("extra-turn skip registry should claim the line");

        assert!(parsed.iter().any(|ability| {
            matches!(
                ability,
                StaticAbilityAst::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::PlayersSkipExtraTurns
            )
        }));
    }

    #[test]
    fn battlefield_conditioned_copy_activated_rule_is_reachable_from_as_head() {
        let tokens = crate::lexer::lex_line(
            "As long as this artifact is on the battlefield, it has all activated abilities of all land cards in all graveyards.",
            0,
        )
        .expect("conditioned copy-ability line should lex");
        assert!(
            parse_copy_activated_abilities_line(&tokens)
                .expect("direct copy-ability parse should not error")
                .is_some(),
            "the specialist must own the complete conditioned line"
        );
        let parsed = registry_result(&tokens)
            .expect("registry parse should not error")
            .expect("the `as` head must reach the copy-ability rule");
        assert!(format!("{parsed:#?}").contains("CopyActivatedAbilities"));
    }

    #[test]
    fn untap_step_limit_rule_is_reachable_and_typed() {
        for (text, head, second, max) in [
            (
                "Players can't untap more than one nonbasic land during their untap steps.",
                "players",
                "cant",
                1,
            ),
            (
                "You can't untap more than two permanents during your untap step.",
                "you",
                "cant",
                2,
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).expect("untap limit line should lex");
            let direct = parse_untap_step_limit_line(&tokens)
                .expect("direct untap limit parse should not error")
                .expect("direct untap limit parser should claim the line");
            assert!(
                format!("{direct:?}").contains(&format!("max: {max}")),
                "{direct:?}"
            );
            let rule_idx = static_ability_ast_line_rules()
                .iter()
                .position(|rule| rule.id.as_str() == "parse_untap_step_limit_line")
                .expect("untap limit rule should be registered");
            assert!(
                STATIC_ABILITY_AST_LINE_RULE_INDEX
                    .candidate_indices(head, Some(second))
                    .contains(&rule_idx),
                "untap limit rule should be indexed under `{head}`"
            );
            let parsed = registry_result(&tokens)
                .expect("untap limit registry parse should not error")
                .expect("untap limit registry should claim the line");
            assert!(
                format!("{parsed:?}").contains("UntapStepLimit"),
                "{parsed:?}"
            );
        }
    }

    #[test]
    fn optional_untap_rule_is_reachable_from_you_may_head() {
        let tokens = crate::lexer::lex_line(
            "You may choose not to untap this artifact during your untap step.",
            0,
        )
        .expect("optional untap line should lex");
        assert!(
            parse_may_choose_not_to_untap_during_untap_step_line(&tokens)
                .expect("direct optional untap parse should not error")
                .is_some(),
            "direct optional untap parser should claim the line"
        );
        let rule_idx = static_ability_ast_line_rules()
            .iter()
            .position(|rule| {
                rule.id.as_str() == "parse_may_choose_not_to_untap_during_untap_step_line"
            })
            .expect("optional untap rule should be registered");
        assert!(
            STATIC_ABILITY_AST_LINE_RULE_INDEX
                .candidate_indices("you", Some("may"))
                .contains(&rule_idx),
            "optional untap rule should be indexed under `you may`"
        );
        assert!(matches!(
            run_static_ability_ast_line_rule(
                static_ability_ast_line_rules()[rule_idx].id,
                static_ability_ast_line_rules()[rule_idx].parse,
                &tokens,
            ),
            ParseOutcome::Match(_)
        ));
        let parsed = registry_result(&tokens)
            .expect("optional untap registry parse should not error")
            .expect("optional untap registry should claim the line");

        assert!(parsed.iter().any(|ability| {
            matches!(
                ability,
                StaticAbilityAst::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::MayChooseNotToUntapDuringUntapStep
            )
        }));
    }

    #[test]
    fn correlated_color_type_choice_is_reachable_from_as_this_head() {
        let tokens = crate::lexer::lex_line(
            "As this artifact enters, choose white Citizen, blue Camarid, black Thrull, red Goblin, or green Saproling.",
            0,
        )
        .expect("correlated color/type choice should lex");
        assert!(
            parse_choose_color_creature_type_pairs_as_enters_line(&tokens)
                .expect("direct correlated choice parse should not error")
                .is_some(),
            "the correlated-choice specialist must own the complete line"
        );

        let rule_idx = static_ability_ast_line_rules()
            .iter()
            .position(|rule| {
                rule.id.as_str() == "parse_choose_color_creature_type_pairs_as_enters_line"
            })
            .expect("correlated-choice rule should be registered");
        assert!(
            STATIC_ABILITY_AST_LINE_RULE_INDEX
                .candidate_indices("as", Some("this"))
                .contains(&rule_idx),
            "correlated-choice rule should be indexed under `as this`"
        );

        let parsed = registry_result(&tokens)
            .expect("correlated choice registry parse should not error")
            .expect("the `as this` head should reach the correlated-choice rule");
        assert!(format!("{parsed:#?}").contains("ChooseNamedOptionAsEnters"));

        let malformed = crate::lexer::lex_line(
            "As this artifact enters, choose white Citizen, blue Camarid, or black.",
            0,
        )
        .expect("changed-pair near miss should lex");
        assert!(
            parse_choose_color_creature_type_pairs_as_enters_line(&malformed)
                .expect("changed-pair near miss should not error")
                .is_none(),
            "an incomplete color/type pair must remain outside the specialist"
        );
    }

    #[test]
    fn attached_must_block_rule_is_reachable_from_all_creatures_head() {
        let tokens =
            crate::lexer::lex_line("All creatures able to block enchanted creature do so.", 0)
                .expect("attached must-block line should lex");
        assert!(
            parse_attached_all_creatures_able_to_block_line(&tokens)
                .expect("direct attached must-block parse should not error")
                .is_some(),
            "direct attached must-block parser should claim the line"
        );
        let rule_idx = static_ability_ast_line_rules()
            .iter()
            .position(|rule| rule.id.as_str() == "parse_attached_all_creatures_able_to_block_line")
            .expect("attached must-block rule should be registered");
        assert!(
            STATIC_ABILITY_AST_LINE_RULE_INDEX
                .candidate_indices("all", Some("creatures"))
                .contains(&rule_idx),
            "attached must-block rule should be indexed under `all creatures`"
        );
        assert!(matches!(
            run_static_ability_ast_line_rule(
                static_ability_ast_line_rules()[rule_idx].id,
                static_ability_ast_line_rules()[rule_idx].parse,
                &tokens,
            ),
            ParseOutcome::Match(_)
        ));
        let parsed = registry_result(&tokens)
            .expect("attached must-block registry parse should not error")
            .expect("attached must-block registry should claim the line");
        assert!(format!("{parsed:#?}").contains("MustBlockSpecificAttacker"));

        let changed_target =
            crate::lexer::lex_line("All creatures able to block target creature do so.", 0)
                .expect("changed-target near miss should lex");
        assert!(
            parse_attached_all_creatures_able_to_block_line(&changed_target)
                .expect("changed-target near miss should not error")
                .is_none(),
            "the attached rule must not consume a target-creature spell effect"
        );
    }

    #[test]
    fn leading_counter_condition_no_defender_rule_is_reachable_from_as_long_head() {
        let tokens = crate::lexer::lex_line(
            "As long as this creature has three or more judgment counters on it, it can attack as though it didn't have defender.",
            0,
        )
        .expect("conditional attack permission should lex");
        assert!(
            parse_as_long_as_condition_can_attack_as_though_no_defender_line(&tokens)
                .expect("direct conditional attack-permission parse should not error")
                .is_some(),
            "the specialist must own the complete conditional line"
        );

        let parsed = registry_result(&tokens)
            .expect("conditional attack-permission registry parse should not error")
            .expect("the `as long` head should reach the conditional attack-permission rule");
        assert!(format!("{parsed:#?}").contains("CanAttackAsThoughNoDefender"));
    }

    #[test]
    fn once_each_turn_graveyard_permission_is_reachable_from_once_head() {
        let tokens = crate::lexer::lex_line(
            "Once during each of your turns, you may cast a permanent spell with mana value 2 or less from your graveyard.",
            0,
        )
        .expect("graveyard permission line should lex");
        let parsed = registry_result(&tokens)
            .expect("graveyard permission registry parse should not error")
            .expect("graveyard permission registry should claim the line");

        assert!(parsed.iter().any(|ability| {
            matches!(ability, StaticAbilityAst::Static(static_ability) if static_ability.id()
                == crate::static_abilities::StaticAbilityId::Grants)
        }));
    }

    #[test]
    fn shuffle_into_library_replacement_is_reachable_from_if_head() {
        let tokens = crate::lexer::lex_line(
            "If this creature would be put into a graveyard from anywhere, reveal it and shuffle it into its owner's library instead.",
            0,
        )
        .expect("shuffle replacement line should lex");
        let parsed = registry_result(&tokens)
            .expect("shuffle replacement registry parse should not error")
            .expect("shuffle replacement registry should claim the line");

        assert!(parsed.iter().any(|ability| {
            matches!(
                ability,
                StaticAbilityAst::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
            )
        }));
    }
}

macro_rules! single_static_ability_ast_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: RuleId::new(stringify!($parse)),
            parse: |tokens| {
                static_single_rule_outcome(
                    RuleId::new(stringify!($parse)),
                    tokens,
                    $parse(tokens).map(|ability| ability.map(StaticAbilityAst::from)),
                )
            },
        }
    };
}

macro_rules! single_static_ability_ast_infallible_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: RuleId::new(stringify!($parse)),
            parse: |tokens| {
                let span = crate::util::span_from_tokens(tokens);
                match $parse(tokens).map(StaticAbilityAst::from) {
                    Some(ability) => ParseOutcome::matched(vec![ability], span),
                    None => ParseOutcome::NoMatch,
                }
            },
        }
    };
}

macro_rules! multi_static_ability_ast_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: RuleId::new(stringify!($parse)),
            parse: |tokens| {
                static_multi_rule_outcome(
                    RuleId::new(stringify!($parse)),
                    tokens,
                    $parse(tokens).map(|abilities| {
                        abilities.map(|abilities| {
                            abilities
                                .into_iter()
                                .map(StaticAbilityAst::from)
                                .collect::<Vec<_>>()
                        })
                    }),
                )
            },
        }
    };
}

macro_rules! single_static_ability_ast_passthrough_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: RuleId::new(stringify!($parse)),
            parse: |tokens| {
                static_single_rule_outcome(RuleId::new(stringify!($parse)), tokens, $parse(tokens))
            },
        }
    };
}

macro_rules! multi_static_ability_ast_passthrough_rule {
    ($parse:ident) => {
        StaticAbilityLineRuleDef {
            id: RuleId::new(stringify!($parse)),
            parse: |tokens| {
                static_multi_rule_outcome(RuleId::new(stringify!($parse)), tokens, $parse(tokens))
            },
        }
    };
}

fn static_ability_ast_line_rules() -> &'static [StaticAbilityLineRuleDef] {
    static RULES: &[StaticAbilityLineRuleDef] = &[
        multi_static_ability_ast_passthrough_rule!(parse_combat_requirement_static_line),
        single_static_ability_ast_rule!(parse_conditional_attack_requirement_line),
        single_static_ability_ast_passthrough_rule!(parse_source_owned_flying_block_limit_line),
        single_static_ability_ast_passthrough_rule!(parse_enchant_attachment_restriction_line),
        multi_static_ability_ast_passthrough_rule!(parse_soulbond_shared_line),
        single_static_ability_ast_rule!(parse_ward_static_ability_line),
        single_static_ability_ast_rule!(parse_skulk_rules_text_line),
        single_static_ability_ast_rule!(
            parse_filter_dont_untap_during_controllers_untap_steps_line
        ),
        single_static_ability_ast_rule!(parse_damage_doubling_mana_value_marker_line),
        single_static_ability_ast_rule!(parse_conditional_source_spell_keyword_line),
        single_static_ability_ast_rule!(parse_affinity_cost_reduction_line),
        single_static_ability_ast_rule!(parse_pregame_begin_on_battlefield_line),
        single_static_ability_ast_rule!(parse_pregame_mulligan_redraw_line),
        multi_static_ability_ast_rule!(parse_combined_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_pregame_choose_color_line),
        single_static_ability_ast_rule!(parse_activated_abilities_cost_increase_line),
        single_static_ability_ast_rule!(parse_choose_basic_land_type_as_enters_line),
        single_static_ability_ast_rule!(
            parse_revealed_hand_choose_nonland_card_name_as_enters_line
        ),
        single_static_ability_ast_rule!(parse_choose_card_name_as_enters_line),
        multi_static_ability_ast_rule!(parse_choose_color_and_creature_type_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_color_creature_type_pairs_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_creature_type_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_named_options_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_player_as_enters_line),
        single_static_ability_ast_rule!(parse_note_life_total_as_enters_line),
        single_static_ability_ast_rule!(parse_discard_hand_as_enters_line),
        single_static_ability_ast_rule!(parse_choose_color_as_becomes_attached_line),
        single_static_ability_ast_rule!(parse_enchanted_land_is_chosen_type_line),
        single_static_ability_ast_rule!(parse_source_land_is_chosen_type_line),
        single_static_ability_ast_rule!(parse_source_is_chosen_type_in_addition_line),
        single_static_ability_ast_rule!(parse_source_is_chosen_color_line),
        single_static_ability_ast_rule!(parse_double_token_creation_replacement_line),
        single_static_ability_ast_passthrough_rule!(parse_token_creation_templates_line),
        single_static_ability_ast_rule!(parse_double_counters_replacement_line),
        single_static_ability_ast_passthrough_rule!(parse_lose_game_replacement_line),
        single_static_ability_ast_rule!(parse_keyword_action_replacement_line),
        single_static_ability_ast_infallible_rule!(parse_static_text_marker_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_choose_color_line),
        single_static_ability_ast_rule!(parse_damage_not_removed_cleanup_line),
        single_static_ability_ast_passthrough_rule!(
            parse_prevent_damage_to_source_remove_counter_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_prevent_damage_to_source_put_counters_line
        ),
        single_static_ability_ast_rule!(parse_prevent_damage_to_you_from_source_filter_line),
        single_static_ability_ast_rule!(parse_filtered_damage_prevention_line),
        single_static_ability_ast_rule!(parse_permanent_self_damage_prevention_line),
        single_static_ability_ast_rule!(parse_persistent_filtered_damage_prevention_line),
        single_static_ability_ast_rule!(parse_shared_color_pair_damage_prevention_line),
        single_static_ability_ast_rule!(parse_first_spell_flash_permission_line),
        single_static_ability_ast_rule!(parse_prevention_amount_follow_up_line),
        single_static_ability_ast_rule!(parse_prevention_proposed_amount_follow_up_line),
        single_static_ability_ast_rule!(parse_damage_prevention_with_owner_shuffle_line),
        single_static_ability_ast_rule!(parse_replace_damage_with_counters_instead_line),
        single_static_ability_ast_rule!(parse_choose_color_as_enters_line),
        single_static_ability_ast_rule!(parse_damage_redirect_to_source_controller_line),
        single_static_ability_ast_rule!(parse_damage_redirect_to_source_line),
        single_static_ability_ast_rule!(parse_scoped_damage_redirection_line),
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
        single_static_ability_ast_rule!(
            parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line
        ),
        single_static_ability_ast_rule!(parse_conditional_draw_replacement_line),
        single_static_ability_ast_rule!(parse_draw_extra_cards_replacement_line),
        single_static_ability_ast_rule!(parse_if_opponent_would_draw_redirect_line),
        single_static_ability_ast_rule!(parse_if_you_would_draw_instead_effects_line),
        single_static_ability_ast_rule!(parse_activate_abilities_as_though_haste_line),
        single_static_ability_ast_rule!(parse_loyalty_abilities_any_time_line),
        single_static_ability_ast_rule!(parse_zero_loyalty_state_based_exception_line),
        single_static_ability_ast_rule!(parse_draw_replacement_double_line),
        single_static_ability_ast_rule!(parse_draw_replacement_skip_empty_library_line),
        single_static_ability_ast_rule!(parse_exile_to_exile_instead_of_graveyard_line),
        single_static_ability_ast_rule!(parse_exile_to_countered_exile_instead_of_graveyard_line),
        single_static_ability_ast_rule!(parse_exile_would_die_instead_line),
        single_static_ability_ast_rule!(parse_redirect_would_enter_line),
        single_static_ability_ast_rule!(parse_mana_output_rewrite_static_line),
        single_static_ability_ast_rule!(parse_if_source_tapped_for_mana_replacement_line),
        single_static_ability_ast_rule!(parse_if_you_tap_for_mana_multiplier_line),
        single_static_ability_ast_rule!(parse_if_you_would_gain_life_replacement_line),
        single_static_ability_ast_rule!(parse_if_event_would_happen_instead_line),
        single_static_ability_ast_rule!(parse_if_event_would_happen_amount_line),
        single_static_ability_ast_rule!(parse_you_may_look_at_additional_cards_each_time_line),
        single_static_ability_ast_rule!(parse_damage_life_floor_static_line),
        single_static_ability_ast_rule!(parse_if_player_would_change_life_double_line),
        single_static_ability_ast_rule!(parse_discard_or_redirect_replacement_line),
        single_static_ability_ast_rule!(parse_sacrifice_or_redirect_replacement_line),
        multi_static_ability_ast_rule!(parse_choose_basic_land_type_then_pay_life_line),
        single_static_ability_ast_rule!(parse_pay_life_or_enter_tapped_line),
        single_static_ability_ast_rule!(parse_reveal_card_or_enter_tapped_line),
        single_static_ability_ast_passthrough_rule!(parse_copy_activated_abilities_line),
        single_static_ability_ast_passthrough_rule!(parse_spend_mana_as_any_color_line),
        single_static_ability_ast_passthrough_rule!(parse_enchanted_has_activated_ability_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_has_base_power_toughness_and_granted_keywords_static_line
        ),
        // The complete animation bundle owns a leading P/T descriptor such
        // as "is a 4/4 Elemental creature in addition to its other types".
        // Route it before the broad type/color-addition parser can claim the
        // tail and reject the numeric descriptor as an unsupported type.
        multi_static_ability_ast_passthrough_rule!(
            parse_filter_is_pt_creature_in_addition_and_has_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_filter_is_pt_creature_in_addition_line),
        multi_static_ability_ast_passthrough_rule!(parse_conditional_copular_creature_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_has_base_power_toughness_and_type_color_addition_static_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_has_base_power_and_granted_ability_static_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_attached_restriction_and_granted_ability_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_subject_color_and_granted_ability_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_anthem_color_and_quoted_activated_grant_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_controlled_creatures_may_assign_as_unblocked_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_anthem_and_no_defender_line),
        multi_static_ability_ast_passthrough_rule!(parse_base_pt_and_blocker_restriction_line),
        multi_static_ability_ast_passthrough_rule!(parse_conditional_no_defender_and_unblockable_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_subject_is_subtype_with_base_pt_and_granted_abilities_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_anthem_and_keyword_line),
        // The comparison tail contains the words `have defender`; let the
        // exact conditional attack-permission grammar own it before the broad
        // granted-ability parser can reinterpret those words as a defender
        // grant to the condition's spell filter.
        single_static_ability_ast_passthrough_rule!(
            parse_as_long_as_condition_can_attack_as_though_no_defender_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_filtered_toughness_assignment_line),
        multi_static_ability_ast_passthrough_rule!(parse_filter_has_granted_ability_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_equipped_gets_and_has_activated_ability_line
        ),
        single_static_ability_ast_rule!(parse_shuffle_into_library_from_graveyard_line),
        single_static_ability_ast_rule!(parse_permanents_enter_tapped_line),
        single_static_ability_ast_rule!(
            parse_creatures_entering_dont_cause_abilities_to_trigger_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_trigger_suppression_line_ast),
        single_static_ability_ast_rule!(parse_creatures_assign_combat_damage_using_toughness_line),
        single_static_ability_ast_rule!(
            parse_you_assign_combat_damage_of_creatures_attacking_you_line
        ),
        single_static_ability_ast_rule!(
            parse_lethal_damage_to_creatures_you_control_uses_power_line
        ),
        single_static_ability_ast_rule!(parse_players_cant_cycle_line),
        single_static_ability_ast_rule!(parse_starting_life_bonus_line),
        single_static_ability_ast_rule!(parse_buyback_cost_reduction_line),
        single_static_ability_ast_passthrough_rule!(parse_can_be_attached_only_to_line),
        single_static_ability_ast_rule!(parse_spell_additional_life_cost_per_target_line),
        single_static_ability_ast_rule!(parse_spell_cost_increase_per_target_beyond_first_line),
        single_static_ability_ast_rule!(parse_equip_cost_modifier_line),
        single_static_ability_ast_rule!(parse_flashback_cost_modifier_line),
        multi_static_ability_ast_rule!(parse_spell_and_player_activated_ability_cost_modifier_line),
        multi_static_ability_ast_rule!(parse_spells_cost_reduction_and_cant_be_countered_line),
        multi_static_ability_ast_rule!(parse_double_conditional_this_spell_cost_reduction_line),
        single_static_ability_ast_rule!(parse_spells_cost_modifier_line),
        single_static_ability_ast_passthrough_rule!(parse_trigger_duplication_line_ast),
        single_static_ability_ast_rule!(
            parse_double_damage_from_sources_you_control_of_chosen_type_line
        ),
        single_static_ability_ast_rule!(parse_double_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_prevent_half_damage_replacement_line),
        single_static_ability_ast_rule!(parse_minimum_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_damage_amount_replacement_line),
        single_static_ability_ast_rule!(parse_foretelling_cards_cost_modifier_line),
        single_static_ability_ast_rule!(parse_players_skip_extra_turns_line),
        single_static_ability_ast_rule!(parse_players_skip_upkeep_line),
        single_static_ability_ast_rule!(parse_skip_untap_steps_line),
        single_static_ability_ast_rule!(parse_players_cast_spells_only_during_own_turns_line),
        single_static_ability_ast_rule!(parse_skip_your_draw_step_static_line),
        single_static_ability_ast_rule!(parse_legend_rule_doesnt_apply_line),
        multi_static_ability_ast_rule!(parse_source_counter_threshold_keyword_and_subtype_line),
        multi_static_ability_ast_rule!(
            parse_subject_are_card_types_in_addition_to_their_other_types_line
        ),
        single_static_ability_ast_rule!(parse_subject_is_card_types_line),
        single_static_ability_ast_rule!(parse_subject_is_also_subtypes_line),
        single_static_ability_ast_rule!(parse_subject_is_chosen_color_line),
        single_static_ability_ast_rule!(parse_all_permanents_colorless_line),
        single_static_ability_ast_rule!(parse_all_cards_spells_permanents_colorless_line),
        multi_static_ability_ast_rule!(parse_all_are_pt_color_type_addition_line),
        multi_static_ability_ast_rule!(parse_all_are_color_and_type_addition_line),
        single_static_ability_ast_rule!(parse_all_cards_spells_permanents_add_chosen_color_line),
        single_static_ability_ast_rule!(parse_all_creatures_are_color_line),
        single_static_ability_ast_rule!(parse_subjects_are_basic_line),
        single_static_ability_ast_rule!(parse_protection_from_colored_spells_line),
        single_static_ability_ast_rule!(parse_nonbasic_lands_are_basic_land_type_line),
        single_static_ability_ast_rule!(parse_land_type_addition_line),
        multi_static_ability_ast_rule!(parse_lands_are_pt_creatures_still_lands_line),
        single_static_ability_ast_rule!(parse_remove_snow_line),
        single_static_ability_ast_rule!(parse_supertype_assertion_line),
        multi_static_ability_ast_rule!(parse_attached_is_legendary_gets_and_has_keywords_line),
        single_static_ability_ast_rule!(parse_landwalk_as_though_block_override_line),
        multi_static_ability_ast_passthrough_rule!(parse_granted_keyword_static_line),
        multi_static_ability_ast_rule!(parse_equipment_you_control_have_equip_line),
        multi_static_ability_ast_passthrough_rule!(parse_attached_gets_and_cant_block_line),
        multi_static_ability_ast_rule!(parse_lose_all_abilities_and_transform_base_pt_line),
        multi_static_ability_ast_rule!(parse_lose_all_abilities_and_base_pt_line),
        // The attached grant/loss and generic subject-loss productions are
        // structurally disjoint; both remain registered for their own heads.
        multi_static_ability_ast_rule!(parse_attached_has_and_loses_keywords_line),
        multi_static_ability_ast_passthrough_rule!(parse_subject_loses_keywords_line),
        single_static_ability_ast_passthrough_rule!(parse_all_creatures_lose_flying_line),
        single_static_ability_ast_passthrough_rule!(
            parse_each_creature_cant_be_blocked_by_more_than_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_filtered_blocker_count_restriction_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_each_creature_can_block_additional_creature_each_combat_line
        ),
        multi_static_ability_ast_rule!(parse_anthem_and_type_color_addition_line),
        single_static_ability_ast_rule!(parse_matching_are_goaded_line),
        multi_static_ability_ast_rule!(parse_anthem_and_goaded_line),
        multi_static_ability_ast_passthrough_rule!(parse_anthem_and_granted_ability_line),
        multi_static_ability_ast_passthrough_rule!(
            parse_subject_has_keywords_and_cant_be_blocked_by_more_than_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_subject_has_keywords_and_cant_be_blocked_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_subject_cant_be_blocked_and_has_keywords_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_subject_is_every_subtype_family_line),
        single_static_ability_ast_passthrough_rule!(parse_all_have_indestructible_line),
        single_static_ability_ast_passthrough_rule!(
            parse_subject_cant_be_blocked_while_defending_player_controls_most_creatures_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_subject_cant_be_blocked_as_long_as_condition_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_subject_cant_be_blocked_line),
        single_static_ability_ast_rule!(parse_may_choose_not_to_untap_during_untap_step_line),
        single_static_ability_ast_rule!(parse_untap_during_each_other_players_untap_step_line),
        single_static_ability_ast_rule!(parse_untap_step_limit_line),
        single_static_ability_ast_rule!(parse_search_limited_to_top_cards_line),
        single_static_ability_ast_rule!(parse_prevent_all_damage_to_you_line),
        single_static_ability_ast_rule!(parse_opponents_must_target_flagbearers_line),
        single_static_ability_ast_passthrough_rule!(parse_doesnt_untap_during_untap_step_line),
        multi_static_ability_ast_rule!(parse_attached_restrictions_with_ignore_special_action_line),
        multi_static_ability_ast_rule!(parse_attached_is_goaded_line),
        multi_static_ability_ast_rule!(parse_attached_has_keywords_and_is_goaded_line),
        multi_static_ability_ast_rule!(parse_attached_has_keywords_and_negated_restriction_line),
        multi_static_ability_ast_rule!(parse_equipped_creature_has_line),
        multi_static_ability_ast_rule!(parse_enchanted_creature_has_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_tap_abilities_cant_be_activated_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_attached_land_ability_reset_line),
        multi_static_ability_ast_rule!(parse_attached_type_transform_line),
        single_static_ability_ast_rule!(parse_you_control_attached_creature_line),
        single_static_ability_ast_passthrough_rule!(parse_attached_cant_attack_or_block_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_can_attack_as_though_no_defender_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_can_attack_as_though_haste_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attacked_player_can_attack_as_though_no_defender_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_targeting_as_though_no_ability_line),
        single_static_ability_ast_passthrough_rule!(
            parse_plain_can_attack_as_though_no_defender_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_source_can_block_shadow_as_though_no_shadow_line
        ),
        single_static_ability_ast_passthrough_rule!(parse_can_block_as_though_untapped_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_to_and_by_attached_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_by_attached_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_combat_damage_dealt_by_attached_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_prevent_all_damage_dealt_to_attached_line
        ),
        multi_static_ability_ast_passthrough_rule!(
            parse_attached_has_keywords_and_triggered_ability_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_attached_gets_and_has_ability_line),
        multi_static_ability_ast_passthrough_rule!(parse_anthem_with_trailing_segments_line),
        multi_static_ability_ast_passthrough_rule!(parse_gets_and_attacks_each_combat_if_able_line),
        single_static_ability_ast_passthrough_rule!(
            parse_conditional_all_creatures_able_to_block_line
        ),
        single_static_ability_ast_passthrough_rule!(
            parse_source_can_attack_as_though_no_defender_as_long_as_line
        ),
        multi_static_ability_ast_passthrough_rule!(parse_self_combat_requirement_line),
        single_static_ability_ast_passthrough_rule!(parse_attacks_each_combat_if_able_line),
        single_static_ability_ast_rule!(parse_source_must_be_blocked_if_able_line),
        multi_static_ability_ast_passthrough_rule!(parse_composed_anthem_effects_line),
        single_static_ability_ast_rule!(parse_enter_as_copy_as_enters_line),
        single_static_ability_ast_rule!(parse_has_base_power_toughness_static_line),
        single_static_ability_ast_rule!(parse_has_base_power_toughness_each_equal_static_line),
        single_static_ability_ast_rule!(parse_isnt_creature_line),
        single_static_ability_ast_passthrough_rule!(parse_as_long_as_source_is_a_land_line),
        single_static_ability_ast_passthrough_rule!(
            parse_during_your_turn_graveyard_cards_have_retrace_line
        ),
        multi_static_ability_ast_rule!(parse_multi_subject_anthem_line),
        multi_static_ability_ast_rule!(parse_play_from_top_pay_life_line),
        single_static_ability_ast_rule!(parse_anthem_line),
        single_static_ability_ast_rule!(parse_flying_restriction_line),
        single_static_ability_ast_rule!(parse_can_block_only_flying_line),
        single_static_ability_ast_rule!(parse_can_block_subtype_as_though_reach_line),
        single_static_ability_ast_rule!(parse_assign_damage_as_unblocked_line),
        single_static_ability_ast_rule!(parse_independent_alternative_price_line),
        single_static_ability_ast_rule!(parse_fixed_mana_cost_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_mana_value_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_life_mana_value_instead_of_mana_cost_grant_line),
        single_static_ability_ast_rule!(parse_as_enters_or_turns_face_up_pt_choice_line),
        single_static_ability_ast_rule!(parse_as_enters_becomes_characteristics_for_filter_line),
        multi_static_ability_ast_rule!(
            parse_you_may_cast_exile_counter_cards_with_mana_permission_line
        ),
        multi_static_ability_ast_rule!(parse_player_may_cast_spells_free_and_flash_line),
        multi_static_ability_ast_rule!(parse_you_have_protection_from_opponents_line),
        multi_static_ability_ast_rule!(parse_surveilled_graveyard_play_life_cost_line),
        multi_static_ability_ast_rule!(parse_source_exiled_play_life_cost_line),
        single_static_ability_ast_rule!(parse_as_you_cascade_land_drop_line),
        single_static_ability_ast_rule!(parse_play_from_permission_with_haste_this_way_line),
        single_static_ability_ast_rule!(
            parse_play_from_permission_with_enter_counter_this_way_line
        ),
        single_static_ability_ast_rule!(parse_play_from_permission_with_enter_tapped_this_way_line),
        multi_static_ability_ast_rule!(parse_you_may_static_grant_line),
        single_static_ability_ast_rule!(parse_grant_flash_to_noncreature_spells_line),
        single_static_ability_ast_rule!(parse_conditional_self_flash_line),
        single_static_ability_ast_rule!(parse_cast_this_spell_as_though_it_had_flash_line),
        single_static_ability_ast_rule!(parse_during_your_turn_prevent_all_damage_to_source_line),
        single_static_ability_ast_rule!(parse_prevent_all_combat_damage_to_source_line),
        single_static_ability_ast_rule!(parse_prevent_all_damage_to_matching_permanents_line),
        single_static_ability_ast_rule!(
            parse_prevent_all_combat_damage_to_matching_permanents_line
        ),
        single_static_ability_ast_rule!(
            parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line
        ),
        single_static_ability_ast_rule!(
            parse_prevent_all_noncombat_damage_to_matching_permanents_line
        ),
        single_static_ability_ast_rule!(parse_prevent_all_damage_to_source_by_creatures_line),
        single_static_ability_ast_rule!(
            parse_prevent_damage_to_other_creature_you_control_put_counters_line
        ),
        single_static_ability_ast_rule!(parse_prevent_all_damage_dealt_to_creatures_line),
        single_static_ability_ast_passthrough_rule!(parse_creatures_cant_block_line),
        single_static_ability_ast_rule!(parse_enters_under_chosen_control_line),
        multi_static_ability_ast_rule!(parse_enters_tapped_with_counters_line),
        single_static_ability_ast_rule!(parse_enters_with_additional_counter_for_filter_line),
        multi_static_ability_ast_rule!(parse_enters_with_counters_line),
        multi_static_ability_ast_rule!(parse_enters_or_escapes_instead_counters_line),
        single_static_ability_ast_rule!(parse_as_enters_reveal_from_hand_line),
        single_static_ability_ast_rule!(parse_reveal_from_hand_or_enters_tapped_line),
        single_static_ability_ast_rule!(parse_conditional_enters_tapped_unless_line),
        single_static_ability_ast_rule!(parse_enters_untapped_for_filter_line),
        single_static_ability_ast_rule!(parse_enters_tapped_for_filter_line),
        single_static_ability_ast_rule!(parse_x_at_most_enters_tapped_line),
        single_static_ability_ast_rule!(parse_enters_prepared_line),
        single_static_ability_ast_rule!(parse_enters_tapped_line),
        multi_static_ability_ast_rule!(parse_additional_land_play_line),
        single_static_ability_ast_rule!(parse_devour_quality_line),
        single_static_ability_ast_rule!(parse_granted_encore_line),
        single_static_ability_ast_rule!(parse_commander_ninjutsu_line),
        single_static_ability_ast_rule!(parse_each_player_additional_land_play_line),
        single_static_ability_ast_rule!(parse_you_may_look_top_card_any_time_line),
        single_static_ability_ast_rule!(
            parse_you_may_look_face_down_creatures_you_dont_control_any_time_line
        ),
        single_static_ability_ast_rule!(parse_players_play_top_card_libraries_revealed_line),
        single_static_ability_ast_rule!(parse_play_top_card_your_library_revealed_line),
        single_static_ability_ast_rule!(parse_your_opponents_play_with_hands_revealed_line),
        single_static_ability_ast_rule!(parse_self_or_global_hands_revealed_line),
        single_static_ability_ast_rule!(parse_control_opponents_while_searching_libraries_line),
        single_static_ability_ast_rule!(parse_opponent_search_exile_found_cards_line),
        single_static_ability_ast_rule!(parse_cast_this_card_from_library_while_searching_line),
        single_static_ability_ast_rule!(parse_play_lands_from_graveyard_line),
        single_static_ability_ast_rule!(parse_graveyard_cards_have_retrace_line),
        single_static_ability_ast_rule!(parse_cast_spells_from_hand_without_paying_mana_costs_line),
        single_static_ability_ast_rule!(parse_cost_reduction_line),
        multi_static_ability_ast_rule!(parse_blocking_capacity_static_line),
        single_static_ability_ast_rule!(parse_can_block_additional_creature_each_combat_line),
        single_static_ability_ast_passthrough_rule!(parse_all_creatures_able_to_block_source_line),
        single_static_ability_ast_rule!(parse_filtered_creatures_able_to_block_source_line),
        single_static_ability_ast_passthrough_rule!(
            parse_attached_all_creatures_able_to_block_line
        ),
        single_static_ability_ast_rule!(parse_activated_abilities_cant_be_activated_line),
        multi_static_ability_ast_rule!(parse_cant_clauses),
        single_static_ability_ast_rule!(parse_granted_casting_keyword_line),
        single_static_ability_ast_rule!(parse_granted_spell_keyword_line),
        single_static_ability_ast_rule!(parse_cast_from_zone_using_keyword_abilities_line),
        single_static_ability_ast_rule!(parse_granted_madness_line),
        multi_static_ability_ast_rule!(parse_domain_landwalk_line),
        single_static_ability_ast_rule!(parse_base_toughness_only_line),
        multi_static_ability_ast_passthrough_rule!(parse_absorb_keyword_line),
        multi_static_ability_ast_passthrough_rule!(parse_leading_condition_wrapped_static_line),
    ];
    RULES
}

#[derive(Debug)]
struct StaticAbilityLineRuleIndex {
    lexical: LexRuleHintIndex,
    whole_line: Vec<usize>,
}

impl StaticAbilityLineRuleIndex {
    fn candidate_indices(&self, head: &str, second: Option<&str>) -> Vec<usize> {
        let mut indices = self.lexical.candidate_indices(head, second);
        indices.extend(self.whole_line.iter().copied());
        indices.sort_unstable();
        indices.dedup();
        indices
    }
}

static STATIC_ABILITY_AST_LINE_RULE_INDEX: LazyLock<StaticAbilityLineRuleIndex> =
    LazyLock::new(|| {
        let rules = static_ability_ast_line_rules();
        let lexical = build_lex_rule_hint_index(rules.len(), |idx| {
            match static_ability_rule_head_discriminator(rules[idx].id) {
                StaticAbilityLineHeadDiscriminator::Lexical(hints) => hints,
                StaticAbilityLineHeadDiscriminator::WholeLine => Vec::new(),
            }
        });
        let whole_line = rules
            .iter()
            .enumerate()
            .filter_map(|(idx, rule)| {
                matches!(
                    static_ability_rule_head_discriminator(rule.id),
                    StaticAbilityLineHeadDiscriminator::WholeLine
                )
                .then_some(idx)
            })
            .collect();
        StaticAbilityLineRuleIndex {
            lexical,
            whole_line,
        }
    });

fn parse_static_ability_ast_line_lowered(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    match recognize_static_ability_ast_line_registry(tokens) {
        ParseOutcome::NoMatch => Ok(None),
        ParseOutcome::Match(matched) => Ok(Some(matched.value)),
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
    }
}

fn recognize_static_ability_ast_line_registry(
    tokens: &[OwnedLexToken],
) -> ParseOutcome<Vec<StaticAbilityAst>> {
    let rules = static_ability_ast_line_rules();
    let words = parser_token_word_refs(tokens);
    let head = words.first().copied().unwrap_or("");
    let second = words.get(1).copied();
    let candidate_indices = STATIC_ABILITY_AST_LINE_RULE_INDEX.candidate_indices(head, second);
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();

    for idx in candidate_indices {
        let rule = &rules[idx];
        let (result, loss) = crate::parse_loss::capture(|| {
            run_static_ability_ast_line_rule(rule.id, rule.parse, tokens)
        });
        match result {
            ParseOutcome::Match(matched) => {
                let candidate = RegistryCandidate::new(
                    RegistryRuleMetadata::distinct(
                        rule.id,
                        HeadDiscriminator::grammar(rule.id.as_str()),
                    ),
                    (matched.value, loss),
                    matched.span,
                );
                // Multiple grammars may prove the same typed static fact.
                // Exact AST and loss-report equality is an explicit semantic
                // equivalence policy; different results remain ambiguous.
                if !candidates
                    .iter()
                    .any(|existing: &RegistryCandidate<_>| existing.value == candidate.value)
                {
                    candidates.push(candidate);
                }
            }
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    match resolve_registry_candidates(
        RuleId::new("static-ability-line-registry"),
        candidates,
        diagnostics,
    ) {
        ParseOutcome::Match(matched) => {
            let rule_match = matched.value;
            let (abilities, loss) = rule_match.value;
            replay_static_rule_parse_loss(&loss);
            if std::env::var("IRONSMITH_STATIC_RULE_TRACE").is_ok() {
                eprintln!("static-rule claim: {}", rule_match.rule);
            }
            ParseOutcome::matched(abilities, rule_match.span)
        }
        ParseOutcome::NoMatch => ParseOutcome::NoMatch,
        ParseOutcome::Error(diagnostic) => ParseOutcome::Error(diagnostic),
    }
}

fn title_case_count_as_card_name(words: &[&str]) -> String {
    words
        .iter()
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                let mut out = first.to_ascii_uppercase().to_string();
                out.push_str(chars.as_str());
                out
            } else {
                String::new()
            }
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[rustfmt::skip]
fn parse_count_as_card_named_for_spell_effect_line(tokens: &[OwnedLexToken])
    -> Option<StaticAbility>
{
    let words = LexedClause::new(tokens).word_refs();
    let shape = early_static_facts::parse_count_as_card_named_shape_words(&words)?;
    let spell_name_words = words.get(shape.spell_name_words)?;
    let counted_name_words = words.get(shape.counted_name_words)?;

    Some(StaticAbility::count_as_card_named_for_spell_effect(
        title_case_count_as_card_name(spell_name_words),
        title_case_count_as_card_name(counted_name_words),
    ))
}

fn parse_source_characteristics_of_last_exiled_creature_card_line(
    tokens: &[OwnedLexToken],
) -> Option<StaticAbility> {
    let words = parser_token_word_refs(tokens);
    let base = [
        "as",
        "long",
        "as",
        "a",
        "card",
        "exiled",
        "with",
        "this",
        "creature",
        "is",
        "a",
        "creature",
        "card",
        "this",
        "creature",
        "has",
        "the",
        "power",
        "toughness",
        "and",
        "creature",
        "types",
        "of",
        "the",
        "last",
        "creature",
        "card",
        "exiled",
        "with",
        "it",
    ];
    if words.len() < base.len() || words[..base.len()] != base {
        return None;
    }

    let retained_subtypes = match words.get(base.len()..) {
        Some([]) => Vec::new(),
        Some(["it's", "still", "a", subtype]) | Some(["its", "still", "a", subtype]) => {
            vec![parse_subtype_flexible(subtype)?]
        }
        _ => return None,
    };

    let mut filter = ObjectFilter::default();
    filter.card_types.push(CardType::Creature);
    filter.nontoken = true;
    filter.zone = Some(Zone::Exile);
    Some(
        StaticAbility::source_characteristics_of_last_exiled_creature_card(
            filter,
            retained_subtypes,
        ),
    )
}

mod early_line_readings;
pub(crate) use early_line_readings::read_aggregate_x_maximum;

fn parse_static_ability_ast_line_early_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let input = early_line_readings::EarlyLine {
        tokens,
        read_by_cache: Default::default(),
    };
    match early_line_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }

    Ok(None)
}

pub fn parse_damage_doubling_mana_value_marker_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if early_static_facts::parse_damage_doubling_mana_value_marker_tokens(tokens).is_none() {
        return Ok(None);
    }

    // Prefer the functional damage-multiplier replacement; only surfaces the
    // rule can't lower (e.g. "to a target, double that damage instead") keep
    // the marker fallback.
    if let Ok(Some(ability)) = parse_double_damage_amount_replacement_line(tokens) {
        return Ok(Some(ability));
    }

    Ok(Some(keyword_static_marker(tokens)))
}

#[inline(never)]
fn parse_complete_attached_restriction_quoted_activation(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = attached_grammar::parse_attached_combat_restriction_grant_tokens(tokens)
    else {
        return Ok(None);
    };
    let Some(parsed) = parse_attached_granted_activated_line(shape.ability_tokens)? else {
        return Ok(None);
    };
    let subject_display = shape.subject.display();
    let (restriction, restriction_display) = match shape.kind {
        attached_grammar::AttachedCombatRestrictionKind::CantAttack => (
            crate::effect::Restriction::attack(ObjectFilter::source()),
            format!("{subject_display} can't attack"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantBlock => (
            crate::effect::Restriction::block(ObjectFilter::source()),
            format!("{subject_display} can't block"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantAttackOrBlock => (
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            format!("{subject_display} can't attack or block"),
        ),
        attached_grammar::AttachedCombatRestrictionKind::CantBeBlocked => return Ok(None),
    };
    Ok(Some(vec![
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                restriction,
                restriction_display.clone(),
            ))),
            display: restriction_display,
            condition: None,
        },
        StaticAbilityAst::AttachedObjectAbilityGrant {
            ability: parsed,
            display: format!(
                "{subject_display} has {}",
                display_text_for_tokens(shape.ability_tokens, true)
            ),
            condition: None,
        },
    ]))
}

pub fn parse_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    // The static line grammar is also used as a speculative probe: line
    // recognizers, trigger-body readers and the document dispatcher all ask
    // "is this a static ability?" before trying other families. Its direct
    // (non-registry) readers may record recovery diagnostics while reading a
    // subject and then decline the body. Like the registry candidates, losses
    // belong only to a committed reading: replay them when this call returns
    // static abilities, and drop them when it declines or errors, so an
    // abandoned static probe cannot taint the trigger/activated/effect
    // reading that actually owns the line.
    let (result, loss) =
        crate::parse_loss::capture(|| parse_static_ability_ast_line_lexed_committed(tokens));
    if matches!(result, Ok(Some(_))) {
        crate::parse_loss::replay(loss.diagnostics());
    }
    result
}

fn parse_static_ability_ast_line_lexed_committed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    crate::clause_support::validate_protection_static_line(tokens)?;
    if let Some(abilities) =
        enters_tapped_untap_conjunction::parse_enters_tapped_and_doesnt_untap_line(tokens)?
    {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_complete_attached_restriction_quoted_activation(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_attached_subtype_and_predicate_static_line(tokens)? {
        return Ok(Some(abilities));
    }
    parse_static_ability_ast_line_lexed_unstacked(tokens)
}

/// "Enchanted creature is a Spirit and can't attack or block." (Fog on the
/// Barrow-Downs): a creature-type setting and a second predicate of the same
/// attached subject are two independent static abilities.
fn parse_attached_subtype_and_predicate_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_any_word(&["enchanted", "equipped"]))
    {
        return Ok(None);
    }
    let Some(is_idx) = tokens.iter().position(|token| token.is_word("is")) else {
        return Ok(None);
    };
    let article = tokens.get(is_idx + 1);
    let and = tokens.get(is_idx + 3);
    // The verb follows the bare attached subject ("enchanted creature is"),
    // and the set word is a type, not the first of a color pair ("is a green
    // and white Citizen creature").
    if is_idx != 2
        || tokens.get(is_idx + 2).is_some_and(|token| {
            token.is_any_word(&["white", "blue", "black", "red", "green", "colorless"])
        })
        || !article.is_some_and(|token| token.is_any_word(&["a", "an"]))
        || !and.is_some_and(|token| token.is_word("and"))
        || tokens.len() <= is_idx + 4
    {
        return Ok(None);
    }
    let mut type_line = tokens[..is_idx + 3].to_vec();
    if let Some(period) = tokens.last().filter(|token| token.is_period()) {
        type_line.push(period.clone());
    }
    let mut predicate_line = tokens[..is_idx].to_vec();
    predicate_line.extend_from_slice(&tokens[is_idx + 4..]);
    let Ok(Some(mut abilities)) = parse_static_ability_ast_line_lexed_unstacked(&type_line) else {
        return Ok(None);
    };
    let Ok(Some(predicate)) = parse_static_ability_ast_line_lexed_unstacked(&predicate_line) else {
        return Ok(None);
    };
    abilities.extend(predicate);
    Ok(Some(abilities))
}

/// "Nonlegendary creatures enchanted player controls have base power and
/// toughness 3/3 and lose all creature types." (Curse of Conformity): two
/// continuous statics over the same subject, a layer-7b base P/T and a
/// layer-4 creature-subtype set to none.
fn parse_base_pt_and_lose_all_creature_types_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let view = TokenWordView::new(&tokens);
    let words = view.word_refs();
    let suffix_len = 5usize;
    if words.len() <= suffix_len
        || !matches!(
            &words[words.len() - suffix_len..],
            ["and", "lose" | "loses", "all", "creature", "types"]
        )
    {
        return Ok(None);
    }
    let and_word = words.len() - suffix_len;
    let Some(and_token) = view.map_word_to_token_start(and_word) else {
        return Ok(None);
    };
    let prefix = &tokens[..and_token];
    let Some(shape) = anthem_grant_grammar::parse_base_power_toughness_shape(prefix) else {
        return Ok(None);
    };
    if !matches!(
        shape.condition,
        anthem_grant_grammar::BasePowerToughnessConditionShape::None
    ) {
        return Ok(None);
    }
    let subject = parse_anthem_subject_with_attached_fallback(shape.subject_tokens, None)?;
    let filter = anthem_subject_filter(&subject);
    Ok(Some(vec![
        StaticAbility::set_base_power_toughness(filter.clone(), shape.power, shape.toughness)
            .into(),
        StaticAbility::set_creature_subtypes(filter, Vec::new()).into(),
    ]))
}

/// "All creatures able to block this creature or enchanted creature do so."
/// (Noble Quarry, bestow): one lure on the creature itself and one granted to
/// the creature it enchants. Both are statics; only one applies at a time.
fn parse_bestow_lure_source_or_enchanted_line(
    tokens: &[OwnedLexToken],
) -> Option<Vec<StaticAbilityAst>> {
    let words = parser_token_word_refs(tokens);
    let matches_shape = matches!(
        words.as_slice(),
        [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "creature" | "permanent",
            "or",
            "enchanted",
            "creature",
            "do",
            "so"
        ]
    );
    if !matches_shape {
        return None;
    }
    let source_display = "All creatures able to block this creature do so".to_string();
    let enchanted_display = "All creatures able to block enchanted creature do so".to_string();
    Some(vec![
        StaticAbilityAst::Static(StaticAbility::restriction(
            crate::effect::Restriction::must_block_specific_attacker(
                ObjectFilter::creature(),
                ObjectFilter::source(),
            ),
            source_display,
        )),
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                crate::effect::Restriction::must_block_specific_attacker(
                    ObjectFilter::creature(),
                    ObjectFilter::source(),
                ),
                enchanted_display.clone(),
            ))),
            display: enchanted_display,
            condition: None,
        },
    ])
}

/// "Tadeas has hexproof unless it's attacking.": the source's own static
/// grant, conditioned on the negated status. A permanent's characteristic
/// line, never a one-shot `unless` control flow.
fn parse_source_has_unless_status_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(unless) = tokens.iter().position(|token| token.is_word("unless")) else {
        return Ok(None);
    };
    let head = &tokens[..unless];
    let tail = &tokens[unless + 1..];
    if head.iter().any(|token| token.kind == TokenKind::Quote) {
        return Ok(None);
    }
    let head_words = parser_token_word_refs(head);
    if head_words.first() != Some(&"this")
        || !head_words
            .iter()
            .any(|word| matches!(*word, "has" | "have"))
    {
        return Ok(None);
    }
    let tail_words = parser_token_word_refs(tail);
    let Some((state, subject)) = tail_words.split_last() else {
        return Ok(None);
    };
    if !matches!(*state, "attacking" | "blocking" | "tapped" | "untapped")
        || !matches!(subject, ["it's"] | ["its"] | ["it", "is"] | ["it", "s"])
    {
        return Ok(None);
    }
    let Some(abilities) = parse_static_ability_ast_line_lexed_unstacked(head)? else {
        return Ok(None);
    };
    if abilities.is_empty() {
        return Ok(None);
    }
    let condition = parse_static_condition_clause(tail)?;
    Ok(Some(
        abilities
            .into_iter()
            .map(|ability| StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition: PredicateAst::Not(Box::new(condition.clone())),
            })
            .collect(),
    ))
}

mod compound_line_readings;
fn parse_static_ability_ast_line_lexed_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    // Direct lexer callers still carry parenthetical reminder text.  It is not
    // a second rules sentence, and parsing it independently can turn an
    // explanatory comma into an unsupported effect clause.  Preserve
    // parentheses inside a quoted granted ability, but discard a trailing
    // top-level reminder before static-line dispatch.
    let mut inside_quote = false;
    let mut paren_depth = 0usize;
    let mut open_start = None;
    let mut trailing_parenthetical = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quote = !inside_quote;
            continue;
        }
        if inside_quote {
            continue;
        }
        match token.kind {
            TokenKind::LParen => {
                if paren_depth == 0 {
                    open_start = Some(index);
                }
                paren_depth = paren_depth.saturating_add(1);
            }
            TokenKind::RParen if paren_depth > 0 => {
                paren_depth -= 1;
                if paren_depth == 0
                    && let Some(start) = open_start.take()
                    && tokens[index + 1..].iter().all(|trailing| {
                        matches!(
                            trailing.kind,
                            TokenKind::Period | TokenKind::Bang | TokenKind::Question
                        )
                    })
                {
                    trailing_parenthetical = Some(start);
                }
            }
            _ => {}
        }
    }
    if let Some(reminder_start) = trailing_parenthetical {
        let visible = trim_edge_punctuation(&tokens[..reminder_start]);
        if !visible.is_empty() {
            return parse_static_ability_ast_line_lexed_unstacked(&visible);
        }
    }
    // A named (non-keyword) ability label before a leading static condition
    // ("Chef's Knife — During your turn, this creature has ...") is flavor:
    // left in place it hides the condition prefix from every condition-aware
    // reading, so the condition is dropped and the subject misread.
    if let Some((label_tokens, body_tokens)) = split_em_dash_label_prefix_tokens(tokens)
        && !label_tokens.is_empty()
        && !body_tokens.is_empty()
        && super::grammar::document_shapes::parse_label_prefix_kind_tokens(label_tokens).is_none()
        && label_tokens.iter().all(|token| token.as_word().is_some())
        && {
            let body_words = parser_token_word_refs(body_tokens);
            matches!(body_words.first(), Some(&"during"))
                || matches!(body_words.get(..3), Some(["as", "long", "as"]))
        }
    {
        return parse_static_ability_ast_line_lexed_unstacked(body_tokens);
    }
    if let Some(abilities) = parse_filtered_toughness_assignment_line(tokens)? {
        return Ok(Some(abilities));
    }
    // Keep optional-size/additive artifact-creature semantics and attached
    // pronouns ahead of the older generic card-type identity wrapper.
    if let Some(abilities) = parse_conditional_copular_creature_line(tokens)? {
        return Ok(Some(abilities));
    }
    // Bound the condition before reading the characteristic-setting subject.
    // Otherwise a permissive subject suffix can absorb the condition words.
    if let Some(spec) = split_as_long_as_condition_prefix_lexed(tokens)
        && let Some(ability) = parse_subject_is_card_types_line(spec.remainder_tokens)?
    {
        let condition = parse_static_condition_clause(spec.condition_tokens)?;
        return Ok(Some(vec![StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition,
        }]));
    }
    if let Some(abilities) = parse_source_has_unless_status_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_bestow_lure_source_or_enchanted_line(tokens) {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_base_pt_and_lose_all_creature_types_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_conditional_source_characteristics_and_predicate_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_carried_attached_subject_line(tokens)? {
        return Ok(Some(abilities));
    }
    let words = crate::lexer::token_word_refs(tokens);
    if words.contains(&"cast")
        && words.contains(&"instead")
        && let Some(abilities) = parse_static_ability_ast_line_lexed_single(tokens)?
    {
        return Ok(Some(abilities));
    }
    // Independent static sentences must be read before a permissive anthem
    // tail can absorb the next sentence into its keyword or subject filter.
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() > 1 {
        let (parsed, loss) = crate::parse_loss::capture(|| {
            let mut combined = Vec::new();
            for sentence in &sentences {
                let Some(mut abilities) = parse_static_ability_ast_line_lexed_single(sentence)?
                else {
                    return Ok::<_, CardTextError>(None);
                };
                combined.append(&mut abilities);
            }
            Ok(Some(combined))
        });
        if !loss.is_lossy()
            && let Ok(Some(abilities)) = parsed
        {
            return Ok(Some(abilities));
        }
    }
    if !tokens.iter().any(|token| token.is_quote())
        || tokens
            .iter()
            .take_while(|token| !token.is_quote())
            .any(|token| token.is_any_word(&["get", "gets"]))
    {
        let (complete_tail, tail_loss) =
            crate::parse_loss::capture(|| parse_anthem_with_trailing_segments_line(tokens));
        if let Ok(Some(abilities)) = complete_tail {
            replay_static_rule_parse_loss(&tail_loss);
            return Ok(Some(abilities));
        }
    }
    let input = compound_line_readings::StaticLine { tokens };
    match compound_line_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    // The declines the ladder made before its fallback still gate the fallback.
    if compound_line_readings::declines(&input) {
        return Ok(None);
    }
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

pub fn parse_removed_draft_leading_conditional_static_sentence_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() < 2 {
        return Ok(None);
    }

    let mut combined = Vec::new();
    for sentence in sentences {
        let mut terminated = sentence.to_vec();
        terminated.push(OwnedLexToken::period(TextSpan::synthetic()));
        let Some(spec) = crate::grammar::static_line_support::parse_leading_if_clause(&terminated)
        else {
            return Ok(None);
        };
        let Ok(condition) = parse_static_condition_clause(spec.condition_tokens) else {
            return Ok(None);
        };
        if !matches!(
            condition,
            PredicateAst::Player(PlayerPredicateAst::PlayerRemovedDraftCardMatching { .. })
        ) {
            return Ok(None);
        }
        let Some(abilities) = parse_static_ability_ast_line_lexed_single_without_leading_condition(
            spec.remainder_tokens,
        )?
        else {
            return Ok(None);
        };
        if abilities.is_empty() {
            return Ok(None);
        }
        for ability in abilities {
            combined.push(add_static_ability_ast_condition(
                ability,
                condition.clone(),
            )?);
        }
    }

    Ok((!combined.is_empty()).then_some(combined))
}

fn parse_players_cant_search_with_any_player_ignore_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [restriction, permission] = sentences.as_slice() else {
        return Ok(None);
    };
    if !crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(restriction),
        &["players", "cant", "search", "libraries"],
    ) {
        return Ok(None);
    }

    let permission = trim_edge_punctuation(permission);
    if permission.len() < 5
        || !permission[0].is_word("any")
        || !permission[1].is_word("player")
        || !permission[2].is_word("may")
        || !permission[3].is_word("pay")
    {
        return Ok(None);
    }
    let Some(cost) = parse_leaf_fixed_mana_cost_prefix_tokens(&permission[4..]) else {
        return Ok(None);
    };
    let tail = trim_edge_punctuation(&permission[4 + cost.consumed..]);
    if !crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(&tail),
        &[
            "for", "that", "player", "to", "ignore", "this", "effect", "until", "end", "of", "turn",
        ],
    ) {
        return Ok(None);
    }

    let display = render_token_slice(&permission);
    Ok(Some(vec![
        StaticAbilityAst::Static(StaticAbility::players_cant_search()),
        StaticAbilityAst::Static(
            StaticAbility::any_player_may_pay_mana_to_ignore_source_effect_until_end_of_turn(
                cost.cost, display,
            ),
        ),
    ]))
}

mod single_line_readings;

/// "Players can cast spells only during their own turns." (Dosan the Falling
/// Leaf): a player who isn't the active player can't cast spells (CR 101.2,
/// 505.6 timing restriction on every player).
fn parse_players_cast_spells_only_during_own_turns_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::grammar::primitives;
    let clean = trim_edge_punctuation(tokens);
    let Some(((), rest)) = primitives::parse_prefix(
        &clean,
        primitives::phrase(&[
            "players", "can", "cast", "spells", "only", "during", "their", "own", "turns",
        ]),
    ) else {
        return Ok(None);
    };
    if !rest.is_empty() {
        return Ok(None);
    }
    Ok(Some(StaticAbility::restriction(
        crate::effect::Restriction::cast_spells(PlayerFilter::Excluding {
            base: Box::new(PlayerFilter::Any),
            excluded: Box::new(PlayerFilter::Active),
        }),
        "Players can cast spells only during their own turns".to_string(),
    )))
}

/// "Players skip their untap steps." (Stasis), "Each player skips their untap
/// step.", "Skip your untap step." (CR 502, 614.10).
fn parse_skip_untap_steps_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    use crate::grammar::primitives;
    let clean = trim_edge_punctuation(tokens);
    let forms: [(&'static [&'static str], PlayerFilter); 3] = [
        (&["players", "skip", "their", "untap", "steps"], PlayerFilter::Any),
        (&["each", "player", "skips", "their", "untap", "step"], PlayerFilter::Any),
        (&["skip", "your", "untap", "step"], PlayerFilter::You),
    ];
    for (phrase, player) in forms {
        if let Some(((), rest)) = primitives::parse_prefix(&clean, primitives::phrase(phrase))
            && rest.is_empty()
        {
            return Ok(Some(StaticAbility::players_skip_untap_steps(player)));
        }
    }
    Ok(None)
}

/// "You may cast this card from your graveyard if <condition>." /
/// "... as long as <condition>." The trailing condition gates the same
/// graveyard cast permission that the leading "As long as <condition>, you may
/// cast this card from your graveyard." form already lowers to (a conditional
/// grant functioning from the graveyard, checked when the card is cast,
/// CR 601.3). Both the permission and the condition are read by their shared
/// grammars; an unreadable condition declines the line.
fn parse_source_graveyard_cast_trailing_condition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    use crate::grammar::primitives;
    const PERMISSION: &[&str] = &["you", "may", "cast", "this", "card", "from", "your", "graveyard"];
    let clean = trim_edge_punctuation(tokens);
    let Some(((), after_permission)) = primitives::parse_prefix(&clean, primitives::phrase(PERMISSION))
    else {
        return Ok(None);
    };
    let condition_tokens = if let Some(((), rest)) =
        primitives::parse_prefix(after_permission, primitives::phrase(&["as", "long", "as"]))
    {
        rest
    } else if let Some((_, rest)) = primitives::parse_prefix(after_permission, primitives::kw("if"))
    {
        rest
    } else {
        return Ok(None);
    };
    // An optional second sentence is a cast-this-way rider ("If you do, this
    // creature enters with a +1/+1 counter on it."), read by the shared
    // permission-with-entry-counter grammar.
    let (condition_tokens, rider_tokens) =
        match condition_tokens.iter().position(|token| token.is_period()) {
            Some(period) => (&condition_tokens[..period], Some(&condition_tokens[period..])),
            None => (condition_tokens, None),
        };
    let condition_tokens = trim_edge_punctuation(condition_tokens);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let Ok(condition) = parse_static_condition_clause(&condition_tokens) else {
        return Ok(None);
    };
    let permission_len = clean.len() - after_permission.len();
    let abilities = match rider_tokens {
        None => parse_static_ability_ast_line_lexed_single_without_leading_condition(
            &clean[..permission_len],
        )?,
        Some(rider_tokens) => {
            let mut composed = clean[..permission_len].to_vec();
            composed.extend_from_slice(rider_tokens);
            parse_play_from_permission_with_enter_counter_this_way_line(&composed)?
                .map(|ability| vec![StaticAbilityAst::from(ability)])
        }
    };
    let Some(abilities) = abilities else {
        return Ok(None);
    };
    if abilities.is_empty() {
        return Ok(None);
    }
    let mut conditioned = Vec::with_capacity(abilities.len());
    for ability in abilities {
        conditioned.push(add_static_ability_ast_condition(ability, condition.clone())?);
    }
    Ok(Some(conditioned))
}

fn parse_static_ability_ast_line_lexed_single(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if let Some(ability) = dynamic_characteristic_statics::parse_timed_source_pt(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(ability) = dynamic_characteristic_statics::parse_bound_base_pt(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }
    if let Some(split) = split_as_long_as_condition_prefix_lexed(tokens)
        && let Some(ability) = parse_characteristic_defining_pt_line(split.remainder_tokens)?
    {
        let condition = parse_static_condition_clause(split.condition_tokens)?;
        return Ok(Some(vec![StaticAbilityAst::Static(
            ability.with_condition(condition),
        )]));
    }
    if let Some(abilities) = parse_source_graveyard_cast_trailing_condition_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) =
        crate::permission_helpers::parse_independent_recent_graveyard_permissions(tokens)
    {
        return Ok(Some(abilities));
    }
    // A complete replacement owns both its conditional prefix and its
    // repeated damage recipient; neither is a resolving damage instruction.
    if let Some(ability) = parse_double_damage_amount_replacement_line(tokens)? {
        return Ok(Some(vec![StaticAbilityAst::Static(ability)]));
    }
    if let Some(abilities) = parse_filtered_toughness_assignment_line(tokens)? {
        return Ok(Some(abilities));
    }
    let input = single_line_readings::SingleStaticLine {
        tokens,
        read_by_cache: Default::default(),
    };
    match single_line_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    let condition_prefix_tokens =
        crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(tokens)
            .filter(|shape| {
                shape.kind
                == crate::grammar::effects::labeled_dispatch::LeadingEffectLabelKind::Conditional
            })
            .map_or(tokens, |shape| shape.body_tokens);
    let existing = parse_static_ability_ast_line_lexed_single_without_leading_condition(tokens)?;
    let Some(spec) = split_as_long_as_condition_prefix_lexed(condition_prefix_tokens) else {
        return Ok(existing);
    };

    // Existing narrow parsers already bind many leading conditions correctly.
    // Keep those typed shapes (and their surface hints) instead of wrapping them
    // a second time. The generic path is for lines whose original parse leaves
    // at least one emitted sibling unconditional or consumes condition words as
    // part of the affected subject.
    if existing.as_ref().is_some_and(|abilities| {
        !abilities.is_empty()
            && abilities
                .iter()
                .all(static_ability_ast_has_explicit_condition)
    }) {
        return Ok(existing);
    }

    // A pronoun here gets its semantic subject from the condition clause (for
    // example, "enchanted creature is black, it gets ..."). Those lines need
    // the attached-subject parsers; stripping the prefix would turn `it` into
    // the source and silently retarget the continuous effect.
    if leading_condition_remainder_has_dependent_subject(spec.remainder_tokens) {
        return Ok(existing);
    }

    let Ok(condition) = parse_static_condition_clause(spec.condition_tokens) else {
        return Ok(existing);
    };
    // Source-zone conditions also determine where the ability functions. Their
    // specialized parsers intentionally lower that information into functional
    // zones rather than a battlefield conditional wrapper.
    if static_condition_references_source_outside_battlefield(&condition) {
        return Ok(existing);
    }

    let Some(abilities) = parse_static_ability_ast_line_lexed_single_without_leading_condition(
        spec.remainder_tokens,
    )?
    else {
        return Ok(existing);
    };
    if abilities.is_empty() {
        return Ok(existing);
    }

    if cast_this_spell_as_though_flash_tokens(spec.remainder_tokens) && abilities.len() == 1 {
        let ability = abilities.into_iter().next().expect("length checked");
        return Ok(Some(vec![
            StaticAbilityAst::LabeledConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
                label: render_token_slice(&trim_edge_punctuation(tokens)),
            },
        ]));
    }

    let mut conditioned = Vec::with_capacity(abilities.len());
    for ability in abilities {
        let Ok(ability) = add_static_ability_ast_condition(ability, condition.clone()) else {
            return Ok(existing);
        };
        conditioned.push(ability);
    }
    Ok(Some(conditioned))
}

/// "As long as ~ is tapped, it's a Human Citizen with base power and
/// toughness 1/1 and can't be blocked" (Futurist Operative): a
/// characteristic-setting clause and a second predicate sharing the pronoun
/// subject under one condition. Read them as the two sentences they
/// abbreviate ("As long as ..., it's a Human Citizen with base power and
/// toughness 1/1. As long as ..., it can't be blocked."); read whole, the
/// copular clause is misread as the subject of the restriction.
fn parse_conditional_source_characteristics_and_predicate_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(spec) = split_as_long_as_condition_prefix_lexed(tokens) else {
        return Ok(None);
    };
    let remainder = trim_edge_punctuation(spec.remainder_tokens);
    let starts_copular = match remainder.as_slice() {
        [first, ..] if first.is_word("it's") || first.is_word("it’s") || first.is_word("its") => {
            true
        }
        [first, second, ..] => first.is_word("it") && second.is_word("is"),
        _ => false,
    };
    if !starts_copular {
        return Ok(None);
    }
    const PREDICATE_HEADS: &[&str] = &[
        "can't",
        "can’t",
        "cant",
        "can",
        "has",
        "have",
        "gets",
        "attacks",
        "blocks",
        "doesn't",
        "doesn’t",
        "doesnt",
    ];
    let Some(and_index) = remainder.iter().enumerate().rposition(|(index, token)| {
        token.is_word("and")
            && remainder
                .get(index + 1)
                .is_some_and(|next| PREDICATE_HEADS.iter().any(|head| next.is_word(head)))
    }) else {
        return Ok(None);
    };
    if and_index < 3 || remainder.iter().any(|token| token.kind == TokenKind::Quote) {
        return Ok(None);
    }
    let Some(comma_index) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
    else {
        return Ok(None);
    };
    let prefix = &tokens[..=comma_index];
    let mut characteristics = prefix.to_vec();
    characteristics.extend_from_slice(&remainder[..and_index]);
    characteristics.push(OwnedLexToken::period(TextSpan::synthetic()));
    let mut predicate = prefix.to_vec();
    // "it" in a static line names the source; say so explicitly so the
    // restriction parser binds the source instead of an unbound pronoun.
    predicate.push(OwnedLexToken::word("this", TextSpan::synthetic()));
    predicate.push(OwnedLexToken::word("permanent", TextSpan::synthetic()));
    predicate.extend_from_slice(&remainder[and_index + 1..]);
    predicate.push(OwnedLexToken::period(TextSpan::synthetic()));
    let Ok(Some(mut first)) = parse_static_ability_ast_line_lexed_unstacked(&characteristics)
    else {
        return Ok(None);
    };
    let Ok(Some(mut second)) = parse_static_ability_ast_line_lexed_unstacked(&predicate) else {
        return Ok(None);
    };
    if first.is_empty() || second.is_empty() {
        return Ok(None);
    }
    first.append(&mut second);
    Ok(Some(first))
}

fn cast_this_spell_as_though_flash_tokens(tokens: &[OwnedLexToken]) -> bool {
    crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(&trim_edge_punctuation(tokens)),
        &[
            "you", "may", "cast", "this", "spell", "as", "though", "it", "had", "flash",
        ],
    )
}

#[cfg(test)]
mod conditional_flash_permission_tests {
    use super::*;

    fn parse_line(text: &str) -> Vec<StaticAbilityAst> {
        let tokens = crate::lexer::lex_line(text, 0).expect("line should lex");
        parse_static_ability_ast_line_lexed(&tokens)
            .expect("line should parse")
            .expect("line should be claimed")
    }

    #[test]
    fn target_dependent_flash_retains_a_structured_object_filter() {
        let parsed = parse_line(
            "You may cast this spell as though it had flash if it targets a permanent you control.",
        );
        let [StaticAbilityAst::Static(ability)] = parsed.as_slice() else {
            panic!("{parsed:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::FlashIfTargetsMatching(filter) = &ability.payload
        else {
            panic!("{ability:#?}");
        };
        assert_eq!(filter.zone, Some(crate::zone::Zone::Battlefield));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn cast_as_though_flash_preserves_authored_surface_for_supported_conditions() {
        for text in [
            "As long as you control a green or blue permanent, you may cast this spell as though it had flash.",
            "If there are three or more Lesson cards in your graveyard, you may cast this spell as though it had flash.",
            "If you control a creature with power 4 or greater, you may cast this spell as though it had flash.",
        ] {
            let parsed = parse_line(text);
            assert_eq!(parsed.len(), 1, "{text}: {parsed:#?}");
            let StaticAbilityAst::LabeledConditionalStaticAbility {
                ability,
                condition: _,
                label,
            } = &parsed[0]
            else {
                panic!("{text} did not retain a labeled typed condition: {parsed:#?}");
            };
            assert_eq!(label, text.trim_end_matches('.'));
            assert!(format!("{ability:#?}").contains("Flash"), "{ability:#?}");
        }
    }

    #[test]
    fn similar_granted_flash_wording_does_not_claim_self_spell_permission_surface() {
        let parsed = parse_line(
            "As long as you control a green permanent, creatures you control have flash.",
        );
        assert!(
            !matches!(
                parsed.as_slice(),
                [StaticAbilityAst::LabeledConditionalStaticAbility { label, .. }]
                    if label.contains("cast this spell as though")
            ),
            "a battlefield grant must not be rewritten as a self-spell cast permission: {parsed:#?}"
        );
    }
}

#[cfg(test)]
mod conditioned_source_block_requirement_tests {
    use super::*;

    #[test]
    fn removed_from_draft_named_source_grant_keeps_its_leading_condition() {
        let parsed = (|| {
                let tokens = crate::lexer::lex_line(
                    "If you removed a creature card with flying from the draft with cards named Animus of Predation, this creature has flying.",
                    0,
                )
                .expect("removed-from-draft grant should lex");
                parse_static_ability_ast_line_lexed(&tokens)
            })()
        .expect("removed-from-draft grant should parse")
        .expect("removed-from-draft grant should be claimed");
        let debug = format!("{parsed:#?}");

        assert_eq!(parsed.len(), 1, "{debug}");
        assert!(debug.contains("ConditionalStaticAbility"), "{debug}");
        assert!(debug.contains("PlayerRemovedDraftCardMatching"), "{debug}");
        assert!(debug.contains("Animus of Predation"), "{debug}");
    }

    #[test]
    fn expanded_removed_draft_chain_preempts_broad_compound_grants() {
        let parsed = (|| {
                let tokens = crate::lexer::lex_line(
                    "If you removed a creature card with flying from the draft with cards named Draft Avatar, this creature has flying. If you removed a creature card with haste from the draft with cards named Draft Avatar, this creature has haste.",
                    0,
                )
                .expect("expanded removed-from-draft chain should lex");
                parse_static_ability_ast_line_lexed(&tokens)
            })()
        .expect("expanded removed-from-draft chain should parse")
        .expect("expanded removed-from-draft chain should be claimed");
        let debug = format!("{parsed:#?}");

        assert_eq!(parsed.len(), 2, "{debug}");
        assert_eq!(debug.matches("PlayerRemovedDraftCardMatching").count(), 2);
        assert!(
            debug.contains("Flying") && debug.contains("Haste"),
            "{debug}"
        );
    }

    #[test]
    fn removed_draft_chain_preemption_rejects_a_mixed_condition_family() {
        let tokens = crate::lexer::lex_line(
            "If you removed a creature card with flying from the draft with cards named Draft Avatar, this creature has flying. If you control an artifact, this creature has haste.",
            0,
        )
        .expect("mixed conditional chain should lex");

        assert!(
            parse_removed_draft_leading_conditional_static_sentence_chain(&tokens)
                .expect("mixed conditional chain should not error")
                .is_none(),
            "a mixed condition family must remain available to ordinary routing"
        );
    }

    #[test]
    fn quoted_animation_bundle_wins_before_the_broad_quoted_grant_route() {
        let tokens = crate::lexer::lex_line(
            "During your turn, each non-Equipment artifact and non-Aura enchantment you control \
             with mana value 4 or greater is a 4/4 Elemental creature in addition to its other \
             types and has indestructible, haste, and \"Whenever this creature deals combat \
             damage to a player, draw a card.\"",
            0,
        )
        .expect("compound animation should lex");

        let parsed = parse_static_ability_ast_line_lexed(&tokens)
            .expect("compound animation router should not hard-error")
            .expect("compound animation router should claim Bello's line");

        assert_eq!(parsed.len(), 6, "{parsed:#?}");
        assert!(
            parsed
                .iter()
                .any(|ability| matches!(ability, StaticAbilityAst::GrantObjectAbility { .. })),
            "{parsed:#?}"
        );
    }

    #[test]
    fn soulbond_shared_quoted_trigger_wins_before_generic_conditional_grant() {
        let tokens = crate::lexer::lex_line(
            "As long as this creature is paired with another creature, each of those creatures has \"When this creature dies, draw cards equal to its power.\"",
            0,
        )
        .expect("soulbond shared trigger should lex");
        let parsed = parse_static_ability_ast_line_lexed(&tokens)
            .expect("soulbond shared trigger should parse")
            .expect("soulbond shared trigger should be claimed");

        assert!(
            matches!(
                parsed.as_slice(),
                [StaticAbilityAst::SoulbondSharedObjectAbility { .. }]
            ),
            "the specialist must preserve the paired two-creature scope: {parsed:#?}"
        );
    }

    #[test]
    fn named_source_soulbond_shared_trigger_keeps_paired_two_creature_scope() {
        let parsed = (|| {
                let tokens = crate::lexer::lex_line(
                    "As long as Doom Weaver is paired with another creature, each of those creatures has \"When this creature dies, draw cards equal to its power.\"",
                    0,
                )
                .expect("named-source soulbond shared trigger should lex");
                parse_static_ability_ast_line_lexed(&tokens)
            })()
        .expect("named-source soulbond shared trigger should parse")
        .expect("named-source soulbond shared trigger should be claimed");

        assert!(
            matches!(
                parsed.as_slice(),
                [StaticAbilityAst::SoulbondSharedObjectAbility { .. }]
            ),
            "the specialist must preserve the named paired two-creature scope: {parsed:#?}"
        );
    }

    #[test]
    fn equipped_named_source_must_be_blocked_is_a_conditioned_static_rule() {
        let tokens = crate::lexer::lex_line(
            "As long as Probe is equipped, it must be blocked if able.",
            0,
        )
        .expect("conditioned block requirement should lex");
        let parsed = parse_static_ability_ast_line_lexed(&tokens)
            .expect("conditioned block requirement should parse")
            .expect("conditioned block requirement should be claimed");
        let debug = format!("{parsed:#?}");
        assert!(debug.contains("ConditionalStaticAbility"), "{debug}");
        assert!(debug.contains("SourceIsEquipped"), "{debug}");
        assert!(debug.contains("MustBeBlocked"), "{debug}");
    }
}

fn parse_static_ability_ast_line_lexed_single_without_leading_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if looks_like_trigger_intro_tokens(tokens) || looks_like_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    // Soulbond's paired-creature scope is a complete typed mechanic shape.
    // Claim it before the broad conditioned-anthem parser can reinterpret
    // "each of those creatures" as an ordinary tagged-object grant.
    if let Some(abilities) = parse_soulbond_shared_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_counters_remain_across_zone_changes_line(tokens) {
        return Ok(Some(vec![StaticAbilityAst::Static(ability)]));
    }
    if let Some(ability) = parse_double_counters_replacement_line(tokens)? {
        return Ok(Some(vec![StaticAbilityAst::Static(ability)]));
    }
    if looks_like_player_counter_gain_effect_tokens(tokens) {
        return Ok(None);
    }

    let (early_result, early_loss) =
        crate::parse_loss::capture(|| parse_static_ability_ast_line_early_lexed(tokens));
    if let Some(abilities) = early_result? {
        replay_static_rule_parse_loss(&early_loss);
        return Ok(Some(abilities));
    }

    parse_static_ability_ast_line_lowered(tokens)
}

fn parse_counters_remain_across_zone_changes_line(
    tokens: &[OwnedLexToken],
) -> Option<StaticAbility> {
    let visible = trim_edge_punctuation_tokens(tokens);
    let words = parser_token_word_refs(visible);
    if words.len() < 12
        || !crate::word_primitives::parse_sequence_prefix(&words, &["counters", "remain", "on"])
    {
        return None;
    }

    const MOVE_TO_OTHER_ZONE: &[&str] =
        &["as", "it", "moves", "to", "any", "zone", "other", "than"];
    let marker = crate::word_primitives::parse_sequence_start(&words[3..], MOVE_TO_OTHER_ZONE)? + 3;
    let source_words = &words[3..marker];
    let looks_like_bare_name = !source_words.is_empty()
        && !source_words.iter().any(|word| {
            matches!(
                *word,
                "a" | "an"
                    | "another"
                    | "any"
                    | "artifact"
                    | "card"
                    | "creature"
                    | "each"
                    | "object"
                    | "permanent"
                    | "target"
            )
        });
    if marker == 3 || (!is_source_reference_words(source_words) && !looks_like_bare_name) {
        return None;
    }

    let mut excluded_destinations = Vec::new();
    let mut idx = marker + MOVE_TO_OTHER_ZONE.len();
    while idx < words.len() {
        if matches!(words[idx], "a" | "an" | "the" | "players" | "and" | "or") {
            idx += 1;
            continue;
        }

        let (zone, consumed) =
            if crate::word_primitives::parse_sequence_prefix(&words[idx..], &["command", "zone"]) {
                (Zone::Command, 2)
            } else if crate::word_primitives::parse_sequence_prefix(
                &words[idx..],
                &["outside", "the", "game"],
            ) {
                (Zone::OutsideGame, 3)
            } else {
                (parse_zone_word(words[idx])?, 1)
            };
        let previous_destination_count = excluded_destinations.len();
        crate::slice_primitives::push_unique(&mut excluded_destinations, zone);
        if excluded_destinations.len() == previous_destination_count {
            return None;
        }
        idx += consumed;
    }
    if excluded_destinations.is_empty() {
        return None;
    }

    Some(StaticAbility::counters_remain_across_zone_changes(
        excluded_destinations,
        render_token_slice(visible),
    ))
}

#[cfg(test)]
mod counters_remain_across_zone_changes_tests {
    use super::*;

    #[test]
    fn parses_named_source_and_typed_excluded_destinations() {
        let parsed = (|| {
            let tokens = crate::lexer::lex_line(
                "Counters remain on Test Relic as it moves to any zone other than a player's hand or library.",
                0,
            )
            .expect("counter-retention line should lex");
            parse_static_ability_ast_line_lexed(&tokens)
        })()
        .expect("counter-retention line should parse")
        .expect("counter-retention line should be claimed");

        let [StaticAbilityAst::Static(ability)] = parsed.as_slice() else {
            panic!("expected one typed static ability: {parsed:#?}");
        };
        assert!(matches!(
            &ability.payload,
            ironsmith_core::StaticAbilityPayload::CountersRemainAcrossZoneChanges {
                excluded_destinations,
                ..
            } if excluded_destinations == &[Zone::Hand, Zone::Library]
        ));
    }

    #[test]
    fn rejects_a_different_subject_or_incomplete_destination_list() {
        for text in [
            "Counters remain on another permanent as it moves to any zone other than a player's hand or library.",
            "Counters remain on Test Relic as it moves to any zone other than.",
        ] {
            let parsed = (|| {
                let tokens = crate::lexer::lex_line(text, 0).expect("near-miss should lex");
                parse_counters_remain_across_zone_changes_line(&tokens)
            })();
            assert!(parsed.is_none(), "near-miss was claimed: {text}");
        }
    }
}

fn leading_condition_remainder_has_dependent_subject(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    matches!(
        words.first().copied(),
        Some(
            "it" | "its"
                | "it's"
                | "they"
                | "them"
                | "their"
                | "that"
                | "those"
                | "these"
                | "he"
                | "him"
                | "his"
                | "she"
                | "her"
                | "both"
        )
    ) || (matches!(
        words.first(),
        Some(&"each" | &"all" | &"one" | &"any" | &"some")
    ) && matches!(words.get(1), Some(&"of"))
        && matches!(words.get(2), Some(&"them" | &"those" | &"these")))
}

fn static_condition_references_source_outside_battlefield(condition: &PredicateAst) -> bool {
    match condition {
        PredicateAst::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            ..
        } => {
            filter.source
                && filter
                    .zone
                    .as_ref()
                    .is_some_and(|zone| *zone != Zone::Battlefield)
        }
        PredicateAst::Source(SourcePredicateAst::SourceIsInZone(zone)) => {
            *zone != Zone::Battlefield
        }
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            static_condition_references_source_outside_battlefield(left)
                || static_condition_references_source_outside_battlefield(right)
        }
        PredicateAst::Not(inner) => static_condition_references_source_outside_battlefield(inner),
        _ => false,
    }
}

fn static_ability_ast_has_explicit_condition(ability: &StaticAbilityAst) -> bool {
    match ability {
        StaticAbilityAst::ConditionalStaticAbility { .. }
        | StaticAbilityAst::LabeledConditionalStaticAbility { .. }
        | StaticAbilityAst::ConditionalKeywordAction { .. } => true,
        StaticAbilityAst::WithSetQuantifierSurface { ability, .. } => {
            static_ability_ast_has_explicit_condition(ability)
        }
        StaticAbilityAst::GrantStaticAbility { condition, .. }
        | StaticAbilityAst::GrantKeywordAction { condition, .. }
        | StaticAbilityAst::AttachedStaticAbilityGrant { condition, .. }
        | StaticAbilityAst::AttachedKeywordActionGrant { condition, .. }
        | StaticAbilityAst::AttachedChosenLandwalkGrant { condition, .. }
        | StaticAbilityAst::GrantObjectAbility { condition, .. }
        | StaticAbilityAst::AttachedObjectAbilityGrant { condition, .. } => condition.is_some(),
        StaticAbilityAst::Static(ability) => match &ability.payload {
            ironsmith_core::StaticAbilityPayload::Conditional { .. } => true,
            ironsmith_core::StaticAbilityPayload::Anthem(anthem) => anthem.condition.is_some(),
            ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
                grant.condition.is_some()
            }
            _ => false,
        },
        _ => false,
    }
}

fn looks_like_player_counter_gain_effect_tokens(tokens: &[OwnedLexToken]) -> bool {
    // A typed conditional anthem can mention player counters in its condition
    // before a later permanent subject "gets" a bonus. Do not let the broad
    // player-resource effect guard steal that already-recognized static shape.
    if anthem_grant_grammar::parse_anthem_keyword_head(tokens).is_some() {
        return false;
    }
    let Some(head) = static_keyword_line_shapes::parse_player_counter_gain_head(tokens) else {
        return false;
    };
    if !head.has_counter_resource {
        return false;
    }

    let tail = tokens.get(head.get.token + 1..).unwrap_or_default();
    if parse_value(tail).is_some() {
        return true;
    }

    tail.iter()
        .find_map(OwnedLexToken::as_word)
        .is_some_and(|word| {
            word == "another" || crate::grammar::leaf::parse_number_complete(word).is_ok()
        })
}

pub fn parse_activated_abilities_cant_be_activated_line(
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

    // "Activated abilities of artifacts and creatures ..." should be a union of types.
    // Our general object filter parser treats type lists joined by "and" as intersection,
    // which is correct for many adjective chains, but incorrect for this rules pattern.
    let subject_words = crate::util::non_article_token_word_refs(subject_tokens);

    let filter = if let Some(filter) = activated_ability_subject_special_filter(subject_tokens) {
        filter
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

pub fn parse_activated_abilities_cost_increase_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = static_keyword_cost_shapes::parse_activated_ability_cost_increase(tokens)
    else {
        return Ok(None);
    };
    let clause_display = render_token_slice(tokens);

    let subject_tokens = trim_lexed_commas(spec.subject_tokens);
    let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability cost increase subject (clause: '{}')",
            clause_display
        ))
    })?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }

    let additional_cost_tokens = trim_outer_quotes(trim_lexed_commas(spec.additional_cost_tokens));
    if additional_cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing activated-ability additional cost (clause: '{}')",
            clause_display
        )));
    }
    let total_cost = parse_compiler_activation_cost(additional_cost_tokens)?;
    if total_cost.is_free() {
        return Err(CardTextError::ParseError(format!(
            "unsupported activated-ability additional cost (clause: '{}')",
            clause_display
        )));
    }

    Ok(Some(if spec.non_mana_only {
        StaticAbility::increase_non_mana_activated_ability_costs(filter, total_cost)
    } else {
        StaticAbility::increase_activated_ability_costs(filter, total_cost)
    }))
}

pub fn parse_activated_abilities_cant_be_activated_line_lexed(
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

    let subject_words = crate::util::non_article_token_word_refs(subject_tokens);

    let filter = if let Some(filter) = activated_ability_subject_special_filter(subject_tokens) {
        filter
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

pub fn parse_pregame_begin_on_battlefield_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_display = render_token_slice(tokens);
    let Some(spec) = keyword_static_lines::parse_pregame_begin_on_battlefield_tokens(tokens)?
    else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::BeginOnBattlefield(
            crate::static_abilities::PregameBeginOnBattlefieldSpec {
                require_not_starting_player: spec.require_not_starting_player,
                counters: spec.counters,
                exile_cards_from_hand: spec.exile_cards_from_hand,
            },
        ),
        clause_display,
    )))
}

fn parse_pregame_reveal_from_opening_hand_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_pregame_reveal_from_opening_hand_tokens(tokens)
    else {
        return Ok(None);
    };

    let effect_tokens = trim_lexed_commas(&tokens[spec.effect_tokens]);
    let effects = super::clause_support::parse_effect_sentences_lexed(effect_tokens)?;
    let (trigger, one_shot, first_spell_of_game) = match spec.timing {
        keyword_static_lines::PregameRevealTiming::FirstUpkeep => (
            crate::cards::builders::TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any),
            true,
            false,
        ),
        keyword_static_lines::PregameRevealTiming::YourFirstUpkeep => (
            crate::cards::builders::TriggerSpec::BeginningOfUpkeep(PlayerFilter::You),
            true,
            false,
        ),
        keyword_static_lines::PregameRevealTiming::YourFirstPrecombatMainPhase => (
            crate::cards::builders::TriggerSpec::BeginningOfPrecombatMain(PlayerFilter::You),
            true,
            false,
        ),
        keyword_static_lines::PregameRevealTiming::EachOpponentFirstSpellOfGame => (
            crate::cards::builders::TriggerSpec::SpellCast {
                filter: None,
                mana_source_filter: None,
                caster: PlayerFilter::Opponent,
                timing: None,
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            },
            false,
            true,
        ),
    };

    Ok(Some(StaticAbilityAst::PregameRevealFromOpeningHand {
        trigger,
        effects,
        one_shot,
        first_spell_of_game,
        effect_before_timing: spec.effect_before_timing,
        display: render_token_slice(tokens),
    }))
}

pub fn parse_pregame_mulligan_redraw_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if early_static_facts::parse_pregame_mulligan_redraw_tokens(tokens).is_none() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount,
        render_token_slice(tokens),
    )))
}

pub fn parse_pregame_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !keyword_static_lines::parse_pregame_choose_color_tokens(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::pregame_action(
        crate::static_abilities::PregameActionKind::ChooseColor,
        render_token_slice(tokens),
    )))
}

pub fn parse_combined_pregame_choose_color_line(
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

pub fn parse_can_block_additional_creature_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(additional) = keyword_static_lines::parse_can_block_additional_creature_tokens(tokens)
    else {
        return Ok(None);
    };

    Ok(Some(
        StaticAbility::can_block_additional_creature_each_combat(additional),
    ))
}

pub fn parse_skulk_rules_text_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_skulk_rules_text_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::cant_be_blocked_by_lower_power_than_source(),
    ))
}

mod ward_cost_readings;

pub fn parse_ward_static_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("ward")) {
        return Ok(None);
    }

    if keyword_static_lines::parse_ward_abilities_dont_trigger_marker_tokens(tokens) {
        return Ok(Some(StaticAbility::suppress_matching_triggered_abilities(
            Some(ObjectFilter::creature().opponent_controls()),
            None,
            render_token_slice(tokens),
        )));
    }

    let Some(ward) = keyword_static_lines::parse_ward_cost_tokens(tokens) else {
        return Err(CardTextError::ParseError(
            "ward keyword missing cost".to_string(),
        ));
    };

    let cost_tokens = trim_commas(ward.cost_tokens);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "ward keyword missing cost".to_string(),
        ));
    }
    let input = ward_cost_readings::WardCost {
        tokens: &cost_tokens,
    };
    match ward_cost_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    Err(CardTextError::ParseError(format!(
        "unsupported ward cost clause (clause: '{}')",
        render_token_slice(tokens)
    )))
}

fn parse_compact_sacrifice_ward_cost(
    tokens: &[OwnedLexToken],
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let cst = match crate::grammar::activation_costs::parse_activation_cost_tokens(tokens) {
        Ok(cst) => cst,
        Err(_) => return Ok(None),
    };
    let [
        crate::grammar::activation_costs::ActivationCostSegmentCst::SacrificeChosen {
            count,
            filter,
        },
    ] = cst.segments.as_slice()
    else {
        return Ok(None);
    };
    if !cst.alternative_branches.is_empty()
        || count.dynamic_x
        || count.min != 1
        || count.max != Some(1)
    {
        return Ok(None);
    }

    Ok(Some(ironsmith_core::TotalCost::from_cost(
        crate::model::CompilerCost::Sacrifice {
            count: ChoiceCount::exactly(1),
            filter: filter.clone(),
            all: false,
            binding: None,
        },
    )))
}

#[rustfmt::skip]
pub fn parse_ward_discard_card_type_cost(tokens: &[OwnedLexToken]) -> Option<ironsmith_core::TotalCost<crate::model::CompilerCost>> {
    let words = LexedClause::new(tokens).words();
    if words
        .first().is_none_or(|word| word != "discard")
    {
        return None;
    }

    let mut idx = 1usize;
    let mut count = 1u32;
    let count_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, idx).unwrap_or(tokens.len());
    if let Some((value, used)) = parse_number(&tokens[count_token_idx..]) {
        count = value;
        let used_end = count_token_idx.saturating_add(used).min(tokens.len());
        idx += LexedClause::new(&tokens[count_token_idx..used_end]).word_len();
    }

    let tail_token_idx = words.map_word_or_end_to_token_boundary(idx).unwrap_or(tokens.len());
    if early_static_facts::parse_ward_discard_hand_tail_tokens(&tokens[tail_token_idx..]).is_some() {
        return Some(ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::DiscardHand));
    }

    while words
        .get(idx)
        .is_some_and(is_article)
    {
        idx += 1;
    }

    let mut card_types = Vec::<CardType>::new();
    while let Some(word) = words.get(idx) {
        if matches!(word, "card" | "cards") {
            idx += 1;
            break;
        }
        if matches!(word, "and" | "or" | "a" | "an") {
            idx += 1;
            continue;
        }
        let parsed = parse_card_type(word)?;
        if !card_types.iter().any(|card_type| card_type == &parsed) {
            card_types.push(parsed);
        }
        idx += 1;
    }

    if idx != words.len() {
        return None;
    }

    Some(ironsmith_core::TotalCost::from_cost(
        crate::model::CompilerCost::Discard {
            count,
            card_types,
            supertypes: Vec::new(),
            filter: None,
            random: false,
            name: None,
            other: false,
            binding: None,
        },
    ))
}

pub fn parse_composed_anthem_effects_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let anthem_head = static_keyword_line_shapes::parse_composed_anthem_head(tokens);
    if matches!(
        anthem_head,
        static_keyword_line_shapes::ComposedAnthemHead::Temporary
    ) {
        return Ok(None);
    }

    let comma_segments = anthem_grant_grammar::split_trailing_grant_segments(tokens);
    if comma_segments.len() < 2 {
        return Ok(None);
    }

    if comma_segments.len() == 2 {
        let where_tail = trim_commas(&comma_segments[1]);
        if keyword_static_lines::parse_where_x_value_prefix_tokens(&where_tail).is_some() {
            // The comma introduces a value binding, not a second predicate.
            // Keep every distributive subject when this route and the
            // multi-subject route recognize the same complete line.
            if let Some(abilities) = parse_multi_subject_anthem_line(tokens)? {
                return Ok(Some(abilities.into_iter().map(StaticAbilityAst::from).collect()));
            }
            if let Some(ability) = parse_anthem_line(tokens)? {
                return Ok(Some(vec![ability.into()]));
            }
        }
    }

    let first_action_idx = match anthem_head {
        static_keyword_line_shapes::ComposedAnthemHead::Temporary => return Ok(None),
        static_keyword_line_shapes::ComposedAnthemHead::Permanent {
            action: Some(action),
        } => action.token,
        static_keyword_line_shapes::ComposedAnthemHead::Permanent { action: None } => {
            return Ok(None);
        }
    };

    // A comma-separated subject enumeration is one static ability, not a
    // sequence of omitted-subject modifier clauses. The composed-family
    // splitter runs before the ordinary anthem rule, so give the complete
    // typed anthem back to that rule when every comma precedes the action.
    let commas_are_subject_punctuation = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.is_comma())
        .all(|(index, _)| index < first_action_idx);
    if commas_are_subject_punctuation && let Some(ability) = parse_anthem_line(tokens)? {
        return Ok(Some(vec![ability.into()]));
    }

    let subject_tokens = trim_commas(&tokens[..first_action_idx]);
    // This is only a speculative shape check.  A later static-line family may
    // own the line, so do not retain recovery diagnostics from this rejected
    // branch.  The committed anthem branches below parse the subject again
    // and retain any genuine recovery loss.
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let (subject_result, _) = crate::parse_loss::capture(|| parse_anthem_subject(&subject_tokens));
    if subject_result.is_err() {
        return Ok(None);
    }

    let mut saw_omitted_subject_clause = false;
    let mut compiled = Vec::new();

    for (idx, raw_segment) in comma_segments.into_iter().enumerate() {
        let Some(parsed) = keyword_static_lines::parse_composed_anthem_segment_tokens(&raw_segment)
        else {
            continue;
        };
        let mut segment = parsed.body_tokens.to_vec();
        if parsed.omitted_subject {
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

#[test]
fn comma_separated_anthem_subject_is_not_split_into_sibling_abilities() {
    for text in [
        "Other Rabbits, Bats, Birds, and Mice you control get +1/+1.",
        "Other Pests, Bats, Insects, Snakes, and Spiders you control get +1/+1.",
        "Other Spiders, Boars, Bats, Bears, Birds, Cats, Dogs, Frogs, Jackals, Lizards, Mice, Otters, Rabbits, Raccoons, Rats, Squirrels, Turtles, and Wolves you control get +1/+1.",
    ] {
        let tokens =
            crate::lexer::lex_line(text, 0).expect("comma-separated anthem subject should lex");
        let abilities = parse_composed_anthem_effects_line(&tokens)
            .expect("comma-separated anthem should parse")
            .expect("the composed family should delegate one complete anthem");
        assert_eq!(abilities.len(), 1, "{text}: {abilities:#?}");
    }
}

pub fn parse_static_text_marker_line(tokens: &[OwnedLexToken]) -> Option<StaticAbility> {
    let words = crate::lexer::token_word_refs(tokens);
    let normalized = words
        .iter()
        .map(|word| {
            if matches!(*word, "can't" | "cannot") {
                "cant"
            } else {
                *word
            }
        })
        .collect::<Vec<_>>();
    if normalized
        == [
            "spells",
            "and",
            "abilities",
            "your",
            "opponents",
            "control",
            "cant",
            "cause",
            "you",
            "to",
            "sacrifice",
            "permanents",
        ]
    {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::BeSacrificedByCause {
                filter: ObjectFilter::permanent().you_control(),
                cause: ironsmith_core::CauseFilter {
                    cause_type: Some(ironsmith_core::CauseTypeFilter::OneOf(vec![
                        ironsmith_core::CauseType::Effect,
                        ironsmith_core::CauseType::Cost,
                    ])),
                    source_filter: None,
                    controller_filter: Some(ironsmith_core::ControllerFilter::Opponent),
                },
            },
            "Spells and abilities your opponents control can't cause you to sacrifice permanents."
                .to_string(),
        ));
    }
    if tokens.is_empty() {
        return None;
    }

    if keyword_static_lines::parse_dungeon_room_trigger_duplication_marker_tokens(tokens) {
        return Some(StaticAbility::dungeon_room_trigger_duplication(
            "Room abilities of dungeons you own trigger an additional time.",
        ));
    }

    if is_once_each_turn_play_from_exile_marker_guard_lexed(tokens) {
        return None;
    }

    if is_doctors_companion_marker_line_lexed(tokens) {
        return Some(StaticAbility::doctors_companion());
    }

    if let Some(marker) = keyword_static_lines::parse_static_text_marker_kind_tokens(tokens) {
        return Some(match marker {
            keyword_static_lines::StaticTextMarkerKind::Banding => StaticAbility::banding(),
            keyword_static_lines::StaticTextMarkerKind::AuraRetentionClarification => {
                StaticAbility::protection_doesnt_remove_auras(
                    false,
                    keyword_static_clause_text(tokens),
                )
            }
            keyword_static_lines::StaticTextMarkerKind::AurasRetentionClarification => {
                // The printed surface; lexed clause text loses the capital
                // of the "Auras" subtype.
                StaticAbility::protection_doesnt_remove_auras(
                    true,
                    "This effect doesn't remove Auras.",
                )
            }
            keyword_static_lines::StaticTextMarkerKind::YouHaveHexproof => {
                StaticAbility::restriction(
                    crate::effect::Restriction::player_hexproof_from(
                        PlayerFilter::You,
                        ObjectFilter::default(),
                    ),
                    "You have hexproof".to_string(),
                )
            }
            keyword_static_lines::StaticTextMarkerKind::OpponentsCastOnlyAsSorcery => {
                StaticAbility::restriction(
                    crate::effect::Restriction::cast_spells_only_as_sorcery(
                        PlayerFilter::Opponent,
                    ),
                    "Each opponent can cast spells only any time they could cast a sorcery."
                        .to_string(),
                )
            }
            keyword_static_lines::StaticTextMarkerKind::DoubleDamageToEnchantedPlayer => {
                StaticAbility::double_damage_amount_replacement(
                    ObjectFilter::default(),
                    Some(PlayerFilter::TaggedPlayer((crate::tag::CompilerReferenceTag::Enchanted.bind()).into())),
                    None,
                    "If a source would deal damage to enchanted player, it deals double that damage to that player instead.".to_string(),
                )
            }
        });
    }

    if is_companion_marker_line_lexed(tokens) {
        return parse_companion_ability(tokens).or_else(|| Some(keyword_static_marker(tokens)));
    }

    if is_more_than_meets_the_eye_marker_line_lexed(tokens) {
        return Some(keyword_static_marker(tokens));
    }

    // Mana-value parity protection is a complete typed protection quality.
    // It is owned by the keyword grammar, not an unsupported marker.

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
        let condition = PredicateAst::Not(Box::new(PredicateAst::TurnEvents(
            TurnEventPredicateAst::ObjectEnteredBattlefieldThisTurn(ObjectFilter::source()),
        )));
        return Some(StaticAbility::new(
            GrantAbility::source(StaticAbility::can_attack_as_though_haste())
                .with_condition(condition),
        ));
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

    if let Ok(Some(ward)) = parse_ward_static_ability_line(tokens) {
        return Some(ward);
    }

    if is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed(tokens) {
        return Some(StaticAbility::restriction(
            crate::effect::Restriction::untap(ObjectFilter::land()),
            "Lands don't untap during their controllers' untap steps".to_string(),
        ));
    }

    None
}

pub fn parse_affinity_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let core_tokens = if let Some(paren_idx) = locate_token_kind(tokens, TokenKind::LParen) {
        trim_commas(&tokens[..paren_idx])
    } else {
        trim_commas(tokens)
    };
    let Some(spec) = static_keyword_line_shapes::parse_affinity_for_filter(&core_tokens) else {
        return Ok(None);
    };

    if spec.is_artifacts {
        return Ok(None);
    }

    let filter_tokens = trim_commas(spec.filter_tokens);
    let mut filter = parse_object_filter_lexed(&filter_tokens, false)?;
    if filter.controller.is_none() {
        filter.controller = Some(PlayerFilter::You);
    }
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(
            Value::Count(filter.clone()),
            crate::static_abilities::ThisSpellCostCondition::Always,
        )
        .with_affinity_filter(filter),
    )))
}

pub fn parse_filter_dont_untap_during_controllers_untap_steps_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if crate::grammar::abilities::parse_doesnt_untap_during_untap_step_spec_lexed(tokens).is_some()
    {
        // Source- and attached-subject suppression lower through the typed
        // subject production (an attached grant); this rule owns only
        // battlefield filter subjects.
        return Ok(None);
    }
    // "As long as you control another snow permanent, enchanted creature
    // doesn't untap ..." (Winter's Rest): the leading condition belongs to
    // the generic conditional-static wrapper, which strips it and re-reads
    // the subject; reading it as part of the subject would fail the line.
    if crate::grammar::abilities::split_as_long_as_condition_prefix_lexed(tokens).is_some() {
        return Ok(None);
    }
    let Some(spec) = keyword_static_lines::parse_dont_untap_during_controllers_step_tokens(tokens)
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.subject_tokens);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    // "Enchanted creature gets +1/+1 and doesn't untap ...": the subject
    // phrase carries another predicate; a filter parse would silently drop it.
    if subject_tokens.iter().any(|token| {
        token.is_any_word(&[
            "gets", "get", "has", "have", "gains", "gain", "loses", "lose", "and",
        ])
    }) {
        return Ok(None);
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = render_token_slice(&subject_tokens);
    let mut display = if spec.singular_subject {
        format!("{subject_text} doesn't untap during its controller's untap step")
    } else {
        format!("{subject_text} don't untap during their controllers' untap steps")
    };
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

fn parse_graveyard_metric_threshold_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::static_abilities::GraveyardCountMetric, u32)>, CardTextError> {
    let Some(quantified) = keyword_static_lines::parse_there_is_or_are_quantified_tokens(tokens)
    else {
        return Ok(None);
    };
    let Ok((comparison, used)) = parse_static_quantity_prefix(quantified, false) else {
        return Ok(None);
    };
    let Some(threshold) = comparison_to_at_least_threshold(&comparison) else {
        return Ok(None);
    };

    let rest = &quantified[used..];
    let metric = match keyword_static_lines::parse_graveyard_metric_tokens(rest) {
        Some(keyword_static_lines::GraveyardMetricKind::CardTypes) => {
            crate::static_abilities::GraveyardCountMetric::CardTypes
        }
        Some(keyword_static_lines::GraveyardMetricKind::ManaValues) => {
            crate::static_abilities::GraveyardCountMetric::ManaValues
        }
        None => return Ok(None),
    };
    Ok(Some((metric, threshold)))
}

pub fn parse_conditional_source_spell_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = static_keyword_line_shapes::parse_conditional_spell_keyword(tokens) else {
        return Ok(None);
    };
    let keyword = match spec.keyword {
        static_keyword_line_shapes::ConditionalSpellKeyword::Flash => {
            crate::static_abilities::ConditionalSpellKeywordKind::Flash
        }
        static_keyword_line_shapes::ConditionalSpellKeyword::Cascade => {
            crate::static_abilities::ConditionalSpellKeywordKind::Cascade
        }
    };

    let condition_tokens = trim_commas(spec.condition_tokens);
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

#[rustfmt::skip]
pub fn parse_enters_tapped_with_choose_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(shape) = early_static_facts::parse_enters_tapped_choice_shape_tokens(tokens) else {
        return Ok(None);
    };
    let trailing = &tokens[shape.tapped_token + 1..];
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

pub fn parse_damage_not_removed_cleanup_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_damage_not_removed_cleanup_tokens(tokens) {
        return Ok(Some(StaticAbility::damage_not_removed_during_cleanup()));
    }
    Ok(None)
}

fn parse_as_enters_choice_subject_tokens<'a>(
    tokens: &'a [OwnedLexToken],
    this_kind_display_pairs: &[(&str, &'static str)],
) -> Option<(&'a [OwnedLexToken], &'static str)> {
    let clause = LexedClause::new(tokens);
    let words = clause.words();
    let (tail_word_idx, display_subject) =
        parse_as_enters_choice_subject_clause(clause, this_kind_display_pairs)?;
    let tail_token_idx = words.map_word_or_end_to_token_boundary(tail_word_idx)?;
    Some((&tokens[tail_token_idx..], display_subject))
}

fn parse_as_enters_choice_subject_clause(
    clause: LexedClause<'_>,
    this_kind_display_pairs: &[(&str, &'static str)],
) -> Option<(usize, &'static str)> {
    let word_refs = clause.word_refs();
    let words = word_refs.as_slice();
    let allowed_kinds = this_kind_display_pairs
        .iter()
        .map(|(kind, _)| *kind)
        .collect::<Vec<_>>();
    let shape = static_keyword_line_shapes::parse_as_enters_subject(words, &allowed_kinds)?;
    let display_subject = match shape.subject {
        static_keyword_line_shapes::AsEntersSubject::This(Some(kind)) => {
            let mut display = None;
            for (candidate, candidate_display) in this_kind_display_pairs {
                if *candidate == kind {
                    display = Some(*candidate_display);
                    break;
                }
            }
            display?
        }
        static_keyword_line_shapes::AsEntersSubject::It => "it",
        static_keyword_line_shapes::AsEntersSubject::This(None)
        | static_keyword_line_shapes::AsEntersSubject::SourceReference => "this",
    };
    Some((shape.tail_word, display_subject))
}

pub fn parse_choose_basic_land_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_AURA_SUBJECTS).or_else(|| {
            parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
        })
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    if let Some(consumed) = parse_choose_basic_land_type_phrase_words(&tail_words)
        && consumed == tail_words.len()
    {
        return Ok(Some(StaticAbility::choose_basic_land_type_as_enters(
            format!("As {display_subject} enters, choose a basic land type."),
        )));
    }
    let Some(consumed) = parse_choose_land_type_phrase_words(&tail_words) else {
        return Ok(None);
    };
    if consumed != tail_words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_land_type_as_enters(format!(
        "As {display_subject} enters, choose a land type."
    ))))
}

/// "This land is the chosen type." (Multiversal Passage)
pub fn parse_source_land_is_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !crate::grammar::abilities::is_source_land_is_chosen_type_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::source_land_is_chosen_type(
        "This land is the chosen type.".to_string(),
    )))
}

pub fn parse_enchanted_land_is_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_enchanted_land_is_chosen_type_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::enchanted_land_is_chosen_type(
        "Enchanted land is the chosen type.".to_string(),
    )))
}

pub fn parse_choose_card_name_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    if early_static_facts::parse_choose_card_name_tail_tokens(tail_tokens).is_none() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_card_name_as_enters(format!(
        "As {display_subject} enters, choose a card name."
    ))))
}

pub fn parse_revealed_hand_choose_nonland_card_name_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() != 2 {
        return Ok(None);
    }

    let first_clause = LexedClause::new(sentences[0]);
    let Some((idx, display_subject)) =
        parse_as_enters_choice_subject_clause(first_clause, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let first_tail = first_clause.after_words(idx).map(LexedClause::tokens);
    if !first_tail.is_some_and(keyword_static_lines::parse_revealed_hand_as_enters_tail_tokens) {
        return Ok(None);
    }

    if !keyword_static_lines::parse_choose_revealed_nonland_name_tail_tokens(sentences[1]) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::choose_revealed_hand_nonland_card_name_as_enters(format!(
            "As {display_subject} enters, each opponent reveals their hand. You choose the name of a nonland card revealed this way."
        )),
    ))
}

pub fn parse_note_life_total_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    if early_static_facts::parse_note_life_total_tail_tokens(tail_tokens).is_none() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::note_life_total_as_enters(format!(
        "As {display_subject} enters, note your life total."
    ))))
}

pub fn parse_discard_hand_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    if !crate::word_primitives::parse_sequence_complete(&tail_words, &["discard", "your", "hand"]) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::discard_hand_as_enters(format!(
        "As {display_subject} enters, discard your hand."
    ))))
}

pub fn parse_source_is_chosen_type_in_addition_line(
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

pub fn parse_source_is_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((subject, has_article)) =
        keyword_static_lines::parse_source_is_chosen_color_tokens(tokens)
    else {
        return Ok(None);
    };
    let display_subject = subject.display();
    let display = if has_article {
        format!("{display_subject} is the chosen color.")
    } else {
        format!("{display_subject} is chosen color.")
    };

    Ok(Some(StaticAbility::set_chosen_color(
        ObjectFilter::source(),
        display,
    )))
}

pub fn parse_choose_creature_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    let Some((consumed, excluded_subtypes)) = parse_choose_creature_type_phrase_words(&tail_words)?
    else {
        return Ok(None);
    };
    if !excluded_subtypes.is_empty() {
        return Ok(None);
    }
    if consumed != tail_words.len() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::choose_creature_type_as_enters(
        format!("As {display_subject} enters, choose a creature type."),
    )))
}

pub fn parse_choose_color_and_creature_type_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    if !crate::word_primitives::parse_sequence_complete(
        &tail_words,
        &["choose", "a", "color", "and", "a", "creature", "type"],
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        StaticAbility::choose_color_as_enters(
            None,
            format!("As {display_subject} enters, choose a color."),
        ),
        StaticAbility::choose_creature_type_as_enters(format!(
            "As {display_subject} enters, choose a creature type."
        )),
    ]))
}

fn parse_color_creature_type_pair_options(tail_words: &[&str]) -> Option<Vec<String>> {
    let (choice_word, option_words) = tail_words.split_first()?;
    if *choice_word != "choose" {
        return None;
    }

    let option_words = option_words
        .iter()
        .copied()
        .filter(|word| *word != "," && *word != "or")
        .collect::<Vec<_>>();
    let mut option_chunks = option_words.chunks_exact(2);
    let raw_options = option_chunks.by_ref().collect::<Vec<_>>();
    if !option_chunks.remainder().is_empty() || raw_options.len() < 2 {
        return None;
    }

    let mut options = Vec::with_capacity(raw_options.len());
    for words in raw_options {
        let [color_word, subtype_word] = words else {
            return None;
        };
        let color = Color::from_name(color_word)?;
        let subtype =
            parse_subtype_flexible(subtype_word).filter(crate::types::Subtype::is_creature_type)?;
        options.push(format!("{} {}", color.name(), subtype));
    }
    Some(options)
}

pub fn parse_choose_color_creature_type_pairs_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    let Some(options) = parse_color_creature_type_pair_options(&tail_words) else {
        return Ok(None);
    };

    let display_options = match options.as_slice() {
        [first, second] => format!("{first} or {second}"),
        [head @ .., last] => format!("{}, or {last}", head.join(", ")),
        _ => return Ok(None),
    };
    Ok(Some(StaticAbility::choose_named_option_as_enters(
        options,
        format!("As {display_subject} enters, choose {display_options}."),
    )))
}

pub fn parse_choose_named_options_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    // Correlated color/type pairs form a separate complete choice domain.
    // Reject that domain here so both registry candidates can be evaluated
    // without a post-evaluation ownership override.
    if parse_color_creature_type_pair_options(&tail_words).is_some() {
        return Ok(None);
    }
    let Some(shape) = early_static_facts::parse_named_choice_alternatives_shape_words(&tail_words)
    else {
        return Ok(None);
    };
    let choice_offset = shape.choice_word;
    let choice_words = &tail_words[choice_offset..];
    // "choose 2, 3, or 4 at random" (Haktos the Unscarred): the game picks
    // one listed option uniformly instead of the controller.
    let (choice_words, at_random) = match choice_words {
        [head @ .., "at", "random"] if !head.is_empty() => (head, true),
        _ => (choice_words, false),
    };
    if parse_choose_color_phrase_words(choice_words)?
        .is_some_and(|(consumed, _)| consumed == choice_words.len())
        || parse_choose_player_phrase_words(choice_words) == Some(choice_words.len())
        || parse_choose_basic_land_type_phrase_words(choice_words) == Some(choice_words.len())
        || parse_choose_land_type_phrase_words(choice_words) == Some(choice_words.len())
        || parse_choose_creature_type_phrase_words(choice_words)?
            .is_some_and(|(consumed, _)| consumed == choice_words.len())
        // "choose a card type other than creature or land" is one card-type
        // choice with excluded options, not two named alternatives.
        || (crate::word_primitives::sequence_occurs(choice_words, &["card", "type"])
            && super::activation_and_restrictions::parse_choose_card_type_phrase_words(
                choice_words,
            )?
            .is_some_and(|(consumed, _)| consumed == choice_words.len()))
    {
        return Ok(None);
    }

    let mut card_type_options = Vec::new();
    for word in choice_words.iter().skip(1).filter(|_| !at_random) {
        if *word == "or" || *word == "," {
            continue;
        }
        let Some(card_type) = parse_card_type(word.trim_end_matches('s')) else {
            card_type_options.clear();
            break;
        };
        crate::slice_primitives::push_unique(&mut card_type_options, card_type);
    }
    if card_type_options.len() >= 2 {
        return Ok(Some(StaticAbility::choose_named_option_as_enters(
            card_type_options
                .iter()
                .map(|card_type| card_type.name().to_string())
                .collect(),
            format!("As {display_subject} enters, {}.", choice_words.join(" ")),
        )));
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
    // "choose Elemental, Elf, Faerie, ..., or Treefolk": the comma list
    // arrives without its commas, so a run of words that are each a subtype
    // is several one-word options, not one multi-word name.
    let options = options
        .into_iter()
        .flat_map(|option| {
            let words = option.split_whitespace().collect::<Vec<_>>();
            if words.len() > 1
                && (words
                    .iter()
                    .all(|word| crate::util::parse_subtype_word(word).is_some())
                    || (at_random && words.iter().all(|word| word.parse::<u32>().is_ok())))
            {
                words.into_iter().map(str::to_string).collect::<Vec<_>>()
            } else {
                vec![option]
            }
        })
        .collect::<Vec<_>>();
    if options.len() < 2 {
        return Ok(None);
    }
    if at_random {
        let display_options = match options.as_slice() {
            [first, second] => format!("{first} or {second}"),
            [head @ .., last] => format!("{}, or {last}", head.join(", ")),
            _ => return Ok(None),
        };
        return Ok(Some(
            StaticAbility::choose_named_option_at_random_as_enters(
                options,
                format!("As {display_subject} enters, choose {display_options} at random."),
            ),
        ));
    }

    let display_options = options
        .iter()
        .map(|option| {
            let mut chars = option.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" or ");

    Ok(Some(StaticAbility::choose_named_option_as_enters(
        options,
        format!("As {display_subject} enters, choose {display_options}."),
    )))
}

fn parse_trigger_duplication_source_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if early_static_facts::parse_trigger_duplication_source_or_owned_emblem_tokens(&tokens)
        .is_some()
    {
        let mut emblem = ObjectFilter::default();
        emblem.zone = Some(Zone::Command);
        emblem.owner = Some(PlayerFilter::You);

        let mut filter = ObjectFilter::default();
        filter.any_of = vec![ObjectFilter::source(), emblem];
        return Ok(filter);
    }

    // "a triggered ability of Cloud or an Equipment attached to it": the
    // source itself, or an object whose `it` names that same source.
    // An unnormalized card-name mention ("of Cloud or ...") is not an object
    // filter; in this position it can only name the source itself.
    if let Some(or_index) = tokens.iter().position(|token| token.is_word("or"))
        && or_index > 0
        && match parse_object_filter_with_grammar_entrypoint(&tokens[..or_index], false) {
            Ok(left) => {
                left.source
                    || (left == ObjectFilter::default()
                        && tokens[..or_index]
                            .iter()
                            .all(|token| token.as_word().is_some()))
            }
            Err(_) => tokens[..or_index]
                .iter()
                .all(|token| token.as_word().is_some()),
        }
        && let Ok(mut right) =
            parse_object_filter_with_grammar_entrypoint(&tokens[or_index + 1..], false)
        && !right.source
    {
        let mut relates_to_source = false;
        right.tagged_constraints.retain(|constraint| {
            let it_attachment = constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject;
            relates_to_source |= it_attachment;
            !it_attachment
        });
        if relates_to_source
            && right.attached_to_object.is_none()
            && right.tagged_constraints.iter().all(|constraint| {
                constraint.tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
            })
        {
            right.attached_to_object = Some(Box::new(ObjectFilter::source()));
            let mut filter = ObjectFilter::default();
            filter.any_of = vec![ObjectFilter::source(), right];
            return Ok(filter);
        }
    }

    // "a colorless spell you control or another colorless permanent you
    // control" (Echoes of Eternity): two independently nouned arms, the second
    // excluding the source itself.
    let or_positions = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.is_word("or"))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if let [or_index] = or_positions.as_slice()
        && *or_index > 0
        && tokens
            .get(or_index + 1)
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        && let Ok(left) = parse_object_filter_with_grammar_entrypoint(&tokens[..*or_index], false)
        && let Ok(mut right) =
            parse_object_filter_with_grammar_entrypoint(&tokens[or_index + 1..], false)
        && left != ObjectFilter::default()
        && right != ObjectFilter::default()
    {
        right.other = true;
        // "a Shaman or another Wizard you control" (Harmonic Prodigy): a
        // trailing controller scope is shared by an arm that names none.
        let mut left = left;
        if left.controller.is_none() && left.owner.is_none() {
            left.controller = right.controller.clone();
        }
        if left.zone.is_none() {
            left.zone = right.zone;
        }
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![left, right];
        return Ok(filter);
    }

    let mut filter = parse_object_filter_with_grammar_entrypoint(&tokens, false)?;
    // "a triggered ability of another Wolf or battle you control" (Chief of
    // the Wilds): the leading determiner excludes the source from the whole
    // coordinated set.
    if tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        && !filter.other
        && filter.any_of.iter().all(|branch| !branch.other)
    {
        filter.other = true;
    }
    Ok(filter)
}

fn parse_trigger_duplication_event_matcher(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let clause_display = render_token_slice(&tokens);

    let build_filter = |subject_tokens: &[OwnedLexToken]| -> Result<ObjectFilter, CardTextError> {
        let subject_tokens = trim_edge_punctuation(subject_tokens);
        // "a legendary permanent or an artifact": each article-led arm is a
        // complete object noun, so the arms are a union, not one object that
        // is both.
        if let Some(or_index) = subject_tokens.iter().position(|token| token.is_word("or"))
            && or_index > 0
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            && subject_tokens
                .get(or_index + 1)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            && subject_tokens[or_index + 1..]
                .iter()
                .all(|token| !token.is_word("or"))
            && let Ok(left) =
                parse_object_filter_with_grammar_entrypoint(&subject_tokens[..or_index], false)
            && let Ok(right) =
                parse_object_filter_with_grammar_entrypoint(&subject_tokens[or_index + 1..], false)
            && left != right
        {
            let mut union = ObjectFilter::default();
            if left.zone == right.zone {
                union.zone = left.zone;
            }
            union.any_of = vec![left, right];
            for branch in &mut union.any_of {
                branch.zone = None;
            }
            return Ok(union);
        }
        parse_object_filter_with_grammar_entrypoint(&subject_tokens, false)
    };

    use early_static_facts::{
        TriggerDuplicationEventKind as EventKind, TriggerDuplicationEventShape as EventShape,
        TriggerDuplicationEventSyntaxError as EventSyntaxError,
        TriggerDuplicationPlayerSubject as PlayerSubject,
    };
    let shape = match early_static_facts::parse_trigger_duplication_event_shape_tokens(&tokens) {
        Ok(Some(shape)) => shape,
        Ok(None) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported trigger-duplication cause clause (clause: '{}')",
                clause_display
            )));
        }
        Err(EventSyntaxError::MissingTurnedFaceUpSubject) => {
            return Err(CardTextError::ParseError(format!(
                "missing turned-face-up subject in trigger-duplication clause (clause: '{}')",
                clause_display
            )));
        }
        Err(EventSyntaxError::MissingSpellSubject) => {
            return Err(CardTextError::ParseError(format!(
                "missing spell subject in trigger-duplication clause (clause: '{}')",
                clause_display
            )));
        }
    };

    match shape {
        EventShape::TurningFaceUp { subject_tokens } => Ok(TriggerSpec::TurnedFaceUp(
            build_filter(&tokens[subject_tokens])?,
        )),
        EventShape::YouCastingOrCopying { subject_tokens } => {
            let filter = build_filter(&tokens[subject_tokens])?;
            Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::SpellCast {
                    filter: Some(filter.clone()),
                    mana_source_filter: None,
                    caster: PlayerFilter::You,
                    timing: None,
                    during_turn: None,
                    min_spells_this_turn: None,
                    exact_spells_this_turn: None,
                    from_not_hand: false,
                }),
                Box::new(TriggerSpec::SpellCopied {
                    filter: Some(filter),
                    copier: PlayerFilter::You,
                }),
            ))
        }
        EventShape::SubjectEvent {
            subject_tokens: _,
            kind: EventKind::DrawsCard(Some(player)),
        } => Ok(TriggerSpec::PlayerDrawsCard(match player {
            PlayerSubject::Any => PlayerFilter::Any,
            PlayerSubject::You => PlayerFilter::You,
            PlayerSubject::Opponent => PlayerFilter::Opponent,
        })),
        EventShape::SubjectEvent {
            subject_tokens,
            kind,
        } => {
            let filter = build_filter(&tokens[subject_tokens])?;
            Ok(match kind {
                EventKind::DealsCombatDamageToPlayer => TriggerSpec::DealsCombatDamageToPlayer {
                    source: filter,
                    player: PlayerFilter::Any,
                },
                EventKind::BecomesTargeted => TriggerSpec::BecomesTargeted(filter),
                EventKind::IsDealtDamage => TriggerSpec::IsDealtDamage(filter),
                EventKind::EntersOrLeavesBattlefield => TriggerSpec::Either(
                    Box::new(TriggerSpec::EntersBattlefield {
                        filter: filter.clone(),
                        cause_filter: None,
                        origin_condition: None,
                        during_turn: None,
                    }),
                    Box::new(TriggerSpec::LeavesBattlefield(filter)),
                ),
                EventKind::EntersBattlefield => TriggerSpec::EntersBattlefield {
                    filter,
                    cause_filter: None,
                    origin_condition: None,
                    during_turn: None,
                },
                EventKind::LeavesBattlefield => TriggerSpec::LeavesBattlefield(filter),
                EventKind::DrawsCard(None) => TriggerSpec::PlayerDrawsCard(PlayerFilter::Any),
                EventKind::DrawsCard(Some(_)) => unreachable!("handled typed player subject"),
                EventKind::Attacks => TriggerSpec::Attacks(filter),
                EventKind::Dies => TriggerSpec::Dies(filter),
            })
        }
    }
}

fn parse_trigger_duplication_core(
    tokens: &[OwnedLexToken],
) -> Result<Option<(StaticAbility, Option<PredicateAst>)>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(shape) = early_static_facts::parse_trigger_duplication_core_shape_tokens(&tokens)
    else {
        return Ok(None);
    };
    use early_static_facts::TriggerDuplicationCoreShape;
    let (source_filter, event_matcher, condition) = match shape {
        TriggerDuplicationCoreShape::AbilityTriggers {
            source_tokens,
            condition_tokens,
        } => (
            parse_trigger_duplication_source_filter(source_tokens)?,
            None,
            condition_tokens
                .map(parse_static_condition_clause)
                .transpose()?,
        ),
        TriggerDuplicationCoreShape::EventCausesAbility {
            event_tokens,
            source_tokens,
        } => (
            parse_trigger_duplication_source_filter(source_tokens)?,
            Some(parse_trigger_duplication_event_matcher(event_tokens)?),
            None,
        ),
    };

    Ok(Some((
        StaticAbility::duplicate_matching_triggered_abilities(
            Some(source_filter),
            event_matcher,
            1,
            crate::lexer::token_word_refs(&tokens).join(" "),
        ),
        condition,
    )))
}

pub fn parse_trigger_duplication_line_ast(
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

pub fn parse_trigger_suppression_line_ast(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed(&tokens) {
        // The dedicated creatures-entering suppression production owns it.
        return Ok(None);
    }
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
    let display = crate::lexer::token_word_refs(&tokens).join(" ");

    Ok(Some(StaticAbilityAst::from(
        StaticAbility::suppress_matching_triggered_abilities(
            source_filter,
            Some(event_matcher),
            display,
        ),
    )))
}

pub fn parse_double_damage_from_sources_you_control_of_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_double_damage_from_sources_you_control_of_chosen_type_line_lexed(tokens) {
        return Ok(None);
    }

    // Use the common typed multiplier owner, including source LKI support.
    // No permanent/creature domain is implied by "sources": Kindred spells
    // with the chosen subtype can deal damage too.
    let mut source_filter = ObjectFilter::default().controlled_by(PlayerFilter::You);
    source_filter.chosen_creature_type = true;
    Ok(Some(StaticAbility::multiply_damage_amount_replacement(
        source_filter,
        Some(PlayerFilter::Any),
        Some(ObjectFilter::default()),
        2,
        false,
        "Double all damage that sources you control of the chosen type would deal.".to_string(),
    )))
}

pub fn parse_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(shape) = keyword_static_lines::parse_additive_damage_amount_tokens(&tokens) else {
        return Ok(None);
    };
    if shape.this_turn {
        return Ok(None);
    }
    let Some(spec) = damage_addition_parts_from_shape(shape)? else {
        return Ok(None);
    };
    let mut display = render_token_slice(&tokens).trim().to_string();
    if !crate::string_primitives::ends_with_char(&display, '.') {
        display.push('.');
    }
    let fixed = match spec.delta.unhinted() {
        Value::Fixed(n) => Some(*n),
        _ => None,
    };
    let ability = StaticAbility::modify_damage_amount_replacement_with_noncombat_only(
        spec.source_filter,
        spec.target_player_filter,
        spec.target_object_filter,
        fixed.unwrap_or(0),
        spec.noncombat_only,
        display,
    );
    Ok(Some(if fixed.is_some() {
        ability
    } else {
        ability.with_dynamic_damage_delta(spec.delta)
    }))
}

pub(crate) fn damage_addition_parts_from_shape(
    spec: keyword_static_lines::AdditiveDamageAmountSpec<'_>,
) -> Result<Option<ironsmith_core::RegisterDamageAdditionEffect>, CardTextError> {
    let damaged_words = parser_token_word_refs(spec.damaged_tokens);
    let (target_player_filter, target_object_filter) =
        parse_damage_amount_replacement_target_filters(&damaged_words)?;
    if target_player_filter.is_none() && target_object_filter.is_none() {
        return Ok(None);
    }
    if let Some(repeated_target_tokens) = spec.repeated_target_tokens {
        let repeated_words = parser_token_word_refs(repeated_target_tokens);
        let demonstrative_only = match repeated_words.as_slice() {
            ["permanent", "or", "player"] | ["player", "or", "permanent"] => {
                target_player_filter.is_some() && target_object_filter.is_some()
            }
            ["permanent"] => target_object_filter.is_some() && target_player_filter.is_none(),
            ["player"] => target_player_filter.is_some() && target_object_filter.is_none(),
            ["creature"] => {
                target_player_filter.is_none()
                    && target_object_filter
                        .as_ref()
                        .is_some_and(|filter| filter.card_types == [CardType::Creature])
            }
            ["creature", "or", "player"] | ["player", "or", "creature"] => {
                target_player_filter.is_some()
                    && target_object_filter
                        .as_ref()
                        .is_some_and(|filter| filter.card_types == [CardType::Creature])
            }
            _ => false,
        };
        if !demonstrative_only {
            let (repeated_player_filter, repeated_object_filter) =
                parse_damage_amount_replacement_target_filters(&repeated_words)?;
            if repeated_player_filter.as_ref() != target_player_filter.as_ref()
                || repeated_object_filter.as_ref() != target_object_filter.as_ref()
            {
                return Ok(None);
            }
        }
    }
    let source_filter = damage_source_filter_from_shape(spec.source)?;
    let delta = if let Some(definition) = spec.definition_tokens {
        let Some(value) = parse_value_binding_clause(definition) else {
            return Ok(None);
        };
        value
    } else {
        let words = parser_token_word_refs(spec.delta_tokens);
        let words = words
            .strip_prefix(&["an", "amount", "of", "damage", "equal", "to"])
            .unwrap_or(&words);
        let Some((value, used)) = parse_value_expr_words(words) else {
            return Ok(None);
        };
        if used != words.len() {
            return Ok(None);
        }
        value
    };
    Ok(Some(ironsmith_core::RegisterDamageAdditionEffect {
        source_filter,
        target_player_filter,
        target_object_filter,
        delta,
        noncombat_only: spec.noncombat_only,
        mode: ironsmith_core::ReplacementApplyMode::UntilEndOfTurn,
    }))
}

pub fn parse_prevent_half_damage_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(spec) = keyword_static_lines::parse_prevent_half_damage_tokens(&tokens) else {
        return Ok(None);
    };
    let damaged_words = parser_token_word_refs(spec.damaged_tokens);
    let (target_player_filter, target_object_filter) =
        parse_damage_amount_replacement_target_filters(&damaged_words)?;
    if target_player_filter.is_none() && target_object_filter.is_none() {
        return Ok(None);
    }
    let source_filter = damage_source_filter_from_shape(spec.source)?;
    let mut display = render_token_slice(&tokens).trim().to_string();
    if !crate::string_primitives::ends_with_char(&display, '.') {
        display.push('.');
    }
    Ok(Some(StaticAbility::prevent_half_damage_replacement(
        source_filter,
        target_player_filter,
        target_object_filter,
        spec.round_up,
        display,
    )))
}

pub fn parse_double_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(ability) = parse_double_damage_from_sources_you_control_of_chosen_type_line(tokens)? {
        return Ok(Some(ability));
    }
    let tokens = trim_edge_punctuation(tokens);
    let Some(spec) = keyword_static_lines::parse_damage_multiplier_tokens(&tokens) else {
        return Ok(None);
    };
    if spec.this_turn {
        return Ok(None);
    }
    let Some(parts) = damage_multiplier_parts_from_shape(spec)? else {
        return Ok(None);
    };
    let mut display = render_token_slice(&tokens).trim().to_string();
    if !crate::string_primitives::ends_with_char(&display, '.') {
        display.push('.');
    }
    let mut ability = StaticAbility::multiply_damage_amount_replacement(
        parts.source_filter,
        parts.target_player_filter,
        parts.target_object_filter,
        parts.factor,
        parts.combat_only,
        display,
    );
    if parts.noncombat_only {
        ability = ability.with_noncombat_only_damage_multiplier();
    }
    if let Some(condition) = spec.condition_tokens {
        ability = ability.with_condition(parse_static_condition_clause(condition)?);
    }
    Ok(Some(ability))
}

/// Both static and resolving replacements use the same source/recipient
/// semantics. The repeated recipient is an anaphor, never a redirection.
pub(crate) fn damage_multiplier_parts_from_shape(
    spec: keyword_static_lines::DamageMultiplierSpec<'_>,
) -> Result<Option<ironsmith_core::RegisterDamageMultiplierEffect>, CardTextError> {
    let (target_player_filter, target_object_filter) = if let Some(tokens) = spec.damaged_tokens {
        let words = parser_token_word_refs(tokens);
        let filters = parse_damage_amount_replacement_target_filters(&words)?;
        if filters.0.is_none() && filters.1.is_none() {
            return Ok(None);
        }
        if let Some(repeated) = spec.repeated_target_tokens {
            let repeated = parser_token_word_refs(repeated);
            let normalized = strip_leading_word_refs_any(&words, &["a", "an", "the"]);
            let same = repeated == words
                || repeated.strip_prefix(&["that"]).is_some_and(|tail| {
                    tail == normalized
                        || (tail == ["player"] && filters.0.is_some() && filters.1.is_none())
                        || (tail == ["creature"] && normalized.ends_with(&["creature"]))
                        || (tail == ["permanent"]
                            && filters.0.is_none()
                            && normalized.ends_with(&["permanent"]))
                        || (filters.0.is_some()
                            && filters.1.is_some()
                            && matches!(
                                tail,
                                ["player", "or", "permanent"] | ["permanent", "or", "player"]
                            ))
                        || tail == ["target"]
                });
            if !same {
                return Ok(None);
            }
        }
        filters
    } else {
        if spec.repeated_target_tokens.is_some() {
            return Ok(None);
        }
        (Some(PlayerFilter::Any), Some(ObjectFilter::default()))
    };
    let source_words = parser_token_word_refs(spec.source.filter_tokens);
    let mut source_filter = damage_source_filter_from_shape(spec.source)?;
    // "another creature" is relative to the equipped recipient, not the
    // Equipment that owns the ability. The host is excluded even on a
    // self-damage event and even if the Equipment is itself a creature.
    if source_words.starts_with(&["another"])
        && spec
            .damaged_tokens
            .is_some_and(|t| parser_token_word_refs(t) == ["equipped", "creature"])
    {
        source_filter.other = false;
        source_filter.without_attached_object = Some(Box::new(ObjectFilter::source()));
    }
    // Intrinsic attachment relations do not fall back to unrelated equipped
    // creatures when this Equipment is unattached.
    if source_words == ["equipped", "creature"] {
        source_filter = {
            let mut filter = ObjectFilter::creature();
            filter.with_attached_object = Some(Box::new(ObjectFilter::source()));
            filter
        };
    }
    Ok(Some(ironsmith_core::RegisterDamageMultiplierEffect {
        source_filter,
        target_player_filter,
        target_object_filter,
        factor: spec.factor,
        combat_only: spec.combat_only,
        noncombat_only: spec.noncombat_only,
        mode: ironsmith_core::ReplacementApplyMode::UntilEndOfTurn,
        amount_override: None,
        minimum: None,
    }))
}

fn damage_source_filter_from_shape(
    shape: keyword_static_lines::DamageSourceShape<'_>,
) -> Result<ObjectFilter, CardTextError> {
    // "a red instant or sorcery spell you control or a red planeswalker you
    // control" (Pyromancer's Gauntlet): independently articled sources, each
    // with its own domain, are alternatives.
    let articled_union = if shape.trailing_filter_tokens.is_empty() {
        shape
            .filter_tokens
            .windows(2)
            .position(|window| window[0].is_word("or") && window[1].is_any_word(&["a", "an"]))
            .filter(|&or_idx| or_idx > 0)
            .and_then(|or_idx| {
                let left = parse_object_filter_lexed(&shape.filter_tokens[..or_idx], false).ok()?;
                let right =
                    parse_object_filter_lexed(&shape.filter_tokens[or_idx + 1..], false).ok()?;
                (left.zone != right.zone).then(|| {
                    let mut union = ObjectFilter::default();
                    union.any_of = vec![left, right];
                    union.set_explicit_union_branch_articles(true);
                    union
                })
            })
    } else {
        None
    };
    let source_words = parser_token_word_refs(shape.filter_tokens);
    let trailing_words = parser_token_word_refs(shape.trailing_filter_tokens);
    let mut filter = if !shape.source_noun && is_source_reference_words(&source_words) {
        let mut source = ObjectFilter::source();
        source.source_surface = source_reference_surface_for_words(&source_words);
        source
    } else if !shape.source_noun && matches!(source_words.as_slice(),
        ["creatures", "it's", "blocking"] | ["creatures", "its", "blocking"])
    {
        let mut creatures = ObjectFilter::creature();
        creatures.blocked_by_source = true;
        creatures
    } else if shape.source_noun && shape.filter_tokens.is_empty()
        && trailing_words.as_slice() == ["of", "the", "chosen", "color"]
    {
        let mut sources = ObjectFilter::default();
        sources.chosen_color = true;
        sources
    } else if shape.source_noun && shape.filter_tokens.is_empty()
        && trailing_words.as_slice() == ["with", "the", "chosen", "name"]
    {
        let mut sources = ObjectFilter::default();
        sources.name = Some("{chosen name}".into());
        sources
    } else if let Some(union) = articled_union {
        union
    } else if shape.filter_tokens.is_empty() && shape.trailing_filter_tokens.is_empty() {
        ObjectFilter::default()
    } else if shape.trailing_filter_tokens.is_empty() {
        parse_object_filter_lexed(shape.filter_tokens, false)?
    } else {
        // Post-controller qualifiers belong to the same noun phrase ("a
        // source you control with an odd mana value"). A bare qualifier has
        // no head noun for the filter grammar, so recognize the parity shape
        // directly and fold anything else into a combined parse.
        let trailing_words = crate::lexer::token_word_refs(shape.trailing_filter_tokens);
        let parity = if crate::word_primitives::parse_sequence_complete(
            &trailing_words,
            &["with", "an", "odd", "mana", "value"],
        ) {
            Some(ironsmith_core::ParityRequirement::Odd)
        } else if crate::word_primitives::parse_sequence_complete(
            &trailing_words,
            &["with", "an", "even", "mana", "value"],
        ) {
            Some(ironsmith_core::ParityRequirement::Even)
        } else {
            None
        };
        if let Some(parity) = parity {
            let base = if shape.filter_tokens.is_empty() {
                ObjectFilter::default()
            } else {
                parse_object_filter_lexed(shape.filter_tokens, false)?
            };
            base.with_mana_value_parity(parity)
        } else {
            let mut combined = shape.filter_tokens.to_vec();
            combined.extend(shape.trailing_filter_tokens.iter().cloned());
            parse_object_filter_lexed(&combined, false)?
        }
    };
    // A subtype noun by itself denotes permanents (CR 109.2): "Deserts"
    // is not "Desert cards" or "Desert sources". The shared filter grammar
    // also serves card-selection contexts, so establish this event-source
    // domain here without inventing a card type or changing explicit zones.
    if !shape.source_noun && filter.zone.is_none() && !filter.has_explicit_card_noun()
        && (!filter.subtypes.is_empty() || !filter.all_subtypes.is_empty())
    {
        filter.zone = Some(Zone::Battlefield);
    }
    // "Creature sources" includes creature cards and spells in any zone.
    // Discard only the battlefield default inferred from a type noun, while
    // retaining an authored battlefield/permanent restriction.
    if shape.source_noun && filter.zone == Some(Zone::Battlefield) {
        let mut source_words = parser_token_word_refs(shape.filter_tokens);
        source_words.extend(parser_token_word_refs(shape.trailing_filter_tokens));
        let explicitly_battlefield = source_words
            .iter()
            .any(|word| matches!(*word, "battlefield" | "permanent" | "permanents"));
        if !explicitly_battlefield {
            filter.zone = None;
        }
    }
    match shape.controller {
        keyword_static_lines::DamageSourceControllerKind::None => {}
        keyword_static_lines::DamageSourceControllerKind::You => filter = filter.you_control(),
        keyword_static_lines::DamageSourceControllerKind::NotYou => {
            filter = filter.controlled_by(PlayerFilter::NotYou);
        }
        keyword_static_lines::DamageSourceControllerKind::Opponent => {
            filter = filter.controlled_by(PlayerFilter::Opponent);
        }
    }
    Ok(filter)
}

fn parse_damage_amount_replacement_target_filters(
    words: &[&str],
) -> Result<(Option<PlayerFilter>, Option<ObjectFilter>), CardTextError> {
    let simple = strip_leading_word_refs_any(words, &["a", "an"]);
    if matches!(
        simple,
        ["player", "or", "battle"] | ["player", "or", "a", "battle"]
    ) {
        return Ok((
            Some(PlayerFilter::Any),
            Some(ObjectFilter::permanent().with_type(CardType::Battle)),
        ));
    }
    let object = match simple {
        ["creature"] => Some(ObjectFilter::creature()),
        ["this", "creature"] | ["this", "permanent"] => Some(ObjectFilter::source()),
        ["equipped", "creature"] | ["enchanted", "creature"] => Some({
            let mut filter = ObjectFilter::creature();
            filter.with_attached_object = Some(Box::new(ObjectFilter::source()));
            filter
        }),
        ["enchanted", "permanent"] => Some({
            let mut filter = ObjectFilter::permanent();
            filter.with_attached_object = Some(Box::new(ObjectFilter::source()));
            filter
        }),
        _ => None,
    };
    if let Some(object) = object {
        return Ok((None, Some(object)));
    }
    if matches!(
        simple,
        [
            "the",
            "chosen",
            "player",
            "or",
            "a",
            "permanent",
            "they",
            "control"
        ] | [
            "the",
            "chosen",
            "player",
            "or",
            "permanent",
            "they",
            "control"
        ]
    ) {
        return Ok((
            Some(PlayerFilter::ChosenPlayer),
            Some(ObjectFilter::permanent().controlled_by(PlayerFilter::ChosenPlayer)),
        ));
    }
    #[derive(Clone, Copy)]
    enum DamageReplacementTargetKind {
        You,
        Opponent,
        AnyPlayer,
        EnchantedPlayer,
        Permanent,
        PermanentOrPlayer,
        OpponentOrPermanentOpponentControls,
        YouOrPermanentYouControl,
        CreatureBattleOrOpponent,
    }

    const DAMAGE_REPLACEMENT_TARGET_PHRASES: &[(&[&str], DamageReplacementTargetKind)] = &[
        (&["you"], DamageReplacementTargetKind::You),
        (&["opponent"], DamageReplacementTargetKind::Opponent),
        (&["player"], DamageReplacementTargetKind::AnyPlayer),
        (
            &["enchanted", "player"],
            DamageReplacementTargetKind::EnchantedPlayer,
        ),
        (&["permanent"], DamageReplacementTargetKind::Permanent),
        (
            &["permanent", "or", "player"],
            DamageReplacementTargetKind::PermanentOrPlayer,
        ),
        (
            &["permanent", "or", "a", "player"],
            DamageReplacementTargetKind::PermanentOrPlayer,
        ),
        (
            &["player", "or", "permanent"],
            DamageReplacementTargetKind::PermanentOrPlayer,
        ),
        (
            &["player", "or", "a", "permanent"],
            DamageReplacementTargetKind::PermanentOrPlayer,
        ),
        (
            &[
                "opponent",
                "or",
                "a",
                "permanent",
                "an",
                "opponent",
                "controls",
            ],
            DamageReplacementTargetKind::OpponentOrPermanentOpponentControls,
        ),
        (
            &["opponent", "or", "permanent", "an", "opponent", "controls"],
            DamageReplacementTargetKind::OpponentOrPermanentOpponentControls,
        ),
        (
            &["you", "or", "a", "permanent", "you", "control"],
            DamageReplacementTargetKind::YouOrPermanentYouControl,
        ),
        (
            &["you", "or", "permanent", "you", "control"],
            DamageReplacementTargetKind::YouOrPermanentYouControl,
        ),
        // "a creature, battle, or opponent" (Disciples of the Inferno).
        (
            &["creature", "battle", "or", "opponent"],
            DamageReplacementTargetKind::CreatureBattleOrOpponent,
        ),
    ];

    let words = strip_leading_word_refs_any(words, &["a", "an"]);
    let Some((_, kind)) = DAMAGE_REPLACEMENT_TARGET_PHRASES
        .iter()
        .find(|(phrase, _)| *phrase == words)
    else {
        return Ok((None, None));
    };

    match kind {
        DamageReplacementTargetKind::You => Ok((Some(PlayerFilter::You), None)),
        DamageReplacementTargetKind::Opponent => Ok((Some(PlayerFilter::Opponent), None)),
        DamageReplacementTargetKind::AnyPlayer => Ok((Some(PlayerFilter::Any), None)),
        DamageReplacementTargetKind::EnchantedPlayer => Ok((
            Some(PlayerFilter::TaggedPlayer(
                (crate::tag::CompilerReferenceTag::Enchanted.bind()).into(),
            )),
            None,
        )),
        DamageReplacementTargetKind::Permanent => Ok((None, Some(ObjectFilter::permanent()))),
        DamageReplacementTargetKind::PermanentOrPlayer => {
            Ok((Some(PlayerFilter::Any), Some(ObjectFilter::permanent())))
        }
        DamageReplacementTargetKind::OpponentOrPermanentOpponentControls => Ok((
            Some(PlayerFilter::Opponent),
            Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
        )),
        DamageReplacementTargetKind::YouOrPermanentYouControl => Ok((
            Some(PlayerFilter::You),
            Some(ObjectFilter::permanent().you_control()),
        )),
        DamageReplacementTargetKind::CreatureBattleOrOpponent => Ok((
            Some(PlayerFilter::Opponent),
            Some(ObjectFilter::creature().with_type(CardType::Battle)),
        )),
    }
}

pub fn parse_minimum_damage_amount_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if !keyword_static_lines::parse_minimum_red_noncombat_damage_tokens(&tokens) {
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

pub fn parse_enter_as_copy_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    fn parse_added_copy_abilities(
        ability_tokens: &[OwnedLexToken],
        clause_words: &[&str],
    ) -> Result<Vec<crate::model::compiler_semantic::CompilerAbilityCore>, CardTextError> {
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported empty enters-as-copy ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (abilities, _choice) =
            parse_granted_abilities_for_gain_clause(ability_tokens, clause_words, false)?;
        if abilities.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported enters-as-copy ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        super::static_ability_helpers::compiler_granted_abilities_ast_to_object_abilities(
            &abilities,
        )
    }

    if crate::lexer::parser_token_word_refs(tokens)
        .starts_with(&["if", "you", "attacked", "this", "turn"])
        && let Some(comma) = tokens.iter().position(OwnedLexToken::is_comma)
        && let Some(ability) = parse_enter_as_copy_as_enters_line(&tokens[comma + 1..])?
    {
        return Ok(Some(ability.with_condition(parse_static_condition_clause(
            &tokens[1..comma],
        )?)));
    }
    let full_display = render_token_slice(tokens).trim().to_string();
    fn strip_followup(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
        keyword_static_lines::split_enter_as_copy_followup_tokens(tokens)
            .map_or(tokens, |(core, _)| core)
    }
    // "Mind Swap — You may have ... enter as a copy ...": the ability word is
    // presentation; the replacement grammar reads the body.
    let (tokens, labeled) =
        match crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(tokens) {
            Some(label)
                if keyword_static_lines::parse_enter_as_copy_tokens(strip_followup(
                    label.body_tokens,
                ))
                .is_some() =>
            {
                (label.body_tokens, true)
            }
            _ => (tokens, false),
        };
    // "When you do, exile that card." / "If you do, it gains haste until end
    // of turn.": the rest of the copy choice, carried on the replacement.
    let (tokens, copy_followup) =
        match keyword_static_lines::split_enter_as_copy_followup_tokens(tokens) {
            Some((core, followup)) => (core, Some(followup)),
            None => (tokens, None),
        };
    let Some(shape) = keyword_static_lines::parse_enter_as_copy_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);
    // An unrecognized result follow-up after an optional copy must not be
    // dropped silently (a quoted granted ability may carry its own "If you
    // do", and the linked-exile wording leads with its own "If you do").
    if copy_followup.is_none()
        && matches!(shape, keyword_static_lines::EnterAsCopyShape::May { .. })
        && !tokens.iter().any(|token| token.kind == TokenKind::Quote)
        && keyword_static_lines::has_enter_as_copy_result_followup_sentence(tokens)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy follow-up sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if copy_followup.is_some()
        && !matches!(shape, keyword_static_lines::EnterAsCopyShape::May { .. })
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy follow-up for a mandatory copy (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let copy_followups: Vec<ironsmith_core::EnterAsCopyFollowup> =
        copy_followup.into_iter().collect();
    let display = render_token_slice(tokens).trim().to_string();

    match shape {
        keyword_static_lines::EnterAsCopyShape::LinkedExilePair(shape) => {
            if shape.exile_count != 2
                || shape.copy_count != 1
                || shape.counter_value
                    != keyword_static_lines::LinkedExileCopyCounterValue::OtherCardPower
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported linked-exile enters-as-copy configuration: exile {}, copy {}, counter value {:?}",
                    shape.exile_count, shape.copy_count, shape.counter_value
                )));
            }
            Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
                crate::model::CompilerEnterAsCopyAsEntersSpecCore {
                    filter: shape.filter,
                    affected_filter: None,
                    may: shape.may,
                    enters_tapped_if_chosen: false,
                    copy_duration: None,
                    linked_exile_pair: Some(
                        crate::static_abilities::EnterAsCopyLinkedExilePairSpec {
                            counter_type: shape.counter_type,
                        },
                    ),
                    copy_source_self: false,
                    copy_source_enchanted: false,
                    name_override: None,
                    added_colors: ColorSet::new(),
                    added_card_types: Vec::new(),
                    removes_other_card_types: false,
                    added_supertypes: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                    set_base_power_toughness: None,
                    additional_counters: Vec::new(),
                    additional_x_counters: Vec::new(),
                    keep_other_source_abilities: false,
                    additional_counters_source_filter: None,
                    added_abilities_source_filter: None,
                    set_base_power_toughness_from_self: false,
                    copy_followups: Vec::new(),
                    conditional_additional_counters: Vec::new(),
                },
                render_token_slice(tokens).trim().to_string(),
            )))
        }
        keyword_static_lines::EnterAsCopyShape::Direct {
            affected_tokens,
            copy_source_tokens,
            copy_source_kind,
        } => {
            let affected_filter = parse_object_filter(affected_tokens, false)?;
            let (filter, copy_source_self, copy_source_enchanted) = match copy_source_kind {
                keyword_static_lines::CopySourceKind::Source => {
                    (ObjectFilter::source(), true, false)
                }
                keyword_static_lines::CopySourceKind::Enchanted => {
                    (ObjectFilter::source(), false, true)
                }
                keyword_static_lines::CopySourceKind::Filter => (
                    parse_object_filter(copy_source_tokens, false)?,
                    false,
                    false,
                ),
            };
            Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
                crate::model::CompilerEnterAsCopyAsEntersSpecCore {
                    filter,
                    affected_filter: Some(affected_filter),
                    may: false,
                    enters_tapped_if_chosen: false,
                    copy_duration: None,
                    linked_exile_pair: None,
                    copy_source_self,
                    copy_source_enchanted,
                    name_override: None,
                    added_colors: ColorSet::new(),
                    added_card_types: Vec::new(),
                    removes_other_card_types: false,
                    added_supertypes: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                    set_base_power_toughness: None,
                    additional_counters: Vec::new(),
                    additional_x_counters: Vec::new(),
                    keep_other_source_abilities: false,
                    additional_counters_source_filter: None,
                    added_abilities_source_filter: None,
                    set_base_power_toughness_from_self: false,
                    copy_followups: Vec::new(),
                    conditional_additional_counters: Vec::new(),
                },
                display,
            )))
        }
        keyword_static_lines::EnterAsCopyShape::May {
            named_subject_tokens,
            enters_tapped,
            until_end_of_turn,
            filter_tokens,
            exception_display_split,
            exception_tokens,
        } => {
            let display = if labeled || !copy_followups.is_empty() {
                full_display.clone()
            } else if let Some(split) = exception_display_split {
                format!(
                    "{} {}",
                    render_token_slice(split.before_separator).trim(),
                    render_token_slice(split.after_separator).trim()
                )
            } else {
                display
            };
            let filter = parse_object_filter(filter_tokens, false)?;
            let named_copy_subject = named_subject_tokens.map(|subject_tokens| {
                parser_token_word_refs(subject_tokens)
                    .into_iter()
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
                    .join(" ")
            });

            let mut name_override = None;
            let mut added_colors = ColorSet::new();
            let mut added_card_types = Vec::new();
            let mut removes_other_card_types = false;
            let mut added_supertypes = Vec::new();
            let mut removed_supertypes = Vec::new();
            let mut added_subtypes = Vec::new();
            let mut added_abilities = Vec::new();
            let mut additional_counters = Vec::new();
            let mut additional_counters_source_filter = None;
            let mut additional_x_counters = Vec::new();
            let mut keep_other_source_abilities = false;
            let mut conditional_additional_counters: Vec<
                ironsmith_core::ConditionalAdditionalCounters,
            > = Vec::new();
            let mut added_abilities_source_filter = None;
            let mut set_base_power_toughness = None;
            let mut set_base_power_toughness_from_self = false;

            if let Some(exception_tokens) = exception_tokens {
                let exception = keyword_static_lines::parse_copy_exception_tokens(exception_tokens)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported enters-as-copy exception clause (clause: '{}')",
                            clause_words.join(" ")
                        ))
                    })?;
                // "except his name is X and he's a 4/4 Spider Human Hero in
                // addition to his other types": a name exception followed by
                // a characteristic exception in one clause.
                let mut exceptions = vec![exception];
                if let keyword_static_lines::CopyExceptionShape::Name { remainder_tokens, .. } = &exceptions[0] {
                    let remainder = trim_lexed_commas(*remainder_tokens);
                    if !remainder.is_empty() {
                        let remainder = if remainder[0].is_word("and") {
                            trim_lexed_commas(&remainder[1..])
                        } else {
                            return Err(CardTextError::ParseError(
                                "unsupported complete enters-as-copy name exception tail".into(),
                            ));
                        };
                        // A name exception must not discard a second exception.
                        // Only the already supported complete characteristic
                        // suffix may accompany it; other compound bodies stay
                        // unsupported instead of merely retaining their prose.
                        let Some(characteristics @ keyword_static_lines::CopyExceptionShape::Characteristics { .. }) =
                            keyword_static_lines::parse_copy_exception_tokens(remainder)
                        else {
                            return Err(CardTextError::ParseError(
                                "unsupported complete enters-as-copy name exception tail".into(),
                            ));
                        };
                        exceptions.push(characteristics);
                    }
                }
                for exception in exceptions {
                    match exception {
                        keyword_static_lines::CopyExceptionShape::OwnOtherAbilities { subject } => {
                            let words = parser_token_word_refs(subject);
                            let normalized = crate::util::possessive_normalized_word_refs(&words);
                            let same_named_source = named_subject_tokens.is_some_and(|name| {
                                let name = parser_token_word_refs(name);
                                if normalized == name {
                                    return true;
                                }
                                // The canonical lexer spelling can remove the
                                // apostrophe while retaining its possessive s.
                                let mut possessive = name
                                    .iter()
                                    .map(|word| (*word).to_string())
                                    .collect::<Vec<_>>();
                                if let Some(last) = possessive.last_mut() {
                                    last.push('s');
                                }
                                words == possessive.iter().map(String::as_str).collect::<Vec<_>>()
                            });
                            if !same_named_source
                                && crate::util::source_reference_surface_for_possessive_words(
                                    &words,
                                )
                                .is_none()
                            {
                                return Err(CardTextError::ParseError(
                                    "copy exception names abilities of an unbound source".into(),
                                ));
                            }
                            keep_other_source_abilities = true;
                        }
                        keyword_static_lines::CopyExceptionShape::EntryCounters {
                            counter_type,
                            count,
                            controlled_copy,
                        } => {
                            match count {
                                Some(count) => additional_counters.push((counter_type, count)),
                                None => additional_x_counters.push(counter_type),
                            }
                            if controlled_copy {
                                additional_counters_source_filter =
                                    Some(ObjectFilter::default().you_control());
                            }
                        }
                        keyword_static_lines::CopyExceptionShape::ConditionalCounters {
                            entries,
                            remove_legendary,
                        } => {
                            if remove_legendary {
                                removed_supertypes.push(crate::types::Supertype::Legendary);
                            }
                            for entry in entries {
                                conditional_additional_counters.push(
                                    ironsmith_core::ConditionalAdditionalCounters {
                                        counter_type: entry.counter_type,
                                        count: entry.count,
                                        source_filter: ObjectFilter::default()
                                            .with_type(entry.card_type),
                                    },
                                );
                            }
                        }
                        keyword_static_lines::CopyExceptionShape::Name {
                            name_tokens,
                            use_named_subject,
                            ..
                        } => {
                            if use_named_subject {
                                name_override = named_copy_subject.clone();
                            } else {
                                // The copy's name is the authored spelling
                                // ("Superior Spider-Man", "Chameleon, Master of
                                // Disguise"), not the lowercased parser words.
                                let mut name = String::new();
                                for token in name_tokens {
                                    if token.kind == TokenKind::Comma {
                                        name.push(',');
                                        continue;
                                    }
                                    if !name.is_empty() {
                                        name.push(' ');
                                    }
                                    name.push_str(token.literal_surface());
                                }
                                if !name.trim().is_empty() {
                                    name_override = Some(name.trim().to_string());
                                }
                            }
                        }
                        keyword_static_lines::CopyExceptionShape::Abilities {
                            ability_tokens,
                            source_filter_tokens,
                        } => {
                            added_abilities_source_filter =
                                if let Some(tokens) = source_filter_tokens {
                                    let (subject, missing) =
                                keyword_static_lines::split_copy_source_missing_ability_tokens(
                                    tokens,
                                )
                                .ok_or_else(|| {
                                    CardTextError::ParseError(
                                        "unsupported conditional copy ability predicate".into(),
                                    )
                                })?;
                                    let actions = parse_ability_line(missing).ok_or_else(|| {
                                        CardTextError::ParseError(
                                            "unsupported copy source keyword predicate".into(),
                                        )
                                    })?;
                                    let [action] = actions.as_slice() else {
                                        return Err(CardTextError::ParseError(
                                            "copy source predicate requires one keyword".into(),
                                        ));
                                    };
                                    let mut filter = parse_object_filter(subject, false)?;
                                    let marker = match action {
                                        KeywordAction::Vanishing(_) => "vanishing".to_string(),
                                        other => other.display_text().to_ascii_lowercase(),
                                    };
                                    filter.excluded_ability_markers.push(marker);
                                    Some(filter)
                                } else {
                                    None
                                };
                            added_abilities =
                                parse_added_copy_abilities(ability_tokens, &clause_words)?;
                        }
                        keyword_static_lines::CopyExceptionShape::Characteristics {
                            remove_legendary,
                            characteristic_tokens,
                            remainder,
                        } => {
                            if remove_legendary {
                                removed_supertypes.push(crate::types::Supertype::Legendary);
                            }
                            let characteristic_words =
                                parser_token_word_refs(characteristic_tokens);
                            if characteristic_words.is_empty() {
                                return Err(CardTextError::ParseError(format!(
                                    "unsupported enters-as-copy exception clause (clause: '{}')",
                                    clause_words.join(" ")
                                )));
                            }
                            let mut cursor = 0usize;
                            if let Ok((power, toughness)) =
                                parse_pt_modifier(characteristic_words[cursor])
                            {
                                set_base_power_toughness = Some((power, toughness));
                                cursor += 1;
                            }
                            let mut parsed_type_or_subtype = false;
                            while cursor < characteristic_words.len() {
                                if characteristic_words[cursor] == "and" {
                                    cursor += 1;
                                    continue;
                                }
                                if let Some(supertype) =
                                    crate::util::parse_supertype_word(characteristic_words[cursor])
                                {
                                    crate::slice_primitives::push_unique(
                                        &mut added_supertypes,
                                        supertype,
                                    );
                                    parsed_type_or_subtype = true;
                                    cursor += 1;
                                    continue;
                                }
                                if let Some(color) = Color::from_name(characteristic_words[cursor])
                                {
                                    added_colors = added_colors.with(color);
                                    parsed_type_or_subtype = true;
                                    cursor += 1;
                                    continue;
                                }
                                if let Some(card_type) =
                                    parse_card_type(characteristic_words[cursor])
                                {
                                    crate::slice_primitives::push_unique(
                                        &mut added_card_types,
                                        card_type,
                                    );
                                    parsed_type_or_subtype = true;
                                    cursor += 1;
                                    continue;
                                }
                                if let Some(subtype) =
                                    parse_subtype_word(characteristic_words[cursor]).or_else(|| {
                                        parse_subtype_flexible(characteristic_words[cursor])
                                    })
                                {
                                    crate::slice_primitives::push_unique(
                                        &mut added_subtypes,
                                        subtype,
                                    );
                                    parsed_type_or_subtype = true;
                                    cursor += 1;
                                    continue;
                                }
                                break;
                            }
                            if (!parsed_type_or_subtype && set_base_power_toughness.is_none())
                                || cursor != characteristic_words.len()
                            {
                                return Err(CardTextError::ParseError(format!(
                                    "unsupported enters-as-copy type '{}' (clause: '{}')",
                                    characteristic_words
                                        [cursor.min(characteristic_words.len().saturating_sub(1))],
                                    clause_words.join(" ")
                                )));
                            }
                            match remainder {
                            keyword_static_lines::CopyCharacteristicRemainder::None => {}
                            keyword_static_lines::CopyCharacteristicRemainder::ConditionalEntry(entry) => {
                                let words = parser_token_word_refs(entry);
                                let parsed = (|| {
                                    let ["its" | "it's", "a" | "an", type_word, "it", "enters", "with", count, "additional", counter, "counters", "on", "it", "and", "has", ..] = words.as_slice() else { return None; };
                                    let card_type = parse_card_type(type_word)?;
                                    let count = u32::try_from(parse_number_word_i32(count)?).ok()?;
                                    let counter = parse_counter_type_word(counter)?;
                                    Some((card_type, count, counter))
                                })().ok_or_else(|| CardTextError::ParseError("unsupported conditional copy entry modification".into()))?;
                                let (card_type, count, counter) = parsed;
                                let filter = ObjectFilter::default().with_type(card_type);
                                additional_counters = vec![(counter, count)];
                                additional_counters_source_filter = Some(filter.clone());
                                added_abilities_source_filter = Some(filter);
                                let has_index = entry.iter().position(|token| token.is_word("has")).ok_or_else(|| CardTextError::ParseError("missing conditional copy ability".into()))?;
                                added_abilities = parse_added_copy_abilities(&entry[has_index + 1..], &clause_words)?;
                            }
                            keyword_static_lines::CopyCharacteristicRemainder::PowerToughnessFromSource => {
                                set_base_power_toughness_from_self = true;
                            }
                            keyword_static_lines::CopyCharacteristicRemainder::Abilities(
                                ability_tokens,
                            ) => {
                                added_abilities =
                                    parse_added_copy_abilities(ability_tokens, &clause_words)?;
                            }
                            keyword_static_lines::CopyCharacteristicRemainder::WithAbilities {
                                abilities,
                                loses_other_card_types,
                            } => {
                                added_abilities =
                                    parse_added_copy_abilities(abilities, &clause_words)?;
                                removes_other_card_types = loses_other_card_types;
                            }
                            keyword_static_lines::CopyCharacteristicRemainder::LosesOtherCardTypes => {
                                removes_other_card_types = true;
                            }
                            keyword_static_lines::CopyCharacteristicRemainder::Unsupported => {
                                return Err(CardTextError::ParseError(format!(
                                    "unsupported enters-as-copy exception clause (clause: '{}')",
                                    clause_words.join(" ")
                                )));
                            }
                        }
                        }
                    }
                }
            }

            Ok(Some(StaticAbility::with_enter_as_copy_as_enters(
                crate::model::CompilerEnterAsCopyAsEntersSpecCore {
                    filter,
                    affected_filter: None,
                    may: true,
                    enters_tapped_if_chosen: enters_tapped,
                    copy_duration: until_end_of_turn.then_some(crate::effect::Until::EndOfTurn),
                    linked_exile_pair: None,
                    copy_source_self: false,
                    copy_source_enchanted: false,
                    name_override,
                    added_colors,
                    added_card_types,
                    removes_other_card_types,
                    added_subtypes,
                    added_abilities,
                    set_base_power_toughness,
                    additional_counters,
                    additional_counters_source_filter,
                    additional_x_counters,
                    keep_other_source_abilities,
                    added_abilities_source_filter,
                    set_base_power_toughness_from_self,
                    copy_followups: copy_followups.clone(),
                    conditional_additional_counters,
                    added_supertypes,
                    removed_supertypes,
                },
                display,
            )))
        }
    }
}

pub fn parse_choose_color_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    let Some((consumed, excluded_color_set)) = parse_choose_color_phrase_words(&tail_words)? else {
        return Ok(None);
    };
    if consumed != tail_words.len() {
        return Ok(None);
    }

    let excluded = if let Some(color_set) = excluded_color_set {
        Some(color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice in choose-color clause (clause: '{}')",
                render_token_slice(tokens)
            ))
        })?)
    } else {
        None
    };
    let display = match excluded {
        Some(color) => format!(
            "As {display_subject} enters, choose a color other than {}.",
            color.name()
        ),
        None => format!("As {display_subject} enters, choose a color."),
    };

    Ok(Some(StaticAbility::choose_color_as_enters(
        excluded, display,
    )))
}

#[cfg(test)]
mod enter_as_copy_added_color_tests {
    use super::*;

    #[test]
    fn copy_name_exception_never_drops_unmodeled_compound_tails() {
        for tail in [
            "and it has flying",
            "and it can't block",
            ", it's legendary in addition to its other types",
        ] {
            let text = format!("You may have Mirror enter as a copy of any creature on the battlefield, except its name is Mirror {tail}.");
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            assert!(parse_enter_as_copy_as_enters_line(&tokens).is_err(), "{text}");
        }
        let text = "You may have Mirror enter as a copy of any creature on the battlefield, except its name is Mirror and it's a 4/4 Spider Human Hero in addition to its other types.";
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let ability = parse_enter_as_copy_as_enters_line(&tokens).unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::EnterAsCopyAsEnters { spec, .. } = ability.payload else {
            panic!("copy model");
        };
        assert_eq!(spec.name_override.as_deref(), Some("Mirror"));
        assert_eq!(spec.set_base_power_toughness, Some((4, 4)));
        assert_eq!(spec.added_subtypes, [Subtype::Spider, Subtype::Human, Subtype::Hero]);
    }

    #[test]
    fn copy_exception_adds_a_color_alongside_types_subtypes_and_fixed_pt() {
        let text = "You may have this creature enter as a copy of any creature card in a graveyard except it's a 4/4 black Zombie in addition to its other colors and types.";
        let tokens = crate::lexer::lex_line(text, 0).expect("copy exception should lex");
        let ability = parse_enter_as_copy_as_enters_line(&tokens)
            .expect("copy exception should parse")
            .expect("copy exception should be claimed");
        let ironsmith_core::StaticAbilityPayload::EnterAsCopyAsEnters { spec, .. } =
            ability.payload
        else {
            panic!("expected a typed enter-as-copy payload")
        };

        assert_eq!(spec.added_colors, ColorSet::BLACK);
        assert_eq!(spec.set_base_power_toughness, Some((4, 4)));
        assert_eq!(spec.added_subtypes, [Subtype::Zombie]);
        assert!(spec.added_card_types.is_empty());
    }
}

pub fn parse_choose_color_as_becomes_attached_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = static_mid_facts::parse_attached_color_choice_fact(tokens) else {
        return Ok(None);
    };
    // The grammar fact has already proved the source kind, the attachment
    // relation, and the complete color-choice tail. Preserve that typed
    // presentation so an authored attachment recipient such as `to a
    // creature` is not erased, while restoring subtype capitalization that
    // the token renderer intentionally normalizes away.
    let (parsed_subject, display_subject) = match fact.subject {
        static_mid_facts::AttachedChoiceSubject::Equipment => ("this equipment", "this Equipment"),
        static_mid_facts::AttachedChoiceSubject::Aura => ("this aura", "this Aura"),
        static_mid_facts::AttachedChoiceSubject::Permanent => ("this permanent", "this permanent"),
        static_mid_facts::AttachedChoiceSubject::Artifact => ("this artifact", "this artifact"),
        static_mid_facts::AttachedChoiceSubject::Enchantment => {
            ("this enchantment", "this enchantment")
        }
    };
    let word_refs = LexedClause::new(fact.choice_tokens).word_refs();
    let Some((consumed, excluded_color_set)) = parse_choose_color_phrase_words(&word_refs)? else {
        return Ok(None);
    };
    if consumed != word_refs.len() {
        return Ok(None);
    }
    let excluded = if let Some(color_set) = excluded_color_set {
        Some(color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice in choose-color attached clause (clause: '{}')",
                render_token_slice(tokens)
            ))
        })?)
    } else {
        None
    };
    if excluded.is_some() {
        return Err(CardTextError::ParseError(format!(
            "excluded color choices for attachment choices are not supported yet (clause: '{}')",
            render_token_slice(tokens)
        )));
    }

    let display = render_token_slice(tokens).replacen(parsed_subject, display_subject, 1);
    Ok(Some(StaticAbility::choose_color_as_becomes_attached(
        display,
    )))
}

pub fn parse_choose_player_as_enters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((tail_tokens, display_subject)) =
        parse_as_enters_choice_subject_tokens(tokens, AS_ENTERS_STANDARD_SUBJECTS_WITH_AURA)
    else {
        return Ok(None);
    };
    let tail_words = LexedClause::new(tail_tokens).word_refs();
    let (filter, choice_surface) =
        if crate::word_primitives::parse_sequence_complete(&tail_words, &["choose", "a", "player"])
        {
            (PlayerFilter::Any, "a player")
        } else if crate::word_primitives::parse_sequence_complete(
            &tail_words,
            &["choose", "an", "opponent"],
        ) {
            (PlayerFilter::Opponent, "an opponent")
        } else {
            return Ok(None);
        };

    Ok(Some(StaticAbility::choose_player_as_enters_matching(
        filter,
        format!("As {display_subject} enters, choose {choice_surface}."),
    )))
}

pub fn parse_damage_redirect_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if static_mid_facts::parse_redirect_damage_to_source_fact(tokens).is_none() {
        return Ok(None);
    }
    Ok(Some(
        StaticAbility::redirect_damage_from_you_and_other_permanents_to_source(),
    ))
}

pub fn parse_damage_redirect_to_source_controller_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_damage_redirect_controller_tokens(tokens) else {
        return Ok(None);
    };
    let source_filter = parse_object_filter_lexed(spec.source_tokens, false)?;
    let mut display = render_token_slice(tokens).trim().to_string();
    if !crate::string_primitives::ends_with_char(&display, '.') {
        display.push('.');
    }

    Ok(Some(StaticAbility::redirect_damage_to_source_controller(
        source_filter,
        PlayerFilter::You,
        display,
    )))
}

pub fn parse_no_more_than_creatures_can_attack_or_block_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if tokens.len() < 8 {
        return Ok(None);
    }

    let Some((maximum, used)) =
        crate::grammar::primitives::probe_shape(parse_less_than_or_equal_quantity_prefix(
            tokens,
            false,
            false,
            "combat maximum restriction",
        ))
        .flatten()
    else {
        return Ok(None);
    };

    let tail = &tokens[used..];
    let Some(kind) = keyword_static_lines::parse_combat_maximum_tail_tokens(tail) else {
        return Ok(None);
    };
    let ability = match kind {
        keyword_static_lines::CombatMaximumKind::AttackYou => {
            StaticAbility::max_attackers_can_attack_you_each_combat(maximum as usize)
        }
        keyword_static_lines::CombatMaximumKind::Attack => {
            StaticAbility::max_attackers_each_combat(maximum as usize)
        }
        keyword_static_lines::CombatMaximumKind::Block => {
            StaticAbility::max_blockers_each_combat(maximum as usize)
        }
        keyword_static_lines::CombatMaximumKind::AttackThis => {
            StaticAbility::max_attackers_can_attack_source_each_combat(maximum as usize)
        }
    };
    Ok(Some(ability))
}

pub(crate) fn characteristic_pt_uses_named_subject_surface(tokens: &[OwnedLexToken]) -> bool {
    let sentence_tokens = trim_edge_punctuation(tokens);
    let sentence_words = parser_token_word_refs(&sentence_tokens);
    sentence_words
        .iter()
        .enumerate()
        .filter(|(_, word)| is_characteristic_axis_word(word))
        .any(|(axis_idx, _)| {
            if axis_idx == 0
                || !sentence_words
                    .get(axis_idx + 1)
                    .is_some_and(|word| matches!(*word, "is" | "and"))
            {
                return false;
            }
            // Static-line registry rules do not receive the surrounding card
            // parse context, so a full nonlegendary name such as "Plague
            // Rats's" cannot be validated through the usual source-alias
            // table here. This grammar slot has already proved that the
            // possessive phrase is the subject of a characteristic-defining
            // axis. Preserve that source-name surface only when the authored
            // tokens form a proper name, rejecting ordinary subjects such as
            // "equipped creature's" even though the possessive suffix can
            // make that phrase look identifier-like after normalization.
            static_keyword_shapes::parse_word_token_offset(&sentence_tokens, axis_idx).is_some_and(
                |subject_end| {
                    let subject_tokens = &sentence_tokens[..subject_end];
                    crate::lexer::is_authored_proper_name_phrase(subject_tokens)
                },
            )
        })
}

pub fn parse_characteristic_defining_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // Conditional characteristic assignments must retain their predicate;
    // the enclosing static parser owns the prefix and wraps the body.
    if split_as_long_as_condition_prefix_lexed(tokens).is_some()
        || anthem_grant_grammar::parse_fixed_prefix_condition_shape(tokens).is_some()
    {
        return Ok(None);
    }
    let sentence_tokens = trim_edge_punctuation(tokens);
    if split_lexed_sentences(&sentence_tokens).len() > 1 {
        return Ok(None);
    }
    // "<objects> have base power and toughness each equal to ..." sets the
    // base P/T of other objects (layer 7b); it is not a CDA of the source.
    if anthem_grant_grammar::parse_base_power_toughness_each_equal_shape(&sentence_tokens).is_some()
    {
        return Ok(None);
    }
    let sentence_words = parser_token_word_refs(&sentence_tokens);
    let uses_source_name_subject = characteristic_pt_uses_named_subject_surface(&sentence_tokens);
    let preserve_subject = |value: Value| {
        if uses_source_name_subject {
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject)
        } else {
            value
        }
    };
    if sentence_words
        .iter()
        .any(|word| matches!(*word, "target" | "targets" | "become" | "becomes" | "until"))
    {
        return Ok(None);
    }

    // "This token's power and toughness are each equal to the number of fade
    // counters on Saproling Burst" reaches this line parser when a token's
    // characteristic-defining ability is authored as its own sentence. The
    // proper name is the creating permanent, so it must stay a named source
    // reference; the generic value reading would otherwise take the leading
    // word of the name as a creature type and count every such permanent.
    if let Some((power, toughness)) =
        crate::grammar::token_definitions::parse_named_source_counter_dynamic_power_toughness_tokens(
            &sentence_tokens,
        )
    {
        return Ok(Some(StaticAbility::characteristic_defining_pt(
            power, toughness,
        )));
    }

    if let Some(tail_tokens) =
        keyword_static_lines::parse_characteristic_shared_value_tail_tokens(tokens)
        && !tail_tokens.is_empty()
    {
        let value = parse_characteristic_defining_stat_value(tail_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported characteristic defining P/T value (value: '{}')",
                crate::lexer::token_word_refs(tail_tokens).join(" ")
            ))
        })?;
        let value = preserve_subject(value);
        return Ok(Some(StaticAbility::characteristic_defining_pt(
            value.clone(),
            value,
        )));
    }

    let line_words = LexedClause::new(tokens).words();
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
            if line_words.get(and_idx) != Some("and") {
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

        let Some(value_start_token_idx) =
            static_keyword_shapes::parse_word_token_offset(tokens, value_start_word_idx)
        else {
            break;
        };
        let value_end_token_idx = if value_end_word_idx < line_words.len() {
            static_keyword_shapes::parse_word_token_offset(tokens, value_end_word_idx)
                .unwrap_or(tokens.len())
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

        // "that number" back-references the previous axis; resolve it before
        // the shared value grammar reads it as a chosen-number metric.
        let value = previous_value
            .as_ref()
            .and_then(|base| parse_characteristic_defining_relative_value(&value_tokens, base))
            .or_else(|| parse_characteristic_defining_stat_value(&value_tokens))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported characteristic defining {} value (value: '{}')",
                    axis,
                    crate::lexer::token_word_refs(&value_tokens).join(" ")
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
        preserve_subject(parsed_power.unwrap_or(Value::SourcePower)),
        preserve_subject(parsed_toughness.unwrap_or(Value::SourceToughness)),
    )))
}

fn parse_characteristic_defining_relative_value(
    tokens: &[OwnedLexToken],
    base: &Value,
) -> Option<Value> {
    let words = crate::lexer::token_word_refs(tokens);
    if words.len() == 4 && words[1..] == ["minus", "that", "number"] {
        if let Some((constant, used)) = crate::util::parse_number(tokens) {
            if used == 1 {
                return Some(Value::Add(Box::new(Value::Fixed(i32::try_from(constant).ok()?)),
                    Box::new(Value::Scaled(Box::new(base.clone()), -1))));
            }
        }
    }
    match keyword_static_lines::parse_characteristic_relative_value_tokens(tokens)? {
        keyword_static_lines::CharacteristicRelativeValue::Same => Some(base.clone()),
        keyword_static_lines::CharacteristicRelativeValue::Plus(amount) => Some(Value::Add(
            Box::new(base.clone()),
            Box::new(Value::Fixed(amount as i32)),
        )),
    }
}

fn parse_characteristic_axis_clause_start<'a>(
    words: &TokenWordView<'a>,
    idx: usize,
) -> Option<(&'a str, usize)> {
    let first = words.get(idx)?;
    if is_characteristic_axis_word(first) && characteristic_equal_to_at(words, idx + 1) {
        return Some((first, idx + 4));
    }

    if first == "creature"
        && words.get(idx + 1).is_some_and(is_characteristic_axis_word)
        && characteristic_equal_to_at(words, idx + 2)
    {
        return Some((words.get(idx + 1)?, idx + 5));
    }

    if !matches!(first, "this" | "thiss" | "its") {
        return None;
    }

    if words.get(idx + 1).is_some_and(is_characteristic_axis_word)
        && characteristic_equal_to_at(words, idx + 2)
    {
        return Some((words.get(idx + 1)?, idx + 5));
    }

    if words.get(idx + 1).is_some_and(|word| word == "creature")
        && words.get(idx + 2).is_some_and(is_characteristic_axis_word)
        && characteristic_equal_to_at(words, idx + 3)
    {
        return Some((words.get(idx + 2)?, idx + 6));
    }

    None
}

fn is_characteristic_axis_word(word: &str) -> bool {
    matches!(word, "power" | "toughness")
}

fn characteristic_equal_to_at(words: &TokenWordView<'_>, idx: usize) -> bool {
    words.get(idx).is_some_and(|word| word == "is")
        && words.get(idx + 1).is_some_and(|word| word == "equal")
        && words.get(idx + 2).is_some_and(|word| word == "to")
}

fn parse_characteristic_defining_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation_tokens(tokens);
    let words = LexedClause::new(trimmed).words();
    if words.is_empty() {
        return None;
    }

    if words.word_refs().as_slice() == ["the", "last", "chosen", "number"] {
        return Some(Value::SourceChosenNumber { if_unset: Some(0), pair: None });
    }
    if let Some(kind) = keyword_static_lines::parse_characteristic_source_value_tokens(trimmed) {
        return Some(match kind {
            keyword_static_lines::CharacteristicSourceValueKind::Power => Value::SourcePower,
            keyword_static_lines::CharacteristicSourceValueKind::Toughness => {
                Value::SourceToughness
            }
        });
    }

    // Preserve specialized aggregate semantics before consulting the broad
    // shared value grammar. The latter can otherwise accept text such as
    // "the number of card types among cards in all graveyards" as a generic
    // object count and erase the card-types-among metric.
    if keyword_static_lines::characteristic_tokens_have_card_types_among_marker(trimmed)
        && let Some(value) = parse_characteristic_defining_pt_value(trimmed)
    {
        return Some(value);
    }

    // Keep characteristic-defining values on the same shared value grammar
    // used by dynamic token definitions. This covers player-relative values
    // such as life totals and player counters, and preserves source-relative
    // zone scopes before the broader object-filter fallback can erase them.
    let value_words = words.word_refs();
    if let Some((value, used)) = parse_value_expr_words(&value_words)
        && used == value_words.len()
    {
        return Some(bind_characteristic_count_to_affected_controller(
            value, trimmed,
        ));
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
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_filter_value(&equal_prefixed))
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(&equal_prefixed))
        .or_else(|| parse_characteristic_defining_pt_value(trimmed))
}

fn bind_characteristic_count_to_affected_controller(
    mut value: Value,
    tokens: &[OwnedLexToken],
) -> Value {
    let count_words =
        crate::util::possessive_normalized_word_refs(&crate::lexer::token_word_refs(tokens));
    if !crate::word_primitives::sequence_occurs(&count_words, &["its", "controller"]) {
        return value;
    }

    fn bind(value: &mut Value) {
        match value {
            Value::SurfaceHinted { value, .. } => bind(value),
            Value::Count(filter) => {
                filter.owner = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
            }
            _ => {}
        }
    }
    bind(&mut value);
    value
}

pub fn parse_characteristic_defining_pt_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = LexedClause::new(tokens).words();
    if words.is_empty() {
        return None;
    }

    let plus_positions: Vec<usize> = words
        .word_refs()
        .into_iter()
        .enumerate()
        .filter_map(|(idx, word)| (word == "plus").then_some(idx))
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

pub fn parse_characteristic_defining_pt_term(tokens: &[OwnedLexToken]) -> Option<Value> {
    if tokens.is_empty() {
        return None;
    }

    if let Some((number, used)) = parse_number(tokens)
        && tokens.len() == used
    {
        return Some(Value::Fixed(number as i32));
    }

    let start = keyword_static_lines::strip_characteristic_number_of_prefix_tokens(tokens);
    if start.is_empty() {
        return None;
    }

    // "the number of cards in the hand of the opponent with the most cards in hand"
    // (Adamaro, First to Desire)
    if let Some(value) = parse_max_cards_in_hand_value_lexed(start) {
        return Some(value);
    }

    if let Some(aggregate) =
        keyword_static_lines::parse_characteristic_aggregate_prefix_tokens(start)
    {
        if aggregate.kind != keyword_static_lines::CharacteristicAggregateKind::CardTypes {
            return parse_aggregate_scope_value_lexed(start);
        }
        let scope_tokens = trim_commas(aggregate.scope_tokens);
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

    let filter = crate::grammar::primitives::probe_shape(parse_object_filter(start, false))?;
    Some(bind_characteristic_count_to_affected_controller(
        Value::Count(filter),
        start,
    ))
}

pub fn parse_shuffle_into_library_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_shuffle_into_library_from_graveyard_line_lexed(tokens) {
        return Ok(Some(StaticAbility::shuffle_into_library_from_graveyard()));
    }

    Ok(None)
}

pub fn parse_permanents_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_permanents_enter_tapped_line_lexed(tokens) {
        return Ok(Some(StaticAbility::permanents_enter_tapped()));
    }
    Ok(None)
}

pub fn parse_creatures_entering_dont_cause_abilities_to_trigger_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::creatures_entering_dont_cause_abilities_to_trigger(),
        ));
    }
    Ok(None)
}

pub fn parse_creatures_assign_combat_damage_using_toughness_line(
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

pub fn parse_you_assign_combat_damage_of_creatures_attacking_you_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_assign_combat_damage_of_creatures_attacking_you_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::you_assign_combat_damage_of_creatures_attacking_you(),
        ));
    }
    Ok(None)
}

pub fn parse_lethal_damage_to_creatures_you_control_uses_power_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_lethal_damage_to_creatures_you_control_uses_power_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::lethal_damage_to_creatures_you_control_uses_power(),
        ));
    }
    Ok(None)
}

/// A rule exception, not an ability granted to the protected permanents.
pub fn parse_zero_loyalty_state_based_exception_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words
        == [
            "planeswalkers",
            "you",
            "control",
            "arent",
            "put",
            "into",
            "their",
            "owners",
            "graveyards",
            "for",
            "having",
            "0",
            "loyalty",
        ]
    {
        return Ok(Some(
            StaticAbility::planeswalkers_you_control_dont_die_at_zero_loyalty(),
        ));
    }
    Ok(None)
}

pub fn parse_players_cant_cycle_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_cant_cycle_line_lexed(tokens) {
        return Ok(Some(StaticAbility::players_cant_cycle()));
    }
    Ok(None)
}

pub fn parse_starting_life_bonus_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(amount) = keyword_static_lines::parse_starting_life_bonus_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::starting_life_bonus(amount as i32)))
}

pub fn parse_buyback_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(amount) = keyword_static_lines::parse_buyback_cost_reduction_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::buyback_cost_reduction(amount)))
}

pub fn parse_spell_cost_increase_per_target_beyond_first_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(cost_head) = static_keyword_cost_shapes::parse_spell_cost_increase_head(tokens) else {
        return Ok(None);
    };
    let line_start = cost_head.line_start.token;
    if !keyword_static_lines::parse_cost_increase_per_target_marker_tokens(tokens) {
        return Ok(None);
    }

    let search_tokens = &tokens[line_start..];
    let costs_idx = cost_head.costs.token.saturating_sub(line_start);
    let amount_tokens = &search_tokens[costs_idx + 1..];
    if let Some((cost, used)) = parse_cost_modifier_mana_cost(amount_tokens)
        && keyword_static_lines::parse_more_cost_tail_prefix_tokens(
            amount_tokens.get(used..).unwrap_or_default(),
        )
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

fn spell_additional_life_cost_per_target_amount(tokens: &[OwnedLexToken]) -> Option<u32> {
    let words = crate::lexer::token_word_refs(tokens);
    if words.len() != 11
        || !words[0].eq_ignore_ascii_case("this")
        || !words[1].eq_ignore_ascii_case("spell")
        || !words[2].eq_ignore_ascii_case("costs")
        || !words[4].eq_ignore_ascii_case("life")
        || !words[5].eq_ignore_ascii_case("more")
        || !words[6].eq_ignore_ascii_case("to")
        || !words[7].eq_ignore_ascii_case("cast")
        || !words[8].eq_ignore_ascii_case("for")
        || !words[9].eq_ignore_ascii_case("each")
        || !words[10].eq_ignore_ascii_case("target")
    {
        return None;
    }
    parse_number_word_i32(words[3])
        .filter(|amount| *amount > 0)
        .map(|amount| amount as u32)
}

/// Parse a mandatory nonmana cost that scales with the number of announced
/// targets. This exact typed domain is excluded from the generic spell-cost
/// modifier grammar.
pub fn parse_spell_additional_life_cost_per_target_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(amount) = spell_additional_life_cost_per_target_amount(tokens) else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::additional_life_cost_per_target(amount)))
}

pub fn parse_if_this_spell_costs_less_to_cast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = split_if_this_spell_costs_line_lexed(tokens) else {
        return Ok(None);
    };

    let condition = parse_this_spell_cost_condition(spec.condition_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        ))
    })?;

    let costs_idx = static_keyword_cost_shapes::parse_cost_verb(spec.tail_tokens)
        .map(|boundary| boundary.token)
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = spec.tail_tokens.get(costs_idx + 1..).unwrap_or_default();
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(0), 0));
    let used = if used > 0 {
        used
    } else if let Some((_, used)) = parsed_mana_cost {
        used
    } else {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    };
    let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
    let remaining_words = parser_token_word_refs(remaining_tokens);
    if static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
        != Some(CostModifierDirection::Less)
        || static_mid_facts::parse_cast_marker_fact(remaining_tokens).is_none()
    {
        return Ok(None);
    }

    if let Some((reduction, _)) = parsed_mana_cost {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::ThisSpellCostReductionManaCost::new(reduction, condition)
                .with_colored_only(
                    costs_replacements_and_permissions::cost_reduction_is_colored_only(tokens),
                ),
        )));
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::ThisSpellCostReduction::new(amount_value, condition),
    )))
}

fn this_spell_cost_condition_from_life_change_this_turn(
    condition: PlayerLifeChangeThisTurnConditionAst,
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    if condition.player != PlayerFilter::You
        || condition.direction != PlayerLifeChangeDirectionAst::Gained
    {
        return None;
    }
    let count = comparison_to_at_least_threshold(&condition.comparison)?;
    Some(crate::static_abilities::ThisSpellCostCondition::YouGainedLifeThisTurnOrMore(count))
}

fn parse_target_whose_controller_has_cards_in_graveyard_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    let words = parser_token_word_refs(tokens);
    let target_start = if crate::word_primitives::parse_sequence_prefix(&words, &["it", "targets"])
    {
        2
    } else if crate::word_primitives::parse_sequence_prefix(&words, &["this", "spell", "targets"]) {
        3
    } else {
        return None;
    };
    let whose =
        crate::word_primitives::parse_sequence_start(&words, &["whose", "controller", "has"])?;
    if whose <= target_start {
        return None;
    }
    let target_start_token = static_keyword_shapes::parse_word_token_offset(tokens, target_start)?;
    let whose_token = static_keyword_shapes::parse_word_token_offset(tokens, whose)?;
    let target_tokens = trim_commas(tokens.get(target_start_token..whose_token)?);
    let mut filter =
        crate::grammar::primitives::probe_shape(parse_object_filter_lexed(&target_tokens, false))?;
    if filter.source || filter.card_types.is_empty() {
        return None;
    }
    // A bare permanent noun in a target condition denotes an object on the
    // battlefield, never a card in another zone.
    filter.zone = Some(Zone::Battlefield);

    let count_start_word = whose + 3;
    let count_start_token =
        static_keyword_shapes::parse_word_token_offset(tokens, count_start_word)?;
    let count_tokens = tokens.get(count_start_token..)?;
    let (count, used) =
        crate::grammar::primitives::probe_shape(parse_greater_than_or_equal_quantity_prefix(
            count_tokens,
            false,
            false,
            "target controller graveyard threshold",
        ))??;
    if !crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(count_tokens.get(used..)?),
        &["cards", "in", "their", "graveyard"],
    ) {
        return None;
    }

    Some(
        crate::static_abilities::ThisSpellCostCondition::TargetsObjectWhoseControllerHasCardsInGraveyardOrMore {
            filter,
            count,
        },
    )
}

mod spell_cost_condition_readings;
mod spell_cost_event_conditions;

pub fn parse_this_spell_cost_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    use crate::static_abilities::ThisSpellCostCondition;
    use static_mid_facts::KnownSpellCostConditionFact as Fact;

    let words = LexedClause::new(tokens).word_refs();
    if words.is_empty() {
        return None;
    }
    let input = spell_cost_condition_readings::SpellCostCondition {
        tokens,
        words: &words,
        read_by_cache: Default::default(),
    };
    match spell_cost_condition_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Some(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(_) => return None,
    }
    // The bound static condition clause is the fallback for what no typed spell-cost condition reads.
    if let Some(condition) = spell_cost_condition_readings::read_bound_static_condition(&input) {
        return Some(condition);
    }

    None
}

#[cfg(test)]
mod target_controller_graveyard_cost_condition_tests {
    use super::*;

    #[test]
    fn parses_target_object_and_its_controllers_graveyard_threshold_as_one_condition() {
        let tokens = crate::lexer::lex_line(
            "it targets a creature whose controller has eight or more cards in their graveyard",
            0,
        )
        .expect("target-controller graveyard condition should lex");
        let condition = parse_this_spell_cost_condition(&tokens)
            .expect("target-controller graveyard condition should parse");

        assert!(matches!(
            condition,
            crate::static_abilities::ThisSpellCostCondition::TargetsObjectWhoseControllerHasCardsInGraveyardOrMore {
                filter,
                count: 8,
            } if filter.zone == Some(Zone::Battlefield)
                && filter.card_types == [CardType::Creature]
        ));
    }

    #[test]
    fn target_controller_graveyard_condition_rejects_changed_provenance_or_card_domain() {
        for text in [
            "it targets a creature whose owner has eight or more cards in their graveyard",
            "it targets a creature whose controller has eight or more creature cards in their graveyard",
            "it targets a creature whose controller has eight or more cards in your graveyard",
        ] {
            let tokens = crate::lexer::lex_line(text, 0)
                .expect("near-miss target-controller condition should lex");
            assert!(
                !matches!(
                    parse_this_spell_cost_condition(&tokens),
                    Some(crate::static_abilities::ThisSpellCostCondition::TargetsObjectWhoseControllerHasCardsInGraveyardOrMore { .. })
                ),
                "near miss was claimed by the correlated condition: {text}"
            );
        }
    }
}

fn parse_conjoined_this_spell_cost_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let and_positions = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| (*word == "and").then_some(idx))
        .collect::<Vec<_>>();
    for and_word_idx in and_positions {
        let and_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, and_word_idx)?;
        let left_tokens = trim_commas(&tokens[..and_token_idx]);
        let right_tokens = trim_commas(&tokens[and_token_idx + 1..]);
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        let Ok(left) = parse_static_condition_clause(&left_tokens) else {
            continue;
        };
        let right = parse_conjoined_this_spell_cost_condition(&right_tokens).or_else(|| {
            crate::grammar::primitives::probe_shape(parse_static_condition_clause(&right_tokens))
        });
        if let Some(right) = right {
            return Some(PredicateAst::And(Box::new(left), Box::new(right)));
        }
    }
    None
}

pub fn parse_trailing_this_spell_cost_condition(
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<crate::static_abilities::ThisSpellCostCondition>, CardTextError> {
    let remaining_words = crate::lexer::token_word_refs(remaining_tokens);
    if let Some(as_long_as_idx) =
        crate::word_primitives::parse_sequence_start(&remaining_words, &["as", "long", "as"])
    {
        let condition_token_idx =
            static_keyword_shapes::parse_word_token_offset(remaining_tokens, as_long_as_idx + 3)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unable to map as-long-as cost condition (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?;
        let condition_tokens = trim_commas(&remaining_tokens[condition_token_idx..]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing as-long-as cost condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let condition = parse_static_condition_clause(&condition_tokens)?;
        let display = render_token_slice(&condition_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return Ok(Some(
            crate::static_abilities::ThisSpellCostCondition::AsLongAsConditionExpr {
                condition: bind_static_condition_predicate(condition)
                    .ok_or_else(|| CardTextError::ParseError(display.clone()))?,
                display,
            },
        ));
    }
    // "during your end step" — a timing condition rather than an if-clause.
    if crate::word_primitives::parse_sequence_suffix(
        &remaining_words,
        &["during", "your", "end", "step"],
    ) {
        return Ok(Some(
            crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
                condition: crate::ConditionExpr::SourceControllersEndStep,
                display: "during your end step".to_string(),
            },
        ));
    }
    // "This spell costs {1} less to cast during your turn" (Mental
    // Modulation): the trailing form of the leading "During your turn, ..."
    // condition.
    if crate::word_primitives::parse_sequence_suffix(&remaining_words, &["during", "your", "turn"])
    {
        return Ok(Some(
            crate::static_abilities::ThisSpellCostCondition::YourTurn,
        ));
    }
    if [
        &["during", "an", "opponent's", "turn"][..],
        &["during", "an", "opponents", "turn"][..],
        &["during", "each", "opponent's", "turn"][..],
        &["during", "each", "opponents", "turn"][..],
    ]
    .iter()
    .any(|suffix| crate::word_primitives::parse_sequence_suffix(&remaining_words, suffix))
    {
        return Ok(Some(
            crate::static_abilities::ThisSpellCostCondition::NotYourTurn,
        ));
    }
    let Some(if_idx) =
        static_keyword_cost_shapes::parse_trailing_cost_condition_if(&remaining_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    let condition_token_idx =
        static_keyword_shapes::parse_word_token_offset(remaining_tokens, if_idx + 1).ok_or_else(
            || {
                CardTextError::ParseError(format!(
                    "unable to map this-spell cost condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            },
        )?;
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

include!("keyword_lines.rs");
include!("anthem_grant_lines.rs");
include!("anthem_grant_conditionals.rs");
include!("etb_static_lines.rs");
include!("attached_object_static_lines.rs");

#[cfg(test)]
mod imperative_multiplier_source_scope_tests {
    use super::*;
    #[test]
    fn imperative_multiplier_source_scope_preserves_explicit_zone_restrictions() {
        for (text, zone) in [
            (
                "Double all damage that creature sources you control would deal.",
                None,
            ),
            (
                "If a creature would deal damage to a player, it deals double that damage instead.",
                Some(Zone::Battlefield),
            ),
            (
                "If a creature source on the battlefield would deal damage to a player, it deals double that damage instead.",
                Some(Zone::Battlefield),
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let ability = parse_double_damage_amount_replacement_line(&tokens)
                .unwrap()
                .expect(text);
            let ironsmith_core::StaticAbilityPayload::DoubleDamageAmountReplacement {
                source_filter,
                ..
            } = ability.payload
            else {
                panic!("expected multiplier")
            };
            assert_eq!(source_filter.zone, zone, "{text}");
            assert_eq!(source_filter.card_types, vec![CardType::Creature]);
        }
    }
}

#[cfg(test)]
mod zero_loyalty_exception_tests {
    use super::*;
    #[test]
    fn zero_loyalty_exception_is_typed_and_preserves_the_controller_scope() {
        let tokens = crate::lexer::lex_line(
            "Planeswalkers you control aren't put into their owners' graveyards for having 0 loyalty.", 0,
        ).unwrap();
        let ParseOutcome::Match(matched) = recognize_static_ability_ast_line_registry(&tokens)
        else {
            panic!("zero-loyalty exception must be claimed by its typed rule");
        };
        let abilities = matched.value;
        assert!(matches!(&abilities[..], [StaticAbilityAst::Static(ability)]
            if ability.id() == crate::static_abilities::StaticAbilityId::PlaneswalkersYouControlDontDieAtZeroLoyalty));
        for text in [
            "Planeswalkers your opponents control aren't put into their owners' graveyards for having 0 loyalty.",
            "Planeswalkers you control aren't put into their owners' graveyards for having 1 loyalty.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(
                parse_zero_loyalty_state_based_exception_line(&tokens)
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[cfg(test)]
mod damage_multiplier_scope_tests;

#[path = "toughness_assignment.rs"]
mod toughness_assignment;
use toughness_assignment::parse_filtered_toughness_assignment_line;

#[cfg(test)]
mod entry_copy_exception_root_tests {
    use super::*;
    #[test]
    fn entry_copy_counter_shapes_keep_x_and_chosen_controller_filters() {
        for (line, dynamic, controlled) in [
            (
                "You may have this creature enter as a copy of any creature on the battlefield, except it enters with X additional +1/+1 counters on it.",
                true,
                false,
            ),
            (
                "You may have this creature enter as a copy of any creature on the battlefield, except it enters with a shield counter on it if you control that creature.",
                false,
                true,
            ),
        ] {
            let ability =
                parse_enter_as_copy_as_enters_line(&crate::lexer::lex_line(line, 0).unwrap())
                    .unwrap()
                    .unwrap();
            let crate::model::CompilerStaticAbilityPayloadCore::EnterAsCopyAsEnters {
                spec, ..
            } = ability.payload
            else {
                panic!("missing copy model");
            };
            assert_eq!(!spec.additional_x_counters.is_empty(), dynamic);
            assert_eq!(spec.additional_counters_source_filter.is_some(), controlled);
        }
    }
    #[test]
    fn own_other_abilities_reference_is_bound_to_the_named_copy_subject() {
        let yes = "You may have Prism enter as a copy of another creature you control, except it has Prism's other abilities.";
        let ability = parse_enter_as_copy_as_enters_line(&crate::lexer::lex_line(yes, 0).unwrap())
            .unwrap()
            .unwrap();
        let crate::model::CompilerStaticAbilityPayloadCore::EnterAsCopyAsEnters { spec, .. } =
            ability.payload
        else {
            panic!("missing copy model");
        };
        assert!(spec.keep_other_source_abilities);
        let no = "You may have Prism enter as a copy of another creature you control, except it has Someone Else's other abilities.";
        assert!(
            parse_enter_as_copy_as_enters_line(&crate::lexer::lex_line(no, 0).unwrap()).is_err()
        );
    }
    #[test]
    fn copy_exceptions_store_executable_attack_keywords_instead_of_markers() {
        for keyword in ["myriad", "dethrone"] {
            let line = format!(
                "You may have this creature enter as a copy of any creature on the battlefield, except it has {keyword}."
            );
            let ability =
                parse_enter_as_copy_as_enters_line(&crate::lexer::lex_line(&line, 0).unwrap())
                    .unwrap()
                    .unwrap();
            let crate::model::CompilerStaticAbilityPayloadCore::EnterAsCopyAsEnters {
                spec, ..
            } = ability.payload
            else {
                panic!("missing copy model");
            };
            assert_eq!(spec.added_abilities.len(), 1);
            assert!(matches!(
                spec.added_abilities[0].kind,
                crate::model::CompilerAbilityKindCore::Triggered(_)
            ));
        }
    }
}

#[cfg(test)]
#[path = "compound_static_body_tests.rs"]
mod compound_static_body_tests;
