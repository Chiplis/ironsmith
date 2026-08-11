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
    view.calculated_characteristics_arc(attacker.id)
        .map(|chars| chars.static_abilities.to_vec())
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

fn required_attack_players_for_attack_preview(
    game: &GameState,
    attacker: &crate::object::Object,
    abilities: &[crate::static_abilities::StaticAbility],
) -> Vec<PlayerId> {
    abilities
        .iter()
        .filter_map(|ability| {
            ability.required_attack_player(game, attacker.id, game.controller_of(attacker))
        })
        .collect()
}

fn active_goaders_for_attack_preview(
    game: &GameState,
    attacker: &crate::object::Object,
    abilities: &[crate::static_abilities::StaticAbility],
) -> std::collections::HashSet<PlayerId> {
    let current_turn = game.turn.turn_number;
    let mut goaders = game
        .effect_store
        .goad_effects
        .iter()
        .filter(|effect| effect.creature == attacker.id && effect.is_active(game, current_turn))
        .map(|effect| effect.goaded_by)
        .collect::<std::collections::HashSet<_>>();

    let controller = game.controller_of(attacker);
    for ability in abilities {
        if let Some(player) = ability.goaded_by_player(game, attacker.id, controller) {
            goaders.insert(player);
        }
    }

    goaders
}

fn generic_attack_tax_preview(
    game: &GameState,
    target: &AttackTarget,
    defending_player: PlayerId,
    view: &DerivedGameView<'_>,
) -> u32 {
    let mut tax = 0u32;
    let target_kind = crate::static_abilities::AttackTaxTargetKind::from(target);

    for &object_id in &game.battlefield {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if game.controller_of(object) != defending_player {
            continue;
        }

        let abilities = view
            .calculated_characteristics_arc(object_id)
            .map(|chars| chars.static_abilities.to_vec())
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
            if !ability.generic_attack_tax_applies_to(target_kind) {
                return acc;
            }
            acc.saturating_add(
                ability
                    .generic_attack_tax_per_attacker_against_you(game, object_id, defending_player)
                    .unwrap_or(0),
            )
        });
    }

    for restriction in &game.effect_store.restriction_effects {
        if restriction.controller != defending_player
            || !restriction.is_active(game, game.turn.turn_number)
        {
            continue;
        }
        if let crate::effect::Restriction::AttackYouUnlessControllerPaysPerAttacker(
            per_attacker_tax,
            covers_planeswalkers,
        ) = &restriction.restriction
        {
            let applies = matches!(
                target_kind,
                crate::static_abilities::AttackTaxTargetKind::Player
            ) || (*covers_planeswalkers
                && matches!(
                    target_kind,
                    crate::static_abilities::AttackTaxTargetKind::Planeswalker
                ));
            if applies {
                tax = tax.saturating_add(*per_attacker_tax);
            }
        }
    }

    tax
}

fn can_declare_attack_target_preview(
    game: &GameState,
    attacker: &crate::object::Object,
    defending_player: PlayerId,
    generic_attack_tax: u32,
    abilities: &[crate::static_abilities::StaticAbility],
    view: &DerivedGameView<'_>,
) -> bool {
    if !crate::rules::combat::can_attack_defending_player_with_view(
        attacker,
        defending_player,
        game,
        view,
    ) {
        return false;
    }

    if abilities.iter().any(|ability| {
        ability
            .can_pay_attack_cost(game, attacker.id, game.controller_of(attacker))
            .is_some_and(|can_pay| !can_pay)
    }) {
        return false;
    }

    let total_generic_cost = abilities.iter().fold(generic_attack_tax, |acc, ability| {
        acc.saturating_add(
            ability
                .generic_attack_mana_cost_for_source(
                    game,
                    attacker.id,
                    game.controller_of(attacker),
                )
                .unwrap_or(0),
        )
    });

    total_generic_cost == 0
        || view
            .potential_mana(game.controller_of(attacker))
            .can_pay(&generic_mana_cost(total_generic_cost), 0)
}

/// Compute legal attackers for the active player.
pub fn compute_legal_attackers(game: &GameState, _combat: &CombatState) -> Vec<AttackerOption> {
    let view = DerivedGameView::new(game);
    compute_legal_attackers_with_view(game, _combat, &view)
}

