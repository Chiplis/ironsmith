//! Mechanical materialization of normalized compiler line chunks.
//!
//! Every semantic choice is complete before this module runs.  Dispatch is
//! therefore exhaustive over typed variants and never inspects source text,
//! tokens, grammar, or previously materialized runtime effects.

use crate::ability::{Ability, AbilityKind};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, GiftTimingAst, KeywordAction, LineInfo, ParseAnnotations,
    PlayerAst, StaticAbilityAst, TriggerSpec,
};
use crate::model::facts::LineSemanticFacts;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;

use super::super::effect_pipeline::{
    NormalizedLineChunk, NormalizedParsedAbility, NormalizedPreparedAbility,
    PreparedEffectsForLowering, PreparedTriggeredEffectsForLowering,
};
use super::*;

pub(super) fn rewrite_apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    chunk: NormalizedLineChunk,
    _info: &LineInfo,
    semantic_facts: &LineSemanticFacts,
    _allow_unsupported: bool,
    _annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    match chunk {
        NormalizedLineChunk::Abilities(actions) => {
            materialize_keyword_actions(builder, state, actions)
        }
        NormalizedLineChunk::StaticAbility(ability) => {
            materialize_static_abilities(builder, vec![ability], semantic_facts)
        }
        NormalizedLineChunk::StaticAbilities(abilities) => {
            materialize_static_abilities(builder, abilities, semantic_facts)
        }
        NormalizedLineChunk::Ability(ability) => materialize_ability(builder, ability),
        NormalizedLineChunk::Triggered {
            trigger,
            prepared,
            max_triggers_per_turn,
        } => materialize_triggered(
            builder,
            trigger,
            prepared,
            max_triggers_per_turn,
            semantic_facts,
        ),
        NormalizedLineChunk::Statement {
            effects_ast,
            prepared,
        } => materialize_statement(
            builder,
            state,
            effects_ast,
            prepared,
            &semantic_facts.statement,
        ),
        NormalizedLineChunk::AdditionalCost {
            effects_ast,
            prepared,
        } => materialize_additional_cost(builder, state, effects_ast, prepared),
        NormalizedLineChunk::OptionalCost(cost) => materialize_optional_cost(builder, cost),
        NormalizedLineChunk::GiftKeyword {
            cost,
            prepared,
            followup_text: _,
            timing,
        } => materialize_gift(builder, cost, prepared, timing),
        NormalizedLineChunk::OptionalCostWithCastTrigger {
            cost,
            prepared,
            followup_text: _,
        } => materialize_optional_cost_trigger(builder, cost, prepared),
        NormalizedLineChunk::AdditionalCostChoice { options } => {
            materialize_additional_cost_choice(builder, state, options)
        }
        NormalizedLineChunk::AlternativeCastingMethod(method) => {
            materialize_alternative_cast(builder, method)
        }
    }
}

fn materialize_keyword_actions(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    actions: Vec<KeywordAction>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    for action in actions {
        match action {
            KeywordAction::Backup(amount) => state.pending_backups.push(PendingBackup {
                ability_boundary: builder.abilities.len(),
                amount,
            }),
            KeywordAction::Cipher => state.pending_cipher = true,
            action => builder = builder.apply_keyword_action(action),
        }
    }
    Ok(builder)
}

fn materialize_static_abilities(
    mut builder: CardDefinitionBuilder,
    abilities: Vec<StaticAbilityAst>,
    semantic_facts: &LineSemanticFacts,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let member_count = abilities
        .iter()
        .filter(|ability| {
            !matches!(
                ability,
                StaticAbilityAst::AttachmentRestriction { .. }
                    | StaticAbilityAst::KeywordAction(KeywordAction::Fuse)
            )
        })
        .count();
    if member_count >= 2 {
        let mut marker =
            crate::static_abilities::StaticAbility::source_line_static_group(member_count);
        if let Some(label) = semantic_facts
            .static_ability
            .presentation_label
            .as_ref()
            .and_then(crate::ability::PresentationLabel::display_prefix)
        {
            marker.label = format!(
                "{}{}",
                ironsmith_core::static_ability_model::EXPLICIT_STATIC_PRESENTATION_LABEL_PREFIX,
                label
            );
        }
        builder = builder.with_ability(Ability::static_ability(marker));
    }
    for ability in abilities {
        match ability {
            StaticAbilityAst::AttachmentRestriction { filter, .. } => {
                builder = builder.enchants(filter);
            }
            StaticAbilityAst::KeywordAction(KeywordAction::Fuse) => {
                builder = builder.has_fuse();
            }
            ability => {
                let ability = rewrite_lower_static_ability_ast(ability)?;
                let ability =
                    materialize_self_spell_cost_facts(ability, &semantic_facts.static_ability);
                builder = builder.with_ability(materialize_static_zones(
                    ability,
                    &semantic_facts.static_ability,
                ));
            }
        }
    }
    Ok(builder)
}

