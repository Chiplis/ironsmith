use super::*;

pub(crate) fn rewrite_unsupported_line_ast(
    raw_line: &str,
    reason: impl Into<String>,
) -> crate::cards::builders::LineAst {
    LineAst::StaticAbility(StaticAbility::unsupported_parser_line(raw_line, reason).into())
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RewriteLoweredCardState {
    pub(crate) haunt_linkage: Option<(Vec<crate::effect::Effect>, Vec<ChooseSpec>)>,
    pub(crate) latest_spell_exports: ReferenceExports,
    pub(crate) latest_additional_cost_exports: ReferenceExports,
    pub(crate) latest_created_token: Option<(
        String,
        crate::runtime_backend::token_definition::TokenDefinitionSpec,
        PlayerAst,
    )>,
    pub(crate) pending_backups: Vec<PendingBackup>,
    pub(crate) pending_cipher: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingBackup {
    /// Number of already-lowered actual abilities at the source position where
    /// the keyword appeared. Migration-only keyword actions are not abilities.
    pub(crate) ability_boundary: usize,
    pub(crate) amount: u32,
}

pub(crate) fn rewrite_update_last_restrictable_ability(
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

pub(crate) fn rewrite_lower_level_ability_ast(
    level: ParsedLevelAbilityAst,
) -> Result<RewriteLoweredLevelAbilityAst, CardTextError> {
    let mut lowered = crate::ability::LevelAbility::new(level.min_level, level.max_level);
    if let Some((power, toughness)) = level.pt {
        lowered = lowered.with_pt(power, toughness);
    }

    let mut activated_lines = Vec::new();
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
            ParsedLevelAbilityItemAst::ActivatedAbility(activated) => {
                let info = activated.info.clone();
                let mut activated = parse_activated_line(
                    info.clone(),
                    activated.cost,
                    activated.cost_parse_tokens,
                    activated.effect_text,
                    activated.effect_parse_tokens,
                    ActivationTiming::AnyTime,
                    false,
                    None,
                    None,
                )?;
                apply_level_range_activation_condition(
                    &mut activated.chunk,
                    level.min_level,
                    level.max_level,
                );
                activated_lines.push(normalize_rewrite_line_ast_standalone(
                    info,
                    vec![activated.chunk],
                    activated.restrictions,
                )?);
            }
        }
    }

    Ok(RewriteLoweredLevelAbilityAst {
        level_ability: lowered,
        activated_lines,
    })
}

pub(crate) struct RewriteLoweredLevelAbilityAst {
    pub(crate) level_ability: crate::ability::LevelAbility,
    pub(crate) activated_lines: Vec<NormalizedLineAst>,
}

fn apply_level_range_activation_condition(
    chunk: &mut LineAst,
    min_level: u32,
    max_level: Option<u32>,
) {
    let LineAst::Ability(parsed) = chunk else {
        return;
    };
    let AbilityKind::Activated(activated) = parsed.kind_mut() else {
        return;
    };

    let min_condition = crate::ConditionExpr::SourceHasCounterAtLeast {
        counter_type: crate::CounterType::Level,
        count: min_level,
    };
    let level_condition = if let Some(max_level) = max_level {
        crate::ConditionExpr::And(
            Box::new(min_condition),
            Box::new(crate::ConditionExpr::ValueComparison {
                left: crate::Value::CountersOnSource(crate::CounterType::Level),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: crate::Value::Fixed(max_level as i32),
            }),
        )
    } else {
        min_condition
    };

    activated.activation_condition = Some(match activated.activation_condition.take() {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(level_condition)),
        None => level_condition,
    });
    let max_label = max_level
        .map(|level| level.to_string())
        .unwrap_or_else(|| "+".to_string());
    activated
        .additional_restrictions
        .push(format!("__ironsmith_level_range:{min_level}:{max_label}"));
}

pub(crate) fn uses_spell_only_functional_zones(static_ability: &StaticAbility) -> bool {
    matches!(
        static_ability.id(),
        crate::static_abilities::StaticAbilityId::ConditionalSpellKeyword
            | crate::static_abilities::StaticAbilityId::CantBeCountered
            | crate::static_abilities::StaticAbilityId::ThisSpellCastRestriction
            | crate::static_abilities::StaticAbilityId::ThisSpellXMaximum
            | crate::static_abilities::StaticAbilityId::ThisSpellCostReduction
            | crate::static_abilities::StaticAbilityId::ThisSpellCostReductionManaCost
    )
}

