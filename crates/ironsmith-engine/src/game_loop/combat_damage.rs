use super::*;

// ============================================================================
// Combat Damage
// ============================================================================

/// Combat damage event for trigger processing.
#[derive(Debug, Clone)]
pub struct CombatDamageEvent {
    /// Event-time characteristics when prevention follow-ups can move objects.
    pub source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    pub target_snapshot: Option<crate::snapshot::ObjectSnapshot>,
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
    /// Life gained by the source's controller because the source has lifelink
    /// (CR 702.15b). A source dealing damage to several recipients at once
    /// produces one life-gain event, carried on its first damage event.
    pub lifelink_gain: Option<(PlayerId, u32)>,
}

/// Why a proposed combat-damage assignment is illegal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombatDamageAssignmentErrorKind {
    /// Damage was assigned to an object that is not a current recipient.
    IllegalRecipient,
    /// The assigned amount does not equal the amount the source must assign.
    WrongTotal,
    /// Trample damage was assigned to the defender before every blocker had lethal damage assigned.
    TrampleBeforeLethal,
}

/// An illegal combat-damage assignment. The proposed assignments remain available
/// on the game state so the assigning player can make the whole choice again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatDamageAssignmentError {
    pub source: ObjectId,
    pub expected_total: u32,
    pub assigned_total: u32,
    pub illegal_recipients: Vec<ObjectId>,
    pub kind: CombatDamageAssignmentErrorKind,
}

impl std::fmt::Display for CombatDamageAssignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            CombatDamageAssignmentErrorKind::IllegalRecipient => write!(
                f,
                "combat damage from #{} was assigned to a nonrecipient",
                self.source.0
            ),
            CombatDamageAssignmentErrorKind::WrongTotal => write!(
                f,
                "combat damage from #{} assigned {} damage, but must assign {}",
                self.source.0, self.assigned_total, self.expected_total
            ),
            CombatDamageAssignmentErrorKind::TrampleBeforeLethal => write!(
                f,
                "combat damage from #{} assigned damage to the defender before assigning lethal damage to every blocker",
                self.source.0
            ),
        }
    }
}

impl std::error::Error for CombatDamageAssignmentError {}

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
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    try_execute_combat_damage_step_with_dm(game, combat, first_strike, &mut dm)
        .expect("illegal combat-damage assignment")
}

/// Execute combat damage, returning an error without changing an illegal
/// assignment so the assigning player can make the choice again.
pub fn try_execute_combat_damage_step(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    try_execute_combat_damage_step_with_dm(game, combat, first_strike, &mut dm)
}

/// Execute a combat-damage step with a decision maker for replacement and
/// prevention choices made across the simultaneous damage batch.
pub fn execute_combat_damage_step_with_dm(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Vec<CombatDamageEvent> {
    try_execute_combat_damage_step_with_dm(game, combat, first_strike, dm)
        .expect("illegal combat-damage assignment")
}

pub fn try_execute_combat_damage_step_with_dm(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    try_execute_combat_damage_step_with_dm_and_first_step_snapshot(
        game,
        combat,
        first_strike,
        None,
        dm,
    )
}

#[allow(dead_code)]
pub(crate) fn execute_combat_damage_step_with_first_step_snapshot(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: &std::collections::HashSet<ObjectId>,
) -> Vec<CombatDamageEvent> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    try_execute_combat_damage_step_with_dm_and_first_step_snapshot(
        game,
        combat,
        first_strike,
        Some(first_step_strikers),
        &mut dm,
    )
    .expect("illegal combat-damage assignment")
}

pub(crate) fn try_execute_combat_damage_step_with_first_step_snapshot(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: &std::collections::HashSet<ObjectId>,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    try_execute_combat_damage_step_with_dm_and_first_step_snapshot(
        game,
        combat,
        first_strike,
        Some(first_step_strikers),
        &mut dm,
    )
}

fn try_execute_combat_damage_step_with_dm_and_first_step_snapshot(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    crate::events::processing::with_deferred_prevention_follow_ups(game, dm, |game, dm| {
        let mut result = apply_combat_damage_step_with_dm_and_first_step_snapshot(
            game,
            combat,
            first_strike,
            first_step_strikers,
            dm,
        );
        if game
            .effect_store
            .prevention_effects
            .has_pending_follow_ups()
            && let Ok(events) = &mut result
        {
            for event in events.iter_mut().filter(|event| event.amount > 0) {
                event.source_snapshot = game.object(event.source).map(|obj| {
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        obj, game,
                    )
                });
                if let DamageEventTarget::Object(target) = event.target {
                    event.target_snapshot = game.object(target).map(|obj| {
                        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                    });
                }
            }
        }
        result
    })
}

fn apply_combat_damage_step_with_dm_and_first_step_snapshot(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    // Combat damage is simultaneous. Refresh once, then use a single immutable
    // characteristic view for the replacement/prevention-free common case so
    // damage applied by an earlier attacker cannot change a later attacker's
    // power or damage keywords within the same step.
    if game.continuous_state_is_clean() {
        // Damage processing historically rebuilt both trackers before every
        // assignment. Rebuild once even from an otherwise clean state before
        // deciding that the guarded fast path is legal, so an empty manager is
        // authoritative rather than a stale cache observation.
        game.update_cant_effects();
        game.update_replacement_effects();
    } else {
        // A full refresh also rebuilds both trackers after regenerating static
        // continuous effects.
        game.refresh_continuous_state();
    }
    if can_use_unblocked_player_damage_fast_path(game, combat) {
        return Ok(execute_unblocked_player_damage_fast_path(
            game,
            combat,
            first_strike,
            first_step_strikers,
        ));
    }
    if is_unblocked_player_damage_batch(combat) {
        return Ok(execute_unblocked_player_damage_batch_path(
            game,
            combat,
            first_strike,
            first_step_strikers,
            dm,
        ));
    }

    execute_general_combat_damage_batch_path(game, combat, first_strike, first_step_strikers, dm)
}

