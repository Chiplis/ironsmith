//! Shared resource, choice, voting, and iteration semantics.
//!
//! These nodes retain chooser and iterator scope explicitly so lowering never
//! has to reconstruct choice identity from tags or copied source phrases.

use crate::color::Color;
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::ast::EffectAst;
use crate::model::clauses::ClauseActorAst;
use crate::model::object_action_clauses::CompilerObjectOperandAst;
use crate::model::selections::{CompilerValueAst, SelectionCardinalityAst};
use crate::model::symbols::{SymbolReference, SymbolScopeId};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, SubtypeFamily};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerResourceOperationAst {
    Gain,
    Lose,
    Pay,
    Set,
    Exchange,
    Draw,
    Discard,
    Sacrifice,
    Tap,
    Untap,
    Double,
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerManaResourceAst {
    Fixed(Vec<ManaSymbol>),
    Cost {
        cost: ManaCost,
        x_value: Option<CompilerValueAst>,
        x_maximum: Option<CompilerValueAst>,
    },
    AnyColor {
        available: Option<Vec<Color>>,
        distinct: bool,
    },
    AnyOneColor,
    ChosenColor(Option<Color>),
    LandCouldProduce {
        filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
        source: CompilerManaTypeSourceAst,
    },
    ColorsAmong(ObjectFilter),
    CommanderIdentity,
    Pool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerManaTypeSourceAst {
    MatchingLandsCouldProduce,
    TriggeringEventProduced,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerResourceKindAst {
    Life,
    Mana(CompilerManaResourceAst),
    Energy,
    Experience,
    Ticket,
    Poison,
    Cards,
    ObjectState,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerResourceAmountAst {
    Value(CompilerValueAst),
    All,
    Any { minimum: CompilerValueAst },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerResourceClauseAst {
    pub operation: CompilerResourceOperationAst,
    pub owner: ClauseActorAst,
    pub resource: CompilerResourceKindAst,
    pub amount: CompilerResourceAmountAst,
    pub objects: Option<CompilerObjectOperandAst>,
    pub random: bool,
    pub result: SymbolReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerChoiceVisibilityAst {
    Public,
    Secret,
    HiddenUntilReveal,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerChoiceDomainAst {
    Color,
    CardType(Vec<CardType>),
    Named(Vec<String>),
    CreatureType {
        excluded: Vec<Subtype>,
        family: SubtypeFamily,
    },
    LandType {
        exclude_basic: bool,
    },
    CardName(Option<ObjectFilter>),
    Player {
        filter: PlayerFilter,
        exclude_previous: usize,
    },
    Number {
        minimum: CompilerValueAst,
        maximum: Option<CompilerValueAst>,
    },
    Object(CompilerObjectOperandAst),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerAggregateConstraintAst {
    pub metric: CompilerAggregateMetricAst,
    pub minimum: Option<CompilerValueAst>,
    pub maximum: CompilerValueAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerAggregateMetricAst {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerChoiceClauseAst {
    pub chooser: ClauseActorAst,
    pub visibility: CompilerChoiceVisibilityAst,
    pub domain: CompilerChoiceDomainAst,
    pub cardinality: SelectionCardinalityAst,
    pub random: bool,
    pub zones: Vec<Zone>,
    pub top_only: bool,
    pub bottom_only: bool,
    pub aggregate: Option<CompilerAggregateConstraintAst>,
    pub scope: SymbolScopeId,
    pub chosen: SymbolReference,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerResourceChoiceClauseAst {
    Resource(CompilerResourceClauseAst),
    Choice(CompilerChoiceClauseAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerVoteOrderAst {
    Simultaneous,
    TurnOrder,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerVoteAst {
    pub voters: PlayerFilter,
    pub exclude_voter: bool,
    pub visibility: CompilerChoiceVisibilityAst,
    pub order: CompilerVoteOrderAst,
    pub starts_with_controller: bool,
    pub options: CompilerChoiceDomainAst,
    pub votes_per_voter: SelectionCardinalityAst,
    pub choice_scope: SymbolScopeId,
    pub choices: SymbolReference,
    pub tally: SymbolReference,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerIterationSourceAst {
    Opponents,
    Players(PlayerFilter),
    SelectedPlayers {
        filter: PlayerFilter,
        collection: SymbolReference,
    },
    Objects(ObjectFilter),
    Reference(SymbolReference),
    Count(CompilerValueAst),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerRepetitionKindAst {
    ForEach,
    Exactly,
    UpTo,
    Optional,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerIterationAst {
    pub kind: CompilerRepetitionKindAst,
    pub source: CompilerIterationSourceAst,
    pub parent_scope: SymbolScopeId,
    pub scope: SymbolScopeId,
    pub iterator: SymbolReference,
    pub selection_cardinality: Option<SelectionCardinalityAst>,
    pub body: Vec<EffectAst>,
    pub aggregate: Option<SymbolReference>,
}