fn materialize_self_spell_cost_facts(
    ability: crate::static_abilities::StaticAbility,
    facts: &crate::model::facts::StaticLineSemanticFacts,
) -> crate::static_abilities::StaticAbility {
    let Some(parsed_surface) = facts.this_spell_cost else {
        return ability;
    };

    match &ability.payload {
        ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => {
            let mut amount = reduction.amount.clone();
            if let Some(cap) = parsed_surface.reduction_cap {
                amount = crate::effect::Value::Min(
                    Box::new(amount),
                    Box::new(crate::effect::Value::Fixed(cap)),
                );
            }
            crate::static_abilities::StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount,
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )
        }
        ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => {
            crate::static_abilities::StaticAbility::new(
                crate::static_abilities::ThisSpellCostReductionManaCost::new(
                    reduction.cost.clone(),
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )
        }
        _ => ability,
    }
}

fn materialize_static_zones(
    ability: crate::static_abilities::StaticAbility,
    facts: &crate::model::facts::StaticLineSemanticFacts,
) -> Ability {
    let mut materialized = Ability::static_ability(ability.clone());
    if uses_spell_only_functional_zones(&ability) {
        materialized = materialized.in_zones(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if uses_all_zone_functional_zones(&ability) {
        materialized = materialized.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
            Zone::Ante,
            Zone::OutsideGame,
        ]);
    }
    if uses_referenced_ability_functional_zones(&ability, facts.references_this_ability_cost) {
        materialized = materialized.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let Some(zones) = &facts.explicit_functional_zones {
        materialized = materialized.in_zones(zones.clone());
    }
    materialized
}

fn materialize_ability(
    builder: CardDefinitionBuilder,
    ability: NormalizedParsedAbility,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let ability = rewrite_lower_prepared_ability(ability)?;
    Ok(builder.with_ability(ability))
}

fn materialize_triggered(
    builder: CardDefinitionBuilder,
    trigger: TriggerSpec,
    prepared: PreparedTriggeredEffectsForLowering,
    max_triggers_per_turn: Option<u32>,
    semantic_facts: &LineSemanticFacts,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let functional_zones = infer_triggered_ability_functional_zones_from_facts(
        &trigger,
        &semantic_facts.triggered_ability.functional_zones,
    );
    let intervening_if = trigger_frequency_condition(
        max_triggers_per_turn,
        &semantic_facts.triggered_ability.frequency,
    );
    let parsed = rewrite_parsed_triggered_ability(
        trigger.clone(),
        prepared.prepared.effects.clone(),
        functional_zones,
        None,
        intervening_if,
        semantic_facts.triggered_ability.presentation_label.as_ref(),
        prepared.prepared.imports.clone(),
    );
    let parsed = rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
    })?;
    Ok(builder.with_ability(parsed))
}

fn trigger_frequency_condition(
    maximum: Option<u32>,
    facts: &crate::model::facts::TriggerFrequencyFacts,
) -> Option<crate::ConditionExpr> {
    maximum.map(|limit| {
        if limit == 1 && facts.first_time_each_or_this_turn && facts.becomes_crewed {
            crate::ConditionExpr::SourceFirstCrewedThisTurn
        } else if limit == 1 && facts.first_time_each_or_this_turn {
            crate::ConditionExpr::FirstTimeThisTurn
        } else if facts.do_this_limit_each_turn.is_some() {
            crate::ConditionExpr::DoThisMaxTimesEachTurn(limit)
        } else {
            crate::ConditionExpr::MaxTimesEachTurn(limit)
        }
    })
}

