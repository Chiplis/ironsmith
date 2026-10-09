//! Turn structure and priority system for MTG.
//!
//! This module handles:
//! - Turn and phase progression (untap, upkeep, draw, main, combat, etc.)
//! - Priority passing and resolution
//! - Step-specific actions (untapping, drawing, cleanup)

use crate::DecisionMaker;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::{
    AddedStepPlacement, GameState, Phase, ScheduledStep, Step, TurnScheduleDestination,
};
use crate::ids::PlayerId;

/// Errors that can occur during turn progression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnError {
    /// Cannot advance past the current step/phase.
    CannotAdvance,
    /// No players left in the game.
    NoPlayersRemaining,
    /// Invalid state for the requested operation.
    InvalidState { message: String },
}

impl std::fmt::Display for TurnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TurnError::CannotAdvance => f.write_str("Cannot advance the turn"),
            TurnError::NoPlayersRemaining => f.write_str("No players remain in the game"),
            TurnError::InvalidState { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for TurnError {}

/// Result of passing priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityResult {
    /// More players need to pass priority.
    Continue,
    /// All players passed in succession; resolve the top of the stack.
    StackResolves,
    /// All players passed with an empty stack; the phase/step ends.
    PhaseEnds,
}

/// Tracks consecutive priority passes for determining when all players have passed.
#[derive(Debug, Clone, Default)]
pub struct PriorityTracker {
    /// Number of consecutive passes without any player taking an action.
    pub consecutive_passes: usize,
    /// Number of players in the game (for determining when all have passed).
    pub players_in_game: usize,
}

impl PriorityTracker {
    /// Creates a new priority tracker for the given number of players.
    pub fn new(players_in_game: usize) -> Self {
        Self {
            consecutive_passes: 0,
            players_in_game,
        }
    }

    /// Records a priority pass. Returns true if all players have now passed.
    pub fn record_pass(&mut self) -> bool {
        self.consecutive_passes += 1;
        self.consecutive_passes >= self.players_in_game
    }

    /// Resets the pass counter (called when a player takes an action).
    pub fn reset(&mut self) {
        self.consecutive_passes = 0;
    }

    /// Updates the number of players (called when a player leaves the game).
    pub fn set_players_in_game(&mut self, count: usize) {
        self.players_in_game = count;
    }

    /// Returns true if all players have passed in succession.
    pub fn all_passed(&self) -> bool {
        self.consecutive_passes >= self.players_in_game
    }
}

/// Returns the next step within a phase, or None if the phase is over.
pub fn next_step(phase: Phase, current_step: Option<Step>) -> Option<Step> {
    match (phase, current_step) {
        // Beginning phase
        (Phase::Beginning, Some(Step::Untap)) => Some(Step::Upkeep),
        (Phase::Beginning, Some(Step::Upkeep)) => Some(Step::Draw),
        (Phase::Beginning, Some(Step::Draw)) => None,
        (Phase::Beginning, None) => Some(Step::Untap),

        // Main phases have no steps
        (Phase::FirstMain, _) => None,
        (Phase::NextMain, _) => None,

        // Combat phase
        (Phase::Combat, Some(Step::BeginCombat)) => Some(Step::DeclareAttackers),
        (Phase::Combat, Some(Step::DeclareAttackers)) => Some(Step::DeclareBlockers),
        (Phase::Combat, Some(Step::DeclareBlockers)) => Some(Step::CombatDamage),
        (Phase::Combat, Some(Step::CombatDamage)) => Some(Step::EndCombat),
        (Phase::Combat, Some(Step::EndCombat)) => None,
        (Phase::Combat, None) => Some(Step::BeginCombat),

        // Ending phase
        (Phase::Ending, Some(Step::End)) => Some(Step::Cleanup),
        (Phase::Ending, Some(Step::Cleanup)) => None,
        (Phase::Ending, None) => Some(Step::End),

        // Invalid combinations
        _ => None,
    }
}

/// Returns the next phase after the given phase.
pub fn next_phase(phase: Phase) -> Option<Phase> {
    match phase {
        Phase::Beginning => Some(Phase::FirstMain),
        Phase::FirstMain => Some(Phase::Combat),
        Phase::Combat => Some(Phase::NextMain),
        Phase::NextMain => Some(Phase::Ending),
        Phase::Ending => None, // Turn ends
    }
}

/// Returns the first step of a phase, if any.
pub fn first_step_of_phase(phase: Phase) -> Option<Step> {
    match phase {
        Phase::Beginning => Some(Step::Untap),
        Phase::FirstMain => None,
        Phase::Combat => Some(Step::BeginCombat),
        Phase::NextMain => None,
        Phase::Ending => Some(Step::End),
    }
}

/// Advances the game to the next step within the current phase.
/// If at the end of a phase, advances to the next phase.
/// If at the end of the turn, advances to the next turn.
pub fn advance_step(game: &mut GameState) -> Result<(), TurnError> {
    if game.players_in_game() == 0 {
        return Err(TurnError::NoPlayersRemaining);
    }

    let phase = game.turn.phase;
    let Some(step) = game.turn.step else {
        return advance_phase(game);
    };
    if let Some(next) = next_step(phase, Some(step)) {
        legacy_finish_step(game, step, TurnScheduleDestination::Step(next))
    } else {
        legacy_finish_step_and_phase(game, step, phase, destination_after_phase(phase))
    }
}

/// Advances the game to the next phase.
/// If at the end of the turn, advances to the next turn.
pub fn advance_phase(game: &mut GameState) -> Result<(), TurnError> {
    if game.players_in_game() == 0 {
        return Err(TurnError::NoPlayersRemaining);
    }

    let phase = game.turn.phase;
    legacy_finish_phase(game, phase, destination_after_phase(phase))
}

fn destination_after_phase(phase: Phase) -> TurnScheduleDestination {
    next_phase(phase)
        .map(TurnScheduleDestination::Phase)
        .unwrap_or(TurnScheduleDestination::Complete)
}

fn legacy_prepend_scheduled_steps(game: &mut GameState, mut steps: Vec<ScheduledStep>) {
    if steps.is_empty() {
        return;
    }
    steps.append(&mut game.turn_store.pending_added_steps);
    game.turn_store.pending_added_steps = steps;
}

fn legacy_activate_next_scheduled_step(game: &mut GameState) -> Result<(), TurnError> {
    loop {
        let Some(next) = game.turn_store.pending_added_steps.first().copied() else {
            game.turn_store.active_added_step = None;
            let continuation = game
                .turn_store
                .added_step_continuation
                .take()
                .unwrap_or(TurnScheduleDestination::Complete);
            return legacy_resolve_schedule_destination(game, continuation);
        };
        game.turn_store.pending_added_steps.remove(0);

        let before = game.take_added_steps(AddedStepPlacement::BeforeStep(next.step));
        if !before.is_empty() {
            let mut sequence = before;
            sequence.push(next);
            legacy_prepend_scheduled_steps(game, sequence);
            continue;
        }

        game.turn_store.active_added_step = Some(next);
        return legacy_enter_step(game, next.step);
    }
}

fn legacy_start_scheduled_steps(
    game: &mut GameState,
    steps: Vec<ScheduledStep>,
    continuation: TurnScheduleDestination,
) -> Result<(), TurnError> {
    game.turn_store.pending_added_steps = steps;
    game.turn_store.active_added_step = None;
    game.turn_store.added_step_continuation = Some(continuation);
    legacy_activate_next_scheduled_step(game)
}

fn legacy_resolve_schedule_destination(
    game: &mut GameState,
    destination: TurnScheduleDestination,
) -> Result<(), TurnError> {
    if matches!(destination, TurnScheduleDestination::ResumePhaseSchedule) {
        return legacy_resume_phase_schedule(game);
    }
    let first_step = match destination {
        TurnScheduleDestination::Step(step) => Some(step),
        TurnScheduleDestination::Phase(phase) => first_step_of_phase(phase),
        TurnScheduleDestination::CombatDamageFirstStrike
        | TurnScheduleDestination::CombatDamageRegular => Some(Step::CombatDamage),
        _ => None,
    };
    if let Some(step) = first_step {
        let before = game.take_added_steps(AddedStepPlacement::BeforeStep(step));
        if !before.is_empty() {
            return legacy_start_scheduled_steps(game, before, destination);
        }
    }

    match destination {
        TurnScheduleDestination::Step(step) => legacy_enter_step(game, step),
        TurnScheduleDestination::CombatDamageFirstStrike
        | TurnScheduleDestination::CombatDamageRegular => {
            legacy_enter_step(game, Step::CombatDamage)
        }
        TurnScheduleDestination::Phase(phase) => legacy_enter_phase(game, phase),
        TurnScheduleDestination::Complete => {
            game.next_turn();
            Ok(())
        }
        TurnScheduleDestination::ResumePhaseSchedule => unreachable!(),
    }
}

fn legacy_enter_step(game: &mut GameState, step: Step) -> Result<(), TurnError> {
    if step != Step::Upkeep {
        game.clear_forecast_revealed_hand_cards();
    }
    game.turn.phase = step.containing_phase();
    game.turn.step = Some(step);
    if game.consume_step_skip(game.turn.active_player, step) {
        let phase = game.turn.phase;
        let ends_phase = game
            .turn_store
            .active_added_step
            .is_some_and(|scheduled| scheduled.isolated_phase)
            || next_step(phase, Some(step)).is_none();
        if ends_phase {
            legacy_finish_step_and_phase(game, step, phase, destination_after_phase(phase))
        } else {
            let next = next_step(phase, Some(step)).expect("nonfinal step has a successor");
            legacy_finish_step(game, step, TurnScheduleDestination::Step(next))
        }
    } else {
        if step == Step::DeclareAttackers {
            if let Some(combat) = game.combat.as_mut() {
                combat.last_attack_declaration_step_players = None;
            }
        }
        game.reset_priority_for_new_window();
        Ok(())
    }
}

