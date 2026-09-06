//! The statchanges actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum StatChangeActionAst {
    Pump {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        condition: Option<PredicateAst>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    PumpForEach {
        power_per: i32,
        toughness_per: i32,
        target: TargetAst,
        count: Value,
        duration: Until,
    },
    PumpAll {
        filter: ObjectFilter,
        power: Value,
        toughness: Value,
        duration: Until,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    PumpByLastEffect {
        power: i32,
        toughness: i32,
        target: TargetAst,
        duration: Until,
        includes_this_way: bool,
    },
    RemoveCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    RemoveSubtypes {
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    },
    RemoveAllSubtypesOfFamily {
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    },
    MakeColorless {
        target: TargetAst,
        duration: Until,
    },
    RemoveAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<PredicateAst>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    RemoveAbilitiesFromTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
}