fn materialize_statement(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    effects_ast: Vec<crate::cards::builders::EffectAst>,
    prepared: PreparedEffectsForLowering,
    statement_facts: &crate::model::facts::StatementLineSemanticFacts,
) -> Result<CardDefinitionBuilder, CardTextError> {
    if effects_ast.is_empty() {
        return Err(CardTextError::InvariantViolation(
            "normalized statement contains no effects".to_string(),
        ));
    }
    let mut lowered = rewrite_lower_prepared_statement_effects(&prepared)?;
    fuse_repeatable_mana_payment_prevention_until_end_of_turn(
        &mut lowered.effects,
        statement_facts.repeatable_instant_timing_payment_until_end_of_turn,
    );
    rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &lowered,
        false,
        "spell text effects",
    )?;
    state.latest_spell_exports = lowered.exports;
    if attach_cross_line_self_replacement(
        builder.spell_effect.as_mut(),
        &lowered.effects,
        statement_facts,
    ) {
        return Ok(builder);
    }
    if let Some(existing) = builder.spell_effect.as_mut() {
        if let Some(first_segment) = lowered.effects.segments.first_mut() {
            first_segment.starts_new_source_line = true;
        }
        existing.extend(lowered.effects);
    } else {
        builder.spell_effect = Some(lowered.effects);
    }
    Ok(builder)
}

fn is_exact_permanent_or_player_object_filter(filter: &ObjectFilter) -> bool {
    if filter == &ObjectFilter::permanent() {
        return true;
    }
    if filter.zone != Some(Zone::Battlefield)
        || filter.card_types != ObjectFilter::permanent_card().card_types
    {
        return false;
    }
    let mut remainder = filter.clone();
    remainder.zone = None;
    remainder.card_types.clear();
    remainder == ObjectFilter::default()
}

/// Materialize the grammar-proven duration/timing surface as a persistent
/// repeatable special action. Every executable detail remains constrained by
/// the typed resolution program.
fn fuse_repeatable_mana_payment_prevention_until_end_of_turn(
    program: &mut crate::resolution::ResolutionProgram,
    grammar_proven_surface: bool,
) -> bool {
    if !grammar_proven_surface {
        return false;
    }
    let [initial_segment, payment_segment, followup_segment] = program.segments.as_slice() else {
        return false;
    };
    if [initial_segment, payment_segment, followup_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return false;
    }
    let initial_effect = match initial_segment.default_effects.as_slice() {
        [initial_effect] => initial_effect,
        [target_only, initial_effect] => {
            let Some(target_only) = target_only.downcast_ref::<crate::effects::TargetOnlyEffect>()
            else {
                return false;
            };
            if !matches!(target_only.target.unhinted(), ChooseSpec::AnyTarget)
                || target_only.chooser.is_some()
                || target_only.explicit_declaration
            {
                return false;
            }
            initial_effect
        }
        _ => return false,
    };
    let Some(initial_prevention) =
        initial_effect.downcast_ref::<crate::effects::PreventDamageEffect>()
    else {
        return false;
    };
    if !matches!(initial_prevention.target.unhinted(), ChooseSpec::AnyTarget)
        || initial_prevention.until != crate::effect::Until::EndOfTurn
        || !initial_prevention.follow_up_effects.is_empty()
        || initial_prevention.source_of_your_choice
        || initial_prevention.protect_you_and_permanents_you_control
    {
        return false;
    }

    let [payment_effect] = payment_segment.default_effects.as_slice() else {
        return false;
    };
    let Some(with_id) = payment_effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return false;
    };
    let Some(may) = with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
    else {
        return false;
    };
    let [payment] = may.effects.as_slice() else {
        return false;
    };
    let Some(pay_mana) = payment.downcast_ref::<crate::effects::PayManaEffect>() else {
        return false;
    };
    if may.decider != Some(PlayerFilter::You)
        || !matches!(
            pay_mana.player.unhinted(),
            ChooseSpec::Player(PlayerFilter::You)
        )
        || pay_mana.x_value.is_some()
        || pay_mana.x_maximum.is_some()
    {
        return false;
    }

    let [followup_effect] = followup_segment.default_effects.as_slice() else {
        return false;
    };
    let Some(if_effect) = followup_effect.downcast_ref::<crate::effects::IfEffect>() else {
        return false;
    };
    if if_effect.condition != with_id.id
        || if_effect.predicate != crate::effect::EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return false;
    }
    let [prevention_effect] = if_effect.then.as_slice() else {
        return false;
    };
    let prevention_effect = prevention_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map_or(prevention_effect, |tagged| tagged.effect.as_ref());
    let Some(prevention) = prevention_effect.downcast_ref::<crate::effects::PreventDamageEffect>()
    else {
        return false;
    };
    if prevention.amount.unhinted() != &crate::effect::Value::Fixed(1)
        || prevention.until != crate::effect::Until::EndOfTurn
        || !matches!(
            prevention.target.unhinted(),
            ChooseSpec::ObjectOrPlayer(object, PlayerFilter::Any)
                if is_exact_permanent_or_player_object_filter(object)
        )
        || !prevention.follow_up_effects.is_empty()
        || prevention.source_of_your_choice
        || prevention.protect_you_and_permanents_you_control
    {
        return false;
    }

    let mut same_target_prevention = prevention.clone();
    same_target_prevention.target = ChooseSpec::AnyTarget;
    let grant = crate::effect::Effect::new(
        crate::effects::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect::new(
            PlayerFilter::You,
            pay_mana.cost.clone(),
            vec![crate::effect::Effect::new(same_target_prevention)],
        ),
    );
    let starts_new_source_line = payment_segment.starts_new_source_line;
    program.segments[1] = crate::resolution::ResolutionSegment {
        default_effects: vec![grant],
        self_replacements: Vec::new(),
        starts_new_source_line,
    };
    program.segments.truncate(2);
    *program = crate::resolution::ResolutionProgram::new(program.segments.clone());
    true
}