fn legacy_enter_phase(game: &mut GameState, phase: Phase) -> Result<(), TurnError> {
    let active = game.turn.active_player;
    let skip_main = matches!(phase, Phase::FirstMain | Phase::NextMain)
        && game
            .turn_store
            .skip_current_turn_main_phases
            .contains(&active);
    let skip_combat = matches!(phase, Phase::Combat)
        && (game
            .turn_store
            .skip_current_turn_combat_phases
            .contains(&active)
            || game.turn_store.skip_next_combat_phases.remove(&active)
            || game.turn_store.pending_combat_phase_skips.remove(&active));
    game.clear_forecast_revealed_hand_cards();
    game.turn.phase = phase;
    game.turn.step = first_step_of_phase(phase);
    if skip_main || skip_combat {
        return legacy_finish_phase(game, phase, destination_after_phase(phase));
    }
    if matches!(phase, Phase::Combat) {
        game.mark_combat_phase_started();
    }
    if matches!(phase, Phase::FirstMain | Phase::NextMain) {
        game.mark_main_phase_started();
    }
    if let Some(step) = game.turn.step
        && game.consume_step_skip(active, step)
    {
        let ends_phase = next_step(phase, Some(step)).is_none();
        if ends_phase {
            return legacy_finish_step_and_phase(game, step, phase, destination_after_phase(phase));
        }
        let next = next_step(phase, Some(step)).expect("nonfinal step has a successor");
        return legacy_finish_step(game, step, TurnScheduleDestination::Step(next));
    }
    game.reset_priority_for_new_window();
    Ok(())
}

fn legacy_prepare_phase_schedule(game: &mut GameState, normal_next: TurnScheduleDestination) {
    if game.turn_store.phase_schedule_continuation.is_none() {
        game.turn_store.phase_schedule_continuation = Some(
            game.turn_store
                .additional_phase_continuation
                .take()
                .map(TurnScheduleDestination::Phase)
                .unwrap_or(normal_next),
        );
    }
}

fn legacy_resume_phase_schedule(game: &mut GameState) -> Result<(), TurnError> {
    if let Some((phase, only_step)) = game.pop_additional_phase() {
        if let Some(step) = only_step {
            return legacy_start_scheduled_steps(
                game,
                vec![ScheduledStep {
                    phase,
                    step,
                    isolated_phase: true,
                }],
                TurnScheduleDestination::ResumePhaseSchedule,
            );
        }
        return legacy_resolve_schedule_destination(game, TurnScheduleDestination::Phase(phase));
    }
    let continuation = game
        .turn_store
        .phase_schedule_continuation
        .take()
        .unwrap_or(TurnScheduleDestination::Complete);
    legacy_resolve_schedule_destination(game, continuation)
}

fn legacy_finish_step(
    game: &mut GameState,
    step: Step,
    normal_next: TurnScheduleDestination,
) -> Result<(), TurnError> {
    if step == Step::Draw {
        game.finish_draw_step_tracking();
    }
    let additions = game.take_added_steps(AddedStepPlacement::AfterStep(step));
    let active = game.turn_store.active_added_step.take();
    if let Some(scheduled) = active {
        if scheduled.isolated_phase {
            game.queue_added_step_phases_after(scheduled.phase);
        }
        legacy_prepend_scheduled_steps(game, additions);
        return legacy_activate_next_scheduled_step(game);
    }
    if additions.is_empty() {
        legacy_resolve_schedule_destination(game, normal_next)
    } else {
        legacy_start_scheduled_steps(game, additions, normal_next)
    }
}

fn legacy_finish_step_and_phase(
    game: &mut GameState,
    step: Step,
    phase: Phase,
    normal_next: TurnScheduleDestination,
) -> Result<(), TurnError> {
    if step == Step::Draw {
        game.finish_draw_step_tracking();
    }
    let additions = game.take_added_steps(AddedStepPlacement::AfterStep(step));
    let active = game.turn_store.active_added_step.take();
    if active.is_none() || active.is_some_and(|scheduled| scheduled.isolated_phase) {
        game.queue_added_step_phases_after(phase);
    }
    if active.is_some() {
        legacy_prepend_scheduled_steps(game, additions);
        return legacy_activate_next_scheduled_step(game);
    }

    legacy_prepare_phase_schedule(game, normal_next);
    if additions.is_empty() {
        legacy_resume_phase_schedule(game)
    } else {
        legacy_start_scheduled_steps(
            game,
            additions,
            TurnScheduleDestination::ResumePhaseSchedule,
        )
    }
}

fn legacy_finish_phase(
    game: &mut GameState,
    phase: Phase,
    normal_next: TurnScheduleDestination,
) -> Result<(), TurnError> {
    if phase == Phase::Beginning && game.turn.step == Some(Step::Draw) {
        game.finish_draw_step_tracking();
    }

    if matches!(phase, Phase::Combat) {
        game.cleanup_effects_end_of_combat();
    }
    game.queue_added_step_phases_after(phase);
    legacy_prepare_phase_schedule(game, normal_next);
    legacy_resume_phase_schedule(game)
}

/// Returns true if the given player currently has priority.
pub fn has_priority(game: &GameState, player: PlayerId) -> bool {
    game.team_has_priority(player)
}

/// Returns the current priority holder, if any.
pub fn priority_holder(game: &GameState) -> Option<PlayerId> {
    game.turn.priority_player
}

/// Passes priority for the current player.
/// Returns the result indicating what should happen next.
pub fn pass_priority(game: &mut GameState, tracker: &mut PriorityTracker) -> PriorityResult {
    tracker.set_players_in_game(if game.grand_melee().is_some() {
        game.priority_players_for_current_turn().len()
    } else {
        game.teams_in_game()
    });
    if tracker.record_pass() {
        // All players have passed
        if game.stack_is_empty() {
            PriorityResult::PhaseEnds
        } else {
            PriorityResult::StackResolves
        }
    } else {
        // Move priority to next player
        advance_priority_to_next_player(game);
        PriorityResult::Continue
    }
}

/// Resets priority to the active player (called after a spell/ability is put on stack).
pub fn reset_priority(game: &mut GameState, tracker: &mut PriorityTracker) {
    tracker.reset();
    game.reset_priority_for_new_window();
}

/// Resets pass tracking and returns priority to the player who just took an action.
///
/// Per CR 117.3c, if a player had priority when they cast a spell, activate an
/// ability, or take a special action, that same player receives priority again
/// afterward.
pub fn priority_after_player_action(
    game: &mut GameState,
    tracker: &mut PriorityTracker,
    player: PlayerId,
) {
    tracker.reset();
    game.turn.priority_player = Some(player);
}

/// Advances priority to the next player in turn order.
fn advance_priority_to_next_player(game: &mut GameState) {
    let current = match game.turn.priority_player {
        Some(p) => p,
        None => return,
    };

    if game.grand_melee().is_some() {
        game.turn.priority_player = game.next_grand_melee_priority_player_after(current);
        return;
    }

    if let Some(next_team) = game.next_priority_team_representative_after(current) {
        game.turn.priority_player = Some(next_team);
        return;
    }

    let current_index = game
        .turn_store
        .turn_order
        .iter()
        .position(|&p| p == current)
        .unwrap_or(0);

    // Find next player who is still in the game
    for i in 1..=game.turn_store.turn_order.len() {
        let next_index = (current_index + i) % game.turn_store.turn_order.len();
        let next_player = game.turn_store.turn_order[next_index];

        if game.player(next_player).is_some_and(|p| p.is_in_game()) {
            game.turn.priority_player = Some(next_player);
            return;
        }
    }
}

/// Returns true if it's currently "sorcery timing" - main phase with empty stack.
pub fn is_sorcery_timing(game: &GameState) -> bool {
    matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain) && game.stack_is_empty()
}

/// Returns true if the current step doesn't grant priority (untap, cleanup normally).
pub fn is_no_priority_step(game: &GameState) -> bool {
    matches!(game.turn.step, Some(Step::Untap) | Some(Step::Cleanup))
}

fn execute_delayed_untap_step_actions(
    game: &mut GameState,
    active_players: &[PlayerId],
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    for &player in active_players {
        if decision_maker.awaiting_choice() {
            return Ok(());
        }
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::PermanentsUntapStepEvent::new(player),
            crate::provenance::ProvNodeId::default(),
        );
        let actions = crate::triggers::check::take_delayed_untap_step_actions(game, &event);
        for action in actions {
            let source = action
                .ability_source_stable_id
                .and_then(|stable_id| game.find_object_by_stable_id(stable_id))
                .or(action.ability_source)
                .or_else(|| action.target_objects.first().copied())
                .unwrap_or_else(|| crate::ids::ObjectId::from_raw(0));
            let mut ctx =
                crate::effects::ExecutionContext::new(source, action.controller, decision_maker);
            ctx.source_snapshot = action.ability_source_snapshot;
            ctx.tagged_objects = action.tagged_objects;
            ctx.tagged_players = action.tagged_players;
            if let Some(x_value) = action.x_value {
                ctx = ctx.with_x(x_value);
            }
            let events = crate::game_loop::execute_resolution_program(
                game,
                &mut ctx,
                action.controller,
                source,
                &action.effects,
                None,
                &[],
            )
            .map_err(|error| {
                crate::effects::ExecutionError::InternalError(format!(
                    "delayed untap action failed: {error}"
                ))
            })?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
            game.effect_store.pending_trigger_events.extend(events);
        }
    }
    Ok(())
}

/// Executes the untap step for the active player.
/// Untaps all permanents controlled by the active player (except those that don't untap).
pub fn execute_untap_step(game: &mut GameState) {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    execute_untap_step_with(game, &mut dm).expect("untap step execution failed");
}

