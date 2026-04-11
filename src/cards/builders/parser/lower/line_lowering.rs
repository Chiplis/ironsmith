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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineChunkLoweringKind {
    Abilities,
    StaticAbility,
    StaticAbilities,
    Ability,
    Statement,
    AdditionalCost,
    OptionalCost,
    GiftKeyword,
    OptionalCostWithCastTrigger,
    AdditionalCostChoice,
    AlternativeCastingMethod,
    Triggered,
}

fn line_chunk_kind(chunk: &NormalizedLineChunk) -> LineChunkLoweringKind {
    match chunk {
        NormalizedLineChunk::Abilities(_) => LineChunkLoweringKind::Abilities,
        NormalizedLineChunk::StaticAbility(_) => LineChunkLoweringKind::StaticAbility,
        NormalizedLineChunk::StaticAbilities(_) => LineChunkLoweringKind::StaticAbilities,
        NormalizedLineChunk::Ability(_) => LineChunkLoweringKind::Ability,
        NormalizedLineChunk::Statement { .. } => LineChunkLoweringKind::Statement,
        NormalizedLineChunk::AdditionalCost { .. } => LineChunkLoweringKind::AdditionalCost,
        NormalizedLineChunk::OptionalCost(_) => LineChunkLoweringKind::OptionalCost,
        NormalizedLineChunk::GiftKeyword { .. } => LineChunkLoweringKind::GiftKeyword,
        NormalizedLineChunk::OptionalCostWithCastTrigger { .. } => {
            LineChunkLoweringKind::OptionalCostWithCastTrigger
        }
        NormalizedLineChunk::AdditionalCostChoice { .. } => {
            LineChunkLoweringKind::AdditionalCostChoice
        }
        NormalizedLineChunk::AlternativeCastingMethod(_) => {
            LineChunkLoweringKind::AlternativeCastingMethod
        }
        NormalizedLineChunk::Triggered { .. } => LineChunkLoweringKind::Triggered,
    }
}

struct LineChunkLoweringInput<'a> {
    builder: CardDefinitionBuilder,
    state: &'a mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &'a LineInfo,
    allow_unsupported: bool,
    annotations: &'a mut ParseAnnotations,
}

type LineChunkLowererFn =
    for<'a> fn(LineChunkLoweringInput<'a>) -> Result<CardDefinitionBuilder, CardTextError>;

#[derive(Clone, Copy)]
struct LineChunkLoweringRuleDef {
    id: &'static str,
    priority: u16,
    kind: LineChunkLoweringKind,
    run: LineChunkLowererFn,
}

const LINE_CHUNK_LOWERING_RULES: [LineChunkLoweringRuleDef; 12] = [
    LineChunkLoweringRuleDef {
        id: "abilities",
        priority: 10,
        kind: LineChunkLoweringKind::Abilities,
        run: lower_abilities_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "static-ability",
        priority: 20,
        kind: LineChunkLoweringKind::StaticAbility,
        run: lower_static_ability_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "static-abilities",
        priority: 30,
        kind: LineChunkLoweringKind::StaticAbilities,
        run: lower_static_abilities_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "ability",
        priority: 40,
        kind: LineChunkLoweringKind::Ability,
        run: lower_parsed_ability_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "statement",
        priority: 50,
        kind: LineChunkLoweringKind::Statement,
        run: lower_statement_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "additional-cost",
        priority: 60,
        kind: LineChunkLoweringKind::AdditionalCost,
        run: lower_additional_cost_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "optional-cost",
        priority: 70,
        kind: LineChunkLoweringKind::OptionalCost,
        run: lower_optional_cost_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "gift-keyword",
        priority: 80,
        kind: LineChunkLoweringKind::GiftKeyword,
        run: lower_gift_keyword_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "optional-cost-cast-trigger",
        priority: 90,
        kind: LineChunkLoweringKind::OptionalCostWithCastTrigger,
        run: lower_optional_cost_with_cast_trigger_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "additional-cost-choice",
        priority: 100,
        kind: LineChunkLoweringKind::AdditionalCostChoice,
        run: lower_additional_cost_choice_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "alternative-casting-method",
        priority: 110,
        kind: LineChunkLoweringKind::AlternativeCastingMethod,
        run: lower_alternative_casting_method_chunk,
    },
    LineChunkLoweringRuleDef {
        id: "triggered",
        priority: 120,
        kind: LineChunkLoweringKind::Triggered,
        run: lower_triggered_chunk,
    },
];

pub(super) fn rewrite_apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &LineInfo,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let kind = line_chunk_kind(&parsed);
    let mut candidate_indices = LINE_CHUNK_LOWERING_RULES
        .iter()
        .enumerate()
        .filter(|(_, rule)| rule.kind == kind)
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    candidate_indices.sort_by_key(|idx| LINE_CHUNK_LOWERING_RULES[*idx].priority);
    let candidate_rule_ids = candidate_indices
        .iter()
        .map(|idx| LINE_CHUNK_LOWERING_RULES[*idx].id)
        .collect::<Vec<_>>()
        .join(",");

    for idx in candidate_indices {
        let rule = &LINE_CHUNK_LOWERING_RULES[idx];
        return (rule.run)(LineChunkLoweringInput {
            builder,
            state,
            parsed,
            info,
            allow_unsupported,
            annotations,
        });
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing line chunk lowerer for '{}' [kind={kind:?} candidate_rules={candidate_rule_ids}]",
        info.raw_line
    )))
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
    if let Some(ref effects_ast) = parsed_ability.effects_ast {
        super::collect_tag_spans_from_effects_with_context(
            effects_ast,
            annotations,
            &info.normalized,
        );
    }
    let mut ability = parsed_ability.ability;
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
    if matches!(
        super::classify_instead_followup_text(&normalized_line),
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && builder.spell_effect.is_none()
        && compiled[0]
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .is_some_and(|replacement| replacement.if_false.is_empty())
    {
        return Err(CardTextError::UnsupportedLine(
            "unsupported self-replacement follow-up without a prior spell segment".to_string(),
        ));
    }
    if matches!(
        super::classify_instead_followup_text(&normalized_line),
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(replacement) = compiled[0].downcast_ref::<crate::effects::ConditionalEffect>()
        && replacement.if_false.is_empty()
    {
        let mut replacement = replacement.clone();
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
            if let AbilityKind::Triggered(ref mut triggered) = parsed.ability.kind {
                triggered
                    .effects
                    .push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            }
            builder = builder.with_ability(parsed.ability);
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
    Ok(builder.with_ability(parsed.ability))
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
    if contains_haunted_creature_dies
        && let AbilityKind::Triggered(triggered) = &parsed.ability.kind
    {
        state.haunt_linkage = Some((triggered.effects.to_vec(), triggered.choices.clone()));
    }
    Ok(builder.with_ability(parsed.ability))
}
