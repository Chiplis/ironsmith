//! Typed semantic assembly for triggered line chunks.
//!
//! These transformations consume grammar facts and semantic AST values. They
//! intentionally run before preparation/lowering and never inspect Oracle
//! text or token shapes.

use super::*;
use crate::runtime_backend::ast::TriggerIntroSurfaceAst;
use crate::runtime_backend::condition_antecedent::{
    ConditionAntecedentBinding, bind_condition_antecedent_in_effects,
    bind_condition_counter_antecedent_in_effects, predicate_object_filter_antecedent,
    predicate_source_counter_antecedent, retarget_it_animations_to_source,
};
use crate::runtime_backend::shared_types::TriggeredLineSemanticFacts;

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

fn apply_trigger_intro_surface(
    trigger: TriggerSpec,
    intro: Option<TriggerIntroSurfaceAst>,
) -> TriggerSpec {
    let Some(intro) = intro else {
        return trigger;
    };
    match trigger {
        TriggerSpec::ThisAttacks
        | TriggerSpec::ThisAttacksPlayerWhoControlsAtLeast { .. }
        | TriggerSpec::ThisAttacksWithNOthers { .. }
        | TriggerSpec::ThisAttacksWithExactlyNOthers(_)
        | TriggerSpec::ThisAttacksAndIsntBlocked
        | TriggerSpec::ThisAttacksWhileSaddled
        | TriggerSpec::Attacks(_)
        | TriggerSpec::AttacksAndIsntBlocked(_)
        | TriggerSpec::AttacksWhileSaddled(_)
        | TriggerSpec::AttacksOneOrMore(_)
        | TriggerSpec::PlayersAttackedOneOrMore(_)
        | TriggerSpec::AttacksOneOrMoreWithMinTotal { .. }
        | TriggerSpec::AttacksOneOrMoreWithExactTotal { .. }
        | TriggerSpec::AttacksAlone(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControl(_)
        | TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(_)
            if intro == TriggerIntroSurfaceAst::When =>
        {
            trigger
        }
        TriggerSpec::KeywordAction { .. }
        | TriggerSpec::KeywordActionTaggedObject { .. }
        | TriggerSpec::KeywordActionFromSource { .. }
        | TriggerSpec::WithIntro { .. } => trigger,
        trigger => TriggerSpec::WithIntro {
            intro,
            trigger: Box::new(trigger),
        },
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
                    timing,
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
                            timing,
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

fn retarget_spell_cast_mana_spent_predicate(
    trigger: &TriggerSpec,
    predicate: PredicateAst,
) -> PredicateAst {
    fn trigger_is_spell_cast(trigger: &TriggerSpec) -> bool {
        match trigger {
            TriggerSpec::WithIntro { trigger, .. } => trigger_is_spell_cast(trigger),
            TriggerSpec::SpellCast { .. } => true,
            _ => false,
        }
    }

    fn retarget(predicate: PredicateAst) -> PredicateAst {
        match predicate {
            PredicateAst::TargetSpellNoManaSpentToCast => PredicateAst::Not(Box::new(
                PredicateAst::TriggeringSpellManaSpentToCastAtLeast {
                    amount: 1,
                    symbol: None,
                },
            )),
            PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
                PredicateAst::TriggeringSpellManaSpentToCastAtLeast { amount, symbol }
            }
            PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(amount) => {
                PredicateAst::TriggeringSpellColoredManaSpentToCastAtLeast(amount)
            }
            PredicateAst::Not(inner) => PredicateAst::Not(Box::new(retarget(*inner))),
            PredicateAst::And(left, right) => {
                PredicateAst::And(Box::new(retarget(*left)), Box::new(retarget(*right)))
            }
            PredicateAst::Or(left, right) => {
                PredicateAst::Or(Box::new(retarget(*left)), Box::new(retarget(*right)))
            }
            other => other,
        }
    }

    if trigger_is_spell_cast(trigger) {
        retarget(predicate)
    } else {
        predicate
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

pub(crate) fn infer_triggered_ability_functional_zones_from_facts(
    trigger: &TriggerSpec,
    facts: &crate::runtime_backend::shared_types::TriggerFunctionalZoneFacts,
) -> Vec<Zone> {
    let mut zones = match trigger {
        TriggerSpec::WithIntro { trigger, .. } => {
            return infer_triggered_ability_functional_zones_from_facts(trigger, facts);
        }
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            ..
        } => vec![Zone::Graveyard],
        _ => vec![Zone::Battlefield],
    };
    if let Some(explicit_zone) = &facts.explicit_zone {
        zones = vec![explicit_zone.clone()];
    }
    if facts.returns_self_from_graveyard && !trigger_references_attached_object(trigger) {
        zones = vec![Zone::Graveyard];
    } else if facts.discards_this_card {
        zones = vec![Zone::Hand];
    }
    zones
}

fn trigger_references_attached_object(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_references_attached_object(trigger),
        TriggerSpec::PutIntoGraveyard(filter) | TriggerSpec::PutIntoGraveyardOneOrMore(filter) => {
            filter_references_tag(filter, "enchanted") || filter_references_tag(filter, "equipped")
        }
        TriggerSpec::PutIntoGraveyardFromZone { filter, .. } => {
            filter_references_tag(filter, "enchanted") || filter_references_tag(filter, "equipped")
        }
        TriggerSpec::Either(left, right) => {
            trigger_references_attached_object(left) || trigger_references_attached_object(right)
        }
        _ => false,
    }
}

fn filter_references_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tag)
        || filter.could_be_targeted_by.as_ref().is_some_and(|constraint| {
            matches!(&constraint.stack_object, crate::filter::ObjectRef::Tagged(object_tag) if object_tag.as_str() == tag)
        })
        || matches!(&filter.blocked_by, Some(crate::filter::ObjectRef::Tagged(object_tag)) if object_tag.as_str() == tag)
        || filter.targets_object.as_deref().is_some_and(|targets| filter_references_tag(targets, tag))
        || filter.targets_only_object.as_deref().is_some_and(|targets| filter_references_tag(targets, tag))
        || filter.attached_to_object.as_deref().is_some_and(|attached_to| filter_references_tag(attached_to, tag))
        || filter.any_of.iter().any(|branch| filter_references_tag(branch, tag))
}

