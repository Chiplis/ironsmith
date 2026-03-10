use crate::ability::{Ability, AbilityKind, ActivationTiming, LevelAbility, TriggeredAbility};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::ParseAnnotations;
use crate::cards::builders::{
    AnnotatedEffectSequence, CardTextError, EffectAst, EffectReferenceResolutionConfig,
    KeywordAction, LineAst, LineInfo, ParsedAbility, ParsedCardAst, ParsedCardItem,
    ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalAst,
    ParsedModalHeader, ParsedRestrictions, PredicateAst, ReferenceEnv, ReferenceExports,
    ReferenceImports, StaticAbilityAst, TriggerSpec, annotate_effect_sequence,
    apply_instead_followup_statement_to_last_ability, collect_tag_spans_from_effects_with_context,
    combine_mana_activation_condition, effects_reference_it_tag, effects_reference_its_controller,
    effects_reference_tag, ensure_concrete_trigger_spec, inferred_trigger_player_filter,
    keyword_action_to_static_ability, lower_prepared_ability,
    lower_prepared_additional_cost_choice_modes_with_exports,
    lower_prepared_effects_with_trigger_context, lower_prepared_statement_effects,
    lower_static_abilities_ast, lower_static_ability_ast, normalize_effects_ast,
    parse_activate_only_timing, parse_activation_condition, parse_mana_output_options_for_line,
    parse_mana_usage_restriction_sentence, parse_triggered_times_each_turn_from_words,
    parsed_triggered_ability, tokenize_line, trigger_supports_event_value, words,
};
use crate::color::ColorSet;
use crate::cost::OptionalCost;
use crate::effect::{
    Condition, Effect, EffectId, EffectMode, EffectPredicate, EventValueSpec, Value,
};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;
use crate::{CardDefinition, CardDefinitionBuilder, TagKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EffectPreludeTag {
    AttachedSource(TagKey),
    TriggeringObject(TagKey),
    TriggeringDamageTarget(TagKey),
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedPredicateForLowering {
    pub(crate) predicate: PredicateAst,
    pub(crate) reference_env: ReferenceEnv,
    pub(crate) saved_last_object_tag: Option<TagKey>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedEffectsForLowering {
    pub(crate) effects: Vec<EffectAst>,
    pub(crate) imports: ReferenceImports,
    pub(crate) initial_env: ReferenceEnv,
    pub(crate) annotated: AnnotatedEffectSequence,
    pub(crate) exports: ReferenceExports,
    pub(crate) prelude: Vec<EffectPreludeTag>,
    pub(crate) force_auto_tag_object_targets: bool,
}

#[derive(Debug, Clone, Default)]
struct LoweredCardState {
    haunt_linkage: Option<(Vec<Effect>, Vec<ChooseSpec>)>,
    latest_spell_exports: ReferenceExports,
    latest_additional_cost_exports: ReferenceExports,
}

impl LoweredCardState {}

#[derive(Debug, Clone, Default)]
struct CardNormalizationState {
    latest_spell_exports: ReferenceExports,
    latest_additional_cost_exports: ReferenceExports,
}

impl CardNormalizationState {
    fn statement_reference_imports(&self) -> ReferenceImports {
        let additional_cost_imports = self.latest_additional_cost_exports.to_imports();
        if !additional_cost_imports.is_empty() {
            return additional_cost_imports;
        }
        self.latest_spell_exports.to_imports()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedTriggeredEffectsForLowering {
    pub(crate) prepared: PreparedEffectsForLowering,
    pub(crate) intervening_if: Option<PreparedPredicateForLowering>,
}

#[derive(Debug, Clone)]
pub(crate) enum NormalizedPreparedAbility {
    Activated(PreparedEffectsForLowering),
    Triggered {
        trigger: TriggerSpec,
        prepared: PreparedTriggeredEffectsForLowering,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedParsedAbility {
    pub(crate) parsed: ParsedAbility,
    pub(crate) prepared: Option<NormalizedPreparedAbility>,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedAdditionalCostChoiceOptionAst {
    pub(crate) description: String,
    pub(crate) effects_ast: Vec<EffectAst>,
    pub(crate) prepared: PreparedEffectsForLowering,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedModalModeAst {
    pub(crate) info: LineInfo,
    pub(crate) description: String,
    pub(crate) prepared: PreparedEffectsForLowering,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedModalAst {
    pub(crate) header: ParsedModalHeader,
    pub(crate) prepared_prefix: Option<PreparedEffectsForLowering>,
    pub(crate) modes: Vec<NormalizedModalModeAst>,
}

#[derive(Debug, Clone)]
pub(crate) enum NormalizedLineChunk {
    Abilities(Vec<KeywordAction>),
    StaticAbility(StaticAbilityAst),
    StaticAbilities(Vec<StaticAbilityAst>),
    Ability(NormalizedParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        prepared: PreparedTriggeredEffectsForLowering,
        max_triggers_per_turn: Option<u32>,
    },
    Statement {
        effects_ast: Vec<EffectAst>,
        prepared: PreparedEffectsForLowering,
    },
    AdditionalCost {
        effects_ast: Vec<EffectAst>,
        prepared: PreparedEffectsForLowering,
    },
    OptionalCost(OptionalCost),
    AdditionalCostChoice {
        options: Vec<NormalizedAdditionalCostChoiceOptionAst>,
    },
    AlternativeCastingMethod(AlternativeCastingMethod),
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedLineAst {
    pub(crate) info: LineInfo,
    pub(crate) chunks: Vec<NormalizedLineChunk>,
    pub(crate) restrictions: ParsedRestrictions,
}

#[derive(Debug, Clone)]
pub(crate) enum NormalizedCardItem {
    Line(NormalizedLineAst),
    Modal(NormalizedModalAst),
    LevelAbility(ParsedLevelAbilityAst),
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedCardAst {
    pub(crate) builder: CardDefinitionBuilder,
    pub(crate) annotations: ParseAnnotations,
    pub(crate) items: Vec<NormalizedCardItem>,
    pub(crate) allow_unsupported: bool,
}

fn prepare_effects_from_normalized(
    semantic_effects: Vec<EffectAst>,
    reference_effects: &[EffectAst],
    mut imports: ReferenceImports,
    config: EffectReferenceResolutionConfig,
    inferred_last_player_filter: Option<PlayerFilter>,
    default_last_object_tag: Option<TagKey>,
    include_trigger_prelude: bool,
) -> Result<PreparedEffectsForLowering, CardTextError> {
    let mut prelude = Vec::new();
    for tag in ["equipped", "enchanted"] {
        if effects_reference_tag(reference_effects, tag) {
            if imports.last_object_tag.is_none() {
                imports.last_object_tag = Some(TagKey::from(tag));
            }
            prelude.push(EffectPreludeTag::AttachedSource(TagKey::from(tag)));
        }
    }

    if imports.last_player_filter.is_none() {
        imports.last_player_filter = inferred_last_player_filter;
    }

    if imports.last_object_tag.is_none()
        && let Some(tag) = default_last_object_tag.as_ref()
    {
        imports.last_object_tag = Some(tag.clone());
    }

    if include_trigger_prelude {
        let needs_triggering_prelude = default_last_object_tag
            .as_ref()
            .is_some_and(|tag| tag.as_str() == "triggering")
            || effects_reference_tag(reference_effects, "triggering");
        if needs_triggering_prelude {
            prelude.insert(
                0,
                EffectPreludeTag::TriggeringObject(TagKey::from("triggering")),
            );
        }
        let needs_damaged_prelude = default_last_object_tag
            .as_ref()
            .is_some_and(|tag| tag.as_str() == "damaged")
            || effects_reference_tag(reference_effects, "damaged");
        if needs_damaged_prelude {
            prelude.insert(
                0,
                EffectPreludeTag::TriggeringDamageTarget(TagKey::from("damaged")),
            );
        }
    }

    let initial_env = ReferenceEnv::from_imports(
        &imports,
        config.initial_iterated_player,
        config.allow_life_event_value,
        config.bind_unbound_x_to_last_effect,
        config.initial_last_effect_id,
    );
    let annotated =
        annotate_effect_sequence(&semantic_effects, &imports, config, Default::default())?;
    let exports = ReferenceExports::from_env(&annotated.final_env);

    Ok(PreparedEffectsForLowering {
        effects: semantic_effects,
        imports,
        initial_env,
        annotated,
        exports,
        prelude,
        force_auto_tag_object_targets: config.force_auto_tag_object_targets,
    })
}

fn runtime_effects_to_costs(effects: Vec<Effect>) -> Result<Vec<crate::costs::Cost>, CardTextError> {
    effects
        .into_iter()
        .map(|effect| {
            crate::costs::Cost::try_from_runtime_effect(effect).map_err(CardTextError::ParseError)
        })
        .collect()
}

pub(crate) fn prepare_effects_for_lowering(
    effects: &[EffectAst],
    imports: ReferenceImports,
) -> Result<PreparedEffectsForLowering, CardTextError> {
    let normalized = normalize_effects_ast(effects);
    prepare_effects_from_normalized(
        normalized.clone(),
        &normalized,
        imports,
        EffectReferenceResolutionConfig {
            force_auto_tag_object_targets: true,
            ..Default::default()
        },
        None,
        None,
        false,
    )
}

pub(crate) fn prepare_effects_with_trigger_context_for_lowering(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
    imports: ReferenceImports,
) -> Result<PreparedEffectsForLowering, CardTextError> {
    let normalized = normalize_effects_ast(effects);
    let default_last_object_tag = if imports.last_object_tag.is_none()
        && (effects_reference_it_tag(&normalized) || effects_reference_its_controller(&normalized))
    {
        Some(TagKey::from(
            if matches!(
                trigger,
                Some(
                    TriggerSpec::ThisDealsDamageTo(_)
                        | TriggerSpec::ThisDealsCombatDamageTo(_)
                        | TriggerSpec::DealsCombatDamageTo { .. }
                )
            ) {
                "damaged"
            } else {
                "triggering"
            },
        ))
    } else {
        None
    };

    prepare_effects_from_normalized(
        normalized.clone(),
        &normalized,
        imports,
        EffectReferenceResolutionConfig {
            allow_life_event_value: trigger
                .map(|trigger| trigger_supports_event_value(trigger, &EventValueSpec::Amount))
                .unwrap_or(false),
            ..Default::default()
        },
        trigger.and_then(inferred_trigger_player_filter),
        default_last_object_tag,
        trigger.is_some(),
    )
}

pub(crate) fn prepare_triggered_effects_for_lowering(
    trigger: &TriggerSpec,
    effects: &[EffectAst],
    imports: ReferenceImports,
) -> Result<PreparedTriggeredEffectsForLowering, CardTextError> {
    ensure_concrete_trigger_spec(trigger)?;

    let normalized = normalize_effects_ast(effects);
    let mut body_effects = normalized.clone();
    let mut intervening_if = None;
    if normalized.len() == 1
        && let EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } = &normalized[0]
        && if_false.is_empty()
        && !if_true.is_empty()
    {
        body_effects = if_true.clone();
        intervening_if = Some(predicate.clone());
    }

    let prepared = prepare_effects_from_normalized(
        body_effects,
        &normalized,
        imports,
        EffectReferenceResolutionConfig {
            allow_life_event_value: trigger_supports_event_value(trigger, &EventValueSpec::Amount),
            ..Default::default()
        },
        inferred_trigger_player_filter(trigger),
        if effects_reference_it_tag(&normalized) || effects_reference_its_controller(&normalized) {
            Some(TagKey::from(
                if matches!(
                    trigger,
                    TriggerSpec::ThisDealsDamageTo(_)
                        | TriggerSpec::ThisDealsCombatDamageTo(_)
                        | TriggerSpec::DealsCombatDamageTo { .. }
                ) {
                    "damaged"
                } else {
                    "triggering"
                },
            ))
        } else {
            None
        },
        true,
    )?;

    let intervening_if = intervening_if.map(|predicate| PreparedPredicateForLowering {
        predicate,
        reference_env: prepared.initial_env.clone(),
        saved_last_object_tag: prepared.imports.last_object_tag.clone(),
    });

    Ok(PreparedTriggeredEffectsForLowering {
        prepared,
        intervening_if,
    })
}

pub(crate) fn parse_text_with_annotations(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let ast = super::parser::parse_card_ast_with_annotations(builder, text, allow_unsupported)?;
    let ast = normalize_card_ast(ast)?;
    lower_card_ast(ast)
}

pub(crate) fn normalize_card_ast(ast: ParsedCardAst) -> Result<NormalizedCardAst, CardTextError> {
    let ParsedCardAst {
        builder,
        annotations,
        items,
        allow_unsupported,
    } = ast;
    let mut state = CardNormalizationState::default();
    let mut normalized_items = Vec::with_capacity(items.len());
    for item in items {
        normalized_items.push(match item {
            ParsedCardItem::Line(line) => {
                NormalizedCardItem::Line(normalize_line_ast(line, &mut state)?)
            }
            ParsedCardItem::Modal(modal) => NormalizedCardItem::Modal(normalize_modal_ast(modal)?),
            ParsedCardItem::LevelAbility(level) => NormalizedCardItem::LevelAbility(level),
        });
    }

    Ok(NormalizedCardAst {
        builder,
        annotations,
        items: normalized_items,
        allow_unsupported,
    })
}

pub(crate) fn lower_card_ast(
    ast: NormalizedCardAst,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let NormalizedCardAst {
        mut builder,
        mut annotations,
        items,
        allow_unsupported,
    } = ast;

    let mut level_abilities = Vec::new();
    let mut last_restrictable_ability: Option<usize> = None;
    let mut state = LoweredCardState::default();

    for item in items {
        match item {
            NormalizedCardItem::Line(line) => {
                lower_line_ast(
                    &mut builder,
                    &mut state,
                    &mut annotations,
                    line,
                    allow_unsupported,
                    &mut last_restrictable_ability,
                )?;
            }
            NormalizedCardItem::Modal(modal) => {
                let abilities_before = builder.abilities.len();
                builder = lower_parsed_modal(builder, modal, allow_unsupported)?;
                update_last_restrictable_ability(
                    &builder,
                    abilities_before,
                    &mut last_restrictable_ability,
                );
            }
            NormalizedCardItem::LevelAbility(level) => {
                level_abilities.push(lower_level_ability_ast(level)?);
            }
        }
    }

    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }

    builder = finalize_lowered_card(builder, &mut state);
    Ok((builder.build(), annotations))
}

fn normalize_line_ast(
    line: ParsedLineAst,
    state: &mut CardNormalizationState,
) -> Result<NormalizedLineAst, CardTextError> {
    let ParsedLineAst {
        info,
        chunks,
        restrictions,
    } = line;
    let mut normalized_chunks = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        normalized_chunks.push(match chunk {
            LineAst::Abilities(actions) => NormalizedLineChunk::Abilities(actions),
            LineAst::StaticAbility(ability) => NormalizedLineChunk::StaticAbility(ability),
            LineAst::StaticAbilities(abilities) => NormalizedLineChunk::StaticAbilities(abilities),
            LineAst::Ability(parsed) => {
                NormalizedLineChunk::Ability(normalize_parsed_ability(parsed)?)
            }
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => {
                let prepared = prepare_triggered_effects_for_lowering(
                    &trigger,
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
                let prepared =
                    prepare_effects_for_lowering(&effects, state.statement_reference_imports())?;
                state.latest_spell_exports = prepared.exports.clone();
                NormalizedLineChunk::Statement {
                    effects_ast: effects,
                    prepared,
                }
            }
            LineAst::AdditionalCost { effects } => {
                let prepared = prepare_effects_for_lowering(&effects, ReferenceImports::default())?;
                state.latest_additional_cost_exports = prepared.exports.clone();
                NormalizedLineChunk::AdditionalCost {
                    effects_ast: effects,
                    prepared,
                }
            }
            LineAst::OptionalCost(cost) => NormalizedLineChunk::OptionalCost(cost),
            LineAst::AdditionalCostChoice { options } => {
                let mut normalized_options = Vec::with_capacity(options.len());
                let mut exports = ReferenceExports::default();
                let mut saw_option = false;
                for option in options {
                    let prepared =
                        prepare_effects_for_lowering(&option.effects, ReferenceImports::default())?;
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
                NormalizedLineChunk::AlternativeCastingMethod(method)
            }
        });
    }

    Ok(NormalizedLineAst {
        info,
        chunks: normalized_chunks,
        restrictions,
    })
}

fn normalize_parsed_ability(
    parsed: ParsedAbility,
) -> Result<NormalizedParsedAbility, CardTextError> {
    let prepared = match parsed.effects_ast.as_ref() {
        None => None,
        Some(_)
            if matches!(
                &parsed.ability.kind,
                AbilityKind::Activated(activated)
                    if !activated.effects.is_empty() || !activated.choices.is_empty()
            ) =>
        {
            None
        }
        Some(_)
            if matches!(
                &parsed.ability.kind,
                AbilityKind::Triggered(triggered)
                    if !triggered.effects.is_empty() || !triggered.choices.is_empty()
            ) =>
        {
            None
        }
        Some(effects_ast) => match (&parsed.ability.kind, parsed.trigger_spec.as_ref()) {
            (AbilityKind::Triggered(_), Some(trigger)) => {
                Some(NormalizedPreparedAbility::Triggered {
                    trigger: trigger.clone(),
                    prepared: prepare_triggered_effects_for_lowering(
                        trigger,
                        effects_ast,
                        parsed.reference_imports.clone(),
                    )?,
                })
            }
            (AbilityKind::Activated(_), _) => Some(NormalizedPreparedAbility::Activated(
                prepare_effects_with_trigger_context_for_lowering(
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

fn normalize_modal_ast(modal: ParsedModalAst) -> Result<NormalizedModalAst, CardTextError> {
    let prepared_prefix = if modal.header.prefix_effects_ast.is_empty() {
        None
    } else if modal.header.trigger.is_some() || modal.header.activated.is_some() {
        Some(prepare_effects_with_trigger_context_for_lowering(
            modal.header.trigger.as_ref(),
            &modal.header.prefix_effects_ast,
            ReferenceImports::default(),
        )?)
    } else {
        Some(prepare_effects_for_lowering(
            &modal.header.prefix_effects_ast,
            ReferenceImports::default(),
        )?)
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let prepared =
            prepare_effects_for_lowering(&mode.effects_ast, ReferenceImports::default())?;
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

fn lower_line_ast(
    builder: &mut CardDefinitionBuilder,
    state: &mut LoweredCardState,
    annotations: &mut ParseAnnotations,
    line: NormalizedLineAst,
    allow_unsupported: bool,
    last_restrictable_ability: &mut Option<usize>,
) -> Result<(), CardTextError> {
    let NormalizedLineAst {
        info,
        chunks,
        mut restrictions,
    } = line;
    let mut handled_restrictions_for_new_ability = false;

    for parsed in chunks {
        if let NormalizedLineChunk::Statement { effects_ast, .. } = &parsed
            && apply_instead_followup_statement_to_last_ability(
                builder,
                *last_restrictable_ability,
                effects_ast,
                &info,
                annotations,
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
            allow_unsupported,
            annotations,
        )?;
        let abilities_after = builder.abilities.len();

        for ability_idx in abilities_before..abilities_after {
            apply_pending_restrictions_to_ability(
                &mut builder.abilities[ability_idx],
                &mut restrictions,
            );
            handled_restrictions_for_new_ability = true;
        }

        update_last_restrictable_ability(builder, abilities_before, last_restrictable_ability);
    }

    if !handled_restrictions_for_new_ability
        && let Some(index) = *last_restrictable_ability
        && index < builder.abilities.len()
    {
        apply_pending_restrictions_to_ability(&mut builder.abilities[index], &mut restrictions);
    }

    Ok(())
}

fn update_last_restrictable_ability(
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

fn lower_level_ability_ast(level: ParsedLevelAbilityAst) -> Result<LevelAbility, CardTextError> {
    let mut lowered = LevelAbility::new(level.min_level, level.max_level);
    if let Some((power, toughness)) = level.pt {
        lowered = lowered.with_pt(power, toughness);
    }

    for item in level.items {
        match item {
            ParsedLevelAbilityItemAst::StaticAbilities(abilities) => {
                lowered
                    .abilities
                    .extend(lower_static_abilities_ast(abilities)?);
            }
            ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                for action in actions {
                    if let Some(ability) = keyword_action_to_static_ability(action) {
                        lowered.abilities.push(ability);
                    }
                }
            }
        }
    }

    Ok(lowered)
}

pub(crate) fn lower_parsed_modal(
    builder: CardDefinitionBuilder,
    modal: NormalizedModalAst,
    allow_unsupported: bool,
) -> Result<CardDefinitionBuilder, CardTextError> {
    finalize_pending_modal(builder, modal, allow_unsupported)
}

fn finalize_lowered_card(
    mut builder: CardDefinitionBuilder,
    state: &mut LoweredCardState,
) -> CardDefinitionBuilder {
    builder = normalize_spell_delayed_trigger_effects(builder);
    builder = normalize_take_to_the_streets_spell_effect(builder);
    apply_pending_mechanic_linkages(builder, state)
}

fn normalize_spell_delayed_trigger_effects(
    mut builder: CardDefinitionBuilder,
) -> CardDefinitionBuilder {
    use crate::ability::AbilityKind;
    use crate::target::PlayerFilter;

    let is_spell = builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery));
    if !is_spell {
        return builder;
    }

    let mut delayed = Vec::new();
    builder.abilities.retain(|ability| {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            return true;
        };
        let ability_text = ability
            .text
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !ability_text.contains("this turn") {
            return true;
        }

        delayed.push(crate::effect::Effect::new(
            crate::effects::ScheduleDelayedTriggerEffect::new(
                triggered.trigger.clone(),
                triggered.effects.clone(),
                false,
                Vec::new(),
                PlayerFilter::You,
            )
            .until_end_of_turn(),
        ));
        false
    });

    if delayed.is_empty() {
        return builder;
    }

    builder
        .spell_effect
        .get_or_insert_with(Vec::new)
        .extend(delayed);
    builder
}

fn normalize_take_to_the_streets_spell_effect(
    mut builder: CardDefinitionBuilder,
) -> CardDefinitionBuilder {
    use crate::continuous::Modification;
    use crate::effect::{Effect, Value};
    use crate::effects::continuous::RuntimeModification;
    use crate::static_abilities::StaticAbilityId;
    use crate::types::Subtype;

    let Some(effects) = builder.spell_effect.as_ref() else {
        return builder;
    };
    if effects.len() != 2 {
        return builder;
    }

    let Some(apply) = effects[1].downcast_ref::<crate::effects::ApplyContinuousEffect>() else {
        return builder;
    };
    if apply.until != crate::effect::Until::EndOfTurn {
        return builder;
    }
    let filter = match &apply.target {
        crate::continuous::EffectTarget::Filter(filter) => filter,
        _ => return builder,
    };
    if filter.controller != Some(crate::target::PlayerFilter::You)
        || !filter.subtypes.contains(&Subtype::Citizen)
    {
        return builder;
    }
    let is_vigilance = apply.modification.as_ref().is_some_and(|m| match m {
        Modification::AddAbility(ability) => ability.id() == StaticAbilityId::Vigilance,
        _ => false,
    });
    if !is_vigilance {
        return builder;
    }
    if apply
        .runtime_modifications
        .iter()
        .any(|m| matches!(m, RuntimeModification::ModifyPowerToughness { .. }))
    {
        return builder;
    }

    let mut updated = apply.clone();
    updated
        .runtime_modifications
        .push(RuntimeModification::ModifyPowerToughness {
            power: Value::Fixed(1),
            toughness: Value::Fixed(1),
        });

    let mut new_effects = effects.clone();
    new_effects[1] = Effect::new(updated);
    builder.spell_effect = Some(new_effects);
    builder
}

fn title_case_words(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn color_set_name(colors: ColorSet) -> Option<&'static str> {
    if colors == ColorSet::WHITE {
        return Some("white");
    }
    if colors == ColorSet::BLUE {
        return Some("blue");
    }
    if colors == ColorSet::BLACK {
        return Some("black");
    }
    if colors == ColorSet::RED {
        return Some("red");
    }
    if colors == ColorSet::GREEN {
        return Some("green");
    }
    None
}

fn keyword_action_line_text(action: &KeywordAction) -> String {
    match action {
        KeywordAction::Flying => "Flying".to_string(),
        KeywordAction::Menace => "Menace".to_string(),
        KeywordAction::Hexproof => "Hexproof".to_string(),
        KeywordAction::Haste => "Haste".to_string(),
        KeywordAction::Improvise => "Improvise".to_string(),
        KeywordAction::Convoke => "Convoke".to_string(),
        KeywordAction::AffinityForArtifacts => "Affinity for artifacts".to_string(),
        KeywordAction::Delve => "Delve".to_string(),
        KeywordAction::FirstStrike => "First strike".to_string(),
        KeywordAction::DoubleStrike => "Double strike".to_string(),
        KeywordAction::Deathtouch => "Deathtouch".to_string(),
        KeywordAction::Lifelink => "Lifelink".to_string(),
        KeywordAction::Vigilance => "Vigilance".to_string(),
        KeywordAction::Trample => "Trample".to_string(),
        KeywordAction::Reach => "Reach".to_string(),
        KeywordAction::Defender => "Defender".to_string(),
        KeywordAction::Flash => "Flash".to_string(),
        KeywordAction::Phasing => "Phasing".to_string(),
        KeywordAction::Indestructible => "Indestructible".to_string(),
        KeywordAction::Shroud => "Shroud".to_string(),
        KeywordAction::Ward(amount) => format!("Ward {{{amount}}}"),
        KeywordAction::Wither => "Wither".to_string(),
        KeywordAction::Afterlife(amount) => format!("Afterlife {amount}"),
        KeywordAction::Fabricate(amount) => format!("Fabricate {amount}"),
        KeywordAction::Infect => "Infect".to_string(),
        KeywordAction::Undying => "Undying".to_string(),
        KeywordAction::Persist => "Persist".to_string(),
        KeywordAction::Prowess => "Prowess".to_string(),
        KeywordAction::Exalted => "Exalted".to_string(),
        KeywordAction::Cascade => "Cascade".to_string(),
        KeywordAction::Storm => "Storm".to_string(),
        KeywordAction::Toxic(amount) => format!("Toxic {amount}"),
        KeywordAction::BattleCry => "Battle cry".to_string(),
        KeywordAction::Dethrone => "Dethrone".to_string(),
        KeywordAction::Evolve => "Evolve".to_string(),
        KeywordAction::Ingest => "Ingest".to_string(),
        KeywordAction::Mentor => "Mentor".to_string(),
        KeywordAction::Skulk => "Skulk".to_string(),
        KeywordAction::Training => "Training".to_string(),
        KeywordAction::Myriad => "Myriad".to_string(),
        KeywordAction::Riot => "Riot".to_string(),
        KeywordAction::Unleash => "Unleash".to_string(),
        KeywordAction::Renown(amount) => format!("Renown {amount}"),
        KeywordAction::Modular(amount) => format!("Modular {amount}"),
        KeywordAction::ModularSunburst => "Modular—Sunburst".to_string(),
        KeywordAction::Graft(amount) => format!("Graft {amount}"),
        KeywordAction::Soulbond => "Soulbond".to_string(),
        KeywordAction::Soulshift(amount) => format!("Soulshift {amount}"),
        KeywordAction::Outlast(cost) => format!("Outlast {}", cost.to_oracle()),
        KeywordAction::Unearth(cost) => format!("Unearth {}", cost.to_oracle()),
        KeywordAction::Ninjutsu(cost) => format!("Ninjutsu {}", cost.to_oracle()),
        KeywordAction::Echo { text, .. } => text.clone(),
        KeywordAction::CumulativeUpkeep { text, .. } => text.clone(),
        KeywordAction::Extort => "Extort".to_string(),
        KeywordAction::Partner => "Partner".to_string(),
        KeywordAction::Assist => "Assist".to_string(),
        KeywordAction::SplitSecond => "Split second".to_string(),
        KeywordAction::Rebound => "Rebound".to_string(),
        KeywordAction::Sunburst => "Sunburst".to_string(),
        KeywordAction::Fading(amount) => format!("Fading {amount}"),
        KeywordAction::Vanishing(amount) => format!("Vanishing {amount}"),
        KeywordAction::Fear => "Fear".to_string(),
        KeywordAction::Intimidate => "Intimidate".to_string(),
        KeywordAction::Shadow => "Shadow".to_string(),
        KeywordAction::Horsemanship => "Horsemanship".to_string(),
        KeywordAction::Flanking => "Flanking".to_string(),
        KeywordAction::Landwalk(subtype) => {
            let mut subtype = subtype.to_string().to_ascii_lowercase();
            subtype.push_str("walk");
            title_case_words(&subtype)
        }
        KeywordAction::Bloodthirst(amount) => format!("Bloodthirst {amount}"),
        KeywordAction::Rampage(amount) => format!("Rampage {amount}"),
        KeywordAction::Bushido(amount) => format!("Bushido {amount}"),
        KeywordAction::Changeling => "Changeling".to_string(),
        KeywordAction::ProtectionFrom(colors) => {
            if let Some(color_name) = color_set_name(*colors) {
                return format!("Protection from {color_name}");
            }
            "Protection from colors".to_string()
        }
        KeywordAction::ProtectionFromAllColors => "Protection from all colors".to_string(),
        KeywordAction::ProtectionFromColorless => "Protection from colorless".to_string(),
        KeywordAction::ProtectionFromEverything => "Protection from everything".to_string(),
        KeywordAction::ProtectionFromCardType(card_type) => {
            format!("Protection from {}", card_type.name()).to_ascii_lowercase()
        }
        KeywordAction::ProtectionFromSubtype(subtype) => {
            format!(
                "Protection from {}",
                subtype.to_string().to_ascii_lowercase()
            )
        }
        KeywordAction::Unblockable => "This creature can't be blocked".to_string(),
        KeywordAction::Devoid => "Devoid".to_string(),
        KeywordAction::Annihilator(amount) => format!("Annihilator {amount}"),
        KeywordAction::ForMirrodin => "For Mirrodin!".to_string(),
        KeywordAction::LivingWeapon => "Living weapon".to_string(),
        KeywordAction::Crew { amount, .. } => format!("Crew {amount}"),
        KeywordAction::Saddle { amount, .. } => format!("Saddle {amount}"),
        KeywordAction::Marker(name) => title_case_words(name),
        KeywordAction::MarkerText(text) => text.clone(),
        KeywordAction::Casualty(power) => format!("Casualty {power}"),
        KeywordAction::Conspire => "Conspire".to_string(),
        KeywordAction::Devour(multiplier) => format!("Devour {multiplier}"),
        KeywordAction::Ravenous => "Ravenous".to_string(),
        KeywordAction::Ascend => "Ascend".to_string(),
        KeywordAction::Daybound => "Daybound".to_string(),
        KeywordAction::Nightbound => "Nightbound".to_string(),
        KeywordAction::Haunt => "Haunt".to_string(),
        KeywordAction::Provoke => "Provoke".to_string(),
        KeywordAction::Undaunted => "Undaunted".to_string(),
        KeywordAction::Enlist => "Enlist".to_string(),
    }
}

fn keyword_actions_line_text(actions: &[KeywordAction], separator: &str) -> Option<String> {
    if actions.is_empty() {
        return None;
    }
    let parts = actions
        .iter()
        .map(keyword_action_line_text)
        .collect::<Vec<_>>();
    Some(parts.join(separator))
}

fn uses_spell_only_functional_zones(static_ability: &StaticAbility) -> bool {
    matches!(
        static_ability.id(),
        crate::static_abilities::StaticAbilityId::ConditionalSpellKeyword
            | crate::static_abilities::StaticAbilityId::ThisSpellCastRestriction
            | crate::static_abilities::StaticAbilityId::ThisSpellCostReduction
            | crate::static_abilities::StaticAbilityId::ThisSpellCostReductionManaCost
    )
}

fn uses_referenced_ability_functional_zones(
    static_ability: &StaticAbility,
    normalized_line: &str,
) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ActivatedAbilityCostReduction
        && normalized_line.starts_with("this ability costs")
}

fn infer_static_ability_functional_zones(normalized_line: &str) -> Option<Vec<Zone>> {
    let mut zones = Vec::new();
    for (needle, zone) in [
        ("this card is in your hand", Zone::Hand),
        ("this card is in your graveyard", Zone::Graveyard),
        ("this card is in your library", Zone::Library),
        ("this card is in exile", Zone::Exile),
        ("this card is in the command zone", Zone::Command),
    ] {
        if normalized_line.contains(needle) {
            zones.push(zone);
        }
    }

    if zones.is_empty() { None } else { Some(zones) }
}

fn infer_triggered_ability_functional_zones(
    trigger: &TriggerSpec,
    normalized_line: &str,
) -> Vec<Zone> {
    let mut zones = match trigger {
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            ..
        } => vec![Zone::Graveyard],
        _ => vec![Zone::Battlefield],
    };

    let normalized = normalized_line.to_ascii_lowercase();
    if normalized.contains("return this card from your graveyard") {
        zones = vec![Zone::Graveyard];
    }

    zones
}

fn apply_line_ast(
    mut builder: CardDefinitionBuilder,
    state: &mut LoweredCardState,
    parsed: NormalizedLineChunk,
    info: &LineInfo,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    match parsed {
        NormalizedLineChunk::Abilities(actions) => {
            let keyword_segment = info
                .raw_line
                .split('(')
                .next()
                .unwrap_or(info.raw_line.as_str());
            let separator = if keyword_segment.contains(';') {
                "; "
            } else {
                ", "
            };
            let line_text = if actions
                .iter()
                .any(|action| matches!(action, KeywordAction::Crew { .. }))
            {
                Some(keyword_segment.trim().to_string())
            } else {
                keyword_actions_line_text(&actions, separator)
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
        }
        NormalizedLineChunk::StaticAbility(ability) => {
            let ability = match lower_static_ability_ast(ability) {
                Ok(ability) => ability,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let mut compiled = Ability::static_ability(ability).with_text(info.raw_line.as_str());
            if let AbilityKind::Static(static_ability) = &compiled.kind
                && uses_spell_only_functional_zones(static_ability)
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
                && uses_referenced_ability_functional_zones(
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
                infer_static_ability_functional_zones(info.normalized.normalized.as_str())
            {
                compiled = compiled.in_zones(zones);
            }
            builder = builder.with_ability(compiled);
        }
        NormalizedLineChunk::StaticAbilities(abilities) => {
            let abilities = match lower_static_abilities_ast(abilities) {
                Ok(abilities) => abilities,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            for ability in abilities {
                let mut compiled =
                    Ability::static_ability(ability).with_text(info.raw_line.as_str());
                if let AbilityKind::Static(static_ability) = &compiled.kind
                    && uses_spell_only_functional_zones(static_ability)
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
                    && uses_referenced_ability_functional_zones(
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
                    infer_static_ability_functional_zones(info.normalized.normalized.as_str())
                {
                    compiled = compiled.in_zones(zones);
                }
                builder = builder.with_ability(compiled);
            }
        }
        NormalizedLineChunk::Ability(parsed_ability) => {
            let parsed_ability = lower_prepared_ability(parsed_ability)?;
            if let Some(ref effects_ast) = parsed_ability.effects_ast {
                collect_tag_spans_from_effects_with_context(
                    effects_ast,
                    annotations,
                    &info.normalized,
                );
            }

            let mut ability = parsed_ability.ability;
            if let AbilityKind::Activated(ref a) = ability.kind
                && a.is_mana_ability()
                && a.effects.is_empty()
            {
                if let Some(options) =
                    parse_mana_output_options_for_line(&info.raw_line, info.line_index)?
                    && options.len() > 1
                {
                    for option in options {
                        let mut split = ability.clone();
                        if let AbilityKind::Activated(ref mut inner) = split.kind {
                            inner.mana_output = Some(option);
                        }
                        builder = builder.with_ability(split.with_text(info.raw_line.as_str()));
                    }
                    return Ok(builder);
                }
            }

            if ability.text.is_none() {
                ability = ability.with_text(info.raw_line.as_str());
            }
            builder = builder.with_ability(ability);
        }
        NormalizedLineChunk::Statement {
            effects_ast,
            prepared,
        } => {
            if effects_ast.is_empty() {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
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

            let lowered = match lower_prepared_statement_effects(&prepared) {
                Ok(lowered) => lowered,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let compiled = lowered.effects;
            state.latest_spell_exports = lowered.exports;

            let normalized_line = info.normalized.normalized.as_str().to_ascii_lowercase();
            if normalized_line.contains(" instead")
                && compiled.len() == 1
                && let Some(ref mut existing) = builder.spell_effect
                && !existing.is_empty()
                && let Some(replacement) =
                    compiled[0].downcast_ref::<crate::effects::ConditionalEffect>()
                && replacement.if_false.is_empty()
                && let Some(previous_target) = existing
                    .last()
                    .and_then(|effect| effect.downcast_ref::<crate::effects::DealDamageEffect>())
                    .map(|damage| damage.target.clone())
                && replacement.if_true.len() == 1
                && let Some(replacement_damage) =
                    replacement.if_true[0].downcast_ref::<crate::effects::DealDamageEffect>()
            {
                let mut replacement = replacement.clone();
                if replacement_damage.target == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
                {
                    replacement.if_true = vec![Effect::deal_damage(
                        replacement_damage.amount.clone(),
                        previous_target,
                    )];
                }

                let previous = existing.pop().expect("checked non-empty above");
                existing.push(Effect::new(crate::effects::ConditionalEffect::new(
                    replacement.condition,
                    replacement.if_true,
                    vec![previous],
                )));
            } else if let Some(ref mut existing) = builder.spell_effect {
                existing.extend(compiled);
            } else {
                builder.spell_effect = Some(compiled);
            }
        }
        NormalizedLineChunk::AdditionalCost {
            effects_ast,
            prepared,
        } => {
            if effects_ast.is_empty() {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
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

            let lowered = match lower_prepared_statement_effects(&prepared) {
                Ok(lowered) => lowered,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let compiled = runtime_effects_to_costs(lowered.effects)?;
            state.latest_additional_cost_exports = lowered.exports;
            let mut costs = builder.additional_cost.costs().to_vec();
            costs.extend(compiled);
            builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
        }
        NormalizedLineChunk::OptionalCost(cost) => {
            builder = builder.optional_cost(cost);
        }
        NormalizedLineChunk::AdditionalCostChoice { options } => {
            if options.len() < 2 {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
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
                        return Ok(push_unsupported_marker(
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
                match lower_prepared_additional_cost_choice_modes_with_exports(&options) {
                    Ok(outputs) => outputs,
                    Err(err) if allow_unsupported => {
                        return Ok(push_unsupported_marker(
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
                crate::costs::Cost::try_from_runtime_effect(Effect::choose_one(modes))
                    .map_err(CardTextError::ParseError)?,
            );
            builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
        }
        NormalizedLineChunk::AlternativeCastingMethod(method) => {
            builder.alternative_casts.push(method);
        }
        NormalizedLineChunk::Triggered {
            trigger,
            prepared,
            max_triggers_per_turn,
        } => {
            let contains_haunted_creature_dies = matches!(
                &trigger,
                TriggerSpec::Either(_, right) if matches!(**right, TriggerSpec::HauntedCreatureDies)
            ) || matches!(
                &trigger,
                TriggerSpec::HauntedCreatureDies
            );

            let functional_zones = infer_triggered_ability_functional_zones(
                &trigger,
                info.normalized.normalized.as_str(),
            );
            let parsed = parsed_triggered_ability(
                trigger.clone(),
                prepared.prepared.effects.clone(),
                functional_zones,
                Some(info.raw_line.clone()),
                max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
                prepared.prepared.imports.clone(),
            );
            let parsed = match lower_prepared_ability(NormalizedParsedAbility {
                parsed,
                prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
            }) {
                Ok(parsed) => parsed,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
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
                state.haunt_linkage = Some((triggered.effects.clone(), triggered.choices.clone()));
            }
            builder = builder.with_ability(parsed.ability);
        }
    }

    Ok(builder)
}

fn push_unsupported_marker(
    builder: CardDefinitionBuilder,
    raw_line: &str,
    reason: String,
) -> CardDefinitionBuilder {
    builder.with_ability(
        Ability::static_ability(StaticAbility::unsupported_parser_line(
            raw_line.trim(),
            reason,
        ))
        .with_text(raw_line),
    )
}

fn apply_pending_mechanic_linkages(
    mut builder: CardDefinitionBuilder,
    state: &mut LoweredCardState,
) -> CardDefinitionBuilder {
    let Some((haunt_effects, haunt_choices)) = state.haunt_linkage.take() else {
        return builder;
    };

    for ability in &mut builder.abilities {
        if ability.text.as_deref() == Some("Haunt") {
            if let crate::ability::AbilityKind::Triggered(ref mut triggered) = ability.kind {
                triggered.effects = vec![crate::effect::Effect::haunt_exile(
                    haunt_effects,
                    haunt_choices,
                )];
                break;
            }
        }
    }

    builder
}

fn try_merge_modal_into_remove_mode(
    effects: &mut Vec<Effect>,
    modal_effect: Effect,
    predicate: EffectPredicate,
) -> bool {
    let Some(last_effect) = effects.pop() else {
        return false;
    };

    let Some(choose_mode) = last_effect.downcast_ref::<crate::effects::ChooseModeEffect>() else {
        effects.push(last_effect);
        return false;
    };
    if choose_mode.modes.len() < 2 {
        effects.push(last_effect);
        return false;
    }

    let Some(remove_mode_idx) = choose_mode
        .modes
        .iter()
        .position(|mode| mode.description.to_ascii_lowercase().starts_with("remove "))
    else {
        effects.push(last_effect);
        return false;
    };

    let mut modes = choose_mode.modes.clone();
    let remove_mode = &mut modes[remove_mode_idx];
    let gate_id = EffectId(1_000_000_000);
    if let Some(last_remove_effect) = remove_mode.effects.pop() {
        remove_mode
            .effects
            .push(Effect::with_id(gate_id.0, last_remove_effect));
        remove_mode
            .effects
            .push(Effect::if_then(gate_id, predicate, vec![modal_effect]));
    } else {
        remove_mode.effects.push(modal_effect);
    }

    effects.push(Effect::new(crate::effects::ChooseModeEffect {
        modes,
        choose_count: choose_mode.choose_count.clone(),
        min_choose_count: choose_mode.min_choose_count.clone(),
        allow_repeated_modes: choose_mode.allow_repeated_modes,
        disallow_previously_chosen_modes: choose_mode.disallow_previously_chosen_modes,
        disallow_previously_chosen_modes_this_turn: choose_mode
            .disallow_previously_chosen_modes_this_turn,
    }));
    true
}

fn finalize_pending_modal(
    mut builder: CardDefinitionBuilder,
    pending_modal: NormalizedModalAst,
    allow_unsupported: bool,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let NormalizedModalAst {
        header,
        prepared_prefix,
        modes,
    } = pending_modal;
    let ParsedModalHeader {
        min: header_min,
        max: header_max,
        same_mode_more_than_once,
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        commander_allows_both,
        trigger,
        activated,
        x_replacement: _,
        prefix_effects_ast: _,
        modal_gate,
        line_text,
    } = header;

    let (prefix_effects, prefix_choices) = if prepared_prefix.is_none() {
        (Vec::new(), Vec::new())
    } else if trigger.is_some() || activated.is_some() {
        match lower_prepared_effects_with_trigger_context(
            prepared_prefix
                .as_ref()
                .expect("prepared prefix exists when checked above"),
        ) {
            Ok(lowered) => (lowered.effects, lowered.choices),
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(builder, line_text.as_str(), format!("{err:?}"));
                return Ok(builder);
            }
            Err(err) => return Err(err),
        }
    } else {
        match lower_prepared_statement_effects(
            prepared_prefix
                .as_ref()
                .expect("prepared prefix exists when checked above"),
        ) {
            Ok(lowered) => (lowered.effects, lowered.choices),
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(builder, line_text.as_str(), format!("{err:?}"));
                return Ok(builder);
            }
            Err(err) => return Err(err),
        }
    };

    let mut compiled_modes = Vec::new();
    for mode in modes {
        let effects = match lower_prepared_statement_effects(&mode.prepared) {
            Ok(lowered) => lowered.effects,
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(
                    builder,
                    mode.info.raw_line.as_str(),
                    format!("{err:?}"),
                );
                continue;
            }
            Err(err) => return Err(err),
        };
        compiled_modes.push(EffectMode {
            description: mode.description,
            effects,
        });
    }

    if compiled_modes.is_empty() {
        return Ok(builder);
    }

    let mode_count = compiled_modes.len() as i32;
    let default_max = Value::Fixed(mode_count);
    let max = header_max.unwrap_or_else(|| default_max.clone());
    let min = header_min;
    let is_fixed_one = |value: &Value| matches!(value, Value::Fixed(1));
    let with_unchosen_requirement = |effect: Effect| {
        if !mode_must_be_unchosen {
            return effect;
        }
        if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
            let choose_mode = choose_mode.clone();
            let choose_mode = if mode_must_be_unchosen_this_turn {
                choose_mode.with_previously_unchosen_modes_only_this_turn()
            } else {
                choose_mode.with_previously_unchosen_modes_only()
            };
            return Effect::new(choose_mode);
        }
        effect
    };

    let modal_effect = if commander_allows_both {
        let max_both = (mode_count.min(2)).max(1);
        let choose_both = if max_both == 1 {
            with_unchosen_requirement(Effect::choose_one(compiled_modes.clone()))
        } else {
            with_unchosen_requirement(Effect::choose_up_to(max_both, 1, compiled_modes.clone()))
        };
        let choose_one = with_unchosen_requirement(Effect::choose_one(compiled_modes.clone()));
        Effect::conditional(
            Condition::YouControlCommander,
            vec![choose_both],
            vec![choose_one],
        )
    } else if same_mode_more_than_once && min == max {
        with_unchosen_requirement(Effect::choose_exactly_allow_repeated_modes(
            max.clone(),
            compiled_modes,
        ))
    } else if is_fixed_one(&min) && is_fixed_one(&max) {
        with_unchosen_requirement(Effect::choose_one(compiled_modes))
    } else if min == max {
        with_unchosen_requirement(Effect::choose_exactly(max.clone(), compiled_modes))
    } else {
        with_unchosen_requirement(Effect::choose_up_to(
            max.clone(),
            min.clone(),
            compiled_modes,
        ))
    };

    let mut combined_effects = prefix_effects;
    if let Some(modal_gate) = modal_gate {
        if modal_gate.remove_mode_only
            && try_merge_modal_into_remove_mode(
                &mut combined_effects,
                modal_effect.clone(),
                modal_gate.predicate.clone(),
            )
        {
        } else if let Some(last_effect) = combined_effects.pop() {
            let gate_id = EffectId(1_000_000_000);
            combined_effects.push(Effect::with_id(gate_id.0, last_effect));
            combined_effects.push(Effect::if_then(
                gate_id,
                modal_gate.predicate,
                vec![modal_effect],
            ));
        } else {
            combined_effects.push(modal_effect);
        }
    } else {
        combined_effects.push(modal_effect);
    }

    if let Some(trigger) = trigger {
        let mut ability = parsed_triggered_ability(
            trigger,
            Vec::new(),
            vec![Zone::Battlefield],
            Some(line_text),
            None,
            ReferenceImports::default(),
        )
        .ability;
        if let AbilityKind::Triggered(triggered) = &mut ability.kind {
            triggered.effects = combined_effects;
            triggered.choices = prefix_choices;
        }
        builder = builder.with_ability(ability);
    } else if let Some(activated) = activated {
        builder = builder.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: activated.mana_cost,
                effects: combined_effects,
                choices: prefix_choices,
                timing: activated.timing,
                additional_restrictions: activated.additional_restrictions,
                activation_restrictions: activated.activation_restrictions,
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: activated.functional_zones,
            text: Some(line_text),
        });
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.extend(combined_effects);
    } else {
        builder.spell_effect = Some(combined_effects);
    }

    Ok(builder)
}

pub(crate) fn apply_pending_restrictions_to_ability(
    ability: &mut Ability,
    pending: &mut ParsedRestrictions,
) {
    let activation_restrictions = std::mem::take(&mut pending.activation);
    let trigger_restrictions = std::mem::take(&mut pending.trigger);

    match &mut ability.kind {
        AbilityKind::Activated(ability) => {
            if activation_restrictions.is_empty() {
                return;
            }
            if ability.is_mana_ability() {
                for restriction in &activation_restrictions {
                    apply_pending_mana_restriction(ability, restriction);
                }
            } else {
                for restriction in &activation_restrictions {
                    apply_pending_activation_restriction(ability, restriction);
                }
            }
        }
        AbilityKind::Triggered(ability) => {
            if trigger_restrictions.is_empty() {
                return;
            }
            for restriction in &trigger_restrictions {
                apply_pending_trigger_restriction(ability, restriction);
            }
        }
        _ => {}
    }

    if !activation_restrictions.is_empty() {
        pending.activation.extend(activation_restrictions);
    }
    if !trigger_restrictions.is_empty() {
        pending.trigger.extend(trigger_restrictions);
    }
}

pub(crate) fn is_restrictable_ability(ability: &Ability) -> bool {
    matches!(
        ability.kind,
        AbilityKind::Activated(_) | AbilityKind::Triggered(_)
    )
}

fn apply_pending_activation_restriction(
    ability: &mut crate::ability::ActivatedAbility,
    restriction: &str,
) {
    fn push_restriction_condition(
        ability: &mut crate::ability::ActivatedAbility,
        condition: crate::ConditionExpr,
    ) {
        if !ability
            .activation_restrictions
            .iter()
            .any(|existing| existing == &condition)
        {
            ability.activation_restrictions.push(condition);
        }
    }

    fn parse_text_only_activation_restriction_condition(
        restriction: &str,
    ) -> Option<crate::ConditionExpr> {
        let lower = restriction
            .trim()
            .to_ascii_lowercase()
            .trim_end_matches('.')
            .to_string();

        if lower.contains("didn't attack this turn")
            || lower.contains("did not attack this turn")
            || lower.contains("has not attacked this turn")
        {
            return Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::SourceAttackedThisTurn,
            )));
        }

        if lower.contains("this creature attacked this turn")
            || lower.contains("it attacked this turn")
            || lower.contains("that creature attacked this turn")
        {
            return Some(crate::ConditionExpr::SourceAttackedThisTurn);
        }

        None
    }

    let tokens = tokenize_line(restriction, 0);
    let parsed_timing = parse_activate_only_timing(&tokens);
    let parsed_condition = parse_activation_condition(&tokens);
    if parsed_condition.is_some() {
        let existing = ability.activation_condition.take();
        ability.activation_condition =
            merge_mana_activation_conditions(existing, parsed_condition.clone());
    }

    let mut timing_applied = false;
    if let Some(parsed_timing) = parsed_timing.as_ref() {
        let merged_timing = merge_activation_timing(&ability.timing, parsed_timing.clone());
        timing_applied = &merged_timing == parsed_timing;
        ability.timing = merged_timing;
        if !timing_applied {
            push_restriction_condition(
                ability,
                crate::ConditionExpr::ActivationTiming(parsed_timing.clone()),
            );
        }
    }

    if let Some(crate::ConditionExpr::MaxActivationsPerTurn(limit)) = parsed_condition {
        push_restriction_condition(ability, crate::ConditionExpr::MaxActivationsPerTurn(limit));
    }

    if let Some(text_condition) = parse_text_only_activation_restriction_condition(restriction) {
        push_restriction_condition(ability, text_condition);
    }

    let restriction = if parsed_timing.is_some() && !timing_applied {
        Some(normalize_restriction_text(restriction))
    } else {
        normalize_activation_restriction(restriction, parsed_timing.as_ref())
    };
    if let Some(restriction) = restriction {
        ability.additional_restrictions.push(restriction);
    }
}

fn apply_pending_trigger_restriction(ability: &mut TriggeredAbility, restriction: &str) {
    let tokens = tokenize_line(restriction, 0);
    let count = parse_triggered_times_each_turn_from_words(&words(&tokens));
    if let Some(parsed_count) = count {
        ability.intervening_if = Some(match ability.intervening_if.take() {
            Some(crate::ConditionExpr::MaxTimesEachTurn(existing)) => {
                crate::ConditionExpr::MaxTimesEachTurn(existing.min(parsed_count))
            }
            _ => crate::ConditionExpr::MaxTimesEachTurn(parsed_count),
        });
    }
}

fn apply_pending_mana_restriction(
    ability: &mut crate::ability::ActivatedAbility,
    restriction: &str,
) {
    let normalized_restriction = normalize_restriction_text(restriction);
    if normalized_restriction.is_empty() {
        return;
    }
    let tokens = tokenize_line(&normalized_restriction, 0);
    let parsed_timing = parse_activate_only_timing(&tokens).unwrap_or_default();
    let parsed_usage_restriction = parse_mana_usage_restriction_sentence(&tokens);
    let has_usage_restriction = parsed_usage_restriction.is_some();
    let parsed_condition = parse_activation_condition(&tokens).or_else(|| {
        if parsed_timing == ActivationTiming::AnyTime && !has_usage_restriction {
            Some(crate::ConditionExpr::Unmodeled(
                normalized_restriction.clone(),
            ))
        } else {
            None
        }
    });

    if let Some(restriction) = parsed_usage_restriction {
        ability.mana_usage_restrictions.push(restriction);
    }

    if parsed_condition.is_none()
        && parsed_timing == ActivationTiming::AnyTime
        && !has_usage_restriction
    {
        return;
    }

    if parsed_condition.is_none() && parsed_timing == ActivationTiming::AnyTime {
        return;
    }

    let condition_with_timing = parsed_condition
        .map(|condition| combine_mana_activation_condition(Some(condition), parsed_timing.clone()))
        .unwrap_or_else(|| combine_mana_activation_condition(None, parsed_timing));

    let existing = ability.activation_condition.take();
    ability.activation_condition =
        merge_mana_activation_conditions(existing, condition_with_timing);
}

fn merge_activation_timing(
    existing: &crate::ability::ActivationTiming,
    next: crate::ability::ActivationTiming,
) -> ActivationTiming {
    match (existing, &next) {
        (current, crate::ability::ActivationTiming::AnyTime) => current.clone(),
        (crate::ability::ActivationTiming::AnyTime, _) => next,
        (current, next_timing) if current == next_timing => current.clone(),
        (current, _) => current.clone(),
    }
}

fn normalize_restriction_text(text: &str) -> String {
    text.trim().trim_end_matches('.').trim().to_string()
}

fn normalize_activation_restriction(
    restriction: &str,
    timing: Option<&ActivationTiming>,
) -> Option<String> {
    if timing != Some(&ActivationTiming::OncePerTurn) {
        return Some(restriction.to_string());
    }
    let mut normalized = restriction.to_ascii_lowercase();
    if normalized == "activate only once each turn" {
        return None;
    }
    let prefix = "activate only once each turn and ";
    if normalized.starts_with(prefix) {
        normalized = normalized[prefix.len()..].trim_start().to_string();
    }
    let suffix = " and only once each turn";
    if normalized.ends_with(suffix) {
        normalized = normalized[..normalized.len() - suffix.len()]
            .trim_end()
            .to_string();
    }
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn merge_mana_activation_conditions(
    existing: Option<crate::ConditionExpr>,
    additional: Option<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    match (existing, additional) {
        (None, None) => None,
        (Some(condition), None) => Some(condition),
        (None, Some(condition)) => Some(condition),
        (Some(left), Some(right)) => {
            Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{IT_TAG, PlayerAst};
    use crate::*;

    #[test]
    fn prepare_effects_for_lowering_preserves_unresolved_it_and_returns_reference_imports() {
        let effects = vec![EffectAst::GrantPlayTaggedUntilEndOfTurn {
            tag: TagKey::from(IT_TAG),
            player: PlayerAst::You,
            allow_land: true,
        }];

        let prepared = prepare_effects_for_lowering(
            &effects,
            ReferenceImports::with_last_object_tag("seeded_target"),
        )
        .expect("statement effects should prepare");

        assert_eq!(
            prepared
                .imports
                .last_object_tag
                .as_ref()
                .map(TagKey::as_str),
            Some("seeded_target")
        );
        assert!(
            format!("{:?}", prepared.effects).contains(IT_TAG),
            "imports should not rewrite unresolved refs in the AST"
        );
    }

    #[test]
    fn prepare_effects_with_trigger_context_seeds_triggering_tag_prelude() {
        let effects = vec![EffectAst::GrantPlayTaggedUntilEndOfTurn {
            tag: TagKey::from(IT_TAG),
            player: PlayerAst::You,
            allow_land: true,
        }];

        let prepared = prepare_effects_with_trigger_context_for_lowering(
            Some(&TriggerSpec::ThisDies),
            &effects,
            ReferenceImports::default(),
        )
        .expect("triggered effects should prepare");

        assert_eq!(
            prepared
                .imports
                .last_object_tag
                .as_ref()
                .map(TagKey::as_str),
            Some("triggering")
        );
        assert!(matches!(
            prepared.prelude.first(),
            Some(EffectPreludeTag::TriggeringObject(tag)) if tag.as_str() == "triggering"
        ));
    }

    #[test]
    fn prepare_triggered_effects_extracts_intervening_if() {
        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::YourTurn,
            if_true: vec![EffectAst::Draw {
                count: Value::Fixed(1),
                player: PlayerAst::You,
            }],
            if_false: Vec::new(),
        }];

        let prepared = prepare_triggered_effects_for_lowering(
            &TriggerSpec::ThisDies,
            &effects,
            ReferenceImports::default(),
        )
        .expect("triggered ability preparation should succeed");

        assert!(prepared.intervening_if.is_some());
        assert!(matches!(
            prepared.prepared.effects.as_slice(),
            [EffectAst::Draw { .. }]
        ));
    }
}