/// Exact facts at an untap occurrence's beginning. A suspended lane retains
/// these even if other lanes create effects or change a land's controller.
#[derive(Debug, Clone)]
pub(crate) struct UntapStepBoundary {
    started_at: (u32, u64),
    beginning_expirations: Vec<crate::continuous::ContinuousEffectId>,
}

pub(crate) fn capture_untap_step_boundary(
    game: &mut GameState,
) -> Result<UntapStepBoundary, crate::effects::ExecutionError> {
    use crate::effect::Until;
    let started_at = (game.turn.turn_number, game.effect_store.continuous_effects.current_timestamp());
    game.refresh_continuous_state()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    let active_players = game.turn_players();
    let beginning_expirations = game.effect_store.continuous_effects.effects().iter()
        .filter_map(|effect| {
            let Until::UntilControllersNextUntapStep {
                object: ironsmith_core::ContinuousDurationObject::Specific(object),
            } = &effect.duration else { return None; };
            let controller = game.object(*object)
                .filter(|object| object.zone == crate::zone::Zone::Battlefield)
                .map(|object| game.controller_of(object));
            controller.is_some_and(|player| active_players.contains(&player)).then_some(effect.id)
        })
        .collect();
    Ok(UntapStepBoundary { started_at, beginning_expirations })
}

/// Executes one actual untap occurrence with transactional optional choices.
pub fn execute_untap_step_with(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    // CR 614.10: a static "skip your untap step" (Stasis) skips the occurrence.
    if game.player_skips_untap_step(game.turn.active_player) {
        return Ok(());
    }
    let checkpoint = game.clone();
    let result = capture_untap_step_boundary(game)
        .and_then(|boundary| execute_untap_step_inner(game, decision_maker, &boundary));
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint;
    }
    result
}

/// Resume the same occurrence, retaining its original registration cutoff and
/// beginning-expiry decisions while fresh game facts remain available to actions.
pub(crate) fn execute_untap_step_with_boundary(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
    boundary: &UntapStepBoundary,
) -> Result<(), crate::effects::ExecutionError> {
    let checkpoint = game.clone();
    let result = execute_untap_step_inner(game, decision_maker, boundary);
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint;
    }
    result
}

fn execute_untap_step_inner(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
    boundary: &UntapStepBoundary,
) -> Result<(), crate::effects::ExecutionError> {
    use crate::decisions::context::BooleanContext;
    use crate::effect::Until;
    use crate::static_abilities::StaticAbilityId;

    let active_player = game.turn.active_player;
    let active_players = game.turn_players();
    let had_restriction_effects = !game.effect_store.restriction_effects.is_empty();
    // The untap gate below reads the cached continuous-effect snapshot; on a
    // dirty state that snapshot can predate freshly generated static effects
    // (e.g. an aura attached since the last refresh), which would skip the
    // characteristics check entirely.
    game.refresh_continuous_state()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    game.turn_store.untap_step_started_at = Some(boundary.started_at);
    // CR 500.4 expires `until this step` before phasing. Membership was
    // frozen at the original beginning, not recomputed after a suspended choice.
    if !boundary.beginning_expirations.is_empty() {
        for &id in &boundary.beginning_expirations {
            game.effect_store.continuous_effects.remove_effect(id);
        }
        game.refresh_continuous_state()
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    }
    game.update_cant_effects();
    game.establish_turn_start_continuous_control();

    // CR 702.26a performs one simultaneous phasing exchange before untapping:
    // visible permanents with phasing phase out, while permanents that phased
    // out directly under this player's control phase in. Snapshot both sides
    // before changing either status.
    let phase_out = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| !game.is_phased_out(*id))
        .filter(|id| {
            game.current_controller(*id)
                .is_some_and(|controller| active_players.contains(&controller))
        })
        .filter(|id| game.current_has_static_ability_id(*id, StaticAbilityId::Phasing))
        .filter(|id| game.can_phase_out(*id))
        .collect::<Vec<_>>();
    let phase_in = active_players
        .iter()
        .flat_map(|player| game.directly_phased_out_under(*player))
        .filter(|id| game.can_phase_in(*id))
        .collect::<Vec<_>>();
    game.phase_simultaneously(&phase_out, &phase_in);

    // Delayed instructions with "as you untap your permanents" timing are
    // turn-based actions, not triggered abilities. They resolve here without
    // using the stack, before any permanent actually becomes untapped.
    execute_delayed_untap_step_actions(game, &active_players, decision_maker)?;
    if decision_maker.awaiting_choice() {
        return Ok(());
    }

    // The delayed action can move a static-ability source away before the
    // simultaneous untap, so rebuild the calculated state it may have changed.
    game.refresh_continuous_state()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    game.update_cant_effects();
    // Freeze which existing exact restrictions this completed step consumes.
    // Replacement additions during untapping cannot create or retarget one of
    // these next-step occurrences retroactively.
    let consumed_untap_restrictions = game.effect_store.restriction_effects.iter()
        .filter(|effect| matches!(effect.duration, Until::ControllersNextUntapStep | Until::YourNextUntapStep
            | Until::PlayersNextUntapStep { .. })
            && (!matches!(effect.duration, Until::PlayersNextUntapStep { .. })
                || effect.is_active(game, game.turn.turn_number))
            && effect.untap_step_player(game).is_some_and(|player| active_players.contains(&player)))
        .map(|effect| effect.timestamp)
        .collect::<std::collections::HashSet<_>>();
    let may_have_untap_static_abilities = game_may_have_untap_static_abilities(game);
    let has_cant_untap_restrictions = !game.effect_store.cant_effects.cant_untap.is_empty();

    // Get all permanents controlled by active player, plus any permanents that
    // other players untap during this untap step (Seedborn Muse-style effects).
    let mut permanents: std::collections::HashSet<_> = active_players
        .iter()
        .flat_map(|player| game.permanents_controlled_by(*player))
        .collect();
    if may_have_untap_static_abilities {
        for source_id in game.battlefield.clone() {
            let Some(source) = game.object(source_id) else {
                continue;
            };
            if active_players.contains(&game.controller_of(source)) {
                continue;
            }
            let source_controller = game.controller_of(source);
            let filter_ctx = game.filter_context_for(source_controller, Some(source_id));
            let Some(source_chars) = game.current_characteristics(source_id) else {
                continue;
            };
            for static_ability in &source_chars.static_abilities {
                let Some(filter) =
                    static_ability.untap_during_each_other_players_untap_step_filter()
                else {
                    continue;
                };
                for &candidate_id in &game.battlefield {
                    if let Some(candidate) = game.object(candidate_id)
                        && filter.matches(candidate, &filter_ctx, game)
                    {
                        permanents.insert(candidate_id);
                    }
                }
            }
        }
    }
    let mut permanents: Vec<_> = permanents.into_iter().collect();
    permanents.sort_by_key(|id| id.0);

    // First pass: collect which permanents should untap
    let should_untap: std::collections::HashSet<_> = if !may_have_untap_static_abilities
        && !has_cant_untap_restrictions
    {
        permanents.iter().copied().collect()
    } else {
        permanents
            .iter()
            .filter_map(|&id| {
                if decision_maker.awaiting_choice() {
                    return None;
                }
                let obj = game.object(id)?;
                let chars = game.current_characteristics(id)?;
                // Check if the permanent has "doesn't untap during your untap step"
                let controller = game.controller_of(obj);
                let controlled_by_active = active_players.contains(&controller);
                let has_doesnt_untap = controlled_by_active
                    && chars.static_abilities.iter().any(|static_ability| {
                        static_ability.affects_untap() && static_ability.is_active(game, id)
                    });
                let has_optional_choice = controlled_by_active
                    && chars.static_abilities.iter().any(|static_ability| {
                        static_ability.id() == StaticAbilityId::MayChooseNotToUntapDuringUntapStep
                    });
                let untap_player = if controlled_by_active {
                    controller
                } else {
                    active_player
                };
                let blocked_by_restriction = !game.can_untap_during_step(id, untap_player);
                if has_doesnt_untap || blocked_by_restriction {
                    None
                } else if has_optional_choice && game.is_tapped(id) {
                    let choice_ctx = BooleanContext::new(
                        controller,
                        Some(id),
                        format!("untap {} during your untap step", obj.name),
                    );
                    decision_maker
                        .decide_boolean(game, &choice_ctx)
                        .then_some(id)
                } else {
                    Some(id)
                }
            })
            .collect()
    };

    if decision_maker.awaiting_choice() {
        return Ok(());
    }
    let should_untap = if may_have_untap_static_abilities {
        apply_untap_step_limits(
            game,
            &active_players,
            &permanents,
            should_untap,
            decision_maker,
        )
    } else {
        should_untap
    };

    if decision_maker.awaiting_choice() {
        return Ok(());
    }

    // Every replacement choice precedes the simultaneous original untaps.
    // CR 502.3 / 122.1d includes stun-counter replacement. There is no
    // priority here; published events wait for the upkeep trigger drain.
    let before = crate::events::other::before_tap_state_snapshots(game);
    let mut prepared = Vec::new();
    for id in permanents {
        if !should_untap.contains(&id) {
            continue;
        }
        let actor = before
            .get(&id)
            .map(|snapshot| snapshot.controller)
            .unwrap_or(game.turn.active_player);
        game.clear_pending_decision_controllers();
        let mut ctx = crate::effects::ExecutionContext::new(id, actor, &mut *decision_maker);
        let original =
            crate::events::processing::prepare_untap_with_execution_context(game, id, &mut ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        prepared.push((id, actor, original));
    }
    let mut ctx = crate::effects::ExecutionContext::new(
        crate::ids::ObjectId::from_raw(0),
        game.turn.active_player,
        &mut *decision_maker,
    );
    let outputs = crate::effects::composition::execute_simultaneous_originals_with_outputs(
        game,
        &mut ctx,
        true,
        |game, ctx| {
            let mut receipts = Vec::new();
            for (id, actor, original) in prepared {
                let mut child =
                    crate::effects::ExecutionContext::new(id, actor, &mut *ctx.decision_maker);
                let receipt = crate::events::processing::commit_prepared_untap_with_outputs(
                    game, &mut child, original,
                )?;
                let receipt =
                    crate::effects::composition::with_original_execution_context(receipt, &child);
                receipts.push(receipt);
                if child.decision_maker.awaiting_choice() {
                    return Ok(Vec::new());
                }
            }
            Ok(receipts)
        },
        |game, _, receipts| {
            let mut observed: Vec<_> = receipts
                .iter()
                .flat_map(|receipt| receipt.outcome.outcome.events.iter().cloned())
                .collect();
            crate::events::other::bind_before_tap_state_snapshots(&mut observed, &before);
            crate::events::other::group_tap_state_events(game, &mut observed, Default::default());
            for receipt in receipts {
                crate::effects::composition::inherit_original_observations(
                    &mut receipt.outcome.outcome,
                    &observed,
                );
                receipt.outcome.synchronize_observations();
            }
            Ok(crate::effects::composition::OriginalTriggerObservation::Capture)
        },
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(());
    }
    let mut untap_events: Vec<_> = outputs
        .into_iter()
        .flat_map(|output| output.outcome.events)
        .collect();
    crate::events::other::bind_before_tap_state_snapshots(&mut untap_events, &before);
    crate::events::other::group_tap_state_events(game, &mut untap_events, Default::default());
    for event in untap_events {
        game.queue_trigger_event(event.provenance(), event);
    }

    for effect in &mut game.effect_store.restriction_effects {
        if consumed_untap_restrictions.contains(&effect.timestamp) {
            // "next two untap steps": each consumed step uses one up.
            if effect.additional_untap_steps > 0 {
                effect.additional_untap_steps -= 1;
            } else {
                effect.consumed_next_untap = true;
            }
        }
    }
    let current_turn = game.turn.turn_number;
    game.effect_store
        .restriction_effects
        .retain(|effect| !effect.is_expired(current_turn));
    if had_restriction_effects {
        game.update_cant_effects();
    }

    // No priority during untap step
    game.turn.priority_player = None;
    Ok(())
}