#[allow(dead_code)]
fn execute_legacy_general_combat_damage_step(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
) -> Vec<CombatDamageEvent> {
    let mut damage_events = Vec::new();

    // Process each attacker
    for attacker_info in &combat.attackers {
        let attacker_id = attacker_info.creature;

        if game.combat_damage_assignment_is_suppressed(attacker_id) {
            continue;
        }

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

        let controller = game.controller_of(attacker);

        if is_blocked(combat, attacker_id) {
            // Blocked attacker - deal damage to blockers
            let events =
                deal_damage_to_blockers(game, attacker_id, combat, combat_stat as u32, controller);
            damage_events.extend(events);
        } else {
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
    // Sorted by blocker: damage results are applied in this order.
    let mut blocker_groups = attackers_by_blocker.into_iter().collect::<Vec<_>>();
    blocker_groups.sort_by_key(|(blocker, _)| blocker.0);
    for (blocker_id, mut attacker_ids) in blocker_groups {
        if game.combat_damage_assignment_is_suppressed(blocker_id) {
            continue;
        }
        let Some(blocker) = game.object(blocker_id).cloned() else {
            continue;
        };

        let participates = if first_strike {
            deals_first_strike_damage_with_game(&blocker, game)
        } else {
            deals_regular_combat_damage_with_game(&blocker, game)
        };
        if !participates {
            continue;
        }

        let Some(combat_stat) = combat_damage_stat_for_creature(game, &blocker) else {
            continue;
        };
        if combat_stat <= 0 {
            continue;
        }

        // Deterministic default order when multiple attackers are blocked.
        attacker_ids.sort_by_key(|id| id.0);

        let controller = game.controller_of(&blocker);
        let explicit_assignments = game.take_combat_damage_assignments(blocker_id);
        if attacker_ids.len() == 1 {
            let attacker_id = attacker_ids[0];
            if game.object(attacker_id).is_none() {
                continue;
            }
            let dmg = explicit_assignments
                .get(&attacker_id)
                .copied()
                .unwrap_or(combat_stat as u32)
                .min(combat_stat as u32);
            let damage_result =
                calculate_damage_with_game(game, &blocker, DamageTarget::Permanent, dmg, true);
            if dmg > 0 {
                blocker_damage_info.push((blocker_id, attacker_id, controller, dmg, damage_result));
            }
            continue;
        }

        let recipients: Vec<&crate::object::Object> = attacker_ids
            .iter()
            .filter_map(|id| game.object(*id))
            .collect();
        if recipients.is_empty() {
            continue;
        }

        let distribution = if explicit_assignments.is_empty() {
            crate::rules::damage::distribute_combat_damage_to_creatures(
                &blocker,
                &recipients,
                combat_stat as u32,
                game,
            )
        } else {
            distribute_explicit_damage_to_creatures(
                game,
                &attacker_ids,
                &recipients,
                combat_stat as u32,
                &explicit_assignments,
            )
        };
        for (idx, (dmg, _is_lethal)) in distribution.into_iter().enumerate() {
            if dmg == 0 {
                continue;
            }
            let attacker_id = attacker_ids[idx];
            if game.object(attacker_id).is_none() {
                continue;
            }
            let damage_result =
                calculate_damage_with_game(game, &blocker, DamageTarget::Permanent, dmg, true);
            blocker_damage_info.push((blocker_id, attacker_id, controller, dmg, damage_result));
        }
    }

    // Now apply all blocker damage.
    for (blocker_id, attacker_id, controller, _assigned, damage_result) in blocker_damage_info {
        let applied = apply_damage_to_permanent(game, attacker_id, blocker_id, &damage_result);

        // Apply lifelink (through event processing)
        apply_combat_lifelink(game, controller, &damage_result, applied.total_damage_dealt);

        damage_events.push(CombatDamageEvent {
            source_snapshot: None,
            target_snapshot: None,
            source: blocker_id,
            target: DamageEventTarget::Object(attacker_id),
            amount: applied.damage_dealt,
            life_lost: 0,
            result: damage_result,
            lifelink_gain: None,
        });
    }

    damage_events
}

#[derive(Debug)]
struct PlannedCombatDamage {
    source: ObjectId,
    target: EventDamageTarget,
    controller: PlayerId,
    amount: u32,
    result: DamageResult,
    cause: crate::events::cause::EventCause,
}

fn combatant_participates_in_damage_step(
    game: &GameState,
    creature: &crate::object::Object,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
) -> bool {
    if first_strike {
        return deals_first_strike_damage_with_game(creature, game);
    }
    if let Some(first_step_strikers) = first_step_strikers {
        return !first_step_strikers.contains(&creature.id)
            || game.object_has_static_ability_id(
                creature.id,
                crate::static_abilities::StaticAbilityId::DoubleStrike,
            );
    }
    deals_regular_combat_damage_with_game(creature, game)
}

fn plan_general_combat_damage(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
) -> Result<Vec<PlannedCombatDamage>, CombatDamageAssignmentError> {
    let mut planned = Vec::new();
    // CR 702.19b: lethal-damage checks for a trampler count the damage other
    // attackers assign to the same blockers in this step. Divisions are
    // consumed below, so keep every recorded one, plus each division as it's
    // planned.
    let (attacker_assigners, _) =
        combat_damage_assigners(game, combat, first_strike, first_step_strikers);
    let mut known_divisions = game.turn_store.combat_damage_assignments.clone();

    for attacker_info in &combat.attackers {
        let attacker_id = attacker_info.creature;
        if game.combat_damage_assignment_is_suppressed(attacker_id) {
            continue;
        }
        let Some(attacker) = game.object(attacker_id).cloned() else {
            continue;
        };
        let participates = combatant_participates_in_damage_step(
            game,
            &attacker,
            first_strike,
            first_step_strikers,
        );
        if !participates {
            continue;
        }
        let Some(combat_stat) = combat_damage_stat_for_creature(game, &attacker) else {
            continue;
        };
        if combat_stat <= 0 {
            continue;
        }
        let total = combat_stat as u32;
        let controller = game.controller_of(&attacker);
        let cause = combat_damage_cause(game, attacker_id);

        // CR 510.1b-c, 702.19b-e: what this attacker may assign damage to. A
        // blocked non-trampler with no creatures blocking it, or a creature
        // attacking nothing, has nowhere to assign damage.
        let Some(division) = attacker_damage_division(game, combat, attacker_info) else {
            continue;
        };
        let explicit_assignments = game.take_combat_damage_assignments(attacker_id);
        let others = simultaneous_combat_damage(&attacker_assigners, attacker_id, &known_divisions);
        let allocation = if !explicit_assignments.is_empty() {
            division
                .check(
                    game,
                    total,
                    &division.allocations_from_record(total, &explicit_assignments),
                    &others,
                )
                .map_err(|(kind, _)| {
                    let illegal = if kind == CombatDamageAssignmentErrorKind::IllegalRecipient {
                        illegal_assignment_recipients(
                            &division.damageable_objects(),
                            &explicit_assignments,
                        )
                    } else {
                        vec![]
                    };
                    assignment_error(
                        attacker_id,
                        total,
                        assignment_total(&explicit_assignments),
                        illegal,
                        kind,
                    )
                })?
        } else if let Some(target) = division.forced_target() {
            vec![(target, total)]
        } else {
            // Without a recorded choice, a trampler spreads lethal damage
            // first; a plain division keeps the historical all-to-first
            // default.
            division.default_allocation(game, total, &others, division.may_trample())
        };
        known_divisions.insert(
            attacker_id,
            allocation
                .iter()
                .filter_map(|(target, amount)| match target {
                    Target::Object(object) => Some((*object, *amount)),
                    Target::Player(_) => None,
                })
                .collect(),
        );
        for (target, amount) in allocation {
            if amount == 0 {
                continue;
            }
            let (event_target, rules_target) = match target {
                Target::Object(object) => {
                    (EventDamageTarget::Object(object), DamageTarget::Permanent)
                }
                Target::Player(player) => (
                    EventDamageTarget::Player(player),
                    DamageTarget::Player(player),
                ),
            };
            planned.push(PlannedCombatDamage {
                source: attacker_id,
                target: event_target,
                controller,
                amount,
                result: calculate_damage_with_game(game, &attacker, rules_target, amount, true),
                cause: cause.clone(),
            });
        }
    }

    let mut attackers_by_blocker: std::collections::HashMap<ObjectId, Vec<ObjectId>> =
        std::collections::HashMap::new();
    for (attacker, blockers) in &combat.blockers {
        for blocker in blockers {
            attackers_by_blocker
                .entry(*blocker)
                .or_default()
                .push(*attacker);
        }
    }
    let mut blocker_groups = attackers_by_blocker.into_iter().collect::<Vec<_>>();
    blocker_groups.sort_by_key(|(blocker, _)| blocker.0);
    for (blocker_id, mut attacker_ids) in blocker_groups {
        if game.combat_damage_assignment_is_suppressed(blocker_id) {
            continue;
        }
        let Some(blocker) = game.object(blocker_id).cloned() else {
            continue;
        };
        let participates = combatant_participates_in_damage_step(
            game,
            &blocker,
            first_strike,
            first_step_strikers,
        );
        if !participates {
            continue;
        }
        let Some(combat_stat) = combat_damage_stat_for_creature(game, &blocker) else {
            continue;
        };
        if combat_stat <= 0 {
            continue;
        }
        attacker_ids.sort_by_key(|id| id.0);
        attacker_ids.retain(|id| game.object(*id).is_some());
        if attacker_ids.is_empty() {
            continue;
        }
        let explicit_assignments = game.take_combat_damage_assignments(blocker_id);
        let distribution = if attacker_ids.len() == 1 {
            if explicit_assignments.is_empty() {
                vec![(combat_stat as u32, false)]
            } else {
                validate_nontrample_damage_assignment(
                    blocker_id,
                    &attacker_ids,
                    combat_stat as u32,
                    &explicit_assignments,
                )?
            }
        } else if explicit_assignments.is_empty() {
            default_combat_damage_distribution(attacker_ids.len(), combat_stat as u32)
        } else {
            validate_nontrample_damage_assignment(
                blocker_id,
                &attacker_ids,
                combat_stat as u32,
                &explicit_assignments,
            )?
        };
        let controller = game.controller_of(&blocker);
        let cause = combat_damage_cause(game, blocker_id);
        for (index, (amount, _)) in distribution.into_iter().enumerate() {
            if amount == 0 {
                continue;
            }
            planned.push(PlannedCombatDamage {
                source: blocker_id,
                target: EventDamageTarget::Object(attacker_ids[index]),
                controller,
                amount,
                result: calculate_damage_with_game(
                    game,
                    &blocker,
                    DamageTarget::Permanent,
                    amount,
                    true,
                ),
                cause: cause.clone(),
            });
        }
    }

    Ok(planned)
}

/// The damage recipient for an attacker's damage to what it is attacking, or
/// `None` when nothing can be assigned (CR 510.1b, 800.4e).
fn attack_target_damage_recipient(
    game: &GameState,
    target: &AttackTarget,
) -> Option<(EventDamageTarget, DamageTarget)> {
    match *target {
        AttackTarget::Player(player) => game
            .player(player)
            .is_some_and(|candidate| candidate.is_in_game())
            .then_some((
                EventDamageTarget::Player(player),
                DamageTarget::Player(player),
            )),
        AttackTarget::Planeswalker(object) | AttackTarget::Battle(object) => game
            .object(object)
            .is_some_and(|candidate| candidate.zone == crate::zone::Zone::Battlefield)
            .then_some((EventDamageTarget::Object(object), DamageTarget::Permanent)),
        // CR 506.4c / 510.1b: a creature attacking nothing assigns no combat
        // damage to anything but its blockers.
        AttackTarget::Nothing { .. } => None,
    }
}

fn execute_general_combat_damage_batch_path(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    let assignments_checkpoint = game.turn_store.combat_damage_assignments.clone();
    let planned = match plan_general_combat_damage(game, combat, first_strike, first_step_strikers)
    {
        Ok(planned) => planned,
        Err(error) => {
            game.turn_store.combat_damage_assignments = assignments_checkpoint;
            return Err(error);
        }
    };
    let batch = planned
        .iter()
        .map(
            |planned| crate::events::processing::SimultaneousDamageEvent {
                source: planned.source,
                target: planned.target,
                amount: planned.amount,
                is_combat: true,
                unpreventable: false,
                cause: planned.cause.clone(),
                source_snapshot: None,
            },
        )
        .collect::<Vec<_>>();
    let processed =
        crate::events::processing::process_simultaneous_damage_assignments_with_event_with_dm(
            game, &batch, dm,
        );

    let mut events = Vec::with_capacity(planned.len());
    let mut lifelink_totals = CombatLifelinkTotals::default();
    for (planned, processed) in planned.into_iter().zip(processed) {
        let keywords = crate::rules::damage::SourceDamageKeywords {
            has_deathtouch: planned.result.has_deathtouch,
            has_infect: planned.result.has_infect,
            has_wither: planned.result.has_wither,
            has_lifelink: planned.result.has_lifelink,
        };
        let mut damage_to_original = 0u32;
        let mut life_lost_to_original = 0u32;
        let mut total_damage_dealt = 0u32;
        let mut redirected = Vec::new();
        if !processed.replacement_prevented {
            for assignment in processed.assignments {
                let applied = crate::rules::damage::apply_processed_damage_assignment(
                    game,
                    planned.source,
                    assignment.target,
                    assignment.amount,
                    keywords,
                    planned.cause.clone(),
                );
                if !applied.applied {
                    continue;
                }
                total_damage_dealt = total_damage_dealt.saturating_add(assignment.amount);
                if let EventDamageTarget::Player(player) = assignment.target {
                    game.record_commander_damage(player, planned.source, assignment.amount);
                    apply_combat_toxic(game, planned.source, planned.controller, player);
                }
                if assignment.target == planned.target {
                    damage_to_original = damage_to_original.saturating_add(assignment.amount);
                    life_lost_to_original = life_lost_to_original.saturating_add(applied.life_lost);
                } else {
                    redirected.push((assignment.target, assignment.amount, applied.life_lost));
                }
            }
        }
        lifelink_totals.record(
            planned.source,
            planned.controller,
            planned.result.has_lifelink,
            total_damage_dealt,
            events.len(),
        );
        let event_target = match planned.target {
            EventDamageTarget::Player(player) => DamageEventTarget::Player(player),
            EventDamageTarget::Object(object) => DamageEventTarget::Object(object),
        };
        events.push(CombatDamageEvent {
            source_snapshot: None,
            target_snapshot: None,
            source: planned.source,
            target: event_target,
            amount: damage_to_original,
            life_lost: life_lost_to_original,
            result: planned.result.clone(),
            lifelink_gain: None,
        });
        push_redirected_combat_damage_events(&mut events, &planned.result, planned.source, redirected);
    }
    lifelink_totals.apply(game, &mut events);
    Ok(events)
}

#[derive(Debug)]
struct PlannedUnblockedPlayerDamage {
    source: ObjectId,
    target: PlayerId,
    controller: PlayerId,
    amount: u32,
    result: DamageResult,
    cause: crate::events::cause::EventCause,
}

#[derive(Debug, Default)]
struct ToughnessCombatDamageSources {
    all_creatures: bool,
    controllers: std::collections::HashSet<PlayerId>,
    individual_sources: std::collections::HashSet<ObjectId>,
}

impl ToughnessCombatDamageSources {
    fn from_view(game: &GameState, view: &crate::derived_view::DerivedGameView<'_>) -> Self {
        let mut sources = Self::default();
        for &source_id in &game.battlefield {
            if game.object(source_id).is_none() {
                continue;
            }
            let Some(characteristics) = view.calculated_characteristics_arc(source_id) else {
                continue;
            };
            let controller = characteristics.controller;
            for ability in &characteristics.static_abilities {
                match ability.id() {
                    crate::static_abilities::StaticAbilityId::ThisCreatureAssignsCombatDamageUsingToughness => {
                        sources.individual_sources.insert(source_id);
                    }
                    crate::static_abilities::StaticAbilityId::CreaturesAssignCombatDamageUsingToughness => {
                        sources.all_creatures = true;
                    }
                    crate::static_abilities::StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness => {
                        sources.controllers.insert(controller);
                    }
                    _ => {}
                }
            }
        }
        sources
    }

    fn applies_to(&self, source: ObjectId, controller: PlayerId) -> bool {
        self.all_creatures
            || self.individual_sources.contains(&source)
            || self.controllers.contains(&controller)
    }
}

fn can_use_unblocked_player_damage_fast_path(game: &GameState, combat: &CombatState) -> bool {
    game.effect_store.replacement_effects.effects().is_empty()
        && game.effect_store.prevention_effects.shields().is_empty()
        && game.effect_store.pending_replacement_choice.is_none()
        && is_unblocked_player_damage_batch(combat)
}

fn is_unblocked_player_damage_batch(combat: &CombatState) -> bool {
    // CR 509.1h / 506.4: an attacker stays blocked after its blockers are
    // removed, so remembered blocks must take the general path.
    combat.blockers.values().all(Vec::is_empty)
        && combat.attackers.iter().all(|attacker| {
            matches!(attacker.target, AttackTarget::Player(_))
                && !is_blocked(combat, attacker.creature)
        })
}

fn plan_unblocked_player_damage(
    game: &GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
) -> Vec<PlannedUnblockedPlayerDamage> {
    // Attachment metadata can be populated while constructing a combat
    // scenario without going through the mutation path that marks the cached
    // continuous state dirty.  Rebuild the view from current effects here so
    // granted combat abilities (such as first strike) cannot come from a stale
    // cache.
    let view =
        crate::derived_view::DerivedGameView::from_effects(game, game.all_continuous_effects());
    view.prewarm_characteristics(&game.battlefield);
    let toughness_sources = ToughnessCombatDamageSources::from_view(game, &view);
    let mut planned = Vec::with_capacity(combat.attackers.len());

    for attacker_info in &combat.attackers {
        let attacker_id = attacker_info.creature;
        if game.combat_damage_assignment_is_suppressed(attacker_id) {
            continue;
        }
        let AttackTarget::Player(target) = &attacker_info.target else {
            unreachable!("fast path only accepts attackers targeting players");
        };
        let target = *target;
        if !game
            .player(target)
            .is_some_and(|player| player.is_in_game())
        {
            continue;
        }
        let Some(attacker) = game.object(attacker_id) else {
            continue;
        };
        let Some(characteristics) = view.calculated_characteristics_arc(attacker_id) else {
            continue;
        };

        let has_first_strike = characteristics
            .static_abilities
            .iter()
            .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::FirstStrike);
        let has_double_strike = characteristics
            .static_abilities
            .iter()
            .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::DoubleStrike);
        let participates = if first_strike {
            has_first_strike || has_double_strike
        } else if let Some(first_step_strikers) = first_step_strikers {
            !first_step_strikers.contains(&attacker_id) || has_double_strike
        } else {
            !has_first_strike || has_double_strike
        };
        if !participates {
            continue;
        }

        let controller = characteristics.controller;
        let combat_stat = if toughness_sources.applies_to(attacker_id, controller) {
            characteristics.toughness.or_else(|| attacker.toughness())
        } else {
            characteristics.power.or_else(|| attacker.power())
        };
        let Some(combat_stat) = combat_stat.filter(|stat| *stat > 0) else {
            continue;
        };
        let amount = combat_stat as u32;

        let has_deathtouch = view.object_has_static_ability_id(
            attacker_id,
            crate::static_abilities::StaticAbilityId::Deathtouch,
        );
        let has_infect = view.object_has_static_ability_id(
            attacker_id,
            crate::static_abilities::StaticAbilityId::Infect,
        );
        let has_wither = view.object_has_static_ability_id(
            attacker_id,
            crate::static_abilities::StaticAbilityId::Wither,
        );
        let has_lifelink = view.object_has_static_ability_id(
            attacker_id,
            crate::static_abilities::StaticAbilityId::Lifelink,
        );
        let result = DamageResult {
            damage_dealt: if has_infect { 0 } else { amount },
            life_gained: if has_lifelink { amount } else { 0 },
            poison_counters: if has_infect { amount } else { 0 },
            has_deathtouch,
            has_infect,
            has_wither,
            has_lifelink,
            ..DamageResult::default()
        };
        planned.push(PlannedUnblockedPlayerDamage {
            source: attacker_id,
            target,
            controller,
            amount,
            result,
            cause: crate::events::cause::EventCause::from_combat_damage(attacker_id, controller),
        });
    }
    planned
}

