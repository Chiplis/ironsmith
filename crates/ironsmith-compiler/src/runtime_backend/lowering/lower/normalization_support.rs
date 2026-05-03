use super::*;
use crate::cards::builders::SubjectVerbEffectAst;

fn predicate_contains_source_match(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::SourceMatches(_) => true,
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_contains_source_match(left) || predicate_contains_source_match(right)
        }
        PredicateAst::Not(inner) => predicate_contains_source_match(inner),
        _ => false,
    }
}

fn predicate_object_filter_antecedent(predicate: &PredicateAst) -> Option<ObjectFilter> {
    match predicate {
        PredicateAst::PlayerControls { filter, .. }
        | PredicateAst::PlayerControlsAtLeast { filter, .. }
        | PredicateAst::PlayerControlsExactly { filter, .. }
        | PredicateAst::PlayerControlsAtLeastWithDifferentPowers { filter, .. }
        | PredicateAst::PlayerControlsNo { filter, .. }
        | PredicateAst::PlayerControlsMost { filter, .. } => Some(filter.clone()),
        PredicateAst::ValueComparison {
            left: crate::effect::Value::Count(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            ..
        } => Some(filter.clone()),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_object_filter_antecedent(left)
                .or_else(|| predicate_object_filter_antecedent(right))
        }
        PredicateAst::Not(inner) => predicate_object_filter_antecedent(inner),
        _ => None,
    }
}

fn merge_filter_overlay(base: &mut ObjectFilter, overlay: ObjectFilter) {
    if let Some(zone) = overlay.zone {
        base.zone.get_or_insert(zone);
    }
    if base.controller.is_none() {
        base.controller = overlay.controller;
    }
    if base.owner.is_none() {
        base.owner = overlay.owner;
    }
    base.other |= overlay.other;
    for card_type in overlay.card_types {
        if !base.card_types.contains(&card_type) {
            base.card_types.push(card_type);
        }
    }
    for subtype in overlay.subtypes {
        if !base.subtypes.contains(&subtype) {
            base.subtypes.push(subtype);
        }
    }
    if let Some(colors) = overlay.colors {
        base.colors = Some(
            base.colors
                .map_or(colors, |existing| existing.intersection(colors)),
        );
    }
}

fn merge_optional_predicates(
    left: Option<PredicateAst>,
    right: Option<PredicateAst>,
) -> Option<PredicateAst> {
    match (left, right) {
        (Some(left), Some(right)) => Some(PredicateAst::And(Box::new(left), Box::new(right))),
        (Some(predicate), None) | (None, Some(predicate)) => Some(predicate),
        (None, None) => None,
    }
}

fn is_stack_object_targeting_filter(filter: &ObjectFilter) -> bool {
    filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || filter.targets_only_player.is_some()
        || filter.targets_only_object.is_some()
        || filter.target_count.is_some()
}

fn is_stack_object_targeting_predicate(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::ItMatches(filter) => is_stack_object_targeting_filter(filter),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            is_stack_object_targeting_predicate(left) && is_stack_object_targeting_predicate(right)
        }
        _ => false,
    }
}

fn merge_spell_cast_trigger_filter(base: &mut ObjectFilter, overlay: ObjectFilter) {
    if let Some(zone) = overlay.zone {
        base.zone.get_or_insert(zone);
    }
    if base.stack_kind.is_none() {
        base.stack_kind = overlay.stack_kind;
    }
    base.has_mana_cost |= overlay.has_mana_cost;
    for card_type in overlay.card_types {
        if !base.card_types.contains(&card_type) {
            base.card_types.push(card_type);
        }
    }
    for card_type in overlay.all_card_types {
        if !base.all_card_types.contains(&card_type) {
            base.all_card_types.push(card_type);
        }
    }
    for card_type in overlay.excluded_card_types {
        if !base.excluded_card_types.contains(&card_type) {
            base.excluded_card_types.push(card_type);
        }
    }
    if base.targets_player.is_none() {
        base.targets_player = overlay.targets_player;
    }
    if base.targets_object.is_none() {
        base.targets_object = overlay.targets_object;
    }
    base.targets_any_of |= overlay.targets_any_of;
    if base.targets_only_player.is_none() {
        base.targets_only_player = overlay.targets_only_player;
    }
    if base.targets_only_object.is_none() {
        base.targets_only_object = overlay.targets_only_object;
    }
    base.targets_only_any_of |= overlay.targets_only_any_of;
    if base.target_count.is_none() {
        base.target_count = overlay.target_count;
    }
}

