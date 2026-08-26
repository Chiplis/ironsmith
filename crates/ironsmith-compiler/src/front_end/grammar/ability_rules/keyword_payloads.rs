use crate::cards::builders::{
    CardTextError, EffectAst, LineAst, PlayerAst, PredicateAst, StaticAbilityAst, TargetAst,
    TriggerSpec,
};

use super::activation_and_restrictions::{
    parse_channel_line_lexed, parse_craft_line_lexed, parse_cycling_line_lexed,
    parse_equip_line_lexed, parse_reconfigure_line_lexed,
};
use super::clause_support::parse_effect_sentences_lexed;
use super::cst::{KeywordLineKindCst, KeywordLinePayloadCst};
use super::grammar::abilities::{
    additional_cost_tail_tokens_lexed, is_additional_cost_choice_line_lexed,
    is_standard_gift_keyword_tokens_lexed,
};
use super::grammar::keyword_dispatch::{
    KeywordPrefixShape, KeywordSpecialFormShape, parse_keyword_prefix_shape_tokens,
    parse_keyword_special_form_shape_tokens,
};
use super::grammar::keyword_special_lines::parse_behold_and_exile_additional_cost_tokens;
use super::grammar::splice_keyword_lines::parse_splice_keyword_line_tokens;
use super::ir::RewriteKeywordLine;
use super::keyword_static::{
    parse_if_this_spell_costs_less_to_cast_line_lexed, parse_static_ability_ast_line_lexed,
};
use super::lexer::{
    OwnedLexToken, TokenKind, TokenWordView, split_lexed_sentences, token_slice_at_is,
    token_slice_first_is, trim_lexed_commas,
};
use super::preprocess::PreprocessedLine;
use super::semantic_line_parsing::{
    parse_exert_attack_keyword_line, parse_gift_keyword_line, parse_keyword_special_cases,
};
use super::token_primitives::locate_index as locate_token_index;
use super::util::{
    leading_mana_cost_from_tokens, parse_additional_cost_choice_options_lexed,
    parse_bargain_line_lexed, parse_bestow_line_lexed, parse_blitz_line_lexed,
    parse_buyback_line_lexed, parse_cast_this_spell_only_line_lexed, parse_entwine_line_lexed,
    parse_epic_line_lexed, parse_escalate_line_lexed, parse_escape_line_lexed,
    parse_eternalize_line_lexed, parse_evoke_line_lexed,
    parse_flash_with_additional_cost_line_lexed, parse_flashback_line_lexed,
    parse_harmonize_line_lexed, parse_if_conditional_alternative_cost_line_lexed,
    parse_jump_start_line_lexed, parse_kicker_line_lexed, parse_madness_line_lexed,
    parse_morph_keyword_line_lexed, parse_multikicker_line_lexed, parse_offspring_line_lexed,
    parse_prowl_line_lexed, parse_reinforce_line_lexed, parse_replicate_line_lexed,
    parse_retrace_line_lexed, parse_self_free_cast_alternative_cost_line_lexed,
    parse_squad_line_lexed, parse_transfigure_line_lexed, parse_transmute_line_lexed,
    parse_warp_line_lexed, parse_you_may_rather_than_spell_cost_line_lexed,
};

type KeywordParseResult = Result<Option<KeywordLinePayloadCst>, CardTextError>;

fn ast(ast: LineAst) -> Option<KeywordLinePayloadCst> {
    Some(KeywordLinePayloadCst::ast(ast))
}

fn rewrite_context(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
    kind: KeywordLineKindCst,
) -> RewriteKeywordLine {
    RewriteKeywordLine {
        info: line.info.clone(),
        kind,
        parse_tokens: tokens.to_vec(),
        full_parse_tokens: full_tokens.to_vec(),
        payload: KeywordLinePayloadCst::ast(LineAst::Abilities(Vec::new())),
    }
}

fn optional_cost_tail_effect_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let comma_idx = locate_token_index(tokens, |token| token.kind == TokenKind::Comma)?;
    let effect_tokens = trim_lexed_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

fn keyword_tokens_for_shape<'a>(
    tokens: &'a [OwnedLexToken],
    full_tokens: &'a [OwnedLexToken],
    shape: KeywordPrefixShape,
) -> Option<&'a [OwnedLexToken]> {
    if parse_keyword_prefix_shape_tokens(tokens) == Some(shape) {
        return Some(tokens);
    }
    if parse_keyword_prefix_shape_tokens(full_tokens) == Some(shape) {
        return Some(full_tokens);
    }
    None
}

