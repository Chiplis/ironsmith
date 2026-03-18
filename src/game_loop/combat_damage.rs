use super::*;

// ============================================================================
// Combat Damage
// ============================================================================

/// Combat damage event for trigger processing.
#[derive(Debug, Clone)]
pub struct CombatDamageEvent {
    /// The source dealing damage.
    pub source: ObjectId,
    /// The target receiving damage.
    pub target: DamageEventTarget,
    /// Amount of damage dealt.
    pub amount: u32,
    /// Amount of life actually lost from this damage (0 for non-player targets, infect, or life-locked players).
    pub life_lost: u32,
    /// The damage result with lifelink/infect info.
    pub result: DamageResult,
}

/// Execute combat damage for a damage step.
///
/// # Arguments
/// * `game` - The game state
/// * `combat` - The combat state
/// * `first_strike` - True for first strike damage step, false for regular
///
/// # Returns
/// A list of damage events that occurred (for trigger processing).
pub fn execute_combat_damage_step(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
) -> Vec<CombatDamageEvent> {
    let mut damage_events = Vec::new();

    // Process each attacker
    for attacker_info in &combat.attackers {
        let attacker_id = attacker_info.creature;

        // Check if this creature deals damage in this step
        let Some(attacker) = game.object(attacker_id) else {
            continue;
        };

        // Use game-aware functions to check abilities from continuous effects
        let participates = if first_strike {
            deals_first_strike_damage_with_game(attacker, game)
        } else {
            deals_regular_combat_damage_with_game(attacker, game)
        };

        if !participates {
            continue;
        }

        // Combat damage assignment usually uses power, but some static abilities
        // replace it with toughness.
        let Some(combat_stat) = combat_damage_stat_for_creature(game, attacker) else {
            continue;
        };
        if combat_stat <= 0 {
            continue;
        }

        let controller = attacker.controller;

        if is_blocked(combat, attacker_id) {
            // Blocked attacker - deal damage to blockers
            let events =
                deal_damage_to_blockers(game, attacker_id, combat, combat_stat as u32, controller);
            damage_events.extend(events);
        } else if is_unblocked(combat, attacker_id) {
            // Unblocked attacker - deal damage to defender
            let event = deal_damage_to_defender(
                game,
                attacker_id,
                &attacker_info.target,
                combat_stat as u32,
            );
            if let Some(e) = event {
                damage_events.push(e);
            }
        }
    }

    // Process blockers dealing damage to attackers.
    //
    // A creature can be declared as blocking multiple attackers (e.g., "can block an additional
    // creature each combat"). In that case it assigns its combat damage among the attackers it
    // blocks, rather than dealing its full power to each attacker.
    let mut attackers_by_blocker: std::collections::HashMap<ObjectId, Vec<ObjectId>> =
        std::collections::HashMap::new();
    for (attacker_id, blocker_ids) in &combat.blockers {
        for &blocker_id in blocker_ids {
            attackers_by_blocker
                .entry(blocker_id)
                .or_default()
                .push(*attacker_id);
        }
    }

    // First, collect all blocker damage info (including per-recipient assigned damage).
    let mut blocker_damage_info: Vec<(ObjectId, ObjectId, PlayerId, u32, DamageResult)> =
        Vec::new();
    for (blocker_id, mut attacker_ids) in attackers_by_blocker {
        let Some(blocker) = game.object(blocker_id) else {
            continue;
        };

        let participates = if first_strike {
            deals_first_strike_damage_with_game(blocker, game)
        } else {
            deals_regular_combat_damage_with_game(blocker, game)
        };
        if !participates {
            continue;
        }

        let Some(combat_stat) = combat_damage_stat_for_creature(game, blocker) else {
            continue;
        };
        if combat_stat <= 0 {
            continue;
        }

        // Deterministic default order when multiple attackers are blocked.
        attacker_ids.sort_by_key(|id| id.0);

        let controller = blocker.controller;
        if attacker_ids.len() == 1 {
            let attacker_id = attacker_ids[0];
            if game.object(attacker_id).is_none() {
                continue;
            }
            let dmg = combat_stat as u32;
            let damage_result =
                calculate_damage_with_game(game, blocker, DamageTarget::Permanent, dmg, true);
            blocker_damage_info.push((blocker_id, attacker_id, controller, dmg, damage_result));
            continue;
        }

        let recipients: Vec<&crate::object::Object> = attacker_ids
            .iter()
            .filter_map(|id| game.object(*id))
            .collect();
        if recipients.is_empty() {
            continue;
        }

        let distribution = crate::rules::damage::distribute_combat_damage_to_creatures(
            blocker,
            &recipients,
            combat_stat as u32,
            game,
        );
        for (idx, (dmg, _is_lethal)) in distribution.into_iter().enumerate() {
            if dmg == 0 {
                continue;
            }
            let attacker_id = attacker_ids[idx];
            if game.object(attacker_id).is_none() {
                continue;
            }
            let damage_result =
                calculate_damage_with_game(game, blocker, DamageTarget::Permanent, dmg, true);
            blocker_damage_info.push((blocker_id, attacker_id, controller, dmg, damage_result));
        }
    }

    // Now apply all blocker damage.
    for (blocker_id, attacker_id, controller, _assigned, damage_result) in blocker_damage_info {
        let applied = apply_damage_to_permanent(game, attacker_id, blocker_id, &damage_result);

        // Apply lifelink (through event processing)
        apply_combat_lifelink(game, controller, &damage_result, applied.total_damage_dealt);

        damage_events.push(CombatDamageEvent {
            source: blocker_id,
            target: DamageEventTarget::Object(attacker_id),
            amount: applied.damage_dealt,
            life_lost: 0,
            result: damage_result,
        });
    }

    damage_events
}