/// "Players can't untap more than N <filter> during their untap steps"
/// (Winter Orb, Smoke, Stoic Angel). When more matching tapped permanents would
/// untap than a limit allows, their controller chooses which ones untap.
fn apply_untap_step_limits(
    game: &GameState,
    active_players: &[crate::ids::PlayerId],
    permanents: &[crate::ids::ObjectId],
    mut should_untap: std::collections::HashSet<crate::ids::ObjectId>,
    decision_maker: &mut impl DecisionMaker,
) -> std::collections::HashSet<crate::ids::ObjectId> {
    use crate::decisions::context::{SelectObjectsContext, SelectableObject};
    use crate::filter::ObjectFilterExt as _;

    let mut limits = Vec::new();
    for &source_id in &game.battlefield {
        if game.is_phased_out(source_id) {
            continue;
        }
        let Some(chars) = game.current_characteristics(source_id) else {
            continue;
        };
        for static_ability in &chars.static_abilities {
            let Some((player, filter, max)) = static_ability.untap_step_limit_spec() else {
                continue;
            };
            if !static_ability.is_active(game, source_id) {
                continue;
            }
            limits.push((source_id, player.clone(), filter.clone(), max));
        }
    }

    for (source_id, player_filter, filter, max) in limits {
        let source_controller = game
            .current_controller(source_id)
            .unwrap_or(game.turn.active_player);
        let filter_ctx = game.filter_context_for(source_controller, Some(source_id));
        for &player in active_players {
            if !crate::filter::player_filter_matches_game(&player_filter, player, game, &filter_ctx)
            {
                continue;
            }
            let candidates: Vec<_> = permanents
                .iter()
                .copied()
                .filter(|id| should_untap.contains(id) && game.is_tapped(*id))
                .filter(|id| game.current_controller(*id) == Some(player))
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                })
                .collect();
            let max = max as usize;
            if candidates.len() <= max {
                continue;
            }
            let selectable = candidates
                .iter()
                .map(|id| SelectableObject {
                    id: *id,
                    name: game
                        .object(*id)
                        .map_or_else(String::new, |object| object.name.to_string()),
                    legal: true,
                    selection_identity: None,
                    reveal_policy: None,
                })
                .collect();
            let ctx = SelectObjectsContext::new(
                player,
                Some(source_id),
                "untap during your untap step",
                selectable,
                max,
                Some(max),
            );
            let mut chosen: Vec<_> = decision_maker
                .decide_objects(game, &ctx)
                .into_iter()
                .filter(|id| candidates.contains(id))
                .collect();
            if decision_maker.awaiting_choice() {
                return should_untap;
            }
            chosen.dedup();
            chosen.truncate(max);
            for id in &candidates {
                if chosen.len() < max && !chosen.contains(id) {
                    chosen.push(*id);
                }
            }
            for id in candidates {
                if !chosen.contains(&id) {
                    should_untap.remove(&id);
                }
            }
        }
    }
    should_untap
}

fn game_may_have_untap_static_abilities(game: &GameState) -> bool {
    game.cached_continuous_effects_snapshot()
        .iter()
        .any(|effect| modification_may_affect_untap(&effect.modification))
        || game
            .battlefield
            .iter()
            .copied()
            .any(|id| object_may_have_untap_static_abilities(game, id))
        || game
            .stack
            .iter()
            .any(|entry| object_may_have_untap_static_abilities(game, entry.object_id))
}

fn object_may_have_untap_static_abilities(
    game: &GameState,
    object_id: crate::ids::ObjectId,
) -> bool {
    let Some(object) = game.object(object_id) else {
        return false;
    };
    object
        .abilities
        .iter()
        .any(|ability| ability.functions_in(&object.zone) && ability_may_affect_untap(ability))
}

fn modification_may_affect_untap(modification: &crate::continuous::Modification) -> bool {
    use crate::continuous::Modification;

    match modification {
        // Text rewrites can introduce arbitrary static abilities.
        Modification::CopyOf { .. }
        | Modification::ChangeText { .. }
        | Modification::RewriteText(_)
        | Modification::SetTextBox(_) => true,
        // Materializes StaticAbility::doesnt_untap() in calculated
        // characteristics (see apply path in continuous.rs).
        Modification::Restriction(restriction) => {
            static_ability_may_affect_untap(restriction.ability())
        }
        Modification::AddAbility(static_ability) => static_ability_may_affect_untap(static_ability),
        Modification::AddAbilityGeneric(ability) => ability_may_affect_untap(ability),
        Modification::SetAbilities(abilities) => abilities.iter().any(ability_may_affect_untap),
        _ => false,
    }
}

fn ability_may_affect_untap(ability: &crate::ability::Ability) -> bool {
    matches!(&ability.kind, crate::ability::AbilityKind::Static(static_ability)
        if static_ability_may_affect_untap(static_ability))
}

fn static_ability_may_affect_untap(
    static_ability: &crate::static_abilities::StaticAbility,
) -> bool {
    use crate::static_abilities::StaticAbilityId;

    static_ability.affects_untap()
        || static_ability.untap_step_limit_spec().is_some()
        || static_ability.id() == StaticAbilityId::MayChooseNotToUntapDuringUntapStep
        || static_ability
            .untap_during_each_other_players_untap_step_filter()
            .is_some()
}

/// Executes the draw step for the active player.
/// Active player draws a card.
///
/// Returns the completed draw observations, or the typed execution failure after
/// restoring the draw-step checkpoint. No resource meter is required by callers.
pub fn execute_draw_step(
    game: &mut GameState,
) -> Result<Vec<crate::triggers::TriggerEvent>, crate::effects::ExecutionError> {
    let mut dm = crate::decision::AutoPassDecisionMaker;
    execute_draw_step_with(game, &mut dm)
}

/// Executes the draw step for the active player with an explicit decision maker.
pub fn execute_draw_step_with(
    game: &mut GameState,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Vec<crate::triggers::TriggerEvent>, crate::effects::ExecutionError> {
    let active_players = game.turn_players();
    if active_players.is_empty() {
        game.reset_priority_for_new_window();
        return Ok(Vec::new());
    }
    if active_players
        .iter()
        .any(|player| game.player_skips_draw_step(*player))
        || game.consume_step_skip(game.turn.active_player, Step::Draw)
    {
        game.reset_priority_for_new_window();
        return Ok(Vec::new());
    }

    let checkpoint = game.clone();
    let mut events = Vec::new();
    for player in active_players {
        match execute_draw_step_for_player_with(game, player, decision_maker) {
            Ok(drawn) => events.extend(drawn),
            Err(error) => {
                game.restore_execution_checkpoint(checkpoint, false);
                game.record_token_resource_failure(&error);
                return Err(error);
            }
        }
        if decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, true);
            return Ok(Vec::new());
        }
    }
    game.reset_priority_for_new_window();
    Ok(events)
}