fn is_supported_sneak_line(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        parse_keyword_special_form_shape_tokens(tokens),
        Some(KeywordSpecialFormShape::SpellSneak | KeywordSpecialFormShape::PermanentSneak)
    )
}

pub(super) fn parse_additional_cost_choice(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !is_additional_cost_choice_line_lexed(tokens) {
        return Ok(None);
    }
    let Some(effect_tokens) = additional_cost_tail_tokens_lexed(tokens) else {
        return Ok(None);
    };
    let Some(options) = parse_additional_cost_choice_options_lexed(effect_tokens)? else {
        return Ok(None);
    };
    let options = options
        .into_iter()
        .map(
            |option| crate::model::compiler_semantic::AdditionalCostChoiceOptionAst {
                description: option.description,
                effects: option.effects,
            },
        )
        .collect();
    Ok(ast(LineAst::AdditionalCostChoice { options }))
}

pub(super) fn parse_additional_cost(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let context = rewrite_context(
        line,
        tokens,
        full_tokens,
        KeywordLineKindCst::AdditionalCost,
    );
    if let Some(parsed) = parse_keyword_special_cases(&context, tokens)? {
        return Ok(ast(parsed));
    }
    if let Some(shape) = parse_behold_and_exile_additional_cost_tokens(tokens) {
        let tag = crate::tag::CompilerReferenceTag::BeheldCost0.key();
        let effects = vec![
            EffectAst::TagAffected {
                effect: Box::new(EffectAst::subject_verb_behold(shape.subtype, 1)),
                tag: tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false),
        ];
        return Ok(ast(LineAst::AdditionalCost { effects }));
    }
    let Some(effect_tokens) = additional_cost_tail_tokens_lexed(tokens) else {
        return Ok(None);
    };
    if is_additional_cost_choice_line_lexed(tokens)
        && parse_additional_cost_choice_options_lexed(effect_tokens)?.is_some()
    {
        return Ok(None);
    }
    let cost_segments = super::grammar::primitives::split_lexed_slices_on_and(effect_tokens);
    let has_heterogeneous_cost_heads = cost_segments.len() > 1
        && cost_segments.iter().all(|segment| {
            super::grammar::primitives::parse_prefix(
                segment,
                super::grammar::leaf::parse_leaf_activation_cost_head_lexed,
            )
            .is_some()
        });
    let effects = if has_heterogeneous_cost_heads {
        let mut effects = Vec::new();
        for segment in cost_segments {
            effects.extend(parse_effect_sentences_lexed(segment)?);
        }
        effects
    } else {
        parse_effect_sentences_lexed(effect_tokens)?
    };
    Ok(ast(LineAst::AdditionalCost { effects }))
}

