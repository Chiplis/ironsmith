use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::cards::ParseAnnotations;
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, EffectAst, KeywordAction, LineInfo, ParsedAbility,
    StaticAbilityAst, TriggerSpec,
};
use crate::effect::{Condition, Effect, EffectMode, EventValueSpec};
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

use super::compile_support::{
    collect_tag_spans_from_effects_with_context, compile_trigger_spec, effects_reference_it_tag,
    effects_reference_its_controller, effects_reference_tag, ensure_concrete_trigger_spec,
    inferred_trigger_player_filter, materialize_prepared_effects_with_trigger_context,
    materialize_prepared_statement_effects, materialize_prepared_triggered_effects,
    trigger_binds_player_reference_context, trigger_supports_event_value,
};
use super::effect_ast_normalization::normalize_effects_ast;
use super::effect_pipeline::{
    EffectPreludeTag, NormalizedAdditionalCostChoiceOptionAst, NormalizedParsedAbility,
    NormalizedPreparedAbility, PreparedEffectsForLowering, PreparedPredicateForLowering,
    PreparedTriggeredEffectsForLowering,
};
use super::reference_model::{LoweredEffects, ReferenceEnv, ReferenceExports, ReferenceImports};
use super::reference_resolution::{EffectReferenceResolutionConfig, annotate_effect_sequence};
use super::util::classify_instead_followup_text;

