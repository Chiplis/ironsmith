//! Preview one scoped cost-object characteristic, using the same choice's
//! filter and cardinality. Actual payment still owns the retained LKI receipt.
use super::*;

pub(super) fn maximum(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    choices: &[crate::effects::ChooseObjectsEffect],
    query: &ironsmith_core::PriorEffectMetricQuery,
) -> Option<i32> {
    use ironsmith_core::EffectMetric;
    let power = match query.metric {
        EffectMetric::TotalPower | EffectMetric::FirstPower | EffectMetric::GreatestPower => true,
        EffectMetric::TotalToughness | EffectMetric::FirstToughness | EffectMetric::GreatestToughness => false,
        _ => return None,
    };
    // Leave existing unscoped payment metrics unchanged. This adapter only
    // claims a single exact choice; it cannot double-count across cost groups.
    let filter = query.filter.as_ref()?;
    let [constraint] = filter.tagged_constraints.as_slice() else { return None; };
    let matching = choices.iter().filter(|choice| choice.tag == constraint.tag).collect::<Vec<_>>();
    let [choice] = matching.as_slice() else { return Some(0); };
    if choice.count.min != 1 || choice.count.max != Some(1) { return Some(0); }
    let mut query_filter = filter.clone();
    query_filter.tagged_constraints.clear();
    let ctx = game.filter_context_for(player, Some(source));
    let cause = crate::events::cause::EventCause::from_cost(source, player);
    Some(game.battlefield.iter().filter_map(|&id| {
        let object = game.object(id)?;
        if game.controller_of(object) != player || !choice.filter.matches(object, &ctx, game)
            || !query_filter.matches(object, &ctx, game) { return None; }
        match query.action {
            Some(ironsmith_core::PriorEffectAction::Tapped) if game.is_tapped(id) => return None,
            Some(ironsmith_core::PriorEffectAction::Sacrificed) if !game.can_be_sacrificed_with_cause(id, &cause)
                || game.player_cant_sacrifice_object_to_cast_or_activate(player, id) => return None,
            _ => {}
        }
        Some(if power { game.current_power(id).unwrap_or(0) } else { game.current_toughness(id).unwrap_or(0) })
    }).max().unwrap_or(0).max(0))
}

/// Check the resource and mana portions jointly for one scoped tap payment.
/// The unmodified effective cost is retained before candidate reductions so
/// a minimum-total floor cannot be inverted into an inflated candidate price.
pub(super) fn can_pay_scoped_tap(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    cost: &crate::mana::ManaCost,
    selected_only: bool,
    x: u32,
) -> Option<bool> {
    let scoped = spell_cost_payment_sacrifice_reductions(spell).filter_map(|reduction| {
        let crate::effect::Value::PendingPriorEffectMetric(query) = reduction.reduction.unhinted() else { return None; };
        if query.action != Some(ironsmith_core::PriorEffectAction::Tapped)
            || !matches!(query.metric, ironsmith_core::EffectMetric::TotalPower)
            || query.filter.as_ref().is_none_or(|filter| filter.tagged_constraints.len() != 1) { return None; }
        Some(query)
    }).collect::<Vec<_>>();
    if scoped.is_empty() { return None; }
    let [query] = scoped.as_slice() else { return Some(false); };
    let mut choices = Vec::new();
    let mut costs = spell.additional_non_mana_costs();
    costs.extend(spell.optional_costs.iter().enumerate()
        .filter(|(index, _)| !selected_only || spell.optional_costs_paid.was_paid(*index))
        .flat_map(|(_, optional)| optional.cost.non_mana_costs().cloned()));
    for cost in costs {
        if let Some(effect) = cost.effect_ref() { collect_cost_choices(effect, &mut choices); }
    }
    let filter = query.filter.as_ref()?;
    let constraint = &filter.tagged_constraints[0];
    let matching = choices.iter().filter(|choice| choice.tag == constraint.tag).collect::<Vec<_>>();
    if matching.is_empty() && selected_only { return None; }
    let [choice] = matching.as_slice() else { return Some(false); };
    if choice.count.min != 1 || choice.count.max != Some(1) { return Some(false); }
    let ctx = game.filter_context_for(player, Some(spell.id));
    let candidates = game.battlefield.iter().copied().filter(|&id| {
        game.object(id).is_some_and(|object| game.controller_of(object) == player
            && choice.filter.matches(object, &ctx, game) && !game.is_tapped(id))
    }).collect::<Vec<_>>();
    let view = DerivedGameView::new(game);
    Some(candidates.into_iter().any(|resource| {
        let power = game.current_power(resource).unwrap_or(0).max(0) as u32;
        let reduced = cost.reduce_generic(power);
        let reduced = mana_cost_with_locked_x_and_generic_reduction(&reduced, x, 0);
        let reduced = apply_minimum_spell_total_mana_with_view(&view, &reduced);
        let mut request = crate::mana_payment::ManaPaymentRequest::new(
            player, spell.id, crate::costs::PaymentReason::CastSpell, reduced,
        ).with_spend_policy(crate::alternative_cast::play_permission::casting_spend_policy(game, player, spell, casting_method));
        request.reserved_tap_sources.push(resource);
        with_cast_payment_proposal(game, player, spell.id, spell, casting_method,
            |proposed| resumable::check_payment(proposed, &request))
    }))
}
