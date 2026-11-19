//! Order-independent replacement batches (CR 616.1).
//!
//! CR 616.1 has the affected player or controller choose which tied
//! replacement effect applies next because the order can change the outcome.
//! When every ordering of the tied set provably yields the same event and the
//! same later replacement opportunities, the choice has no effect. The
//! processor then applies the whole set in a canonical order without asking,
//! which keeps a board of N identical doublers from costing N prompts.
//!
//! The proof has four parts, each checked by [`tied_replacements_commute`]:
//!
//! 1. Each member is pure arithmetic on the event's quantity: it reads the
//!    event and unchanged game state, mutates nothing, and either scales
//!    (factor >= 1) or offsets (addend >= 0) the amount. Scalings commute with
//!    scalings and offsets with offsets, including under the fixed upper
//!    clamps the events apply (`min(M, ...)`), so every order of a single
//!    family reaches the same final amount. Mixed families do not commute
//!    (`2(q + 1) != 2q + 1`), so a set needs a single family.
//! 2. Each member's matcher ignores the amount
//!    ([`ReplacementMatcher::ignores_event_amount`]). Because the members only
//!    grow positive amounts, a member that matched the proposal still matches
//!    every intermediate event, so each member applies exactly once in every
//!    order.
//! 3. Every other unapplied replacement that could see this event also
//!    ignores the amount, and its applicability does not depend on it either.
//!    No outside effect can therefore start or stop applying part way through
//!    the batch. Lower-priority effects that already apply then see the same
//!    final event in every order.
//! 4. Members have distinct application keys, so applying one never retires
//!    another.
//!
//! Overflow is monotone under these operations. If any order overflows, the
//! final amount overflows, so every order fails at its last step.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuantityFamily {
    /// The action leaves this event unchanged (for example a counter doubler
    /// for another counter type). The identity commutes with everything.
    Identity,
    Scale,
    Offset,
}

/// The quantity-only events this batching models. Entry events are excluded:
/// their continuation yields between replacements, and they carry several
/// independent quantities at once.
fn is_scalar_quantity_event(event: &Event) -> bool {
    use crate::events::{
        DamageEvent, DrawEvent, LifeGainEvent, LifeLossEvent, PutCountersEvent, downcast_event,
    };
    let inner = event.inner();
    downcast_event::<DamageEvent>(inner).is_some()
        || downcast_event::<LifeGainEvent>(inner).is_some()
        || downcast_event::<LifeLossEvent>(inner).is_some()
        || downcast_event::<PutCountersEvent>(inner).is_some()
        || downcast_event::<DrawEvent>(inner).is_some()
}

fn is_token_creation_event(event: &Event) -> bool {
    crate::events::downcast_event::<crate::events::CreateTokensEvent>(event.inner()).is_some()
}

/// The arithmetic family of one action on one event, or `None` when the
/// action is anything other than pure scaling or non-negative offsetting.
fn quantity_family(
    game: &GameState,
    effect: &ReplacementEffect,
    event: &Event,
) -> Option<QuantityFamily> {
    use crate::replacement::EventModification;
    let tokens = is_token_creation_event(event);
    let scalar = is_scalar_quantity_event(event);
    if !tokens && !scalar {
        return None;
    }
    let put_counters =
        crate::events::downcast_event::<crate::events::PutCountersEvent>(event.inner());
    match &effect.replacement {
        // Token groups scale independently, so scalings on any covered sets
        // of groups commute. Group coverage depends only on each group's
        // characteristics, which scaling never changes.
        ReplacementAction::Double => Some(QuantityFamily::Scale),
        ReplacementAction::Modify(EventModification::Multiply(factor)) if *factor >= 1 => {
            Some(QuantityFamily::Scale)
        }
        // Token additions go through a total-adjustment policy, not group
        // arithmetic, so they stay with the player's choice.
        ReplacementAction::Modify(EventModification::Add(delta)) if scalar && *delta >= 0 => {
            Some(QuantityFamily::Offset)
        }
        ReplacementAction::Modify(EventModification::AddDynamic(value)) if scalar => {
            // Resolved against the unchanged game, so every member of the
            // batch sees the same addend in every order.
            let delta = application::resolve_signed_value_for_replacement(value, game, effect).ok()?;
            (delta >= 0).then_some(QuantityFamily::Offset)
        }
        ReplacementAction::DoubleCounters { counter_type } => {
            let put_counters = put_counters?;
            Some(if counter_type.is_none_or(|kind| kind == put_counters.counter_type) {
                QuantityFamily::Scale
            } else {
                QuantityFamily::Identity
            })
        }
        ReplacementAction::AddCountersToPlacement {
            counter_type,
            additional,
        } if *additional >= 0 => {
            let put_counters = put_counters?;
            Some(if counter_type.is_none_or(|kind| kind == put_counters.counter_type) {
                QuantityFamily::Offset
            } else {
                QuantityFamily::Identity
            })
        }
        _ => None,
    }
}