fn rewrite_prepare_effects_from_normalized(
    semantic_effects: Vec<EffectAst>,
    reference_effects: &[EffectAst],
    mut imports: ReferenceImports,
    config: EffectReferenceResolutionConfig,
    inferred_last_player_filter: Option<PlayerFilter>,
    default_last_object_tag: Option<crate::cards::builders::TagKey>,
    include_trigger_prelude: bool,
) -> Result<PreparedEffectsForLowering, CardTextError> {
    let mut prelude = Vec::new();
    for tag in ["equipped", "enchanted"] {
        if effects_reference_tag(reference_effects, tag) {
            if imports.last_object_tag.is_none() {
                imports.last_object_tag = Some(crate::cards::builders::TagKey::from(tag));
            }
            prelude.push(EffectPreludeTag::AttachedSource(
                crate::cards::builders::TagKey::from(tag),
            ));
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
                EffectPreludeTag::TriggeringObject(crate::cards::builders::TagKey::from(
                    "triggering",
                )),
            );
        }
        let needs_damaged_prelude = default_last_object_tag
            .as_ref()
            .is_some_and(|tag| tag.as_str() == "damaged")
            || effects_reference_tag(reference_effects, "damaged");
        if needs_damaged_prelude {
            prelude.insert(
                0,
                EffectPreludeTag::TriggeringDamageTarget(crate::cards::builders::TagKey::from(
                    "damaged",
                )),
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

pub(crate) fn rewrite_prepare_effects_for_lowering(
    effects: &[EffectAst],
    imports: impl Into<ReferenceImports>,
) -> Result<PreparedEffectsForLowering, CardTextError> {
    let imports = imports.into();
    let normalized = normalize_effects_ast(effects);
    rewrite_prepare_effects_from_normalized(
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

pub(crate) fn rewrite_prepare_effects_with_trigger_context_for_lowering(
    trigger: Option<&TriggerSpec>,
    effects: &[EffectAst],
    imports: impl Into<ReferenceImports>,
) -> Result<PreparedEffectsForLowering, CardTextError> {
    let imports = imports.into();
    let normalized = normalize_effects_ast(effects);
    let default_last_object_tag = if imports.last_object_tag.is_none()
        && (effects_reference_it_tag(&normalized) || effects_reference_its_controller(&normalized))
    {
        Some(crate::cards::builders::TagKey::from(
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

    rewrite_prepare_effects_from_normalized(
        normalized.clone(),
        &normalized,
        imports,
        EffectReferenceResolutionConfig {
            allow_life_event_value: trigger
                .map(|t| trigger_supports_event_value(t, &EventValueSpec::Amount))
                .unwrap_or(false),
            ..Default::default()
        },
        trigger.and_then(inferred_trigger_player_filter),
        default_last_object_tag,
        trigger.is_some(),
    )
}

pub(crate) fn rewrite_prepare_triggered_effects_for_lowering(
    trigger: &TriggerSpec,
    effects: &[EffectAst],
    imports: impl Into<ReferenceImports>,
) -> Result<PreparedTriggeredEffectsForLowering, CardTextError> {
    let imports = imports.into();
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

    let prepared = rewrite_prepare_effects_from_normalized(
        body_effects,
        &normalized,
        imports,
        EffectReferenceResolutionConfig {
            allow_life_event_value: trigger_supports_event_value(trigger, &EventValueSpec::Amount),
            ..Default::default()
        },
        inferred_trigger_player_filter(trigger),
        if effects_reference_it_tag(&normalized) || effects_reference_its_controller(&normalized) {
            Some(crate::cards::builders::TagKey::from(
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

pub(crate) fn rewrite_lower_prepared_statement_effects(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    materialize_prepared_statement_effects(prepared)
}

pub(crate) fn rewrite_lower_prepared_additional_cost_choice_modes_with_exports(
    options: &[NormalizedAdditionalCostChoiceOptionAst],
) -> Result<(Vec<EffectMode>, ReferenceExports), CardTextError> {
    let mut exports = ReferenceExports::default();
    let mut first = true;
    let mut modes = Vec::with_capacity(options.len());
    for option in options {
        let lowered = rewrite_lower_prepared_statement_effects(&option.prepared)?;
        if first {
            exports = lowered.exports.clone();
            first = false;
        } else {
            exports = ReferenceExports::join(&exports, &lowered.exports);
        }
        modes.push(EffectMode {
            description: option.description.trim().to_string(),
            effects: lowered.effects.flattened_default_effects().to_vec(),
        });
    }
    Ok((modes, exports))
}

fn rewrite_prepare_parsed_ability_payload(
    parsed: &ParsedAbility,
) -> Result<Option<NormalizedPreparedAbility>, CardTextError> {
    let Some(effects_ast) = parsed.effects_ast.as_ref() else {
        return Ok(None);
    };

    if let AbilityKind::Activated(activated) = &parsed.ability.kind
        && (!activated.effects.is_empty() || !activated.choices.is_empty())
    {
        return Ok(None);
    }
    if let AbilityKind::Triggered(triggered) = &parsed.ability.kind
        && (!triggered.effects.is_empty() || !triggered.choices.is_empty())
    {
        return Ok(None);
    }

    Ok(match (&parsed.ability.kind, parsed.trigger_spec.as_ref()) {
        (AbilityKind::Triggered(_), Some(trigger)) => Some(NormalizedPreparedAbility::Triggered {
            trigger: trigger.clone(),
            prepared: rewrite_prepare_triggered_effects_for_lowering(
                trigger,
                effects_ast,
                parsed.reference_imports.clone(),
            )?,
        }),
        (AbilityKind::Activated(_), _) => Some(NormalizedPreparedAbility::Activated(
            rewrite_prepare_effects_with_trigger_context_for_lowering(
                None,
                effects_ast,
                parsed.reference_imports.clone(),
            )?,
        )),
        _ => None,
    })
}

fn rewrite_merge_intervening_conditions(
    existing: Option<crate::ConditionExpr>,
    additional: Option<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    match (existing, additional) {
        (Some(primary), Some(secondary)) => Some(crate::ConditionExpr::And(
            Box::new(primary),
            Box::new(secondary),
        )),
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (None, None) => None,
    }
}

fn rewrite_lower_parsed_ability_internal(
    mut parsed: ParsedAbility,
    prepared: Option<NormalizedPreparedAbility>,
) -> Result<ParsedAbility, CardTextError> {
    let Some(_) = parsed.effects_ast.as_ref() else {
        return Ok(parsed);
    };

    let prepared = match prepared {
        Some(prepared) => Some(prepared),
        None => rewrite_prepare_parsed_ability_payload(&parsed)?,
    };

    let AbilityKind::Activated(activated) = &mut parsed.ability.kind else {
        if let AbilityKind::Triggered(triggered) = &mut parsed.ability.kind {
            if !triggered.effects.is_empty() || !triggered.choices.is_empty() {
                return Ok(parsed);
            }
            let Some(NormalizedPreparedAbility::Triggered { trigger, prepared }) = prepared else {
                return Ok(parsed);
            };
            let (lowered, parsed_intervening_if) =
                materialize_prepared_triggered_effects(&prepared)?;
            rewrite_validate_iterated_player_bindings_in_lowered_effects(
                &lowered,
                trigger_binds_player_reference_context(&trigger),
                "triggered ability effects",
            )?;
            triggered.trigger = compile_trigger_spec(trigger);
            triggered.effects = lowered.effects;
            triggered.choices = lowered.choices;
            triggered.intervening_if = rewrite_merge_intervening_conditions(
                triggered.intervening_if.take(),
                parsed_intervening_if,
            );
            return Ok(parsed);
        }
        return Ok(parsed);
    };

    if !activated.effects.is_empty() || !activated.choices.is_empty() {
        return Ok(parsed);
    }

    let Some(NormalizedPreparedAbility::Activated(prepared)) = prepared else {
        return Ok(parsed);
    };
    let lowered = materialize_prepared_effects_with_trigger_context(&prepared)?;
    rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &lowered,
        false,
        "activated ability effects",
    )?;
    activated.effects = lowered.effects;
    activated.choices = lowered.choices;
    Ok(parsed)
}

pub(crate) fn rewrite_lower_parsed_ability(
    parsed: ParsedAbility,
) -> Result<ParsedAbility, CardTextError> {
    rewrite_lower_parsed_ability_internal(parsed, None)
}

pub(crate) fn rewrite_lower_prepared_ability(
    normalized: NormalizedParsedAbility,
) -> Result<ParsedAbility, CardTextError> {
    rewrite_lower_parsed_ability_internal(normalized.parsed, normalized.prepared)
}

pub(crate) fn rewrite_apply_instead_followup_statement_to_last_ability(
    builder: &mut CardDefinitionBuilder,
    last_restrictable_ability: Option<usize>,
    effects: &[EffectAst],
    info: &LineInfo,
    annotations: &mut ParseAnnotations,
) -> Result<bool, CardTextError> {
    let Some(index) = last_restrictable_ability else {
        return Ok(false);
    };
    if index >= builder.abilities.len() {
        return Ok(false);
    }

    let normalized = info.normalized.normalized.as_str().to_ascii_lowercase();
    if !normalized.starts_with("if ")
        || !matches!(
            classify_instead_followup_text(&normalized),
            crate::cards::builders::InsteadSemantics::SelfReplacement
        )
    {
        return Ok(false);
    }

    let compiled = rewrite_lower_prepared_statement_effects(
        &rewrite_prepare_effects_for_lowering(effects, ReferenceImports::default())?,
    )?;
    if compiled.effects.len() != 1 {
        return Ok(false);
    }

    let segment = match compiled.effects.segments.as_slice() {
        [segment] => segment,
        _ => return Ok(false),
    };
    if !segment.default_effects.is_empty() || segment.self_replacements.len() != 1 {
        return Ok(false);
    }

    let replacement = &segment.self_replacements[0];
    if !compiled.choices.is_empty() {
        return Ok(false);
    }

    collect_tag_spans_from_effects_with_context(effects, annotations, &info.normalized);

    match &mut builder.abilities[index].kind {
        AbilityKind::Triggered(ability) => {
            let Some(segment) = ability.effects.last_segment_mut() else {
                return Ok(false);
            };
            if segment.default_effects.is_empty() {
                return Ok(false);
            }
            segment
                .self_replacements
                .push(crate::resolution::SelfReplacementBranch::new(
                    replacement.condition.clone(),
                    replacement.replacement_effects.clone(),
                ));
        }
        AbilityKind::Activated(ability) => {
            let Some(segment) = ability.effects.last_segment_mut() else {
                return Ok(false);
            };
            if segment.default_effects.is_empty() {
                return Ok(false);
            }
            segment
                .self_replacements
                .push(crate::resolution::SelfReplacementBranch::new(
                    replacement.condition.clone(),
                    replacement.replacement_effects.clone(),
                ));
        }
        _ => return Ok(false),
    }

    Ok(true)
}

pub(crate) fn rewrite_parsed_triggered_ability(
    trigger: TriggerSpec,
    effects_ast: Vec<EffectAst>,
    functional_zones: Vec<Zone>,
    text: Option<String>,
    intervening_if: Option<crate::ConditionExpr>,
    reference_imports: impl Into<ReferenceImports>,
) -> ParsedAbility {
    let reference_imports = reference_imports.into();
    ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: compile_trigger_spec(trigger.clone()),
                effects: crate::resolution::ResolutionProgram::default(),
                choices: Vec::new(),
                intervening_if,
            }),
            functional_zones,
            text,
        },
        effects_ast: Some(effects_ast),
        trigger_spec: Some(trigger),
        reference_imports: reference_imports.into(),
    }
}

pub(crate) fn rewrite_static_ability_for_keyword_action(
    action: KeywordAction,
) -> Option<StaticAbility> {
    if !action.lowers_to_static_ability() {
        return None;
    }

    match action {
        KeywordAction::Flying => Some(StaticAbility::flying()),
        KeywordAction::Menace => Some(StaticAbility::menace()),
        KeywordAction::Hexproof => Some(StaticAbility::hexproof()),
        KeywordAction::Haste => Some(StaticAbility::haste()),
        KeywordAction::Improvise => Some(StaticAbility::improvise()),
        KeywordAction::Convoke => Some(StaticAbility::convoke()),
        KeywordAction::AffinityForArtifacts => Some(StaticAbility::affinity_for_artifacts()),
        KeywordAction::Delve => Some(StaticAbility::delve()),
        KeywordAction::FirstStrike => Some(StaticAbility::first_strike()),
        KeywordAction::DoubleStrike => Some(StaticAbility::double_strike()),
        KeywordAction::Deathtouch => Some(StaticAbility::deathtouch()),
        KeywordAction::Lifelink => Some(StaticAbility::lifelink()),
        KeywordAction::Vigilance => Some(StaticAbility::vigilance()),
        KeywordAction::Trample => Some(StaticAbility::trample()),
        KeywordAction::Reach => Some(StaticAbility::reach()),
        KeywordAction::Defender => Some(StaticAbility::defender()),
        KeywordAction::Flash => Some(StaticAbility::flash()),
        KeywordAction::Phasing => Some(StaticAbility::phasing()),
        KeywordAction::Indestructible => Some(StaticAbility::indestructible()),
        KeywordAction::Shroud => Some(StaticAbility::shroud()),
        KeywordAction::Ward(amount) => u8::try_from(amount).ok().map(|generic| {
            StaticAbility::ward(crate::cost::TotalCost::mana(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(generic),
            ])))
        }),
        KeywordAction::Wither => Some(StaticAbility::wither()),
        KeywordAction::Afterlife(amount) => {
            Some(StaticAbility::keyword_marker(format!("afterlife {amount}")))
        }
        KeywordAction::Fabricate(amount) => {
            Some(StaticAbility::keyword_marker(format!("fabricate {amount}")))
        }
        KeywordAction::Infect => Some(StaticAbility::infect()),
        KeywordAction::Undying => Some(StaticAbility::keyword_marker("undying".to_string())),
        KeywordAction::Persist => Some(StaticAbility::keyword_marker("persist".to_string())),
        KeywordAction::Prowess => Some(StaticAbility::keyword_marker("prowess".to_string())),
        KeywordAction::Exalted => Some(StaticAbility::keyword_marker("exalted".to_string())),
        KeywordAction::Cascade => Some(StaticAbility::cascade()),
        KeywordAction::Storm => Some(StaticAbility::keyword_marker("storm".to_string())),
        KeywordAction::Toxic(amount) => {
            Some(StaticAbility::keyword_marker(format!("toxic {amount}")))
        }
        KeywordAction::BattleCry => Some(StaticAbility::keyword_marker("battle cry".to_string())),
        KeywordAction::Dethrone => Some(StaticAbility::keyword_marker("dethrone".to_string())),
        KeywordAction::Evolve => Some(StaticAbility::keyword_marker("evolve".to_string())),
        KeywordAction::Ingest => Some(StaticAbility::keyword_marker("ingest".to_string())),
        KeywordAction::Mentor => Some(StaticAbility::keyword_marker("mentor".to_string())),
        KeywordAction::Skulk => Some(StaticAbility::skulk()),
        KeywordAction::Training => Some(StaticAbility::keyword_marker("training".to_string())),
        KeywordAction::Riot => Some(StaticAbility::keyword_marker("riot".to_string())),
        KeywordAction::Unleash => Some(StaticAbility::unleash()),
        KeywordAction::Renown(amount) => {
            Some(StaticAbility::keyword_marker(format!("renown {amount}")))
        }
        KeywordAction::Modular(amount) => {
            Some(StaticAbility::keyword_marker(format!("modular {amount}")))
        }
        KeywordAction::Graft(amount) => {
            Some(StaticAbility::keyword_marker(format!("graft {amount}")))
        }
        KeywordAction::Soulbond => Some(StaticAbility::keyword_marker("soulbond".to_string())),
        KeywordAction::Soulshift(amount) => {
            Some(StaticAbility::keyword_marker(format!("soulshift {amount}")))
        }
        KeywordAction::Outlast(cost) => Some(StaticAbility::keyword_marker(format!(
            "outlast {}",
            cost.to_oracle()
        ))),
        KeywordAction::Unearth(cost) => Some(StaticAbility::keyword_marker(format!(
            "unearth {}",
            cost.to_oracle()
        ))),
        KeywordAction::Ninjutsu(cost) => Some(StaticAbility::keyword_marker(format!(
            "ninjutsu {}",
            cost.to_oracle()
        ))),
        KeywordAction::Extort => Some(StaticAbility::keyword_marker("extort".to_string())),
        KeywordAction::Partner => Some(StaticAbility::partner()),
        KeywordAction::Assist => Some(StaticAbility::assist()),
        KeywordAction::SplitSecond => Some(StaticAbility::split_second()),
        KeywordAction::Rebound => Some(StaticAbility::rebound()),
        KeywordAction::Sunburst => Some(StaticAbility::keyword_marker("sunburst".to_string())),
        KeywordAction::Fading(amount) => {
            Some(StaticAbility::keyword_marker(format!("fading {amount}")))
        }
        KeywordAction::Vanishing(amount) => {
            Some(StaticAbility::keyword_marker(format!("vanishing {amount}")))
        }
        KeywordAction::Fear => Some(StaticAbility::fear()),
        KeywordAction::Intimidate => Some(StaticAbility::intimidate()),
        KeywordAction::Shadow => Some(StaticAbility::shadow()),
        KeywordAction::Horsemanship => Some(StaticAbility::horsemanship()),
        KeywordAction::Flanking => Some(StaticAbility::flanking()),
        KeywordAction::UmbraArmor => Some(StaticAbility::umbra_armor()),
        KeywordAction::Landwalk(subtype) => Some(StaticAbility::landwalk(subtype)),
        KeywordAction::Bloodthirst(amount) => Some(StaticAbility::bloodthirst(amount)),
        KeywordAction::Rampage(amount) => {
            Some(StaticAbility::keyword_marker(format!("rampage {amount}")))
        }
        KeywordAction::Bushido(amount) => {
            Some(StaticAbility::keyword_marker(format!("bushido {amount}")))
        }
        KeywordAction::Changeling => Some(StaticAbility::changeling()),
        KeywordAction::ProtectionFrom(colors) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(colors),
        )),
        KeywordAction::ProtectionFromAllColors => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::AllColors,
        )),
        KeywordAction::ProtectionFromColorless => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Colorless,
        )),
        KeywordAction::ProtectionFromEverything => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Everything,
        )),
        KeywordAction::ProtectionFromChosenPlayer => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::ChosenPlayer,
        )),
        KeywordAction::ProtectionFromCardType(card_type) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        )),
        KeywordAction::ProtectionFromSubtype(subtype) => {
            Some(StaticAbility::keyword_marker(format!(
                "protection from {}",
                subtype.to_string().to_ascii_lowercase()
            )))
        }
        KeywordAction::Unblockable => Some(StaticAbility::unblockable()),
        KeywordAction::Devoid => Some(StaticAbility::make_colorless(ObjectFilter::source())),
        KeywordAction::Annihilator(amount) => Some(StaticAbility::keyword_marker(format!(
            "annihilator {amount}"
        ))),
        KeywordAction::Marker(name) => Some(StaticAbility::keyword_marker(name)),
        KeywordAction::MarkerText(text) => Some(StaticAbility::keyword_marker(text)),
        _ => None,
    }
}

