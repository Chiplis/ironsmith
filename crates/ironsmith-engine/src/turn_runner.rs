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
    blocker_mana_ability_window_context, try_drain_pending_trigger_events,
    finish_attack_declaration_transaction,
    finish_blocker_declaration_transaction_deferring_triggers, generate_and_queue_step_triggers,
    get_declare_attackers_decision, get_declare_blockers_decision,
    preview_attack_cost_needs_mana_window, preview_optional_attack_cost_prompts,
    queue_block_declaration_events, try_queue_combat_damage_triggers,
};
use crate::game_state::{
    AddedStepPlacement, GameState, Phase, ScheduledStep, Step, TurnScheduleDestination,
};
use crate::ids::{ObjectId, PlayerId};
use crate::rules::combat::deals_first_strike_damage_with_game;
use crate::rules::state_based::check_state_based_actions;
use crate::triggers::TriggerQueue;
use crate::turn::execute_cleanup_step;

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
    FirstMainSagas,
    FirstMainAttractions,
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
    /// Collecting CR 510.1c-d damage divisions before first-strike damage.
    CombatDamageFirstStrikeAssign,
    CombatDamageFirstStrikeSbas,
    CombatDamageFirstStrikePriority,
    CombatDamageRegular,
    /// Collecting CR 510.1c-d damage divisions before regular damage.
    CombatDamageRegularAssign,
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
    UntapEndMana,
    SkippedPhaseEndMana,
}

