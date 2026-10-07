//! Assignment-based planning for the common mana payment shape.
//!
//! The search in [`super::planner`] clones a whole [`GameState`] for every
//! branch it explores, because activating a mana ability can change the board
//! in ways only simulation can predict. Most payments are not like that: every
//! source taps for a fixed bundle, nothing leaves the battlefield, and the only
//! real question is which sources to tap for which pips. That question is a
//! bipartite assignment, and solving it directly replaces thousands of clones
//! with a compact resource assignment and validation of the selected sequence.
//!
//! Two rules keep this honest:
//!
//! 1. **Only reviewed bundles are projected.** Fixed-output tap abilities use
//!    read-only production events, including supported triggers/replacements.
//!    Unknown candidates are measured by simulation. Typed units qualify
//!    spending restrictions and snow provenance before matching cost pips.
//! 2. **Failure means "fall back", never "unpayable".** Every exit here returns
//!    `None` and hands the request to the search. A gap in the model can cost
//!    speed; it cannot change which payments are legal.
//!
//! The chosen sequence is replayed against a staged clone and re-checked with
//! the same `can_pay_request` the search uses, so a plan leaving here has been
//! validated by the authoritative predicate, not by this module's arithmetic.

use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::mana::ManaSymbol;
use super::resources::PaymentManaUnit;

use super::ManaPaymentRequest;
use super::PlannedManaActivation;
use super::planner::{
    ActivationChoice, can_pay_request, collect_activation_choices_with_view, prepare_activation,
    prepare_owned_activation,
};

/// One pip that still needs a mana unit, as a set of acceptable symbols.
type PipSlot = Vec<ManaSymbol>;

/// A measured activation: what it produces, and what it costs us to keep.
struct MeasuredChoice {
    // None represents floating mana, which requires no activation.
    choice: Option<ActivationChoice>,
    produced: Vec<PaymentManaUnit>,
    flexibility: usize,
}

