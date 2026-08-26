use super::*;
use crate::cards::builders::{GrantedAbilityAst, StaticAbilityAst, TargetAst};
#[cfg(test)]
use crate::ir::RewriteSemanticDocument;

#[derive(Debug, Clone, Default)]
pub(super) struct RewriteNormalizationState {
    latest_spell_exports: ReferenceExports,
    latest_additional_cost_exports: ReferenceExports,
}

impl RewriteNormalizationState {
    fn statement_reference_imports(&self) -> ReferenceImports {
        let additional_cost_imports = self.latest_additional_cost_exports.to_imports();
        if !additional_cost_imports.is_empty() {
            return additional_cost_imports;
        }
        self.latest_spell_exports.to_imports()
    }
}

fn materialize_optional_cost(
    cost: crate::model::compiler_semantic::ParsedOptionalCostAst,
) -> Result<crate::cost::OptionalCost, CardTextError> {
    match cost {
        crate::model::compiler_semantic::ParsedOptionalCostAst::LegacyRuntime(cost) => Ok(cost),
        crate::model::compiler_semantic::ParsedOptionalCostAst::Compiler(cost) => {
            Ok(ironsmith_core::OptionalCost {
                kind: cost.kind,
                reference: cost.reference,
                source_label: cost.source_label,
                cost: crate::lowering::cost_materialization::materialize_compiler_core_total_cost(
                    &cost.cost,
                )?,
                repeatable: cost.repeatable,
                returns_to_hand: cost.returns_to_hand,
            })
        }
    }
}

fn materialize_alternative_casting_method(
    method: crate::model::compiler_semantic::ParsedAlternativeCastingMethodAst,
) -> Result<crate::alternative_cast::AlternativeCastingMethod, CardTextError> {
    match method {
        crate::model::compiler_semantic::ParsedAlternativeCastingMethodAst::LegacyRuntime(
            method,
        ) => Ok(method),
        crate::model::compiler_semantic::ParsedAlternativeCastingMethodAst::Compiler(method) => {
            use ironsmith_core::AlternativeCastingMethod as Method;

            let lower_effects = |effects: Vec<EffectAst>| {
                effects
                    .into_iter()
                    .map(crate::lowering_support::lower_compiler_child_effect)
                    .collect::<Result<Vec<_>, _>>()
            };
            let lower_cost = |cost: ironsmith_core::TotalCost<crate::model::CompilerCost>| {
                crate::lowering::cost_materialization::materialize_compiler_core_total_cost(&cost)
            };

            Ok(match method {
                Method::Dash { cost } => Method::Dash { cost },
                Method::Blitz { total_cost } => Method::Blitz {
                    total_cost: lower_cost(total_cost)?,
                },
                Method::Warp { cost } => Method::Warp { cost },
                Method::Plot { cost } => Method::Plot { cost },
                Method::Suspend { cost, time } => Method::Suspend { cost, time },
                Method::Disturb { cost } => Method::Disturb { cost },
                Method::Overload { cost, effects } => Method::Overload {
                    cost,
                    effects: lower_effects(effects)?,
                },
                Method::Cleave { cost, effects } => Method::Cleave {
                    cost,
                    effects: lower_effects(effects)?,
                },
                Method::Awaken {
                    amount,
                    cost,
                    effects,
                } => Method::Awaken {
                    amount,
                    cost,
                    effects: lower_effects(effects)?,
                },
                Method::Flashback { total_cost } => Method::Flashback {
                    total_cost: lower_cost(total_cost)?,
                },
                Method::Harmonize { total_cost } => Method::Harmonize {
                    total_cost: lower_cost(total_cost)?,
                },
                Method::Retrace { total_cost } => Method::Retrace {
                    total_cost: lower_cost(total_cost)?,
                },
                Method::JumpStart { additional_cost } => Method::JumpStart {
                    additional_cost: lower_cost(additional_cost)?,
                },
                Method::Escape {
                    cost,
                    exile_count,
                    additional_cost,
                } => Method::Escape {
                    cost,
                    exile_count,
                    additional_cost: lower_cost(additional_cost)?,
                },
                Method::Madness { cost } => Method::Madness { cost },
                Method::Miracle { cost } => Method::Miracle { cost },
                Method::FlashWithAdditionalCost {
                    additional_cost,
                    total_cost,
                } => Method::FlashWithAdditionalCost {
                    additional_cost,
                    total_cost: lower_cost(total_cost)?,
                },
                Method::Foretell { cost } => Method::Foretell { cost },
                Method::Composed {
                    name,
                    total_cost,
                    condition,
                    prototype_power_toughness,
                } => Method::Composed {
                    name,
                    total_cost: lower_cost(total_cost)?,
                    condition,
                    prototype_power_toughness,
                },
                Method::FromZone {
                    name,
                    zone,
                    total_cost,
                    condition,
                    exiles_after_resolution,
                } => Method::FromZone {
                    name,
                    zone,
                    total_cost: lower_cost(total_cost)?,
                    condition,
                    exiles_after_resolution,
                },
                Method::Trap {
                    name,
                    cost,
                    condition,
                } => Method::Trap {
                    name,
                    cost,
                    condition,
                },
                Method::Bestow { total_cost } => Method::Bestow {
                    total_cost: lower_cost(total_cost)?,
                },
                Method::Mutate { cost } => Method::Mutate { cost },
            })
        }
    }
}

