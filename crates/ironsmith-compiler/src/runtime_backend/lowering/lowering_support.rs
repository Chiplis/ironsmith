use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, EffectAst, IT_TAG, KeywordAction, ParsedAbility,
    PredicateAst, StaticAbilityAst, SubjectVerbActionAst, SubjectVerbEffectAst, TagKey, TargetAst,
    TriggerSpec,
};
use crate::effect::{Condition, Effect, EffectMode, EventValueSpec, Value};
use crate::filter::{ObjectFilter, ObjectRef};
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter, SourceReferenceSurface, TaggedOpbjectRelation};
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;

use super::compile_support::{
    choose_spec_mentions_iterated_player, compile_delayed_trigger_spec, compile_trigger_effects,
    compile_trigger_spec, condition_mentions_iterated_player, effect_mentions_iterated_player,
    effect_references_it_tag, effect_references_its_controller, effect_references_tag,
    effects_contain_pending_effect_metric, effects_reference_it_tag,
    effects_reference_its_controller, effects_reference_tag, ensure_concrete_trigger_spec,
    filter_references_tag, inferred_trigger_player_filter,
    is_sentence_helper_exiled_collection_tag, materialize_prepared_effects_with_trigger_context,
    materialize_prepared_statement_effects, materialize_prepared_triggered_effects,
    object_filter_mentions_iterated_player, trigger_binds_player_reference_context,
    trigger_supports_event_value, value_mentions_iterated_player,
};
use super::condition_antecedent::{
    ConditionAntecedentBinding, bind_condition_antecedent_in_effects,
    bind_condition_collection_antecedent_in_effects, bind_condition_counter_antecedent_in_effects,
    bind_random_count_condition_antecedent_in_effects,
    bind_trigger_antecedent_after_top_library_observation, predicate_object_filter_antecedent,
    predicate_source_counter_antecedent, retarget_it_animations_to_source,
    retarget_source_damage_attack_followups_to_source,
};
use super::effect_ast_normalization::{
    correlate_conditional_quantified_choice_followups, normalize_effects_ast,
};
use super::effect_ast_traversal::{for_each_nested_effects, for_each_nested_effects_mut};
use super::effect_pipeline::{
    EffectPreludeTag, NormalizedAdditionalCostChoiceOptionAst, NormalizedParsedAbility,
    NormalizedPreparedAbility, PreparedEffectsForLowering, PreparedPredicateForLowering,
    PreparedTriggeredEffectsForLowering, SourceSentenceSegment,
};
use super::reference_model::{LoweredEffects, ReferenceEnv, ReferenceExports, ReferenceImports};
use super::reference_resolution::{EffectReferenceResolutionConfig, annotate_effect_sequence};
use super::static_ability_helpers::executable_object_abilities_for_keyword_action;

pub(crate) fn replace_pending_removed_counter_metrics_with_x(effects: &mut [EffectAst]) {
    fn replace_value(value: &mut Value) {
        let hints = value.surface_hints().to_vec();
        if matches!(
            value.unhinted(),
            Value::PendingPriorEffectMetric(query)
                if query.action == Some(ironsmith_core::PriorEffectAction::Removed)
        ) {
            *value = Value::X.with_surface_hints(hints);
            return;
        }
        match value {
            Value::Add(left, right) | Value::Min(left, right) => {
                replace_value(left);
                replace_value(right);
            }
            Value::Scaled(inner, _)
            | Value::DividedRoundedDown(inner, _)
            | Value::HalfRoundedDown(inner)
            | Value::SurfaceHinted { value: inner, .. } => replace_value(inner),
            _ => {}
        }
    }

    fn replace_effect(effect: &mut EffectAst) {
        if let EffectAst::SubjectVerb(subject_verb) = effect {
            match &mut subject_verb.action {
                SubjectVerbActionAst::AddManaScaled { amount, .. }
                | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { amount }
                | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { amount }
                | SubjectVerbActionAst::PutCounters { count: amount, .. }
                | SubjectVerbActionAst::PutCounterChoice { count: amount, .. }
                | SubjectVerbActionAst::PutCountersAll { count: amount, .. } => {
                    replace_value(amount)
                }
                _ => {}
            }
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                replace_effect(nested_effect);
            }
        });
    }

    for effect in effects {
        replace_effect(effect);
    }
}

fn predicate_counts_creature_deaths(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::CreatureDiedThisTurn | PredicateAst::CreatureDiedThisTurnOrMore(_) => true,
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_counts_creature_deaths(left) || predicate_counts_creature_deaths(right)
        }
        PredicateAst::Not(inner) => predicate_counts_creature_deaths(inner),
        PredicateAst::ValueComparison { left, right, .. } => {
            fn value_counts_creature_deaths(value: &Value) -> bool {
                match value {
                    Value::CreaturesDiedThisTurn
                    | Value::CreaturesDiedThisTurnControlledBy(_)
                    | Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::Died { .. }) => {
                        true
                    }
                    Value::SurfaceHinted { value, .. }
                    | Value::Scaled(value, _)
                    | Value::DividedRoundedDown(value, _)
                    | Value::HalfRoundedDown(value) => value_counts_creature_deaths(value),
                    Value::Add(left, right) | Value::Min(left, right) => {
                        value_counts_creature_deaths(left) || value_counts_creature_deaths(right)
                    }
                    _ => false,
                }
            }
            value_counts_creature_deaths(left) || value_counts_creature_deaths(right)
        }
        _ => false,
    }
}

fn trigger_allows_event_derived_life_value(trigger: &TriggerSpec) -> bool {
    trigger_supports_event_value(trigger, &EventValueSpec::Amount)
        || match trigger {
            TriggerSpec::WithIntro { trigger, .. } => {
                trigger_allows_event_derived_life_value(trigger)
            }
            TriggerSpec::StateBased { condition, .. } => {
                predicate_counts_creature_deaths(condition)
            }
            _ => false,
        }
}

fn target_can_establish_local_object_reference(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(_, _)
        | TargetAst::Object(_, _, _)
        | TargetAst::ObjectOrPlayer(_, _, _) => true,
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_can_establish_local_object_reference(inner)
        }
        TargetAst::Source(_)
        | TargetAst::AnyTarget(_)
        | TargetAst::AnyOtherTarget(_)
        | TargetAst::Player(_, _)
        | TargetAst::PlayerOrPlaneswalker(_, _)
        | TargetAst::AttackedPlayerOrPlaneswalker(_)
        | TargetAst::Spell(_) => false,
    }
}

fn replace_creature_death_event_amounts(effects: &mut [EffectAst]) {
    fn replace_value(value: &mut Value) {
        let hints = value.surface_hints().to_vec();
        if matches!(value.unhinted(), Value::EventValue(EventValueSpec::Amount)) {
            *value = Value::CreaturesDiedThisTurn.with_surface_hints(hints);
        }
    }

    fn replace_effect(effect: &mut EffectAst) {
        if let EffectAst::SubjectVerb(subject_verb) = effect {
            match &mut subject_verb.action {
                SubjectVerbActionAst::Draw { count }
                | SubjectVerbActionAst::Mill { count }
                | SubjectVerbActionAst::ExileTopOfLibrary { count, .. }
                | SubjectVerbActionAst::Scry { count }
                | SubjectVerbActionAst::Surveil { count }
                | SubjectVerbActionAst::Proliferate { count }
                | SubjectVerbActionAst::Investigate { count }
                | SubjectVerbActionAst::Discover { count }
                | SubjectVerbActionAst::Fateseal { count }
                | SubjectVerbActionAst::Populate { count, .. }
                | SubjectVerbActionAst::Connive { count, .. }
                | SubjectVerbActionAst::CreateTokenCopy { count, .. }
                | SubjectVerbActionAst::CreateTokenCopyFromSource { count, .. }
                | SubjectVerbActionAst::CreateTokenWithMods { count, .. }
                | SubjectVerbActionAst::Incubate { amount: count, .. }
                | SubjectVerbActionAst::Monstrosity { amount: count }
                | SubjectVerbActionAst::LoseLife { amount: count }
                | SubjectVerbActionAst::PayLife { amount: count }
                | SubjectVerbActionAst::GainLife { amount: count }
                | SubjectVerbActionAst::DealDamage { amount: count, .. }
                | SubjectVerbActionAst::DealDamageEqualToPower { amount: count, .. }
                | SubjectVerbActionAst::DealDistributedDamage { amount: count, .. }
                | SubjectVerbActionAst::DealDamageEach { amount: count, .. }
                | SubjectVerbActionAst::PreventDamage { amount: count, .. }
                | SubjectVerbActionAst::PreventDamageEach { amount: count, .. }
                | SubjectVerbActionAst::CopySpell { count, .. }
                | SubjectVerbActionAst::PutCounters { count, .. }
                | SubjectVerbActionAst::PutCounterChoice { count, .. }
                | SubjectVerbActionAst::PutCountersAll { count, .. }
                | SubjectVerbActionAst::RemoveUpToAnyCounters { amount: count, .. }
                | SubjectVerbActionAst::RemoveCountersAll { amount: count, .. }
                | SubjectVerbActionAst::Discard { count, .. }
                | SubjectVerbActionAst::PoisonCounters { count }
                | SubjectVerbActionAst::EnergyCounters { count }
                | SubjectVerbActionAst::ExperienceCounters { count }
                | SubjectVerbActionAst::TicketCounters { count }
                | SubjectVerbActionAst::PayEnergy { amount: count }
                | SubjectVerbActionAst::SetLifeTotal { amount: count }
                | SubjectVerbActionAst::AddManaScaled { amount: count, .. }
                | SubjectVerbActionAst::AddManaAnyColor { amount: count, .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { amount: count }
                | SubjectVerbActionAst::AddManaChosenColor { amount: count, .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount: count, .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { amount: count }
                | SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                    amount: count,
                    ..
                }
                | SubjectVerbActionAst::LookAtTopCards { count, .. }
                | SubjectVerbActionAst::MoveToLibraryNthFromTop {
                    position: count, ..
                }
                | SubjectVerbActionAst::AdditionalLandPlays { count, .. }
                | SubjectVerbActionAst::HealDamage {
                    amount: Some(count),
                    ..
                } => replace_value(count),
                _ => {}
            }
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                replace_effect(nested_effect);
            }
        });
    }

    for effect in effects {
        replace_effect(effect);
    }
}

fn effects_have_creature_death_gate(effects: &[EffectAst]) -> bool {
    effects.iter().any(|effect| {
        let mut found = matches!(
            effect,
            EffectAst::Conditional { predicate, .. }
                if predicate_counts_creature_deaths(predicate)
        );
        for_each_nested_effects(effect, true, |nested| {
            found |= effects_have_creature_death_gate(nested);
        });
        found
    })
}

fn damaged_death_condition_target_filter(condition: &Condition) -> Option<ObjectFilter> {
    match condition {
        Condition::CreatureDealtDamageBySourceDiedThisTurn {
            victim,
            damager,
            count,
        } if *count == 1 => {
            let mut filter = victim.clone();
            filter.zone = Some(Zone::Graveyard);
            filter.entered_graveyard_from_battlefield_this_turn = true;
            filter.dealt_damage_by_source_this_turn = Some(*damager);
            Some(filter)
        }
        Condition::And(left, right) => damaged_death_condition_target_filter(left)
            .or_else(|| damaged_death_condition_target_filter(right)),
        _ => None,
    }
}

fn retarget_source_move_to_damaged_death_card(lowered: &mut LoweredEffects, condition: &Condition) {
    let Some(filter) = damaged_death_condition_target_filter(condition) else {
        return;
    };
    let Some(segment) = lowered.effects.segments.first_mut() else {
        return;
    };
    let Some(effect) = segment.default_effects.first_mut() else {
        return;
    };
    let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(move_to_zone) = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return;
    };
    if !matches!(move_to_zone.target.base(), ChooseSpec::Source)
        || move_to_zone.zone != Zone::Battlefield
    {
        return;
    }

    let mut replacement = move_to_zone.clone();
    replacement.target =
        ChooseSpec::Object(filter).with_count(crate::effect::ChoiceCount::exactly(1));
    *effect = Effect::new(crate::effects::TaggedEffect::new(
        tagged.tag.clone(),
        Effect::new(replacement),
    ));
}

fn object_filter_is_it_reference(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == IT_TAG
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    })
}

fn object_filter_has_single_tag_reference(filter: &ObjectFilter, tag: &crate::tag::TagKey) -> bool {
    filter.tagged_constraints.len() == 1
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *tag && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
}

