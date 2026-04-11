use crate::Until;
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::builders::{
    CardDefinition, CardDefinitionBuilder, CardTextError, ChoiceCount, EffectAst, GiftTimingAst,
    IT_TAG, InsteadSemantics, LineAst, LineInfo, OptionalCost, ParseAnnotations, ParsedAbility,
    ParsedCardItem, ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLineAst,
    ParsedModalAst, ParsedModalModeAst, ParsedRestrictions, PlayerAst, PredicateAst,
    ReferenceImports, ReturnControllerAst, TagKey, TargetAst, TextSpan, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::mana::ManaSymbol;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod activated_lowering;
mod line_lowering;
mod normalization_support;
mod parser_semantic_lowering;
mod rewrite_support;

pub(crate) use activated_lowering::lower_rewrite_activated_to_chunk;
use activated_lowering::{LoweredRewriteActivatedLine, align_rewrite_activated_parse_sentences};
pub(crate) use normalization_support::rewrite_document_to_normalized_card_ast;
pub(crate) use parser_semantic_lowering::{
    lower_exert_attack_keyword_line, lower_gift_keyword_line, lower_keyword_special_cases,
    lower_rewrite_statement_token_groups_to_chunks, lower_rewrite_static_to_chunk,
    lower_rewrite_triggered_to_chunk,
};
#[cfg(test)]
pub(crate) use parser_semantic_lowering::lower_rewrite_keyword_to_chunk;
#[cfg(test)]
use parser_semantic_lowering::{
    normalize_exert_followup_source_reference_tokens, parse_single_effect_lexed,
    strip_lexed_suffix_phrase,
};
use parser_semantic_lowering::{
    infer_rewrite_triggered_functional_zones, lower_rewrite_modal_to_item,
};
pub(crate) use parser_semantic_lowering::{
    lower_rewrite_nissas_encouragement_statement_to_chunk, lower_rewrite_pact_statement_to_chunk,
    lower_rewrite_shape_anew_statement_to_chunk, lower_rewrite_soul_partition_statement_to_chunk,
    lower_rewrite_statement_to_unsupported_chunk, lower_rewrite_divvy_statement_to_chunk,
    lower_rewrite_empty_laboratory_statement_to_chunk, lower_special_rewrite_triggered_chunk,
    try_lower_optional_behold_additional_cost, try_lower_optional_cost_with_cast_trigger,
};
use normalization_support::{
    apply_chosen_option_to_triggered_chunk, apply_explicit_intervening_if_to_triggered_chunk,
};

use rewrite_support::{
    infer_static_ability_functional_zones, infer_triggered_ability_functional_zones,
    rewrite_finalize_lowered_card, rewrite_normalize_additional_cost_sacrifice_tags,
    runtime_effects_to_costs,
};

use super::activation_and_restrictions::{
    find_word_sequence_start, infer_activated_functional_zones_lexed,
    is_any_player_may_activate_sentence_lexed, parse_activation_cost,
    parse_mana_spend_bonus_sentence_lexed, parse_mana_usage_restriction_sentence_lexed,
};
use super::activation_and_restrictions::{
    parse_channel_line_lexed, parse_cycling_line_lexed, parse_equip_line_lexed,
};
use super::clause_support::{
    parse_ability_line_lexed, parse_effect_sentences_lexed, parse_static_ability_ast_line_lexed,
    parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::compile_support::{
    collect_tag_spans_from_effects_with_context, compile_condition_from_predicate_ast_with_env,
    materialize_prepared_effects_with_trigger_context,
    trigger_binds_player_reference_context as rewrite_trigger_binds_player_reference_context,
};
use super::effect_pipeline::{
    NormalizedAdditionalCostChoiceOptionAst, NormalizedCardAst, NormalizedCardItem,
    NormalizedLineAst, NormalizedLineChunk, NormalizedModalAst, NormalizedModalModeAst,
    NormalizedParsedAbility, NormalizedPreparedAbility,
};
use super::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed;
use super::ir::{
    RewriteKeywordLine, RewriteKeywordLineKind, RewriteLevelHeader, RewriteModalBlock,
    RewriteSagaChapterLine, RewriteSemanticDocument, RewriteSemanticItem, RewriteStatementLine,
    RewriteStaticLine, RewriteTriggeredLine,
};
use super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed;
use super::lexer::{
    OwnedLexToken, TokenKind, TokenWordView, lex_line, render_token_slice, split_lexed_sentences,
    token_word_refs, trim_lexed_commas,
};
use super::lowering_support::{
    rewrite_apply_instead_followup_statement_to_last_ability, rewrite_lower_prepared_ability,
    rewrite_lower_prepared_additional_cost_choice_modes_with_exports,
    rewrite_lower_prepared_statement_effects, rewrite_lower_static_abilities_ast,
    rewrite_lower_static_ability_ast, rewrite_parsed_triggered_ability,
    rewrite_prepare_effects_for_lowering,
    rewrite_prepare_effects_with_trigger_context_for_lowering,
    rewrite_prepare_triggered_effects_for_lowering, rewrite_static_ability_for_keyword_action,
    rewrite_validate_iterated_player_bindings_in_lowered_effects,
};
use super::modal_support::{parse_modal_header, replace_modal_header_x_in_effects_ast};
use super::parser_support::split_text_for_parse;
use super::reference_model::LoweredEffects;
use super::reference_model::ReferenceEnv;
use super::reference_model::ReferenceExports;
use super::restriction_support::{
    apply_pending_mana_restriction, apply_pending_restrictions_to_ability, is_restrictable_ability,
};
use super::token_primitives::{
    find_index, iter_contains, lexed_tokens_contain_non_prefix_instead,
    remove_copy_exception_type_removal_lexed, rewrite_followup_intro_to_if_lexed, slice_contains,
    slice_ends_with, slice_starts_with, split_em_dash_label_prefix, str_contains, str_ends_with,
    str_find, str_split_once, str_split_once_char, str_starts_with, str_strip_prefix,
    str_strip_suffix, word_view_has_any_prefix, word_view_has_prefix,
};
use super::util::{
    classify_instead_followup_text, find_first_sacrifice_cost_choice_tag,
    find_last_exile_cost_choice_tag, join_sentences_with_period,
    parse_additional_cost_choice_options_lexed, parse_bargain_line_lexed, parse_bestow_line_lexed,
    parse_buyback_line_lexed, parse_cast_this_spell_only_line_lexed, parse_entwine_line_lexed,
    parse_escape_line_lexed, parse_flashback_line_lexed, parse_harmonize_line_lexed,
    parse_if_conditional_alternative_cost_line_lexed, parse_kicker_line_lexed,
    parse_level_up_line_lexed, parse_madness_line_lexed, parse_mana_symbol,
    parse_morph_keyword_line_lexed, parse_multikicker_line_lexed, parse_number_or_x_value_lexed,
    parse_offspring_line_lexed, parse_reinforce_line_lexed, parse_scryfall_mana_cost,
    parse_self_free_cast_alternative_cost_line_lexed, parse_squad_line_lexed,
    parse_transmute_line_lexed, parse_warp_line_lexed,
    parse_you_may_rather_than_spell_cost_line_lexed, preserve_keyword_prefix_for_parse,
    token_index_for_word_index, trim_commas, words,
};

fn rewrite_unsupported_line_ast(
    raw_line: &str,
    reason: impl Into<String>,
) -> crate::cards::builders::LineAst {
    LineAst::StaticAbility(StaticAbility::unsupported_parser_line(raw_line, reason).into())
}

fn lexed_tokens(text: &str, line_index: usize) -> Result<Vec<OwnedLexToken>, CardTextError> {
    lex_line(text, line_index)
}

fn word_refs_have_prefix(words: &[&str], prefix: &[&str]) -> bool {
    slice_starts_with(words, prefix)
}

fn word_refs_have_suffix(words: &[&str], suffix: &[&str]) -> bool {
    slice_ends_with(words, suffix)
}

fn word_refs_find(words: &[&str], expected: &str) -> Option<usize> {
    find_index(words, |word| *word == expected)
}

#[derive(Debug, Clone, Default)]
struct RewriteLoweredCardState {
    haunt_linkage: Option<(Vec<crate::effect::Effect>, Vec<ChooseSpec>)>,
    latest_spell_exports: ReferenceExports,
    latest_additional_cost_exports: ReferenceExports,
}

fn rewrite_update_last_restrictable_ability(
    builder: &CardDefinitionBuilder,
    abilities_before: usize,
    last_restrictable_ability: &mut Option<usize>,
) {
    let abilities_after = builder.abilities.len();
    if abilities_after <= abilities_before {
        return;
    }

    for ability_idx in (abilities_before..abilities_after).rev() {
        if is_restrictable_ability(&builder.abilities[ability_idx]) {
            *last_restrictable_ability = Some(ability_idx);
            return;
        }
    }
}

fn rewrite_lower_level_ability_ast(
    level: ParsedLevelAbilityAst,
) -> Result<crate::ability::LevelAbility, CardTextError> {
    let mut lowered = crate::ability::LevelAbility::new(level.min_level, level.max_level);
    if let Some((power, toughness)) = level.pt {
        lowered = lowered.with_pt(power, toughness);
    }

    for item in level.items {
        match item {
            ParsedLevelAbilityItemAst::StaticAbilities(abilities) => {
                lowered
                    .abilities
                    .extend(rewrite_lower_static_abilities_ast(abilities)?);
            }
            ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                for action in actions {
                    if let Some(ability) = rewrite_static_ability_for_keyword_action(action) {
                        lowered.abilities.push(ability);
                    }
                }
            }
        }
    }

    Ok(lowered)
}

fn title_case_words(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn color_set_name(colors: ColorSet) -> Option<&'static str> {
    if colors == ColorSet::WHITE {
        return Some("white");
    }
    if colors == ColorSet::BLUE {
        return Some("blue");
    }
    if colors == ColorSet::BLACK {
        return Some("black");
    }
    if colors == ColorSet::RED {
        return Some("red");
    }
    if colors == ColorSet::GREEN {
        return Some("green");
    }
    None
}

fn describe_hexproof_from_filter(filter: &crate::target::ObjectFilter) -> String {
    if !filter.any_of.is_empty() {
        return filter
            .any_of
            .iter()
            .map(describe_hexproof_from_filter)
            .collect::<Vec<_>>()
            .join(" or ");
    }

    let description = filter.description();
    str_strip_suffix(description.as_str(), " permanent")
        .or_else(|| str_strip_suffix(description.as_str(), " spell"))
        .or_else(|| str_strip_suffix(description.as_str(), " source"))
        .unwrap_or(description.as_str())
        .to_string()
}

fn keyword_action_line_text(action: &crate::cards::builders::KeywordAction) -> String {
    use crate::cards::builders::KeywordAction;

    match action {
        KeywordAction::Flying => "Flying".to_string(),
        KeywordAction::Menace => "Menace".to_string(),
        KeywordAction::Hexproof => "Hexproof".to_string(),
        KeywordAction::Haste => "Haste".to_string(),
        KeywordAction::Improvise => "Improvise".to_string(),
        KeywordAction::Convoke => "Convoke".to_string(),
        KeywordAction::AffinityForArtifacts => "Affinity for artifacts".to_string(),
        KeywordAction::Delve => "Delve".to_string(),
        KeywordAction::FirstStrike => "First strike".to_string(),
        KeywordAction::DoubleStrike => "Double strike".to_string(),
        KeywordAction::Deathtouch => "Deathtouch".to_string(),
        KeywordAction::Lifelink => "Lifelink".to_string(),
        KeywordAction::Vigilance => "Vigilance".to_string(),
        KeywordAction::Trample => "Trample".to_string(),
        KeywordAction::Reach => "Reach".to_string(),
        KeywordAction::Defender => "Defender".to_string(),
        KeywordAction::Flash => "Flash".to_string(),
        KeywordAction::Phasing => "Phasing".to_string(),
        KeywordAction::Indestructible => "Indestructible".to_string(),
        KeywordAction::Shroud => "Shroud".to_string(),
        KeywordAction::Ward(amount) => format!("Ward {{{amount}}}"),
        KeywordAction::Wither => "Wither".to_string(),
        KeywordAction::Afterlife(amount) => format!("Afterlife {amount}"),
        KeywordAction::Fabricate(amount) => format!("Fabricate {amount}"),
        KeywordAction::Infect => "Infect".to_string(),
        KeywordAction::Undying => "Undying".to_string(),
        KeywordAction::Persist => "Persist".to_string(),
        KeywordAction::Prowess => "Prowess".to_string(),
        KeywordAction::Exalted => "Exalted".to_string(),
        KeywordAction::Cascade => "Cascade".to_string(),
        KeywordAction::Storm => "Storm".to_string(),
        KeywordAction::Toxic(amount) => format!("Toxic {amount}"),
        KeywordAction::BattleCry => "Battle cry".to_string(),
        KeywordAction::Dethrone => "Dethrone".to_string(),
        KeywordAction::Evolve => "Evolve".to_string(),
        KeywordAction::Ingest => "Ingest".to_string(),
        KeywordAction::Mentor => "Mentor".to_string(),
        KeywordAction::Skulk => "Skulk".to_string(),
        KeywordAction::Training => "Training".to_string(),
        KeywordAction::Myriad => "Myriad".to_string(),
        KeywordAction::Riot => "Riot".to_string(),
        KeywordAction::Unleash => "Unleash".to_string(),
        KeywordAction::Renown(amount) => format!("Renown {amount}"),
        KeywordAction::Modular(amount) => format!("Modular {amount}"),
        KeywordAction::ModularSunburst => "Modular—Sunburst".to_string(),
        KeywordAction::Graft(amount) => format!("Graft {amount}"),
        KeywordAction::Soulbond => "Soulbond".to_string(),
        KeywordAction::Soulshift(amount) => format!("Soulshift {amount}"),
        KeywordAction::Outlast(cost) => format!("Outlast {}", cost.to_oracle()),
        KeywordAction::Scavenge(cost) => format!("Scavenge {}", cost.to_oracle()),
        KeywordAction::Unearth(cost) => format!("Unearth {}", cost.to_oracle()),
        KeywordAction::Ninjutsu(cost) => format!("Ninjutsu {}", cost.to_oracle()),
        KeywordAction::Backup(amount) => format!("Backup {amount}"),
        KeywordAction::Cipher => "Cipher".to_string(),
        KeywordAction::Dash(cost) => format!("Dash {}", cost.to_oracle()),
        KeywordAction::Warp(cost) => format!("Warp {}", cost.to_oracle()),
        KeywordAction::Plot(cost) => format!("Plot {}", cost.to_oracle()),
        KeywordAction::Melee => "Melee".to_string(),
        KeywordAction::Mobilize(amount) => format!("Mobilize {amount}"),
        KeywordAction::Suspend { time, cost } => format!("Suspend {time}—{}", cost.to_oracle()),
        KeywordAction::Disturb(cost) => format!("Disturb {}", cost.to_oracle()),
        KeywordAction::Overload(cost) => format!("Overload {}", cost.to_oracle()),
        KeywordAction::Spectacle(cost) => format!("Spectacle {}", cost.to_oracle()),
        KeywordAction::Foretell(cost) => format!("Foretell {}", cost.to_oracle()),
        KeywordAction::Echo { text, .. } => text.clone(),
        KeywordAction::CumulativeUpkeep { text, .. } => text.clone(),
        KeywordAction::Extort => "Extort".to_string(),
        KeywordAction::Partner => "Partner".to_string(),
        KeywordAction::Assist => "Assist".to_string(),
        KeywordAction::SplitSecond => "Split second".to_string(),
        KeywordAction::Rebound => "Rebound".to_string(),
        KeywordAction::Sunburst => "Sunburst".to_string(),
        KeywordAction::Fading(amount) => format!("Fading {amount}"),
        KeywordAction::Vanishing(amount) => format!("Vanishing {amount}"),
        KeywordAction::Fear => "Fear".to_string(),
        KeywordAction::Intimidate => "Intimidate".to_string(),
        KeywordAction::Shadow => "Shadow".to_string(),
        KeywordAction::Horsemanship => "Horsemanship".to_string(),
        KeywordAction::Flanking => "Flanking".to_string(),
        KeywordAction::UmbraArmor => "Umbra armor".to_string(),
        KeywordAction::Landwalk(kind) => kind.display(),
        KeywordAction::Bloodthirst(amount) => format!("Bloodthirst {amount}"),
        KeywordAction::Rampage(amount) => format!("Rampage {amount}"),
        KeywordAction::Bushido(amount) => format!("Bushido {amount}"),
        KeywordAction::Changeling => "Changeling".to_string(),
        KeywordAction::HexproofFrom(filter) => {
            format!("Hexproof from {}", describe_hexproof_from_filter(filter))
        }
        KeywordAction::ProtectionFrom(colors) => {
            if let Some(color_name) = color_set_name(*colors) {
                return format!("Protection from {color_name}");
            }
            "Protection from colors".to_string()
        }
        KeywordAction::ProtectionFromAllColors => "Protection from all colors".to_string(),
        KeywordAction::ProtectionFromColorless => "Protection from colorless".to_string(),
        KeywordAction::ProtectionFromEverything => "Protection from everything".to_string(),
        KeywordAction::ProtectionFromChosenPlayer => {
            "Protection from the chosen player".to_string()
        }
        KeywordAction::ProtectionFromCardType(card_type) => {
            format!("Protection from {}", card_type.name()).to_ascii_lowercase()
        }
        KeywordAction::ProtectionFromSubtype(subtype) => {
            format!(
                "Protection from {}",
                subtype.to_string().to_ascii_lowercase()
            )
        }
        KeywordAction::Unblockable => "This creature can't be blocked".to_string(),
        KeywordAction::Devoid => "Devoid".to_string(),
        KeywordAction::Annihilator(amount) => format!("Annihilator {amount}"),
        KeywordAction::ForMirrodin => "For Mirrodin!".to_string(),
        KeywordAction::LivingWeapon => "Living weapon".to_string(),
        KeywordAction::Crew { amount, .. } => format!("Crew {amount}"),
        KeywordAction::Saddle { amount, .. } => format!("Saddle {amount}"),
        KeywordAction::Marker(name) => title_case_words(name),
        KeywordAction::MarkerText(text) => text.clone(),
        KeywordAction::Casualty(power) => format!("Casualty {power}"),
        KeywordAction::Conspire => "Conspire".to_string(),
        KeywordAction::Devour(multiplier) => format!("Devour {multiplier}"),
        KeywordAction::Ravenous => "Ravenous".to_string(),
        KeywordAction::Ascend => "Ascend".to_string(),
        KeywordAction::Daybound => "Daybound".to_string(),
        KeywordAction::Nightbound => "Nightbound".to_string(),
        KeywordAction::Haunt => "Haunt".to_string(),
        KeywordAction::Provoke => "Provoke".to_string(),
        KeywordAction::Undaunted => "Undaunted".to_string(),
        KeywordAction::Enlist => "Enlist".to_string(),
    }
}

fn keyword_actions_line_text(
    actions: &[crate::cards::builders::KeywordAction],
    separator: &str,
) -> Option<String> {
    if actions.is_empty() {
        return None;
    }
    Some(
        actions
            .iter()
            .map(keyword_action_line_text)
            .collect::<Vec<_>>()
            .join(separator),
    )
}

fn uses_spell_only_functional_zones(static_ability: &StaticAbility) -> bool {
    matches!(
        static_ability.id(),
        crate::static_abilities::StaticAbilityId::ConditionalSpellKeyword
            | crate::static_abilities::StaticAbilityId::ThisSpellCastRestriction
            | crate::static_abilities::StaticAbilityId::ThisSpellCostReduction
            | crate::static_abilities::StaticAbilityId::ThisSpellCostReductionManaCost
    )
}

fn uses_referenced_ability_functional_zones(
    static_ability: &StaticAbility,
    normalized_line: &str,
) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ActivatedAbilityCostReduction
        && str_starts_with(normalized_line, "this ability costs")
}

