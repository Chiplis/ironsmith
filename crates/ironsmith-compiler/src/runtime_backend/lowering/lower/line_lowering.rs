use crate::ability::{Ability, AbilityKind};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, EffectAst, GiftTimingAst, LineInfo, ParseAnnotations,
    PlayerAst, TriggerSpec,
};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

use super::super::effect_pipeline::{
    NormalizedLineChunk, NormalizedParsedAbility, NormalizedPreparedAbility,
};
use super::*;

struct LineChunkLoweringInput<'a> {
    builder: CardDefinitionBuilder,
    state: &'a mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &'a LineInfo,
    allow_unsupported: bool,
    annotations: &'a mut ParseAnnotations,
}

fn conditional_self_replacement_followup(
    effect: &crate::effect::Effect,
) -> Option<crate::effects::ConditionalEffect> {
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return Some(conditional.clone());
    }
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .and_then(|tagged| conditional_self_replacement_followup(&tagged.effect))
}

fn materialized_self_replacement_followup(
    program: &crate::resolution::ResolutionProgram,
) -> Option<crate::resolution::SelfReplacementBranch> {
    let [segment] = program.segments.as_slice() else {
        return None;
    };
    if !segment.default_effects.is_empty() || segment.self_replacements.len() != 1 {
        return None;
    }
    Some(segment.self_replacements[0].clone())
}

fn retarget_replacement_effects(
    effects: Vec<crate::effect::Effect>,
    previous_target: &ChooseSpec,
) -> Vec<crate::effect::Effect> {
    effects
        .into_iter()
        .map(|effect| {
            if let Some(replacement_damage) =
                effect.downcast_ref::<crate::effects::DealDamageEffect>()
                && replacement_damage.target == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
            {
                crate::effect::Effect::deal_damage(
                    replacement_damage.amount.clone(),
                    previous_target.clone(),
                )
            } else {
                super::rewrite_replacement_effect_target(&effect, previous_target).unwrap_or(effect)
            }
        })
        .collect()
}

fn compile_trailing_instead_if_condition(
    normalized_line: &str,
    line_index: usize,
    prepared: &super::super::effect_pipeline::PreparedEffectsForLowering,
) -> Result<Option<crate::effect::Condition>, CardTextError> {
    let tokens = lex_line(normalized_line, line_index).map_err(|err| {
        CardTextError::ParseError(format!(
            "failed to lex instead-if follow-up '{}': {err:?}",
            normalized_line
        ))
    })?;
    let Some(instead_idx) = tokens.iter().enumerate().find_map(|(idx, token)| {
        (token.is_word("instead") && tokens.get(idx + 1).is_some_and(|next| next.is_word("if")))
            .then_some(idx)
    }) else {
        return Ok(None);
    };
    let Some(predicate) =
        crate::runtime_backend::grammar::structure::parse_trailing_instead_if_predicate_lexed(
            &tokens[instead_idx..],
        )
    else {
        return Ok(None);
    };
    compile_condition_from_predicate_ast_with_env(
        &predicate,
        &prepared.initial_env,
        prepared.imports.last_object_tag.as_ref(),
    )
    .map(Some)
}