impl TurnState {
    pub fn sync_name(&self) -> &'static str {
        match self {
            Self::BeginTurn => "begin_turn",
            Self::Untap => "untap",
            Self::UntapEndMana => "untap_end_mana",
            Self::SkippedPhaseEndMana => "skipped_phase_end_mana",
            Self::Upkeep => "upkeep",
            Self::UpkeepPriority => "upkeep_priority",
            Self::Draw => "draw",
            Self::DrawPriority => "draw_priority",
            Self::FirstMain => "first_main",
            Self::FirstMainSagas => "first_main_sagas",
            Self::FirstMainAttractions => "first_main_attractions",
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
            Self::CombatDamageFirstStrikeAssign => "combat_damage_first_strike_assign",
            Self::CombatDamageFirstStrikeSbas => "combat_damage_first_strike_sbas",
            Self::CombatDamageFirstStrikePriority => "combat_damage_first_strike_priority",
            Self::CombatDamageRegular => "combat_damage_regular",
            Self::CombatDamageRegularAssign => "combat_damage_regular_assign",
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
            "untap_end_mana" => Self::UntapEndMana,
            "skipped_phase_end_mana" => Self::SkippedPhaseEndMana,
            "upkeep" => Self::Upkeep,
            "upkeep_priority" => Self::UpkeepPriority,
            "draw" => Self::Draw,
            "draw_priority" => Self::DrawPriority,
            "first_main" => Self::FirstMain,
            "first_main_sagas" => Self::FirstMainSagas,
            "first_main_attractions" => Self::FirstMainAttractions,
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
            "combat_damage_first_strike_assign" => Self::CombatDamageFirstStrikeAssign,
            "combat_damage_first_strike_sbas" => Self::CombatDamageFirstStrikeSbas,
            "combat_damage_first_strike_priority" => Self::CombatDamageFirstStrikePriority,
            "combat_damage_regular" => Self::CombatDamageRegular,
            "combat_damage_regular_assign" => Self::CombatDamageRegularAssign,
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
    draw_step_player: PlayerId,
    player: PlayerId,
    applicable_effects: Vec<crate::replacement::ReplacementEffectId>,
    event: crate::events::Event,
    applied_effects: std::collections::HashSet<crate::replacement::ReplacementEffectId>,
    applied_effect_keys: std::collections::HashSet<crate::replacement::ReplacementEffectKey>,
}

/// Choices asked while the runner applies one SBA check (CR 616.1: which
/// replacement effect applies to a dying creature, "you may ... instead",
/// battle protectors). Answers are replayed against a private copy until the
/// whole check can be applied without a new prompt.
#[derive(Debug, Clone)]
struct PendingSbaChoices {
    answers: Vec<AttackCostAnswer>,
    prompt: DecisionContext,
    response: Option<AttackCostAnswer>,
}

/// A turn-based action waiting for replacement/prevention or nested effect
/// choices. Its trial state is discarded until every answer is available.
#[derive(Debug, Clone)]
struct PendingTurnActionChoices {
    answers: Vec<AttackCostAnswer>,
    prompt: DecisionContext,
    response: Option<AttackCostAnswer>,
}

#[derive(Debug, Clone)]
struct PendingCleanupDiscard {
    cards: Vec<ObjectId>,
    choices: PendingTurnActionChoices,
}

#[derive(Debug, Clone)]
struct PendingTurnDraw {
    player: PlayerId,
    count: usize,
    choices: PendingUntapChoices,
}

/// A draw-step draw replaced by effects that ask for choices (CR 616.1,
/// 608.2d: Abundance's land/nonland choice, Underrealm Lich's pick). The
/// effects run on a private copy with the answers so far and are published
/// only once every prompt has been answered.
#[derive(Debug, Clone)]
struct PendingDrawReplacementEffects {
    player: PlayerId,
    effects: Vec<crate::effect::Effect>,
    context: Box<crate::events::processing::ReplacementEventContext>,
    source: ObjectId,
    controller: PlayerId,
    answers: Vec<AttackCostAnswer>,
    prompt: Option<DecisionContext>,
    response: Option<AttackCostAnswer>,
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
    requires_mana_window: bool,
    declaration_source: ObjectId,
}

#[derive(Debug, Clone)]
struct PendingAttackerManaWindow {
    transaction: AttackDeclarationTransaction,
    optional_cost_answers: Vec<AttackCostAnswer>,
    declaration_source: ObjectId,
}

#[derive(Debug, Clone)]
struct PendingAttackerPaymentChoices {
    transaction: AttackDeclarationTransaction,
    answers: Vec<AttackCostAnswer>,
    prompt: DecisionContext,
    response: Option<AttackCostAnswer>,
}

#[derive(Debug, Clone)]
struct PendingBlockerManaWindow {
    transaction: BlockDeclarationTransaction,
    payers: Vec<PlayerId>,
    next_payer: usize,
    declaration_source: ObjectId,
}

#[derive(Debug, Clone)]
struct PendingCombatManaChoices {
    player: PlayerId,
    choice: usize,
    attack: bool,
    answers: Vec<AttackCostAnswer>,
    prompt: DecisionContext,
    response: Option<AttackCostAnswer>,
}

#[derive(Debug, Clone)]
struct PendingBlockerPreparationChoices {
    declarations: Vec<BlockerDeclaration>,
    defending_player: PlayerId,
    answers: Vec<AttackCostAnswer>,
    prompt: DecisionContext,
    response: Option<AttackCostAnswer>,
}

#[derive(Debug, Clone)]
struct PendingBlockerPaymentChoices {
    transaction: BlockDeclarationTransaction,
    answers: Vec<AttackCostAnswer>,
    prompt: DecisionContext,
    response: Option<AttackCostAnswer>,
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

/// Untap-step choices (CR 502.3): "you may choose not to untap" prompts and
/// the Winter Orb family's choice of which permanents untap. The step runs on
/// a clone with the answers so far; a new prompt pauses the runner.
#[derive(Debug, Clone)]
struct PendingUntapChoices {
    answers: Vec<AttackCostAnswer>,
    prompt: Option<DecisionContext>,
    response: Option<AttackCostAnswer>,
}

enum RunnerProgress<T> {
    Complete(T),
    NeedsDecision(DecisionContext),
}

#[derive(Debug, Clone)]
enum AttackCostAnswer {
    Boolean(bool),
    Number(u32),
    Text(String),
    Objects(Vec<ObjectId>),
    Options(Vec<usize>),
    Order(Vec<ObjectId>),
    Colors(Vec<crate::color::Color>),
    Counters(Vec<(crate::object::CounterType, u32)>),
    Partition(Vec<ObjectId>),
    Proliferate(crate::decisions::specs::ProliferateResponse),
    Targets(Vec<crate::game_state::Target>),
    Distribute(Vec<(crate::game_state::Target, u32)>),
    Priority(crate::decision::LegalAction),
    Attackers(Vec<crate::decisions::spec::AttackerDeclaration>),
    Blockers(Vec<crate::decisions::spec::BlockerDeclaration>),
    ManaPayment(crate::mana_payment::ManaPaymentResponse),
}

#[derive(Debug, Clone)]
struct QueuedAttackCostDecisionMaker {
    capture_mana_payment: bool,
    answers: Vec<AttackCostAnswer>,
    next: usize,
    pending_prompt: Option<DecisionContext>,
}

impl QueuedAttackCostDecisionMaker {
    fn new(answers: Vec<AttackCostAnswer>) -> Self {
        Self {
            capture_mana_payment: false,
            answers,
            next: 0,
            pending_prompt: None,
        }
    }
    fn next_answer(&mut self) -> Option<AttackCostAnswer> {
        let answer = self.answers.get(self.next).cloned();
        self.next += 1;
        answer
    }
}

impl DecisionMaker for QueuedAttackCostDecisionMaker {
    fn decide_priority(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> crate::decision::LegalAction {
        match self.next_answer() {
            Some(AttackCostAnswer::Priority(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Priority(ctx.clone()));
                crate::decision::LegalAction::PassPriority
            }
        }
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        match self.next_answer() {
            Some(AttackCostAnswer::Attackers(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Attackers(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        match self.next_answer() {
            Some(AttackCostAnswer::Blockers(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Blockers(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        match self.next_answer() {
            Some(AttackCostAnswer::Number(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Number(ctx.clone()));
                ctx.min
            }
        }
    }

    fn decide_text(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        match self.next_answer() {
            Some(AttackCostAnswer::Text(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::TextInput(ctx.clone()));
                ctx.initial_value.clone().unwrap_or_default()
            }
        }
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        match self.next_answer() {
            Some(AttackCostAnswer::Colors(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Colors(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        match self.next_answer() {
            Some(AttackCostAnswer::Counters(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Counters(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_partition(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        match self.next_answer() {
            Some(AttackCostAnswer::Partition(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Partition(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        match self.next_answer() {
            Some(AttackCostAnswer::Proliferate(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Proliferate(ctx.clone()));
                Default::default()
            }
        }
    }

    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        match self.next_answer() {
            Some(AttackCostAnswer::Targets(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Targets(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(crate::game_state::Target, u32)> {
        match self.next_answer() {
            Some(AttackCostAnswer::Distribute(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Distribute(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ManaPaymentContext,
    ) -> crate::mana_payment::ManaPaymentResponse {
        use crate::mana_payment::ManaPaymentResponse;
        if !self.capture_mana_payment {
            let options = crate::decisions::context::SelectOptionsContext::new(
                ctx.player,
                Some(ctx.source),
                format!("Confirm mana payment for {}", ctx.subject),
                vec![
                    crate::decisions::context::SelectableOption::new(1, "Confirm payment"),
                    crate::decisions::context::SelectableOption::new(0, "Cancel"),
                ],
                1,
                1,
            );
            return if self.decide_options(game, &options).first().copied() == Some(1) {
                ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                }
            } else {
                ManaPaymentResponse::Cancel
            };
        }
        match self.next_answer() {
            Some(AttackCostAnswer::ManaPayment(response)) => response,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::ManaPayment(ctx.clone()));
                ManaPaymentResponse::Cancel
            }
        }
    }
    fn awaiting_choice(&self) -> bool {
        self.pending_prompt.is_some()
    }
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        match self.next_answer() {
            Some(AttackCostAnswer::Boolean(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Boolean(ctx.clone()));
                false
            }
        }
    }
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        match self.next_answer() {
            Some(AttackCostAnswer::Objects(objects)) => objects,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::SelectObjects(ctx.clone()));
                Vec::new()
            }
        }
    }
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        match self.next_answer() {
            Some(AttackCostAnswer::Options(options)) => options,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::SelectOptions(ctx.clone()));
                Vec::new()
            }
        }
    }
    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // "In any order" (Abundance's bottom-of-library cards): the chooser
        // orders them (CR 401.4 / 608.2d); a trivial order needs no prompt.
        let default_order = || ctx.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        if ctx.items.len() < 2 {
            return default_order();
        }
        match self.next_answer() {
            Some(AttackCostAnswer::Order(order)) => {
                let mut remaining = default_order();
                let mut ordered = Vec::with_capacity(remaining.len());
                for id in order {
                    if let Some(index) = remaining.iter().position(|item| *item == id) {
                        ordered.push(remaining.remove(index));
                    }
                }
                ordered.extend(remaining);
                ordered
            }
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Order(ctx.clone()));
                default_order()
            }
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
    /// Original occurrence boundary while optional untap choices suspend a
    /// lane. Retried actions use this receipt rather than newly registered state.
    pending_untap_boundary: Option<crate::turn::UntapStepBoundary>,
    pending_attraction_choices: Option<PendingUntapChoices>,
    /// Choices made while a restart's deferred cards enter (CR 726.4).
    pending_restart_entry_choices: Option<PendingUntapChoices>,
    /// Pending attacker-cost prompts and their collected answers.
    pending_attacker_optional_costs: Option<PendingAttackerOptionalCosts>,
    /// Attack declaration paused after CR 508.1f tapping and before costs.
    pending_attacker_mana_window: Option<PendingAttackerManaWindow>,
    pending_attacker_payment_choices: Option<PendingAttackerPaymentChoices>,
    /// Block declaration paused after CR 509.1d cost locking and before payment.
    pending_blocker_mana_window: Option<PendingBlockerManaWindow>,
    /// Nested choices made while activating mana abilities to pay combat costs.
    pending_combat_mana_choices: Option<PendingCombatManaChoices>,
    pending_blocker_preparation_choices: Option<PendingBlockerPreparationChoices>,
    pending_blocker_payment_choices: Option<PendingBlockerPaymentChoices>,
    /// Pending single-option response for a runner-driven SelectOptions decision.
    pending_option: Option<usize>,
    /// Pending blocker declarations from the caller.
    pending_blockers: Option<(Vec<BlockerDeclaration>, PlayerId)>,
    /// Pending discard selection from the caller.
    pending_discard: Option<Vec<ObjectId>>,
    /// Pending combat-damage division for the current assignment prompt.
    pending_distribution: Option<Vec<(crate::game_state::Target, u32)>>,
    /// Pending yes/no response for runner-driven boolean decisions.
    pending_boolean: Option<bool>,
    /// Pending CR 616 choice among draw replacement effects.
    pending_draw_replacement: Option<PendingDrawReplacementChoice>,
    /// Pending first-draw reveal decisions that pause the draw step.
    pending_draw_reveal: Option<PendingDrawRevealChoice>,
    /// Draw-step replacement effects waiting on their controller's choices.
    pending_draw_replacement_effects: Option<PendingDrawReplacementEffects>,
    pending_turn_draw: Option<PendingTurnDraw>,
    /// Choices of an SBA check the runner is applying.
    pending_sba_choices: Option<PendingSbaChoices>,
    pending_combat_damage_choices: Option<PendingTurnActionChoices>,
    pending_saga_lore_choices: Option<PendingTurnActionChoices>,
    pending_mana_loss_choices: Option<PendingTurnActionChoices>,
    skipped_phase_boundary: Option<(Step, Phase, TurnScheduleDestination)>,
    pending_cleanup_discard: Option<PendingCleanupDiscard>,
    /// Active teammates whose turn-based draw is still pending this draw step.
    remaining_draw_players: Vec<PlayerId>,
    /// Draw events accumulated while shared-team draw choices pause and resume.
    shared_draw_events: Vec<crate::triggers::TriggerEvent>,
    /// Private draw-step state retained across prompts, never partially published.
    draw_step_working_game: Option<Box<GameState>>,
    /// Runner state before the draw operation, restored if execution fails.
    draw_step_checkpoint: Option<Box<TurnRunner>>,
    /// Commander-specific choice that paused the runner.
    pending_commander_choice: Option<PendingCommanderChoice>,
    /// Legend-rule keep choice that paused the runner.
    pending_legend_choice: Option<PendingLegendRuleChoice>,
    /// Legend-rule keep choices already answered for the current SBA check,
    /// applied together with that check's other actions (CR 704.3).
    resolved_legend_keeps: Vec<(ObjectId, Vec<ObjectId>)>,
    /// Space-sculptor sector choices collected without partial state writes.
    pending_sector_designations: Option<PendingSectorDesignationChoices>,
    /// Defending player for the current combat.
    defending_player: Option<PlayerId>,
    /// Defending players who still need to declare blockers, in APNAP order.
    remaining_defending_players: Vec<PlayerId>,
    /// (blocker, attacker) pairs declared so far this declare-blockers step.
    /// Their trigger events are queued once, after the last defending player
    /// declares (CR 509.1 is a single turn-based action).
    declared_block_pairs: Vec<(ObjectId, ObjectId)>,
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
            pending_untap_boundary: None,
            pending_attraction_choices: None,
            pending_restart_entry_choices: None,
            pending_attacker_optional_costs: None,
            pending_attacker_mana_window: None,
            pending_attacker_payment_choices: None,
            pending_blocker_mana_window: None,
            pending_combat_mana_choices: None,
            pending_blocker_preparation_choices: None,
            pending_blocker_payment_choices: None,
            pending_option: None,
            pending_blockers: None,
            pending_discard: None,
            pending_distribution: None,
            pending_boolean: None,
            pending_draw_replacement: None,
            pending_draw_reveal: None,
            pending_draw_replacement_effects: None,
            pending_turn_draw: None,
            pending_sba_choices: None,
            pending_combat_damage_choices: None,
            pending_saga_lore_choices: None,
            pending_mana_loss_choices: None,
            skipped_phase_boundary: None,
            pending_cleanup_discard: None,
            remaining_draw_players: Vec::new(),
            shared_draw_events: Vec::new(),
            draw_step_working_game: None,
            draw_step_checkpoint: None,
            pending_commander_choice: None,
            pending_legend_choice: None,
            resolved_legend_keeps: Vec::new(),
            pending_sector_designations: None,
            defending_player: None,
            remaining_defending_players: Vec::new(),
            declared_block_pairs: Vec::new(),
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
        let retrying_untap_choice = self.has_pending_untap_continuation();
        let runner_checkpoint = self.clone();
        let game_checkpoint = game.clone();
        let queue_checkpoint = tq.clone();
        let result = self.advance_inner(game, tq);
        if (retrying_untap_choice && result.is_err())
            || matches!(&result, Err(GameLoopError::ExecutionFailed(error)) if error.is_incomplete_execution())
        {
            // Retain the accepted prefix and submitted response together with
            // the original untap boundary. A replacement error must not turn
            // a resumed step into a fresh optional-choice collection.
            *self = runner_checkpoint;
            game.restore_execution_checkpoint(game_checkpoint, false);
            *tq = queue_checkpoint;
        }
        result
    }

    fn advance_inner(
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
            TurnState::Untap if self.pending_untap_boundary.is_none() => Some((Step::Untap, false)),
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
                self.skipped_phase_boundary = Some((step, phase, normal_next));
                // The skipped step does not exist, but the containing phase
                // still ends. Retain that exact continuation across choices.
                TurnState::SkippedPhaseEndMana
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
                // CR 726.4: a restarting effect's deferred battlefield entries
                // happen just before the new game's first untap step (hosts
                // that run the rule 103 procedure apply them when it ends).
                if !game.pending_restart_battlefield_entries().is_empty()
                    && let Some(prompt) = self.apply_restart_battlefield_entries_with_choices(game)
                {
                    return Ok(TurnAction::Decision(prompt));
                }
                game.record_turn_start_hand_sizes();
                for player in game.turn_players() {
                    game.activate_pending_player_control(player);
                }

                game.establish_turn_start_continuous_control();

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

                self.pending_untap_choices = None;
                self.pending_untap_boundary = None;
                self.pending_boolean = None;
                if let Some(prompt) = self.run_untap_step_with_choices(game, Vec::new())? {
                    self.state = TurnState::Untap;
                    return Ok(TurnAction::Decision(prompt));
                }

                self.state = TurnState::UntapEndMana;
                Ok(TurnAction::Continue)
            }

            TurnState::Untap => {
                let mut pending =
                    self.pending_untap_choices
                        .take()
                        .unwrap_or_else(|| PendingUntapChoices {
                            answers: Vec::new(),
                            prompt: None,
                            response: self.pending_boolean.take().map(AttackCostAnswer::Boolean),
                        });
                if let Some(answer) = pending.response.take() {
                    pending.answers.push(answer);
                } else if let Some(prompt) = pending.prompt.clone() {
                    self.pending_untap_choices = Some(pending);
                    return Ok(TurnAction::Decision(prompt));
                }
                if let Some(prompt) = self.run_untap_step_with_choices(game, pending.answers)? {
                    return Ok(TurnAction::Decision(prompt));
                }
                self.state = TurnState::UntapEndMana;
                Ok(TurnAction::Continue)
            }

            TurnState::SkippedPhaseEndMana => {
                let (step, phase, next) = self.skipped_phase_boundary.clone().ok_or_else(|| {
                    GameLoopError::InvalidState("missing skipped-phase mana continuation".into())
                })?;
                game.turn.phase = phase;
                game.turn.step = Some(if phase == Phase::Combat {
                    Step::EndCombat
                } else {
                    step
                });
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
                if phase == Phase::Combat {
                    self.sync_combat_from_game(game);
                    game.retain_ending_combat(&self.combat);
                    crate::combat_state::end_combat(&mut self.combat);
                    game.combat = Some(self.combat.clone());
                    game.cleanup_effects_end_of_combat();
                }
                self.skipped_phase_boundary = None;
                self.state = finish_step_and_phase(game, step, phase, next);
                Ok(TurnAction::Continue)
            }

            TurnState::UntapEndMana => {
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                try_drain_pending_trigger_events(game, tq)?;
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::UpkeepPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::UpkeepPriority => {
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                game.refresh_continuous_state()
                    .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
                // CR 614.10 / 500.11: "Skip your draw step" (Necropotence)
                // proceeds past the whole step as though it didn't exist: no
                // turn-based draw, no "beginning of draw step" triggers and no
                // priority window. Anything waiting for the draw step waits
                // for the next one that actually happens (CR 614.10a).
                if self.remaining_draw_players.is_empty()
                    && self.pending_draw_replacement.is_none()
                    && self.pending_draw_replacement_effects.is_none()
                    && self.pending_draw_reveal.is_none()
                    && self.pending_commander_choice.is_none()
                    && game
                        .turn_players()
                        .iter()
                        .any(|player| game.player_skips_draw_step(*player))
                {
                    game.reset_priority_for_new_window();
                    self.state = finish_step_and_phase(
                        game,
                        Step::Draw,
                        Phase::Beginning,
                        TurnScheduleDestination::Phase(Phase::FirstMain),
                    );
                    return Ok(TurnAction::Continue);
                }
                let draw_events = match self.execute_draw_step_with_choices(game)? {
                    RunnerProgress::Complete(draw_events) => draw_events,
                    RunnerProgress::NeedsDecision(ctx) => return Ok(TurnAction::Decision(ctx)),
                };
                crate::game_loop::try_drain_pending_trigger_events(game, tq)?;
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
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                game.mark_main_phase_started();
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
                self.state = TurnState::FirstMainSagas;
                self.advance_saga_lore(game, tq)
            }

            TurnState::FirstMainSagas => self.advance_saga_lore(game, tq),

            TurnState::FirstMainAttractions => self.advance_attraction_roll(game, tq),

            TurnState::FirstMainPriority => {
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                    || game
                        .turn_store
                        .pending_combat_phase_skips
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
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
                // Creatures put onto the battlefield attacking during this
                // step were recorded in `game.combat` (CR 508.4).
                self.sync_combat_from_game(game);
                self.state = finish_step(
                    game,
                    Step::BeginCombat,
                    TurnScheduleDestination::Step(Step::DeclareAttackers),
                );
                Ok(TurnAction::Continue)
            }

            TurnState::DeclareAttackersDecision => {
                // An added declaration step can share the same combat phase.
                // Synchronize live combat, then invalidate only step evidence;
                // phase-wide melee history and existing attackers remain intact.
                self.sync_combat_from_game(game);
                self.combat.last_attack_declaration_step_players = None;
                game.combat = Some(self.combat.clone());
                game.turn.step = Some(Step::DeclareAttackers);
                game.reset_priority_for_new_window();
                self.pending_attacker_optional_costs = None;
                self.pending_attacker_mana_window = None;
                self.pending_attacker_payment_choices = None;
                self.pending_combat_mana_choices = None;
                self.pending_option = None;
                self.pending_boolean = None;
                self.pending_discard = None;
                self.pending_draw_replacement = None;

                // Refresh continuous state for the new step before combat
                // queries so conditional abilities share the characteristics
                // cache instead of recursively rebuilding it.
                game.refresh_continuous_state()
                    .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
                let ctx = get_declare_attackers_decision(game, &self.combat);
                self.state = TurnState::DeclareAttackersApply;
                Ok(TurnAction::Decision(ctx))
            }

            TurnState::DeclareAttackersApply => {
                if let Some(mut pending) = self.pending_attacker_payment_choices.take() {
                    let Some(answer) = pending.response.take() else {
                        let prompt = pending.prompt.clone();
                        self.pending_attacker_payment_choices = Some(pending);
                        return Ok(TurnAction::Decision(prompt));
                    };
                    pending.answers.push(answer);
                    if let Some(action) = self.finish_attack_payment_with_choices(
                        pending.transaction,
                        pending.answers,
                        game,
                        tq,
                    )? {
                        return Ok(action);
                    }
                } else if let Some(pending) = self.pending_attacker_mana_window.take() {
                    let choice = self.pending_option.take();
                    let window_closed = match self.apply_combat_mana_with_choices(
                        game,
                        tq,
                        game.turn.active_player,
                        choice,
                        true,
                    ) {
                        Ok(RunnerProgress::Complete(closed)) => closed,
                        Ok(RunnerProgress::NeedsDecision(prompt)) => {
                            self.pending_attacker_mana_window = Some(pending);
                            return Ok(TurnAction::Decision(prompt));
                        }
                        Err(err) => {
                            self.pending_attacker_mana_window = Some(pending);
                            return Err(err);
                        }
                    };

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

                    if let Some(action) = self.finish_attack_payment_with_choices(
                        pending.transaction,
                        pending.optional_cost_answers,
                        game,
                        tq,
                    )? {
                        return Ok(action);
                    }
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
                    if pending.requires_mana_window {
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
                        if let Some(action) = self.finish_attack_payment_with_choices(
                            mana_pending.transaction,
                            mana_pending.optional_cost_answers,
                            game,
                            tq,
                        )? {
                            return Ok(action);
                        }
                    } else {
                        if let Some(action) = self.finish_attack_payment_with_choices(
                            pending.transaction,
                            pending.answers,
                            game,
                            tq,
                        )? {
                            return Ok(action);
                        }
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
                    let requires_mana_window =
                        preview_attack_cost_needs_mana_window(game, &self.combat, &declarations)?;

                    if !prompts.is_empty() || requires_mana_window {
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
                                    requires_mana_window,
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
                        if let Some(action) = self.finish_attack_payment_with_choices(
                            pending.transaction,
                            Vec::new(),
                            game,
                            tq,
                        )? {
                            return Ok(action);
                        }
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
                crate::game_loop::try_drain_pending_trigger_events(game, tq)?;
                // The caller's priority loop puts these triggers on the stack
                // with its interactive decision maker. Stacking here with the
                // default chooser silently selects modes, targets, and order.

                // Also sync game.combat for anything that reads it
                game.combat = Some(self.combat.clone());

                self.state = TurnState::DeclareAttackersPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::DeclareAttackersPriority => {
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                self.pending_combat_mana_choices = None;
                self.pending_blocker_preparation_choices = None;
                self.pending_blocker_payment_choices = None;
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
                    self.declared_block_pairs.clear();
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

                game.refresh_continuous_state()
                    .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
                let ctx = get_declare_blockers_decision(game, &self.combat, defending_player);
                self.state = TurnState::DeclareBlockersApply;
                Ok(TurnAction::Decision(ctx))
            }

            TurnState::DeclareBlockersApply => {
                let defending_player;
                if let Some(mut pending) = self.pending_blocker_payment_choices.take() {
                    let Some(response) = pending.response.take() else {
                        let prompt = pending.prompt.clone();
                        self.pending_blocker_payment_choices = Some(pending);
                        return Ok(TurnAction::Decision(prompt));
                    };
                    pending.answers.push(response);
                    defending_player = pending.transaction.defending_player().ok_or_else(|| {
                        GameLoopError::InvalidState(
                            "blocking-cost transaction has no defending player".to_string(),
                        )
                    })?;
                    if let Some(action) = self.finish_block_payment_with_choices(
                        pending.transaction,
                        pending.answers,
                        game,
                        tq,
                    )? {
                        return Ok(action);
                    }
                } else if let Some(mut pending) = self.pending_blocker_mana_window.take() {
                    let payer =
                        pending
                            .payers
                            .get(pending.next_payer)
                            .copied()
                            .ok_or_else(|| {
                                GameLoopError::InvalidState(
                                    "blocking-cost mana window has no current payer".to_string(),
                                )
                            })?;
                    let choice = self.pending_option.take();
                    match self.apply_combat_mana_with_choices(game, tq, payer, choice, false) {
                        Ok(RunnerProgress::Complete(true)) => pending.next_payer += 1,
                        Ok(RunnerProgress::Complete(false)) => {}
                        Ok(RunnerProgress::NeedsDecision(prompt)) => {
                            self.pending_blocker_mana_window = Some(pending);
                            return Ok(TurnAction::Decision(prompt));
                        }
                        Err(error) => {
                            self.pending_blocker_mana_window = Some(pending);
                            return Err(error);
                        }
                    }
                    if let Some(context) = next_blocker_mana_window_context(game, &mut pending) {
                        self.pending_blocker_mana_window = Some(pending);
                        return Ok(TurnAction::Decision(DecisionContext::SelectOptions(
                            context,
                        )));
                    }
                    defending_player = pending.transaction.defending_player().ok_or_else(|| {
                        GameLoopError::InvalidState(
                            "blocking-cost transaction has no defending player".to_string(),
                        )
                    })?;
                    if let Some(action) = self.finish_block_payment_with_choices(
                        pending.transaction,
                        Vec::new(),
                        game,
                        tq,
                    )? {
                        return Ok(action);
                    }
                } else {
                    let (declarations, player, answers) = if let Some(mut pending) =
                        self.pending_blocker_preparation_choices.take()
                    {
                        let Some(response) = pending.response.take() else {
                            let prompt = pending.prompt.clone();
                            self.pending_blocker_preparation_choices = Some(pending);
                            return Ok(TurnAction::Decision(prompt));
                        };
                        pending.answers.push(response);
                        (
                            pending.declarations,
                            pending.defending_player,
                            pending.answers,
                        )
                    } else {
                        let (declarations, player) =
                            self.pending_blockers.take().unwrap_or_else(|| {
                                (
                                    Vec::new(),
                                    self.defending_player.unwrap_or(game.turn.active_player),
                                )
                            });
                        (declarations, player, Vec::new())
                    };
                    defending_player = player;
                    if self.defending_player != Some(defending_player) {
                        return Err(crate::decision::ResponseError::InvalidBlockers(
                            "blocker declaration was submitted for the wrong defending player"
                                .to_string(),
                        )
                        .into());
                    }
                    let transaction = match self.prepare_block_payment_with_choices(
                        declarations,
                        defending_player,
                        answers,
                        game,
                        tq,
                    )? {
                        RunnerProgress::Complete(transaction) => transaction,
                        RunnerProgress::NeedsDecision(prompt) => {
                            return Ok(TurnAction::Decision(prompt));
                        }
                    };
                    transaction.stage_proposed_combat_for_payment(game);
                    let payers = transaction.mana_cost_payers();
                    if !payers.is_empty() {
                        let declaration_source =
                            transaction.declaration_source().ok_or_else(|| {
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
                        if let Some(context) = next_blocker_mana_window_context(game, &mut pending)
                        {
                            self.pending_blocker_mana_window = Some(pending);
                            return Ok(TurnAction::Decision(DecisionContext::SelectOptions(
                                context,
                            )));
                        }
                        if let Some(action) = self.finish_block_payment_with_choices(
                            pending.transaction,
                            Vec::new(),
                            game,
                            tq,
                        )? {
                            return Ok(action);
                        }
                    } else if let Some(action) =
                        self.finish_block_payment_with_choices(transaction, Vec::new(), game, tq)?
                    {
                        return Ok(action);
                    }
                }
                if self.remaining_defending_players.first().copied() == Some(defending_player) {
                    self.remaining_defending_players.remove(0);
                }
                self.defending_player = None;
                if !self.remaining_defending_players.is_empty() {
                    self.state = TurnState::DeclareBlockersDecision;
                    return Ok(TurnAction::Continue);
                }
                self.queue_declared_block_events(game, tq);
                // The priority loop announces trigger modes, targets, and order.
                game.combat = Some(self.combat.clone());
                game.reset_priority_for_new_window();
                self.state = TurnState::DeclareBlockersPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::DeclareBlockersPriority => {
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                self.pending_distribution = None;
                self.state = TurnState::CombatDamageFirstStrikeAssign;
                Ok(TurnAction::Continue)
            }

            TurnState::CombatDamageFirstStrikeAssign => {
                if self.pending_combat_damage_choices.is_none()
                    && let Some(ctx) =
                        self.next_combat_damage_assignment_decision(game, true, false)?
                {
                    return Ok(TurnAction::Decision(ctx));
                }
                if let Some(ctx) = self.apply_combat_damage_with_choices(game, tq, true)? {
                    return Ok(TurnAction::Decision(ctx));
                }
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
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                self.pending_distribution = None;
                self.state = TurnState::CombatDamageRegularAssign;
                Ok(TurnAction::Continue)
            }

            TurnState::CombatDamageRegularAssign => {
                if self.pending_combat_damage_choices.is_none()
                    && let Some(ctx) =
                        self.next_combat_damage_assignment_decision(game, false, true)?
                {
                    return Ok(TurnAction::Decision(ctx));
                }
                if let Some(ctx) = self.apply_combat_damage_with_choices(game, tq, false)? {
                    return Ok(TurnAction::Decision(ctx));
                }
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
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
                self.sync_combat_from_game(game);
                game.retain_ending_combat(&self.combat);
                crate::combat_state::end_combat(&mut self.combat);
                game.combat = Some(self.combat.clone());
                game.mark_continuous_state_dirty();
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
                        if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                            return Ok(TurnAction::Decision(prompt));
                        }
                        self.sync_combat_from_game(game);
                        game.retain_ending_combat(&self.combat);
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
                game.mark_main_phase_started();
                game.reset_priority_for_new_window();
                generate_and_queue_step_triggers(game, tq);

                self.state = TurnState::NextMainPriority;
                Ok(TurnAction::RunPriority)
            }

            TurnState::NextMainPriority => {
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
                        let ending_combat = matches!(game.turn.phase, Phase::Combat);
                        if ending_combat {
                            // CR 500.5a / 511.3: combat ends here, so "until end
                            // of combat" mana and effects expire now. Naming the
                            // end-of-combat step lets mana-retention cleanup see
                            // the combat boundary (as in `EndCombatPhaseSbas`).
                            game.turn.step = Some(Step::EndCombat);
                        }
                        if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                            return Ok(TurnAction::Decision(prompt));
                        }
                        self.sync_combat_from_game(game);
                        game.retain_ending_combat(&self.combat);
                        crate::combat_state::end_combat(&mut self.combat);
                        if let Some(combat) = game.combat.as_mut() {
                            crate::combat_state::end_combat(combat);
                        }
                        if ending_combat {
                            game.cleanup_effects_end_of_combat();
                        }
                        // CR 724.1d: every phase and step between here and the
                        // cleanup step is skipped, including additional phases
                        // and steps created earlier this turn. Only steps added
                        // after the cleanup step itself survive.
                        {
                            let store = &mut game.turn_store;
                            store.additional_phases.clear();
                            store.additional_phase_orders.clear();
                            store.additional_phase_only_steps.clear();
                            store.additional_phase_continuation = None;
                            store.phase_schedule_continuation = None;
                            store.pending_added_steps.clear();
                            store.active_added_step = None;
                            store.added_step_continuation = None;
                            store.added_steps.retain(|added| {
                                added.placement
                                    == crate::game_state::AddedStepPlacement::AfterStep(
                                        Step::Cleanup,
                                    )
                            });
                        }
                        game.finish_draw_step_tracking();
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
                // CR 603.2: the cleanup discard triggers against the game
                // before damage is removed and "until end of turn" effects
                // end (CR 514.2).
                try_drain_pending_trigger_events(game, tq)?;
                execute_cleanup_step(game);
                self.state = TurnState::CleanupRecursiveCheck;
                Ok(TurnAction::Continue)
            }

            TurnState::CleanupRecursiveCheck => {
                try_drain_pending_trigger_events(game, tq)?;
                let triggers_fired = !tq.is_empty();
                let sbas_happened = !check_state_based_actions(game).is_empty();

                if triggers_fired || sbas_happened {
                    match self.apply_sbas_until_commander_choice(game, tq)? {
                        RunnerProgress::Complete(()) => {}
                        RunnerProgress::NeedsDecision(ctx) => return Ok(TurnAction::Decision(ctx)),
                    }
                    // CR 514.3a grants the active player priority after either
                    // state-based actions or waiting triggers, even if those
                    // actions left the stack empty.
                    game.reset_priority_for_new_window();
                    self.state = TurnState::CleanupRecursivePriority;
                    Ok(TurnAction::RunPriority)
                } else {
                    if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                        return Ok(TurnAction::Decision(prompt));
                    }
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
                if let Some(prompt) = self.empty_mana_pools_with_choices(game)? {
                    return Ok(TurnAction::Decision(prompt));
                }
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
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Attackers(
                declarations
                    .into_iter()
                    .map(|d| crate::decisions::spec::AttackerDeclaration {
                        creature: d.creature,
                        target: d.target,
                    })
                    .collect(),
            ));
            return;
        }
        self.pending_attackers = Some(declarations);
        self.pending_attacking_bands = Some(bands);
        self.pending_attacker_optional_costs = None;
        self.pending_attacker_mana_window = None;
        self.pending_attacker_payment_choices = None;
        self.pending_combat_mana_choices = None;
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
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Blockers(
                declarations
                    .into_iter()
                    .map(|d| crate::decisions::spec::BlockerDeclaration {
                        blocker: d.blocker,
                        blocking: d.blocking,
                    })
                    .collect(),
            ));
            return;
        }
        self.pending_blockers = Some((declarations, defending_player));
    }

    /// The wire state name cannot carry collected mana-loss answers or a
    /// skipped phase's exact continuation. Native savepoints retain both.
    pub fn has_pending_mana_loss_continuation(&self) -> bool {
        self.pending_mana_loss_choices.is_some() || self.skipped_phase_boundary.is_some()
    }

    /// An actual untap occurrence is suspended with a retained prompt/prefix.
    /// Hosts use this owner to keep a failed response retryable as one command.
    pub fn has_pending_untap_continuation(&self) -> bool {
        self.pending_untap_boundary.is_some() && self.pending_untap_choices.is_some()
    }

    /// True while a turn-based operation is waiting on a nested player choice.
    pub fn has_pending_replay_choice(&self) -> bool {
        self.pending_sba_choices.is_some()
            || self.pending_untap_choices.is_some()
            || self.pending_attraction_choices.is_some()
            || self.pending_restart_entry_choices.is_some()
            || self.pending_draw_replacement_effects.is_some()
            || self.pending_turn_draw.is_some()
            || self.pending_attacker_payment_choices.is_some()
            || self.pending_combat_damage_choices.is_some()
            || self.pending_saga_lore_choices.is_some()
            || self.pending_mana_loss_choices.is_some()
            || self.pending_cleanup_discard.is_some()
            || self.pending_combat_mana_choices.is_some()
            || self.pending_blocker_preparation_choices.is_some()
            || self.pending_blocker_payment_choices.is_some()
    }

    fn replay_response_slot(&mut self) -> Option<&mut Option<AttackCostAnswer>> {
        if self.has_pending_combat_cost_choice() {
            return self.combat_cost_response_slot();
        }
        if let Some(pending) = self.pending_combat_damage_choices.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_mana_loss_choices.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_saga_lore_choices.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_cleanup_discard.as_mut() {
            return Some(&mut pending.choices.response);
        }
        if let Some(pending) = self.pending_sba_choices.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_untap_choices.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_attraction_choices.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_restart_entry_choices.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_draw_replacement_effects.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_turn_draw.as_mut() {
            return Some(&mut pending.choices.response);
        }
        if let Some(pending) = self.pending_attacker_payment_choices.as_mut() {
            return Some(&mut pending.response);
        }
        None
    }

    pub fn respond_discard(&mut self, answer: Vec<ObjectId>) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Objects(answer));
            return;
        }
        self.pending_discard = Some(answer);
    }

    pub fn respond_boolean(&mut self, answer: bool) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Boolean(answer));
            return;
        }
        self.pending_boolean = Some(answer);
    }

    pub fn respond_options(&mut self, answer: Vec<usize>) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Options(answer));
            return;
        }
        self.pending_option = answer.first().copied();
    }

    pub fn respond_order(&mut self, answer: Vec<ObjectId>) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Order(answer));
            return;
        }
    }

    pub fn respond_distribute(&mut self, answer: Vec<(crate::game_state::Target, u32)>) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Distribute(answer));
            return;
        }
        self.pending_distribution = Some(answer);
    }

    pub fn respond_number(&mut self, answer: u32) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Number(answer));
            return;
        }
    }

    pub fn respond_text(&mut self, answer: String) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Text(answer));
            return;
        }
    }

