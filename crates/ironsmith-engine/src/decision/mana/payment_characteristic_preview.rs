//! Preview one scoped cost-object characteristic, using the same choice's
//! filter and cardinality. Actual payment still owns the retained LKI receipt.
use super::*;

/// Correlate cost choices through the actual tap instruction, rather than an
/// internal tag spelling or a count of every cost resource on the card.
pub(super) fn collect_tap_cost_tags(
    effect: &crate::effect::Effect,
    tags: &mut Vec<crate::tag::TagKey>,
) {
    if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
        match &tap.target {
            crate::target::ChooseSpec::Tagged(tag) => {
                if !tags.contains(tag) {
                    tags.push(tag.clone());
                }
            }
            // `Tap all <objects chosen for this payment>`: the tap instruction
            // names the same choice through one exact tagged-object constraint.
            crate::target::ChooseSpec::All(filter) => {
                if let [constraint] = filter.tagged_constraints.as_slice()
                    && constraint.relation
                        == crate::target::TaggedOpbjectRelation::IsTaggedObject
                    && !tags.contains(&constraint.tag)
                {
                    tags.push(constraint.tag.clone());
                }
            }
            _ => {}
        }
    }
    if let Some(inner) = effect.transparent_child_effect() {
        collect_tap_cost_tags(inner, tags);
        return;
    }
    effect
        .0
        .visit_child_effects(&mut |child| collect_tap_cost_tags(child, tags));
}

fn unique_choice<'a>(
    choices: &'a [crate::effects::ChooseObjectsEffect],
    filter: &crate::target::ObjectFilter,
    tap_tags: &[crate::tag::TagKey],
) -> Option<&'a crate::effects::ChooseObjectsEffect> {
    let explicit = match filter.tagged_constraints.as_slice() {
        [] => None,
        [constraint] => Some(&constraint.tag),
        _ => return None,
    };
    let matching = choices
        .iter()
        .filter(|choice| {
            tap_tags.contains(&choice.tag) && explicit.is_none_or(|tag| choice.tag == *tag)
        })
        .collect::<Vec<_>>();
    match matching.as_slice() {
        [choice] => Some(*choice),
        _ => None,
    }
}