fn attack_targets_for_player(
    game: &GameState,
    attacking_player: PlayerId,
    view: &DerivedGameView<'_>,
) -> Vec<(AttackTarget, PlayerId)> {
    let mut targets = Vec::new();
    for opponent in &game.players {
        if game.are_opponents(attacking_player, opponent.id)
            && opponent.is_in_game()
            && game.player_is_within_range(attacking_player, opponent.id)
            && game.attack_direction_allows_defender(attacking_player, opponent.id)
        {
            targets.push((AttackTarget::Player(opponent.id), opponent.id));
        }
    }
    for &other_perm_id in &game.battlefield {
        let Some(other_perm) = game.object(other_perm_id) else {
            continue;
        };
        let controller = game.controller_of(other_perm);
        if game.are_opponents(attacking_player, controller)
            && game.object_is_within_range(attacking_player, other_perm_id, None)
            && view.object_has_card_type(other_perm_id, crate::types::CardType::Planeswalker)
            && game.attack_direction_allows_defender(attacking_player, controller)
        {
            targets.push((AttackTarget::Planeswalker(other_perm_id), controller));
        }
        if view.object_has_card_type(other_perm_id, crate::types::CardType::Battle)
            && let Some(protector) = game.battle_protector(other_perm_id)
            && game.are_opponents(attacking_player, protector)
            && game.player_is_within_range(attacking_player, protector)
            && game.attack_direction_allows_defender(attacking_player, protector)
            && game
                .player(protector)
                .is_some_and(|player| player.is_in_game())
        {
            targets.push((AttackTarget::Battle(other_perm_id), protector));
        }
    }
    targets
}

pub(crate) fn compute_legal_attackers_with_view(
    game: &GameState,
    _combat: &CombatState,
    view: &DerivedGameView<'_>,
) -> Vec<AttackerOption> {
    let mut options = Vec::new();
    let mut attack_capable = Vec::new();

    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        if !game.is_active_player(game.controller_of(perm)) {
            continue;
        }
        if !view.object_has_card_type(perm_id, crate::types::CardType::Creature) {
            continue;
        }
        if crate::rules::combat::can_attack_with_view(perm, game, view) {
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

        let abilities = static_abilities_for_attack_preview(view, perm);
        let goaded_by = active_goaders_for_attack_preview(game, perm, &abilities);
        let attack_targets = attack_targets_for_player(game, game.controller_of(perm), view);

        // Determine valid attack targets
        let mut legal_targets = Vec::new();

        // Can attack each opponent
        let mut goad_targets = Vec::new();
        let mut nongoad_targets = Vec::new();

        for (target, defending_player) in &attack_targets {
            let generic_attack_tax =
                generic_attack_tax_preview(game, target, *defending_player, view);
            if can_declare_attack_target_preview(
                game,
                perm,
                *defending_player,
                generic_attack_tax,
                &abilities,
                view,
            ) {
                legal_targets.push(target.clone());
                if goaded_by.contains(defending_player) {
                    goad_targets.push(target.clone());
                } else {
                    nongoad_targets.push(target.clone());
                }
            }
        }

        let required_attack_players =
            required_attack_players_for_attack_preview(game, perm, &abilities);
        let required_player_targets = legal_targets
            .iter()
            .filter(|target| match target {
                AttackTarget::Player(player) => required_attack_players.contains(player),
                AttackTarget::Planeswalker(_) | AttackTarget::Battle(_) => false,
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut valid_targets = Vec::new();
        if !required_player_targets.is_empty() {
            valid_targets.extend(required_player_targets);
        } else if !nongoad_targets.is_empty() {
            valid_targets.extend(nongoad_targets);
        } else {
            valid_targets.extend(goad_targets);
        }

        let has_required_attack_target = !required_attack_players.is_empty()
            && valid_targets.iter().any(|target| match target {
                AttackTarget::Player(player) => required_attack_players.contains(player),
                AttackTarget::Planeswalker(_) | AttackTarget::Battle(_) => false,
            });
        let must_attack = abilities
            .iter()
            .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::MustAttack)
            || !goaded_by.is_empty()
            || has_required_attack_target;

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
        let Some(attacked_player) =
            crate::combat_state::defending_player_for_attack_target(game, &attacker_info.target)
        else {
            continue;
        };
        if attacked_player != defending_player
            && !(game.shared_team_turns_enabled()
                && game.are_teammates(attacked_player, defending_player))
        {
            continue;
        }
        let Some(attacker) = game.object(attacker_id) else {
            continue;
        };

        let mut valid_blockers = Vec::new();

        // Find creatures controlled by defending player that can block this attacker
        for &perm_id in &game.battlefield {
            let Some(blocker) = game.object(perm_id) else {
                continue;
            };

            if game.controller_of(blocker) != defending_player
                && !(game.shared_team_turns_enabled()
                    && game.are_teammates(game.controller_of(blocker), defending_player))
            {
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
