use super::*;

fn generic_mana_cost(amount: u32) -> crate::mana::ManaCost {
    if amount == 0 {
        return crate::mana::ManaCost::new();
    }

    let mut pips = Vec::new();
    let mut remaining = amount;
    while remaining > 0 {
        let chunk = remaining.min(u8::MAX as u32) as u8;
        pips.push(vec![ManaSymbol::Generic(chunk)]);
        remaining -= chunk as u32;
    }
    crate::mana::ManaCost::from_pips(pips)
}

fn static_abilities_for_attack_preview(
    view: &DerivedGameView<'_>,
    attacker: &crate::object::Object,
) -> Vec<crate::static_abilities::StaticAbility> {
    view.calculated_characteristics(attacker.id)
        .map(|chars| chars.static_abilities)
        .unwrap_or_else(|| {
            attacker
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    crate::ability::AbilityKind::Static(static_ability) => {
                        Some(static_ability.clone())
                    }
                    _ => None,
                })
                .collect()
        })
}

fn defending_player_for_attack_target(game: &GameState, target: &AttackTarget) -> Option<PlayerId> {
    match target {
        AttackTarget::Player(player) => Some(*player),
        AttackTarget::Planeswalker(planeswalker) => {
            game.object(*planeswalker).map(|object| object.controller)
        }
    }
}

fn generic_attack_tax_preview(
    game: &GameState,
    defending_player: PlayerId,
    view: &DerivedGameView<'_>,
) -> u32 {
    let mut tax = 0u32;

    for &object_id in &game.battlefield {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if object.controller != defending_player {
            continue;
        }

        let abilities = view
            .calculated_characteristics(object_id)
            .map(|chars| chars.static_abilities)
            .unwrap_or_else(|| {
                object
                    .abilities
                    .iter()
                    .filter_map(|ability| match &ability.kind {
                        crate::ability::AbilityKind::Static(static_ability) => {
                            Some(static_ability.clone())
                        }
                        _ => None,
                    })
                    .collect()
            });

        tax = abilities.into_iter().fold(tax, |acc, ability| {
            acc.saturating_add(
                ability
                    .generic_attack_tax_per_attacker_against_you(game, object_id, defending_player)
                    .unwrap_or(0),
            )
        });
    }

    tax
}

fn can_declare_attack_target_preview(
    game: &GameState,
    attacker: &crate::object::Object,
    target: &AttackTarget,
    view: &DerivedGameView<'_>,
) -> bool {
    let Some(defending_player) = defending_player_for_attack_target(game, target) else {
        return false;
    };
    if !crate::rules::combat::can_attack_defending_player_with_view(
        attacker,
        defending_player,
        game,
        view,
    ) {
        return false;
    }

    let abilities = static_abilities_for_attack_preview(view, attacker);
    if abilities.iter().any(|ability| {
        ability
            .can_pay_attack_cost(game, attacker.id, attacker.controller)
            .is_some_and(|can_pay| !can_pay)
    }) {
        return false;
    }

    let total_generic_cost = abilities.iter().fold(
        generic_attack_tax_preview(game, defending_player, view),
        |acc, ability| {
            acc.saturating_add(
                ability
                    .generic_attack_mana_cost_for_source(game, attacker.id, attacker.controller)
                    .unwrap_or(0),
            )
        },
    );

    total_generic_cost == 0
        || game.can_pay_mana_cost(
            attacker.controller,
            None,
            &generic_mana_cost(total_generic_cost),
            0,
        )
}

/// Compute legal attackers for the active player.
pub fn compute_legal_attackers(game: &GameState, _combat: &CombatState) -> Vec<AttackerOption> {
    let mut options = Vec::new();
    let active_player = game.turn.active_player;
    let mut attack_capable = Vec::new();
    let view = DerivedGameView::new(game);

    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        if perm.controller != active_player {
            continue;
        }
        if !view.object_has_card_type(perm_id, crate::types::CardType::Creature) {
            continue;
        }
        if crate::rules::combat::can_attack_with_view(perm, game, &view) {
            attack_capable.push(perm_id);
        }
    }
    let has_other_attacker = attack_capable.len() >= 2;

    // Find all creatures controlled by active player that can attack
    for &perm_id in &attack_capable {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        if !has_other_attacker && !game.can_attack_alone(perm_id) {
            continue;
        }

        let goaded_by = game.active_goaders_for(perm.id);

        // Determine valid attack targets
        let mut valid_targets = Vec::new();

        // Can attack each opponent
        let mut goad_targets = Vec::new();
        let mut nongoad_targets = Vec::new();

        for opponent in &game.players {
            if opponent.id != active_player && opponent.is_in_game() {
                let target = AttackTarget::Player(opponent.id);
                if can_declare_attack_target_preview(game, perm, &target, &view) {
                    if goaded_by.contains(&opponent.id) {
                        goad_targets.push(target);
                    } else {
                        nongoad_targets.push(target);
                    }
                }
            }
        }

        // Can attack planeswalkers controlled by opponents
        for &other_perm_id in &game.battlefield {
            if let Some(other_perm) = game.object(other_perm_id)
                && other_perm.controller != active_player
                && view.object_has_card_type(other_perm_id, crate::types::CardType::Planeswalker)
            {
                let target = AttackTarget::Planeswalker(other_perm_id);
                if can_declare_attack_target_preview(game, perm, &target, &view) {
                    if goaded_by.contains(&other_perm.controller) {
                        goad_targets.push(target);
                    } else {
                        nongoad_targets.push(target);
                    }
                }
            }
        }

        if !nongoad_targets.is_empty() {
            valid_targets.extend(nongoad_targets);
        } else {
            valid_targets.extend(goad_targets);
        }

        let must_attack = crate::rules::combat::must_attack_with_view(perm, game, &view);

        if !valid_targets.is_empty() {
            options.push(AttackerOption {
                creature: perm_id,
                valid_targets,
                must_attack,
            });
        }
    }

    options
}

/// Compute legal blockers for the defending player.
pub fn compute_legal_blockers(
    game: &GameState,
    combat: &CombatState,
    defending_player: PlayerId,
) -> Vec<BlockerOption> {
    use std::collections::HashSet;

    let mut options = Vec::new();
    let mut potential_blockers = HashSet::new();
    let view = DerivedGameView::new(game);

    // For each attacker, find creatures that can block it
    for attacker_info in &combat.attackers {
        let attacker_id = attacker_info.creature;
        let Some(attacker) = game.object(attacker_id) else {
            continue;
        };

        let mut valid_blockers = Vec::new();

        // Find creatures controlled by defending player that can block this attacker
        for &perm_id in &game.battlefield {
            let Some(blocker) = game.object(perm_id) else {
                continue;
            };

            if blocker.controller != defending_player {
                continue;
            }

            if !view.object_has_card_type(perm_id, crate::types::CardType::Creature) {
                continue;
            }

            // Check if this creature can block this attacker
            if crate::rules::combat::can_block_with_view(attacker, blocker, game, &view) {
                valid_blockers.push(perm_id);
                potential_blockers.insert(perm_id);
            }
        }

        let min_blockers = crate::rules::combat::minimum_blockers_with_view(attacker, &view);

        options.push(BlockerOption {
            attacker: attacker_id,
            valid_blockers,
            min_blockers,
        });
    }

    if potential_blockers.len() == 1
        && let Some(&only_blocker) = potential_blockers.iter().next()
        && !game.can_block_alone(only_blocker)
    {
        for option in &mut options {
            option.valid_blockers.clear();
        }
    }

    options
}
