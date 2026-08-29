use crate::effect::Value;
use crate::model::ast::{EffectAst, PredicateAst};
use crate::model::provenance::SemanticProvenance;
use crate::model::{CompilerActivationLegalityAst, CompilerTotalCost, TargetAst};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationTimingAst {
    AnyTime,
    SorcerySpeed,
    DuringYourTurn,
    DuringCombat,
    ManaAbility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationUsePeriodAst {
    Turn,
    Round,
    Combat,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivationUseLimitAst {
    pub count: u32,
    pub period: ActivationUsePeriodAst,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivationRestrictionAst {
    pub timing: Option<ActivationTimingAst>,
    pub condition: Option<PredicateAst>,
    pub use_limit: Option<ActivationUseLimitAst>,
    pub functional_zones: Vec<Zone>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoyaltyCostAst {
    Add(i32),
    Remove(i32),
    SetToZero,
    Variable(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaAbilityFacts {
    pub produces_mana: bool,
    pub targets: bool,
    pub loyalty_ability: bool,
    pub triggered_from_non_mana_ability: bool,
}

impl ManaAbilityFacts {
    pub fn is_mana_ability(self) -> bool {
        self.produces_mana
            && !self.targets
            && !self.loyalty_ability
            && !self.triggered_from_non_mana_ability
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivatedLineBoundaryAst {
    pub line: Option<SemanticProvenance>,
    pub cost: Option<SemanticProvenance>,
    pub colon: Option<SemanticProvenance>,
    pub effect: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerActivatedAbilityAst {
    pub cost: CompilerTotalCost,
    pub effects: Vec<EffectAst>,
    pub targets: Vec<TargetAst>,
    pub timing: ActivationTimingAst,
    pub restrictions: Vec<CompilerActivationLegalityAst>,
    pub loyalty_cost: Option<LoyaltyCostAst>,
    pub mana_facts: ManaAbilityFacts,
    pub boundary: ActivatedLineBoundaryAst,
    pub provenance: Option<SemanticProvenance>,
}
