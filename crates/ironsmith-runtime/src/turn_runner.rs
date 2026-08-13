//! Unified turn state machine that both CLI and WASM frontends drive.
//!
//! The [`TurnRunner`] sequences an entire MTG turn as a state machine,
//! yielding at decision points and priority windows so that callers can
//! provide player input (sync or async) and re-enter.

use crate::combat_state::CombatState;
use crate::decision::{
    AttackerDeclaration, AutoPassDecisionMaker, BlockerDeclaration, DecisionMaker, GameResult,
};
use crate::decisions::context::{BooleanContext, DecisionContext};
use crate::game_loop::{
    AttackDeclarationTransaction, BlockDeclarationTransaction, GameLoopError,
    apply_attack_mana_ability_window_response, apply_attacker_declarations_with_dm,
    apply_blocker_mana_ability_window_response, attack_mana_ability_window_context,
    begin_attack_declaration_transaction, begin_blocker_declaration_transaction,
    blocker_mana_ability_window_context, drain_pending_trigger_events,
    finish_attack_declaration_transaction, finish_blocker_declaration_transaction,
    generate_and_queue_step_triggers, get_declare_attackers_decision,
    get_declare_blockers_decision, preview_optional_attack_cost_prompts,
    preview_required_attack_mana_cost, put_triggers_on_stack, queue_combat_damage_triggers,
    try_execute_combat_damage_step, try_execute_combat_damage_step_with_first_step_snapshot,
};
use crate::game_state::{
    AddedStepPlacement, GameState, Phase, ScheduledStep, Step, TurnScheduleDestination,
};
use crate::ids::{ObjectId, PlayerId};
use crate::rules::combat::deals_first_strike_damage_with_game;
use crate::rules::state_based::check_state_based_actions;
use crate::triggers::TriggerQueue;
use crate::turn::{execute_cleanup_step, execute_untap_step, execute_untap_step_with};

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
    Untap,
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
    EndCombatPhaseSbas,

    // === Second Main Phase ===
    NextMain,
    NextMainPriority,

    // === Ending Phase ===
    EndStep,
    EndStepPriority,
    EndTurnSbas,
    CleanupDiscard,
    CleanupApply,
    CleanupRecursiveCheck,
    CleanupRecursivePriority,
    CleanupRecursiveDiscard,

    // === Terminal ===
    Complete,
}

impl TurnState {
    pub fn sync_name(&self) -> &'static str {
        match self {
            Self::BeginTurn => "begin_turn",
            Self::Untap => "untap",
            Self::Upkeep => "upkeep",
            Self::UpkeepPriority => "upkeep_priority",
            Self::Draw => "draw",
            Self::DrawPriority => "draw_priority",
            Self::FirstMain => "first_main",
            Self::FirstMainPriority => "first_main_priority",
            Self::BeginCombat => "begin_combat",
            Self::BeginCombatPriority => "begin_combat_priority",
            Self::DeclareAttackersDecision => "declare_attackers_decision",
            Self::DeclareAttackersApply => "declare_attackers_apply",
            Self::DeclareAttackersPriority => "declare_attackers_priority",
            Self::DeclareBlockersCheck => "declare_blockers_check",
            Self::DeclareBlockersDecision => "declare_blockers_decision",
            Self::DeclareBlockersApply => "declare_blockers_apply",
            Self::DeclareBlockersPriority => "declare_blockers_priority",
            Self::CombatDamageFirstStrike => "combat_damage_first_strike",
            Self::CombatDamageFirstStrikeSbas => "combat_damage_first_strike_sbas",
            Self::CombatDamageFirstStrikePriority => "combat_damage_first_strike_priority",
            Self::CombatDamageRegular => "combat_damage_regular",
            Self::CombatDamageRegularSbas => "combat_damage_regular_sbas",
            Self::CombatDamageRegularPriority => "combat_damage_regular_priority",
            Self::EndCombat => "end_combat",
            Self::EndCombatPriority => "end_combat_priority",
            Self::EndCombatPhaseSbas => "end_combat_phase_sbas",
            Self::NextMain => "next_main",
            Self::NextMainPriority => "next_main_priority",
            Self::EndStep => "end_step",
            Self::EndStepPriority => "end_step_priority",
            Self::EndTurnSbas => "end_turn_sbas",
            Self::CleanupDiscard => "cleanup_discard",
            Self::CleanupApply => "cleanup_apply",
            Self::CleanupRecursiveCheck => "cleanup_recursive_check",
            Self::CleanupRecursivePriority => "cleanup_recursive_priority",
            Self::CleanupRecursiveDiscard => "cleanup_recursive_discard",
            Self::Complete => "complete",
        }
    }

    pub fn from_sync_name(raw: &str) -> Option<Self> {
        Some(match raw {
            "begin_turn" => Self::BeginTurn,
            "untap" => Self::Untap,
            "upkeep" => Self::Upkeep,
            "upkeep_priority" => Self::UpkeepPriority,
            "draw" => Self::Draw,
            "draw_priority" => Self::DrawPriority,
            "first_main" => Self::FirstMain,
            "first_main_priority" => Self::FirstMainPriority,
            "begin_combat" => Self::BeginCombat,
            "begin_combat_priority" => Self::BeginCombatPriority,
            "declare_attackers_decision" => Self::DeclareAttackersDecision,
            "declare_attackers_apply" => Self::DeclareAttackersApply,
            "declare_attackers_priority" => Self::DeclareAttackersPriority,
            "declare_blockers_check" => Self::DeclareBlockersCheck,
            "declare_blockers_decision" => Self::DeclareBlockersDecision,
            "declare_blockers_apply" => Self::DeclareBlockersApply,
            "declare_blockers_priority" => Self::DeclareBlockersPriority,
            "combat_damage_first_strike" => Self::CombatDamageFirstStrike,
            "combat_damage_first_strike_sbas" => Self::CombatDamageFirstStrikeSbas,
            "combat_damage_first_strike_priority" => Self::CombatDamageFirstStrikePriority,
            "combat_damage_regular" => Self::CombatDamageRegular,
            "combat_damage_regular_sbas" => Self::CombatDamageRegularSbas,
            "combat_damage_regular_priority" => Self::CombatDamageRegularPriority,
            "end_combat" => Self::EndCombat,
            "end_combat_priority" => Self::EndCombatPriority,
            "end_combat_phase_sbas" => Self::EndCombatPhaseSbas,
            "next_main" => Self::NextMain,
            "next_main_priority" => Self::NextMainPriority,
            "end_step" => Self::EndStep,
            "end_step_priority" => Self::EndStepPriority,
            "end_turn_sbas" => Self::EndTurnSbas,
            "cleanup_discard" => Self::CleanupDiscard,
            "cleanup_apply" => Self::CleanupApply,
            "cleanup_recursive_check" => Self::CleanupRecursiveCheck,
            "cleanup_recursive_priority" => Self::CleanupRecursivePriority,
            "cleanup_recursive_discard" => Self::CleanupRecursiveDiscard,
            "complete" => Self::Complete,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
enum PendingCommanderChoice {
    DrawToHand { object_id: ObjectId },
    StateBasedReturn { object_id: ObjectId },
}

/// Identifies the legend-rule violation the runner paused on, so a
/// `respond_discard` answer is only consumed by the prompt that asked for it.
#[derive(Debug, Clone)]
struct PendingLegendRuleChoice {
    player: PlayerId,
    legends: Vec<ObjectId>,
}

/// A CR 704.5u assignment batch paused between asynchronous sector choices.
/// Choices remain private here and are committed together after the final one.
#[derive(Debug, Clone)]
struct PendingSectorDesignationChoices {
    source: ObjectId,
    creatures: Vec<(PlayerId, ObjectId)>,
    choices: Vec<crate::marker::SectorDesignation>,
}

#[derive(Debug, Clone)]
struct PendingDrawReplacementChoice {
    player: PlayerId,
    applicable_effects: Vec<crate::replacement::ReplacementEffectId>,
    event: crate::events::Event,
    applied_effects: std::collections::HashSet<crate::replacement::ReplacementEffectId>,
    applied_effect_keys: std::collections::HashSet<crate::replacement::ReplacementEffectKey>,
}

fn draw_replacement_choice_context(
    game: &GameState,
    pending: &PendingDrawReplacementChoice,
) -> DecisionContext {
    let options = pending
        .applicable_effects
        .iter()
        .enumerate()
        .filter_map(|(index, effect_id)| {
            game.effect_store
                .replacement_effects
                .get_effect(*effect_id)
                .map(|effect| {
                    crate::decisions::context::SelectableOption::new(
                        index,
                        crate::events::processing::replacement_effect_choice_description(
                            game, effect,
                        ),
                    )
                    .with_object(effect.source)
                })
        })
        .collect();
    DecisionContext::SelectOptions(crate::decisions::context::SelectOptionsContext::new(
        pending.player,
        None,
        "Choose a draw replacement effect to apply",
        options,
        1,
        1,
    ))
}

#[derive(Debug, Clone)]
struct PendingDrawRevealChoice {
    active_player: PlayerId,
    drawn: Vec<ObjectId>,
    is_first_draw: bool,
    draw_event_provenance: crate::provenance::ProvNodeId,
    candidates: Vec<crate::effects::cards::AutomaticDrawRevealCandidate>,
    next_candidate_index: usize,
    reveal_events: Vec<crate::triggers::TriggerEvent>,
}

#[derive(Debug, Clone)]
struct PendingAttackerOptionalCosts {
    transaction: AttackDeclarationTransaction,
    prompts: Vec<DecisionContext>,
    answers: Vec<AttackCostAnswer>,
    required_mana_cost: u32,
    declaration_source: ObjectId,
}

#[derive(Debug, Clone)]
struct PendingAttackerManaWindow {
    transaction: AttackDeclarationTransaction,
    optional_cost_answers: Vec<AttackCostAnswer>,
    declaration_source: ObjectId,
}

#[derive(Debug, Clone)]
struct PendingBlockerManaWindow {
    transaction: BlockDeclarationTransaction,
    payers: Vec<PlayerId>,
    next_payer: usize,
    declaration_source: ObjectId,
}

fn next_blocker_mana_window_context(
    game: &GameState,
    pending: &mut PendingBlockerManaWindow,
) -> Option<crate::decisions::context::SelectOptionsContext> {
    while let Some(&payer) = pending.payers.get(pending.next_payer) {
        if let Some(context) =
            blocker_mana_ability_window_context(game, payer, pending.declaration_source)
        {
            return Some(context);
        }
        pending.next_payer += 1;
    }
    None
}

#[derive(Debug, Clone)]
struct PendingUntapChoices {
    prompts: Vec<BooleanContext>,
    answers: Vec<bool>,
}

enum RunnerProgress<T> {
    Complete(T),
    NeedsDecision(DecisionContext),
}

#[derive(Debug, Clone)]
struct QueuedBooleanDecisionMaker {
    answers: Vec<bool>,
    next: usize,
}

#[derive(Debug, Clone)]
enum AttackCostAnswer {
    Boolean(bool),
    Objects(Vec<ObjectId>),
}

#[derive(Debug, Clone)]
struct QueuedAttackCostDecisionMaker {
    answers: Vec<AttackCostAnswer>,
    next: usize,
}

#[derive(Default)]
struct BooleanPromptCollector {
    prompts: Vec<BooleanContext>,
}

impl DecisionMaker for BooleanPromptCollector {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.prompts.push(ctx.clone());
        false
    }
}

impl QueuedBooleanDecisionMaker {
    fn new(answers: Vec<bool>) -> Self {
        Self { answers, next: 0 }
    }
}

impl DecisionMaker for QueuedBooleanDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        let answer = self.answers.get(self.next).copied().unwrap_or(false);
        self.next += 1;
        answer
    }
}

impl QueuedAttackCostDecisionMaker {
    fn new(answers: Vec<AttackCostAnswer>) -> Self {
        Self { answers, next: 0 }
    }

    fn next_answer(&mut self) -> Option<AttackCostAnswer> {
        let answer = self.answers.get(self.next).cloned();
        self.next += 1;
        answer
    }
}

impl DecisionMaker for QueuedAttackCostDecisionMaker {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        match self.next_answer() {
            Some(AttackCostAnswer::Boolean(answer)) => answer,
            _ => false,
        }
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        match self.next_answer() {
            Some(AttackCostAnswer::Objects(objects)) => objects,
            _ => Vec::new(),
        }
    }
}

fn validate_declared_attacking_bands(
    game: &GameState,
    declarations: &[AttackerDeclaration],
    bands: &[Vec<ObjectId>],
) -> Result<(), GameLoopError> {
    let mut proposed = CombatState::default();
    proposed.attackers = declarations
        .iter()
        .map(|declaration| crate::combat_state::AttackerInfo {
            creature: declaration.creature,
            target: declaration.target.clone(),
        })
        .collect();
    for band in bands {
        crate::combat_state::set_attacking_band(game, &mut proposed, band.clone())?;
    }
    Ok(())
}

fn record_declared_attacking_bands(combat: &mut CombatState, bands: Vec<Vec<ObjectId>>) {
    for band in bands {
        let survivors = band
            .into_iter()
            .filter(|member| crate::combat_state::is_attacking(combat, *member))
            .collect::<Vec<_>>();
        if survivors.len() > 1 {
            combat.attacking_bands.push(survivors);
        }
    }
}

/// Drives a single turn as a state machine.
#[derive(Debug, Clone)]
pub struct TurnRunner {
    state: TurnState,
    /// Combat state owned by the runner for the duration of combat.
    combat: CombatState,
    /// Whether first-strike creatures were detected this combat.
    has_first_strike: bool,
    /// Creatures that had first or double strike as the first combat-damage
    /// step began (CR 510.4 eligibility snapshot).
    first_step_strikers: std::collections::HashSet<ObjectId>,
    /// Pending attacker declarations from the caller.
    pending_attackers: Option<Vec<AttackerDeclaration>>,
    /// Bands announced as part of the pending attacker declaration.
    pending_attacking_bands: Option<Vec<Vec<ObjectId>>>,
    /// Mandatory choices for permanents that may remain tapped this untap step.
    pending_untap_choices: Option<PendingUntapChoices>,
    /// Pending attacker-cost prompts and their collected answers.
    pending_attacker_optional_costs: Option<PendingAttackerOptionalCosts>,
    /// Attack declaration paused after CR 508.1f tapping and before costs.
    pending_attacker_mana_window: Option<PendingAttackerManaWindow>,
    /// Block declaration paused after CR 509.1d cost locking and before payment.
    pending_blocker_mana_window: Option<PendingBlockerManaWindow>,
    /// Pending single-option response for a runner-driven SelectOptions decision.
    pending_option: Option<usize>,
    /// Pending blocker declarations from the caller.
    pending_blockers: Option<(Vec<BlockerDeclaration>, PlayerId)>,
    /// Pending discard selection from the caller.
    pending_discard: Option<Vec<ObjectId>>,
    /// Pending yes/no response for runner-driven boolean decisions.
    pending_boolean: Option<bool>,
    /// Pending CR 616 choice among draw replacement effects.
    pending_draw_replacement: Option<PendingDrawReplacementChoice>,
    /// Pending first-draw reveal decisions that pause the draw step.
    pending_draw_reveal: Option<PendingDrawRevealChoice>,
    /// Active teammates whose turn-based draw is still pending this draw step.
    remaining_draw_players: Vec<PlayerId>,
    /// Draw events accumulated while shared-team draw choices pause and resume.
    shared_draw_events: Vec<crate::triggers::TriggerEvent>,
    /// Commander-specific choice that paused the runner.
    pending_commander_choice: Option<PendingCommanderChoice>,
    /// Legend-rule keep choice that paused the runner.
    pending_legend_choice: Option<PendingLegendRuleChoice>,
    /// Space-sculptor sector choices collected without partial state writes.
    pending_sector_designations: Option<PendingSectorDesignationChoices>,
    /// Defending player for the current combat.
    defending_player: Option<PlayerId>,
    /// Defending players who still need to declare blockers, in APNAP order.
    remaining_defending_players: Vec<PlayerId>,
}

