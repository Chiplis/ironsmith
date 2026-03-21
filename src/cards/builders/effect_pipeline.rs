use crate::cards::ParseAnnotations;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::builders::{
    AnnotatedEffectSequence, CardTextError, EffectAst, EffectReferenceResolutionConfig,
    KeywordAction, LineInfo, ParsedAbility, ParsedLevelAbilityAst, ParsedModalHeader,
    ParsedRestrictions, PredicateAst, ReferenceEnv, ReferenceExports, ReferenceImports,
    StaticAbilityAst, TriggerSpec,
    annotate_effect_sequence, effects_reference_it_tag, effects_reference_its_controller,
    effects_reference_tag, ensure_concrete_trigger_spec, inferred_trigger_player_filter,
    normalize_effects_ast, trigger_supports_event_value,
};
use crate::cost::OptionalCost;
use crate::effect::EventValueSpec;
use crate::filter::PlayerFilter;
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
    OptionalCostWithCastTrigger {
        cost: OptionalCost,
        prepared: PreparedEffectsForLowering,
        followup_text: String,
    },
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
    super::parse_text_with_annotations_rewrite_lowered(builder, text, allow_unsupported)
}
