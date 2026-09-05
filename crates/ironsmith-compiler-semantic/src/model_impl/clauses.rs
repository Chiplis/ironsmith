//! Composable semantic clauses shared by effect recognition and lowering.
//!
//! These orthogonal clause parts are shared by the executable effect,
//! coordination, permission, and control-flow models; they are vocabulary,
//! not a second top-level effect representation.

use ironsmith_core::tag::TagKeyWalk;

use crate::effect::ValueComparisonOperator;
use crate::model::costs::CompilerTotalCost;
use crate::model::selections::{CompilerFilterAst, CompilerSelectionAst, CompilerValueAst};
use crate::model::symbols::SymbolReference;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub enum ClauseVerbAst {
    Add,
    Attach,
    Activate,
    Become,
    Cast,
    Choose,
    Control,
    Copy,
    Counter,
    Create,
    DealDamage,
    Destroy,
    Discard,
    Draw,
    Exchange,
    Exile,
    Fight,
    Gain,
    Give,
    Look,
    Lose,
    Mill,
    Move,
    Pay,
    Play,
    Prevent,
    Put,
    Remove,
    Return,
    Reveal,
    Sacrifice,
    Search,
    Shuffle,
    Tap,
    Transform,
    Untap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub enum ClausePolarityAst {
    Positive,
    Negative,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ClauseActionAst {
    pub verb: ClauseVerbAst,
    pub polarity: ClausePolarityAst,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ClauseActorAst {
    SourceController,
    ActivePlayer,
    EachOpponent,
    EachPlayer,
    Selection(CompilerSelectionAst),
    Reference(SymbolReference),
    Player(CompilerPlayerAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub enum CompilerPlayerAst {
    Any,
    Chosen,
    Defending,
    Attacking,
    Opponent,
    Target,
    TargetOpponent,
    Enchanted,
    OtherThanSourceController,
    Contextual,
    TriggeringSourceController,
    ReferencedObjectController,
    ReferencedObjectOwner,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ClauseSubjectAst {
    Source,
    Actor(ClauseActorAst),
    Selection(CompilerSelectionAst),
    Filter(CompilerFilterAst),
    Reference(SymbolReference),
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ClauseObjectAst {
    Subject(ClauseSubjectAst),
    Selection(CompilerSelectionAst),
    Filter(CompilerFilterAst),
    Reference(SymbolReference),
    Cost(CompilerTotalCost),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClauseQuantityUnitAst {
    Objects,
    Cards,
    Players,
    Life,
    Damage,
    Mana,
    Counters,
    Turns,
    Times,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClauseDistributionAst {
    Total,
    Each,
    Divided,
    UpTo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClauseQuantityAst {
    pub value: CompilerValueAst,
    pub unit: ClauseQuantityUnitAst,
    pub distribution: ClauseDistributionAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClauseDestinationRelationAst {
    Into,
    From,
    To,
    Onto,
    Under,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClauseZonePlacementAst {
    Default,
    Top,
    Bottom,
    Shuffled,
    Tapped,
    FaceDown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClauseDestinationAst {
    pub relation: ClauseDestinationRelationAst,
    pub zone: Zone,
    pub placement: ClauseZonePlacementAst,
    pub controller: Option<ClauseActorAst>,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ClausePredicateAst {
    Constant(bool),
    Matches {
        subject: ClauseSubjectAst,
        filter: CompilerFilterAst,
    },
    Compare {
        left: CompilerValueAst,
        operator: ValueComparisonOperator,
        right: CompilerValueAst,
    },
    ReferenceExists(SymbolReference),
    Not(Box<ClausePredicateAst>),
    All(Vec<ClausePredicateAst>),
    Any(Vec<ClausePredicateAst>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub enum ClauseConditionKindAst {
    If,
    Unless,
    When,
    While,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ClauseConditionAst {
    pub kind: ClauseConditionKindAst,
    pub predicate: ClausePredicateAst,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ClauseDurationAst {
    Permanent,
    ThisTurn,
    UntilEndOfTurn,
    UntilEndOfCombat,
    UntilNextTurn,
    ForTurns(CompilerValueAst),
    While(ClauseConditionAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClauseBindingSourceAst {
    Actor,
    Subject,
    Object,
    Result,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClauseReferenceBindingAst {
    pub reference: SymbolReference,
    pub source: ClauseBindingSourceAst,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClauseComplementAst {
    Object(ClauseObjectAst),
    Quantity(ClauseQuantityAst),
    Destination(ClauseDestinationAst),
    Duration(ClauseDurationAst),
    Condition(ClauseConditionAst),
    Binding(ClauseReferenceBindingAst),
}
