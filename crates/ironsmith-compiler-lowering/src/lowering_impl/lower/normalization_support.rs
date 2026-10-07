use super::*;
use crate::cards::builders::GrantActionAst;
use crate::cards::builders::{
    CardDefinitionBuilder, GrantedAbilityAst, StaticAbilityAst, TargetAst,
};
#[cfg(test)]
use ironsmith_compiler::ir::RewriteSemanticDocument;

#[derive(Debug, Clone, Default)]
pub(super) struct RewriteNormalizationState {
    spell_has_announced_x: bool,
    latest_spell_exports: ReferenceExports,
    latest_additional_cost_exports: ReferenceExports,
    source_number_domain: Option<(u32, Option<u32>)>,
}

impl RewriteNormalizationState {
    fn statement_reference_imports(&self) -> ReferenceImports {
        let mut additional_cost_imports = self.latest_additional_cost_exports.to_imports();
        if let Some(tag) = additional_cost_imports.last_object_tag.as_ref()
            && tag.as_str().starts_with("discarded_")
        {
            // Effect-backed discard payments export their own concrete tag.
            // Bind the explicit cost reference before body effects can advance
            // ordinary object memory (for example, by drawing more cards).
            additional_cost_imports.snapshot_tag_aliases.push((
                crate::tag::CompilerReferenceTag::DiscardedCost.key(),
                tag.clone(),
            ));
        }
        if let Some(tag) = additional_cost_imports.last_object_tag.as_ref()
            && crate::tag::is_sentence_helper_tag(tag, "revealed")
        {
            // "As an additional cost ..., reveal a card ... the revealed
            // card's power": the reveal payment is the public reveal the body
            // names.
            additional_cost_imports.snapshot_tag_aliases.push((
                crate::tag::CompilerReferenceTag::PublicRevealed.key(),
                tag.clone(),
            ));
        }
        // A cost's X binding alone must not displace an earlier statement's
        // object/result antecedents when choosing which references to import.
        let additional_cost_has_announced_x = additional_cost_imports.has_announced_x;
        additional_cost_imports.has_announced_x = false;
        let mut imports = if !additional_cost_imports.is_empty() {
            additional_cost_imports
        } else {
            self.latest_spell_exports.to_imports()
        };
        imports.has_announced_x |= self.spell_has_announced_x
            || self.latest_spell_exports.has_announced_x
            || additional_cost_has_announced_x;
        imports
    }
}

fn spell_costs_have_announced_x(items: &[ParsedCardItem]) -> bool {
    fn chunk_has_x(chunk: &LineAst) -> bool {
        match chunk {
            LineAst::Multiple(chunks) => chunks.iter().any(chunk_has_x),
            LineAst::OptionalCost(cost)
            | LineAst::GiftKeyword { cost, .. }
            | LineAst::OptionalCostWithCastTrigger { cost, .. } => {
                crate::model::costs::cost_has_announced_x(&cost.cost)
            }
            LineAst::AlternativeCastingMethod(method) => {
                method.mana_cost().is_some_and(crate::mana::ManaCost::has_x)
                    || method.total_cost().is_some_and(crate::model::costs::cost_has_announced_x)
            }
            // A separate ability has its own X scope, even on an X spell.
            _ => false,
        }
    }
    items.iter().any(|item| match item {
        ParsedCardItem::Line(line) => line.chunks.iter().any(chunk_has_x),
        _ => false,
    })
}

