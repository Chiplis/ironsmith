use super::*;

/// Check if a player could pay a mana cost using potential mana.
///
/// This considers mana currently in pool plus mana from untapped sources.
pub fn can_potentially_pay(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
    x_value: u32,
) -> bool {
    let potential = compute_potential_mana(game, player);
    potential.can_pay(cost, x_value)
}

/// Calculate the effective mana cost for a spell with Delve, given available graveyard cards.
///
/// For Delve, each card exiled from graveyard pays for {1} of generic mana.
/// This function calculates the minimum mana needed given maximum Delve usage.
pub fn calculate_delve_effective_cost(
    base_cost: &crate::mana::ManaCost,
    available_graveyard_cards: u32,
) -> crate::mana::ManaCost {
    base_cost.reduce_generic(available_graveyard_cards)
}

/// Calculate how many cards to exile for Delve to minimize mana cost while being castable.
///
/// Returns (cards_to_exile, effective_mana_cost).
/// This greedily exiles cards to pay generic mana.
pub fn calculate_optimal_delve(
    game: &GameState,
    player: PlayerId,
    base_cost: &crate::mana::ManaCost,
) -> (u32, crate::mana::ManaCost) {
    let graveyard_count = count_cards_in_graveyard(game, player);
    let delve_amount = base_cost.generic_mana_total().min(graveyard_count);
    (
        delve_amount,
        calculate_delve_effective_cost(base_cost, delve_amount),
    )
}

/// Check if a spell has the Convoke ability.
pub fn has_convoke(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_convoke()
        } else {
            false
        }
    })
}

/// Calculate which creatures to tap for Convoke.
///
/// Returns the creature IDs to tap for maximum Convoke usage.
/// This takes into account Affinity and Delve reductions first.
pub fn calculate_convoke_creatures_to_tap(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> Vec<crate::ids::ObjectId> {
    if !spell_has_convoke(game, spell) {
        return Vec::new();
    }

    let cost = cost_after_affinity_and_delve(game, player, spell, base_cost);
    calculate_convoke_cost(game, player, &cost).0
}

fn cost_after_affinity_and_delve(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    // Apply increases before reductions, including affinity, then preview delve.
    let mut totals =
        collect_spell_cost_modifiers(game, player, spell, 1, &[], &CastingMethod::Normal, None);
    totals.add_generic_reduction(affinity_for_artifacts_reduction(game, player, spell));
    let cost = totals.apply(base_cost);
    if spell_has_delve(game, spell) {
        calculate_delve_effective_cost(&cost, count_cards_in_graveyard(game, player))
    } else {
        cost
    }
}

/// Check if a spell has the Improvise ability.
pub fn has_improvise(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_improvise()
        } else {
            false
        }
    })
}

/// Get untapped artifacts controlled by a player that can be tapped for Improvise.
///
/// Returns a list of artifact ObjectIds.
pub fn get_improvise_artifacts(game: &GameState, player: PlayerId) -> Vec<crate::ids::ObjectId> {
    game.battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Must be an artifact controlled by player
            if game.controller_of(obj) != player
                || game.is_phased_out(id)
                || !game.current_has_card_type(id, crate::types::CardType::Artifact)
            {
                return None;
            }
            // Must be untapped
            if game.is_tapped(id) {
                return None;
            }
            Some(id)
        })
        .collect()
}

/// Calculate the effective mana cost for a spell with Improvise.
///
/// For Improvise, each artifact tapped pays for {1} of generic mana.
/// Returns (artifacts_to_tap, effective_mana_cost).
pub fn calculate_improvise_cost(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
) -> (Vec<crate::ids::ObjectId>, crate::mana::ManaCost) {
    let artifacts = get_improvise_artifacts(game, player);
    if artifacts.is_empty() {
        return (Vec::new(), cost.clone());
    }

    let mut tapped = Vec::new();
    let mut pips = cost.pips().to_vec();
    tap_for_generic_mana(&mut pips, artifacts.into_iter(), &mut tapped);
    (tapped, cost.with_pips(pips))
}

fn tap_for_generic_mana(
    pips: &mut Vec<Vec<ManaSymbol>>,
    mut available: impl ExactSizeIterator<Item = ObjectId>,
    tapped: &mut Vec<ObjectId>,
) {
    pips.retain_mut(|pip| {
        if let [ManaSymbol::Generic(amount)] = pip.as_mut_slice()
            && available.len() > 0
        {
            let paid = usize::from(*amount).min(available.len());
            tapped.extend(available.by_ref().take(paid));
            *amount -= paid as u8;
            return *amount > 0;
        }
        true
    });
}

/// Calculate which artifacts to tap for Improvise.
///
/// Returns the artifact IDs to tap for maximum Improvise usage.
/// This takes into account Affinity, Delve, and Convoke reductions first.
pub fn calculate_improvise_artifacts_to_tap(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> Vec<crate::ids::ObjectId> {
    if !spell_has_improvise(game, spell) {
        return Vec::new();
    }

    let mut cost = cost_after_affinity_and_delve(game, player, spell, base_cost);
    if spell_has_convoke(game, spell) {
        cost = calculate_convoke_cost(game, player, &cost).1;
    }
    calculate_improvise_cost(game, player, &cost).0
}

/// Count untapped creatures controlled by a player that can be tapped for convoke.
///
/// Returns a tuple of (total_untapped_creatures, creature_ids_with_colors).
pub fn get_convoke_creatures(
    game: &GameState,
    player: PlayerId,
) -> Vec<(crate::ids::ObjectId, crate::color::ColorSet)> {
    game.battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Must be a creature controlled by player
            if game.controller_of(obj) != player || !game.current_is_creature(id) {
                return None;
            }
            // Must be untapped
            if game.is_tapped(id) {
                return None;
            }
            Some((id, game.current_colors(id).unwrap_or_else(|| obj.colors())))
        })
        .collect()
}

/// Calculate the effective mana cost for a spell with Convoke.
///
/// For Convoke, each creature tapped can pay for {1} or one mana of its colors.
/// This function calculates the minimum mana needed given maximum Convoke usage.
///
/// Returns (creatures_to_tap, effective_mana_cost).
pub fn calculate_convoke_cost(
    game: &GameState,
    player: PlayerId,
    cost: &crate::mana::ManaCost,
) -> (Vec<crate::ids::ObjectId>, crate::mana::ManaCost) {
    let mut available = get_convoke_creatures(game, player);
    if available.is_empty() {
        return (Vec::new(), cost.clone());
    }

    let mut tapped = Vec::new();
    let mut pips = cost.pips().to_vec();
    // Assign matching creatures to single colored pips before paying generic mana.
    pips.retain(|pip| {
        if let [symbol] = pip.as_slice()
            && let Some(color) = symbol.unit_color()
            && let Some(index) = available
                .iter()
                .position(|(_, colors)| colors.contains(color))
        {
            tapped.push(available.remove(index).0);
            return false;
        }
        true
    });
    tap_for_generic_mana(
        &mut pips,
        available.into_iter().map(|(id, _)| id),
        &mut tapped,
    );
    (tapped, cost.with_pips(pips))
}