/// Candidate plans for a request the assignment model can express, or `None`
/// to fall back to the full search.
pub(super) fn try_candidates(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Option<Vec<(GameState, Vec<PlannedManaActivation>)>> {
    try_candidates_inner(game, request, false)
}

/// Cheap existence-check proposal: unknown sources go directly to the lazy
/// fallback instead of being simulated just to populate an assignment table.
pub(super) fn try_projected_candidates(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Option<Vec<(GameState, Vec<PlannedManaActivation>)>> {
    try_candidates_inner(game, request, true)
}

fn try_candidates_inner(
    game: &GameState,
    request: &ManaPaymentRequest,
    projection_only: bool,
) -> Option<Vec<(GameState, Vec<PlannedManaActivation>)>> {
    if !request_shape_is_supported(request) {
        return None;
    }

    let slots = expand_pip_slots(request)?;
    if slots.is_empty() {
        return None;
    }

    let mut measured = measure_choices_inner(game, request, projection_only);
    let produced = game.payment_mana_units(request);
    if !produced.is_empty() {
        measured.push(MeasuredChoice {
            choice: None,
            produced,
            flexibility: 0,
        });
    }
    if measured.is_empty() {
        return None;
    }

    let sequence = solve_assignment(&slots, &measured)?;
    replay(game, request, &sequence).map(|candidate| vec![candidate])
}

/// Reject requests whose payment rules the assignment model does not encode.
///
/// Each of these is expressible in principle; leaving them to the search keeps
/// the fast path's edge predicate simple enough to be obviously correct.
fn request_shape_is_supported(request: &ManaPaymentRequest) -> bool {
    // A simple bipartite matching does not represent the X/base allocation
    // or shared per-color caps. Let the complete assignment owner decide.
    if request.cost.has_x_spending_restriction() || request.assist_completion.is_some() { return false; }
    if !request.allow_mana_abilities {
        return false;
    }
    // Player-pinned sources and pre-committed activations constrain the search
    // in ways the assignment does not model.
    if !request.preferences.required_sources.is_empty()
        || !request.preferences.required_activations.is_empty()
        || !request.preferences.required_alternatives.is_empty()
        || !request.preferences.required_life_pips.is_empty()
        || !request.preferences.preserve_sources.is_empty()
    {
        return false;
    }
    // `allow_life_payment` is on by default and only bites when the cost
    // actually offers a life alternative, which `expand_pip_slots` rejects on
    // sight. `allow_black_life` is different: it synthesises life alternatives
    // during expansion that the printed pips do not show, so a black cost under
    // that rule is not the cost this module would be solving.
    if request.preferences.prefer_life
        || request.spend_policy.has_any_color_spending()
        || (request.allow_black_life && crate::decision::mana_cost_has_black_symbol(&request.cost))
    {
        return false;
    }
    // The outer planner has already selected alternative payments and reduced
    // this cost. Collection excludes reserved tap sources; fixed tap-and-mana
    // activations cannot consume reserved graveyard cards or sacrifice a
    // reserved permanent. Replay checks every reservation before accepting.
    true
}

/// Expand the request's cost into one slot per pip that must be paid.
///
/// Returns `None` for any symbol whose payability depends on more than the
/// produced symbol, so those costs reach the search untouched.
fn expand_pip_slots(request: &ManaPaymentRequest) -> Option<Vec<PipSlot>> {
    let mut slots = Vec::new();
    for pip in request.cost.pips() {
        match pip.as_slice() {
            [ManaSymbol::Generic(amount)] => {
                for _ in 0..*amount {
                    slots.push(vec![ManaSymbol::Generic(1)]);
                }
            }
            [ManaSymbol::X] => {
                for _ in 0..request.x_value {
                    slots.push(vec![ManaSymbol::Generic(1)]);
                }
            }
            alternatives => {
                // Life alternatives require a shared life budget; snow is a
                // property of the typed mana unit and is handled below.
                if alternatives.iter().any(|symbol| {
                    matches!(
                        symbol,
                        ManaSymbol::Life(_) | ManaSymbol::X
                    )
                }) {
                    return None;
                }
                slots.push(alternatives.to_vec());
            }
        }
    }
    Some(slots)
}

/// Project reviewed activations, measuring other candidates on scratch state.
///
/// Anything whose cost does more than tap the source is dropped rather than
/// modelled: exiling, sacrificing, or paying life can change what *other*
/// sources produce, which breaks the independence the assignment relies on.
/// That is what `undo_safe` reports here. Dropping a source can only cost us a
/// solution we then fall back to find.
#[cfg(test)]
fn measure_choices(game: &GameState, request: &ManaPaymentRequest) -> Vec<MeasuredChoice> {
    measure_choices_inner(game, request, false)
}

fn measure_choices_inner(
    game: &GameState,
    request: &ManaPaymentRequest,
    projection_only: bool,
) -> Vec<MeasuredChoice> {
    let mut measured = Vec::new();
    let analysis = super::sources::ManaSourceAnalysis::new(game);
    for choice in collect_activation_choices_with_view(game, request, false, &analysis.view) {
        if request.preferences.excluded_sources.contains(&choice.source)
            || request.activation_excluded_sources.contains(&choice.source) {
            continue;
        }
        let produced = if let Some(projected) = analysis.project(&choice) {
            projected.credits.iter().flat_map(|credit| credit.spendable_units(game, request)).collect()
        } else {
            if projection_only {
                continue;
            }
            let Some((_, staged, activation)) = prepare_activation(game, request, choice.clone()) else {
                continue;
            };
            if !activation.undo_safe {
                continue;
            }
            let mut produced = staged.payment_mana_units(request);
            // Undo-safe activations only add units. Remove the pre-existing
            // qualified pool without discarding snow or spending eligibility.
            for old in game.payment_mana_units(request) {
                let Some(index) = produced.iter().position(|unit| *unit == old) else { return measured; };
                produced.remove(index);
            }
            produced
        };
        if produced.is_empty() {
            continue;
        }
        measured.push(MeasuredChoice {
            flexibility: choice.flexibility,
            choice: Some(choice),
            produced,
        });
    }
    measured
}

/// Whether a produced symbol may pay a slot.
fn unit_pays_slot(unit: PaymentManaUnit, slot: &PipSlot) -> bool {
    slot.iter().any(|required| match required {
        ManaSymbol::Generic(_) => true,
        ManaSymbol::Snow => unit.snow,
        other => *other == unit.symbol,
    })
}

/// Choose which sources to tap, preferring to leave flexible ones untapped.
///
/// Sources are considered in ascending flexibility so a dual land is spent
/// before a source that could have covered several colours, matching how the
/// search's score ranks finished plans. Augmenting paths then guarantee that a
/// greedy early pick never strands a slot that only one source could pay.
fn solve_assignment(
    slots: &[PipSlot],
    measured: &[MeasuredChoice],
) -> Option<Vec<ActivationChoice>> {
    // How many distinct symbols each *source* can make, counted across all of
    // its abilities. Real dual lands compile to one single-colour ability per
    // colour, so a per-ability count cannot tell a Swamp from an Overgrown
    // Tomb — both look like "one colour" — and the preference collapses into
    // board order.
    let mut source_reach: std::collections::HashMap<Option<ObjectId>, Vec<ManaSymbol>> =
        std::collections::HashMap::new();
    for entry in measured {
        let reach = source_reach
            .entry(entry.choice.as_ref().map(|choice| choice.source))
            .or_default();
        for unit in &entry.produced {
            if !reach.contains(&unit.symbol) {
                reach.push(unit.symbol);
            }
        }
    }

    // One activation per source: tapping is the cost, so a source contributes
    // at most one bundle no matter how many abilities it offers.
    let mut order: Vec<usize> = (0..measured.len()).collect();
    order.sort_by_key(|&index| {
        let entry = &measured[index];
        (
            entry.choice.is_some(), // Spend floating mana before tapping a source.
            source_reach
                .get(&entry.choice.as_ref().map(|choice| choice.source))
                .map_or(entry.flexibility, Vec::len),
            entry.produced.len(),
            entry.choice.as_ref().map(|choice| choice.source.0),
            entry.choice.as_ref().map(|choice| choice.ability_index),
        )
    });

    let conservative = assign_sources(slots, measured, &order);
    if !measured
        .iter()
        .any(|entry| entry.choice.is_some() && entry.produced.len() > 1)
    {
        return conservative.map(|chosen| {
            chosen
                .into_iter()
                .filter_map(|index| measured[index].choice.clone())
                .collect()
        });
    }
    // Fixed bundles have a per-activation cost: using two units from one
    // source avoids another authoritative activation. Try that ordering too,
    // then compare compact results before replaying only the better proposal.
    order.sort_by_key(|&index| {
        let entry = &measured[index];
        (
            entry.choice.is_some(),
            source_reach
                .get(&entry.choice.as_ref().map(|choice| choice.source))
                .map_or(entry.flexibility, Vec::len),
            entry.produced.len().saturating_sub(slots.len()),
            std::cmp::Reverse(entry.produced.len()),
            entry
                .choice
                .as_ref()
                .map(|choice| (choice.source.0, choice.ability_index)),
        )
    });
    let bundled = assign_sources(slots, measured, &order);
    let score = |indices: &Vec<usize>| {
        indices
            .iter()
            .filter_map(|&index| {
                let entry = &measured[index];
                entry
                    .choice
                    .as_ref()
                    .map(|_| (entry.produced.len(), entry.flexibility, 1usize))
            })
            .fold((0usize, 0usize, 0usize), |total, entry| {
                (total.0 + entry.0, total.1 + entry.1, total.2 + entry.2)
            })
    };
    // Floating mana is constant for both proposals. Fewer produced units
    // therefore means less excess, followed by flexibility and source count,
    // matching the applicable dimensions of the payment score.
    let mut chosen = match (conservative, bundled) {
        (Some(first), Some(second)) if score(&second) < score(&first) => second,
        (Some(first), _) => first,
        (None, second) => second?,
    };
    // Different source flexibility can leave a single-unit source selected
    // beside a bundle that covers the whole payment. Remove such sources in
    // the compact model before replaying, without any extra game simulations.
    // Dropping sources cannot make a previously indispensable source redundant,
    // so one bounded pass is sufficient for this fixed-output assignment.
    for index in chosen.clone().into_iter().rev().take(32) {
        if measured[index].choice.is_none() { continue; }
        let reduced: Vec<_> = chosen.iter().copied().filter(|selected| *selected != index).collect();
        if let Some(assignment) = assign_sources(slots, measured, &reduced) {
            chosen = assignment;
        }
    }
    Some(
        chosen
            .into_iter()
            .filter_map(|index| measured[index].choice.clone())
            .collect(),
    )
}

fn assign_sources(
    slots: &[PipSlot],
    measured: &[MeasuredChoice],
    order: &[usize],
) -> Option<Vec<usize>> {
    // Colour-constrained slots first: they have the fewest candidate sources.
    let mut slot_order: Vec<usize> = (0..slots.len()).collect();
    slot_order.sort_by_key(|&index| {
        let slot = &slots[index];
        let generic = slot.iter().any(|s| matches!(s, ManaSymbol::Generic(_)));
        (generic, slot.len())
    });

    // slot -> (choice index, which unit of that choice's bundle)
    let mut slot_assignment: Vec<Option<(usize, usize)>> = vec![None; slots.len()];
    // (choice index, unit index) -> slot
    let mut unit_taken: Vec<Vec<Option<usize>>> = measured
        .iter()
        .map(|entry| vec![None; entry.produced.len()])
        .collect();
    // Sources already committed, so a second ability on the same permanent is
    // not offered a slot.
    let mut used_sources: Vec<Option<ObjectId>> = Vec::new();

    for &slot_index in &slot_order {
        let mut visited = vec![false; measured.len()];
        if !assign_slot(
            slot_index,
            slots,
            measured,
            order,
            &mut slot_assignment,
            &mut unit_taken,
            &mut used_sources,
            &mut visited,
        ) {
            return None;
        }
    }

    let mut chosen: Vec<usize> = slot_assignment
        .iter()
        .filter_map(|entry| entry.map(|(choice_index, _)| choice_index))
        .collect();
    chosen.sort_unstable();
    chosen.dedup();
    Some(chosen)
}

/// Reassign every pip already paid by a source plus one new pip to a different
/// output of that same activation. This preserves the one-activation resource
/// constraint while allowing WW to become WU when a later blue pip needs it.
fn match_bundle(
    required: &[usize], slots: &[PipSlot], units: &[PaymentManaUnit],
) -> Option<Vec<(usize, usize)>> {
    if required.len() > units.len() { return None; }
    fn assign(
        slot: usize, slots: &[PipSlot], units: &[PaymentManaUnit],
        taken: &mut [Option<usize>], seen: &mut [bool],
    ) -> bool {
        for unit in 0..units.len() {
            if seen[unit] || !unit_pays_slot(units[unit], &slots[slot]) { continue; }
            seen[unit] = true;
            if taken[unit].is_none_or(|previous| assign(previous, slots, units, taken, seen)) {
                taken[unit] = Some(slot);
                return true;
            }
        }
        false
    }
    let mut taken = vec![None; units.len()];
    for &slot in required {
        if !assign(slot, slots, units, &mut taken, &mut vec![false; units.len()]) { return None; }
    }
    Some(taken.into_iter().enumerate().filter_map(|(unit, slot)| slot.map(|slot| (slot, unit))).collect())
}

/// Kuhn's augmenting path over (slot, mana unit) pairs.
#[allow(clippy::too_many_arguments)]
fn assign_slot(
    slot_index: usize,
    slots: &[PipSlot],
    measured: &[MeasuredChoice],
    order: &[usize],
    slot_assignment: &mut Vec<Option<(usize, usize)>>,
    unit_taken: &mut Vec<Vec<Option<usize>>>,
    used_sources: &mut Vec<Option<ObjectId>>,
    visited: &mut Vec<bool>,
) -> bool {
    for &choice_index in order {
        if visited[choice_index] {
            continue;
        }
        let entry = &measured[choice_index];
        // A source already tapped for another slot may still supply a second
        // unit from the same bundle, but an untapped source must not collide
        // with a different ability on a permanent we already committed.
        let already_committed = unit_taken[choice_index].iter().any(Option::is_some);
        if !already_committed
            && used_sources.contains(&entry.choice.as_ref().map(|choice| choice.source))
        {
            let Some(previous) = measured.iter().enumerate().find_map(|(index, other)| {
                (other.choice.as_ref().map(|choice| choice.source)
                    == entry.choice.as_ref().map(|choice| choice.source)
                    && unit_taken[index].iter().any(Option::is_some)).then_some(index)
            }) else { continue; };
            // An augmenting-path ancestor still owns its pending unit update.
            // Do not replace that ancestor's whole activation underneath it.
            if !visited[previous] {
                let mut required: Vec<_> = slot_assignment.iter().enumerate()
                    .filter_map(|(slot, assignment)| assignment
                        .is_some_and(|(index, _)| index == previous).then_some(slot))
                    .collect();
                if !required.contains(&slot_index) { required.push(slot_index); }
                if let Some(assignment) = match_bundle(&required, slots, &entry.produced) {
                    unit_taken[previous].fill(None);
                    for (slot, unit) in assignment {
                        unit_taken[choice_index][unit] = Some(slot);
                        slot_assignment[slot] = Some((choice_index, unit));
                    }
                    return true;
                }
            }
            continue;
        }
        // Fill spare units in this bundle before displacing an existing pip.
        // Otherwise augmenting paths spread pips across new sources while
        // already-selected two-mana sources still have unused units.
        for occupied in [false, true] {
            for unit_index in 0..entry.produced.len() {
                if unit_taken[choice_index][unit_index].is_some() != occupied {
                    continue;
                }
                if !unit_pays_slot(entry.produced[unit_index], &slots[slot_index]) {
                    continue;
                }
                match unit_taken[choice_index][unit_index] {
                    None => {
                        unit_taken[choice_index][unit_index] = Some(slot_index);
                        slot_assignment[slot_index] = Some((choice_index, unit_index));
                        if !used_sources
                            .contains(&entry.choice.as_ref().map(|choice| choice.source))
                        {
                            used_sources.push(entry.choice.as_ref().map(|choice| choice.source));
                        }
                        return true;
                    }
                    Some(other_slot) => {
                        visited[choice_index] = true;
                        if assign_slot(
                            other_slot,
                            slots,
                            measured,
                            order,
                            slot_assignment,
                            unit_taken,
                            used_sources,
                            visited,
                        ) {
                            unit_taken[choice_index][unit_index] = Some(slot_index);
                            slot_assignment[slot_index] = Some((choice_index, unit_index));
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Replay the chosen activations and let the authoritative predicate decide.
///
/// The bundles were measured independently against the root state; applying
/// them in order is what proves they compose. If they do not, this returns
/// `None` and the search plans the payment instead.
fn replay(
    game: &GameState,
    request: &ManaPaymentRequest,
    sequence: &[ActivationChoice],
) -> Option<(GameState, Vec<PlannedManaActivation>)> {
    let mut staged = game.clone();
    let mut steps = Vec::with_capacity(sequence.len());
    for choice in sequence {
        let (_, next, activation) = prepare_owned_activation(staged, request, choice.clone())?;
        staged = next;
        steps.push(activation);
    }
    can_pay_request(&staged, request).then_some((staged, steps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::cost::TotalCost;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn game() -> (GameState, PlayerId) {
        (
            GameState::new(vec!["Alice".to_string()], 20),
            PlayerId::from_index(0),
        )
    }

    fn add_source(
        game: &mut GameState,
        owner: PlayerId,
        cost: crate::costs::Cost,
        produces: Vec<ManaSymbol>,
    ) -> ObjectId {
        let definition = CardBuilder::new(CardId::new(), "Test Source")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&definition, owner, Zone::Battlefield);
        game.object_mut(land)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::mana(
                TotalCost::from_cost(cost),
                produces,
            ));
        land
    }

    fn request(game: &mut GameState, payer: PlayerId, cost: ManaCost) -> ManaPaymentRequest {
        let source = game.new_object_id();
        ManaPaymentRequest::new(payer, source, crate::costs::PaymentReason::Effect, cost)
            .with_spend_policy(game.mana_spend_policy(payer, Some(source)))
    }

    /// The ordinary shape — tap lands for fixed mana — must be answered by the
    /// assignment, and the plan it returns must pay the cost.
    #[test]
    fn tap_only_sources_are_planned_without_searching() {
        let (mut game, alice) = game();
        for symbol in [ManaSymbol::Green, ManaSymbol::Green, ManaSymbol::Black] {
            add_source(&mut game, alice, crate::costs::Cost::tap(), vec![symbol]);
        }
        let request = request(
            &mut game,
            alice,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green], vec![ManaSymbol::Black]]),
        );
        let candidates =
            try_candidates(&game, &request).expect("tap-only board should use the assignment");
        assert_eq!(candidates.len(), 1);
        let (staged, steps) = &candidates[0];
        assert_eq!(steps.len(), 2, "one source per pip");
        assert!(can_pay_request(staged, &request));
    }

    #[test]
    fn snow_assignment_uses_each_producing_source_and_snapshot() {
        for snow_pips in [1, 2] {
            let (mut game, alice) = game();
            let definition = CardBuilder::new(CardId::new(), "Snow source")
                .card_types(vec![CardType::Land])
                .supertypes(vec![crate::types::Supertype::Snow]).build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land).unwrap().abilities_mut().push(crate::Ability::mana(
                TotalCost::from_cost(crate::costs::Cost::tap()), vec![ManaSymbol::Green]));
            let bonus = CardBuilder::new(CardId::new(), "Ordinary mana bonus")
                .card_types(vec![CardType::Enchantment]).build();
            let bonus = game.create_object_from_card(&bonus, alice, Zone::Battlefield);
            game.object_mut(bonus).unwrap().abilities_mut().push(crate::Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(
                    crate::target::PlayerFilter::You, crate::target::ObjectFilter::land()),
                vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Green])],
            ));
            game.refresh_continuous_state().unwrap();
            let mut pips = vec![vec![ManaSymbol::Snow]; snow_pips];
            if snow_pips == 1 { pips.push(vec![ManaSymbol::Green]); }
            let request = request(&mut game, alice, ManaCost::from_pips(pips));
            let candidates = try_projected_candidates(&game, &request);
            assert_eq!(candidates.is_some(), snow_pips == 1,
                "the non-snow trigger cannot inherit its activation source's snow property");
            if let Some(candidates) = candidates {
                let (staged, steps) = &candidates[0];
                assert_eq!(steps.len(), 1);
                let units = staged.payment_mana_units(&request);
                assert_eq!(units.iter().filter(|unit| unit.snow).count(), 1);
                assert_eq!(units.len(), 2);
                assert!(can_pay_request(staged, &request));
            }
            assert!(!game.is_tapped(land));
        }
    }

    #[test]
    fn assignment_preserves_resources_reserved_for_other_payments() {
        let (mut game, alice) = game();
        let reserved_tap = add_source(
            &mut game,
            alice,
            crate::costs::Cost::tap(),
            vec![ManaSymbol::Green],
        );
        let reserved_sacrifice = add_source(
            &mut game,
            alice,
            crate::costs::Cost::tap(),
            vec![ManaSymbol::Green],
        );
        let card = CardBuilder::new(CardId::new(), "Reserved graveyard card").build();
        let graveyard = game.create_object_from_card(&card, alice, Zone::Graveyard);
        let mut request = request(
            &mut game,
            alice,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        );
        request.reserved_tap_sources.push(reserved_tap);
        request.reserved_permanent_sources.push(reserved_sacrifice);
        request.reserved_graveyard_sources.push(graveyard);
        let candidates =
            try_candidates(&game, &request).expect("reservations permit fixed mana assignment");
        let (staged, steps) = &candidates[0];
        assert_eq!(steps.len(), 1);
        assert_eq!(
            steps[0].source, reserved_sacrifice,
            "may tap before paying a sacrifice cost"
        );
        assert!(
            !staged.is_tapped(reserved_tap),
            "convoke/improvise resource remains untapped"
        );
        assert_eq!(
            staged.object(reserved_sacrifice).unwrap().zone,
            Zone::Battlefield
        );
        assert_eq!(staged.object(graveyard).unwrap().zone, Zone::Graveyard);
        assert!(can_pay_request(staged, &request));
        assert!(
            !game.is_tapped(reserved_sacrifice),
            "planning does not mutate live resources"
        );
    }

    #[test]
    fn fixed_bundles_reduce_activations_without_increasing_excess() {
        for amount in [1u32, 4] {
            let (mut game, alice) = game();
            for _ in 0..4 {
                add_source(
                    &mut game,
                    alice,
                    crate::costs::Cost::tap(),
                    vec![ManaSymbol::Green],
                );
            }
            for _ in 0..2 {
                add_source(
                    &mut game,
                    alice,
                    crate::costs::Cost::tap(),
                    vec![ManaSymbol::Green; 2],
                );
            }
            let request = request(&mut game, alice, ManaCost::new().add_generic(amount));
            let candidates = try_projected_candidates(&game, &request).unwrap();
            assert_eq!(candidates[0].1.len(), if amount == 1 { 1 } else { 2 });
            assert_eq!(
                candidates[0].0.player(alice).unwrap().mana_pool.total(),
                u32::from(amount)
            );
            assert!(can_pay_request(&candidates[0].0, &request));
            assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
        }
    }

    #[test]
    fn existence_checks_project_plain_sources_and_keep_complex_fallback() {
        for sacrifice in [false, true] {
            let (mut game, alice) = game();
            let cost = if sacrifice {
                crate::costs::Cost::sacrifice_self()
            } else {
                crate::costs::Cost::tap()
            };
            let source = add_source(&mut game, alice, cost, vec![ManaSymbol::Blue]);
            let request = request(
                &mut game,
                alice,
                ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
            );
            crate::mana_payment::check_mana_payment(&game, &request).unwrap();
            let metrics = crate::mana_payment::last_mana_payment_perf();
            assert_eq!(metrics.analytic_selections, usize::from(!sacrifice));
            assert_eq!(metrics.searched_selections, usize::from(sacrifice));
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert!(!game.is_tapped(source));
        }
    }

    #[test]
    fn assignment_combines_floating_mana_with_sources_without_freeing_restrictions() {
        for restricted in [false, true] {
            let (mut game, alice) = game();
            let land = add_source(
                &mut game,
                alice,
                crate::costs::Cost::tap(),
                vec![ManaSymbol::Green],
            );
            if restricted {
                game.player_mut(alice).unwrap().add_restricted_mana(
                    crate::ability::RestrictedManaUnit {
                        source_controller: None,
                        symbol: ManaSymbol::Blue,
                        source: land,
                        source_chosen_creature_type: None,
                        restrictions: vec![crate::ability::ManaUsageRestriction::CastSpell {
                            card_types: vec![CardType::Creature],
                            subtype_requirement: None,
                            restrict_to_matching_spell: true,
                            grant_uncounterable: false,
                            enters_with_counters: vec![],
                            granted_abilities: vec![],
                        }],
                    },
                );
            } else {
                game.player_mut(alice)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Blue, 1);
            }
            let request = request(
                &mut game,
                alice,
                ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Green]]),
            );
            let candidate = try_projected_candidates(&game, &request);
            if restricted {
                assert!(
                    candidate.is_none(),
                    "creature-only mana cannot pay this effect"
                );
            } else {
                let candidate =
                    candidate.expect("floating blue plus one green source pays analytically");
                assert_eq!(candidate[0].1.len(), 1);
                assert_eq!(candidate[0].1[0].source, land);
                assert!(can_pay_request(&candidate[0].0, &request));
            }
            assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
            assert!(!game.is_tapped(land));
        }
    }

    /// A colour that only one source can make must not be stranded by a greedy
    /// pick: the augmenting path has to reclaim it.
    #[test]
    fn scarce_colour_is_not_stranded_by_an_earlier_assignment() {
        let (mut game, alice) = game();
        // A dual-ish source that can cover either pip, plus a green-only one.
        add_source(
            &mut game,
            alice,
            crate::costs::Cost::tap(),
            vec![ManaSymbol::Green],
        );
        add_source(
            &mut game,
            alice,
            crate::costs::Cost::tap(),
            vec![ManaSymbol::Green],
        );
        let request = request(
            &mut game,
            alice,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green], vec![ManaSymbol::Generic(1)]]),
        );
        let candidates = try_candidates(&game, &request).expect("assignment should succeed");
        let (staged, steps) = &candidates[0];
        assert_eq!(steps.len(), 2);
        assert!(can_pay_request(staged, &request));
    }

    /// A source whose cost does more than tap changes the board, so the
    /// assignment must decline rather than model it — the search still plans it.
    #[test]
    fn non_tap_cost_sources_fall_back_to_the_search() {
        let (mut game, alice) = game();
        add_source(
            &mut game,
            alice,
            crate::costs::Cost::sacrifice_self(),
            vec![ManaSymbol::Green],
        );
        let request = request(
            &mut game,
            alice,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        );
        assert!(
            try_candidates(&game, &request).is_none(),
            "sacrifice-for-mana must not be answered by the assignment"
        );
        // The authoritative planner still finds it.
        assert!(super::super::plan_first_mana_payment(&game, &request).is_ok());
    }

    /// Declining is never a verdict: an unpayable cost still reaches the search.
    #[test]
    fn unpayable_cost_is_declined_not_reported_unpayable() {
        let (mut game, alice) = game();
        add_source(
            &mut game,
            alice,
            crate::costs::Cost::tap(),
            vec![ManaSymbol::Green],
        );
        let request = request(
            &mut game,
            alice,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Blue]]),
        );
        assert!(try_candidates(&game, &request).is_none());
        assert!(super::super::plan_first_mana_payment(&game, &request).is_err());
    }
}

