use super::*;
use crate::perf::PerfTimer;
#[cfg(feature = "serialization")]
use serde::Serialize;
use std::cell::RefCell;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize))]
pub struct PriorityAdvancePerfMetrics {
    pub replacement_choice_ms: f64,
    pub state_based_actions_ms: f64,
    pub put_triggers_ms: f64,
    pub game_over_check_ms: f64,
    pub compute_legal_actions_ms: f64,
    pub compute_legal_actions_detail: Option<crate::decision::ComputeLegalActionsPerfMetrics>,
    pub compute_commander_actions_ms: f64,
    pub total_ms: f64,
    pub result_kind: String,
    pub priority_player: Option<u8>,
    pub action_count: usize,
    pub commander_action_count: usize,
}

thread_local! {
    static LAST_PRIORITY_ADVANCE_PERF: RefCell<Option<PriorityAdvancePerfMetrics>> = const { RefCell::new(None) };
}

fn store_priority_advance_perf(metrics: PriorityAdvancePerfMetrics) {
    LAST_PRIORITY_ADVANCE_PERF.with(|slot| {
        *slot.borrow_mut() = Some(metrics);
    });
}

pub fn last_priority_advance_perf() -> Option<PriorityAdvancePerfMetrics> {
    LAST_PRIORITY_ADVANCE_PERF.with(|slot| slot.borrow().clone())
}

fn terminal_progress_or_resume_subgame(
    game: &mut GameState,
    result: GameResult,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if game.is_subgame() {
        game.finish_subgame_with(result, decision_maker)
            .map_err(|error| GameLoopError::ResolutionFailed(error.to_string()))?;
        Ok(GameProgress::StackResolved)
    } else {
        Ok(GameProgress::GameOver(result))
    }
}

pub(super) fn finish_mandatory_loop_draw(
    game: &mut GameState,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if !game.mandatory_loop_draw_pending() {
        game.mark_mandatory_loop_draw();
    }
    if game.resolve_mandatory_loop_draw() {
        terminal_progress_or_resume_subgame(game, GameResult::Draw, decision_maker)
    } else {
        Ok(GameProgress::StackResolved)
    }
}

