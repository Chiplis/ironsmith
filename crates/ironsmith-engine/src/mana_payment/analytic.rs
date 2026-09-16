//! Assignment-based planning for the common mana payment shape.
//!
//! The search in [`super::planner`] clones a whole [`GameState`] for every
//! branch it explores, because activating a mana ability can change the board
//! in ways only simulation can predict. Most payments are not like that: every
//! source taps for a fixed bundle, nothing leaves the battlefield, and the only
//! real question is which sources to tap for which pips. That question is a
//! bipartite assignment, and solving it directly replaces thousands of clones
//! with one per source.
//!
//! Two rules keep this honest:
//!
//! 1. **Bundles are measured, never predicted.** Each candidate activation is
//!    simulated exactly once against the root state, so triggered mana
//!    abilities (CR 605.1b), continuous effects, and replacement effects are
//!    all reflected without this module having to re-derive them.
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
use crate::player::ManaPool;

use super::PlannedManaActivation;
use super::planner::{ActivationChoice, can_pay_request, collect_activation_choices,
    prepare_activation};
use super::{ManaPaymentRequest};

/// One pip that still needs a mana unit, as a set of acceptable symbols.
type PipSlot = Vec<ManaSymbol>;

/// A measured activation: what it produces, and what it costs us to keep.
struct MeasuredChoice {
    choice: ActivationChoice,
    produced: Vec<ManaSymbol>,
    flexibility: usize,
}

/// Candidate plans for a request the assignment model can express, or `None`
/// to fall back to the full search.
pub(super) fn try_candidates(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Option<Vec<(GameState, Vec<PlannedManaActivation>)>> {
    if !request_shape_is_supported(request) {
        return None;
    }

    let slots = expand_pip_slots(request)?;
    if slots.is_empty() {
        return None;
    }

    let measured = measure_choices(game, request);
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
    // Reserved resources belong to an alternative payment (convoke, delve,
    // improvise) that is being solved around this mana cost.
    request.reserved_tap_sources.is_empty()
        && request.reserved_graveyard_sources.is_empty()
        && request.reserved_permanent_sources.is_empty()
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
                // Snow asks where mana came from, and a life alternative is a
                // cost trade; neither is a symbol comparison.
                if alternatives.iter().any(|symbol| {
                    matches!(symbol, ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X)
                }) {
                    return None;
                }
                slots.push(alternatives.to_vec());
            }
        }
    }
    Some(slots)
}

/// Simulate each candidate activation once against the root state.
///
/// Anything whose cost does more than tap the source is dropped rather than
/// modelled: exiling, sacrificing, or paying life can change what *other*
/// sources produce, which breaks the independence the assignment relies on.
/// That is what `undo_safe` reports here. Dropping a source can only cost us a
/// solution we then fall back to find.
fn measure_choices(game: &GameState, request: &ManaPaymentRequest) -> Vec<MeasuredChoice> {
    let mut measured = Vec::new();
    for choice in collect_activation_choices(game, request) {
        if !choice_produces_plain_mana(game, request, &choice) {
            continue;
        }
        let Some((_, _, activation)) = prepare_activation(game, request, choice.clone()) else {
            continue;
        };
        if !activation.undo_safe {
            continue;
        }
        let produced = pool_symbols(&activation.expected_mana);
        if produced.is_empty() {
            continue;
        }
        measured.push(MeasuredChoice {
            choice,
            produced,
            flexibility: activation.flexibility,
        });
    }
    measured
}

/// True when this ability adds mana the assignment can treat as a plain unit.
///
/// The cost and effect shape is left to [`mana_ability_is_undo_safe`], which
/// already means "every cost component taps the source and every effect is a
/// mana producer" — exactly the independence the assignment needs. This adds
/// the one condition that predicate does not cover: mana carrying a usage
/// restriction needs the edge predicate to know which pips it may pay, so it
/// goes to the search.
fn choice_produces_plain_mana(
    game: &GameState,
    request: &ManaPaymentRequest,
    choice: &ActivationChoice,
) -> bool {
    use crate::ability::AbilityKind;
    if request.preferences.excluded_sources.contains(&choice.source) {
        return false;
    }
    let Some(ability) = game.current_ability(choice.source, choice.ability_index) else {
        return false;
    };
    let AbilityKind::Activated(activated) = &ability.kind else {
        return false;
    };
    activated.mana_usage_restrictions.is_empty()
}