fn rewrite_lower_keyword_action_or_err(
    action: KeywordAction,
) -> Result<StaticAbility, CardTextError> {
    rewrite_static_ability_for_keyword_action(action).ok_or_else(|| {
        CardTextError::InvariantViolation(
            "static-ability lowering received a non-static keyword action".to_string(),
        )
    })
}

fn rewrite_lower_attached_keyword_action_grant(
    action: KeywordAction,
    display: String,
    condition: Option<crate::ConditionExpr>,
) -> Result<StaticAbility, CardTextError> {
    let granted = Ability::static_ability(rewrite_lower_keyword_action_or_err(action)?)
        .with_text(display.as_str());
    let mut grant = crate::static_abilities::AttachedAbilityGrant::new(granted, display);
    if let Some(condition) = condition {
        grant = grant.with_condition(condition);
    }
    Ok(StaticAbility::new(grant))
}

fn rewrite_lower_conditional_static_ability(
    ability: StaticAbilityAst,
    condition: crate::ConditionExpr,
) -> Result<StaticAbility, CardTextError> {
    let lowered = rewrite_lower_static_ability_ast(ability)?;
    Ok(lowered
        .clone()
        .with_condition(condition.clone())
        .unwrap_or_else(|| {
            StaticAbility::new(
                crate::static_abilities::GrantAbility::source(lowered).with_condition(condition),
            )
        }))
}