fn absorb_predicate_into_trigger(
    trigger: TriggerSpec,
    predicate: PredicateAst,
) -> (TriggerSpec, Option<PredicateAst>) {
    match predicate {
        PredicateAst::And(left, right) => {
            let (trigger, left_remainder) = absorb_predicate_into_trigger(trigger, *left);
            let (trigger, right_remainder) = absorb_predicate_into_trigger(trigger, *right);
            (
                trigger,
                merge_optional_predicates(left_remainder, right_remainder),
            )
        }
        PredicateAst::Or(left, right) => {
            let (trigger_after_left, left_remainder) =
                absorb_predicate_into_trigger(trigger.clone(), (*left).clone());
            let (trigger_after_right, right_remainder) =
                absorb_predicate_into_trigger(trigger, (*right).clone());
            if left_remainder.is_none() && right_remainder.is_none() {
                (trigger_after_left, None)
            } else {
                (
                    trigger_after_right,
                    Some(PredicateAst::Or(
                        Box::new(left_remainder.unwrap_or(*left)),
                        Box::new(right_remainder.unwrap_or(*right)),
                    )),
                )
            }
        }
        PredicateAst::ItMatches(filter) if is_stack_object_targeting_filter(&filter) => {
            match trigger {
                TriggerSpec::SpellCast {
                    filter: trigger_filter,
                    caster,
                    during_turn,
                    min_spells_this_turn,
                    exact_spells_this_turn,
                    from_not_hand,
                } => {
                    let mut merged_filter = trigger_filter.unwrap_or_else(ObjectFilter::spell);
                    merge_spell_cast_trigger_filter(&mut merged_filter, filter);
                    (
                        TriggerSpec::SpellCast {
                            filter: Some(merged_filter),
                            caster,
                            during_turn,
                            min_spells_this_turn,
                            exact_spells_this_turn,
                            from_not_hand,
                        },
                        None,
                    )
                }
                other => (other, Some(PredicateAst::ItMatches(filter))),
            }
        }
        other => (trigger, Some(other)),
    }
}

fn absorb_single_conditional_effect_into_trigger(
    trigger: TriggerSpec,
    effects: Vec<EffectAst>,
) -> (TriggerSpec, Vec<EffectAst>) {
    if effects.len() != 1 {
        return (trigger, effects);
    }

    let mut effects = effects;
    let Some(effect) = effects.pop() else {
        return (trigger, Vec::new());
    };
    match effect {
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } if if_false.is_empty() => {
            let (trigger, predicate) = absorb_predicate_into_trigger(trigger, predicate);
            if let Some(predicate) = predicate {
                (
                    trigger,
                    vec![EffectAst::Conditional {
                        predicate,
                        if_true,
                        if_false: Vec::new(),
                    }],
                )
            } else {
                (trigger, if_true)
            }
        }
        other => (trigger, vec![other]),
    }
}

fn bind_condition_filter_antecedent(filter: &mut ObjectFilter, antecedent: &ObjectFilter) {
    let references_it = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == IT_TAG
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    });
    if !references_it {
        return;
    }

    let mut overlay = filter.clone();
    overlay.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == IT_TAG
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            ))
    });
    let mut replacement = antecedent.clone();
    merge_filter_overlay(&mut replacement, overlay);
    *filter = replacement;
}

