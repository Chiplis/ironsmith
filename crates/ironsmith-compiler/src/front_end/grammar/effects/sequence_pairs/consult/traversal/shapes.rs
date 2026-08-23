use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ConsultTraversalPlayerShape {
    ImpliedByPrefixOrYou,
    ThatPlayer,
    Subject(Vec<OwnedLexToken>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsultTraversalStopKind {
    Passive,
    Active,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsultTraversalStopShape {
    pub stop_rule: LibraryConsultStopRuleAst,
    pub max_exposed: Option<Value>,
    pub filter: Vec<OwnedLexToken>,
    pub kind: ConsultTraversalStopKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsultTraversalShape {
    pub prefix: Option<Vec<OwnedLexToken>>,
    pub player: ConsultTraversalPlayerShape,
    pub mode: LibraryConsultModeAst,
    pub stop: ConsultTraversalStopShape,
    pub where_x: Option<Value>,
    pub trailing_effect: Vec<OwnedLexToken>,
}