fn tagged_damage_target(
    effect: &crate::effect::Effect,
) -> Option<(&crate::TagKey, &crate::effects::DealDamageEffect)> {
    let effect = effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map_or(effect, |with_id| with_id.effect.as_ref());
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let damage = tagged
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    Some((&tagged.tag, damage))
}

fn damage_target(effect: &crate::effect::Effect) -> Option<&ChooseSpec> {
    let effect = effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map_or(effect, |with_id| with_id.effect.as_ref());
    let effect = effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map_or(effect, |tagged| tagged.effect.as_ref());
    effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .map(|damage| &damage.target)
}

fn retarget_damage_effect(
    effect: &crate::effect::Effect,
    target: ChooseSpec,
) -> Option<crate::effect::Effect> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        let mut with_id = with_id.clone();
        with_id.effect = Box::new(retarget_damage_effect(with_id.effect.as_ref(), target)?);
        return Some(crate::effect::Effect::new(with_id));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let mut tagged = tagged.clone();
        tagged.effect = Box::new(retarget_damage_effect(tagged.effect.as_ref(), target)?);
        return Some(crate::effect::Effect::new(tagged));
    }
    let mut damage = effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?
        .clone();
    damage.target = target;
    Some(crate::effect::Effect::new(damage))
}

fn retarget_coordinated_damage_replacement(
    default_effect: &crate::effect::Effect,
    replacement_effects: &[crate::effect::Effect],
) -> Option<Vec<crate::effect::Effect>> {
    let default_root = default_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map_or(default_effect, |with_id| with_id.effect.as_ref());
    let default_sequence = default_root.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [replacement_root] = replacement_effects else {
        return None;
    };
    let replacement_inner = replacement_root
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map_or(replacement_root, |with_id| with_id.effect.as_ref());
    let replacement_sequence =
        replacement_inner.downcast_ref::<crate::effects::SequenceEffect>()?;
    if default_sequence.effects.len() != replacement_sequence.effects.len()
        || default_sequence.effects.len() < 2
    {
        return None;
    }

    let effects = default_sequence
        .effects
        .iter()
        .zip(&replacement_sequence.effects)
        .map(|(default, replacement)| {
            retarget_damage_effect(replacement, damage_target(default)?.clone())
        })
        .collect::<Option<Vec<_>>>()?;
    let mut sequence = replacement_sequence.clone();
    sequence.effects = effects;
    let sequence = crate::effect::Effect::new(sequence);
    if let Some(with_id) = replacement_root.downcast_ref::<crate::effects::WithIdEffect>() {
        let mut with_id = with_id.clone();
        with_id.effect = Box::new(sequence);
        Some(vec![crate::effect::Effect::new(with_id)])
    } else {
        Some(vec![sequence])
    }
}