fn normalize_rewrite_parsed_ability(
    parsed: ParsedAbility,
) -> Result<NormalizedParsedAbility, CardTextError> {
    fn cost_removes_source_counters(cost: &crate::model::CompilerCost) -> bool {
        matches!(cost, crate::model::CompilerCost::RemoveCounters { .. })
    }

    fn total_cost_removes_source_counters(
        cost: &ironsmith_core::TotalCost<crate::model::CompilerCost>,
    ) -> bool {
        cost.as_all()
            .is_some_and(|costs| costs.iter().any(cost_removes_source_counters))
            || cost
                .as_one_of()
                .is_some_and(|branches| branches.iter().any(total_cost_removes_source_counters))
    }

    let prepared = match parsed.effects_ast.as_ref() {
        None => None,
        Some(_)
            if matches!(
                parsed.kind(),
                crate::model::CompilerAbilityKindCore::Activated(activated)
                    if !activated.effects.is_empty() || !activated.choices.is_empty()
            ) =>
        {
            None
        }
        Some(_)
            if matches!(
                parsed.kind(),
                crate::model::CompilerAbilityKindCore::Triggered(triggered)
                    if !triggered.effects.is_empty() || !triggered.choices.is_empty()
            ) =>
        {
            None
        }
        Some(effects_ast) => match (parsed.kind(), parsed.trigger_spec.as_ref()) {
            (crate::model::CompilerAbilityKindCore::Triggered(_), Some(trigger)) => {
                let (trigger, prepared) = rewrite_prepare_triggered_effects_for_lowering(
                    (**trigger).clone(),
                    effects_ast,
                    parsed.reference_imports.clone(),
                )?;
                Some(NormalizedPreparedAbility::Triggered { trigger, prepared })
            }
            (crate::model::CompilerAbilityKindCore::Activated(activated), _) => {
                let mut effects = effects_ast.clone();
                if total_cost_removes_source_counters(&activated.mana_cost) {
                    super::super::lowering_support::replace_pending_removed_counter_metrics_with_x(
                        &mut effects,
                    );
                }
                Some(NormalizedPreparedAbility::Activated(
                    rewrite_prepare_effects_with_trigger_context_for_lowering(
                        None,
                        &effects,
                        parsed.reference_imports.clone(),
                    )?,
                ))
            }
            _ => None,
        },
    };

    Ok(NormalizedParsedAbility { parsed, prepared })
}

fn normalize_rewrite_line_ast(
    info: crate::cards::builders::LineInfo,
    chunks: Vec<LineAst>,
    restrictions: ParsedRestrictions,
    semantic_facts: crate::model::facts::LineSemanticFacts,
    state: &mut RewriteNormalizationState,
) -> Result<NormalizedLineAst, CardTextError> {
    let mut normalized_chunks = Vec::with_capacity(chunks.len());
    let source_pronoun_enters_with_counter_surface = semantic_facts
        .statement
        .as_enters_effect_program
        .as_ref()
        .is_some_and(|facts| facts.source_pronoun_enters_with_counter_surface);
    for chunk in chunks {
        normalize_rewrite_line_chunk(
            chunk,
            state,
            &mut normalized_chunks,
            source_pronoun_enters_with_counter_surface,
        )?;
    }

    Ok(NormalizedLineAst {
        info,
        chunks: normalized_chunks,
        restrictions,
        semantic_facts,
    })
}