fn execute_draw_step_for_player_with(
    game: &mut GameState,
    active_player: PlayerId,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Vec<crate::triggers::TriggerEvent>, crate::effects::ExecutionError> {
    use crate::events::other::CardsDrawnEvent;
    use crate::triggers::TriggerEvent;

    if !game
        .player(active_player)
        .is_some_and(|player| player.is_in_game())
    {
        game.reset_priority_for_new_window();
        return Ok(Vec::new());
    }
    let (is_during_players_draw_step, cards_previously_drawn_this_draw_step) =
        game.draw_step_context_for_player(active_player);
    if game.should_skip_first_turn_draw(active_player) {
        game.reset_priority_for_new_window();
        return Ok(Vec::new());
    }

    // Check if player can draw (the draw step draw is the first draw of the turn)
    let is_first_draw = !game
        .turn_store
        .turn_history
        .has_drawn_cards_this_turn(active_player)?;

    // Check for "can't draw extra cards" restriction (e.g., Narset)
    // The draw step draw is only blocked if they've already drawn this turn
    let can_draw = if !game.can_draw_extra_cards(active_player) {
        // Only allow if they haven't drawn yet this turn
        is_first_draw
    } else {
        true
    };

    let mut draw_events = Vec::new();

    if can_draw {
        let draw_event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::CardsDrawn);
        let completed = crate::effects::cards::draw_cards_with_miracle_window(
            game,
            active_player,
            1,
            is_first_draw,
            decision_maker,
            draw_event_provenance,
        )?;
        if decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        let drawn = completed.cards;

        // Create a single CardsDrawnEvent if any cards were drawn
        if !drawn.is_empty() {
            let event = CardsDrawnEvent::new_with_step_context(
                active_player,
                drawn,
                is_first_draw,
                is_during_players_draw_step,
                cards_previously_drawn_this_draw_step,
            )
            .with_miracle_decision(completed.miracle);
            let miracle_reveal =
                crate::effects::cards::miracle_reveal_event(game, &event, draw_event_provenance);
            let event = TriggerEvent::new_with_provenance(event, draw_event_provenance);
            if let Some(drawn_event) = event.downcast::<CardsDrawnEvent>() {
                game.record_cards_drawn_in_current_draw_step(active_player, drawn_event.amount());
            }
            game.stage_turn_history_event(&event);
            game.note_hidden_draw_for_reveal_window(&event);
            draw_events.push(event);
            for reveal in miracle_reveal {
                game.stage_turn_history_event(&reveal);
                draw_events.push(reveal);
            }
            for reveal_event in completed.automatic_reveals {
                game.stage_turn_history_event(&reveal_event);
                draw_events.push(reveal_event);
            }
        }
    }

    Ok(draw_events)
}

/// Checks if the active player needs to discard during cleanup.
/// Returns a spec and player ID if the player must choose which cards to discard.
pub fn get_cleanup_discard_spec(
    game: &GameState,
) -> Option<(PlayerId, crate::decisions::specs::DiscardToHandSizeSpec)> {
    get_cleanup_discard_spec_after(game, None)
}

/// Like [`get_cleanup_discard_spec`], but only considers active players after
/// `after` in turn-player order. CR 805.4 / 514.1: in a shared team turn every
/// active player discards down to their own maximum hand size, so the cleanup
/// procedure asks each teammate in turn.
pub fn get_cleanup_discard_spec_after(
    game: &GameState,
    after: Option<PlayerId>,
) -> Option<(PlayerId, crate::decisions::specs::DiscardToHandSizeSpec)> {
    use crate::decisions::specs::DiscardToHandSizeSpec;

    let turn_players = game.turn_players();
    let start = after
        .and_then(|after| turn_players.iter().position(|player| *player == after))
        .map_or(0, |index| index + 1);
    for active_player in turn_players.into_iter().skip(start) {
        let Some(player) = game.player(active_player) else {
            continue;
        };
        let max_hand = player.max_hand_size.max(0) as usize;
        let excess = player.hand.len().saturating_sub(max_hand);

        if excess > 0 {
            return Some((
                active_player,
                DiscardToHandSizeSpec::new(excess, player.hand.to_vec()),
            ));
        }
    }

    None
}

/// Applies the discard chosen by the player during cleanup.
pub fn apply_cleanup_discard(
    game: &mut GameState,
    cards_to_discard: &[crate::ids::ObjectId],
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Vec<crate::ids::ObjectId>, crate::effects::ExecutionError> {
    let checkpoint = game.clone();
    let opened_batch = game.open_simultaneous_action();
    let result = apply_cleanup_discard_inner(game, cards_to_discard, decision_maker);
    game.close_simultaneous_action(opened_batch);
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint;
    }
    if decision_maker.awaiting_choice() {
        return Ok(Vec::new());
    }
    result
}

fn apply_cleanup_discard_inner(
    game: &mut GameState,
    cards_to_discard: &[crate::ids::ObjectId],
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Vec<crate::ids::ObjectId>, crate::effects::ExecutionError> {
    use crate::effects::ExecutionContext;
    use crate::events::cause::EventCause;
    if cards_to_discard.is_empty() {
        return Ok(Vec::new());
    }
    let source = cards_to_discard[0];
    let source_snapshot = game
        .object(source)
        .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game));
    let player = source_snapshot
        .as_ref()
        .map(|snapshot| snapshot.owner)
        .unwrap_or(game.turn.active_player);
    let cause = EventCause::from_game_rule();
    let provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::Discard);
    // Cleanup chooses the complete hand subset before replacement preparation.
    // Keep its rule-caused original source policy, then restore completion evidence.
    let mut ctx = ExecutionContext::new(source, player, decision_maker)
        .with_cause(cause)
        .with_provenance(provenance);
    let Some(prepared) = crate::effects::cards::prepare_selected_discard_batch(
        game,
        &mut ctx,
        player,
        cards_to_discard.to_vec(),
        None,
        true,
    )?
    else {
        return Ok(Vec::new());
    };
    ctx.source_snapshot = source_snapshot;
    let committed = crate::effects::cards::commit_selected_discard_batch(game, &mut ctx, prepared)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(Vec::new());
    }
    let Some(originals) = committed.prepare_completion_with_outputs(game, &mut ctx)? else {
        return Ok(Vec::new());
    };
    let madness_cards = originals.madness_cards.clone();
    let mut outcome = originals
        .complete_added_programs_with_outputs(game, &mut ctx)?
        .into_outcome();
    if ctx.decision_maker.awaiting_choice() {
        return Ok(Vec::new());
    }
    crate::effects::retain_unmatched_outcome_events(game, &mut outcome.events);
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
    Ok(madness_cards)
}

/// Executes CR 514.2 cleanup (damage removal and expiring turn effects).
/// Mana empties only after the cleanup step ends, through TurnRunner's
/// resumable boundary owner; it must remain available during 514.3 priority.
pub fn execute_cleanup_step(game: &mut GameState) {
    game.cleanup_damage_and_regeneration_end_of_turn();

    // Clear one-shot replacement effects (like regeneration shields)
    // These only last "until end of turn" per MTG rules
    game.effect_store
        .replacement_effects
        .clear_one_shot_effects();
    game.effect_store
        .replacement_effects
        .clear_until_end_of_turn_effects();

    // Prevention shields with an end-of-turn duration expire during cleanup.
    // Keep only accumulated metrics still owned by a pending delayed trigger;
    // this covers "the next end step" instructions created after the current
    // turn's end-step trigger point.
    let retained_prevention_metrics = game
        .effect_store
        .delayed_triggers
        .iter()
        .filter_map(|delayed| delayed.prevention_shield)
        .collect::<std::collections::HashSet<_>>();
    game.effect_store
        .prevention_effects
        .cleanup_end_of_turn_retaining_metrics(&retained_prevention_metrics);

    // Clean up expired grants (e.g., flashback from Snapcaster Mage)
    let turn_number = game.turn.turn_number;
    let battlefield = game.battlefield.clone();
    game.effect_store
        .grant_registry
        .cleanup_expired(turn_number, &battlefield);

    game.effect_store.attack_player_requirements.clear();
    game.cleanup_restrictions_end_of_turn();
    game.cleanup_mana_spend_permissions_end_of_turn();
    game.cleanup_granted_mana_abilities_end_of_turn();
    game.cleanup_temporary_spell_cost_reductions_end_of_turn();
    game.cleanup_temporary_spell_ability_grants_end_of_turn();
    game.cleanup_repeatable_mana_payment_actions_end_of_turn();
    game.cleanup_temporary_object_static_ability_grants_end_of_turn();

    // End "until end of turn" effects would happen here
    // (Handled by continuous effect manager)
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.cleanup_player_control_end_of_turn();
    game.cleanup_combat_choice_control_end_of_turn();
    game.cleanup_vote_control_end_of_turn();

    // Restrictions are materialized into CantEffectTracker. Rebuild it only
    // after both direct restriction instances and continuous effects have
    // expired so end-of-turn "can't" effects cannot remain cached.
    game.refresh_continuous_state();
    game.update_cant_effects();

    // Normally no priority during cleanup, but if triggers/SBAs happen, there's a new cleanup
    game.turn.priority_player = None;
}

/// Returns a human-readable description of the current phase/step.
pub fn current_phase_description(game: &GameState) -> String {
    let phase_name = match game.turn.phase {
        Phase::Beginning => "Beginning",
        Phase::FirstMain => "Precombat Main",
        Phase::Combat => "Combat",
        Phase::NextMain => "Postcombat Main",
        Phase::Ending => "Ending",
    };

    if let Some(step) = game.turn.step {
        let step_name = match step {
            Step::Untap => "Untap",
            Step::Upkeep => "Upkeep",
            Step::Draw => "Draw",
            Step::BeginCombat => "Beginning of Combat",
            Step::DeclareAttackers => "Declare Attackers",
            Step::DeclareBlockers => "Declare Blockers",
            Step::CombatDamage => "Combat Damage",
            Step::EndCombat => "End of Combat",
            Step::End => "End Step",
            Step::Cleanup => "Cleanup",
        };
        format!("{} Phase - {} Step", phase_name, step_name)
    } else {
        format!("{} Phase", phase_name)
    }
}

/// Checks if the game is in a main phase (pre or post combat).
pub fn is_main_phase(game: &GameState) -> bool {
    matches!(game.turn.phase, Phase::FirstMain | Phase::NextMain)
}