fn bind_condition_target_antecedent(target: &mut TargetAst, antecedent: &ObjectFilter) {
    match target {
        TargetAst::Object(filter, _, _) => bind_condition_filter_antecedent(filter, antecedent),
        TargetAst::WithCount(inner, _) => bind_condition_target_antecedent(inner, antecedent),
        _ => {}
    }
}

fn bind_condition_antecedent_in_effect(effect: &mut EffectAst, antecedent: &ObjectFilter) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. } => {
                bind_condition_target_antecedent(target, antecedent);
            }
            _ => {}
        },
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, .. } => {
            bind_condition_filter_antecedent(filter, antecedent);
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            bind_condition_antecedent_in_effects(if_true, antecedent);
            bind_condition_antecedent_in_effects(if_false, antecedent);
        }
        _ => {}
    }
}

fn bind_condition_antecedent_in_effects(effects: &mut [EffectAst], antecedent: &ObjectFilter) {
    for effect in effects {
        bind_condition_antecedent_in_effect(effect, antecedent);
    }
}

fn retarget_it_animation_to_source(effect: EffectAst) -> EffectAst {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeBasePtCreature {
                    power,
                    toughness,
                    target,
                    card_types,
                    subtypes,
                    colors,
                    abilities,
                    granted_abilities,
                    duration,
                },
            ..
        }) => {
            let target = match target {
                TargetAst::Tagged(tag, span) if tag.as_str() == IT_TAG => TargetAst::Source(span),
                other => other,
            };
            EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                card_types,
                subtypes,
                colors,
                abilities,
                granted_abilities,
                duration,
            )
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => EffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(retarget_it_animation_to_source)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(retarget_it_animation_to_source)
                .collect(),
        },
        EffectAst::IfResult { predicate, effects } => EffectAst::IfResult {
            predicate,
            effects: effects
                .into_iter()
                .map(retarget_it_animation_to_source)
                .collect(),
        },
        other => other,
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct RewriteNormalizationState {
    latest_spell_exports: ReferenceExports,
    latest_additional_cost_exports: ReferenceExports,
}

impl RewriteNormalizationState {
    fn statement_reference_imports(&self) -> ReferenceImports {
        let additional_cost_imports = self.latest_additional_cost_exports.to_imports();
        if !additional_cost_imports.is_empty() {
            return additional_cost_imports.into();
        }
        self.latest_spell_exports.to_imports().into()
    }
}

fn normalize_rewrite_parsed_ability(
    parsed: ParsedAbility,
) -> Result<NormalizedParsedAbility, CardTextError> {
    let prepared = match parsed.effects_ast.as_ref() {
        None => None,
        Some(_)
            if matches!(
                parsed.kind(),
                AbilityKind::Activated(activated)
                    if !activated.effects.is_empty() || !activated.choices.is_empty()
            ) =>
        {
            None
        }
        Some(_)
            if matches!(
                parsed.kind(),
                AbilityKind::Triggered(triggered)
                    if !triggered.effects.is_empty() || !triggered.choices.is_empty()
            ) =>
        {
            None
        }
        Some(effects_ast) => match (parsed.kind(), parsed.trigger_spec.as_ref()) {
            (AbilityKind::Triggered(_), Some(trigger)) => {
                let (trigger, prepared) = rewrite_prepare_triggered_effects_for_lowering(
                    trigger.clone(),
                    effects_ast,
                    parsed.reference_imports.clone(),
                )?;
                Some(NormalizedPreparedAbility::Triggered { trigger, prepared })
            }
            (AbilityKind::Activated(_), _) => Some(NormalizedPreparedAbility::Activated(
                rewrite_prepare_effects_with_trigger_context_for_lowering(
                    None,
                    effects_ast,
                    parsed.reference_imports.clone(),
                )?,
            )),
            _ => None,
        },
    };

    Ok(NormalizedParsedAbility { parsed, prepared })
}