pub fn normalize_rewrite_line_ast_standalone(
    info: crate::cards::builders::LineInfo,
    chunks: Vec<LineAst>,
    restrictions: ParsedRestrictions,
) -> Result<NormalizedLineAst, CardTextError> {
    let mut state = RewriteNormalizationState::default();
    let semantic_facts = info.semantic_facts.clone();
    normalize_rewrite_line_ast(info, chunks, restrictions, semantic_facts, &mut state)
}

fn normalize_rewrite_line_chunk(
    chunk: LineAst,
    state: &mut RewriteNormalizationState,
    normalized_chunks: &mut Vec<NormalizedLineChunk>,
    source_pronoun_enters_with_counter_surface: bool,
) -> Result<(), CardTextError> {
    if let LineAst::Multiple(chunks) = chunk {
        for chunk in chunks {
            normalize_rewrite_line_chunk(
                chunk,
                state,
                normalized_chunks,
                source_pronoun_enters_with_counter_surface,
            )?;
        }
        return Ok(());
    }

    normalized_chunks.push(match chunk {
        LineAst::Multiple(_) => {
            unreachable!("multiple line chunks are flattened before normalization")
        }
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
        LineAst::Statement { mut effects } => {
            if source_pronoun_enters_with_counter_surface {
                retarget_as_enters_source_counter_grants(&mut effects);
            }
            let mut imports = state.statement_reference_imports();
            if let Some(cost_tag) = imports.last_object_tag.as_ref()
                && cost_tag.as_str().starts_with("tapped_")
                && let Some(cost_index) = cost_tag.as_str().get("tapped_".len()..)
            {
                let alias = format!("tap_cost_{cost_index}");
                if effects
                    .iter()
                    .any(|effect| effect_references_tag(effect, &alias))
                {
                    // A later effect in this statement may advance the ordinary
                    // last-object reference before the cost-linked reference is
                    // lowered. Snapshot the explicit additional-cost alias now.
                    imports
                        .snapshot_tag_aliases
                        .retain(|(existing, _)| existing != &alias);
                    imports
                        .snapshot_tag_aliases
                        .push((alias, cost_tag.as_str().to_string()));
                }
            }
            if let Some(cost_tag) = imports.last_object_tag.as_ref()
                && (cost_tag.as_str().starts_with("sacrifice_cost_")
                    || effects
                        .iter()
                        .any(|effect| effect_references_tag(effect, ADDITIONAL_COST_OBJECT_TAG)))
            {
                // Bind the cost export before annotating any body effect. The
                // ordinary last-object reference is intentionally free to
                // advance through damage, destroy, create, and return effects;
                // this alias must remain attached to the paid cost object.
                // Preserve a chosen sacrifice set proactively: a later plural
                // demonstrative can initially carry the generic `it` marker
                // and only become recognizable as cost-linked after an
                // intervening source move advances ordinary object memory.
                imports
                    .snapshot_tag_aliases
                    .retain(|(alias, _)| alias != ADDITIONAL_COST_OBJECT_TAG);
                imports.snapshot_tag_aliases.push((
                    ADDITIONAL_COST_OBJECT_TAG.to_string(),
                    cost_tag.as_str().to_string(),
                ));
            }
            let prepared = rewrite_prepare_statement_effects_for_lowering(&effects, imports)?;
            state.latest_spell_exports = prepared.exports.clone();
            NormalizedLineChunk::Statement {
                effects_ast: effects,
                prepared,
            }
        }
        LineAst::AdditionalCost { effects } => {
            let effects = rewrite_normalize_selected_sacrifice_tags(effects);
            let prepared = rewrite_prepare_additional_cost_effects_for_lowering(
                &effects,
                ReferenceImports::default(),
            )?;
            state.latest_additional_cost_exports = prepared.exports.clone();
            NormalizedLineChunk::AdditionalCost {
                effects_ast: effects,
                prepared,
            }
        }
        LineAst::OptionalCost(cost) => {
            NormalizedLineChunk::OptionalCost(materialize_optional_cost(cost)?)
        }
        LineAst::GiftKeyword {
            cost,
            effects,
            followup_text,
            timing,
        } => {
            let prepared =
                rewrite_prepare_effects_for_lowering(&effects, ReferenceImports::default())?;
            NormalizedLineChunk::GiftKeyword {
                cost: materialize_optional_cost(cost)?,
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
                cost: materialize_optional_cost(cost)?,
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
        LineAst::AlternativeCastingMethod(method) => NormalizedLineChunk::AlternativeCastingMethod(
            materialize_alternative_casting_method(method)?,
        ),
    });
    Ok(())
}

/// In an as-enters replacement program, the authored subject of `it enters
/// with ... counters` is the entering source.  Ordinary cross-sentence
/// antecedent resolution can otherwise bind `it` to an object sacrificed by
/// the preceding optional action.  The line fact above proves the pronoun
/// surface; this traversal then retargets only the matching typed
/// entry-counter grant.
fn retarget_as_enters_source_counter_grants(effects: &mut [EffectAst]) {
    fn retarget(effect: &mut EffectAst) {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::GrantAbilitiesToTarget {
                target, abilities, ..
            } = &mut subject_verb.action
            && matches!(target, TargetAst::Tagged(_, _))
            && abilities.iter().any(|ability| {
                matches!(
                    ability,
                    GrantedAbilityAst::StaticAbility(static_ability)
                        if matches!(
                            static_ability.as_ref(),
                            StaticAbilityAst::Static(ability)
                                if matches!(
                                    ability.payload,
                                    ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { .. }
                                )
                        )
                )
            })
        {
            *target = TargetAst::Source(None);
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            retarget_as_enters_source_counter_grants(nested);
        });
    }

    for effect in effects {
        retarget(effect);
    }
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

    let prepared_common_prefix = if modal.header.common_prefix_effects_ast.is_empty() {
        None
    } else if modal.header.trigger.is_some() || modal.header.activated.is_some() {
        Some(rewrite_prepare_effects_with_trigger_context_for_lowering(
            modal.header.trigger.as_ref(),
            &modal.header.common_prefix_effects_ast,
            ReferenceImports::default(),
        )?)
    } else {
        Some(rewrite_prepare_effects_for_lowering(
            &modal.header.common_prefix_effects_ast,
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
            point_cost: mode.point_cost,
            additional_mana_cost: mode.additional_mana_cost,
            prepared,
        });
    }

    Ok(NormalizedModalAst {
        header: modal.header,
        prepared_prefix,
        prepared_common_prefix,
        modes,
    })
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
            line.semantic_facts,
            state,
        )?)),
        ParsedCardItem::Modal(modal) => Ok(NormalizedCardItem::Modal(normalize_rewrite_modal_ast(
            modal,
        )?)),
        ParsedCardItem::LevelAbility(level) => Ok(NormalizedCardItem::LevelAbility(level)),
    }
}