fn rewrite_delayed_return_control_loss_sacrifice_followup(lowered: &mut LoweredEffects) -> bool {
    let Some(segment) = lowered.effects.segments.first_mut() else {
        return false;
    };
    if segment.default_effects.len() < 4 {
        return false;
    }

    let Some(triggering) =
        segment.default_effects[0].downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    else {
        return false;
    };
    let triggering_tag = triggering.tag.clone();
    let Some(end_step_schedule) =
        segment.default_effects[1].downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
    else {
        return false;
    };
    if !matches!(
        end_step_schedule.trigger,
        ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(_)
    ) || !end_step_schedule.one_shot
        || end_step_schedule.effects.len() != 1
    {
        return false;
    }
    let Some(returned) =
        end_step_schedule.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    if returned.zone != Zone::Battlefield
        || returned.battlefield_controller != crate::effects::BattlefieldController::You
        || !matches!(returned.target.base(), ChooseSpec::Tagged(tag) if tag == &triggering_tag)
    {
        return false;
    }

    let Some(choose) =
        segment.default_effects[2].downcast_ref::<crate::effects::ChooseObjectsEffect>()
    else {
        return false;
    };
    let mut plain_creature = choose.filter.clone();
    let creature_zone = choose.zone.or(plain_creature.zone);
    plain_creature.zone = None;
    plain_creature.controller = None;
    plain_creature.card_types.clear();
    if creature_zone != Some(Zone::Battlefield)
        || choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.filter.controller != Some(PlayerFilter::You)
        || choose.filter.card_types.as_slice() != [crate::types::CardType::Creature]
        || plain_creature != ObjectFilter::default()
    {
        return false;
    }
    let sacrificed_tag = choose.tag.clone();

    let Some(sacrifice) =
        segment.default_effects[3].downcast_ref::<crate::effects::SacrificePlayerEffect>()
    else {
        return false;
    };
    if sacrifice.player != PlayerFilter::You
        || !matches!(sacrifice.count, Value::Fixed(1))
        || !object_filter_has_single_tag_reference(&sacrifice.filter, &sacrificed_tag)
    {
        return false;
    }

    let returned_tag = crate::tag::TagKey::from("returned_control_loss");
    let tagged_return = Effect::new(crate::effects::TaggedEffect::new(
        returned_tag.clone(),
        Effect::new(returned.clone()),
    ));
    let delayed_sacrifice = Effect::sacrifice_player(
        ObjectFilter::tagged(returned_tag.clone()),
        Value::Fixed(1),
        PlayerFilter::You,
    );
    let control_loss_schedule = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
        returned_tag,
        ironsmith_core::DelayedTriggerSpec::SourceControllerLosesControl {
            source_description: "this creature".to_string(),
        },
        vec![delayed_sacrifice],
        true,
        Vec::new(),
        PlayerFilter::You,
    )
    .watch_ability_source();

    let mut rewritten_end_step = end_step_schedule.clone();
    rewritten_end_step.effects = vec![tagged_return, Effect::new(control_loss_schedule)];
    segment.default_effects[1] = Effect::new(rewritten_end_step);
    segment.default_effects.drain(2..4);
    true
}

fn rewrite_source_control_loss_sacrifice_followup(lowered: &mut LoweredEffects) {
    if rewrite_delayed_return_control_loss_sacrifice_followup(lowered) {
        return;
    }
    let Some(segment) = lowered.effects.segments.first_mut() else {
        return;
    };
    if segment.default_effects.len() < 3 {
        return;
    }

    let Some(tagged_move) =
        segment.default_effects[0].downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return;
    };
    let Some(move_to_zone) = tagged_move
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return;
    };
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
    {
        return;
    }
    let moved_tag = tagged_move.tag.clone();

    let Some(choose) =
        segment.default_effects[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
    else {
        return;
    };
    if choose.zone.or(choose.filter.zone) != Some(Zone::Battlefield)
        || !choose.count.is_single()
        || !object_filter_has_single_tag_reference(&choose.filter, &moved_tag)
    {
        return;
    }
    let sacrificed_tag = choose.tag.clone();

    let Some(sacrifice) =
        segment.default_effects[2].downcast_ref::<crate::effects::SacrificePlayerEffect>()
    else {
        return;
    };
    if sacrifice.player != PlayerFilter::You
        || !matches!(sacrifice.count, Value::Fixed(1))
        || !object_filter_has_single_tag_reference(&sacrifice.filter, &sacrificed_tag)
    {
        return;
    }

    let delayed_sacrifice = Effect::sacrifice_player(
        ObjectFilter::tagged(moved_tag.clone()),
        Value::Fixed(1),
        PlayerFilter::You,
    );
    let schedule = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
        moved_tag,
        ironsmith_core::DelayedTriggerSpec::SourceControllerLosesControl {
            source_description: "this creature".to_string(),
        },
        vec![delayed_sacrifice],
        true,
        Vec::new(),
        PlayerFilter::You,
    )
    .watch_ability_source();
    segment
        .default_effects
        .splice(1..3, [Effect::new(schedule)]);
}

fn replace_it_target_with_filter(target: &mut TargetAst, filter: &ObjectFilter) -> bool {
    match target {
        TargetAst::Tagged(tag, span) if tag.as_str() == IT_TAG => {
            *target = TargetAst::Object(filter.clone(), *span, None);
            true
        }
        TargetAst::Object(target_filter, _, _) if object_filter_is_it_reference(target_filter) => {
            *target_filter = filter.clone();
            true
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            replace_it_target_with_filter(inner, filter)
        }
        _ => false,
    }
}

fn replace_it_object_followup_filter(effect: &mut EffectAst, filter: &ObjectFilter) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::GrantAbilitiesAll {
                filter: target_filter,
                ..
            }
            | SubjectVerbActionAst::RemoveAbilitiesAll {
                filter: target_filter,
                ..
            }
            | SubjectVerbActionAst::PumpAll {
                filter: target_filter,
                ..
            } if object_filter_is_it_reference(target_filter) => {
                *target_filter = filter.clone();
                true
            }
            SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
            | SubjectVerbActionAst::Pump { target, .. } => {
                replace_it_target_with_filter(target, filter)
            }
            _ => false,
        },
        _ => false,
    }
}

fn carry_all_object_sweep_filter_to_it_followups(effects: &mut [EffectAst]) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let filter = match &effects[idx] {
            EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
                SubjectVerbActionAst::PumpAll { filter, .. }
                | SubjectVerbActionAst::ScalePowerToughnessAll { filter, .. } => filter.clone(),
                _ => {
                    idx += 1;
                    continue;
                }
            },
            _ => {
                idx += 1;
                continue;
            }
        };

        if replace_it_object_followup_filter(&mut effects[idx + 1], &filter) {
            idx += 2;
        } else {
            idx += 1;
        }
    }
}

fn discard_one_or_more_trigger_uses_event_count(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => {
            discard_one_or_more_trigger_uses_event_count(trigger)
        }
        TriggerSpec::PlayerDiscardsCard { one_or_more, .. } => *one_or_more,
        _ => false,
    }
}

fn counter_removed_this_way_trigger_uses_event_count(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => {
            counter_removed_this_way_trigger_uses_event_count(trigger)
        }
        TriggerSpec::CounterRemovedFrom {
            caused_by_source: true,
            ..
        } => true,
        _ => false,
    }
}

fn preserve_counter_removed_this_way_damage_amount(effect: &mut EffectAst) {
    fn preserve_in_effects(effects: &mut [EffectAst]) {
        for effect in effects {
            preserve_counter_removed_this_way_damage_amount(effect);
        }
    }

    if let EffectAst::SubjectVerb(subject_verb) = effect {
        let amount = match &mut subject_verb.action {
            SubjectVerbActionAst::DealDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { amount, .. }
            | SubjectVerbActionAst::DealDistributedDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEach { amount, .. } => Some(amount),
            _ => None,
        };
        if let Some(amount) = amount
            && matches!(amount.unhinted(), Value::EventValue(EventValueSpec::Amount))
            && !amount.has_surface_hint(ValueSurfaceHint::CountersRemovedThisWay)
        {
            *amount = amount
                .clone()
                .with_surface_hint(ValueSurfaceHint::CountersRemovedThisWay);
        }
    }

    for_each_nested_effects_mut(effect, false, preserve_in_effects);
}

fn replace_it_count_with_event_count(effect: &mut EffectAst) {
    fn is_it_count(value: &Value) -> bool {
        matches!(value, Value::Count(filter) if object_filter_is_it_reference(filter))
    }

    fn replace_in_effects(effects: &mut [EffectAst]) {
        for effect in effects {
            replace_it_count_with_event_count(effect);
        }
    }

    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::PumpForEach { count, .. } = &mut subject_verb.action
        && is_it_count(count)
    {
        *count = Value::EventValue(EventValueSpec::Amount)
            .with_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay);
    }

    for_each_nested_effects_mut(effect, false, replace_in_effects);
}

fn death_trigger_counts_counters_on_triggering_object(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => {
            death_trigger_counts_counters_on_triggering_object(trigger)
        }
        TriggerSpec::Dies(filter) | TriggerSpec::DiesOneOrMore(filter) => {
            filter.with_counter.is_some()
        }
        TriggerSpec::DiesDuringTurn { filter, .. } => filter.with_counter.is_some(),
        _ => false,
    }
}

fn replace_exile_top_event_count_with_triggering_counter_count(effect: &mut EffectAst) {
    fn replace_in_effects(effects: &mut [EffectAst]) {
        for effect in effects {
            replace_exile_top_event_count_with_triggering_counter_count(effect);
        }
    }

    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::ExileTopOfLibrary { count, .. } = &mut subject_verb.action
        && matches!(
            count.unhinted(),
            Value::EventValue(EventValueSpec::Amount)
                | Value::EventValue(EventValueSpec::LifeAmount)
        )
    {
        *count = Value::CountersOn(Box::new(ChooseSpec::Tagged("triggering".into())), None);
    }

    for_each_nested_effects_mut(effect, false, replace_in_effects);
}

fn phase_step_trigger_object_reference_tag(trigger: &TriggerSpec) -> Option<&str> {
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return phase_step_trigger_object_reference_tag(trigger);
    }
    let player = match trigger {
        TriggerSpec::BeginningOfUpkeep(player)
        | TriggerSpec::BeginningOfDrawStep(player)
        | TriggerSpec::BeginningOfCombat(player)
        | TriggerSpec::BeginningOfEndStep(player)
        | TriggerSpec::BeginningOfPrecombatMain(player)
        | TriggerSpec::BeginningOfPostcombatMain(player) => player,
        _ => return None,
    };
    match player {
        PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
        | PlayerFilter::OwnerOf(ObjectRef::Tagged(tag))
        | PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag))
        | PlayerFilter::AliasedOwnerOf(ObjectRef::Tagged(tag)) => Some(tag.as_str()),
        _ => None,
    }
}

fn phase_step_trigger_has_no_object_reference(trigger: &TriggerSpec) -> bool {
    if phase_step_trigger_object_reference_tag(trigger).is_some() {
        return false;
    }
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return phase_step_trigger_has_no_object_reference(trigger);
    }
    matches!(
        trigger,
        TriggerSpec::BeginningOfUpkeep(_)
            | TriggerSpec::BeginningOfDrawStep(_)
            | TriggerSpec::BeginningOfCombat(_)
            | TriggerSpec::BeginningOfEndStep(_)
            | TriggerSpec::BeginningOfTheEndStep
            | TriggerSpec::BeginningOfPrecombatMain(_)
            | TriggerSpec::BeginningOfPostcombatMain(_)
    )
}

pub(crate) fn default_trigger_last_object_tag(trigger: &TriggerSpec) -> Option<&str> {
    if let TriggerSpec::WithIntro { trigger, .. } = trigger {
        return default_trigger_last_object_tag(trigger);
    }
    if let Some(tag) = phase_step_trigger_object_reference_tag(trigger) {
        return Some(tag);
    }
    if phase_step_trigger_has_no_object_reference(trigger) {
        return None;
    }
    if match trigger {
        TriggerSpec::ThisBecomesBlockedByObject(_)
        | TriggerSpec::BecomesBlockedByObjectWithLesserPower { .. } => true,
        TriggerSpec::WithIntro { trigger, .. } => {
            matches!(
                **trigger,
                TriggerSpec::ThisBecomesBlockedByObject(_)
                    | TriggerSpec::BecomesBlockedByObjectWithLesserPower { .. }
            )
        }
        _ => false,
    } {
        return Some("blocking");
    }
    if matches!(trigger, TriggerSpec::BlocksObjectWithLesserPower { .. }) {
        return Some("blocked");
    }
    if matches!(
        trigger,
        TriggerSpec::KeywordActionTaggedObject { object_tag, .. }
            if object_tag.as_str() == IT_TAG
    ) {
        return Some(IT_TAG);
    }
    if matches!(
        trigger,
        TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ManifestDread,
            ..
        }
    ) {
        return Some(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG);
    }
    if matches!(
        trigger,
        TriggerSpec::ThisIsDealtDamage
            | TriggerSpec::ThisIsDealtCombatDamage
            | TriggerSpec::IsDealtDamage(_)
            | TriggerSpec::IsDealtCombatDamage(_)
            | TriggerSpec::ThisDealsDamageTo(_)
            | TriggerSpec::ThisDealsCombatDamageTo(_)
            | TriggerSpec::DealsDamageTo { .. }
            | TriggerSpec::DealsCombatDamageTo { .. }
    ) {
        Some("damaged")
    } else {
        Some("triggering")
    }
}