fn normalize_rewrite_line_ast(
    info: crate::cards::builders::LineInfo,
    chunks: Vec<LineAst>,
    restrictions: ParsedRestrictions,
    state: &mut RewriteNormalizationState,
) -> Result<NormalizedLineAst, CardTextError> {
    let mut normalized_chunks = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        normalized_chunks.push(match chunk {
            LineAst::Abilities(actions) => NormalizedLineChunk::Abilities(actions),
            LineAst::StaticAbility(ability) => NormalizedLineChunk::StaticAbility(ability),
            LineAst::StaticAbilities(abilities) => NormalizedLineChunk::StaticAbilities(abilities),
            LineAst::Ability(parsed) => {
                NormalizedLineChunk::Ability(normalize_rewrite_parsed_ability(parsed)?)
            }
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => {
                let (trigger, prepared) = rewrite_prepare_triggered_effects_for_lowering(
                    trigger,
                    &effects,
                    ReferenceImports::default(),
                )?;
                NormalizedLineChunk::Triggered {
                    trigger,
                    prepared,
                    max_triggers_per_turn,
                }
            }
            LineAst::Statement { effects } => {
                let prepared = rewrite_prepare_effects_for_lowering(
                    &effects,
                    state.statement_reference_imports(),
                )?;
                state.latest_spell_exports = prepared.exports.clone();
                NormalizedLineChunk::Statement {
                    effects_ast: effects,
                    prepared,
                }
            }
            LineAst::AdditionalCost { effects } => {
                let effects = rewrite_normalize_additional_cost_sacrifice_tags(effects);
                let prepared =
                    rewrite_prepare_effects_for_lowering(&effects, ReferenceImports::default())?;
                state.latest_additional_cost_exports = prepared.exports.clone();
                NormalizedLineChunk::AdditionalCost {
                    effects_ast: effects,
                    prepared,
                }
            }
            LineAst::OptionalCost(cost) => NormalizedLineChunk::OptionalCost(cost.into_runtime()),
            LineAst::GiftKeyword {
                cost,
                effects,
                followup_text,
                timing,
            } => {
                let prepared =
                    rewrite_prepare_effects_for_lowering(&effects, ReferenceImports::default())?;
                NormalizedLineChunk::GiftKeyword {
                    cost: cost.into_runtime(),
                    prepared,
                    followup_text,
                    timing,
                }
            }
            LineAst::OptionalCostWithCastTrigger {
                cost,
                effects,
                followup_text,
            } => {
                let prepared = rewrite_prepare_effects_for_lowering(
                    &effects,
                    state.latest_additional_cost_exports.to_imports(),
                )?;
                NormalizedLineChunk::OptionalCostWithCastTrigger {
                    cost: cost.into_runtime(),
                    prepared,
                    followup_text,
                }
            }
            LineAst::AdditionalCostChoice { options } => {
                let mut normalized_options = Vec::with_capacity(options.len());
                let mut exports = ReferenceExports::default();
                let mut saw_option = false;
                for option in options {
                    let prepared = rewrite_prepare_effects_for_lowering(
                        &option.effects,
                        ReferenceImports::default(),
                    )?;
                    exports = if saw_option {
                        ReferenceExports::join(&exports, &prepared.exports)
                    } else {
                        saw_option = true;
                        prepared.exports.clone()
                    };
                    normalized_options.push(NormalizedAdditionalCostChoiceOptionAst {
                        description: option.description,
                        effects_ast: option.effects,
                        prepared,
                    });
                }
                state.latest_additional_cost_exports = exports;
                NormalizedLineChunk::AdditionalCostChoice {
                    options: normalized_options,
                }
            }
            LineAst::AlternativeCastingMethod(method) => {
                NormalizedLineChunk::AlternativeCastingMethod(method.into_runtime())
            }
        });
    }

    Ok(NormalizedLineAst {
        info,
        chunks: normalized_chunks,
        restrictions,
    })
}