    pub fn respond_colors(&mut self, answer: Vec<crate::color::Color>) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Colors(answer));
            return;
        }
    }

    pub fn respond_counters(&mut self, answer: Vec<(crate::object::CounterType, u32)>) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Counters(answer));
            return;
        }
    }

    pub fn respond_partition(&mut self, answer: Vec<ObjectId>) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Partition(answer));
            return;
        }
    }

    pub fn respond_proliferate(&mut self, answer: crate::decisions::specs::ProliferateResponse) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Proliferate(answer));
            return;
        }
    }

    pub fn respond_targets(&mut self, answer: Vec<crate::game_state::Target>) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Targets(answer));
            return;
        }
    }

    pub fn respond_priority(&mut self, answer: crate::decision::LegalAction) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::Priority(answer));
        }
    }

    /// Check a proposed division for the combat-damage prompt the runner is
    /// currently waiting on, without recording it (CR 510.1e).
    pub fn validate_combat_damage_distribution(
        &self,
        game: &GameState,
        distribution: &[(crate::game_state::Target, u32)],
    ) -> Result<(), String> {
        let prompt = self
            .current_combat_damage_assignment_prompt(game)
            .ok_or_else(|| "no combat-damage assignment is pending".to_string())?;
        prompt.validate(game, distribution).map(|_| ())
    }

    fn current_combat_damage_assignment_prompt(
        &self,
        game: &GameState,
    ) -> Option<crate::game_loop::CombatDamageAssignmentPrompt> {
        let (first_strike, use_snapshot) = match self.state {
            TurnState::CombatDamageFirstStrikeAssign => (true, false),
            TurnState::CombatDamageRegularAssign => (false, true),
            _ => return None,
        };
        crate::game_loop::next_combat_damage_assignment_prompt(
            game,
            &self.combat,
            first_strike,
            use_snapshot.then_some(&self.first_step_strikers),
        )
    }

    /// Record a pending division (if any), then return the next division
    /// decision the players must make before this damage step (CR 510.1).
    fn next_combat_damage_assignment_decision(
        &mut self,
        game: &mut GameState,
        first_strike: bool,
        use_snapshot: bool,
    ) -> Result<Option<DecisionContext>, GameLoopError> {
        game.refresh_continuous_state()
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
        // The refresh may have removed permanents from combat (CR 506.4).
        self.sync_combat_from_game(game);
        let first_step_strikers = use_snapshot.then_some(&self.first_step_strikers);
        if let Some(distribution) = self.pending_distribution.take()
            && let Some(prompt) = crate::game_loop::next_combat_damage_assignment_prompt(
                game,
                &self.combat,
                first_strike,
                first_step_strikers,
            )
        {
            prompt.record(game, &distribution);
        }
        Ok(crate::game_loop::next_combat_damage_assignment_prompt(
            game,
            &self.combat,
            first_strike,
            first_step_strikers,
        )
        .map(|prompt| DecisionContext::Distribute(prompt.decision_context(game))))
    }

    /// Signal that the priority loop has completed.
    pub fn priority_done(&mut self) {
        // This is a no-op on the runner itself; the state transition
        // happens in advance() when the *Priority state is re-entered.
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    fn combat_cost_response_slot(&mut self) -> Option<&mut Option<AttackCostAnswer>> {
        if let Some(pending) = self.pending_combat_mana_choices.as_mut() {
            return Some(&mut pending.response);
        }
        if let Some(pending) = self.pending_blocker_preparation_choices.as_mut() {
            return Some(&mut pending.response);
        }
        self.pending_blocker_payment_choices
            .as_mut()
            .map(|pending| &mut pending.response)
    }

    fn has_pending_combat_cost_choice(&self) -> bool {
        self.pending_combat_mana_choices.is_some()
            || self.pending_blocker_preparation_choices.is_some()
            || self.pending_blocker_payment_choices.is_some()
    }

    /// Mana abilities can ask for colors, objects, or other choices of their
    /// own. Replay the activation on a private copy until all are answered;
    /// opening a prompt must not tap its source or spend an activation cost.
    fn apply_combat_mana_with_choices(
        &mut self,
        game: &mut GameState,
        tq: &mut TriggerQueue,
        player: PlayerId,
        choice: Option<usize>,
        attack: bool,
    ) -> Result<RunnerProgress<bool>, GameLoopError> {
        let (choice, answers) = if let Some(mut pending) = self.pending_combat_mana_choices.take() {
            if pending.player != player || pending.attack != attack {
                self.pending_combat_mana_choices = Some(pending);
                return Err(GameLoopError::InvalidState(
                    "combat mana choice resumed for a different declaration".to_string(),
                ));
            }
            let Some(response) = pending.response.take() else {
                let prompt = pending.prompt.clone();
                self.pending_combat_mana_choices = Some(pending);
                return Ok(RunnerProgress::NeedsDecision(prompt));
            };
            pending.answers.push(response);
            (pending.choice, pending.answers)
        } else {
            let Some(choice) = choice else {
                return Ok(RunnerProgress::Complete(false));
            };
            (choice, Vec::new())
        };

        let mut activation_game = game.clone();
        let mut activation_triggers = tq.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        dm.capture_mana_payment = true;
        let result = if attack {
            apply_attack_mana_ability_window_response(
                &mut activation_game,
                &mut activation_triggers,
                player,
                choice,
                &mut dm,
            )
        } else {
            apply_blocker_mana_ability_window_response(
                &mut activation_game,
                &mut activation_triggers,
                player,
                choice,
                &mut dm,
            )
        };
        if let Some(prompt) = dm.pending_prompt {
            self.pending_combat_mana_choices = Some(PendingCombatManaChoices {
                player,
                choice,
                attack,
                answers,
                prompt: prompt.clone(),
                response: None,
            });
            return Ok(RunnerProgress::NeedsDecision(prompt));
        }
        let closed = result?;
        *game = activation_game;
        *tq = activation_triggers;
        Ok(RunnerProgress::Complete(closed))
    }

    fn prepare_block_payment_with_choices(
        &mut self,
        declarations: Vec<BlockerDeclaration>,
        defending_player: PlayerId,
        answers: Vec<AttackCostAnswer>,
        game: &GameState,
        tq: &TriggerQueue,
    ) -> Result<RunnerProgress<BlockDeclarationTransaction>, GameLoopError> {
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        let result = begin_blocker_declaration_transaction(
            game,
            &self.combat,
            tq,
            &declarations,
            defending_player,
            &mut dm,
        );
        if let Some(prompt) = dm.pending_prompt {
            self.pending_blocker_preparation_choices = Some(PendingBlockerPreparationChoices {
                declarations,
                defending_player,
                answers,
                prompt: prompt.clone(),
                response: None,
            });
            return Ok(RunnerProgress::NeedsDecision(prompt));
        }
        match result {
            Ok(transaction) => Ok(RunnerProgress::Complete(transaction)),
            Err(error) => {
                self.state = TurnState::DeclareBlockersDecision;
                Err(error)
            }
        }
    }

    /// As with attack payments, each blocking-cost choice is tentative until
    /// the complete payment succeeds. This also preserves the declaration's
    /// original rollback checkpoint if a later cost cannot be paid.
    fn finish_block_payment_with_choices(
        &mut self,
        transaction: BlockDeclarationTransaction,
        answers: Vec<AttackCostAnswer>,
        game: &mut GameState,
        tq: &mut TriggerQueue,
    ) -> Result<Option<TurnAction>, GameLoopError> {
        let mut payment_game = game.clone();
        let mut payment_combat = self.combat.clone();
        let mut payment_triggers = tq.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        let result = finish_blocker_declaration_transaction_deferring_triggers(
            transaction.clone(),
            &mut payment_game,
            &mut payment_combat,
            &mut payment_triggers,
            &mut dm,
        );
        if let Some(prompt) = dm.pending_prompt {
            self.pending_blocker_payment_choices = Some(PendingBlockerPaymentChoices {
                transaction,
                answers,
                prompt: prompt.clone(),
                response: None,
            });
            return Ok(Some(TurnAction::Decision(prompt)));
        }
        *game = payment_game;
        self.combat = payment_combat;
        *tq = payment_triggers;
        match result {
            Ok(pairs) => self.declared_block_pairs.extend(pairs),
            Err(error) => {
                self.state = TurnState::DeclareBlockersDecision;
                return Err(error);
            }
        }
        Ok(None)
    }

    /// Replay answers against a private payment transaction. Publish only a
    /// completed payment or its rollback; prompts never spend real resources.
    fn finish_attack_payment_with_choices(
        &mut self,
        transaction: AttackDeclarationTransaction,
        answers: Vec<AttackCostAnswer>,
        game: &mut GameState,
        tq: &mut TriggerQueue,
    ) -> Result<Option<TurnAction>, GameLoopError> {
        let mut payment_game = game.clone();
        let mut payment_combat = self.combat.clone();
        let mut payment_triggers = tq.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        let result = finish_attack_declaration_transaction(
            transaction.clone(),
            &mut payment_game,
            &mut payment_combat,
            &mut payment_triggers,
            &mut dm,
        );
        if let Some(prompt) = dm.pending_prompt {
            self.pending_attacker_payment_choices = Some(PendingAttackerPaymentChoices {
                transaction,
                answers,
                prompt: prompt.clone(),
                response: None,
            });
            return Ok(Some(TurnAction::Decision(prompt)));
        }
        *game = payment_game;
        self.combat = payment_combat;
        *tq = payment_triggers;
        result?;
        Ok(None)
    }

    fn sync_combat_from_game(&mut self, game: &GameState) {
        if let Some(combat) = &game.combat {
            self.combat = combat.clone();
        }
    }

    /// Resolve one whole boundary on a private state. A replacement choice
    /// must not publish an earlier player's loss or duration expiry first.
    fn empty_mana_pools_with_choices(
        &mut self,
        game: &mut GameState,
    ) -> Result<Option<DecisionContext>, GameLoopError> {
        let mut answers = Vec::new();
        if let Some(mut pending) = self.pending_mana_loss_choices.take() {
            let Some(answer) = pending.response.take() else {
                let prompt = pending.prompt.clone();
                self.pending_mana_loss_choices = Some(pending);
                return Ok(Some(prompt));
            };
            answers = pending.answers;
            answers.push(answer);
        }
        let mut trial = game.clone();
        // CR 500.5: duration expiry precedes mana loss. Keep both mutations
        // private until every affected player has supplied an ordering choice.
        if trial.turn.phase == Phase::Combat && trial.turn.step == Some(Step::EndCombat) {
            trial.cleanup_effects_end_of_combat();
        }
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        let result = trial.empty_mana_pools_with_dm(&mut dm);
        if let Some(prompt) = dm.pending_prompt {
            self.pending_mana_loss_choices = Some(PendingTurnActionChoices {
                answers,
                prompt: prompt.clone(),
                response: None,
            });
            return Ok(Some(prompt));
        }
        result?;
        *game = trial;
        Ok(None)
    }

    /// Resolve the entire simultaneous damage batch on a private state. A
    /// replacement choice must never publish some creatures' damage first.
    fn apply_combat_damage_with_choices(
        &mut self,
        game: &mut GameState,
        tq: &mut TriggerQueue,
        first_strike: bool,
    ) -> Result<Option<DecisionContext>, GameLoopError> {
        let mut answers = Vec::new();
        if let Some(mut pending) = self.pending_combat_damage_choices.take() {
            let Some(answer) = pending.response.take() else {
                let prompt = pending.prompt.clone();
                self.pending_combat_damage_choices = Some(pending);
                return Ok(Some(prompt));
            };
            answers = pending.answers;
            answers.push(answer);
        }
        let mut hypothetical = game.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        let result =
            crate::game_loop::try_execute_combat_damage_step_with_dm_and_first_step_snapshot(
                &mut hypothetical,
                &self.combat,
                first_strike,
                (!first_strike).then_some(&self.first_step_strikers),
                &mut dm,
            );
        if let Some(prompt) = dm.pending_prompt {
            self.pending_combat_damage_choices = Some(PendingTurnActionChoices {
                answers,
                prompt: prompt.clone(),
                response: None,
            });
            return Ok(Some(prompt));
        }
        let events = result.map_err(|error| GameLoopError::InvalidState(error.to_string()))?;
        let mut hypothetical_triggers = tq.clone();
        try_queue_combat_damage_triggers(&mut hypothetical, &events, &mut hypothetical_triggers)?;
        *game = hypothetical;
        *tq = hypothetical_triggers;
        Ok(None)
    }

    fn advance_saga_lore(
        &mut self,
        game: &mut GameState,
        tq: &mut TriggerQueue,
    ) -> Result<TurnAction, GameLoopError> {
        let mut answers = Vec::new();
        if let Some(mut pending) = self.pending_saga_lore_choices.take() {
            let Some(answer) = pending.response.take() else {
                let prompt = pending.prompt.clone();
                self.pending_saga_lore_choices = Some(pending);
                return Ok(TurnAction::Decision(prompt));
            };
            answers = pending.answers;
            answers.push(answer);
        }
        let mut hypothetical = game.clone();
        let mut hypothetical_triggers = tq.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        let result = crate::game_loop::add_saga_lore_counters_with_dm(
            &mut hypothetical,
            &mut hypothetical_triggers,
            &mut dm,
        );
        if let Some(prompt) = dm.pending_prompt {
            self.pending_saga_lore_choices = Some(PendingTurnActionChoices {
                answers,
                prompt: prompt.clone(),
                response: None,
            });
            return Ok(TurnAction::Decision(prompt));
        }
        result.map_err(|error| GameLoopError::InvalidState(error.to_string()))?;
        *game = hypothetical;
        *tq = hypothetical_triggers;
        // CR 505.5: Attractions follow the completed Saga turn-based action.
        self.state = TurnState::FirstMainAttractions;
        self.advance_attraction_roll(game, tq)
    }

    fn advance_cleanup_discard(
        &mut self,
        game: &mut GameState,
    ) -> Result<TurnAction, GameLoopError> {
        self.advance_cleanup_discard_with_choices(game, false)
    }

    fn advance_cleanup_discard_recursive(
        &mut self,
        game: &mut GameState,
    ) -> Result<TurnAction, GameLoopError> {
        self.advance_cleanup_discard_with_choices(game, true)
    }

    /// Keep the whole selected discard batch in hand while collecting every
    /// replacement choice. Initial and recursive cleanup share this path.
    fn advance_cleanup_discard_with_choices(
        &mut self,
        game: &mut GameState,
        recursive: bool,
    ) -> Result<TurnAction, GameLoopError> {
        let checkpoint = self.clone();
        let result = self.advance_cleanup_discard_with_choices_inner(game, recursive);
        if result.is_err() {
            *self = checkpoint;
        }
        result
    }

    fn advance_cleanup_discard_with_choices_inner(
        &mut self,
        game: &mut GameState,
        recursive: bool,
    ) -> Result<TurnAction, GameLoopError> {
        let (cards, answers) = if let Some(mut pending) = self.pending_cleanup_discard.take() {
            let Some(answer) = pending.choices.response.take() else {
                let prompt = pending.choices.prompt.clone();
                self.pending_cleanup_discard = Some(pending);
                return Ok(TurnAction::Decision(prompt));
            };
            pending.choices.answers.push(answer);
            (pending.cards, pending.choices.answers)
        } else if let Some(cards) = self.pending_discard.take() {
            (cards, Vec::new())
        } else {
            if let Some((player, spec)) = crate::turn::get_cleanup_discard_spec(game) {
                use crate::decisions::DecisionSpec;
                return Ok(TurnAction::Decision(spec.build_context(player, None, game)));
            }
            self.state = TurnState::CleanupApply;
            return Ok(TurnAction::Continue);
        };

        let discarding_player = cleanup_discard_owner(game, &cards);
        let mut hypothetical = game.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        crate::turn::apply_cleanup_discard(&mut hypothetical, &cards, &mut dm)
            .map_err(GameLoopError::ExecutionFailed)?;
        if let Some(prompt) = dm.pending_prompt {
            self.pending_cleanup_discard = Some(PendingCleanupDiscard {
                cards,
                choices: PendingTurnActionChoices {
                    answers,
                    prompt: prompt.clone(),
                    response: None,
                },
            });
            return Ok(TurnAction::Decision(prompt));
        }
        *game = hypothetical;
        // Each teammate of a shared turn discards to their own hand size.
        if let Some(player) = discarding_player
            && let Some((next, spec)) =
                crate::turn::get_cleanup_discard_spec_after(game, Some(player))
        {
            use crate::decisions::DecisionSpec;
            self.state = if recursive {
                TurnState::CleanupRecursiveDiscard
            } else {
                TurnState::CleanupDiscard
            };
            return Ok(TurnAction::Decision(spec.build_context(next, None, game)));
        }
        self.state = TurnState::CleanupApply;
        Ok(TurnAction::Continue)
    }

    fn execute_draw_step_with_choices(
        &mut self,
        game: &mut GameState,
    ) -> Result<RunnerProgress<Vec<crate::triggers::TriggerEvent>>, GameLoopError> {
        if self.draw_step_checkpoint.is_none() {
            self.draw_step_checkpoint = Some(Box::new(self.clone()));
        }
        let mut working = self
            .draw_step_working_game
            .take()
            .unwrap_or_else(|| Box::new(game.clone()));
        match self.execute_draw_step_on_working_game(&mut working) {
            Ok(RunnerProgress::Complete(events)) => {
                *game = *working;
                self.draw_step_checkpoint = None;
                Ok(RunnerProgress::Complete(events))
            }
            Ok(RunnerProgress::NeedsDecision(prompt)) => {
                self.draw_step_working_game = Some(working);
                Ok(RunnerProgress::NeedsDecision(prompt))
            }
            Err(error) => {
                if let Some(checkpoint) = self.draw_step_checkpoint.take() {
                    *self = *checkpoint;
                }
                Err(error)
            }
        }
    }

    fn execute_draw_step_on_working_game(
        &mut self,
        game: &mut GameState,
    ) -> Result<RunnerProgress<Vec<crate::triggers::TriggerEvent>>, GameLoopError> {
        if self.remaining_draw_players.is_empty() {
            let active_players = game.turn_players();
            if active_players
                .iter()
                .any(|player| game.player_skips_draw_step(*player))
            {
                game.reset_priority_for_new_window();
                return Ok(RunnerProgress::Complete(Vec::new()));
            }
            self.remaining_draw_players = active_players;
            self.shared_draw_events.clear();
        }

        loop {
            let Some(active_player) = self.remaining_draw_players.first().copied() else {
                game.reset_priority_for_new_window();
                return Ok(RunnerProgress::Complete(std::mem::take(
                    &mut self.shared_draw_events,
                )));
            };
            match self.execute_draw_step_for_player_with_choices(game, active_player)? {
                RunnerProgress::NeedsDecision(ctx) => {
                    return Ok(RunnerProgress::NeedsDecision(ctx));
                }
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
    ) -> Result<RunnerProgress<Vec<crate::triggers::TriggerEvent>>, GameLoopError> {
        game.sync_draw_step_tracking();
        if let Some(pending) = self.pending_turn_draw.take() {
            if pending.player != active_player {
                return Err(GameLoopError::ResolutionFailed(
                    "draw continuation belongs to another player".into(),
                ));
            }
            return self.run_turn_draw_with_choices(game, pending);
        }
        if !game
            .player(active_player)
            .is_some_and(|player| player.is_in_game())
            || game.player_skips_draw_step(active_player)
            || game.should_skip_first_turn_draw(active_player)
            || !game.can_draw(active_player)
        {
            return Ok(RunnerProgress::Complete(Vec::new()));
        }
        let has_drawn = game.turn_store.turn_history.has_drawn_cards_this_turn(active_player)
            .map_err(GameLoopError::ExecutionFailed)?;
        if !game.can_draw_extra_cards(active_player) && has_drawn {
            return Ok(RunnerProgress::Complete(Vec::new()));
        }
        self.run_turn_draw_with_choices(
            game,
            PendingTurnDraw {
                player: active_player,
                count: 1,
                choices: PendingUntapChoices {
                    answers: Vec::new(),
                    prompt: None,
                    response: None,
                },
            },
        )
    }

    fn run_turn_draw_with_choices(
        &mut self,
        game: &mut GameState,
        mut pending: PendingTurnDraw,
    ) -> Result<RunnerProgress<Vec<crate::triggers::TriggerEvent>>, GameLoopError> {
        if let Some(answer) = pending.choices.response.take() {
            pending.choices.answers.push(answer);
        } else if let Some(prompt) = pending.choices.prompt.clone() {
            self.pending_turn_draw = Some(pending);
            return Ok(RunnerProgress::NeedsDecision(prompt));
        }
        let mut hypothetical = game.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(pending.choices.answers.clone());
        let provenance = hypothetical
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::CardsDrawn);
        let outcome = {
            let mut ctx = crate::effects::ExecutionContext::new(
                ObjectId::from_raw(0),
                pending.player,
                &mut dm,
            )
            .with_cause(crate::events::cause::EventCause::from_game_rule())
            .with_provenance(provenance);
            crate::effects::cards::execute_turn_draw_proposal(
                &mut hypothetical,
                &mut ctx,
                pending.player,
            )
        }
        .map_err(GameLoopError::ExecutionFailed)?;
        if let Some(prompt) = dm.pending_prompt.take() {
            pending.choices.prompt = Some(prompt.clone());
            self.pending_turn_draw = Some(pending);
            return Ok(RunnerProgress::NeedsDecision(prompt));
        }
        for event in &outcome.events {
            hypothetical.stage_turn_history_event(event);
        }
        *game = hypothetical;
        game.reset_priority_for_new_window();
        Ok(RunnerProgress::Complete(outcome.events))
    }

    fn finish_turn_draw(
        &mut self,
        game: &mut GameState,
        active_player: PlayerId,
        drawn: Vec<ObjectId>,
        current_draws: u32,
    ) -> Result<RunnerProgress<Vec<crate::triggers::TriggerEvent>>, GameLoopError> {
        if !drawn.is_empty() {
            let draw_event_provenance = game
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::CardsDrawn);
            let candidates = crate::effects::cards::collect_automatic_draw_reveal_candidates(
                game,
                active_player,
                &drawn,
                current_draws,
            ).map_err(GameLoopError::ExecutionFailed)?;
            return self.finish_pending_draw_reveal_choices(
                game,
                PendingDrawRevealChoice {
                    active_player,
                    drawn,
                    is_first_draw: current_draws == 0,
                    draw_event_provenance,
                    candidates,
                    next_candidate_index: 0,
                    reveal_events: Vec::new(),
                },
            );
        }

        game.reset_priority_for_new_window();
        Ok(RunnerProgress::Complete(Vec::new()))
    }

    /// Apply a restart's deferred battlefield entries (CR 726.4) on a clone
    /// with the answers collected so far, so the entering cards' choices are
    /// asked of the right players. A new prompt pauses before the untap step.
    fn apply_restart_battlefield_entries_with_choices(
        &mut self,
        game: &mut GameState,
    ) -> Option<DecisionContext> {
        let mut pending = self
            .pending_restart_entry_choices
            .take()
            .unwrap_or_else(|| PendingUntapChoices {
                answers: Vec::new(),
                prompt: None,
                response: None,
            });
        if let Some(answer) = pending.response.take() {
            pending.answers.push(answer);
        } else if let Some(prompt) = pending.prompt.clone() {
            self.pending_restart_entry_choices = Some(pending);
            return Some(prompt);
        }
        let mut hypothetical = game.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(pending.answers.clone());
        hypothetical.apply_pending_restart_battlefield_entries_with(&mut dm);
        if let Some(prompt) = dm.pending_prompt.take() {
            pending.prompt = Some(prompt.clone());
            self.pending_restart_entry_choices = Some(pending);
            return Some(prompt);
        }
        *game = hypothetical;
        None
    }

    pub fn respond_mana_payment(&mut self, response: crate::mana_payment::ManaPaymentResponse) {
        if let Some(slot) = self.replay_response_slot() {
            *slot = Some(AttackCostAnswer::ManaPayment(response));
        }
    }

    fn advance_attraction_roll(
        &mut self,
        game: &mut GameState,
        tq: &mut TriggerQueue,
    ) -> Result<TurnAction, GameLoopError> {
        let mut pending = self
            .pending_attraction_choices
            .take()
            .unwrap_or(PendingUntapChoices {
                answers: Vec::new(),
                prompt: None,
                response: None,
            });
        if let Some(answer) = pending.response.take() {
            pending.answers.push(answer);
        } else if let Some(prompt) = pending.prompt.clone() {
            self.pending_attraction_choices = Some(pending);
            return Ok(TurnAction::Decision(prompt));
        }
        // Replay from the same RNG/forced-roll state. Nothing, including mana,
        // life, modifier usage or trigger history, is committed before all
        // choices complete. Earlier main-phase turn-based actions run once.
        let mut hypothetical = game.clone();
        let mut hypothetical_queue = tq.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(pending.answers.clone());
        dm.capture_mana_payment = true;
        crate::game_loop::roll_to_visit_attractions_with_dm(
            &mut hypothetical,
            &mut hypothetical_queue,
            &mut dm,
        )?;
        if let Some(prompt) = dm.pending_prompt.take() {
            pending.prompt = Some(prompt.clone());
            self.pending_attraction_choices = Some(pending);
            return Ok(TurnAction::Decision(prompt));
        }
        *game = hypothetical;
        *tq = hypothetical_queue;
        self.state = TurnState::FirstMainPriority;
        Ok(TurnAction::RunPriority)
    }

    /// Run the untap step on a clone with the answers collected so far. A new
    /// prompt pauses the step without publishing anything; otherwise the
    /// untapped state is committed.
    fn run_untap_step_with_choices(
        &mut self,
        game: &mut GameState,
        answers: Vec<AttackCostAnswer>,
    ) -> Result<Option<DecisionContext>, GameLoopError> {
        let mut hypothetical = game.clone();
        if self.pending_untap_boundary.is_none() {
            self.pending_untap_boundary = Some(crate::turn::capture_untap_step_boundary(&mut hypothetical)
                .map_err(|error| GameLoopError::ResolutionFailed(error.to_string()))?);
        }
        let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
        crate::turn::execute_untap_step_with_boundary(
            &mut hypothetical, &mut dm, self.pending_untap_boundary.as_ref().unwrap(),
        ).map_err(|error| GameLoopError::ResolutionFailed(error.to_string()))?;
        if let Some(prompt) = dm.pending_prompt.take() {
            self.pending_untap_choices = Some(PendingUntapChoices {
                answers,
                prompt: Some(prompt.clone()),
                response: None,
            });
            return Ok(Some(prompt));
        }
        *game = hypothetical;
        self.pending_untap_boundary = None;
        Ok(None)
    }

    /// Run a draw-step draw's replacement effects with the answers collected
    /// so far. A new prompt pauses the draw step without publishing anything;
    /// otherwise the result (including its events, with CR 614.5 suppression
    /// of the applied replacement) is committed.
    fn run_draw_replacement_effects(
        &mut self,
        game: &mut GameState,
        mut pending: PendingDrawReplacementEffects,
    ) -> Result<RunnerProgress<Vec<crate::triggers::TriggerEvent>>, GameLoopError> {
        let mut hypothetical = game.clone();
        let mut dm = QueuedAttackCostDecisionMaker::new(pending.answers.clone());
        let outcome = {
            let mut ctx =
                crate::effects::ExecutionContext::new(pending.source, pending.controller, &mut dm);
            ctx.iteration.iterated_player = Some(pending.player);
            crate::effects::cards::execute_scoped_draw_replacement_effects(
                &mut hypothetical,
                &mut ctx,
                &pending.effects,
                pending.source,
                pending.controller,
                &pending.context,
            )
        };
        let outcome =
            outcome.map_err(|error| GameLoopError::ResolutionFailed(error.to_string()))?;
        if let Some(prompt) = dm.pending_prompt.take() {
            pending.prompt = Some(prompt.clone());
            pending.response = None;
            self.pending_draw_replacement_effects = Some(pending);
            return Ok(RunnerProgress::NeedsDecision(prompt));
        }
        *game = hypothetical;
        game.reset_priority_for_new_window();
        Ok(RunnerProgress::Complete(outcome.events))
    }

    fn finish_pending_draw_reveal_choices(
        &mut self,
        game: &mut GameState,
        mut pending: PendingDrawRevealChoice,
    ) -> Result<RunnerProgress<Vec<crate::triggers::TriggerEvent>>, GameLoopError> {
        use crate::events::other::CardsDrawnEvent;

        let (is_during_players_draw_step, cards_previously_drawn_this_draw_step) =
            game.draw_step_context_for_player(pending.active_player);

        while let Some(candidate) = pending
            .candidates
            .get(pending.next_candidate_index)
            .cloned()
        {
            // Hidden-information matches: a private drawn card is revealed
            // through the owner-answered draw reveal windows before triggers
            // are put on the stack, so every peer reads the same card.
            if game.hidden_identity_is_private(candidate.card_id) {
                game.defer_hidden_automatic_draw_reveal(
                    crate::effects::cards::pending_hidden_automatic_draw_reveal(&candidate),
                    pending.draw_event_provenance,
                );
                pending.next_candidate_index += 1;
                continue;
            }
            let should_reveal = if candidate.optional {
                if let Some(answer) = self.pending_boolean.take() {
                    answer
                } else {
                    self.pending_draw_reveal = Some(pending);
                    return Ok(RunnerProgress::NeedsDecision(DecisionContext::Boolean(
                        crate::effects::cards::automatic_draw_reveal_boolean_context(&candidate),
                    )));
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

        let event = crate::effects::cards::draw_observation(
            game,
            pending.active_player,
            pending.drawn,
            pending.is_first_draw,
            (
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
        game.note_hidden_draw_for_reveal_window(&event);
        let mut draw_events = vec![event];
        for reveal_event in pending.reveal_events {
            game.stage_turn_history_event(&reveal_event);
            draw_events.push(reveal_event);
        }

        game.reset_priority_for_new_window();
        Ok(RunnerProgress::Complete(draw_events))
    }

    /// CR 509.1: queue every defending player's block events as the one
    /// declare-blockers batch, against the completed blocking configuration.
    fn queue_declared_block_events(&mut self, game: &mut GameState, tq: &mut TriggerQueue) {
        self.combat.block_declaration_complete = true;
        game.combat = Some(self.combat.clone());
        game.mark_continuous_state_dirty();
        let pairs = std::mem::take(&mut self.declared_block_pairs);
        queue_block_declaration_events(game, &self.combat, tq, &pairs, None);
    }

    fn apply_sbas_until_commander_choice(
        &mut self,
        game: &mut GameState,
        tq: &mut TriggerQueue,
    ) -> Result<RunnerProgress<()>, GameLoopError> {
        use crate::rules::state_based::{
            StateBasedAction, StateBasedActionContext, apply_sector_designation_choices_from_group,
            apply_state_based_actions_with_legend_choices, check_state_based_actions_with_context,
            legend_rule_specs_from_actions,
        };

        loop {
            // CR 603.2, 603.10: events that already happened (combat damage
            // removing loyalty counters, the previous check's actions) trigger
            // against the game as it was then. Match them before this check's
            // actions put anything into a graveyard.
            crate::game_loop::try_drain_pending_trigger_events(game, tq)?;
            // Every applied SBA can change which static effects exist. Refresh
            // at the fixed-point boundary; this is a no-op while state is clean.
            game.refresh_continuous_state()
                .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
            let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
            let all_effects = view.effects_arc();
            let context = StateBasedActionContext::from_trigger_queue(tq);
            let actions = check_state_based_actions_with_context(game, &view, &context);
            drop(view);
            if actions.is_empty() {
                game.clear_empty_library_draw_attempts_since_sba();
                // CR 704.5h: deathtouch damage counts only since the last SBA check.
                game.clear_deathtouch_damage_since_sba();
                self.pending_boolean = None;
                self.pending_commander_choice = None;
                self.pending_draw_replacement = None;
                self.pending_legend_choice = None;
                self.pending_sector_designations = None;
                self.pending_sba_choices = None;
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
                    crate::game_loop::try_drain_pending_trigger_events(game, tq)?;
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

            // Collect legend-rule keep choices one prompt at a time, without
            // applying them: CR 704.3 performs every action found by this
            // check, including the legend-rule removals, simultaneously.
            // Violations arrive in APNAP order.
            let legend_specs = legend_rule_specs_from_actions(&actions);
            self.resolved_legend_keeps
                .retain(|(_, group)| legend_specs.iter().any(|(_, spec)| spec.legends == *group));
            if let Some((player, spec)) = legend_specs.into_iter().find(|(_, spec)| {
                !self
                    .resolved_legend_keeps
                    .iter()
                    .any(|(_, group)| *group == spec.legends)
            }) {
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
                        self.resolved_legend_keeps.push((keep_id, spec.legends));
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
            let legend_keeps = std::mem::take(&mut self.resolved_legend_keeps);

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

            if !other_actions.is_empty() || !legend_keeps.is_empty() {
                // CR 616.1: replacement-effect (and other) choices made while
                // these SBAs are performed belong to the affected players.
                // Probe the check on a private copy with the answers so far;
                // publish it only once no new prompt is raised.
                // Choices also arise without a replacement effect, such as
                // ordering the front cards of a melded permanent in a graveyard.
                let applied = {
                    let answers = match self.pending_sba_choices.take() {
                        Some(mut pending) => {
                            let Some(answer) = pending.response.take() else {
                                let prompt = pending.prompt.clone();
                                self.pending_sba_choices = Some(pending);
                                self.resolved_legend_keeps = legend_keeps;
                                return Ok(RunnerProgress::NeedsDecision(prompt));
                            };
                            pending.answers.push(answer);
                            pending.answers
                        }
                        None => Vec::new(),
                    };
                    let mut hypothetical = game.clone();
                    let mut dm = QueuedAttackCostDecisionMaker::new(answers.clone());
                    let applied = apply_state_based_actions_with_legend_choices(
                        &mut hypothetical,
                        other_actions,
                        &legend_keeps,
                        all_effects.as_slice(),
                        &mut dm,
                    )
                    .map_err(GameLoopError::ExecutionFailed)?;
                    if let Some(prompt) = dm.pending_prompt.take() {
                        self.pending_sba_choices = Some(PendingSbaChoices {
                            answers,
                            prompt: prompt.clone(),
                            response: None,
                        });
                        self.resolved_legend_keeps = legend_keeps;
                        return Ok(RunnerProgress::NeedsDecision(prompt));
                    }
                    *game = hypothetical;
                    applied
                };
                // CR 704.5h: this check consumed the deathtouch damage tracked
                // since the previous one (regeneration / umbra armor survivors
                // must not be destroyed again by the next pass).
                game.clear_deathtouch_damage_since_sba();
                crate::game_loop::try_drain_pending_trigger_events(game, tq)?;
                if !applied {
                    self.pending_boolean = None;
                    self.pending_commander_choice = None;
                    self.pending_draw_replacement = None;
                    self.pending_legend_choice = None;
                    self.pending_sector_designations = None;
                    self.pending_sba_choices = None;
                    return Ok(RunnerProgress::Complete(()));
                }
                continue;
            }

            let Some(obj_id) = commander_returns.first().copied() else {
                game.clear_empty_library_draw_attempts_since_sba();
                game.clear_deathtouch_damage_since_sba();
                self.pending_boolean = None;
                self.pending_commander_choice = None;
                self.pending_draw_replacement = None;
                self.pending_legend_choice = None;
                self.pending_sector_designations = None;
                self.pending_sba_choices = None;
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
                    crate::game_loop::try_drain_pending_trigger_events(game, tq)?;
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

/// The player whose hand a chosen cleanup discard came from.
fn cleanup_discard_owner(game: &GameState, discard: &[crate::ids::ObjectId]) -> Option<PlayerId> {
    discard
        .first()
        .and_then(|card| game.object(*card))
        .map(|card| card.owner)
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
    if step == Step::Draw {
        game.finish_draw_step_tracking();
    }
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
    if step == Step::Draw {
        game.finish_draw_step_tracking();
    }
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
    if phase == Phase::Beginning && game.turn.step == Some(Step::Draw) {
        game.finish_draw_step_tracking();
    }

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
#[path = "turn_runner_combat_cost_tests.rs"]
mod combat_cost_choice_tests;

#[cfg(test)]
#[path = "turn_runner_choice_tests.rs"]
mod choice_tests;

#[cfg(test)]
#[path = "turn_runner_lifelink_choice_tests.rs"]
mod lifelink_choice_tests;

#[cfg(test)]
#[path = "turn_runner_saga_choice_tests.rs"]
mod saga_choice_tests;

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
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::tag::TagKey;
    use crate::triggers::TriggerQueue;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn untap_runner_retains_pending_payload_and_propagates_errors_without_publication() {
        for pause in [false, true] {
            let player = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut objects = Vec::new();
            for name in ["First", "Second"] {
                let id = game.create_object_from_card(
                    &CardBuilder::new(CardId::new(), name).build(),
                    player,
                    Zone::Battlefield,
                );
                game.tap(id);
                objects.push(id);
            }
            game.turn.active_player = player;
            let mut payload = vec![crate::effect::Effect::gain_life(2)];
            payload.push(if pause {
                crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(1)])
            } else {
                crate::effect::Effect::lose_life(crate::effect::Value::X)
            });
            let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    objects[1],
                    player,
                    crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                        crate::target::ObjectFilter::specific(objects[1]),
                    ),
                    crate::replacement::ReplacementAction::Instead(payload),
                ),
            );
            game.take_pending_trigger_events();
            let mut runner = TurnRunner::new();
            let result = runner.run_untap_step_with_choices(&mut game, Vec::new());
            if pause {
                assert!(result.unwrap().is_some());
            } else {
                assert!(matches!(result, Err(GameLoopError::ResolutionFailed(_))));
            }
            assert_eq!(game.player(player).unwrap().life, 20);
            assert!(objects.iter().all(|object| game.is_tapped(*object)));
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(one_shot)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            if pause {
                assert!(
                    runner
                        .run_untap_step_with_choices(
                            &mut game,
                            vec![AttackCostAnswer::Boolean(true)]
                        )
                        .unwrap()
                        .is_none()
                );
                assert_eq!(game.player(player).unwrap().life, 23);
                assert!(!game.is_tapped(objects[0]));
                assert!(game.is_tapped(objects[1]));
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(one_shot)
                        .is_none()
                );
                assert_eq!(game.take_pending_trigger_events().len(), 3);
            }
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn attacker_decision_refreshes_conditional_characteristics_before_queries() {
        use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let creature = create_battlefield_creature(&mut game, alice, "Conditional Attacker");
        game.remove_summoning_sickness(creature);
        for card_type in [
            CardType::Artifact,
            CardType::Land,
            CardType::Instant,
            CardType::Sorcery,
        ] {
            let card = CardBuilder::new(CardId::new(), "Graveyard Probe")
                .card_types(vec![card_type])
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }
        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                creature,
                alice,
                EffectTarget::Specific(creature),
                Modification::AddAbility(StaticAbility::must_attack()),
            )
            .with_condition(
                crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
                    player: crate::target::PlayerFilter::You,
                    count: 4,
                },
            ),
        );
        let before = game.work_counters();
        let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareAttackersDecision);
        let TurnAction::Decision(DecisionContext::Attackers(ctx)) = runner
            .advance(&mut game, &mut TriggerQueue::new())
            .expect("attacker decision")
        else {
            panic!("expected attackers");
        };
        assert_eq!(ctx.attacker_options.len(), 1);
        assert!(ctx.attacker_options[0].must_attack);
        let recomputes = game.work_counters().characteristics_full_recomputes
            - before.characteristics_full_recomputes;
        assert!(
            recomputes < 100,
            "small board recomputed characteristics {recomputes} times"
        );
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
                    keyword: None,
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
        assert!(matches!(runner.state(), TurnState::UntapEndMana));
        assert!(matches!(
            runner
                .advance(&mut game, &mut tq)
                .expect("finish untap mana boundary"),
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
            let card = CardBuilder::new(CardId::new(), format!("Hand Card {idx}"))
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
            let card = CardBuilder::new(CardId::new(), format!("Library Creature {idx}"))
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
            let card = CardBuilder::new(CardId::new(), format!("Skip Probe {index}"))
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
            let card = CardBuilder::new(CardId::new(), format!("Library Card {idx}"))
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
            let card = CardBuilder::new(CardId::new(), format!("Library Card {idx}"))
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

        // BeginTurn -> untap's explicit mana boundary -> Upkeep
        let action = runner.advance(&mut game, &mut tq).unwrap();
        assert!(matches!(action, TurnAction::Continue));
        assert!(matches!(runner.state(), TurnState::UntapEndMana));
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
            TurnAction::Continue
        ));
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
        assert!(!runner.combat.blockers.contains_key(&attacks_charlie));

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
        assert!(!runner.combat.blockers.contains_key(&attacks_charlie));
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
        for typed in [false, true] {
            let mut game = setup_game();
            let mut tq = TriggerQueue::new();
            let mut runner = TurnRunner::new();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let attacker = create_battlefield_creature(&mut game, alice, "Taxed Attacker");
            game.remove_summoning_sickness(attacker);
            let mountain = create_mountain(&mut game, alice);
            let second_mountain = create_mountain(&mut game, alice);
            let tax = add_attack_tax(&mut game, bob, 1);
            if typed {
                let abilities = game.object_mut(tax).unwrap().abilities_mut();
                abilities.clear();
                abilities.push(Ability::static_ability(StaticAbility::attack_cost(
                    crate::target::ObjectFilter::creature(),
                    true,
                    crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::dynamic_mana(
                        ironsmith_core::DynamicManaCost::generic_equal_to(
                            crate::effect::Value::Count(
                                crate::target::ObjectFilter::enchantment().you_control(),
                            ),
                        ),
                    )]),
                    "Dynamic attack tax",
                )));
            }
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
    }

    #[test]
    fn typed_attack_payment_prompts_preserve_order_choices_and_commit_once() {
        use crate::mana::{ManaCost, ManaSymbol};
        for invalid_second_choice in [false, true] {
            let mut game = setup_game();
            let mut runner = TurnRunner::new();
            let mut tq = TriggerQueue::new();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let first = create_battlefield_creature(&mut game, alice, "First attacker");
            let second = create_battlefield_creature(&mut game, alice, "Second attacker");
            game.remove_summoning_sickness(first);
            game.remove_summoning_sickness(second);
            let card = CardBuilder::new(CardId::new(), "Attack tax")
                .card_types(vec![CardType::Enchantment])
                .build();
            let tax = game.create_object_from_card(&card, bob, Zone::Battlefield);
            game.object_mut(tax)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::attack_cost(
                    crate::target::ObjectFilter::creature(),
                    true,
                    crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![
                        ManaSymbol::White,
                        ManaSymbol::Life(2),
                    ]])),
                    "Attack tax",
                )));
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::White, 1);
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.refresh_continuous_state();
            runner.state = TurnState::DeclareAttackersApply;
            runner.respond_attackers(
                vec![first, second]
                    .into_iter()
                    .map(|creature| AttackerDeclaration {
                        creature,
                        target: AttackTarget::Player(bob),
                    })
                    .collect(),
            );
            let TurnAction::Decision(DecisionContext::SelectOptions(order)) =
                runner.advance(&mut game, &mut tq).unwrap()
            else {
                panic!("payment order expected");
            };
            assert_eq!(order.min, 2);
            runner.respond_options(vec![1, 0]);
            let TurnAction::Decision(DecisionContext::SelectOptions(first_payment)) =
                runner.advance(&mut game, &mut tq).unwrap()
            else {
                panic!("first mana/life choice expected");
            };
            let life = first_payment
                .options
                .iter()
                .find(|option| option.description.contains("life"))
                .unwrap()
                .index;
            assert!(
                matches!(
                    runner.advance(&mut game, &mut tq).unwrap(),
                    TurnAction::Decision(DecisionContext::SelectOptions(_))
                ),
                "waiting does not submit an answer"
            );
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
            runner.respond_options(vec![life]);
            let TurnAction::Decision(DecisionContext::SelectOptions(second_payment)) =
                runner.advance(&mut game, &mut tq).unwrap()
            else {
                panic!("second mana/life choice expected");
            };
            assert_eq!(
                game.player(alice).unwrap().life,
                20,
                "partial replay must not spend life"
            );
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
            assert!(runner.combat.attackers.is_empty());
            let mana = second_payment
                .options
                .iter()
                .find(|option| option.description.contains("{W}"))
                .unwrap()
                .index;
            runner.respond_options(vec![if invalid_second_choice { 999 } else { mana }]);
            let result = runner.advance(&mut game, &mut tq);
            if invalid_second_choice {
                assert!(result.is_err());
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
                assert!(!game.is_tapped(first));
                assert!(!game.is_tapped(second));
                assert!(runner.combat.attackers.is_empty());
            } else {
                assert!(matches!(result.unwrap(), TurnAction::RunPriority));
                assert_eq!(game.player(alice).unwrap().life, 18);
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
                assert_eq!(runner.combat.attackers.len(), 2);
            }
        }
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
            let draw = CardBuilder::new(CardId::new(), format!("Ordered Draw {index}")).build();
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
        assert!(matches!(runner.state(), TurnState::SkippedPhaseEndMana));
        assert!(matches!(
            runner.advance(&mut game, &mut tq).unwrap(),
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
        assert!(game.stack.is_empty());
        assert_eq!(tq.entries.len(), 1);
        crate::game_loop::advance_priority_with_dm(&mut game, &mut tq, &mut AutoPassDecisionMaker)
            .expect("the priority loop should stack the exert trigger");
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
        assert!(game.stack.is_empty());
        assert_eq!(tq.entries.len(), 1);
        crate::game_loop::advance_priority_with_dm(&mut game, &mut tq, &mut AutoPassDecisionMaker)
            .expect("the priority loop should stack the enlist trigger");
        assert_eq!(
            game.stack.len(),
            1,
            "the linked enlist trigger uses the stack"
        );
    }

    #[test]
    fn test_turn_runner_pauses_for_drawn_commander_choice() {
        let mut game = setup_game();
        // Not the starting player's first turn, which skips the draw (CR 103.8a).
        game.turn.turn_number = 2;
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
    fn test_turn_runner_skips_starting_players_first_draw_in_two_player_commander_game() {
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
        // CR 103.8a: a two-player Commander game is a two-player game.
        assert_eq!(
            game.player(alice).expect("alice should exist").hand.len(),
            0
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
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

    fn replacement_draw_step_fixture() -> (
        GameState,
        TurnRunner,
        TriggerQueue,
        ObjectId,
        PlayerId,
        Vec<ObjectId>,
    ) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Draw step replacement source")
                .card_types(vec![CardType::Enchantment])
                .build(),
            alice,
            Zone::Battlefield,
        );
        for name in [
            "First draw step library card",
            "Second draw step library card",
        ] {
            game.create_object_from_card(
                &CardBuilder::new(CardId::new(), name)
                    .card_types(vec![CardType::Instant])
                    .build(),
                alice,
                Zone::Library,
            );
        }
        let library = game.player(alice).unwrap().library.as_slice().to_vec();
        game.take_pending_trigger_events();
        let mut runner = TurnRunner::new();
        runner.state = TurnState::Draw;
        (game, runner, TriggerQueue::new(), source, alice, library)
    }

    #[test]
    fn draw_step_payload_error_is_reported_without_committing_prefix() {
        let (mut game, mut runner, mut queue, source, alice, library) =
            replacement_draw_step_fixture();
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Instead(vec![
                    crate::effect::Effect::gain_life(2),
                    crate::effect::Effect::lose_life(crate::effect::Value::X),
                ]),
            ),
        );
        let result = runner.advance(&mut game, &mut queue);
        assert!(
            matches!(result, Err(GameLoopError::ExecutionFailed(crate::effects::ExecutionError::UnresolvableValue(_)))),
            "a draw-step replacement failure must not become a successful priority window"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            library.as_slice()
        );
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(queue.entries.is_empty());
        assert!(matches!(runner.state(), TurnState::Draw));
    }

    #[test]
    fn draw_step_pending_payload_retains_shield_and_replays_once() {
        let (mut game, mut runner, mut queue, source, alice, library) =
            replacement_draw_step_fixture();
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Instead(vec![
                    crate::effect::Effect::gain_life(2),
                    crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(4)]),
                ]),
            ),
        );
        let action = runner.advance(&mut game, &mut queue).unwrap();
        let TurnAction::Decision(DecisionContext::Boolean(choice)) = action else {
            panic!("the draw-step payload must expose its own boolean choice");
        };
        assert_eq!(choice.player, alice);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            library.as_slice()
        );
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some(),
            "a suspended draw step must not consume its replacement shield"
        );
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(queue.entries.is_empty());
        runner.respond_boolean(true);
        let action = runner.advance(&mut game, &mut queue).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(game.player(alice).unwrap().life, 26);
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            library.as_slice()
        );
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
    }

    #[test]
    fn draw_step_instead_payload_keeps_preceding_draw_replacement_history() {
        let (mut game, mut runner, mut queue, source, alice, _) = replacement_draw_step_fixture();
        let mut double = crate::replacement::ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::cards::matchers::WouldDrawCardMatcher::you(),
            crate::replacement::ReplacementAction::Double,
        );
        double.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
        let double = game
            .effect_store
            .replacement_effects
            .add_resolution_effect(double);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::draw(
                    1,
                )]),
            ),
        );
        let action = runner.advance(&mut game, &mut queue).unwrap();
        assert!(matches!(action, TurnAction::RunPriority));
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            1,
            "the draw-step replacement must carry history into its nested draw"
        );
        assert_eq!(game.player(alice).unwrap().library.len(), 1);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(double)
                .is_some()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
    }
    #[test]
    fn draw_step_commits_the_resolved_recipient() {
        let (mut game, mut runner, mut queue, source, alice, alice_library) =
            replacement_draw_step_fixture();
        let bob = PlayerId::from_index(1);
        for name in [
            "First redirected library card",
            "Second redirected library card",
        ] {
            game.create_object_from_card(
                &CardBuilder::new(CardId::new(), name)
                    .card_types(vec![CardType::Instant])
                    .build(),
                bob,
                Zone::Library,
            );
        }
        let bob_library = game.player(bob).unwrap().library.as_slice().to_vec();
        let watcher = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Resolved drawer watcher")
                .card_types(vec![CardType::Enchantment])
                .build(),
            bob,
            Zone::Battlefield,
        );
        game.object_mut(watcher)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::triggered(
                crate::triggers::Trigger::you_draw_cards(),
                vec![crate::effect::Effect::gain_life(1)],
            ));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Redirect {
                    target: crate::replacement::RedirectTarget::ToPlayer(bob),
                    which: crate::replacement::RedirectWhich::First,
                },
            ),
        );
        // Prove the shared processor supports this redirect independently of
        // whether the public turn-action caller preserves its result.
        let mut processor_game = game.clone();
        let processed = crate::events::processing::process_trait_event(
            &mut processor_game,
            crate::events::Event::draw_in_instruction(alice, 1, true, true, true),
        )
        .expect("finite replacement fixture evaluates successfully");
        let (crate::events::processing::TraitEventResult::Proceed(event)
        | crate::events::processing::TraitEventResult::Modified(event)) = processed
        else {
            panic!("the draw redirect must produce a modified carrier");
        };
        assert_eq!(
            crate::events::downcast_event::<crate::events::DrawEvent>(event.inner())
                .unwrap()
                .player,
            bob
        );
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        assert!(
            game.player(alice).unwrap().hand.is_empty(),
            "the caller must not draw for the authored recipient"
        );
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            alice_library.as_slice()
        );
        assert_eq!(game.player(bob).unwrap().hand.len(), 1);
        assert_eq!(
            game.player(bob).unwrap().library.len(),
            bob_library.len() - 1
        );
        let drawn = game.player(bob).unwrap().hand[0];
        assert_eq!(
            game.object(drawn).unwrap().name,
            "Second redirected library card"
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(bob), 1);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert_eq!(
            queue.entries.len(),
            1,
            "the actual drawer must receive exactly one draw trigger"
        );
        let event = queue.entries[0]
            .triggering_event
            .downcast::<crate::events::other::CardsDrawnEvent>()
            .expect("the watcher must retain the actual draw observation");
        assert_eq!(event.player, bob);
        assert_eq!(event.amount(), 1);
    }

    fn shared_draw_step_payload_boundary(pending: bool) {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        game.set_teams(vec![vec![alice, bob], vec![charlie]])
            .unwrap();
        game.enable_shared_team_turns().unwrap();
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        for player in [alice, bob] {
            game.create_object_from_card(
                &CardBuilder::new(CardId::new(), "Shared draw library card")
                    .card_types(vec![CardType::Instant])
                    .build(),
                player,
                Zone::Library,
            );
        }
        let alice_library = game.player(alice).unwrap().library.as_slice().to_vec();
        let bob_library = game.player(bob).unwrap().library.as_slice().to_vec();
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Shared draw replacement")
                .card_types(vec![CardType::Enchantment])
                .build(),
            bob,
            Zone::Battlefield,
        );
        let tail = if pending {
            crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(4)])
        } else {
            crate::effect::Effect::lose_life(crate::effect::Value::X)
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Instead(vec![
                    crate::effect::Effect::gain_life(2),
                    tail,
                ]),
            ),
        );
        let mut runner = TurnRunner::new();
        runner.state = TurnState::Draw;
        let mut queue = TriggerQueue::new();
        game.take_pending_trigger_events();
        let action = runner.advance(&mut game, &mut queue);
        assert!(
            game.player(alice).unwrap().hand.is_empty(),
            "a later teammate suspension/failure must not publish an earlier draw"
        );
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            alice_library.as_slice()
        );
        assert_eq!(
            game.player(bob).unwrap().library.as_slice(),
            bob_library.as_slice()
        );
        assert!(game.player(bob).unwrap().hand.is_empty());
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(queue.entries.is_empty());
        if pending {
            let TurnAction::Decision(DecisionContext::Boolean(choice)) = action.unwrap() else {
                panic!("expected payload choice");
            };
            assert_eq!(choice.player, bob);
            runner.respond_boolean(true);
            assert!(matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                TurnAction::RunPriority
            ));
            assert_eq!(game.player(alice).unwrap().hand.len(), 1);
            assert!(game.player(alice).unwrap().library.is_empty());
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
            assert_eq!(game.player(bob).unwrap().life, 26);
            assert_eq!(
                game.player(bob).unwrap().library.as_slice(),
                bob_library.as_slice()
            );
            assert!(game.player(bob).unwrap().hand.is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        } else {
            assert!(matches!(action, Err(GameLoopError::ExecutionFailed(crate::effects::ExecutionError::UnresolvableValue(_)))));
            assert!(matches!(runner.state(), TurnState::Draw));
        }
    }

    #[test]
    fn shared_draw_step_later_payload_error_restores_earlier_teammate() {
        shared_draw_step_payload_boundary(false);
    }

    #[test]
    fn shared_draw_step_later_payload_pending_replays_earlier_teammate_once() {
        shared_draw_step_payload_boundary(true);
    }

    #[test]
    fn draw_step_invalid_replacement_option_does_not_choose_a_fallback() {
        let (mut game, mut runner, mut queue, source, alice, library) =
            replacement_draw_step_fixture();
        let prevent_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Draw choice preventer")
                .card_types(vec![CardType::Enchantment])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let double = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Double,
            ),
        );
        let prevent = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                prevent_source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Prevent,
            ),
        );
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::Decision(DecisionContext::SelectOptions(_))
        ));
        runner.respond_options(vec![usize::MAX]);
        let result = runner.advance(&mut game, &mut queue);
        assert!(
            matches!(result, Err(GameLoopError::ExecutionFailed(crate::effects::ExecutionError::InternalError(_)))),
            "an invalid answer must not select the first offered replacement"
        );
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            library.as_slice()
        );
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(double)
                .is_some()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(prevent)
                .is_some()
        );
        assert!(queue.entries.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
        let TurnAction::Decision(DecisionContext::SelectOptions(choice)) =
            runner.advance(&mut game, &mut queue).unwrap()
        else {
            panic!("the failed operation must offer a fresh valid choice");
        };
        let index = choice
            .options
            .iter()
            .find(|option| option.object_id == Some(prevent_source))
            .unwrap()
            .index;
        runner.respond_options(vec![index]);
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            library.as_slice()
        );
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(double)
                .is_some()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(prevent)
                .is_none()
        );
    }

    #[test]
    fn draw_step_redirected_replacement_choice_resumes_the_same_event() {
        let (mut game, mut runner, mut queue, source, alice, alice_library) =
            replacement_draw_step_fixture();
        let bob = PlayerId::from_index(1);
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Bob choice library card")
                .card_types(vec![CardType::Instant])
                .build(),
            bob,
            Zone::Library,
        );
        let bob_library = game.player(bob).unwrap().library.as_slice().to_vec();
        let double_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Bob draw choice doubler")
                .card_types(vec![CardType::Enchantment])
                .build(),
            bob,
            Zone::Battlefield,
        );
        let prevent_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Bob draw choice preventer")
                .card_types(vec![CardType::Enchantment])
                .build(),
            bob,
            Zone::Battlefield,
        );
        let mut redirect = crate::replacement::ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::cards::matchers::WouldDrawCardMatcher::you(),
            crate::replacement::ReplacementAction::Redirect {
                target: crate::replacement::RedirectTarget::ToPlayer(bob),
                which: crate::replacement::RedirectWhich::First,
            },
        );
        redirect.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
        let redirect = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(redirect);
        let double = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                double_source,
                bob,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Double,
            ),
        );
        let prevent = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                prevent_source,
                bob,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                crate::replacement::ReplacementAction::Prevent,
            ),
        );
        let TurnAction::Decision(DecisionContext::SelectOptions(choice)) =
            runner.advance(&mut game, &mut queue).unwrap()
        else {
            panic!("Bob must choose between replacements for the redirected event");
        };
        assert_eq!(choice.player, bob);
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            alice_library.as_slice()
        );
        assert_eq!(
            game.player(bob).unwrap().library.as_slice(),
            bob_library.as_slice()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(redirect)
                .is_some()
        );
        let index = choice
            .options
            .iter()
            .find(|option| option.object_id == Some(double_source))
            .unwrap()
            .index;
        runner.respond_options(vec![index]);
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        assert!(
            game.player(alice).unwrap().hand.is_empty(),
            "resuming Bob's choice must not fall back to Alice's authored draw"
        );
        assert!(game.player(bob).unwrap().hand.is_empty());
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            alice_library.as_slice()
        );
        assert_eq!(
            game.player(bob).unwrap().library.as_slice(),
            bob_library.as_slice()
        );
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(bob), 0);
        for shield in [redirect, double, prevent] {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(queue.entries.is_empty());
    }
}

