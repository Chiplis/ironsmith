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

    fn amount(&self) -> &crate::effect::Value {
        match self {
            Self::Direct { amount, .. }
            | Self::ObjectConditional { amount, .. }
            | Self::Conditional { amount, .. } => amount,
        }
    }

    fn with_tag(mut self, replacement: TagKey) -> Self {
        match &mut self {
            Self::Direct { tag, .. }
            | Self::ObjectConditional { tag, .. }
            | Self::Conditional { tag, .. } => *tag = replacement,
        }
        self
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
    let is_inline_entry_counter = put
        .amount
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter);
    // A counter placed in a later authored clause happens after the object
    // enters. Folding it into the zone move would change replacement-effect
    // semantics (and erase the authored sentence/"then" boundary). A counter
    // explicitly parsed from the producer's own "with ... on it" entry clause
    // is already typed as entry-time and may carry a broad sentence-level
    // follow-up hint from an earlier, unrelated "then".
    if !is_inline_entry_counter
        && (put
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence)
            || put
                .amount
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupThen))
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

fn coordinated_counter_followups(effect: &Effect) -> Option<Vec<CounterFollowup>> {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        if sequence.effects.is_empty() {
            return None;
        }
        return sequence.effects.iter().map(counter_followup).collect();
    }
    counter_followup(effect).map(|followup| vec![followup])
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

fn result_counter_followup(
    effect: &Effect,
    producer_id: Option<EffectId>,
) -> Option<CounterFollowup> {
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
            .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
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
        return schedule.effects.iter().find_map(entry_producer_tag);
    }
    if let Some(return_effect) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        && let ChooseSpec::Tagged(tag) = return_effect.target.base()
    {
        return Some(tag.clone());
    }
    if let Some(move_effect) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_effect.zone == Zone::Battlefield
        && let ChooseSpec::Tagged(tag) = move_effect.target.base()
    {
        return Some(tag.clone());
    }
    None
}

fn filter_selects_tag(filter: &ObjectFilter, tag: &TagKey) -> bool {
    let directly_selects_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    });
    directly_selects_tag
        || (!filter.any_of.is_empty()
            && filter
                .any_of
                .iter()
                .all(|branch| filter_selects_tag(branch, tag)))
}

fn entry_producer_body_consumes_tag(effect: &Effect, tag: &TagKey) -> bool {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return entry_producer_body_consumes_tag(&tagged.effect, tag);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return entry_producer_body_consumes_tag(&with_id.effect, tag);
    }
    if let Some(return_effect) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        return matches!(return_effect.target.base(), ChooseSpec::Tagged(target) if target == tag);
    }
    if let Some(return_all) = effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
    {
        return filter_selects_tag(&return_all.filter, tag);
    }
    effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .is_some_and(|move_effect| {
            move_effect.zone == Zone::Battlefield
                && matches!(move_effect.target.base(), ChooseSpec::Tagged(target) if target == tag)
        })
}

/// Prove that an entry producer's output tag is an alias for objects selected
/// from an earlier tag. Delayed-return cards commonly keep the original exile
/// tag in their authored "each of them enters" follow-up while the return
/// itself receives a fresh result tag. Retagging that follow-up to the return
/// result is safe only when the producer structurally consumes the earlier set.
fn producer_output_consumes_tag(
    effect: &Effect,
    output_tag: &TagKey,
    consumed_tag: &TagKey,
) -> bool {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if &tagged.tag == output_tag
            && entry_producer_body_consumes_tag(&tagged.effect, consumed_tag)
        {
            return true;
        }
        return producer_output_consumes_tag(&tagged.effect, output_tag, consumed_tag);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return producer_output_consumes_tag(&with_id.effect, output_tag, consumed_tag);
    }
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        return schedule
            .effects
            .iter()
            .any(|nested| producer_output_consumes_tag(nested, output_tag, consumed_tag));
    }
    entry_producer_tag(effect).as_ref() == Some(output_tag)
        && entry_producer_body_consumes_tag(effect, consumed_tag)
}