pub(super) fn rewrite_apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &LineInfo,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    match parsed {
        parsed @ NormalizedLineChunk::Abilities(_) => {
            lower_abilities_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::StaticAbility(_) => {
            lower_static_ability_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::StaticAbilities(_) => {
            lower_static_abilities_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Ability(_) => {
            lower_parsed_ability_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Statement { .. } => {
            lower_statement_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AdditionalCost { .. } => {
            lower_additional_cost_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::OptionalCost(_) => {
            lower_optional_cost_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::GiftKeyword { .. } => {
            lower_gift_keyword_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::OptionalCostWithCastTrigger { .. } => {
            lower_optional_cost_with_cast_trigger_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AdditionalCostChoice { .. } => {
            lower_additional_cost_choice_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AlternativeCastingMethod(_) => {
            lower_alternative_casting_method_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Triggered { .. } => {
            lower_triggered_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
    }
}

fn lower_abilities_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        ..
    } = input;
    let NormalizedLineChunk::Abilities(actions) = parsed else {
        unreachable!("abilities lowerer received mismatched chunk");
    };

    let keyword_segment = info
        .raw_line
        .split('(')
        .next()
        .unwrap_or(info.raw_line.as_str());
    let separator = if super::str_find(keyword_segment, ";").is_some() {
        "; "
    } else {
        ", "
    };
    let line_text = if actions
        .iter()
        .any(|action| matches!(action, crate::cards::builders::KeywordAction::Crew { .. }))
    {
        Some(keyword_segment.trim().to_string())
    } else {
        super::keyword_actions_line_text(&actions, separator)
    };
    for action in actions {
        let ability_count_before = builder.abilities.len();
        builder = builder.apply_keyword_action(action);
        if let Some(line_text) = line_text.as_ref() {
            for ability in &mut builder.abilities[ability_count_before..] {
                ability.text = Some(line_text.clone());
            }
        }
    }
    Ok(builder)
}

fn compile_static_ability_with_zones(
    ability: crate::static_abilities::StaticAbility,
    info: &LineInfo,
) -> Ability {
    let mut compiled = Ability::static_ability(ability).with_text(info.raw_line.as_str());
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_spell_only_functional_zones(static_ability)
    {
        compiled = compiled.in_zones(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_all_zone_functional_zones(static_ability)
    {
        compiled = compiled.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_referenced_ability_functional_zones(
            static_ability,
            info.normalized.normalized.as_str(),
        )
    {
        compiled = compiled.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let Some(zones) =
        super::infer_static_ability_functional_zones(info.normalized.normalized.as_str())
    {
        compiled = compiled.in_zones(zones);
    }
    compiled
}

fn lower_static_ability_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::StaticAbility(ability) = parsed else {
        unreachable!("static-ability lowerer received mismatched chunk");
    };

    let ability = match super::rewrite_lower_static_ability_ast(ability) {
        Ok(ability) => ability,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    Ok(builder.with_ability(compile_static_ability_with_zones(ability, info)))
}

fn lower_static_abilities_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::StaticAbilities(abilities) = parsed else {
        unreachable!("static-abilities lowerer received mismatched chunk");
    };

    let abilities = match super::rewrite_lower_static_abilities_ast(abilities) {
        Ok(abilities) => abilities,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    for ability in abilities {
        builder = builder.with_ability(compile_static_ability_with_zones(ability, info));
    }
    Ok(builder)
}

fn lower_parsed_ability_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        annotations,
        ..
    } = input;
    let NormalizedLineChunk::Ability(parsed_ability) = parsed else {
        unreachable!("ability lowerer received mismatched chunk");
    };

    let parsed_ability = super::rewrite_lower_prepared_ability(parsed_ability)?;
    if let Some(effects_ast) = parsed_ability.effects_ast.as_ref().map(Vec::as_slice) {
        super::collect_tag_spans_from_effects_with_context(
            effects_ast,
            annotations,
            &info.normalized,
        );
    }
    let mut ability = parsed_ability.into_runtime();
    if ability.text.is_none() {
        ability = ability.with_text(info.raw_line.as_str());
    }
    builder = builder.with_ability(ability);
    Ok(builder)
}

fn lower_statement_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::Statement {
        effects_ast,
        prepared,
    } = parsed
    else {
        unreachable!("statement lowerer received mismatched chunk");
    };

    if effects_ast.is_empty() {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "empty effect statement".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to empty effect statement: '{}'",
            info.raw_line
        )));
    }
    if let Some(enchant_filter) = effects_ast.iter().find_map(|effect| {
        if let EffectAst::Enchant { filter } = effect {
            Some(filter.clone())
        } else {
            None
        }
    }) {
        builder.aura_attach_filter = Some(enchant_filter);
    }
    let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
        Ok(lowered) => lowered,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    super::rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &lowered,
        false,
        "spell text effects",
    )?;
    let compiled = lowered.effects;
    state.latest_spell_exports = lowered.exports;

    let normalized_line = info.normalized.normalized.as_str().to_ascii_lowercase();
    let instead_semantics = super::classify_instead_followup_text(&normalized_line);
    let trailing_instead_if_condition = if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) {
        compile_trailing_instead_if_condition(&normalized_line, info.line_index, &prepared)?
    } else {
        None
    };
    if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && builder.spell_effect.is_none()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[1])
        && replacement.if_false.is_empty()
    {
        let previous = compiled[0].clone();
        let mut replacement = replacement;
        if let Some(previous_target) = super::extract_previous_replacement_target(&previous) {
            replacement.if_true =
                retarget_replacement_effects(replacement.if_true, &previous_target);
        }
        let mut spell_effect = crate::resolution::ResolutionProgram::from_effects(vec![previous]);
        let Some(segment) = spell_effect.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for inline self-replacement"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                replacement.condition,
                replacement.if_true,
            ));
        builder.spell_effect = Some(spell_effect);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[1])
        && replacement.if_false.is_empty()
    {
        let mut replacement = replacement;
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
            .or_else(|| super::extract_previous_replacement_target(&compiled[0]))
        {
            replacement.if_true =
                retarget_replacement_effects(replacement.if_true, &previous_target);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for repeated self-replacement"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                replacement.condition,
                replacement.if_true,
            ));
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(condition) = trailing_instead_if_condition
    {
        let mut replacement_effects = vec![compiled[1].clone()];
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
            .or_else(|| super::extract_previous_replacement_target(&compiled[0]))
        {
            replacement_effects =
                retarget_replacement_effects(replacement_effects, &previous_target);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for plain instead-if follow-up"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                condition,
                replacement_effects,
            ));
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && let Some(mut replacement) = materialized_self_replacement_followup(&compiled)
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
    {
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
        {
            replacement.replacement_effects =
                retarget_replacement_effects(replacement.replacement_effects, &previous_target);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for materialized self-replacement"
                    .to_string(),
            ));
        };
        segment.self_replacements.push(replacement);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && builder.spell_effect.is_none()
        && (normalized_line.starts_with("if ")
            || conditional_self_replacement_followup(&compiled[0])
                .is_some_and(|replacement| replacement.if_false.is_empty()))
    {
        return Err(CardTextError::UnsupportedLine(
            "unsupported self-replacement follow-up without a prior spell segment".to_string(),
        ));
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && builder.spell_effect.is_none()
        && materialized_self_replacement_followup(&compiled).is_some()
    {
        return Err(CardTextError::UnsupportedLine(
            "unsupported self-replacement follow-up without a prior spell segment".to_string(),
        ));
    }
    if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[0])
        && replacement.if_false.is_empty()
    {
        let mut replacement = replacement;
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
        {
            replacement.if_true = replacement
                .if_true
                .into_iter()
                .map(|effect| {
                    if let Some(replacement_damage) =
                        effect.downcast_ref::<crate::effects::DealDamageEffect>()
                        && replacement_damage.target
                            == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
                    {
                        crate::effect::Effect::deal_damage(
                            replacement_damage.amount.clone(),
                            previous_target.clone(),
                        )
                    } else {
                        super::rewrite_replacement_effect_target(&effect, &previous_target)
                            .unwrap_or(effect)
                    }
                })
                .collect();
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for self-replacement".to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                replacement.condition,
                replacement.if_true,
            ));
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.extend(compiled);
    } else {
        builder.spell_effect = Some(compiled);
    }
    Ok(builder)
}