fn normalize_rewrite_modal_ast(modal: ParsedModalAst) -> Result<NormalizedModalAst, CardTextError> {
    let prepared_prefix = if modal.header.prefix_effects_ast.is_empty() {
        None
    } else if modal.header.trigger.is_some() || modal.header.activated.is_some() {
        Some(rewrite_prepare_effects_with_trigger_context_for_lowering(
            modal.header.trigger.as_ref(),
            &modal.header.prefix_effects_ast,
            ReferenceImports::default(),
        )?)
    } else {
        Some(rewrite_prepare_effects_for_lowering(
            &modal.header.prefix_effects_ast,
            ReferenceImports::default(),
        )?)
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let prepared =
            rewrite_prepare_effects_for_lowering(&mode.effects_ast, ReferenceImports::default())?;
        modes.push(NormalizedModalModeAst {
            info: mode.info,
            description: mode.description,
            prepared,
        });
    }

    Ok(NormalizedModalAst {
        header: modal.header,
        prepared_prefix,
        modes,
    })
}

pub(super) fn apply_chosen_option_to_triggered_chunk(
    chunk: LineAst,
    full_text: &str,
    max_triggers_per_turn: Option<u32>,
    chosen_option_label: Option<&str>,
    presentation_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    let max_condition =
        crate::runtime_backend::trigger_frequency_condition(Some(full_text), max_triggers_per_turn);
    let combined_condition = match (chosen_option_label, max_condition.clone()) {
        (Some(label), Some(max)) => Some(crate::ConditionExpr::And(
            Box::new(crate::ConditionExpr::SourceChosenOption(label.to_string())),
            Box::new(max),
        )),
        (Some(label), None) => Some(crate::ConditionExpr::SourceChosenOption(label.to_string())),
        (None, Some(max)) => Some(max),
        (None, None) => None,
    };

    match chunk {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: chunk_max_triggers_per_turn,
        } => {
            let merged_max_condition = chunk_max_triggers_per_turn
                .or(max_triggers_per_turn)
                .and_then(|count| {
                    crate::runtime_backend::trigger_frequency_condition(
                        Some(full_text),
                        Some(count),
                    )
                });
            let merged_condition = match (chosen_option_label, merged_max_condition) {
                (Some(label), Some(max)) => Some(crate::ConditionExpr::And(
                    Box::new(crate::ConditionExpr::SourceChosenOption(label.to_string())),
                    Box::new(max),
                )),
                (Some(label), None) => {
                    Some(crate::ConditionExpr::SourceChosenOption(label.to_string()))
                }
                (None, Some(max)) => Some(max),
                (None, None) => None,
            };
            Ok(LineAst::Ability(rewrite_parsed_triggered_ability(
                trigger.clone(),
                effects,
                infer_rewrite_triggered_functional_zones(&trigger, full_text),
                Some(full_text.to_string()),
                merged_condition,
                presentation_label,
                ReferenceImports::default(),
            )))
        }
        LineAst::Ability(mut parsed) => {
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                rewrite_do_this_trigger_frequency_surface(full_text, triggered);
            }
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut()
                && let Some(condition) = combined_condition
            {
                triggered.intervening_if = Some(match triggered.intervening_if.take() {
                    Some(existing) => {
                        crate::ConditionExpr::And(Box::new(existing), Box::new(condition))
                    }
                    None => condition,
                });
            }
            if parsed.text().is_none() {
                *parsed.text_mut() = Some(full_text.to_string());
            }
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut()
                && triggered.presentation_label.is_none()
            {
                triggered.presentation_label = presentation_label.map(str::to_string);
            }
            Ok(LineAst::Ability(parsed))
        }
        other => Ok(other),
    }
}