#[cfg(test)]
mod basic_land_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    /// Basic lands carry their mana ability from the continuous layers rather
    /// than the printed card, so the model must see them like any other source.
    #[test]
    fn intrinsic_basic_land_abilities_are_modelled() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        for _ in 0..2 {
            let definition = CardBuilder::new(CardId::new(), "Swamp")
                .card_types(vec![CardType::Land])
                .subtypes(vec![Subtype::Swamp])
                .build();
            game.create_object_from_card(&definition, alice, Zone::Battlefield);
        }
        let spell = game.new_object_id();
        let request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::Effect,
            ManaCost::from_pips(vec![vec![ManaSymbol::Black], vec![ManaSymbol::Black]]),
        )
        .with_spend_policy(game.mana_spend_policy(alice, Some(spell)));

        let measured = measure_choices(&game, &request);
        assert_eq!(
            measured.len(),
            2,
            "both basic Swamps must be modelled, got {measured:?}",
        );
        let candidates =
            try_candidates(&game, &request).expect("two Swamps should pay {B}{B} analytically");
        assert_eq!(candidates[0].1.len(), 2);
    }
}

impl std::fmt::Debug for MeasuredChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeasuredChoice")
            .field("choice", &self.choice)
            .field("produced", &self.produced)
            .finish()
    }
}