pub(super) fn static_abilities_for_object(
    game: &GameState,
    object: &crate::object::Object,
) -> Vec<crate::static_abilities::StaticAbility> {
    game.calculated_characteristics(object.id)
        .map(|characteristics| characteristics.static_abilities.clone())
        .unwrap_or_else(|| {
            object
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                    _ => None,
                })
                .collect()
        })
}

pub(super) fn creature_assigns_combat_damage_using_toughness(
    game: &GameState,
    creature: &crate::object::Object,
) -> bool {
    for &source_id in &game.battlefield {
        let Some(source) = game.object(source_id) else {
            continue;
        };
        for ability in static_abilities_for_object(game, source) {
            match ability.id() {
                crate::static_abilities::StaticAbilityId::CreaturesAssignCombatDamageUsingToughness => {
                    return true;
                }
                crate::static_abilities::StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness => {
                    if source.controller == creature.controller {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}

pub(super) fn combat_damage_stat_for_creature(
    game: &GameState,
    creature: &crate::object::Object,
) -> Option<i32> {
    if creature_assigns_combat_damage_using_toughness(game, creature) {
        game.calculated_toughness(creature.id)
            .or_else(|| creature.toughness())
    } else {
        game.calculated_power(creature.id)
            .or_else(|| creature.power())
    }
}

pub(super) fn apply_combat_lifelink(
    game: &mut GameState,
    controller: PlayerId,
    damage_result: &DamageResult,
    total_damage_dealt: u32,
) {
    if !damage_result.has_lifelink || total_damage_dealt == 0 {
        return;
    }

    let life_to_gain =
        crate::event_processor::process_life_gain_with_event(game, controller, total_damage_dealt);
    if life_to_gain > 0
        && let Some(player) = game.player_mut(controller)
    {
        player.gain_life(life_to_gain);
    }
}

fn combat_damage_cause(game: &GameState, source_id: ObjectId) -> crate::events::cause::EventCause {
    game.object(source_id)
        .map(|obj| crate::events::cause::EventCause::from_combat_damage(source_id, obj.controller))
        .unwrap_or_else(|| crate::events::cause::EventCause::combat_damage(source_id))
}

fn combat_damage_amount_to_permanent(result: &DamageResult) -> u32 {
    result.damage_dealt.max(result.minus_counters)
}

fn combat_damage_amount_to_player(result: &DamageResult) -> u32 {
    result.damage_dealt.max(result.poison_counters)
}

/// Deal damage from an attacker to its blockers.
pub(super) fn deal_damage_to_blockers(
    game: &mut GameState,
    attacker_id: ObjectId,
    combat: &CombatState,
    total_damage: u32,
    controller: PlayerId,
) -> Vec<CombatDamageEvent> {
    let mut events = Vec::new();

    let blocker_ids = get_damage_assignment_order(combat, attacker_id);
    if blocker_ids.is_empty() {
        return events;
    }

    // Get blocker objects for distribution calculation
    let blockers: Vec<&crate::object::Object> = blocker_ids
        .iter()
        .filter_map(|&id| game.object(id))
        .collect();

    let Some(attacker) = game.object(attacker_id) else {
        return events;
    };

    // Calculate damage distribution (handles trample)
    let (distribution, excess) = distribute_trample_damage(attacker, &blockers, total_damage, game);

    // Get the attack target for potential trample damage
    let attack_target = get_attack_target(combat, attacker_id).cloned();

    // Collect damage results first (while we still have the immutable borrow)
    let mut blocker_damages: Vec<(ObjectId, DamageResult)> = Vec::new();
    for (i, (damage, _is_lethal)) in distribution.iter().enumerate() {
        if *damage == 0 {
            continue;
        }
        let blocker_id = blocker_ids[i];
        let damage_result =
            calculate_damage_with_game(game, attacker, DamageTarget::Permanent, *damage, true);
        blocker_damages.push((blocker_id, damage_result));
    }

    // Calculate excess damage result
    let excess_damage_result = if excess > 0 {
        if let Some(AttackTarget::Player(player_id)) = attack_target {
            Some((
                player_id,
                calculate_damage_with_game(
                    game,
                    attacker,
                    DamageTarget::Player(player_id),
                    excess,
                    true,
                ),
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Now apply all damage (borrow of attacker is dropped)
    for (blocker_id, damage_result) in blocker_damages {
        let applied = apply_damage_to_permanent(game, blocker_id, attacker_id, &damage_result);

        // Apply lifelink (through event processing)
        apply_combat_lifelink(game, controller, &damage_result, applied.total_damage_dealt);

        events.push(CombatDamageEvent {
            source: attacker_id,
            target: DamageEventTarget::Object(blocker_id),
            amount: applied.damage_dealt,
            life_lost: 0,
            result: damage_result,
        });
    }

    // Apply excess damage to defending player (trample)
    if let Some((player_id, damage_result)) = excess_damage_result {
        let applied = apply_damage_to_player(game, player_id, attacker_id, &damage_result);

        // Apply lifelink (through event processing)
        apply_combat_lifelink(game, controller, &damage_result, applied.total_damage_dealt);

        events.push(CombatDamageEvent {
            source: attacker_id,
            target: DamageEventTarget::Player(player_id),
            amount: applied.damage_dealt,
            life_lost: applied.life_lost,
            result: damage_result,
        });
    }

    events
}

/// Deal damage from an unblocked attacker to its target.
pub(super) fn deal_damage_to_defender(
    game: &mut GameState,
    attacker_id: ObjectId,
    target: &AttackTarget,
    damage: u32,
) -> Option<CombatDamageEvent> {
    let attacker = game.object(attacker_id)?;
    let controller = attacker.controller;

    match target {
        AttackTarget::Player(player_id) => {
            let damage_result = calculate_damage_with_game(
                game,
                attacker,
                DamageTarget::Player(*player_id),
                damage,
                true,
            );

            let applied = apply_damage_to_player(game, *player_id, attacker_id, &damage_result);

            // Apply lifelink (through event processing)
            apply_combat_lifelink(game, controller, &damage_result, applied.total_damage_dealt);

            Some(CombatDamageEvent {
                source: attacker_id,
                target: DamageEventTarget::Player(*player_id),
                amount: applied.damage_dealt,
                life_lost: applied.life_lost,
                result: damage_result,
            })
        }
        AttackTarget::Planeswalker(pw_id) => {
            use crate::event_processor::process_damage_assignments_with_event;
            use crate::game_event::DamageTarget as EventDamageTarget;

            let damage_result =
                calculate_damage_with_game(game, attacker, DamageTarget::Permanent, damage, true);

            let processed = process_damage_assignments_with_event(
                game,
                attacker_id,
                EventDamageTarget::Object(*pw_id),
                damage,
                true, // is_combat
                combat_damage_cause(game, attacker_id),
            );

            let mut final_damage = 0u32;
            let mut total_damage_dealt = 0u32;
            let keywords = crate::rules::damage::SourceDamageKeywords {
                has_deathtouch: damage_result.has_deathtouch,
                has_infect: damage_result.has_infect,
                has_wither: damage_result.has_wither,
                has_lifelink: damage_result.has_lifelink,
            };
            if !processed.replacement_prevented {
                for assignment in processed.assignments {
                    match assignment.target {
                        EventDamageTarget::Object(object_id) => {
                            if object_id == *pw_id {
                                if let Some(pw) = game.object_mut(*pw_id) {
                                    let current_loyalty = pw
                                        .counters
                                        .get(&CounterType::Loyalty)
                                        .copied()
                                        .unwrap_or(0);
                                    let new_loyalty =
                                        current_loyalty.saturating_sub(assignment.amount);
                                    if new_loyalty == 0 {
                                        pw.counters.remove(&CounterType::Loyalty);
                                    } else {
                                        pw.counters.insert(CounterType::Loyalty, new_loyalty);
                                    }
                                }
                                final_damage = final_damage.saturating_add(assignment.amount);
                                total_damage_dealt =
                                    total_damage_dealt.saturating_add(assignment.amount);
                                continue;
                            }
                            let applied = crate::rules::damage::apply_processed_damage_assignment(
                                game,
                                attacker_id,
                                assignment.target,
                                assignment.amount,
                                keywords,
                                combat_damage_cause(game, attacker_id),
                            );
                            if applied.applied {
                                total_damage_dealt =
                                    total_damage_dealt.saturating_add(assignment.amount);
                            }
                        }
                        EventDamageTarget::Player(_) => {
                            let applied = crate::rules::damage::apply_processed_damage_assignment(
                                game,
                                attacker_id,
                                assignment.target,
                                assignment.amount,
                                keywords,
                                combat_damage_cause(game, attacker_id),
                            );
                            if applied.applied {
                                total_damage_dealt =
                                    total_damage_dealt.saturating_add(assignment.amount);
                            }
                        }
                    }
                }
            }

            // Apply lifelink (only if damage was dealt, through event processing)
            apply_combat_lifelink(game, controller, &damage_result, total_damage_dealt);

            Some(CombatDamageEvent {
                source: attacker_id,
                target: DamageEventTarget::Object(*pw_id),
                amount: final_damage,
                life_lost: 0,
                result: damage_result,
            })
        }
    }
}

/// Apply damage to a permanent (creature or planeswalker).
///
/// This processes the damage through replacement/prevention effects before applying.
#[derive(Debug, Clone, Copy)]
pub(super) struct AppliedPermanentDamage {
    damage_dealt: u32,
    total_damage_dealt: u32,
}

pub(super) fn apply_damage_to_permanent(
    game: &mut GameState,
    permanent_id: ObjectId,
    source_id: ObjectId,
    result: &DamageResult,
) -> AppliedPermanentDamage {
    use crate::event_processor::process_damage_assignments_with_event;
    use crate::game_event::DamageTarget;

    let processed = process_damage_assignments_with_event(
        game,
        source_id,
        DamageTarget::Object(permanent_id),
        combat_damage_amount_to_permanent(result),
        true, // is_combat
        combat_damage_cause(game, source_id),
    );

    if processed.replacement_prevented {
        return AppliedPermanentDamage {
            damage_dealt: 0,
            total_damage_dealt: 0,
        };
    }

    let keywords = crate::rules::damage::SourceDamageKeywords {
        has_deathtouch: result.has_deathtouch,
        has_infect: result.has_infect,
        has_wither: result.has_wither,
        has_lifelink: result.has_lifelink,
    };
    let mut damage_to_original = 0u32;
    let mut total_damage_dealt = 0u32;

    for assignment in processed.assignments {
        let applied = crate::rules::damage::apply_processed_damage_assignment(
            game,
            source_id,
            assignment.target,
            assignment.amount,
            keywords,
            combat_damage_cause(game, source_id),
        );
        if !applied.applied {
            continue;
        }
        total_damage_dealt = total_damage_dealt.saturating_add(assignment.amount);
        if let DamageTarget::Object(object_id) = assignment.target
            && object_id == permanent_id
        {
            damage_to_original = damage_to_original.saturating_add(assignment.amount);
        }
    }

    AppliedPermanentDamage {
        damage_dealt: damage_to_original,
        total_damage_dealt,
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AppliedPlayerDamage {
    damage_dealt: u32,
    life_lost: u32,
    total_damage_dealt: u32,
}

/// Apply damage to a player.
///
/// This processes the damage through replacement/prevention effects before applying.
pub(super) fn apply_damage_to_player(
    game: &mut GameState,
    player_id: PlayerId,
    source_id: ObjectId,
    result: &DamageResult,
) -> AppliedPlayerDamage {
    use crate::event_processor::process_damage_assignments_with_event;
    use crate::game_event::DamageTarget;

    let processed = process_damage_assignments_with_event(
        game,
        source_id,
        DamageTarget::Player(player_id),
        combat_damage_amount_to_player(result),
        true, // is_combat
        combat_damage_cause(game, source_id),
    );

    if processed.replacement_prevented {
        return AppliedPlayerDamage {
            damage_dealt: 0,
            life_lost: 0,
            total_damage_dealt: 0,
        };
    }

    let keywords = crate::rules::damage::SourceDamageKeywords {
        has_deathtouch: result.has_deathtouch,
        has_infect: result.has_infect,
        has_wither: result.has_wither,
        has_lifelink: result.has_lifelink,
    };
    let mut damage_to_original = 0u32;
    let mut life_lost_to_original = 0u32;
    let mut total_damage_dealt = 0u32;

    for assignment in processed.assignments {
        let applied = crate::rules::damage::apply_processed_damage_assignment(
            game,
            source_id,
            assignment.target,
            assignment.amount,
            keywords,
            combat_damage_cause(game, source_id),
        );
        if !applied.applied {
            continue;
        }
        total_damage_dealt = total_damage_dealt.saturating_add(assignment.amount);
        if let DamageTarget::Player(target_player) = assignment.target {
            game.record_commander_damage(target_player, source_id, assignment.amount);
            if target_player == player_id {
                damage_to_original = damage_to_original.saturating_add(assignment.amount);
                life_lost_to_original = life_lost_to_original.saturating_add(applied.life_lost);
            }
        }
    }

    AppliedPlayerDamage {
        damage_dealt: damage_to_original,
        life_lost: life_lost_to_original,
        total_damage_dealt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::cause::CauseFilter;
    use crate::events::counters::matchers::WouldPutCountersMatcher;
    use crate::events::damage::matchers::DamageFromSourceMatcher;
    use crate::game_event::DamageTarget as EventDamageTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::rules::damage::DamageTarget;
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        power: i32,
        toughness: i32,
        controller: PlayerId,
        abilities: Vec<StaticAbility>,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        for ability in abilities {
            obj.abilities.push(Ability::static_ability(ability));
        }
        game.add_object(obj);
        id
    }

    fn add_doubling_season_like_effect(
        game: &mut GameState,
        controller: PlayerId,
        target: ObjectId,
    ) {
        let source = game.new_object_id();
        game.replacement_effects
            .add_resolution_effect(ReplacementEffect::with_matcher(
                source,
                controller,
                WouldPutCountersMatcher::new(
                    ObjectFilter::specific(target),
                    Some(CounterType::MinusOneMinusOne),
                )
                .with_cause_filter(CauseFilter::from_effect()),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ));
    }

    fn add_fiery_emancipation_like_effect(
        game: &mut GameState,
        controller: PlayerId,
        source: ObjectId,
    ) {
        game.replacement_effects
            .add_resolution_effect(ReplacementEffect::with_matcher(
                source,
                controller,
                DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ReplacementAction::Modify(EventModification::Multiply(3)),
            ));
    }

    #[test]
    fn combat_wither_damage_to_creature_ignores_effect_only_counter_doublers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = create_creature(
            &mut game,
            "Witherer",
            1,
            1,
            alice,
            vec![StaticAbility::wither()],
        );
        let blocker = create_creature(&mut game, "Blocker", 2, 2, bob, vec![]);
        add_doubling_season_like_effect(&mut game, bob, blocker);

        let damage_result = {
            let attacker_obj = game.object(attacker).expect("attacker exists");
            calculate_damage_with_game(&game, attacker_obj, DamageTarget::Permanent, 1, true)
        };
        assert_eq!(damage_result.minus_counters, 1);
        assert_eq!(damage_result.damage_dealt, 0);

        let applied = apply_damage_to_permanent(&mut game, blocker, attacker, &damage_result);

        assert_eq!(applied.damage_dealt, 1);
        assert_eq!(applied.total_damage_dealt, 1);
        assert_eq!(
            game.counter_count(blocker, CounterType::MinusOneMinusOne),
            1
        );
        assert_eq!(game.damage_on(blocker), 0);
    }

    #[test]
    fn combat_wither_damage_with_damage_tripler_still_skips_effect_only_counter_doublers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = create_creature(
            &mut game,
            "Witherer",
            1,
            1,
            alice,
            vec![StaticAbility::wither()],
        );
        let blocker = create_creature(&mut game, "Blocker", 6, 6, bob, vec![]);
        add_doubling_season_like_effect(&mut game, bob, blocker);
        add_fiery_emancipation_like_effect(&mut game, alice, attacker);

        let processed = crate::event_processor::process_damage_assignments_with_event(
            &mut game,
            attacker,
            EventDamageTarget::Object(blocker),
            1,
            true,
            crate::events::cause::EventCause::from_combat_damage(attacker, alice),
        );
        assert_eq!(processed.assignments.len(), 1);
        assert_eq!(processed.assignments[0].amount, 3);

        let damage_result = {
            let attacker_obj = game.object(attacker).expect("attacker exists");
            calculate_damage_with_game(&game, attacker_obj, DamageTarget::Permanent, 1, true)
        };
        assert_eq!(damage_result.minus_counters, 1);
        assert_eq!(damage_result.damage_dealt, 0);

        let applied = apply_damage_to_permanent(&mut game, blocker, attacker, &damage_result);

        assert_eq!(applied.damage_dealt, 3);
        assert_eq!(applied.total_damage_dealt, 3);
        assert_eq!(
            game.counter_count(blocker, CounterType::MinusOneMinusOne),
            3
        );
        assert_eq!(game.damage_on(blocker), 0);
    }

    #[test]
    fn combat_infect_damage_to_player_adds_poison_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = create_creature(
            &mut game,
            "Infector",
            1,
            1,
            alice,
            vec![StaticAbility::infect()],
        );

        let damage_result = {
            let attacker_obj = game.object(attacker).expect("attacker exists");
            calculate_damage_with_game(&game, attacker_obj, DamageTarget::Player(bob), 1, true)
        };
        assert_eq!(damage_result.poison_counters, 1);
        assert_eq!(damage_result.damage_dealt, 0);

        let applied = apply_damage_to_player(&mut game, bob, attacker, &damage_result);

        assert_eq!(applied.damage_dealt, 1);
        assert_eq!(applied.life_lost, 0);
        assert_eq!(applied.total_damage_dealt, 1);
        assert_eq!(game.player(bob).expect("player exists").poison_counters, 1);
    }
}
