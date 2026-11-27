//! Resolve the operands of a plural owner-library instruction before moving any of them.

use crate::effect::Effect;
use crate::effects::{
    ExecutionContext, ForEachObject, ForEachTaggedEffect, MoveToZoneEffect, ResolvedTarget,
    SequenceEffect, ShuffleLibraryEffect, ShuffleObjectsIntoLibraryEffect, TaggedEffect,
};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};
use crate::zone::Zone;

/// "Put ... on top of their owners' libraries, then those players shuffle."
/// The two independent references can name the same permanent. Both the move
/// and its ensuing owner set use the objects identified before the first move.
pub(super) fn put_then_shuffle(
    sequence: &SequenceEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Option<SequenceEffect> {
    let (last, operands) = sequence.effects.split_last()?;
    let shuffle = last.downcast_ref::<ShuffleLibraryEffect>()?;
    let PlayerFilter::OwnerOf(ObjectRef::Tagged(tag)) = &shuffle.player else {
        return None;
    };
    if !crate::cards::is_sentence_helper_tag(tag.as_str(), "moved_to_owners_libraries") {
        return None;
    }
    fn operand<'a>(effect: &'a Effect, tag: &crate::tag::TagKey) -> Option<&'a Effect> {
        let wrapper = effect.downcast_ref::<TaggedEffect>()?;
        (&wrapper.tag == tag && wrapper.outcome_only).then_some(wrapper.effect.as_ref())
    }
    let (movement, objects) = match operands {
        [group] => {
            let group = operand(group, tag)?.downcast_ref::<ForEachObject>()?;
            let [child] = group.effects.as_slice() else {
                return None;
            };
            let movement = child.downcast_ref::<MoveToZoneEffect>()?;
            let objects = super::for_each_object::matching_objects(group, game, ctx)
                .into_iter()
                .map(|(_, snapshot)| snapshot)
                .collect();
            (movement, objects)
        }
        [source, target] => {
            let source = operand(source, tag)?.downcast_ref::<MoveToZoneEffect>()?;
            let target = operand(target, tag)?.downcast_ref::<MoveToZoneEffect>()?;
            if source.target.base() != &ChooseSpec::Source || !target.target.is_target() {
                return None;
            }
            let mut ids = vec![ctx.source];
            for target in &ctx.targets {
                if let ResolvedTarget::Object(id) = target
                    && !ids.contains(id)
                {
                    ids.push(*id);
                }
            }
            let objects = ids
                .into_iter()
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
                })
                .filter_map(|id| ObjectSnapshot::from_object_id(game, id))
                .collect();
            (source, objects)
        }
        _ => return None,
    };
    if !movement.to_top || movement.zone != Zone::Library {
        return None;
    }
    ctx.set_tagged_objects(tag.clone(), objects);
    ctx.exact_object_reference_tags.insert(tag.clone());
    let mut movement = movement.clone();
    movement.target = ChooseSpec::Tagged(tag.clone());
    Some(SequenceEffect::new(vec![
        Effect::new(movement),
        last.clone(),
    ]))
}

/// "The owners of those cards shuffle them into their libraries" is a single
/// instruction on the declared target set. A shared owner shuffles only once.
pub(super) fn shuffle_targeted_cards(
    effect: &ForEachTaggedEffect,
    ctx: &ExecutionContext,
) -> Option<Effect> {
    let [move_card, shuffle] = effect.effects.as_slice() else {
        return None;
    };
    let tagged = move_card.downcast_ref::<TaggedEffect>()?;
    let movement = tagged.effect.downcast_ref::<MoveToZoneEffect>()?;
    let shuffle = shuffle.downcast_ref::<ShuffleLibraryEffect>()?;
    let self_in_graveyard = effect.tag.as_str() == "triggering"
        && crate::effects::zones::shuffle_objects_into_library::graveyard_shuffle_reference(ctx)
            .is_some();
    if !(effect.tag.as_str().starts_with("targeted_") || self_in_graveyard)
        || effect.controller_at_last_blocked_by.is_some()
        || movement.zone != Zone::Library
        || movement.to_top
        || movement.library_order.is_some()
        || movement.target != ChooseSpec::Iterated
        || !matches!(&shuffle.player, PlayerFilter::OwnerOf(ObjectRef::Tagged(tag)) if tag.as_str() == "__it__")
    {
        return None;
    }
    let target = ChooseSpec::Tagged(effect.tag.clone());
    let instruction = if self_in_graveyard {
        ShuffleObjectsIntoLibraryEffect::new(target, PlayerFilter::You)
            .with_owner_library_destination()
    } else {
        ShuffleObjectsIntoLibraryEffect::new(
            target,
            PlayerFilter::OwnerOf(ObjectRef::Tagged(effect.tag.clone())),
        )
    };
    Some(Effect::new(tagged.with_effect(Effect::new(instruction))))
}

