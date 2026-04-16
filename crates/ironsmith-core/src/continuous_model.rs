use crate::{CardType, ChooseSpec, ColorSet, ObjectFilter, Subtype, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum CompiledContinuousEffectTarget {
    Source,
    Filter(ObjectFilter),
}

impl From<ChooseSpec> for CompiledContinuousEffectTarget {
    fn from(value: ChooseSpec) -> Self {
        match value {
            ChooseSpec::Source => Self::Source,
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => Self::Filter(filter),
            _ => Self::Source,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledPtSublayer {
    Setting,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompiledContinuousModification<StaticAbility, Ability> {
    AddAbility(StaticAbility),
    AddAbilityGeneric(Ability),
    RemoveAbility(Ability),
    AddCardTypes(Vec<CardType>),
    RemoveCardTypes(Vec<CardType>),
    AddSubtypes(Vec<Subtype>),
    SetColors(ColorSet),
    SetPowerToughness {
        power: Value,
        toughness: Value,
        sublayer: CompiledPtSublayer,
    },
    SetPower {
        power: Value,
        sublayer: CompiledPtSublayer,
    },
    DoesntUntap,
    MakeColorless,
    SwitchPowerToughness,
}