fn retarget_amount_replacement(
    default_effect: &crate::effect::Effect,
    replacement_effects: &[crate::effect::Effect],
) -> Option<Vec<crate::effect::Effect>> {
    if let Some(replacement) =
        retarget_coordinated_damage_replacement(default_effect, replacement_effects)
    {
        return Some(replacement);
    }
    let (default_tag, default_damage) = tagged_damage_target(default_effect)?;
    let (declared_target, replacement) = match replacement_effects {
        [target_only, replacement] => (
            Some(
                &target_only
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()?
                    .target,
            ),
            replacement,
        ),
        [replacement] => (None, replacement),
        _ => return None,
    };

    if declared_target.is_none()
        && let Some(replacement) =
            retarget_damage_effect(replacement, default_damage.target.clone())
    {
        return Some(vec![replacement]);
    }

    if let Some(with_source) = replacement.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        && matches!(with_source.source.base(), ChooseSpec::Tagged(tag) if tag == default_tag)
        && let Some(replacement_damage) = with_source
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && declared_target.is_none_or(|target| target == &replacement_damage.target)
    {
        let mut replacement_damage = replacement_damage.clone();
        replacement_damage.target = default_damage.target.clone();
        return Some(vec![crate::effect::Effect::new(replacement_damage)]);
    }

    let replacement_damage = replacement.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if declared_target.is_some_and(|target| target != &replacement_damage.target) {
        return None;
    }
    let mut replacement_damage = replacement_damage.clone();
    replacement_damage.target = default_damage.target.clone();
    Some(vec![crate::effect::Effect::new(replacement_damage)])
}

fn search_limit_shape(
    effect: &crate::effect::Effect,
) -> Option<(ObjectFilter, Vec<Zone>, crate::effect::ChoiceCount)> {
    let effect = effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map_or(effect, |with_id| with_id.effect.as_ref());
    if let Some(search) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && search.is_search
        && search.count_value.is_none()
    {
        let mut zones = search.zone.into_iter().collect::<Vec<_>>();
        zones.extend(search.additional_zones.iter().copied());
        if zones.is_empty()
            && let Some(zone) = search.filter.zone
        {
            zones.push(zone);
        }
        return Some((search.filter.clone(), zones, search.count));
    }
    let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let mut found = None;
    for child in &sequence.effects {
        if let Some(shape) = search_limit_shape(child) {
            if found.is_some() {
                return None;
            }
            found = Some(shape);
        }
    }
    found
}

fn sole_search_limit_shape(
    effects: &[crate::effect::Effect],
) -> Option<(ObjectFilter, Vec<Zone>, crate::effect::ChoiceCount)> {
    let mut found = None;
    for effect in effects {
        if let Some(shape) = search_limit_shape(effect) {
            if found.is_some() {
                return None;
            }
            found = Some(shape);
        }
    }
    found
}

fn same_library_search_filter(left: &ObjectFilter, right: &ObjectFilter) -> bool {
    if left == right {
        return true;
    }
    let mut left = left.clone();
    let mut right = right.clone();
    if (left.owner.as_ref() == Some(&PlayerFilter::You) && right.owner.is_none())
        || (left.owner.is_none() && right.owner.as_ref() == Some(&PlayerFilter::You))
    {
        left.owner = None;
        right.owner = None;
    }
    left == right
}