pub(super) fn parse_alternative_cast(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if parse_keyword_special_form_shape_tokens(tokens) == Some(KeywordSpecialFormShape::ExertAttack)
    {
        return Ok(None);
    }
    if token_slice_first_is(tokens, "aftermath") {
        let mut ability = crate::model::CompilerStaticAbilityCore::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::graveyard_cast_from_cards_mana_cost(
                    Vec::<crate::model::CompilerCost>::new(),
                    true,
                ),
                crate::target::ObjectFilter::source(),
                crate::zone::Zone::Graveyard,
            ),
        );
        ability.label = "Aftermath".to_string();
        return Ok(ast(LineAst::StaticAbility(ability.into())));
    }
    if token_slice_first_is(tokens, "encore") {
        let (cost, _) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "encore keyword missing mana cost '{}'",
                    line.info.raw_line
                ))
            })?;
        return Ok(ast(LineAst::StaticAbility(
            crate::model::CompilerStaticAbilityCore::keyword_marker(format!(
                "Encore {}",
                cost.to_oracle()
            ))
            .into(),
        )));
    }
    if let Some(raw_tokens) =
        keyword_tokens_for_shape(tokens, full_tokens, KeywordPrefixShape::Surge)
    {
        let (cost, _) = leading_mana_cost_from_tokens(raw_tokens.get(1..).unwrap_or_default())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "surge keyword missing cost '{}'",
                    line.info.raw_line
                ))
            })?;
        let condition = crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
            condition: crate::ConditionExpr::Or(
                Box::new(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: crate::target::PlayerFilter::You,
                    count: 1,
                }),
                Box::new(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: crate::target::PlayerFilter::Teammate,
                    count: 1,
                }),
            ),
            display: "you or a teammate has cast another spell this turn".to_string(),
        };
        return Ok(ast(LineAst::AlternativeCastingMethod(
            crate::alternative_cast::AlternativeCastingMethod::alternative_cost_with_condition(
                "Surge",
                Some(cost),
                Vec::new(),
                condition,
            )
            .into(),
        )));
    }
    if let Some(keyword_tokens) =
        keyword_tokens_for_shape(tokens, full_tokens, KeywordPrefixShape::Freerunning)
    {
        let (cost, _) = leading_mana_cost_from_tokens(keyword_tokens.get(1..).unwrap_or_default())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "freerunning keyword missing cost '{}'",
                    line.info.raw_line
                ))
            })?;
        return Ok(ast(LineAst::AlternativeCastingMethod(
            crate::alternative_cast::AlternativeCastingMethod::alternative_cost_with_condition(
                "Freerunning",
                Some(cost),
                Vec::new(),
                crate::static_abilities::ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                    crate::types::Subtype::Assassin,
                ),
            )
            .into(),
        )));
    }
    if let Some(keyword_tokens) =
        keyword_tokens_for_shape(tokens, full_tokens, KeywordPrefixShape::Sneak)
    {
        let support_tokens = if full_tokens.is_empty() {
            keyword_tokens
        } else {
            full_tokens
        };
        if !is_supported_sneak_line(support_tokens) {
            return Err(CardTextError::ParseError(format!(
                "sneak keyword form is not yet supported: '{}'",
                line.info.raw_line
            )));
        }
        let (cost, _) = leading_mana_cost_from_tokens(keyword_tokens.get(1..).unwrap_or_default())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "sneak keyword missing cost '{}'",
                    line.info.raw_line
                ))
            })?;
        return Ok(ast(LineAst::AlternativeCastingMethod(
            crate::model::CompilerAlternativeCastingMethod::alternative_cost(
                "Sneak",
                Some(cost),
                vec![crate::model::CompilerCost::Sneak],
            )
            .into(),
        )));
    }
    if parse_keyword_special_form_shape_tokens(tokens)
        == Some(KeywordSpecialFormShape::BlitzFromGraveyard)
    {
        return Ok(ast(LineAst::Abilities(vec![
            crate::cards::builders::KeywordAction::BlitzFromGraveyard,
        ])));
    }
    if let Some(method) = parse_self_free_cast_alternative_cost_line_lexed(tokens) {
        return Ok(ast(LineAst::AlternativeCastingMethod(method.into())));
    }
    if let Some(method) = parse_flash_with_additional_cost_line_lexed(tokens) {
        return Ok(ast(LineAst::AlternativeCastingMethod(method.into())));
    }
    if let Some(method) = parse_jump_start_line_lexed(tokens)? {
        return Ok(ast(LineAst::AlternativeCastingMethod(method.into())));
    }
    let surface = line.info.normalized.normalized.as_str();
    if let Some(method) = parse_you_may_rather_than_spell_cost_line_lexed(tokens, surface)? {
        return Ok(ast(LineAst::AlternativeCastingMethod(method.into())));
    }
    if let Some(method) = parse_if_conditional_alternative_cost_line_lexed(tokens, surface)? {
        return Ok(ast(LineAst::AlternativeCastingMethod(method.into())));
    }
    if let Some(method) = parse_prowl_line_lexed(tokens)? {
        return Ok(ast(LineAst::AlternativeCastingMethod(method.into())));
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(tokens)? {
        return Ok(ast(LineAst::StaticAbility(ability.into())));
    }
    Ok(None)
}

macro_rules! alternative_method_parser {
    ($name:ident, $parser:ident) => {
        pub(super) fn $name(
            _line: &PreprocessedLine,
            tokens: &[OwnedLexToken],
            _full_tokens: &[OwnedLexToken],
        ) -> KeywordParseResult {
            Ok($parser(tokens)?.map(|parsed| {
                KeywordLinePayloadCst::ast(LineAst::AlternativeCastingMethod(parsed.into()))
            }))
        }
    };
}