fn rewrite_do_this_trigger_frequency_surface(
    full_text: &str,
    triggered: &mut crate::ability::TriggeredAbility,
) {
    let normalized = full_text.trim().to_ascii_lowercase();
    if !normalized.contains("do this only once each turn")
        && !normalized.contains("do this only twice each turn")
    {
        return;
    }
    let Some(condition) = triggered.intervening_if.take() else {
        return;
    };
    triggered.intervening_if = Some(match condition {
        crate::ConditionExpr::MaxTimesEachTurn(1) => {
            crate::ConditionExpr::DoThisMaxTimesEachTurn(1)
        }
        crate::ConditionExpr::MaxTimesEachTurn(2) => {
            crate::ConditionExpr::DoThisMaxTimesEachTurn(2)
        }
        other => other,
    });
}

pub(super) fn apply_explicit_intervening_if_to_triggered_chunk(
    chunk: LineAst,
    explicit_intervening_if: Option<PredicateAst>,
) -> Result<LineAst, CardTextError> {
    let Some(predicate) = explicit_intervening_if else {
        return Ok(chunk);
    };

    match chunk {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => {
            let (trigger, predicate) = absorb_predicate_into_trigger(trigger, predicate);
            let (trigger, effects) =
                absorb_single_conditional_effect_into_trigger(trigger, effects);
            let Some(predicate) = predicate else {
                return Ok(LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                });
            };
            let mut effects = effects;
            if let Some(antecedent) = predicate_object_filter_antecedent(&predicate) {
                bind_condition_antecedent_in_effects(&mut effects, &antecedent);
            }
            if predicate_contains_source_match(&predicate) {
                effects = effects
                    .into_iter()
                    .map(retarget_it_animation_to_source)
                    .collect();
            }
            if matches!(
                effects.as_slice(),
                [EffectAst::Conditional { if_false, .. }] if if_false.is_empty()
            ) {
                Ok(LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                })
            } else {
                Ok(LineAst::Triggered {
                    trigger,
                    effects: vec![EffectAst::Conditional {
                        predicate,
                        if_true: effects,
                        if_false: Vec::new(),
                    }],
                    max_triggers_per_turn,
                })
            }
        }
        LineAst::Ability(mut parsed) => {
            if let Some(mut effects_ast) = parsed.effects_ast.take() {
                parsed.reference_imports.source_object_antecedent |=
                    predicate.establishes_source_object_antecedent();
                if let Some(antecedent) = predicate_object_filter_antecedent(&predicate) {
                    bind_condition_antecedent_in_effects(&mut effects_ast, &antecedent);
                }
                if predicate_contains_source_match(&predicate) {
                    effects_ast = effects_ast
                        .into_iter()
                        .map(retarget_it_animation_to_source)
                        .collect();
                }
                parsed.effects_ast = Some(effects_ast);
            }
            if is_stack_object_targeting_predicate(&predicate) {
                if let Some(effects_ast) = parsed.effects_ast.take() {
                    if let [
                        EffectAst::Conditional {
                            predicate,
                            if_true,
                            if_false,
                        },
                    ] = effects_ast.as_slice()
                        && if_false.is_empty()
                        && is_stack_object_targeting_predicate(predicate)
                    {
                        parsed.effects_ast = Some(if_true.clone());
                    } else {
                        parsed.effects_ast = Some(effects_ast);
                    }
                }
                return Ok(LineAst::Ability(parsed));
            }
            let compiled_condition = compile_condition_from_predicate_ast_with_env(
                &predicate,
                &ReferenceEnv::from_imports(&parsed.reference_imports, false, false, false, None),
                None,
            );
            if let Ok(condition) = compiled_condition {
                if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                    triggered.intervening_if = Some(match triggered.intervening_if.take() {
                        Some(existing) => {
                            crate::ConditionExpr::And(Box::new(existing), Box::new(condition))
                        }
                        None => condition,
                    });
                }
                if let Some(effects_ast) = parsed.effects_ast.take() {
                    if let [
                        EffectAst::Conditional {
                            if_true, if_false, ..
                        },
                    ] = effects_ast.as_slice()
                        && if_false.is_empty()
                    {
                        parsed.effects_ast = Some(if_true.clone());
                    } else {
                        parsed.effects_ast = Some(effects_ast);
                    }
                }
            } else if let Some(effects_ast) = parsed.effects_ast.take() {
                parsed.effects_ast = Some(effects_ast);
            }
            Ok(LineAst::Ability(parsed))
        }
        other => Ok(other),
    }
}