fn materialize_optional_cost(
    cost: crate::model::compiler_semantic::ParsedOptionalCostAst,
) -> Result<crate::cost::OptionalCost, CardTextError> {
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

fn materialize_alternative_casting_method(
    method: crate::model::compiler_semantic::ParsedAlternativeCastingMethodAst,
) -> Result<crate::alternative_cast::AlternativeCastingMethod, CardTextError> {
    // Materialize the whole cost algebra: one authored payment ("return an
    // Island you control to its owner's hand") expands into sibling runtime
    // components, and per-component mapping would fold them back into a
    // single composite that prints as two authored sentences.
    method.try_map_total_costs(
        crate::lowering_support::lower_compiler_child_effect,
        |cost| crate::lowering::cost_materialization::materialize_compiler_core_total_cost(&cost),
    )
}

fn normalize_parsed_ability(
    mut parsed: ParsedAbility,
) -> Result<NormalizedParsedAbility, CardTextError> {
    if let crate::model::CompilerAbilityKindCore::Activated(activated) = parsed.kind() {
        let has_announced_x = crate::model::costs::cost_has_announced_x(&activated.mana_cost);
        parsed.reference_imports.has_announced_x |= has_announced_x;
    }
    let runtime_payload_present = match parsed.kind() {
        crate::model::CompilerAbilityKindCore::Activated(activated) => {
            !activated.effects.is_empty() || !activated.choices.is_empty()
        }
        crate::model::CompilerAbilityKindCore::Triggered(triggered) => {
            !triggered.effects.is_empty() || !triggered.choices.is_empty()
        }
        _ => false,
    };
    let triggered_spec = matches!(
        parsed.kind(),
        crate::model::CompilerAbilityKindCore::Triggered(_)
    )
    .then(|| parsed.trigger_spec.as_deref().cloned())
    .flatten();
    let prepared = if parsed.effects_ast.is_none() || runtime_payload_present {
        None
    } else {
        let effects = std::mem::take(
            parsed
                .effects_ast
                .as_mut()
                .expect("checked compiler effect sidecar"),
        );
        if let Some(trigger) = triggered_spec {
            let (trigger, prepared) = stage_owned_triggered_effects_for_lowering(
                trigger,
                effects,
                parsed.reference_imports.clone(),
            )?;
            Some(NormalizedPreparedAbility::Triggered { trigger, prepared })
        } else if matches!(
            parsed.kind(),
            crate::model::CompilerAbilityKindCore::Activated(_)
        ) {
            Some(NormalizedPreparedAbility::Activated(
                stage_effects_with_trigger_context_for_lowering(
                    None,
                    &effects,
                    parsed.reference_imports.clone(),
                )?,
            ))
        } else {
            None
        }
    };

    Ok(NormalizedParsedAbility { parsed, prepared })
}

/// An entry numeric producer owns the source's persistent slot. A later
/// matching upkeep reselection is linked to that domain; local spell and
/// activated number choices continue to use only their execution receipt.
fn retain_source_number_choices(
    effects: &mut [EffectAst],
    required_domain: Option<(u32, Option<u32>)>,
    discovered: &mut Option<(u32, Option<u32>)>,
) {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject) = effect {
            if let crate::cards::builders::SubjectVerbActionAst::Choices(
                crate::cards::builders::ChoiceActionAst::ChooseNumber { min, max, source_owned },
            ) = &mut subject.action {
                let domain = (*min, *max);
                if required_domain.is_none_or(|required| required == domain) {
                    *source_owned = true;
                    *discovered = Some(domain);
                }
            }
        }
        ironsmith_compiler_semantic::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            retain_source_number_choices(nested, required_domain, discovered);
        });
    }
}

fn own_upkeep(trigger:&TriggerSpec)->bool{
    match trigger {
        TriggerSpec::WithIntro{trigger,..}|TriggerSpec::ConditionQualified{trigger,..}=>own_upkeep(trigger),
        TriggerSpec::BeginningOfUpkeep(ironsmith_core::PlayerFilter::You)=>true,
        _=>false,
    }
}
fn retain_entry_numeric_choices(chunk:&mut LineAst,domain:&mut Option<(u32,Option<u32>)>){
    match chunk {
        LineAst::Multiple(chunks)=>for chunk in chunks {retain_entry_numeric_choices(chunk,domain);},
        LineAst::Statement{effects}=>retain_source_number_choices(effects,None,domain),
        _=>{},
    }
}
fn normalize_line_ast(
    info: crate::model::facts::LineInfo,
    chunks: Vec<LineAst>,
    restrictions: ParsedRestrictions,
    semantic_facts: crate::model::facts::LineSemanticFacts,
    state: &mut RewriteNormalizationState,
) -> Result<NormalizedLineAst, CardTextError> {
    let mut normalized_chunks = Vec::with_capacity(chunks.len());
    let source_reference_enters_with_counter_surface = semantic_facts
        .statement
        .as_enters_effect_program
        .as_ref()
        .is_some_and(|facts| facts.source_reference_enters_with_counter_surface);
    for mut chunk in chunks {
        if semantic_facts.statement.as_enters_effect_program.is_some() {
            retain_entry_numeric_choices(&mut chunk,&mut state.source_number_domain);
        }
        normalize_line_chunk(
            chunk,
            state,
            &mut normalized_chunks,
            source_reference_enters_with_counter_surface,
        )?;
    }

    Ok(NormalizedLineAst {
        info,
        chunks: normalized_chunks,
        restrictions,
        semantic_facts,
    })
}

