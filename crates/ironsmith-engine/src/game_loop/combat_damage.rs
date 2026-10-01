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
    /// Complete damage consequences, including replacement payload notifications.
    pub consequence_outcome: Option<crate::effect::EffectOutcome>,
    /// The damage result with lifelink/infect info.
    pub result: DamageResult,
    /// Resolved lifelink life change or replacement payload, retained once
    /// on the source's first damage event for a simultaneous damage group.
    pub lifelink_outcome: Option<crate::effect::EffectOutcome>,
}

/// Why a proposed combat-damage assignment is illegal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombatDamageAssignmentErrorKind {
    /// Damage was assigned to an object that is not a current recipient.
    Execution(crate::effects::ExecutionError),
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
        match &self.kind {
            CombatDamageAssignmentErrorKind::Execution(error) => write!(f, "combat damage execution failed: {error}"),
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

impl From<crate::events::processing::DamageProcessingError> for CombatDamageAssignmentError {
    fn from(failure: crate::events::processing::DamageProcessingError) -> Self {
        Self::execution(failure.source, failure.error)
    }
}


impl CombatDamageAssignmentError {
    fn execution(source: ObjectId, error: crate::effects::ExecutionError) -> Self {
        Self { source, expected_total: 0, assigned_total: 0,
            illegal_recipients: Vec::new(), kind: CombatDamageAssignmentErrorKind::Execution(error) }
    }
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

pub(crate) fn try_execute_combat_damage_step_with_dm_and_first_step_snapshot(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    if dm.awaiting_choice() {
        return Ok(Vec::new());
    }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let result = crate::events::processing::with_deferred_prevention_follow_ups(
        game,
        dm,
        |game, dm| {
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
                    if let Some(snapshot) = game.object(event.source).map(|obj| {
                        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                    }) {
                        event.source_snapshot = Some(snapshot);
                    }
                    if let DamageEventTarget::Object(target) = event.target {
                        event.target_snapshot = game.object(target).map(|obj| {
                        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                    });
                    }
                }
            }
            result
        },
    );
    if dm.awaiting_choice() && result.is_ok() {
        game.restore_execution_checkpoint(checkpoint, true);
        return Ok(Vec::new());
    }
    if matches!(&result, Err(error) if matches!(error.kind, CombatDamageAssignmentErrorKind::Execution(_)))
    {
        game.restore_execution_checkpoint(checkpoint, false);
    }
    result
}

fn apply_combat_damage_step_with_dm_and_first_step_snapshot(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    let Some(first_attacker) = combat.attackers.first() else { return Ok(Vec::new()); };
    let discovery_failure = |error| CombatDamageAssignmentError::execution(
        first_attacker.creature, crate::effects::ExecutionError::ContinuousDiscovery(error));
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
        game.update_replacement_effects().map_err(discovery_failure)?;
    } else {
        // A full refresh also rebuilds both trackers after regenerating static
        // continuous effects.
        game.refresh_continuous_state().map_err(discovery_failure)?;
    }
    if can_use_unblocked_player_damage_fast_path(game, combat) {
        return execute_unblocked_player_damage_fast_path(
            game,
            combat,
            first_strike,
            first_step_strikers,
            dm,
        );
    }
    if is_unblocked_player_damage_batch(combat) {
        return execute_unblocked_player_damage_batch_path(
            game,
            combat,
            first_strike,
            first_step_strikers,
            dm,
        );
    }

    execute_general_combat_damage_batch_path(game, combat, first_strike, first_step_strikers, dm)
}

#[derive(Debug)]
struct PlannedCombatDamage {
    source: ObjectId,
    source_snapshot: crate::snapshot::ObjectSnapshot,
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
                source_snapshot: crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(&attacker, game),
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
                source_snapshot: crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(&blocker, game),
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
    let mut planned = match plan_general_combat_damage(game, combat, first_strike, first_step_strikers)
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
                source_snapshot: Some(planned.source_snapshot.clone()),
            },
        )
        .collect::<Vec<_>>();
    let processed =
        crate::events::processing::process_simultaneous_damage_assignments_with_event_with_dm(
            game, &batch, dm,
        )
        .map_err(CombatDamageAssignmentError::from)?;
    if dm.awaiting_choice() {
        game.turn_store.combat_damage_assignments = assignments_checkpoint;
        return Ok(Vec::new());
    }
    for proposal in &mut planned {
        proposal.source_snapshot = combat_damage_source_snapshot(
            game, proposal.source, &proposal.source_snapshot,
        );
    }

    // CR 120.10: excess damage is judged against each permanent's state
    // before this step's damage is dealt.
    let excess_capacities = CombatExcessCapacities::before_damage(
        game,
        processed
            .iter()
            .filter(|processed| !processed.replacement_prevented)
            .flat_map(|processed| processed.assignments.iter())
            .filter_map(|assignment| match assignment.target {
                EventDamageTarget::Object(object) => Some(object),
                EventDamageTarget::Player(_) => None,
            }),
    );

    let mut events = Vec::with_capacity(planned.len());
    let mut lifelink_totals = CombatLifelinkTotals::default();
    let mut additions = Vec::new();
    for (planned, processed) in planned.into_iter().zip(processed) {
        if !processed.programs.is_empty() {
            additions.push((events.len(), planned.source, planned.controller,
                planned.source_snapshot.clone(), planned.cause.clone(), processed.programs));
        }
        let keywords = crate::rules::damage::SourceDamageKeywords {
            has_deathtouch: planned.result.has_deathtouch,
            has_infect: planned.result.has_infect,
            has_wither: planned.result.has_wither,
            has_lifelink: planned.result.has_lifelink,
        };
        let mut damage_to_original = 0u32;
        let mut life_lost_to_original = 0u32;
        let mut consequence_outcomes = processed.payload_outcome.into_iter().collect::<Vec<_>>();
        let mut total_damage_dealt = 0u32;
        let mut redirected = Vec::new();
        if !processed.replacement_prevented {
            for assignment in processed.assignments {
                let mut applied = crate::rules::damage::apply_processed_damage_assignment_with_dm(
                    game,
                    planned.source,
                    assignment.target,
                    assignment.amount,
                    keywords,
                    planned.cause.clone(),
                    dm,
                )
                .map_err(|error| CombatDamageAssignmentError::execution(planned.source, error))?;
                if dm.awaiting_choice() {
                    game.turn_store.combat_damage_assignments = assignments_checkpoint;
                    return Ok(Vec::new());
                }
                if !applied.applied {
                    continue;
                }
                total_damage_dealt = total_damage_dealt.saturating_add(assignment.amount);
                if let EventDamageTarget::Player(player) = assignment.target {
                    game.record_commander_damage(player, planned.source, assignment.amount);
                    if let Some(toxic_outcome) = apply_combat_toxic(game, planned.source, &planned.source_snapshot, player, dm)
                        .map_err(|error| CombatDamageAssignmentError::execution(planned.source, error))? {
                        applied.consequence_outcome = Some(crate::effect::EffectOutcome::aggregate(
                            applied.consequence_outcome.into_iter().chain(std::iter::once(toxic_outcome)),
                        ));
                    }
                    if dm.awaiting_choice() {
                        game.turn_store.combat_damage_assignments = assignments_checkpoint;
                        return Ok(Vec::new());
                    }
                }
                if assignment.target == planned.target {
                    damage_to_original = damage_to_original.saturating_add(assignment.amount);
                    life_lost_to_original = life_lost_to_original.saturating_add(applied.life_lost);
                    consequence_outcomes.extend(applied.consequence_outcome);
                } else {
                    redirected.push((
                        assignment.target,
                        assignment.amount,
                        applied.life_lost,
                        applied.consequence_outcome,
                    ));
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
            source_snapshot: Some(planned.source_snapshot.clone()),
            target_snapshot: None,
            source: planned.source,
            target: event_target,
            amount: damage_to_original,
            life_lost: life_lost_to_original,
            consequence_outcome: Some(crate::effect::EffectOutcome::aggregate(
                consequence_outcomes,
            )),
            result: planned.result.clone(),
            lifelink_outcome: None,
        });
        push_redirected_combat_damage_events(
            &mut events,
            &planned.result,
            planned.source,
            redirected,
        );
    }
    lifelink_totals.apply(game, &mut events, dm)?;
    if dm.awaiting_choice() {
        game.turn_store.combat_damage_assignments = assignments_checkpoint;
        return Ok(Vec::new());
    }
    excess_capacities.assign_excess(&mut events);
    finish_combat_damage_additions(game, &mut events, additions, dm)?;
    if dm.awaiting_choice() {
        game.turn_store.combat_damage_assignments = assignments_checkpoint;
        return Ok(Vec::new());
    }
    Ok(events)
}