pub fn prepare_parsed_card_ast_for_lowering(
    ast: ParsedCardAst,
) -> Result<NormalizedCardAst, CardTextError> {
    let ParsedCardAst {
        builder,
        annotations,
        provenance,
        symbols,
        reference_resolution,
        items,
        overload_branch,
        cleave_branch,
        allow_unsupported,
    } = ast;
    if let Some(diagnostic) = reference_resolution.diagnostics.first() {
        return Err(CardTextError::ParseError(format!(
            "canonical reference resolution failed before lowering: {diagnostic:?}"
        )));
    }
    let overload_branch = if let Some(branch) = overload_branch {
        let mut state = RewriteNormalizationState::default();
        let mut items = Vec::new();
        for item in branch.items {
            items.push(prepare_parsed_item_to_normalized_item(item, &mut state)?);
        }
        Some(NormalizedOverloadBranch { items })
    } else {
        None
    };
    let cleave_branch = if let Some(branch) = cleave_branch {
        let mut state = RewriteNormalizationState::default();
        let mut items = Vec::new();
        for item in branch.items {
            items.push(prepare_parsed_item_to_normalized_item(item, &mut state)?);
        }
        Some(NormalizedCleaveBranch { items })
    } else {
        None
    };
    let mut state = RewriteNormalizationState::default();
    let mut normalized_items = Vec::new();
    for item in items {
        normalized_items.push(prepare_parsed_item_to_normalized_item(item, &mut state)?);
    }

    Ok(NormalizedCardAst {
        builder,
        annotations,
        provenance,
        symbols,
        items: normalized_items,
        overload_branch,
        cleave_branch,
        allow_unsupported,
    })
}

#[cfg(test)]
pub fn rewrite_document_to_normalized_card_ast(
    doc: RewriteSemanticDocument,
) -> Result<NormalizedCardAst, CardTextError> {
    prepare_parsed_card_ast_for_lowering(super::super::semantic_document::parse_semantic_document(
        doc,
    )?)
}