macro_rules! optional_cost_parser {
    ($name:ident, $parser:ident) => {
        pub(super) fn $name(
            _line: &PreprocessedLine,
            tokens: &[OwnedLexToken],
            _full_tokens: &[OwnedLexToken],
        ) -> KeywordParseResult {
            Ok($parser(tokens)?
                .map(|parsed| KeywordLinePayloadCst::ast(LineAst::OptionalCost(parsed.into()))))
        }
    };
}

macro_rules! ability_parser {
    ($name:ident, $parser:ident) => {
        pub(super) fn $name(
            _line: &PreprocessedLine,
            tokens: &[OwnedLexToken],
            _full_tokens: &[OwnedLexToken],
        ) -> KeywordParseResult {
            Ok($parser(tokens)?.map(|parsed| KeywordLinePayloadCst::ast(LineAst::Ability(parsed))))
        }
    };
}

alternative_method_parser!(parse_bestow, parse_bestow_line_lexed);
alternative_method_parser!(parse_escape, parse_escape_line_lexed);
alternative_method_parser!(parse_harmonize, parse_harmonize_line_lexed);
alternative_method_parser!(parse_retrace, parse_retrace_line_lexed);
alternative_method_parser!(parse_madness, parse_madness_line_lexed);
alternative_method_parser!(parse_warp, parse_warp_line_lexed);

pub(super) fn parse_flashback(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    // Semantic preprocessing may present the keyword registry with only the
    // leading keyword sentence while retaining the complete source line in
    // `full_tokens`.  The Commander 2021 "Visions of" cycle uses the second
    // sentence to qualify its flashback cost reduction, so parsing only the
    // selected sentence silently loses the alternative casting method.
    let selected_sentences = split_lexed_sentences(tokens);
    let full_sentences = split_lexed_sentences(full_tokens);
    let sentences = if selected_sentences.len() >= 2 {
        selected_sentences
    } else if token_slice_first_is(full_tokens, "flashback") && full_sentences.len() >= 2 {
        full_sentences[..2].to_vec()
    } else {
        selected_sentences
    };
    let Some(flashback_tokens) = sentences.first().copied() else {
        return Ok(None);
    };
    let Some(method) = parse_flashback_line_lexed(flashback_tokens)? else {
        return Ok(None);
    };

    if sentences.len() == 1 {
        return Ok(ast(LineAst::AlternativeCastingMethod(method.into())));
    }
    if sentences.len() != 2 {
        return Ok(None);
    }

    let reduction_tokens = sentences[1];
    let reduction_words = TokenWordView::new(reduction_tokens).word_refs();
    if !crate::word_primitives::parse_sequence_prefix(&reduction_words, &["this", "spell", "costs"])
        || !crate::word_primitives::sequence_occurs(
            &reduction_words,
            &["to", "cast", "this", "way"],
        )
    {
        return Ok(None);
    }

    let Some(mut abilities) = parse_static_ability_ast_line_lexed(reduction_tokens)? else {
        return Ok(None);
    };
    if abilities.len() != 1 {
        return Ok(None);
    }
    let StaticAbilityAst::Static(mut ability) = abilities.pop().expect("checked one ability")
    else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) =
        &mut ability.payload
    else {
        return Ok(None);
    };
    reduction.alternative_cast = Some(crate::filter::AlternativeCastKind::Flashback);

    Ok(ast(LineAst::Multiple(vec![
        LineAst::AlternativeCastingMethod(method.into()),
        LineAst::StaticAbility(StaticAbilityAst::Static(ability)),
    ])))
}

optional_cost_parser!(parse_bargain, parse_bargain_line_lexed);
optional_cost_parser!(parse_buyback, parse_buyback_line_lexed);
optional_cost_parser!(parse_multikicker, parse_multikicker_line_lexed);
optional_cost_parser!(parse_replicate, parse_replicate_line_lexed);
optional_cost_parser!(parse_offspring, parse_offspring_line_lexed);
optional_cost_parser!(parse_entwine, parse_entwine_line_lexed);

ability_parser!(parse_channel, parse_channel_line_lexed);
ability_parser!(parse_cycling, parse_cycling_line_lexed);
ability_parser!(parse_craft, parse_craft_line_lexed);
ability_parser!(parse_reinforce, parse_reinforce_line_lexed);
ability_parser!(parse_equip, parse_equip_line_lexed);
ability_parser!(parse_reconfigure, parse_reconfigure_line_lexed);
ability_parser!(parse_morph, parse_morph_keyword_line_lexed);
ability_parser!(parse_transmute, parse_transmute_line_lexed);
ability_parser!(parse_transfigure, parse_transfigure_line_lexed);