fn rewrite_lower_grant_static_ability(
    filter: crate::filter::ObjectFilter,
    ability: StaticAbilityAst,
    condition: Option<crate::ConditionExpr>,
) -> Result<StaticAbility, CardTextError> {
    let mut grant = crate::static_abilities::GrantAbility::new(
        filter,
        rewrite_lower_static_ability_ast(ability)?,
    );
    if let Some(condition) = condition {
        grant = grant.with_condition(condition);
    }
    Ok(StaticAbility::new(grant))
}

fn rewrite_lower_attached_static_ability_grant(
    ability: StaticAbilityAst,
    display: String,
    condition: Option<crate::ConditionExpr>,
) -> Result<StaticAbility, CardTextError> {
    let granted = Ability::static_ability(rewrite_lower_static_ability_ast(ability)?)
        .with_text(display.as_str());
    let mut grant = crate::static_abilities::AttachedAbilityGrant::new(granted, display);
    if let Some(condition) = condition {
        grant = grant.with_condition(condition);
    }
    Ok(StaticAbility::new(grant))
}

pub(crate) fn rewrite_lower_static_ability_ast(
    ability: StaticAbilityAst,
) -> Result<StaticAbility, CardTextError> {
    match ability {
        StaticAbilityAst::Static(ability) => Ok(ability),
        StaticAbilityAst::KeywordAction(action) => rewrite_lower_keyword_action_or_err(action),
        StaticAbilityAst::ConditionalStaticAbility { ability, condition } => {
            rewrite_lower_conditional_static_ability(*ability, condition)
        }
        StaticAbilityAst::ConditionalKeywordAction { action, condition } => {
            rewrite_lower_conditional_static_ability(
                StaticAbilityAst::KeywordAction(action),
                condition,
            )
        }
        StaticAbilityAst::GrantStaticAbility {
            filter,
            ability,
            condition,
        } => rewrite_lower_grant_static_ability(filter, *ability, condition),
        StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition,
        } => rewrite_lower_grant_static_ability(
            filter,
            StaticAbilityAst::KeywordAction(action),
            condition,
        ),
        StaticAbilityAst::RemoveStaticAbility { filter, ability } => Ok(
            StaticAbility::remove_ability(filter, rewrite_lower_static_ability_ast(*ability)?),
        ),
        StaticAbilityAst::RemoveKeywordAction { filter, action } => Ok(
            StaticAbility::remove_ability(filter, rewrite_lower_keyword_action_or_err(action)?),
        ),
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            display,
            condition,
        } => rewrite_lower_attached_static_ability_grant(*ability, display, condition),
        StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display,
            condition,
        } => rewrite_lower_attached_keyword_action_grant(action, display, condition),
        StaticAbilityAst::EquipmentKeywordActionsGrant { actions } => {
            let mut lowered = Vec::with_capacity(actions.len());
            for action in actions {
                lowered.push(rewrite_lower_keyword_action_or_err(action)?);
            }
            Ok(StaticAbility::equipment_grant(lowered))
        }
        StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition,
        } => {
            let mut lowered = rewrite_lower_parsed_ability(ability)?.ability;
            lowered.text = Some(display.clone());
            let mut grant =
                crate::static_abilities::GrantObjectAbilityForFilter::new(filter, lowered, display);
            if let Some(condition) = condition {
                grant = grant.with_condition(condition);
            }
            Ok(StaticAbility::new(grant))
        }
        StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display,
            condition,
        } => {
            let mut lowered = rewrite_lower_parsed_ability(ability)?.ability;
            lowered.text = Some(display.clone());
            let mut grant = crate::static_abilities::AttachedAbilityGrant::new(lowered, display);
            if let Some(condition) = condition {
                grant = grant.with_condition(condition);
            }
            Ok(StaticAbility::new(grant))
        }
        StaticAbilityAst::SoulbondSharedObjectAbility { ability, display } => {
            let mut lowered = rewrite_lower_parsed_ability(ability)?.ability;
            if lowered.text.is_none() {
                lowered.text = Some(display);
            }
            Ok(StaticAbility::soulbond_shared_object_ability(lowered))
        }
    }
}