pub fn normalize_line_ast_standalone(
    info: crate::model::facts::LineInfo,
    chunks: Vec<LineAst>,
    restrictions: ParsedRestrictions,
    semantic_facts: crate::model::facts::LineSemanticFacts,
) -> Result<NormalizedLineAst, CardTextError> {
    let mut state = RewriteNormalizationState::default();
    normalize_line_ast(info, chunks, restrictions, semantic_facts, &mut state)
}

fn normalize_line_chunk(
    chunk: LineAst,
    state: &mut RewriteNormalizationState,
    normalized_chunks: &mut Vec<NormalizedLineChunk>,
    source_reference_enters_with_counter_surface: bool,
) -> Result<(), CardTextError> {
    if let LineAst::Multiple(chunks) = chunk {
        for chunk in chunks {
            normalize_line_chunk(
                chunk,
                state,
                normalized_chunks,
                source_reference_enters_with_counter_surface,
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
        LineAst::Ability(mut parsed) => {
            if parsed.trigger_spec.as_deref().is_some_and(own_upkeep) {
                if let (Some(domain),Some(effects))=(state.source_number_domain,parsed.effects_ast.as_mut()) {
                    retain_source_number_choices(effects,Some(domain),&mut None);
                }
            }
            NormalizedLineChunk::Ability(normalize_parsed_ability(parsed)?)
        },
        LineAst::Triggered {
            trigger,
            mut effects,
            max_triggers_per_turn,
        } => {
            if own_upkeep(&trigger) {
                if let Some(domain) = state.source_number_domain {
                    retain_source_number_choices(&mut effects, Some(domain), &mut None);
                }
            }
            let (trigger, prepared) = stage_owned_triggered_effects_for_lowering(
                trigger,
                effects,
                ReferenceImports::default(),
            )?;
            NormalizedLineChunk::Triggered {
                trigger,
                prepared,
                max_triggers_per_turn,
            }
        }
        LineAst::Statement { mut effects } => {
            if source_reference_enters_with_counter_surface {
                resolve_as_enters_source_counter_grants(&mut effects);
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
                    let alias = ironsmith_compiler_semantic::tag::declared_key(alias);
                    // A later effect in this statement may advance the ordinary
                    // last-object reference before the cost-linked reference is
                    // lowered. Snapshot the explicit additional-cost alias now.
                    imports
                        .snapshot_tag_aliases
                        .retain(|(existing, _)| *existing != alias.key);
                    imports
                        .snapshot_tag_aliases
                        .push((alias.key.clone(), cost_tag.clone()));
                }
            }
            if let Some(cost_tag) = imports.last_object_tag.as_ref()
                && (cost_tag.as_str().starts_with("sacrifice_cost_")
                    || effects.iter().any(|effect| {
                        effect_references_tag(
                            effect,
                            crate::tag::CompilerReferenceTag::AdditionalCostObject.as_str(),
                        )
                    }))
            {
                // Bind the cost export before annotating any body effect. The
                // ordinary last-object reference is intentionally free to
                // advance through damage, destroy, create, and return effects;
                // this alias must remain attached to the paid cost object.
                // Preserve a chosen sacrifice set proactively: a later plural
                // demonstrative can initially carry the generic `it` marker
                // and only become recognizable as cost-linked after an
                // intervening source move advances ordinary object memory.
                imports.snapshot_tag_aliases.retain(|(alias, _)| {
                    alias != &crate::tag::CompilerReferenceTag::AdditionalCostObject.key()
                });
                imports.snapshot_tag_aliases.push((
                    (crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()).into(),
                    cost_tag.clone(),
                ));
            }
            let prepared = stage_statement_effects_for_lowering(&effects, imports)?;
            state.latest_spell_exports = prepared.exports.clone();
            NormalizedLineChunk::Statement {
                effects_ast: effects,
                prepared,
            }
        }
        LineAst::AdditionalCost { effects } => {
            let effects = normalize_selected_sacrifice_tags(effects);
            let prepared =
                stage_additional_cost_effects_for_lowering(&effects, ReferenceImports::default())?;
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
            timing,
        } => {
            let prepared = stage_effects_for_lowering(&effects, ReferenceImports::default())?;
            NormalizedLineChunk::GiftKeyword {
                cost: materialize_optional_cost(cost)?,
                prepared,
                timing,
            }
        }
        LineAst::OptionalCostWithCastTrigger { cost, effects } => {
            let prepared = stage_effects_for_lowering(
                &effects,
                state.latest_additional_cost_exports.to_imports(),
            )?;
            NormalizedLineChunk::OptionalCostWithCastTrigger {
                cost: materialize_optional_cost(cost)?,
                prepared,
            }
        }
        LineAst::AdditionalCostChoice { options } => {
            let mut normalized_options = Vec::with_capacity(options.len());
            let mut exports = ReferenceExports::default();
            let mut saw_option = false;
            for option in options {
                let prepared =
                    stage_effects_for_lowering(&option.effects, ReferenceImports::default())?;
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
fn resolve_as_enters_source_counter_grants(effects: &mut [EffectAst]) {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
                ..
            }) = &subject_verb.action
            && matches!(target, TargetAst::Tagged(_, _) | TargetAst::Source(_))
            && *duration == ironsmith_core::Until::Forever
            && let [GrantedAbilityAst::StaticAbility(static_ability)] = abilities.as_slice()
            && let StaticAbilityAst::Static(ability) = static_ability.as_ref()
            && let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count } =
                &ability.payload
        {
            // This program runs during entry preparation. Its source counter
            // additions are transferred to the entering object; granting a
            // future entry replacement to the source-zone card is too late.
            *effect = EffectAst::subject_verb_put_counters(
                *counter,
                count.clone(),
                TargetAst::Source(None),
                None,
                false,
            );
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            resolve_as_enters_source_counter_grants(nested);
        });
    }
}