fn execute_unblocked_player_damage_fast_path(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
) -> Vec<CombatDamageEvent> {
    let planned = plan_unblocked_player_damage(game, combat, first_strike, first_step_strikers);

    let events = planned
        .into_iter()
        .map(|planned| apply_planned_unblocked_player_damage(game, planned))
        .collect();

    // Damage/life/counter application dirties derived state. The caller checks
    // one trigger event per assignment immediately after this function returns;
    // make those checks share one refreshed, prewarmed state instead of each
    // falling back to dirty single-object characteristic calculation.
    game.refresh_continuous_state();
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    view.prewarm_characteristics(&game.battlefield);
    events
}

fn execute_unblocked_player_damage_batch_path(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Vec<CombatDamageEvent> {
    let planned = plan_unblocked_player_damage(game, combat, first_strike, first_step_strikers);
    let batch = planned
        .iter()
        .map(
            |planned| crate::events::processing::SimultaneousDamageEvent {
                source: planned.source,
                target: crate::events::DamageTarget::Player(planned.target),
                amount: planned.amount,
                is_combat: true,
                unpreventable: false,
                cause: planned.cause.clone(),
                source_snapshot: None,
            },
        )
        .collect::<Vec<_>>();
    let processed =
        crate::events::processing::process_simultaneous_damage_assignments_with_event_with_dm(
            game, &batch, dm,
        );

    // Replacement/prevention is collected for the entire batch first. Only
    // after every source has a final assignment do we commit actual damage.
    let mut events = Vec::with_capacity(planned.len());
    let mut lifelink_totals = CombatLifelinkTotals::default();
    for (planned, processed) in planned.into_iter().zip(processed) {
        let keywords = crate::rules::damage::SourceDamageKeywords {
            has_deathtouch: planned.result.has_deathtouch,
            has_infect: planned.result.has_infect,
            has_wither: planned.result.has_wither,
            has_lifelink: planned.result.has_lifelink,
        };
        let mut damage_to_original = 0u32;
        let mut life_lost_to_original = 0u32;
        let mut total_damage_dealt = 0u32;
        let mut redirected = Vec::new();
        if !processed.replacement_prevented {
            for assignment in processed.assignments {
                let applied = crate::rules::damage::apply_processed_damage_assignment(
                    game,
                    planned.source,
                    assignment.target,
                    assignment.amount,
                    keywords,
                    planned.cause.clone(),
                );
                if !applied.applied {
                    continue;
                }
                total_damage_dealt = total_damage_dealt.saturating_add(assignment.amount);
                if let crate::events::DamageTarget::Player(player) = assignment.target {
                    game.record_commander_damage(player, planned.source, assignment.amount);
                    apply_combat_toxic(game, planned.source, planned.controller, player);
                }
                if assignment.target == crate::events::DamageTarget::Player(planned.target) {
                    damage_to_original = damage_to_original.saturating_add(assignment.amount);
                    life_lost_to_original = life_lost_to_original.saturating_add(applied.life_lost);
                } else {
                    redirected.push((assignment.target, assignment.amount, applied.life_lost));
                }
            }
        }
        lifelink_totals.record(
            planned.source,
            planned.controller,
            planned.result.has_lifelink,
            total_damage_dealt,
            events.len(),
        );
        events.push(CombatDamageEvent {
            source_snapshot: None,
            target_snapshot: None,
            source: planned.source,
            target: DamageEventTarget::Player(planned.target),
            amount: damage_to_original,
            life_lost: life_lost_to_original,
            result: planned.result.clone(),
            lifelink_gain: None,
        });
        push_redirected_combat_damage_events(&mut events, &planned.result, planned.source, redirected);
    }
    lifelink_totals.apply(game, &mut events);

    game.refresh_continuous_state();
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    view.prewarm_characteristics(&game.battlefield);
    events
}

fn apply_planned_unblocked_player_damage(
    game: &mut GameState,
    planned: PlannedUnblockedPlayerDamage,
) -> CombatDamageEvent {
    // The normal replacement pipeline allocates one provenance root before it
    // discovers that no effect applies. Preserve that deterministic graph
    // progression even though this guarded path can skip event processing.
    let _damage_provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::Damage);
    let keywords = crate::rules::damage::SourceDamageKeywords {
        has_deathtouch: planned.result.has_deathtouch,
        has_infect: planned.result.has_infect,
        has_wither: planned.result.has_wither,
        has_lifelink: planned.result.has_lifelink,
    };
    let applied = crate::rules::damage::apply_processed_damage_assignment(
        game,
        planned.source,
        crate::events::DamageTarget::Player(planned.target),
        planned.amount,
        keywords,
        planned.cause,
    );
    let total_damage_dealt = if applied.applied { planned.amount } else { 0 };
    if applied.applied {
        game.record_commander_damage(planned.target, planned.source, planned.amount);
        apply_combat_toxic(game, planned.source, planned.controller, planned.target);
    }
    let lifelink_gain = apply_combat_lifelink(
        game,
        planned.controller,
        &planned.result,
        total_damage_dealt,
    )
    .map(|gained| (planned.controller, gained));

    CombatDamageEvent {
        source_snapshot: None,
        target_snapshot: None,
        source: planned.source,
        target: DamageEventTarget::Player(planned.target),
        amount: total_damage_dealt,
        life_lost: applied.life_lost,
        result: planned.result,
        lifelink_gain,
    }
}

