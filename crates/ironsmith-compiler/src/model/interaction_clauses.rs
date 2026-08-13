//! Typed complements for damage, prevention, counters, combat, and characteristic changes.

use crate::model::clauses::ClauseDurationAst;
use crate::model::object_action_clauses::CompilerObjectOperandAst;
use crate::model::selections::{CompilerFilterAst, CompilerValueAst};
use crate::model::static_abilities::ContinuousLayerAst;
use crate::model::symbols::SymbolReference;
use crate::object::CounterType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerDamageDivisionAst {
    None,
    Evenly,
    AsChosen,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerDamageClauseAst {
    pub source: CompilerObjectOperandAst,
    pub recipients: CompilerObjectOperandAst,
    pub amount: CompilerValueAst,
    pub division: CompilerDamageDivisionAst,
    pub chooser: Option<CompilerFilterAst>,
    pub combat_damage: bool,
    pub unpreventable: bool,
    pub result: SymbolReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerPreventionKindAst {
    Amount,
    All,
    NextEvent,
    Combat,
    Redirect,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerPreventionClauseAst {
    pub kind: CompilerPreventionKindAst,
    pub source: Option<CompilerObjectOperandAst>,
    pub recipient: Option<CompilerObjectOperandAst>,
    pub amount: Option<CompilerValueAst>,
    pub duration: Option<ClauseDurationAst>,
    pub redirect_to: Option<CompilerObjectOperandAst>,
    pub shield: SymbolReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerCounterOperationAst {
    Add,
    Remove,
    Move,
    Double,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerCounterAmountAst {
    Value(CompilerValueAst),
    All,
    Existing,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerCounterClauseAst {
    pub operation: CompilerCounterOperationAst,
    pub counter_type: Option<CounterType>,
    pub amount: CompilerCounterAmountAst,
    pub object: CompilerObjectOperandAst,
    pub destination: Option<CompilerObjectOperandAst>,
    pub distributed: bool,
    pub affected: SymbolReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerCombatOperationAst {
    Fight,
    Goad,
    Detain,
    Suspect,
    ClearSuspected,
    RemoveFromCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerCombatRoleAst {
    Fighter,
    Attacker,
    Blocker,
    Affected,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerCombatClauseAst {
    pub operation: CompilerCombatOperationAst,
    pub primary: CompilerObjectOperandAst,
    pub primary_role: CompilerCombatRoleAst,
    pub opposing: Option<CompilerObjectOperandAst>,
    pub opposing_role: Option<CompilerCombatRoleAst>,
    pub duration: Option<ClauseDurationAst>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerModificationModeAst {
    OneShot,
    Continuous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerCharacteristicOperationAst {
    AddPowerToughness,
    SetPowerToughness,
    SetPower,
    SwitchPowerToughness,
    ScalePowerToughness,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerCharacteristicClauseAst {
    pub mode: CompilerModificationModeAst,
    pub layer: ContinuousLayerAst,
    pub operation: CompilerCharacteristicOperationAst,
    pub object: CompilerObjectOperandAst,
    pub power: Option<CompilerValueAst>,
    pub toughness: Option<CompilerValueAst>,
    pub duration: Option<ClauseDurationAst>,
    pub affected: SymbolReference,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerInteractionClauseAst {
    Damage(CompilerDamageClauseAst),
    Prevention(CompilerPreventionClauseAst),
    Counter(CompilerCounterClauseAst),
    Combat(CompilerCombatClauseAst),
    Characteristic(CompilerCharacteristicClauseAst),
}