fn uses_all_zone_functional_zones(static_ability: &StaticAbility) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
}

fn effect_target_uses_it_reference(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(_) => true,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            effect_target_uses_it_reference(inner)
        }
        _ => false,
    }
}

fn extract_previous_replacement_target(effect: &crate::effect::Effect) -> Option<ChooseSpec> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return extract_previous_replacement_target(&tagged.effect);
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some(damage.target.clone());
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some(destroy.spec.clone());
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        return Some(destroy.spec.clone());
    }
    if let Some(modify) = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>() {
        return Some(modify.target.clone());
    }
    if let Some(continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        if let Some(target_spec) = &continuous.target_spec {
            return Some(target_spec.clone());
        }
    }
    None
}

fn rewrite_replacement_effect_target(
    effect: &crate::effect::Effect,
    previous_target: &ChooseSpec,
) -> Option<crate::effect::Effect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(rewritten_inner) =
            rewrite_replacement_effect_target(&tagged.effect, previous_target)
    {
        return Some(crate::effect::Effect::new(
            crate::effects::TaggedEffect::new(tagged.tag.clone(), rewritten_inner),
        ));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>()
        && effect_target_uses_it_reference(&destroy.spec)
    {
        return Some(crate::effect::Effect::new(
            crate::effects::DestroyEffect::with_spec(previous_target.clone()),
        ));
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>()
        && effect_target_uses_it_reference(&damage.target)
    {
        return Some(crate::effect::Effect::deal_damage(
            damage.amount.clone(),
            previous_target.clone(),
        ));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
        && effect_target_uses_it_reference(&destroy.spec)
    {
        return Some(crate::effect::Effect::new(
            crate::effects::DestroyNoRegenerationEffect::with_spec(previous_target.clone()),
        ));
    }
    None
}

fn push_unsupported_marker(
    builder: CardDefinitionBuilder,
    raw_line: &str,
    reason: String,
) -> CardDefinitionBuilder {
    builder.with_ability(
        Ability::static_ability(StaticAbility::unsupported_parser_line(
            raw_line.trim(),
            reason,
        ))
        .with_text(raw_line),
    )
}

fn rewrite_apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &crate::cards::builders::LineInfo,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    line_lowering::rewrite_apply_line_ast(
        builder,
        state,
        parsed,
        info,
        allow_unsupported,
        annotations,
    )
}

fn rewrite_lower_line_ast(
    builder: &mut CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    annotations: &mut ParseAnnotations,
    line: NormalizedLineAst,
    allow_unsupported: bool,
    last_restrictable_ability: &mut Option<usize>,
) -> Result<(), CardTextError> {
    let NormalizedLineAst {
        info,
        chunks,
        mut restrictions,
    } = line;
    let mut handled_restrictions_for_new_ability = false;

    for parsed in chunks {
        if let NormalizedLineChunk::Statement { effects_ast, .. } = &parsed
            && rewrite_apply_instead_followup_statement_to_last_ability(
                builder,
                *last_restrictable_ability,
                effects_ast,
                &info,
                annotations,
            )?
        {
            handled_restrictions_for_new_ability = true;
            continue;
        }

        let abilities_before = builder.abilities.len();
        *builder = rewrite_apply_line_ast(
            builder.clone(),
            state,
            parsed,
            &info,
            allow_unsupported,
            annotations,
        )?;
        let abilities_after = builder.abilities.len();

        for ability_idx in abilities_before..abilities_after {
            apply_pending_restrictions_to_ability(
                &mut builder.abilities[ability_idx],
                &mut restrictions,
            );
            handled_restrictions_for_new_ability = true;
        }

        rewrite_update_last_restrictable_ability(
            builder,
            abilities_before,
            last_restrictable_ability,
        );
    }

    if !handled_restrictions_for_new_ability
        && let Some(index) = *last_restrictable_ability
        && index < builder.abilities.len()
    {
        apply_pending_restrictions_to_ability(&mut builder.abilities[index], &mut restrictions);
    }

    Ok(())
}

fn lower_compound_buff_and_unblockable_static_chunk(
    _line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some((buff_tokens, unblockable_tokens)) =
        split_compound_buff_and_unblockable_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    if let Some(abilities) = parse_static_ability_ast_line_lexed(parse_tokens)? {
        return Ok(Some(LineAst::StaticAbilities(abilities)));
    }

    let Some(mut abilities) = parse_static_ability_ast_line_lexed(&buff_tokens)? else {
        return Ok(None);
    };
    let Some(unblockable_abilities) = parse_static_ability_ast_line_lexed(&unblockable_tokens)?
    else {
        return Ok(None);
    };
    abilities.extend(unblockable_abilities);
    Ok(Some(LineAst::StaticAbilities(abilities)))
}

fn split_compound_buff_and_unblockable_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let words = TokenWordView::new(tokens);
    let gets_idx = words.find_word("gets")?;
    let and_idx = words.find_phrase_start(&["and", "cant", "be", "blocked"])?;
    if and_idx + 4 != words.len() {
        return None;
    }

    let subject_token_end = words.token_index_for_word_index(gets_idx)?;
    let and_token_idx = words.token_index_for_word_index(and_idx)?;
    let cant_token_idx = words.token_index_for_word_index(and_idx + 1)?;
    if subject_token_end == 0
        || subject_token_end >= and_token_idx
        || cant_token_idx <= and_token_idx
    {
        return None;
    }

    let left_tokens = tokens[..and_token_idx].to_vec();
    let mut right_tokens =
        Vec::with_capacity(subject_token_end + tokens.len().saturating_sub(cant_token_idx));
    right_tokens.extend_from_slice(&tokens[..subject_token_end]);
    right_tokens.extend_from_slice(&tokens[cant_token_idx..]);
    Some((left_tokens, right_tokens))
}