fn default_trigger_last_object_prelude(
    trigger: &TriggerSpec,
    tag: &crate::cards::builders::TagKey,
) -> Option<EffectPreludeTag> {
    let event_participant_filter = |filter: &ObjectFilter| {
        let mut filter = filter.clone();
        // The event snapshot is the source of truth for the exact combat
        // participant. Requiring the object to still advertise a live combat
        // role while the trigger resolves makes the snapshot tag fail in
        // synthetic/LKI scenarios and after combat state has advanced.
        filter.blocking = false;
        filter.attacking = false;
        filter
    };
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => default_trigger_last_object_prelude(trigger, tag),
        TriggerSpec::ThisBecomesBlockedByObject(filter) => Some(
            EffectPreludeTag::TriggeringBlockers(tag.clone(), event_participant_filter(filter)),
        ),
        TriggerSpec::BecomesBlockedByObjectWithLesserPower { blocker, .. } => Some(
            EffectPreludeTag::TriggeringBlockers(tag.clone(), event_participant_filter(blocker)),
        ),
        TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ManifestDread,
            ..
        } if tag.as_str() == crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG => {
            Some(EffectPreludeTag::TriggeringObject(tag.clone()))
        }
        _ => None,
    }
}

fn spell_cast_trigger_targets_source(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => spell_cast_trigger_targets_source(trigger),
        TriggerSpec::SpellCast {
            filter: Some(filter),
            ..
        } => filter
            .targets_object
            .as_deref()
            .is_some_and(|target_filter| target_filter.source),
        _ => false,
    }
}

fn trigger_is_spell_cast(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_is_spell_cast(trigger),
        TriggerSpec::SpellCast { .. } | TriggerSpec::NthSpellOfTurnCast { .. } => true,
        _ => false,
    }
}

fn triggering_stack_object_kind(trigger: &TriggerSpec) -> Option<crate::filter::StackObjectKind> {
    use crate::filter::StackObjectKind;

    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => triggering_stack_object_kind(trigger),
        TriggerSpec::SpellCast { .. }
        | TriggerSpec::NthSpellOfTurnCast { .. }
        | TriggerSpec::SpellCopied { .. }
        | TriggerSpec::SpellCountered { .. } => Some(StackObjectKind::Spell),
        TriggerSpec::AbilityActivated { .. } | TriggerSpec::AbilityTriggered { .. } => {
            Some(StackObjectKind::Ability)
        }
        TriggerSpec::Either(left, right) => {
            let left = triggering_stack_object_kind(left)?;
            let right = triggering_stack_object_kind(right)?;
            if left == right {
                Some(left)
            } else if matches!(
                (left, right),
                (StackObjectKind::Spell, StackObjectKind::Ability)
                    | (StackObjectKind::Ability, StackObjectKind::Spell)
                    | (StackObjectKind::SpellOrAbility, _)
                    | (_, StackObjectKind::SpellOrAbility)
            ) {
                Some(StackObjectKind::SpellOrAbility)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn copy_target_is_triggering_stack_object(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => matches!(tag.as_str(), "triggering" | IT_TAG),
        TargetAst::WithCount(target, _) | TargetAst::WithCountValue(target, _, _) => {
            copy_target_is_triggering_stack_object(target)
        }
        _ => false,
    }
}

fn preserve_copy_reference_kind_from_trigger(effects: &mut [EffectAst], trigger: &TriggerSpec) {
    let trigger_kind = triggering_stack_object_kind(trigger);
    for effect in effects {
        match effect {
            EffectAst::DelayedTriggerThisTurn {
                trigger, effects, ..
            }
            | EffectAst::DelayedTriggerForDuration {
                trigger, effects, ..
            } => {
                preserve_copy_reference_kind_from_trigger(effects, trigger);
                continue;
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CopySpell {
                        target,
                        target_reference_kind,
                        ..
                    },
                ..
            }) if target_reference_kind.is_none()
                && copy_target_is_triggering_stack_object(target) =>
            {
                *target_reference_kind = trigger_kind;
            }
            _ => {}
        }

        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| preserve_copy_reference_kind_from_trigger(nested, trigger),
        );
    }
}

fn retarget_spell_cast_mana_spent_predicate(
    trigger: &TriggerSpec,
    predicate: PredicateAst,
) -> PredicateAst {
    if !trigger_is_spell_cast(trigger) {
        return predicate;
    }

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
        PredicateAst::Not(inner) => PredicateAst::Not(Box::new(
            retarget_spell_cast_mana_spent_predicate(trigger, *inner),
        )),
        PredicateAst::And(left, right) => PredicateAst::And(
            Box::new(retarget_spell_cast_mana_spent_predicate(trigger, *left)),
            Box::new(retarget_spell_cast_mana_spent_predicate(trigger, *right)),
        ),
        PredicateAst::Or(left, right) => PredicateAst::Or(
            Box::new(retarget_spell_cast_mana_spent_predicate(trigger, *left)),
            Box::new(retarget_spell_cast_mana_spent_predicate(trigger, *right)),
        ),
        other => other,
    }
}

fn retarget_spell_cast_mana_spent_predicates_in_effects(
    trigger: &TriggerSpec,
    effects: &mut [EffectAst],
) {
    if !trigger_is_spell_cast(trigger) {
        return;
    }

    for effect in effects {
        match effect {
            EffectAst::Conditional { predicate, .. }
            | EffectAst::TrailingIf { predicate, .. }
            | EffectAst::TrailingUnless { predicate, .. }
            | EffectAst::SelfReplacement { predicate, .. } => {
                *predicate = retarget_spell_cast_mana_spent_predicate(trigger, predicate.clone());
            }
            _ => {}
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            retarget_spell_cast_mana_spent_predicates_in_effects(trigger, nested);
        });
    }
}

fn retarget_spell_cast_mana_spent_condition(
    trigger: &TriggerSpec,
    condition: Condition,
) -> Condition {
    if !trigger_is_spell_cast(trigger) {
        return condition;
    }

    match condition {
        Condition::TargetSpellManaSpentToCastAtLeast { amount, symbol } => {
            Condition::TriggeringSpellManaSpentToCastAtLeast { amount, symbol }
        }
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            Condition::TriggeringSpellManaSpentToCastAtLeast { amount, symbol }
        }
        Condition::ColoredManaSpentToCastThisSpellAtLeast(amount) => {
            Condition::TriggeringSpellColoredManaSpentToCastAtLeast(amount)
        }
        Condition::Not(inner) => Condition::Not(Box::new(
            retarget_spell_cast_mana_spent_condition(trigger, *inner),
        )),
        Condition::And(left, right) => Condition::And(
            Box::new(retarget_spell_cast_mana_spent_condition(trigger, *left)),
            Box::new(retarget_spell_cast_mana_spent_condition(trigger, *right)),
        ),
        Condition::Or(left, right) => Condition::Or(
            Box::new(retarget_spell_cast_mana_spent_condition(trigger, *left)),
            Box::new(retarget_spell_cast_mana_spent_condition(trigger, *right)),
        ),
        other => other,
    }
}

fn retarget_it_target_to_source(target: &mut TargetAst) {
    match target {
        TargetAst::Tagged(tag, span) if tag.as_str() == IT_TAG => {
            *target = TargetAst::Source(span.clone());
        }
        TargetAst::Object(filter, span, _) if *filter == ObjectFilter::tagged(IT_TAG) => {
            *target = TargetAst::Source(span.clone());
        }
        TargetAst::WithCount(inner, _) => retarget_it_target_to_source(inner),
        _ => {}
    }
}

fn retarget_bare_it_effect_targets_to_source(effect: &mut EffectAst) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::PutCounters { target, .. }
            | SubjectVerbActionAst::PutCounterChoice { target, .. }
            | SubjectVerbActionAst::PutOrRemoveCounters { target, .. }
            | SubjectVerbActionAst::RemoveUpToAnyCounters { target, .. }
            | SubjectVerbActionAst::DoubleCountersOnTarget { target, .. }
            | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { target, .. } => {
                retarget_it_target_to_source(target);
            }
            SubjectVerbActionAst::MoveToZone {
                target,
                attached_to,
                ..
            } => {
                retarget_it_target_to_source(target);
                if let Some(attached_to) = attached_to {
                    retarget_it_target_to_source(attached_to);
                }
            }
            _ => {}
        },
        _ => {}
    }
    if matches!(effect, EffectAst::ForEachTagged { .. }) {
        return;
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for effect in nested {
            retarget_bare_it_effect_targets_to_source(effect);
        }
    });
}

fn trigger_provides_stack_object(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_provides_stack_object(trigger),
        TriggerSpec::SpellCast { .. } | TriggerSpec::AbilityActivated { .. } => true,
        // Becomes-targeted triggers record the TARGETING spell or ability as
        // the triggering event object ("counter that spell", "choose new
        // targets for that spell").
        TriggerSpec::ThisBecomesTargeted
        | TriggerSpec::BecomesTargeted(_)
        | TriggerSpec::ThisBecomesTargetedBySpell(_)
        | TriggerSpec::ThisBecomesTargetedByStackObject(_)
        | TriggerSpec::BecomesTargetedByStackObject { .. } => true,
        TriggerSpec::Either(left, right) => {
            trigger_provides_stack_object(left) || trigger_provides_stack_object(right)
        }
        _ => false,
    }
}

fn bind_stack_retargets_to_triggering_object(effects: &mut [EffectAst]) {
    fn visit(effect: &mut EffectAst) {
        if std::env::var("IRONSMITH_CHOICE_TRACE").is_ok()
            && let EffectAst::SubjectVerb(subject_verb) = &*effect
            && let SubjectVerbActionAst::RetargetStackObject { target, .. } = &subject_verb.action
        {
            eprintln!("bind-stack-retarget: target={target:?}");
        }
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::RetargetStackObject { target, .. } =
                &mut subject_verb.action
            && matches!(
                target,
                TargetAst::Tagged(tag, _)
                    if tag.as_str() == crate::cards::builders::IT_TAG
            )
        {
            *target = TargetAst::Tagged(crate::tag::TagKey::from("triggering"), None);
        }
        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| {
                for nested_effect in nested {
                    visit(nested_effect);
                }
            },
        );
    }

    for effect in effects {
        visit(effect);
    }
}

fn retarget_phase_step_it_targets_to_source(effects: &mut [EffectAst]) {
    for effect in effects {
        retarget_bare_it_effect_targets_to_source(effect);
    }
}

fn has_local_target_prelude_before_it_reference(effects: &[EffectAst]) -> bool {
    for effect in effects {
        if effect_references_it_tag(effect) {
            return false;
        }
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::TargetOnly { target, .. } = &subject_verb.action
            && target_can_establish_local_object_reference(target)
        {
            return true;
        }
    }
    false
}

fn has_prior_effect_before_it_reference(effects: &[EffectAst]) -> bool {
    let mut saw_prior_effect = false;
    for effect in effects {
        if effect_references_it_tag(effect) {
            if saw_prior_effect {
                return true;
            }

            let mut nested_has_prior_effect = false;
            for_each_nested_effects(effect, true, |nested| {
                nested_has_prior_effect |= has_prior_effect_before_it_reference(nested);
            });
            return nested_has_prior_effect;
        }
        saw_prior_effect = true;
    }
    false
}

fn tagged_target_key(target: &TargetAst) -> Option<&crate::cards::builders::TagKey> {
    match target {
        TargetAst::Tagged(tag, _) => Some(tag),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            tagged_target_key(inner)
        }
        _ => None,
    }
}

/// Rebind a plural "return them" back-reference to the concrete helper tag
/// established by the preceding typed choose/exile chain.  The parser emits
/// `SOURCE_EXILED_TAG` as a cross-sentence placeholder; resolving it only from
/// `last_object_tag` is too late because lowering the aggregate exile itself
/// replaces that environment entry with the placeholder.  Preparing the
/// typed chain here also lets the return remain explicitly plural.
fn rebind_aggregate_source_exiled_returns(effects: &mut [EffectAst]) {
    let mut helper_choices = std::collections::HashMap::<String, (usize, bool)>::new();
    let mut last_aggregate_exile = None;

    fn collect(
        effect: &EffectAst,
        helper_choices: &mut std::collections::HashMap<String, (usize, bool)>,
        last_aggregate_exile: &mut Option<crate::cards::builders::TagKey>,
    ) {
        if let EffectAst::ChooseObjects { tag, count, .. } = effect
            && is_sentence_helper_exiled_collection_tag(tag.as_str())
        {
            let entry = helper_choices
                .entry(tag.as_str().to_string())
                .or_insert((0, false));
            entry.0 += 1;
            entry.1 |= count.max.map_or(true, |max| max > 1);
        }
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::Exile { target, .. } = &subject_verb.action
            && let Some(tag) = tagged_target_key(target)
            && let Some((choice_count, explicitly_plural)) = helper_choices.get(tag.as_str())
            && (*choice_count > 1 || *explicitly_plural)
        {
            *last_aggregate_exile = Some(tag.clone());
        }
        for_each_nested_effects(effect, true, |nested| {
            for child in nested {
                collect(child, helper_choices, last_aggregate_exile);
            }
        });
    }

    for effect in effects.iter() {
        collect(effect, &mut helper_choices, &mut last_aggregate_exile);
    }

    let Some(tag) = last_aggregate_exile else {
        return;
    };
    fn rewrite(effect: &mut EffectAst, tag: &crate::cards::builders::TagKey) {
        if let EffectAst::SubjectVerb(subject_verb) = effect {
            let replacement = match &subject_verb.action {
                SubjectVerbActionAst::ReturnToBattlefield {
                    target,
                    tapped,
                    transformed,
                    converted,
                    controller,
                    count_value,
                    as_aura,
                    top_only,
                } if tagged_target_key(target).is_some_and(|target_tag| {
                    matches!(target_tag.as_str(), crate::tag::SOURCE_EXILED_TAG | IT_TAG)
                }) && !*transformed
                    && !*converted
                    && !*top_only
                    && count_value.is_none()
                    && as_aura.is_none() =>
                {
                    Some(SubjectVerbActionAst::ReturnAllToBattlefield {
                        filter: ObjectFilter::tagged(tag.clone()).in_zone(Zone::Exile),
                        tapped: *tapped,
                        face_down: false,
                        controller: controller.clone(),
                        verb_surface: ironsmith_core::MoveToZoneVerbSurface::Return,
                    })
                }
                _ => None,
            };
            if let Some(replacement) = replacement {
                subject_verb.action = replacement;
            }
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            for child in nested {
                rewrite(child, tag);
            }
        });
    }
    for effect in effects {
        rewrite(effect, &tag);
    }
}