/// Checks if the game is in the combat phase.
pub fn is_combat_phase(game: &GameState) -> bool {
    game.turn.phase == Phase::Combat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{Restriction, Until};
    use crate::ids::{CardId, ObjectId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_artifact(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        abilities: Vec<StaticAbility>,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        for ability in abilities {
            obj.abilities_mut().push(Ability::static_ability(ability));
        }
        game.add_object(obj);
        id
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[test]
    fn generic_step_advancement_clears_forecast_reveal_on_draw_entry() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        let card = CardBuilder::new(CardId::new(), "Forecast Step Probe").build();
        let source = game.create_object_from_card(&card, alice, Zone::Hand);
        assert!(game.reveal_hand_card_until_upkeep_ends(source));

        advance_step(&mut game).expect("advance to draw");

        assert_eq!(game.turn.step, Some(Step::Draw));
        assert!(!game.is_hand_card_revealed_until_upkeep_ends(source));
    }

    #[test]
    fn legacy_advancement_runs_added_steps_before_and_after_named_steps() {
        let mut game = setup_game();
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        game.add_step_after(Step::Upkeep, Step::Upkeep);
        game.add_step_before(Step::Upkeep, Step::Draw);

        advance_step(&mut game).expect("finish the normal upkeep");
        assert_eq!(game.turn.step, Some(Step::Upkeep));
        assert!(game.turn_store.active_added_step.is_some());

        advance_step(&mut game).expect("finish the after-upkeep addition");
        assert_eq!(game.turn.step, Some(Step::Upkeep));
        assert!(game.turn_store.active_added_step.is_some());

        advance_step(&mut game).expect("finish the before-draw addition");
        assert_eq!(game.turn.step, Some(Step::Draw));
        assert!(game.turn_store.active_added_step.is_none());
    }

    #[test]
    fn legacy_phase_schedule_orders_full_and_synthetic_phases_by_creation() {
        let mut newer_full_phase = setup_game();
        newer_full_phase.turn.turn_number = 2;
        newer_full_phase.turn.phase = Phase::FirstMain;
        newer_full_phase.turn.step = None;
        newer_full_phase.add_step_after_phase(Step::Draw, Phase::FirstMain);
        newer_full_phase.add_additional_phase_group([Phase::Combat]);

        advance_phase(&mut newer_full_phase).expect("leave first main");
        assert_eq!(newer_full_phase.turn.phase, Phase::Combat);
        assert_eq!(newer_full_phase.turn.step, Some(Step::BeginCombat));
        newer_full_phase.turn.step = None;
        advance_phase(&mut newer_full_phase).expect("finish inserted combat");
        assert_eq!(newer_full_phase.turn.phase, Phase::Beginning);
        assert_eq!(newer_full_phase.turn.step, Some(Step::Draw));
        assert!(
            newer_full_phase
                .turn_store
                .active_added_step
                .is_some_and(|scheduled| scheduled.isolated_phase)
        );

        let mut newer_synthetic_phase = setup_game();
        newer_synthetic_phase.turn.turn_number = 2;
        newer_synthetic_phase.turn.phase = Phase::FirstMain;
        newer_synthetic_phase.turn.step = None;
        newer_synthetic_phase.add_additional_phase_group([Phase::Combat]);
        newer_synthetic_phase.add_step_after_phase(Step::Draw, Phase::FirstMain);

        advance_phase(&mut newer_synthetic_phase).expect("leave first main");
        assert_eq!(newer_synthetic_phase.turn.phase, Phase::Beginning);
        assert_eq!(newer_synthetic_phase.turn.step, Some(Step::Draw));
    }

    #[test]
    fn legacy_counted_step_skips_survive_into_an_extra_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.skip_next_step(alice, Step::Draw);
        game.skip_next_step(alice, Step::Draw);

        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        advance_step(&mut game).expect("consume the first draw skip");
        assert_eq!(game.turn.phase, Phase::FirstMain);
        assert_eq!(game.pending_step_skips(alice, Step::Draw), 1);

        game.turn_store.extra_turns.push(alice);
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::Cleanup);
        advance_step(&mut game).expect("start the queued extra turn");
        assert_eq!(game.turn.active_player, alice);

        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        advance_step(&mut game).expect("consume the second draw skip in the extra turn");
        assert_eq!(game.turn.phase, Phase::FirstMain);
        assert_eq!(game.pending_step_skips(alice, Step::Draw), 0);
    }

    #[derive(Default)]
    struct AlwaysYesDecisionMaker;

    impl DecisionMaker for AlwaysYesDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    #[derive(Default)]
    struct AlwaysNoDecisionMaker;

    impl DecisionMaker for AlwaysNoDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }

    #[test]
    fn cleanup_batches_sparse_damage_state_without_dirtying_an_empty_mana_pool() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let persistent = create_artifact(&mut game, "Persistent Damage", alice, vec![]);
        let ordinary = create_artifact(&mut game, "Ordinary Damage", alice, vec![]);
        game.refresh_continuous_state();
        game.mark_damage(persistent, 2);
        game.mark_damage(ordinary, 3);
        game.keep_damage_marked(persistent);
        game.add_regeneration_shield(ordinary, 2);
        assert!(game.use_regeneration_shield(ordinary));
        assert!(game.continuous_state_is_clean());
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );

        execute_cleanup_step(&mut game);

        assert_eq!(game.damage_on(persistent), 2);
        assert_eq!(game.damage_on(ordinary), 0);
        assert_eq!(game.regeneration_shield_count(ordinary), 0);
        assert_eq!(game.regenerated_this_turn_count(ordinary), 0);
        assert!(
            game.continuous_state_is_clean(),
            "clearing an already-empty mana pool must not invalidate continuous state"
        );

        let before_sba = game.work_counters();
        assert!(crate::rules::state_based::check_state_based_actions(&game).is_empty());
        let after_sba = game.work_counters();
        assert_eq!(
            after_sba.static_ability_regens, before_sba.static_ability_regens,
            "a clean post-cleanup SBA check should reuse cached static effects"
        );
    }

    #[test]
    fn cleanup_expires_prevention_shields_but_retains_delayed_prevention_metrics() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let ordinary_shield = game.effect_store.prevention_effects.add_shield(
            crate::prevention::PreventionShield::prevent_next_n(
                source,
                alice,
                crate::prevention::PreventionTarget::You,
                2,
            ),
        );
        let delayed_metric = game.effect_store.prevention_effects.add_shield(
            crate::prevention::PreventionShield::prevent_next_n(
                source,
                alice,
                crate::prevention::PreventionTarget::You,
                3,
            ),
        );
        let result =
            game.effect_store
                .prevention_effects
                .apply_chosen_shield(delayed_metric, 3, true, None);
        assert_eq!(result.remaining, 0);
        assert_eq!(
            game.effect_store
                .prevention_effects
                .prevented_by_shield(delayed_metric),
            3
        );

        crate::effects::delayed::queue_delayed_trigger(
            &mut game,
            crate::effects::delayed::DelayedTriggerConfig::new(
                crate::triggers::Trigger::beginning_of_end_step(PlayerFilter::Any),
                Vec::<crate::effect::Effect>::new(),
                true,
                Vec::new(),
                alice,
            )
            .with_prevention_shield(Some(delayed_metric)),
        );

        execute_cleanup_step(&mut game);

        assert!(
            game.effect_store.prevention_effects.shields().is_empty(),
            "ordinary end-of-turn prevention shields must expire during cleanup"
        );
        assert_eq!(
            game.effect_store
                .prevention_effects
                .prevented_by_shield(ordinary_shield),
            0,
            "an unreferenced shield must not leave a cleanup metric behind"
        );
        assert_eq!(
            game.effect_store
                .prevention_effects
                .prevented_by_shield(delayed_metric),
            3,
            "a pending delayed registration must retain its completed metric"
        );

        game.effect_store.delayed_triggers.clear();
        execute_cleanup_step(&mut game);
        assert_eq!(
            game.effect_store
                .prevention_effects
                .prevented_by_shield(delayed_metric),
            0,
            "the metric must be released after its delayed registration is gone"
        );
    }

    #[test]
    fn execute_untap_step_with_optional_choice_can_untap_when_chosen() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let artifact = create_artifact(
            &mut game,
            "Courier Relic",
            alice,
            vec![StaticAbility::may_choose_not_to_untap_during_untap_step(
                "this artifact",
            )],
        );
        game.tap(artifact);

        let mut dm = AlwaysYesDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        assert!(
            !game.is_tapped(artifact),
            "artifact should untap when controller chooses to untap"
        );
    }

    #[test]
    fn execute_untap_step_with_optional_choice_can_stay_tapped_when_declined() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let artifact = create_artifact(
            &mut game,
            "Courier Relic",
            alice,
            vec![StaticAbility::may_choose_not_to_untap_during_untap_step(
                "this artifact",
            )],
        );
        game.tap(artifact);

        let mut dm = AlwaysNoDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        assert!(
            game.is_tapped(artifact),
            "artifact should stay tapped when controller declines untap"
        );
    }

    #[test]
    fn execute_untap_step_with_optional_choice_respects_doesnt_untap_and_restrictions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let doesnt_untap_artifact = create_artifact(
            &mut game,
            "Locked Relic",
            alice,
            vec![
                StaticAbility::may_choose_not_to_untap_during_untap_step("this artifact"),
                StaticAbility::doesnt_untap(),
            ],
        );
        let cant_untap_artifact = create_artifact(
            &mut game,
            "Frozen Relic",
            alice,
            vec![StaticAbility::may_choose_not_to_untap_during_untap_step(
                "this artifact",
            )],
        );
        game.tap(doesnt_untap_artifact);
        game.tap(cant_untap_artifact);
        game.add_restriction_effect(
            Restriction::untap(ObjectFilter::specific(cant_untap_artifact)),
            Until::Forever,
            cant_untap_artifact,
            alice,
            None,
        );

        let mut dm = AlwaysYesDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        assert!(
            game.is_tapped(doesnt_untap_artifact),
            "doesn't-untap static ability should prevent untapping"
        );
        assert!(
            game.is_tapped(cant_untap_artifact),
            "can't-untap restriction should prevent untapping"
        );
    }

    #[test]
    fn execute_untap_step_with_other_players_untap_support_untaps_matching_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        let _seedborn_like = create_artifact(
            &mut game,
            "Seedborn Relic",
            alice,
            vec![StaticAbility::untap_during_each_other_players_untap_step(
                crate::target::ObjectFilter::permanent().you_control(),
                "Untap all permanents you control during each other player's untap step"
                    .to_string(),
            )],
        );
        let alices_artifact = create_artifact(&mut game, "Alice Relic", alice, vec![]);
        game.tap(alices_artifact);

        let mut dm = AlwaysYesDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        assert!(
            !game.is_tapped(alices_artifact),
            "matching permanent should untap during another player's untap step"
        );
    }

    #[test]
    fn off_turn_untap_does_not_end_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        let _seedborn_like = create_artifact(
            &mut game,
            "Seedborn Relic",
            alice,
            vec![StaticAbility::untap_during_each_other_players_untap_step(
                crate::target::ObjectFilter::permanent().you_control(),
                "Untap all permanents you control during each other player's untap step"
                    .to_string(),
            )],
        );
        let creature = create_creature(&mut game, "New Recruit", alice);
        game.tap(creature);
        game.set_summoning_sick(creature);

        let mut dm = AlwaysYesDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        assert!(!game.is_tapped(creature));
        assert!(
            game.is_summoning_sick(creature),
            "another player's untap step does not establish continuous control since Alice's turn began"
        );
    }

    #[test]
    fn untap_step_performs_the_simultaneous_phasing_exchange() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let phases_out = create_artifact(
            &mut game,
            "Shimmering Relic",
            alice,
            vec![StaticAbility::phasing()],
        );
        let phases_in = create_artifact(&mut game, "Returning Relic", alice, vec![]);
        game.phase_out(phases_in);

        let mut dm = AlwaysYesDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        assert!(game.is_phased_out(phases_out));
        assert!(!game.is_phased_out(phases_in));
    }

    #[test]
    fn phased_out_permanents_are_absent_from_filters_and_controlled_counts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let visible = create_creature(&mut game, "Visible Bear", alice);
        let absent = create_creature(&mut game, "Vanishing Bear", alice);
        let anthem = create_artifact(
            &mut game,
            "Visible Anthem",
            alice,
            vec![StaticAbility::anthem(
                ObjectFilter::creature().you_control(),
                1,
                1,
            )],
        );
        game.refresh_continuous_state();
        assert_eq!(game.current_power(visible), Some(3));
        game.phase_out(anthem);
        game.refresh_continuous_state();
        assert_eq!(game.current_power(visible), Some(2));
        game.phase_out(absent);

        assert_eq!(game.permanents_controlled_by(alice), vec![visible]);
        assert!(game.current_characteristics(absent).is_none());

        let ctx = crate::effects::ExecutionContext::new_default(absent, alice);
        let resolved = crate::effects::helpers::resolve_objects_from_spec(
            &game,
            &crate::target::ChooseSpec::all(ObjectFilter::creature()),
            &ctx,
        )
        .expect("all creatures resolves");
        assert_eq!(resolved, vec![visible]);
    }

    #[test]
    fn phasing_removes_combatants_and_carries_attached_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Blinking Attacker", alice);
        let attachment = create_artifact(&mut game, "Attached Relic", alice, vec![]);
        assert!(game.attach_object_to_target(
            attachment,
            crate::object::AttachmentTarget::Object(attacker)
        ));
        game.combat = Some(crate::combat_state::CombatState {
            attackers: vec![crate::combat_state::AttackerInfo {
                creature: attacker,
                target: crate::combat_state::AttackTarget::Player(bob),
            }],
            ..Default::default()
        });

        game.phase_out(attacker);

        assert!(game.is_phased_out(attacker));
        assert!(game.is_phased_out(attachment));
        assert!(
            game.combat
                .as_ref()
                .is_some_and(|combat| combat.attackers.is_empty())
        );

        game.phase_in(attacker);
        assert!(!game.is_phased_out(attacker));
        assert!(!game.is_phased_out(attachment));
        assert_eq!(
            game.object(attachment)
                .and_then(|object| object.attached_to),
            Some(crate::object::AttachmentTarget::Object(attacker))
        );
    }

    #[test]
    fn execute_untap_step_with_other_players_untap_support_ignores_controller_only_untap_limits() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        let _seedborn_like = create_artifact(
            &mut game,
            "Seedborn Relic",
            alice,
            vec![StaticAbility::untap_during_each_other_players_untap_step(
                crate::target::ObjectFilter::permanent().you_control(),
                "Untap all permanents you control during each other player's untap step"
                    .to_string(),
            )],
        );
        let doesnt_untap_artifact = create_artifact(
            &mut game,
            "Locked Relic",
            alice,
            vec![StaticAbility::doesnt_untap()],
        );
        let cant_untap_artifact = create_artifact(&mut game, "Frozen Relic", alice, vec![]);
        game.tap(doesnt_untap_artifact);
        game.tap(cant_untap_artifact);
        game.effect_store
            .cant_effects
            .add_cant_untap(cant_untap_artifact);

        let mut dm = AlwaysYesDecisionMaker;
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        assert!(
            !game.is_tapped(doesnt_untap_artifact),
            "controller-only doesnt-untap ability should not block another player's untap step"
        );
        assert!(
            !game.is_tapped(cant_untap_artifact),
            "controller-only cant-untap restriction should not block another player's untap step"
        );
    }

    #[test]
    fn delayed_as_untap_instruction_resolves_before_permanents_untap_without_using_stack() {
        use crate::effects::EffectExecutor as _;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let land_card = CardBuilder::new(CardId::new(), "Delayed Paradise")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&land_card, alice, Zone::Battlefield);
        let stable_id = game.object(land).expect("land exists").stable_id;
        let companion = create_artifact(&mut game, "Companion Relic", alice, vec![]);
        game.tap(land);
        game.tap(companion);

        let schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
            crate::triggers::Trigger::as_permanents_untap(PlayerFilter::You, true),
            vec![crate::Effect::new(
                crate::effects::ReturnToHandEffect::with_spec(crate::target::ChooseSpec::Source),
            )],
            true,
            Vec::new(),
            PlayerFilter::You,
        )
        .watch_ability_source();
        let mut ctx = crate::effects::ExecutionContext::new_default(land, alice);
        schedule
            .execute(&mut game, &mut ctx)
            .expect("schedule the delayed untap instruction");
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);

        let mut dm = AlwaysYesDecisionMaker;
        game.turn.active_player = bob;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        execute_untap_step_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.object(land).map(|object| object.zone),
            Some(Zone::Battlefield),
            "another player's untap step must not consume the instruction"
        );
        assert!(game.is_tapped(land));
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);

        game.turn.active_player = alice;
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        let returned = game
            .find_object_by_stable_id(stable_id)
            .expect("the source should keep its stable identity in hand");
        assert_eq!(
            game.object(returned).map(|object| object.zone),
            Some(Zone::Hand)
        );
        assert!(
            game.effect_store
                .pending_trigger_events
                .iter()
                .any(|event| {
                    event.kind() == crate::events::EventKind::ZoneChange
                        && event.snapshot().is_some_and(|snapshot| {
                            snapshot.stable_id == stable_id && snapshot.tapped
                        })
                }),
            "the land's leave-the-battlefield snapshot must still be tapped"
        );
        assert!(
            !game.is_tapped(companion),
            "the instruction must resolve before the remaining permanents untap"
        );
        assert!(game.effect_store.delayed_triggers.is_empty());
        assert!(
            game.stack.is_empty(),
            "an as-you-untap instruction is a turn-based action, not a triggered ability"
        );
    }

    #[test]
    fn execute_untap_step_consumes_controllers_next_untap_restriction_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let relic = create_artifact(&mut game, "Exerted Relic", alice, vec![]);
        game.tap(relic);
        game.add_restriction_effect(
            Restriction::untap(ObjectFilter::specific(relic)),
            Until::ControllersNextUntapStep,
            relic,
            alice,
            None,
        );
        game.update_cant_effects();

        let mut dm = AlwaysYesDecisionMaker;

        game.turn.active_player = bob;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        execute_untap_step_with(&mut game, &mut dm).unwrap();
        assert!(
            game.is_tapped(relic),
            "another player's untap step should not untap Alice's tapped artifact"
        );

        game.next_turn();
        execute_untap_step_with(&mut game, &mut dm).unwrap();
        assert!(
            game.is_tapped(relic),
            "controller's next untap step should keep the artifact tapped once"
        );

        game.next_turn();
        execute_untap_step_with(&mut game, &mut dm).unwrap();

        game.next_turn();
        execute_untap_step_with(&mut game, &mut dm).unwrap();
        assert!(
            !game.is_tapped(relic),
            "the restriction should be consumed after that untap step"
        );
    }

    #[test]
    fn execute_draw_step_with_can_move_drawn_commander_to_command_zone() {
        let mut game = setup_game();
        // Not the starting player's first turn, which skips the draw (CR 103.8a).
        game.turn.turn_number = 2;
        let alice = PlayerId::from_index(0);
        let commander = CardBuilder::new(CardId::from_raw(9000), "Topdeck Commander")
            .card_types(vec![CardType::Creature])
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Library);
        game.set_as_commander(commander_id, alice);

        let mut dm = AlwaysYesDecisionMaker;
        let events = execute_draw_step_with(&mut game, &mut dm).unwrap();

        assert!(
            events.is_empty(),
            "redirected commander should not count as a draw"
        );
        assert!(
            game.player(alice)
                .expect("alice should exist")
                .hand
                .is_empty()
        );
        assert_eq!(game.objects_in_zone(Zone::Command).len(), 1);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
    }

    #[test]
    fn execute_draw_step_with_can_leave_commander_in_hand() {
        let mut game = setup_game();
        // Not the starting player's first turn, which skips the draw (CR 103.8a).
        game.turn.turn_number = 2;
        let alice = PlayerId::from_index(0);
        let commander = CardBuilder::new(CardId::from_raw(9001), "Honest Commander")
            .card_types(vec![CardType::Creature])
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Library);
        game.set_as_commander(commander_id, alice);

        let mut dm = AlwaysNoDecisionMaker;
        let events = execute_draw_step_with(&mut game, &mut dm).unwrap();

        assert_eq!(
            events.len(),
            1,
            "keeping the commander should produce a draw event"
        );
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            1
        );
        assert!(game.objects_in_zone(Zone::Command).is_empty());
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
    }

    #[test]
    fn active_skip_draw_static_rule_tracks_controller_and_source_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_artifact(
            &mut game,
            "Draw-Step Lock",
            alice,
            vec![StaticAbility::player_skips_draw_step(PlayerFilter::You)],
        );
        let alice_card = CardBuilder::new(CardId::from_raw(9100), "Alice Draw")
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&alice_card, alice, Zone::Library);
        game.turn.turn_number = 2;
        game.turn.active_player = alice;

        let mut dm = AlwaysNoDecisionMaker;
        assert!(execute_draw_step_with(&mut game, &mut dm).unwrap().is_empty());
        assert!(game.player(alice).expect("Alice exists").hand.is_empty());

        game.set_current_controller(source, bob)
            .expect("finite controller fixture must refresh successfully");
        assert!(!game.player_skips_draw_step(alice));
        assert!(game.player_skips_draw_step(bob));
        assert_eq!(execute_draw_step_with(&mut game, &mut dm).unwrap().len(), 1);
        assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 1);

        let bob_card = CardBuilder::new(CardId::from_raw(9101), "Bob Draw")
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&bob_card, bob, Zone::Library);
        game.turn.active_player = bob;
        assert!(execute_draw_step_with(&mut game, &mut dm).unwrap().is_empty());
        assert!(game.player(bob).expect("Bob exists").hand.is_empty());

        game.move_object_by_effect(source, Zone::Graveyard);
        assert!(!game.player_skips_draw_step(bob));
        assert_eq!(execute_draw_step_with(&mut game, &mut dm).unwrap().len(), 1);
        assert_eq!(game.player(bob).expect("Bob exists").hand.len(), 1);
    }

    #[test]
    fn execute_draw_step_skips_starting_players_first_draw_in_non_commander_game() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(9002), "First Turn Draw")
            .card_types(vec![CardType::Creature])
            .build();
        let _card_id = game.create_object_from_card(&card, alice, Zone::Library);

        let mut dm = AlwaysNoDecisionMaker;
        let events = execute_draw_step_with(&mut game, &mut dm).unwrap();

        assert!(
            events.is_empty(),
            "normal games should skip the opening draw"
        );
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            0
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
    }

    #[test]
    fn execute_draw_step_skips_starting_players_first_draw_in_two_player_commander_game() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let commander = CardBuilder::new(CardId::from_raw(9003), "Opening Commander")
            .card_types(vec![CardType::Creature])
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Command);
        game.set_as_commander(commander_id, alice);

        let card = CardBuilder::new(CardId::from_raw(9004), "Commander First Turn Draw")
            .card_types(vec![CardType::Creature])
            .build();
        let _card_id = game.create_object_from_card(&card, alice, Zone::Library);

        let mut dm = AlwaysNoDecisionMaker;
        let events = execute_draw_step_with(&mut game, &mut dm).unwrap();

        // CR 103.8a: a two-player Commander game is a two-player game, so the
        // starting player skips their first draw.
        assert!(
            events.is_empty(),
            "two-player commander games skip the starting player's first draw"
        );
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            0
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
    }

    #[test]
    fn execute_draw_step_keeps_starting_players_first_draw_in_normal_multiplayer_game() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(9005), "Multiplayer First Turn Draw")
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);

        let mut dm = AlwaysNoDecisionMaker;
        let events = execute_draw_step_with(&mut game, &mut dm).unwrap();

        assert_eq!(
            events.len(),
            1,
            "CR 103.8c preserves the multiplayer opening draw"
        );
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            1
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
    }
}