pub(super) fn static_abilities_for_object(
    game: &GameState,
    object: &crate::object::Object,
) -> Vec<crate::static_abilities::StaticAbility> {
    game.calculated_characteristics(object.id)
        .map(|characteristics| characteristics.static_abilities.to_vec())
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
                crate::static_abilities::StaticAbilityId::ThisCreatureAssignsCombatDamageUsingToughness => {
                    if source_id == creature.id {
                        return true;
                    }
                }
                crate::static_abilities::StaticAbilityId::CreaturesAssignCombatDamageUsingToughness => {
                    return true;
                }
                crate::static_abilities::StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness => {
                    if game.controller_of(source) == game.controller_of(creature) {
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

/// Returns the life actually gained, if any, so the caller can emit the
/// life-gain event that "whenever you gain life" abilities watch for.
pub(super) fn apply_combat_lifelink(
    game: &mut GameState,
    controller: PlayerId,
    damage_result: &DamageResult,
    total_damage_dealt: u32,
) -> Option<u32> {
    if !damage_result.has_lifelink || total_damage_dealt == 0 {
        return None;
    }

    let life_to_gain = crate::events::processing::process_life_gain_with_event(
        game,
        controller,
        total_damage_dealt,
    );
    if life_to_gain == 0 {
        return None;
    }
    let gained = game.gain_life(controller, life_to_gain);
    (gained > 0).then_some(gained)
}

/// CR 702.164c: combat damage dealt to a player by a creature with toxic
/// causes that creature's controller to give the player poison counters equal
/// to its total toxic value, in addition to the damage's other results.
fn apply_combat_toxic(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    player: PlayerId,
) {
    let Some(source_object) = game.object(source) else {
        return;
    };
    let toxic: u32 = static_abilities_for_object(game, source_object)
        .iter()
        .filter_map(crate::static_abilities::StaticAbility::toxic_amount)
        .fold(0u32, u32::saturating_add);
    if toxic == 0 {
        return;
    }
    if let Some(event) = game.add_player_counters_with_source(
        player,
        crate::object::CounterType::Poison,
        toxic,
        Some(source),
        Some(controller),
    ) {
        game.queue_trigger_event(event.provenance(), event);
    }
}

/// Per-source lifelink totals for one simultaneous combat-damage batch.
///
/// CR 702.15b / 120.3f: a lifelink source that deals damage to several
/// recipients at once causes a single life gain equal to the total.
/// CR 614.9 / 510.2: damage a replacement effect redirected to another
/// recipient is still combat damage dealt by the source, so each applied
/// redirected assignment gets its own event (damage and, for players, life
/// loss triggers and turn history see it).
fn push_redirected_combat_damage_events(
    events: &mut Vec<CombatDamageEvent>,
    result: &DamageResult,
    source: ObjectId,
    redirected: Vec<(crate::events::DamageTarget, u32, u32)>,
) {
    for (target, amount, life_lost) in redirected {
        if amount == 0 {
            continue;
        }
        let target = match target {
            crate::events::DamageTarget::Player(player) => DamageEventTarget::Player(player),
            crate::events::DamageTarget::Object(object) => DamageEventTarget::Object(object),
        };
        events.push(CombatDamageEvent {
            source_snapshot: None,
            target_snapshot: None,
            source,
            target,
            amount,
            life_lost,
            result: result.clone(),
            lifelink_gain: None,
        });
    }
}

#[derive(Default)]
struct CombatLifelinkTotals {
    /// (source, controller, total damage dealt, index of the source's first event)
    sources: Vec<(ObjectId, PlayerId, u32, usize)>,
}

impl CombatLifelinkTotals {
    fn record(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        has_lifelink: bool,
        damage_dealt: u32,
        event_index: usize,
    ) {
        if !has_lifelink {
            return;
        }
        if let Some(entry) = self.sources.iter_mut().find(|entry| entry.0 == source) {
            entry.2 = entry.2.saturating_add(damage_dealt);
        } else {
            self.sources
                .push((source, controller, damage_dealt, event_index));
        }
    }

    fn apply(self, game: &mut GameState, events: &mut [CombatDamageEvent]) {
        for (_source, controller, total, event_index) in self.sources {
            let Some(event) = events.get_mut(event_index) else {
                continue;
            };
            let result = DamageResult {
                has_lifelink: true,
                ..DamageResult::default()
            };
            event.lifelink_gain = apply_combat_lifelink(game, controller, &result, total)
                .map(|gained| (controller, gained));
        }
    }
}

fn combat_damage_cause(game: &GameState, source_id: ObjectId) -> crate::events::cause::EventCause {
    game.object(source_id)
        .map(|obj| {
            crate::events::cause::EventCause::from_combat_damage(
                source_id,
                game.current_controller(source_id)
                    .unwrap_or_else(|| game.controller_of(obj)),
            )
        })
        .unwrap_or_else(|| crate::events::cause::EventCause::combat_damage(source_id))
}

fn combat_damage_amount_to_permanent(result: &DamageResult) -> u32 {
    result.damage_dealt.max(result.minus_counters)
}

fn default_combat_damage_distribution(recipients: usize, total_damage: u32) -> Vec<(u32, bool)> {
    (0..recipients)
        .map(|index| (if index == 0 { total_damage } else { 0 }, false))
        .collect()
}

fn assignment_error(
    source: ObjectId,
    expected_total: u32,
    assigned_total: u32,
    illegal_recipients: Vec<ObjectId>,
    kind: CombatDamageAssignmentErrorKind,
) -> CombatDamageAssignmentError {
    CombatDamageAssignmentError {
        source,
        expected_total,
        assigned_total,
        illegal_recipients,
        kind,
    }
}

fn assignment_total(assignments: &std::collections::HashMap<ObjectId, u32>) -> u32 {
    assignments
        .values()
        .copied()
        .fold(0u32, u32::saturating_add)
}

fn illegal_assignment_recipients(
    recipient_ids: &[ObjectId],
    assignments: &std::collections::HashMap<ObjectId, u32>,
) -> Vec<ObjectId> {
    let mut illegal = assignments
        .keys()
        .copied()
        .filter(|recipient| !recipient_ids.contains(recipient))
        .collect::<Vec<_>>();
    illegal.sort_by_key(|id| id.0);
    illegal
}

fn validate_nontrample_damage_assignment(
    source: ObjectId,
    recipient_ids: &[ObjectId],
    total_damage: u32,
    explicit_assignments: &std::collections::HashMap<ObjectId, u32>,
) -> Result<Vec<(u32, bool)>, CombatDamageAssignmentError> {
    let assigned_total = assignment_total(explicit_assignments);
    let illegal_recipients = illegal_assignment_recipients(recipient_ids, explicit_assignments);
    if !illegal_recipients.is_empty() {
        return Err(assignment_error(
            source,
            total_damage,
            assigned_total,
            illegal_recipients,
            CombatDamageAssignmentErrorKind::IllegalRecipient,
        ));
    }
    if assigned_total != total_damage {
        return Err(assignment_error(
            source,
            total_damage,
            assigned_total,
            vec![],
            CombatDamageAssignmentErrorKind::WrongTotal,
        ));
    }
    Ok(recipient_ids
        .iter()
        .map(|recipient| {
            (
                explicit_assignments.get(recipient).copied().unwrap_or(0),
                false,
            )
        })
        .collect())
}

// Superseded by `CombatDamageDivision::check`; kept for the retired path.
#[allow(dead_code)]
fn validate_attacker_damage_assignment(
    game: &GameState,
    attacker: &crate::object::Object,
    blocker_ids: &[ObjectId],
    blockers: &[&crate::object::Object],
    total_damage: u32,
    explicit_assignments: &std::collections::HashMap<ObjectId, u32>,
    others: &std::collections::HashMap<ObjectId, SimultaneousCombatDamage>,
) -> Result<(Vec<(u32, bool)>, u32), CombatDamageAssignmentError> {
    let has_trample = game.object_has_static_ability_id(
        attacker.id,
        crate::static_abilities::StaticAbilityId::Trample,
    );
    let has_deathtouch = game.object_has_static_ability_id(
        attacker.id,
        crate::static_abilities::StaticAbilityId::Deathtouch,
    );
    if !has_trample {
        return validate_nontrample_damage_assignment(
            attacker.id,
            blocker_ids,
            total_damage,
            explicit_assignments,
        )
        .map(|distribution| (distribution, 0));
    }

    let assigned_total = assignment_total(explicit_assignments);
    let illegal_recipients = illegal_assignment_recipients(blocker_ids, explicit_assignments);
    if !illegal_recipients.is_empty() {
        return Err(assignment_error(
            attacker.id,
            total_damage,
            assigned_total,
            illegal_recipients,
            CombatDamageAssignmentErrorKind::IllegalRecipient,
        ));
    }
    if assigned_total > total_damage {
        return Err(assignment_error(
            attacker.id,
            total_damage,
            assigned_total,
            vec![],
            CombatDamageAssignmentErrorKind::WrongTotal,
        ));
    }

    let mut distribution = Vec::with_capacity(blockers.len());

    for &blocker_id in blocker_ids.iter().take(blockers.len()) {
        // CR 702.19b: marked damage and other creatures' assignments in this
        // step count toward lethal; CR 702.2c: deathtouch makes 1 lethal.
        let lethal = remaining_lethal_damage(
            game,
            blocker_id,
            has_deathtouch,
            others.get(&blocker_id).copied().unwrap_or_default(),
        );
        let damage_to_blocker = explicit_assignments.get(&blocker_id).copied().unwrap_or(0);
        if assigned_total < total_damage && damage_to_blocker < lethal {
            return Err(assignment_error(
                attacker.id,
                total_damage,
                assigned_total,
                vec![],
                CombatDamageAssignmentErrorKind::TrampleBeforeLethal,
            ));
        }
        distribution.push((damage_to_blocker, damage_to_blocker >= lethal && lethal > 0));
    }

    Ok((distribution, total_damage - assigned_total))
}

// Compatibility helpers for the retired sequential combat-damage path below.
// The live simultaneous path validates assignments before applying anything.
#[allow(dead_code)]
fn distribute_explicit_trample_damage(
    game: &GameState,
    attacker: &crate::object::Object,
    blocker_ids: &[ObjectId],
    blockers: &[&crate::object::Object],
    total_damage: u32,
    explicit_assignments: &std::collections::HashMap<ObjectId, u32>,
) -> (Vec<(u32, bool)>, u32) {
    validate_attacker_damage_assignment(
        game,
        attacker,
        blocker_ids,
        blockers,
        total_damage,
        explicit_assignments,
        &std::collections::HashMap::new(),
    )
    .unwrap_or_else(|_| {
        (
            default_combat_damage_distribution(blockers.len(), total_damage),
            0,
        )
    })
}

#[allow(dead_code)]
fn distribute_explicit_damage_to_creatures(
    _game: &GameState,
    recipient_ids: &[ObjectId],
    _recipients: &[&crate::object::Object],
    total_damage: u32,
    explicit_assignments: &std::collections::HashMap<ObjectId, u32>,
) -> Vec<(u32, bool)> {
    recipient_ids
        .first()
        .and_then(|source| {
            validate_nontrample_damage_assignment(
                *source,
                recipient_ids,
                total_damage,
                explicit_assignments,
            )
            .ok()
        })
        .unwrap_or_else(|| default_combat_damage_distribution(recipient_ids.len(), total_damage))
}

#[allow(dead_code)]
fn distribute_explicit_damage_among_blockers_as_chosen(
    blocker_ids: &[ObjectId],
    blockers_len: usize,
    total_damage: u32,
    explicit_assignments: &std::collections::HashMap<ObjectId, u32>,
) -> Vec<(u32, bool)> {
    blocker_ids
        .first()
        .and_then(|source| {
            validate_nontrample_damage_assignment(
                *source,
                &blocker_ids[..blockers_len],
                total_damage,
                explicit_assignments,
            )
            .ok()
        })
        .unwrap_or_else(|| default_combat_damage_distribution(blockers_len, total_damage))
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

    let explicit_assignments = game.take_combat_damage_assignments(attacker_id);

    // Get blocker objects for distribution calculation
    let blockers: Vec<&crate::object::Object> = blocker_ids
        .iter()
        .filter_map(|&id| game.object(id))
        .collect();

    let Some(attacker) = game.object(attacker_id) else {
        return events;
    };

    // Calculate damage distribution (handles trample and explicit assignment choices)
    let defender_assigns_damage =
        defender_assigns_combat_damage_for_attacker(game, combat, attacker_id);
    let (distribution, excess) = if explicit_assignments.is_empty() {
        distribute_trample_damage(attacker, &blockers, total_damage, game)
    } else if defender_assigns_damage {
        (
            distribute_explicit_damage_among_blockers_as_chosen(
                &blocker_ids,
                blockers.len(),
                total_damage,
                &explicit_assignments,
            ),
            0,
        )
    } else {
        distribute_explicit_trample_damage(
            game,
            attacker,
            &blocker_ids,
            &blockers,
            total_damage,
            &explicit_assignments,
        )
    };

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
            game.player(player_id)
                .is_some_and(|player| player.is_in_game())
                .then(|| {
                    (
                        player_id,
                        calculate_damage_with_game(
                            game,
                            attacker,
                            DamageTarget::Player(player_id),
                            excess,
                            true,
                        ),
                    )
                })
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
            source_snapshot: None,
            target_snapshot: None,
            source: attacker_id,
            target: DamageEventTarget::Object(blocker_id),
            amount: applied.damage_dealt,
            life_lost: 0,
            result: damage_result,
            lifelink_gain: None,
        });
    }

    // Apply excess damage to defending player (trample)
    if let Some((player_id, damage_result)) = excess_damage_result {
        let applied = apply_damage_to_player(game, player_id, attacker_id, &damage_result);

        // Apply lifelink (through event processing)
        apply_combat_lifelink(game, controller, &damage_result, applied.total_damage_dealt);

        events.push(CombatDamageEvent {
            source_snapshot: None,
            target_snapshot: None,
            source: attacker_id,
            target: DamageEventTarget::Player(player_id),
            amount: applied.damage_dealt,
            life_lost: applied.life_lost,
            result: damage_result,
            lifelink_gain: None,
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
    let controller = game.controller_of(attacker);

    match target {
        AttackTarget::Player(player_id) => {
            if !game
                .player(*player_id)
                .is_some_and(|player| player.is_in_game())
            {
                return None;
            }
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
                source_snapshot: None,
                target_snapshot: None,
                source: attacker_id,
                target: DamageEventTarget::Player(*player_id),
                amount: applied.damage_dealt,
                life_lost: applied.life_lost,
                result: damage_result,
                lifelink_gain: None,
            })
        }
        AttackTarget::Planeswalker(pw_id) | AttackTarget::Battle(pw_id) => {
            use crate::events::DamageTarget as EventDamageTarget;
            use crate::events::processing::process_damage_assignments_with_event;

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
                                if object_id == *pw_id {
                                    final_damage = final_damage.saturating_add(assignment.amount);
                                }
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
                source_snapshot: None,
                target_snapshot: None,
                source: attacker_id,
                target: DamageEventTarget::Object(*pw_id),
                amount: final_damage,
                life_lost: 0,
                result: damage_result,
                lifelink_gain: None,
            })
        }
        // CR 506.4c / 510.1b: an unblocked creature attacking nothing assigns
        // no combat damage.
        AttackTarget::Nothing { .. } => None,
    }
}