fn lower_additional_cost_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::AdditionalCost {
        effects_ast,
        prepared,
    } = parsed
    else {
        unreachable!("additional-cost lowerer received mismatched chunk");
    };

    if effects_ast.is_empty() {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "empty additional cost statement".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to empty additional-cost statement: '{}'",
            info.raw_line
        )));
    }
    let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
        Ok(lowered) => lowered,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    let compiled = super::runtime_effects_to_costs(lowered.effects.to_vec())?;
    state.latest_additional_cost_exports = lowered.exports;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.extend(compiled);
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    Ok(builder)
}

fn lower_optional_cost_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        builder, parsed, ..
    } = input;
    let NormalizedLineChunk::OptionalCost(cost) = parsed else {
        unreachable!("optional-cost lowerer received mismatched chunk");
    };
    Ok(builder.optional_cost(cost))
}

fn lower_gift_keyword_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::GiftKeyword {
        cost,
        prepared,
        followup_text,
        timing,
    } = parsed
    else {
        unreachable!("gift-keyword lowerer received mismatched chunk");
    };

    builder = builder.optional_cost(cost);
    match timing {
        GiftTimingAst::SpellResolution => {
            let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
                Ok(lowered) => lowered,
                Err(err) if allow_unsupported => {
                    return Ok(super::push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let mut gift_effects = lowered.effects.to_vec();
            gift_effects.push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            let gift_effect = crate::effect::Effect::conditional(
                crate::ConditionExpr::ThisSpellPaidLabel("Gift".to_string()),
                gift_effects,
                Vec::new(),
            );
            if let Some(ref mut existing) = builder.spell_effect {
                existing.push(gift_effect);
            } else {
                builder.spell_effect =
                    Some(crate::resolution::ResolutionProgram::from_effects(vec![
                        gift_effect,
                    ]));
            }
        }
        GiftTimingAst::PermanentEtb => {
            let parsed = super::rewrite_parsed_triggered_ability(
                TriggerSpec::ThisEntersBattlefield,
                prepared.effects.clone(),
                vec![Zone::Battlefield],
                Some(format!(
                    "When this permanent enters, if the gift was promised, {followup_text}"
                )),
                Some(crate::ConditionExpr::ThisSpellPaidLabel("Gift".to_string())),
                prepared.imports.clone(),
            );
            let parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
                parsed,
                prepared: Some(NormalizedPreparedAbility::Triggered {
                    trigger: TriggerSpec::ThisEntersBattlefield,
                    prepared: super::super::effect_pipeline::PreparedTriggeredEffectsForLowering {
                        prepared,
                        intervening_if: None,
                    },
                }),
            }) {
                Ok(parsed) => parsed,
                Err(err) if allow_unsupported => {
                    return Ok(super::push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let mut parsed = parsed;
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                triggered
                    .effects
                    .push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            }
            builder = builder.with_ability(parsed.into_runtime());
        }
    }
    Ok(builder)
}

fn lower_optional_cost_with_cast_trigger_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::OptionalCostWithCastTrigger {
        cost,
        prepared,
        followup_text,
    } = parsed
    else {
        unreachable!("optional-cost-cast-trigger lowerer received mismatched chunk");
    };

    let cost_label = cost.label.clone();
    builder = builder.optional_cost(cost);
    let parsed = super::rewrite_parsed_triggered_ability(
        TriggerSpec::YouCastThisSpell,
        prepared.effects.clone(),
        vec![Zone::Stack],
        Some(followup_text),
        Some(crate::ConditionExpr::ThisSpellPaidLabel(cost_label)),
        prepared.imports.clone(),
    );
    let parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered {
            trigger: TriggerSpec::YouCastThisSpell,
            prepared: super::super::effect_pipeline::PreparedTriggeredEffectsForLowering {
                prepared,
                intervening_if: None,
            },
        }),
    }) {
        Ok(parsed) => parsed,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    Ok(builder.with_ability(parsed.into_runtime()))
}

