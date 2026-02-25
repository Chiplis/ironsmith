//! Unified turn state machine that both CLI and WASM frontends drive.
//!
//! The [`TurnRunner`] sequences an entire MTG turn as a state machine,
//! yielding at decision points and priority windows so that callers can
//! provide player input (sync or async) and re-enter.

use crate::combat_state::CombatState;
use crate::decision::{AttackerDeclaration, BlockerDeclaration, GameResult};
use crate::decisions::context::DecisionContext;
use crate::game_loop::{
    apply_attacker_declarations, apply_blocker_declarations, check_and_apply_sbas,
    execute_combat_damage_step, generate_and_queue_step_triggers, get_declare_attackers_decision,
    get_declare_blockers_decision, put_triggers_on_stack, queue_combat_damage_triggers,
    GameLoopError,
};
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId};
use crate::rules::combat::deals_first_strike_damage_with_game;
use crate::rules::state_based::check_state_based_actions;
use crate::triggers::TriggerQueue;
use crate::turn::{execute_cleanup_step, execute_draw_step, execute_untap_step};

/// What the caller should do next after calling [`TurnRunner::advance`].
#[derive(Debug)]
pub enum TurnAction {
    /// Internal work done; call `advance()` again immediately.
    Continue,
    /// A player decision is needed. Inspect the context, collect the answer,
    /// call the appropriate `respond_*()` method, then `advance()` again.
    Decision(DecisionContext),
    /// Run the priority loop (SBAs, triggers, player actions).
    /// When the priority loop finishes, call `priority_done()` then `advance()`.
    RunPriority,
    /// The turn has ended.
    TurnComplete,
    /// The game is over.
    GameOver(GameResult),
}

/// Internal state of the turn state machine.
#[derive(Debug, Clone)]
pub enum TurnState {
    // === Beginning Phase ===
    BeginTurn,
    Upkeep,
    UpkeepPriority,
    Draw,
    DrawPriority,

    // === First Main Phase ===
    FirstMain,
    FirstMainPriority,

    // === Combat Phase ===
    BeginCombat,
    BeginCombatPriority,
    DeclareAttackersDecision,
    DeclareAttackersApply,
    DeclareAttackersPriority,
    DeclareBlockersCheck,
    DeclareBlockersDecision,
    DeclareBlockersApply,
    DeclareBlockersPriority,
    CombatDamageFirstStrike,
    CombatDamageFirstStrikePriority,
    CombatDamageRegular,
    CombatDamageRegularPriority,
    EndCombat,
    EndCombatPriority,

    // === Second Main Phase ===
    NextMain,
    NextMainPriority,

    // === Ending Phase ===
    EndStep,
    EndStepPriority,
    CleanupDiscard,
    CleanupApply,
    CleanupRecursiveCheck,
    CleanupRecursivePriority,
    CleanupRecursiveDiscard,

    // === Terminal ===
    Complete,
}

/// Drives a single turn as a state machine.
#[derive(Debug, Clone)]
pub struct TurnRunner {
    state: TurnState,
    /// Combat state owned by the runner for the duration of combat.
    combat: CombatState,
    /// Whether first-strike creatures were detected this combat.
    has_first_strike: bool,
    /// Pending attacker declarations from the caller.
    pending_attackers: Option<Vec<AttackerDeclaration>>,
    /// Pending blocker declarations from the caller.
    pending_blockers: Option<(Vec<BlockerDeclaration>, PlayerId)>,
    /// Pending discard selection from the caller.
    pending_discard: Option<Vec<ObjectId>>,
    /// Defending player for the current combat.
    defending_player: Option<PlayerId>,
}

impl TurnRunner {
    /// Create a new TurnRunner starting at the beginning of a turn.
    pub fn new() -> Self {
        Self {
            state: TurnState::BeginTurn,
            combat: CombatState::default(),
            has_first_strike: false,
            pending_attackers: None,
            pending_blockers: None,
            pending_discard: None,
            defending_player: None,
        }
    }

    /// Return a reference to the current state (for checkpoint/debug).
    pub fn state(&self) -> &TurnState {
        &self.state
    }

    /// Return a reference to the combat state.
    pub fn combat(&self) -> &CombatState {
        &self.combat
    }

    /// Return a mutable reference to the combat state.
    pub fn combat_mut(&mut self) -> &mut CombatState {
        &mut self.combat
    }

