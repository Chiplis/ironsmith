//! Searches granted by each permanent in an earlier instruction's result.
use crate::effect::{Condition, Effect, Value, ValueComparisonOperator};
use crate::effects::{
    ChooseObjectsEffect, ConditionalEffect, ForEachTaggedEffect, ForPlayersEffect, MayEffect,
    PutOntoBattlefieldEffect, SequenceEffect, ShuffleLibraryEffect,
};
use crate::filter::ObjectRef;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

fn prior_controller(player: &PlayerFilter) -> bool {
    match player {
        PlayerFilter::IteratedPlayer => true,
        PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
        | PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag)) => tag.as_str() == "__it__",
        _ => false,
    }
}

/// CR 701.23h–i. The controllers of qualifying permanents each get one search
/// for the number they controlled. Only combine searches sharing one shuffle.
/// A shuffle inside an explicitly per-permanent procedure still happens there.
pub(crate) fn grouped_controller_search(
    each: &ForEachTaggedEffect,
    game: &crate::game_state::GameState,
    ctx: &mut crate::effects::ExecutionContext,
) -> Option<ForPlayersEffect> {
    if each.controller_at_last_blocked_by.is_some() {
        return None;
    }
    let [guard] = each.effects.as_slice() else {
        return None;
    };
    let guard = guard.downcast_ref::<ConditionalEffect>()?;
    let Condition::TaggedObjectMatchedLastKnown(tag, lost_permanent) = &guard.condition else {
        return None;
    };
    if tag.as_str() != "__it__"
        || !guard.if_false.is_empty()
        || guard.capture_condition_result
        || lost_permanent.controller.is_some()
    {
        return None;
    }
    let [body] = guard.if_true.as_slice() else {
        return None;
    };
    let offer = body.downcast_ref::<MayEffect>();
    let body = match offer {
        Some(offer) => {
            let [body] = offer.effects.as_slice() else {
                return None;
            };
            if !offer.decider.as_ref().is_some_and(prior_controller)
                || offer.pay_as_cost
                || offer.payment_receipt.is_some()
            {
                return None;
            }
            body
        }
        None => body,
    };
    let sequence = body.downcast_ref::<SequenceEffect>()?;
    let [choice, movement, rest @ ..] = sequence.effects.as_slice() else {
        return None;
    };
    let search = choice.downcast_ref::<ChooseObjectsEffect>()?;
    let movement = movement.downcast_ref::<PutOntoBattlefieldEffect>()?;
    if !search.is_search
        || search.zone != Some(Zone::Library)
        || !search.additional_zones.is_empty()
        || search.count != crate::effect::ChoiceCount::exactly(1)
        || search.count_value.is_some()
        || search.aggregate_constraint.is_some()
        || (offer.is_some() && !search.filter.has_search_stated_quality())
        || !prior_controller(&search.chooser)
        || !search.filter.owner.as_ref().is_some_and(prior_controller)
        || movement.target != ChooseSpec::Tagged(search.tag.clone())
        || !prior_controller(&movement.controller)
    {
        return None;
    }
    let tagged = ctx.get_tagged_all(&each.tag).map_or(&[][..], Vec::as_slice);
    let history_key = format!("__pre_move_history__{}", each.tag.as_str());
    let history = ctx
        .get_tagged_all(&history_key)
        .map_or(&[][..], Vec::as_slice);
    let mut seen = std::collections::HashSet::new();
    let origins: Vec<_> = tagged
        .iter()
        .filter_map(|current| {
            // Keep membership tied to the actual result. A replacement may have
            // moved other objects; a melded permanent may have several successors.
            let origin = history
                .iter()
                .find(|prior| {
                    prior.object_id == current.object_id
                        || game
                            .zone_change_result_ids(prior.object_id)
                            .contains(&current.object_id)
                })
                .unwrap_or(current);
            seen.insert(origin.object_id).then_some(origin)
        })
        .collect();
    let distinct_controllers = || {
        use crate::filter::ObjectFilterExt as _;
        let filter_ctx = ctx.filter_context(game);
        let mut controllers = std::collections::HashSet::new();
        origins
            .iter()
            .filter(|s| lost_permanent.matches_snapshot(s, &filter_ctx, game))
            .all(|s| controllers.insert(s.controller))
    };
    let shuffle = match rest {
        [] if ctx.library_search_groups.has_concluding_shuffle(choice) => None,
        [last]
            if distinct_controllers()
                || search.search_result_reference_surface
                    == Some(ironsmith_core::SearchResultReferenceSurface::ThoseCards) =>
        {
            let shuffle = last.downcast_ref::<ShuffleLibraryEffect>()?;
            if !prior_controller(&shuffle.player) {
                return None;
            }
            let mut shuffle = shuffle.clone();
            shuffle.player = PlayerFilter::IteratedPlayer;
            Some(shuffle)
        }
        _ => return None,
    };

    // Count last-known controllers. Ownership and control of the new objects
    // in the graveyard or exile do not determine who receives the search.
    let mut lost_permanent = lost_permanent.clone();
    lost_permanent.controller = Some(PlayerFilter::IteratedPlayer);
    // Keep this receipt historical even if an in-search action refreshes live
    // object tags. The ordinary result tag still names the result objects.
    let count_tag = crate::tag::TagKey::from(format!(
        "__pre_move_history__controller_search__{}",
        each.tag.as_str()
    ));
    let origins = origins.into_iter().cloned().collect();
    ctx.set_tagged_objects(count_tag.clone(), origins);
    let count = Value::TaggedSnapshotCount {
        tag: count_tag,
        filter: lost_permanent,
    };
    let mut search = search.clone();
    search.chooser = PlayerFilter::IteratedPlayer;
    search.filter.owner = Some(PlayerFilter::IteratedPlayer);
    search.count_value = Some(count.clone());
    let mut movement = movement.clone();
    movement.controller = PlayerFilter::IteratedPlayer;
    let mut actions = vec![Effect::new(search), Effect::new(movement)];
    if let Some(shuffle) = shuffle {
        actions.push(Effect::new(shuffle));
    }
    if let Some(offer) = offer {
        let mut offer = offer.clone();
        offer.effects = actions;
        offer.decider = Some(PlayerFilter::IteratedPlayer);
        actions = vec![Effect::new(offer)];
    }
    let eligible = Condition::ValueComparison {
        left: count,
        operator: ValueComparisonOperator::GreaterThan,
        right: Value::Fixed(0),
    };
    Some(ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::new(ConditionalEffect::if_only(eligible, actions))],
    ))
}
