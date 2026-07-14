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
    parse_squad_line_lexed, parse_transmute_line_lexed, parse_warp_line_lexed,
    parse_you_may_rather_than_spell_cost_line_lexed,
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
            |option| crate::runtime_backend::semantic::AdditionalCostChoiceOptionAst {
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
    let Some(effect_tokens) = additional_cost_tail_tokens_lexed(tokens) else {
        return Ok(None);
    };
    if is_additional_cost_choice_line_lexed(tokens)
        && parse_additional_cost_choice_options_lexed(effect_tokens)?.is_some()
    {
        return Ok(None);
    }
    let effects = parse_effect_sentences_lexed(effect_tokens)?;
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
        let mut ability =
            crate::static_abilities::StaticAbility::grants(crate::grant::GrantSpec::new(
                crate::grant::Grantable::graveyard_cast_from_cards_mana_cost(
                    Vec::<crate::costs::Cost>::new(),
                    true,
                ),
                crate::target::ObjectFilter::source(),
                crate::zone::Zone::Graveyard,
            ));
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
            crate::static_abilities::StaticAbility::keyword_marker(format!(
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
            crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
                "Sneak",
                Some(cost),
                vec![crate::costs::Cost::effect(crate::effect::Effect::new(
                    crate::effects::SneakCostEffect::new(),
                ))],
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
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let sentences = split_lexed_sentences(tokens);
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
    if !reduction_words.starts_with(&["this", "spell", "costs"])
        || !reduction_words
            .windows(4)
            .any(|window| window == ["to", "cast", "this", "way"])
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
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !token_slice_first_is(tokens, "mutate") {
        return Ok(None);
    }
    let Some((cost, _)) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()) else {
        return Ok(None);
    };
    let _ = line;
    Ok(ast(LineAst::StaticAbility(
        crate::static_abilities::StaticAbility::keyword_marker(format!(
            "Mutate {}",
            cost.to_oracle()
        ))
        .into(),
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
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let Some(parsed) = parse_splice_keyword_line_tokens(tokens)? else {
        return Ok(None);
    };
    let label = format!(
        "Splice onto {} {}",
        parsed.subject.oracle_surface(),
        parsed.cost.to_oracle()
    );
    Ok(ast(LineAst::StaticAbility(
        crate::static_abilities::StaticAbility::keyword_marker(label).into(),
    )))
}

pub(super) fn parse_escalate(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let Some((_cost, display)) = parse_escalate_line_lexed(tokens)? else {
        return Ok(None);
    };
    Ok(ast(LineAst::StaticAbility(
        crate::static_abilities::StaticAbility::keyword_marker(format!("Escalate {display}"))
            .into(),
    )))
}

pub(super) fn parse_eternalize(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    Ok(parse_eternalize_line_lexed(tokens)?.map(|cost| {
        KeywordLinePayloadCst::ast(LineAst::Abilities(vec![
            crate::cards::builders::KeywordAction::Eternalize(cost),
        ]))
    }))
}

pub(super) fn parse_evoke(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    let Some(method) = parse_evoke_line_lexed(tokens)? else {
        return Ok(None);
    };
    Ok(ast(LineAst::Multiple(vec![
        LineAst::AlternativeCastingMethod(method.into()),
        LineAst::Triggered {
            trigger: TriggerSpec::ThisEntersBattlefield,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::ThisSpellPaidLabel("Evoke".into()),
                if_true: vec![EffectAst::subject_verb_sacrifice(
                    PlayerAst::ItsController,
                    crate::target::ObjectFilter::source(),
                    1,
                    Some(TargetAst::Source(None)),
                )],
                if_false: Vec::new(),
            }],
            max_triggers_per_turn: None,
        },
    ])))
}

pub(super) fn parse_epic(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !parse_epic_line_lexed(tokens) {
        return Ok(None);
    }
    Ok(ast(LineAst::StaticAbility(
        crate::static_abilities::StaticAbility::keyword_marker("Epic").into(),
    )))
}

pub(super) fn parse_cast_this_spell_only(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    Ok(parse_cast_this_spell_only_line_lexed(tokens)?
        .map(|ability| KeywordLinePayloadCst::ast(LineAst::StaticAbility(ability.into()))))
}

pub(super) fn parse_gift(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if !is_standard_gift_keyword_tokens_lexed(tokens) {
        return Ok(None);
    }
    let context = rewrite_context(line, tokens, full_tokens, KeywordLineKindCst::Gift);
    Ok(ast(parse_gift_keyword_line(&context)?))
}

pub(super) fn parse_exert_attack(
    line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if parse_keyword_special_form_shape_tokens(tokens) != Some(KeywordSpecialFormShape::ExertAttack)
    {
        return Ok(None);
    }
    let context = rewrite_context(line, tokens, full_tokens, KeywordLineKindCst::ExertAttack);
    Ok(ast(parse_exert_attack_keyword_line(&context, tokens)?))
}

pub(super) fn parse_exploit(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
    _full_tokens: &[OwnedLexToken],
) -> KeywordParseResult {
    if parse_keyword_prefix_shape_tokens(tokens) != Some(KeywordPrefixShape::Exploit) {
        return Ok(None);
    }
    Ok(ast(LineAst::Triggered {
        trigger: TriggerSpec::ThisEntersBattlefield,
        effects: vec![EffectAst::subject_verb_exploit()],
        max_triggers_per_turn: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{LineInfo, NormalizedLine};
    use crate::runtime_backend::lexer::lex_line;

    fn line(text: &str) -> PreprocessedLine {
        let tokens = lex_line(text, 0).expect("keyword test line should lex");
        PreprocessedLine {
            info: LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: text.to_string(),
                source_tokens: tokens.clone(),
                normalized: NormalizedLine {
                    original: text.to_string(),
                    normalized: text.to_ascii_lowercase(),
                    char_map: Vec::new(),
                },
                semantic_facts: Default::default(),
            },
            tokens,
        }
    }

    #[test]
    fn kicker_parser_carries_cost_without_lowering_reparse() {
        let line = line("Kicker {2}{R}");
        let payload = parse_kicker(&line, &line.tokens, &line.info.source_tokens)
            .expect("kicker parser should succeed")
            .expect("kicker should match");
        assert!(matches!(payload, KeywordLinePayloadCst::Kicker { .. }));
    }

    #[test]
    fn blitz_cost_modifier_is_not_claimed_as_keyword_payload() {
        let line = line("Blitz costs you pay cost {1} less.");
        assert!(
            parse_blitz(&line, &line.tokens, &line.info.source_tokens)
                .expect("blitz parser should not fail")
                .is_none()
        );
    }
}