fn normalize_modal_ast(
    modal: ParsedModalAst,
    spell_has_announced_x: bool,
) -> Result<NormalizedModalAst, CardTextError> {
    let imports = ReferenceImports {
        has_announced_x: if let Some(activated) = modal.header.activated.as_ref() {
            crate::model::costs::cost_has_announced_x(&activated.mana_cost)
        } else {
            modal.header.trigger.is_none() && spell_has_announced_x
        },
        ..Default::default()
    };
    let prepared_prefix = if modal.header.prefix_effects_ast.is_empty() {
        None
    } else if modal.header.trigger.is_some() || modal.header.activated.is_some() {
        Some(stage_effects_with_trigger_context_for_lowering(
            modal.header.trigger.as_ref(),
            &modal.header.prefix_effects_ast,
            imports.clone(),
        )?)
    } else {
        Some(stage_effects_for_lowering(
            &modal.header.prefix_effects_ast,
            imports.clone(),
        )?)
    };

    let prepared_common_prefix = if modal.header.common_prefix_effects_ast.is_empty() {
        None
    } else if modal.header.trigger.is_some() || modal.header.activated.is_some() {
        Some(stage_effects_with_trigger_context_for_lowering(
            modal.header.trigger.as_ref(),
            &modal.header.common_prefix_effects_ast,
            imports.clone(),
        )?)
    } else {
        Some(stage_effects_for_lowering(
            &modal.header.common_prefix_effects_ast,
            imports.clone(),
        )?)
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut mode_imports = imports.clone();
        mode_imports.has_announced_x |= mode
            .additional_mana_cost
            .as_ref()
            .is_some_and(crate::mana::ManaCost::has_x);
        // A triggered mode's pronouns ("• Put a +1/+1 counter on that
        // creature", "• It gains double strike") name the trigger's event
        // object exactly like an unmoded trigger body does.
        let prepared = if let Some(trigger) = modal.header.trigger.as_ref() {
            stage_effects_with_trigger_context_for_lowering(
                Some(trigger),
                &mode.effects_ast,
                mode_imports,
            )?
        } else {
            stage_effects_for_lowering(&mode.effects_ast, mode_imports)?
        };
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
fn normalized_item_from_parsed_item(
    item: ParsedCardItem,
    state: &mut RewriteNormalizationState,
) -> Result<NormalizedCardItem, CardTextError> {
    match item {
        ParsedCardItem::Line(line) => Ok(NormalizedCardItem::Line(normalize_line_ast(
            line.info,
            line.chunks,
            line.restrictions,
            line.semantic_facts,
            state,
        )?)),
        ParsedCardItem::Modal(modal) => Ok(NormalizedCardItem::Modal(normalize_modal_ast(
            modal,
            state.spell_has_announced_x,
        )?)),
        ParsedCardItem::LevelAbility(level) => Ok(NormalizedCardItem::LevelAbility(level)),
    }
}

pub fn normalize_parsed_card_ast_for_lowering(
    ast: ParsedCardAst,
    seed: CardDefinitionBuilder,
) -> Result<NormalizedCardAst, CardTextError> {
    let ParsedCardAst {
        card,
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
    // Normalization rewrites references (cost aliases, pronoun antecedents):
    // the keys it mints bind in the document's scope.
    let symbols = std::cell::RefCell::new(symbols);
    let document_scope = {
        let table = symbols.borrow();
        table
            .scopes()
            .iter()
            .find(|scope| scope.kind == crate::model::symbols::SymbolScopeKind::Document)
            .map(|scope| scope.id)
            .unwrap_or(table.root_scope())
    };
    let document_references = ironsmith_compiler_ast::reference_ledger::ReferenceScopeGuard::enter(
        &symbols,
        document_scope,
    );
    let initial_state = RewriteNormalizationState {
        // Alternative/optional costs can be printed after the resolution
        // text, so establish their X binding before preparing any statement.
        spell_has_announced_x: card.mana_cost_ref().is_some_and(crate::mana::ManaCost::has_x)
            || spell_costs_have_announced_x(&items),
        ..Default::default()
    };
    let overload_branch = if let Some(branch) = overload_branch {
        let mut state = initial_state.clone();
        let mut items = Vec::new();
        for item in branch.items {
            items.push(normalized_item_from_parsed_item(item, &mut state)?);
        }
        Some(NormalizedOverloadBranch { items })
    } else {
        None
    };
    let cleave_branch = if let Some(branch) = cleave_branch {
        let mut state = initial_state.clone();
        let mut items = Vec::new();
        for item in branch.items {
            items.push(normalized_item_from_parsed_item(item, &mut state)?);
        }
        Some(NormalizedCleaveBranch { items })
    } else {
        None
    };
    let mut state = initial_state;
    let mut normalized_items = Vec::new();
    for item in items {
        normalized_items.push(normalized_item_from_parsed_item(item, &mut state)?);
    }

    drop(document_references);
    Ok(NormalizedCardAst {
        builder: seed.with_face(card),
        annotations,
        provenance,
        symbols: symbols.into_inner(),
        items: normalized_items,
        overload_branch,
        cleave_branch,
        allow_unsupported,
    })
}

#[cfg(test)]
pub fn document_to_normalized_card_ast(
    doc: RewriteSemanticDocument,
) -> Result<NormalizedCardAst, CardTextError> {
    normalize_parsed_card_ast_for_lowering(
        ironsmith_compiler::semantic_document::parse_semantic_document(doc)?,
        CardDefinitionBuilder::seed(),
    )
}