fn normalize_followup_tags(
    producer: &Effect,
    producer_tag: &TagKey,
    followups: Vec<CounterFollowup>,
) -> Option<Vec<CounterFollowup>> {
    followups
        .into_iter()
        .map(|followup| {
            if followup.tag() == producer_tag {
                Some(followup)
            } else if producer_output_consumes_tag(producer, producer_tag, followup.tag()) {
                Some(followup.with_tag(producer_tag.clone()))
            } else {
                None
            }
        })
        .collect()
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
                .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
            ) {
                return characteristic_filter_from_choose_spec(&return_effect.target);
            }
            if let Some(move_effect) = tagged
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
            {
                return characteristic_filter_from_choose_spec(&move_effect.target);
            }
            if let Some(return_all) = tagged
                .effect
                .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
            {
                return characteristic_filter_from_choose_spec(&ChooseSpec::All(
                    return_all.filter.clone(),
                ));
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
                .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
            ) {
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
            if let Some(return_all) = tagged
                .effect
                .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
            {
                // The generic MoveToZone executor already batches ChooseSpec::All
                // battlefield entries and resolves per-object entry-counter
                // conditions before the simultaneous entry event. Reuse that
                // composable capability instead of growing a parallel aggregate
                // return implementation.
                let mut replacement = crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::All(return_all.filter.clone()),
                    Zone::Battlefield,
                    false,
                );
                replacement.enters_tapped = return_all.tapped;
                replacement.enters_face_down = return_all.face_down;
                replacement.battlefield_controller = return_all.battlefield_controller;
                replacement.controller_surface_explicit = return_all.controller_surface_explicit;
                replacement.verb_surface = return_all.verb_surface;
                replacement = replacement.with_entry_counter(counter.clone());
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
    if let Some(return_effect) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        && matches!(return_effect.target.base(), ChooseSpec::Tagged(target) if target == tag)
    {
        return Some(Effect::new(
            return_effect.clone().with_entry_counter(counter.clone()),
        ));
    }
    if let Some(move_effect) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_effect.zone == Zone::Battlefield
        && matches!(move_effect.target.base(), ChooseSpec::Tagged(target) if target == tag)
    {
        return Some(Effect::new(
            move_effect.clone().with_entry_counter(counter.clone()),
        ));
    }
    None
}

/// Whether the entry producer brings back exactly one object. "Each of them
/// enters with a counter on it" is a set surface; a single returned card takes
/// the counter inline in its own sentence ("return … to the battlefield with a
/// finality counter on it"), which is how oracle writes it.
fn producer_selects_single_object(effect: &Effect) -> bool {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return producer_selects_single_object(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return producer_selects_single_object(&with_id.effect);
    }
    if let Some(return_effect) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        return return_effect.target.count().is_single();
    }
    if let Some(move_effect) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return move_effect.target.count().is_single();
    }
    false
}

fn build_counter_spec(
    producer: &Effect,
    followup: CounterFollowup,
    result_wrapper: bool,
) -> BattlefieldEntryCounterSpec {
    fn consume_inline_entry_surface(amount: crate::effect::Value) -> crate::effect::Value {
        if !amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
        {
            return amount;
        }
        amount
            .without_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
            .without_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupThen)
            .without_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence)
    }

    let tag = followup.tag().clone();
    let inferred_filter = producer_characteristic_filter(producer, &tag);
    match followup {
        CounterFollowup::Direct {
            counter_type,
            amount,
            ..
        } => {
            let surface = if amount
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence)
                && !producer_selects_single_object(producer)
            {
                BattlefieldEntryCounterSurface::EachOfThemEnters
            } else {
                BattlefieldEntryCounterSurface::Inline
            };
            BattlefieldEntryCounterSpec::new(
                counter_type,
                consume_inline_entry_surface(amount),
                surface,
            )
            .for_matching_object_optional(inferred_filter)
        }
        CounterFollowup::ObjectConditional {
            counter_type,
            amount,
            filter,
            ..
        } => {
            // The result wrapper is the typed surface for "if [it/a creature]
            // enters this way". Sentence-boundary metadata is broader and must
            // not replace that authored conditional with "each of them enters".
            let surface = if result_wrapper {
                BattlefieldEntryCounterSurface::IfObjectEntersThisWay
            } else if amount
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence)
            {
                BattlefieldEntryCounterSurface::EachOfThemEnters
            } else if producer_is_delayed(producer) {
                BattlefieldEntryCounterSurface::IfItEntersAsObject
            } else {
                BattlefieldEntryCounterSurface::ItEntersIfObject
            };
            BattlefieldEntryCounterSpec::new(
                counter_type,
                consume_inline_entry_surface(amount),
                surface,
            )
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
                consume_inline_entry_surface(amount),
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
    if let Some(reflexive) = effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>() {
        let mut replacement = reflexive.clone();
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
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<Effect>>() {
        let mut replacement = may.clone();
        fuse_effect_list(&mut replacement.effects);
        return Effect::new(replacement);
    }
    effect.clone()
}

/// The source's own put-counters follow-up, in either the bare or the
/// tag/id-wrapped spelling.
fn source_put_counters(
    effect: &Effect,
) -> Option<(crate::object::CounterType, crate::effect::Value)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return source_put_counters(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return source_put_counters(&with_id.effect);
    }
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() {
        return None;
    }
    if !matches!(put.target.base(), ChooseSpec::Source) {
        return None;
    }
    Some((put.counter_type, put.amount.clone()))
}

