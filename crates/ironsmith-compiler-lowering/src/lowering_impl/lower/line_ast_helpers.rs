use super::*;

#[derive(Debug, Clone, Default)]
pub struct RewriteLoweredCardState {
    pub haunt_linkage: Option<(Vec<crate::effect::Effect>, Vec<ChooseSpec>)>,
    pub latest_spell_exports: ReferenceExports,
    pub latest_additional_cost_exports: ReferenceExports,
    pub latest_created_token: Option<(
        String,
        crate::model::token_definition::TokenDefinitionSpec,
        PlayerAst,
    )>,
    pub pending_backups: Vec<PendingBackup>,
    pub pending_cipher: bool,
    /// Last source line that contributed a top-level statement program.
    /// Used only to retain authored line boundaries when adjacent spell
    /// instructions are appended to one resolution program.
    pub latest_statement_line_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingBackup {
    /// Number of already-lowered actual abilities at the source position where
    /// the keyword appeared. Migration-only keyword actions are not abilities.
    pub ability_boundary: usize,
    pub amount: u32,
}

pub fn update_last_restrictable_ability(
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

pub fn lower_level_ability_ast(
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
                    .extend(lower_static_abilities_ast(abilities)?);
            }
            ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                for action in actions {
                    if let Some(ability) = runtime_static_ability_for_keyword_action(action.clone())
                    {
                        lowered.abilities.push(ability);
                    } else {
                        let display = action.display_text();
                        let object_abilities = lower_keyword_action_to_object_abilities(action)?;
                        lowered.abilities.extend(
                            crate::runtime_static_ability_helpers::object_abilities_to_static_carriers(
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
                activated_lines.push(normalize_line_ast_standalone(
                    info,
                    vec![chunk],
                    activated.restrictions,
                    activated.semantic_facts,
                )?);
            }
        }
    }

    Ok(RewriteLoweredLevelAbilityAst {
        level_ability: lowered,
        activated_lines,
    })
}

pub struct RewriteLoweredLevelAbilityAst {
    pub level_ability: crate::ability::LevelAbility,
    pub activated_lines: Vec<NormalizedLineAst>,
}

fn apply_level_range_activation_condition(
    chunk: &mut LineAst,
    min_level: u32,
    max_level: Option<u32>,
) {
    let LineAst::Ability(parsed) = chunk else {
        return;
    };
    let crate::model::CompilerAbilityKindCore::Activated(activated) = parsed.kind_mut() else {
        return;
    };

    let min_condition = PredicateAst::SourceHasCounterAtLeast {
        counter_type: crate::CounterType::Level,
        count: min_level,
        surface: crate::SourceCounterThresholdSurface::SourceHas,
    };
    let level_condition = if let Some(max_level) = max_level {
        PredicateAst::And(
            Box::new(min_condition),
            Box::new(PredicateAst::ValueComparison {
                left: crate::Value::CountersOnSource(crate::CounterType::Level),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: crate::Value::Fixed(max_level as i32),
            }),
        )
    } else {
        min_condition
    };

    activated.activation_condition = Some(match activated.activation_condition.take() {
        Some(existing) => PredicateAst::And(Box::new(existing), Box::new(level_condition)),
        None => level_condition,
    });
    let max_label = max_level
        .map(|level| level.to_string())
        .unwrap_or_else(|| "+".to_string());
    activated
        .additional_restrictions
        .push(format!("__ironsmith_level_range:{min_level}:{max_label}"));
}

pub fn uses_spell_only_functional_zones(static_ability: &StaticAbility) -> bool {
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

pub fn uses_referenced_ability_functional_zones(
    static_ability: &StaticAbility,
    references_this_ability_cost: bool,
) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ActivatedAbilityCostReduction
        && references_this_ability_cost
}

pub fn uses_all_zone_functional_zones(static_ability: &StaticAbility) -> bool {
    matches!(
        static_ability.id(),
        crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
            | crate::static_abilities::StaticAbilityId::CountersRemainAcrossZoneChanges
    )
}

pub fn effect_target_uses_it_reference(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(_) => true,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            effect_target_uses_it_reference(inner)
        }
        ChooseSpec::SurfaceHinted { spec, .. } | ChooseSpec::WithCountValue(spec, _, _) => {
            effect_target_uses_it_reference(spec)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                matches!(
                    constraint.relation,
                    crate::target::TaggedOpbjectRelation::IsTaggedObject
                )
            })
        }
        _ => false,
    }
}

fn replacement_choose_spec_object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => replacement_choose_spec_object_filter(spec),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => Some(filter),
        _ => None,
    }
}