///
/// This is the main entry point for the decision-based game loop.
/// Call this repeatedly, handling decisions as they come, until
/// it returns `GameProgress::Continue` (phase ends) or `GameProgress::GameOver`.
pub fn advance_priority(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<GameProgress, GameLoopError> {
    let mut dm = crate::decision::AutoPassDecisionMaker;
    advance_priority_with_dm(game, trigger_queue, &mut dm)
}

/// Advance priority with a decision maker for triggered ability targeting.
///
/// This version allows proper target selection for triggered abilities.
pub fn advance_priority_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let total_started_at = PerfTimer::start();
    let mut perf = PriorityAdvancePerfMetrics::default();
    if game.subgame_starting_procedure_pending() {
        return Err(GameLoopError::InvalidState(
            "the subgame starting procedure must finish before priority can advance".to_string(),
        ));
    }
    if game.mandatory_loop_draw_pending() {
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "mandatory_loop_draw".to_string();
        store_priority_advance_perf(perf);
        return finish_mandatory_loop_draw(game, decision_maker);
    }
    // Check for pending replacement effect choice first
    // This takes priority over normal game flow
    let replacement_started_at = PerfTimer::start();
    if let Some(pending) = &game.effect_store.pending_replacement_choice {
        let options: Vec<ReplacementOption> = pending
            .applicable_effects
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                game.effect_store
                    .replacement_effects
                    .get_effect(*id)
                    .map(|e| ReplacementOption {
                        index: i,
                        source: e.source,
                        description: crate::decisions::specs::replacement_option_description(
                            game, e.source,
                        ),
                    })
            })
            .collect();

        // Convert to SelectOptionsContext for replacement effect choice
        let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
            .iter()
            .map(|opt| {
                crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
                    .with_object(opt.source)
            })
            .collect();
        let ctx = crate::decisions::context::SelectOptionsContext::new(
            pending.player,
            None,
            "Choose replacement effect to apply",
            selectable_options,
            1,
            1,
        );
        perf.replacement_choice_ms = replacement_started_at.elapsed_ms();
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "pending_replacement_choice".to_string();
        store_priority_advance_perf(perf);
        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ));
    }
    perf.replacement_choice_ms = replacement_started_at.elapsed_ms();

    // Check and apply state-based actions
    let sba_started_at = PerfTimer::start();
    if let Err(error) = check_and_apply_sbas_with(game, trigger_queue, decision_maker) {
        if matches!(error, GameLoopError::MandatoryLoopDraw) {
            perf.state_based_actions_ms = sba_started_at.elapsed_ms();
            perf.total_ms = total_started_at.elapsed_ms();
            perf.result_kind = "mandatory_loop_draw".to_string();
            store_priority_advance_perf(perf);
            return finish_mandatory_loop_draw(game, decision_maker);
        }
        return Err(error);
    }
    perf.state_based_actions_ms = sba_started_at.elapsed_ms();

    // Put triggered abilities on the stack with target selection
    let triggers_started_at = PerfTimer::start();
    if let Err(error) = put_triggers_on_stack_with_dm(game, trigger_queue, decision_maker) {
        if matches!(error, GameLoopError::MandatoryLoopDraw) {
            perf.put_triggers_ms = triggers_started_at.elapsed_ms();
            perf.total_ms = total_started_at.elapsed_ms();
            perf.result_kind = "mandatory_loop_draw".to_string();
            store_priority_advance_perf(perf);
            return finish_mandatory_loop_draw(game, decision_maker);
        }
        return Err(error);
    }
    perf.put_triggers_ms = triggers_started_at.elapsed_ms();

    // Check if game is over
    let game_over_started_at = PerfTimer::start();
    let remaining: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.is_in_game())
        .map(|p| p.id)
        .collect();

    if remaining.is_empty() {
        perf.game_over_check_ms = game_over_started_at.elapsed_ms();
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "game_over_draw".to_string();
        store_priority_advance_perf(perf);
        return terminal_progress_or_resume_subgame(game, GameResult::Draw, decision_maker);
    }
    if let Some(winners) = game.sole_surviving_team_winners() {
        perf.game_over_check_ms = game_over_started_at.elapsed_ms();
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "game_over_team".to_string();
        store_priority_advance_perf(perf);
        for winner in &winners {
            game.mark_team_winner(*winner);
        }
        let result = if winners.len() == 1 {
            GameResult::Winner(winners[0])
        } else {
            GameResult::Remaining(winners)
        };
        return terminal_progress_or_resume_subgame(game, result, decision_maker);
    }
    if remaining.len() == 1 {
        perf.game_over_check_ms = game_over_started_at.elapsed_ms();
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "game_over_winner".to_string();
        store_priority_advance_perf(perf);
        let winner = remaining[0];
        if game.is_subgame() {
            return terminal_progress_or_resume_subgame(
                game,
                GameResult::Winner(winner),
                decision_maker,
            );
        }
        if let Some(player) = game.player_mut(winner) {
            player.has_won = true;
        }
        game.finalize_ante_ownership(winner);
        return Ok(GameProgress::GameOver(GameResult::Winner(winner)));
    }
    perf.game_over_check_ms = game_over_started_at.elapsed_ms();

    // Get current priority player
    let Some(priority_player) = game.turn.priority_player else {
        // No one has priority, phase should end
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "continue_no_priority_player".to_string();
        store_priority_advance_perf(perf);
        return Ok(GameProgress::Continue);
    };
    perf.priority_player = Some(priority_player.index() as u8);

    // A CR 805 priority decision exposes actions for every member of the team.
    // The primary player remains the prompt recipient/fallback decision maker.
    let legal_actions_started_at = PerfTimer::start();
    let priority_players = game.priority_team_players();
    let mut actions = Vec::new();
    for player in priority_players.iter().copied() {
        for action in compute_legal_actions(game, player) {
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
    }
    perf.compute_legal_actions_ms = legal_actions_started_at.elapsed_ms();
    perf.compute_legal_actions_detail = crate::decision::last_compute_legal_actions_perf();
    let commander_actions_started_at = PerfTimer::start();
    let mut commander_actions = Vec::new();
    for player in priority_players {
        for action in compute_commander_actions(game, player) {
            if !commander_actions.contains(&action) && !actions.contains(&action) {
                commander_actions.push(action);
            }
        }
    }
    perf.compute_commander_actions_ms = commander_actions_started_at.elapsed_ms();
    perf.action_count = actions.len();
    perf.commander_action_count = commander_actions.len();
    actions.extend(commander_actions);

    // Return decision for the player using the new context-based system
    let ctx = crate::decisions::context::PriorityContext::new(priority_player, actions);
    perf.total_ms = total_started_at.elapsed_ms();
    perf.result_kind = "needs_priority_decision".to_string();
    store_priority_advance_perf(perf);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::Priority(ctx),
    ))
}

pub(super) fn priority_actor_for_action(
    game: &GameState,
    action: &LegalAction,
) -> Option<PlayerId> {
    if matches!(action, LegalAction::PassPriority) {
        return game.turn.priority_player;
    }
    game.priority_team_players().into_iter().find(|player| {
        compute_legal_actions(game, *player).contains(action)
            || compute_commander_actions(game, *player).contains(action)
    })
}

/// Apply a player's response to a decision during the priority loop.
///
/// This handles both `PriorityAction` responses (for normal priority decisions)
/// and `Targets` responses (when a spell is being cast and needs targets).
pub fn apply_priority_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &PriorityResponse,
) -> Result<GameProgress, GameLoopError> {
    let mut auto_dm = crate::decision::CliDecisionMaker;
    apply_priority_response_with_dm(game, trigger_queue, state, response, &mut auto_dm)
}
