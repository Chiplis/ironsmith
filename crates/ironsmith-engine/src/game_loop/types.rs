use super::*;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during game loop execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameLoopError {
    /// Turn progression error.
    TurnError(TurnError),
    /// Stack resolution failed.
    ResolutionFailed(String),
    /// A fallible engine operation failed, retaining the underlying error.
    ExecutionFailed(crate::effects::ExecutionError),
    /// Invalid game state.
    InvalidState(String),
    /// A proposed cast or activation was illegal and has been rolled back.
    ActionCancelled(String),
    /// No players remaining.
    GameOver,
    /// A mandatory rules-procedure cycle was proven under CR 104.4b.
    MandatoryLoopDraw,
    /// Invalid player response.
    ResponseError(ResponseError),
    /// Combat error.
    CombatError(CombatError),
    /// Special action error.
    ActionError(crate::special_actions::ActionError),
}

/// Ordinary inability to pay reverses an announcement. Execution faults keep
/// their typed error so the session can retry without substituting choices.
pub(super) fn cost_payment_failure(
    description: String,
    error: crate::cost::CostPaymentError,
) -> GameLoopError {
    use crate::cost::CostPaymentError;
    match error {
        CostPaymentError::Cancelled
        | CostPaymentError::InsufficientMana
        | CostPaymentError::AlreadyTapped
        | CostPaymentError::SummoningSickness
        | CostPaymentError::AlreadyUntapped
        | CostPaymentError::InsufficientLife
        | CostPaymentError::SourceNotOnBattlefield
        | CostPaymentError::NoValidSacrificeTarget
        | CostPaymentError::InsufficientCardsInHand
        | CostPaymentError::InsufficientCounters
        | CostPaymentError::InsufficientEnergy
        | CostPaymentError::InsufficientCardsToExile
        | CostPaymentError::InsufficientCardsInGraveyard
        | CostPaymentError::NoValidReturnTarget
        | CostPaymentError::InsufficientCardsToReveal => GameLoopError::ActionCancelled(description),
        CostPaymentError::ExecutionFailed(error) => GameLoopError::ExecutionFailed(error),
        CostPaymentError::SourceNotFound
        | CostPaymentError::PlayerNotFound
        | CostPaymentError::Other(_) => GameLoopError::InvalidState(description),
    }
}

/// Response payload for externally driving a pending priority decision.
///
/// This is intentionally limited to decisions that can occur during the
/// priority loop (`GameProgress::NeedsDecisionCtx`).
#[derive(Debug, Clone, PartialEq)]
pub enum PriorityResponse {
    PriorityAction(LegalAction),
    Attackers(Vec<AttackerDeclaration>),
    Blockers {
        defending_player: PlayerId,
        declarations: Vec<BlockerDeclaration>,
    },
    Targets(Vec<Target>),
    Distribution(Vec<(Target, u32)>),
    XValue(u32),
    NumberChoice(u32),
    Modes(Vec<usize>),
    SpliceCards(Vec<ObjectId>),
    OptionalCosts(Vec<(usize, u32)>),
    AssistChoice(usize),
    /// Respond to the authoritative mana-payment planner.
    ManaPaymentPlan(crate::mana_payment::ManaPaymentResponse),
    NextCostChoice(usize),
    SacrificeTarget(ObjectId),
    CardCostChoice(ObjectId),
    HybridChoice(usize),
    CastingMethodChoice(usize),
    ReplacementChoice(usize),
    ExilePlayChoice(usize),
    ExileFaceDownChoice(usize),
}

impl From<TurnError> for GameLoopError {
    fn from(err: TurnError) -> Self {
        GameLoopError::TurnError(err)
    }
}

impl From<ResponseError> for GameLoopError {
    fn from(err: ResponseError) -> Self {
        GameLoopError::ResponseError(err)
    }
}

impl From<CombatError> for GameLoopError {
    fn from(err: CombatError) -> Self {
        match err {
            CombatError::ExecutionFailed(error) => GameLoopError::ExecutionFailed(error),
            err => GameLoopError::CombatError(err),
        }
    }
}

impl From<crate::special_actions::ActionError> for GameLoopError {
    fn from(err: crate::special_actions::ActionError) -> Self {
        GameLoopError::ActionError(err)
    }
}

impl From<crate::effects::ExecutionError> for GameLoopError {
    fn from(error: crate::effects::ExecutionError) -> Self {
        Self::ExecutionFailed(error)
    }
}

impl std::fmt::Display for GameLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameLoopError::TurnError(e) => write!(f, "Turn error: {e}"),
            GameLoopError::ResolutionFailed(msg) => write!(f, "Resolution failed: {}", msg),
            GameLoopError::ExecutionFailed(error) => write!(f, "Effect execution failed: {error}"),
            GameLoopError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            GameLoopError::ActionCancelled(msg) => write!(f, "Action cancelled: {}", msg),
            GameLoopError::GameOver => write!(f, "Game over"),
            GameLoopError::MandatoryLoopDraw => write!(f, "mandatory action loop is a draw"),
            GameLoopError::ResponseError(e) => write!(f, "Response error: {}", e),
            GameLoopError::CombatError(e) => write!(f, "Combat error: {}", e),
            GameLoopError::ActionError(e) => write!(f, "Action error: {e}"),
        }
    }
}

impl std::error::Error for GameLoopError {}