pub fn extract_previous_replacement_target(effect: &crate::effect::Effect) -> Option<ChooseSpec> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return extract_previous_replacement_target(&tagged.effect);
    }
    if let Some(unless_pays) =
        effect.downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
        && let [inner] = unless_pays.effects.as_slice()
    {
        return extract_previous_replacement_target(inner);
    }
    if let Some(counter) = effect.downcast_ref::<crate::effects::CounterEffect>() {
        return Some(counter.target.clone());
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some(damage.target.clone());
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some(destroy.spec.clone());
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        return Some(return_to_hand.spec.clone());
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return Some(exile.spec.clone());
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return Some(move_to_zone.target.clone());
    }
    if let Some(return_to_hand) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        return Some(return_to_hand.target.clone());
    }
    if let Some(return_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        return Some(return_to_battlefield.target.clone());
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        return destroy.target.clone();
    }
    if let Some(modify) = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>() {
        return Some(modify.target.clone());
    }
    if let Some(continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        if let Some(target_spec) = &continuous.target_spec {
            return Some(target_spec.clone());
        }
        if let crate::continuous::EffectTarget::Filter(filter) = &continuous.target {
            return Some(ChooseSpec::Object(filter.clone()));
        }
    }
    None
}

pub fn replacement_effect_with_target(
    effect: &crate::effect::Effect,
    previous_target: &ChooseSpec,
) -> Option<crate::effect::Effect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(rewritten_inner) =
            replacement_effect_with_target(&tagged.effect, previous_target)
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
        let mut damage = damage.clone();
        damage.target = previous_target.clone();
        return Some(crate::effect::Effect::new(damage));
    }
    if let Some(counter) = effect.downcast_ref::<crate::effects::CounterEffect>()
        && effect_target_uses_it_reference(&counter.target)
    {
        return Some(crate::effect::Effect::new(
            crate::effects::CounterEffect::new(previous_target.clone()),
        ));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
        && destroy
            .target
            .as_ref()
            .is_some_and(effect_target_uses_it_reference)
    {
        return Some(crate::effect::Effect::new(
            crate::effects::DestroyNoRegenerationEffect::with_spec(previous_target.clone()),
        ));
    }
    if let Some(continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
        && let crate::continuous::EffectTarget::Filter(replacement_filter) = &continuous.target
        && let Some(previous_filter) = replacement_choose_spec_object_filter(previous_target)
    {
        // "Those ..." is an explicit reference to the entire set affected by
        // the default branch. The noun parsed in the replacement clause only
        // describes that antecedent; it must not become a fresh, broader
        // runtime filter.
        if continuous.set_quantifier_surface == Some(ironsmith_core::SetQuantifierSurface::Those) {
            let mut rewritten = continuous.clone();
            rewritten.target = crate::continuous::EffectTarget::Filter(previous_filter.clone());
            return Some(crate::effect::Effect::new(rewritten));
        }
        // A separately lowered "those creatures get ... instead" branch can
        // lose the antecedent's controller while keeping the same typed set.
        // Restore only that exact provenance loss; an explicitly different
        // replacement filter remains untouched.
        let mut previous_without_controller = previous_filter.clone();
        previous_without_controller.controller = None;
        if &previous_without_controller == replacement_filter
            && previous_filter.controller.is_some()
            && replacement_filter.controller.is_none()
        {
            let mut rewritten = continuous.clone();
            rewritten.target = crate::continuous::EffectTarget::Filter(previous_filter.clone());
            return Some(crate::effect::Effect::new(rewritten));
        }
    }
    None
}

pub fn push_unsupported_marker(
    builder: CardDefinitionBuilder,
    source_display: &str,
    reason: String,
) -> CardDefinitionBuilder {
    crate::parse_loss::record(
        "allow_unsupported_line",
        format!("{} ({reason})", source_display.trim()),
    );
    builder.with_ability(Ability::static_ability(
        StaticAbility::unsupported_parser_line(source_display.trim(), reason),
    ))
}

pub fn apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &crate::model::facts::LineInfo,
    semantic_facts: &crate::model::facts::LineSemanticFacts,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    line_lowering::apply_line_ast(
        builder,
        state,
        parsed,
        info,
        semantic_facts,
        allow_unsupported,
        annotations,
    )
}

pub fn lower_line_ast(
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
            && apply_instead_followup_statement_to_last_ability(
                builder,
                *last_restrictable_ability,
                effects_ast,
            )?
        {
            handled_restrictions_for_new_ability = true;
            continue;
        }
        if let NormalizedLineChunk::Statement { effects_ast, .. } = &parsed
            && apply_delayed_trigger_followup_statement_to_last_ability(
                builder,
                *last_restrictable_ability,
                effects_ast,
            )?
        {
            handled_restrictions_for_new_ability = true;
            continue;
        }

        let abilities_before = builder.abilities.len();
        *builder = apply_line_ast(
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
                )?;
                handled_restrictions_for_new_ability = true;
            }
        }

        update_last_restrictable_ability(builder, abilities_before, last_restrictable_ability);
    }

    if !handled_restrictions_for_new_ability
        && let Some(index) = *last_restrictable_ability
        && index < builder.abilities.len()
    {
        apply_pending_restrictions_to_ability(&mut builder.abilities[index], &mut restrictions)?;
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
        let conditional = StaticAbility::new(crate::static_abilities::CostIncreaseManaCost::<
            crate::ConditionExpr,
        >::new(
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

        let rewritten = replacement_effect_with_target(&replacement, &previous_target)
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
