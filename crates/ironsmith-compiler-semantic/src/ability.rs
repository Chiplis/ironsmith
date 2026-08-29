pub type Ability = ironsmith_core::Ability<
    crate::static_abilities::StaticAbility,
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
>;
pub type AbilityKind = ironsmith_core::AbilityKind<
    crate::static_abilities::StaticAbility,
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
>;
pub type TriggeredAbility =
    ironsmith_core::TriggeredAbility<crate::triggers::Trigger, crate::effect::Effect>;
pub type ActivatedAbility =
    ironsmith_core::ActivatedAbility<crate::effect::Effect, crate::costs::Cost>;
pub type LevelAbility = ironsmith_core::LevelAbility<crate::static_abilities::StaticAbility>;
pub use ironsmith_core::{
    ActivatedPresentationLabel, ActivationTiming, ManaPaymentPredicate, ManaPaymentPurpose,
    ManaSpendAbilityGrantDuration, ManaSpendBonusCondition, ManaSpendGrantedKeyword,
    ManaSpendPayload, ManaUsageSubtypeRequirement, PresentationKeyword, PresentationLabel,
    ProtectionFrom,
};
pub type ManaUsageRestriction = ironsmith_core::ManaUsageRestriction<crate::effect::Effect>;
pub type RestrictedManaUnit = ironsmith_core::RestrictedManaUnit<crate::effect::Effect>;

pub fn extract_static_abilities(
    abilities: &[Ability],
) -> Vec<crate::static_abilities::StaticAbility> {
    abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.clone()),
            _ => None,
        })
        .collect()
}