impl TurnRunner {
    /// Create a new TurnRunner starting at the beginning of a turn.
    pub fn new() -> Self {
        Self {
            state: TurnState::BeginTurn,
            combat: CombatState::default(),
            has_first_strike: false,
            first_step_strikers: std::collections::HashSet::new(),
            pending_attackers: None,
            pending_attacking_bands: None,
            pending_untap_choices: None,
            pending_attacker_optional_costs: None,
            pending_attacker_mana_window: None,
            pending_blocker_mana_window: None,
            pending_option: None,
            pending_blockers: None,
            pending_discard: None,
            pending_boolean: None,
            pending_draw_replacement: None,
            pending_draw_reveal: None,
            remaining_draw_players: Vec::new(),
            shared_draw_events: Vec::new(),
            pending_commander_choice: None,
            pending_legend_choice: None,
            pending_sector_designations: None,
            defending_player: None,
            remaining_defending_players: Vec::new(),
        }
    }

    /// Rebuild a runner at a previously checkpointed state.
    pub fn from_state_for_sync(state: TurnState) -> Self {
        let mut runner = Self::new();
        runner.state = state;
        runner
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
        if game.turn_store.end_combat_phase_procedure_pending {
            // CR 724.2a: external entries triggered before the procedure cease
            // to exist. Events produced during stack exile remain staged in
            // GameState until the following phase's priority window.
            *tq = TriggerQueue::new();
            game.turn_store.end_combat_phase_procedure_pending = false;
            self.state = TurnState::EndCombatPhaseSbas;
        }
        if game.turn_store.end_turn_procedure_pending {
            // CR 724.1a: entries already in the external queue triggered
            // before the procedure. Events created by stack exile remain in
            // GameState and are discovered during the cleanup trigger check.
            *tq = TriggerQueue::new();
            game.turn_store.end_turn_procedure_pending = false;
            self.state = TurnState::EndTurnSbas;
        }
        let active_player = game.turn.active_player;
        let skipped_state = match self.state {
            TurnState::Untap => Some((Step::Untap, false)),
            TurnState::Upkeep => Some((Step::Upkeep, false)),
            TurnState::Draw => Some((Step::Draw, true)),
            TurnState::BeginCombat => Some((Step::BeginCombat, false)),
            TurnState::DeclareAttackersDecision => Some((Step::DeclareAttackers, false)),
            TurnState::DeclareBlockersCheck => Some((Step::DeclareBlockers, false)),
            TurnState::CombatDamageFirstStrike | TurnState::CombatDamageRegular => {
                Some((Step::CombatDamage, false))
            }
            TurnState::EndCombat => Some((Step::EndCombat, true)),
            TurnState::EndStep => Some((Step::End, false)),
            TurnState::CleanupDiscard => Some((Step::Cleanup, true)),
            _ => None,
        };
        if let Some((step, ends_phase)) = skipped_state
            && game.consume_step_skip(active_player, step)
        {
            let ends_phase = ends_phase
                || game
                    .turn_store
                    .active_added_step
                    .is_some_and(|scheduled| scheduled.isolated_phase);
            if matches!(self.state, TurnState::DeclareBlockersCheck) {
                self.first_step_strikers = first_step_strikers(game, &self.combat);
                self.has_first_strike = !self.first_step_strikers.is_empty();
            }
            let normal_next = match self.state {
                TurnState::Untap => TurnScheduleDestination::Step(Step::Upkeep),
                TurnState::Upkeep => TurnScheduleDestination::Step(Step::Draw),
                TurnState::Draw => TurnScheduleDestination::Phase(Phase::FirstMain),
                TurnState::BeginCombat => TurnScheduleDestination::Step(Step::DeclareAttackers),
                TurnState::DeclareAttackersDecision => {
                    TurnScheduleDestination::Step(Step::DeclareBlockers)
                }
                TurnState::DeclareBlockersCheck if self.has_first_strike => {
                    TurnScheduleDestination::CombatDamageFirstStrike
                }
                TurnState::DeclareBlockersCheck => TurnScheduleDestination::CombatDamageRegular,
                TurnState::CombatDamageFirstStrike => TurnScheduleDestination::CombatDamageRegular,
                TurnState::CombatDamageRegular => TurnScheduleDestination::Step(Step::EndCombat),
                TurnState::EndCombat => TurnScheduleDestination::Phase(Phase::NextMain),
                TurnState::EndStep => TurnScheduleDestination::Step(Step::Cleanup),
                TurnState::CleanupDiscard => TurnScheduleDestination::Complete,
                _ => TurnScheduleDestination::Complete,
            };
            self.state = if ends_phase {
                let phase = game.turn.phase;
                finish_step_and_phase(game, step, phase, normal_next)
            } else {
                finish_step(game, step, normal_next)
            };
            return Ok(TurnAction::Continue);
        }

        match self.state {
            // ================================================================
            // Beginning Phase
            // ================================================================
            TurnState::BeginTurn => {
                game.record_turn_start_hand_sizes();
                for player in game.turn_players() {
                    game.activate_pending_player_control(player);
                }

                // Untap step — no priority
                game.turn.phase = Phase::Beginning;
                game.turn.step = Some(Step::Untap);

                if game.consume_step_skip(game.turn.active_player, Step::Untap) {
                    self.state = finish_step(
                        game,
                        Step::Untap,
                        TurnScheduleDestination::Step(Step::Upkeep),
                    );
                    return Ok(TurnAction::Continue);
                }

                let mut hypothetical = game.clone();
                let mut collector = BooleanPromptCollector::default();
                execute_untap_step_with(&mut hypothetical, &mut collector);
                if let Some(first_prompt) = collector.prompts.first().cloned() {
                    self.pending_untap_choices = Some(PendingUntapChoices {
                        prompts: collector.prompts,
                        answers: Vec::new(),
                    });
                    self.pending_boolean = None;
                    self.state = TurnState::Untap;
                    return Ok(TurnAction::Decision(DecisionContext::Boolean(first_prompt)));
                }

                execute_untap_step(game);

                self.state = finish_step(
                    game,
                    Step::Untap,
                    TurnScheduleDestination::Step(Step::Upkeep),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::Untap => {
                if self.pending_untap_choices.is_none() {
                    let mut hypothetical = game.clone();
                    let mut collector = BooleanPromptCollector::default();
                    execute_untap_step_with(&mut hypothetical, &mut collector);
                    self.pending_untap_choices = Some(PendingUntapChoices {
                        prompts: collector.prompts,
                        answers: Vec::new(),
                    });
                }
                let pending = self
                    .pending_untap_choices
                    .as_mut()
                    .expect("untap choices initialized");
                if let Some(answer) = self.pending_boolean.take() {
                    pending.answers.push(answer);
                }
                if pending.answers.len() < pending.prompts.len() {
                    return Ok(TurnAction::Decision(DecisionContext::Boolean(
                        pending.prompts[pending.answers.len()].clone(),
                    )));
                }

                let pending = self
                    .pending_untap_choices
                    .take()
                    .expect("untap choices remain initialized");
                let mut dm = QueuedBooleanDecisionMaker::new(pending.answers);
                execute_untap_step_with(game, &mut dm);
                self.state = finish_step(
                    game,
                    Step::Untap,
                    TurnScheduleDestination::Step(Step::Upkeep),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::Upkeep => {
                if game
                    .turn_players()
                    .into_iter()
                    .any(|player| game.player_skips_upkeep_step(player))
                {
                    game.turn.step = Some(Step::Upkeep);
                    game.reset_priority_for_new_window();
                    self.state = finish_step(
                        game,
                        Step::Upkeep,
                        TurnScheduleDestination::Step(Step::Draw),
                    );
                    return Ok(TurnAction::Continue);
                }
                game.turn.step = Some(Step::Upkeep);
                for player in game.turn_players() {
                    game.mark_upkeep_began(player);
                }
                game.reset_priority_for_new_window();
                drain_pending_trigger_events(game, tq);
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::UpkeepPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::UpkeepPriority => {
                game.empty_mana_pools();
                self.state = finish_step(
                    game,
                    Step::Upkeep,
                    TurnScheduleDestination::Step(Step::Draw),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::Draw => {
                // CR 702.57b: a Forecast card stops being revealed as soon as
                // a step other than upkeep begins.
                game.clear_forecast_revealed_hand_cards();
                game.turn.step = Some(Step::Draw);
                let draw_events = match self.execute_draw_step_with_choices(game) {
                    RunnerProgress::Complete(draw_events) => draw_events,
                    RunnerProgress::NeedsDecision(ctx) => return Ok(TurnAction::Decision(ctx)),
                };
                crate::game_loop::drain_pending_trigger_events(game, tq);
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
                self.state = finish_step_and_phase(
                    game,
                    Step::Draw,
                    Phase::Beginning,
                    TurnScheduleDestination::Phase(Phase::FirstMain),
                );
                Ok(TurnAction::Continue)
            }

            // ================================================================
            // First Main Phase
            // ================================================================
            TurnState::FirstMain => {
                if game
                    .turn_store
                    .skip_current_turn_main_phases
                    .contains(&game.turn.active_player)
                {
                    game.turn.phase = Phase::FirstMain;
                    self.state = finish_phase(
                        game,
                        Phase::FirstMain,
                        TurnScheduleDestination::Phase(Phase::Combat),
                    );
                    return Ok(TurnAction::Continue);
                }
                game.turn.phase = Phase::FirstMain;
                game.turn.step = None;
                game.reset_priority_for_new_window();
                generate_and_queue_step_triggers(game, tq);
                // CR 505.3: an archenemy sets the top scheme in motion as a
                // turn-based action before Saga lore counters are added.
                if game.is_archenemy(game.turn.active_player)
                    && game
                        .scheme_deck(game.turn.active_player)
                        .is_some_and(|deck| !deck.is_empty())
                {
                    game.set_scheme_in_motion(game.turn.active_player)
                        .map_err(GameLoopError::InvalidState)?;
                }
                crate::game_loop::add_saga_lore_counters(game, tq);
                // CR 505.5: after the Saga turn-based action, roll to visit
                // Attractions if the active player controls one.
                crate::game_loop::roll_to_visit_attractions(game, tq)?;

                self.state = TurnState::FirstMainPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::FirstMainPriority => {
                game.empty_mana_pools();
                self.state = next_runner_state_after_phase(game, TurnState::BeginCombat);
                Ok(TurnAction::Continue)
            }

            // ================================================================
            // Combat Phase
            // ================================================================
            TurnState::BeginCombat => {
                if game
                    .turn_store
                    .skip_current_turn_combat_phases
                    .contains(&game.turn.active_player)
                    || game
                        .turn_store
                        .skip_next_combat_phases
                        .remove(&game.turn.active_player)
                {
                    game.turn.phase = Phase::Combat;
                    self.state = if game
                        .turn_store
                        .active_added_step
                        .is_some_and(|scheduled| scheduled.isolated_phase)
                    {
                        finish_step_and_phase(
                            game,
                            Step::BeginCombat,
                            Phase::Combat,
                            TurnScheduleDestination::Phase(Phase::NextMain),
                        )
                    } else if game.turn_store.active_added_step.is_some() {
                        finish_step(
                            game,
                            Step::BeginCombat,
                            TurnScheduleDestination::Step(Step::DeclareAttackers),
                        )
                    } else {
                        finish_phase(
                            game,
                            Phase::Combat,
                            TurnScheduleDestination::Phase(Phase::NextMain),
                        )
                    };
                    return Ok(TurnAction::Continue);
                }
                game.turn.phase = Phase::Combat;
                game.mark_combat_phase_started();
                game.turn.step = Some(Step::BeginCombat);
                game.reset_priority_for_new_window();
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::BeginCombatPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::BeginCombatPriority => {
                game.empty_mana_pools();
                self.state = finish_step(
                    game,
                    Step::BeginCombat,
                    TurnScheduleDestination::Step(Step::DeclareAttackers),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::DeclareAttackersDecision => {
                game.turn.step = Some(Step::DeclareAttackers);
                game.reset_priority_for_new_window();
                self.pending_attacker_optional_costs = None;
                self.pending_attacker_mana_window = None;
                self.pending_option = None;
                self.pending_boolean = None;
                self.pending_discard = None;
                self.pending_draw_replacement = None;

                let ctx = get_declare_attackers_decision(game, &self.combat);
                self.state = TurnState::DeclareAttackersApply;
                Ok(TurnAction::Decision(ctx))
            }

            TurnState::DeclareAttackersApply => {
                if let Some(pending) = self.pending_attacker_mana_window.take() {
                    let mut window_closed = false;
                    if let Some(choice) = self.pending_option.take() {
                        match apply_attack_mana_ability_window_response(
                            game,
                            tq,
                            game.turn.active_player,
                            choice,
                        ) {
                            Ok(closed) => window_closed = closed,
                            Err(err) => {
                                self.pending_attacker_mana_window = Some(pending);
                                return Err(err);
                            }
                        }
                    }

                    if !window_closed
                        && let Some(ctx) = attack_mana_ability_window_context(
                            game,
                            game.turn.active_player,
                            pending.declaration_source,
                        )
                    {
                        self.pending_attacker_mana_window = Some(pending);
                        return Ok(TurnAction::Decision(DecisionContext::SelectOptions(ctx)));
                    }

                    let mut dm = QueuedAttackCostDecisionMaker::new(pending.optional_cost_answers);
                    finish_attack_declaration_transaction(
                        pending.transaction,
                        game,
                        &mut self.combat,
                        tq,
                        &mut dm,
                    )?;
                } else if let Some(pending) = self.pending_attacker_optional_costs.as_mut() {
                    if let Some(prompt) = pending.prompts.get(pending.answers.len()).cloned() {
                        let answer = match &prompt {
                            DecisionContext::Boolean(_) => {
                                self.pending_boolean.take().map(AttackCostAnswer::Boolean)
                            }
                            DecisionContext::SelectObjects(_) => {
                                self.pending_discard.take().map(AttackCostAnswer::Objects)
                            }
                            _ => {
                                return Err(GameLoopError::InvalidState(
                                    "unsupported optional attack-cost decision".to_string(),
                                ));
                            }
                        };
                        let Some(answer) = answer else {
                            return Ok(TurnAction::Decision(prompt));
                        };
                        pending.answers.push(answer);
                        if pending.answers.len() < pending.prompts.len() {
                            return Ok(TurnAction::Decision(
                                pending.prompts[pending.answers.len()].clone(),
                            ));
                        }
                    }

                    let pending = self
                        .pending_attacker_optional_costs
                        .take()
                        .expect("pending attacker optional costs should still exist");
                    if pending.required_mana_cost > 0 {
                        let mana_pending = PendingAttackerManaWindow {
                            transaction: pending.transaction,
                            optional_cost_answers: pending.answers,
                            declaration_source: pending.declaration_source,
                        };
                        if let Some(ctx) = attack_mana_ability_window_context(
                            game,
                            game.turn.active_player,
                            mana_pending.declaration_source,
                        ) {
                            self.pending_attacker_mana_window = Some(mana_pending);
                            return Ok(TurnAction::Decision(DecisionContext::SelectOptions(ctx)));
                        }
                        let mut dm =
                            QueuedAttackCostDecisionMaker::new(mana_pending.optional_cost_answers);
                        finish_attack_declaration_transaction(
                            mana_pending.transaction,
                            game,
                            &mut self.combat,
                            tq,
                            &mut dm,
                        )?;
                    } else {
                        let mut dm = QueuedAttackCostDecisionMaker::new(pending.answers);
                        finish_attack_declaration_transaction(
                            pending.transaction,
                            game,
                            &mut self.combat,
                            tq,
                            &mut dm,
                        )?;
                    }
                } else {
                    let declarations = self.pending_attackers.take().unwrap_or_default();
                    validate_declared_attacking_bands(
                        game,
                        &declarations,
                        self.pending_attacking_bands.as_deref().unwrap_or_default(),
                    )?;
                    let prompts =
                        preview_optional_attack_cost_prompts(game, &self.combat, &declarations)?;
                    let required_mana_cost =
                        preview_required_attack_mana_cost(game, &self.combat, &declarations)?;

                    if !prompts.is_empty() || required_mana_cost > 0 {
                        let declaration_source = declarations
                            .first()
                            .map(|declaration| declaration.creature)
                            .ok_or_else(|| {
                                GameLoopError::InvalidState(
                                    "attack costs exist without an attacker".to_string(),
                                )
                            })?;
                        let transaction = begin_attack_declaration_transaction(
                            game,
                            &self.combat,
                            tq,
                            &declarations,
                        )?;
                        if let Some(first_prompt) = prompts.first().cloned() {
                            self.pending_attacker_optional_costs =
                                Some(PendingAttackerOptionalCosts {
                                    transaction,
                                    prompts,
                                    answers: Vec::new(),
                                    required_mana_cost,
                                    declaration_source,
                                });
                            return Ok(TurnAction::Decision(first_prompt));
                        }

                        let pending = PendingAttackerManaWindow {
                            transaction,
                            optional_cost_answers: Vec::new(),
                            declaration_source,
                        };
                        if let Some(ctx) = attack_mana_ability_window_context(
                            game,
                            game.turn.active_player,
                            declaration_source,
                        ) {
                            self.pending_attacker_mana_window = Some(pending);
                            return Ok(TurnAction::Decision(DecisionContext::SelectOptions(ctx)));
                        }
                        let mut dm = QueuedAttackCostDecisionMaker::new(Vec::new());
                        finish_attack_declaration_transaction(
                            pending.transaction,
                            game,
                            &mut self.combat,
                            tq,
                            &mut dm,
                        )?;
                    } else {
                        let mut dm = QueuedAttackCostDecisionMaker::new(Vec::new());
                        apply_attacker_declarations_with_dm(
                            game,
                            &mut self.combat,
                            tq,
                            &declarations,
                            &mut dm,
                        )?;
                    }
                }
                if let Some(bands) = self.pending_attacking_bands.take() {
                    record_declared_attacking_bands(&mut self.combat, bands);
                }
                crate::game_loop::drain_pending_trigger_events(game, tq);
                put_triggers_on_stack(game, tq)?;

                // Also sync game.combat for anything that reads it
                game.combat = Some(self.combat.clone());

                self.state = TurnState::DeclareAttackersPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::DeclareAttackersPriority => {
                game.empty_mana_pools();
                self.sync_combat_from_game(game);
                self.state = finish_step(
                    game,
                    Step::DeclareAttackers,
                    TurnScheduleDestination::Step(Step::DeclareBlockers),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::DeclareBlockersCheck => {
                self.pending_blocker_mana_window = None;
                self.pending_option = None;
                if self.combat.attackers.is_empty() {
                    // Skip blockers and combat damage
                    self.state = finish_step(
                        game,
                        Step::DeclareBlockers,
                        TurnScheduleDestination::Step(Step::EndCombat),
                    );
                    Ok(TurnAction::Continue)
                } else {
                    self.remaining_defending_players =
                        attacked_defending_players_in_apnap_order(game, &self.combat);
                    self.state = TurnState::DeclareBlockersDecision;
                    Ok(TurnAction::Continue)
                }
            }

            TurnState::DeclareBlockersDecision => {
                game.turn.step = Some(Step::DeclareBlockers);

                let defending_player = self
                    .remaining_defending_players
                    .first()
                    .copied()
                    .unwrap_or(game.turn.active_player);
                self.defending_player = Some(defending_player);

                game.turn.priority_player = Some(defending_player);

                let ctx = get_declare_blockers_decision(game, &self.combat, defending_player);
                self.state = TurnState::DeclareBlockersApply;
                Ok(TurnAction::Decision(ctx))
            }

            TurnState::DeclareBlockersApply => {
                if let Some(mut pending) = self.pending_blocker_mana_window.take() {
                    if let Some(choice) = self.pending_option.take() {
                        let payer =
                            pending
                                .payers
                                .get(pending.next_payer)
                                .copied()
                                .ok_or_else(|| {
                                    GameLoopError::InvalidState(
                                        "blocking-cost mana window has no current payer"
                                            .to_string(),
                                    )
                                })?;
                        match apply_blocker_mana_ability_window_response(game, tq, payer, choice) {
                            Ok(true) => pending.next_payer += 1,
                            Ok(false) => {}
                            Err(error) => {
                                self.pending_blocker_mana_window = Some(pending);
                                return Err(error);
                            }
                        }
                    }
                    if let Some(context) = next_blocker_mana_window_context(game, &mut pending) {
                        self.pending_blocker_mana_window = Some(pending);
                        return Ok(TurnAction::Decision(DecisionContext::SelectOptions(
                            context,
                        )));
                    }
                    let defending_player =
                        pending.transaction.defending_player().ok_or_else(|| {
                            GameLoopError::InvalidState(
                                "blocking-cost transaction has no defending player".to_string(),
                            )
                        })?;
                    let mut decision_maker = AutoPassDecisionMaker;
                    if let Err(error) = finish_blocker_declaration_transaction(
                        pending.transaction,
                        game,
                        &mut self.combat,
                        tq,
                        &mut decision_maker,
                    ) {
                        self.state = TurnState::DeclareBlockersDecision;
                        return Err(error);
                    }
                    if self.remaining_defending_players.first().copied() == Some(defending_player) {
                        self.remaining_defending_players.remove(0);
                    }
                    self.defending_player = None;
                    if !self.remaining_defending_players.is_empty() {
                        self.state = TurnState::DeclareBlockersDecision;
                        return Ok(TurnAction::Continue);
                    }
                    put_triggers_on_stack(game, tq)?;
                    game.combat = Some(self.combat.clone());
                    game.reset_priority_for_new_window();
                    self.state = TurnState::DeclareBlockersPriority;
                    return Ok(TurnAction::RunPriority);
                }

                let (declarations, defending_player) =
                    self.pending_blockers.take().unwrap_or_else(|| {
                        (
                            Vec::new(),
                            self.defending_player.unwrap_or(game.turn.active_player),
                        )
                    });
                if self.defending_player != Some(defending_player) {
                    return Err(crate::decision::ResponseError::InvalidBlockers(
                        "blocker declaration was submitted for the wrong defending player"
                            .to_string(),
                    )
                    .into());
                }
                let mut decision_maker = AutoPassDecisionMaker;
                let transaction = match begin_blocker_declaration_transaction(
                    game,
                    &self.combat,
                    tq,
                    &declarations,
                    defending_player,
                    &mut decision_maker,
                ) {
                    Ok(transaction) => transaction,
                    Err(error) => {
                        self.state = TurnState::DeclareBlockersDecision;
                        return Err(error);
                    }
                };
                transaction.stage_proposed_combat_for_payment(game);
                let payers = transaction.mana_cost_payers();
                if !payers.is_empty() {
                    let declaration_source = transaction.declaration_source().ok_or_else(|| {
                        GameLoopError::InvalidState(
                            "blocking costs exist without a declaration source".to_string(),
                        )
                    })?;
                    let mut pending = PendingBlockerManaWindow {
                        transaction,
                        payers,
                        next_payer: 0,
                        declaration_source,
                    };
                    if let Some(context) = next_blocker_mana_window_context(game, &mut pending) {
                        self.pending_blocker_mana_window = Some(pending);
                        return Ok(TurnAction::Decision(DecisionContext::SelectOptions(
                            context,
                        )));
                    }
                    if let Err(error) = finish_blocker_declaration_transaction(
                        pending.transaction,
                        game,
                        &mut self.combat,
                        tq,
                        &mut decision_maker,
                    ) {
                        self.state = TurnState::DeclareBlockersDecision;
                        return Err(error);
                    }
                } else if let Err(error) = finish_blocker_declaration_transaction(
                    transaction,
                    game,
                    &mut self.combat,
                    tq,
                    &mut decision_maker,
                ) {
                    self.state = TurnState::DeclareBlockersDecision;
                    return Err(error);
                }
                if self.remaining_defending_players.first().copied() == Some(defending_player) {
                    self.remaining_defending_players.remove(0);
                }
                self.defending_player = None;
                if !self.remaining_defending_players.is_empty() {
                    self.state = TurnState::DeclareBlockersDecision;
                    return Ok(TurnAction::Continue);
                }
                put_triggers_on_stack(game, tq)?;

                // Sync game.combat
                game.combat = Some(self.combat.clone());
                game.reset_priority_for_new_window();

                self.state = TurnState::DeclareBlockersPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::DeclareBlockersPriority => {
                game.empty_mana_pools();
                self.sync_combat_from_game(game);

                // Check for first strike
                self.first_step_strikers = first_step_strikers(game, &self.combat);
                self.has_first_strike = !self.first_step_strikers.is_empty();

                if self.has_first_strike {
                    self.state = finish_step(
                        game,
                        Step::DeclareBlockers,
                        TurnScheduleDestination::CombatDamageFirstStrike,
                    );
                } else {
                    self.state = finish_step(
                        game,
                        Step::DeclareBlockers,
                        TurnScheduleDestination::CombatDamageRegular,
                    );
                }
                Ok(TurnAction::Continue)
            }

            TurnState::CombatDamageFirstStrike => {
                game.turn.step = Some(Step::CombatDamage);

                let events = try_execute_combat_damage_step(game, &self.combat, true)
                    .map_err(|error| GameLoopError::InvalidState(error.to_string()))?;
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
                self.sync_combat_from_game(game);
                self.state = finish_step(
                    game,
                    Step::CombatDamage,
                    TurnScheduleDestination::CombatDamageRegular,
                );
                Ok(TurnAction::Continue)
            }

            TurnState::CombatDamageRegular => {
                game.turn.step = Some(Step::CombatDamage);

                let events = try_execute_combat_damage_step_with_first_step_snapshot(
                    game,
                    &self.combat,
                    false,
                    &self.first_step_strikers,
                )
                .map_err(|error| GameLoopError::InvalidState(error.to_string()))?;
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
                self.sync_combat_from_game(game);
                self.state = finish_step(
                    game,
                    Step::CombatDamage,
                    TurnScheduleDestination::Step(Step::EndCombat),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::EndCombat => {
                game.turn.step = Some(Step::EndCombat);
                game.reset_priority_for_new_window();
                generate_and_queue_step_triggers(game, tq);
                game.combat = Some(self.combat.clone());

                self.state = TurnState::EndCombatPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::EndCombatPriority => {
                game.empty_mana_pools();
                crate::combat_state::end_combat(&mut self.combat);
                game.combat = Some(self.combat.clone());
                game.cleanup_effects_end_of_combat();
                self.state = finish_step_and_phase(
                    game,
                    Step::EndCombat,
                    Phase::Combat,
                    TurnScheduleDestination::Phase(Phase::NextMain),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::EndCombatPhaseSbas => {
                // CR 724.2c: perform SBAs to a fixed point without granting
                // priority or putting any resulting triggers on the stack.
                match self.apply_sbas_until_commander_choice(game, tq)? {
                    RunnerProgress::NeedsDecision(ctx) => Ok(TurnAction::Decision(ctx)),
                    RunnerProgress::Complete(()) => {
                        // CR 724.2d-e: end combat, expire its effects, and skip
                        // the end-of-combat step entirely. Temporarily naming
                        // that step lets mana-retention cleanup recognize the
                        // combat boundary without generating its trigger event.
                        game.turn.phase = Phase::Combat;
                        game.turn.step = Some(Step::EndCombat);
                        game.empty_mana_pools();
                        crate::combat_state::end_combat(&mut self.combat);
                        game.combat = Some(self.combat.clone());
                        game.cleanup_effects_end_of_combat();
                        game.turn.priority_player = None;
                        self.state = finish_phase(
                            game,
                            Phase::Combat,
                            TurnScheduleDestination::Phase(Phase::NextMain),
                        );
                        Ok(TurnAction::Continue)
                    }
                }
            }

            // ================================================================
            // Second Main Phase
            // ================================================================
            TurnState::NextMain => {
                if game
                    .turn_store
                    .skip_current_turn_main_phases
                    .contains(&game.turn.active_player)
                {
                    game.turn.phase = Phase::NextMain;
                    self.state = finish_phase(
                        game,
                        Phase::NextMain,
                        TurnScheduleDestination::Phase(Phase::Ending),
                    );
                    return Ok(TurnAction::Continue);
                }
                game.turn.phase = Phase::NextMain;
                game.turn.step = None;
                game.reset_priority_for_new_window();
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::NextMainPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::NextMainPriority => {
                game.empty_mana_pools();
                self.state = next_runner_state_after_phase(game, TurnState::EndStep);
                Ok(TurnAction::Continue)
            }

            // ================================================================
            // Ending Phase
            // ================================================================
            TurnState::EndStep => {
                game.turn.phase = Phase::Ending;
                game.turn.step = Some(Step::End);
                game.reset_priority_for_new_window();
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::EndStepPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::EndStepPriority => {
                game.empty_mana_pools();
                self.state = finish_step(
                    game,
                    Step::End,
                    TurnScheduleDestination::Step(Step::Cleanup),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::EndTurnSbas => {
                // CR 724.1c: check SBAs to a fixed point without granting
                // priority or putting the resulting triggers on the stack.
                match self.apply_sbas_until_commander_choice(game, tq)? {
                    RunnerProgress::NeedsDecision(ctx) => Ok(TurnAction::Decision(ctx)),
                    RunnerProgress::Complete(()) => {
                        // CR 724.1d-f: end combat, skip the end step, and enter
                        // the ordinary resumable cleanup procedure directly.
                        game.empty_mana_pools();
                        crate::combat_state::end_combat(&mut self.combat);
                        if let Some(combat) = game.combat.as_mut() {
                            crate::combat_state::end_combat(combat);
                        }
                        game.turn.phase = Phase::Ending;
                        game.turn.step = Some(Step::Cleanup);
                        game.turn.priority_player = None;
                        self.state = TurnState::CleanupDiscard;
                        Ok(TurnAction::Continue)
                    }
                }
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
                drain_pending_trigger_events(game, tq);
                let triggers_fired = !tq.is_empty();
                let sbas_happened = !check_state_based_actions(game).is_empty();

                if triggers_fired || sbas_happened {
                    match self.apply_sbas_until_commander_choice(game, tq)? {
                        RunnerProgress::Complete(()) => {}
                        RunnerProgress::NeedsDecision(ctx) => return Ok(TurnAction::Decision(ctx)),
                    }
                    put_triggers_on_stack(game, tq)?;
                    // CR 514.3a grants the active player priority after either
                    // state-based actions or waiting triggers, even if those
                    // actions left the stack empty.
                    game.reset_priority_for_new_window();
                    self.state = TurnState::CleanupRecursivePriority;
                    Ok(TurnAction::RunPriority)
                } else {
                    self.state = finish_step_and_phase(
                        game,
                        Step::Cleanup,
                        Phase::Ending,
                        TurnScheduleDestination::Complete,
                    );
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
        self.respond_attackers_with_bands(declarations, Vec::new());
    }

    /// Provide attacker declarations and the bands announced with them.
    pub fn respond_attackers_with_bands(
        &mut self,
        declarations: Vec<AttackerDeclaration>,
        bands: Vec<Vec<ObjectId>>,
    ) {
        self.pending_attackers = Some(declarations);
        self.pending_attacking_bands = Some(bands);
        self.pending_attacker_optional_costs = None;
        self.pending_attacker_mana_window = None;
        self.pending_option = None;
        self.pending_boolean = None;
        self.pending_draw_replacement = None;
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

    /// Provide a response to a runner-driven single-select options decision.
    pub fn respond_options(&mut self, option_indices: Vec<usize>) {
        self.pending_option = option_indices.first().copied();
    }

    /// Signal that the priority loop has completed.
    pub fn priority_done(&mut self) {
        // This is a no-op on the runner itself; the state transition
        // happens in advance() when the *Priority state is re-entered.
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    fn sync_combat_from_game(&mut self, game: &GameState) {
        if let Some(combat) = &game.combat {
            self.combat = combat.clone();
        }
    }

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
        if self.remaining_draw_players.is_empty() {
            let active_players = game.turn_players();
            if active_players
                .iter()
                .any(|player| game.player_skips_draw_step(*player))
            {
                game.reset_priority_for_new_window();
                return RunnerProgress::Complete(Vec::new());
            }
            self.remaining_draw_players = active_players;
            self.shared_draw_events.clear();
        }

        loop {
            let Some(active_player) = self.remaining_draw_players.first().copied() else {
                game.turn_store.tracked_draw_step_player = None;
                game.turn_store.cards_drawn_this_draw_step = 0;
                game.reset_priority_for_new_window();
                return RunnerProgress::Complete(std::mem::take(&mut self.shared_draw_events));
            };
            match self.execute_draw_step_for_player_with_choices(game, active_player) {
                RunnerProgress::NeedsDecision(ctx) => return RunnerProgress::NeedsDecision(ctx),
                RunnerProgress::Complete(events) => {
                    self.shared_draw_events.extend(events);
                    self.remaining_draw_players.remove(0);
                }
            }
        }
    }

    fn execute_draw_step_for_player_with_choices(
        &mut self,
        game: &mut GameState,
        active_player: PlayerId,
    ) -> RunnerProgress<Vec<crate::triggers::TriggerEvent>> {
        game.sync_draw_step_tracking();
        if let Some(pending) = self.pending_draw_reveal.take() {
            return self.finish_pending_draw_reveal_choices(game, pending);
        }
        if !game
            .player(active_player)
            .is_some_and(|player| player.is_in_game())
        {
            game.reset_priority_for_new_window();
            return RunnerProgress::Complete(Vec::new());
        }
        if game.player_skips_draw_step(active_player) {
            game.reset_priority_for_new_window();
            return RunnerProgress::Complete(Vec::new());
        }
        if game.should_skip_first_turn_draw(active_player) {
            game.reset_priority_for_new_window();
            return RunnerProgress::Complete(Vec::new());
        }

        let current_draws = game
            .turn_store
            .turn_history
            .cards_drawn_by_player(active_player);
        let is_first_draw = current_draws == 0;
        let can_draw = if !game.can_draw_extra_cards(active_player) {
            current_draws == 0
        } else {
            true
        };

        let mut drawn = Vec::new();
        if can_draw {
            use crate::events::processing::{
                TraitEventResult, process_event_with_chosen_replacement_trait_and_applied_effects,
                process_trait_event,
            };

            let mut final_draw_count = 1;
            let replacement_result = if self.pending_commander_choice.is_some() {
                None
            } else if let Some(pending) = self.pending_draw_replacement.take() {
                if pending.player != active_player {
                    self.pending_draw_replacement = Some(pending);
                    None
                } else if let Some(chosen_index) = self.pending_option.take() {
                    let chosen_effect = pending
                        .applicable_effects
                        .get(chosen_index)
                        .copied()
                        .or_else(|| pending.applicable_effects.first().copied());
                    chosen_effect.map(|chosen_effect| {
                        process_event_with_chosen_replacement_trait_and_applied_effects(
                            game,
                            pending.event,
                            chosen_effect,
                            &pending.applied_effects,
                            &pending.applied_effect_keys,
                        )
                    })
                } else {
                    let context = draw_replacement_choice_context(game, &pending);
                    self.pending_draw_replacement = Some(pending);
                    return RunnerProgress::NeedsDecision(context);
                }
            } else {
                game.update_replacement_effects();
                Some(process_trait_event(
                    game,
                    crate::events::Event::draw(active_player, 1, is_first_draw),
                ))
            };

            if let Some(result) = replacement_result {
                match result {
                    TraitEventResult::NeedsChoice {
                        player,
                        applicable_effects,
                        event,
                        applied_effects,
                        applied_effect_keys,
                    } => {
                        let pending = PendingDrawReplacementChoice {
                            player,
                            applicable_effects,
                            event: *event,
                            applied_effects,
                            applied_effect_keys,
                        };
                        let context = draw_replacement_choice_context(game, &pending);
                        self.pending_draw_replacement = Some(pending);
                        return RunnerProgress::NeedsDecision(context);
                    }
                    TraitEventResult::Replaced {
                        effects,
                        source,
                        controller,
                        ..
                    } => {
                        let mut dm = AutoPassDecisionMaker;
                        let mut ctx =
                            crate::effects::ExecutionContext::new(source, controller, &mut dm);
                        ctx.iteration.iterated_player = Some(active_player);
                        for effect in effects {
                            let _ = crate::effects::execute_effect(game, &effect, &mut ctx);
                        }
                        game.reset_priority_for_new_window();
                        return RunnerProgress::Complete(Vec::new());
                    }
                    TraitEventResult::Prevented => {
                        game.reset_priority_for_new_window();
                        return RunnerProgress::Complete(Vec::new());
                    }
                    TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                        final_draw_count = crate::events::downcast_event::<
                            crate::events::cards::DrawEvent,
                        >(event.inner())
                        .map(|draw| draw.count)
                        .unwrap_or(1);
                    }
                    TraitEventResult::NeedsInteraction { .. } => {}
                }
            }

            if final_draw_count != 1 {
                let mut dm = AutoPassDecisionMaker;
                drawn.extend(game.draw_cards_with_dm(
                    active_player,
                    final_draw_count as usize,
                    &mut dm,
                ));
            } else {
                match self.pending_commander_choice.take() {
                    Some(PendingCommanderChoice::DrawToHand { object_id }) => {
                        let send_to_command = self.pending_boolean.take().unwrap_or(false);
                        let final_zone = if send_to_command {
                            crate::zone::Zone::Command
                        } else {
                            crate::zone::Zone::Hand
                        };
                        if let Some(new_id) = game.move_object_by_effect(object_id, final_zone)
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
                                    .with_source_name(obj.name.to_string()),
                                );
                                    self.pending_commander_choice =
                                        Some(PendingCommanderChoice::DrawToHand {
                                            object_id: card_id,
                                        });
                                    return RunnerProgress::NeedsDecision(ctx);
                                }
                            } else if let Some(new_id) =
                                game.move_object_by_effect(card_id, crate::zone::Zone::Hand)
                            {
                                drawn.push(new_id);
                            }
                        } else {
                            game.record_empty_library_draw_attempt(active_player);
                        }
                    }
                }
            }
        }

        if !drawn.is_empty() {
            let draw_event_provenance = game
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::CardsDrawn);
            let candidates = crate::effects::cards::collect_automatic_draw_reveal_candidates(
                game,
                active_player,
                &drawn,
                current_draws,
            );
            return self.finish_pending_draw_reveal_choices(
                game,
                PendingDrawRevealChoice {
                    active_player,
                    drawn,
                    is_first_draw,
                    draw_event_provenance,
                    candidates,
                    next_candidate_index: 0,
                    reveal_events: Vec::new(),
                },
            );
        }

        game.reset_priority_for_new_window();
        RunnerProgress::Complete(Vec::new())
    }

    fn finish_pending_draw_reveal_choices(
        &mut self,
        game: &mut GameState,
        mut pending: PendingDrawRevealChoice,
    ) -> RunnerProgress<Vec<crate::triggers::TriggerEvent>> {
        use crate::events::other::CardsDrawnEvent;
        use crate::triggers::TriggerEvent;

        let (is_during_players_draw_step, cards_previously_drawn_this_draw_step) =
            game.draw_step_context_for_player(pending.active_player);

        while let Some(candidate) = pending
            .candidates
            .get(pending.next_candidate_index)
            .cloned()
        {
            let should_reveal = if candidate.optional {
                if let Some(answer) = self.pending_boolean.take() {
                    answer
                } else {
                    self.pending_draw_reveal = Some(pending);
                    return RunnerProgress::NeedsDecision(DecisionContext::Boolean(
                        crate::effects::cards::automatic_draw_reveal_boolean_context(&candidate),
                    ));
                }
            } else {
                true
            };

            if should_reveal {
                let mut dm = AutoPassDecisionMaker;
                pending.reveal_events.push(
                    crate::effects::cards::emit_automatic_draw_reveal_event(
                        game,
                        &mut dm,
                        &candidate,
                        pending.draw_event_provenance,
                    ),
                );
            }
            pending.next_candidate_index += 1;
        }

        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new_with_step_context(
                pending.active_player,
                pending.drawn,
                pending.is_first_draw,
                is_during_players_draw_step,
                cards_previously_drawn_this_draw_step,
            ),
            pending.draw_event_provenance,
        );
        if let Some(drawn_event) = event.downcast::<CardsDrawnEvent>() {
            game.record_cards_drawn_in_current_draw_step(
                pending.active_player,
                drawn_event.amount(),
            );
        }
        game.stage_turn_history_event(&event);
        let mut draw_events = vec![event];
        for reveal_event in pending.reveal_events {
            game.stage_turn_history_event(&reveal_event);
            draw_events.push(reveal_event);
        }

        game.reset_priority_for_new_window();
        RunnerProgress::Complete(draw_events)
    }

    fn apply_sbas_until_commander_choice(
        &mut self,
        game: &mut GameState,
        tq: &mut TriggerQueue,
    ) -> Result<RunnerProgress<()>, GameLoopError> {
        use crate::rules::state_based::{
            StateBasedAction, StateBasedActionContext, apply_sector_designation_choices_from_group,
            apply_state_based_actions_from_actions_with, check_state_based_actions_with_context,
            legend_rule_specs_from_actions,
        };

        loop {
            // Every applied SBA can change which static effects exist. Refresh
            // at the fixed-point boundary; this is a no-op while state is clean.
            game.refresh_continuous_state();
            let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
            let all_effects = view.effects_arc();
            let context = StateBasedActionContext::from_trigger_queue(tq);
            let actions = check_state_based_actions_with_context(game, &view, &context);
            drop(view);
            if actions.is_empty() {
                game.clear_empty_library_draw_attempts_since_sba();
                self.pending_boolean = None;
                self.pending_commander_choice = None;
                self.pending_draw_replacement = None;
                self.pending_legend_choice = None;
                self.pending_sector_designations = None;
                return Ok(RunnerProgress::Complete(()));
            }

            let sector_action = actions.iter().find_map(|action| match action {
                StateBasedAction::SectorDesignationChoices { source, creatures } => {
                    Some((*source, creatures.clone()))
                }
                _ => None,
            });
            if let Some((source, creatures)) = sector_action {
                let mut pending = match self.pending_sector_designations.take() {
                    Some(pending) if pending.source == source && pending.creatures == creatures => {
                        pending
                    }
                    Some(_) | None => {
                        self.pending_option = None;
                        PendingSectorDesignationChoices {
                            source,
                            creatures,
                            choices: Vec::new(),
                        }
                    }
                };

                if pending.choices.len() < pending.creatures.len()
                    && let Some(index) = self.pending_option.take()
                {
                    pending.choices.push(
                        crate::marker::SectorDesignation::from_option_index(index)
                            .unwrap_or(crate::marker::SectorDesignation::Alpha),
                    );
                }

                if pending.choices.len() == pending.creatures.len() {
                    apply_sector_designation_choices_from_group(
                        game,
                        pending.source,
                        &pending.creatures,
                        &pending.choices,
                    );
                    crate::game_loop::drain_pending_trigger_events(game, tq);
                    continue;
                }

                let (player, creature) = pending.creatures[pending.choices.len()];
                let name = game
                    .object(creature)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| "this creature".to_string());
                let options = crate::marker::SectorDesignation::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(index, sector)| {
                        crate::decisions::context::SelectableOption::new(
                            index,
                            sector.description(),
                        )
                    })
                    .collect();
                let context = crate::decisions::context::SelectOptionsContext::new(
                    player,
                    Some(pending.source),
                    format!("Choose a sector for {name}"),
                    options,
                    1,
                    1,
                );
                self.pending_sector_designations = Some(pending);
                return Ok(RunnerProgress::NeedsDecision(
                    DecisionContext::SelectOptions(context),
                ));
            } else if self.pending_sector_designations.take().is_some() {
                self.pending_option = None;
            }

            // Handle one legend-rule violation per pass: applying a keep choice
            // can change which violations remain, so re-check SBAs before
            // prompting for the next one. Violations arrive in APNAP order.
            let legend_specs = legend_rule_specs_from_actions(&actions);
            if let Some((player, spec)) = legend_specs.into_iter().next() {
                use crate::decisions::DecisionSpec;
                if let Some(pending) = self.pending_legend_choice.take() {
                    // Any queued object selection belongs to the legend prompt
                    // we paused on; consume it with the marker so it can never
                    // leak into a later object-selection prompt.
                    let answer = self.pending_discard.take();
                    if pending.player == player && pending.legends == spec.legends {
                        let keep_id = answer
                            .into_iter()
                            .flatten()
                            .find(|id| spec.legends.contains(id))
                            .unwrap_or_else(|| {
                                spec.default_response(crate::decision::FallbackStrategy::Decline)
                            });
                        crate::rules::state_based::apply_legend_rule_choice_from_group(
                            game,
                            keep_id,
                            &spec.legends,
                        );
                        crate::game_loop::drain_pending_trigger_events(game, tq);
                        continue;
                    }
                }
                // No matching answer (or the board shifted since we paused):
                // surface the keep choice to the violating permanents' controller.
                let ctx = spec.build_context(player, None, game);
                self.pending_legend_choice = Some(PendingLegendRuleChoice {
                    player,
                    legends: spec.legends,
                });
                return Ok(RunnerProgress::NeedsDecision(ctx));
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
                    all_effects.as_slice(),
                    &mut auto_dm,
                );
                crate::game_loop::drain_pending_trigger_events(game, tq);
                if !applied {
                    self.pending_boolean = None;
                    self.pending_commander_choice = None;
                    self.pending_draw_replacement = None;
                    self.pending_legend_choice = None;
                    self.pending_sector_designations = None;
                    return Ok(RunnerProgress::Complete(()));
                }
                continue;
            }

            let Some(obj_id) = commander_returns.first().copied() else {
                game.clear_empty_library_draw_attempts_since_sba();
                self.pending_boolean = None;
                self.pending_commander_choice = None;
                self.pending_draw_replacement = None;
                self.pending_legend_choice = None;
                self.pending_sector_designations = None;
                return Ok(RunnerProgress::Complete(()));
            };

            match self.pending_commander_choice.take() {
                Some(PendingCommanderChoice::StateBasedReturn { object_id })
                    if object_id == obj_id =>
                {
                    let send_to_command = self.pending_boolean.take().unwrap_or(false);
                    if send_to_command {
                        game.move_object_by_effect(obj_id, crate::zone::Zone::Command);
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
                    .with_source_name(obj.name.to_string()),
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

/// Snapshot every combatant that had first strike or double strike as the
/// first combat-damage step began (CR 510.4).
fn first_step_strikers(
    game: &GameState,
    combat: &CombatState,
) -> std::collections::HashSet<ObjectId> {
    combat
        .attackers
        .iter()
        .map(|info| info.creature)
        .chain(combat.blockers.values().flatten().copied())
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| deals_first_strike_damage_with_game(object, game))
        })
        .collect()
}

/// CR 802.4: every player actually being attacked declares blockers in APNAP order.
fn attacked_defending_players_in_apnap_order(
    game: &GameState,
    combat: &CombatState,
) -> Vec<PlayerId> {
    let attacked = combat
        .attackers
        .iter()
        .filter_map(|attacker| {
            crate::combat_state::defending_player_for_attack_target(game, &attacker.target)
        })
        .collect::<std::collections::HashSet<_>>();

    if game.shared_team_turns_enabled() {
        let attacked_teams = attacked
            .iter()
            .filter_map(|player| game.team_index_for(*player))
            .collect::<std::collections::HashSet<_>>();
        return game
            .team_apnap_player_order()
            .into_iter()
            .filter_map(|player| {
                let team = game.team_index_for(player)?;
                attacked_teams
                    .contains(&team)
                    .then(|| game.primary_player_for_team(team))
                    .flatten()
            })
            .fold(Vec::new(), |mut players, player| {
                if !players.contains(&player) {
                    players.push(player);
                }
                players
            });
    }

    let turn_order = &game.turn_store.turn_order;
    let mut ordered = Vec::new();
    if !turn_order.is_empty() {
        let active_position = turn_order
            .iter()
            .position(|player| *player == game.turn.active_player)
            .unwrap_or(0);
        for offset in 0..turn_order.len() {
            let player = turn_order[(active_position + offset) % turn_order.len()];
            if attacked.contains(&player)
                && game
                    .player(player)
                    .is_some_and(|player| player.is_in_game())
            {
                ordered.push(player);
            }
        }
    }
    for player in game
        .players
        .iter()
        .filter(|player| player.is_in_game())
        .map(|player| player.id)
    {
        if attacked.contains(&player) && !ordered.contains(&player) {
            ordered.push(player);
        }
    }
    ordered
}

fn runner_state_for_destination(destination: TurnScheduleDestination) -> TurnState {
    match destination {
        TurnScheduleDestination::Step(Step::Untap) => TurnState::Untap,
        TurnScheduleDestination::Step(Step::Upkeep) => TurnState::Upkeep,
        TurnScheduleDestination::Step(Step::Draw) => TurnState::Draw,
        TurnScheduleDestination::Step(Step::BeginCombat) => TurnState::BeginCombat,
        TurnScheduleDestination::Step(Step::DeclareAttackers) => {
            TurnState::DeclareAttackersDecision
        }
        TurnScheduleDestination::Step(Step::DeclareBlockers) => TurnState::DeclareBlockersCheck,
        TurnScheduleDestination::Step(Step::CombatDamage)
        | TurnScheduleDestination::CombatDamageRegular => TurnState::CombatDamageRegular,
        TurnScheduleDestination::CombatDamageFirstStrike => TurnState::CombatDamageFirstStrike,
        TurnScheduleDestination::Step(Step::EndCombat) => TurnState::EndCombat,
        TurnScheduleDestination::Step(Step::End) => TurnState::EndStep,
        TurnScheduleDestination::Step(Step::Cleanup) => TurnState::CleanupDiscard,
        TurnScheduleDestination::Phase(Phase::Beginning) => TurnState::Untap,
        TurnScheduleDestination::Phase(Phase::FirstMain) => TurnState::FirstMain,
        TurnScheduleDestination::Phase(Phase::Combat) => TurnState::BeginCombat,
        TurnScheduleDestination::Phase(Phase::NextMain) => TurnState::NextMain,
        TurnScheduleDestination::Phase(Phase::Ending) => TurnState::EndStep,
        TurnScheduleDestination::ResumePhaseSchedule => {
            unreachable!("phase-schedule continuations are resolved before state conversion")
        }
        TurnScheduleDestination::Complete => TurnState::Complete,
    }
}

fn prepend_scheduled_steps(game: &mut GameState, mut steps: Vec<ScheduledStep>) {
    if steps.is_empty() {
        return;
    }
    steps.append(&mut game.turn_store.pending_added_steps);
    game.turn_store.pending_added_steps = steps;
}

fn activate_next_scheduled_step(game: &mut GameState) -> TurnState {
    loop {
        let Some(next) = game.turn_store.pending_added_steps.first().copied() else {
            game.turn_store.active_added_step = None;
            let continuation = game
                .turn_store
                .added_step_continuation
                .take()
                .unwrap_or(TurnScheduleDestination::Complete);
            return resolve_schedule_destination(game, continuation);
        };
        game.turn_store.pending_added_steps.remove(0);

        let before = game.take_added_steps(AddedStepPlacement::BeforeStep(next.step));
        if !before.is_empty() {
            let mut sequence = before;
            sequence.push(next);
            prepend_scheduled_steps(game, sequence);
            continue;
        }

        game.turn_store.active_added_step = Some(next);
        game.turn.phase = next.phase;
        game.turn.step = Some(next.step);
        return runner_state_for_destination(TurnScheduleDestination::Step(next.step));
    }
}

fn start_scheduled_steps(
    game: &mut GameState,
    steps: Vec<ScheduledStep>,
    continuation: TurnScheduleDestination,
) -> TurnState {
    game.turn_store.pending_added_steps = steps;
    game.turn_store.active_added_step = None;
    game.turn_store.added_step_continuation = Some(continuation);
    activate_next_scheduled_step(game)
}

fn resolve_schedule_destination(
    game: &mut GameState,
    destination: TurnScheduleDestination,
) -> TurnState {
    if matches!(destination, TurnScheduleDestination::ResumePhaseSchedule) {
        return resume_phase_schedule(game);
    }
    let first_step = match destination {
        TurnScheduleDestination::Step(step) => Some(step),
        TurnScheduleDestination::Phase(phase) => crate::turn::first_step_of_phase(phase),
        TurnScheduleDestination::CombatDamageFirstStrike
        | TurnScheduleDestination::CombatDamageRegular => Some(Step::CombatDamage),
        _ => None,
    };
    if let Some(step) = first_step {
        let before = game.take_added_steps(AddedStepPlacement::BeforeStep(step));
        if !before.is_empty() {
            return start_scheduled_steps(game, before, destination);
        }
    }
    runner_state_for_destination(destination)
}

fn prepare_phase_schedule(game: &mut GameState, normal_next: TurnScheduleDestination) {
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

fn resume_phase_schedule(game: &mut GameState) -> TurnState {
    if let Some((phase, only_step)) = game.pop_additional_phase() {
        if let Some(step) = only_step {
            return start_scheduled_steps(
                game,
                vec![ScheduledStep {
                    phase,
                    step,
                    isolated_phase: true,
                }],
                TurnScheduleDestination::ResumePhaseSchedule,
            );
        }
        return resolve_schedule_destination(game, TurnScheduleDestination::Phase(phase));
    }
    let continuation = game
        .turn_store
        .phase_schedule_continuation
        .take()
        .unwrap_or(TurnScheduleDestination::Complete);
    resolve_schedule_destination(game, continuation)
}

fn begin_phase_schedule(
    game: &mut GameState,
    phase: Phase,
    normal_next: TurnScheduleDestination,
) -> TurnState {
    game.queue_added_step_phases_after(phase);
    prepare_phase_schedule(game, normal_next);
    resume_phase_schedule(game)
}

fn finish_step(
    game: &mut GameState,
    step: Step,
    normal_next: TurnScheduleDestination,
) -> TurnState {
    let additions = game.take_added_steps(AddedStepPlacement::AfterStep(step));
    let active = game.turn_store.active_added_step.take();
    if let Some(scheduled) = active {
        if scheduled.isolated_phase {
            game.queue_added_step_phases_after(scheduled.phase);
        }
        prepend_scheduled_steps(game, additions);
        return activate_next_scheduled_step(game);
    }
    if additions.is_empty() {
        resolve_schedule_destination(game, normal_next)
    } else {
        start_scheduled_steps(game, additions, normal_next)
    }
}

fn finish_step_and_phase(
    game: &mut GameState,
    step: Step,
    phase: Phase,
    normal_next: TurnScheduleDestination,
) -> TurnState {
    let additions = game.take_added_steps(AddedStepPlacement::AfterStep(step));
    let active = game.turn_store.active_added_step.take();
    if active.is_none() || active.is_some_and(|scheduled| scheduled.isolated_phase) {
        game.queue_added_step_phases_after(phase);
    }

    if active.is_some() {
        prepend_scheduled_steps(game, additions);
        return activate_next_scheduled_step(game);
    }

    prepare_phase_schedule(game, normal_next);
    if additions.is_empty() {
        resume_phase_schedule(game)
    } else {
        start_scheduled_steps(
            game,
            additions,
            TurnScheduleDestination::ResumePhaseSchedule,
        )
    }
}

fn finish_phase(
    game: &mut GameState,
    phase: Phase,
    normal_next: TurnScheduleDestination,
) -> TurnState {
    begin_phase_schedule(game, phase, normal_next)
}

fn next_runner_state_after_phase(game: &mut GameState, normal_next: TurnState) -> TurnState {
    if matches!(game.turn.phase, Phase::Combat) {
        game.cleanup_effects_end_of_combat();
    }

    let normal_destination = match normal_next {
        TurnState::FirstMain => TurnScheduleDestination::Phase(Phase::FirstMain),
        TurnState::BeginCombat => TurnScheduleDestination::Phase(Phase::Combat),
        TurnState::NextMain => TurnScheduleDestination::Phase(Phase::NextMain),
        TurnState::EndStep => TurnScheduleDestination::Phase(Phase::Ending),
        TurnState::Complete => TurnScheduleDestination::Complete,
        _ => return normal_next,
    };
    let phase = game.turn.phase;
    finish_phase(game, phase, normal_destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::CardDefinitionBuilder;
    use crate::combat_state::{AttackTarget, AttackerInfo};
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::tag::TagKey;
    use crate::triggers::TriggerQueue;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_battlefield_creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let object_id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(object_id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object = Object::from_card(object_id, &card, owner, Zone::Battlefield);
        game.add_object(object);
        object_id
    }

    fn create_mountain(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Mountain")
            .card_types(vec![CardType::Land])
            .build();
        let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
        game.object_mut(id)
            .expect("Mountain exists")
            .abilities_mut()
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    effects: crate::resolution::ResolutionProgram::default(),
                    choices: vec![],
                    timing: ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: Some(vec![crate::mana::ManaSymbol::Red]),
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: false,
                }),
                functional_zones: vec![Zone::Battlefield],
            });
        id
    }

    fn create_optional_untap_artifact(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
    ) -> ObjectId {
        let object_id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(object_id.0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        let mut object = Object::from_card(object_id, &card, owner, Zone::Battlefield);
        object.abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::may_choose_not_to_untap_during_untap_step(
                "this artifact",
            ),
        ));
        game.add_object(object);
        object_id
    }

    #[test]
    fn turn_runner_yields_each_may_choose_not_to_untap_decision() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let stays_tapped = create_optional_untap_artifact(&mut game, alice, "Sleeping Relic");
        let untaps = create_optional_untap_artifact(&mut game, alice, "Waking Relic");
        game.tap(stays_tapped);
        game.tap(untaps);
        let mut runner = TurnRunner::new();
        let mut tq = TriggerQueue::new();

        let first = runner.advance(&mut game, &mut tq).expect("first prompt");
        let TurnAction::Decision(DecisionContext::Boolean(first)) = first else {
            panic!("expected optional untap decision");
        };
        assert_eq!(first.player, alice);
        assert_eq!(first.source, Some(stays_tapped));
        assert!(game.is_tapped(stays_tapped));
        assert!(game.is_tapped(untaps));

        runner.respond_boolean(false);
        let second = runner.advance(&mut game, &mut tq).expect("second prompt");
        let TurnAction::Decision(DecisionContext::Boolean(second)) = second else {
            panic!("expected second optional untap decision");
        };
        assert_eq!(second.source, Some(untaps));

        runner.respond_boolean(true);
        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("finish untap"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Upkeep));
        assert!(game.is_tapped(stays_tapped));
        assert!(!game.is_tapped(untaps));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    fn gibbering_descent_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Gibbering Descent")
            .card_types(vec![CardType::Enchantment])
            .parse_text(
                "At the beginning of each player's upkeep, that player loses 1 life and discards a card.\n\
                 Hellbent — Skip your upkeep step if you have no cards in hand.\n\
                 Madness {2}{B}{B} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)",
            )
            .expect("Gibbering Descent should parse for turn-runner tests")
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    fn run_gibbering_descent_upkeep_with_hand_size(
        hand_size: usize,
    ) -> (TurnAction, TurnRunner, TriggerQueue) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        let gibbering_descent = gibbering_descent_definition();
        game.create_object_from_definition(&gibbering_descent, alice, Zone::Battlefield);
        for idx in 0..hand_size {
            let card = CardBuilder::new(CardId::new(), &format!("Hand Card {idx}"))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Hand);
        }

        let mut runner = TurnRunner::from_state_for_sync(TurnState::Upkeep);
        let mut tq = TriggerQueue::new();
        let action = runner
            .advance(&mut game, &mut tq)
            .expect("Gibbering Descent upkeep should advance");
        (action, runner, tq)
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn gibbering_descent_skips_your_upkeep_when_you_have_no_cards_in_hand() {
        let (action, runner, tq) = run_gibbering_descent_upkeep_with_hand_size(0);

        assert!(matches!(action, TurnAction::Continue));
        assert!(matches!(runner.state(), TurnState::Draw));
        assert!(
            tq.is_empty(),
            "skipping the upkeep step should not queue Gibbering Descent's upkeep trigger"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn gibbering_descent_keeps_your_upkeep_when_you_have_cards_in_hand() {
        let (action, runner, tq) = run_gibbering_descent_upkeep_with_hand_size(1);

        assert!(matches!(action, TurnAction::RunPriority));
        assert!(matches!(runner.state(), TurnState::UpkeepPriority));
        assert_eq!(
            tq.entries.len(),
            1,
            "not satisfying hellbent should queue the normal upkeep trigger"
        );
    }

    #[test]
    fn draw_step_can_replace_draw_with_dredge() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);

        let dredger = CardDefinitionBuilder::new(CardId::new(), "Dredge Probe")
            .card_types(vec![CardType::Creature])
            .with_ability(
                crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::dredge(2),
                )
                .in_zones(vec![Zone::Graveyard]),
            )
            .build();
        let dredger_id = game.create_object_from_definition(&dredger, alice, Zone::Graveyard);
        for idx in 0..2 {
            let card = CardBuilder::new(CardId::new(), &format!("Library Creature {idx}"))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let mut runner = TurnRunner::new();
        runner.state = TurnState::Draw;
        let mut tq = TriggerQueue::new();

        let action = runner
            .advance(&mut game, &mut tq)
            .expect("draw step should request dredge choice");
        let TurnAction::Decision(DecisionContext::SelectOptions(ctx)) = action else {
            panic!("expected shared replacement choice, got {action:?}");
        };
        assert_eq!(ctx.player, alice);
        let dredge_index = ctx
            .options
            .iter()
            .find(|option| {
                option.object_id == Some(dredger_id)
                    && !option.description.starts_with("Do not apply")
            })
            .map(|option| option.index)
            .expect("the typed dredge replacement should be selectable");
        assert!(ctx.options.iter().any(|option| {
            option.object_id == Some(dredger_id) && option.description.starts_with("Do not apply")
        }));

        runner.respond_options(vec![dredge_index]);
        let action = runner
            .advance(&mut game, &mut tq)
            .expect("accepted dredge should finish the draw step");
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(game.player(alice).expect("alice").hand.len(), 1);
        assert_eq!(game.player(alice).expect("alice").graveyard.len(), 2);
        assert!(game.player(alice).expect("alice").library.is_empty());
        assert_eq!(
            game.current_name(game.player(alice).expect("alice").hand[0])
                .as_deref(),
            Some("Dredge Probe")
        );
    }

    #[test]
    fn duplicate_skip_next_draw_step_effects_are_consumed_one_per_extra_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        for index in 0..2 {
            let card = CardBuilder::new(CardId::new(), &format!("Skip Probe {index}"))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        // Two independently created effects must each replace one future draw
        // step. A set collapses them and incorrectly consumes both at once.
        game.skip_next_step(alice, Step::Draw);
        game.skip_next_step(alice, Step::Draw);
        game.turn_store.extra_turns.push(alice);

        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::from_state_for_sync(TurnState::Draw);
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("first skipped draw"),
            TurnAction::Continue
        ));
        assert!(game.player(alice).expect("alice").hand.is_empty());

        game.next_turn();
        assert_eq!(game.turn.active_player, alice);
        let mut runner = TurnRunner::from_state_for_sync(TurnState::Draw);
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("second independently skipped draw"),
            TurnAction::Continue
        ));
        assert!(
            game.player(alice).expect("alice").hand.is_empty(),
            "the second skip must survive for the extra turn's draw step"
        );
    }

    #[test]
    fn draw_step_declining_one_dredge_still_offers_another() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);

        let dredger = |name: &str, amount| {
            CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .with_ability(
                    crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::dredge(amount),
                    )
                    .in_zones(vec![Zone::Graveyard]),
                )
                .build()
        };
        let first_id = game.create_object_from_definition(
            &dredger("First Dredger", 2),
            alice,
            Zone::Graveyard,
        );
        let second_id = game.create_object_from_definition(
            &dredger("Second Dredger", 3),
            alice,
            Zone::Graveyard,
        );
        for idx in 0..4 {
            let card = CardBuilder::new(CardId::new(), &format!("Library Card {idx}"))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let mut runner = TurnRunner::from_state_for_sync(TurnState::Draw);
        let mut tq = TriggerQueue::new();
        let first_action = runner.advance(&mut game, &mut tq).expect("first choice");
        let TurnAction::Decision(DecisionContext::SelectOptions(first_ctx)) = first_action else {
            panic!("expected all eligible draw replacements");
        };
        assert_eq!(first_ctx.options.len(), 4);
        let decline_first = first_ctx
            .options
            .iter()
            .find(|option| {
                option.object_id == Some(first_id) && option.description.starts_with("Do not apply")
            })
            .map(|option| option.index)
            .expect("first dredge must have an explicit decline choice");

        runner.respond_options(vec![decline_first]);
        let second_action = runner.advance(&mut game, &mut tq).expect("second choice");
        let TurnAction::Decision(DecisionContext::SelectOptions(second_ctx)) = second_action else {
            panic!("declining one dredge must leave the other pair available");
        };
        assert_eq!(second_ctx.options.len(), 2);
        assert!(
            second_ctx
                .options
                .iter()
                .all(|option| option.object_id == Some(second_id))
        );
        let choose_second = second_ctx
            .options
            .iter()
            .find(|option| !option.description.starts_with("Do not apply"))
            .map(|option| option.index)
            .expect("second dredge should remain selectable");

        runner.respond_options(vec![choose_second]);
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("finish draw step"),
            TurnAction::RunPriority
        ));
        let player = game.player(alice).expect("alice");
        assert_eq!(player.library.len(), 1);
        assert_eq!(player.graveyard.len(), 4);
        assert!(player.graveyard.contains(&first_id));
        assert_eq!(
            game.current_name(player.hand[0]).as_deref(),
            Some("Second Dredger")
        );
    }

    #[test]
    fn draw_step_dredge_is_ineligible_without_enough_library_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        let dredger = CardDefinitionBuilder::new(CardId::new(), "Too Large Dredger")
            .card_types(vec![CardType::Creature])
            .with_ability(
                crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::dredge(3),
                )
                .in_zones(vec![Zone::Graveyard]),
            )
            .build();
        let dredger_id = game.create_object_from_definition(&dredger, alice, Zone::Graveyard);
        for idx in 0..2 {
            let card = CardBuilder::new(CardId::new(), &format!("Library Card {idx}"))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let mut runner = TurnRunner::from_state_for_sync(TurnState::Draw);
        let mut tq = TriggerQueue::new();
        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("ordinary draw"),
            TurnAction::RunPriority
        ));
        let player = game.player(alice).expect("alice");
        assert_eq!(player.hand.len(), 1);
        assert_eq!(player.library.len(), 1);
        assert!(player.graveyard.contains(&dredger_id));
    }

    #[test]
    fn draw_step_records_empty_library_attempt_for_sbas() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        assert!(game.player(alice).expect("Alice exists").library.is_empty());
        let mut runner = TurnRunner::from_state_for_sync(TurnState::Draw);
        let mut tq = TriggerQueue::new();

        let action = runner
            .advance(&mut game, &mut tq)
            .expect("draw step should advance to priority");

        assert!(matches!(action, TurnAction::RunPriority));
        assert!(
            game.player(alice)
                .expect("Alice exists")
                .attempted_draw_from_empty_library
        );
        assert!(
            crate::rules::state_based::check_state_based_actions(&game)
                .iter()
                .any(|action| matches!(
                    action,
                    crate::rules::state_based::StateBasedAction::PlayerLoses {
                        player,
                        reason: crate::rules::state_based::LoseReason::DrewFromEmptyLibrary,
                    } if *player == alice
                ))
        );
    }

    #[test]
    fn draw_step_empty_library_win_replacement_preempts_the_failed_draw() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);

        let laboratory_maniac = CardBuilder::new(CardId::new(), "Laboratory Maniac")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&laboratory_maniac, alice, Zone::Battlefield);
        game.object_mut(source)
            .expect("Laboratory Maniac exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::conditional_draw_replacement(
                    crate::effect::Condition::ValueComparison {
                        left: crate::effect::Value::CardsInLibrary(
                            crate::target::PlayerFilter::You,
                        ),
                        operator: crate::effect::ValueComparisonOperator::Equal,
                        right: crate::effect::Value::Fixed(0),
                    },
                    vec![crate::effect::Effect::win_the_game()],
                    false,
                    "If you would draw a card while your library has no cards in it, you win the game instead.",
                ),
            ));