fn rewrite_prepare_effects_from_normalized(
    mut semantic_effects: Vec<EffectAst>,
    reference_effects: &[EffectAst],
    mut imports: ReferenceImports,
    config: EffectReferenceResolutionConfig,
    inferred_last_player_filter: Option<PlayerFilter>,
    default_last_object_tag: Option<crate::cards::builders::TagKey>,
    default_last_object_prelude: Option<EffectPreludeTag>,
    include_trigger_prelude: bool,
) -> Result<PreparedEffectsForLowering, CardTextError> {
    let (flattened_effects, source_sentence_segments) =
        flatten_top_level_source_sentences(semantic_effects);
    // Source-sentence flattening can expose a cross-sentence construct that
    // was not visible when the individual sentence wrappers were normalized
    // (notably `... If you do, repeat this process`). The special shared
    // lowering paths return no sentence counts, so normalize those flattened
    // sequences again without disturbing the counts used by ordinary source
    // sentence segmentation.
    semantic_effects = if source_sentence_segments.is_empty() {
        normalize_effects_ast(&flattened_effects)
    } else {
        flattened_effects
    };
    rebind_aggregate_source_exiled_returns(&mut semantic_effects);
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

    let initial_env = ReferenceEnv::from_imports(
        &imports,
        config.initial_iterated_player,
        config.allow_life_event_value,
        config.bind_unbound_x_to_last_effect,
        config.initial_last_effect_id,
    );
    let annotated =
        annotate_effect_sequence(&semantic_effects, &imports, config, Default::default())?;

    if include_trigger_prelude {
        let needs_triggering_prelude =
            annotated
                .effects
                .iter()
                .zip(&semantic_effects)
                .any(|(annotated, semantic_effect)| {
                    effect_references_tag(&annotated.effect, "triggering")
                        || ((effect_references_it_tag(semantic_effect)
                            || effect_references_its_controller(semantic_effect))
                            && annotated
                                .in_env
                                .known_last_object_tag()
                                .is_some_and(|tag| tag.as_str() == "triggering"))
                });
        if needs_triggering_prelude {
            let tag = default_last_object_tag
                .as_ref()
                .filter(|tag| tag.as_str() == IT_TAG)
                .cloned()
                .unwrap_or_else(|| crate::cards::builders::TagKey::from("triggering"));
            prelude.insert(0, EffectPreludeTag::TriggeringObject(tag));
        }
        if let Some(default_prelude) = default_last_object_prelude {
            prelude.insert(0, default_prelude);
        }
        if effects_reference_tag(reference_effects, "triggering_source") {
            prelude.insert(
                0,
                EffectPreludeTag::TriggeringSource(crate::cards::builders::TagKey::from(
                    "triggering_source",
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

    let exports = ReferenceExports::from_env(&annotated.final_env);

    Ok(PreparedEffectsForLowering {
        effects: semantic_effects,
        source_sentence_segments,
        imports,
        initial_env,
        annotated,
        exports,
        prelude,
        force_auto_tag_object_targets: config.force_auto_tag_object_targets,
    })
}

fn source_sentence_followup_requires_shared_lowering(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::ForEachOpponentDoesNot { .. }
            | EffectAst::ForEachPlayerDoesNot { .. }
            | EffectAst::ForEachOpponentDid { .. }
            | EffectAst::ForEachPlayerDid { .. }
            | EffectAst::VoteOption { .. }
            | EffectAst::VoteExtra { .. }
    )
}

fn source_sentence_boundary_continues_repeat_process(
    effects: &[EffectAst],
    boundary: usize,
) -> bool {
    if boundary + 1 != effects.len() {
        return false;
    }
    if matches!(
        effects.get(boundary),
        Some(
            EffectAst::RepeatThisProcess
                | EffectAst::RepeatThisProcessOnce
                | EffectAst::RepeatThisProcessMay
        )
    ) {
        return true;
    }
    let Some(EffectAst::IfResult { effects, .. }) = effects.get(boundary) else {
        return false;
    };
    match effects.last() {
        Some(EffectAst::RepeatThisProcess) => true,
        Some(EffectAst::Coordinated { effects, .. }) => {
            matches!(effects.last(), Some(EffectAst::RepeatThisProcess))
        }
        _ => false,
    }
}

fn push_unique_source_sentence_hand_tag(tags: &mut Vec<TagKey>, tag: &TagKey) {
    if !tags.iter().any(|known| known == tag) {
        tags.push(tag.clone());
    }
}

fn collect_source_sentence_hand_pipeline_tags(effects: &[EffectAst], tags: &mut Vec<TagKey>) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RevealCardsFromHand { tag, .. },
                ..
            }) => push_unique_source_sentence_hand_tag(tags, tag),
            EffectAst::ChooseObjects { filter, tag, .. }
                if filter.zone == Some(Zone::Hand)
                    || tags
                        .iter()
                        .any(|hand_tag| filter_references_tag(filter, hand_tag.as_str())) =>
            {
                push_unique_source_sentence_hand_tag(tags, tag);
            }
            _ => {}
        }

        for_each_nested_effects(effect, true, |nested| {
            collect_source_sentence_hand_pipeline_tags(nested, tags);
        });
    }
}

fn source_sentence_effect_consumes_hand_pipeline_tag(effect: &EffectAst, tag: &TagKey) -> bool {
    let directly_consumes = match effect {
        EffectAst::ChooseObjects { filter, .. } => filter_references_tag(filter, tag.as_str()),
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Discard { filter, .. },
            ..
        }) => {
            effect_references_tag(effect, tag.as_str())
                || filter
                    .as_ref()
                    .is_some_and(|filter| filter_references_tag(filter, tag.as_str()))
        }
        _ => false,
    };
    if directly_consumes {
        return true;
    }

    let mut nested_consumes = false;
    for_each_nested_effects(effect, true, |nested| {
        nested_consumes |= nested.iter().any(|nested_effect| {
            source_sentence_effect_consumes_hand_pipeline_tag(nested_effect, tag)
        });
    });
    nested_consumes
}

fn source_sentence_boundary_splits_hand_pipeline(effects: &[EffectAst], boundary: usize) -> bool {
    let mut hand_tags = Vec::new();
    collect_source_sentence_hand_pipeline_tags(&effects[..boundary], &mut hand_tags);
    hand_tags.iter().any(|tag| {
        effects[boundary..]
            .iter()
            .any(|effect| source_sentence_effect_consumes_hand_pipeline_tag(effect, tag))
    })
}

fn flatten_top_level_source_sentences(
    effects: Vec<EffectAst>,
) -> (Vec<EffectAst>, Vec<SourceSentenceSegment>) {
    fn preserve_participant_order_on_first_effect(effects: &mut [EffectAst]) {
        let Some(first) = effects.first_mut() else {
            return;
        };
        match first {
            EffectAst::VoteStart {
                starting_with_controller,
                ..
            }
            | EffectAst::VoteStartObjects {
                starting_with_controller,
                ..
            }
            | EffectAst::VoteStartPlayers {
                starting_with_controller,
                ..
            } => *starting_with_controller = true,
            EffectAst::Sequence { effects }
            | EffectAst::CommaThen { effects }
            | EffectAst::Coordinated { effects, .. }
            | EffectAst::SourceSentence { effects, .. } => {
                preserve_participant_order_on_first_effect(effects);
            }
            _ => {}
        }
    }

    let has_source_sentence = effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::SourceSentence { .. }));
    if !has_source_sentence {
        return (effects, Vec::new());
    }

    let all_source_sentences = effects
        .iter()
        .all(|effect| matches!(effect, EffectAst::SourceSentence { .. }));
    let mut flattened = Vec::new();
    let mut source_segments = Vec::new();
    for effect in effects {
        match effect {
            EffectAst::SourceSentence {
                mut effects,
                leading_then,
                starting_with_controller,
            } => {
                if starting_with_controller {
                    // Source-sentence grouping may be flattened when later
                    // vote-option sentences must lower with the vote start.
                    // Retain the ordering on the typed vote itself so that
                    // semantic execution and rendering do not depend on the
                    // provenance wrapper surviving that cross-sentence join.
                    preserve_participant_order_on_first_effect(&mut effects);
                }
                source_segments.push(SourceSentenceSegment {
                    effect_count: effects.len(),
                    leading_then,
                    starting_with_controller,
                });
                flattened.extend(effects);
            }
            effect => flattened.push(effect),
        }
    }

    if correlate_conditional_quantified_choice_followups(&mut flattened) {
        // The consumer is authored in the next sentence but is semantically
        // branch-local because it uses a collection produced only by that
        // conditional choice. Lower the correlated branch as one unit.
        return (flattened, Vec::new());
    }

    if all_source_sentences
        && source_segments.len() > 1
        && source_segments
            .iter()
            .all(|segment| segment.effect_count > 0)
    {
        let mut boundary = 0usize;
        let mut splits_shared_lowering_followup = false;
        let mut splits_hand_pipeline = false;
        let mut continues_repeat_process = false;
        for segment in &source_segments[..source_segments.len() - 1] {
            boundary += segment.effect_count;
            splits_shared_lowering_followup |= flattened
                .get(boundary)
                .is_some_and(source_sentence_followup_requires_shared_lowering);
            continues_repeat_process |=
                source_sentence_boundary_continues_repeat_process(&flattened, boundary);
            splits_hand_pipeline |=
                source_sentence_boundary_splits_hand_pipeline(&flattened, boundary);
        }
        if continues_repeat_process {
            // Repeat-process normalization needs the preceding process body
            // and its trailing conditional in the same AST slice. Re-run it
            // only for the exact cross-sentence continuation shape.
            return (normalize_effects_ast(&flattened), Vec::new());
        }
        if splits_hand_pipeline {
            // Hand producers, choices, and dependent moves form a typed tag
            // pipeline. Keep that pipeline in one lowering slice so its
            // specialist can see the complete operation.
            return (flattened, Vec::new());
        }
        if splits_shared_lowering_followup {
            // Correlated participant and vote followups are deliberately
            // lowered together with their preceding clause. Keep the
            // flattened program in one lowering slice rather than turning a
            // followup into an orphan at a source-sentence segment boundary.
            return (flattened, Vec::new());
        }
        return (flattened, source_segments);
    }

    (flattened, Vec::new())
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
        None,
        false,
    )
}

pub(crate) fn rewrite_prepare_additional_cost_effects_for_lowering(
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
            force_export_last_memory_effect_id: true,
            ..Default::default()
        },
        None,
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
    let mut normalized = normalize_effects_ast(effects);
    if let Some(trigger) = trigger {
        preserve_copy_reference_kind_from_trigger(&mut normalized, trigger);
        // A stack retarget can never act on the source permanent, so an
        // "it"/"that spell" reference inside its clause always means the
        // triggering stack object — even when an earlier body sentence
        // re-seeded the object antecedent to the source ("this creature gets
        // +2/+2 until end of turn. You may choose new targets for that
        // spell." — Speedball).
        if trigger_provides_stack_object(trigger) {
            bind_stack_retargets_to_triggering_object(&mut normalized);
        }
    }
    if effects_have_creature_death_gate(&normalized) {
        replace_creature_death_event_amounts(&mut normalized);
    }
    if let Some(antecedent_tag) = trigger.and_then(default_trigger_last_object_tag) {
        bind_trigger_antecedent_after_top_library_observation(
            &mut normalized,
            &crate::tag::TagKey::from(antecedent_tag),
        );
    }
    carry_all_object_sweep_filter_to_it_followups(&mut normalized);
    let has_local_target_prelude = has_local_target_prelude_before_it_reference(&normalized);
    let has_phase_step_it_prelude =
        has_local_target_prelude || has_prior_effect_before_it_reference(&normalized);
    if let Some(trigger) = trigger
        && phase_step_trigger_has_no_object_reference(trigger)
        && !has_phase_step_it_prelude
    {
        retarget_phase_step_it_targets_to_source(&mut normalized);
    }
    if let Some(trigger) = trigger
        && spell_cast_trigger_targets_source(trigger)
        && !has_phase_step_it_prelude
    {
        retarget_phase_step_it_targets_to_source(&mut normalized);
    }
    let references_trigger_event_tag = trigger
        .and_then(default_trigger_last_object_tag)
        .is_some_and(|tag| effects_reference_tag(&normalized, tag));
    let default_last_object_tag = if imports.last_object_tag.is_none()
        && !has_local_target_prelude
        && (effects_reference_it_tag(&normalized)
            || effects_reference_its_controller(&normalized)
            || references_trigger_event_tag)
    {
        trigger
            .and_then(default_trigger_last_object_tag)
            .map(crate::cards::builders::TagKey::from)
    } else {
        None
    };
    let default_last_object_prelude = default_last_object_tag.as_ref().and_then(|tag| {
        trigger.and_then(|trigger| default_trigger_last_object_prelude(trigger, tag))
    });

    rewrite_prepare_effects_from_normalized(
        normalized.clone(),
        &normalized,
        imports,
        EffectReferenceResolutionConfig {
            allow_life_event_value: trigger.is_some_and(trigger_allows_event_derived_life_value)
                || effects_have_creature_death_gate(&normalized),
            ..Default::default()
        },
        trigger.and_then(inferred_trigger_player_filter),
        default_last_object_tag,
        default_last_object_prelude,
        trigger.is_some(),
    )
}