fn lower_split_rewrite_static_chunk(
    line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let sentences = split_lexed_sentences(parse_tokens);
    if sentences.len() <= 1 {
        return Ok(None);
    }

    let mut abilities = Vec::new();
    for sentence_tokens in sentences {
        if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(sentence_tokens)? {
            abilities.push(ability.into());
            continue;
        }
        if let Some(parsed) = parse_static_ability_ast_line_lexed(sentence_tokens)? {
            abilities.extend(parsed);
            continue;
        }
        return Ok(None);
    }

    wrap_chosen_option_static_chunk(
        LineAst::StaticAbilities(abilities),
        effective_chosen_option_label(&line.info.raw_line, line.chosen_option_label.as_deref()),
    )
    .map(Some)
}

fn should_skip_keyword_action_static_probe(normalized: &str) -> bool {
    let normalized = normalized.trim();
    (str_ends_with(normalized, "can't be blocked.")
        || str_ends_with(normalized, "can't be blocked"))
        && !str_starts_with(normalized, "this ")
        && !str_starts_with(normalized, "it ")
}

fn split_statement_label_prefix_for_lowering_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(String, &[OwnedLexToken])> {
    split_em_dash_label_prefix(tokens)
}

fn strip_non_keyword_label_prefix_for_lowering_lexed(
    mut tokens: &[OwnedLexToken],
) -> &[OwnedLexToken] {
    if looks_like_numeric_result_prefix_lexed(tokens) {
        return tokens;
    }
    while let Some((label, body_tokens)) = split_statement_label_prefix_for_lowering_lexed(tokens) {
        if preserve_keyword_prefix_for_parse(label.as_str()) {
            break;
        }
        tokens = body_tokens;
    }
    tokens
}