#[cfg(test)]
#[path = "turn_runner_draw_choice_tests.rs"]
mod draw_choice_tests;

#[cfg(test)]
mod replacement_turn_draw_expansion_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::{Effect, Value};
    use crate::ids::CardId;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::types::CardType;
    use crate::zone::Zone;
    fn fixture(
        payload: Vec<Effect>,
    ) -> (
        GameState,
        TurnRunner,
        PlayerId,
        PlayerId,
        crate::replacement::ReplacementEffectId,
    ) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn.turn_number = 2;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Turn draw addition source")
                .card_types(vec![CardType::Enchantment])
                .build(),
            bob,
            Zone::Battlefield,
        );
        for name in ["Draw one", "Draw two"] {
            game.create_object_from_card(
                &CardBuilder::new(CardId::new(), name)
                    .card_types(vec![CardType::Instant])
                    .build(),
                alice,
                Zone::Library,
            );
        }
        let mut effect = ReplacementEffect::with_matcher(
            source,
            bob,
            crate::events::cards::matchers::WouldDrawCardMatcher::new(
                crate::target::PlayerFilter::Specific(alice),
            ),
            ReplacementAction::Additionally(payload),
        );
        effect.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
        let shield = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(effect);
        game.take_pending_trigger_events();
        let mut runner = TurnRunner::new();
        runner.state = TurnState::Draw;
        (game, runner, alice, bob, shield)
    }
    #[test]
    fn addition_preserves_original_draw_and_its_recipient() {
        let (mut game, mut runner, alice, bob, shield) = fixture(vec![Effect::gain_life(3)]);
        let RunnerProgress::Complete(events) =
            runner.execute_draw_step_with_choices(&mut game).unwrap()
        else {
            panic!("draw should complete")
        };
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
        assert_eq!(game.player(alice).unwrap().library.len(), 1);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 23);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
        assert!(
            events
                .first()
                .unwrap()
                .downcast::<crate::events::CardsDrawnEvent>()
                .is_some()
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.downcast::<crate::events::CardsDrawnEvent>().is_some())
                .count(),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
    }
    #[test]
    fn addition_error_restores_draw_and_consumption() {
        let (mut game, mut runner, alice, bob, shield) =
            fixture(vec![Effect::gain_life(3), Effect::lose_life(Value::X)]);
        let library = game.player(alice).unwrap().library.as_slice().to_vec();
        let ids = game.next_object_id_counter();
        let error = match runner.execute_draw_step_with_choices(&mut game) {
            Err(error) => error,
            Ok(_) => panic!("added draw error must propagate"),
        };
        assert!(
            matches!(&error, GameLoopError::ExecutionFailed(crate::effects::ExecutionError::UnresolvableValue(_)))
        );
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            library.as_slice()
        );
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), ids);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }
    #[test]
    fn addition_pending_replays_the_whole_draw_once() {
        let (mut game, mut runner, alice, bob, shield) = fixture(vec![
            Effect::gain_life(3),
            Effect::new(crate::effects::composition::MayEffect::new_for_player(
                vec![Effect::gain_life(4)],
                crate::target::PlayerFilter::You,
            )),
        ]);
        let library = game.player(alice).unwrap().library.as_slice().to_vec();
        let ids = game.next_object_id_counter();
        let RunnerProgress::NeedsDecision(DecisionContext::Boolean(prompt)) =
            runner.execute_draw_step_with_choices(&mut game).unwrap()
        else {
            panic!("addition must pause")
        };
        assert_eq!(prompt.player, bob);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(
            game.player(alice).unwrap().library.as_slice(),
            library.as_slice()
        );
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), ids);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        runner.respond_boolean(true);
        let RunnerProgress::Complete(events) =
            runner.execute_draw_step_with_choices(&mut game).unwrap()
        else {
            panic!("answered draw must complete")
        };
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
        assert_eq!(game.player(bob).unwrap().life, 27);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 1);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.downcast::<crate::events::CardsDrawnEvent>().is_some())
                .count(),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
    }
}

#[cfg(test)]
#[path = "turn_runner_combat_prevention_tests.rs"]
mod combat_prevention_lifecycle_tests;

pub(crate) fn empty_mana_pools_without_choices(
    game: &mut GameState,
) -> Result<(), crate::effects::ExecutionError> {
    let mut dm = QueuedAttackCostDecisionMaker::new(Vec::new());
    game.empty_mana_pools_with_dm(&mut dm)?;
    if let Some(prompt) = dm.pending_prompt {
        return Err(crate::effects::ExecutionError::UnresolvedPlayerDecision {
            player: prompt.player(),
            decision: "mana loss replacement",
        });
    }
    Ok(())
}