#[cfg(test)]
mod draw_step_ordinal_tests {
    use super::*;
    use crate::events::other::CardsDrawnEvent;
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;
    fn setup() -> GameState {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
        game.turn.turn_number = 3;
        game.turn.active_player = PlayerId(0);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        let card = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Draw resource").build();
        for player in [PlayerId(0), PlayerId(1)] {
            for _ in 0..5 {
                game.create_object_from_definition(&card, player, Zone::Library);
            }
        }
        game
    }
    #[test]
    fn public_draw_adapters_return_missing_history_without_a_manual_meter() {
        for explicit_decision_maker in [false, true] {
            let mut game = setup();
            let player = game.turn.active_player;
            let library = game.player(player).unwrap().library.to_vec();
            let ids = game.next_object_id_counter();
            let step = game.draw_step_context_for_player(player);
            game.turn_store.turn_history.draw_occurrences = None;
            assert!(game.token_resource_failure().is_none());
            let result = if explicit_decision_maker {
                execute_draw_step_with(&mut game, &mut crate::decision::AutoPassDecisionMaker)
            } else {
                execute_draw_step(&mut game)
            };
            assert!(matches!(result, Err(crate::effects::ExecutionError::IncompleteEvidence(_))));
            assert_eq!(game.player(player).unwrap().library.as_slice(), library.as_slice());
            assert!(game.player(player).unwrap().hand.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.draw_step_context_for_player(player), step);
            assert!(game.turn_store.turn_history.draw_occurrences.is_none());
            assert!(game.token_resource_failure().is_none(), "the Result owns the failure without a meter");

            game.next_turn();
            game.turn.phase = Phase::Beginning;
            game.turn.step = Some(Step::Draw);
            let player = game.turn.active_player;
            let completed = execute_draw_step(&mut game).unwrap();
            let draw = completed.iter().find_map(|event| event.downcast::<CardsDrawnEvent>()).unwrap();
            assert_eq!(draw.player, player);
            assert!(draw.is_first_this_turn);
            assert_eq!(game.player(player).unwrap().hand.len(), 1);
        }
    }