fn rewrite_item_to_parsed_item(
    item: RewriteSemanticItem,
) -> Result<Option<ParsedCardItem>, CardTextError> {
    match item {
        RewriteSemanticItem::Metadata => Ok(None),
        RewriteSemanticItem::Keyword(line) => {
            let parsed =
                super::super::keyword_registry::lower_keyword_line_ast(&line, &line.parse_tokens)?;
            Ok(Some(ParsedCardItem::Line(ParsedLineAst {
                info: line.info.clone(),
                chunks: vec![parsed],
                restrictions: ParsedRestrictions::default(),
            })))
        }
        RewriteSemanticItem::Activated(line) => {
            let lowered = lower_rewrite_activated_to_chunk(
                line.info.clone(),
                line.cost.clone(),
                line.cost_parse_tokens.clone(),
                line.effect_text.clone(),
                line.effect_parse_tokens.clone(),
                line.timing_hint.clone(),
                line.is_loyalty_ability,
                line.chosen_option_label.clone(),
            )?;
            Ok(Some(ParsedCardItem::Line(ParsedLineAst {
                info: line.info.clone(),
                chunks: vec![lowered.chunk],
                restrictions: lowered.restrictions,
            })))
        }
        RewriteSemanticItem::Triggered(line) => {
            let parsed = apply_explicit_intervening_if_to_triggered_chunk(
                lower_rewrite_triggered_to_chunk(
                    line.info.clone(),
                    &line.full_text,
                    &line.full_parse_tokens,
                    &line.trigger_text,
                    &line.trigger_parse_tokens,
                    &line.effect_text,
                    &line.effect_parse_tokens,
                    line.intervening_if.clone(),
                    line.presentation_label.as_deref(),
                    line.max_triggers_per_turn,
                    line.chosen_option_label.as_deref(),
                )?,
                line.intervening_if.clone(),
            )?;
            Ok(Some(ParsedCardItem::Line(ParsedLineAst {
                info: line.info.clone(),
                chunks: vec![parsed],
                restrictions: ParsedRestrictions::default(),
            })))
        }
        RewriteSemanticItem::Static(line) => {
            let (parsed_sentences, restrictions) =
                split_text_for_parse(&line.text, &line.text, line.info.line_index);
            let chunks = if !restrictions.activation.is_empty() || !restrictions.trigger.is_empty()
            {
                if parsed_sentences.is_empty() {
                    Vec::new()
                } else {
                    let parsed_text = parsed_sentences.join(". ");
                    let parsed_tokens = lex_line(&parsed_text, line.info.line_index)?;
                    vec![lower_rewrite_static_to_chunk(
                        line.info.clone(),
                        &parsed_text,
                        &parsed_tokens,
                        line.chosen_option_label.as_deref(),
                    )?]
                }
            } else {
                vec![lower_rewrite_static_to_chunk(
                    line.info.clone(),
                    &line.text,
                    &line.parse_tokens,
                    line.chosen_option_label.as_deref(),
                )?]
            };
            Ok(Some(ParsedCardItem::Line(ParsedLineAst {
                info: line.info.clone(),
                chunks,
                restrictions,
            })))
        }
        RewriteSemanticItem::Statement(line) => {
            let parsed_chunks = lower_rewrite_statement_token_groups_to_chunks(
                line.info.clone(),
                &line.text,
                &line.parse_tokens,
                &line.parse_groups,
            )?;
            Ok(Some(ParsedCardItem::Line(ParsedLineAst {
                info: line.info.clone(),
                chunks: parsed_chunks,
                restrictions: ParsedRestrictions::default(),
            })))
        }
        RewriteSemanticItem::Unsupported(line) => Ok(Some(ParsedCardItem::Line(ParsedLineAst {
            info: line.info.clone(),
            chunks: vec![rewrite_unsupported_line_ast(
                line.info.raw_line.as_str(),
                line.reason_code,
            )],
            restrictions: ParsedRestrictions::default(),
        }))),
        RewriteSemanticItem::Modal(modal) => Ok(Some(lower_rewrite_modal_to_item(modal)?)),
        RewriteSemanticItem::LevelHeader(level) => {
            Ok(Some(ParsedCardItem::LevelAbility(ParsedLevelAbilityAst {
                min_level: level.min_level,
                max_level: level.max_level,
                pt: level.pt,
                items: level.items.into_iter().map(|item| item.parsed).collect(),
            })))
        }
        RewriteSemanticItem::SagaChapter(saga) => Ok(Some(ParsedCardItem::Line(ParsedLineAst {
            info: saga.info.clone(),
            chunks: vec![LineAst::Triggered {
                trigger: TriggerSpec::SagaChapter(saga.chapters),
                effects: saga.effects_ast,
                max_triggers_per_turn: None,
            }],
            restrictions: ParsedRestrictions::default(),
        }))),
    }
}