pub(super) fn parse_blitz(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if token_slice_at_is(tokens, 1, "costs") {
        return Ok(None);
    }
    Ok(parse_blitz_line_lexed(tokens)?
        .map(|parsed| KeywordLinePayloadCst::ast(LineAst::AlternativeCastingMethod(parsed.into()))))
}

pub(super) fn parse_kicker(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    Ok(parse_kicker_line_lexed(tokens)?.map(|parsed| KeywordLinePayloadCst::kicker(parsed.cost)))
}

pub(super) fn parse_mutate(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !token_slice_first_is(tokens, "mutate") {
        return Ok(None);
    }
    let Some((cost, _)) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()) else {
        return Ok(None);
    };
    Ok(ast(LineAst::AlternativeCastingMethod(
        crate::alternative_cast::AlternativeCastingMethod::Mutate { cost }.into(),
    )))
}

pub(super) fn parse_squad(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if let Some(cost) = parse_squad_line_lexed(tokens)? {
        return Ok(ast(LineAst::OptionalCost(cost.into())));
    }
    if let Some(effect_tokens) = optional_cost_tail_effect_tokens(tokens)
        && let Ok(effects) = parse_effect_sentences_lexed(effect_tokens)
        && !effects.is_empty()
    {
        return Ok(ast(LineAst::Statement { effects }));
    }
    Ok(None)
}

pub(super) fn parse_splice(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let Some(parsed) = parse_splice_keyword_line_tokens(tokens)? else {
        return Ok(None);
    };
    let is_edge_punctuation = |token: &crate::lexer::OwnedLexToken| {
        matches!(
            token.kind,
            crate::lexer::TokenKind::Dash
                | crate::lexer::TokenKind::EmDash
                | crate::lexer::TokenKind::Period
        )
    };
    let start = crate::slice_primitives::select_position(parsed.cost_tokens, |token| {
        !is_edge_punctuation(token)
    })
    .unwrap_or(parsed.cost_tokens.len());
    let end = crate::slice_primitives::select_last_position(parsed.cost_tokens, |token| {
        !is_edge_punctuation(token)
    })
    .map_or(start, |index| index + 1);
    let cost_tokens = &parsed.cost_tokens[start..end];
    let cost_surface = cost_tokens
        .first()
        .zip(cost_tokens.last())
        .and_then(|(first, last)| line.info.raw_line.get(first.span.start..last.span.end))
        .map(str::trim)
        .filter(|surface| !surface.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            crate::lexer::render_token_slice(cost_tokens)
                .trim()
                .to_string()
        });
    let cost = crate::activation_and_restrictions::keyword_action_costs::parse_payment_clause_as_total_cost(
        cost_tokens,
    )?
        .ok_or_else(|| {
            crate::cards::builders::CardTextError::ParseError(format!(
                "unsupported splice cost clause: {}",
                crate::lexer::render_token_slice(cost_tokens).trim()
            ))
        })?;
    let quality = match parsed.subject {
        super::grammar::splice_keyword_lines::SpliceSubject::Arcane => {
            crate::static_abilities::SpliceQuality::Arcane
        }
        super::grammar::splice_keyword_lines::SpliceSubject::InstantOrSorcery => {
            crate::static_abilities::SpliceQuality::InstantOrSorcery
        }
    };
    Ok(ast(LineAst::StaticAbility(
        crate::model::CompilerStaticAbilityCore::splice_with_cost_surface(
            quality,
            cost,
            Some(cost_surface),
        )
        .into(),
    )))
}

#[cfg(test)]
#[path = "keyword_payloads_inline_tests.rs"]
mod tests;

#[path = "keyword_payloads/core_programs.rs"]
mod core_programs;
pub(super) use core_programs::{
    parse_epic, parse_escalate, parse_eternalize, parse_evoke, parse_exploit,
};
#[path = "keyword_payloads/combat_programs.rs"]
mod combat_programs;
pub(super) use combat_programs::parse_exert_attack;
#[path = "keyword_payloads/condition_programs.rs"]
mod condition_programs;
pub(super) use condition_programs::parse_gift;
#[path = "keyword_payloads/permission_programs.rs"]
mod permission_programs;
pub(super) use permission_programs::parse_cast_this_spell_only;
