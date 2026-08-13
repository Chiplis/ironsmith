use crate::model::ast::{EffectAst, PredicateAst};
use crate::model::provenance::SemanticProvenance;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StaticSubjectAst {
    Source,
    Objects(ObjectFilter),
    Player(PlayerFilter),
    Spells(ObjectFilter),
    Cards(ObjectFilter),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaticScopeAst {
    pub source_zones: Vec<Zone>,
    pub affected_zones: Vec<Zone>,
    pub controller_only: bool,
    pub owner_only: bool,
    pub during_your_turn: bool,
    pub during_opponents_turns: bool,
}

impl Default for StaticScopeAst {
    fn default() -> Self {
        Self {
            source_zones: vec![Zone::Battlefield],
            affected_zones: Vec::new(),
            controller_only: false,
            owner_only: false,
            during_your_turn: false,
            during_opponents_turns: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ContinuousLayerAst {
    Copy,
    Control,
    Text,
    Type,
    Color,
    Ability,
    PowerToughnessCharacteristicDefining,
    PowerToughnessSet,
    PowerToughnessModify,
    PowerToughnessCounter,
    Rules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CharacteristicValueAst<T> {
    Set(T),
    Add(T),
    Remove(T),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CharacteristicChangeAst {
    pub supertypes: Vec<CharacteristicValueAst<Supertype>>,
    pub card_types: Vec<CharacteristicValueAst<CardType>>,
    pub subtypes: Vec<CharacteristicValueAst<Subtype>>,
    pub power_toughness: Option<(i32, i32)>,
    pub characteristic_defining: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StaticRestrictionAst {
    CantAttack,
    CantBlock,
    CantActivate,
    CantCast,
    DoesntUntap,
    MustAttack,
    MustBlock,
    CostIncrease(crate::model::CompilerTotalCost),
    CostReduction(crate::model::CompilerTotalCost),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerGrantedAbilityAst {
    Keyword(crate::model::CompilerKeywordAbilityAst),
    Static(Box<CompilerStaticAbilityAst>),
    Activated(Box<crate::model::CompilerActivatedAbilityAst>),
    Triggered(Box<crate::model::CompilerTriggeredAbilityAst>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StaticOperationAst {
    Characteristics(CharacteristicChangeAst),
    Grant {
        abilities: Vec<CompilerGrantedAbilityAst>,
        remove_other_abilities: bool,
    },
    RemoveKeywords(Vec<KeywordAction>),
    Restriction(StaticRestrictionAst),
    Replacement(Vec<EffectAst>),
    Permission(Vec<EffectAst>),
    RuleChange,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerStaticAbilityAst {
    pub subject: StaticSubjectAst,
    pub scope: StaticScopeAst,
    pub condition: Option<PredicateAst>,
    pub layer: ContinuousLayerAst,
    pub operation: StaticOperationAst,
    pub provenance: Option<SemanticProvenance>,
}

impl CompilerStaticAbilityAst {
    pub(crate) fn granted_abilities(&self) -> &[CompilerGrantedAbilityAst] {
        match &self.operation {
            StaticOperationAst::Grant { abilities, .. } => abilities,
            _ => &[],
        }
    }
}