fn rewrite_statement_followup_intro_for_lowering_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    rewrite_followup_intro_to_if_lexed(tokens)
}

fn rewrite_copy_exception_type_removal_for_lowering_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    remove_copy_exception_type_removal_lexed(tokens)
}

fn looks_like_numeric_result_prefix_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        tokens.first().map(|token| token.kind),
        Some(TokenKind::Number)
    ) && matches!(
        tokens.get(1).map(|token| token.kind),
        Some(TokenKind::Dash | TokenKind::EmDash)
    ) && matches!(
        tokens.get(2).map(|token| token.kind),
        Some(TokenKind::Number)
    ) && tokens
        .iter()
        .skip(3)
        .any(|token| token.kind == TokenKind::Pipe)
}

fn rewrite_statement_parse_sentences_for_lowering_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence_tokens| !sentence_tokens.is_empty())
        .map(strip_non_keyword_label_prefix_for_lowering_lexed)
        .map(rewrite_statement_followup_intro_for_lowering_lexed)
        .map(|tokens| rewrite_copy_exception_type_removal_for_lowering_lexed(&tokens))
        .filter(|tokens| !tokens.is_empty())
        .collect()
}

fn statement_sentence_contains_instead_split_for_lowering(tokens: &[OwnedLexToken]) -> bool {
    lexed_tokens_contain_non_prefix_instead(tokens)
}