pub(crate) fn rewrite_lower_static_abilities_ast(
    abilities: Vec<StaticAbilityAst>,
) -> Result<Vec<StaticAbility>, CardTextError> {
    abilities
        .into_iter()
        .map(rewrite_lower_static_ability_ast)
        .collect()
}

fn rewrite_validate_unbound_iterated_player(
    debug_repr: String,
    context: &str,
) -> Result<(), CardTextError> {
    if debug_repr.contains("IteratedPlayer") {
        return Err(CardTextError::InvariantViolation(format!(
            "{context} references PlayerFilter::IteratedPlayer without a trigger or loop that binds \"that player\": {debug_repr}"
        )));
    }
    Ok(())
}

fn rewrite_validate_choose_specs_for_iterated_player(
    choices: &[ChooseSpec],
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if iterated_player_bound {
        return Ok(());
    }
    for choice in choices {
        rewrite_validate_unbound_iterated_player(format!("{choice:?}"), context)?;
    }
    Ok(())
}

fn rewrite_validate_condition_for_iterated_player(
    condition: &Condition,
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if iterated_player_bound {
        return Ok(());
    }
    rewrite_validate_unbound_iterated_player(format!("{condition:?}"), context)
}

fn rewrite_validate_effects_for_iterated_player(
    effects: &[Effect],
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        rewrite_validate_effect_for_iterated_player(effect, iterated_player_bound, context)?;
    }
    Ok(())
}

