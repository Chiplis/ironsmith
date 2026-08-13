use super::*;
#[cfg(test)]
use crate::runtime_backend::ir::RewriteSemanticDocument;

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
    fn cost_removes_source_counters(cost: &crate::costs::Cost) -> bool {
        matches!(
            cost,
            crate::costs::Cost::RemoveCounters { .. }
                | crate::costs::Cost::RemoveAnyCountersFromSource { .. }
        ) || cost.effect_ref().is_some_and(|effect| {
            effect
                .downcast_ref::<crate::effects::RemoveCountersEffect>()
                .is_some()
                || effect
                    .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                    .is_some()
                || effect
                    .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
                    .is_some()
        })
    }

    fn total_cost_removes_source_counters(
        cost: &ironsmith_core::TotalCost<crate::costs::Cost>,
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
            (AbilityKind::Activated(activated), _) => {
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
    semantic_facts: crate::runtime_backend::shared_types::LineSemanticFacts,
    state: &mut RewriteNormalizationState,
) -> Result<NormalizedLineAst, CardTextError> {
    let mut normalized_chunks = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        normalize_rewrite_line_chunk(chunk, state, &mut normalized_chunks)?;
    }

    Ok(NormalizedLineAst {
        info,
        chunks: normalized_chunks,
        restrictions,
        semantic_facts,
    })
}

pub(crate) fn normalize_rewrite_line_ast_standalone(
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
) -> Result<(), CardTextError> {
    if let LineAst::Multiple(chunks) = chunk {
        for chunk in chunks {
            normalize_rewrite_line_chunk(chunk, state, normalized_chunks)?;
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
        LineAst::Statement { effects } => {
            let mut imports = state.statement_reference_imports();
            if let Some(cost_tag) = imports.last_object_tag.as_ref()
                && let Some(cost_index) = cost_tag.as_str().strip_prefix("tapped_")
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
    Ok(())
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

pub(crate) fn prepare_parsed_card_ast_for_lowering(
    ast: ParsedCardAst,
) -> Result<NormalizedCardAst, CardTextError> {
    let ParsedCardAst {
        builder,
        annotations,
        provenance,
        items,
        overload_branch,
        cleave_branch,
        allow_unsupported,
    } = ast;
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
        items: normalized_items,
        overload_branch,
        cleave_branch,
        allow_unsupported,
    })
}

#[cfg(test)]
pub(crate) fn rewrite_document_to_normalized_card_ast(
    doc: RewriteSemanticDocument,
) -> Result<NormalizedCardAst, CardTextError> {
    prepare_parsed_card_ast_for_lowering(super::super::semantic_document::parse_semantic_document(
        doc,
    )?)
}
