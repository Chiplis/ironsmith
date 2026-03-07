//! Unified turn state machine that both CLI and WASM frontends drive.
//!
//! The [`TurnRunner`] sequences an entire MTG turn as a state machine,
//! yielding at decision points and priority windows so that callers can
//! provide player input (sync or async) and re-enter.

use crate::combat_state::CombatState;
use crate::decision::{AttackerDeclaration, BlockerDeclaration, GameResult};
use crate::decisions::context::{BooleanContext, DecisionContext};
use crate::game_loop::{
    GameLoopError, apply_attacker_declarations, apply_blocker_declarations,
    execute_combat_damage_step, generate_and_queue_step_triggers, get_declare_attackers_decision,
    get_declare_blockers_decision, put_triggers_on_stack, queue_combat_damage_triggers,
};
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId};
use crate::rules::combat::deals_first_strike_damage_with_game;
use crate::rules::state_based::check_state_based_actions;
use crate::triggers::TriggerQueue;
use crate::turn::{execute_cleanup_step, execute_untap_step};

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
    CombatDamageFirstStrikeSbas,
    CombatDamageFirstStrikePriority,
    CombatDamageRegular,
    CombatDamageRegularSbas,
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

#[derive(Debug, Clone)]
enum PendingCommanderChoice {
    DrawToHand { object_id: ObjectId },
    StateBasedReturn { object_id: ObjectId },
}

enum RunnerProgress<T> {
    Complete(T),
    NeedsDecision(DecisionContext),
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
    /// Pending yes/no response for runner-driven boolean decisions.
    pending_boolean: Option<bool>,
    /// Commander-specific choice that paused the runner.
    pending_commander_choice: Option<PendingCommanderChoice>,
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
            pending_boolean: None,
            pending_commander_choice: None,
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
                let draw_events = match self.execute_draw_step_with_choices(game) {
                    RunnerProgress::Complete(draw_events) => draw_events,
                    RunnerProgress::NeedsDecision(ctx) => return Ok(TurnAction::Decision(ctx)),
                };
                generate_and_queue_step_triggers(game, tq);

                // Queue triggers for each drawn card (Miracle, etc.)
                for draw_event in draw_events {
                    let triggered = crate::triggers::check::check_triggers(game, &draw_event);
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
                apply_attacker_declarations(game, &mut self.combat, tq, &declarations)?;
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

                let ctx = get_declare_blockers_decision(game, &self.combat, defending_player);
                self.state = TurnState::DeclareBlockersApply;
                Ok(TurnAction::Decision(ctx))
            }