fn group_statement_sentences_for_lowering_lexed(
    sentence_tokens: Vec<Vec<OwnedLexToken>>,
    fallback_tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    if sentence_tokens.len() <= 1 {
        let only_sentence = sentence_tokens
            .into_iter()
            .next()
            .or_else(|| {
                let fallback = strip_non_keyword_label_prefix_for_lowering_lexed(fallback_tokens);
                (!fallback.is_empty()).then(|| {
                    rewrite_copy_exception_type_removal_for_lowering_lexed(
                        &rewrite_statement_followup_intro_for_lowering_lexed(fallback),
                    )
                })
            })
            .unwrap_or_default();
        return (!only_sentence.is_empty())
            .then_some(only_sentence)
            .into_iter()
            .collect();
    }

    let split_idx = sentence_tokens
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, sentence)| {
            statement_sentence_contains_instead_split_for_lowering(sentence).then_some(idx)
        });

    let Some(split_idx) = split_idx else {
        return vec![join_sentences_with_period(&sentence_tokens)];
    };

    let mut groups = Vec::new();
    if !sentence_tokens[..split_idx].is_empty() {
        groups.push(join_sentences_with_period(&sentence_tokens[..split_idx]));
    }
    if !sentence_tokens[split_idx..].is_empty() {
        groups.push(join_sentences_with_period(&sentence_tokens[split_idx..]));
    }
    groups
}

fn wrap_chosen_option_static_chunk(
    chunk: LineAst,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    let Some(label) = chosen_option_label else {
        return Ok(chunk);
    };
    let condition = crate::ConditionExpr::SourceChosenOption(label.to_string());
    Ok(match chunk {
        LineAst::StaticAbility(ability) => LineAst::StaticAbility(
            crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            },
        ),
        LineAst::StaticAbilities(abilities) => LineAst::StaticAbilities(
            abilities
                .into_iter()
                .map(
                    |ability| crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(ability),
                        condition: condition.clone(),
                    },
                )
                .collect(),
        ),
        other => other,
    })
}