/// Apply damage to a permanent (creature, planeswalker, or battle).
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
    use crate::events::DamageTarget;
    use crate::events::processing::process_damage_assignments_with_event;

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
    use crate::events::DamageTarget;
    use crate::events::processing::process_damage_assignments_with_event;

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

// ============================================================================
// Combat damage assignment choices (CR 510.1c-e)
// ============================================================================

/// Combat damage that other creatures are assigning to one creature in the
/// same combat damage step (CR 702.19b), and whether any of it comes from a
/// source with deathtouch, which makes it lethal (CR 702.2c).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SimultaneousCombatDamage {
    pub amount: u32,
    pub deathtouch: bool,
}

impl SimultaneousCombatDamage {
    fn add(&mut self, amount: u32, deathtouch: bool) {
        if amount == 0 {
            return;
        }
        self.amount = self.amount.saturating_add(amount);
        self.deathtouch |= deathtouch;
    }
}

/// Lethal damage a source must still assign to `recipient` before it may
/// assign damage elsewhere (CR 702.19b): damage already marked on it and
/// damage other creatures are assigning to it this step count toward lethal;
/// any nonzero damage from a deathtouch source is lethal (CR 702.2c).
fn remaining_lethal_damage(
    game: &GameState,
    recipient: ObjectId,
    source_has_deathtouch: bool,
    others: SimultaneousCombatDamage,
) -> u32 {
    if others.deathtouch {
        return 0;
    }
    let Some(object) = game.object(recipient) else {
        return 0;
    };
    let Some(threshold) = crate::rules::damage::lethal_damage_threshold_for_creature(game, object)
    else {
        return 0;
    };
    let remaining =
        (i64::from(threshold) - i64::from(game.damage_on(recipient)) - i64::from(others.amount))
            .max(0) as u32;
    if source_has_deathtouch {
        remaining.min(1)
    } else {
        remaining
    }
}

/// How an attacking or blocking creature may divide its combat damage in this
/// step (CR 510.1, 702.19): among the creatures it's in combat with, then (an
/// unblocked creature or a trampler) what it's attacking. A creature with
/// trample over planeswalkers attacking a planeswalker may also assign damage
/// to that planeswalker's controller (CR 702.19c), and one whose planeswalker
/// left combat may assign it to the defending player (CR 702.19e).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatDamageDivision {
    /// Creatures it's in combat with, in assignment order. Nothing else may be
    /// assigned damage until each of them is assigned lethal damage.
    pub creatures: Vec<ObjectId>,
    /// Trample over planeswalkers attacking a planeswalker: that planeswalker.
    /// It may be assigned damage once every creature is assigned lethal.
    pub planeswalker: Option<ObjectId>,
    /// Where damage may go beyond the creatures: what an unblocked creature or
    /// a trampler is attacking; with trample over planeswalkers attacking a
    /// planeswalker, that planeswalker's controller, once the planeswalker is
    /// assigned damage at least equal to its loyalty (CR 702.19c); or the
    /// defending player under CR 702.19e.
    pub excess_target: Option<Target>,
    /// Whether the source has deathtouch (1 damage counts as lethal, 702.2c).
    pub deathtouch: bool,
}

impl CombatDamageDivision {
    /// Every legal recipient, in assignment order.
    pub fn targets(&self) -> Vec<Target> {
        self.creatures
            .iter()
            .copied()
            .chain(self.planeswalker)
            .map(Target::Object)
            .chain(self.excess_target)
            .collect()
    }

    /// The permanents among the legal recipients.
    fn damageable_objects(&self) -> Vec<ObjectId> {
        self.targets()
            .into_iter()
            .filter_map(|target| match target {
                Target::Object(object) => Some(object),
                Target::Player(_) => None,
            })
            .collect()
    }

