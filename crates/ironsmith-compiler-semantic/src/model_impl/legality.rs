use crate::model::CompilerTotalCost;
use crate::model::ast::PredicateAst;
use crate::model::provenance::SemanticProvenance;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub enum TurnOwnerAst {
    Any,
    You,
    Opponent,
    ActivePlayer,
    Player(PlayerFilter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStepAst {
    Untap,
    Upkeep,
    Draw,
    PrecombatMain,
    BeginningOfCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndOfCombat,
    PostcombatMain,
    EndStep,
    Cleanup,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimingWindowAst {
    AnyTime,
    SorcerySpeed,
    DuringTurn(TurnOwnerAst),
    DuringPhase {
        turn: TurnOwnerAst,
        phase: PhaseStepAst,
    },
    DuringCombat(TurnOwnerAst),
    BeforePhase {
        turn: TurnOwnerAst,
        phase: PhaseStepAst,
    },
    AfterPhase {
        turn: TurnOwnerAst,
        phase: PhaseStepAst,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegalityRelationshipAst {
    Only,
    Except,
    Unless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegalityPeriodAst {
    Turn,
    Round,
    Combat,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegalityFrequencyAst {
    pub maximum: u32,
    pub period: LegalityPeriodAst,
    pub per_object: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ManaUseConstraintAst {
    Any,
    CastSpells(ObjectFilter),
    ActivateAbilities(ObjectFilter),
    PayCostOf(ObjectFilter),
    SpendOnlyAsThoughAnyColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKindAst {
    Cast,
    PlayLand,
    Activate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerPermissionAst {
    pub kind: PermissionKindAst,
    pub subject: ObjectFilter,
    pub from_zones: Vec<Zone>,
    pub timing_override: Option<TimingWindowAst>,
    pub alternative_cost: Option<CompilerTotalCost>,
    pub without_paying_mana_cost: bool,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerActivationLegalityAst {
    pub relationship: LegalityRelationshipAst,
    pub timing: Option<TimingWindowAst>,
    pub condition: Option<PredicateAst>,
    pub functional_zones: Vec<Zone>,
    pub frequency: Option<LegalityFrequencyAst>,
    pub mana_use: Option<ManaUseConstraintAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerCastingLegalityAst {
    pub relationship: LegalityRelationshipAst,
    pub timing: Option<TimingWindowAst>,
    pub condition: Option<PredicateAst>,
    pub cast_from: Vec<Zone>,
    pub frequency: Option<LegalityFrequencyAst>,
    pub permission: Option<CompilerPermissionAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerTriggerLegalityAst {
    pub condition: Option<PredicateAst>,
    pub frequency: Option<LegalityFrequencyAst>,
    pub provenance: Option<SemanticProvenance>,
}