fn effective_chosen_option_label<'a>(
    raw_line: &str,
    chosen_option_label: Option<&'a str>,
) -> Option<&'a str> {
    let _ = raw_line;
    chosen_option_label
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::parser::RewriteKeywordLineKind;
    use crate::cards::builders::parser::pipeline::parse_text_to_semantic_document;
    use crate::cards::builders::{
        CardDefinitionBuilder, CardId, CardType, LineAst, NormalizedLine,
    };

    #[test]
    fn rewrite_activated_sentence_alignment_merges_inner_quoted_periods() {
        let effect_text = r#"target creature gains "{t}: add {g}." until end of turn. any player may activate this ability."#;
        let effect_parse_tokens = lex_line(effect_text, 0)
            .expect("rewrite lexer should classify quoted activated effect");
        let rendered_effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        let (parsed_sentences, _) = split_text_for_parse(
            rendered_effect_text.as_str(),
            rendered_effect_text.as_str(),
            0,
        );
        let token_sentence_texts = split_lexed_sentences(&effect_parse_tokens)
            .into_iter()
            .map(|tokens| render_token_slice(tokens).trim().to_string())
            .collect::<Vec<_>>();

        let aligned = align_rewrite_activated_parse_sentences(&parsed_sentences, &effect_parse_tokens)
            .unwrap_or_else(|| {
                panic!(
                    "quoted activated sentences should align against existing token slices: parsed={parsed_sentences:?} token_sentences={token_sentence_texts:?}"
                )
            });

        assert_eq!(aligned.len(), 2);
        assert_eq!(render_token_slice(&aligned[0]).trim(), parsed_sentences[0]);
        assert_eq!(render_token_slice(&aligned[1]).trim(), parsed_sentences[1]);
    }

    #[test]
    fn rewrite_exert_followup_subject_rewrite_uses_existing_tokens() {
        let tokens = lex_line("he can't block this turn.", 0)
            .expect("rewrite lexer should classify exert followup");

        let normalized = normalize_exert_followup_source_reference_tokens(
            "Champion",
            trim_lexed_commas(&tokens),
        );

        assert_eq!(
            render_token_slice(&normalized).trim(),
            "this creature can't block this turn."
        );
    }

    #[test]
    fn rewrite_exert_keyword_lowering_reuses_token_followup_for_linked_trigger()
    -> Result<(), CardTextError> {
        let text = "you may exert champion as it attacks. when you do, he can't block this turn.";
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify exert keyword line");

        let parsed = lower_rewrite_keyword_to_chunk(
            super::LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: text.to_string(),
                normalized: NormalizedLine {
                    original: text.to_string(),
                    normalized: text.to_string(),
                    char_map: Vec::new(),
                },
            },
            text,
            &tokens,
            RewriteKeywordLineKind::ExertAttack,
        )?;

        match parsed {
            LineAst::StaticAbility(ability) => {
                let debug = format!("{ability:?}");
                assert!(str_contains(debug.as_str(), "ExertAttack"), "{debug}");
                assert!(
                    str_contains(debug.as_str(), "linked_trigger: Some"),
                    "{debug}"
                );
            }
            other => panic!("expected exert static ability, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn rewrite_special_triggered_burning_rune_demon_accepts_stored_parse_tokens()
    -> Result<(), CardTextError> {
        let full_text = "when this creature enters, you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle.";
        let trigger_text = "when this creature enters";
        let effect_text = "you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle.";
        let full_tokens =
            lex_line(full_text, 0).expect("rewrite lexer should classify burning rune demon line");
        let trigger_tokens = lex_line(trigger_text, 0)
            .expect("rewrite lexer should classify burning rune demon trigger");
        let effect_tokens = lex_line(effect_text, 0)
            .expect("rewrite lexer should classify burning rune demon effect");

        let parsed = lower_rewrite_triggered_to_chunk(
            super::LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: full_text.to_string(),
                normalized: NormalizedLine {
                    original: full_text.to_string(),
                    normalized: full_text.to_string(),
                    char_map: Vec::new(),
                },
            },
            full_text,
            &full_tokens,
            trigger_text,
            &trigger_tokens,
            effect_text,
            &effect_tokens,
            None,
            None,
            None,
        )?;

        let debug = format!("{parsed:?}");
        assert!(str_contains(debug.as_str(), "Triggered"), "{debug}");
        assert!(str_contains(debug.as_str(), "divvy_source"), "{debug}");
        assert!(str_contains(debug.as_str(), "divvy_chosen"), "{debug}");
        assert!(str_contains(debug.as_str(), "ShuffleLibrary"), "{debug}");

        Ok(())
    }

    #[test]
    fn rewrite_divvy_suffix_trim_reuses_first_sentence_tokens() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "Exile up to five target permanent cards from your graveyard and separate them into two piles.",
            0,
        )
        .expect("rewrite lexer should classify divvy exile sentence");
        let first_sentence = split_lexed_sentences(&tokens)
            .into_iter()
            .next()
            .expect("expected first sentence tokens");
        let trimmed = strip_lexed_suffix_phrase(
            first_sentence,
            &["and", "separate", "them", "into", "two", "piles"],
        )
        .expect("expected divvy pile suffix to trim");

        assert_eq!(
            render_token_slice(trimmed).trim(),
            "Exile up to five target permanent cards from your graveyard"
        );
        assert!(matches!(
            parse_single_effect_lexed(trimmed)?,
            EffectAst::Exile { .. }
        ));

        Ok(())
    }

    #[test]
    fn rewrite_triggered_normalization_keeps_explicit_intervening_if_predicate()
    -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Portcullis Variant")
            .card_types(vec![CardType::Artifact]);
        let (doc, _) = parse_text_to_semantic_document(
            builder,
            "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.".to_string(),
            false,
        )?;

        let normalized = rewrite_document_to_normalized_card_ast(doc)?;
        let parsed = normalized
            .items
            .into_iter()
            .find_map(|item| match item {
                NormalizedCardItem::Line(line) => line.chunks.into_iter().find_map(|chunk| {
                    if let NormalizedLineChunk::Ability(parsed) = chunk {
                        Some(parsed)
                    } else {
                        None
                    }
                }),
                _ => None,
            })
            .expect("expected Portcullis-style line to normalize into a triggered ability");

        let AbilityKind::Triggered(triggered) = parsed.parsed.ability.kind else {
            panic!(
                "expected Portcullis-style line to normalize into a triggered ability, got {:?}",
                parsed.parsed.ability.kind
            );
        };
        let debug = format!("{:?}", triggered.intervening_if);
        assert!(
            triggered.intervening_if.is_some(),
            "expected trigger predicate to survive normalization, got {debug}"
        );
        assert!(
            debug.contains("ValueComparison"),
            "expected battlefield-count predicate to survive normalization, got {debug}"
        );

        Ok(())
    }
}

fn apply_pending_mana_restrictions(
    parsed: &mut ParsedAbility,
    restrictions: &[String],
) -> Result<(), CardTextError> {
    let AbilityKind::Activated(ability) = &mut parsed.ability.kind else {
        return Err(CardTextError::InvariantViolation(
            "rewrite activated lowering expected activated ability kind".to_string(),
        ));
    };
    for restriction in restrictions {
        apply_pending_mana_restriction(ability, restriction);
    }
    Ok(())
}