fn replace_search_limit(
    effect: &crate::effect::Effect,
    filter: &ObjectFilter,
    zones: &[Zone],
    count: crate::effect::ChoiceCount,
) -> Option<crate::effect::Effect> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        let mut with_id = with_id.clone();
        with_id.effect = Box::new(replace_search_limit(
            with_id.effect.as_ref(),
            filter,
            zones,
            count,
        )?);
        return Some(crate::effect::Effect::new(with_id));
    }
    if let Some(search) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        let (search_filter, search_zones, _) = search_limit_shape(effect)?;
        if !same_library_search_filter(&search_filter, filter) || search_zones != zones {
            return None;
        }
        let mut search = search.clone();
        search.count = count;
        return Some(crate::effect::Effect::new(search));
    }
    let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let mut sequence = sequence.clone();
    let mut replacement = None;
    for (index, child) in sequence.effects.iter().enumerate() {
        if let Some(replaced) = replace_search_limit(child, filter, zones, count) {
            if replacement.is_some() {
                return None;
            }
            replacement = Some((index, replaced));
        }
    }
    let (index, replaced) = replacement?;
    sequence.effects[index] = replaced;
    Some(crate::effect::Effect::new(sequence))
}

fn attach_cross_line_search_limit_replacement(
    existing: &mut crate::resolution::ResolutionProgram,
    conditional: &crate::effects::ConditionalEffect,
    presentation_label: Option<crate::ability::PresentationLabel>,
) -> bool {
    let Some((filter, zones, replacement_count)) = sole_search_limit_shape(&conditional.if_true)
    else {
        return false;
    };
    for segment in existing.segments.iter_mut().rev() {
        let mut matched = None;
        for (index, default) in segment.default_effects.iter().enumerate() {
            let Some((default_filter, default_zones, default_count)) = search_limit_shape(default)
            else {
                continue;
            };
            if !same_library_search_filter(&default_filter, &filter)
                || default_zones != zones
                || default_count == replacement_count
            {
                continue;
            }
            let Some(replacement) =
                replace_search_limit(default, &filter, &zones, replacement_count)
            else {
                continue;
            };
            if matched.is_some() {
                return false;
            }
            matched = Some((index, replacement));
        }
        let Some((index, replacement)) = matched else {
            continue;
        };
        segment.self_replacements.push(
            crate::resolution::SelfReplacementBranch::new(
                conditional.condition.clone(),
                vec![replacement],
            )
            .with_presentation_label(presentation_label)
            .with_starts_new_source_line(true),
        );
        debug_assert!(index < segment.default_effects.len());
        return true;
    }
    false
}

/// A separately authored `... instead` line is already classified by the
/// front end as a spell self-replacement. Materialization only has to attach
/// its typed conditional branch to the preceding resolution segment. For an
/// amount-only damage replacement, reuse the original source and target rather
/// than executing the parser's temporary pronoun source wrapper.
fn attach_cross_line_self_replacement(
    existing: Option<&mut crate::resolution::ResolutionProgram>,
    followup: &crate::resolution::ResolutionProgram,
    facts: &crate::model::facts::StatementLineSemanticFacts,
) -> bool {
    if facts.instead_followup.semantics != crate::cards::builders::InsteadSemantics::SelfReplacement
    {
        return false;
    }
    let Some(existing) = existing else {
        return false;
    };
    let [followup_segment] = followup.segments.as_slice() else {
        return false;
    };
    if !followup_segment.self_replacements.is_empty() {
        return false;
    }
    let conditional = match followup_segment.default_effects.as_slice() {
        [conditional] => conditional.downcast_ref::<crate::effects::ConditionalEffect>(),
        [target_only, conditional]
            if target_only
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            conditional.downcast_ref::<crate::effects::ConditionalEffect>()
        }
        _ => None,
    };
    let Some(conditional) = conditional else {
        return false;
    };
    if !conditional.if_false.is_empty() {
        return false;
    }
    if attach_cross_line_search_limit_replacement(
        existing,
        conditional,
        facts.presentation_label.clone(),
    ) {
        return true;
    }
    let Some(existing_segment) = existing.last_segment_mut() else {
        return false;
    };
    let Some(default_effect) = existing_segment.default_effects.last() else {
        return false;
    };
    let replacement_effects = retarget_amount_replacement(default_effect, &conditional.if_true)
        .unwrap_or_else(|| conditional.if_true.clone());
    existing_segment.self_replacements.push(
        crate::resolution::SelfReplacementBranch::new(
            conditional.condition.clone(),
            replacement_effects,
        )
        .with_presentation_label(facts.presentation_label.clone())
        .with_starts_new_source_line(true),
    );
    true
}

