use super::*;

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
) -> Result<LineAst, CardTextError> {
    let max_condition = max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn);
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
                .map(crate::ConditionExpr::MaxTimesEachTurn);
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
                ReferenceImports::default(),
            )))
        }
        LineAst::Ability(mut parsed) => {
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
            Ok(LineAst::Ability(parsed))
        }
        other => Ok(other),
    }
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
            let mut restrictions = ParsedRestrictions::default();
            let chunks = if line.text == "activate only once each turn." {
                restrictions
                    .activation
                    .push("Activate only once each turn".to_string());
                Vec::new()
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