fn rewrite_validate_effect_for_iterated_player(
    effect: &Effect,
    iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return rewrite_validate_effects_for_iterated_player(
            &sequence.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        if !iterated_player_bound && let Some(decider) = &may.decider {
            rewrite_validate_unbound_iterated_player(format!("{decider:?}"), context)?;
        }
        return rewrite_validate_effects_for_iterated_player(
            &may.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(format!("{:?}", unless_pays.player), context)?;
        }
        return rewrite_validate_effects_for_iterated_player(
            &unless_pays.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                format!("{:?}", unless_action.player),
                context,
            )?;
        }
        rewrite_validate_effects_for_iterated_player(
            &unless_action.effects,
            iterated_player_bound,
            context,
        )?;
        return rewrite_validate_effects_for_iterated_player(
            &unless_action.alternative,
            iterated_player_bound,
            context,
        );
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(format!("{:?}", for_players.filter), context)?;
        }
        return rewrite_validate_effects_for_iterated_player(&for_players.effects, true, context);
    }
    if let Some(for_each_object) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                format!("{:?}", for_each_object.filter),
                context,
            )?;
        }
        return rewrite_validate_effects_for_iterated_player(
            &for_each_object.effects,
            true,
            context,
        );
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return rewrite_validate_effects_for_iterated_player(
            &for_each_tagged.effects,
            true,
            context,
        );
    }
    if let Some(for_each_controller) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        return rewrite_validate_effects_for_iterated_player(
            &for_each_controller.effects,
            true,
            context,
        );
    }
    if let Some(for_each_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        return rewrite_validate_effects_for_iterated_player(
            &for_each_player.effects,
            true,
            context,
        );
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        rewrite_validate_condition_for_iterated_player(
            &conditional.condition,
            iterated_player_bound,
            context,
        )?;
        rewrite_validate_effects_for_iterated_player(
            &conditional.if_true,
            iterated_player_bound,
            context,
        )?;
        return rewrite_validate_effects_for_iterated_player(
            &conditional.if_false,
            iterated_player_bound,
            context,
        );
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        rewrite_validate_effects_for_iterated_player(
            &if_effect.then,
            iterated_player_bound,
            context,
        )?;
        return rewrite_validate_effects_for_iterated_player(
            &if_effect.else_,
            iterated_player_bound,
            context,
        );
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return rewrite_validate_effect_for_iterated_player(
            &tagged.effect,
            iterated_player_bound,
            context,
        );
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return rewrite_validate_effect_for_iterated_player(
            &with_id.effect,
            iterated_player_bound,
            context,
        );
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        for mode in &choose_mode.modes {
            rewrite_validate_effects_for_iterated_player(
                &mode.effects,
                iterated_player_bound,
                context,
            )?;
        }
        return Ok(());
    }
    if let Some(vote) = effect.downcast_ref::<crate::effects::VoteEffect>() {
        for option in &vote.options {
            rewrite_validate_effects_for_iterated_player(
                &option.effects_per_vote,
                iterated_player_bound,
                context,
            )?;
        }
        return Ok(());
    }
    if let Some(reflexive) = effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>() {
        rewrite_validate_choose_specs_for_iterated_player(&reflexive.choices, false, context)?;
        return rewrite_validate_effects_for_iterated_player(&reflexive.effects, false, context);
    }
    if let Some(schedule_delayed) =
        effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
    {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                format!("{:?}", schedule_delayed.controller),
                context,
            )?;
            if let Some(filter) = &schedule_delayed.target_filter {
                rewrite_validate_unbound_iterated_player(format!("{filter:?}"), context)?;
            }
        }
        return rewrite_validate_effects_for_iterated_player(
            &schedule_delayed.effects,
            false,
            context,
        );
    }
    if let Some(schedule_when_leaves) =
        effect.downcast_ref::<crate::effects::ScheduleEffectsWhenTaggedLeavesEffect>()
    {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                format!("{:?}", schedule_when_leaves.controller),
                context,
            )?;
        }
        return rewrite_validate_effects_for_iterated_player(
            &schedule_when_leaves.effects,
            false,
            context,
        );
    }
    if let Some(haunt) = effect.downcast_ref::<crate::effects::HauntExileEffect>() {
        rewrite_validate_choose_specs_for_iterated_player(&haunt.haunt_choices, false, context)?;
        return rewrite_validate_effects_for_iterated_player(&haunt.haunt_effects, false, context);
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        if !iterated_player_bound && matches!(choose.chooser, PlayerFilter::Target(_)) {
            return Ok(());
        }
    }

    if !iterated_player_bound {
        rewrite_validate_unbound_iterated_player(format!("{effect:?}"), context)?;
    }
    Ok(())
}

pub(crate) fn rewrite_validate_iterated_player_bindings_in_lowered_effects(
    lowered: &LoweredEffects,
    initial_iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    let iterated_player_bound = initial_iterated_player_bound || lowered.exports.iterated_player;
    rewrite_validate_effects_for_iterated_player(&lowered.effects, iterated_player_bound, context)?;
    rewrite_validate_choose_specs_for_iterated_player(
        &lowered.choices,
        iterated_player_bound,
        context,
    )
}
