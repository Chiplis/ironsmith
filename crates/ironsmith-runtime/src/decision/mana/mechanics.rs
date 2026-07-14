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
    let generic_in_cost = base_cost.generic_mana_total();
    let delve_amount = generic_in_cost.min(available_graveyard_cards);
    base_cost.reduce_generic(delve_amount)
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
    let generic_in_cost = base_cost.generic_mana_total();

    // Exile up to the generic mana cost
    let delve_amount = generic_in_cost.min(graveyard_count);
    let effective_cost = base_cost.reduce_generic(delve_amount);

    (delve_amount, effective_cost)
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
    if !has_convoke(spell) {
        return Vec::new();
    }

    // First apply other cost reductions (like Affinity and Delve)
    let mut cost_after_reductions = base_cost.clone();

    if has_affinity_for_artifacts(spell) {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &cost_after_reductions,
        1,
        &[],
        &CastingMethod::Normal,
        None,
    );

    let has_delve_ability = has_delve(spell);

    if has_delve_ability {
        let graveyard_count = count_cards_in_graveyard(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(graveyard_count);
    }

    // Now calculate Convoke creatures to tap
    let (creatures_to_tap, _) = calculate_convoke_cost(game, player, &cost_after_reductions);
    creatures_to_tap
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
                || !obj.has_card_type(crate::types::CardType::Artifact)
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
    use crate::mana::ManaSymbol;

    let improvise_artifacts = get_improvise_artifacts(game, player);
    if improvise_artifacts.is_empty() {
        return (Vec::new(), cost.clone());
    }

    let mut artifacts_to_tap = Vec::new();
    let mut remaining_pips: Vec<Vec<ManaSymbol>> = cost.pips().to_vec();

    // Improvise only pays generic mana
    let mut i = 0;
    while i < remaining_pips.len() && artifacts_to_tap.len() < improvise_artifacts.len() {
        let pip = &remaining_pips[i];

        // Check if this is a generic pip
        if pip.len() == 1
            && let ManaSymbol::Generic(n) = pip[0]
        {
            let available = improvise_artifacts.len() - artifacts_to_tap.len();
            let to_tap = (n as usize).min(available);

            for j in 0..to_tap {
                artifacts_to_tap.push(improvise_artifacts[artifacts_to_tap.len()]);
                let _ = j; // Suppress unused warning
            }

            // Reduce or remove the generic pip
            let paid = to_tap as u8;
            if paid >= n {
                remaining_pips.remove(i);
                continue;
            } else {
                remaining_pips[i] = vec![ManaSymbol::Generic(n - paid)];
            }
        }
        i += 1;
    }

    let effective_cost = crate::mana::ManaCost::from_pips(remaining_pips);
    (artifacts_to_tap, effective_cost)
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
    if !has_improvise(spell) {
        return Vec::new();
    }

    // First apply other cost reductions (Affinity, Delve, Convoke)
    let mut cost_after_reductions = base_cost.clone();

    if has_affinity_for_artifacts(spell) {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &cost_after_reductions,
        1,
        &[],
        &CastingMethod::Normal,
        None,
    );

    let has_delve_ability = has_delve(spell);

    if has_delve_ability {
        let graveyard_count = count_cards_in_graveyard(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(graveyard_count);
    }

    let has_convoke_ability = has_convoke(spell);

    if has_convoke_ability {
        let (_, convoked_cost) = calculate_convoke_cost(game, player, &cost_after_reductions);
        cost_after_reductions = convoked_cost;
    }

    // Now calculate Improvise artifacts to tap
    let (artifacts_to_tap, _) = calculate_improvise_cost(game, player, &cost_after_reductions);
    artifacts_to_tap
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
    use crate::mana::ManaSymbol;

    let convoke_creatures = get_convoke_creatures(game, player);
    if convoke_creatures.is_empty() {
        return (Vec::new(), cost.clone());
    }

    let mut creatures_to_tap = Vec::new();
    let mut remaining_pips: Vec<Vec<ManaSymbol>> = cost.pips().to_vec();
    let mut available_creatures = convoke_creatures;

    // First pass: pay colored mana with matching creatures
    let mut i = 0;
    while i < remaining_pips.len() {
        let pip = &remaining_pips[i];

        // Check if this is a single colored pip
        if pip.len() == 1 {
            let color_opt = match pip[0] {
                ManaSymbol::White => Some(crate::color::Color::White),
                ManaSymbol::Blue => Some(crate::color::Color::Blue),
                ManaSymbol::Black => Some(crate::color::Color::Black),
                ManaSymbol::Red => Some(crate::color::Color::Red),
                ManaSymbol::Green => Some(crate::color::Color::Green),
                _ => None,
            };

            if let Some(color) = color_opt {
                // Find a creature with this color
                if let Some(idx) = available_creatures
                    .iter()
                    .position(|(_, colors)| colors.contains(color))
                {
                    let (creature_id, _) = available_creatures.remove(idx);
                    creatures_to_tap.push(creature_id);
                    remaining_pips.remove(i);
                    continue;
                }
            }
        }
        i += 1;
    }

    // Second pass: pay generic mana with any remaining creatures
    let mut i = 0;
    while i < remaining_pips.len() && !available_creatures.is_empty() {
        let pip = &remaining_pips[i];

        // Check if this is a generic pip
        if pip.len() == 1
            && let ManaSymbol::Generic(n) = pip[0]
        {
            let creatures_needed = (n as usize).min(available_creatures.len());
            for _ in 0..creatures_needed {
                let (creature_id, _) = available_creatures.remove(0);
                creatures_to_tap.push(creature_id);
            }

            // Reduce or remove the generic pip
            let paid = creatures_needed as u8;
            if paid >= n {
                remaining_pips.remove(i);
                continue;
            } else {
                remaining_pips[i] = vec![ManaSymbol::Generic(n - paid)];
            }
        }
        i += 1;
    }

    let effective_cost = crate::mana::ManaCost::from_pips(remaining_pips);
    (creatures_to_tap, effective_cost)
}
