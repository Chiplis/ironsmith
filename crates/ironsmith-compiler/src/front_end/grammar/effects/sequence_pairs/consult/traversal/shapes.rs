use super::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConsultTraversalPlayerShape {
    ImpliedByPrefixOrYou,
    ThatPlayer,
    Subject(Vec<OwnedLexToken>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsultTraversalStopKind {
    Passive,
    Active,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConsultTraversalStopShape {
    pub(crate) stop_rule: LibraryConsultStopRuleAst,
    pub(crate) max_exposed: Option<Value>,
    pub(crate) filter: Vec<OwnedLexToken>,
    pub(crate) kind: ConsultTraversalStopKind,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConsultTraversalShape {
    pub(crate) prefix: Option<Vec<OwnedLexToken>>,
    pub(crate) player: ConsultTraversalPlayerShape,
    pub(crate) mode: LibraryConsultModeAst,
    pub(crate) stop: ConsultTraversalStopShape,
    pub(crate) where_x: Option<Value>,
    pub(crate) trailing_effect: Vec<OwnedLexToken>,
}
