use super::*;

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
    /// Last source line that contributed a top-level statement program.
    /// Used only to retain authored line boundaries when adjacent spell
    /// instructions are appended to one resolution program.
    pub(crate) latest_statement_line_index: Option<usize>,
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
                    if let Some(ability) = rewrite_static_ability_for_keyword_action(action.clone())
                    {
                        lowered.abilities.push(ability);
                    } else {
                        let display = action.display_text();
                        let object_abilities =
                            rewrite_lower_keyword_action_to_object_abilities(action)?;
                        lowered.abilities.extend(
                            crate::runtime_backend::families::static_ability_helpers::object_abilities_to_static_carriers(
                                object_abilities,
                                display,
                            )?,
                        );
                    }
                }
            }
            ParsedLevelAbilityItemAst::ActivatedAbility(activated) => {
                let info = activated.info.clone();
                let mut chunk = activated.chunk;
                apply_level_range_activation_condition(
                    &mut chunk,
                    level.min_level,
                    level.max_level,
                );
                activated_lines.push(normalize_rewrite_line_ast_standalone(
                    info,
                    vec![chunk],
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
        surface: crate::SourceCounterThresholdSurface::SourceHas,
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
            | crate::static_abilities::StaticAbilityId::CostIncreasePerAdditionalTarget
            | crate::static_abilities::StaticAbilityId::CostIncreaseManaCostPerAdditionalTarget
    ) || matches!(
        &static_ability.payload,
        ironsmith_core::StaticAbilityPayload::CostIncrease(increase) if increase.filter.source
    ) || matches!(
        &static_ability.payload,
        ironsmith_core::StaticAbilityPayload::CostIncreaseManaCost(increase)
            if increase.filter.source
    ) || match &static_ability.payload {
        ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
            ability.id == Some(crate::static_abilities::StaticAbilityId::Flash)
                || uses_spell_only_functional_zones(ability)
        }
        _ => false,
    }
}

pub(crate) fn uses_referenced_ability_functional_zones(
    static_ability: &StaticAbility,
    references_this_ability_cost: bool,
) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ActivatedAbilityCostReduction
        && references_this_ability_cost
}

pub(crate) fn uses_all_zone_functional_zones(static_ability: &StaticAbility) -> bool {
    matches!(
        static_ability.id(),
        crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
            | crate::static_abilities::StaticAbilityId::CountersRemainAcrossZoneChanges
    )
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

    for parsed in chunks {
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

        for ability in &mut builder.abilities[abilities_before..] {
            if is_restrictable_ability(ability) {
                apply_pending_restrictions_to_ability(ability, &mut restrictions);
            }
        }
        rewrite_update_last_restrictable_ability(
            builder,
            abilities_before,
            last_restrictable_ability,
        );
    }

    if !restrictions.is_empty()
        && let Some(index) = *last_restrictable_ability
        && let Some(ability) = builder.abilities.get_mut(index)
    {
        apply_pending_restrictions_to_ability(ability, &mut restrictions);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_only_functional_zones_include_target_based_cost_increases() {
        let generic = StaticAbility::cost_increase_per_target_beyond_first(1);
        let colored = StaticAbility::cost_increase_mana_cost_per_target_beyond_first(
            crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::White]),
        );

        assert!(uses_spell_only_functional_zones(&generic));
        assert!(uses_spell_only_functional_zones(&colored));
    }

    #[test]
    fn spell_only_functional_zones_recurse_through_conditionals() {
        let conditional = StaticAbility::new(crate::static_abilities::CostIncreaseManaCost::new(
            ObjectFilter::source(),
            crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::White]),
        ))
        .with_condition(ironsmith_core::Condition::YourTurn);

        assert!(matches!(
            &conditional.payload,
            ironsmith_core::StaticAbilityPayload::Conditional { .. }
        ));

        assert!(uses_spell_only_functional_zones(&conditional));
    }

    #[test]
    fn demonstrative_replacement_reuses_the_entire_previous_set() {
        let previous_filter = ObjectFilter::creature().you_control().other();
        let previous_target = ChooseSpec::Object(previous_filter.clone());
        let replacement_filter = ObjectFilter::creature().match_tagged(
            "triggering",
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        );
        let replacement = crate::effect::Effect::new(
            crate::effects::ApplyContinuousEffect::new_runtime(
                crate::continuous::EffectTarget::Filter(replacement_filter),
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: crate::effect::Value::Fixed(2),
                    toughness: crate::effect::Value::Fixed(0),
                },
                crate::effect::Until::EndOfTurn,
            )
            .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those)),
        );

        let rewritten = rewrite_replacement_effect_target(&replacement, &previous_target)
            .expect("explicit those-set should reuse its antecedent");
        let continuous = rewritten
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("continuous replacement");
        assert_eq!(
            continuous.target,
            crate::continuous::EffectTarget::Filter(previous_filter)
        );
        assert_eq!(
            continuous.set_quantifier_surface,
            Some(ironsmith_core::SetQuantifierSurface::Those)
        );
    }
}