fn materialize_additional_cost(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    effects_ast: Vec<crate::cards::builders::EffectAst>,
    prepared: PreparedEffectsForLowering,
) -> Result<CardDefinitionBuilder, CardTextError> {
    if effects_ast.is_empty() {
        return Err(CardTextError::InvariantViolation(
            "normalized additional cost contains no effects".to_string(),
        ));
    }
    let lowered = rewrite_lower_prepared_statement_effects(&prepared)?;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.extend(runtime_effects_to_costs(lowered.effects.to_vec())?);
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    state.latest_additional_cost_exports = lowered.exports;
    Ok(builder)
}

fn materialize_optional_cost(
    mut builder: CardDefinitionBuilder,
    cost: crate::cost::OptionalCost,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let kind = cost.kind.clone();
    let reference = cost.cost_ref();
    builder = builder.optional_cost(cost);
    match kind {
        crate::cost::OptionalCostKind::Squad => {
            builder = builder.with_ability(Ability::triggered(
                crate::triggers::Trigger::this_enters_battlefield(),
                vec![crate::effect::Effect::new(
                    crate::effects::CreateTokenCopyEffect::new(
                        ChooseSpec::Source,
                        crate::effect::Value::TimesPaidLabel(reference),
                        PlayerFilter::You,
                    ),
                )],
            ));
        }
        crate::cost::OptionalCostKind::Offspring => {
            builder = builder.with_ability(Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: crate::triggers::Trigger::this_enters_battlefield(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        crate::effect::Effect::new(
                            crate::effects::CreateTokenCopyEffect::new(
                                ChooseSpec::Source,
                                crate::effect::Value::WasPaidLabel(reference.clone()),
                                PlayerFilter::You,
                            )
                            .set_base_power_toughness(1, 1),
                        ),
                    ]),
                    choices: Vec::new(),
                    intervening_if: Some(crate::effect::Condition::ThisSpellPaidLabel(reference)),
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            });
        }
        _ => {}
    }
    Ok(builder)
}

fn materialize_gift(
    mut builder: CardDefinitionBuilder,
    cost: crate::cost::OptionalCost,
    prepared: PreparedEffectsForLowering,
    timing: GiftTimingAst,
) -> Result<CardDefinitionBuilder, CardTextError> {
    builder = builder.optional_cost(cost);
    match timing {
        GiftTimingAst::SpellResolution => {
            let lowered = rewrite_lower_prepared_statement_effects(&prepared)?;
            let mut effects = lowered.effects.to_vec();
            effects.push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            let gift = crate::effect::Effect::conditional(
                crate::ConditionExpr::ThisSpellPaidLabel("Gift".into()),
                effects,
                Vec::new(),
            );
            if let Some(existing) = builder.spell_effect.as_mut() {
                existing.push(gift);
            } else {
                builder.spell_effect =
                    Some(crate::resolution::ResolutionProgram::from_effects(vec![
                        gift,
                    ]));
            }
        }
        GiftTimingAst::PermanentEtb => {
            let trigger = TriggerSpec::ThisEntersBattlefield {
                origin_condition: None,
            };
            let parsed = rewrite_parsed_triggered_ability(
                trigger.clone(),
                prepared.effects.clone(),
                vec![Zone::Battlefield],
                None,
                Some(crate::ConditionExpr::ThisSpellPaidLabel("Gift".into())),
                None,
                prepared.imports.clone(),
            );
            let prepared = PreparedTriggeredEffectsForLowering {
                prepared,
                intervening_if: None,
            };
            let mut parsed = rewrite_lower_prepared_ability(NormalizedParsedAbility {
                parsed,
                prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
            })?;
            if let AbilityKind::Triggered(triggered) = &mut parsed.kind {
                triggered
                    .effects
                    .push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            }
            builder = builder.with_ability(parsed);
        }
    }
    Ok(builder)
}