            TurnState::DeclareBlockersApply => {
                let (declarations, defending_player) =
                    self.pending_blockers.take().unwrap_or_else(|| {
                        (
                            Vec::new(),
                            self.defending_player.unwrap_or(game.turn.active_player),
                        )
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
                self.state = TurnState::CombatDamageFirstStrikeSbas;
                Ok(TurnAction::Continue)
            }

            TurnState::CombatDamageFirstStrikeSbas => {
                match self.apply_sbas_until_commander_choice(game, tq)? {
                    RunnerProgress::Complete(()) => {
                        self.state = TurnState::CombatDamageFirstStrikePriority;
                        Ok(TurnAction::RunPriority)
                    }
                    RunnerProgress::NeedsDecision(ctx) => Ok(TurnAction::Decision(ctx)),
                }
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
                self.state = TurnState::CombatDamageRegularSbas;
                Ok(TurnAction::Continue)
            }

            TurnState::CombatDamageRegularSbas => {
                match self.apply_sbas_until_commander_choice(game, tq)? {
                    RunnerProgress::Complete(()) => {
                        self.state = TurnState::CombatDamageRegularPriority;
                        Ok(TurnAction::RunPriority)
                    }
                    RunnerProgress::NeedsDecision(ctx) => Ok(TurnAction::Decision(ctx)),
                }
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
                    match self.apply_sbas_until_commander_choice(game, tq)? {
                        RunnerProgress::Complete(()) => {}
                        RunnerProgress::NeedsDecision(ctx) => return Ok(TurnAction::Decision(ctx)),
                    }
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

            TurnState::CleanupRecursiveDiscard => self.advance_cleanup_discard_recursive(game),

            TurnState::Complete => Ok(TurnAction::TurnComplete),
        }
    }

    /// Provide attacker declarations in response to a `Decision(Attackers(...))`.
    pub fn respond_attackers(&mut self, declarations: Vec<AttackerDeclaration>) {
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

    /// Provide a boolean response in response to a `Decision(Boolean(...))`.
    pub fn respond_boolean(&mut self, answer: bool) {
        self.pending_boolean = Some(answer);
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

    fn execute_draw_step_with_choices(
        &mut self,
        game: &mut GameState,
    ) -> RunnerProgress<Vec<crate::triggers::TriggerEvent>> {
        use crate::events::other::CardsDrawnEvent;
        use crate::triggers::TriggerEvent;

        let active_player = game.turn.active_player;
        if game.skip_next_draw_step.remove(&active_player) {
            game.turn.priority_player = Some(active_player);
            return RunnerProgress::Complete(Vec::new());
        }

        let current_draws = game
            .cards_drawn_this_turn
            .get(&active_player)
            .copied()
            .unwrap_or(0);
        let is_first_draw = current_draws == 0;
        let can_draw = if !game.can_draw_extra_cards(active_player) {
            current_draws == 0
        } else {
            true
        };

        let mut drawn = Vec::new();
        if can_draw {
            match self.pending_commander_choice.take() {
                Some(PendingCommanderChoice::DrawToHand { object_id }) => {
                    let send_to_command = self.pending_boolean.take().unwrap_or(false);
                    let final_zone = if send_to_command {
                        crate::zone::Zone::Command
                    } else {
                        crate::zone::Zone::Hand
                    };
                    if let Some(new_id) = game.move_object(object_id, final_zone)
                        && final_zone == crate::zone::Zone::Hand
                    {
                        drawn.push(new_id);
                    }
                }
                Some(other) => {
                    self.pending_commander_choice = Some(other);
                }
                None => {
                    if let Some(card_id) = game
                        .player(active_player)
                        .and_then(|player| player.library.last().copied())
                    {
                        if game.is_commander(card_id) {
                            if let Some(obj) = game.object(card_id) {
                                let ctx = DecisionContext::Boolean(
                                    BooleanContext::new(
                                        obj.owner,
                                        Some(card_id),
                                        "move it to the command zone instead of putting it into its owner's hand",
                                    )
                                    .with_source_name(obj.name.clone()),
                                );
                                self.pending_commander_choice =
                                    Some(PendingCommanderChoice::DrawToHand { object_id: card_id });
                                return RunnerProgress::NeedsDecision(ctx);
                            }
                        } else if let Some(new_id) =
                            game.move_object(card_id, crate::zone::Zone::Hand)
                        {
                            drawn.push(new_id);
                        }
                    }
                }
            }
        }

        *game.cards_drawn_this_turn.entry(active_player).or_insert(0) += drawn.len() as u32;

        let mut draw_events = Vec::new();
        if !drawn.is_empty() {
            let draw_event_provenance = game
                .provenance_graph
                .alloc_root_event(crate::events::EventKind::CardsDrawn);
            let event = CardsDrawnEvent::new(active_player, drawn, is_first_draw);
            draw_events.push(TriggerEvent::new_with_provenance(
                event,
                draw_event_provenance,
            ));
        }

        game.turn.priority_player = Some(active_player);
        RunnerProgress::Complete(draw_events)
    }

    fn apply_sbas_until_commander_choice(
        &mut self,
        game: &mut GameState,
        tq: &mut TriggerQueue,
    ) -> Result<RunnerProgress<()>, GameLoopError> {
        use crate::decisions::make_decision;
        use crate::rules::state_based::{
            StateBasedAction, apply_legend_rule_choice,
            apply_state_based_actions_from_actions_with, check_state_based_actions_with_view,
            legend_rule_specs_from_actions,
        };

        game.refresh_continuous_state();

        loop {
            let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
            let all_effects = view.effects().to_vec();
            let actions = check_state_based_actions_with_view(game, &view);
            drop(view);
            if actions.is_empty() {
                self.pending_boolean = None;
                self.pending_commander_choice = None;
                return Ok(RunnerProgress::Complete(()));
            }

            let legend_specs = legend_rule_specs_from_actions(&actions);
            if !legend_specs.is_empty() {
                let mut auto_dm = crate::decision::AutoPassDecisionMaker;
                for (player, spec) in legend_specs {
                    let keep_id: ObjectId = make_decision(game, &mut auto_dm, player, None, spec);
                    apply_legend_rule_choice(game, keep_id);
                }
                crate::game_loop::drain_pending_trigger_events(game, tq);
                continue;
            }

            let mut commander_returns = Vec::new();
            let mut other_actions = Vec::new();
            for action in actions {
                match action {
                    StateBasedAction::CommanderReturnsToCommandZone(obj_id) => {
                        commander_returns.push(obj_id);
                    }
                    other => other_actions.push(other),
                }
            }

            if !other_actions.is_empty() {
                let mut auto_dm = crate::decision::AutoPassDecisionMaker;
                let applied = apply_state_based_actions_from_actions_with(
                    game,
                    other_actions,
                    &all_effects,
                    &mut auto_dm,
                );
                crate::game_loop::drain_pending_trigger_events(game, tq);
                if !applied {
                    self.pending_boolean = None;
                    self.pending_commander_choice = None;
                    return Ok(RunnerProgress::Complete(()));
                }
                continue;
            }

            let Some(obj_id) = commander_returns.first().copied() else {
                self.pending_boolean = None;
                self.pending_commander_choice = None;
                return Ok(RunnerProgress::Complete(()));
            };

            match self.pending_commander_choice.take() {
                Some(PendingCommanderChoice::StateBasedReturn { object_id })
                    if object_id == obj_id =>
                {
                    let send_to_command = self.pending_boolean.take().unwrap_or(false);
                    if send_to_command {
                        game.move_object(obj_id, crate::zone::Zone::Command);
                    } else {
                        game.decline_commander_command_zone_move(obj_id);
                    }
                    crate::game_loop::drain_pending_trigger_events(game, tq);
                    continue;
                }
                Some(other) => {
                    self.pending_commander_choice = Some(other);
                }
                None => {}
            }

            let Some(obj) = game.object(obj_id) else {
                continue;
            };
            let ctx = DecisionContext::Boolean(
                BooleanContext::new(obj.owner, Some(obj_id), "move it to the command zone")
                    .with_source_name(obj.name.clone()),
            );
            self.pending_commander_choice =
                Some(PendingCommanderChoice::StateBasedReturn { object_id: obj_id });
            return Ok(RunnerProgress::NeedsDecision(ctx));
        }
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
        crate::tests::test_helpers::setup_two_player_game()
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
                    crate::game_loop::run_priority_loop_with(&mut game, &mut tq, &mut dm).unwrap();
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
                        DecisionContext::Boolean(_) => {
                            runner.respond_boolean(false);
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

    #[test]
    fn test_turn_runner_pauses_for_drawn_commander_choice() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let commander =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(9100), "Runner Commander")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let commander_id =
            game.create_object_from_card(&commander, alice, crate::zone::Zone::Library);
        game.set_as_commander(commander_id, alice);

        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        runner.state = TurnState::Draw;

        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(
            action,
            TurnAction::Decision(DecisionContext::Boolean(_))
        ));

        runner.respond_boolean(true);
        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(game.objects_in_zone(crate::zone::Zone::Command).len(), 1);
    }

    #[test]
    fn test_turn_runner_pauses_for_commander_sba_choice() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let commander =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(9101), "Fallen Commander")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let commander_id =
            game.create_object_from_card(&commander, alice, crate::zone::Zone::Graveyard);
        game.set_as_commander(commander_id, alice);

        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();

        let action = runner
            .apply_sbas_until_commander_choice(&mut game, &mut tq)
            .unwrap();
        assert!(matches!(
            action,
            RunnerProgress::NeedsDecision(DecisionContext::Boolean(_))
        ));

        runner.respond_boolean(true);
        let action = runner
            .apply_sbas_until_commander_choice(&mut game, &mut tq)
            .unwrap();
        assert!(matches!(action, RunnerProgress::Complete(())));
        assert_eq!(game.objects_in_zone(crate::zone::Zone::Command).len(), 1);
    }
}