/// Whether an outside replacement's applicability could change as the
/// batch grows this event's amount.
fn outside_effect_may_observe_amount(effect: &ReplacementEffect, kind: crate::events::EventKind) -> bool {
    let Some(matcher) = effect.matcher.as_ref() else {
        // Matcherless effects never apply.
        return false;
    };
    if !matcher.may_match_event_kind(kind) {
        return false;
    }
    // Excess-damage redirection applies only while the amount exceeds lethal.
    matches!(effect.replacement, ReplacementAction::RedirectExcessDamage { .. })
        || !matcher.ignores_event_amount()
}

/// True when every ordering of `tied` produces the same event and the same
/// later replacement opportunities (see the module documentation).
pub(super) fn tied_replacements_commute(
    game: &GameState,
    event: &Event,
    state: &TraitEventProcessingState,
    additional_effects: &[ReplacementEffect],
    tied: &[ReplacementEffect],
) -> bool {
    if tied.len() < 2 {
        return false;
    }
    let mut family = None;
    let mut keys = std::collections::HashSet::with_capacity(tied.len());
    for effect in tied {
        if effect.optional
            || effect.optional_by_controller
            || effect.replacement.needs_mana_color_choice()
            || !effect
                .matcher
                .as_ref()
                .is_some_and(|matcher| matcher.ignores_event_amount())
            || !keys.insert(effect.application_key())
        {
            return false;
        }
        match quantity_family(game, effect, event) {
            None => return false,
            Some(QuantityFamily::Identity) => {}
            Some(member) => match family {
                None => family = Some(member),
                Some(existing) if existing == member => {}
                Some(_) => return false,
            },
        }
    }
    let kind = event.kind();
    let tied_ids: std::collections::HashSet<_> = tied.iter().map(|effect| effect.id).collect();
    game.effect_store
        .replacement_effects
        .effects()
        .iter()
        .chain(additional_effects)
        .filter(|effect| !tied_ids.contains(&effect.id) && !state.was_applied_effect(effect))
        .all(|effect| !outside_effect_may_observe_amount(effect, kind))
}

/// Apply a commuting tied set in its canonical (registration) order with the
/// same per-application bookkeeping as an individually chosen replacement.
pub(super) fn apply_commuting_batch(
    game: &mut GameState,
    mut event: Event,
    tied: &[ReplacementEffect],
    state: &mut TraitEventProcessingState,
) -> Result<Event, crate::effects::ExecutionError> {
    for effect in tied {
        let result = apply_trait_replacement_retaining_damage_branches(game, event, effect, state)?;
        mark_applied_replacement_choice(state, effect);
        consume_one_shot_if_applied(game, effect.id, &result);
        event = match result {
            TraitApplyResult::Modified(next) | TraitApplyResult::Unchanged(next) => next,
            _ => {
                return Err(crate::effects::ExecutionError::InternalError(
                    "order-independent replacement did not return a quantity event".into(),
                ));
            }
        };
    }
    Ok(event)
}
