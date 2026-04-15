//! Shared domain-model crate for the workspace refactor.
//!
//! Shared domain-model crate for the workspace refactor.
//!
//! Low-level card model and value types live here so runtime, compiler, and
//! registry layers can share them without introducing forbidden dependency
//! edges.

pub mod ability_model;
pub mod anthem_model;
pub mod attachment_model;
pub mod alternative_cast_model;
pub mod card;
pub mod color;
pub mod counter;
pub mod cost_model;
pub mod definition_model;
pub mod effect;
pub mod effect_model;
pub mod filter_model;
pub mod ids;
pub mod mana;
pub mod resolution_model;
pub mod spell_timing_model;
pub mod static_ability_model;
pub mod static_ability_id;
pub mod tag;
pub mod target_model;
pub mod types;
pub mod value_model;
pub mod zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceSplitMarker;

pub use ability_model::{
    Ability, AbilityKind, ActivatedAbility, ActivationTiming, CoreCostComponent, LevelAbility,
    ManaUsageRestriction, ManaUsageSubtypeRequirement, ProtectionFrom, RestrictedManaUnit,
    TriggeredAbility,
};
pub use anthem_model::{AnthemCountExpression, AnthemValue};
pub use alternative_cast_model::{
    AlternativeCastRequirements, AlternativeCastingMethod, TrapCondition,
};
pub use attachment_model::AuraAttachmentFilter;
pub use card::{Card, CardBuilder, LinkedFaceLayout, PowerToughness, PtValue};
pub use color::{Color, ColorSet};
pub use counter::CounterType;
pub use cost_model::{CostComponent, OptionalCost, OptionalCostsPaid, TotalCost};
pub use definition_model::CardDefinition;
pub use effect::{ChoiceCount, EffectId, SearchSelectionMode};
pub use effect_model::{Comparison, EventValueSpec, ValueComparisonOperator};
pub use filter_model::{
    AlternativeCastKind, Comparison as FilterComparison, CounterConstraint, ObjectFilter,
    ObjectRef, ParityRequirement, PlayerFilter, PtReference, SourcePowerRelation, StackObjectKind,
    TaggedObjectConstraint, TaggedOpbjectRelation,
};
pub use ids::{
    CardId, IdCountersSnapshot, ObjectId, PlayerId, StableId, reset_runtime_id_counters,
    restore_id_counters, snapshot_id_counters,
};
pub use mana::{ManaCost, ManaSymbol};
pub use resolution_model::{ResolutionProgram, ResolutionSegment, SelfReplacementBranch};
pub use spell_timing_model::ThisSpellCastTiming;
pub use static_ability_model::{
    ConditionalSpellKeywordKind, ConditionalSpellKeywordSpec, GraveyardCountMetric,
    PregameActionKind, PregameBeginOnBattlefieldSpec,
};
pub use static_ability_id::StaticAbilityId;
pub use tag::{SOURCE_EXILED_TAG, TagKey};
pub use target_model::ChooseSpec;
pub use types::{CardType, Subtype, SubtypeFamily, Supertype};
pub use value_model::{Condition, ManaSpendPermission, ManaSpendScope, Restriction, Value};
pub use zone::Zone;