    /// The only recipient, when the rules leave no choice (CR 510.1b-d).
    fn forced_target(&self) -> Option<Target> {
        match self.targets().as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// Whether any damage may go beyond the creatures (CR 702.19).
    fn may_trample(&self) -> bool {
        self.planeswalker.is_some() || self.excess_target.is_some()
    }

    /// Lethal damage still needed by each creature (CR 702.19b, 702.2c).
    fn lethal_amounts(
        &self,
        game: &GameState,
        others: &std::collections::HashMap<ObjectId, SimultaneousCombatDamage>,
    ) -> Vec<u32> {
        self.creatures
            .iter()
            .map(|creature| {
                remaining_lethal_damage(
                    game,
                    *creature,
                    self.deathtouch,
                    others.get(creature).copied().unwrap_or_default(),
                )
            })
            .collect()
    }

    /// Damage the planeswalker must still be assigned before its controller
    /// may be: its loyalty, less damage other creatures are assigning to it
    /// this step (CR 702.19c).
    fn loyalty_needed(
        &self,
        game: &GameState,
        others: &std::collections::HashMap<ObjectId, SimultaneousCombatDamage>,
    ) -> u32 {
        let Some(planeswalker) = self.planeswalker else {
            return 0;
        };
        let loyalty = game
            .object(planeswalker)
            .and_then(|object| object.loyalty())
            .unwrap_or(0);
        loyalty.saturating_sub(others.get(&planeswalker).map_or(0, |damage| damage.amount))
    }

    /// The division used without a (legal) choice. When `spread`, lethal
    /// damage goes to each creature in order, then damage equal to the
    /// planeswalker's remaining loyalty, then the rest beyond; otherwise, and
    /// for anything left with nowhere else to go, it lands on the first
    /// recipient.
    fn default_allocation(
        &self,
        game: &GameState,
        total: u32,
        others: &std::collections::HashMap<ObjectId, SimultaneousCombatDamage>,
        spread: bool,
    ) -> Vec<(Target, u32)> {
        let targets = self.targets();
        if !spread {
            return targets
                .first()
                .map(|first| vec![(*first, total)])
                .unwrap_or_default();
        }
        let mut remaining = total;
        let mut allocation = self
            .creatures
            .iter()
            .zip(self.lethal_amounts(game, others))
            .map(|(creature, lethal)| {
                let amount = remaining.min(lethal);
                remaining -= amount;
                (Target::Object(*creature), amount)
            })
            .collect::<Vec<_>>();
        if let Some(planeswalker) = self.planeswalker {
            let amount = if self.excess_target.is_some() {
                remaining.min(self.loyalty_needed(game, others))
            } else {
                remaining
            };
            remaining -= amount;
            allocation.push((Target::Object(planeswalker), amount));
        }
        if let Some(target) = self.excess_target {
            allocation.push((target, remaining));
            remaining = 0;
        }
        if remaining > 0
            && let Some(first) = allocation.first_mut()
        {
            first.1 += remaining;
        }
        allocation.retain(|(_, amount)| *amount > 0);
        allocation
    }

    /// Turn a recorded per-permanent division into allocations; damage it
    /// leaves unassigned goes beyond the creatures (e.g. a trampler's excess
    /// to the player it's attacking).
    fn allocations_from_record(
        &self,
        total: u32,
        record: &std::collections::HashMap<ObjectId, u32>,
    ) -> Vec<(Target, u32)> {
        let mut allocations = record
            .iter()
            .map(|(object, amount)| (Target::Object(*object), *amount))
            .collect::<Vec<_>>();
        allocations.sort_by_key(|(target, _)| match target {
            Target::Object(object) => object.0,
            Target::Player(_) => u64::MAX,
        });
        let assigned = assignment_total(record);
        if assigned < total {
            if let Some(target) = self.excess_target {
                allocations.push((target, total - assigned));
            } else if let Some(planeswalker) = self.planeswalker {
                allocations.push((Target::Object(planeswalker), total - assigned));
            }
        }
        allocations
    }

    /// Check a division against CR 510.1c-e and 702.19b-e, returning it in
    /// assignment order (every legal recipient, zeros included).
    fn check(
        &self,
        game: &GameState,
        total: u32,
        allocations: &[(Target, u32)],
        others: &std::collections::HashMap<ObjectId, SimultaneousCombatDamage>,
    ) -> Result<Vec<(Target, u32)>, (CombatDamageAssignmentErrorKind, String)> {
        let targets = self.targets();
        let mut amounts = vec![0u32; targets.len()];
        let mut assigned = 0u32;
        for (target, amount) in allocations {
            let Some(index) = targets.iter().position(|candidate| candidate == target) else {
                return Err((
                    CombatDamageAssignmentErrorKind::IllegalRecipient,
                    format!("{target:?} can't be assigned this combat damage"),
                ));
            };
            amounts[index] = amounts[index].saturating_add(*amount);
            assigned = assigned.saturating_add(*amount);
        }
        if assigned != total {
            return Err((
                CombatDamageAssignmentErrorKind::WrongTotal,
                format!("exactly {total} combat damage must be assigned (got {assigned})"),
            ));
        }
        let creature_count = self.creatures.len();
        let beyond_creatures = amounts[creature_count..].iter().sum::<u32>();
        if beyond_creatures > 0
            && amounts[..creature_count]
                .iter()
                .zip(self.lethal_amounts(game, others))
                .any(|(amount, lethal)| *amount < lethal)
        {
            // CR 702.19b: nothing beyond the blockers before each blocker is
            // assigned lethal damage.
            return Err((
                CombatDamageAssignmentErrorKind::TrampleBeforeLethal,
                "combat damage can't trample over before each blocker is assigned lethal damage"
                    .to_string(),
            ));
        }
        if self.planeswalker.is_some() && self.excess_target.is_some() {
            let to_planeswalker = amounts[creature_count];
            let to_controller = amounts[creature_count + 1];
            // CR 702.19c: the controller gets damage only once the
            // planeswalker is assigned damage equal to its loyalty.
            if to_controller > 0 && to_planeswalker < self.loyalty_needed(game, others) {
                return Err((
                    CombatDamageAssignmentErrorKind::TrampleBeforeLethal,
                    "combat damage can't trample over a planeswalker before it's assigned damage equal to its loyalty"
                        .to_string(),
                ));
            }
        }
        Ok(targets.into_iter().zip(amounts).collect())
    }
}

/// Where an attacking creature may assign combat damage (CR 510.1b-c,
/// 702.19b-e), or `None` when it assigns none.
fn attacker_damage_division(
    game: &GameState,
    combat: &CombatState,
    attacker_info: &crate::combat_state::AttackerInfo,
) -> Option<CombatDamageDivision> {
    use crate::static_abilities::StaticAbilityId;

    let attacker_id = attacker_info.creature;
    // CR 702.19g: trample over planeswalkers includes trample's rules, so a
    // creature with both assigns damage as with trample over planeswalkers.
    let over_planeswalkers =
        game.object_has_static_ability_id(attacker_id, StaticAbilityId::TrampleOverPlaneswalkers);
    let trample = over_planeswalkers
        || game.object_has_static_ability_id(attacker_id, StaticAbilityId::Trample);
    let deathtouch = game.object_has_static_ability_id(attacker_id, StaticAbilityId::Deathtouch);
    let blocked = is_blocked(combat, attacker_id);
    let creatures = if blocked {
        combat
            .blockers
            .get(&attacker_id)
            .map(|blockers| {
                blockers
                    .iter()
                    .copied()
                    .filter(|id| game.object(*id).is_some())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if blocked && !trample {
        // CR 510.1c: a blocked creature assigns damage only to its blockers.
        return (!creatures.is_empty()).then_some(CombatDamageDivision {
            creatures,
            planeswalker: None,
            excess_target: None,
            deathtouch,
        });
    }
    // Unblocked (CR 510.1b), or a trampler: past its blockers (all of them
    // assigned lethal damage, or none left, CR 702.19d) to what it's attacking.
    let (planeswalker, excess_target) = if over_planeswalkers {
        trample_over_planeswalkers_targets(game, &attacker_info.target)
    } else {
        (None, trample_excess_target(game, &attacker_info.target))
    };
    if creatures.is_empty() && planeswalker.is_none() && excess_target.is_none() {
        return None;
    }
    Some(CombatDamageDivision {
        creatures,
        planeswalker,
        excess_target,
        deathtouch,
    })
}

/// Recipients beyond the blockers for a creature with trample over
/// planeswalkers: the planeswalker it's attacking, then its controller
/// (CR 702.19c); the defending player if that planeswalker was removed from
/// combat (CR 702.19e, without the creature attacking that player); else what
/// it's attacking, as with trample.
fn trample_over_planeswalkers_targets(
    game: &GameState,
    target: &AttackTarget,
) -> (Option<ObjectId>, Option<Target>) {
    let player_in_game = |player: PlayerId| {
        game.player(player)
            .is_some_and(|candidate| candidate.is_in_game())
            .then_some(Target::Player(player))
    };
    match *target {
        AttackTarget::Planeswalker(planeswalker)
            if game
                .object(planeswalker)
                .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                && game
                    .object_has_card_type(planeswalker, crate::types::CardType::Planeswalker) =>
        {
            (
                Some(planeswalker),
                game.controller_of_id(planeswalker).and_then(player_in_game),
            )
        }
        AttackTarget::Nothing {
            defending_player: Some(defender),
            was_planeswalker: true,
        } => (None, player_in_game(defender)),
        _ => (None, trample_excess_target(game, target)),
    }
}

/// An attacking or blocking creature that assigns combat damage this step.
#[derive(Debug, Clone)]
struct CombatDamageAssigner {
    source: ObjectId,
    total: u32,
    division: CombatDamageDivision,
}

/// The damage recipient for what an attacker is attacking, as a target.
fn trample_excess_target(game: &GameState, target: &AttackTarget) -> Option<Target> {
    attack_target_damage_recipient(game, target).map(|(target, _)| match target {
        EventDamageTarget::Player(player) => Target::Player(player),
        EventDamageTarget::Object(object) => Target::Object(object),
    })
}

/// The creatures that assign combat damage in this step: attackers (in
/// declaration order), then blockers (by id).
fn combat_damage_assigners(
    game: &GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
) -> (Vec<CombatDamageAssigner>, Vec<CombatDamageAssigner>) {
    let assigning_stat = |creature: &crate::object::Object| -> Option<u32> {
        if game.combat_damage_assignment_is_suppressed(creature.id)
            || !combatant_participates_in_damage_step(
                game,
                creature,
                first_strike,
                first_step_strikers,
            )
        {
            return None;
        }
        combat_damage_stat_for_creature(game, creature)
            .filter(|stat| *stat > 0)
            .map(|stat| stat as u32)
    };

    let mut attackers = Vec::new();
    for attacker_info in &combat.attackers {
        let Some(attacker) = game.object(attacker_info.creature) else {
            continue;
        };
        let Some(total) = assigning_stat(attacker) else {
            continue;
        };
        let Some(division) = attacker_damage_division(game, combat, attacker_info) else {
            continue;
        };
        attackers.push(CombatDamageAssigner {
            source: attacker_info.creature,
            total,
            division,
        });
    }

    let mut attackers_by_blocker: std::collections::HashMap<ObjectId, Vec<ObjectId>> =
        std::collections::HashMap::new();
    for (attacker, blockers) in &combat.blockers {
        for blocker in blockers {
            attackers_by_blocker
                .entry(*blocker)
                .or_default()
                .push(*attacker);
        }
    }
    let mut blocker_groups = attackers_by_blocker.into_iter().collect::<Vec<_>>();
    blocker_groups.sort_by_key(|(blocker, _)| blocker.0);
    let mut blockers = Vec::new();
    for (blocker_id, mut attacker_ids) in blocker_groups {
        let Some(blocker) = game.object(blocker_id) else {
            continue;
        };
        let Some(total) = assigning_stat(blocker) else {
            continue;
        };
        attacker_ids.sort_by_key(|id| id.0);
        attacker_ids.retain(|id| game.object(*id).is_some());
        if attacker_ids.is_empty() {
            continue;
        }
        blockers.push(CombatDamageAssigner {
            source: blocker_id,
            total,
            division: CombatDamageDivision {
                creatures: attacker_ids,
                planeswalker: None,
                excess_target: None,
                deathtouch: game.object_has_static_ability_id(
                    blocker_id,
                    crate::static_abilities::StaticAbilityId::Deathtouch,
                ),
            },
        });
    }
    (attackers, blockers)
}

/// Damage the `assigners` other than `source` are assigning to each permanent
/// this step (CR 702.19b-c): their `known` divisions (chosen or already
/// planned), or all the damage of one whose division is forced (e.g. an
/// unblocked creature attacking a planeswalker). A division not chosen yet
/// counts as nothing: it's announced after this one, and it sees this one
/// instead (CR 510.1).
fn simultaneous_combat_damage(
    assigners: &[CombatDamageAssigner],
    source: ObjectId,
    known: &std::collections::HashMap<ObjectId, std::collections::HashMap<ObjectId, u32>>,
) -> std::collections::HashMap<ObjectId, SimultaneousCombatDamage> {
    let mut assigned = std::collections::HashMap::<ObjectId, SimultaneousCombatDamage>::new();
    for assigner in assigners
        .iter()
        .filter(|assigner| assigner.source != source)
    {
        let deathtouch = assigner.division.deathtouch;
        if let Some(division) = known.get(&assigner.source) {
            let objects = assigner.division.damageable_objects();
            for (recipient, amount) in division {
                if objects.contains(recipient) {
                    assigned
                        .entry(*recipient)
                        .or_default()
                        .add(*amount, deathtouch);
                }
            }
        } else if let Some(Target::Object(recipient)) = assigner.division.forced_target() {
            assigned
                .entry(recipient)
                .or_default()
                .add(assigner.total, deathtouch);
        }
    }
    assigned
}

/// Position of `player` in APNAP order, starting from the active player.
fn apnap_position(game: &GameState, player: PlayerId) -> usize {
    let order = &game.turn_store.turn_order;
    let Some(active) = order
        .iter()
        .position(|candidate| *candidate == game.turn.active_player)
    else {
        return player.0 as usize;
    };
    order
        .iter()
        .position(|candidate| *candidate == player)
        .map_or(usize::MAX, |index| {
            (index + order.len() - active) % order.len()
        })
}

/// A combat-damage division that the assigning player chooses before a
/// combat-damage step (CR 510.1c-d, 702.19b-e).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatDamageAssignmentPrompt {
    /// The attacking or blocking creature assigning its damage.
    pub source: ObjectId,
    /// The player who divides the damage (usually the source's controller).
    pub player: PlayerId,
    /// The combat damage the source assigns.
    pub total: u32,
    /// Where the damage may go, and in what order.
    pub division: CombatDamageDivision,
    /// Damage other creatures are assigning to each permanent in this step,
    /// which counts toward lethal damage and loyalty (CR 702.19b-c).
    pub assigned_by_others: std::collections::HashMap<ObjectId, SimultaneousCombatDamage>,
}

impl CombatDamageAssignmentPrompt {
    /// Build the decision context shown to the assigning player.
    pub fn decision_context(
        &self,
        game: &GameState,
    ) -> crate::decisions::context::DistributeContext {
        let object_name = |id: ObjectId| {
            game.object(id)
                .map(|object| object.name.to_string())
                .unwrap_or_else(|| format!("#{}", id.0))
        };
        let targets = self
            .division
            .targets()
            .into_iter()
            .map(|target| crate::decisions::context::DistributeTarget {
                name: match target {
                    Target::Object(id) => object_name(id),
                    Target::Player(player) => game
                        .player(player)
                        .map(|candidate| candidate.name.clone())
                        .unwrap_or_else(|| format!("Player {}", player.0)),
                },
                target,
            })
            .collect();
        crate::decisions::context::DistributeContext::new(
            self.player,
            Some(self.source),
            format!(
                "Assign {} combat damage from {}",
                self.total,
                object_name(self.source)
            ),
            self.total,
            targets,
            0,
        )
    }

    /// The division used when the assigning player makes no (legal) choice:
    /// lethal damage to each creature in order, then the planeswalker's
    /// remaining loyalty, then the rest beyond, or else onto the first
    /// recipient.
    pub fn default_assignment(&self, game: &GameState) -> Vec<(Target, u32)> {
        self.division
            .default_allocation(game, self.total, &self.assigned_by_others, true)
    }

    /// Check a proposed division against CR 510.1c-d and 702.19b-e, returning
    /// the per-permanent assignment to record. Damage it leaves to a player
    /// is what goes beyond the permanents.
    pub fn validate(
        &self,
        game: &GameState,
        allocations: &[(Target, u32)],
    ) -> Result<std::collections::HashMap<ObjectId, u32>, String> {
        let checked = self
            .division
            .check(game, self.total, allocations, &self.assigned_by_others)
            .map_err(|(_, message)| format!("combat damage from #{}: {message}", self.source.0))?;
        Ok(checked
            .into_iter()
            .filter_map(|(target, amount)| match target {
                Target::Object(object) => Some((object, amount)),
                Target::Player(_) => None,
            })
            .collect())
    }

    /// Record the assigning player's division, falling back to the default
    /// division when the proposal is illegal or empty.
    pub fn record(&self, game: &mut GameState, allocations: &[(Target, u32)]) {
        let per_permanent = self
            .validate(game, allocations)
            .or_else(|_| self.validate(game, &self.default_assignment(game)))
            .unwrap_or_else(|_| {
                self.division
                    .damageable_objects()
                    .into_iter()
                    .enumerate()
                    .map(|(index, recipient)| (recipient, if index == 0 { self.total } else { 0 }))
                    .collect()
            });
        game.turn_store
            .combat_damage_assignments
            .insert(self.source, per_permanent);
    }
}

/// Return the next combat-damage division the players must choose before the
/// given damage step, or `None` once every choice has been recorded.
///
/// CR 510.1: the attacking creatures' assignments are announced first, then
/// the blocking creatures', each group in APNAP order of the assigning
/// players. Only sources with a real choice are asked: an attacker blocked
/// by two or more creatures, a blocked trampler, a creature with trample over
/// planeswalkers attacking a planeswalker, or a blocker blocking two or more
/// attackers. Each prompt sees the divisions announced before it, so lethal
/// and loyalty checks count other creatures' damage (CR 702.19b-c).
pub fn next_combat_damage_assignment_prompt(
    game: &GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
) -> Option<CombatDamageAssignmentPrompt> {
    let known = &game.turn_store.combat_damage_assignments;
    let (attackers, blockers) =
        combat_damage_assigners(game, combat, first_strike, first_step_strikers);

    let assigning_player = |assigner: &CombatDamageAssigner, is_attacker: bool| {
        let controller = || {
            game.object(assigner.source)
                .map(|object| game.controller_of(object))
                .unwrap_or(game.turn.active_player)
        };
        if is_attacker
            && defender_assigns_combat_damage_for_attacker(game, combat, assigner.source)
            && let Some(AttackTarget::Player(defender)) =
                crate::combat_state::get_attack_target(combat, assigner.source)
        {
            return *defender;
        }
        crate::combat_state::combat_damage_assignment_player(game, combat, assigner.source)
            .unwrap_or_else(controller)
    };

    for (group, is_attacker) in [(&attackers, true), (&blockers, false)] {
        let mut pending = group
            .iter()
            .filter(|assigner| !known.contains_key(&assigner.source))
            .filter(|assigner| assigner.division.targets().len() >= 2)
            .map(|assigner| (assigning_player(assigner, is_attacker), assigner))
            .collect::<Vec<_>>();
        // Stable: declaration (attackers) or id (blockers) order within a player.
        pending.sort_by_key(|(player, _)| apnap_position(game, *player));
        let Some((player, assigner)) = pending.into_iter().next() else {
            continue;
        };
        return Some(CombatDamageAssignmentPrompt {
            source: assigner.source,
            player,
            total: assigner.total,
            division: assigner.division.clone(),
            assigned_by_others: simultaneous_combat_damage(group, assigner.source, known),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::DamageTarget as EventDamageTarget;
    use crate::events::cause::CauseFilter;
    use crate::events::counters::matchers::WouldPutCountersMatcher;
    use crate::events::damage::matchers::DamageFromSourceMatcher;
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
            obj.abilities_mut().push(Ability::static_ability(ability));
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
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                controller,
                WouldPutCountersMatcher::new(
                    ObjectFilter::specific(target),
                    Some(CounterType::MinusOneMinusOne),
                )
                .with_cause_filter(CauseFilter::from_effect()),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );
    }

    fn add_fiery_emancipation_like_effect(
        game: &mut GameState,
        controller: PlayerId,
        source: ObjectId,
    ) {
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                controller,
                DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ReplacementAction::Modify(EventModification::Multiply(3)),
            ),
        );
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

        let processed = crate::events::processing::process_damage_assignments_with_event(
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

    #[test]
    fn unblocked_combat_damage_uses_one_pre_damage_characteristic_view() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let first = create_creature(
            &mut game,
            "First Attacker",
            1,
            1,
            alice,
            vec![StaticAbility::first_strike(), StaticAbility::infect()],
        );
        game.player_mut(bob).expect("player exists").poison_counters = 2;
        let poison_power = create_creature(
            &mut game,
            "Poison-Power Attacker",
            1,
            1,
            alice,
            vec![StaticAbility::first_strike()],
        );
        game.effect_store
            .continuous_effects
            .add_effect(crate::continuous::ContinuousEffect::new(
                poison_power,
                alice,
                crate::continuous::EffectTarget::Specific(poison_power),
                crate::continuous::Modification::SetPower {
                    value: crate::effect::Value::PlayerCounters(
                        crate::target::PlayerFilter::Specific(bob),
                        crate::object::CounterType::Poison,
                    ),
                    sublayer: crate::continuous::PtSublayer::Setting,
                },
            ));

        let combat = CombatState {
            attackers: vec![
                crate::combat_state::AttackerInfo {
                    creature: first,
                    target: AttackTarget::Player(bob),
                },
                crate::combat_state::AttackerInfo {
                    creature: poison_power,
                    target: AttackTarget::Player(bob),
                },
            ],
            ..CombatState::default()
        };

        let events = execute_combat_damage_step(&mut game, &combat, true);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].source, first);
        assert_eq!(events[0].amount, 1);
        assert!(events[0].result.has_infect);
        assert_eq!(events[1].source, poison_power);
        assert_eq!(events[1].amount, 2);
        assert_eq!(game.player(bob).expect("player exists").life, 18);
        assert_eq!(game.player(bob).expect("player exists").poison_counters, 3);
    }

    #[test]
    fn unblocked_combat_damage_preserves_keywords_order_and_commander_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let lifelink_commander = create_creature(
            &mut game,
            "Lifelink Commander",
            2,
            2,
            alice,
            vec![
                StaticAbility::first_strike(),
                StaticAbility::lifelink(),
                StaticAbility::deathtouch(),
                StaticAbility::wither(),
            ],
        );
        game.set_as_commander(lifelink_commander, alice);
        let infector = create_creature(
            &mut game,
            "Infector",
            3,
            3,
            alice,
            vec![StaticAbility::first_strike(), StaticAbility::infect()],
        );

        let combat = CombatState {
            attackers: vec![
                crate::combat_state::AttackerInfo {
                    creature: lifelink_commander,
                    target: AttackTarget::Player(bob),
                },
                crate::combat_state::AttackerInfo {
                    creature: infector,
                    target: AttackTarget::Player(bob),
                },
            ],
            ..CombatState::default()
        };

        let events = execute_combat_damage_step(&mut game, &combat, true);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].source, lifelink_commander);
        assert_eq!(events[0].amount, 2);
        assert_eq!(events[0].life_lost, 2);
        assert!(events[0].result.has_lifelink);
        assert!(events[0].result.has_deathtouch);
        assert!(events[0].result.has_wither);
        assert_eq!(events[1].source, infector);
        assert_eq!(events[1].amount, 3);
        assert_eq!(events[1].life_lost, 0);
        assert!(events[1].result.has_infect);
        assert_eq!(game.player(alice).expect("player exists").life, 22);
        assert_eq!(game.player(bob).expect("player exists").life, 18);
        assert_eq!(game.player(bob).expect("player exists").poison_counters, 3);
        assert_eq!(
            game.player(bob)
                .expect("player exists")
                .commander_damage_from(lifelink_commander),
            2
        );
    }

    #[test]
    fn unblocked_combat_damage_falls_back_when_prevention_is_active() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = create_creature(
            &mut game,
            "Prevented Attacker",
            3,
            3,
            alice,
            vec![StaticAbility::first_strike()],
        );
        let shield = crate::prevention::PreventionShield::prevent_all(
            attacker,
            bob,
            crate::prevention::PreventionTarget::Player(bob),
        );
        game.effect_store.prevention_effects.add_shield(shield);

        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        };

        let events = execute_combat_damage_step(&mut game, &combat, true);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source, attacker);
        assert_eq!(events[0].amount, 0);
        assert_eq!(events[0].life_lost, 0);
        assert_eq!(game.player(bob).expect("player exists").life, 20);
    }

    #[test]
    fn unblocked_combat_batch_allocates_limited_shield_to_chosen_source() {
        struct AllocateToLaterSource(PlayerId);

        impl crate::decision::DecisionMaker for AllocateToLaterSource {
            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                assert_eq!(ctx.player, self.0);
                ctx.min
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let earlier = create_creature(&mut game, "Earlier Attacker", 3, 3, alice, vec![]);
        let later = create_creature(&mut game, "Later Attacker", 3, 3, alice, vec![]);
        let shield_source = create_creature(&mut game, "Shield Source", 1, 1, bob, vec![]);
        game.effect_store.prevention_effects.add_shield(
            crate::prevention::PreventionShield::prevent_next_n(
                shield_source,
                bob,
                crate::prevention::PreventionTarget::Player(bob),
                2,
            ),
        );
        let combat = CombatState {
            attackers: vec![
                crate::combat_state::AttackerInfo {
                    creature: earlier,
                    target: AttackTarget::Player(bob),
                },
                crate::combat_state::AttackerInfo {
                    creature: later,
                    target: AttackTarget::Player(bob),
                },
            ],
            ..CombatState::default()
        };

        let mut dm = AllocateToLaterSource(bob);
        let events = execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].source, earlier);
        assert_eq!(events[0].amount, 3);
        assert_eq!(events[1].source, later);
        assert_eq!(events[1].amount, 1);
        assert_eq!(game.player(bob).expect("player exists").life, 16);
        assert!(game.effect_store.prevention_effects.shields().is_empty());
    }

    #[test]
    fn blocked_combat_batch_allocates_limited_shield_between_attackers() {
        struct AllocateToLaterSource(PlayerId);

        impl crate::decision::DecisionMaker for AllocateToLaterSource {
            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                assert_eq!(ctx.player, self.0);
                ctx.min
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let earlier = create_creature(&mut game, "Earlier Attacker", 3, 3, alice, vec![]);
        let later = create_creature(&mut game, "Later Attacker", 3, 3, alice, vec![]);
        let blocker = create_creature(&mut game, "Shared Blocker", 0, 10, bob, vec![]);
        game.effect_store.prevention_effects.add_shield(
            crate::prevention::PreventionShield::prevent_next_n(
                blocker,
                bob,
                crate::prevention::PreventionTarget::Permanent(blocker),
                2,
            ),
        );
        let combat = CombatState {
            attackers: vec![
                crate::combat_state::AttackerInfo {
                    creature: earlier,
                    target: AttackTarget::Player(bob),
                },
                crate::combat_state::AttackerInfo {
                    creature: later,
                    target: AttackTarget::Player(bob),
                },
            ],
            blockers: std::collections::BTreeMap::from([
                (earlier, vec![blocker]),
                (later, vec![blocker]),
            ]),
            ..CombatState::default()
        };

        let mut dm = AllocateToLaterSource(bob);
        let events = execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].source, earlier);
        assert_eq!(events[0].target, DamageEventTarget::Object(blocker));
        assert_eq!(events[0].amount, 3);
        assert_eq!(events[1].source, later);
        assert_eq!(events[1].target, DamageEventTarget::Object(blocker));
        assert_eq!(events[1].amount, 1);
        assert_eq!(game.damage_on(blocker), 4);
        assert!(game.effect_store.prevention_effects.shields().is_empty());
    }

    #[test]
    fn blocker_damage_is_planned_before_another_damage_follow_up_removes_it() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Attacker", 3, 3, alice, vec![]);
        let blocker = create_creature(&mut game, "Vanishing Blocker", 2, 3, bob, vec![]);
        game.effect_store.prevention_effects.add_shield(
            crate::prevention::PreventionShield::prevent_all(
                blocker,
                bob,
                crate::prevention::PreventionTarget::Permanent(blocker),
            )
            .with_follow_up_effects(vec![crate::effect::Effect::exile(ChooseSpec::AnyTarget)]),
        );
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            blockers: std::collections::BTreeMap::from([(attacker, vec![blocker])]),
            ..CombatState::default()
        };

        let events = execute_combat_damage_step(&mut game, &combat, false);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].source, attacker);
        assert_eq!(events[0].amount, 0, "attacker damage should be prevented");
        assert_eq!(events[1].source, blocker);
        assert_eq!(events[1].target, DamageEventTarget::Object(attacker));
        assert_eq!(events[1].amount, 2);
        assert_eq!(game.damage_on(attacker), 2);
        assert!(
            game.exile.iter().any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Vanishing Blocker")),
            "the prevention follow-up should still exile the blocker afterward"
        );
    }

    #[test]
    fn regular_damage_step_uses_first_step_strike_snapshot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let lost_first_strike = create_creature(
            &mut game,
            "Lost First Strike",
            1,
            1,
            alice,
            vec![StaticAbility::first_strike()],
        );
        let gained_first_strike =
            create_creature(&mut game, "Gained First Strike", 2, 2, alice, vec![]);
        let combat = CombatState {
            attackers: vec![
                crate::combat_state::AttackerInfo {
                    creature: lost_first_strike,
                    target: AttackTarget::Player(bob),
                },
                crate::combat_state::AttackerInfo {
                    creature: gained_first_strike,
                    target: AttackTarget::Player(bob),
                },
            ],
            ..CombatState::default()
        };
        let first_step_strikers = std::collections::HashSet::from([lost_first_strike]);

        let first = execute_combat_damage_step(&mut game, &combat, true);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].source, lost_first_strike);
        assert_eq!(game.player(bob).expect("player exists").life, 19);

        game.object_mut(lost_first_strike)
            .expect("first striker exists")
            .abilities_mut()
            .clear();
        game.object_mut(gained_first_strike)
            .expect("regular striker exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::first_strike()));
        game.refresh_continuous_state();

        let regular = execute_combat_damage_step_with_first_step_snapshot(
            &mut game,
            &combat,
            false,
            &first_step_strikers,
        );
        assert_eq!(regular.len(), 1);
        assert_eq!(regular[0].source, gained_first_strike);
        assert_eq!(regular[0].amount, 2);
        assert_eq!(game.player(bob).expect("player exists").life, 17);
    }

    #[test]
    fn combat_damage_accepts_arbitrary_nonlethal_split() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Attacker", 4, 6, alice, vec![]);
        let first = create_creature(&mut game, "First Blocker", 0, 5, bob, vec![]);
        let second = create_creature(&mut game, "Second Blocker", 0, 5, bob, vec![]);
        game.set_combat_damage_assignment(attacker, first, 1);
        game.set_combat_damage_assignment(attacker, second, 3);
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            blockers: std::collections::BTreeMap::from([(attacker, vec![first, second])]),
            ..CombatState::default()
        };

        let events = try_execute_combat_damage_step(&mut game, &combat, false)
            .expect("arbitrary division is legal");
        let attacker_events = events
            .iter()
            .filter(|event| event.source == attacker)
            .map(|event| (event.target, event.amount))
            .collect::<Vec<_>>();

        assert_eq!(
            attacker_events,
            vec![
                (DamageEventTarget::Object(first), 1),
                (DamageEventTarget::Object(second), 3),
            ]
        );
    }

    #[test]
    fn illegal_combat_damage_assignment_is_rejected_and_restored() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Attacker", 4, 6, alice, vec![]);
        let first = create_creature(&mut game, "First Blocker", 0, 5, bob, vec![]);
        let second = create_creature(&mut game, "Second Blocker", 0, 5, bob, vec![]);
        game.set_combat_damage_assignment(attacker, first, 1);
        game.set_combat_damage_assignment(attacker, second, 1);
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            blockers: std::collections::BTreeMap::from([(attacker, vec![first, second])]),
            ..CombatState::default()
        };

        let error = try_execute_combat_damage_step(&mut game, &combat, false)
            .expect_err("partial assignment must be rejected");

        assert_eq!(error.kind, CombatDamageAssignmentErrorKind::WrongTotal);
        assert_eq!(error.expected_total, 4);
        assert_eq!(error.assigned_total, 2);
        assert_eq!(game.damage_on(first), 0);
        assert_eq!(game.damage_on(second), 0);
        assert_eq!(
            game.turn_store.combat_damage_assignments.get(&attacker),
            Some(&std::collections::HashMap::from([(first, 1), (second, 1)]))
        );
    }

    #[test]
    fn trample_cannot_assign_excess_before_lethal_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(
            &mut game,
            "Trampler",
            5,
            5,
            alice,
            vec![StaticAbility::trample()],
        );
        let blocker = create_creature(&mut game, "Blocker", 0, 3, bob, vec![]);
        game.set_combat_damage_assignment(attacker, blocker, 1);
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            blockers: std::collections::BTreeMap::from([(attacker, vec![blocker])]),
            ..CombatState::default()
        };

        let error = try_execute_combat_damage_step(&mut game, &combat, false)
            .expect_err("trample cannot pass four damage through a 3-toughness blocker");

        assert_eq!(
            error.kind,
            CombatDamageAssignmentErrorKind::TrampleBeforeLethal
        );
        assert_eq!(game.player(bob).expect("player exists").life, 20);
        assert_eq!(game.damage_on(blocker), 0);
    }

    #[test]
    fn default_combat_assignment_ignores_obsolete_blocker_order() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Attacker", 4, 6, alice, vec![]);
        let first = create_creature(&mut game, "First Blocker", 0, 1, bob, vec![]);
        let second = create_creature(&mut game, "Second Blocker", 0, 5, bob, vec![]);
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            blockers: std::collections::BTreeMap::from([(attacker, vec![first, second])]),
            damage_assignment_order: std::collections::BTreeMap::from([(
                attacker,
                vec![second, first],
            )]),
            ..CombatState::default()
        };

        let events = try_execute_combat_damage_step(&mut game, &combat, false)
            .expect("default assignment is legal");
        let attacker_events = events
            .iter()
            .filter(|event| event.source == attacker)
            .map(|event| (event.target, event.amount))
            .collect::<Vec<_>>();

        assert_eq!(attacker_events, vec![(DamageEventTarget::Object(first), 4)]);
        assert_eq!(game.damage_on(first), 4);
        assert_eq!(game.damage_on(second), 0);
    }

    #[test]
    fn multiplayer_800_4e_assigns_no_combat_damage_to_player_who_left() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Late Attacker", 4, 4, alice, vec![]);
        game.player_mut(bob).expect("Bob").has_left_game = true;
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        };

        let events = try_execute_combat_damage_step(&mut game, &combat, false)
            .expect("a departed defender is simply omitted from assignment");

        assert!(events.is_empty());
        assert_eq!(game.player(bob).expect("Bob").life, 20);
    }
}