fn parse_next_spell_cost_reduction_sentence_rewrite(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let clause_words = token_word_refs(tokens);
    if !word_refs_have_prefix(clause_words.as_slice(), &["the", "next"]) {
        return None;
    }

    let spell_idx = word_refs_find(clause_words.as_slice(), "spell")?;
    let costs_idx = word_refs_find(clause_words.as_slice(), "costs")?;
    let less_idx = word_refs_find(clause_words.as_slice(), "less")?;
    if clause_words.get(spell_idx + 1).copied() != Some("you")
        || clause_words.get(spell_idx + 2).copied() != Some("cast")
        || clause_words.get(spell_idx + 3).copied() != Some("this")
        || clause_words.get(spell_idx + 4).copied() != Some("turn")
        || clause_words.get(less_idx + 1).copied() != Some("to")
        || clause_words.get(less_idx + 2).copied() != Some("cast")
        || costs_idx <= spell_idx
    {
        return None;
    }

    let spell_token_idx = find_index(tokens, |token| token.is_word("spell"))?;
    let costs_token_idx = find_index(tokens, |token| token.is_word("costs"))?;
    let less_token_idx = find_index(tokens, |token| token.is_word("less"))?;
    if less_token_idx <= costs_token_idx + 1 {
        return None;
    }
    let spell_filter_tokens = trim_lexed_commas(&tokens[2..spell_token_idx]);
    let reduction_tokens = trim_lexed_commas(&tokens[costs_token_idx + 1..less_token_idx]);
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spell_filter_tokens);
    let reduction_symbols = reduction_tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::ManaGroup => Some(token.slice.trim_start_matches('{').trim_end_matches('}')),
            TokenKind::Word | TokenKind::Number => token.as_word(),
            TokenKind::Comma | TokenKind::Period => None,
            _ => Some(""),
        })
        .map(parse_mana_symbol)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if reduction_symbols.is_empty() {
        return None;
    }
    let reduction = crate::mana::ManaCost::from_symbols(reduction_symbols);

    Some(EffectAst::ReduceNextSpellCostThisTurn {
        player: crate::cards::builders::PlayerAst::You,
        filter,
        reduction,
    })
}

fn parse_each_player_and_their_creatures_damage_sentence_rewrite(
    effect_text: &str,
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let normalized = effect_text
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let matches_shape = str_contains(
        normalized.as_str(),
        " damage to each player and each creature they control",
    ) || str_contains(
        normalized.as_str(),
        " damage to each player and each creatures they control",
    ) || str_contains(
        normalized.as_str(),
        " damage to each player and each creature that player controls",
    ) || str_contains(
        normalized.as_str(),
        " damage to each player and each creatures that player controls",
    );
    if !matches_shape {
        return None;
    }
    let clause_words = token_word_refs(tokens);
    let deals_idx = find_index(clause_words.as_slice(), |word| {
        matches!(*word, "deal" | "deals")
    })?;
    let amount_start = token_index_for_word_index(tokens, deals_idx + 1)?;
    let (amount, _used) = parse_number_or_x_value_lexed(&tokens[amount_start..])?;

    let mut filter = crate::filter::ObjectFilter::default();
    filter.card_types = vec![crate::types::CardType::Creature];
    filter.controller = Some(crate::PlayerFilter::IteratedPlayer);

    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: crate::cards::builders::TargetAst::Player(
                    crate::PlayerFilter::IteratedPlayer,
                    None,
                ),
            },
            EffectAst::DealDamageEach { amount, filter },
        ],
    }])
}

pub(crate) fn lower_rewrite_document(
    doc: RewriteSemanticDocument,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let ast = rewrite_document_to_normalized_card_ast(doc)?;
    let NormalizedCardAst {
        mut builder,
        mut annotations,
        items,
        allow_unsupported,
    } = ast;

    let mut level_abilities = Vec::new();
    let mut last_restrictable_ability: Option<usize> = None;
    let mut state = RewriteLoweredCardState::default();

    for item in items {
        match item {
            NormalizedCardItem::Line(line) => {
                rewrite_lower_line_ast(
                    &mut builder,
                    &mut state,
                    &mut annotations,
                    line,
                    allow_unsupported,
                    &mut last_restrictable_ability,
                )?;
            }
            NormalizedCardItem::Modal(modal) => {
                let abilities_before = builder.abilities.len();
                builder = rewrite_lower_parsed_modal(builder, modal, allow_unsupported)?;
                rewrite_update_last_restrictable_ability(
                    &builder,
                    abilities_before,
                    &mut last_restrictable_ability,
                );
            }
            NormalizedCardItem::LevelAbility(level) => {
                level_abilities.push(rewrite_lower_level_ability_ast(level)?);
            }
        }
    }

    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }

    builder = rewrite_finalize_lowered_card(builder, &mut state);
    Ok((builder.build(), annotations))
}

fn try_merge_modal_into_remove_mode(
    effects: &mut crate::resolution::ResolutionProgram,
    modal_effect: crate::effect::Effect,
    predicate: crate::effect::EffectPredicate,
) -> bool {
    let Some(last_effect) = effects.pop() else {
        return false;
    };

    let Some(choose_mode) = last_effect.downcast_ref::<crate::effects::ChooseModeEffect>() else {
        effects.push(last_effect);
        return false;
    };
    if choose_mode.modes.len() < 2 {
        effects.push(last_effect);
        return false;
    }

    let Some(remove_mode_idx) = find_index(choose_mode.modes.as_slice(), |mode| {
        str_starts_with(mode.description.to_ascii_lowercase().as_str(), "remove ")
    }) else {
        effects.push(last_effect);
        return false;
    };

    let mut modes = choose_mode.modes.clone();
    let remove_mode = &mut modes[remove_mode_idx];
    let gate_id = crate::effect::EffectId(1_000_000_000);
    if let Some(last_remove_effect) = remove_mode.effects.pop() {
        remove_mode.effects.push(crate::effect::Effect::with_id(
            gate_id.0,
            last_remove_effect,
        ));
        remove_mode.effects.push(crate::effect::Effect::if_then(
            gate_id,
            predicate,
            vec![modal_effect],
        ));
    } else {
        remove_mode.effects.push(modal_effect);
    }

    effects.push(crate::effect::Effect::new(
        crate::effects::ChooseModeEffect {
            modes,
            choose_count: choose_mode.choose_count.clone(),
            min_choose_count: choose_mode.min_choose_count.clone(),
            allow_repeated_modes: choose_mode.allow_repeated_modes,
            disallow_previously_chosen_modes: choose_mode.disallow_previously_chosen_modes,
            disallow_previously_chosen_modes_this_turn: choose_mode
                .disallow_previously_chosen_modes_this_turn,
        },
    ));
    true
}