    #[test]
    fn turn_draw_retains_ordinal_through_priority_and_resets_for_adjacent_extra_step() {
        let mut game = setup();
        let events = execute_draw_step(&mut game).unwrap();
        let draw = events
            .iter()
            .find_map(|event| event.downcast::<CardsDrawnEvent>())
            .unwrap();
        assert_eq!(draw.cards_previously_drawn_this_draw_step, 0);
        assert_eq!(game.draw_step_context_for_player(PlayerId(0)), (true, 1));
        game.record_cards_drawn_in_current_draw_step(PlayerId(0), 2);
        assert_eq!(game.draw_step_context_for_player(PlayerId(0)), (true, 3));
        game.add_step_after(Step::Draw, Step::Draw);
        advance_step(&mut game).unwrap();
        assert_eq!(game.turn.step, Some(Step::Draw));
        assert_eq!(game.draw_step_context_for_player(PlayerId(0)), (true, 0));
        execute_draw_step(&mut game).unwrap();
        assert_eq!(game.draw_step_context_for_player(PlayerId(0)), (true, 1));
        advance_step(&mut game).unwrap();
        assert_eq!(game.draw_step_context_for_player(PlayerId(0)), (false, 0));
    }
    #[test]
    fn shared_turn_partners_keep_independent_draw_step_ordinals() {
        let mut game = setup();
        game.restore_two_headed_giant(
            vec![
                vec![PlayerId(0), PlayerId(1)],
                vec![PlayerId(2), PlayerId(3)],
            ],
            0,
            PlayerId(1),
        )
        .unwrap();
        execute_draw_step(&mut game).unwrap();
        for player in [PlayerId(0), PlayerId(1)] {
            assert_eq!(game.draw_step_context_for_player(player), (true, 1));
        }
        game.record_cards_drawn_in_current_draw_step(PlayerId(0), 2);
        game.record_cards_drawn_in_current_draw_step(PlayerId(1), 1);
        assert_eq!(game.draw_step_context_for_player(PlayerId(0)), (true, 3));
        assert_eq!(game.draw_step_context_for_player(PlayerId(1)), (true, 2));
        game.record_cards_drawn_in_current_draw_step(PlayerId(2), 7);
        assert_eq!(game.draw_step_context_for_player(PlayerId(2)), (false, 0));
    }

    #[test]
    fn ending_the_turn_clears_draw_ordinals_at_the_cleanup_jump() {
        use crate::effects::EffectExecutor;
        let mut game = setup();
        execute_draw_step(&mut game).unwrap();
        let source = game.new_object_id();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = crate::effects::EffectContext::new(source, PlayerId(0), &mut dm);
        crate::effects::EndTurnEffect::new(crate::target::PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .unwrap();
        let mut runner =
            crate::TurnRunner::from_state_for_sync(crate::TurnRunnerState::DrawPriority);
        let mut queue = crate::triggers::TriggerQueue::new();
        for _ in 0..10 {
            if game.turn.step == Some(Step::Cleanup) {
                break;
            }
            runner.advance(&mut game, &mut queue).unwrap();
        }
        assert_eq!(game.turn.step, Some(Step::Cleanup));
        assert!(game.turn_store.cards_drawn_this_draw_step.is_empty());
    }
}
