//! The characteristics actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum CharacteristicActionAst {
    SetBasePowerToughness {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    BecomeBasePtCreature {
        power: Value,
        toughness: Value,
        target: TargetAst,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        subtype_families: Vec<SubtypeFamily>,
        colors: Option<ColorSet>,
        abilities: Vec<crate::model::CompilerStaticAbilityCore>,
        granted_abilities: Vec<GrantedAbilityAst>,
        preserve_other_types: bool,
        type_retention_surface: Option<ironsmith_core::TypeRetentionSurface>,
        animation_pt_surface: Option<ironsmith_core::AnimationPtSurface>,
        animation_duration_surface: Option<ironsmith_core::AnimationDurationSurface>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
        duration: Until,
    },
    SetBasePower {
        power: Value,
        target: TargetAst,
        duration: Until,
    },
    AddCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    SetCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    AddSubtypes {
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    },
    /// "becomes a Bird Giant" without "in addition": replaces the object's
    /// creature subtypes (CR 205.1b) instead of adding to them.
    SetCreatureSubtypes {
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    },
    BecomeSaddledUntilEndOfTurn {
        target: TargetAst,
    },
    AddColors {
        target: TargetAst,
        colors: ColorSet,
        duration: Until,
    },
    AddAllSubtypesOfFamily {
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    },
    BecomeAuraEnchantment {
        target: TargetAst,
        attachment_filter: ObjectFilter,
        granted_abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    BecomeBasicLandType {
        target: TargetAst,
        subtype: Subtype,
        duration: Until,
    },
    SetColors {
        target: TargetAst,
        colors: ColorSet,
        duration: Until,
    },
    BecomeBasicLandTypeChoice {
        target: TargetAst,
        duration: Until,
    },
    BecomeCreatureTypeChoice {
        target: TargetAst,
        duration: Until,
        excluded_subtypes: Vec<Subtype>,
    },
    BecomeColorChoice {
        target: TargetAst,
        duration: Until,
        allow_multiple: bool,
    },
    BecomeCopy {
        target: TargetAst,
        source: TargetAst,
        duration: Until,
        preserve_source_abilities: bool,
        name_override: Option<String>,
        name_override_surface: Option<SourceReferenceSurface>,
        add_supertypes: Vec<Supertype>,
        remove_supertypes: Vec<Supertype>,
        add_colors: ColorSet,
        add_card_types: Vec<CardType>,
        set_card_types: Vec<CardType>,
        add_subtypes: Vec<Subtype>,
        set_subtypes: Vec<Subtype>,
        granted_abilities: Vec<GrantedAbilityAst>,
        set_base_power_toughness: Option<(Value, Value)>,
        copy_exception_surface: Option<String>,
    },
    SetLifeTotal {
        amount: Value,
    },
    BecomeMonarch,
}