fn lower_additional_cost_choice_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::AdditionalCostChoice { options } = parsed else {
        unreachable!("additional-cost-choice lowerer received mismatched chunk");
    };

    if options.len() < 2 {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "additional cost choice requires at least two options".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to invalid additional-cost choice (line: '{}')",
            info.raw_line
        )));
    }
    for option in &options {
        if option.effects_ast.is_empty() {
            if allow_unsupported {
                return Ok(super::push_unsupported_marker(
                    builder,
                    info.raw_line.as_str(),
                    "additional cost choice option produced no effects".to_string(),
                ));
            }
            return Err(CardTextError::ParseError(format!(
                "line parsed to empty additional-cost option (line: '{}')",
                info.raw_line
            )));
        }
    }
    let (modes, exports) =
        match super::rewrite_lower_prepared_additional_cost_choice_modes_with_exports(&options) {
            Ok(outputs) => outputs,
            Err(err) if allow_unsupported => {
                return Ok(super::push_unsupported_marker(
                    builder,
                    info.raw_line.as_str(),
                    format!("{err:?}"),
                ));
            }
            Err(err) => return Err(err),
        };
    state.latest_additional_cost_exports = exports;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.push(
        crate::costs::Cost::try_from_runtime_effect(crate::effect::Effect::choose_one(modes))
            .map_err(CardTextError::ParseError)?,
    );
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    Ok(builder)
}

fn lower_alternative_casting_method_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        ..
    } = input;
    let NormalizedLineChunk::AlternativeCastingMethod(method) = parsed else {
        unreachable!("alternative-casting-method lowerer received mismatched chunk");
    };
    builder.alternative_casts.push(method);
    Ok(builder)
}

fn lower_triggered_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::Triggered {
        trigger,
        prepared,
        max_triggers_per_turn,
    } = parsed
    else {
        unreachable!("triggered lowerer received mismatched chunk");
    };

    let contains_haunted_creature_dies = matches!(
        &trigger,
        TriggerSpec::Either(_, right) if matches!(**right, TriggerSpec::HauntedCreatureDies)
    ) || matches!(&trigger, TriggerSpec::HauntedCreatureDies);
    let functional_zones = super::infer_triggered_ability_functional_zones(
        &trigger,
        info.normalized.normalized.as_str(),
    );
    let parsed = super::rewrite_parsed_triggered_ability(
        trigger.clone(),
        prepared.prepared.effects.clone(),
        functional_zones,
        Some(info.raw_line.clone()),
        max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
        prepared.prepared.imports.clone(),
    );
    let parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
    }) {
        Ok(parsed) => parsed,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    if contains_haunted_creature_dies && let AbilityKind::Triggered(triggered) = parsed.kind() {
        state.haunt_linkage = Some((triggered.effects.to_vec(), triggered.choices.clone()));
    }
    Ok(builder.with_ability(parsed.into_runtime()))
}
