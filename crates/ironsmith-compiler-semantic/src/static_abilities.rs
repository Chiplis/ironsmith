pub use ironsmith_core::{
    ActivatedAbilityCostCondition, Anthem, AnthemCountExpression, AnthemReplacementSurface,
    AnthemValue, AttachedChosenLandwalkGrant, AttackCostCondition, AttackingGroupAttackCondition,
    CantAttackUnlessConditionSpec, CompanionDeckCardFacts, CompanionDeckCondition,
    ConditionalSpellKeywordKind, ConditionalSpellKeywordSpec, CopyActivatedAbilities,
    CopyTriggeredAbilities, CostIncrease, CostIncreaseManaCost, CostReduction,
    CostReductionManaCost, CounterRemovalFollowUp, DefendingPlayerAttackCondition,
    EnterAsCopyLinkedExilePairSpec, GraveyardCountMetric, LandwalkKind, ManaSpendPermission,
    OptionalLifeAdditionalCost, PregameActionKind, PregameBeginOnBattlefieldSpec,
    PregameRevealFromOpeningHandSpec, RemoveCardTypesForFilter, SetColorsForFilter, SpliceQuality,
    SpliceSpec, StaticAbilityId, ThisSpellCastRestrictionKind, ThisSpellCastTiming,
};

pub const PREVENT_ALL_DAMAGE_DEALT_BY_THIS_PERMANENT: StaticAbilityId =
    StaticAbilityId::PreventAllDamageDealtByThisPermanent;
pub const PREVENT_ALL_COMBAT_DAMAGE_DEALT_BY_THIS_PERMANENT: StaticAbilityId =
    StaticAbilityId::PreventAllCombatDamageDealtByThisPermanent;

pub type ThisSpellCastCondition = ironsmith_core::ThisSpellCostCondition;
pub type ThisSpellCostCondition = ironsmith_core::ThisSpellCostCondition;

pub type StaticAbility = ironsmith_core::StaticAbility<
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;
pub type StaticAbilityPayload = ironsmith_core::StaticAbilityPayload<
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;
pub type PowerToughnessChoiceOption = ironsmith_core::PowerToughnessChoiceOption<
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;
pub type AttachedAbilityGrant = ironsmith_core::AttachedAbilityGrant<
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;
pub type GrantAbility = ironsmith_core::GrantAbility<
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;
pub type GrantObjectAbilityForFilter = ironsmith_core::GrantObjectAbilityForFilter<
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;
pub type ThisSpellCostReduction = ironsmith_core::ThisSpellCostReduction<ThisSpellCostCondition>;
pub type ThisSpellCostReductionManaCost =
    ironsmith_core::ThisSpellCostReductionManaCost<ThisSpellCostCondition>;
pub type EnterAsCopyAsEntersSpec = ironsmith_core::EnterAsCopyAsEntersSpec<
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;