fn trigger_frequency_condition_from_facts(
    facts: &TriggeredLineSemanticFacts,
    max_triggers_per_turn: Option<u32>,
) -> Option<crate::ConditionExpr> {
    max_triggers_per_turn.map(|limit| {
        if limit == 1
            && facts.frequency.first_time_each_or_this_turn
            && facts.frequency.becomes_crewed
        {
            crate::ConditionExpr::SourceFirstCrewedThisTurn
        } else if limit == 1 && facts.frequency.first_time_each_or_this_turn {
            crate::ConditionExpr::FirstTimeThisTurn
        } else if facts.frequency.do_this_limit_each_turn.is_some() {
            crate::ConditionExpr::DoThisMaxTimesEachTurn(limit)
        } else {
            crate::ConditionExpr::MaxTimesEachTurn(limit)
        }
    })
}

fn rewrite_do_this_trigger_frequency_surface(
    facts: &TriggeredLineSemanticFacts,
    triggered: &mut crate::ability::TriggeredAbility,
) {
    let Some(surface_count) = facts.frequency.do_this_limit_each_turn else {
        return;
    };
    let Some(condition) = triggered.intervening_if.take() else {
        return;
    };
    triggered.intervening_if = Some(match condition {
        crate::ConditionExpr::MaxTimesEachTurn(count) if count == surface_count => {
            crate::ConditionExpr::DoThisMaxTimesEachTurn(count)
        }
        other => other,
    });
}