pub(crate) fn uses_referenced_ability_functional_zones(
    static_ability: &StaticAbility,
    references_this_ability_cost: bool,
) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ActivatedAbilityCostReduction
        && references_this_ability_cost
}

pub(crate) fn uses_all_zone_functional_zones(static_ability: &StaticAbility) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
}

pub(crate) fn effect_target_uses_it_reference(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(_) => true,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            effect_target_uses_it_reference(inner)
        }
        _ => false,
    }
}

pub(crate) fn extract_previous_replacement_target(
    effect: &crate::effect::Effect,
) -> Option<ChooseSpec> {
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
        #[cfg(not(feature = "serialization"))]
        {
            return destroy.target.clone();
        }
        #[cfg(feature = "serialization")]
        {
            return Some(destroy.spec.clone());
        }
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

pub(crate) fn rewrite_replacement_effect_target(
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
        && {
            #[cfg(not(feature = "serialization"))]
            {
                destroy
                    .target
                    .as_ref()
                    .is_some_and(effect_target_uses_it_reference)
            }
            #[cfg(feature = "serialization")]
            {
                effect_target_uses_it_reference(&destroy.spec)
            }
        }
    {
        return Some(crate::effect::Effect::new(
            crate::effects::DestroyNoRegenerationEffect::with_spec(previous_target.clone()),
        ));
    }
    None
}

pub(crate) fn push_unsupported_marker(
    builder: CardDefinitionBuilder,
    raw_line: &str,
    reason: String,
) -> CardDefinitionBuilder {
    crate::parse_loss::record(
        "allow_unsupported_line",
        format!("{} ({reason})", raw_line.trim()),
    );
    builder.with_ability(Ability::static_ability(
        StaticAbility::unsupported_parser_line(raw_line.trim(), reason),
    ))
}

pub(crate) fn rewrite_apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &crate::cards::builders::LineInfo,
    semantic_facts: &crate::runtime_backend::shared_types::LineSemanticFacts,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    line_lowering::rewrite_apply_line_ast(
        builder,
        state,
        parsed,
        info,
        semantic_facts,
        allow_unsupported,
        annotations,
    )
}

pub(crate) fn rewrite_lower_line_ast(
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
        semantic_facts,
    } = line;
    let mut handled_restrictions_for_new_ability = false;

    for parsed in chunks {
        if let NormalizedLineChunk::Statement { effects_ast, .. } = &parsed
            && rewrite_apply_instead_followup_statement_to_last_ability(
                builder,
                *last_restrictable_ability,
                effects_ast,
            )?
        {
            collect_tag_spans_from_effects_with_context(effects_ast, annotations, &info.normalized);
            handled_restrictions_for_new_ability = true;
            continue;
        }
        if let NormalizedLineChunk::Statement { effects_ast, .. } = &parsed
            && rewrite_apply_delayed_trigger_followup_statement_to_last_ability(
                builder,
                *last_restrictable_ability,
                effects_ast,
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
            &semantic_facts,
            allow_unsupported,
            annotations,
        )?;
        let abilities_after = builder.abilities.len();

        for ability_idx in abilities_before..abilities_after {
            if is_restrictable_ability(&builder.abilities[ability_idx]) {
                apply_pending_restrictions_to_ability(
                    &mut builder.abilities[ability_idx],
                    &mut restrictions,
                );
                handled_restrictions_for_new_ability = true;
            }
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

pub(crate) fn lower_compound_buff_and_unblockable_static_chunk(
    _line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(parsed) = effect_grammar::parse_compound_buff_unblockable_tokens(parse_tokens) else {
        return Ok(None);
    };
    let buff_tokens = parsed.buff_tokens.to_vec();
    let mut unblockable_tokens =
        Vec::with_capacity(parsed.subject_tokens.len() + parsed.unblockable_tail_tokens.len());
    unblockable_tokens.extend_from_slice(parsed.subject_tokens);
    unblockable_tokens.extend_from_slice(parsed.unblockable_tail_tokens);

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

pub(crate) fn lower_split_rewrite_static_chunk(
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
        line.chosen_option.as_ref(),
    )
    .map(Some)
}

pub(crate) fn split_statement_label_prefix_for_lowering_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_em_dash_label_prefix_tokens(tokens)
}
