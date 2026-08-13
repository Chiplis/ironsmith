//! Typed casting, play, activation, and cost-permission semantics.
//!
//! A permission is a relationship between an actor, an object, an action,
//! an origin, and a lifetime.  Keeping those facts orthogonal prevents
//! sentence-sized permission recipes from leaking into the compiler AST.

use crate::mana::ManaCost;
use crate::model::ast::PredicateAst;
use crate::model::clauses::ClauseActorAst;
use crate::model::legality::TimingWindowAst;
use crate::model::object_action_clauses::CompilerObjectOperandAst;
use crate::model::selections::{CompilerFilterAst, CompilerValueAst};
use crate::model::symbols::SymbolReference;
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerPermissionDispositionAst {
    Execute,
    Permit,
    Prohibit,
    Restrict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerCastingActionAst {
    CastSpell,
    PlayLand,
    CastOrPlay,
    ActivateAbility,
    ModifySpellCost,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerPermissionActorAst {
    Actor(ClauseActorAst),
    Players(PlayerFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerCastingOriginAst {
    Default,
    Zones {
        zones: Vec<Zone>,
        owner: Option<ClauseActorAst>,
    },
    CurrentZone,
    TopOfLibrary {
        owner: Option<ClauseActorAst>,
    },
    ExiledWithSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerManaFlexibilityAst {
    AsWritten,
    AnyColor,
    AnyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerAlternativeCastAst {
    Blitz,
    Dash,
    Flashback,
    JumpStart,
    Escape,
    Madness,
    Miracle,
    Suspend,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerCastingPaymentAst {
    PrintedCost,
    WithoutPayingManaCost,
    Alternative(CompilerAlternativeCastAst),
    PayLifeByManaValue,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerCostAdjustmentAst {
    AddMana(ManaCost),
    ReduceMana(ManaCost),
    ReduceValue(CompilerValueAst),
    IncreaseMana(ManaCost),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerCastingCostAst {
    pub payment: CompilerCastingPaymentAst,
    pub adjustments: Vec<CompilerCostAdjustmentAst>,
    pub mana_flexibility: CompilerManaFlexibilityAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerPermissionExpirationAst {
    Immediate,
    UntilEndOfTurn,
    UntilYourNextTurn,
    UntilYourNextEndStep,
    UntilYourNextUpkeep,
    UntilControllerNextUntap,
    UntilEndOfCombat,
    UntilSourceLeavesBattlefield,
    UntilSourceUntaps,
    WhileExiled,
    WhileYouControlSource,
    WhileOnTopOfLibrary,
    UntilSourceExilesAnother,
    DuringTurnsCounterPutOnSource(CounterType),
    ForTurns(CompilerValueAst),
    Until(PredicateAst),
    BoundedBy(Vec<CompilerPermissionExpirationAst>),
    Permanent,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerPermissionStartAst {
    Immediate,
    NextTurn(PlayerFilter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerPermissionFrequencyAst {
    Unbounded,
    Once,
    AtMost(u32),
    MoreThanOnePerTurn,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerPermissionClauseAst {
    pub disposition: CompilerPermissionDispositionAst,
    pub action: CompilerCastingActionAst,
    pub actor: CompilerPermissionActorAst,
    pub object: CompilerObjectOperandAst,
    pub qualification: Option<CompilerFilterAst>,
    pub origin: CompilerCastingOriginAst,
    pub timing: Option<TimingWindowAst>,
    pub cost: CompilerCastingCostAst,
    pub starts: CompilerPermissionStartAst,
    pub expiration: CompilerPermissionExpirationAst,
    pub frequency: CompilerPermissionFrequencyAst,
    pub linked_object: Option<SymbolReference>,
    pub as_copy: bool,
    pub lands_enter_tapped: bool,
}