pub(crate) fn rewrite_prepare_triggered_effects_for_lowering(
    trigger: TriggerSpec,
    effects: &[EffectAst],
    imports: impl Into<ReferenceImports>,
) -> Result<(TriggerSpec, PreparedTriggeredEffectsForLowering), CardTextError> {
    fn merge_intervening_predicates(
        left: Option<PredicateAst>,
        right: Option<PredicateAst>,
    ) -> Option<PredicateAst> {
        match (left, right) {
            (Some(left), Some(right)) => Some(PredicateAst::And(Box::new(left), Box::new(right))),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            (None, None) => None,
        }
    }

    fn predicate_can_promote_to_intervening_if(predicate: &PredicateAst) -> bool {
        match predicate {
            PredicateAst::TargetMatches(_) => false,
            PredicateAst::CountParity { .. } => false,
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                predicate_can_promote_to_intervening_if(left)
                    && predicate_can_promote_to_intervening_if(right)
            }
            _ => true,
        }
    }

    fn trigger_object_is_stack_object(trigger: &TriggerSpec) -> bool {
        match trigger {
            TriggerSpec::WithIntro { trigger, .. } => trigger_object_is_stack_object(trigger),
            TriggerSpec::SpellCast { .. } | TriggerSpec::NthSpellOfTurnCast { .. } => true,
            _ => false,
        }
    }

    /// An "it" predicate that checks battlefield-only state (tapped,
    /// attacking, ...) cannot refer to a stack object; when the trigger's
    /// implicit object is a spell, the pronoun must bind to an object
    /// introduced by the body effects instead, so the condition resolves with
    /// the effect rather than gating the trigger.
    fn predicate_requires_battlefield_state(predicate: &PredicateAst) -> bool {
        match predicate {
            PredicateAst::ItMatches(filter) | PredicateAst::ItMatchedLastKnown(filter) => {
                filter.tapped
                    || filter.untapped
                    || filter.attacking
                    || filter.nonattacking
                    || filter.blocking
                    || filter.nonblocking
                    || filter.blocked
                    || filter.unblocked
            }
            PredicateAst::Not(inner) => predicate_requires_battlefield_state(inner),
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                predicate_requires_battlefield_state(left)
                    || predicate_requires_battlefield_state(right)
            }
            _ => false,
        }
    }

    fn is_win_the_game_effect(effects: &[EffectAst]) -> bool {
        matches!(
            effects,
            [EffectAst::SubjectVerb(subject_verb)]
                if matches!(subject_verb.action, SubjectVerbActionAst::WinGame)
        )
    }

    fn extract_exact_other_attack_predicate(
        predicate: PredicateAst,
    ) -> (Option<u32>, Option<PredicateAst>) {
        match predicate {
            PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count) => {
                (Some(count), None)
            }
            PredicateAst::And(left, right) => {
                let (left_count, left_remainder) = extract_exact_other_attack_predicate(*left);
                let (right_count, right_remainder) = extract_exact_other_attack_predicate(*right);
                (
                    left_count.or(right_count),
                    merge_intervening_predicates(left_remainder, right_remainder),
                )
            }
            PredicateAst::Or(left, right) => (None, Some(PredicateAst::Or(left, right))),
            other => (None, Some(other)),
        }
    }

    fn predicate_uses_implicit_object_reference(predicate: &PredicateAst) -> bool {
        match predicate {
            PredicateAst::ItIsLandCard
            | PredicateAst::ItIsSoulbondPaired
            | PredicateAst::ItMatches(_)
            | PredicateAst::ItMatchedLastKnown(_)
            | PredicateAst::TargetMatches(_) => true,
            PredicateAst::Not(inner) => predicate_uses_implicit_object_reference(inner),
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                predicate_uses_implicit_object_reference(left)
                    || predicate_uses_implicit_object_reference(right)
            }
            _ => false,
        }
    }

    let mut imports = imports.into();
    let mut trigger = trigger;
    ensure_concrete_trigger_spec(&trigger)?;

    let mut normalized = normalize_effects_ast(effects);
    if std::env::var("IRONSMITH_CHOICE_TRACE").is_ok() {
        let variant = format!("{trigger:?}");
        eprintln!(
            "prepare-triggered: trigger_stack={} effects={} variant={}",
            trigger_provides_stack_object(&trigger),
            normalized.len(),
            &variant[..variant.len().min(120)]
        );
    }
    preserve_copy_reference_kind_from_trigger(&mut normalized, &trigger);
    // "You may choose new targets for that spell" after a body sentence about
    // the source must still bind the TRIGGERING stack object (Speedball).
    if trigger_provides_stack_object(&trigger) {
        bind_stack_retargets_to_triggering_object(&mut normalized);
    }
    if let Some(antecedent_tag) = default_trigger_last_object_tag(&trigger) {
        bind_trigger_antecedent_after_top_library_observation(
            &mut normalized,
            &crate::tag::TagKey::from(antecedent_tag),
        );
    }
    carry_all_object_sweep_filter_to_it_followups(&mut normalized);
    let has_local_target_prelude = has_local_target_prelude_before_it_reference(&normalized);
    let has_phase_step_it_prelude =
        has_local_target_prelude || has_prior_effect_before_it_reference(&normalized);
    let mut body_effects = normalized.clone();
    let mut intervening_if = match &trigger {
        TriggerSpec::WithIntro { trigger, .. } => match &**trigger {
            TriggerSpec::StateBased { condition, .. } => Some(condition.clone()),
            _ => None,
        },
        TriggerSpec::StateBased { condition, .. } => Some(condition.clone()),
        _ => None,
    };
    if normalized.len() == 1
        && let EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } = &normalized[0]
        && if_false.is_empty()
        && !if_true.is_empty()
        && predicate_can_promote_to_intervening_if(predicate)
        && !(trigger_object_is_stack_object(&trigger)
            && predicate_requires_battlefield_state(predicate))
        // "Whenever this attacks, you win the game if ..." checks on resolution;
        // it is not an intervening-if trigger gate.
        && !(
            (
                matches!(trigger, TriggerSpec::ThisAttacks)
                    || matches!(
                        trigger,
                        TriggerSpec::WithIntro { ref trigger, .. }
                            if matches!(**trigger, TriggerSpec::ThisAttacks)
                    )
            )
                && is_win_the_game_effect(if_true)
        )
    {
        body_effects = if_true.clone();
        intervening_if = merge_intervening_predicates(intervening_if, Some(predicate.clone()));
    }
    if let Some(predicate) = intervening_if.take() {
        intervening_if = Some(retarget_spell_cast_mana_spent_predicate(
            &trigger, predicate,
        ));
    }
    retarget_spell_cast_mana_spent_predicates_in_effects(&trigger, &mut body_effects);
    if discard_one_or_more_trigger_uses_event_count(&trigger) {
        for effect in &mut body_effects {
            replace_it_count_with_event_count(effect);
        }
    }
    if counter_removed_this_way_trigger_uses_event_count(&trigger) {
        for effect in &mut body_effects {
            preserve_counter_removed_this_way_damage_amount(effect);
        }
    }
    if death_trigger_counts_counters_on_triggering_object(&trigger) {
        for effect in &mut body_effects {
            replace_exile_top_event_count_with_triggering_counter_count(effect);
        }
    }
    if intervening_if
        .as_ref()
        .is_some_and(predicate_counts_creature_deaths)
    {
        replace_creature_death_event_amounts(&mut body_effects);
    }
    imports.source_object_antecedent |= intervening_if
        .as_ref()
        .is_some_and(PredicateAst::establishes_source_object_antecedent);
    if let Some(antecedent) = intervening_if
        .as_ref()
        .and_then(predicate_object_filter_antecedent)
    {
        bind_condition_antecedent_in_effects(
            &mut body_effects,
            &antecedent,
            ConditionAntecedentBinding::TaggedItOnly,
        );
    }
    if let Some(predicate) = intervening_if.as_ref() {
        bind_condition_collection_antecedent_in_effects(&mut body_effects, predicate);
        bind_random_count_condition_antecedent_in_effects(&mut body_effects, predicate);
    }
    retarget_source_damage_attack_followups_to_source(&mut body_effects);
    if let Some(counter_type) = intervening_if
        .as_ref()
        .and_then(predicate_source_counter_antecedent)
    {
        bind_condition_counter_antecedent_in_effects(&mut body_effects, counter_type);
    }
    if phase_step_trigger_has_no_object_reference(&trigger) && !has_phase_step_it_prelude {
        retarget_phase_step_it_targets_to_source(&mut body_effects);
    }
    if spell_cast_trigger_targets_source(&trigger) && !has_phase_step_it_prelude {
        retarget_phase_step_it_targets_to_source(&mut body_effects);
    }

    if intervening_if
        .as_ref()
        .is_some_and(PredicateAst::establishes_source_object_antecedent)
    {
        retarget_it_animations_to_source(&mut body_effects);
    }

    if (matches!(trigger, TriggerSpec::ThisAttacks)
        || matches!(
            trigger,
            TriggerSpec::WithIntro { ref trigger, .. }
                if matches!(**trigger, TriggerSpec::ThisAttacks)
        ))
        && let Some(predicate) = intervening_if.take()
    {
        let (exact_other_count, remainder) = extract_exact_other_attack_predicate(predicate);
        intervening_if = remainder;
        if let Some(other_count) = exact_other_count {
            trigger = TriggerSpec::ThisAttacksWithExactlyNOthers(other_count);
        }
    }

    let intervening_if_uses_trigger_object = intervening_if
        .as_ref()
        .is_some_and(predicate_uses_implicit_object_reference);
    // A battlefield-state "it" condition in the body can never denote a
    // stack-object trigger subject; leave the pronoun for the body's own
    // target introduction instead of pre-binding it to the triggering spell.
    let body_it_binds_to_body_target = trigger_object_is_stack_object(&trigger)
        && matches!(
            normalized.as_slice(),
            [EffectAst::Conditional { predicate, .. }]
                if predicate_requires_battlefield_state(predicate)
        );
    let references_trigger_event_tag = default_trigger_last_object_tag(&trigger)
        .is_some_and(|tag| effects_reference_tag(&normalized, tag));
    let (default_last_object_tag, default_last_object_prelude) = if !has_local_target_prelude
        && !body_it_binds_to_body_target
        && (effects_reference_it_tag(&normalized)
            || effects_reference_its_controller(&normalized)
            || intervening_if_uses_trigger_object
            || references_trigger_event_tag)
    {
        let default_tag = if matches!(&trigger, TriggerSpec::ThisAttacksWithExactlyNOthers(1))
            || matches!(
                &trigger,
                TriggerSpec::WithIntro { trigger, .. }
                    if matches!(**trigger, TriggerSpec::ThisAttacksWithExactlyNOthers(1))
            ) {
            // Exact single-partner attack triggers can bind "that creature"
            // to the other attacker snapshot captured at trigger time.
            Some("other_attacker")
        } else {
            default_trigger_last_object_tag(&trigger)
        };
        let default_tag = default_tag.map(crate::cards::builders::TagKey::from);
        let default_prelude = default_tag
            .as_ref()
            .and_then(|tag| default_trigger_last_object_prelude(&trigger, tag));
        (default_tag, default_prelude)
    } else {
        (None, None)
    };

    let allow_life_event_value = trigger_allows_event_derived_life_value(&trigger)
        || intervening_if
            .as_ref()
            .is_some_and(predicate_counts_creature_deaths);
    let prepared = rewrite_prepare_effects_from_normalized(
        body_effects,
        &normalized,
        imports,
        EffectReferenceResolutionConfig {
            allow_life_event_value,
            ..Default::default()
        },
        inferred_trigger_player_filter(&trigger),
        default_last_object_tag,
        default_last_object_prelude,
        true,
    )?;

    let intervening_if = intervening_if.map(|predicate| PreparedPredicateForLowering {
        predicate,
        reference_env: prepared.initial_env.clone(),
        saved_last_object_tag: prepared.imports.last_object_tag.clone(),
    });

    Ok((
        trigger,
        PreparedTriggeredEffectsForLowering {
            prepared,
            intervening_if,
        },
    ))
}

