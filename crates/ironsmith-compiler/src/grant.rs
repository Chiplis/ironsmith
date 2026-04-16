use crate::costs::Cost;
use crate::static_abilities::StaticAbility;

pub type DerivedAlternativeCast = ironsmith_core::DerivedAlternativeCast<Cost>;
pub type Grantable = ironsmith_core::Grantable<
    StaticAbility,
    crate::effect::Effect,
    Cost,
    crate::static_abilities::ThisSpellCastCondition,
>;
pub type GrantSpec = ironsmith_core::GrantSpec<
    StaticAbility,
    crate::effect::Effect,
    Cost,
    crate::static_abilities::ThisSpellCastCondition,
>;
pub use ironsmith_core::GrantDuration;