type CombatDamageAdditions = Vec<(usize, ObjectId, crate::ids::PlayerId,
    crate::snapshot::ObjectSnapshot, crate::events::cause::EventCause,
    Vec<crate::events::processing::PreparedReplacementProgram>)>;

fn finish_combat_damage_additions(
    game: &mut GameState,
    events: &mut [CombatDamageEvent],
    additions: CombatDamageAdditions,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<(), CombatDamageAssignmentError> {
    // Freeze every original target before any added instruction can move it.
    for event in events.iter_mut().filter(|event| event.amount > 0) {
        if let DamageEventTarget::Object(target) = event.target {
            event.target_snapshot = game.object(target).map(|object|
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, game));
        }
    }
    for (index, source, controller, snapshot, cause, programs) in additions {
        let mut parent = crate::effects::ExecutionContext::new(source, controller, dm).with_cause(cause);
        parent.source_snapshot = Some(snapshot);
        let outcome = crate::effects::damage::finish_damage_replacement_programs(
            game, &mut parent, crate::effect::EffectOutcome::resolved(), programs)
            .map_err(|error| CombatDamageAssignmentError::execution(source, error))?;
        if parent.decision_maker.awaiting_choice() { return Ok(()); }
        let event = events.get_mut(index).ok_or_else(|| CombatDamageAssignmentError::execution(source,
            crate::effects::ExecutionError::InternalError("damage addition lost its original combat receipt".into())))?;
        event.consequence_outcome = Some(crate::effect::EffectOutcome::aggregate(
            event.consequence_outcome.take().into_iter().chain(std::iter::once(outcome))));
    }
    Ok(())
}

/// Pre-damage lethal/loyalty/defense of each permanent dealt combat damage
/// this step, used to compute excess damage (CR 120.10).
struct CombatExcessCapacities {
    /// (permanent, lethal damage remaining if a creature, loyalty if a
    /// planeswalker, defense if a battle)
    entries: Vec<(ObjectId, Option<u32>, Option<u32>, Option<u32>)>,
}

impl CombatExcessCapacities {
    fn before_damage(game: &GameState, targets: impl IntoIterator<Item = ObjectId>) -> Self {
        let mut entries: Vec<(ObjectId, Option<u32>, Option<u32>, Option<u32>)> = Vec::new();
        for target in targets {
            if entries.iter().any(|entry| entry.0 == target) {
                continue;
            }
            let Some(object) = game.object(target) else {
                continue;
            };
            let lethal = game
                .current_has_card_type(target, crate::types::CardType::Creature)
                .then(|| {
                    game.calculated_toughness(target)
                        .or_else(|| object.toughness())
                        .map(|toughness| (toughness - game.damage_on(target) as i32).max(0) as u32)
                })
                .flatten();
            let loyalty = game
                .current_has_card_type(target, crate::types::CardType::Planeswalker)
                .then(|| object.loyalty().unwrap_or(0));
            let defense = game
                .current_has_card_type(target, crate::types::CardType::Battle)
                .then(|| {
                    object
                        .counters
                        .get(&crate::object::CounterType::Defense)
                        .copied()
                        .unwrap_or(0)
                });
            entries.push((target, lethal, loyalty, defense));
        }
        Self { entries }
    }

    /// Spread each permanent's excess over the events that dealt it, in
    /// event order, so the events' excess sums to the total excess of all
    /// the sources together. Any deathtouch source among them makes 1 damage
    /// lethal (CR 702.2c).
    fn assign_excess(&self, events: &mut [CombatDamageEvent]) {
        for &(target, lethal, loyalty, defense) in &self.entries {
            let dealt_to_target = |event: &CombatDamageEvent| {
                event.amount > 0 && event.target == DamageEventTarget::Object(target)
            };
            let deathtouch = events
                .iter()
                .any(|event| dealt_to_target(event) && event.result.has_deathtouch);
            let lethal = lethal.map(|lethal| if deathtouch { lethal.min(1) } else { lethal });
            // The greatest excess among the permanent's types is the excess
            // over its smallest capacity.
            let Some(capacity) = [lethal, loyalty, defense].into_iter().flatten().min() else {
                continue;
            };
            let mut dealt = 0u32;
            for event in events.iter_mut().filter(|event| dealt_to_target(event)) {
                let before = dealt.max(capacity);
                dealt = dealt.saturating_add(event.amount);
                event.result.excess_damage = dealt.saturating_sub(before);
            }
        }
    }
}

#[derive(Debug)]
struct PlannedUnblockedPlayerDamage {
    source: ObjectId,
    source_snapshot: crate::snapshot::ObjectSnapshot,
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
            source_snapshot: crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(attacker, game),
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
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    let planned = plan_unblocked_player_damage(game, combat, first_strike, first_step_strikers);

    let mut events = Vec::with_capacity(planned.len());
    for planned in planned {
        let Some(event) = apply_planned_unblocked_player_damage(game, planned, dm)? else {
            return Ok(Vec::new());
        };
        events.push(event);
    }

    // Damage/life/counter application dirties derived state. The caller checks
    // one trigger event per assignment immediately after this function returns;
    // make those checks share one refreshed, prewarmed state instead of each
    // falling back to dirty single-object characteristic calculation.
    game.refresh_continuous_state();
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    view.prewarm_characteristics(&game.battlefield);
    Ok(events)
}