pub(crate) fn rewrite_lower_prepared_statement_effects(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    let mut lowered = materialize_prepared_statement_effects(prepared)?;
    super::battlefield_entry_counter_fusion::fuse_program(&mut lowered.effects);
    Ok(lowered)
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
            source_text: option.description.trim().to_string(),
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

    if let AbilityKind::Activated(activated) = parsed.kind()
        && (!activated.effects.is_empty() || !activated.choices.is_empty())
    {
        return Ok(None);
    }
    if let AbilityKind::Triggered(triggered) = parsed.kind()
        && (!triggered.effects.is_empty() || !triggered.choices.is_empty())
    {
        return Ok(None);
    }

    Ok(match (parsed.kind(), parsed.trigger_spec.as_ref()) {
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

    let AbilityKind::Activated(activated) = parsed.kind_mut() else {
        if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
            if !triggered.effects.is_empty() || !triggered.choices.is_empty() {
                return Ok(parsed);
            }
            let Some(NormalizedPreparedAbility::Triggered { trigger, prepared }) = prepared else {
                return Ok(parsed);
            };
            let (mut lowered, parsed_intervening_if) =
                materialize_prepared_triggered_effects(&prepared)?;
            rewrite_validate_iterated_player_bindings_in_lowered_effects(
                &lowered,
                trigger_binds_player_reference_context(&trigger),
                "triggered ability effects",
            )?;
            let intervening_if = rewrite_merge_intervening_conditions(
                triggered.intervening_if.take(),
                parsed_intervening_if,
            );
            let intervening_if = intervening_if
                .map(|condition| retarget_spell_cast_mana_spent_condition(&trigger, condition));
            if let Some(condition) = intervening_if.as_ref() {
                retarget_source_move_to_damaged_death_card(&mut lowered, condition);
            }
            rewrite_source_control_loss_sacrifice_followup(&mut lowered);
            triggered.trigger = compile_trigger_spec(trigger);
            triggered.effects = lowered.effects;
            triggered.choices = lowered.choices;
            triggered.intervening_if = intervening_if;
            return Ok(parsed);
        }
        return Ok(parsed);
    };

    if !activated.effects.is_empty() || !activated.choices.is_empty() {
        mark_activated_mana_output_if_needed(activated);
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
    mark_activated_mana_output_if_needed(activated);
    Ok(parsed)
}

fn mark_activated_mana_output_if_needed(activated: &mut crate::ability::ActivatedAbility) {
    if activated.mana_output.is_none() && resolution_program_produces_mana(&activated.effects) {
        activated.mana_output = Some(vec![]);
    }
}

fn resolution_program_produces_mana(program: &crate::resolution::ResolutionProgram) -> bool {
    program
        .flattened_default_effects()
        .iter()
        .any(effect_produces_mana)
}

fn effect_produces_mana(effect: &crate::effect::Effect) -> bool {
    effect.contains_mana_production()
}

pub(crate) fn rewrite_lower_parsed_ability(
    parsed: ParsedAbility,
) -> Result<ParsedAbility, CardTextError> {
    stacker::maybe_grow(1024 * 1024, 2 * 1024 * 1024, || {
        rewrite_lower_parsed_ability_internal(parsed, None)
    })
}

pub(crate) fn rewrite_lower_prepared_ability(
    normalized: NormalizedParsedAbility,
) -> Result<ParsedAbility, CardTextError> {
    stacker::maybe_grow(1024 * 1024, 2 * 1024 * 1024, || {
        rewrite_lower_parsed_ability_internal(normalized.parsed, normalized.prepared)
    })
}

pub(crate) fn rewrite_apply_instead_followup_statement_to_last_ability(
    builder: &mut CardDefinitionBuilder,
    last_restrictable_ability: Option<usize>,
    effects: &[EffectAst],
) -> Result<bool, CardTextError> {
    let Some(index) = last_restrictable_ability else {
        return Ok(false);
    };
    if index >= builder.abilities.len() {
        return Ok(false);
    }

    if !effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SelfReplacement {
                attach_to_previous_ability: true,
                ..
            }
        )
    }) {
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

pub(crate) fn rewrite_apply_delayed_trigger_followup_statement_to_last_ability(
    builder: &mut CardDefinitionBuilder,
    last_restrictable_ability: Option<usize>,
    effects: &[EffectAst],
) -> Result<bool, CardTextError> {
    let Some(index) = last_restrictable_ability else {
        return Ok(false);
    };
    if index >= builder.abilities.len() {
        return Ok(false);
    }

    if !effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::DelayedTriggerThisTurn {
                attach_to_previous_ability: true,
                ..
            }
        )
    }) {
        return Ok(false);
    }

    let AbilityKind::Triggered(triggered) = &mut builder.abilities[index].kind else {
        return Ok(false);
    };
    if triggered.choices.is_empty() {
        return Ok(false);
    }

    let prepared = rewrite_prepare_effects_for_lowering(
        effects,
        ReferenceImports::with_last_object_tag("targeted_0"),
    )?;
    let compiled = rewrite_lower_prepared_statement_effects(&prepared)?;
    if compiled.effects.is_empty() {
        return Ok(false);
    }

    for segment in compiled.effects.segments {
        triggered.effects.push_segment(segment);
    }

    Ok(true)
}

pub(crate) fn rewrite_parsed_triggered_ability(
    trigger: TriggerSpec,
    effects_ast: Vec<EffectAst>,
    functional_zones: Vec<Zone>,
    text: Option<String>,
    intervening_if: Option<crate::ConditionExpr>,
    presentation_label: Option<&crate::ability::PresentationLabel>,
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
                presentation_label: presentation_label.cloned(),
            }),
            functional_zones,
        }
        .into(),
        text,
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
        KeywordAction::Decayed => Some(StaticAbility::cant_block()),
        KeywordAction::Flash => Some(StaticAbility::flash()),
        KeywordAction::Phasing => Some(StaticAbility::phasing()),
        KeywordAction::Indestructible => Some(StaticAbility::indestructible()),
        KeywordAction::Shroud => Some(StaticAbility::shroud()),
        KeywordAction::Daybound => Some(StaticAbility::daybound()),
        KeywordAction::Nightbound => Some(StaticAbility::nightbound()),
        KeywordAction::Ward(amount) => u8::try_from(amount).ok().map(|generic| {
            StaticAbility::ward(crate::cost::TotalCost::mana(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(generic),
            ])))
        }),
        KeywordAction::Wither => Some(StaticAbility::wither()),
        KeywordAction::Afflict(_) => None,
        KeywordAction::Amplify(_) => None,
        KeywordAction::Afterlife(_) | KeywordAction::Fabricate(_) => None,
        KeywordAction::Infect => Some(StaticAbility::infect()),
        KeywordAction::Undying
        | KeywordAction::Persist
        | KeywordAction::Prowess
        | KeywordAction::Exalted => None,
        KeywordAction::Cascade => Some(StaticAbility::cascade()),
        KeywordAction::Storm
        | KeywordAction::Gravestorm
        | KeywordAction::Toxic(_)
        | KeywordAction::Poisonous(_)
        | KeywordAction::BattleCry
        | KeywordAction::Dethrone
        | KeywordAction::Evolve
        | KeywordAction::Ingest
        | KeywordAction::Mentor => None,
        KeywordAction::Skulk => Some(StaticAbility::skulk()),
        KeywordAction::Training | KeywordAction::Riot => None,
        KeywordAction::Unleash => Some(StaticAbility::unleash()),
        KeywordAction::Renown(_)
        | KeywordAction::Modular(_)
        | KeywordAction::Graft(_)
        | KeywordAction::Soulbond
        | KeywordAction::Soulshift(_)
        | KeywordAction::SoulshiftValue(_)
        | KeywordAction::Outlast(_)
        | KeywordAction::Unearth(_)
        | KeywordAction::Eternalize(_)
        | KeywordAction::Ninjutsu(_)
        | KeywordAction::Extort => None,
        KeywordAction::Partner => Some(StaticAbility::partner()),
        KeywordAction::StartYourEngines => Some(StaticAbility::start_your_engines()),
        KeywordAction::Assist => Some(StaticAbility::assist()),
        KeywordAction::SplitSecond => Some(StaticAbility::split_second()),
        KeywordAction::Rebound => Some(StaticAbility::rebound()),
        KeywordAction::Sunburst => None,
        KeywordAction::ReadAhead => Some(StaticAbility::read_ahead()),
        KeywordAction::Firebending(_)
        | KeywordAction::FirebendingValue { .. }
        | KeywordAction::Fading(_)
        | KeywordAction::Vanishing(_) => None,
        KeywordAction::Fear => Some(StaticAbility::fear()),
        KeywordAction::Intimidate => Some(StaticAbility::intimidate()),
        KeywordAction::Shadow => Some(StaticAbility::shadow()),
        KeywordAction::Horsemanship => Some(StaticAbility::horsemanship()),
        KeywordAction::Flanking => Some(StaticAbility::flanking()),
        KeywordAction::UmbraArmor => Some(StaticAbility::umbra_armor()),
        KeywordAction::Landwalk(kind) => Some(match kind {
            crate::static_abilities::LandwalkKind::Subtype {
                subtype,
                snow: false,
            } => StaticAbility::landwalk(subtype),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype,
                snow: true,
            } => StaticAbility::snow_landwalk(subtype),
            crate::static_abilities::LandwalkKind::AnyLand => StaticAbility::any_landwalk(),
            crate::static_abilities::LandwalkKind::NonbasicLand => {
                StaticAbility::nonbasic_landwalk()
            }
            crate::static_abilities::LandwalkKind::ArtifactLand => {
                StaticAbility::artifact_landwalk()
            }
        }),
        KeywordAction::Bloodthirst(amount) => Some(StaticAbility::bloodthirst(amount)),
        KeywordAction::Tribute(amount) => Some(StaticAbility::tribute(amount)),
        KeywordAction::Rampage(_) | KeywordAction::Bushido(_) | KeywordAction::Frenzy(_) => None,
        KeywordAction::Changeling => Some(StaticAbility::changeling()),
        KeywordAction::HexproofFrom(filter) => Some(StaticAbility::hexproof_from(filter.clone())),
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
        KeywordAction::ProtectionFromChosenColor => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::ChosenColor,
        )),
        KeywordAction::ProtectionFromFilter(filter) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(filter),
        )),
        KeywordAction::ProtectionFromEachManaValueAmong(filter) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::EachManaValueAmong(filter),
        )),
        KeywordAction::ProtectionFromCardType(card_type) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        )),
        KeywordAction::ProtectionFromSubtype(subtype) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Permanents(
                ObjectFilter::default().with_subtype(subtype),
            ),
        )),
        KeywordAction::Unblockable => Some(StaticAbility::unblockable()),
        KeywordAction::Devoid => Some(StaticAbility::make_colorless(ObjectFilter::source())),
        KeywordAction::Annihilator(_) => None,
        KeywordAction::Dredge(amount) => Some(StaticAbility::dredge(amount)),
        KeywordAction::StaticMarker(name) => Some(StaticAbility::keyword_marker(name)),
        KeywordAction::StaticMarkerText(text) => Some(StaticAbility::keyword_marker(text)),
        KeywordAction::Marker(name) => Some(StaticAbility::keyword_fallback_text(name)),
        KeywordAction::MarkerText(text) => Some(StaticAbility::keyword_fallback_text(text)),
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

pub(crate) fn rewrite_lower_keyword_action_to_object_abilities(
    action: KeywordAction,
) -> Result<Vec<Ability>, CardTextError> {
    if let Some(abilities) = executable_object_abilities_for_keyword_action(&action) {
        return Ok(abilities);
    }
    Ok(vec![Ability::static_ability(
        rewrite_lower_keyword_action_or_err(action)?,
    )])
}

fn rewrite_object_abilities_grant(
    filter: ObjectFilter,
    abilities: Vec<Ability>,
    display: String,
    condition: Option<crate::ConditionExpr>,
) -> Result<StaticAbility, CardTextError> {
    let mut abilities = abilities.into_iter();
    let first = abilities.next().ok_or_else(|| {
        CardTextError::InvariantViolation("keyword grant produced no abilities".to_string())
    })?;
    let mut grant =
        crate::static_abilities::GrantObjectAbilityForFilter::new(filter, first, display)
            .with_additional_abilities(abilities.collect());
    if let Some(condition) = condition {
        grant = grant.with_condition(condition);
    }
    Ok(StaticAbility::new(grant))
}

fn direct_named_granting_source_spec(effect: &Effect) -> Option<ChooseSpec> {
    if let Some(spec) = effect.target_spec()
        && matches!(spec.base(), ChooseSpec::Source)
        && matches!(
            spec.source_reference_surface(),
            Some(SourceReferenceSurface::FullName(_) | SourceReferenceSurface::ShortName(_))
        )
    {
        return Some(spec.clone());
    }

    None
}

fn preserve_named_granting_source_in_effect(effect: Effect) -> Effect {
    // Keep source rebinding as narrow as the runtime composition permits.
    // A quoted ability may refer both to its granting Aura by proper name and
    // to `this creature`, meaning the object that received the ability. If a
    // coordinated sequence were rebound wholesale, both references would
    // incorrectly resolve to the Aura.
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut sequence = sequence.clone();
        sequence.effects = sequence
            .effects
            .into_iter()
            .map(preserve_named_granting_source_in_effect)
            .collect();
        return Effect::new(sequence);
    }

    let Some(source) = direct_named_granting_source_spec(&effect) else {
        return effect;
    };
    Effect::new(crate::effects::ExecuteWithSourceEffect::new(source, effect))
}

