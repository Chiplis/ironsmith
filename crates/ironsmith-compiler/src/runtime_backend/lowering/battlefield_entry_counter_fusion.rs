use crate::effect::{Condition, Effect, EffectId, EffectPredicate};
use crate::filter::ObjectFilter;
use crate::resolution::ResolutionProgram;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::zone::Zone;
use ironsmith_core::{
    BattlefieldEntryCounterSpec, BattlefieldEntryCounterSurface, ConditionalSurface,
};

#[derive(Clone)]
enum CounterFollowup {
    Direct {
        tag: TagKey,
        counter_type: crate::object::CounterType,
        amount: crate::effect::Value,
    },
    ObjectConditional {
        tag: TagKey,
        counter_type: crate::object::CounterType,
        amount: crate::effect::Value,
        filter: ObjectFilter,
    },
    Conditional {
        tag: TagKey,
        counter_type: crate::object::CounterType,
        amount: crate::effect::Value,
        condition: Condition,
    },
}

impl CounterFollowup {
    fn tag(&self) -> &TagKey {
        match self {
            Self::Direct { tag, .. }
            | Self::ObjectConditional { tag, .. }
            | Self::Conditional { tag, .. } => tag,
        }
    }
}

fn tagged_put_counters(
    effect: &Effect,
) -> Option<(TagKey, crate::object::CounterType, crate::effect::Value)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return tagged_put_counters(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_put_counters(&with_id.effect);
    }
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() {
        return None;
    }
    // A counter placed in a later authored clause happens after the object
    // enters. Folding it into the zone move would change replacement-effect
    // semantics (and erase the authored sentence/"then" boundary).
    if put
        .amount
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence)
        || put
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupThen)
    {
        return None;
    }
    let ChooseSpec::Tagged(tag) = put.target.base() else {
        return None;
    };
    Some((tag.clone(), put.counter_type, put.amount.clone()))
}

fn counter_followup(effect: &Effect) -> Option<CounterFollowup> {
    if let Some((tag, counter_type, amount)) = tagged_put_counters(effect) {
        return Some(CounterFollowup::Direct {
            tag,
            counter_type,
            amount,
        });
    }

    let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.surface != ConditionalSurface::LeadingIf
        || !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
    {
        return None;
    }
    let (tag, counter_type, amount) = tagged_put_counters(&conditional.if_true[0])?;
    if let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition {
        if condition_tag != &tag {
            return None;
        }
        return Some(CounterFollowup::ObjectConditional {
            tag,
            counter_type,
            amount,
            filter: filter.clone(),
        });
    }
    Some(CounterFollowup::Conditional {
        tag,
        counter_type,
        amount,
        condition: conditional.condition.clone(),
    })
}

fn producer_effect_id(effect: &Effect) -> Option<EffectId> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Some(with_id.id);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return producer_effect_id(&tagged.effect);
    }
    None
}

fn result_counter_followup(effect: &Effect, producer_id: Option<EffectId>) -> Option<CounterFollowup> {
    let producer_id = producer_id?;
    let if_effect = effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != producer_id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 1
    {
        return None;
    }
    match counter_followup(&if_effect.then[0])? {
        CounterFollowup::ObjectConditional {
            tag,
            counter_type,
            amount,
            filter,
        } => Some(CounterFollowup::ObjectConditional {
            tag,
            counter_type,
            amount,
            filter,
        }),
        _ => None,
    }
}

fn is_entry_producer(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        .is_some()
        || effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_effect| move_effect.zone == Zone::Battlefield)
}

fn entry_producer_tag(effect: &Effect) -> Option<TagKey> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if is_entry_producer(&tagged.effect) {
            return Some(tagged.tag.clone());
        }
        return entry_producer_tag(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return entry_producer_tag(&with_id.effect);
    }
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        return schedule
            .effects
            .iter()
            .find_map(entry_producer_tag);
    }
    None
}

fn producer_is_delayed(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        .is_some()
}

fn characteristic_filter_from_choose_spec(spec: &ChooseSpec) -> Option<ObjectFilter> {
    let filter = match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return None,
    };
    if filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.supertypes.is_empty()
    {
        return None;
    }
    Some(ObjectFilter {
        card_types: filter.card_types.clone(),
        all_card_types: filter.all_card_types.clone(),
        subtypes: filter.subtypes.clone(),
        supertypes: filter.supertypes.clone(),
        ..Default::default()
    })
}

fn producer_characteristic_filter(effect: &Effect, tag: &TagKey) -> Option<ObjectFilter> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if &tagged.tag == tag {
            if let Some(return_effect) = tagged
                .effect
                .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
            {
                return characteristic_filter_from_choose_spec(&return_effect.target);
            }
            if let Some(move_effect) = tagged
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            {
                return characteristic_filter_from_choose_spec(&move_effect.target);
            }
        }
        return producer_characteristic_filter(&tagged.effect, tag);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return producer_characteristic_filter(&with_id.effect, tag);
    }
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        return schedule
            .effects
            .iter()
            .find_map(|effect| producer_characteristic_filter(effect, tag));
    }
    None
}