/// Fuse "Exile this with three time counters on it" — one authored zone change
/// whose counters are part of the move. Parsing splits it into an exile plus a
/// put-counters on the source, but a zone change mints a new object, so the
/// follow-up landed on the object that just left and the counters were silently
/// dropped (a suspended card that never gets its time counters never returns).
/// Battlefield moves keep the existing tag-matched fusion below.
fn fuse_source_zone_move_entry_counters(effects: &mut Vec<Effect>) {
    let mut index = 0usize;
    while index + 1 < effects.len() {
        let Some(move_effect) = effects[index].downcast_ref::<crate::effects::MoveToZoneEffect>()
        else {
            index += 1;
            continue;
        };
        if move_effect.zone == Zone::Battlefield
            || !move_effect.enters_with_counters.is_empty()
            || !matches!(move_effect.target.base(), ChooseSpec::Source)
        {
            index += 1;
            continue;
        }
        let Some((counter_type, amount)) = source_put_counters(&effects[index + 1]) else {
            index += 1;
            continue;
        };
        let mut fused = move_effect.clone();
        fused
            .enters_with_counters
            .push(BattlefieldEntryCounterSpec::new(
                counter_type,
                amount
                    .without_surface_hint(
                        ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence,
                    )
                    .without_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupThen),
                BattlefieldEntryCounterSurface::Inline,
            ));
        effects[index] = Effect::new(fused);
        effects.remove(index + 1);
    }
}

fn fuse_effect_list(effects: &mut Vec<Effect>) {
    for effect in effects.iter_mut() {
        *effect = rewrite_nested_effect(effect);
    }

    fuse_source_zone_move_entry_counters(effects);

    let mut index = 0usize;
    while index + 1 < effects.len() {
        let Some(producer_tag) = entry_producer_tag(&effects[index]) else {
            index += 1;
            continue;
        };
        let producer_id = producer_effect_id(&effects[index]);
        let (followups, result_wrapper) =
            if let Some(followup) = result_counter_followup(&effects[index + 1], producer_id) {
                (vec![followup], true)
            } else if let Some(followups) = coordinated_counter_followups(&effects[index + 1]) {
                (followups, false)
            } else {
                index += 1;
                continue;
            };
        let Some(followups) = normalize_followup_tags(&effects[index], &producer_tag, followups)
        else {
            index += 1;
            continue;
        };

        let mut rewritten = effects[index].clone();
        let mut attached_all = true;
        for followup in followups {
            let spec = build_counter_spec(&rewritten, followup, result_wrapper);
            let Some(attached) = attach_counter_to_producer(&rewritten, &producer_tag, &spec)
            else {
                attached_all = false;
                break;
            };
            rewritten = attached;
        }
        if !attached_all {
            index += 1;
            continue;
        }
        effects[index] = rewritten;
        effects.remove(index + 1);
    }
}

/// Fuse an explicitly authored entry-time counter sentence with the move in
/// the preceding source sentence. Ordinary later "put a counter" actions stay
/// separate; only the InlineBattlefieldEntryCounter marker can cross a source
/// sentence boundary.
fn fuse_across_segment_boundaries(segments: &mut Vec<crate::resolution::ResolutionSegment>) {
    let mut index = 0usize;
    while index + 1 < segments.len() {
        if !segments[index].self_replacements.is_empty()
            || !segments[index + 1].self_replacements.is_empty()
        {
            index += 1;
            continue;
        }
        let Some(producer) = segments[index].default_effects.last().cloned() else {
            index += 1;
            continue;
        };
        let Some(followup_effect) = segments[index + 1].default_effects.first() else {
            index += 1;
            continue;
        };
        let producer_id = producer_effect_id(&producer);
        let (followups, result_wrapper) =
            if let Some(followup) = result_counter_followup(followup_effect, producer_id) {
                (vec![followup], true)
            } else if let Some(followups) = coordinated_counter_followups(followup_effect) {
                (followups, false)
            } else {
                index += 1;
                continue;
            };
        if followups.iter().any(|followup| {
            !followup
                .amount()
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
        }) {
            index += 1;
            continue;
        }
        let Some(producer_tag) = entry_producer_tag(&producer) else {
            index += 1;
            continue;
        };
        let Some(followups) = normalize_followup_tags(&producer, &producer_tag, followups) else {
            index += 1;
            continue;
        };

        let mut rewritten = producer;
        let mut attached_all = true;
        for followup in followups {
            let spec = build_counter_spec(&rewritten, followup, result_wrapper);
            let Some(attached) = attach_counter_to_producer(&rewritten, &producer_tag, &spec)
            else {
                attached_all = false;
                break;
            };
            rewritten = attached;
        }
        if !attached_all {
            index += 1;
            continue;
        }
        let producer_index = segments[index].default_effects.len() - 1;
        segments[index].default_effects[producer_index] = rewritten;
        segments[index + 1].default_effects.remove(0);
        if segments[index + 1].default_effects.is_empty() {
            segments.remove(index + 1);
        }
    }
}

pub fn fuse_program(program: &mut ResolutionProgram) {
    let mut segments = program.segments.clone();
    for segment in &mut segments {
        fuse_effect_list(&mut segment.default_effects);
        for branch in &mut segment.self_replacements {
            fuse_effect_list(&mut branch.replacement_effects);
        }
    }
    fuse_across_segment_boundaries(&mut segments);
    *program = ResolutionProgram::new(segments);
}