/// A proper-name reference inside a quoted attached-object ability names the
/// granting attachment, not the object receiving the ability. Keep that
/// distinction through lowering by executing the source sentence with the
/// named source. `AttachedAbilityGrant` materializes this marker to the
/// concrete attachment object when it generates its continuous effect.
fn preserve_named_granting_source(mut ability: Ability) -> Ability {
    fn wrap_effects(effects: &mut [Effect]) {
        for effect in effects {
            *effect = preserve_named_granting_source_in_effect(effect.clone());
        }
    }

    let program = match &mut ability.kind {
        AbilityKind::Triggered(triggered) => &mut triggered.effects,
        AbilityKind::Activated(activated) => &mut activated.effects,
        AbilityKind::Static(_) => return ability,
    };
    let mut segments = std::mem::take(&mut program.segments);
    for segment in &mut segments {
        wrap_effects(&mut segment.default_effects);
        for replacement in &mut segment.self_replacements {
            wrap_effects(&mut replacement.replacement_effects);
        }
    }
    *program = crate::resolution::ResolutionProgram::new(segments);
    ability
}

fn rewrite_attached_object_abilities_grant(
    abilities: Vec<Ability>,
    display: String,
    condition: Option<crate::ConditionExpr>,
) -> Result<StaticAbility, CardTextError> {
    let mut abilities = abilities.into_iter().map(preserve_named_granting_source);
    let first = abilities.next().ok_or_else(|| {
        CardTextError::InvariantViolation(
            "attached keyword grant produced no abilities".to_string(),
        )
    })?;
    let mut grant = crate::static_abilities::AttachedAbilityGrant::new(first, display)
        .with_additional_abilities(abilities.collect());
    if let Some(condition) = condition {
        grant = grant.with_condition(condition);
    }
    Ok(StaticAbility::new(grant))
}

fn rewrite_lower_attached_keyword_action_grant(
    action: KeywordAction,
    display: String,
    condition: Option<crate::ConditionExpr>,
) -> Result<StaticAbility, CardTextError> {
    rewrite_attached_object_abilities_grant(
        rewrite_lower_keyword_action_to_object_abilities(action)?,
        display,
        condition,
    )
}

fn rewrite_lower_conditional_static_ability(
    ability: StaticAbilityAst,
    condition: crate::ConditionExpr,
) -> Result<StaticAbility, CardTextError> {
    if let StaticAbilityAst::KeywordAction(action) = ability {
        let display = action.display_text();
        return rewrite_object_abilities_grant(
            ObjectFilter::source(),
            rewrite_lower_keyword_action_to_object_abilities(action)?,
            display,
            Some(condition),
        );
    }
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
    if let StaticAbilityAst::KeywordAction(action) = ability {
        let display = action.display_text();
        return rewrite_object_abilities_grant(
            filter,
            rewrite_lower_keyword_action_to_object_abilities(action)?,
            display,
            condition,
        );
    }

    let mut grant = crate::static_abilities::GrantAbility::new(
        filter,
        rewrite_lower_static_ability_ast(ability)?.into(),
    );
    if let Some(condition) = condition {
        grant = grant.with_condition(condition);
    }
    Ok(StaticAbility::new(grant))
}

fn rewrite_lower_static_set_quantifier_surface(
    ability: StaticAbilityAst,
    surface: ironsmith_core::SetQuantifierSurface,
) -> Result<StaticAbility, CardTextError> {
    let mut lowered = rewrite_lower_static_ability_ast(ability)?;
    match &mut lowered.payload {
        crate::static_abilities::StaticAbilityPayload::GrantAbility(grant) => {
            grant.set_quantifier_surface = Some(surface);
        }
        crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
            grant.set_quantifier_surface = Some(surface);
        }
        _ => {
            return Err(CardTextError::InvariantViolation(
                "set-quantifier surface wrapper requires a filter-wide granted ability".to_string(),
            ));
        }
    }
    Ok(lowered)
}

fn rewrite_lower_attached_static_ability_grant(
    ability: StaticAbilityAst,
    display: String,
    condition: Option<crate::ConditionExpr>,
) -> Result<StaticAbility, CardTextError> {
    if let StaticAbilityAst::KeywordAction(action) = ability {
        return rewrite_attached_object_abilities_grant(
            rewrite_lower_keyword_action_to_object_abilities(action)?,
            display,
            condition,
        );
    }

    let granted = Ability::static_ability(rewrite_lower_static_ability_ast(ability)?);
    let mut grant = crate::static_abilities::AttachedAbilityGrant::new(granted, display);
    if let Some(condition) = condition {
        grant = grant.with_condition(condition);
    }
    Ok(StaticAbility::new(grant))
}

fn rewrite_lower_attached_chosen_landwalk_grant(
    display: String,
    snow: bool,
    condition: Option<crate::ConditionExpr>,
) -> Result<StaticAbility, CardTextError> {
    let mut grant = crate::static_abilities::AttachedChosenLandwalkGrant::new(display, snow);
    if let Some(condition) = condition {
        grant = grant.with_condition(condition);
    }
    Ok(StaticAbility::new(grant))
}

fn rewrite_lower_pregame_reveal_from_opening_hand(
    trigger: TriggerSpec,
    effects: Vec<EffectAst>,
    one_shot: bool,
    first_spell_of_game: bool,
    effect_before_timing: bool,
    display: String,
) -> Result<StaticAbility, CardTextError> {
    let (effects, choices) = compile_trigger_effects(Some(&trigger), &effects)?;
    if !choices.is_empty() {
        return Err(CardTextError::InvariantViolation(
            "opening-hand delayed consequences cannot require choices before the game begins"
                .to_string(),
        ));
    }
    let mut delayed_trigger = compile_delayed_trigger_spec(&trigger)?;
    if first_spell_of_game {
        let ironsmith_core::DelayedTriggerSpec::SpellCast {
            first_spell_of_game,
            ..
        } = &mut delayed_trigger
        else {
            return Err(CardTextError::InvariantViolation(
                "first-spell-of-game pregame consequence requires a spell-cast trigger".to_string(),
            ));
        };
        *first_spell_of_game = true;
    }

    let schedule = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
        delayed_trigger,
        effects,
        one_shot,
        Vec::new(),
        PlayerFilter::You,
    ));
    Ok(StaticAbility::pregame_action_with_effects(
        crate::static_abilities::PregameActionKind::RevealFromOpeningHand(
            crate::static_abilities::PregameRevealFromOpeningHandSpec {
                effect_before_timing,
            },
        ),
        display,
        vec![schedule],
    ))
}

