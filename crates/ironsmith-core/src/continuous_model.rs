use crate::{
    CardType, ChooseSpec, ColorSet, ObjectFilter, Subtype, SubtypeFamily, Supertype, Value,
};

#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "continuous targets preserve the shared object-filter value model"
)]
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
#[expect(
    clippy::large_enum_variant,
    reason = "compiled modifications are shared generic values whose payload sizes vary"
)]
pub enum CompiledContinuousModification<StaticAbility, Ability> {
    AddAbility(StaticAbility),
    AddAbilityGeneric(Ability),
    RemoveAbility(Ability),
    AddCardTypes(Vec<CardType>),
    RemoveCardTypes(Vec<CardType>),
    RemoveSupertypes(Vec<Supertype>),
    SetCardTypes(Vec<CardType>),
    AddSubtypes(Vec<Subtype>),
    RemoveSubtypes(Vec<Subtype>),
    AddAllSubtypesOfFamily(SubtypeFamily),
    RemoveAllSubtypesOfFamily(SubtypeFamily),
    AddColors(ColorSet),
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