        let mut runner = TurnRunner::from_state_for_sync(TurnState::Draw);
        let mut tq = TriggerQueue::new();
        let action = runner
            .advance(&mut game, &mut tq)
            .expect("the draw-step replacement should resolve");

        assert!(matches!(action, TurnAction::RunPriority));
        assert!(
            !game
                .player(alice)
                .expect("alice exists")
                .attempted_draw_from_empty_library,
            "the replaced draw must not create an empty-library loss observation"
        );
        assert!(!game.player(bob).expect("bob exists").is_in_game());
    }

    #[test]
    fn forecast_reveal_ends_exactly_when_the_draw_step_begins() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);

        let forecast = CardBuilder::new(CardId::new(), "Forecast Reveal Probe").build();
        let forecast_id = game.create_object_from_card(&forecast, alice, Zone::Hand);
        assert!(game.reveal_hand_card_until_upkeep_ends(forecast_id));
        let draw_card = CardBuilder::new(CardId::new(), "Draw Step Card").build();
        game.create_object_from_card(&draw_card, alice, Zone::Library);

        let mut runner = TurnRunner::from_state_for_sync(TurnState::UpkeepPriority);
        let mut tq = TriggerQueue::new();
        let action = runner.advance(&mut game, &mut tq).expect("upkeep ends");
        assert!(matches!(action, TurnAction::Continue));
        assert!(
            game.is_hand_card_revealed_until_upkeep_ends(forecast_id),
            "the card remains revealed until the next step actually begins"
        );

        let action = runner.advance(&mut game, &mut tq).expect("draw begins");
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(game.turn.step, Some(Step::Draw));
        assert!(!game.is_hand_card_revealed_until_upkeep_ends(forecast_id));
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
    fn test_declare_blockers_priority_starts_with_active_player() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_battlefield_creature(&mut game, alice, "Priority Probe");

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareBlockers);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(bob);

        runner.state = TurnState::DeclareBlockersApply;
        runner.combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        runner.pending_blockers = Some((Vec::new(), bob));
        runner.defending_player = Some(bob);
        game.combat = Some(runner.combat.clone());

        let action = runner.advance(&mut game, &mut tq).unwrap();

        assert!(matches!(action, TurnAction::RunPriority));
        assert!(matches!(runner.state(), TurnState::DeclareBlockersPriority));
        assert_eq!(game.turn.priority_player, Some(alice));
    }

    #[test]
    fn multiplayer_defenders_declare_blockers_in_apnap_and_keep_every_block() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let attacks_bob = create_battlefield_creature(&mut game, alice, "Attacks Bob");
        let attacks_charlie = create_battlefield_creature(&mut game, alice, "Attacks Charlie");
        let bob_blocker = create_battlefield_creature(&mut game, bob, "Bob Blocker");
        let charlie_blocker = create_battlefield_creature(&mut game, charlie, "Charlie Blocker");
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareBlockers);

        let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareBlockersCheck);
        runner.combat.attackers = vec![
            AttackerInfo {
                creature: attacks_charlie,
                target: AttackTarget::Player(charlie),
            },
            AttackerInfo {
                creature: attacks_bob,
                target: AttackTarget::Player(bob),
            },
        ];
        let mut tq = TriggerQueue::new();

        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("start blockers"),
            TurnAction::Continue
        ));
        let TurnAction::Decision(DecisionContext::Blockers(bob_context)) = runner
            .advance(&mut game, &mut tq)
            .expect("Bob should declare first")
        else {
            panic!("expected Bob's blocker decision");
        };
        assert_eq!(bob_context.player, bob);
        assert_eq!(
            bob_context
                .blocker_options
                .iter()
                .map(|option| option.attacker)
                .collect::<Vec<_>>(),
            vec![attacks_bob]
        );
        runner.respond_blockers(
            vec![BlockerDeclaration {
                blocker: bob_blocker,
                blocking: attacks_bob,
            }],
            bob,
        );
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("collect Bob's blocks"),
            TurnAction::Continue
        ));
        assert_eq!(
            runner.combat.blockers.get(&attacks_bob),
            Some(&vec![bob_blocker]),
            "Bob's complete CR 509.1 declaration must publish before Charlie starts"
        );
        assert!(runner.combat.blockers.get(&attacks_charlie).is_none());

        let TurnAction::Decision(DecisionContext::Blockers(charlie_context)) = runner
            .advance(&mut game, &mut tq)
            .expect("Charlie should declare second")
        else {
            panic!("expected Charlie's blocker decision");
        };
        assert_eq!(charlie_context.player, charlie);
        assert_eq!(
            charlie_context
                .blocker_options
                .iter()
                .map(|option| option.attacker)
                .collect::<Vec<_>>(),
            vec![attacks_charlie]
        );
        runner.respond_blockers(
            vec![BlockerDeclaration {
                blocker: charlie_blocker,
                blocking: attacks_charlie,
            }],
            charlie,
        );
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("publish all blocks"),
            TurnAction::RunPriority
        ));

        assert!(matches!(runner.state(), TurnState::DeclareBlockersPriority));
        assert_eq!(
            runner.combat.blockers.get(&attacks_bob),
            Some(&vec![bob_blocker])
        );
        assert_eq!(
            runner.combat.blockers.get(&attacks_charlie),
            Some(&vec![charlie_blocker])
        );
        assert_eq!(game.turn.priority_player, Some(alice));
    }

    #[test]
    fn multiplayer_defenders_finish_each_block_cost_transaction_before_the_next_defender() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let attacks_bob = create_battlefield_creature(&mut game, alice, "Attacks Bob");
        let attacks_charlie = create_battlefield_creature(&mut game, alice, "Attacks Charlie");
        let bob_blocker = create_battlefield_creature(&mut game, bob, "Bob Blocker");
        let charlie_blocker = create_battlefield_creature(&mut game, charlie, "Charlie Blocker");
        let bob_mountain = create_mountain(&mut game, bob);
        let charlie_mountain = create_mountain(&mut game, charlie);
        let tax = CardBuilder::new(CardId::new(), "Multiplayer Blocking Tax")
            .card_types(vec![CardType::Enchantment])
            .build();
        let tax = game.create_object_from_card(&tax, alice, Zone::Battlefield);
        game.object_mut(tax)
            .expect("blocking tax exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::block_cost(
                crate::target::ObjectFilter::default(),
                crate::target::ObjectFilter::default(),
                crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(vec![vec![
                    crate::mana::ManaSymbol::Generic(1),
                ]])),
                "Creatures can't block unless their controller pays {1} for each blocking creature.",
            )));
        game.refresh_continuous_state();
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareBlockers);

        let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareBlockersCheck);
        runner.combat.attackers = vec![
            AttackerInfo {
                creature: attacks_charlie,
                target: AttackTarget::Player(charlie),
            },
            AttackerInfo {
                creature: attacks_bob,
                target: AttackTarget::Player(bob),
            },
        ];
        let mut tq = TriggerQueue::new();

        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("start blockers"),
            TurnAction::Continue
        ));
        let TurnAction::Decision(DecisionContext::Blockers(bob_context)) = runner
            .advance(&mut game, &mut tq)
            .expect("Bob declares first")
        else {
            panic!("expected Bob's blocker declaration");
        };
        runner.respond_blockers(
            vec![BlockerDeclaration {
                blocker: bob_blocker,
                blocking: attacks_bob,
            }],
            bob_context.player,
        );
        let bob_window = match runner
            .advance(&mut game, &mut tq)
            .expect("Bob enters the blocking-cost mana window")
        {
            TurnAction::Decision(DecisionContext::SelectOptions(context)) => context,
            other => panic!("expected Bob's mana window before Charlie, got {other:?}"),
        };
        assert_eq!(bob_window.player, bob);
        assert!(runner.combat.blockers.is_empty());
        let bob_mana_choice = bob_window
            .options
            .iter()
            .find(|option| option.object_id == Some(bob_mountain))
            .map(|option| option.index)
            .expect("Bob's Mountain should be offered");
        runner.respond_options(vec![bob_mana_choice]);
        let bob_finish_window = match runner
            .advance(&mut game, &mut tq)
            .expect("Bob may keep activating mana abilities")
        {
            TurnAction::Decision(DecisionContext::SelectOptions(context)) => context,
            other => panic!("Bob's mana window should remain open, got {other:?}"),
        };
        let bob_finish = bob_finish_window
            .options
            .iter()
            .find(|option| option.description.starts_with("Finish"))
            .map(|option| option.index)
            .expect("Bob should be able to close the mana window");
        runner.respond_options(vec![bob_finish]);
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("Bob pays before Charlie declares"),
            TurnAction::Continue
        ));
        assert_eq!(
            runner.combat.blockers.get(&attacks_bob),
            Some(&vec![bob_blocker])
        );
        assert!(runner.combat.blockers.get(&attacks_charlie).is_none());
        assert_eq!(game.player(bob).expect("Bob exists").mana_pool.total(), 0);

        let TurnAction::Decision(DecisionContext::Blockers(charlie_context)) = runner
            .advance(&mut game, &mut tq)
            .expect("Charlie declares only after Bob completed payment")
        else {
            panic!("expected Charlie's blocker declaration");
        };
        assert_eq!(charlie_context.player, charlie);
        runner.respond_blockers(
            vec![BlockerDeclaration {
                blocker: charlie_blocker,
                blocking: attacks_charlie,
            }],
            charlie,
        );
        let charlie_window = match runner
            .advance(&mut game, &mut tq)
            .expect("Charlie enters a separate mana window")
        {
            TurnAction::Decision(DecisionContext::SelectOptions(context)) => context,
            other => panic!("expected Charlie's mana window, got {other:?}"),
        };
        assert_eq!(charlie_window.player, charlie);
        let charlie_mana_choice = charlie_window
            .options
            .iter()
            .find(|option| option.object_id == Some(charlie_mountain))
            .map(|option| option.index)
            .expect("Charlie's Mountain should be offered");
        runner.respond_options(vec![charlie_mana_choice]);
        let charlie_finish_window = match runner
            .advance(&mut game, &mut tq)
            .expect("Charlie's mana window remains repeatable")
        {
            TurnAction::Decision(DecisionContext::SelectOptions(context)) => context,
            other => panic!("Charlie's mana window should remain open, got {other:?}"),
        };
        let charlie_finish = charlie_finish_window
            .options
            .iter()
            .find(|option| option.description.starts_with("Finish"))
            .map(|option| option.index)
            .expect("Charlie should be able to close the mana window");
        runner.respond_options(vec![charlie_finish]);
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("publish the completed blocker declarations"),
            TurnAction::RunPriority
        ));
        assert_eq!(
            runner.combat.blockers.get(&attacks_charlie),
            Some(&vec![charlie_blocker])
        );
        assert_eq!(
            game.player(charlie)
                .expect("Charlie exists")
                .mana_pool
                .total(),
            0
        );
    }

    #[test]
    fn defender_cannot_block_an_attacker_attacking_another_player() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let attacker = create_battlefield_creature(&mut game, alice, "Attacks Charlie");
        let bob_blocker = create_battlefield_creature(&mut game, bob, "Bob Blocker");
        let mut combat = CombatState {
            attackers: vec![AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(charlie),
            }],
            ..CombatState::default()
        };
        let mut tq = TriggerQueue::new();

        let error = crate::game_loop::apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut tq,
            &[BlockerDeclaration {
                blocker: bob_blocker,
                blocking: attacker,
            }],
            bob,
        )
        .expect_err("Bob cannot block a creature attacking Charlie");

        assert!(
            error
                .to_string()
                .contains("can block only creatures attacking")
        );
        assert!(combat.blockers.is_empty());
    }

    fn add_attack_tax(game: &mut GameState, controller: PlayerId, amount: u32) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Runner Attack Tax")
            .card_types(vec![CardType::Enchantment])
            .build();
        let tax = game.create_object_from_card(&card, controller, Zone::Battlefield);
        game.object_mut(tax)
            .expect("attack tax exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(amount),
            ));
        tax
    }

    #[test]
    fn attack_cost_mana_window_taps_attackers_then_allows_mana_abilities_before_payment() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_battlefield_creature(&mut game, alice, "Taxed Attacker");
        game.remove_summoning_sickness(attacker);
        let mountain = create_mountain(&mut game, alice);
        let second_mountain = create_mountain(&mut game, alice);
        add_attack_tax(&mut game, bob, 1);
        game.refresh_continuous_state();

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        runner.state = TurnState::DeclareAttackersApply;
        runner.pending_attackers = Some(vec![AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(bob),
        }]);

        let first_window = match runner.advance(&mut game, &mut tq).unwrap() {
            TurnAction::Decision(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("expected the attack-cost mana window, got {other:?}"),
        };
        assert!(
            game.is_tapped(attacker),
            "CR 508.1f precedes the mana window"
        );
        assert!(!game.is_tapped(mountain));
        assert!(runner.combat.attackers.is_empty());
        assert_eq!(
            game.player(alice).expect("Alice exists").mana_pool.total(),
            0
        );

        let mana_choice = first_window
            .options
            .iter()
            .find(|option| option.object_id == Some(mountain))
            .map(|option| option.index)
            .expect("the Mountain should be offered in the attack-cost mana window");
        runner.respond_options(vec![mana_choice]);
        let second_window = match runner.advance(&mut game, &mut tq).unwrap() {
            TurnAction::Decision(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("the repeatable mana window should remain open, got {other:?}"),
        };
        assert!(game.is_tapped(mountain));
        assert_eq!(
            game.player(alice).expect("Alice exists").mana_pool.total(),
            1
        );
        assert!(runner.combat.attackers.is_empty());

        let finish_choice = second_window
            .options
            .iter()
            .find(|option| option.description.starts_with("Finish"))
            .map(|option| option.index)
            .expect("the mana window should have a finish option");
        runner.respond_options(vec![finish_choice]);
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(
            game.player(alice).expect("Alice exists").mana_pool.total(),
            0
        );
        assert_eq!(runner.combat.attackers.len(), 1);
        assert_eq!(runner.combat.attackers[0].creature, attacker);
        assert!(!game.is_tapped(second_mountain));
    }

    #[test]
    fn blocking_cost_locks_then_offers_repeatable_mana_window_before_publication() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_battlefield_creature(&mut game, alice, "Taxed Attack");
        let blocker = create_battlefield_creature(&mut game, bob, "Paying Blocker");
        let mountain = create_mountain(&mut game, bob);
        let tax = CardBuilder::new(CardId::new(), "Runner Blocking Tax")
            .card_types(vec![CardType::Enchantment])
            .build();
        let tax = game.create_object_from_card(&tax, alice, Zone::Battlefield);
        game.object_mut(tax)
            .expect("blocking tax exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::block_cost(
                crate::target::ObjectFilter::default(),
                crate::target::ObjectFilter::default(),
                crate::cost::TotalCost::mana(crate::mana::ManaCost::from_pips(vec![vec![
                    crate::mana::ManaSymbol::Generic(1),
                ]])),
                "Creatures can't block unless their controller pays {1} for each blocking creature.",
            )));
        game.refresh_continuous_state();
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareBlockers);
        game.turn.active_player = alice;

        let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareBlockersApply);
        runner.combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        runner.pending_blockers = Some((
            vec![BlockerDeclaration {
                blocker,
                blocking: attacker,
            }],
            bob,
        ));
        runner.defending_player = Some(bob);
        game.combat = Some(runner.combat.clone());

        let first_window = match runner.advance(&mut game, &mut tq).unwrap() {
            TurnAction::Decision(DecisionContext::SelectOptions(context)) => context,
            other => panic!("expected the blocking-cost mana window, got {other:?}"),
        };
        assert!(runner.combat.blockers.is_empty());
        assert!(!game.is_tapped(mountain));
        let mana_choice = first_window
            .options
            .iter()
            .find(|option| option.object_id == Some(mountain))
            .map(|option| option.index)
            .expect("the defending player's Mountain should be offered");
        runner.respond_options(vec![mana_choice]);

        let second_window = match runner.advance(&mut game, &mut tq).unwrap() {
            TurnAction::Decision(DecisionContext::SelectOptions(context)) => context,
            other => panic!("the blocking mana window should remain open, got {other:?}"),
        };
        assert!(game.is_tapped(mountain));
        assert_eq!(game.player(bob).expect("Bob exists").mana_pool.total(), 1);
        assert!(runner.combat.blockers.is_empty());
        let finish_choice = second_window
            .options
            .iter()
            .find(|option| option.description.starts_with("Finish"))
            .map(|option| option.index)
            .expect("the blocking mana window should have a finish option");
        runner.respond_options(vec![finish_choice]);

        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(runner.combat.blockers.get(&attacker), Some(&vec![blocker]));
        assert_eq!(game.player(bob).expect("Bob exists").mana_pool.total(), 0);
    }

    #[test]
    fn failed_attack_cost_after_mana_activation_rolls_back_the_whole_declaration() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_battlefield_creature(&mut game, alice, "Overtaxed Attacker");
        game.remove_summoning_sickness(attacker);
        let mountain = create_mountain(&mut game, alice);
        let second_mountain = create_mountain(&mut game, alice);
        add_attack_tax(&mut game, bob, 2);
        game.refresh_continuous_state();

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        runner.state = TurnState::DeclareAttackersApply;
        runner.pending_attackers = Some(vec![AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(bob),
        }]);

        let window = match runner.advance(&mut game, &mut tq).unwrap() {
            TurnAction::Decision(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("expected the attack-cost mana window, got {other:?}"),
        };
        let mana_choice = window
            .options
            .iter()
            .find(|option| option.object_id == Some(mountain))
            .map(|option| option.index)
            .expect("the Mountain should be activatable");
        runner.respond_options(vec![mana_choice]);
        let second_window = match runner.advance(&mut game, &mut tq).unwrap() {
            TurnAction::Decision(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("the second Mountain should keep the window open, got {other:?}"),
        };
        let finish_choice = second_window
            .options
            .iter()
            .find(|option| option.description.starts_with("Finish"))
            .map(|option| option.index)
            .expect("the player may close the window before producing enough mana");
        runner.respond_options(vec![finish_choice]);
        runner
            .advance(&mut game, &mut tq)
            .expect_err("one mana cannot pay the two-mana attack cost");

        assert!(!game.is_tapped(attacker));
        assert!(!game.is_tapped(mountain));
        assert!(!game.is_tapped(second_mountain));
        assert_eq!(
            game.player(alice).expect("Alice exists").mana_pool.total(),
            0
        );
        assert!(runner.combat.attackers.is_empty());
        assert!(tq.is_empty());
    }

    #[test]
    fn optional_attack_cost_choice_happens_after_attackers_are_tapped() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_battlefield_creature(&mut game, alice, "Exert Order Probe");
        game.remove_summoning_sickness(attacker);
        game.object_mut(attacker)
            .expect("attacker exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::exert_attack(
                true,
                None,
                "You may exert this creature as it attacks",
            )));
        game.refresh_continuous_state();

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        runner.state = TurnState::DeclareAttackersApply;
        runner.pending_attackers = Some(vec![AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(bob),
        }]);

        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::Decision(DecisionContext::Boolean(_))
        ));
        assert!(game.is_tapped(attacker));
        assert!(runner.combat.attackers.is_empty());

        runner.respond_boolean(false);
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(runner.combat.attackers.len(), 1);
        assert!(!game.object_exerted_this_turn(attacker));
    }

    #[test]
    fn test_end_combat_keeps_attackers_through_priority_window() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_battlefield_creature(&mut game, alice, "End Combat Probe");

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::CombatDamage);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        runner.state = TurnState::EndCombat;
        runner.combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(runner.combat.clone());

        let action = runner.advance(&mut game, &mut tq).unwrap();

        assert!(matches!(action, TurnAction::RunPriority));
        assert!(matches!(runner.state(), TurnState::EndCombatPriority));
        assert_eq!(game.turn.step, Some(Step::EndCombat));
        assert_eq!(
            game.combat
                .as_ref()
                .expect("combat should remain active through end combat priority")
                .attackers
                .len(),
            1
        );

        runner.priority_done();
        let follow_up = runner.advance(&mut game, &mut tq).unwrap();

        assert!(matches!(follow_up, TurnAction::Continue));
        assert!(matches!(runner.state(), TurnState::NextMain));
        assert!(
            game.combat
                .as_ref()
                .expect("combat should still exist")
                .attackers
                .is_empty()
        );
    }

    #[test]
    fn test_turn_runner_consumes_additional_combat_before_normal_next_main() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::EndCombat);
        game.turn_store.combat_phases_started_this_turn = 1;
        game.turn_store.additional_phases.push(Phase::Combat);
        game.turn_store.additional_phase_continuation = Some(Phase::NextMain);
        runner.state = TurnState::EndCombatPriority;

        let action = runner.advance(&mut game, &mut tq).unwrap();

        assert!(matches!(action, TurnAction::Continue));
        assert!(matches!(runner.state(), TurnState::BeginCombat));
        assert!(game.turn_store.additional_phases.is_empty());
        assert_eq!(
            game.turn_store.phase_schedule_continuation,
            Some(TurnScheduleDestination::Phase(Phase::NextMain))
        );

        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(game.turn.phase, Phase::Combat);
        assert_eq!(game.turn_store.combat_phases_started_this_turn, 2);

        runner.state = TurnState::EndCombatPriority;
        let action = runner.advance(&mut game, &mut tq).unwrap();

        assert!(matches!(action, TurnAction::Continue));
        assert!(matches!(runner.state(), TurnState::NextMain));
        assert_eq!(game.turn_store.additional_phase_continuation, None);
        assert_eq!(game.turn_store.phase_schedule_continuation, None);
    }

    #[test]
    fn full_and_synthetic_phase_additions_share_creation_order() {
        let mut newer_full_phase = setup_game();
        newer_full_phase.turn.turn_number = 2;
        newer_full_phase.turn.phase = Phase::FirstMain;
        newer_full_phase.turn.step = None;
        newer_full_phase.add_step_after_phase(Step::Draw, Phase::FirstMain);
        newer_full_phase.add_additional_phase_group([Phase::Combat]);
        let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
        let mut tq = TriggerQueue::new();

        runner
            .advance(&mut newer_full_phase, &mut tq)
            .expect("leave first main");
        assert!(matches!(runner.state(), TurnState::BeginCombat));

        let mut newer_synthetic_phase = setup_game();
        newer_synthetic_phase.turn.turn_number = 2;
        newer_synthetic_phase.turn.phase = Phase::FirstMain;
        newer_synthetic_phase.turn.step = None;
        newer_synthetic_phase.add_additional_phase_group([Phase::Combat]);
        newer_synthetic_phase.add_step_after_phase(Step::Draw, Phase::FirstMain);
        let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
        let mut tq = TriggerQueue::new();

        runner
            .advance(&mut newer_synthetic_phase, &mut tq)
            .expect("leave first main");
        assert!(matches!(runner.state(), TurnState::Draw));
        assert!(
            newer_synthetic_phase
                .turn_store
                .active_added_step
                .is_some_and(|scheduled| scheduled.isolated_phase)
        );
    }

    #[test]
    fn added_steps_after_phase_use_isolated_phases_newest_first() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let draw = CardBuilder::new(CardId::new(), "Synthetic Draw").build();
        game.create_object_from_card(&draw, alice, Zone::Library);

        game.add_step_after_phase(Step::Upkeep, Phase::FirstMain);
        game.add_step_after_phase(Step::Draw, Phase::FirstMain);

        let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
        let mut tq = TriggerQueue::new();
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("leave first main"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Draw));
        assert_eq!(
            game.turn_store.active_added_step,
            Some(ScheduledStep {
                phase: Phase::Beginning,
                step: Step::Draw,
                isolated_phase: true,
            })
        );

        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("additional draw"),
            TurnAction::RunPriority
        ));
        assert_eq!(game.player(alice).expect("alice").hand.len(), 1);
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("finish additional draw"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Upkeep));

        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("additional upkeep"),
            TurnAction::RunPriority
        ));
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("finish additional upkeep"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::BeginCombat));
    }

    #[test]
    fn added_steps_after_same_step_are_newest_first_before_normal_continuation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        for index in 0..2 {
            let draw = CardBuilder::new(CardId::new(), &format!("Ordered Draw {index}")).build();
            game.create_object_from_card(&draw, alice, Zone::Library);
        }

        game.add_step_after(Step::Upkeep, Step::Upkeep);
        game.add_step_after(Step::Draw, Step::Upkeep);

        let mut runner = TurnRunner::from_state_for_sync(TurnState::UpkeepPriority);
        let mut tq = TriggerQueue::new();
        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("finish upkeep"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Draw));
        assert_eq!(
            game.turn_store
                .pending_added_steps
                .first()
                .map(|scheduled| scheduled.step),
            Some(Step::Upkeep)
        );

        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("newest draw"),
            TurnAction::RunPriority
        ));
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("finish newest draw"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Upkeep));
        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("older upkeep"),
            TurnAction::RunPriority
        ));
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("finish older upkeep"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Draw));
    }

    #[test]
    fn added_step_before_named_step_runs_before_that_normal_step() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        game.add_step_before(Step::Upkeep, Step::Draw);

        let mut runner = TurnRunner::from_state_for_sync(TurnState::UpkeepPriority);
        let mut tq = TriggerQueue::new();
        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("finish upkeep"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Upkeep));
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("additional upkeep before draw"),
            TurnAction::RunPriority
        ));
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("finish additional upkeep"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Draw));
    }

    #[test]
    fn added_step_before_combat_damage_precedes_the_first_damage_step() {
        let mut game = setup_game();
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareBlockers);
        game.add_step_before(Step::Upkeep, Step::CombatDamage);

        let state = resolve_schedule_destination(
            &mut game,
            TurnScheduleDestination::CombatDamageFirstStrike,
        );
        assert!(matches!(state, TurnState::Upkeep));
        assert_eq!(game.turn.step, Some(Step::Upkeep));

        let state = finish_step(
            &mut game,
            Step::Upkeep,
            TurnScheduleDestination::Step(Step::Draw),
        );
        assert!(matches!(state, TurnState::CombatDamageFirstStrike));
    }

    #[test]
    fn you_get_gate_and_step_skip_apply_to_added_steps() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;

        assert!(!game.add_step_after_phase_for_controller(bob, Step::Upkeep, Phase::FirstMain));
        assert!(game.add_step_after_phase_for_controller(alice, Step::Upkeep, Phase::FirstMain));
        game.add_step_after_phase(Step::Upkeep, Phase::FirstMain);
        game.skip_next_step(alice, Step::Upkeep);

        let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMainPriority);
        let mut tq = TriggerQueue::new();
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("leave first main"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Upkeep));
        assert!(matches!(
            runner.advance(&mut game, &mut tq).expect("skip one upkeep"),
            TurnAction::Continue
        ));
        assert!(matches!(runner.state(), TurnState::Upkeep));
        assert_eq!(game.pending_step_skips(alice, Step::Upkeep), 0);
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("run second upkeep"),
            TurnAction::RunPriority
        ));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_turn_runner_pauses_for_exert_attack_choice_before_applying_attackers() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let exert_probe =
            CardDefinitionBuilder::new(crate::ids::CardId::from_raw(9105), "Runner Exert Probe")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .parse_text("You may exert this creature as it attacks. When you do, draw a card.")
                .expect("runner exert probe should parse");
        let attacker = game.create_object_from_definition(&exert_probe, alice, Zone::Battlefield);

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        runner.state = TurnState::DeclareAttackersApply;
        runner.pending_attackers = Some(vec![AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(bob),
        }]);

        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(
            action,
            TurnAction::Decision(DecisionContext::Boolean(_))
        ));
        assert!(
            runner.combat.attackers.is_empty(),
            "attacker should not be committed before the exert prompt is answered"
        );
        assert!(
            game.stack.is_empty(),
            "linked exert trigger should not be created before the choice is made"
        );
        assert!(
            game.is_tapped(attacker),
            "CR 508.1f taps the attacker before the optional 508.1g exert choice"
        );

        runner.respond_boolean(true);
        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(runner.combat.attackers.len(), 1);
        assert!(
            game.is_tapped(attacker),
            "attacker should be tapped after the attack is applied"
        );
        assert_eq!(
            game.stack.len(),
            1,
            "accepting exert should queue the linked trigger onto the stack"
        );
    }

    #[test]
    fn turn_runner_collects_enlist_creature_before_committing_attackers() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let enlist_probe = CardDefinitionBuilder::new(CardId::new(), "Runner Enlist Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .enlist()
            .build();
        let attacker = game.create_object_from_definition(&enlist_probe, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker);
        let support = create_battlefield_creature(&mut game, alice, "Enlist Support");
        game.remove_summoning_sickness(support);
        game.refresh_continuous_state();

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        runner.state = TurnState::DeclareAttackersApply;
        runner.pending_attackers = Some(vec![AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(bob),
        }]);

        let prompt = match runner.advance(&mut game, &mut tq).unwrap() {
            TurnAction::Decision(DecisionContext::SelectObjects(prompt)) => prompt,
            other => panic!("expected the 508.1g enlist selection, got {other:?}"),
        };
        assert!(game.is_tapped(attacker), "508.1f occurs before enlist");
        assert!(!game.is_tapped(support));
        assert!(runner.combat.attackers.is_empty());
        assert_eq!(
            prompt
                .candidates
                .iter()
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>(),
            vec![support]
        );

        runner.respond_discard(vec![support]);
        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert!(game.is_tapped(support));
        assert_eq!(runner.combat.attackers.len(), 1);
        assert_eq!(
            game.stack.len(),
            1,
            "the linked enlist trigger uses the stack"
        );
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

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_turn_runner_pauses_for_optional_reveal_first_draw_and_queues_trigger() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.turn_number = 2;
        let revealer =
            CardDefinitionBuilder::new(crate::ids::CardId::from_raw(9103), "Reveal Oracle")
                .card_types(vec![crate::types::CardType::Enchantment])
                .parse_text(
                    "You may reveal the first card you draw each turn as you draw it. Whenever you reveal a creature card this way, draw a card.",
                )
                .expect("reveal oracle should parse");
        game.create_object_from_definition(&revealer, alice, crate::zone::Zone::Battlefield);

        let creature =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(9104), "Drawn Creature")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        game.create_object_from_card(&creature, alice, crate::zone::Zone::Library);

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
        assert_eq!(game.player(alice).expect("alice exists").hand.len(), 1);
        assert_eq!(tq.entries.len(), 1);
        let drawn = *game
            .player(alice)
            .expect("alice exists")
            .hand
            .last()
            .expect("drawn card should be in hand");
        let revealed = tq.entries[0]
            .tagged_objects
            .get(&TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
            .expect("queued trigger should preserve revealed card");
        assert_eq!(revealed.len(), 1);
        assert_eq!(revealed[0].object_id, drawn);
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

    #[test]
    fn u036_turn_runner_collects_every_sector_choice_before_atomic_commit() {
        use crate::marker::SectorDesignation::{Alpha, Beta, Gamma};

        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Bob".into(),
                "Charlie".into(),
                "Dana".into(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let dana = PlayerId::from_index(3);
        game.turn.active_player = alice;
        let sculptor = |name: &str| {
            CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::space_sculptor()))
                .build()
        };
        game.create_object_from_definition(&sculptor("Alice Sculptor"), alice, Zone::Battlefield);
        game.create_object_from_definition(
            &sculptor("Charlie Sculptor"),
            charlie,
            Zone::Battlefield,
        );
        let alice_creature = create_battlefield_creature(&mut game, alice, "Alice Creature");
        let bob_creature = create_battlefield_creature(&mut game, bob, "Bob Creature");
        let charlie_creature = create_battlefield_creature(&mut game, charlie, "Charlie Creature");
        let dana_creature = create_battlefield_creature(&mut game, dana, "Dana Creature");

        let mut runner = TurnRunner::new();
        let mut tq = TriggerQueue::new();
        for (expected_player, answer) in [(bob, 1), (dana, 2), (alice, 0), (charlie, 1)] {
            let progress = runner
                .apply_sbas_until_commander_choice(&mut game, &mut tq)
                .expect("sector prompt");
            let RunnerProgress::NeedsDecision(DecisionContext::SelectOptions(context)) = progress
            else {
                panic!("expected sector decision")
            };
            assert_eq!(context.player, expected_player);
            assert!(
                [
                    alice_creature,
                    bob_creature,
                    charlie_creature,
                    dana_creature
                ]
                .into_iter()
                .all(|creature| game.sector_designation(creature).is_none()),
                "no designation may be committed before the final answer"
            );
            runner.respond_options(vec![answer]);
        }

        assert!(matches!(
            runner
                .apply_sbas_until_commander_choice(&mut game, &mut tq)
                .expect("commit assignment batch"),
            RunnerProgress::Complete(())
        ));
        assert_eq!(game.sector_designation(bob_creature), Some(Beta));
        assert_eq!(game.sector_designation(dana_creature), Some(Gamma));
        assert_eq!(game.sector_designation(alice_creature), Some(Alpha));
        assert_eq!(game.sector_designation(charlie_creature), Some(Beta));
    }

    #[test]
    fn test_turn_runner_skips_starting_players_first_draw_in_non_commander_game() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(9102), "Normal Draw Skip")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let _card_id = game.create_object_from_card(&card, alice, crate::zone::Zone::Library);

        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        runner.state = TurnState::Draw;

        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            0
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
    }

    #[test]
    fn test_turn_runner_keeps_starting_players_first_draw_in_commander_game() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let commander =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(9103), "Turn One Commander")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let commander_id =
            game.create_object_from_card(&commander, alice, crate::zone::Zone::Command);
        game.set_as_commander(commander_id, alice);

        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(9104), "Commander Draw")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let _card_id = game.create_object_from_card(&card, alice, crate::zone::Zone::Library);

        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        runner.state = TurnState::Draw;

        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            1
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
    }

    #[test]
    fn test_turn_runner_keeps_starting_players_first_draw_in_normal_multiplayer_game() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(
            crate::ids::CardId::from_raw(9105),
            "Runner Multiplayer Draw",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .build();
        game.create_object_from_card(&card, alice, crate::zone::Zone::Library);

        let mut tq = TriggerQueue::new();
        let mut runner = TurnRunner::new();
        runner.state = TurnState::Draw;

        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            1
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
    }

    #[test]
    fn attacker_response_records_announced_band_before_blockers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let bander = create_battlefield_creature(&mut game, alice, "Bander");
        let companion = create_battlefield_creature(&mut game, alice, "Companion");
        game.object_mut(bander)
            .unwrap()
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::banding()));
        game.remove_summoning_sickness(bander);
        game.remove_summoning_sickness(companion);

        let declarations = vec![
            AttackerDeclaration {
                creature: bander,
                target: AttackTarget::Player(bob),
            },
            AttackerDeclaration {
                creature: companion,
                target: AttackTarget::Player(bob),
            },
        ];
        let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareAttackersApply);
        runner.respond_attackers_with_bands(declarations, vec![vec![bander, companion]]);
        let mut tq = TriggerQueue::new();

        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(runner.combat.attacking_bands, vec![vec![bander, companion]]);
        assert_eq!(
            game.combat.as_ref().unwrap().attacking_bands,
            vec![vec![bander, companion]]
        );

        let game_combat = game.combat.as_mut().unwrap();
        game_combat
            .attackers
            .retain(|attacker| attacker.creature != bander);
        game_combat.attacking_bands[0].retain(|member| *member != bander);
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::Continue
        ));
        assert_eq!(runner.combat.attacking_bands, vec![vec![companion]]);
    }
}