pub(crate) fn rewrite_lower_static_ability_ast(
    ability: StaticAbilityAst,
) -> Result<StaticAbility, CardTextError> {
    match ability {
        StaticAbilityAst::Static(ability) => Ok(ability),
        StaticAbilityAst::KeywordAction(action) => {
            if executable_object_abilities_for_keyword_action(&action).is_some()
                || matches!(
                    action,
                    KeywordAction::Firebending(_) | KeywordAction::FirebendingValue { .. }
                )
            {
                let display = action.display_text();
                rewrite_object_abilities_grant(
                    ObjectFilter::source(),
                    rewrite_lower_keyword_action_to_object_abilities(action)?,
                    display,
                    None,
                )
            } else {
                rewrite_lower_keyword_action_or_err(action)
            }
        }
        StaticAbilityAst::PregameRevealFromOpeningHand {
            trigger,
            effects,
            one_shot,
            first_spell_of_game,
            effect_before_timing,
            display,
        } => rewrite_lower_pregame_reveal_from_opening_hand(
            trigger,
            effects,
            one_shot,
            first_spell_of_game,
            effect_before_timing,
            display,
        ),
        StaticAbilityAst::LoseGameReplacement {
            effects,
            optional,
            display,
        } => {
            let (effects, choices) = compile_trigger_effects(None, &effects)?;
            if !choices.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "lose-game replacement effects cannot carry unresolved spell targets"
                        .to_string(),
                ));
            }
            Ok(StaticAbility::lose_game_replacement(
                effects, optional, display,
            ))
        }
        StaticAbilityAst::ConditionalStaticAbility { ability, condition } => {
            rewrite_lower_conditional_static_ability(*ability, condition)
        }
        StaticAbilityAst::LabeledConditionalStaticAbility {
            ability,
            condition,
            label,
        } => Ok(
            rewrite_lower_static_ability_ast(*ability)?.with_labeled_condition(condition, label)
        ),
        StaticAbilityAst::ConditionalKeywordAction { action, condition } => {
            rewrite_lower_conditional_static_ability(
                StaticAbilityAst::KeywordAction(action),
                condition,
            )
        }
        StaticAbilityAst::WithSetQuantifierSurface { ability, surface } => {
            rewrite_lower_static_set_quantifier_surface(*ability, surface)
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
        StaticAbilityAst::RemoveKeywordAction {
            filter,
            action,
            mode,
        } => {
            if executable_object_abilities_for_keyword_action(&action).is_some()
                || matches!(
                    &action,
                    KeywordAction::Firebending(_) | KeywordAction::FirebendingValue { .. }
                )
            {
                let display = action.display_text();
                return Ok(StaticAbility::remove_object_abilities_with_mode(
                    filter,
                    rewrite_lower_keyword_action_to_object_abilities(action)?,
                    display,
                    mode,
                ));
            }
            Ok(StaticAbility::remove_ability_with_mode(
                filter,
                rewrite_lower_keyword_action_or_err(action)?,
                mode,
            ))
        }
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
        StaticAbilityAst::AttachedChosenLandwalkGrant {
            snow,
            display,
            condition,
        } => rewrite_lower_attached_chosen_landwalk_grant(display, snow, condition),
        StaticAbilityAst::EquipmentKeywordActionsGrant { actions } => {
            let mut lowered = Vec::new();
            let mut names = Vec::with_capacity(actions.len());
            for action in actions {
                let display = action.display_text();
                let mut name = display.clone();
                if let Some(first) = name.get(..1) {
                    name = format!("{}{}", first.to_ascii_lowercase(), &display[1..]);
                }
                names.push(name);
                lowered.extend(rewrite_lower_keyword_action_to_object_abilities(action)?);
            }
            // The printed line for a multi-keyword equipment grant is a full
            // sentence ("Equipped creature has deathtouch and lifelink."),
            // not a bare keyword header.
            let joined = match names.as_slice() {
                [] => String::new(),
                [only] => only.clone(),
                [first, second] => format!("{first} and {second}"),
                [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
            };
            rewrite_attached_object_abilities_grant(
                lowered,
                format!("Equipped creature has {joined}."),
                None,
            )
        }
        StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition,
        } => {
            let lowered = rewrite_lower_parsed_ability(ability)?.into_runtime();
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
            let lowered = rewrite_lower_parsed_ability(ability)?.into_runtime();
            rewrite_attached_object_abilities_grant(vec![lowered], display, condition)
        }
        StaticAbilityAst::SoulbondSharedObjectAbility { ability } => {
            let lowered = rewrite_lower_parsed_ability(ability)?.into_runtime();
            Ok(StaticAbility::soulbond_shared_object_ability(lowered))
        }
        StaticAbilityAst::AttachmentRestriction { .. } => Err(CardTextError::InvariantViolation(
            "attachment restrictions must be lowered through card definition state".to_string(),
        )),
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

fn rewrite_validate_unbound_iterated_player<T: std::fmt::Debug + ?Sized>(
    mentions_iterated_player: bool,
    value: &T,
    context: &str,
) -> Result<(), CardTextError> {
    if mentions_iterated_player {
        return Err(CardTextError::InvariantViolation(format!(
            "{context} references PlayerFilter::IteratedPlayer without a trigger or loop that binds \"that player\": {value:?}"
        )));
    }
    Ok(())
}

fn rewrite_validate_no_unresolved_dynamic_values<T: std::fmt::Debug + ?Sized>(
    contains_pending_effect_metric: bool,
    value: &T,
    context: &str,
) -> Result<(), CardTextError> {
    if contains_pending_effect_metric {
        return Err(CardTextError::ParseError(format!(
            "{context} contains an unresolved prior-effect metric value: {value:?}"
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
        rewrite_validate_unbound_iterated_player(
            choose_spec_mentions_iterated_player(choice),
            choice,
            context,
        )?;
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
    rewrite_validate_unbound_iterated_player(
        condition_mentions_iterated_player(condition),
        condition,
        context,
    )
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
    if !iterated_player_bound
        && let Some(skip_turn) = effect.downcast_ref::<crate::effects::SkipTurnEffect>()
        && matches!(skip_turn.player, PlayerFilter::IteratedPlayer)
    {
        return Ok(());
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return rewrite_validate_effects_for_iterated_player(
            &sequence.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>() {
        if !iterated_player_bound && let Some(decider) = &may.decider {
            rewrite_validate_unbound_iterated_player(
                decider.mentions_iterated_player(),
                decider,
                context,
            )?;
        }
        return rewrite_validate_effects_for_iterated_player(
            &may.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(unless_pays) =
        effect.downcast_ref::<crate::effects::UnlessPaysEffect<crate::effect::Effect>>()
    {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                unless_pays.player.mentions_iterated_player(),
                &unless_pays.player,
                context,
            )?;
        }
        return rewrite_validate_effects_for_iterated_player(
            &unless_pays.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(unless_action) =
        effect.downcast_ref::<crate::effects::UnlessActionEffect<crate::effect::Effect>>()
    {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                unless_action.player.mentions_iterated_player(),
                &unless_action.player,
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
    if let Some(for_players) =
        effect.downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
    {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                for_players.filter.mentions_iterated_player(),
                &for_players.filter,
                context,
            )?;
        }
        return rewrite_validate_effects_for_iterated_player(&for_players.effects, true, context);
    }
    if let Some(for_each_object) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                object_filter_mentions_iterated_player(&for_each_object.filter),
                &for_each_object.filter,
                context,
            )?;
        }
        return rewrite_validate_effects_for_iterated_player(
            &for_each_object.effects,
            true,
            context,
        );
    }
    if let Some(for_each_tagged) =
        effect.downcast_ref::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>()
    {
        return rewrite_validate_effects_for_iterated_player(
            &for_each_tagged.effects,
            true,
            context,
        );
    }
    if let Some(for_each_controller) = effect
        .downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect<crate::effect::Effect>>()
    {
        return rewrite_validate_effects_for_iterated_player(
            &for_each_controller.effects,
            true,
            context,
        );
    }
    if let Some(for_each_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect<crate::effect::Effect>>()
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
        // IfEffect can bind IteratedPlayer from PlayerCounts recorded by its antecedent.
        rewrite_validate_effects_for_iterated_player(&if_effect.then, true, context)?;
        return rewrite_validate_effects_for_iterated_player(&if_effect.else_, true, context);
    }
    if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatProcessEffect>() {
        return rewrite_validate_effects_for_iterated_player(
            &repeat.effects,
            iterated_player_bound,
            context,
        );
    }
    if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                value_mentions_iterated_player(&repeat.count),
                &repeat.count,
                context,
            )?;
        }
        return rewrite_validate_effects_for_iterated_player(
            &repeat.effects,
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
        if let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice {
            for option in options {
                rewrite_validate_effects_for_iterated_player(
                    &option.effects_per_vote,
                    true,
                    context,
                )?;
            }
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
                schedule_delayed.controller.mentions_iterated_player(),
                &schedule_delayed.controller,
                context,
            )?;
            if let Some(filter) = &schedule_delayed.target_filter {
                rewrite_validate_unbound_iterated_player(
                    object_filter_mentions_iterated_player(filter),
                    filter,
                    context,
                )?;
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
                schedule_when_leaves.controller.mentions_iterated_player(),
                &schedule_when_leaves.controller,
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
    if let Some(create_token) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        if !iterated_player_bound {
            rewrite_validate_unbound_iterated_player(
                create_token.controller.mentions_iterated_player(),
                &create_token.controller,
                context,
            )?;
            if let Some(controller_target) = &create_token.controller_target {
                rewrite_validate_unbound_iterated_player(
                    choose_spec_mentions_iterated_player(controller_target),
                    controller_target,
                    context,
                )?;
            }
            rewrite_validate_unbound_iterated_player(
                value_mentions_iterated_player(&create_token.count),
                &create_token.count,
                context,
            )?;
        }
        return rewrite_validate_card_definition_for_iterated_player(
            &create_token.token,
            "created token definition",
        );
    }

    if !iterated_player_bound {
        rewrite_validate_unbound_iterated_player(
            effect_mentions_iterated_player(effect),
            effect,
            context,
        )?;
    }
    Ok(())
}

fn rewrite_validate_ability_for_iterated_player(
    ability: &Ability,
    context: &str,
) -> Result<(), CardTextError> {
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            rewrite_validate_effects_for_iterated_player(
                triggered.effects.flattened_default_effects(),
                false,
                context,
            )?;
            rewrite_validate_choose_specs_for_iterated_player(&triggered.choices, false, context)?;
            if let Some(intervening_if) = &triggered.intervening_if {
                rewrite_validate_condition_for_iterated_player(intervening_if, false, context)?;
            }
            Ok(())
        }
        AbilityKind::Activated(activated) => {
            rewrite_validate_effects_for_iterated_player(
                activated.effects.flattened_default_effects(),
                false,
                context,
            )?;
            rewrite_validate_choose_specs_for_iterated_player(&activated.choices, false, context)?;
            for restriction in &activated.activation_restrictions {
                rewrite_validate_condition_for_iterated_player(restriction, false, context)?;
            }
            if let Some(condition) = &activated.activation_condition {
                rewrite_validate_condition_for_iterated_player(condition, false, context)?;
            }
            Ok(())
        }
        AbilityKind::Static(static_ability) => {
            // Static abilities are opaque runtime trait objects; their nested
            // triggered/activated abilities are validated through card definitions.
            let _ = (static_ability, context);
            Ok(())
        }
    }
}

fn rewrite_validate_card_definition_for_iterated_player(
    card_definition: &crate::cards::CardDefinition,
    context: &str,
) -> Result<(), CardTextError> {
    for ability in &card_definition.abilities {
        rewrite_validate_ability_for_iterated_player(ability, context)?;
    }
    if let Some(spell_effect) = &card_definition.spell_effect {
        rewrite_validate_effects_for_iterated_player(
            spell_effect.flattened_default_effects(),
            false,
            context,
        )?;
    }
    Ok(())
}

pub(crate) fn rewrite_validate_iterated_player_bindings_in_lowered_effects(
    lowered: &LoweredEffects,
    initial_iterated_player_bound: bool,
    context: &str,
) -> Result<(), CardTextError> {
    rewrite_validate_no_unresolved_dynamic_values(
        effects_contain_pending_effect_metric(&lowered.effects),
        &lowered.effects,
        context,
    )?;
    let iterated_player_bound = initial_iterated_player_bound || lowered.exports.iterated_player;
    rewrite_validate_effects_for_iterated_player(&lowered.effects, iterated_player_bound, context)?;
    rewrite_validate_choose_specs_for_iterated_player(
        &lowered.choices,
        iterated_player_bound,
        context,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn dynamic_remove_counter_cost_metrics_bind_counter_followups_to_x() {
        let query = ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::AffectedObjects,
            ironsmith_core::EffectMetric::Count,
        )
        .with_action(ironsmith_core::PriorEffectAction::Removed);
        let mut effects = vec![EffectAst::subject_verb_put_counters(
            crate::object::CounterType::PlusOnePlusOne,
            Value::PendingPriorEffectMetric(query)
                .with_surface_hint(ValueSurfaceHint::CountersRemovedThisWay),
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
            None,
            false,
        )];

        replace_pending_removed_counter_metrics_with_x(&mut effects);

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutCounters { count, .. },
            ..
        }) = &effects[0]
        else {
            panic!("expected a typed counter-placement effect");
        };
        assert_eq!(count.unhinted(), &Value::X);
        assert!(count.has_surface_hint(ValueSurfaceHint::CountersRemovedThisWay));
    }

    #[test]
    fn triggering_blocker_prelude_uses_event_identity_not_live_blocking_state() {
        let mut blocker = ObjectFilter::creature();
        blocker.blocking = true;
        let prelude = default_trigger_last_object_prelude(
            &TriggerSpec::ThisBecomesBlockedByObject(blocker),
            &TagKey::from("blocking"),
        )
        .expect("becomes-blocked trigger should capture its blocker");

        let EffectPreludeTag::TriggeringBlockers(tag, filter) = prelude else {
            panic!("expected blocker prelude, got {prelude:#?}");
        };
        assert_eq!(tag.as_str(), "blocking");
        assert!(!filter.blocking, "{filter:#?}");
    }

    #[test]
    fn coordinated_tap_does_not_replace_countered_spell_controller_actor() {
        let tokens = lex_line(
            "Counter target spell unless its controller pays {X}. If that player doesn't, they tap all lands with mana abilities they control and lose all unspent mana.",
            0,
        )
        .expect("power-sink probe should lex");
        let effects =
            crate::runtime_backend::effect_sentences::parse_effect_sentences_lexed(&tokens)
                .expect("power-sink probe should parse");
        let prepared = rewrite_prepare_effects_for_lowering(&effects, ReferenceImports::default())
            .expect("power-sink probe should prepare");
        let lowered = materialize_prepared_statement_effects(&prepared)
            .expect("power-sink probe should lower");
        let debug = format!("{:#?}", lowered.effects);

        assert!(
            debug.contains("AliasedControllerOf")
                && debug.contains("countered_0")
                && !debug.contains("AliasedControllerOf(Tagged(TagKey(\"tapped_"),
            "nonpayment actor must remain the countered spell's controller: {debug}"
        );
    }

    #[test]
    fn flattened_vote_followups_keep_starting_with_controller_order() {
        let effects = vec![
            EffectAst::SourceSentence {
                effects: vec![EffectAst::VoteStart {
                    options: vec!["time".to_string(), "money".to_string()],
                    secret: false,
                    starting_with_controller: false,
                }],
                leading_then: false,
                starting_with_controller: true,
            },
            EffectAst::SourceSentence {
                effects: vec![EffectAst::VoteOption {
                    option: "time".to_string(),
                    effects: vec![EffectAst::Sequence {
                        effects: Vec::new(),
                    }],
                }],
                leading_then: false,
                starting_with_controller: false,
            },
        ];

        let (flattened, _) = flatten_top_level_source_sentences(effects.clone());
        assert!(matches!(
            flattened.first(),
            Some(EffectAst::VoteStart {
                starting_with_controller: true,
                ..
            })
        ));

        let prepared = rewrite_prepare_effects_for_lowering(&effects, ReferenceImports::default())
            .expect("prepare vote with cross-sentence option");
        let lowered = materialize_prepared_statement_effects(&prepared)
            .expect("lower vote with cross-sentence option");
        let vote_starts_with_controller = lowered
            .effects
            .flattened_default_effects()
            .into_iter()
            .any(|effect| {
                effect
                    .downcast_ref::<crate::effects::VoteEffect>()
                    .is_some_and(|vote| vote.starting_with_controller)
            });
        assert!(
            vote_starts_with_controller,
            "typed vote lost participant order"
        );
    }

    #[test]
    fn repeated_optional_exiles_prepare_plural_return_with_aggregate_tag() {
        let tokens = lex_line(
            "Exile up to one target artifact you control and up to one target creature you control. Then return them to the battlefield under their owners' control.",
            0,
        )
        .expect("lex");
        let effects =
            crate::runtime_backend::effect_sentences::parse_effect_sentences_lexed(&tokens)
                .expect("parse");
        let prepared = rewrite_prepare_effects_for_lowering(&effects, ReferenceImports::default())
            .expect("prepare");
        let debug = format!("{:#?}", prepared.effects);

        assert!(
            debug.contains("ReturnAllToBattlefield")
                && !debug.contains(crate::tag::SOURCE_EXILED_TAG),
            "expected aggregate helper tag to replace the plural placeholder, got {debug}"
        );
    }

    #[test]
    fn trailing_unless_stays_a_resolution_time_conditional() {
        let cases = [
            (
                "This creature deals 4 damage to that player unless they control a commander.",
                TriggerSpec::BeginningOfEndStep(PlayerFilter::Any),
            ),
            (
                "This creature deals 2 damage to that player unless they control two or more basic lands.",
                TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any),
            ),
            (
                "This artifact deals 2 damage to that player unless they have exactly three or exactly four cards in hand.",
                TriggerSpec::BeginningOfUpkeep(PlayerFilter::Opponent),
            ),
            (
                "This Aura deals 2 damage to that player unless that creature attacked this turn.",
                TriggerSpec::BeginningOfEndStep(PlayerFilter::ControllerOf(ObjectRef::tagged(
                    "enchanted",
                ))),
            ),
        ];

        for (text, trigger) in cases {
            let tokens = lex_line(text, 0).expect("lex trailing-unless sentence");
            let effects =
                crate::runtime_backend::effect_sentences::parse_effect_sentences_lexed(&tokens)
                    .expect("parse trailing-unless sentence");
            assert!(
                matches!(effects.as_slice(), [EffectAst::TrailingUnless { .. }]),
                "expected explicit trailing-unless AST for {text}: {effects:#?}"
            );

            let (_, prepared) = rewrite_prepare_triggered_effects_for_lowering(
                trigger,
                &effects,
                ReferenceImports::default(),
            )
            .expect("prepare trailing-unless trigger");
            assert!(
                prepared.intervening_if.is_none(),
                "trailing unless must not be promoted to intervening-if: {text}"
            );

            let (lowered, intervening_if) = materialize_prepared_triggered_effects(&prepared)
                .expect("lower trailing-unless trigger");
            assert!(
                intervening_if.is_none(),
                "unexpected intervening-if: {text}"
            );
            let conditional = lowered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
                .expect("lowered trailing-unless conditional");
            assert_eq!(
                conditional.surface,
                ironsmith_core::ConditionalSurface::TrailingUnless,
                "missing trailing-unless runtime surface: {text}"
            );
            assert!(
                matches!(&conditional.condition, Condition::Not(_)),
                "runtime true branch must retain the negated executable gate: {text}"
            );
        }
    }
}