pub(super) fn maximum(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    choices: &[crate::effects::ChooseObjectsEffect],
    tap_tags: &[crate::tag::TagKey],
    query: &ironsmith_core::PriorEffectMetricQuery,
) -> Option<i32> {
    use ironsmith_core::EffectMetric;
    let power = match query.metric {
        EffectMetric::TotalPower | EffectMetric::FirstPower | EffectMetric::GreatestPower => true,
        EffectMetric::TotalToughness
        | EffectMetric::FirstToughness
        | EffectMetric::GreatestToughness => false,
        _ => return None,
    };
    // Leave existing unscoped payment metrics unchanged. This adapter only
    // claims a single exact choice; it cannot double-count across cost groups.
    let filter = query.filter.as_ref()?;
    let choice = if query.action == Some(ironsmith_core::PriorEffectAction::Tapped) {
        match unique_choice(choices, filter, tap_tags) {
            Some(choice) => choice,
            None => return Some(0),
        }
    } else {
        let [constraint] = filter.tagged_constraints.as_slice() else {
            return None;
        };
        let matching = choices
            .iter()
            .filter(|choice| choice.tag == constraint.tag)
            .collect::<Vec<_>>();
        let [choice] = matching.as_slice() else {
            return Some(0);
        };
        *choice
    };
    let mut query_filter = filter.clone();
    query_filter.tagged_constraints.clear();
    let ctx = game.filter_context_for(player, Some(source));
    let cause = crate::events::cause::EventCause::from_cost(source, player);
    let mut values = game
        .battlefield
        .iter()
        .filter_map(|&id| {
            let object = game.object(id)?;
            if game.controller_of(object) != player
                || !choice.filter.matches(object, &ctx, game)
                || !query_filter.matches(object, &ctx, game)
            {
                return None;
            }
            match query.action {
                Some(ironsmith_core::PriorEffectAction::Tapped) if game.is_tapped(id) => {
                    return None;
                }
                Some(ironsmith_core::PriorEffectAction::Sacrificed)
                    if !game.can_be_sacrificed_with_cause(id, &cause)
                        || game.player_cant_sacrifice_object_to_cast_or_activate(player, id) =>
                {
                    return None;
                }
                _ => {}
            }
            Some(if power {
                game.current_power(id).unwrap_or(0)
            } else {
                game.current_toughness(id).unwrap_or(0)
            })
        })
        .collect::<Vec<_>>();
    if values.len() < choice.count.min {
        return Some(0);
    }
    values.sort_unstable_by(|a, b| b.cmp(a));
    let maximum = choice.count.max.unwrap_or(values.len()).min(values.len());
    if maximum < choice.count.min {
        return Some(0);
    }
    if !matches!(
        query.metric,
        EffectMetric::TotalPower | EffectMetric::TotalToughness
    ) {
        return Some(if maximum == 0 {
            0
        } else {
            values.first().copied().unwrap_or(0).max(0)
        });
    }
    // A minimum can require negative-power members; an optional larger set
    // must never make those members mandatory just to obtain the preview.
    let mut sum = 0i32;
    let mut best = if choice.count.min == 0 { 0 } else { i32::MIN };
    for (index, value) in values.into_iter().take(maximum).enumerate() {
        sum = sum.saturating_add(value);
        if index + 1 >= choice.count.min {
            best = best.max(sum);
        }
    }
    Some(best.max(0))
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
    let scoped = spell_cost_payment_sacrifice_reductions(spell)
        .filter_map(|reduction| {
            let crate::effect::Value::PendingPriorEffectMetric(query) =
                reduction.reduction.unhinted()
            else {
                return None;
            };
            if query.action != Some(ironsmith_core::PriorEffectAction::Tapped)
                || !matches!(query.metric, ironsmith_core::EffectMetric::TotalPower)
                || query
                    .filter
                    .as_ref()
                    .is_none_or(|filter| filter.tagged_constraints.len() > 1)
            {
                return None;
            }
            Some(query)
        })
        .collect::<Vec<_>>();
    if scoped.is_empty() {
        return None;
    }
    let mut choices = Vec::new();
    let mut tap_tags = Vec::new();
    let mut selected_choices = Vec::new();
    let mut selected_taps = Vec::new();
    for cost in spell.additional_non_mana_costs() {
        if let Some(effect) = cost.effect_ref() {
            collect_cost_choices(effect, &mut choices);
            collect_tap_cost_tags(effect, &mut tap_tags);
            collect_cost_choices(effect, &mut selected_choices);
            collect_tap_cost_tags(effect, &mut selected_taps);
        }
    }
    for (index, optional) in spell.optional_costs.iter().enumerate() {
        for cost in optional.cost.non_mana_costs() {
            if let Some(effect) = cost.effect_ref() {
                collect_cost_choices(effect, &mut choices);
                collect_tap_cost_tags(effect, &mut tap_tags);
                if !selected_only || spell.optional_costs_paid.was_paid(index) {
                    collect_cost_choices(effect, &mut selected_choices);
                    collect_tap_cost_tags(effect, &mut selected_taps);
                }
            }
        }
    }
    let mut metrics =
        std::collections::BTreeMap::<crate::tag::TagKey, Vec<crate::target::ObjectFilter>>::new();
    for query in scoped {
        let Some(filter) = query.filter.as_ref() else {
            return Some(false);
        };
        let Some(choice) = unique_choice(&choices, filter, &tap_tags) else {
            return Some(false);
        };
        if selected_taps.contains(&choice.tag) {
            let mut filter = filter.clone();
            filter.tagged_constraints.clear();
            metrics.entry(choice.tag.clone()).or_default().push(filter);
        }
    }
    if metrics.is_empty() {
        return None;
    }
    struct Candidate {
        id: ObjectId,
        powers: Vec<i32>,
    }
    struct Group {
        candidates: Vec<Candidate>,
        minimum: usize,
        maximum: usize,
        metrics: usize,
    }
    let ctx = game.filter_context_for(player, Some(spell.id));
    let mut groups = Vec::new();
    for choice in selected_choices
        .iter()
        .filter(|choice| selected_taps.contains(&choice.tag))
    {
        let filters = metrics.get(&choice.tag).cloned().unwrap_or_default();
        let mut candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id).is_some_and(|object| {
                    game.controller_of(object) == player
                        && choice.filter.matches(object, &ctx, game)
                        && !game.is_tapped(*id)
                })
            })
            .map(|id| Candidate {
                id,
                powers: filters
                    .iter()
                    .map(|filter| {
                        if game
                            .object(id)
                            .is_some_and(|object| filter.matches(object, &ctx, game))
                        {
                            game.current_power(id).unwrap_or(0)
                        } else {
                            0
                        }
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        candidates.sort_unstable_by_key(|candidate| {
            std::cmp::Reverse(
                candidate
                    .powers
                    .iter()
                    .fold(0i32, |sum, power| sum.saturating_add((*power).max(0))),
            )
        });
        let maximum = choice
            .count
            .max
            .unwrap_or(candidates.len())
            .min(candidates.len());
        if choice.count.min > maximum {
            return Some(false);
        }
        groups.push(Group {
            candidates,
            minimum: choice.count.min,
            maximum,
            metrics: filters.len(),
        });
    }
    let view = DerivedGameView::new(game);
    let payable = |power: i32, resources: &[ObjectId]| {
        let locked = mana_cost_with_locked_x_and_generic_reduction(cost, x, 0);
        let reduced = locked.reduce_generic(power.max(0) as u32);
        let reduced = apply_minimum_spell_total_mana_with_view(&view, &reduced);
        let mut request = crate::mana_payment::ManaPaymentRequest::new(
            player,
            spell.id,
            crate::costs::PaymentReason::CastSpell,
            reduced,
        )
        .with_spend_policy(
            crate::alternative_cast::play_permission::casting_spend_policy(
                game,
                player,
                spell,
                casting_method,
            ),
        );
        request.reserved_tap_sources.extend_from_slice(resources);
        if menu_bound_skips_proposal(casting_method) {
            return resumable::check_payment(game, &request);
        }
        with_cast_payment_proposal(game, player, spell.id, spell, casting_method, true, |proposed| {
            resumable::check_payment(proposed, &request)
        })
    };
    fn groups_search(
        groups: &[Group],
        group: usize,
        resources: &mut Vec<ObjectId>,
        power: i32,
        payable: &impl Fn(i32, &[ObjectId]) -> bool,
    ) -> bool {
        if group == groups.len() {
            return payable(power, resources);
        }
        subset_search(
            groups,
            group,
            0,
            0,
            resources,
            &vec![0; groups[group].metrics],
            power,
            payable,
        )
    }
    fn subset_search(
        groups: &[Group],
        group: usize,
        index: usize,
        count: usize,
        resources: &mut Vec<ObjectId>,
        powers: &[i32],
        total: i32,
        payable: &impl Fn(i32, &[ObjectId]) -> bool,
    ) -> bool {
        let current = &groups[group];
        let contribution = powers
            .iter()
            .fold(0i32, |sum, power| sum.saturating_add((*power).max(0)));
        if count >= current.minimum
            && groups_search(
                groups,
                group + 1,
                resources,
                total.saturating_add(contribution),
                payable,
            )
        {
            return true;
        }
        if index == current.candidates.len()
            || count == current.maximum
            || count + current.candidates.len() - index < current.minimum
        {
            return false;
        }
        let optimistic_current = (0..current.metrics).fold(0i32, |sum, metric| {
            sum.saturating_add(
                current.candidates[index..]
                    .iter()
                    .filter(|candidate| !resources.contains(&candidate.id))
                    .map(|candidate| candidate.powers[metric].max(0))
                    .fold(powers[metric], |power, value| power.saturating_add(value))
                    .max(0),
            )
        });
        let optimistic_later = groups[group + 1..].iter().fold(0i32, |sum, next| {
            sum.saturating_add(
                next.candidates
                    .iter()
                    .flat_map(|candidate| candidate.powers.iter())
                    .fold(0i32, |power, value| power.saturating_add((*value).max(0))),
            )
        });
        if !payable(
            total
                .saturating_add(optimistic_current)
                .saturating_add(optimistic_later),
            resources,
        ) {
            return false;
        }
        let candidate = &current.candidates[index];
        if !resources.contains(&candidate.id) {
            resources.push(candidate.id);
            let next = powers
                .iter()
                .zip(&candidate.powers)
                .map(|(power, value)| power.saturating_add(*value))
                .collect::<Vec<_>>();
            if subset_search(
                groups,
                group,
                index + 1,
                count + 1,
                resources,
                &next,
                total,
                payable,
            ) {
                return true;
            }
            resources.pop();
        }
        subset_search(
            groups,
            group,
            index + 1,
            count,
            resources,
            powers,
            total,
            payable,
        )
    }
    Some(groups_search(&groups, 0, &mut Vec::new(), 0, &payable))
}