pub(crate) fn rewrite_lower_parsed_modal(
    mut builder: CardDefinitionBuilder,
    pending_modal: NormalizedModalAst,
    allow_unsupported: bool,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let NormalizedModalAst {
        header,
        prepared_prefix,
        modes,
    } = pending_modal;
    let crate::cards::builders::ParsedModalHeader {
        min: header_min,
        max: header_max,
        same_mode_more_than_once,
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        commander_allows_both,
        trigger,
        activated,
        x_replacement: _,
        prefix_effects_ast: _,
        modal_gate,
        line_text,
    } = header;

    let (prefix_effects, prefix_choices) = if prepared_prefix.is_none() {
        (crate::resolution::ResolutionProgram::default(), Vec::new())
    } else if trigger.is_some() || activated.is_some() {
        match super::compile_support::materialize_prepared_effects_with_trigger_context(
            prepared_prefix
                .as_ref()
                .expect("prepared prefix exists when checked above"),
        ) {
            Ok(lowered) => (lowered.effects, lowered.choices),
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(builder, line_text.as_str(), format!("{err:?}"));
                return Ok(builder);
            }
            Err(err) => return Err(err),
        }
    } else {
        match rewrite_lower_prepared_statement_effects(
            prepared_prefix
                .as_ref()
                .expect("prepared prefix exists when checked above"),
        ) {
            Ok(lowered) => (lowered.effects, lowered.choices),
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(builder, line_text.as_str(), format!("{err:?}"));
                return Ok(builder);
            }
            Err(err) => return Err(err),
        }
    };

    let mut compiled_modes = Vec::new();
    for mode in modes {
        let effects = match rewrite_lower_prepared_statement_effects(&mode.prepared) {
            Ok(lowered) => lowered.effects,
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(
                    builder,
                    mode.info.raw_line.as_str(),
                    format!("{err:?}"),
                );
                continue;
            }
            Err(err) => return Err(err),
        };
        compiled_modes.push(crate::effect::EffectMode {
            description: mode.description,
            effects: effects.to_vec(),
        });
    }

    if compiled_modes.is_empty() {
        return Ok(builder);
    }

    let mode_count = compiled_modes.len() as i32;
    let default_max = crate::effect::Value::Fixed(mode_count);
    let max = header_max.unwrap_or_else(|| default_max.clone());
    let min = header_min;
    let is_fixed_one =
        |value: &crate::effect::Value| matches!(value, crate::effect::Value::Fixed(1));
    let with_unchosen_requirement = |effect: crate::effect::Effect| {
        if !mode_must_be_unchosen {
            return effect;
        }
        if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
            let choose_mode = choose_mode.clone();
            let choose_mode = if mode_must_be_unchosen_this_turn {
                choose_mode.with_previously_unchosen_modes_only_this_turn()
            } else {
                choose_mode.with_previously_unchosen_modes_only()
            };
            return crate::effect::Effect::new(choose_mode);
        }
        effect
    };

    let modal_effect = if commander_allows_both {
        let max_both = (mode_count.min(2)).max(1);
        let choose_both = if max_both == 1 {
            with_unchosen_requirement(crate::effect::Effect::choose_one(compiled_modes.clone()))
        } else {
            with_unchosen_requirement(crate::effect::Effect::choose_up_to(
                max_both,
                1,
                compiled_modes.clone(),
            ))
        };
        let choose_one =
            with_unchosen_requirement(crate::effect::Effect::choose_one(compiled_modes.clone()));
        crate::effect::Effect::conditional(
            crate::effect::Condition::YouControlCommander,
            vec![choose_both],
            vec![choose_one],
        )
    } else if same_mode_more_than_once && min == max {
        with_unchosen_requirement(crate::effect::Effect::choose_exactly_allow_repeated_modes(
            max.clone(),
            compiled_modes,
        ))
    } else if is_fixed_one(&min) && is_fixed_one(&max) {
        with_unchosen_requirement(crate::effect::Effect::choose_one(compiled_modes))
    } else if min == max {
        with_unchosen_requirement(crate::effect::Effect::choose_exactly(
            max.clone(),
            compiled_modes,
        ))
    } else {
        with_unchosen_requirement(crate::effect::Effect::choose_up_to(
            max.clone(),
            min.clone(),
            compiled_modes,
        ))
    };

    let mut combined_effects = prefix_effects;
    if let Some(modal_gate) = modal_gate {
        if modal_gate.remove_mode_only
            && try_merge_modal_into_remove_mode(
                &mut combined_effects,
                modal_effect.clone(),
                modal_gate.predicate.clone(),
            )
        {
        } else if let Some(last_effect) = combined_effects.pop() {
            let gate_id = crate::effect::EffectId(1_000_000_000);
            combined_effects.push(crate::effect::Effect::with_id(gate_id.0, last_effect));
            combined_effects.push(crate::effect::Effect::if_then(
                gate_id,
                modal_gate.predicate,
                vec![modal_effect],
            ));
        } else {
            combined_effects.push(modal_effect);
        }
    } else {
        combined_effects.push(modal_effect);
    }

    let modal_lowered = LoweredEffects {
        effects: combined_effects.clone(),
        choices: prefix_choices.clone(),
        exports: ReferenceExports::default(),
    };
    rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &modal_lowered,
        trigger
            .as_ref()
            .is_some_and(rewrite_trigger_binds_player_reference_context),
        if trigger.is_some() {
            "triggered modal ability effects"
        } else if activated.is_some() {
            "activated modal ability effects"
        } else {
            "modal spell effects"
        },
    )?;

    if let Some(trigger) = trigger {
        let mut ability = rewrite_parsed_triggered_ability(
            trigger,
            Vec::new(),
            vec![Zone::Battlefield],
            Some(line_text),
            None,
            ReferenceImports::default(),
        )
        .ability;
        if let AbilityKind::Triggered(triggered) = &mut ability.kind {
            triggered.effects = combined_effects.clone();
            triggered.choices = prefix_choices;
        }
        builder = builder.with_ability(ability);
    } else if let Some(activated) = activated {
        builder = builder.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: activated.mana_cost,
                effects: combined_effects.clone(),
                choices: prefix_choices,
                timing: activated.timing,
                additional_restrictions: activated.additional_restrictions,
                activation_restrictions: activated.activation_restrictions,
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: activated.functional_zones,
            text: Some(line_text),
        });
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.extend(combined_effects);
    } else {
        builder.spell_effect = Some(combined_effects);
    }

    Ok(builder)
}
