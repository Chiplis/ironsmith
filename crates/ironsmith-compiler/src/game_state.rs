#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerControlStart {
    Immediate,
    NextTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerControlDuration {
    UntilEndOfTurn,
    Forever,
    UntilSourceLeaves,
}