fn execute_unblocked_player_damage_batch_path(
    game: &mut GameState,
    combat: &CombatState,
    first_strike: bool,
    first_step_strikers: Option<&std::collections::HashSet<ObjectId>>,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Vec<CombatDamageEvent>, CombatDamageAssignmentError> {
    let mut planned = plan_unblocked_player_damage(game, combat, first_strike, first_step_strikers);
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
                source_snapshot: Some(planned.source_snapshot.clone()),
            },
        )
        .collect::<Vec<_>>();
    let processed =
        crate::events::processing::process_simultaneous_damage_assignments_with_event_with_dm(
            game, &batch, dm,
        )
        .map_err(CombatDamageAssignmentError::from)?;

    if dm.awaiting_choice() {
        return Ok(Vec::new());
    }
    for proposal in &mut planned {
        proposal.source_snapshot = combat_damage_source_snapshot(
            game, proposal.source, &proposal.source_snapshot,
        );
    }
    for proposal in &mut planned {
        proposal.source_snapshot = combat_damage_source_snapshot(
            game, proposal.source, &proposal.source_snapshot,
        );
    }



    // Replacement/prevention is collected for the entire batch first. Only
    // after every source has a final assignment do we commit actual damage.
    let mut events = Vec::with_capacity(planned.len());
    let mut lifelink_totals = CombatLifelinkTotals::default();
    let mut additions = Vec::new();
    for (planned, processed) in planned.into_iter().zip(processed) {
        if !processed.programs.is_empty() {
            additions.push((events.len(), planned.source, planned.controller,
                planned.source_snapshot.clone(), planned.cause.clone(), processed.programs));
        }
        let keywords = crate::rules::damage::SourceDamageKeywords {
            has_deathtouch: planned.result.has_deathtouch,
            has_infect: planned.result.has_infect,
            has_wither: planned.result.has_wither,
            has_lifelink: planned.result.has_lifelink,
        };
        let mut damage_to_original = 0u32;
        let mut life_lost_to_original = 0u32;
        let mut consequence_outcomes = processed.payload_outcome.into_iter().collect::<Vec<_>>();
        let mut total_damage_dealt = 0u32;
        let mut redirected = Vec::new();
        if !processed.replacement_prevented {
            for assignment in processed.assignments {
                let mut applied = crate::rules::damage::apply_processed_damage_assignment_with_dm(
                    game,
                    planned.source,
                    assignment.target,
                    assignment.amount,
                    keywords,
                    planned.cause.clone(),
                    dm,
                )
                .map_err(|error| CombatDamageAssignmentError::execution(planned.source, error))?;
                if dm.awaiting_choice() {
                    return Ok(Vec::new());
                }
                if !applied.applied {
                    continue;
                }
                total_damage_dealt = total_damage_dealt.saturating_add(assignment.amount);
                if let crate::events::DamageTarget::Player(player) = assignment.target {
                    game.record_commander_damage(player, planned.source, assignment.amount);
                    if let Some(toxic_outcome) = apply_combat_toxic(game, planned.source, &planned.source_snapshot, player, dm)
                        .map_err(|error| CombatDamageAssignmentError::execution(planned.source, error))? {
                        applied.consequence_outcome = Some(crate::effect::EffectOutcome::aggregate(
                            applied.consequence_outcome.into_iter().chain(std::iter::once(toxic_outcome)),
                        ));
                    }
                    if dm.awaiting_choice() {
                        return Ok(Vec::new());
                    }
                }
                if assignment.target == crate::events::DamageTarget::Player(planned.target) {
                    damage_to_original = damage_to_original.saturating_add(assignment.amount);
                    life_lost_to_original = life_lost_to_original.saturating_add(applied.life_lost);
                    consequence_outcomes.extend(applied.consequence_outcome);
                } else {
                    redirected.push((
                        assignment.target,
                        assignment.amount,
                        applied.life_lost,
                        applied.consequence_outcome,
                    ));
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
            source_snapshot: Some(planned.source_snapshot.clone()),
            target_snapshot: None,
            source: planned.source,
            target: DamageEventTarget::Player(planned.target),
            amount: damage_to_original,
            life_lost: life_lost_to_original,
            consequence_outcome: Some(crate::effect::EffectOutcome::aggregate(
                consequence_outcomes,
            )),
            result: planned.result.clone(),
            lifelink_outcome: None,
        });
        push_redirected_combat_damage_events(
            &mut events,
            &planned.result,
            planned.source,
            redirected,
        );
    }
    lifelink_totals.apply(game, &mut events, dm)?;
    if dm.awaiting_choice() {
        return Ok(Vec::new());
    }

    finish_combat_damage_additions(game, &mut events, additions, dm)?;
    if dm.awaiting_choice() { return Ok(Vec::new()); }
    game.refresh_continuous_state();
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    view.prewarm_characteristics(&game.battlefield);
    Ok(events)
}