/// The target declaration fixes a permanent for each participant. Its following
/// library consult belongs to the controller recorded before that permanent moved.
pub(super) fn consult_former_controllers(
    effect: &ForEachTaggedEffect,
) -> Option<crate::effects::ForEachControllerOfTaggedEffect> {
    if !crate::cards::is_sentence_helper_tag(effect.tag.as_str(), "participant_permanent_targets") {
        return None;
    }
    let [consult, remainder] = effect.effects.as_slice() else {
        return None;
    };
    let mut consult = consult
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?
        .clone();
    let mut remainder = remainder
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?
        .clone();
    if consult.all_tag != remainder.tag
        || remainder.keep_tagged.as_ref() != Some(&consult.match_tag)
    {
        return None;
    }
    consult.player = PlayerFilter::IteratedPlayer;
    remainder.player = PlayerFilter::IteratedPlayer;
    Some(crate::effects::ForEachControllerOfTaggedEffect::new(
        effect.tag.clone(),
        vec![Effect::new(consult), Effect::new(remainder)],
    ))
}

/// Lowering can split the two operands of "shuffle this and target ... into
/// their owners' libraries" into adjacent moves. Both belong to one shuffle.
pub(crate) fn group_source_and_target_shuffles(
    effects: &[Effect],
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Option<Vec<Effect>> {
    fn shuffle(effect: &Effect) -> Option<&ShuffleObjectsIntoLibraryEffect> {
        let effect = effect
            .downcast_ref::<TaggedEffect>()
            .map_or(effect, |tagged| tagged.effect.as_ref());
        let effect = effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .map_or(effect, |record| record.effect.as_ref());
        effect.downcast_ref()
    }
    let (declarations, moves) = effects.split_at(effects.len().saturating_sub(2));
    let [first, second] = moves else { return None };
    let first = shuffle(first)?;
    let second = shuffle(second)?;
    // The materializer drops a synthetic target prelude after binding it to
    // its consumer. The live compiler may still supply that declaration.
    match declarations {
        [] => {}
        [declaration]
            if declaration
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some_and(|declared| declared.target == second.target) => {}
        _ => return None,
    }
    if first.target != ChooseSpec::Source
        || !second.target.is_target()
        || !first.owner_library_destination
        || !second.owner_library_destination
        || !first.possessive_owner_subject
        || !second.possessive_owner_subject
    {
        return None;
    }
    let objects_tag = crate::tag::TagKey::from("__compound_owner_shuffle");
    let mut cards = Vec::new();
    if let Some(source) =
        ObjectSnapshot::from_object_id(game, ctx.source).or_else(|| ctx.source_snapshot.clone())
    {
        cards.push(source);
    }
    for target in &ctx.targets {
        if let ResolvedTarget::Object(id) = target
            && !cards.iter().any(|card| card.object_id == *id)
            && let Some(card) = ObjectSnapshot::from_object_id(game, *id)
        {
            cards.push(card);
        }
    }
    ctx.set_tagged_objects(objects_tag.clone(), cards);
    ctx.exact_object_reference_tags.insert(objects_tag.clone());
    let grouped =
        ShuffleObjectsIntoLibraryEffect::new(ChooseSpec::Tagged(objects_tag), PlayerFilter::You)
            .with_owner_library_destination();
    let mut resolved = declarations.to_vec();
    resolved.push(Effect::new(grouped));
    Some(resolved)
}