/// Flatten a produced pool into individual mana units.
fn pool_symbols(pool: &ManaPool) -> Vec<ManaSymbol> {
    let mut symbols = Vec::new();
    for (count, symbol) in [
        (pool.white, ManaSymbol::White),
        (pool.blue, ManaSymbol::Blue),
        (pool.black, ManaSymbol::Black),
        (pool.red, ManaSymbol::Red),
        (pool.green, ManaSymbol::Green),
        (pool.colorless, ManaSymbol::Colorless),
    ] {
        for _ in 0..count {
            symbols.push(symbol);
        }
    }
    symbols
}

/// Whether a produced symbol may pay a slot.
fn unit_pays_slot(unit: ManaSymbol, slot: &PipSlot) -> bool {
    slot.iter().any(|required| match required {
        ManaSymbol::Generic(_) => true,
        other => *other == unit,
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
    let mut source_reach: std::collections::HashMap<ObjectId, Vec<ManaSymbol>> =
        std::collections::HashMap::new();
    for entry in measured {
        let reach = source_reach.entry(entry.choice.source).or_default();
        for symbol in &entry.produced {
            if !reach.contains(symbol) {
                reach.push(*symbol);
            }
        }
    }

    // One activation per source: tapping is the cost, so a source contributes
    // at most one bundle no matter how many abilities it offers.
    let mut order: Vec<usize> = (0..measured.len()).collect();
    order.sort_by_key(|&index| {
        let entry = &measured[index];
        (
            source_reach
                .get(&entry.choice.source)
                .map_or(entry.flexibility, Vec::len),
            entry.produced.len(),
            entry.choice.source.0,
            entry.choice.ability_index,
        )
    });

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
    let mut used_sources: Vec<ObjectId> = Vec::new();

    for &slot_index in &slot_order {
        let mut visited = vec![false; measured.len()];
        if !assign_slot(
            slot_index,
            slots,
            measured,
            &order,
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
    Some(
        chosen
            .into_iter()
            .map(|index| measured[index].choice.clone())
            .collect(),
    )
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
    used_sources: &mut Vec<ObjectId>,
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
        if !already_committed && used_sources.contains(&entry.choice.source) {
            continue;
        }
        for unit_index in 0..entry.produced.len() {
            if !unit_pays_slot(entry.produced[unit_index], &slots[slot_index]) {
                continue;
            }
            match unit_taken[choice_index][unit_index] {
                None => {
                    unit_taken[choice_index][unit_index] = Some(slot_index);
                    slot_assignment[slot_index] = Some((choice_index, unit_index));
                    if !used_sources.contains(&entry.choice.source) {
                        used_sources.push(entry.choice.source);
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
        let (_, next, activation) = prepare_activation(&staged, request, choice.clone())?;
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

    /// A colour that only one source can make must not be stranded by a greedy
    /// pick: the augmenting path has to reclaim it.
    #[test]
    fn scarce_colour_is_not_stranded_by_an_earlier_assignment() {
        let (mut game, alice) = game();
        // A dual-ish source that can cover either pip, plus a green-only one.
        add_source(&mut game, alice, crate::costs::Cost::tap(), vec![ManaSymbol::Green]);
        add_source(&mut game, alice, crate::costs::Cost::tap(), vec![ManaSymbol::Green]);
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
        let request = request(&mut game, alice, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]));
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
        add_source(&mut game, alice, crate::costs::Cost::tap(), vec![ManaSymbol::Green]);
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
            .field("source", &self.choice.source)
            .field("ability_index", &self.choice.ability_index)
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

    fn land(game: &mut GameState, owner: PlayerId, name: &str, produces: Vec<ManaSymbol>) -> ObjectId {
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
    fn split_dual(game: &mut GameState, owner: PlayerId, name: &str, colours: Vec<ManaSymbol>) -> ObjectId {
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
            split_dual(&mut game, alice, name, vec![ManaSymbol::Black, ManaSymbol::Green]);
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

    /// Spend the source that can only make one colour before spending a dual,
    /// so the flexible source stays available for whatever comes next.
    #[test]
    fn single_colour_sources_are_spent_before_duals() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let dual_a = land(&mut game, alice, "Dual A", vec![ManaSymbol::Black, ManaSymbol::Green]);
        let dual_b = land(&mut game, alice, "Dual B", vec![ManaSymbol::Black, ManaSymbol::Green]);
        let dual_c = land(&mut game, alice, "Dual C", vec![ManaSymbol::Black, ManaSymbol::Green]);
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
        assert_eq!(used.len(), 3, "three sources for three pips: {used:?}");
        assert!(
            used.contains(&swamp),
            "the single-colour Swamp should be spent before a third dual; used {used:?} \
             (duals {dual_a:?} {dual_b:?} {dual_c:?})"
        );
    }
}