fn apply_planned_unblocked_player_damage(
    game: &mut GameState,
    planned: PlannedUnblockedPlayerDamage,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Option<CombatDamageEvent>, CombatDamageAssignmentError> {
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
    let mut applied = crate::rules::damage::apply_processed_damage_assignment_with_dm(
        game,
        planned.source,
        crate::events::DamageTarget::Player(planned.target),
        planned.amount,
        keywords,
        planned.cause,
        dm,
    )
    .map_err(|error| CombatDamageAssignmentError::execution(planned.source, error))?;
    if dm.awaiting_choice() {
        return Ok(None);
    }
    let total_damage_dealt = if applied.applied { planned.amount } else { 0 };
    if applied.applied {
        game.record_commander_damage(planned.target, planned.source, planned.amount);
        if let Some(toxic_outcome) = apply_combat_toxic(game, planned.source, &planned.source_snapshot, planned.target, dm)
                        .map_err(|error| CombatDamageAssignmentError::execution(planned.source, error))? {
                        applied.consequence_outcome = Some(crate::effect::EffectOutcome::aggregate(
                            applied.consequence_outcome.into_iter().chain(std::iter::once(toxic_outcome)),
                        ));
                    }
        if dm.awaiting_choice() {
            return Ok(None);
        }
    }
    let lifelink_outcome = apply_combat_lifelink_with_dm(
        game,
        planned.source,
        planned.controller,
        &planned.result,
        total_damage_dealt,
        dm,
    )
    .map_err(|error| CombatDamageAssignmentError::execution(planned.source, error))?;
    if dm.awaiting_choice() {
        return Ok(None);
    }

    Ok(Some(CombatDamageEvent {
        source_snapshot: Some(planned.source_snapshot.clone()),
        target_snapshot: None,
        source: planned.source,
        target: DamageEventTarget::Player(planned.target),
        amount: total_damage_dealt,
        life_lost: applied.life_lost,
        consequence_outcome: applied.consequence_outcome,
        result: planned.result,
        lifelink_outcome,
    }))
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

/// Retain the resolved life event and any replacement payload notifications.
pub(super) fn apply_combat_lifelink(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    damage_result: &DamageResult,
    total_damage_dealt: u32,
) -> Option<crate::effect::EffectOutcome> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    apply_combat_lifelink_with_dm(
        game,
        source,
        controller,
        damage_result,
        total_damage_dealt,
        &mut dm,
    )
    .expect("combat lifelink execution failed")
}

/// Lifelink choices use the damage step's decision maker and transaction.
fn apply_combat_lifelink_with_dm(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    damage_result: &DamageResult,
    total_damage_dealt: u32,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Option<crate::effect::EffectOutcome>, crate::effects::ExecutionError> {
    if !damage_result.has_lifelink || total_damage_dealt == 0 {
        return Ok(None);
    }
    let snapshot = game.object(source).map(|object| {
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
    });
    let mut ctx = crate::effects::ExecutionContext::new(source, controller, dm);
    ctx.source_snapshot = snapshot;
    ctx.cause = crate::events::cause::EventCause::from_combat_damage(source, controller);
    let outcome = crate::effects::life::life_change::execute_life_change(
        game,
        &mut ctx,
        crate::events::Event::new_with_provenance(
            crate::events::LifeGainEvent::new(controller, total_damage_dealt).with_source(source),
            crate::provenance::ProvNodeId::default(),
        ),
    )?;
    Ok(Some(outcome))
}

/// Capture the source as damage is dealt, before applying damage results.
fn combat_damage_source_snapshot(
    game: &GameState,
    source: ObjectId,
    proposed_snapshot: &crate::snapshot::ObjectSnapshot,
) -> crate::snapshot::ObjectSnapshot {
    game.object(source)
        .filter(|_| !game.is_phased_out(source))
        .map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .or_else(|| {
            game.turn_store
                .turn_history
                .departed_object_snapshot(source)
                .cloned()
        })
        .unwrap_or_else(|| proposed_snapshot.clone())
}

/// CR 702.164c: combat damage dealt to a player by a creature with toxic
/// causes that creature's controller to give the player poison counters equal
/// to its total toxic value, in addition to the damage's other results.
fn apply_combat_toxic(
    game: &mut GameState,
    source: ObjectId,
    source_snapshot: &crate::snapshot::ObjectSnapshot,
    player: PlayerId,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<Option<crate::effect::EffectOutcome>, crate::effects::ExecutionError> {
    // The damage event's characteristics were frozen before applying its
    // results. A life-loss replacement may subsequently move or change the
    // source without changing this damage event's toxic result.
    let toxic: u32 = source_snapshot
        .abilities
        .iter()
        .filter_map(|ability| {
            if let AbilityKind::Static(ability) = &ability.kind {
                ability.toxic_amount()
            } else {
                None
            }
        })
        .fold(0u32, u32::saturating_add);
    if toxic == 0 {
        return Ok(None);
    }
    let controller = source_snapshot.controller;
    let cause = crate::events::cause::EventCause::from_combat_damage(source, controller);
    let mut ctx =
        crate::effects::ExecutionContext::new(source, controller, dm).with_cause(cause.clone());
    ctx.source_snapshot = Some(source_snapshot.clone());
    let event = crate::events::Event::put_player_counters(
        player,
        crate::object::CounterType::Poison,
        toxic,
        cause,
    );
    crate::effects::counters::execute_player_counter_placement(game, &mut ctx, event).map(Some)
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
    redirected: Vec<(
        crate::events::DamageTarget,
        u32,
        u32,
        Option<crate::effect::EffectOutcome>,
    )>,
) {
    for (target, amount, life_lost, consequence_outcome) in redirected {
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
            consequence_outcome,
            result: result.clone(),
            lifelink_outcome: None,
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

    fn apply(
        self,
        game: &mut GameState,
        events: &mut [CombatDamageEvent],
        dm: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<(), CombatDamageAssignmentError> {
        for (source, controller, total, event_index) in self.sources {
            let Some(event) = events.get_mut(event_index) else {
                continue;
            };
            let result = DamageResult {
                has_lifelink: true,
                ..DamageResult::default()
            };
            event.lifelink_outcome = apply_combat_lifelink_with_dm(game, source, controller, &result, total, dm)
                .map_err(|error| CombatDamageAssignmentError::execution(source, error))?;
            if dm.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
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
    /// "You may have this creature assign its combat damage as though it
    /// weren't blocked": a blocked attacker may instead assign all its combat
    /// damage to what it's attacking (CR 510.1c; never split between that and
    /// its blockers).
    pub unblocked_alternative: Option<Target>,
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
            .chain(self.separate_unblocked_alternative())
            .collect()
    }

    /// The "as though it weren't blocked" recipient, when it isn't already a
    /// recipient (a trampler's excess goes to the same place).
    fn separate_unblocked_alternative(&self) -> Option<Target> {
        self.unblocked_alternative.filter(|alternative| {
            self.excess_target != Some(*alternative)
                && !matches!(alternative, Target::Object(object)
                    if self.planeswalker == Some(*object) || self.creatures.contains(object))
        })
    }

    /// Whether `amounts` (in [`Self::targets`] order) puts every point of
    /// damage on the "as though it weren't blocked" recipient.
    fn is_all_unblocked_alternative(&self, targets: &[Target], amounts: &[u32]) -> bool {
        let Some(alternative) = self.unblocked_alternative else {
            return false;
        };
        targets
            .iter()
            .zip(amounts)
            .all(|(target, amount)| *amount == 0 || *target == alternative)
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
            // The "as though it weren't blocked" option is only taken by
            // choice; with no blockers left it is the only recipient.
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
            } else if let Some(target) = self.unblocked_alternative {
                allocations.push((target, total - assigned));
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
        // CR 510.1c: assigning as though it weren't blocked sends all of the
        // damage to what it's attacking, ignoring its blockers.
        if self.is_all_unblocked_alternative(&targets, &amounts) {
            return Ok(targets.into_iter().zip(amounts).collect());
        }
        if let Some(alternative) = self.separate_unblocked_alternative()
            && let Some(index) = targets.iter().position(|target| *target == alternative)
            && amounts[index] > 0
        {
            return Err((
                CombatDamageAssignmentErrorKind::IllegalRecipient,
                "combat damage assigned as though it weren't blocked can't be split with its blockers"
                    .to_string(),
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
    let unblocked_alternative = (blocked
        && game.object_has_static_ability_id(
            attacker_id,
            StaticAbilityId::MayAssignDamageAsUnblocked,
        ))
    .then(|| trample_excess_target(game, &attacker_info.target))
    .flatten();
    if blocked && !trample {
        // CR 510.1c: a blocked creature assigns damage only to its blockers,
        // unless it may assign it as though it weren't blocked.
        return (!creatures.is_empty() || unblocked_alternative.is_some()).then_some(
            CombatDamageDivision {
                creatures,
                planeswalker: None,
                excess_target: None,
                deathtouch,
                unblocked_alternative,
            },
        );
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
        unblocked_alternative,
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
                unblocked_alternative: None,
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
    fn combat_toxic_uses_damage_time_and_actual_departure_snapshots() {
        for blocked in [false, true] {
            for redirect in [false, true] {
                for remove_abilities in [false, true] {
                    let mut game = setup_game();
                    let alice = PlayerId::from_index(0);
                    let bob = PlayerId::from_index(1);
                    let earlier =
                        create_creature(&mut game, "Earlier damage source", 1, 1, alice, vec![]);
                    let source = create_creature(
                        &mut game,
                        "Departing toxic source",
                        2,
                        2,
                        alice,
                        vec![StaticAbility::toxic(2)],
                    );
                    let mut payload = Vec::new();
                    if remove_abilities {
                        payload.push(crate::effect::Effect::new(
                            crate::effects::ApplyContinuousEffect::new(
                                crate::continuous::EffectTarget::Specific(source),
                                crate::continuous::Modification::RemoveAllAbilities,
                                crate::effect::Until::EndOfTurn,
                            ),
                        ));
                    }
                    payload.push(crate::effect::Effect::exile(ChooseSpec::SpecificObject(
                        source,
                    )));
                    game.effect_store.replacement_effects.add_one_shot_effect(
                        ReplacementEffect::with_matcher(
                            earlier,
                            alice,
                            DamageFromSourceMatcher::new(ObjectFilter::specific(earlier)),
                            ReplacementAction::Instead(payload),
                        ),
                    );
                    let poison_redirect = if redirect {
                        let mut replacement = StaticAbility::double_player_counters_replacement(
                            crate::target::PlayerFilter::Specific(bob),
                            Some(CounterType::Poison),
                            "Redirect poison".into(),
                        )
                        .generate_replacement_effect(earlier, alice)
                        .unwrap();
                        replacement.replacement = ReplacementAction::Redirect {
                            target: crate::replacement::RedirectTarget::ToPlayer(alice),
                            which: crate::replacement::RedirectWhich::First,
                        };
                        Some(
                            game.effect_store
                                .replacement_effects
                                .add_one_shot_effect(replacement),
                        )
                    } else {
                        None
                    };
                    let mut combat = CombatState {
                        attackers: vec![
                            crate::combat_state::AttackerInfo {
                                creature: earlier,
                                target: AttackTarget::Player(bob),
                            },
                            crate::combat_state::AttackerInfo {
                                creature: source,
                                target: AttackTarget::Player(bob),
                            },
                        ],
                        ..CombatState::default()
                    };
                    if blocked {
                        let blocker = create_creature(&mut game, "Blocker", 1, 2, bob, vec![]);
                        combat.blockers.insert(earlier, vec![blocker]);
                    }
                    game.take_pending_trigger_events();
                    let mut dm = crate::decision::SelectFirstDecisionMaker;
                    let events =
                        try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm)
                            .unwrap();
                    assert!(game.object(source).is_none());
                    assert_eq!(game.player(bob).unwrap().life, 18);
                    let poison = if remove_abilities { 0 } else { 2 };
                    assert_eq!(
                        game.player(alice).unwrap().poison_counters,
                        if redirect { poison } else { 0 }
                    );
                    assert_eq!(
                        game.player(bob).unwrap().poison_counters,
                        if redirect { 0 } else { poison }
                    );
                    let damage = events.iter().find(|event| event.source == source).unwrap();
                    assert_eq!(damage.amount, 2);
                    let snapshot = damage
                        .source_snapshot
                        .as_ref()
                        .expect("damage retains departed source");
                    assert_eq!(snapshot.object_id, source);
                    let toxic = snapshot
                        .abilities
                        .iter()
                        .filter_map(|ability| {
                            if let AbilityKind::Static(ability) = &ability.kind {
                                ability.toxic_amount()
                            } else {
                                None
                            }
                        })
                        .sum::<u32>();
                    assert_eq!(
                        toxic, poison,
                        "LKI must use the departure state, not stale planning text"
                    );
                    let mut notifications = game.take_pending_trigger_events();
                    notifications.extend(
                        events
                            .iter()
                            .filter_map(|event| event.consequence_outcome.as_ref())
                            .flat_map(|outcome| outcome.events.clone()),
                    );
                    let markers = notifications
                        .iter()
                        .filter_map(|event| {
                            event
                                .downcast::<crate::events::MarkersChangedEvent>()
                                .map(|marker| (event, marker))
                        })
                        .filter(|(_, marker)| marker.is_added())
                        .collect::<Vec<_>>();
                    assert_eq!(markers.len(), usize::from(!remove_abilities));
                    if !remove_abilities {
                        let (event, marker) = markers[0];
                        assert_eq!(
                            marker.location,
                            crate::marker::MarkerLocation::Player(if redirect { alice } else { bob })
                        );
                        assert_eq!(marker.amount, 2);
                        assert_eq!(marker.source, Some(source));
                        assert_eq!(marker.source_controller, Some(alice));
                        assert_eq!(event.source_snapshot().unwrap().object_id, source);
                    }
                    if let Some(replacement) = poison_redirect {
                        assert_eq!(
                            game.effect_store
                                .replacement_effects
                                .get_effect(replacement)
                                .is_some(),
                            remove_abilities
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn combat_toxic_keeps_damage_time_text_when_life_loss_payload_changes_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(
            &mut game,
            "Toxic attacker",
            2,
            2,
            alice,
            vec![StaticAbility::toxic(2)],
        );
        let one_shot =
            game.effect_store
                .replacement_effects
                .add_one_shot_effect(ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::WouldLoseLifeMatcher::new(crate::target::PlayerFilter::Specific(
                        bob,
                    )),
                    ReplacementAction::Instead(vec![
                        crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::new(
                            crate::continuous::EffectTarget::Specific(source),
                            crate::continuous::Modification::RemoveAllAbilities,
                            crate::effect::Until::EndOfTurn,
                        )),
                        crate::effect::Effect::exile(ChooseSpec::SpecificObject(source)),
                    ]),
                ));
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        };
        game.take_pending_trigger_events();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let events =
            try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm).unwrap();
        assert!(game.object(source).is_none());
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().poison_counters, 2);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 2);
        assert_eq!(events[0].life_lost, 0);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
        let departure = game
            .turn_store
            .turn_history
            .departed_object_snapshot(source)
            .unwrap();
        assert!(!departure.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.toxic_amount().is_some())));
        let damage_snapshot = events[0].source_snapshot.as_ref().unwrap();
        assert!(damage_snapshot.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.toxic_amount() == Some(2))));
        let mut notifications = game.take_pending_trigger_events();
        notifications.extend(
            events[0]
                .consequence_outcome
                .as_ref()
                .unwrap()
                .events
                .clone(),
        );
        let poison_events = notifications
            .iter()
            .filter_map(|event| {
                event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .map(|marker| (event, marker))
            })
            .filter(|(_, marker)| marker.is_added())
            .collect::<Vec<_>>();
        assert_eq!(poison_events.len(), 1);
        let (event, marker) = poison_events[0];
        assert_eq!(marker.source, Some(source));
        assert_eq!(marker.source_controller, Some(alice));
        assert_eq!(marker.amount, 2);
        assert_eq!(marker.location, crate::marker::MarkerLocation::Player(bob));
        assert!(event.source_snapshot().unwrap().abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.toxic_amount() == Some(2))));
    }

    #[test]
    fn combat_player_counter_replacements_retain_outcomes_and_rollback_payloads() {
        struct Answers { pause: bool, pending: bool, calls: usize }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
                assert!(!self.pending, "no later counter payload questions while pending");
                self.calls += 1;
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        for infect in [false, true] {
            for action in 0..4 {
                let mut game = setup_game();
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let first = create_creature(&mut game, "First attacker", 1, 1, alice, vec![]);
                let source = create_creature(&mut game, "Counter attacker", 2, 2, alice,
                    vec![if infect { StaticAbility::infect() } else { StaticAbility::toxic(2) }]);
                let mut replacement = StaticAbility::double_player_counters_replacement(crate::target::PlayerFilter::Specific(bob), Some(CounterType::Poison), "Counter proposal".into()).generate_replacement_effect(source, alice).unwrap();
                replacement.replacement = if action == 0 {
                    ReplacementAction::Redirect { target: crate::replacement::RedirectTarget::ToPlayer(alice), which: crate::replacement::RedirectWhich::First }
                } else {
                    let mut payload = vec![crate::effect::Effect::gain_life(4)];
                    if action == 2 {
                        payload.push(crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(1)]));
                        payload.push(crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(2)]));
                    } else if action == 3 { payload.push(crate::effect::Effect::lose_life(crate::effect::Value::X)); }
                    ReplacementAction::Instead(payload)
                };
                let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(replacement);
                let combat = CombatState { attackers: vec![
                    crate::combat_state::AttackerInfo { creature: first, target: AttackTarget::Player(bob) },
                    crate::combat_state::AttackerInfo { creature: source, target: AttackTarget::Player(bob) },
                ], ..CombatState::default() };
                game.take_pending_trigger_events();
                let mut dm = Answers { pause: action == 2, pending: false, calls: 0 };
                let result = try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm);
                let events = if action >= 2 {
                    if action == 2 { assert!(result.unwrap().is_empty()); assert!(dm.pending); assert_eq!(dm.calls, 1); }
                    else { let error = result.unwrap_err(); assert_eq!(error.source, source); assert!(matches!(error.kind, CombatDamageAssignmentErrorKind::Execution(_))); }
                    assert_eq!(game.player(alice).unwrap().life, 20);
                    assert_eq!(game.player(bob).unwrap().life, 20);
                    assert_eq!(game.player(alice).unwrap().poison_counters, 0);
                    assert_eq!(game.player(bob).unwrap().poison_counters, 0);
                    assert!(game.effect_store.replacement_effects.get_effect(one_shot).is_some());
                    assert!(game.take_pending_trigger_events().is_empty());
                    if action == 3 { continue; }
                    let mut dm = Answers { pause: false, pending: false, calls: 0 };
                    let events = try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm).unwrap();
                    assert_eq!(dm.calls, 2);
                    assert!(!dm.pending);
                    events
                } else { result.unwrap() };
                assert_eq!(events.len(), 2);
                assert_eq!(events[0].amount, 1);
                assert_eq!(events[1].amount, 2);
                assert_eq!(game.player(bob).unwrap().life, if infect { 19 } else { 17 });
                assert_eq!(game.player(bob).unwrap().poison_counters, 0);
                assert_eq!(game.player(alice).unwrap().poison_counters, if action == 0 { 2 } else { 0 });
                assert_eq!(game.player(alice).unwrap().life, match action { 0 => 20, 1 => 24, _ => 27 });
                assert!(game.effect_store.replacement_effects.get_effect(one_shot).is_none());
                let notifications = events.iter().flat_map(|event| event.consequence_outcome.as_ref()).flat_map(|outcome| &outcome.events).collect::<Vec<_>>();
                let markers = notifications.iter().filter_map(|event| event.downcast::<crate::events::MarkersChangedEvent>()).collect::<Vec<_>>();
                assert_eq!(markers.len(), usize::from(action == 0));
                if action == 0 { assert_eq!(markers[0].location, crate::marker::MarkerLocation::Player(alice)); assert_eq!(markers[0].amount, 2); assert_eq!(markers[0].source, Some(source)); }
                assert_eq!(notifications.iter().filter(|event| event.downcast::<crate::events::LifeGainEvent>().is_some()).count(), match action { 0 => 0, 1 => 1, _ => 3 });
                assert!(game.take_pending_trigger_events().is_empty());
                let mut queue = crate::triggers::TriggerQueue::new();
                generate_damage_triggers(&mut game, &events, &mut queue);
                assert_eq!(game.trigger_event_kind_count_this_turn(crate::events::EventKind::Damage), 2);
                assert_eq!(game.trigger_event_kind_count_this_turn(crate::events::EventKind::MarkersChanged), u32::from(action == 0));
            }
        }
    }

    #[test]
    fn combat_damage_life_loss_redirect_preserves_resolved_recipient() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Attacker", 3, 3, alice, vec![]);
        game.effect_store
            .replacement_effects
            .add_resolution_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldLoseLifeMatcher::new(crate::target::PlayerFilter::Specific(bob)),
                ReplacementAction::Redirect {
                    target: crate::replacement::RedirectTarget::ToPlayer(alice),
                    which: crate::replacement::RedirectWhich::First,
                },
            ));
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        };
        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 3);
        assert_eq!(events[0].target, DamageEventTarget::Player(bob));
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().life, 17);
        let loss = events[0]
            .consequence_outcome
            .as_ref()
            .unwrap()
            .events
            .iter()
            .find_map(|event| event.downcast::<crate::events::LifeLossEvent>())
            .unwrap();
        assert_eq!(loss.player, alice);
        assert_eq!(loss.amount, 3);
        assert!(loss.from_damage);
    }

    #[test]
    fn combat_damage_life_loss_payload_pause_or_error_restores_the_whole_step() {
        struct Answers {
            pause: bool,
            pending: bool,
            calls: usize,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(!self.pending, "stop after unanswered input");
                self.calls += 1;
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for pending_case in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let first = create_creature(&mut game, "First attacker", 2, 2, alice, vec![]);
            let second = create_creature(&mut game, "Second attacker", 3, 3, alice, vec![]);
            let tail = if pending_case {
                crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(1)])
            } else {
                crate::effect::Effect::lose_life(crate::effect::Value::X)
            };
            let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    second,
                    alice,
                    crate::events::WouldLoseLifeMatcher::new(crate::target::PlayerFilter::Specific(
                        charlie,
                    )),
                    ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(4), tail]),
                ),
            );
            let combat = CombatState {
                attackers: vec![
                    crate::combat_state::AttackerInfo {
                        creature: first,
                        target: AttackTarget::Player(bob),
                    },
                    crate::combat_state::AttackerInfo {
                        creature: second,
                        target: AttackTarget::Player(charlie),
                    },
                ],
                ..CombatState::default()
            };
            game.take_pending_trigger_events();
            let mut dm = Answers {
                pause: pending_case,
                pending: false,
                calls: 0,
            };
            let result = try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm);
            if pending_case {
                assert!(dm.pending);
                assert_eq!(dm.calls, 1);
                assert!(result.unwrap().is_empty());
            } else {
                assert!(matches!(
                    result.unwrap_err().kind,
                    CombatDamageAssignmentErrorKind::Execution(_)
                ));
            }
            for player in [alice, bob, charlie] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(
                game.trigger_event_kind_count_this_turn(crate::events::EventKind::Damage),
                0
            );
            assert_eq!(
                game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain),
                0
            );
            if pending_case {
                dm.pause = false;
                dm.pending = false;
                dm.calls = 0;
                let events =
                    try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm).unwrap();
                assert_eq!(events.len(), 2);
                assert_eq!(dm.calls, 1);
                assert_eq!(game.player(alice).unwrap().life, 25);
                assert_eq!(game.player(bob).unwrap().life, 18);
                assert_eq!(game.player(charlie).unwrap().life, 20);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(replacement)
                        .is_none()
                );
                let life_events = events
                    .iter()
                    .flat_map(|event| event.consequence_outcome.as_ref())
                    .flat_map(|outcome| &outcome.events)
                    .collect::<Vec<_>>();
                assert_eq!(
                    life_events
                        .iter()
                        .filter(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
                        .count(),
                    2
                );
                assert_eq!(
                    life_events
                        .iter()
                        .filter(|event| event.downcast::<crate::events::LifeLossEvent>().is_some())
                        .count(),
                    1
                );
                let mut queue = crate::triggers::TriggerQueue::new();
                generate_damage_triggers(&mut game, &events, &mut queue);
                assert_eq!(
                    game.trigger_event_kind_count_this_turn(crate::events::EventKind::Damage),
                    2
                );
                assert_eq!(
                    game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain),
                    2
                );
                assert_eq!(
                    game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeLoss),
                    1
                );
            }
        }
    }

    #[test]
    fn combat_lifelink_redirect_preserves_resolved_player_and_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(
            &mut game,
            "Lifelink attacker",
            3,
            3,
            alice,
            vec![StaticAbility::lifelink()],
        );
        game.effect_store
            .replacement_effects
            .add_resolution_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldGainLifeMatcher::you(),
                ReplacementAction::Redirect {
                    target: crate::replacement::RedirectTarget::ToPlayer(bob),
                    which: crate::replacement::RedirectWhich::First,
                },
            ));
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        };
        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 3);
        let gain = events[0]
            .lifelink_outcome
            .as_ref()
            .unwrap()
            .events
            .iter()
            .find_map(|event| event.downcast::<crate::events::LifeGainEvent>())
            .unwrap();
        assert_eq!(gain.player, bob);
        assert_eq!(gain.amount, 3);
        assert_eq!(gain.source, Some(source));
        let mut queue = crate::triggers::TriggerQueue::new();
        generate_damage_triggers(&mut game, &events, &mut queue);
        assert_eq!(
            game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain),
            1
        );
    }

    #[test]
    fn combat_lifelink_instead_executes_payload_without_original_gain() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(
            &mut game,
            "Lifelink attacker",
            3,
            3,
            alice,
            vec![StaticAbility::lifelink()],
        );
        game.effect_store
            .replacement_effects
            .add_resolution_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldGainLifeMatcher::you(),
                ReplacementAction::Instead(vec![crate::effect::Effect::lose_life(4)]),
            ));
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        };
        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(game.player(alice).unwrap().life, 16);
        assert_eq!(game.player(bob).unwrap().life, 17);
        assert_eq!(events[0].amount, 3);
        let mut queue = crate::triggers::TriggerQueue::new();
        generate_damage_triggers(&mut game, &events, &mut queue);
        assert_eq!(
            game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain),
            0
        );
        assert_eq!(
            game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeLoss),
            2
        );
    }

    #[test]
    fn combat_lifelink_payload_pause_or_error_restores_damage_and_replays_once() {
        struct Answers {
            pause: bool,
            pending: bool,
            calls: usize,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(!self.pending);
                self.calls += 1;
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for pending_case in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let first = create_creature(&mut game, "First attacker", 2, 2, alice, vec![]);
            let source = create_creature(
                &mut game,
                "Lifelink attacker",
                3,
                3,
                alice,
                vec![StaticAbility::lifelink()],
            );
            let tail = if pending_case {
                crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(1)])
            } else {
                crate::effect::Effect::lose_life(crate::effect::Value::X)
            };
            let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::WouldGainLifeMatcher::you(),
                    ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(4), tail]),
                ),
            );
            let combat = CombatState {
                attackers: vec![
                    crate::combat_state::AttackerInfo {
                        creature: first,
                        target: AttackTarget::Player(bob),
                    },
                    crate::combat_state::AttackerInfo {
                        creature: source,
                        target: AttackTarget::Player(bob),
                    },
                ],
                ..CombatState::default()
            };
            game.take_pending_trigger_events();
            let mut dm = Answers {
                pause: pending_case,
                pending: false,
                calls: 0,
            };
            let result = try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm);
            if pending_case {
                assert!(result.unwrap().is_empty());
                assert!(dm.pending);
                assert_eq!(dm.calls, 1);
            } else {
                assert!(matches!(
                    result.unwrap_err().kind,
                    CombatDamageAssignmentErrorKind::Execution(_)
                ));
            }
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(
                game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain),
                0
            );
            if pending_case {
                dm.pause = false;
                dm.pending = false;
                dm.calls = 0;
                let events =
                    try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm).unwrap();
                assert_eq!(events.len(), 2);
                assert_eq!(events[0].amount, 2);
                assert_eq!(events[1].amount, 3);
                assert_eq!(game.player(alice).unwrap().life, 25);
                assert_eq!(game.player(bob).unwrap().life, 15);
                assert_eq!(dm.calls, 1);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(replacement)
                        .is_none()
                );
                let outcome = events[1].lifelink_outcome.as_ref().unwrap();
                assert_eq!(
                    outcome.count_or_zero(),
                    0,
                    "original lifelink gain was replaced"
                );
                assert_eq!(outcome.events.len(), 2);
                let mut queue = crate::triggers::TriggerQueue::new();
                generate_damage_triggers(&mut game, &events, &mut queue);
                assert_eq!(
                    game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain),
                    2
                );
                assert_eq!(
                    game.trigger_event_kind_count_this_turn(crate::events::EventKind::Damage),
                    2
                );
                assert_eq!(
                    game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeLoss),
                    2
                );
            }
        }
    }

    #[test]
    fn combat_lifelink_fast_path_retains_source_and_single_notification() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(
            &mut game,
            "Lifelink attacker",
            3,
            3,
            alice,
            vec![StaticAbility::lifelink()],
        );
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        };
        game.refresh_continuous_state();
        assert!(can_use_unblocked_player_damage_fast_path(&game, &combat));
        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(game.player(alice).unwrap().life, 23);
        assert_eq!(game.player(bob).unwrap().life, 17);
        let outcome = events[0].lifelink_outcome.as_ref().unwrap();
        assert_eq!(outcome.events.len(), 1);
        let gain = outcome.events[0]
            .downcast::<crate::events::LifeGainEvent>()
            .unwrap();
        assert_eq!(gain.player, alice);
        assert_eq!(gain.source, Some(source));
        let mut queue = crate::triggers::TriggerQueue::new();
        generate_damage_triggers(&mut game, &events, &mut queue);
        assert_eq!(
            game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain),
            1
        );
    }

    #[test]
    fn combat_lifelink_split_damage_uses_modified_total_and_history_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Trampling lifelink attacker", 3, 3, alice,
            vec![StaticAbility::trample(), StaticAbility::lifelink()]);
        let blocker = create_creature(&mut game, "Blocker", 0, 1, bob, vec![]);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(source, alice, crate::events::WouldGainLifeMatcher::you(), ReplacementAction::Double));
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(source, alice, crate::events::WouldGainLifeMatcher::you(),
                ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(crate::effect::Value::EventValue(crate::effect::EventValueSpec::Amount))])));
        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo { creature: source, target: AttackTarget::Player(bob) }],
            blockers: std::collections::BTreeMap::from([(source, vec![blocker])]),
            ..CombatState::default()
        };
        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(game.damage_on(blocker), 1);
        assert_eq!(game.player(bob).unwrap().life, 18);
        assert_eq!(game.player(alice).unwrap().life, 26);
        let outcomes = events.iter().filter_map(|event| event.lifelink_outcome.as_ref()).collect::<Vec<_>>();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].count_or_zero(), 0);
        assert_eq!(outcomes[0].events.len(), 1);
        let gain = outcomes[0].events[0].downcast::<crate::events::LifeGainEvent>().unwrap();
        assert_eq!(gain.amount, 6);
        let mut queue = crate::triggers::TriggerQueue::new();
        generate_damage_triggers(&mut game, &events, &mut queue);
        assert_eq!(game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain), 1);
    }

    #[test]
    fn damage_payload_pause_or_error_restores_combat_and_replays_notifications_once() {
        struct Answers {
            pause: bool,
            pending: bool,
            calls: usize,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(
                    !self.pending,
                    "do not ask the next payload question while suspended"
                );
                self.calls += 1;
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for prevention_case in [false, true] {
            for pending_case in [false, true] {
                let mut game = setup_game();
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let first = create_creature(&mut game, "First attacker", 2, 2, alice, vec![]);
                let second = create_creature(&mut game, "Second attacker", 3, 3, alice, vec![]);
                let shield_source = create_creature(&mut game, "Shield source", 0, 1, alice, vec![]);
                let mut payload = vec![crate::effect::Effect::gain_life(4)];
                if pending_case {
                    payload.push(crate::effect::Effect::may(vec![
                        crate::effect::Effect::gain_life(1),
                    ]));
                    payload.push(crate::effect::Effect::may(vec![
                        crate::effect::Effect::gain_life(2),
                    ]));
                } else {
                    payload.push(crate::effect::Effect::lose_life(crate::effect::Value::X));
                }
                let one_shot = if prevention_case {
                    game.effect_store.prevention_effects.add_shield(
                        crate::prevention::PreventionShield::prevent_next_n(
                            shield_source,
                            alice,
                            crate::prevention::PreventionTarget::Player(bob),
                            2,
                        )
                        .with_filter(crate::prevention::DamageFilter {
                            from_source: Some(ObjectFilter::specific(second)),
                            ..Default::default()
                        })
                        .with_follow_up_effects(payload),
                    );
                    None
                } else {
                    Some(game.effect_store.replacement_effects.add_one_shot_effect(
                        ReplacementEffect::with_matcher(
                            second,
                            alice,
                            crate::events::damage::matchers::DamageFromSourceMatcher::new(
                                ObjectFilter::specific(second),
                            ),
                            ReplacementAction::Instead(payload),
                        ),
                    ))
                };
                let combat = CombatState {
                    attackers: vec![
                        crate::combat_state::AttackerInfo {
                            creature: first,
                            target: AttackTarget::Player(bob),
                        },
                        crate::combat_state::AttackerInfo {
                            creature: second,
                            target: AttackTarget::Player(bob),
                        },
                    ],
                    ..CombatState::default()
                };
                game.take_pending_trigger_events();
                let mut dm = Answers {
                    pause: pending_case,
                    pending: false,
                    calls: 0,
                };
                let result = try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm);
                if pending_case {
                    assert!(dm.pending);
                    assert_eq!(dm.calls, 1);
                    assert!(result.unwrap().is_empty());
                } else {
                    let error = result.unwrap_err();
                    assert!(matches!(
                        error.kind,
                        CombatDamageAssignmentErrorKind::Execution(_)
                    ));
                    assert_eq!(
                        error.source,
                        if prevention_case {
                            shield_source
                        } else {
                            second
                        }
                    );
                }
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert!(game.take_pending_trigger_events().is_empty());
                assert_eq!(game.effect_store.trigger_matching_holds, 0);
                assert!(
                    !game
                        .effect_store
                        .prevention_effects
                        .follow_ups_are_deferred()
                );
                if let Some(id) = one_shot {
                    assert!(
                        game.effect_store
                            .replacement_effects
                            .get_effect(id)
                            .is_some()
                    );
                } else {
                    let shields = game.effect_store.prevention_effects.shields();
                    assert_eq!(shields.len(), 1);
                    assert_eq!(shields[0].amount_remaining, Some(2));
                }
                if pending_case {
                    dm.pause = false;
                    dm.pending = false;
                    dm.calls = 0;
                    let events =
                        try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut dm)
                            .unwrap();
                    assert_eq!(dm.calls, 2);
                    assert_eq!(game.player(alice).unwrap().life, 27);
                    assert_eq!(
                        game.player(bob).unwrap().life,
                        if prevention_case { 17 } else { 18 }
                    );
                    if let Some(id) = one_shot {
                        assert!(
                            game.effect_store
                                .replacement_effects
                                .get_effect(id)
                                .is_none()
                        );
                        let replacement_event =
                            events.iter().find(|event| event.source == second).unwrap();
                        assert_eq!(replacement_event.amount, 0);
                        assert_eq!(
                            replacement_event
                                .consequence_outcome
                                .as_ref()
                                .unwrap()
                                .events
                                .len(),
                            3
                        );
                    } else {
                        assert!(game.effect_store.prevention_effects.shields().is_empty());
                    }
                    let mut queue = crate::triggers::TriggerQueue::new();
                    generate_damage_triggers(&mut game, &events, &mut queue);
                    drain_pending_trigger_events(&mut game, &mut queue);
                    assert_eq!(
                        game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeGain),
                        3
                    );
                    assert_eq!(
                        game.trigger_event_kind_count_this_turn(crate::events::EventKind::Damage),
                        if prevention_case { 2 } else { 1 }
                    );
                    assert_eq!(game.effect_store.trigger_matching_holds, 0);
                    assert!(
                        !game
                            .effect_store
                            .prevention_effects
                            .follow_ups_are_deferred()
                    );
                }
            }
        }
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

        let mut combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo { creature: attacker, target: AttackTarget::Player(bob) }],
            ..CombatState::default()
        };
        combat.blockers.insert(attacker, vec![blocker]);
        let events = try_execute_combat_damage_step(&mut game, &combat, false).unwrap();
        let damage_dealt: u32 = events.iter().filter(|event| event.source == attacker && event.target == DamageEventTarget::Object(blocker)).map(|event| event.amount).sum();
        let total_damage_dealt: u32 = events.iter().filter(|event| event.source == attacker).map(|event| event.amount).sum();

        assert_eq!(damage_dealt, 1);
        assert_eq!(total_damage_dealt, 1);
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
        ).expect("damage test proposal must process successfully");
        assert_eq!(processed.assignments.len(), 1);
        assert_eq!(processed.assignments[0].amount, 3);

        let damage_result = {
            let attacker_obj = game.object(attacker).expect("attacker exists");
            calculate_damage_with_game(&game, attacker_obj, DamageTarget::Permanent, 1, true)
        };
        assert_eq!(damage_result.minus_counters, 1);
        assert_eq!(damage_result.damage_dealt, 0);

        let mut combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo { creature: attacker, target: AttackTarget::Player(bob) }],
            ..CombatState::default()
        };
        combat.blockers.insert(attacker, vec![blocker]);
        let events = try_execute_combat_damage_step(&mut game, &combat, false).unwrap();
        let damage_dealt: u32 = events.iter().filter(|event| event.source == attacker && event.target == DamageEventTarget::Object(blocker)).map(|event| event.amount).sum();
        let total_damage_dealt: u32 = events.iter().filter(|event| event.source == attacker).map(|event| event.amount).sum();

        assert_eq!(damage_dealt, 3);
        assert_eq!(total_damage_dealt, 3);
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

        let combat = CombatState {
            attackers: vec![crate::combat_state::AttackerInfo { creature: attacker, target: AttackTarget::Player(bob) }],
            ..CombatState::default()
        };
        let events = try_execute_combat_damage_step(&mut game, &combat, false).unwrap();
        let damage_dealt: u32 = events.iter().filter(|event| event.source == attacker && event.target == DamageEventTarget::Player(bob)).map(|event| event.amount).sum();
        let life_lost: u32 = events.iter().filter(|event| event.source == attacker && event.target == DamageEventTarget::Player(bob)).map(|event| event.life_lost).sum();
        let total_damage_dealt: u32 = events.iter().filter(|event| event.source == attacker).map(|event| event.amount).sum();

        assert_eq!(damage_dealt, 1);
        assert_eq!(life_lost, 0);
        assert_eq!(total_damage_dealt, 1);
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