fn prepare_parsed_item_to_normalized_item(
    item: ParsedCardItem,
    state: &mut RewriteNormalizationState,
) -> Result<NormalizedCardItem, CardTextError> {
    match item {
        ParsedCardItem::Line(line) => Ok(NormalizedCardItem::Line(normalize_rewrite_line_ast(
            line.info,
            line.chunks,
            line.restrictions,
            state,
        )?)),
        ParsedCardItem::Modal(modal) => Ok(NormalizedCardItem::Modal(normalize_rewrite_modal_ast(
            modal,
        )?)),
        ParsedCardItem::LevelAbility(level) => Ok(NormalizedCardItem::LevelAbility(level)),
    }
}

pub(crate) fn rewrite_document_to_parsed_card_ast(
    doc: RewriteSemanticDocument,
) -> Result<ParsedCardAst, CardTextError> {
    let RewriteSemanticDocument {
        builder,
        annotations,
        items,
        allow_unsupported,
    } = doc;
    let mut parsed_items = Vec::new();
    for item in items {
        let maybe_item = rewrite_item_to_parsed_item(item)?;
        if let Some(item) = maybe_item {
            parsed_items.push(item);
        }
    }

    Ok(ParsedCardAst {
        builder,
        annotations,
        items: parsed_items,
        allow_unsupported,
    })
}

pub(crate) fn prepare_parsed_card_ast_for_lowering(
    ast: ParsedCardAst,
) -> Result<NormalizedCardAst, CardTextError> {
    let ParsedCardAst {
        builder,
        annotations,
        items,
        allow_unsupported,
    } = ast;
    let mut state = RewriteNormalizationState::default();
    let mut normalized_items = Vec::new();
    for item in items {
        normalized_items.push(prepare_parsed_item_to_normalized_item(item, &mut state)?);
    }

    Ok(NormalizedCardAst {
        builder,
        annotations,
        items: normalized_items,
        allow_unsupported,
    })
}

#[allow(dead_code)]
pub(crate) fn rewrite_document_to_normalized_card_ast(
    doc: RewriteSemanticDocument,
) -> Result<NormalizedCardAst, CardTextError> {
    prepare_parsed_card_ast_for_lowering(rewrite_document_to_parsed_card_ast(doc)?)
}
