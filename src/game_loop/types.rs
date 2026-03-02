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
    /// Invalid game state.
    InvalidState(String),
    /// No players remaining.
    GameOver,
    /// Invalid player response.
    ResponseError(ResponseError),
    /// Combat error.
    CombatError(CombatError),
    /// Special action error.
    ActionError(crate::special_actions::ActionError),
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
    XValue(u32),
    NumberChoice(u32),
    Modes(Vec<usize>),
    OptionalCosts(Vec<(usize, u32)>),
    ManaPayment(usize),
    ManaPipPayment(usize),
    SacrificeTarget(ObjectId),
    CardCostChoice(ObjectId),
    HybridChoice(usize),
    CastingMethodChoice(usize),
    ReplacementChoice(usize),
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
        GameLoopError::CombatError(err)
    }
}

impl From<crate::special_actions::ActionError> for GameLoopError {
    fn from(err: crate::special_actions::ActionError) -> Self {
        GameLoopError::ActionError(err)
    }
}

impl std::fmt::Display for GameLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameLoopError::TurnError(e) => write!(f, "Turn error: {:?}", e),
            GameLoopError::ResolutionFailed(msg) => write!(f, "Resolution failed: {}", msg),
            GameLoopError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            GameLoopError::GameOver => write!(f, "Game over"),
            GameLoopError::ResponseError(e) => write!(f, "Response error: {}", e),
            GameLoopError::CombatError(e) => write!(f, "Combat error: {}", e),
            GameLoopError::ActionError(e) => write!(f, "Action error: {:?}", e),
        }
    }
}

impl std::error::Error for GameLoopError {}