fn attach_counter_to_producer(
    effect: &Effect,
    tag: &TagKey,
    counter: &BattlefieldEntryCounterSpec,
) -> Option<Effect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if &tagged.tag == tag {
            if let Some(return_effect) = tagged
                .effect
                .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
            {
                let replacement = return_effect.clone().with_entry_counter(counter.clone());
                return Some(Effect::new(crate::effects::TaggedEffect::new(
                    tagged.tag.clone(),
                    Effect::new(replacement),
                )));
            }
            if let Some(move_effect) = tagged
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                && move_effect.zone == Zone::Battlefield
            {
                let replacement = move_effect.clone().with_entry_counter(counter.clone());
                return Some(Effect::new(crate::effects::TaggedEffect::new(
                    tagged.tag.clone(),
                    Effect::new(replacement),
                )));
            }
        }
        let replacement = attach_counter_to_producer(&tagged.effect, tag, counter)?;
        return Some(Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            replacement,
        )));
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        let replacement = attach_counter_to_producer(&with_id.effect, tag, counter)?;
        return Some(Effect::with_id(with_id.id.0, replacement));
    }
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        let mut replacement = schedule.clone();
        for nested in &mut replacement.effects {
            if let Some(attached) = attach_counter_to_producer(nested, tag, counter) {
                *nested = attached;
                return Some(Effect::new(replacement));
            }
        }
    }
    None
}

fn build_counter_spec(
    producer: &Effect,
    followup: CounterFollowup,
    result_wrapper: bool,
) -> BattlefieldEntryCounterSpec {
    let tag = followup.tag().clone();
    let inferred_filter = producer_characteristic_filter(producer, &tag);
    match followup {
        CounterFollowup::Direct {
            counter_type,
            amount,
            ..
        } => BattlefieldEntryCounterSpec::new(
            counter_type,
            amount,
            BattlefieldEntryCounterSurface::Inline,
        )
        .for_matching_object_optional(inferred_filter),
        CounterFollowup::ObjectConditional {
            counter_type,
            amount,
            filter,
            ..
        } => {
            let surface = if result_wrapper {
                BattlefieldEntryCounterSurface::IfObjectEntersThisWay
            } else if producer_is_delayed(producer) {
                BattlefieldEntryCounterSurface::IfItEntersAsObject
            } else {
                BattlefieldEntryCounterSurface::ItEntersIfObject
            };
            BattlefieldEntryCounterSpec::new(counter_type, amount, surface)
                .for_matching_object(filter)
        }
        CounterFollowup::Conditional {
            counter_type,
            amount,
            condition,
            ..
        } => {
            let mut spec = BattlefieldEntryCounterSpec::new(
                counter_type,
                amount,
                BattlefieldEntryCounterSurface::ThatObjectEntersIfCondition,
            )
            .with_condition(condition);
            spec.object_filter = inferred_filter;
            spec
        }
    }
}

trait OptionalObjectFilter {
    fn for_matching_object_optional(self, filter: Option<ObjectFilter>) -> Self;
}

impl OptionalObjectFilter for BattlefieldEntryCounterSpec {
    fn for_matching_object_optional(mut self, filter: Option<ObjectFilter>) -> Self {
        self.object_filter = filter;
        self
    }
}

fn rewrite_nested_effect(effect: &Effect) -> Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            rewrite_nested_effect(&tagged.effect),
        ));
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Effect::with_id(with_id.id.0, rewrite_nested_effect(&with_id.effect));
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        let mut replacement = conditional.clone();
        fuse_effect_list(&mut replacement.if_true);
        fuse_effect_list(&mut replacement.if_false);
        return Effect::new(replacement);
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        let mut replacement = if_effect.clone();
        fuse_effect_list(&mut replacement.then);
        fuse_effect_list(&mut replacement.else_);
        return Effect::new(replacement);
    }
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        let mut replacement = schedule.clone();
        fuse_effect_list(&mut replacement.effects);
        return Effect::new(replacement);
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        let mut replacement = choose_mode.clone();
        for mode in &mut replacement.modes {
            fuse_effect_list(&mut mode.effects);
        }
        return Effect::new(replacement);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut replacement = sequence.clone();
        fuse_effect_list(&mut replacement.effects);
        return Effect::new(replacement);
    }
    effect.clone()
}

fn fuse_effect_list(effects: &mut Vec<Effect>) {
    for effect in effects.iter_mut() {
        *effect = rewrite_nested_effect(effect);
    }

    let mut index = 0usize;
    while index + 1 < effects.len() {
        let Some(producer_tag) = entry_producer_tag(&effects[index]) else {
            index += 1;
            continue;
        };
        let producer_id = producer_effect_id(&effects[index]);
        let (followup, result_wrapper) = if let Some(followup) =
            result_counter_followup(&effects[index + 1], producer_id)
        {
            (followup, true)
        } else if let Some(followup) = counter_followup(&effects[index + 1]) {
            (followup, false)
        } else {
            index += 1;
            continue;
        };
        if followup.tag() != &producer_tag {
            index += 1;
            continue;
        }

        let spec = build_counter_spec(&effects[index], followup, result_wrapper);
        let Some(rewritten) =
            attach_counter_to_producer(&effects[index], &producer_tag, &spec)
        else {
            index += 1;
            continue;
        };
        effects[index] = rewritten;
        effects.remove(index + 1);
    }
}

pub(crate) fn fuse_program(program: &mut ResolutionProgram) {
    let mut segments = program.segments.clone();
    for segment in &mut segments {
        fuse_effect_list(&mut segment.default_effects);
        for branch in &mut segment.self_replacements {
            fuse_effect_list(&mut branch.replacement_effects);
        }
    }
    *program = ResolutionProgram::new(segments);
}