fn materialize_optional_cost_trigger(
    mut builder: CardDefinitionBuilder,
    cost: crate::cost::OptionalCost,
    prepared: PreparedEffectsForLowering,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let reference = cost.cost_ref();
    builder = builder.optional_cost(cost);
    let trigger = TriggerSpec::YouCastThisSpell;
    let parsed = rewrite_parsed_triggered_ability(
        trigger.clone(),
        prepared.effects.clone(),
        vec![Zone::Stack],
        None,
        Some(crate::ConditionExpr::ThisSpellPaidLabel(reference)),
        None,
        prepared.imports.clone(),
    );
    let parsed = rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered {
            trigger,
            prepared: PreparedTriggeredEffectsForLowering {
                prepared,
                intervening_if: None,
            },
        }),
    })?;
    Ok(builder.with_ability(parsed))
}

fn materialize_additional_cost_choice(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    options: Vec<NormalizedAdditionalCostChoiceOptionAst>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    if options.len() < 2 || options.iter().any(|option| option.effects_ast.is_empty()) {
        return Err(CardTextError::InvariantViolation(
            "normalized additional-cost choice requires two nonempty modes".to_string(),
        ));
    }
    let (modes, exports) =
        rewrite_lower_prepared_additional_cost_choice_modes_with_exports(&options)?;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.push(
        crate::costs::payment_effect_to_cost(crate::effect::Effect::choose_one(modes))
            .map_err(CardTextError::InvariantViolation)?,
    );
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    state.latest_additional_cost_exports = exports;
    Ok(builder)
}

fn materialize_alternative_cast(
    builder: CardDefinitionBuilder,
    mut method: crate::alternative_cast::AlternativeCastingMethod,
) -> Result<CardDefinitionBuilder, CardTextError> {
    if let crate::alternative_cast::AlternativeCastingMethod::FlashWithAdditionalCost {
        additional_cost,
        ..
    } = &method
    {
        let printed = builder
            .card_builder
            .mana_cost_ref()
            .cloned()
            .unwrap_or_default();
        let mut pips = printed.pips().to_vec();
        pips.extend(additional_cost.pips().iter().cloned());
        method = crate::alternative_cast::AlternativeCastingMethod::flash_with_additional_cost(
            additional_cost.clone(),
            crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(pips)),
        );
    }
    if let crate::alternative_cast::AlternativeCastingMethod::Retrace { total_cost } = &method {
        let printed = builder
            .card_builder
            .mana_cost_ref()
            .cloned()
            .unwrap_or_default();
        let mut costs = vec![crate::costs::Cost::mana(printed)];
        costs.extend(total_cost.costs().iter().cloned());
        method = crate::alternative_cast::AlternativeCastingMethod::Retrace {
            total_cost: crate::cost::TotalCost::from_costs(costs),
        };
    }
    Ok(builder.alternative_cast(method))
}

#[cfg(test)]
#[test]
fn public_two_line_damage_replacement_reuses_both_announced_targets() {
    let definition = CardDefinitionBuilder::new(crate::CardId::from_raw(1), "Damage Pair Variant")
        .parse_text(
            "This spell deals 1 damage to target player or planeswalker and 1 damage to target creature that player or that planeswalker's controller controls.\nLandfall — If you had a land enter the battlefield under your control this turn, this spell deals 3 damage to that player or planeswalker and 3 damage to that creature instead.",
        )
        .expect("full public document route should lower the damage replacement");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("damage pair should produce a spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one replacement segment: {program:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one typed self-replacement: {segment:#?}");
    };

    fn damage_targets(effect: &crate::effect::Effect) -> Vec<ChooseSpec> {
        let leaf = effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .map_or(effect, |with_id| with_id.effect.as_ref());
        let sequence = leaf
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("coordinated damage pair");
        sequence
            .effects
            .iter()
            .map(|effect| {
                let leaf = effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .map_or(effect, |tagged| tagged.effect.as_ref());
                leaf.downcast_ref::<crate::effects::DealDamageEffect>()
                    .expect("damage leaf")
                    .target
                    .clone()
            })
            .collect()
    }

    let [default] = segment.default_effects.as_slice() else {
        panic!("expected one coordinated default effect: {segment:#?}");
    };
    let [replacement] = branch.replacement_effects.as_slice() else {
        panic!("expected one coordinated replacement effect: {branch:#?}");
    };
    assert_eq!(damage_targets(default), damage_targets(replacement));
    assert!(matches!(
        branch.presentation_label,
        Some(crate::cards::builders::PresentationLabel::AbilityWord(ref label)) if label == "Landfall"
    ));
}