pub(crate) fn apply_chosen_option_to_triggered_chunk(
    chunk: LineAst,
    full_text: &str,
    facts: &TriggeredLineSemanticFacts,
    max_triggers_per_turn: Option<u32>,
    chosen_option: Option<&ChosenOptionContext>,
    presentation: Option<&PresentationLabel>,
) -> Result<LineAst, CardTextError> {
    let during_your_turn_condition = facts
        .becomes_tapped_during_your_turn
        .then_some(crate::ConditionExpr::YourTurn);
    let max_condition = trigger_frequency_condition_from_facts(facts, max_triggers_per_turn);
    let combined_condition = match (chosen_option, max_condition.clone()) {
        (Some(label), Some(max)) => Some(crate::ConditionExpr::And(
            Box::new(condition_for_chosen_option(label)),
            Box::new(max),
        )),
        (Some(label), None) => Some(condition_for_chosen_option(label)),
        (None, Some(max)) => Some(max),
        (None, None) => None,
    };

    match chunk {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: chunk_max_triggers_per_turn,
        } => {
            let trigger = apply_trigger_intro_surface(trigger, facts.intro_surface);
            let merged_max_condition = chunk_max_triggers_per_turn
                .or(max_triggers_per_turn)
                .and_then(|count| trigger_frequency_condition_from_facts(facts, Some(count)));
            let merged_condition = match (chosen_option, merged_max_condition) {
                (Some(label), Some(max)) => Some(crate::ConditionExpr::And(
                    Box::new(condition_for_chosen_option(label)),
                    Box::new(max),
                )),
                (Some(label), None) => Some(condition_for_chosen_option(label)),
                (None, Some(max)) => Some(max),
                (None, None) => None,
            };
            let merged_condition = match (during_your_turn_condition.clone(), merged_condition) {
                (Some(condition), Some(existing)) => Some(crate::ConditionExpr::And(
                    Box::new(condition),
                    Box::new(existing),
                )),
                (Some(condition), None) => Some(condition),
                (None, existing) => existing,
            };
            Ok(LineAst::Ability(rewrite_parsed_triggered_ability(
                trigger.clone(),
                effects,
                infer_triggered_ability_functional_zones_from_facts(
                    &trigger,
                    &facts.functional_zones,
                ),
                Some(full_text.to_string()),
                merged_condition,
                presentation,
                ReferenceImports::default(),
            )))
        }
        LineAst::Ability(mut parsed) => {
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut()
                && let Some(intro) = facts.intro_surface
            {
                triggered.trigger = triggered.trigger.clone().with_intro_surface(match intro {
                    TriggerIntroSurfaceAst::When => crate::triggers::TriggerIntroSurface::When,
                    TriggerIntroSurfaceAst::Whenever => {
                        crate::triggers::TriggerIntroSurface::Whenever
                    }
                    TriggerIntroSurfaceAst::At => crate::triggers::TriggerIntroSurface::At,
                });
            }
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                rewrite_do_this_trigger_frequency_surface(facts, triggered);
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
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut()
                && let Some(condition) = during_your_turn_condition
            {
                triggered.intervening_if = Some(match triggered.intervening_if.take() {
                    Some(existing) => {
                        crate::ConditionExpr::And(Box::new(condition), Box::new(existing))
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
                triggered.presentation_label = presentation.cloned();
            }
            Ok(LineAst::Ability(parsed))
        }
        other => Ok(other),
    }
}

pub(crate) fn apply_explicit_intervening_if_to_triggered_chunk(
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
            let predicate = retarget_spell_cast_mana_spent_predicate(&trigger, predicate);
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
                bind_condition_antecedent_in_effects(
                    &mut effects,
                    &antecedent,
                    ConditionAntecedentBinding::TaggedItOnly,
                );
            }
            if let Some(counter_type) = predicate_source_counter_antecedent(&predicate) {
                bind_condition_counter_antecedent_in_effects(&mut effects, counter_type);
            }
            if predicate.establishes_source_object_antecedent() {
                retarget_it_animations_to_source(&mut effects);
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
                    bind_condition_antecedent_in_effects(
                        &mut effects_ast,
                        &antecedent,
                        ConditionAntecedentBinding::TaggedItOnly,
                    );
                }
                if let Some(counter_type) = predicate_source_counter_antecedent(&predicate) {
                    bind_condition_counter_antecedent_in_effects(&mut effects_ast, counter_type);
                }
                if predicate.establishes_source_object_antecedent() {
                    retarget_it_animations_to_source(&mut effects_ast);
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
            let mut reference_imports = parsed.reference_imports.clone();
            let default_last_object_tag = reference_imports.last_object_tag.clone().or_else(|| {
                parsed
                    .trigger_spec
                    .as_ref()
                    .and_then(super::super::lowering_support::default_trigger_last_object_tag)
                    .map(TagKey::from)
            });
            if reference_imports.last_object_tag.is_none() {
                reference_imports.last_object_tag = default_last_object_tag.clone();
            }
            let compiled_condition = compile_condition_from_predicate_ast_with_env(
                &predicate,
                &ReferenceEnv::from_imports(&reference_imports, false, false, false, None),
                default_last_object_tag.as_ref(),
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