#[cfg(test)]
mod quality_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::cost::TotalCost;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn land(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        produces: Vec<ManaSymbol>,
    ) -> ObjectId {
        let definition = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .build();
        let id = game.create_object_from_card(&definition, owner, Zone::Battlefield);
        game.object_mut(id)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::mana(
                TotalCost::from_cost(crate::costs::Cost::tap()),
                produces,
            ));
        id
    }

    /// Add a source in the shape real dual lands compile to: one
    /// single-colour mana ability per colour, rather than one ability listing
    /// both. Per-ability flexibility cannot tell this apart from a basic.
    fn split_dual(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        colours: Vec<ManaSymbol>,
    ) -> ObjectId {
        let definition = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .build();
        let id = game.create_object_from_card(&definition, owner, Zone::Battlefield);
        for colour in colours {
            game.object_mut(id)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![colour],
                ));
        }
        id
    }

    /// The shape that actually ships: three duals that each expose {B} and {G}
    /// as separate abilities, plus one basic. The basic must still be spent
    /// first even though every individual ability makes exactly one colour.
    #[test]
    fn split_ability_duals_are_preserved_over_a_basic() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        for name in ["Tomb A", "Tomb B", "Tomb C"] {
            split_dual(
                &mut game,
                alice,
                name,
                vec![ManaSymbol::Black, ManaSymbol::Green],
            );
        }
        let swamp = land(&mut game, alice, "Swamp", vec![ManaSymbol::Black]);

        let spell = game.new_object_id();
        let request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::Effect,
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Black]]),
        )
        .with_spend_policy(game.mana_spend_policy(alice, Some(spell)));

        let candidates = try_candidates(&game, &request).expect("split duals should be assigned");
        let used: Vec<ObjectId> = candidates[0].1.iter().map(|step| step.source).collect();
        assert_eq!(used.len(), 3, "three sources for three pips: {used:?}");
        assert!(
            used.contains(&swamp),
            "the basic must be spent before a third dual; used {used:?}"
        );
    }

    /// These abilities produce both B and G (not a choice). One such bundle
    /// plus a Swamp pays three pips without the excess from two bundles.
    #[test]
    fn single_unit_source_avoids_excess_from_mixed_bundles() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let dual_a = land(
            &mut game,
            alice,
            "Dual A",
            vec![ManaSymbol::Black, ManaSymbol::Green],
        );
        let dual_b = land(
            &mut game,
            alice,
            "Dual B",
            vec![ManaSymbol::Black, ManaSymbol::Green],
        );
        let dual_c = land(
            &mut game,
            alice,
            "Dual C",
            vec![ManaSymbol::Black, ManaSymbol::Green],
        );
        let swamp = land(&mut game, alice, "Swamp", vec![ManaSymbol::Black]);

        let spell = game.new_object_id();
        let request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::Effect,
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Black]]),
        )
        .with_spend_policy(game.mana_spend_policy(alice, Some(spell)));

        let candidates = try_candidates(&game, &request).expect("plain duals should be assigned");
        let used: Vec<ObjectId> = candidates[0].1.iter().map(|step| step.source).collect();
        assert_eq!(used.len(), 2, "one bundle plus one single unit: {used:?}");
        assert_eq!(candidates[0].0.player(alice).unwrap().mana_pool.total(), 3);
        assert!(
            used.contains(&swamp),
            "the Swamp should avoid excess from another bundle; used {used:?} \
             (duals {dual_a:?} {dual_b:?} {dual_c:?})"
        );
    }
}
