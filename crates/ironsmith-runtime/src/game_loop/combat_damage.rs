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
    for (blocker_id, mut attacker_ids) in attackers_by_blocker {
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
            source: blocker_id,
            target: DamageEventTarget::Object(attacker_id),
            amount: applied.damage_dealt,
            life_lost: 0,
            result: damage_result,
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
        let controller = game.controller_of(&attacker);
        let cause = combat_damage_cause(game, attacker_id);

        if !is_blocked(combat, attacker_id) {
            if let AttackTarget::Player(player) = attacker_info.target
                && !game
                    .player(player)
                    .is_some_and(|candidate| candidate.is_in_game())
            {
                // CR 800.4e: combat damage is not assigned to a player who
                // has left the game.
                continue;
            }
            let (target, rules_target) = match attacker_info.target {
                AttackTarget::Player(player) => (
                    EventDamageTarget::Player(player),
                    DamageTarget::Player(player),
                ),
                AttackTarget::Planeswalker(object) => {
                    (EventDamageTarget::Object(object), DamageTarget::Permanent)
                }
                AttackTarget::Battle(object) => {
                    (EventDamageTarget::Object(object), DamageTarget::Permanent)
                }
            };
            let amount = combat_stat as u32;
            let result = calculate_damage_with_game(game, &attacker, rules_target, amount, true);
            planned.push(PlannedCombatDamage {
                source: attacker_id,
                target,
                controller,
                amount,
                result,
                cause,
            });
            continue;
        }

        let blocker_ids = combat
            .blockers
            .get(&attacker_id)
            .cloned()
            .unwrap_or_default();
        if blocker_ids.is_empty() {
            continue;
        }
        let explicit_assignments = game.take_combat_damage_assignments(attacker_id);
        let blocker_pairs = blocker_ids
            .iter()
            .filter_map(|id| game.object(*id).map(|object| (*id, object)))
            .collect::<Vec<_>>();
        if blocker_pairs.is_empty() {
            continue;
        }
        let aligned_ids = blocker_pairs.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        let blockers = blocker_pairs
            .iter()
            .map(|(_, object)| *object)
            .collect::<Vec<_>>();
        let (distribution, excess) = if explicit_assignments.is_empty() {
            if game.object_has_static_ability_id(
                attacker_id,
                crate::static_abilities::StaticAbilityId::Trample,
            ) {
                distribute_trample_damage(&attacker, &blockers, combat_stat as u32, game)
            } else {
                (
                    default_combat_damage_distribution(blockers.len(), combat_stat as u32),
                    0,
                )
            }
        } else {
            validate_attacker_damage_assignment(
                game,
                &attacker,
                &aligned_ids,
                &blockers,
                combat_stat as u32,
                &explicit_assignments,
            )?
        };
        for (index, (amount, _)) in distribution.into_iter().enumerate() {
            if amount == 0 {
                continue;
            }
            let target = aligned_ids[index];
            planned.push(PlannedCombatDamage {
                source: attacker_id,
                target: EventDamageTarget::Object(target),
                controller,
                amount,
                result: calculate_damage_with_game(
                    game,
                    &attacker,
                    DamageTarget::Permanent,
                    amount,
                    true,
                ),
                cause: cause.clone(),
            });
        }
        if excess > 0
            && let AttackTarget::Player(player) = attacker_info.target
            && game
                .player(player)
                .is_some_and(|candidate| candidate.is_in_game())
        {
            planned.push(PlannedCombatDamage {
                source: attacker_id,
                target: EventDamageTarget::Player(player),
                controller,
                amount: excess,
                result: calculate_damage_with_game(
                    game,
                    &attacker,
                    DamageTarget::Player(player),
                    excess,
                    true,
                ),
                cause,
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
        } else {
            if explicit_assignments.is_empty() {
                default_combat_damage_distribution(attacker_ids.len(), combat_stat as u32)
            } else {
                validate_nontrample_damage_assignment(
                    blocker_id,
                    &attacker_ids,
                    combat_stat as u32,
                    &explicit_assignments,
                )?
            }
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
                }
                if assignment.target == planned.target {
                    damage_to_original = damage_to_original.saturating_add(assignment.amount);
                    life_lost_to_original = life_lost_to_original.saturating_add(applied.life_lost);
                }
            }
        }
        apply_combat_lifelink(
            game,
            planned.controller,
            &planned.result,
            total_damage_dealt,
        );
        let event_target = match planned.target {
            EventDamageTarget::Player(player) => DamageEventTarget::Player(player),
            EventDamageTarget::Object(object) => DamageEventTarget::Object(object),
        };
        events.push(CombatDamageEvent {
            source: planned.source,
            target: event_target,
            amount: damage_to_original,
            life_lost: life_lost_to_original,
            result: planned.result,
        });
    }
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
    combat.blockers.values().all(Vec::is_empty)
        && combat
            .attackers
            .iter()
            .all(|attacker| matches!(attacker.target, AttackTarget::Player(_)))
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
                    if player == planned.target {
                        damage_to_original = damage_to_original.saturating_add(assignment.amount);
                        life_lost_to_original =
                            life_lost_to_original.saturating_add(applied.life_lost);
                    }
                }
            }
        }
        apply_combat_lifelink(
            game,
            planned.controller,
            &planned.result,
            total_damage_dealt,
        );
        events.push(CombatDamageEvent {
            source: planned.source,
            target: DamageEventTarget::Player(planned.target),
            amount: damage_to_original,
            life_lost: life_lost_to_original,
            result: planned.result,
        });
    }

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
    }
    apply_combat_lifelink(
        game,
        planned.controller,
        &planned.result,
        total_damage_dealt,
    );

    CombatDamageEvent {
        source: planned.source,
        target: DamageEventTarget::Player(planned.target),
        amount: total_damage_dealt,
        life_lost: applied.life_lost,
        result: planned.result,
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

pub(super) fn apply_combat_lifelink(
    game: &mut GameState,
    controller: PlayerId,
    damage_result: &DamageResult,
    total_damage_dealt: u32,
) {
    if !damage_result.has_lifelink || total_damage_dealt == 0 {
        return;
    }

    let life_to_gain = crate::events::processing::process_life_gain_with_event(
        game,
        controller,
        total_damage_dealt,
    );
    if life_to_gain > 0 {
        game.gain_life(controller, life_to_gain);
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

fn validate_attacker_damage_assignment(
    game: &GameState,
    attacker: &crate::object::Object,
    blocker_ids: &[ObjectId],
    blockers: &[&crate::object::Object],
    total_damage: u32,
    explicit_assignments: &std::collections::HashMap<ObjectId, u32>,
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

    for (index, blocker) in blockers.iter().enumerate() {
        let blocker_id = blocker_ids[index];
        let lethal = if has_deathtouch {
            1
        } else if let Some(threshold) =
            crate::rules::damage::lethal_damage_threshold_for_creature(game, blocker)
        {
            let existing_damage = game.damage_on(blocker.id);
            (threshold - existing_damage as i32).max(0) as u32
        } else {
            0
        };
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
                source: attacker_id,
                target: DamageEventTarget::Player(*player_id),
                amount: applied.damage_dealt,
                life_lost: applied.life_lost,
                result: damage_result,
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
                source: attacker_id,
                target: DamageEventTarget::Object(*pw_id),
                amount: final_damage,
                life_lost: 0,
                result: damage_result,
            })
        }
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
            blockers: std::collections::HashMap::from([
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
            blockers: std::collections::HashMap::from([(attacker, vec![blocker])]),
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
            blockers: std::collections::HashMap::from([(attacker, vec![first, second])]),
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
            blockers: std::collections::HashMap::from([(attacker, vec![first, second])]),
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
            blockers: std::collections::HashMap::from([(attacker, vec![blocker])]),
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
            blockers: std::collections::HashMap::from([(attacker, vec![first, second])]),
            damage_assignment_order: std::collections::HashMap::from([(
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