    /// Advance the state machine one step.
    ///
    /// Returns a [`TurnAction`] telling the caller what to do next.
    /// The caller should loop calling `advance()` until it gets
    /// `TurnComplete` or `GameOver`.
    pub fn advance(
        &mut self,
        game: &mut GameState,
        tq: &mut TriggerQueue,
    ) -> Result<TurnAction, GameLoopError> {
        match self.state {
            // ================================================================
            // Beginning Phase
            // ================================================================
            TurnState::BeginTurn => {
                game.activate_pending_player_control(game.turn.active_player);

                // Untap step — no priority
                game.turn.phase = Phase::Beginning;
                game.turn.step = Some(Step::Untap);
                execute_untap_step(game);

                self.state = TurnState::Upkeep;
                Ok(TurnAction::Continue)
            }

            TurnState::Upkeep => {
                game.turn.step = Some(Step::Upkeep);
                game.turn.priority_player = Some(game.turn.active_player);
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::UpkeepPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::UpkeepPriority => {
                game.empty_mana_pools();
                self.state = TurnState::Draw;
                Ok(TurnAction::Continue)
            }

            TurnState::Draw => {
                game.turn.step = Some(Step::Draw);
                let draw_events = execute_draw_step(game);
                generate_and_queue_step_triggers(game, tq);

                // Queue triggers for each drawn card (Miracle, etc.)
                for draw_event in draw_events {
                    let triggered =
                        crate::triggers::check::check_triggers(game, &draw_event);
                    for entry in triggered {
                        tq.add(entry);
                    }
                }

                self.state = TurnState::DrawPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::DrawPriority => {
                game.empty_mana_pools();
                self.state = TurnState::FirstMain;
                Ok(TurnAction::Continue)
            }

            // ================================================================
            // First Main Phase
            // ================================================================
            TurnState::FirstMain => {
                game.turn.phase = Phase::FirstMain;
                game.turn.step = None;
                game.turn.priority_player = Some(game.turn.active_player);
                generate_and_queue_step_triggers(game, tq);
                crate::game_loop::add_saga_lore_counters(game, tq);

                self.state = TurnState::FirstMainPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::FirstMainPriority => {
                game.empty_mana_pools();
                self.state = TurnState::BeginCombat;
                Ok(TurnAction::Continue)
            }

            // ================================================================
            // Combat Phase
            // ================================================================
            TurnState::BeginCombat => {
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(Step::BeginCombat);
                game.turn.priority_player = Some(game.turn.active_player);
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::BeginCombatPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::BeginCombatPriority => {
                game.empty_mana_pools();
                self.state = TurnState::DeclareAttackersDecision;
                Ok(TurnAction::Continue)
            }

            TurnState::DeclareAttackersDecision => {
                game.turn.step = Some(Step::DeclareAttackers);
                game.turn.priority_player = Some(game.turn.active_player);

                let ctx = get_declare_attackers_decision(game, &self.combat);
                self.state = TurnState::DeclareAttackersApply;
                Ok(TurnAction::Decision(ctx))
            }

            TurnState::DeclareAttackersApply => {
                let declarations = self.pending_attackers.take().unwrap_or_default();
                apply_attacker_declarations(
                    game,
                    &mut self.combat,
                    tq,
                    &declarations,
                )?;
                put_triggers_on_stack(game, tq)?;

                // Also sync game.combat for anything that reads it
                game.combat = Some(self.combat.clone());

                self.state = TurnState::DeclareAttackersPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::DeclareAttackersPriority => {
                game.empty_mana_pools();
                self.state = TurnState::DeclareBlockersCheck;
                Ok(TurnAction::Continue)
            }

            TurnState::DeclareBlockersCheck => {
                if self.combat.attackers.is_empty() {
                    // Skip blockers and combat damage
                    self.state = TurnState::EndCombat;
                    Ok(TurnAction::Continue)
                } else {
                    self.state = TurnState::DeclareBlockersDecision;
                    Ok(TurnAction::Continue)
                }
            }

            TurnState::DeclareBlockersDecision => {
                game.turn.step = Some(Step::DeclareBlockers);

                let defending_player = game
                    .players
                    .iter()
                    .find(|p| p.id != game.turn.active_player && p.is_in_game())
                    .map(|p| p.id)
                    .unwrap_or(game.turn.active_player);
                self.defending_player = Some(defending_player);

                game.turn.priority_player = Some(defending_player);

                let ctx =
                    get_declare_blockers_decision(game, &self.combat, defending_player);
                self.state = TurnState::DeclareBlockersApply;
                Ok(TurnAction::Decision(ctx))
            }

            TurnState::DeclareBlockersApply => {
                let (declarations, defending_player) =
                    self.pending_blockers.take().unwrap_or_else(|| {
                        (Vec::new(), self.defending_player.unwrap_or(game.turn.active_player))
                    });
                apply_blocker_declarations(
                    game,
                    &mut self.combat,
                    tq,
                    &declarations,
                    defending_player,
                )?;
                put_triggers_on_stack(game, tq)?;

                // Sync game.combat
                game.combat = Some(self.combat.clone());

                self.state = TurnState::DeclareBlockersPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::DeclareBlockersPriority => {
                game.empty_mana_pools();

                // Check for first strike
                self.has_first_strike = check_first_strike(game, &self.combat);

                if self.has_first_strike {
                    self.state = TurnState::CombatDamageFirstStrike;
                } else {
                    self.state = TurnState::CombatDamageRegular;
                }
                Ok(TurnAction::Continue)
            }

            TurnState::CombatDamageFirstStrike => {
                game.turn.step = Some(Step::CombatDamage);

                let events = execute_combat_damage_step(game, &self.combat, true);
                queue_combat_damage_triggers(game, &events, tq);
                check_and_apply_sbas(game, tq)?;

                self.state = TurnState::CombatDamageFirstStrikePriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::CombatDamageFirstStrikePriority => {
                game.empty_mana_pools();
                self.state = TurnState::CombatDamageRegular;
                Ok(TurnAction::Continue)
            }

            TurnState::CombatDamageRegular => {
                game.turn.step = Some(Step::CombatDamage);

                let events = execute_combat_damage_step(game, &self.combat, false);
                queue_combat_damage_triggers(game, &events, tq);
                check_and_apply_sbas(game, tq)?;

                self.state = TurnState::CombatDamageRegularPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::CombatDamageRegularPriority => {
                game.empty_mana_pools();
                self.state = TurnState::EndCombat;
                Ok(TurnAction::Continue)
            }

            TurnState::EndCombat => {
                game.turn.step = Some(Step::EndCombat);
                game.turn.priority_player = Some(game.turn.active_player);
                generate_and_queue_step_triggers(game, tq);
                crate::combat_state::end_combat(&mut self.combat);
                game.combat = Some(self.combat.clone());

                self.state = TurnState::EndCombatPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::EndCombatPriority => {
                game.empty_mana_pools();
                self.state = TurnState::NextMain;
                Ok(TurnAction::Continue)
            }

            // ================================================================
            // Second Main Phase
            // ================================================================
            TurnState::NextMain => {
                game.turn.phase = Phase::NextMain;
                game.turn.step = None;
                game.turn.priority_player = Some(game.turn.active_player);
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::NextMainPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::NextMainPriority => {
                game.empty_mana_pools();
                self.state = TurnState::EndStep;
                Ok(TurnAction::Continue)
            }

            // ================================================================
            // Ending Phase
            // ================================================================
            TurnState::EndStep => {
                game.turn.phase = Phase::Ending;
                game.turn.step = Some(Step::End);
                game.turn.priority_player = Some(game.turn.active_player);
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::EndStepPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::EndStepPriority => {
                game.empty_mana_pools();
                self.state = TurnState::CleanupDiscard;
                Ok(TurnAction::Continue)
            }

            TurnState::CleanupDiscard => {
                game.turn.step = Some(Step::Cleanup);
                self.advance_cleanup_discard(game)
            }

            TurnState::CleanupApply => {
                execute_cleanup_step(game);
                self.state = TurnState::CleanupRecursiveCheck;
                Ok(TurnAction::Continue)
            }

            TurnState::CleanupRecursiveCheck => {
                let triggers_fired = !tq.is_empty();
                let sbas_happened = !check_state_based_actions(game).is_empty();

                if triggers_fired || sbas_happened {
                    check_and_apply_sbas(game, tq)?;
                    put_triggers_on_stack(game, tq)?;
                    if !game.stack_is_empty() {
                        self.state = TurnState::CleanupRecursivePriority;
                        Ok(TurnAction::RunPriority)
                    } else {
                        self.state = TurnState::CleanupRecursiveDiscard;
                        Ok(TurnAction::Continue)
                    }
                } else {
                    self.state = TurnState::Complete;
                    Ok(TurnAction::Continue)
                }
            }

            TurnState::CleanupRecursivePriority => {
                game.empty_mana_pools();
                self.state = TurnState::CleanupRecursiveDiscard;
                Ok(TurnAction::Continue)
            }

            TurnState::CleanupRecursiveDiscard => {
                self.advance_cleanup_discard_recursive(game)
            }

            TurnState::Complete => Ok(TurnAction::TurnComplete),
        }
    }

    /// Provide attacker declarations in response to a `Decision(Attackers(...))`.
    pub fn respond_attackers(
        &mut self,
        declarations: Vec<AttackerDeclaration>,
    ) {
        self.pending_attackers = Some(declarations);
    }

    /// Provide blocker declarations in response to a `Decision(Blockers(...))`.
    pub fn respond_blockers(
        &mut self,
        declarations: Vec<BlockerDeclaration>,
        defending_player: PlayerId,
    ) {
        self.pending_blockers = Some((declarations, defending_player));
    }

    /// Provide a discard selection in response to a `Decision(SelectObjects(...))`.
    pub fn respond_discard(&mut self, cards: Vec<ObjectId>) {
        self.pending_discard = Some(cards);
    }

    /// Signal that the priority loop has completed.
    pub fn priority_done(&mut self) {
        // This is a no-op on the runner itself; the state transition
        // happens in advance() when the *Priority state is re-entered.
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    /// Handle the first cleanup discard check.
    fn advance_cleanup_discard(
        &mut self,
        game: &mut GameState,
    ) -> Result<TurnAction, GameLoopError> {
        if let Some(discard) = self.pending_discard.take() {
            // Caller already provided a discard selection (from a prior Decision yield).
            // Apply it with an auto-pass DM for madness replacement.
            let mut auto_dm = crate::decision::AutoPassDecisionMaker;
            crate::turn::apply_cleanup_discard(game, &discard, &mut auto_dm);
            self.state = TurnState::CleanupApply;
            return Ok(TurnAction::Continue);
        }

        if let Some((player, spec)) = crate::turn::get_cleanup_discard_spec(game) {
            use crate::decisions::DecisionSpec;
            let ctx = spec.build_context(player, None, game);
            // Yield the discard decision to the caller
            self.state = TurnState::CleanupDiscard; // stay here until respond_discard
            return Ok(TurnAction::Decision(ctx));
        }

        // No discard needed
        self.state = TurnState::CleanupApply;
        Ok(TurnAction::Continue)
    }

    /// Handle recursive cleanup discard check.
    fn advance_cleanup_discard_recursive(
        &mut self,
        game: &mut GameState,
    ) -> Result<TurnAction, GameLoopError> {
        if let Some(discard) = self.pending_discard.take() {
            let mut auto_dm = crate::decision::AutoPassDecisionMaker;
            crate::turn::apply_cleanup_discard(game, &discard, &mut auto_dm);
            // Another cleanup step
            self.state = TurnState::CleanupApply;
            return Ok(TurnAction::Continue);
        }

        if let Some((player, spec)) = crate::turn::get_cleanup_discard_spec(game) {
            use crate::decisions::DecisionSpec;
            let ctx = spec.build_context(player, None, game);
            self.state = TurnState::CleanupRecursiveDiscard; // stay here
            return Ok(TurnAction::Decision(ctx));
        }

        // Done with cleanup, execute final cleanup step
        self.state = TurnState::CleanupApply;
        Ok(TurnAction::Continue)
    }
}

impl Default for TurnRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Check whether any creature in combat has first strike or double strike.
fn check_first_strike(game: &GameState, combat: &CombatState) -> bool {
    combat.attackers.iter().any(|info| {
        game.object(info.creature)
            .is_some_and(|obj| deals_first_strike_damage_with_game(obj, game))
    }) || combat.blockers.values().any(|blockers| {
        blockers.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| deals_first_strike_damage_with_game(obj, game))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::triggers::TriggerQueue;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_turn_runner_reaches_complete() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();

        // Drive the turn runner, providing auto-pass responses
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > 200 {
                panic!("TurnRunner did not complete within 200 iterations");
            }

            match runner.advance(&mut game, &mut tq).unwrap() {
                TurnAction::Continue => continue,
                TurnAction::RunPriority => {
                    // Auto-pass priority: run the priority loop with auto-pass DM
                    let mut dm = crate::decision::AutoPassDecisionMaker;
                    crate::game_loop::run_priority_loop_with(&mut game, &mut tq, &mut dm)
                        .unwrap();
                    runner.priority_done();
                }
                TurnAction::Decision(ctx) => {
                    // Auto-pass all decisions
                    match ctx {
                        DecisionContext::Attackers(_) => {
                            runner.respond_attackers(Vec::new());
                        }
                        DecisionContext::Blockers(ref bctx) => {
                            runner.respond_blockers(Vec::new(), bctx.player);
                        }
                        DecisionContext::SelectObjects(_) => {
                            runner.respond_discard(Vec::new());
                        }
                        _ => {
                            // Other decisions: skip
                        }
                    }
                }
                TurnAction::TurnComplete => break,
                TurnAction::GameOver(_) => break,
            }
        }

        assert!(matches!(runner.state(), TurnState::Complete));
    }

    #[test]
    fn test_state_machine_sequence() {
        // Verify the state machine progresses through expected phases
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();

        // BeginTurn -> Upkeep
        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::Continue));
        assert!(matches!(runner.state(), TurnState::Upkeep));

        // Upkeep -> RunPriority
        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert!(matches!(runner.state(), TurnState::UpkeepPriority));
    }
}
